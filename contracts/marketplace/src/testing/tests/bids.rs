use cosmwasm_std::{Addr, Timestamp};
use cw721::{Cw721QueryMsg, OwnerOfResponse};
use cw_multi_test::{BankSudo, Executor, SudoMsg as CwSudoMsg};
use sg_marketplace_common::query::QueryOptions;
use sg_std::GENESIS_MINT_START_TIME;

use cosmwasm_std::{coin, coins, Coin};
use sg_std::NATIVE_DENOM;
use std::collections::HashSet;
use test_suite::common_setup::setup_accounts_and_block::setup_block_time;

use crate::{
    msg::{ExecuteMsg, QueryMsg},
    state::{CollectionOffer, Offer},
    testing::helpers::{
        funds::{
            add_funds_for_incremental_fee, calculated_creator_balance_after_fairburn, listing_funds,
        },
        nft_functions::{approve, get_next_token_id_and_map, mint, transfer},
    },
    testing::setup::{
        setup_accounts::{setup_second_bidder_account, CREATION_FEE, INITIAL_BALANCE},
        setup_marketplace::{setup_fair_burn, setup_marketplace, LISTING_FEE, MIN_EXPIRY},
        templates::{minter_template_high_fee, minter_two_collections, standard_minter_template},
    },
};

#[test]
fn remove_bid_refund() {
    let vt = standard_minter_template(1);
    let (mut router, owner, bidder, creator) =
        (vt.router, vt.accts.owner, vt.accts.bidder, vt.accts.creator);
    let fair_burn = setup_fair_burn(&mut router, &creator).unwrap();
    let marketplace = setup_marketplace(&mut router, fair_burn, owner).unwrap();
    let minter_addr = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let _start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);
    let token_id = 1;
    // Mint NFT for creator
    mint(&mut router, &creator, &minter_addr);
    approve(&mut router, &creator, &collection, &marketplace, token_id);

    // Bidder makes offer
    let set_offer_msg = ExecuteMsg::SetOffer {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        asset_recipient: None,
        finders_fee_bps: None,
        finder: None,
        expires: None,
    };
    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_offer_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(response.is_ok());

    // Bidder sent bid money
    let bidder_native_balances = router.wrap().query_all_balances(bidder.clone()).unwrap();
    assert_eq!(
        bidder_native_balances,
        coins(INITIAL_BALANCE - 100, NATIVE_DENOM)
    );
    // Contract has been paid
    let contract_balances = router
        .wrap()
        .query_all_balances(marketplace.clone())
        .unwrap();
    assert_eq!(contract_balances, coins(100, NATIVE_DENOM));

    // Bidder removes bid
    let remove_offer_msg = ExecuteMsg::RemoveOffer {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
    };
    let res = router.execute_contract(bidder.clone(), marketplace, &remove_offer_msg, &[]);
    assert!(res.is_ok());

    // Bidder has money back
    let bidder_native_balances = router.wrap().query_all_balances(bidder).unwrap();
    assert_eq!(bidder_native_balances, coins(INITIAL_BALANCE, NATIVE_DENOM));
}

#[test]
fn new_bid_refund() {
    let vt = standard_minter_template(1);
    let (mut router, owner, bidder, creator) =
        (vt.router, vt.accts.owner, vt.accts.bidder, vt.accts.creator);
    let fair_burn = setup_fair_burn(&mut router, &creator).unwrap();
    let marketplace = setup_marketplace(&mut router, fair_burn, owner).unwrap();
    let minter_addr = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let _start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);
    let token_id = 1;
    // Mint NFT for creator
    mint(&mut router, &creator, &minter_addr);
    approve(&mut router, &creator, &collection, &marketplace, token_id);

    // Bidder makes offer
    let set_offer_msg = ExecuteMsg::SetOffer {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        asset_recipient: None,
        finders_fee_bps: None,
        finder: None,
        expires: None,
    };
    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_offer_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(response.is_ok());

    // Bidder sent bid money
    let bidder_native_balances = router.wrap().query_all_balances(bidder.clone()).unwrap();
    assert_eq!(
        bidder_native_balances,
        coins(INITIAL_BALANCE - 100, NATIVE_DENOM)
    );

    // Contract has been paid
    let contract_balances = router
        .wrap()
        .query_all_balances(marketplace.clone())
        .unwrap();
    assert_eq!(contract_balances, coins(100, NATIVE_DENOM));

    // Bidder makes higher bid
    let set_offer_msg = ExecuteMsg::SetOffer {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        asset_recipient: None,
        finders_fee_bps: None,
        finder: None,
        expires: None,
    };
    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_offer_msg,
        &coins(150, NATIVE_DENOM),
    );
    assert!(response.is_ok());

    // Bidder has money back
    let bidder_native_balances = router.wrap().query_all_balances(bidder.clone()).unwrap();
    assert_eq!(
        bidder_native_balances,
        coins(INITIAL_BALANCE - 150, NATIVE_DENOM)
    );

    // Contract has been paid
    let contract_balances = router
        .wrap()
        .query_all_balances(marketplace.clone())
        .unwrap();
    assert_eq!(contract_balances, coins(150, NATIVE_DENOM));

    // Check new bid has been saved
    let query_bid_msg = QueryMsg::Offer {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        bidder: bidder.to_string(),
    };
    let expected_offer = Offer {
        collection,
        token_id: token_id.to_string(),
        bidder,
        price: coin(150u128, NATIVE_DENOM),
        asset_recipient: None,
        finders_fee_percent: None,
        expires: None,
    };

    let offer: Option<Offer> = router
        .wrap()
        .query_wasm_smart(marketplace, &query_bid_msg)
        .unwrap();
    assert_eq!(offer, Some(expected_offer));
}

