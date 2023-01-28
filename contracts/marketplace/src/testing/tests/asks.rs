use crate::error::ContractError;
use crate::msg::{
    AskCountResponse, AskOffset, AskResponse, AsksResponse, CollectionOffset, CollectionsResponse,
};
use crate::msg::{ExecuteMsg, QueryMsg};
use crate::state::SaleType;
use crate::testing::helpers::funds::{
    add_funds_for_incremental_fee, calculated_creator_balance_after_fairburn, listing_funds,
};
use crate::testing::helpers::nft_functions::{
    approve, burn, get_next_token_id_and_map, mint, mint_for, transfer,
};
use crate::testing::setup::setup_accounts::{
    setup_second_bidder_account, CREATION_FEE, INITIAL_BALANCE, MINT_PRICE,
};
use crate::testing::setup::setup_marketplace::{
    setup_marketplace, LISTING_FEE, MAX_EXPIRY, MIN_EXPIRY,
};
use cosmwasm_std::{Addr, Empty, Timestamp};
use cw721::{Cw721QueryMsg, OwnerOfResponse};
use cw_multi_test::Executor;
use sg_std::GENESIS_MINT_START_TIME;

use cosmwasm_std::{coin, coins};
use cw_utils::Expiration;
use sg721::ExecuteMsg as Sg721ExecuteMsg;
use std::collections::HashSet;
use test_suite::common_setup::setup_accounts_and_block::setup_block_time;

use crate::testing::setup::templates::{minter_template_owner_admin, standard_minter_template};
use sg_std::NATIVE_DENOM;

#[test]
fn try_query_sorted_asks() {
    let vt = standard_minter_template(3);
    let (mut router, creator) = (vt.router, vt.accts.creator);
    let marketplace = setup_marketplace(&mut router, creator.clone()).unwrap();
    let minter_addr = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);
    // Mint NFT for creator

    mint(&mut router, &creator, &minter_addr);
    let nft_hash: HashSet<String> = HashSet::from([]);
    let (nft_hash, token_id_0) =
        get_next_token_id_and_map(&mut router, &nft_hash, collection.clone());
    approve(
        &mut router,
        &creator,
        &collection.clone(),
        &marketplace,
        token_id_0,
    );
    // // Add funds to creator for listing fees
    add_funds_for_incremental_fee(&mut router, &creator, MINT_PRICE, 3u128).unwrap();

    mint(&mut router, &creator, &minter_addr);
    let (nft_hash, token_id_1) =
        get_next_token_id_and_map(&mut router, &nft_hash, collection.clone());
    approve(&mut router, &creator, &collection, &marketplace, token_id_1);
    mint(&mut router, &creator, &minter_addr);
    let (_, token_id_2) = get_next_token_id_and_map(&mut router, &nft_hash, collection.clone());
    approve(&mut router, &creator, &collection, &marketplace, token_id_2);
    // An asking price is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: token_id_0,
        price: coin(110, NATIVE_DENOM),
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
    // An asking price is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: token_id_1,
        price: coin(109, NATIVE_DENOM),
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
    // An asking price is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: token_id_2,
        price: coin(111, NATIVE_DENOM),
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

    let query_asks_msg = QueryMsg::AsksSortedByPrice {
        collection: collection.to_string(),
        include_inactive: Some(true),
        start_after: None,
        limit: None,
    };
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_asks_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 3);
    assert_eq!(res.asks[0].price.u128(), 109u128);
    assert_eq!(res.asks[1].price.u128(), 110u128);
    assert_eq!(res.asks[2].price.u128(), 111u128);

    let start_after = AskOffset::new(res.asks[0].price, res.asks[0].token_id);
    let query_msg = QueryMsg::AsksSortedByPrice {
        collection: collection.to_string(),
        include_inactive: Some(true),
        start_after: Some(start_after),
        limit: None,
    };

    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 2);
    assert_eq!(res.asks[0].price.u128(), 110u128);
    assert_eq!(res.asks[1].price.u128(), 111u128);

    let reverse_query_asks_msg = QueryMsg::ReverseAsksSortedByPrice {
        collection: collection.to_string(),
        include_inactive: Some(true),
        start_before: None,
        limit: None,
    };

    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &reverse_query_asks_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 3);
    assert_eq!(res.asks[0].price.u128(), 111u128);
    assert_eq!(res.asks[1].price.u128(), 110u128);
    assert_eq!(res.asks[2].price.u128(), 109u128);

    let start_before = AskOffset::new(res.asks[0].price, res.asks[0].token_id);
    let reverse_query_asks_start_before_first_desc_msg = QueryMsg::ReverseAsksSortedByPrice {
        collection: collection.to_string(),
        include_inactive: Some(true),
        start_before: Some(start_before),
        limit: None,
    };

    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(
            marketplace.clone(),
            &reverse_query_asks_start_before_first_desc_msg,
        )
        .unwrap();
    assert_eq!(res.asks.len(), 2);
    assert_eq!(res.asks[0].price.u128(), 110u128);
    assert_eq!(res.asks[1].price.u128(), 109u128);

    let res: AskCountResponse = router
        .wrap()
        .query_wasm_smart(
            marketplace.clone(),
            &QueryMsg::AskCount {
                collection: collection.to_string(),
            },
        )
        .unwrap();
    assert_eq!(res.count, 3);
}

