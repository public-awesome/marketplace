use crate::error::ContractError;
use crate::execute::migrate;
use crate::helpers::ExpiryRange;
use crate::msg::{
    AskCountResponse, AskOffset, AskResponse, AsksResponse, BidOffset, BidResponse,
    CollectionBidOffset, CollectionOffset, CollectionsResponse, ParamsResponse, SudoMsg,
};
use crate::msg::{
    BidsResponse, CollectionBidResponse, CollectionBidsResponse, ExecuteMsg, QueryMsg,
};
use crate::state::{Bid, SaleType, SudoParams, SUDO_PARAMS};
use crate::testing::helpers::funds::{
    add_funds_for_incremental_fee, calculated_creator_balance_after_fairburn,
};
use crate::testing::helpers::nft_functions::{approve, burn, mint, mint_for, transfer};
use crate::testing::setup::constants::{
    BID_REMOVAL_REWARD_BPS, CREATION_FEE, INITIAL_BALANCE, LISTING_FEE, MAX_EXPIRY,
    MAX_FINDERS_FEE_BPS, MINT_FEE_FAIR_BURN, MINT_PRICE, MIN_EXPIRY, TRADING_FEE_BPS,
};
use crate::testing::setup::setup_marketplace::{
    setup_marketplace, setup_marketplace_and_collections_with_params,
};
use crate::testing::setup::setup_second_bidder_account::setup_second_bidder_account;
use cosmwasm_std::testing::{mock_dependencies, mock_env};
use cosmwasm_std::{Addr, Empty, Timestamp};
use cw721::{Cw721QueryMsg, OwnerOfResponse, TokensResponse};
use cw_multi_test::{BankSudo, Executor, SudoMsg as CwSudoMsg};
use sg_controllers::HooksResponse;
use sg_multi_test::StargazeApp;
use sg_std::GENESIS_MINT_START_TIME;

use cosmwasm_std::{coin, coins, Coin, Decimal, Uint128};
use cw_utils::{Duration, Expiration};
use sg721::ExecuteMsg as Sg721ExecuteMsg;
use std::collections::HashSet;
use std::iter::FromIterator;
use test_suite::common_setup::setup_accounts_and_block::setup_block_time;

use crate::testing::setup::templates::{
    minter_template_high_fee, minter_template_owner_admin, minter_two_collections,
    minter_two_collections_with_time, minter_with_curator, standard_minter_template,
};
use sg_std::NATIVE_DENOM;

pub fn listing_funds(listing_fee: u128) -> Result<Vec<Coin>, ContractError> {
    if listing_fee > 0 {
        Ok(vec![coin(listing_fee, NATIVE_DENOM)])
    } else {
        Ok(vec![])
    }
}

#[test]
fn try_set_accept_fixed_price_bid() {
    let vt = standard_minter_template(1);
    let (mut router, owner, creator, bidder) =
        (vt.router, vt.accts.owner, vt.accts.creator, vt.accts.bidder);
    let marketplace = setup_marketplace(&mut router, creator.clone()).unwrap();
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
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id,
        price: coin(110, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: start_time.plus_seconds(MIN_EXPIRY - 1),
        finders_fee_bps: Some(0),
    };
    let res = router.execute_contract(
        creator.clone(),
        marketplace.clone(),
        &set_ask,
        &listing_funds(LISTING_FEE).unwrap(),
    );
    assert!(res.is_err());

    // // // An asking price is made by the creator
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

    // // // Transfer nft from creator to owner. Creates a stale ask that needs to be updated
    transfer(&mut router, &creator, &owner, &collection, token_id);

    // // // Should error on non-admin trying to update active state
    let update_ask_state = ExecuteMsg::SyncAsk {
        collection: collection.to_string(),
        token_id,
    };
    router
        .execute_contract(creator.clone(), marketplace.clone(), &update_ask_state, &[])
        .unwrap_err();

    // // // Should not error on admin updating active state to false
    let res = router.execute_contract(
        Addr::unchecked("operator1"),
        marketplace.clone(),
        &update_ask_state,
        &[],
    );
    assert!(res.is_ok());

    // // // Should error when ask is unchanged
    router
        .execute_contract(
            Addr::unchecked("operator1"),
            marketplace.clone(),
            &update_ask_state,
            &[],
        )
        .unwrap_err();

    let ask_msg = QueryMsg::Ask {
        collection: collection.to_string(),
        token_id,
    };
    let res: AskResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &ask_msg)
        .unwrap();
    assert!(!res.ask.unwrap().is_active);

    // // // Reset active state
    transfer(&mut router, &owner, &creator, &collection, token_id);
    // // after transfer, needs another approval
    approve(&mut router, &creator, &collection, &marketplace, token_id);
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
    let ask_msg = QueryMsg::Ask {
        collection: collection.to_string(),
        token_id,
    };
    let res: AskResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &ask_msg)
        .unwrap();
    assert!(res.ask.unwrap().is_active);

    // // // Bidder makes bid
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
        marketplace.clone(),
        &set_bid_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    // // // Check contract has been paid
    let contract_balances = router
        .wrap()
        .query_all_balances(marketplace.clone())
        .unwrap();
    assert_eq!(contract_balances, coins(100, NATIVE_DENOM));

    // // // Check creator hasn't been paid yet
    let creator_native_balances = router.wrap().query_all_balances(creator.clone()).unwrap();

    let final_balance = calculated_creator_balance_after_fairburn();
    assert_eq!(
        creator_native_balances,
        coins(final_balance.u128(), NATIVE_DENOM)
    );

    // // Creator accepts bid
    let accept_bid_msg = ExecuteMsg::AcceptBid {
        collection: collection.to_string(),
        token_id,
        bidder: bidder.to_string(),
        finder: None,
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &accept_bid_msg, &[]);
    assert!(res.is_ok());

    // // Check money is transferred
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

    // // Check NFT is transferred
    let query_owner_msg = Cw721QueryMsg::OwnerOf {
        token_id: token_id.to_string(),
        include_expired: None,
    };
    let res: OwnerOfResponse = router
        .wrap()
        .query_wasm_smart(collection, &query_owner_msg)
        .unwrap();
    assert_eq!(res.owner, bidder.to_string());

    // // Check contract has zero balance
    let contract_balances = router.wrap().query_all_balances(marketplace).unwrap();
    assert_eq!(contract_balances, []);
}