#[test]
fn auto_accept_bid() {
    let vt = standard_minter_template(1);
    let (mut router, owner, bidder, creator) =
        (vt.router, vt.accts.owner, vt.accts.bidder, vt.accts.creator);
    let fair_burn = setup_fair_burn(&mut router, &creator).unwrap();
    let marketplace = setup_marketplace(&mut router, fair_burn, owner).unwrap();
    let minter_addr = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let _start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);
    let token_id = 1;

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
    let response = router.execute_contract(
        creator.clone(),
        marketplace.clone(),
        &set_ask,
        &listing_funds(LISTING_FEE).unwrap(),
    );
    assert!(response.is_err());

    // // Creator Authorizes NFT
    approve(&mut router, &creator, &collection, &marketplace, token_id);

    // Now set_ask succeeds
    let response = router.execute_contract(
        creator.clone(),
        marketplace.clone(),
        &set_ask,
        &listing_funds(LISTING_FEE).unwrap(),
    );
    assert!(response.is_ok());

    // Bidder makes offer with a random token in the same amount as the ask
    router
        .sudo(CwSudoMsg::Bank({
            BankSudo::Mint {
                to_address: bidder.to_string(),
                amount: coins(1000, "random"),
            }
        }))
        .map_err(|err| println!("{:?}", err))
        .ok();
    let set_offer_msg = ExecuteMsg::SetOffer {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        asset_recipient: None,
        finders_fee_bps: None,
        finder: None,
        expires: None,
    };
    router
        .execute_contract(
            bidder.clone(),
            marketplace.clone(),
            &set_offer_msg,
            &coins(100, "random"),
        )
        .unwrap_err();

    // Bidder makes offer that meets the ask criteria
    let set_offer_msg = ExecuteMsg::SetOffer {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        asset_recipient: None,
        finders_fee_bps: None,
        finder: None,
        expires: None,
    };
    let res = router
        .execute_contract(
            bidder.clone(),
            marketplace,
            &set_offer_msg,
            &coins(100, NATIVE_DENOM),
        )
        .unwrap();

    // Bid is accepted, sale has been finalized
    assert_eq!(res.events[3].ty, "wasm-finalize-sale");
    let creator_native_balances = router.wrap().query_all_balances(creator).unwrap();
    let creator_balance_minus_fee = calculated_creator_balance_after_fairburn();
    // Check money is transferred
    assert_eq!(
        creator_native_balances,
        coins(creator_balance_minus_fee.u128() + 100 - 2, NATIVE_DENOM)
    );
    let bidder_native_balances = router.wrap().query_all_balances(bidder.clone()).unwrap();
    assert_eq!(
        bidder_native_balances,
        vec![
            coin(1000, "random"),
            coin(INITIAL_BALANCE - 100, NATIVE_DENOM),
        ]
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
fn reject_bid() {
    let vt = standard_minter_template(1);
    let (mut router, owner, bidder, seller) =
        (vt.router, vt.accts.owner, vt.accts.bidder, vt.accts.creator);
    let fair_burn = setup_fair_burn(&mut router, &owner).unwrap();
    let marketplace = setup_marketplace(&mut router, fair_burn, owner).unwrap();
    let minter_addr = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let _start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);
    let token_id = 1;

    mint(&mut router, &seller, &minter_addr);

    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        price: coin(200, NATIVE_DENOM),
        asset_recipient: None,
        reserve_for: None,
        finders_fee_bps: Some(0),
        expires: None,
    };

    approve(&mut router, &seller, &collection, &marketplace, token_id);

    let response = router.execute_contract(
        seller.clone(),
        marketplace.clone(),
        &set_ask,
        &listing_funds(LISTING_FEE).unwrap(),
    );
    assert!(response.is_ok());

    let bidder_bal = router.wrap().query_all_balances(bidder.clone()).unwrap();
    assert_eq!(bidder_bal, vec![coin(INITIAL_BALANCE, NATIVE_DENOM),]);

    let set_offer_msg = ExecuteMsg::SetOffer {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        asset_recipient: None,
        finders_fee_bps: None,
        finder: None,
        expires: None,
    };
    let bid_amount = 100;
    router
        .execute_contract(
            bidder.clone(),
            marketplace.clone(),
            &set_offer_msg,
            &coins(bid_amount, NATIVE_DENOM),
        )
        .unwrap();

    let bidder_bal = router.wrap().query_all_balances(bidder.clone()).unwrap();
    assert_eq!(
        bidder_bal,
        vec![coin(INITIAL_BALANCE - bid_amount, NATIVE_DENOM),]
    );

    // seller rejects bid
    let reject_bid_msg = ExecuteMsg::RejectOffer {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        bidder: bidder.to_string(),
    };
    let response = router
        .execute_contract(seller.clone(), marketplace, &reject_bid_msg, &[])
        .unwrap();
    assert_eq!(response.events[1].ty, "wasm-remove-offer");

    let bidder_bal = router.wrap().query_all_balances(bidder).unwrap();
    assert_eq!(bidder_bal, vec![coin(INITIAL_BALANCE, NATIVE_DENOM),]);
}