#[test]
fn max_set_ask_amount() {
    let vt = standard_minter_template(1);
    let (mut router, owner, creator) = (vt.router, vt.accts.owner, vt.accts.creator);
    let marketplace = setup_marketplace(&mut router, owner).unwrap();
    let minter = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    setup_block_time(&mut router, start_time.nanos(), None);
    let token_id = 1;

    // Mint NFT for creator
    mint(&mut router, &creator, &minter);
    approve(&mut router, &creator, &collection, &marketplace, token_id);

    // An asking price is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id,
        price: coin(100_000_000_000_001u128, NATIVE_DENOM),
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
    assert_eq!(
        res.unwrap_err().source().unwrap().to_string(),
        "PriceTooHigh: 100000000000001".to_string()
    );

    // An asking price is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id,
        price: coin(1, NATIVE_DENOM),
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
    assert_eq!(
        res.unwrap_err().source().unwrap().to_string(),
        "PriceTooSmall: 1".to_string()
    );

    // An asking price is made by the creator at the limit of 100M
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id,
        price: coin(100_000_000_000_000u128, NATIVE_DENOM),
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
}

#[test]
fn try_ask_with_filter_inactive() {
    let vt = standard_minter_template(1);
    let (mut router, owner, creator) = (vt.router, vt.accts.owner, vt.accts.creator);
    let marketplace = setup_marketplace(&mut router, owner.clone()).unwrap();
    let minter_addr = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let token_id = 1;
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    setup_block_time(&mut router, start_time.nanos(), None);

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
        finders_fee_bps: Some(500), // 5%
    };
    let res = router.execute_contract(
        creator.clone(),
        marketplace.clone(),
        &set_ask,
        &listing_funds(LISTING_FEE).unwrap(),
    );
    assert!(res.is_ok());

    // transfer nft from creator to owner. Creates a stale ask that needs to be updated
    transfer(&mut router, &creator, &owner, &collection, token_id);

    let update_ask_state = ExecuteMsg::SyncAsk {
        collection: collection.to_string(),
        token_id,
    };
    let res = router.execute_contract(
        Addr::unchecked("operator1"),
        marketplace.clone(),
        &update_ask_state,
        &[],
    );
    assert!(res.is_ok());

    let ask_msg = QueryMsg::Asks {
        collection: collection.to_string(),
        include_inactive: None,
        start_after: None,
        limit: None,
    };
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &ask_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 0);

    let ask_msg = QueryMsg::Asks {
        collection: collection.to_string(),
        include_inactive: Some(false),
        start_after: None,
        limit: None,
    };
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &ask_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 0);

    let ask_msg = QueryMsg::Asks {
        collection: collection.to_string(),
        include_inactive: Some(true),
        start_after: None,
        limit: None,
    };
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &ask_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 1);

    // updating price of inactive ask throws error
    let update_ask = ExecuteMsg::UpdateAskPrice {
        collection: collection.to_string(),
        token_id,
        price: coin(200, NATIVE_DENOM),
    };
    router
        .execute_contract(creator.clone(), marketplace.clone(), &update_ask, &[])
        .unwrap_err();
}

