use cosmwasm_std::{Empty, Timestamp};
use cw721::{Cw721QueryMsg, OwnerOfResponse};
use cw_multi_test::Executor;
use sg_controllers::HooksResponse;
use sg_std::GENESIS_MINT_START_TIME;

use cosmwasm_std::{coin, coins, Uint128};
use sg721::ExecuteMsg as Sg721ExecuteMsg;
use sg_std::NATIVE_DENOM;
use test_suite::common_setup::setup_accounts_and_block::setup_block_time;

use crate::{
    helpers::ExpiryRange,
    msg::{ExecuteMsg, QueryMsg, SudoMsg},
    state::PriceRange,
    testing::{
        helpers::{funds::listing_funds, nft_functions::mint},
        setup::{
            setup_marketplace::{
                setup_fair_burn, setup_marketplace, setup_marketplace_and_collections_with_params,
                LISTING_FEE, MAX_ENTRY_REMOVAL_PER_BLOCK, MAX_EXPIRY, MAX_FINDERS_FEE_BPS,
                MAX_FIXED_PRICE_ASK_AMOUNT, MIN_EXPIRY, REMOVAL_REWARD_BPS, TRADING_FEE_BPS,
            },
            templates::standard_minter_template,
        },
    },
};

#[test]
fn try_add_remove_listed_hooks() {
    let vt = standard_minter_template(1);
    let (mut router, owner) = (vt.router, vt.accts.owner);
    let fair_burn = setup_fair_burn(&mut router, &owner).unwrap();
    let marketplace = setup_marketplace(&mut router, fair_burn, owner).unwrap();
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);
    let add_hook_msg = SudoMsg::AddAskHook {
        hook: "hook".to_string(),
    };
    let response = router.wasm_sudo(marketplace.clone(), &add_hook_msg);
    assert!(response.is_ok());

    let query_hooks_msg = QueryMsg::AskHooks {};
    let response: HooksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_hooks_msg)
        .unwrap();
    assert_eq!(response.hooks, vec!["hook".to_string()]);

    let remove_hook_msg = SudoMsg::RemoveAskHook {
        hook: "hook".to_string(),
    };
    let response = router.wasm_sudo(marketplace.clone(), &remove_hook_msg);
    assert!(response.is_ok());

    let response: HooksResponse = router
        .wrap()
        .query_wasm_smart(marketplace, &query_hooks_msg)
        .unwrap();
    assert!(response.hooks.is_empty());
}

#[test]
fn try_add_remove_bid_hooks() {
    let vt = standard_minter_template(1);
    let (mut router, owner) = (vt.router, vt.accts.owner);
    let fair_burn = setup_fair_burn(&mut router, &owner).unwrap();
    let marketplace = setup_marketplace(&mut router, fair_burn, owner).unwrap();
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);

    let add_offer_hook_msg = SudoMsg::AddOfferHook {
        hook: "offer_hook".to_string(),
    };
    let response = router.wasm_sudo(marketplace.clone(), &add_offer_hook_msg);
    assert!(response.is_ok());

    let query_hooks_msg = QueryMsg::OfferHooks {};
    let response: HooksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_hooks_msg)
        .unwrap();
    assert_eq!(response.hooks, vec!["offer_hook".to_string()]);

    let remove_hook_msg = SudoMsg::RemoveOfferHook {
        hook: "offer_hook".to_string(),
    };
    let response = router.wasm_sudo(marketplace.clone(), &remove_hook_msg);
    assert!(response.is_ok());

    let response: HooksResponse = router
        .wrap()
        .query_wasm_smart(marketplace, &query_hooks_msg)
        .unwrap();
    assert!(response.hooks.is_empty());
}

#[test]
fn try_add_remove_sales_hooks() {
    let vt = standard_minter_template(1);
    let (mut router, owner) = (vt.router, vt.accts.owner);
    let fair_burn = setup_fair_burn(&mut router, &owner).unwrap();
    let marketplace = setup_marketplace(&mut router, fair_burn, owner).unwrap();
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);

    let add_hook_msg = SudoMsg::AddSaleHook {
        hook: "hook".to_string(),
    };
    let response = router.wasm_sudo(marketplace.clone(), &add_hook_msg);
    assert!(response.is_ok());

    let query_hooks_msg = QueryMsg::SaleHooks {};
    let response: HooksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_hooks_msg)
        .unwrap();
    assert_eq!(response.hooks, vec!["hook".to_string()]);

    let remove_hook_msg = SudoMsg::RemoveSaleHook {
        hook: "hook".to_string(),
    };
    let response = router.wasm_sudo(marketplace.clone(), &remove_hook_msg);
    assert!(response.is_ok());

    let response: HooksResponse = router
        .wrap()
        .query_wasm_smart(marketplace, &query_hooks_msg)
        .unwrap();
    assert!(response.hooks.is_empty());
}