#[test]
fn try_query_bids() {
    let vt = standard_minter_template(3);
    let (mut router, owner, bidder, creator) =
        (vt.router, vt.accts.owner, vt.accts.bidder, vt.accts.creator);
    let fair_burn = setup_fair_burn(&mut router, &creator).unwrap();
    let marketplace = setup_marketplace(&mut router, fair_burn, owner).unwrap();
    let minter_addr = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);

    // Mint NFT for creator
    mint(&mut router, &creator, &minter_addr);
    let nft_hash: HashSet<String> = HashSet::from([]);
    let (_, token_id_0) = get_next_token_id_and_map(&mut router, &nft_hash, collection.clone());
    approve(&mut router, &creator, &collection, &marketplace, token_id_0);

    // An asking price is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_id_0.to_string(),
        price: coin(130, NATIVE_DENOM),
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

    // test before bid is made
    let query_offers_msg = QueryMsg::OffersByCollection {
        collection: collection.to_string(),
        query_options: None,
    };
    let offers: Vec<Offer> = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_offers_msg)
        .unwrap();
    assert_eq!(offers, vec![]);

    // Bidder makes offer
    let set_offer_msg = ExecuteMsg::SetOffer {
        collection: collection.to_string(),
        token_id: token_id_0.to_string(),
        asset_recipient: None,
        finders_fee_bps: None,
        finder: None,
        expires: Some(start_time.plus_seconds(MIN_EXPIRY + 10)),
    };
    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_offer_msg,
        &coins(120, NATIVE_DENOM),
    );
    assert!(response.is_ok());

    let set_offer_msg = ExecuteMsg::SetOffer {
        collection: collection.to_string(),
        token_id: (token_id_0 + 1).to_string(),
        asset_recipient: None,
        finders_fee_bps: None,
        finder: None,
        expires: Some(start_time.plus_seconds(MIN_EXPIRY + 1)),
    };
    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_offer_msg,
        &coins(115, NATIVE_DENOM),
    );
    assert!(response.is_ok());

    let offers: Vec<Offer> = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_offers_msg)
        .unwrap();
    assert_eq!(offers[0].token_id, token_id_0.to_string());
    assert_eq!(offers[0].price.amount.u128(), 120u128);
    let query_offers_msg = QueryMsg::OffersByTokenPrice {
        collection: collection.to_string(),
        token_id: (token_id_0 + 1).to_string(),
        denom: NATIVE_DENOM.to_string(),
        query_options: None,
    };
    let offers: Vec<Offer> = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_offers_msg)
        .unwrap();
    assert_eq!(offers[0].token_id, (token_id_0 + 1).to_string());
    assert_eq!(offers[0].price.amount.u128(), 115u128);

    let query_bids_msg = QueryMsg::OffersByBidder {
        bidder: bidder.to_string(),
        query_options: None,
    };
    let offers: Vec<Offer> = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_bids_msg)
        .unwrap();
    assert_eq!(offers.len(), 2);
    let query_bids_msg = QueryMsg::OffersByExpiration {
        query_options: None,
    };
    let offers: Vec<Offer> = router
        .wrap()
        .query_wasm_smart(marketplace, &query_bids_msg)
        .unwrap();
    assert_eq!(offers.len(), 2);
    assert_eq!(
        offers[0].expires.unwrap().seconds(),
        start_time.plus_seconds(MIN_EXPIRY + 1).seconds()
    );
    assert_eq!(
        offers[1].expires.unwrap().seconds(),
        start_time.plus_seconds(MIN_EXPIRY + 10).seconds()
    );
}

#[test]
fn try_remove_stale_offer() {
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

    // Bidder makes offer
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
        marketplace.clone(),
        &set_offer_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(response.is_ok());

    let operator = Addr::unchecked("operator0".to_string());
    // Try to remove the bid (not yet stale) as an operator
    let remove_msg = ExecuteMsg::RemoveStaleOffer {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        bidder: bidder.to_string(),
    };
    router
        .execute_contract(operator.clone(), marketplace.clone(), &remove_msg, &[])
        .unwrap_err();

    setup_block_time(
        &mut router,
        start_time.plus_seconds(MIN_EXPIRY + 101).nanos(),
        None,
    );
    let response = router.execute_contract(operator.clone(), marketplace.clone(), &remove_msg, &[]);
    assert!(response.is_ok());
}