#[test]
fn try_set_accept_bid_no_ask() {
    let vt = standard_minter_template(1);
    let (mut router, creator, bidder) = (vt.router, vt.accts.creator, vt.accts.bidder);
    let marketplace = setup_marketplace(&mut router, creator.clone()).unwrap();
    let minter_addr = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let token_id = 1;
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);
    // Mint NFT for creator
    mint(&mut router, &creator, &minter_addr);
    approve(&mut router, &creator, &collection, &marketplace, token_id);

    //     // Bidder makes bid
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::Auction,
        collection: collection.to_string(),
        token_id,
        finders_fee_bps: None,
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
    let accept_bid_msg = ExecuteMsg::AcceptBid {
        collection: collection.to_string(),
        token_id,
        bidder: bidder.to_string(),
        finder: None,
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &accept_bid_msg, &[]);
    assert!(res.is_ok());

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
    let res: OwnerOfResponse = router
        .wrap()
        .query_wasm_smart(collection, &query_owner_msg)
        .unwrap();
    assert_eq!(res.owner, bidder.to_string());

    // Check contract has zero balance
    let contract_balances = router.wrap().query_all_balances(marketplace).unwrap();
    assert_eq!(contract_balances, []);
}

#[test]
fn try_set_accept_bid_high_fees() {
    let vt = minter_template_high_fee(1);
    let (mut router, owner, creator, bidder) =
        (vt.router, vt.accts.owner, vt.accts.creator, vt.accts.bidder);
    let marketplace = setup_marketplace(&mut router, creator.clone()).unwrap();
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

    // Bidder makes bid
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::Auction,
        collection: collection.to_string(),
        token_id,
        finders_fee_bps: Some(10),
        expires: start_time.plus_seconds(MIN_EXPIRY + 1),
        finder: Some(owner.to_string()),
    };
    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_bid_msg,
        &coins(10000, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    let accept_bid_msg = ExecuteMsg::AcceptBid {
        collection: collection.to_string(),
        token_id,
        bidder: bidder.to_string(),
        finder: Some(owner.to_string()),
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &accept_bid_msg, &[]);
    assert_eq!(
        res.unwrap_err().source().unwrap().to_string(),
        "Generic error: Fees exceed payment".to_string()
    );
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

fn get_next_token_id_and_map(
    router: &mut StargazeApp,
    incoming_hash: &HashSet<String>,
    collection: Addr,
) -> (HashSet<std::string::String>, u32) {
    let query_msg = sg721_base::msg::QueryMsg::AllTokens {
        start_after: None,
        limit: None,
    };
    let res: TokensResponse = router
        .wrap()
        .query_wasm_smart(collection, &query_msg)
        .unwrap();
    let tokens_hash: HashSet<String> = HashSet::from_iter(res.tokens.iter().cloned());
    let difference = tokens_hash.difference(incoming_hash);
    let nft_hash = tokens_hash.clone();
    let token_id: Option<&String> = difference.into_iter().next();
    let token_id_unwrapped = token_id.unwrap().parse::<u32>().unwrap();
    (nft_hash, token_id_unwrapped)
}

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
fn try_query_sorted_bids() {
    let vt = standard_minter_template(3);
    let (mut router, owner, bidder, creator) =
        (vt.router, vt.accts.owner, vt.accts.bidder, vt.accts.creator);
    let marketplace = setup_marketplace(&mut router, owner).unwrap();
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

    // An asking price is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::Auction,
        collection: collection.to_string(),
        token_id: token_id_0,
        price: coin(10, NATIVE_DENOM),
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
        sale_type: SaleType::Auction,
        collection: collection.to_string(),
        token_id: token_id_1,
        price: coin(10, NATIVE_DENOM),
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
        sale_type: SaleType::Auction,
        collection: collection.to_string(),
        token_id: token_id_2,
        price: coin(10, NATIVE_DENOM),
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
        token_id: token_id_0,
        finders_fee_bps: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };
    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_bid_msg,
        &coins(50, NATIVE_DENOM),
    );
    assert!(res.is_ok());
    // Bidder makes bid
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::Auction,
        collection: collection.to_string(),
        token_id: token_id_1,
        finders_fee_bps: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };
    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_bid_msg,
        &coins(70, NATIVE_DENOM),
    );
    assert!(res.is_ok());
    // Bidder makes bid
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::Auction,
        collection: collection.to_string(),
        token_id: token_id_2,
        finders_fee_bps: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };
    router
        .execute_contract(
            bidder.clone(),
            marketplace.clone(),
            &set_bid_msg,
            &coins(1, NATIVE_DENOM),
        )
        .unwrap_err();
    let res = router.execute_contract(
        bidder,
        marketplace.clone(),
        &set_bid_msg,
        &coins(60, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    let query_bids_msg = QueryMsg::BidsSortedByPrice {
        collection: collection.to_string(),
        limit: None,
        start_after: None,
    };
    let res: BidsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_bids_msg)
        .unwrap();
    assert_eq!(res.bids.len(), 3);
    assert_eq!(res.bids[0].price.u128(), 50u128);
    assert_eq!(res.bids[1].price.u128(), 60u128);
    assert_eq!(res.bids[2].price.u128(), 70u128);

    // test adding another bid to an existing ask
    let bidder2: Addr = Addr::unchecked("bidder2");
    let funds: Vec<Coin> = coins(INITIAL_BALANCE, NATIVE_DENOM);
    router
        .sudo(CwSudoMsg::Bank({
            BankSudo::Mint {
                to_address: bidder2.to_string(),
                amount: funds,
            }
        }))
        .map_err(|err| println!("{:?}", err))
        .ok();

    // Bidder makes bid
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: token_id_0,
        finders_fee_bps: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };
    let res = router.execute_contract(
        bidder2,
        marketplace.clone(),
        &set_bid_msg,
        &coins(40, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    let res: BidsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_bids_msg)
        .unwrap();
    assert_eq!(res.bids.len(), 4);
    assert_eq!(res.bids[0].price.u128(), 40u128);
    assert_eq!(res.bids[1].price.u128(), 50u128);
    assert_eq!(res.bids[2].price.u128(), 60u128);
    assert_eq!(res.bids[3].price.u128(), 70u128);

    // test start_after query
    let start_after = BidOffset {
        price: res.bids[2].price,
        token_id: res.bids[2].token_id,
        bidder: res.bids[2].bidder.clone(),
    };
    let query_start_after_bids_msg = QueryMsg::BidsSortedByPrice {
        collection: collection.to_string(),
        limit: None,
        start_after: Some(start_after),
    };
    let res: BidsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_start_after_bids_msg)
        .unwrap();
    assert_eq!(res.bids.len(), 1);
    assert_eq!(res.bids[0].price.u128(), 70u128);

    // test reverse bids query
    let reverse_query_bids_msg = QueryMsg::ReverseBidsSortedByPrice {
        collection: collection.to_string(),
        limit: None,
        start_before: None,
    };
    let res: BidsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &reverse_query_bids_msg)
        .unwrap();
    assert_eq!(res.bids.len(), 4);
    assert_eq!(res.bids[0].price.u128(), 70u128);
    assert_eq!(res.bids[1].price.u128(), 60u128);
    assert_eq!(res.bids[2].price.u128(), 50u128);
    assert_eq!(res.bids[3].price.u128(), 40u128);

    // test start_before reverse bids query
    let start_before = BidOffset {
        price: res.bids[1].price,
        token_id: res.bids[1].token_id,
        bidder: res.bids[1].bidder.clone(),
    };
    let reverse_query_start_before_bids_msg = QueryMsg::ReverseBidsSortedByPrice {
        collection: collection.to_string(),
        limit: None,
        start_before: Some(start_before),
    };
    let res: BidsResponse = router
        .wrap()
        .query_wasm_smart(marketplace, &reverse_query_start_before_bids_msg)
        .unwrap();
    assert_eq!(res.bids.len(), 2);
    assert_eq!(res.bids[0].price.u128(), 50u128);
    assert_eq!(res.bids[1].price.u128(), 40u128);
}

