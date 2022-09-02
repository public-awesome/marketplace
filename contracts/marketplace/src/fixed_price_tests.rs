use crate::msg::{AskResponse, BidResponse, ExecuteMsg, QueryMsg};
use crate::multitest::{
    approve, custom_mock_app, listing_funds, mint, setup_accounts, setup_contracts,
    setup_second_bidder_account,
};
use crate::multitest::{LISTING_FEE, MIN_EXPIRY, TOKEN_ID};
use crate::state::SaleType;
use cosmwasm_std::{coin, coins, Uint128};
use cw_multi_test::Executor;
use sg_std::NATIVE_DENOM;

#[test]
fn try_set_bid_fixed_price() {
    let mut router = custom_mock_app();
    // Setup intial accounts
    let (_owner, bidder, creator) = setup_accounts(&mut router).unwrap();
    // Instantiate and configure contracts
    let (marketplace, collection) = setup_contracts(&mut router, &creator).unwrap();

    // Mint NFT for creator
    mint(&mut router, &creator, &collection, TOKEN_ID);
    approve(&mut router, &creator, &collection, &marketplace, TOKEN_ID);

    // An asking price is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        price: coin(150, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
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
        token_id: TOKEN_ID,
        finders_fee_bps: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };

    // Bidder makes bid higher than the asking price
    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_bid_msg,
        &coins(200, NATIVE_DENOM),
    );
    assert!(res.is_err());

    // Bidder makes bid lower than the asking price
    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_bid_msg,
        &coins(50, NATIVE_DENOM),
    );
    assert!(res.is_ok());
    let ask_query = QueryMsg::Ask {
        collection: collection.to_string(),
        token_id: TOKEN_ID,
    };

    // ask should be returned
    let res: AskResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &ask_query)
        .unwrap();
    assert_ne!(res.ask, None);

    let bid_query = QueryMsg::Bid {
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        bidder: bidder.to_string(),
    };

    // bid should be returned
    let res: BidResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &bid_query)
        .unwrap();
    assert_ne!(res.bid, None);
    let bid = res.bid.unwrap();
    assert_eq!(bid.price, Uint128::from(50u128));

    let bidder2 = setup_second_bidder_account(&mut router).unwrap();

    // Bidder 2 makes a matching bid
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        finders_fee_bps: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };

    let res = router.execute_contract(
        bidder2.clone(),
        marketplace.clone(),
        &set_bid_msg,
        &coins(150, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    // ask should have been removed
    let res: AskResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &ask_query)
        .unwrap();
    assert_eq!(res.ask, None);

    // bid should be returned for bidder 1
    let res: BidResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &bid_query)
        .unwrap();
    assert_ne!(res.bid, None);
    let bid = res.bid.unwrap();
    assert_eq!(bid.price, Uint128::from(50u128));

    let bid_query = QueryMsg::Bid {
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        bidder: bidder2.to_string(),
    };

    // bid should not be returned for bidder 2
    let res: BidResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &bid_query)
        .unwrap();
    assert_eq!(res.bid, None);

    // Check creator has been paid
    let creator_native_balances = router.wrap().query_all_balances(creator.clone()).unwrap();
    assert_eq!(creator_native_balances, coins(150 - 3, NATIVE_DENOM));

    // Check contract has first bid balance
    let contract_balances = router.wrap().query_all_balances(marketplace).unwrap();
    assert_eq!(contract_balances, coins(50, NATIVE_DENOM));
}