#[test]
fn try_remove_stale_collection_offer() {
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

    let expiry_time = start_time.plus_seconds(MIN_EXPIRY + 1).seconds();

    // Bidder makes collection bid
    let set_col_offer_msg = ExecuteMsg::SetCollectionOffer {
        collection: collection.to_string(),
        asset_recipient: None,
        finders_fee_bps: None,
        expires: Some(Timestamp::from_seconds(expiry_time)),
    };
    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_col_offer_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(response.is_ok());

    let operator = Addr::unchecked("operator0".to_string());

    // Try to remove the collection bid (not yet stale) as an operator
    let remove_col_msg = ExecuteMsg::RemoveStaleCollectionOffer {
        collection: collection.to_string(),
        bidder: bidder.to_string(),
    };
    router
        .execute_contract(operator.clone(), marketplace.clone(), &remove_col_msg, &[])
        .unwrap_err();

    // make bid stale by adding stale_offer_duration
    let new_time = Timestamp::from_seconds(expiry_time)
        .plus_seconds(100)
        .nanos();
    setup_block_time(&mut router, new_time, None);

    let response =
        router.execute_contract(operator.clone(), marketplace.clone(), &remove_col_msg, &[]);
    assert!(response.is_ok());
}

#[test]
fn try_collection_offers() {
    let vt = minter_two_collections(1);
    let (mut router, owner, bidder, creator) =
        (vt.router, vt.accts.owner, vt.accts.bidder, vt.accts.creator);
    let fair_burn = setup_fair_burn(&mut router, &creator).unwrap();
    let marketplace = setup_marketplace(&mut router, fair_burn, owner).unwrap();
    let minter_addr = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);
    let bidder2 = setup_second_bidder_account(&mut router).unwrap();
    add_funds_for_incremental_fee(&mut router, &creator, INITIAL_BALANCE, 1u128).unwrap();

    let collection_two = vt.collection_response_vec[1].collection.clone().unwrap();
    let token_id = 1;

    setup_block_time(&mut router, start_time.nanos(), None);
    // Mint NFT for creator
    mint(&mut router, &creator, &minter_addr);
    approve(&mut router, &creator, &collection, &marketplace, token_id);

    // A collection bid is made by the bidder
    let set_collection_offer = ExecuteMsg::SetCollectionOffer {
        collection: collection.to_string(),
        asset_recipient: None,
        finders_fee_bps: None,
        expires: Some(start_time.plus_seconds(MIN_EXPIRY + 10)),
    };
    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_collection_offer,
        &coins(150, NATIVE_DENOM),
    );
    assert!(response.is_ok());

    // An invalid collection bid is attempted by the bidder
    let set_collection_offer = ExecuteMsg::SetCollectionOffer {
        collection: collection.to_string(),
        asset_recipient: None,
        finders_fee_bps: Some(10100),
        expires: Some(start_time.plus_seconds(MIN_EXPIRY + 10)),
    };
    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_collection_offer,
        &coins(151, NATIVE_DENOM),
    );
    assert!(response.is_err());

    // A collection bid is made by bidder2
    let set_collection_offer = ExecuteMsg::SetCollectionOffer {
        collection: collection.to_string(),
        asset_recipient: None,
        finders_fee_bps: None,
        expires: Some(start_time.plus_seconds(MIN_EXPIRY + 5)),
    };
    let response = router.execute_contract(
        bidder2.clone(),
        marketplace.clone(),
        &set_collection_offer,
        &coins(180, NATIVE_DENOM),
    );
    assert!(response.is_ok());

    // test querying a single collection bid
    let query_collection_bid = QueryMsg::CollectionOffer {
        collection: collection.to_string(),
        bidder: bidder.to_string(),
    };
    let collection_offer: Option<CollectionOffer> = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_collection_bid)
        .unwrap();
    assert_eq!(collection_offer.unwrap().price.amount.u128(), 150u128);

    // test querying all collection bids by bidder
    let query_collection_bids = QueryMsg::CollectionOffersByBidder {
        bidder: bidder.to_string(),
        query_options: None,
    };
    let collection_offers: Vec<CollectionOffer> = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_collection_bids)
        .unwrap();
    assert_eq!(collection_offers[0].price.amount.u128(), 150u128);

    // test querying all sorted collection bids by bidder
    let query_collection_bids_by_price = QueryMsg::CollectionOffersByPrice {
        collection: collection.to_string(),
        denom: NATIVE_DENOM.to_string(),
        query_options: None,
    };
    let collection_offers: Vec<CollectionOffer> = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_collection_bids_by_price)
        .unwrap();
    assert_eq!(collection_offers.len(), 2);
    assert_eq!(collection_offers[0].price.amount.u128(), 150u128);
    assert_eq!(collection_offers[1].price.amount.u128(), 180u128);

    // test start_after
    let query_collection_bids_by_price = QueryMsg::CollectionOffersByPrice {
        collection: collection.to_string(),
        denom: NATIVE_DENOM.to_string(),
        query_options: Some(QueryOptions {
            descending: None,
            start_after: Some((
                collection_offers[0].price.amount.u128(),
                collection_offers[0].bidder.to_string(),
            )),
            limit: Some(10),
        }),
    };
    let collection_offers: Vec<CollectionOffer> = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_collection_bids_by_price)
        .unwrap();
    assert_eq!(collection_offers.len(), 1);
    assert_eq!(collection_offers[0].price.amount.u128(), 180u128);

    let set_collection_offer = ExecuteMsg::SetCollectionOffer {
        collection: collection_two.to_string(),
        asset_recipient: None,
        finders_fee_bps: None,
        expires: Some(start_time.plus_seconds(MIN_EXPIRY + 20)),
    };
    let response = router.execute_contract(
        bidder2.clone(),
        marketplace.clone(),
        &set_collection_offer,
        &coins(180, NATIVE_DENOM),
    );
    assert!(response.is_ok());
    let query_collection_bids_by_expiration = QueryMsg::CollectionOffersByExpiration {
        query_options: None,
    };
    let collection_offers: Vec<CollectionOffer> = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_collection_bids_by_expiration)
        .unwrap();
    assert_eq!(collection_offers.len(), 3);
    assert_eq!(
        collection_offers[0].expires.unwrap().seconds(),
        start_time.plus_seconds(MIN_EXPIRY + 5).seconds()
    );
    assert_eq!(
        collection_offers[1].expires.unwrap().seconds(),
        start_time.plus_seconds(MIN_EXPIRY + 10).seconds()
    );
    assert_eq!(
        collection_offers[2].expires.unwrap().seconds(),
        start_time.plus_seconds(MIN_EXPIRY + 20).seconds()
    );

    // test querying all sorted collection bids by bidder in reverse
    let reverse_query_sorted_collection_bids = QueryMsg::CollectionOffersByPrice {
        collection: collection.to_string(),
        denom: NATIVE_DENOM.to_string(),
        query_options: Some(QueryOptions {
            descending: Some(true),
            start_after: None,
            limit: Some(10),
        }),
    };
    let collection_offers: Vec<CollectionOffer> = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &reverse_query_sorted_collection_bids)
        .unwrap();
    assert_eq!(collection_offers.len(), 2);
    assert_eq!(collection_offers[0].price.amount.u128(), 180u128);
    assert_eq!(collection_offers[1].price.amount.u128(), 150u128);

    // test start_before
    let reverse_query_sorted_collection_bids = QueryMsg::CollectionOffersByPrice {
        collection: collection.to_string(),
        denom: NATIVE_DENOM.to_string(),
        query_options: Some(QueryOptions {
            descending: Some(true),
            start_after: Some((
                collection_offers[0].price.amount.u128(),
                collection_offers[0].bidder.to_string(),
            )),
            limit: Some(10),
        }),
    };
    let collection_offers: Vec<CollectionOffer> = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &reverse_query_sorted_collection_bids)
        .unwrap();
    assert_eq!(collection_offers.len(), 1);
    assert_eq!(collection_offers[0].price.amount.u128(), 150u128);

    // test removing collection bid
    let remove_collection_bid = ExecuteMsg::RemoveCollectionOffer {
        collection: collection.to_string(),
    };
    let response =
        router.execute_contract(bidder2, marketplace.clone(), &remove_collection_bid, &[]);
    assert!(response.is_ok());
    let query_sorted_collection_bids = QueryMsg::CollectionOffersByPrice {
        collection: collection.to_string(),
        denom: NATIVE_DENOM.to_string(),
        query_options: None,
    };
    let collection_offers: Vec<CollectionOffer> = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_sorted_collection_bids)
        .unwrap();
    assert_eq!(collection_offers.len(), 1);
    assert_eq!(collection_offers[0].price.amount.u128(), 150u128);

    // A collection bid is accepted
    let accept_collection_bid = ExecuteMsg::AcceptCollectionOffer {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        bidder: bidder.to_string(),
        asset_recipient: None,
        finder: None,
    };

    let response =
        router.execute_contract(creator.clone(), marketplace, &accept_collection_bid, &[]);
    assert!(response.is_ok());
}

