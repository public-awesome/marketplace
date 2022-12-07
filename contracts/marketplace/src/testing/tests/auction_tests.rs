use crate::msg::{AskResponse, BidResponse, ExecuteMsg, QueryMsg};
use crate::state::SaleType;
use crate::testing::helpers::funds::calculated_creator_balance_after_fairburn;
use crate::testing::helpers::nft_functions::{approve, mint};
use crate::testing::setup::constants::{LISTING_FEE, MIN_EXPIRY};
use crate::testing::setup::mock_collection_params::mock_collection_params_1;
use crate::testing::setup::msg::SetupContractsParams;
use crate::testing::setup::setup_accounts::setup_second_bidder_account;
use crate::testing::setup::setup_accounts_and_block::{setup_accounts, setup_block_time};
use crate::testing::setup::setup_contracts::custom_mock_app;
use crate::testing::setup::setup_marketplace::setup_marketplace_and_collections;
use crate::testing::tests::multitest::listing_funds;
use cosmwasm_std::{coin, coins, Timestamp, Uint128};
use cw_multi_test::Executor;
use sg_std::{GENESIS_MINT_START_TIME, NATIVE_DENOM};

#[test]
fn set_auction_bids() {
    let mut router = custom_mock_app();
    let (_, bidder, creator) = setup_accounts(&mut router).unwrap();
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    let collection_params_1 = mock_collection_params_1(Some(start_time));
    let setup_params = SetupContractsParams {
        minter_admin: creator.clone(),
        collection_params_vec: vec![collection_params_1],
        num_tokens: 1,
        router: &mut router,
    };
    let (marketplace, minter_collections) =
        setup_marketplace_and_collections(setup_params).unwrap();
    let minter = minter_collections[0].minter.clone();
    let collection = minter_collections[0].collection.clone();
    let token_id = 1;
    setup_block_time(&mut router, start_time.seconds());
    mint(&mut router, &creator, &minter);
    approve(&mut router, &creator, &collection, &marketplace, token_id);

    // An asking price is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::Auction,
        collection: collection.to_string(),
        token_id,
        price: coin(150, NATIVE_DENOM),
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
        sale_type: SaleType::Auction,
        collection: collection.to_string(),
        token_id,
        finders_fee_bps: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };

    // Bidder makes bid lower than the asking price
    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_bid_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(res.is_err());

    // Bidder makes bid higher than the asking price
    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_bid_msg,
        &coins(200, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    let ask_query = QueryMsg::Ask {
        collection: collection.to_string(),
        token_id,
    };

    // ask should be returned
    let res: AskResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &ask_query)
        .unwrap();
    assert_ne!(res.ask, None);

    let bid_query = QueryMsg::Bid {
        collection: collection.to_string(),
        token_id,
        bidder: bidder.to_string(),
    };

    // bid should be returned
    let res: BidResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &bid_query)
        .unwrap();
    assert_ne!(res.bid, None);
    let bid = res.bid.unwrap();
    assert_eq!(bid.price, Uint128::from(200u128));

    let bidder2 = setup_second_bidder_account(&mut router).unwrap();

    // Bidder 2 makes bid equal to the asking price
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id,
        finders_fee_bps: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };

    let res = router.execute_contract(
        bidder2.clone(),
        marketplace.clone(),
        &set_bid_msg,
        &coins(150, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    // bid should be returned for bidder 1
    let res: BidResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &bid_query)
        .unwrap();
    assert_ne!(res.bid, None);
    let bid = res.bid.unwrap();
    assert_eq!(bid.price, Uint128::from(200u128));

    let bid_query = QueryMsg::Bid {
        collection: collection.to_string(),
        token_id,
        bidder: bidder2.to_string(),
    };

    // bid should  be returned for bidder 2
    let res: BidResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &bid_query)
        .unwrap();
    assert_ne!(res.bid, None);
    let bid = res.bid.unwrap();
    assert_eq!(bid.price, Uint128::from(150u128));

    // Creator accepts bid
    let accept_bid_msg = ExecuteMsg::AcceptBid {
        collection: collection.to_string(),
        token_id,
        bidder: bidder.to_string(),
        finder: None,
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &accept_bid_msg, &[]);
    assert!(res.is_ok());
    // ask should have been removed
    let res: AskResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &ask_query)
        .unwrap();
    assert_eq!(res.ask, None);

    // Check creator has been paid
    let creator_native_balances = router.wrap().query_all_balances(creator.clone()).unwrap();
    let creator_balance_after_fee = calculated_creator_balance_after_fairburn();
    assert_eq!(
        creator_native_balances,
        coins(creator_balance_after_fee.u128() + 200 - 4, NATIVE_DENOM)
    );

    // Check contract has second bid balance
    let contract_balances = router.wrap().query_all_balances(marketplace).unwrap();
    assert_eq!(contract_balances, coins(150, NATIVE_DENOM));
}