#[test]
fn try_query_bids() {
    let vt = standard_minter_template(3);
    let (mut router, owner, bidder, creator) =
        (vt.router, vt.accts.owner, vt.accts.bidder, vt.accts.creator);
    let marketplace = setup_marketplace(&mut router, owner).unwrap();
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
        sale_type: SaleType::Auction,
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

    // test before bid is made
    let query_bids_msg = QueryMsg::Bids {
        collection: collection.to_string(),
        token_id: token_id_0,
        start_after: None,
        limit: None,
    };
    let res: BidsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_bids_msg)
        .unwrap();
    assert_eq!(res.bids, vec![]);

    // Bidder makes bids
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::Auction,
        collection: collection.to_string(),
        token_id: token_id_0,
        finders_fee_bps: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 10),
        finder: None,
    };
    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_bid_msg,
        &coins(120, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::Auction,
        collection: collection.to_string(),
        token_id: token_id_0 + 1,
        finders_fee_bps: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };
    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_bid_msg,
        &coins(115, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    let res: BidsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_bids_msg)
        .unwrap();
    assert_eq!(res.bids[0].token_id, token_id_0);
    assert_eq!(res.bids[0].price.u128(), 120u128);
    let query_bids_msg = QueryMsg::Bids {
        collection: collection.to_string(),
        token_id: token_id_0 + 1,
        start_after: None,
        limit: None,
    };
    let res: BidsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_bids_msg)
        .unwrap();
    assert_eq!(res.bids[0].token_id, token_id_0 + 1);
    assert_eq!(res.bids[0].price.u128(), 115u128);

    let query_bids_msg = QueryMsg::BidsByBidder {
        bidder: bidder.to_string(),
        start_after: Some(CollectionOffset::new(
            collection.to_string(),
            token_id_0 - 1,
        )),
        limit: None,
    };
    let res: BidsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_bids_msg)
        .unwrap();
    assert_eq!(res.bids.len(), 2);
    let query_bids_msg = QueryMsg::BidsByBidderSortedByExpiration {
        bidder: bidder.to_string(),
        start_after: Some(CollectionOffset::new(
            collection.to_string(),
            token_id_0 - 1,
        )),
        limit: None,
    };
    let res: BidsResponse = router
        .wrap()
        .query_wasm_smart(marketplace, &query_bids_msg)
        .unwrap();
    assert_eq!(res.bids.len(), 2);
    assert_eq!(
        res.bids[0].expires_at.seconds(),
        start_time.plus_seconds(MIN_EXPIRY + 1).seconds()
    );
    assert_eq!(
        res.bids[1].expires_at.seconds(),
        start_time.plus_seconds(MIN_EXPIRY + 10).seconds()
    );
}