#[test]
fn try_set_accept_offer_high_fees() {
    let vt = minter_template_high_fee(1);
    let (mut router, owner, creator, bidder) =
        (vt.router, vt.accts.owner, vt.accts.creator, vt.accts.bidder);
    let fair_burn = setup_fair_burn(&mut router, &creator).unwrap();
    let marketplace = setup_marketplace(&mut router, fair_burn, creator.clone()).unwrap();
    let minter_addr = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let token_id = 1;
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);

    let creator_funds: Vec<Coin> = coins(CREATION_FEE, NATIVE_DENOM);

    router
        .sudo(CwSudoMsg::Bank({
            BankSudo::Mint {
                to_address: creator.to_string(),
                amount: creator_funds,
            }
        }))
        .map_err(|err| println!("{:?}", err))
        .ok();
    // Mint NFT for creator
    mint(&mut router, &creator, &minter_addr);
    approve(&mut router, &creator, &collection, &marketplace, token_id);

    // Bidder makes offer
    let set_offer_msg = ExecuteMsg::SetOffer {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        asset_recipient: None,
        finders_fee_bps: Some(10),
        finder: Some(owner.to_string()),
        expires: Some(start_time.plus_seconds(MIN_EXPIRY + 1)),
    };
    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_offer_msg,
        &coins(10000, NATIVE_DENOM),
    );
    assert!(response.is_ok());

    let accept_offer_msg = ExecuteMsg::AcceptOffer {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        bidder: bidder.to_string(),
        asset_recipient: None,
        finder: Some(owner.to_string()),
    };
    let response =
        router.execute_contract(creator.clone(), marketplace.clone(), &accept_offer_msg, &[]);
    assert!(response.is_err());
    assert!(response
        .unwrap_err()
        .source()
        .unwrap()
        .to_string()
        .contains("Overflow"));
}