#[test]
fn try_init_hook() {
    let vt = standard_minter_template(1);
    let (mut router, creator) = (vt.router, vt.accts.creator);
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);

    let msg = crate::msg::InstantiateMsg {
        fair_burn: "fair_burn".to_string(),
        listing_fee: coin(LISTING_FEE, NATIVE_DENOM),
        ask_expiry: ExpiryRange::new(MIN_EXPIRY, MAX_EXPIRY),
        offer_expiry: ExpiryRange::new(MIN_EXPIRY, MAX_EXPIRY),
        operators: vec!["operator1".to_string()],
        max_asks_removed_per_block: MAX_ENTRY_REMOVAL_PER_BLOCK,
        max_offers_removed_per_block: MAX_ENTRY_REMOVAL_PER_BLOCK,
        max_collection_offers_removed_per_block: MAX_ENTRY_REMOVAL_PER_BLOCK,
        trading_fee_bps: TRADING_FEE_BPS,
        max_finders_fee_bps: MAX_FINDERS_FEE_BPS,
        removal_reward_bps: REMOVAL_REWARD_BPS,
        price_ranges: vec![(
            NATIVE_DENOM.to_string(),
            PriceRange {
                min: Uint128::from(5u128),
                max: Uint128::from(MAX_FIXED_PRICE_ASK_AMOUNT),
            },
        )],
        sale_hook: Some("hook".to_string()),
    };
    let marketplace =
        setup_marketplace_and_collections_with_params(&mut router, creator, msg).unwrap();

    let query_hooks_msg = QueryMsg::SaleHooks {};
    let response: HooksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_hooks_msg)
        .unwrap();
    assert_eq!(response.hooks, vec!["hook".to_string()]);

    let remove_hook_msg = SudoMsg::RemoveSaleHook {
        hook: "hook".to_string(),
    };
    let response = router.wasm_sudo(marketplace.clone(), &remove_hook_msg);
    assert!(response.is_ok());

    let response: HooksResponse = router
        .wrap()
        .query_wasm_smart(marketplace, &query_hooks_msg)
        .unwrap();
    assert!(response.hooks.is_empty());
}

#[test]
fn try_hook_was_run() {
    let vt = standard_minter_template(1);
    let (mut router, owner, bidder, creator) =
        (vt.router, vt.accts.owner, vt.accts.bidder, vt.accts.creator);
    let fair_burn = setup_fair_burn(&mut router, &owner).unwrap();
    let marketplace = setup_marketplace(&mut router, fair_burn, owner).unwrap();
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

    // Add offer created hook
    let add_offer_hook_msg = SudoMsg::AddOfferHook {
        hook: "offer_created_hook".to_string(),
    };
    let _res = router.wasm_sudo(marketplace.clone(), &add_offer_hook_msg);

    // Mint NFT for creator
    mint(&mut router, &creator, &minter_addr);

    // An ask is made by the creator, but fails because NFT is not authorized
    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        price: coin(100, NATIVE_DENOM),
        asset_recipient: None,
        reserve_for: None,
        finders_fee_bps: Some(0),
        expires: None,
    };
    // Creator Authorizes NFT
    let approve_msg: Sg721ExecuteMsg<Empty, Empty> = Sg721ExecuteMsg::Approve {
        spender: marketplace.to_string(),
        token_id: token_id.to_string(),
        expires: None,
    };
    let response = router.execute_contract(creator.clone(), collection.clone(), &approve_msg, &[]);
    assert!(response.is_ok());
    // Now set_ask succeeds
    let response = router.execute_contract(
        creator.clone(),
        marketplace.clone(),
        &set_ask,
        &listing_funds(LISTING_FEE).unwrap(),
    );
    assert!(response.is_ok());
    assert_eq!(
        "ask-hook-failed",
        response.unwrap().events[3].attributes[1].value
    );

    // Bidder makes offer that meets the ask criteria
    let set_offer_msg = ExecuteMsg::SetOffer {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        asset_recipient: None,
        finders_fee_bps: None,
        finder: None,
        expires: Some(start_time.plus_seconds(MIN_EXPIRY + 1)),
    };

    // Offer succeeds even though the hook contract cannot be found
    let response = router.execute_contract(
        bidder.clone(),
        marketplace,
        &set_offer_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(response.is_ok());
    assert_eq!(
        "sale-hook-failed",
        response.as_ref().unwrap().events[11].attributes[1].value
    );

    // NFT is still transferred despite a sale finalized hook failing
    let query_owner_msg = Cw721QueryMsg::OwnerOf {
        token_id: token_id.to_string(),
        include_expired: None,
    };
    let response: OwnerOfResponse = router
        .wrap()
        .query_wasm_smart(collection, &query_owner_msg)
        .unwrap();
    assert_eq!(response.owner, bidder.to_string());
}

#[test]
fn try_add_too_many_sales_hooks() {
    let vt = standard_minter_template(1);
    let (mut router, owner) = (vt.router, vt.accts.owner);
    let fair_burn = setup_fair_burn(&mut router, &owner).unwrap();
    let marketplace = setup_marketplace(&mut router, fair_burn, owner).unwrap();
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
