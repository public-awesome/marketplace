use std::collections::HashSet;

use cosmwasm_std::{coin, coins, Decimal, Timestamp, Uint128};
use cw721::{Cw721QueryMsg, OwnerOfResponse};
use cw_multi_test::Executor;
use sg_marketplace_common::MarketplaceCommonError;
use sg_std::{GENESIS_MINT_START_TIME, NATIVE_DENOM};
use test_suite::common_setup::setup_accounts_and_block::setup_block_time;

use crate::{
    msg::{ExecuteMsg, QueryMsg},
    state::{Ask, Offer},
    testing::{
        helpers::{
            funds::{
                add_funds_for_incremental_fee, calculated_creator_balance_after_fairburn,
                listing_funds, MINT_FEE_FAIR_BURN,
            },
            nft_functions::{approve, get_next_token_id_and_map, mint, MINT_PRICE},
        },
        setup::{
            setup_accounts::{setup_second_bidder_account, CREATION_FEE, INITIAL_BALANCE},
            setup_marketplace::{setup_fair_burn, setup_marketplace, LISTING_FEE, MIN_EXPIRY},
            templates::{minter_two_collections_with_time, standard_minter_template},
        },
    },
    ContractError,
};

#[test]
fn try_set_bid_fixed_price() {
    let vt = standard_minter_template(1);
    let (mut router, creator, bidder) = (vt.router, vt.accts.creator, vt.accts.bidder);
    let fair_burn = setup_fair_burn(&mut router, &creator).unwrap();
    let marketplace = setup_marketplace(&mut router, fair_burn, creator.clone()).unwrap();
    let minter = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let token_id = 1;
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);

    mint(&mut router, &creator, &minter);
    approve(&mut router, &creator, &collection, &marketplace, token_id);

    // An asking price is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        price: coin(150, NATIVE_DENOM),
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

    // Bidder makes offer
    let set_offer_msg = ExecuteMsg::SetOffer {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        asset_recipient: None,
        finders_fee_bps: None,
        finder: None,
        expires: Some(router.block_info().time.plus_seconds(MIN_EXPIRY + 1)),
    };

    // Bidder makes offer lower than the asking price
    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_offer_msg,
        &coins(50, NATIVE_DENOM),
    );
    assert!(response.is_ok());

    // ask should be returned
    let ask_query = QueryMsg::Ask {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
    };
    let ask: Option<Ask> = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &ask_query)
        .unwrap();
    assert_ne!(ask, None);

    // bid should be returned
    let offer_query = QueryMsg::Offer {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        bidder: bidder.to_string(),
    };
    let offer: Option<Offer> = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &offer_query)
        .unwrap();
    assert_ne!(offer, None);
    assert_eq!(offer.unwrap().price.amount, Uint128::from(50u128));

    let bidder2 = setup_second_bidder_account(&mut router).unwrap();

    // Bidder 2 makes a matching offer
    let set_offer_msg = ExecuteMsg::SetOffer {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        asset_recipient: None,
        finders_fee_bps: None,
        finder: None,
        expires: Some(start_time.plus_seconds(MIN_EXPIRY + 1)),
    };

    let response = router.execute_contract(
        bidder2.clone(),
        marketplace.clone(),
        &set_offer_msg,
        &coins(150, NATIVE_DENOM),
    );
    assert!(response.is_ok());

    // ask should have been removed
    let ask: Option<Ask> = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &ask_query)
        .unwrap();
    assert_eq!(ask, None);

    // offer should be returned for bidder 1
    let offer: Option<Offer> = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &offer_query)
        .unwrap();
    assert_ne!(offer, None);
    assert_eq!(offer.unwrap().price.amount, Uint128::from(50u128));

    // offer should not be returned for bidder 2
    let offer_query = QueryMsg::Offer {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        bidder: bidder2.to_string(),
    };
    let offer: Option<Offer> = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &offer_query)
        .unwrap();
    assert_eq!(offer, None);

    // Check creator has been paid
    let creator_native_balances = router.wrap().query_all_balances(creator.clone()).unwrap();
    let creator_balance_after_fee = calculated_creator_balance_after_fairburn();
    assert_eq!(
        creator_native_balances,
        coins(creator_balance_after_fee.u128() + 150 - 3, NATIVE_DENOM)
    );

    // Check contract has first bid balance
    let contract_balances = router.wrap().query_all_balances(marketplace).unwrap();
    assert_eq!(contract_balances, coins(50, NATIVE_DENOM));
}

#[test]
fn try_buy_now() {
    let vt = standard_minter_template(1);
    let (mut router, creator, bidder) = (vt.router, vt.accts.creator, vt.accts.bidder);
    let fair_burn = setup_fair_burn(&mut router, &creator).unwrap();
    let marketplace = setup_marketplace(&mut router, fair_burn, creator.clone()).unwrap();
    let minter = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let token_id = 1;
    let _start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);

    mint(&mut router, &creator, &minter);
    approve(&mut router, &creator, &collection, &marketplace, token_id);

    // An asking price is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        price: coin(150, NATIVE_DENOM),
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

    // bidder buys now
    let buy_now_msg = ExecuteMsg::BuyNow {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        asset_recipient: None,
        finder: None,
    };

    let res = router.execute_contract(
        bidder,
        marketplace.clone(),
        &buy_now_msg,
        &coins(150, NATIVE_DENOM),
    );
    assert!(res.is_ok());
    // ask should have been removed
    let ask_query = QueryMsg::Ask {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
    };
    let ask: Option<Ask> = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &ask_query)
        .unwrap();
    assert_eq!(ask, None);

    // Bidder 2 also buys now
    let bidder2 = setup_second_bidder_account(&mut router).unwrap();
    let buy_now_msg = ExecuteMsg::BuyNow {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        asset_recipient: None,
        finder: None,
    };
    let response = router.execute_contract(
        bidder2,
        marketplace.clone(),
        &buy_now_msg,
        &coins(150, NATIVE_DENOM),
    );
    let err = response.unwrap_err();
    assert_eq!(
        err.source().unwrap().to_string(),
        ContractError::ItemNotForSale {}.to_string()
    );

    // Check creator has been paid
    let creator_native_balances = router.wrap().query_all_balances(creator.clone()).unwrap();
    let creator_balance_after_fee = calculated_creator_balance_after_fairburn();
    assert_eq!(
        creator_native_balances,
        coins(creator_balance_after_fee.u128() + 150 - 3, NATIVE_DENOM)
    );

    // Check contract has first bid balance
    let contract_balances = router.wrap().query_all_balances(marketplace).unwrap();
    assert_eq!(contract_balances, vec![]);
}

#[test]
fn try_start_trading_time() {
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    let vt = minter_two_collections_with_time(2, start_time, start_time.plus_seconds(1));
    let (mut router, owner, bidder, creator) =
        (vt.router, vt.accts.owner, vt.accts.bidder, vt.accts.creator);
    let fair_burn = setup_fair_burn(&mut router, &owner).unwrap();
    let marketplace = setup_marketplace(&mut router, fair_burn, owner).unwrap();
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
