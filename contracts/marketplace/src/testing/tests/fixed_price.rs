use cosmwasm_std::{coin, coins, Timestamp, Uint128};
use cw_multi_test::Executor;
use sg_std::{GENESIS_MINT_START_TIME, NATIVE_DENOM};
use test_suite::common_setup::setup_accounts_and_block::setup_block_time;

use crate::{
    msg::{ExecuteMsg, QueryMsg},
    state::{Ask, Offer},
    testing::{
        helpers::{
            funds::{calculated_creator_balance_after_fairburn, listing_funds},
            nft_functions::{approve, mint},
        },
        setup::{
            setup_accounts::setup_second_bidder_account,
            setup_marketplace::{setup_fair_burn, setup_marketplace, LISTING_FEE, MIN_EXPIRY},
            templates::standard_minter_template,
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
