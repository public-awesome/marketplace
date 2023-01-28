use crate::error::ContractError;
use crate::msg::ExecuteMsg;
use crate::state::SaleType;
use crate::testing::helpers::funds::{
    add_funds_for_incremental_fee, calculated_creator_balance_after_fairburn, listing_funds,
};
use crate::testing::helpers::nft_functions::{approve, mint};
use crate::testing::setup::setup_accounts::INITIAL_BALANCE;
use crate::testing::setup::setup_marketplace::{setup_marketplace, LISTING_FEE, MIN_EXPIRY};
use cosmwasm_std::{Addr, Timestamp};
use cw721::{Cw721QueryMsg, OwnerOfResponse};
use cw_multi_test::Executor;
use sg_std::GENESIS_MINT_START_TIME;

use cosmwasm_std::{coin, coins};
use test_suite::common_setup::setup_accounts_and_block::setup_block_time;

use crate::testing::setup::templates::{minter_with_curator, standard_minter_template};
use sg_std::NATIVE_DENOM;

#[test]
fn try_bidder_cannot_be_finder() {
    let vt = standard_minter_template(1);
    let (mut router, owner, bidder, creator) =
        (vt.router, vt.accts.owner, vt.accts.bidder, vt.accts.creator);
    let marketplace = setup_marketplace(&mut router, owner).unwrap();
    let minter_addr = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let token_id = 1;
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    setup_block_time(&mut router, start_time.nanos(), None);

    // Mint NFT for creator
    mint(&mut router, &creator, &minter_addr);
    approve(&mut router, &creator, &collection, &marketplace, token_id);

    // Bidder makes bid with a finder's fee
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id,
        finders_fee_bps: Some(500),
        expires: start_time.plus_seconds(MIN_EXPIRY + 1),
        finder: Some(bidder.to_string()),
    };
    router
        .execute_contract(
            bidder,
            marketplace.clone(),
            &set_bid_msg,
            &coins(100, NATIVE_DENOM),
        )
        .unwrap_err();
}

#[test]
fn try_bid_finders_fee() {
    let vt = standard_minter_template(1);
    let (mut router, owner, bidder, creator) =
        (vt.router, vt.accts.owner, vt.accts.bidder, vt.accts.creator);
    let marketplace = setup_marketplace(&mut router, owner).unwrap();
    let minter_addr = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let token_id = 1;
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    setup_block_time(&mut router, start_time.nanos(), None);

    // Mint NFT for creator
    mint(&mut router, &creator, &minter_addr);
    approve(&mut router, &creator, &collection, &marketplace, token_id);

    // Bidder makes failed bid with a large finder's fee
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id,
        finders_fee_bps: Some(5000),
        expires: start_time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };
    let err = router
        .execute_contract(
            bidder.clone(),
            marketplace.clone(),
            &set_bid_msg,
            &coins(100, NATIVE_DENOM),
        )
        .unwrap_err();
    assert_eq!(
        err.source().unwrap().to_string(),
        ContractError::InvalidFindersFeeBps(5000).to_string()
    );

    // Bidder makes bid with a finder's fee
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::Auction,
        collection: collection.to_string(),
        token_id,
        finders_fee_bps: Some(500),
        expires: start_time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };
    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_bid_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    let finder = Addr::unchecked("finder".to_string());

    // Token owner accepts the bid with a finder address
    let accept_bid_msg = ExecuteMsg::AcceptBid {
        collection: collection.to_string(),
        token_id,
        bidder: bidder.to_string(),
        finder: Some(finder.to_string()),
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &accept_bid_msg, &[]);
    assert!(res.is_ok());

    let finder_balances = router.wrap().query_all_balances(finder).unwrap();
    assert_eq!(finder_balances, coins(5, NATIVE_DENOM));
}

#[test]
fn try_royalties() {
    let vt = minter_with_curator(1);
    let (mut router, owner, bidder, creator) =
        (vt.router, vt.accts.owner, vt.accts.bidder, vt.accts.creator);
    let marketplace = setup_marketplace(&mut router, owner).unwrap();
    let minter_addr = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);
    let token_id = 1;
    add_funds_for_incremental_fee(
        &mut router,
        &Addr::unchecked("curator"),
        INITIAL_BALANCE,
        1u128,
    )
    .unwrap();
    // Mint NFT for creator
    mint(&mut router, &creator, &minter_addr);
    approve(&mut router, &creator, &collection, &marketplace, token_id);

    // An ask is made by the creator
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
    let res = router.execute_contract(
        creator.clone(),
        marketplace.clone(),
        &set_ask,
        &listing_funds(LISTING_FEE).unwrap(),
    );
    assert!(res.is_ok());

    // Bidder makes bid
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id,
        finders_fee_bps: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };
    let res = router.execute_contract(
        bidder.clone(),
        marketplace,
        &set_bid_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    // Check money is transferred correctly and royalties paid
    let curator_native_balances = router
        .wrap()
        .query_all_balances("curator".to_string())
        .unwrap();
    assert_eq!(
        curator_native_balances,
        coins(INITIAL_BALANCE + 10, NATIVE_DENOM)
    );

    let creator_native_balances = router.wrap().query_all_balances(creator).unwrap();
    // 100 - 10 (royalties) - 2 (fee)
    let creator_balance_after_fee = calculated_creator_balance_after_fairburn();
    assert_eq!(
        creator_native_balances,
        coins(
            creator_balance_after_fee.u128() + 100 - 10 - 2,
            NATIVE_DENOM
        )
    );
    let bidder_native_balances = router.wrap().query_all_balances(bidder.clone()).unwrap();
    assert_eq!(
        bidder_native_balances,
        coins(INITIAL_BALANCE - 100, NATIVE_DENOM)
    );

    // Check NFT is transferred
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