#[test]
fn try_ask_with_finders_fee() {
    let vt = standard_minter_template(1);
    let (mut router, owner, bidder, creator) =
        (vt.router, vt.accts.owner, vt.accts.bidder, vt.accts.creator);
    let marketplace = setup_marketplace(&mut router, owner).unwrap();
    let minter_addr = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);
    let token_id = 1;

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
        finders_fee_bps: Some(500), // 5%
    };
    let res = router.execute_contract(
        creator.clone(),
        marketplace.clone(),
        &set_ask,
        &listing_funds(LISTING_FEE).unwrap(),
    );
    assert!(res.is_ok());

    let finder = Addr::unchecked("finder".to_string());

    // Bidder makes bid that meets ask price
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id,
        finders_fee_bps: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 1),
        finder: Some(finder.to_string()),
    };
    let res = router.execute_contract(
        bidder.clone(),
        marketplace,
        &set_bid_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    // Check money is transferred
    let creator_balances = router.wrap().query_all_balances(creator).unwrap();
    let creator_balance_minus_fee = calculated_creator_balance_after_fairburn();
    assert_eq!(
        creator_balances,
        coins(creator_balance_minus_fee.u128() + 100 - 2 - 5, NATIVE_DENOM)
    );
    let bidder_balances = router.wrap().query_all_balances(bidder).unwrap();
    assert_eq!(
        bidder_balances,
        vec![coin(INITIAL_BALANCE - 100, NATIVE_DENOM),]
    );
    let finder_balances = router.wrap().query_all_balances(finder).unwrap();
    assert_eq!(finder_balances, coins(5, NATIVE_DENOM));
}

