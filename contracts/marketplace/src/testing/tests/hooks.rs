use crate::helpers::ExpiryRange;
use crate::msg::SudoMsg;
use crate::msg::{ExecuteMsg, QueryMsg};
use crate::state::SaleType;
use crate::testing::helpers::funds::listing_funds;
use crate::testing::helpers::nft_functions::mint;
use crate::testing::setup::setup_marketplace::{
    setup_marketplace, setup_marketplace_and_collections_with_params, BID_REMOVAL_REWARD_BPS,
    LISTING_FEE, MAX_EXPIRY, MAX_FINDERS_FEE_BPS, MIN_EXPIRY, TRADING_FEE_BPS,
};
use cosmwasm_std::{Empty, Timestamp};
use cw721::{Cw721QueryMsg, OwnerOfResponse};
use cw_multi_test::Executor;
use sg_controllers::HooksResponse;
use sg_std::GENESIS_MINT_START_TIME;

use cosmwasm_std::{coin, coins, Uint128};
use cw_utils::Duration;
use sg721::ExecuteMsg as Sg721ExecuteMsg;
use test_suite::common_setup::setup_accounts_and_block::setup_block_time;

use crate::testing::setup::templates::standard_minter_template;
use sg_std::NATIVE_DENOM;

#[test]
fn try_add_remove_listed_hooks() {
    let vt = standard_minter_template(1);
    let (mut router, owner) = (vt.router, vt.accts.owner);
    let marketplace = setup_marketplace(&mut router, owner).unwrap();
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);
    let add_hook_msg = SudoMsg::AddAskHook {
        hook: "hook".to_string(),
    };
    let res = router.wasm_sudo(marketplace.clone(), &add_hook_msg);
    assert!(res.is_ok());

    let query_hooks_msg = QueryMsg::AskHooks {};
    let res: HooksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_hooks_msg)
        .unwrap();
    assert_eq!(res.hooks, vec!["hook".to_string()]);

    let remove_hook_msg = SudoMsg::RemoveAskHook {
        hook: "hook".to_string(),
    };
    let res = router.wasm_sudo(marketplace.clone(), &remove_hook_msg);
    assert!(res.is_ok());

    let res: HooksResponse = router
        .wrap()
        .query_wasm_smart(marketplace, &query_hooks_msg)
        .unwrap();
    assert!(res.hooks.is_empty());
}

#[test]
fn try_add_remove_bid_hooks() {
    let vt = standard_minter_template(1);
    let (mut router, owner) = (vt.router, vt.accts.owner);
    let marketplace = setup_marketplace(&mut router, owner).unwrap();
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);

    let add_bid_hook_msg = SudoMsg::AddBidHook {
        hook: "bid_hook".to_string(),
    };
    let res = router.wasm_sudo(marketplace.clone(), &add_bid_hook_msg);
    assert!(res.is_ok());

    let query_hooks_msg = QueryMsg::BidHooks {};
    let res: HooksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_hooks_msg)
        .unwrap();
    assert_eq!(res.hooks, vec!["bid_hook".to_string()]);

    let remove_hook_msg = SudoMsg::RemoveBidHook {
        hook: "bid_hook".to_string(),
    };
    let res = router.wasm_sudo(marketplace.clone(), &remove_hook_msg);
    assert!(res.is_ok());

    let res: HooksResponse = router
        .wrap()
        .query_wasm_smart(marketplace, &query_hooks_msg)
        .unwrap();
    assert!(res.hooks.is_empty());
}

#[test]
fn try_add_remove_sales_hooks() {
    let vt = standard_minter_template(1);
    let (mut router, owner) = (vt.router, vt.accts.owner);
    let marketplace = setup_marketplace(&mut router, owner).unwrap();
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);

    let add_hook_msg = SudoMsg::AddSaleHook {
        hook: "hook".to_string(),
    };
    let res = router.wasm_sudo(marketplace.clone(), &add_hook_msg);
    assert!(res.is_ok());

    let query_hooks_msg = QueryMsg::SaleHooks {};
    let res: HooksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_hooks_msg)
        .unwrap();
    assert_eq!(res.hooks, vec!["hook".to_string()]);

    let remove_hook_msg = SudoMsg::RemoveSaleHook {
        hook: "hook".to_string(),
    };
    let res = router.wasm_sudo(marketplace.clone(), &remove_hook_msg);
    assert!(res.is_ok());

    let res: HooksResponse = router
        .wrap()
        .query_wasm_smart(marketplace, &query_hooks_msg)
        .unwrap();
    assert!(res.hooks.is_empty());
}