#[test]
fn try_query_sorted_offers() {
    let vt = standard_minter_template(3);
    let (mut router, owner, bidder, creator) =
        (vt.router, vt.accts.owner, vt.accts.bidder, vt.accts.creator);
    let fair_burn = setup_fair_burn(&mut router, &creator).unwrap();
    let marketplace = setup_marketplace(&mut router, fair_burn, owner).unwrap();
    let minter_addr = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);

    // Mint NFT for creator
    mint(&mut router, &creator, &minter_addr);
    let nft_hash: HashSet<String> = HashSet::from([]);
    let (nft_hash, token_id_0) =
        get_next_token_id_and_map(&mut router, &nft_hash, collection.clone());
    approve(&mut router, &creator, &collection, &marketplace, token_id_0);
    mint(&mut router, &creator, &minter_addr);
    let (nft_hash, token_id_1) =
        get_next_token_id_and_map(&mut router, &nft_hash, collection.clone());
    approve(&mut router, &creator, &collection, &marketplace, token_id_1);
    mint(&mut router, &creator, &minter_addr);
    let (_, token_id_2) = get_next_token_id_and_map(&mut router, &nft_hash, collection.clone());
    approve(&mut router, &creator, &collection, &marketplace, token_id_2);

    // Bidder makes offer
    let set_offer_msg = ExecuteMsg::SetOffer {
        collection: collection.to_string(),
        token_id: token_id_0.to_string(),
        asset_recipient: None,
        finders_fee_bps: None,
        finder: None,
        expires: Some(start_time.plus_seconds(MIN_EXPIRY + 1)),
    };
    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_offer_msg,
        &coins(50, NATIVE_DENOM),
    );
    assert!(response.is_ok());

    // Bidder makes offer
    let set_offer_msg = ExecuteMsg::SetOffer {
        collection: collection.to_string(),
        token_id: token_id_1.to_string(),
        asset_recipient: None,
        finders_fee_bps: None,
        finder: None,
        expires: Some(start_time.plus_seconds(MIN_EXPIRY + 1)),
    };
    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_offer_msg,
        &coins(70, NATIVE_DENOM),
    );
    assert!(response.is_ok());

    // Bidder makes offer
    let set_offer_msg = ExecuteMsg::SetOffer {
        collection: collection.to_string(),
        token_id: token_id_2.to_string(),
        asset_recipient: None,
        finders_fee_bps: None,
        finder: None,
        expires: Some(start_time.plus_seconds(MIN_EXPIRY + 1)),
    };
    let response = router.execute_contract(
        bidder,
        marketplace.clone(),
        &set_offer_msg,
        &coins(60, NATIVE_DENOM),
    );
    assert!(response.is_ok());

    let query_bids_msg = QueryMsg::OffersByTokenPrice {
        collection: collection.to_string(),
        token_id: token_id_0.to_string(),
        denom: NATIVE_DENOM.to_string(),
        query_options: None,
    };
    let offers: Vec<Offer> = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_bids_msg)
        .unwrap();
    assert_eq!(offers.len(), 1);
    assert_eq!(offers[0].price.amount.u128(), 50u128);

    let query_bids_msg = QueryMsg::OffersByTokenPrice {
        collection: collection.to_string(),
        token_id: token_id_1.to_string(),
        denom: NATIVE_DENOM.to_string(),
        query_options: None,
    };
    let offers: Vec<Offer> = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_bids_msg)
        .unwrap();
    assert_eq!(offers.len(), 1);
    assert_eq!(offers[0].price.amount.u128(), 70u128);

    let query_bids_msg = QueryMsg::OffersByTokenPrice {
        collection: collection.to_string(),
        token_id: token_id_2.to_string(),
        denom: NATIVE_DENOM.to_string(),
        query_options: None,
    };
    let offers: Vec<Offer> = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_bids_msg)
        .unwrap();
    assert_eq!(offers.len(), 1);
    assert_eq!(offers[0].price.amount.u128(), 60u128);

    // test adding another offer to an existing ask
    let bidder2: Addr = Addr::unchecked("bidder2");
    let funds: Vec<Coin> = coins(INITIAL_BALANCE, NATIVE_DENOM);
    let response = router.sudo(CwSudoMsg::Bank({
        BankSudo::Mint {
            to_address: bidder2.to_string(),
            amount: funds,
        }
    }));
    assert!(response.is_ok());

    // Bidder2 makes offer
    let set_offer_msg = ExecuteMsg::SetOffer {
        collection: collection.to_string(),
        token_id: token_id_0.to_string(),
        asset_recipient: None,
        finders_fee_bps: None,
        finder: None,
        expires: Some(start_time.plus_seconds(MIN_EXPIRY + 1)),
    };
    let response = router.execute_contract(
        bidder2,
        marketplace.clone(),
        &set_offer_msg,
        &coins(40, NATIVE_DENOM),
    );
    assert!(response.is_ok());

    let query_bids_msg = QueryMsg::OffersByTokenPrice {
        collection: collection.to_string(),
        token_id: token_id_0.to_string(),
        denom: NATIVE_DENOM.to_string(),
        query_options: None,
    };
    let offers: Vec<Offer> = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_bids_msg)
        .unwrap();
    assert_eq!(offers.len(), 2);
    assert_eq!(offers[0].price.amount.u128(), 40u128);
    assert_eq!(offers[1].price.amount.u128(), 50u128);

    // test start_after query
    let query_start_after_offers_msg = QueryMsg::OffersByTokenPrice {
        collection: collection.to_string(),
        token_id: token_id_0.to_string(),
        denom: NATIVE_DENOM.to_string(),
        query_options: Some(QueryOptions {
            descending: None,
            start_after: Some((45u128, offers[0].bidder.to_string())),
            limit: None,
        }),
    };
    let offers: Vec<Offer> = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_start_after_offers_msg)
        .unwrap();
    assert_eq!(offers.len(), 1);
    assert_eq!(offers[0].price.amount.u128(), 50u128);

    // test reverse bids query
    let reverse_query_bids_msg = QueryMsg::OffersByTokenPrice {
        collection: collection.to_string(),
        token_id: token_id_0.to_string(),
        denom: NATIVE_DENOM.to_string(),
        query_options: Some(QueryOptions {
            descending: Some(true),
            start_after: None,
            limit: None,
        }),
    };
    let offers: Vec<Offer> = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &reverse_query_bids_msg)
        .unwrap();
    assert_eq!(offers.len(), 2);
    assert_eq!(offers[0].price.amount.u128(), 50u128);
    assert_eq!(offers[1].price.amount.u128(), 40u128);

    // test start_before reverse bids query
    let reverse_query_start_before_bids_msg = QueryMsg::OffersByTokenPrice {
        collection: collection.to_string(),
        token_id: token_id_0.to_string(),
        denom: NATIVE_DENOM.to_string(),
        query_options: Some(QueryOptions {
            descending: Some(true),
            start_after: Some((offers[0].price.amount.u128(), offers[0].bidder.to_string())),
            limit: None,
        }),
    };
    let offers: Vec<Offer> = router
        .wrap()
        .query_wasm_smart(marketplace, &reverse_query_start_before_bids_msg)
        .unwrap();
    assert_eq!(offers.len(), 1);
    assert_eq!(offers[0].price.amount.u128(), 40u128);
}