#[test]
fn try_query_asks() {
    let vt = standard_minter_template(1);
    let (mut router, creator) = (vt.router, vt.accts.creator);
    let marketplace = setup_marketplace(&mut router, creator.clone()).unwrap();
    let minter_addr = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let token_id = 1;
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);
    // Mint NFT for creator
    mint(&mut router, &creator, &minter_addr);
    approve(&mut router, &creator, &collection, &marketplace, token_id);

    // test before ask is made, without using pagination
    let query_asks_msg = QueryMsg::Asks {
        collection: collection.to_string(),
        include_inactive: Some(true),
        start_after: None,
        limit: None,
    };
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_asks_msg)
        .unwrap();
    assert_eq!(res.asks, vec![]);

    // An asking price is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id,
        price: coin(110, NATIVE_DENOM),
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

    // test after ask is made
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_asks_msg)
        .unwrap();
    assert_eq!(res.asks[0].token_id, token_id);
    assert_eq!(res.asks[0].price.u128(), 110);

    // test pagination, starting when tokens exist
    let query_asks_msg = QueryMsg::Asks {
        collection: collection.to_string(),
        include_inactive: Some(true),
        start_after: Some(token_id - 1),
        limit: None,
    };
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_asks_msg)
        .unwrap();
    assert_eq!(res.asks[0].token_id, token_id);

    // test pagination, starting when token don't exist
    let query_asks_msg = QueryMsg::Asks {
        collection: collection.to_string(),
        include_inactive: Some(true),
        start_after: Some(token_id),
        limit: None,
    };
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_asks_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 0);

    // test pagination starting before token exists
    let query_reverse_asks_msg = QueryMsg::ReverseAsks {
        collection: collection.to_string(),
        include_inactive: Some(true),
        start_before: Some(token_id + 1),
        limit: None,
    };
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_reverse_asks_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 1);

    // test listed collections query
    let res: CollectionsResponse = router
        .wrap()
        .query_wasm_smart(
            marketplace,
            &QueryMsg::Collections {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(res.collections[0], "contract2");
}

#[test]
fn try_query_asks_by_seller() {
    let vt = minter_template_owner_admin(4);
    let (mut router, owner) = (vt.router, vt.accts.owner);
    let marketplace = setup_marketplace(&mut router, owner.clone()).unwrap();
    let minter_addr = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);
    // Mint NFT for creator
    let owner2: Addr = Addr::unchecked("owner2");
    // Add funds to owner2 for creation fees
    add_funds_for_incremental_fee(&mut router, &owner, CREATION_FEE, 1u128).unwrap();
    // Add funds to owner2 for listing fees
    add_funds_for_incremental_fee(&mut router, &owner2, LISTING_FEE, 2u128).unwrap();

    //     // Mint NFT for creator

    mint(&mut router, &owner, &minter_addr);
    let nft_hash: HashSet<String> = HashSet::from([]);
    let (_, token_id_0) = get_next_token_id_and_map(&mut router, &nft_hash, collection.clone());
    approve(
        &mut router,
        &owner,
        &collection.clone(),
        &marketplace,
        token_id_0,
    );

    mint_for(&mut router, &owner, &owner2, &minter_addr, token_id_0 + 1);
    approve(
        &mut router,
        &owner2,
        &collection,
        &marketplace,
        token_id_0 + 1,
    );
    mint_for(&mut router, &owner, &owner2, &minter_addr, token_id_0 + 2);
    approve(
        &mut router,
        &owner2,
        &collection,
        &marketplace,
        token_id_0 + 2,
    );

    // Owner1 lists their token for sale
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: token_id_0,
        price: coin(110, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 1),
        finders_fee_bps: Some(0),
    };
    let res = router.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &set_ask,
        &listing_funds(LISTING_FEE).unwrap(),
    );
    assert!(res.is_ok());

    // Owner2 lists their token for sale
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: token_id_0 + 1,
        price: coin(109, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 1),
        finders_fee_bps: Some(0),
    };
    let res = router.execute_contract(
        owner2.clone(),
        marketplace.clone(),
        &set_ask,
        &listing_funds(LISTING_FEE).unwrap(),
    );
    assert!(res.is_ok());

    // Owner2 lists another token for sale
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: token_id_0 + 2,
        price: coin(111, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 1),
        finders_fee_bps: Some(0),
    };
    let res = router.execute_contract(
        owner2.clone(),
        marketplace.clone(),
        &set_ask,
        &listing_funds(LISTING_FEE).unwrap(),
    );
    assert!(res.is_ok());

    let res: AskCountResponse = router
        .wrap()
        .query_wasm_smart(
            marketplace.clone(),
            &QueryMsg::AskCount {
                collection: collection.to_string(),
            },
        )
        .unwrap();
    assert_eq!(res.count, 3);

    // owner1 should only have 1 token
    let query_asks_msg = QueryMsg::AsksBySeller {
        seller: owner.to_string(),
        include_inactive: Some(true),
        start_after: None,
        limit: None,
    };
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.to_string(), &query_asks_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 1);

    // owner2 should have 2 token
    let query_asks_msg = QueryMsg::AsksBySeller {
        seller: owner2.to_string(),
        include_inactive: Some(true),
        start_after: None,
        limit: None,
    };
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.to_string(), &query_asks_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 2);

    // owner2 should have 0 tokens when paginated by a non-existing collection
    let query_asks_msg = QueryMsg::AsksBySeller {
        seller: owner2.to_string(),
        include_inactive: Some(true),
        start_after: Some(CollectionOffset::new(
            "non-existing-collection".to_string(),
            token_id_0,
        )),
        limit: None,
    };
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.to_string(), &query_asks_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 0);

    // owner2 should have 2 tokens when paginated by a existing collection
    let query_asks_msg = QueryMsg::AsksBySeller {
        seller: owner2.to_string(),
        include_inactive: Some(true),
        start_after: Some(CollectionOffset::new(collection.to_string(), 0)),
        limit: None,
    };
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.to_string(), &query_asks_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 2);

    // owner2 should have 1 token when paginated by a existing collection starting after a token
    let query_asks_msg = QueryMsg::AsksBySeller {
        seller: owner2.to_string(),
        include_inactive: Some(true),
        start_after: Some(CollectionOffset::new(
            collection.to_string(),
            token_id_0 + 1,
        )),
        limit: None,
    };
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.to_string(), &query_asks_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 1);
}

