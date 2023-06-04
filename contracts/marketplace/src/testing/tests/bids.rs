use crate::msg::{AskResponse, BidOffset, BidResponse, CollectionBidOffset, CollectionOffset};
use crate::msg::{
    BidsResponse, CollectionBidResponse, CollectionBidsResponse, ExecuteMsg, QueryMsg,
};
use crate::state_deprecated::{Bid, SaleType};
use crate::testing::helpers::funds::{
    add_funds_for_incremental_fee, calculated_creator_balance_after_fairburn, listing_funds,
};
use crate::testing::helpers::nft_functions::{approve, get_next_token_id_and_map, mint, transfer};
use crate::testing::setup::setup_accounts::{
    setup_second_bidder_account, CREATION_FEE, INITIAL_BALANCE,
};
use crate::testing::setup::setup_marketplace::{setup_marketplace, LISTING_FEE, MIN_EXPIRY};
use cosmwasm_std::{Addr, Timestamp};
use cw721::{Cw721QueryMsg, OwnerOfResponse};
use cw_multi_test::{BankSudo, Executor, SudoMsg as CwSudoMsg};
use sg_std::GENESIS_MINT_START_TIME;

use cosmwasm_std::{coin, coins, Coin, Uint128};
use std::collections::HashSet;
use test_suite::common_setup::setup_accounts_and_block::setup_block_time;

use crate::testing::setup::templates::{
    minter_template_high_fee, minter_two_collections, standard_minter_template,
};
use sg_std::NATIVE_DENOM;

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
fn reject_bid() {
    let vt = standard_minter_template(1);
    let (mut router, owner, bidder, seller) =
        (vt.router, vt.accts.owner, vt.accts.bidder, vt.accts.creator);
    let marketplace = setup_marketplace(&mut router, owner).unwrap();
    let minter_addr = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);
    let token_id = 1;

    mint(&mut router, &seller, &minter_addr);

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

    approve(&mut router, &seller, &collection, &marketplace, token_id);

    let res = router.execute_contract(
        seller.clone(),
        marketplace.clone(),
        &set_ask,
        &listing_funds(LISTING_FEE).unwrap(),
    );
    assert!(res.is_ok());

    let bidder_bal = router.wrap().query_all_balances(bidder.clone()).unwrap();
    assert_eq!(bidder_bal, vec![coin(INITIAL_BALANCE, NATIVE_DENOM),]);

    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::Auction,
        collection: collection.to_string(),
        token_id,
        finders_fee_bps: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };
    let bid_amount = 100;
    router
        .execute_contract(
            bidder.clone(),
            marketplace.clone(),
            &set_bid_msg,
            &coins(bid_amount, NATIVE_DENOM),
        )
        .unwrap();

    let bidder_bal = router.wrap().query_all_balances(bidder.clone()).unwrap();
    assert_eq!(
        bidder_bal,
        vec![coin(INITIAL_BALANCE - bid_amount, NATIVE_DENOM),]
    );

    // seller rejects bid
    let reject_bid_msg = ExecuteMsg::RejectBid {
        collection: collection.to_string(),
        token_id,
        bidder: bidder.to_string(),
    };
    let res = router
        .execute_contract(seller.clone(), marketplace, &reject_bid_msg, &[])
        .unwrap();
    assert_eq!(res.events[1].ty, "wasm-reject-bid");

    let bidder_bal = router.wrap().query_all_balances(bidder).unwrap();
    assert_eq!(bidder_bal, vec![coin(INITIAL_BALANCE, NATIVE_DENOM),]);
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
    assert!(res.is_err());
    assert!(res
        .unwrap_err()
        .source()
        .unwrap()
        .to_string()
        .contains("Overflow"));
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