#[test]
fn try_set_accept_bid_no_ask() {
    let vt = standard_minter_template(1);
    let (mut router, creator, bidder) = (vt.router, vt.accts.creator, vt.accts.bidder);
    let fair_burn = setup_fair_burn(&mut router, &creator).unwrap();
    let marketplace = setup_marketplace(&mut router, fair_burn, creator.clone()).unwrap();
    let minter_addr = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let token_id = 1;
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);

    // Mint NFT for creator
    mint(&mut router, &creator, &minter_addr);
    approve(&mut router, &creator, &collection, &marketplace, token_id);

    // Bidder makes offer
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
        marketplace.clone(),
        &set_offer_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(response.is_ok());

    // Check contract has been paid
    let contract_balances = router
        .wrap()
        .query_all_balances(marketplace.clone())
        .unwrap();
    assert_eq!(contract_balances, coins(100, NATIVE_DENOM));

    // Check creator hasn't been paid yet
    let final_balance = calculated_creator_balance_after_fairburn();
    let creator_native_balances = router.wrap().query_all_balances(creator.clone()).unwrap();
    assert_eq!(
        creator_native_balances,
        coins(final_balance.u128(), NATIVE_DENOM)
    );

    // Creator accepts bid
    let accept_bid_msg = ExecuteMsg::AcceptOffer {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        bidder: bidder.to_string(),
        asset_recipient: None,
        finder: None,
    };
    let response =
        router.execute_contract(creator.clone(), marketplace.clone(), &accept_bid_msg, &[]);
    assert!(response.is_ok());

    // Check money is transferred
    let creator_native_balances = router.wrap().query_all_balances(creator).unwrap();
    // 100  - 2 (fee)
    assert_eq!(
        creator_native_balances,
        coins(final_balance.u128() + 100 - 2, NATIVE_DENOM)
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

    // Check contract has zero balance
    let contract_balances = router.wrap().query_all_balances(marketplace).unwrap();
    assert_eq!(contract_balances, []);
}