#[test]
fn try_reserved_ask() {
    let vt = standard_minter_template(1);
    let (mut router, owner, bidder, creator) =
        (vt.router, vt.accts.owner, vt.accts.bidder, vt.accts.creator);
    let marketplace = setup_marketplace(&mut router, owner.clone()).unwrap();
    let minter_addr = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);
    let token_id = 1;

    // Mint NFT for creator
    mint(&mut router, &creator, &minter_addr);
    approve(&mut router, &creator, &collection, &marketplace, token_id);

    // An ask is made by the owner
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id,
        price: coin(100, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: Some(bidder.to_string()),
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

    // Non-bidder makes bid that meets the ask price
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id,
        finders_fee_bps: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };
    let err = router
        .execute_contract(
            owner,
            marketplace.clone(),
            &set_bid_msg,
            &coins(100, NATIVE_DENOM),
        )
        .unwrap_err();
    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::TokenReserved {}
    );

    // Bidder makes bid that meets ask price
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

    // Check NFT is transferred to bidder (with reserved address)
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
fn try_remove_stale_ask() {
    let vt = standard_minter_template(2);
    let (mut router, owner, creator) = (vt.router, vt.accts.owner, vt.accts.creator);
    let marketplace = setup_marketplace(&mut router, owner.clone()).unwrap();
    let minter_addr = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    setup_block_time(&mut router, start_time.nanos(), None);

    // Mint NFT for creator
    mint(&mut router, &creator, &minter_addr);
    let nft_hash = HashSet::from([]);
    let (nft_hash, token_id_0) =
        get_next_token_id_and_map(&mut router, &nft_hash, collection.clone());
    approve(&mut router, &creator, &collection, &marketplace, token_id_0);

    // An ask is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: token_id_0,
        price: coin(100, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 1),
        finders_fee_bps: Some(500), // 5%
    };
    let res = router.execute_contract(
        creator.clone(),
        marketplace.clone(),
        &set_ask,
        &listing_funds(LISTING_FEE).unwrap(),
    );
    assert!(res.is_ok());

    // trying to remove a valid ask
    let remove_ask = ExecuteMsg::RemoveStaleAsk {
        collection: collection.to_string(),
        token_id: token_id_0,
    };
    let res = router.execute_contract(
        Addr::unchecked("operator1"),
        marketplace.clone(),
        &remove_ask,
        &[],
    );
    assert!(res.is_err());

    let ask_msg = QueryMsg::Asks {
        collection: collection.to_string(),
        include_inactive: Some(false),
        start_after: None,
        limit: None,
    };

    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &ask_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 1);

    // burn the token
    burn(&mut router, &creator, &collection, token_id_0);

    // try again to remove the ask
    let res = router.execute_contract(
        Addr::unchecked("operator1"),
        marketplace.clone(),
        &remove_ask,
        &[],
    );
    assert!(res.is_ok());
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &ask_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 0);

    // Mint NFT for creator
    mint(&mut router, &creator, &minter_addr);
    let (_, token_id_1) = get_next_token_id_and_map(&mut router, &nft_hash, collection.clone());
    approve(&mut router, &creator, &collection, &marketplace, token_id_1);

    add_funds_for_incremental_fee(&mut router, &creator, 100, 2u128).unwrap();
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: token_id_1,
        price: coin(100, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 1),
        finders_fee_bps: Some(500), // 5%
    };

    // set ask again
    let res = router.execute_contract(
        creator.clone(),
        marketplace.clone(),
        &set_ask,
        &listing_funds(LISTING_FEE).unwrap(),
    );
    assert!(res.is_ok());

    let res = router.execute_contract(
        Addr::unchecked("operator1"),
        marketplace.clone(),
        &remove_ask,
        &[],
    );
    assert!(res.is_err());

    // Transfer NFT from creator to owner. Creates a stale ask that needs to be updated
    transfer(&mut router, &creator, &owner, &collection, token_id_1);
    let res = router.execute_contract(
        Addr::unchecked("operator1"),
        marketplace.clone(),
        &remove_ask,
        &[],
    );
    assert!(res.is_err());

    // transfer nft back
    transfer(&mut router, &owner, &creator, &collection, token_id_1);

    // move time forward
    let time = router.block_info().time;
    setup_block_time(&mut router, time.plus_seconds(MIN_EXPIRY + 2).nanos(), None);

    let remove_ask = ExecuteMsg::RemoveStaleAsk {
        collection: collection.to_string(),
        token_id: token_id_1,
    };

    // remove stale ask
    let res = router.execute_contract(
        Addr::unchecked("operator1"),
        marketplace.clone(),
        &remove_ask,
        &[],
    );
    assert!(res.is_ok());
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &ask_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 0);
}

