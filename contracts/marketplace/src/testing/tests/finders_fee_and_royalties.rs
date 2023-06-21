use cosmwasm_std::Decimal;
use cosmwasm_std::{coin, coins};
use cosmwasm_std::{Addr, Timestamp};
use cw721::{Cw721QueryMsg, OwnerOfResponse};
use cw_multi_test::Executor;
use sg721::RoyaltyInfoResponse;
use sg_std::GENESIS_MINT_START_TIME;
use sg_std::NATIVE_DENOM;
use test_suite::common_setup::setup_accounts_and_block::setup_block_time;

use crate::{
    msg::ExecuteMsg,
    testing::{
        helpers::{
            funds::{
                add_funds_for_incremental_fee, calculated_creator_balance_after_fairburn,
                listing_funds,
            },
            nft_functions::{approve, mint},
        },
        setup::{
            setup_accounts::INITIAL_BALANCE,
            setup_marketplace::{setup_fair_burn, setup_marketplace, LISTING_FEE, MIN_EXPIRY},
            templates::{minter_with_curator, minter_with_royalties, standard_minter_template},
        },
    },
    ContractError,
};

#[test]
fn try_bidder_cannot_be_finder() {
    let vt = standard_minter_template(1);
    let (mut router, owner, bidder, creator) =
        (vt.router, vt.accts.owner, vt.accts.bidder, vt.accts.creator);
    let fair_burn = setup_fair_burn(&mut router, &creator).unwrap();
    let marketplace = setup_marketplace(&mut router, fair_burn, owner).unwrap();
    let minter_addr = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let token_id = 1;
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    setup_block_time(&mut router, start_time.nanos(), None);

    // Mint NFT for creator
    mint(&mut router, &creator, &minter_addr);
    approve(&mut router, &creator, &collection, &marketplace, token_id);

    // Bidder makes offer with a finder's fee
    let set_offer_msg = ExecuteMsg::SetOffer {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        asset_recipient: None,
        finders_fee_bps: Some(500),
        finder: Some(bidder.to_string()),
        expires: Some(start_time.plus_seconds(MIN_EXPIRY + 1)),
    };
    router
        .execute_contract(
            bidder,
            marketplace.clone(),
            &set_offer_msg,
            &coins(100, NATIVE_DENOM),
        )
        .unwrap_err();
}

#[test]
fn try_bid_finders_fee() {
    let vt = standard_minter_template(1);
    let (mut router, owner, bidder, creator) =
        (vt.router, vt.accts.owner, vt.accts.bidder, vt.accts.creator);
    let fair_burn = setup_fair_burn(&mut router, &creator).unwrap();
    let marketplace = setup_marketplace(&mut router, fair_burn, owner).unwrap();
    let minter_addr = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let token_id = 1;
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    setup_block_time(&mut router, start_time.nanos(), None);

    // Mint NFT for creator
    mint(&mut router, &creator, &minter_addr);
    approve(&mut router, &creator, &collection, &marketplace, token_id);

    // Bidder makes failed bid with a large finder's fee
    let set_offer_msg = ExecuteMsg::SetOffer {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        asset_recipient: None,
        finders_fee_bps: Some(5000),
        finder: None,
        expires: Some(start_time.plus_seconds(MIN_EXPIRY + 1)),
    };
    let err = router
        .execute_contract(
            bidder.clone(),
            marketplace.clone(),
            &set_offer_msg,
            &coins(100, NATIVE_DENOM),
        )
        .unwrap_err();
    assert_eq!(
        err.source().unwrap().to_string(),
        ContractError::InvalidFindersFeePercent(Decimal::percent(50)).to_string()
    );

    // Bidder makes offer with a finder's fee
    let set_offer_msg = ExecuteMsg::SetOffer {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        asset_recipient: None,
        finders_fee_bps: Some(500),
        finder: None,
        expires: Some(start_time.plus_seconds(MIN_EXPIRY + 1)),
    };
    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_offer_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(response.is_ok());

    let finder = Addr::unchecked("finder".to_string());

    // Token owner accepts the bid with a finder address
    let accept_offer_msg = ExecuteMsg::AcceptOffer {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        bidder: bidder.to_string(),
        asset_recipient: None,
        finder: Some(finder.to_string()),
    };
    let response =
        router.execute_contract(creator.clone(), marketplace.clone(), &accept_offer_msg, &[]);
    assert!(response.is_ok());

    let finder_balances = router.wrap().query_all_balances(finder).unwrap();
    assert_eq!(finder_balances, coins(5, NATIVE_DENOM));
}