#[test]
fn auto_accept_bid() {
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

    // An ask is made by the creator, but fails because NFT is not authorized
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
    assert!(res.is_err());

    // // Creator Authorizes NFT
    approve(&mut router, &creator, &collection, &marketplace, token_id);

    // Now set_ask succeeds
    let res = router.execute_contract(
        creator.clone(),
        marketplace.clone(),
        &set_ask,
        &listing_funds(LISTING_FEE).unwrap(),
    );
    assert!(res.is_ok());

    // Bidder makes bid with a random token in the same amount as the ask
    router
        .sudo(CwSudoMsg::Bank({
            BankSudo::Mint {
                to_address: bidder.to_string(),
                amount: coins(1000, "random"),
            }
        }))
        .map_err(|err| println!("{:?}", err))
        .ok();
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id,
        finders_fee_bps: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };
    router
        .execute_contract(
            bidder.clone(),
            marketplace.clone(),
            &set_bid_msg,
            &coins(100, "random"),
        )
        .unwrap_err();

    // Bidder makes bid that meets the ask criteria
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id,
        finders_fee_bps: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };
    let res = router
        .execute_contract(
            bidder.clone(),
            marketplace,
            &set_bid_msg,
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
    let res: OwnerOfResponse = router
        .wrap()
        .query_wasm_smart(collection, &query_owner_msg)
        .unwrap();
    assert_eq!(res.owner, bidder.to_string());
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
fn remove_bid_refund() {
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

    // An asking price is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::Auction,
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
        sale_type: SaleType::Auction,
        collection: collection.to_string(),
        token_id,
        finders_fee_bps: None,
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
    let remove_bid_msg = ExecuteMsg::RemoveBid {
        collection: collection.to_string(),
        token_id,
    };
    let res = router.execute_contract(bidder.clone(), marketplace, &remove_bid_msg, &[]);
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
        sale_type: SaleType::Auction,
        collection: collection.to_string(),
        token_id,
        price: coin(50, NATIVE_DENOM),
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
    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_bid_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(res.is_ok());

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
        marketplace.clone(),
        &set_bid_msg,
        &coins(150, NATIVE_DENOM),
    );
    assert!(res.is_ok());

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
    let query_bid_msg = QueryMsg::Bid {
        collection: collection.to_string(),
        token_id,
        bidder: bidder.to_string(),
    };
    let bid = Bid {
        collection,
        token_id,
        bidder,
        price: Uint128::from(150u128),
        expires_at: (start_time.plus_seconds(MIN_EXPIRY + 1)),
        finders_fee_bps: None,
    };

    let res: BidResponse = router
        .wrap()
        .query_wasm_smart(marketplace, &query_bid_msg)
        .unwrap();
    assert_eq!(res.bid, Some(bid));
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

#[test]
fn try_sudo_update_params() {
    let vt = standard_minter_template(1);
    let (mut router, owner) = (vt.router, vt.accts.owner);
    let marketplace = setup_marketplace(&mut router, owner).unwrap();
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);

    let update_params_msg = SudoMsg::UpdateParams {
        trading_fee_bps: Some(5),
        ask_expiry: Some(ExpiryRange::new(100, 2)),
        bid_expiry: None,
        operators: Some(vec!["operator".to_string()]),
        max_finders_fee_bps: None,
        min_price: Some(Uint128::from(5u128)),
        stale_bid_duration: None,
        bid_removal_reward_bps: None,
        listing_fee: Some(Uint128::from(LISTING_FEE)),
    };
    router
        .wasm_sudo(marketplace.clone(), &update_params_msg)
        .unwrap_err();

    let update_params_msg = SudoMsg::UpdateParams {
        trading_fee_bps: Some(5),
        ask_expiry: Some(ExpiryRange::new(1, 2)),
        bid_expiry: Some(ExpiryRange::new(3, 4)),
        operators: Some(vec!["operator".to_string()]),
        max_finders_fee_bps: None,
        min_price: Some(Uint128::from(5u128)),
        stale_bid_duration: Some(10),
        bid_removal_reward_bps: Some(20),
        listing_fee: Some(Uint128::from(LISTING_FEE)),
    };
    let res = router.wasm_sudo(marketplace.clone(), &update_params_msg);
    assert!(res.is_ok());

    let query_params_msg = QueryMsg::Params {};
    let res: ParamsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_params_msg)
        .unwrap();
    assert_eq!(res.params.trading_fee_percent, Decimal::percent(5));
    assert_eq!(res.params.ask_expiry, ExpiryRange::new(1, 2));
    assert_eq!(res.params.bid_expiry, ExpiryRange::new(3, 4));
    assert_eq!(res.params.operators, vec!["operator1".to_string()]);
    assert_eq!(res.params.min_price, Uint128::from(5u128));
    assert_eq!(res.params.stale_bid_duration, Duration::Time(10));
    assert_eq!(res.params.bid_removal_reward_percent, Decimal::percent(20));
    assert_eq!(res.params.listing_fee, Uint128::from(LISTING_FEE));

    let update_params_msg = SudoMsg::UpdateParams {
        trading_fee_bps: None,
        ask_expiry: None,
        bid_expiry: None,
        operators: Some(vec![
            "operator3".to_string(),
            "operator1".to_string(),
            "operator2".to_string(),
            "operator1".to_string(),
            "operator4".to_string(),
        ]),
        max_finders_fee_bps: None,
        min_price: None,
        stale_bid_duration: None,
        bid_removal_reward_bps: None,
        listing_fee: None,
    };
    let res = router.wasm_sudo(marketplace.clone(), &update_params_msg);
    assert!(res.is_ok());
    // query params
    let res: ParamsResponse = router
        .wrap()
        .query_wasm_smart(marketplace, &query_params_msg)
        .unwrap();
    assert_eq!(
        res.params.operators,
        vec![Addr::unchecked("operator1".to_string()),]
    );
}

#[test]
fn try_add_remove_sales_hooks() {
    let vt = standard_minter_template(1);
    let (mut router, owner) = (vt.router, vt.accts.owner);
    let marketplace = setup_marketplace(&mut router, owner).unwrap();
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);

    let add_hook_msg = SudoMsg::AddSaleHook {
        hook: "hook".to_string(),
    };
    let res = router.wasm_sudo(marketplace.clone(), &add_hook_msg);
    assert!(res.is_ok());

    let query_hooks_msg = QueryMsg::SaleHooks {};
    let res: HooksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_hooks_msg)
        .unwrap();
    assert_eq!(res.hooks, vec!["hook".to_string()]);

    let remove_hook_msg = SudoMsg::RemoveSaleHook {
        hook: "hook".to_string(),
    };
    let res = router.wasm_sudo(marketplace.clone(), &remove_hook_msg);
    assert!(res.is_ok());

    let res: HooksResponse = router
        .wrap()
        .query_wasm_smart(marketplace, &query_hooks_msg)
        .unwrap();
    assert!(res.hooks.is_empty());
}

#[test]
fn try_add_too_many_sales_hooks() {
    let vt = standard_minter_template(1);
    let (mut router, owner) = (vt.router, vt.accts.owner);
    let marketplace = setup_marketplace(&mut router, owner).unwrap();
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

#[test]
fn try_add_remove_bid_hooks() {
    let vt = standard_minter_template(1);
    let (mut router, owner) = (vt.router, vt.accts.owner);
    let marketplace = setup_marketplace(&mut router, owner).unwrap();
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);

    let add_bid_hook_msg = SudoMsg::AddBidHook {
        hook: "bid_hook".to_string(),
    };
    let res = router.wasm_sudo(marketplace.clone(), &add_bid_hook_msg);
    assert!(res.is_ok());

    let query_hooks_msg = QueryMsg::BidHooks {};
    let res: HooksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_hooks_msg)
        .unwrap();
    assert_eq!(res.hooks, vec!["bid_hook".to_string()]);

    let remove_hook_msg = SudoMsg::RemoveBidHook {
        hook: "bid_hook".to_string(),
    };
    let res = router.wasm_sudo(marketplace.clone(), &remove_hook_msg);
    assert!(res.is_ok());

    let res: HooksResponse = router
        .wrap()
        .query_wasm_smart(marketplace, &query_hooks_msg)
        .unwrap();
    assert!(res.hooks.is_empty());
}