#[test]
fn try_set_ask_reserve_for() {
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

    // Can't reserve to themselves.
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id,
        price: coin(110, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: Some(creator.clone().to_string()),
        expires: start_time.plus_seconds(MIN_EXPIRY + 10),
        finders_fee_bps: Some(0),
    };
    let err = router
        .execute_contract(
            creator.clone(),
            marketplace.clone(),
            &set_ask,
            &listing_funds(LISTING_FEE).unwrap(),
        )
        .unwrap_err();
    assert_eq!(
        err.source().unwrap().to_string(),
        ContractError::InvalidReserveAddress {
            reason: "cannot reserve to the same address".to_owned(),
        }
        .to_string()
    );
    // Can't reserve for auction sale type
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::Auction,
        collection: collection.to_string(),
        token_id,
        price: coin(110, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: Some(bidder.to_string()),
        expires: start_time.plus_seconds(MIN_EXPIRY + 10),
        finders_fee_bps: Some(0),
    };
    let err = router
        .execute_contract(
            creator.clone(),
            marketplace.clone(),
            &set_ask,
            &listing_funds(LISTING_FEE).unwrap(),
        )
        .unwrap_err();
    assert_eq!(
        err.source().unwrap().to_string(),
        ContractError::InvalidReserveAddress {
            reason: "can only reserve for fixed_price sales".to_owned(),
        }
        .to_string()
    );

    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id,
        price: coin(110, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: Some(bidder.to_string()),
        expires: start_time.plus_seconds(MIN_EXPIRY + 10),
        finders_fee_bps: Some(0),
    };
    let res = router.execute_contract(
        creator.clone(),
        marketplace.clone(),
        &set_ask,
        &listing_funds(LISTING_FEE).unwrap(),
    );
    assert!(res.is_ok());

    let bidder2 = setup_second_bidder_account(&mut router).unwrap();
    // Bidder2 makes bid
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id,
        finders_fee_bps: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };
    let res = router.execute_contract(
        bidder2,
        marketplace.clone(),
        &set_bid_msg,
        &coins(110, NATIVE_DENOM),
    );
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert_eq!(
        err.source().unwrap().to_string(),
        ContractError::TokenReserved {}.to_string()
    );

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
        bidder,
        marketplace.clone(),
        &set_bid_msg,
        &coins(110, NATIVE_DENOM),
    );
    assert!(res.is_ok());
}