#[test]
fn try_royalties() {
    let vt = minter_with_curator(1);
    let (mut router, owner, bidder, creator) =
        (vt.router, vt.accts.owner, vt.accts.bidder, vt.accts.creator);
    let fair_burn = setup_fair_burn(&mut router, &creator).unwrap();
    let marketplace = setup_marketplace(&mut router, fair_burn, owner).unwrap();
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
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        price: coin(100, NATIVE_DENOM),
        asset_recipient: None,
        reserve_for: None,
        finders_fee_bps: Some(0),
        expires: None,
    };
    let response = router.execute_contract(
        creator.clone(),
        marketplace.clone(),
        &set_ask,
        &listing_funds(LISTING_FEE).unwrap(),
    );
    assert!(response.is_ok());

    // Bidder makes bid
    let set_offer_msg = ExecuteMsg::SetOffer {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        asset_recipient: None,
        finders_fee_bps: None,
        finder: None,
        expires: Some(start_time.plus_seconds(MIN_EXPIRY + 1)),
    };
    let response = router.execute_contract(
        bidder.clone(),
        marketplace,
        &set_offer_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(response.is_ok());

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
    let response: OwnerOfResponse = router
        .wrap()
        .query_wasm_smart(collection, &query_owner_msg)
        .unwrap();
    assert_eq!(response.owner, bidder.to_string());
}