#[test]
fn try_init_hook() {
    let vt = standard_minter_template(1);
    let (mut router, creator) = (vt.router, vt.accts.creator);
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);

    let msg = crate::msg::InstantiateMsg {
        operators: vec!["operator1".to_string()],
        trading_fee_bps: TRADING_FEE_BPS,
        ask_expiry: ExpiryRange::new(MIN_EXPIRY, MAX_EXPIRY),
        bid_expiry: ExpiryRange::new(MIN_EXPIRY, MAX_EXPIRY),
        sale_hook: Some("hook".to_string()),
        max_finders_fee_bps: MAX_FINDERS_FEE_BPS,
        min_price: Uint128::from(5u128),
        stale_bid_duration: Duration::Time(100),
        bid_removal_reward_bps: BID_REMOVAL_REWARD_BPS,
        listing_fee: Uint128::from(LISTING_FEE),
    };
    let marketplace =
        setup_marketplace_and_collections_with_params(&mut router, creator, msg).unwrap();

    let query_hooks_msg = QueryMsg::SaleHooks {};
    let res: HooksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_hooks_msg)
        .unwrap();
    assert_eq!(res.hooks, vec!["hook".to_string()]);

    let remove_hook_msg = SudoMsg::RemoveSaleHook {
        hook: "hook".to_string(),
    };
    let res = router.wasm_sudo(marketplace.clone(), &remove_hook_msg);
    assert!(res.is_ok());

    let res: HooksResponse = router
        .wrap()
        .query_wasm_smart(marketplace, &query_hooks_msg)
        .unwrap();
    assert!(res.hooks.is_empty());
}

#[test]
fn try_hook_was_run() {
    let vt = standard_minter_template(1);
    let (mut router, owner, bidder, creator) =
        (vt.router, vt.accts.owner, vt.accts.bidder, vt.accts.creator);
    let marketplace = setup_marketplace(&mut router, owner).unwrap();
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

    // Add bid created hook
    let add_ask_hook_msg = SudoMsg::AddBidHook {
        hook: "bid_created_hook".to_string(),
    };
    let _res = router.wasm_sudo(marketplace.clone(), &add_ask_hook_msg);

    // Mint NFT for creator
    mint(&mut router, &creator, &minter_addr);

    // An ask is made by the creator, but fails because NFT is not authorized
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
    // Creator Authorizes NFT
    let approve_msg: Sg721ExecuteMsg<Empty, Empty> = Sg721ExecuteMsg::Approve {
        spender: marketplace.to_string(),
        token_id: token_id.to_string(),
        expires: None,
    };
    let res = router.execute_contract(creator.clone(), collection.clone(), &approve_msg, &[]);
    assert!(res.is_ok());
    // Now set_ask succeeds
    let res = router.execute_contract(
        creator.clone(),
        marketplace.clone(),
        &set_ask,
        &listing_funds(LISTING_FEE).unwrap(),
    );
    assert!(res.is_ok());
    assert_eq!(
        "ask-hook-failed",
        res.unwrap().events[3].attributes[1].value
    );

    // Bidder makes bid that meets the ask criteria
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id,
        finders_fee_bps: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };

    // Bid succeeds even though the hook contract cannot be found
    let res = router.execute_contract(
        bidder.clone(),
        marketplace,
        &set_bid_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(res.is_ok());
    assert_eq!(
        "sale-hook-failed",
        res.as_ref().unwrap().events[10].attributes[1].value
    );

    // NFT is still transferred despite a sale finalized hook failing
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
fn try_add_remove_listed_hooks() {
    let vt = standard_minter_template(1);
    let (mut router, owner) = (vt.router, vt.accts.owner);
    let marketplace = setup_marketplace(&mut router, owner).unwrap();
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);
    let add_hook_msg = SudoMsg::AddAskHook {
        hook: "hook".to_string(),
    };
    let res = router.wasm_sudo(marketplace.clone(), &add_hook_msg);
    assert!(res.is_ok());

    let query_hooks_msg = QueryMsg::AskHooks {};
    let res: HooksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_hooks_msg)
        .unwrap();
    assert_eq!(res.hooks, vec!["hook".to_string()]);

    let remove_hook_msg = SudoMsg::RemoveAskHook {
        hook: "hook".to_string(),
    };
    let res = router.wasm_sudo(marketplace.clone(), &remove_hook_msg);
    assert!(res.is_ok());

    let res: HooksResponse = router
        .wrap()
        .query_wasm_smart(marketplace, &query_hooks_msg)
        .unwrap();
    assert!(res.hooks.is_empty());
}