#[test]
fn try_init_hook() {
    let vt = standard_minter_template(1);
    let (mut router, creator) = (vt.router, vt.accts.creator);
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);

    let msg = crate::msg::InstantiateMsg {
        operators: vec!["operator1".to_string()],
        trading_fee_bps: TRADING_FEE_BPS,
        ask_expiry: ExpiryRange::new(MIN_EXPIRY, MAX_EXPIRY),
        bid_expiry: ExpiryRange::new(MIN_EXPIRY, MAX_EXPIRY),
        sale_hook: Some("hook".to_string()),
        max_finders_fee_bps: MAX_FINDERS_FEE_BPS,
        min_price: Uint128::from(5u128),
        stale_bid_duration: Duration::Time(100),
        bid_removal_reward_bps: BID_REMOVAL_REWARD_BPS,
        listing_fee: Uint128::from(LISTING_FEE),
    };
    let marketplace =
        setup_marketplace_and_collections_with_params(&mut router, creator, msg).unwrap();

    let query_hooks_msg = QueryMsg::SaleHooks {};
    let res: HooksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_hooks_msg)
        .unwrap();
    assert_eq!(res.hooks, vec!["hook".to_string()]);

    let remove_hook_msg = SudoMsg::RemoveSaleHook {
        hook: "hook".to_string(),
    };
    let res = router.wasm_sudo(marketplace.clone(), &remove_hook_msg);
    assert!(res.is_ok());

    let res: HooksResponse = router
        .wrap()
        .query_wasm_smart(marketplace, &query_hooks_msg)
        .unwrap();
    assert!(res.hooks.is_empty());
}

#[test]
fn try_hook_was_run() {
    let vt = standard_minter_template(1);
    let (mut router, owner, bidder, creator) =
        (vt.router, vt.accts.owner, vt.accts.bidder, vt.accts.creator);
    let marketplace = setup_marketplace(&mut router, owner).unwrap();
    let minter_addr = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);
    let token_id = 1;

    // Add sales hook
    let add_hook_msg = SudoMsg::AddSaleHook {
        hook: "hook".to_string(),
    };
    let _res = router.wasm_sudo(marketplace.clone(), &add_hook_msg);

    // Add listed hook
    let add_ask_hook_msg = SudoMsg::AddAskHook {
        hook: "ask_created_hook".to_string(),
    };
    let _res = router.wasm_sudo(marketplace.clone(), &add_ask_hook_msg);

    // Add bid created hook
    let add_ask_hook_msg = SudoMsg::AddBidHook {
        hook: "bid_created_hook".to_string(),
    };
    let _res = router.wasm_sudo(marketplace.clone(), &add_ask_hook_msg);

    // Mint NFT for creator
    mint(&mut router, &creator, &minter_addr);

    // An ask is made by the creator, but fails because NFT is not authorized
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id,
        price: coin(100, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 1),
        finders_fee_bps: Some(0),
    };
    // Creator Authorizes NFT
    let approve_msg: Sg721ExecuteMsg<Empty, Empty> = Sg721ExecuteMsg::Approve {
        spender: marketplace.to_string(),
        token_id: token_id.to_string(),
        expires: None,
    };
    let res = router.execute_contract(creator.clone(), collection.clone(), &approve_msg, &[]);
    assert!(res.is_ok());
    // Now set_ask succeeds
    let res = router.execute_contract(
        creator.clone(),
        marketplace.clone(),
        &set_ask,
        &listing_funds(LISTING_FEE).unwrap(),
    );
    assert!(res.is_ok());
    assert_eq!(
        "ask-hook-failed",
        res.unwrap().events[3].attributes[1].value
    );

    // Bidder makes bid that meets the ask criteria
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id,
        finders_fee_bps: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };

    // Bid succeeds even though the hook contract cannot be found
    let res = router.execute_contract(
        bidder.clone(),
        marketplace,
        &set_bid_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(res.is_ok());
    assert_eq!(
        "sale-hook-failed",
        res.as_ref().unwrap().events[10].attributes[1].value
    );

    // NFT is still transferred despite a sale finalized hook failing
    let query_owner_msg = Cw721QueryMsg::OwnerOf {
        token_id: token_id.to_string(),
        include_expired: None,
    };
    let res: OwnerOfResponse = router
        .wrap()
        .query_wasm_smart(collection, &query_owner_msg)
        .unwrap();
    assert_eq!(res.owner, bidder.to_string());
}

#[test]
fn try_add_too_many_sales_hooks() {
    let vt = standard_minter_template(1);
    let (mut router, owner) = (vt.router, vt.accts.owner);
    let marketplace = setup_marketplace(&mut router, owner).unwrap();
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);
    let add_hook_msg = SudoMsg::AddSaleHook {
        hook: "hook2".to_string(),
    };
    let res = router.wasm_sudo(marketplace.clone(), &add_hook_msg);
    assert!(res.is_ok());

    let add_hook_msg = SudoMsg::AddSaleHook {
        hook: "hook3".to_string(),
    };
    let res = router.wasm_sudo(marketplace.clone(), &add_hook_msg);
    assert!(res.is_ok());

    let add_hook_msg = SudoMsg::AddSaleHook {
        hook: "hook4".to_string(),
    };
    let res = router.wasm_sudo(marketplace.clone(), &add_hook_msg);
    assert!(res.is_ok());

    let add_hook_msg = SudoMsg::AddSaleHook {
        hook: "hook5".to_string(),
    };
    let res = router.wasm_sudo(marketplace.clone(), &add_hook_msg);
    assert!(res.is_ok());

    let add_hook_msg = SudoMsg::AddSaleHook {
        hook: "hook6".to_string(),
    };
    let res = router.wasm_sudo(marketplace.clone(), &add_hook_msg);
    assert!(res.is_ok());

    let add_hook_msg = SudoMsg::AddSaleHook {
        hook: "hook7".to_string(),
    };
    let res = router.wasm_sudo(marketplace, &add_hook_msg);
    assert!(res.is_err());
}
