use cosmwasm_std::{coin, coins, Decimal, Uint128};
use cosmwasm_std::{Addr, Timestamp};
use cw721::{Cw721QueryMsg, OwnerOfResponse};
use cw_multi_test::Executor;
use sg_marketplace_common::coin::bps_to_decimal;
use sg_marketplace_common::MarketplaceCommonError;
use sg_std::GENESIS_MINT_START_TIME;
use sg_std::NATIVE_DENOM;
use std::collections::HashSet;
use test_suite::common_setup::setup_accounts_and_block::setup_block_time;

use crate::helpers::ExpiryRangeError;
use crate::{
    helpers::ExpiryRange,
    msg::{ExecuteMsg, QueryMsg, SudoMsg},
    state::SudoParams,
    testing::{
        helpers::{
            funds::{add_funds_for_incremental_fee, listing_funds, MINT_FEE_FAIR_BURN},
            nft_functions::{approve, get_next_token_id_and_map, mint},
        },
        setup::{
            setup_accounts::{
                setup_second_bidder_account, CREATION_FEE, INITIAL_BALANCE, MINT_PRICE,
            },
            setup_marketplace::{setup_fair_burn, setup_marketplace, LISTING_FEE, MIN_EXPIRY},
            templates::{minter_two_collections_with_time, standard_minter_template},
        },
    },
};

#[test]
fn try_sudo_update_params() {
    let vt = standard_minter_template(1);
    let (mut router, owner) = (vt.router, vt.accts.owner);
    let fair_burn = setup_fair_burn(&mut router, &owner).unwrap();
    let marketplace = setup_marketplace(&mut router, owner, fair_burn).unwrap();
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);

    // Invalid expiry range (min > max) throws error
    let update_params_msg = SudoMsg::UpdateParams {
        fair_burn: None,
        listing_fee: Some(coin(LISTING_FEE, NATIVE_DENOM)),
        ask_expiry: Some(ExpiryRange::new(100, 2)),
        offer_expiry: None,
        operators: Some(vec!["operator1".to_string()]),
        max_asks_removed_per_block: None,
        max_offers_removed_per_block: None,
        max_collection_offers_removed_per_block: None,
        trading_fee_bps: Some(5),
        max_finders_fee_bps: None,
        removal_reward_bps: None,
    };
    let response = router.wasm_sudo(marketplace.clone(), &update_params_msg);
    assert_eq!(
        response.unwrap_err().to_string(),
        ExpiryRangeError::InvalidExpirationRange("range min > max".to_string()).to_string()
    );

    // Invalid operators list is deduped
    let update_params_msg = SudoMsg::UpdateParams {
        fair_burn: None,
        listing_fee: None,
        ask_expiry: None,
        offer_expiry: None,
        operators: Some(vec![
            "operator3".to_string(),
            "operator1".to_string(),
            "operator2".to_string(),
            "operator1".to_string(),
            "operator4".to_string(),
        ]),
        max_asks_removed_per_block: None,
        max_offers_removed_per_block: None,
        max_collection_offers_removed_per_block: None,
        trading_fee_bps: None,
        max_finders_fee_bps: None,
        removal_reward_bps: None,
    };
    let response = router.wasm_sudo(marketplace.clone(), &update_params_msg);
    assert!(response.is_ok());

    let query_params_msg = QueryMsg::SudoParams {};
    let sudo_params: SudoParams = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_params_msg)
        .unwrap();
    assert_eq!(
        sudo_params.operators,
        vec![
            Addr::unchecked("operator1".to_string()),
            Addr::unchecked("operator2".to_string()),
            Addr::unchecked("operator3".to_string()),
            Addr::unchecked("operator4".to_string())
        ]
    );

    // Validate sudo params can be updated
    let update_params_msg = SudoMsg::UpdateParams {
        fair_burn: Some("fair-burn".to_string()),
        listing_fee: Some(coin(LISTING_FEE + 1, NATIVE_DENOM)),
        ask_expiry: Some(ExpiryRange::new(1, 2)),
        offer_expiry: Some(ExpiryRange::new(3, 4)),
        operators: Some(vec!["operator1".to_string()]),
        max_asks_removed_per_block: Some(10),
        max_offers_removed_per_block: Some(20),
        max_collection_offers_removed_per_block: Some(30),
        trading_fee_bps: Some(40),
        max_finders_fee_bps: Some(50),
        removal_reward_bps: Some(60),
    };
    let response = router.wasm_sudo(marketplace.clone(), &update_params_msg);
    assert!(response.is_ok());

    let sudo_params: SudoParams = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_params_msg)
        .unwrap();
    assert_eq!(sudo_params.fair_burn, Addr::unchecked("fair-burn"));
    assert_eq!(sudo_params.listing_fee, coin(LISTING_FEE + 1, NATIVE_DENOM));
    assert_eq!(sudo_params.ask_expiry, ExpiryRange::new(1, 2));
    assert_eq!(sudo_params.offer_expiry, ExpiryRange::new(3, 4));
    assert_eq!(sudo_params.operators, vec!["operator1".to_string()]);
    assert_eq!(sudo_params.max_asks_removed_per_block, 10);
    assert_eq!(sudo_params.max_offers_removed_per_block, 20);
    assert_eq!(sudo_params.max_collection_offers_removed_per_block, 30);
    assert_eq!(sudo_params.trading_fee_percent, bps_to_decimal(40));
    assert_eq!(sudo_params.max_finders_fee_percent, bps_to_decimal(50));
    assert_eq!(sudo_params.removal_reward_percent, bps_to_decimal(60));
}