#[test]
fn try_collection_bids() {
    let vt = minter_two_collections(1);
    let (mut router, owner, bidder, creator) =
        (vt.router, vt.accts.owner, vt.accts.bidder, vt.accts.creator);
    let marketplace = setup_marketplace(&mut router, owner).unwrap();
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
    let set_collection_bid = ExecuteMsg::SetCollectionBid {
        collection: collection.to_string(),
        finders_fee_bps: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 10),
    };
    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_collection_bid,
        &coins(150, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    // An invalid collection bid is attempted by the bidder
    let set_collection_bid = ExecuteMsg::SetCollectionBid {
        collection: collection.to_string(),
        finders_fee_bps: Some(10100),
        expires: start_time.plus_seconds(MIN_EXPIRY + 10),
    };
    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_collection_bid,
        &coins(151, NATIVE_DENOM),
    );
    assert!(res.is_err());

    // A collection bid is made by bidder2
    let set_collection_bid = ExecuteMsg::SetCollectionBid {
        collection: collection.to_string(),
        finders_fee_bps: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 5),
    };
    let res = router.execute_contract(
        bidder2.clone(),
        marketplace.clone(),
        &set_collection_bid,
        &coins(180, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    // test querying a single collection bid
    let query_collection_bid = QueryMsg::CollectionBid {
        collection: collection.to_string(),
        bidder: bidder.to_string(),
    };
    let res: CollectionBidResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_collection_bid)
        .unwrap();
    assert_eq!(res.bid.unwrap().price.u128(), 150u128);

    // test querying all collection bids by bidder
    let query_collection_bids = QueryMsg::CollectionBidsByBidder {
        bidder: bidder.to_string(),
        start_after: None,
        limit: None,
    };
    let res: CollectionBidsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_collection_bids)
        .unwrap();
    assert_eq!(res.bids[0].price.u128(), 150u128);

    // test querying all sorted collection bids by bidder
    let query_collection_bids_by_price = QueryMsg::CollectionBidsSortedByPrice {
        collection: collection.to_string(),
        start_after: None,
        limit: Some(10),
    };
    let res: CollectionBidsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_collection_bids_by_price)
        .unwrap();
    assert_eq!(res.bids.len(), 2);
    assert_eq!(res.bids[0].price.u128(), 150u128);
    assert_eq!(res.bids[1].price.u128(), 180u128);

    // test start_after
    let start_after = CollectionBidOffset::new(
        res.bids[0].price,
        collection.to_string(),
        res.bids[0].bidder.to_string(),
    );
    let query_sorted_collection_bids = QueryMsg::CollectionBidsSortedByPrice {
        collection: collection.to_string(),
        start_after: Some(start_after),
        limit: Some(10),
    };
    let res: CollectionBidsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_sorted_collection_bids)
        .unwrap();
    assert_eq!(res.bids.len(), 1);
    assert_eq!(res.bids[0].price.u128(), 180u128);

    let set_collection_bid = ExecuteMsg::SetCollectionBid {
        collection: collection_two.to_string(),
        finders_fee_bps: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 10),
    };
    let res = router.execute_contract(
        bidder2.clone(),
        marketplace.clone(),
        &set_collection_bid,
        &coins(180, NATIVE_DENOM),
    );
    assert!(res.is_ok());
    let query_collection_bids_by_expiration = QueryMsg::CollectionBidsByBidderSortedByExpiration {
        bidder: bidder2.to_string(),
        start_after: None,
        limit: None,
    };
    let res: CollectionBidsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_collection_bids_by_expiration)
        .unwrap();
    assert_eq!(res.bids.len(), 2);
    assert_eq!(
        res.bids[0].expires_at.seconds(),
        start_time.plus_seconds(MIN_EXPIRY + 5).seconds()
    );
    assert_eq!(
        res.bids[1].expires_at.seconds(),
        start_time.plus_seconds(MIN_EXPIRY + 10).seconds()
    );

    // test querying all sorted collection bids by bidder in reverse
    let reverse_query_sorted_collection_bids = QueryMsg::ReverseCollectionBidsSortedByPrice {
        collection: collection.to_string(),
        start_before: None,
        limit: Some(10),
    };
    let res: CollectionBidsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &reverse_query_sorted_collection_bids)
        .unwrap();
    assert_eq!(res.bids.len(), 2);
    assert_eq!(res.bids[0].price.u128(), 180u128);
    assert_eq!(res.bids[1].price.u128(), 150u128);

    // test start_before
    let start_before = CollectionBidOffset::new(
        res.bids[0].price,
        collection.to_string(),
        res.bids[0].bidder.to_string(),
    );
    let reverse_query_sorted_collection_bids = QueryMsg::ReverseCollectionBidsSortedByPrice {
        collection: collection.to_string(),
        start_before: Some(start_before),
        limit: Some(10),
    };
    let res: CollectionBidsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &reverse_query_sorted_collection_bids)
        .unwrap();
    assert_eq!(res.bids.len(), 1);
    assert_eq!(res.bids[0].price.u128(), 150u128);

    // test removing collection bid
    let remove_collection_bid = ExecuteMsg::RemoveCollectionBid {
        collection: collection.to_string(),
    };
    let res = router.execute_contract(bidder2, marketplace.clone(), &remove_collection_bid, &[]);
    assert!(res.is_ok());
    let query_sorted_collection_bids = QueryMsg::CollectionBidsSortedByPrice {
        collection: collection.to_string(),
        start_after: None,
        limit: Some(10),
    };
    let res: CollectionBidsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_sorted_collection_bids)
        .unwrap();
    assert_eq!(res.bids.len(), 1);
    assert_eq!(res.bids[0].price.u128(), 150u128);

    // A collection bid is accepted
    let accept_collection_bid = ExecuteMsg::AcceptCollectionBid {
        collection: collection.to_string(),
        token_id,
        bidder: bidder.to_string(),
        finder: None,
    };

    let res = router.execute_contract(creator.clone(), marketplace, &accept_collection_bid, &[]);
    assert!(res.is_ok());
}