#[test]
fn try_update_ask() {
    let vt = standard_minter_template(1);
    let (mut router, creator) = (vt.router, vt.accts.creator);
    let minter_addr = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let marketplace = setup_marketplace(&mut router, creator.clone()).unwrap();
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    let token_id = 1;
    // Mint NFT for creator
    setup_block_time(&mut router, start_time.nanos(), None);
    mint(&mut router, &creator, &minter_addr);
    approve(&mut router, &creator, &collection, &marketplace, token_id);
    // An asking price is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id,
        price: coin(110, NATIVE_DENOM),
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

    let update_ask = ExecuteMsg::UpdateAskPrice {
        collection: collection.to_string(),
        token_id,
        price: coin(200, NATIVE_DENOM),
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &update_ask, &[]);
    assert!(res.is_ok());

    let update_ask = ExecuteMsg::UpdateAskPrice {
        collection: collection.to_string(),
        token_id,
        price: coin(200, "bobo"),
    };
    router
        .execute_contract(creator.clone(), marketplace.clone(), &update_ask, &[])
        .unwrap_err();

    let update_ask = ExecuteMsg::UpdateAskPrice {
        collection: collection.to_string(),
        token_id,
        price: coin(0, NATIVE_DENOM),
    };
    router
        .execute_contract(creator.clone(), marketplace.clone(), &update_ask, &[])
        .unwrap_err();

    // can not update ask price for expired ask
    let time = router.block_info().time;
    setup_block_time(
        &mut router,
        start_time.plus_seconds(MAX_EXPIRY).nanos(),
        None,
    );
    let update_ask = ExecuteMsg::UpdateAskPrice {
        collection: collection.to_string(),
        token_id,
        price: coin(150, NATIVE_DENOM),
    };
    router
        .execute_contract(creator.clone(), marketplace.clone(), &update_ask, &[])
        .unwrap_err();
    // reset time to original
    setup_block_time(&mut router, time.nanos(), None);

    // confirm ask removed
    let remove_ask_msg = ExecuteMsg::RemoveAsk {
        collection: collection.to_string(),
        token_id,
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &remove_ask_msg, &[]);
    assert!(res.is_ok());
    let res: AskResponse = router
        .wrap()
        .query_wasm_smart(
            marketplace.to_string(),
            &QueryMsg::Ask {
                collection: collection.to_string(),
                token_id,
            },
        )
        .unwrap();
    assert_eq!(res.ask, None);
}
#[test]
fn try_sync_ask() {
    let vt = standard_minter_template(1);
    let (mut router, owner, creator) = (vt.router, vt.accts.owner, vt.accts.creator);
    let marketplace = setup_marketplace(&mut router, owner.clone()).unwrap();
    let minter_addr = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let token_id = 1;
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    setup_block_time(&mut router, start_time.nanos(), None);

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
        finders_fee_bps: Some(500), // 5%
    };
    let res = router.execute_contract(
        creator.clone(),
        marketplace.clone(),
        &set_ask,
        &listing_funds(LISTING_FEE).unwrap(),
    );
    assert!(res.is_ok());

    // Transfer NFT from creator to owner. Creates a stale ask that needs to be updated
    transfer(&mut router, &creator, &owner, &collection, token_id);

    let update_ask_state = ExecuteMsg::SyncAsk {
        collection: collection.to_string(),
        token_id,
    };
    let res = router.execute_contract(
        Addr::unchecked("operator1"),
        marketplace.clone(),
        &update_ask_state,
        &[],
    );
    assert!(res.is_ok());

    let ask_msg = QueryMsg::Asks {
        collection: collection.to_string(),
        include_inactive: Some(false),
        start_after: None,
        limit: None,
    };
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &ask_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 0);

    // transfer nft back
    transfer(&mut router, &owner, &creator, &collection, token_id);

    // Transfer Back should have unchanged operation (still not active)
    let res = router.execute_contract(
        Addr::unchecked("operator1"),
        marketplace.clone(),
        &update_ask_state,
        &[],
    );
    assert!(res.is_err());

    let ask_msg = QueryMsg::Asks {
        collection: collection.to_string(),
        include_inactive: Some(false),
        start_after: None,
        limit: None,
    };
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &ask_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 0);

    // Approving again should have a success sync ask after
    approve(&mut router, &creator, &collection, &marketplace, token_id);

    // SyncAsk should be ok
    let res = router.execute_contract(
        Addr::unchecked("operator1"),
        marketplace.clone(),
        &update_ask_state,
        &[],
    );
    assert!(res.is_ok());
    let ask_msg = QueryMsg::Asks {
        collection: collection.to_string(),
        include_inactive: Some(false),
        start_after: None,
        limit: None,
    };
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &ask_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 1);

    // Approve for shorter period than ask
    let approve_msg: Sg721ExecuteMsg<Empty, Empty> = Sg721ExecuteMsg::Approve {
        spender: marketplace.to_string(),
        token_id: token_id.to_string(),
        expires: Some(Expiration::AtTime(start_time.plus_seconds(MIN_EXPIRY - 10))),
    };
    let res = router.execute_contract(creator.clone(), collection.clone(), &approve_msg, &[]);
    assert!(res.is_ok());

    // SyncAsk should fail (Unchanged)
    let res = router.execute_contract(
        Addr::unchecked("operator1"),
        marketplace.clone(),
        &update_ask_state,
        &[],
    );
    assert!(res.is_err());

    let expiry_time = start_time.plus_seconds(MIN_EXPIRY - 5);
    // move clock before ask expire but after approval expiration time
    setup_block_time(&mut router, expiry_time.nanos(), None);

    // SyncAsk should succeed as approval is no longer valid
    let res = router.execute_contract(
        Addr::unchecked("operator1"),
        marketplace.clone(),
        &update_ask_state,
        &[],
    );
    assert!(res.is_ok());

    // No more valid asks
    let ask_msg = QueryMsg::Asks {
        collection: collection.to_string(),
        include_inactive: Some(false),
        start_after: None,
        limit: None,
    };
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace, &ask_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 0);
}