#[test]
fn try_set_accept_offer() {
    let vt = standard_minter_template(1);
    let (mut router, _owner, creator, bidder) =
        (vt.router, vt.accts.owner, vt.accts.creator, vt.accts.bidder);
    let fair_burn = setup_fair_burn(&mut router, &creator).unwrap();
    let marketplace = setup_marketplace(&mut router, fair_burn, creator.clone()).unwrap();
    let minter_addr = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let token_id = 1;
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);

    // Mint NFT for creator
    mint(&mut router, &creator, &minter_addr);
    approve(&mut router, &creator, &collection, &marketplace, token_id);

    // Should error with expiry lower than min
    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        price: coin(110, NATIVE_DENOM),
        asset_recipient: None,
        reserve_for: None,
        finders_fee_bps: Some(0),
        expires: Some(start_time.plus_seconds(MIN_EXPIRY - 1)),
    };
    let response = router.execute_contract(
        creator.clone(),
        marketplace.clone(),
        &set_ask,
        &listing_funds(LISTING_FEE).unwrap(),
    );
    assert!(response.is_err());

    // An asking price is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
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

    // Bidder makes offer
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
        marketplace.clone(),
        &set_offer_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(response.is_ok());

    // Check contract has been paid
    let contract_balances = router
        .wrap()
        .query_all_balances(marketplace.clone())
        .unwrap();
    assert_eq!(contract_balances, coins(100, NATIVE_DENOM));

    // Check creator hasn't been paid yet
    let creator_native_balances = router.wrap().query_all_balances(creator.clone()).unwrap();

    let final_balance = calculated_creator_balance_after_fairburn();
    assert_eq!(
        creator_native_balances,
        coins(final_balance.u128(), NATIVE_DENOM)
    );

    // Creator accepts offer
    let accept_bid_msg = ExecuteMsg::AcceptOffer {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        bidder: bidder.to_string(),
        asset_recipient: None,
        finder: None,
    };
    let response =
        router.execute_contract(creator.clone(), marketplace.clone(), &accept_bid_msg, &[]);
    assert!(response.is_ok());

    // Check money is transferred
    let creator_native_balances = router.wrap().query_all_balances(creator).unwrap();
    // 100  - 2 (fee)
    assert_eq!(
        creator_native_balances,
        coins(final_balance.u128() + 100 - 2, NATIVE_DENOM)
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

    // Check contract has zero balance
    let contract_balances = router.wrap().query_all_balances(marketplace).unwrap();
    assert_eq!(contract_balances, []);
}

#[test]
fn try_bid_sale_type() {
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

    // An asking price is made by the creator
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

    // Bidder makes offer
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
        marketplace.clone(),
        &set_offer_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(response.is_ok());

    // Check creator has been paid
    let creator_balance_minus_fee = calculated_creator_balance_after_fairburn();
    let creator_native_balances = router.wrap().query_all_balances(creator.clone()).unwrap();
    assert_eq!(
        creator_native_balances,
        coins(creator_balance_minus_fee.u128() + 100 - 2, NATIVE_DENOM)
    );

    // Check contract has zero balance
    let contract_balances = router
        .wrap()
        .query_all_balances(marketplace.clone())
        .unwrap();
    assert_eq!(contract_balances, []);

    transfer(&mut router, &bidder, &creator, &collection, token_id);

    let bidder2 = setup_second_bidder_account(&mut router).unwrap();

    // Bidder makes offer on NFT with no ask
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
        &coins(100, NATIVE_DENOM),
    );

    assert!(response.is_ok());

    // Bidder makes offer with Auction
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
        &coins(100, NATIVE_DENOM),
    );

    assert!(response.is_ok());

    let query_bids_msg = QueryMsg::OffersByBidder {
        bidder: bidder2.to_string(),
        query_options: None,
    };
    let offers: Vec<Offer> = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_bids_msg)
        .unwrap();
    assert_eq!(offers.len(), 1);
    assert_eq!(offers[0].price.amount.u128(), 100u128);
}