#[test]
fn try_remove_stale_bid() {
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

    // Bidder makes bid
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::Auction,
        collection: collection.to_string(),
        token_id,
        finders_fee_bps: None,
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

    let operator = Addr::unchecked("operator1".to_string());
    // Try to remove the bid (not yet stale) as an operator
    let remove_msg = ExecuteMsg::RemoveStaleBid {
        collection: collection.to_string(),
        token_id,
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
    let res = router.execute_contract(operator.clone(), marketplace.clone(), &remove_msg, &[]);
    assert!(res.is_ok());

    let operator_balances = router.wrap().query_all_balances(operator).unwrap();
    assert_eq!(operator_balances, coins(5, NATIVE_DENOM));
}

#[test]
fn try_remove_stale_collection_bid() {
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

    let expiry_time = start_time.plus_seconds(MIN_EXPIRY + 1).seconds();

    // Bidder makes collection bid
    let set_col_bid_msg = ExecuteMsg::SetCollectionBid {
        collection: collection.to_string(),
        finders_fee_bps: None,
        expires: Timestamp::from_seconds(expiry_time),
    };
    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_col_bid_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    let operator = Addr::unchecked("operator1".to_string());

    // Try to remove the collection bid (not yet stale) as an operator
    let remove_col_msg = ExecuteMsg::RemoveStaleCollectionBid {
        collection: collection.to_string(),
        bidder: bidder.to_string(),
    };
    router
        .execute_contract(operator.clone(), marketplace.clone(), &remove_col_msg, &[])
        .unwrap_err();

    // make bid stale by adding stale_bid_duration
    let new_time = Timestamp::from_seconds(expiry_time)
        .plus_seconds(100)
        .nanos();
    setup_block_time(&mut router, new_time, None);

    let res = router.execute_contract(operator.clone(), marketplace.clone(), &remove_col_msg, &[]);
    assert!(res.is_ok());

    let operator_balances = router.wrap().query_all_balances(operator).unwrap();
    assert_eq!(operator_balances, coins(5, NATIVE_DENOM));
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
fn try_add_and_remove_operators() {
    let vt = standard_minter_template(1);
    let (mut router, owner) = (vt.router, vt.accts.owner);
    let marketplace = setup_marketplace(&mut router, owner).unwrap();
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    setup_block_time(&mut router, start_time.nanos(), None);

    let update_params_msg = SudoMsg::UpdateParams {
        trading_fee_bps: Some(5),
        ask_expiry: Some(ExpiryRange::new(1, 2)),
        bid_expiry: Some(ExpiryRange::new(3, 4)),
        operators: Some(vec!["operator2".to_string()]),
        max_finders_fee_bps: Some(1),
        min_price: Some(Uint128::from(5u128)),
        stale_bid_duration: Some(10),
        bid_removal_reward_bps: Some(20),
        listing_fee: Some(Uint128::from(LISTING_FEE)),
    };
    let res = router.wasm_sudo(marketplace.clone(), &update_params_msg);
    assert!(res.is_ok());

    // add single operator
    let add_operator_msg = SudoMsg::AddOperator {
        operator: "operator2".to_string(),
    };

    let res = router.wasm_sudo(marketplace.clone(), &add_operator_msg);
    assert!(res.is_ok());

    let query_params_msg = QueryMsg::Params {};
    let mut res: ParamsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_params_msg)
        .unwrap();
    assert_eq!(res.params.trading_fee_percent, Decimal::percent(5));
    assert_eq!(res.params.ask_expiry, ExpiryRange::new(1, 2));
    res.params.operators.sort();
    assert_eq!(
        res.params.operators,
        vec![
            Addr::unchecked("operator1".to_string()),
            Addr::unchecked("operator2".to_string()),
        ]
    );

    // remove single operator
    let add_operator_msg = SudoMsg::RemoveOperator {
        operator: "operator1".to_string(),
    };

    let res = router.wasm_sudo(marketplace.clone(), &add_operator_msg);
    assert!(res.is_ok());

    let query_params_msg = QueryMsg::Params {};
    let res: ParamsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_params_msg)
        .unwrap();
    assert_eq!(res.params.trading_fee_percent, Decimal::percent(5));
    assert_eq!(res.params.ask_expiry, ExpiryRange::new(1, 2));

    assert_eq!(res.params.trading_fee_percent, Decimal::percent(5));
    assert_eq!(res.params.ask_expiry, ExpiryRange::new(1, 2));
    assert_eq!(res.params.bid_expiry, ExpiryRange::new(3, 4));
    assert_eq!(res.params.operators, vec!["operator2".to_string()]);
    assert_eq!(res.params.stale_bid_duration, Duration::Time(10));
    assert_eq!(res.params.min_price, Uint128::from(5u128));
    assert_eq!(res.params.bid_removal_reward_percent, Decimal::percent(20));
    assert_eq!(
        res.params.operators,
        vec![Addr::unchecked("operator2".to_string()),]
    );

    // remove unexisting operator
    let remove_operator = SudoMsg::RemoveOperator {
        operator: "operator1".to_string(),
    };
    let res = router.wasm_sudo(marketplace.clone(), &remove_operator);
    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err().to_string(),
        ContractError::OperatorNotRegistered {}.to_string()
    );

    // add existing operator
    let add_operator_msg = SudoMsg::AddOperator {
        operator: "operator2".to_string(),
    };
    let res = router.wasm_sudo(marketplace, &add_operator_msg);
    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err().to_string(),
        ContractError::OperatorAlreadyRegistered {}.to_string()
    );
}

#[test]
fn try_bid_sale_type() {
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

    // An asking price is made by the creator
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
        marketplace.clone(),
        &set_bid_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(res.is_ok());

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

    // Bidder makes bid on NFT with no ask
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
        &coins(100, NATIVE_DENOM),
    );

    assert!(res.is_ok());

    // Bidder makes bid with Auction
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::Auction,
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
        &coins(100, NATIVE_DENOM),
    );

    assert!(res.is_ok());

    let query_bids_msg = QueryMsg::BidsByBidder {
        bidder: bidder2.to_string(),
        limit: None,
        start_after: None,
    };
    let res: BidsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_bids_msg)
        .unwrap();
    assert_eq!(res.bids.len(), 1);
    assert_eq!(res.bids[0].price.u128(), 100u128);
}

use cw2::set_contract_version;
#[test]
fn try_migrate() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let old_params = SudoParams {
        operators: vec![Addr::unchecked("operator1")],
        trading_fee_percent: Decimal::percent(TRADING_FEE_BPS),
        ask_expiry: ExpiryRange::new(MIN_EXPIRY, MAX_EXPIRY),
        bid_expiry: ExpiryRange::new(MIN_EXPIRY, MAX_EXPIRY),
        max_finders_fee_percent: Decimal::percent(MAX_FINDERS_FEE_BPS),
        min_price: Uint128::from(5u128),
        stale_bid_duration: Duration::Time(100),
        bid_removal_reward_percent: Decimal::percent(BID_REMOVAL_REWARD_BPS),
        listing_fee: Uint128::from(LISTING_FEE),
    };

    SUDO_PARAMS.save(&mut deps.storage, &old_params).unwrap();

    // should error when different name
    set_contract_version(&mut deps.storage, "crates.io:marketplace", "0.15.0").unwrap();
    let err = migrate(deps.as_mut(), env.clone(), Empty {}).unwrap_err();

    assert_eq!(
        err.to_string(),
        "Generic error: Cannot upgrade to a different contract"
    );

    // should error when version is greater version
    set_contract_version(&mut deps.storage, "crates.io:sg-marketplace", "2.0.0").unwrap();
    let err = migrate(deps.as_mut(), env.clone(), Empty {}).unwrap_err();
    assert_eq!(
        err.to_string(),
        "Generic error: Cannot upgrade to a previous contract version"
    );

    // no op when same version
    set_contract_version(
        &mut deps.storage,
        "crates.io:sg-marketplace",
        env!("CARGO_PKG_VERSION"),
    )
    .unwrap();
    migrate(deps.as_mut(), env.clone(), Empty {}).unwrap();

    set_contract_version(&mut deps.storage, "crates.io:sg-marketplace", "1.0.0").unwrap();
    migrate(deps.as_mut(), env, Empty {}).unwrap();

    let new_params = SUDO_PARAMS.load(&deps.storage).unwrap();

    assert_eq!(new_params.operators, old_params.operators);
    assert_eq!(
        new_params.trading_fee_percent,
        old_params.trading_fee_percent
    );
    assert_eq!(new_params.ask_expiry, old_params.ask_expiry);
    assert_eq!(new_params.bid_expiry, old_params.bid_expiry);
    assert_eq!(
        new_params.max_finders_fee_percent,
        old_params.max_finders_fee_percent
    );
    assert_eq!(new_params.min_price, old_params.min_price);
    assert_eq!(new_params.stale_bid_duration, old_params.stale_bid_duration);
    assert_eq!(
        new_params.bid_removal_reward_percent,
        old_params.bid_removal_reward_percent
    );
    assert_eq!(new_params.listing_fee, old_params.listing_fee);
}