#[test]
fn try_start_trading_time() {
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    let vt = minter_two_collections_with_time(2, start_time, start_time.plus_seconds(1));
    let (mut router, owner, bidder, creator) =
        (vt.router, vt.accts.owner, vt.accts.bidder, vt.accts.creator);
    let fair_burn = setup_fair_burn(&mut router, &owner).unwrap();
    let marketplace = setup_marketplace(&mut router, fair_burn.clone(), owner).unwrap();
    let minter_addr = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();

    let minter_2 = vt.collection_response_vec[1].minter.clone().unwrap();
    let collection_2 = vt.collection_response_vec[1].collection.clone().unwrap();
    setup_block_time(&mut router, start_time.nanos(), None);
    add_funds_for_incremental_fee(&mut router, &creator, CREATION_FEE, 1u128).unwrap();

    // Mint NFT for creator
    mint(&mut router, &creator, &minter_addr);

    // // after transfer, needs another approval
    let nft_hash_minter_1: HashSet<String> = HashSet::from([]);
    let (_, minter_1_token_id_0) =
        get_next_token_id_and_map(&mut router, &nft_hash_minter_1, collection.clone());
    approve(
        &mut router,
        &creator,
        &collection,
        &marketplace,
        minter_1_token_id_0,
    );
    // // Mint NFT for creator
    mint(&mut router, &creator, &minter_2);
    // // after transfer, needs another approval
    let nft_hash_minter_2: HashSet<String> = HashSet::from([]);
    let (_, minter_2_token_id_0) =
        get_next_token_id_and_map(&mut router, &nft_hash_minter_2, collection_2.clone());
    approve(
        &mut router,
        &creator,
        &collection_2,
        &marketplace,
        minter_2_token_id_0,
    );

    let creator_native_balances = router.wrap().query_all_balances(creator.clone()).unwrap();

    // Bidder makes bid on NFT with no ask
    let set_offer_msg = ExecuteMsg::SetOffer {
        collection: collection.to_string(),
        token_id: minter_1_token_id_0.to_string(),
        asset_recipient: None,
        finder: None,
        finders_fee_bps: None,
        expires: Some(start_time.plus_seconds(MIN_EXPIRY + 1)),
    };
    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_offer_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(response.is_ok());

    // Bidder makes bid on NFT with no ask to collection 2
    let set_offer_msg = ExecuteMsg::SetOffer {
        collection: collection_2.to_string(),
        token_id: minter_2_token_id_0.to_string(),
        asset_recipient: None,
        finder: None,
        finders_fee_bps: None,
        expires: Some(start_time.plus_seconds(MIN_EXPIRY + 1)),
    };
    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_offer_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(response.is_ok());

    // A collection offer is made by the bidder
    let bidder2 = setup_second_bidder_account(&mut router).unwrap();
    let set_collection_offer = ExecuteMsg::SetCollectionOffer {
        collection: collection_2.to_string(),
        asset_recipient: None,
        finders_fee_bps: None,
        expires: Some(start_time.plus_seconds(MIN_EXPIRY + 10)),
    };
    let response = router.execute_contract(
        bidder2.clone(),
        marketplace.clone(),
        &set_collection_offer,
        &coins(100, NATIVE_DENOM),
    );
    assert!(response.is_ok());

    // An asking price is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: minter_1_token_id_0.to_string(),
        price: coin(110, NATIVE_DENOM),
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

    // An asking price is made by the creator to collection 2 (should fail)
    let set_ask = ExecuteMsg::SetAsk {
        collection: collection_2.to_string(),
        token_id: minter_2_token_id_0.to_string(),
        price: coin(110, NATIVE_DENOM),
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
    assert_eq!(
        response.unwrap_err().source().unwrap().to_string(),
        MarketplaceCommonError::CollectionNotTradable {}.to_string()
    );

    // Check creator hasn't been paid yet
    let (uint128_two, fair_burn_percent) = (
        Uint128::from(2u32),
        Decimal::percent(MINT_FEE_FAIR_BURN / 100),
    );
    let mint_price = Uint128::from(MINT_PRICE);
    let creator_balance_minus_two_fees =
        Uint128::from(INITIAL_BALANCE) - (mint_price * uint128_two * fair_burn_percent);
    assert_eq!(
        creator_native_balances[0],
        coin((creator_balance_minus_two_fees).u128(), NATIVE_DENOM)
    );

    // Creator accepts offer
    let accept_offer_msg = ExecuteMsg::AcceptOffer {
        collection: collection.to_string(),
        token_id: minter_1_token_id_0.to_string(),
        bidder: bidder.to_string(),
        asset_recipient: None,
        finder: None,
    };
    let response =
        router.execute_contract(creator.clone(), marketplace.clone(), &accept_offer_msg, &[]);
    assert!(response.is_ok());

    // Check money is transferred
    let creator_native_balances = router.wrap().query_all_balances(creator.clone()).unwrap();
    // 100  - 2 (fee)
    assert_eq!(
        creator_native_balances,
        coins(
            creator_balance_minus_two_fees.u128() + 100 - 2,
            NATIVE_DENOM
        )
    );
    let bidder_native_balances = router.wrap().query_all_balances(bidder.clone()).unwrap();
    assert_eq!(
        bidder_native_balances,
        coins(INITIAL_BALANCE - 200, NATIVE_DENOM)
    );

    // Check NFT is transferred
    let query_owner_msg = Cw721QueryMsg::OwnerOf {
        token_id: minter_1_token_id_0.to_string(),
        include_expired: None,
    };
    let response: OwnerOfResponse = router
        .wrap()
        .query_wasm_smart(collection, &query_owner_msg)
        .unwrap();
    assert_eq!(response.owner, bidder.to_string());

    // Creator tries to accept accept bid on collection 2 (should fail)
    let accept_bid_msg = ExecuteMsg::AcceptCollectionOffer {
        collection: collection_2.to_string(),
        token_id: minter_2_token_id_0.to_string(),
        bidder: bidder2.to_string(),
        asset_recipient: None,
        finder: None,
    };
    let response =
        router.execute_contract(creator.clone(), marketplace.clone(), &accept_bid_msg, &[]);
    assert!(response.is_err());
    assert_eq!(
        response.unwrap_err().source().unwrap().to_string(),
        MarketplaceCommonError::CollectionNotTradable {}.to_string()
    );

    // move time to start trading time
    setup_block_time(&mut router, start_time.plus_seconds(1).nanos(), None);

    // Creator tries to accept accept offer on collection 2  should work now
    let accept_offer_msg = ExecuteMsg::AcceptOffer {
        collection: collection_2.to_string(),
        token_id: minter_2_token_id_0.to_string(),
        bidder: bidder.to_string(),
        asset_recipient: None,
        finder: None,
    };
    let response =
        router.execute_contract(creator.clone(), marketplace.clone(), &accept_offer_msg, &[]);
    assert!(response.is_ok());

    // Check money is transferred
    let creator_native_balances = router.wrap().query_all_balances(creator).unwrap();
    // 200  - 4  sold 2 items
    assert_eq!(
        creator_native_balances,
        coins(
            creator_balance_minus_two_fees.u128() + 200 - 4,
            NATIVE_DENOM
        )
    );

    // bidder approves marketplace to transfer NFT
    approve(
        &mut router,
        &bidder,
        &collection_2,
        &marketplace,
        minter_2_token_id_0,
    );

    // An asking price is made by the bidder to collection
    let set_ask = ExecuteMsg::SetAsk {
        collection: collection_2.to_string(),
        token_id: minter_2_token_id_0.to_string(),
        price: coin(110, NATIVE_DENOM),
        asset_recipient: None,
        reserve_for: None,
        finders_fee_bps: Some(0),
        expires: None,
    };
    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_ask,
        &listing_funds(LISTING_FEE).unwrap(),
    );
    assert!(response.is_ok());

    // A collection offer is accepted
    let accept_collection_offer = ExecuteMsg::AcceptCollectionOffer {
        collection: collection_2.to_string(),
        token_id: minter_2_token_id_0.to_string(),
        bidder: bidder2.to_string(),
        asset_recipient: None,
        finder: None,
    };
    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &accept_collection_offer,
        &[],
    );
    assert!(response.is_ok());

    approve(
        &mut router,
        &bidder2,
        &collection_2,
        &marketplace,
        minter_2_token_id_0,
    );

    // An asking price is made by the bidder to collection
    let set_ask = ExecuteMsg::SetAsk {
        collection: collection_2.to_string(),
        token_id: minter_2_token_id_0.to_string(),
        price: coin(110, NATIVE_DENOM),
        asset_recipient: None,
        reserve_for: None,
        finders_fee_bps: Some(0),
        expires: None,
    };
    let response = router.execute_contract(
        bidder2.clone(),
        marketplace.clone(),
        &set_ask,
        &listing_funds(LISTING_FEE).unwrap(),
    );
    assert!(response.is_ok());

    // Bidder buys now
    let set_offer_msg = ExecuteMsg::SetOffer {
        collection: collection_2.to_string(),
        token_id: minter_2_token_id_0.to_string(),
        asset_recipient: None,
        finder: None,
        finders_fee_bps: None,
        expires: Some(start_time.plus_seconds(MIN_EXPIRY + 2)),
    };
    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_offer_msg,
        &coins(110, NATIVE_DENOM),
    );
    assert!(response.is_ok());

    // Check NFT is transferred
    let query_owner_msg = Cw721QueryMsg::OwnerOf {
        token_id: minter_2_token_id_0.to_string(),
        include_expired: None,
    };
    let response: OwnerOfResponse = router
        .wrap()
        .query_wasm_smart(collection_2, &query_owner_msg)
        .unwrap();
    assert_eq!(response.owner, bidder.to_string());
}