#[test]
fn try_empty_royalties() {
    let vt = minter_with_royalties(1, None);
    let (mut router, owner, bidder, creator) =
        (vt.router, vt.accts.owner, vt.accts.bidder, vt.accts.creator);
    let fair_burn = setup_fair_burn(&mut router, &creator).unwrap();
    let marketplace = setup_marketplace(&mut router, fair_burn, owner).unwrap();
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
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        price: coin(100, NATIVE_DENOM),
        asset_recipient: None,
        reserve_for: None,
        finders_fee_bps: Some(0),
        expires: None,
    };
    let response = router.execute_contract(
        creator.clone(),
        marketplace.clone(),
        &set_ask,
        &listing_funds(LISTING_FEE).unwrap(),
    );
    assert!(response.is_ok());

    // Bidder makes bid
    let set_offer_msg = ExecuteMsg::SetOffer {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        asset_recipient: None,
        finders_fee_bps: None,
        finder: None,
        expires: Some(start_time.plus_seconds(MIN_EXPIRY + 1)),
    };
    let response = router.execute_contract(
        bidder.clone(),
        marketplace,
        &set_offer_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(response.is_ok());

    // Check money is transferred correctly and not royalties
    let curator_native_balances = router
        .wrap()
        .query_all_balances("curator".to_string())
        .unwrap();
    assert_eq!(
        curator_native_balances,
        coins(INITIAL_BALANCE, NATIVE_DENOM)
    );

    let creator_native_balances = router.wrap().query_all_balances(creator).unwrap();
    // only network fee and no royalties
    // 100 - 2 (fee)
    let creator_balance_after_fee = calculated_creator_balance_after_fairburn();
    assert_eq!(
        creator_native_balances,
        coins(creator_balance_after_fee.u128() + 100 - 2, NATIVE_DENOM)
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
    let response: OwnerOfResponse = router
        .wrap()
        .query_wasm_smart(collection, &query_owner_msg)
        .unwrap();
    assert_eq!(response.owner, bidder.to_string());
}

#[test]
fn try_zero_royalties() {
    let royalty_info = RoyaltyInfoResponse {
        payment_address: "royalty_receiver".to_string(),
        share: Decimal::percent(0),
    };
    let vt = minter_with_royalties(1, Some(royalty_info));
    let (mut router, owner, bidder, creator) =
        (vt.router, vt.accts.owner, vt.accts.bidder, vt.accts.creator);
    let fair_burn = setup_fair_burn(&mut router, &creator).unwrap();
    let marketplace = setup_marketplace(&mut router, fair_burn, owner).unwrap();
    let minter_addr = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);
    let token_id = 1;
    add_funds_for_incremental_fee(
        &mut router,
        &Addr::unchecked("buyer"),
        INITIAL_BALANCE,
        1u128,
    )
    .unwrap();

    let minter = Addr::unchecked("minter");
    add_funds_for_incremental_fee(&mut router, &minter, INITIAL_BALANCE, 1u128).unwrap();
    // Mint NFT for creator
    mint(&mut router, &minter, &minter_addr);
    approve(&mut router, &minter, &collection, &marketplace, token_id);

    // List on marketplace by the minter
    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        price: coin(100, NATIVE_DENOM),
        asset_recipient: None,
        reserve_for: None,
        finders_fee_bps: Some(0),
        expires: None,
    };
    let response = router.execute_contract(
        minter.clone(),
        marketplace.clone(),
        &set_ask,
        &listing_funds(LISTING_FEE).unwrap(),
    );
    assert!(response.is_ok());

    // Bidder makes bid
    let set_offer_msg = ExecuteMsg::SetOffer {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        asset_recipient: None,
        finders_fee_bps: None,
        finder: None,
        expires: Some(start_time.plus_seconds(MIN_EXPIRY + 1)),
    };
    let response = router.execute_contract(
        bidder.clone(),
        marketplace,
        &set_offer_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(response.is_ok());

    // royalty payment address should have no funds
    let royalty_receiver_balance = router
        .wrap()
        .query_all_balances("royalty_receiver".to_string())
        .unwrap();
    assert_eq!(royalty_receiver_balance, vec![]);

    let creator_native_balances = router.wrap().query_all_balances(creator).unwrap();

    // creator should only have mint fees
    assert_eq!(
        creator_native_balances,
        coins(INITIAL_BALANCE + 100_000_000 - 10_000_000, NATIVE_DENOM)
    );

    // buyer should have been deducted
    let bidder_native_balances = router.wrap().query_all_balances(bidder.clone()).unwrap();
    assert_eq!(
        bidder_native_balances,
        coins(INITIAL_BALANCE - 100, NATIVE_DENOM)
    );
    // seller should recive full amount minus fairburn (2%)
    let minter_balance = router.wrap().query_all_balances(minter.clone()).unwrap();
    assert_eq!(
        minter_balance,
        coins(INITIAL_BALANCE - 100_000_000 + 100 - 2, NATIVE_DENOM)
    );
    // Check NFT is transferred
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
fn test_custom_royalties() {
    let royalty_info = RoyaltyInfoResponse {
        payment_address: "royalty_receiver".to_string(),
        share: Decimal::percent(5),
    };
    let vt = minter_with_royalties(1, Some(royalty_info));
    let (mut router, owner, bidder, creator) =
        (vt.router, vt.accts.owner, vt.accts.bidder, vt.accts.creator);
    let fair_burn = setup_fair_burn(&mut router, &creator).unwrap();
    let marketplace = setup_marketplace(&mut router, fair_burn, owner).unwrap();
    let minter_addr = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);
    let token_id = 1;
    add_funds_for_incremental_fee(
        &mut router,
        &Addr::unchecked("buyer"),
        INITIAL_BALANCE,
        1u128,
    )
    .unwrap();

    let minter = Addr::unchecked("minter");
    add_funds_for_incremental_fee(&mut router, &minter, INITIAL_BALANCE, 1u128).unwrap();
    // Mint NFT for creator
    mint(&mut router, &minter, &minter_addr);
    approve(&mut router, &minter, &collection, &marketplace, token_id);

    // List on marketplace by the minter
    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        price: coin(100, NATIVE_DENOM),
        asset_recipient: None,
        reserve_for: None,
        finders_fee_bps: Some(0),
        expires: None,
    };
    let response = router.execute_contract(
        minter.clone(),
        marketplace.clone(),
        &set_ask,
        &listing_funds(LISTING_FEE).unwrap(),
    );
    assert!(response.is_ok());

    // Bidder makes bid
    let set_offer_msg = ExecuteMsg::SetOffer {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        asset_recipient: None,
        finders_fee_bps: None,
        finder: None,
        expires: Some(start_time.plus_seconds(MIN_EXPIRY + 1)),
    };
    let response = router.execute_contract(
        bidder.clone(),
        marketplace,
        &set_offer_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(response.is_ok());

    // royalty receiver should have 5%
    let royalty_receiver_balance = router
        .wrap()
        .query_all_balances("royalty_receiver".to_string())
        .unwrap();
    assert_eq!(royalty_receiver_balance, coins(5, NATIVE_DENOM));

    let creator_native_balances = router.wrap().query_all_balances(creator).unwrap();

    // creator should only have mint fees
    assert_eq!(
        creator_native_balances,
        coins(INITIAL_BALANCE + 100_000_000 - 10_000_000, NATIVE_DENOM)
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
    let response: OwnerOfResponse = router
        .wrap()
        .query_wasm_smart(collection, &query_owner_msg)
        .unwrap();
    assert_eq!(response.owner, bidder.to_string());
}