#[test]
fn try_start_trading_time() {
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    let vt = minter_two_collections_with_time(2, start_time, start_time.plus_seconds(1));
    let (mut router, owner, bidder, creator) =
        (vt.router, vt.accts.owner, vt.accts.bidder, vt.accts.creator);
    let marketplace = setup_marketplace(&mut router, owner).unwrap();
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
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: minter_1_token_id_0,
        finders_fee_bps: None,
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

    // Bidder makes bid on NFT with no ask to collection 2
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection_2.to_string(),
        token_id: minter_2_token_id_0,
        finders_fee_bps: None,
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

    let bidder2 = setup_second_bidder_account(&mut router).unwrap();

    // A collection bid is made by the bidder
    let set_collection_bid = ExecuteMsg::SetCollectionBid {
        collection: collection_2.to_string(),
        finders_fee_bps: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 10),
    };
    let res = router.execute_contract(
        bidder2.clone(),
        marketplace.clone(),
        &set_collection_bid,
        &coins(100, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    // An asking price is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: minter_1_token_id_0,
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

    // An asking price is made by the creator to collection 2 (should fail)
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection_2.to_string(),
        token_id: minter_2_token_id_0,
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

    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err().source().unwrap().to_string(),
        "Collection not tradable yet".to_string()
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

    // Creator accepts bid
    let accept_bid_msg = ExecuteMsg::AcceptBid {
        collection: collection.to_string(),
        token_id: minter_1_token_id_0,
        bidder: bidder.to_string(),
        finder: None,
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &accept_bid_msg, &[]);
    assert!(res.is_ok());

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
    let res: OwnerOfResponse = router
        .wrap()
        .query_wasm_smart(collection, &query_owner_msg)
        .unwrap();
    assert_eq!(res.owner, bidder.to_string());

    // Creator tries to accept accept bid on collection 2 (should fail)
    let accept_bid_msg = ExecuteMsg::AcceptCollectionBid {
        collection: collection_2.to_string(),
        token_id: minter_2_token_id_0,
        bidder: bidder2.to_string(),
        finder: None,
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &accept_bid_msg, &[]);
    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err().source().unwrap().to_string(),
        "Collection not tradable yet".to_string()
    );

    // A collection bid is accepted
    let accept_collection_bid = ExecuteMsg::AcceptCollectionBid {
        collection: collection_2.to_string(),
        token_id: minter_2_token_id_0,
        bidder: bidder2.to_string(),
        finder: None,
    };

    let res = router.execute_contract(
        creator.clone(),
        marketplace.clone(),
        &accept_collection_bid,
        &[],
    );
    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err().source().unwrap().to_string(),
        "Collection not tradable yet".to_string()
    );

    // move time to start trading time
    setup_block_time(&mut router, start_time.plus_seconds(1).nanos(), None);

    // Creator tries to accept accept bid on collection 2  should work now
    let accept_bid_msg = ExecuteMsg::AcceptBid {
        collection: collection_2.to_string(),
        token_id: minter_2_token_id_0,
        bidder: bidder.to_string(),
        finder: None,
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &accept_bid_msg, &[]);
    assert!(res.is_ok());

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
        sale_type: SaleType::FixedPrice,
        collection: collection_2.to_string(),
        token_id: minter_2_token_id_0,
        price: coin(110, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 2),
        finders_fee_bps: Some(0),
    };
    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_ask,
        &listing_funds(LISTING_FEE).unwrap(),
    );
    assert!(res.is_ok());

    // A collection bid is accepted
    let accept_collection_bid = ExecuteMsg::AcceptCollectionBid {
        collection: collection_2.to_string(),
        token_id: minter_2_token_id_0,
        bidder: bidder2.to_string(),
        finder: None,
    };

    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &accept_collection_bid,
        &[],
    );

    assert!(res.is_ok());

    approve(
        &mut router,
        &bidder2,
        &collection_2,
        &marketplace,
        minter_2_token_id_0,
    );

    // An asking price is made by the bidder to collection
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection_2.to_string(),
        token_id: minter_2_token_id_0,
        price: coin(110, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 2),
        finders_fee_bps: Some(0),
    };
    let res = router.execute_contract(
        bidder2.clone(),
        marketplace.clone(),
        &set_ask,
        &listing_funds(LISTING_FEE).unwrap(),
    );
    assert!(res.is_ok());

    // Bidder buys now
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection_2.to_string(),
        token_id: minter_2_token_id_0,
        finders_fee_bps: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 2),
        finder: None,
    };
    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_bid_msg,
        &coins(110, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    // Check NFT is transferred
    let query_owner_msg = Cw721QueryMsg::OwnerOf {
        token_id: minter_2_token_id_0.to_string(),
        include_expired: None,
    };
    let res: OwnerOfResponse = router
        .wrap()
        .query_wasm_smart(collection_2, &query_owner_msg)
        .unwrap();
    assert_eq!(res.owner, bidder.to_string());
}

mod query {
    use super::*;

    #[test]
    fn collections() {
        let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
        let vt = minter_two_collections(1);
        let (mut router, owner, creator) = (vt.router, vt.accts.owner, vt.accts.creator);
        add_funds_for_incremental_fee(&mut router, &creator, CREATION_FEE, 1u128).unwrap();
        let marketplace = setup_marketplace(&mut router, owner).unwrap();
        let minter1 = vt.collection_response_vec[0].minter.clone().unwrap();
        let collection1 = vt.collection_response_vec[0].collection.clone().unwrap();

        let minter2 = vt.collection_response_vec[1].minter.clone().unwrap();
        let collection2 = vt.collection_response_vec[1].collection.clone().unwrap();
        setup_block_time(&mut router, start_time.nanos(), None);

        let token_id = 1;
        // place two asks
        mint(&mut router, &creator, &minter1);
        mint(&mut router, &creator, &minter2);
        approve(&mut router, &creator, &collection1, &marketplace, token_id);
        approve(&mut router, &creator, &collection2, &marketplace, token_id);

        let set_ask = ExecuteMsg::SetAsk {
            sale_type: SaleType::FixedPrice,
            collection: collection1.to_string(),
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

        let set_ask = ExecuteMsg::SetAsk {
            sale_type: SaleType::FixedPrice,
            collection: collection2.to_string(),
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

        // check collections query
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
        assert_eq!(res.collections[0], collection1);
        assert_eq!(res.collections[1], collection2);
    }
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
