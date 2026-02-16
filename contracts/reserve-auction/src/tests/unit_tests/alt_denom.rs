use std::str::FromStr;

use crate::msg::{ExecuteMsg, QueryMsg};
use crate::tests::helpers::constants::{
    CREATE_AUCTION_FEE, DEFAULT_DURATION, EXTEND_DURATION, MAX_DURATION, MIN_BID_INCREMENT_PCT,
    MIN_DURATION, MIN_RESERVE_PRICE,
};
use crate::tests::setup::setup_accounts::{fund_account, setup_addtl_account, INITIAL_BALANCE};
use crate::tests::setup::setup_fair_burn::setup_fair_burn;
use crate::tests::{
    helpers::{
        auction_functions::{create_standard_auction, place_bid, query_auction},
        constants::TRADING_FEE_PCT,
        nft_functions::{approve, mint, query_owner_of},
        utils::{assert_error, calc_min_bid_increment},
    },
    setup::{
        setup_auctions::{setup_reserve_auction, DUMMY_DENOM},
        setup_minters::standard_minter_template,
    },
};
use crate::ContractError;

use crate::tests::setup::setup_auctions::DUMMY_MIN_RESERVE_PRICE_MANAGER;
use cosmwasm_std::{coin, Coin, Decimal, StdError, Uint128};
use cw_multi_test::Executor;
use regex::Regex;
use sg721_base::msg::{CollectionInfoResponse, QueryMsg as Sg721QueryMsg};
use sg_std::{GENESIS_MINT_START_TIME, NATIVE_DENOM};
use test_suite::common_setup::setup_accounts_and_block::setup_block_time;

#[test]
fn try_create_auction() {
    let vt = standard_minter_template(1);
    let (mut router, creator, _) = (vt.router, vt.accts.creator, vt.accts.bidder);
    let fair_burn = setup_fair_burn(&mut router, creator.clone());
    let auction = setup_reserve_auction(&mut router, creator.clone(), fair_burn).unwrap();
    let minter = vt.collection_response_vec[0].minter.clone().unwrap();
    let token_id: u32 = 1;
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();

    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);
    let _block_time = router.block_info().time;

    let auction_creator =
        setup_addtl_account(&mut router, "auction_creator", INITIAL_BALANCE).unwrap();

    // mint nft for creator
    mint(&mut router, &minter, &creator, &auction_creator);

    // create auction as non-owner fails
    let res = create_standard_auction(
        &mut router,
        &creator,
        &auction,
        collection.as_ref(),
        &token_id.to_string(),
        coin(MIN_RESERVE_PRICE, DUMMY_DENOM),
        DEFAULT_DURATION,
        None,
        coin(CREATE_AUCTION_FEE.u128(), NATIVE_DENOM),
    );
    assert_error(res, StdError::generic_err("Unauthorized").to_string());

    // create auction without approval fails
    let res = create_standard_auction(
        &mut router,
        &auction_creator,
        &auction,
        collection.as_ref(),
        &token_id.to_string(),
        coin(MIN_RESERVE_PRICE, DUMMY_DENOM),
        DEFAULT_DURATION,
        None,
        coin(CREATE_AUCTION_FEE.u128(), NATIVE_DENOM),
    );
    assert_error(
        res,
        "Generic error: Querier contract error: Approval not found not found".to_string(),
    );

    approve(
        &mut router,
        &auction_creator,
        &collection,
        &auction,
        token_id,
    );

    // create auction with invalid reserve price fails
    let res = create_standard_auction(
        &mut router,
        &auction_creator,
        &auction,
        collection.as_ref(),
        &token_id.to_string(),
        coin(MIN_RESERVE_PRICE - 1, DUMMY_DENOM),
        DEFAULT_DURATION,
        None,
        coin(CREATE_AUCTION_FEE.u128(), NATIVE_DENOM),
    );
    assert_error(
        res,
        ContractError::InvalidReservePrice {
            min: coin(MIN_RESERVE_PRICE, DUMMY_DENOM),
        }
        .to_string(),
    );

    // create auction with duration below minimum fails
    let res = create_standard_auction(
        &mut router,
        &auction_creator,
        &auction,
        collection.as_ref(),
        &token_id.to_string(),
        coin(MIN_RESERVE_PRICE, DUMMY_DENOM),
        MIN_DURATION - 1,
        None,
        coin(CREATE_AUCTION_FEE.u128(), NATIVE_DENOM),
    );
    assert_error(
        res,
        ContractError::InvalidDuration {
            min: MIN_DURATION,
            max: MAX_DURATION,
            got: MIN_DURATION - 1,
        }
        .to_string(),
    );

    // create auction with duration above max fails
    let res = create_standard_auction(
        &mut router,
        &auction_creator,
        &auction,
        collection.as_ref(),
        &token_id.to_string(),
        coin(MIN_RESERVE_PRICE, DUMMY_DENOM),
        MAX_DURATION + 1,
        None,
        coin(CREATE_AUCTION_FEE.u128(), NATIVE_DENOM),
    );
    assert_error(
        res,
        ContractError::InvalidDuration {
            min: MIN_DURATION,
            max: MAX_DURATION,
            got: MAX_DURATION + 1,
        }
        .to_string(),
    );

    // create auction with invalid create auction fee fails
    let res = create_standard_auction(
        &mut router,
        &auction_creator,
        &auction,
        collection.as_ref(),
        &token_id.to_string(),
        coin(MIN_RESERVE_PRICE, DUMMY_DENOM),
        DEFAULT_DURATION,
        None,
        coin(CREATE_AUCTION_FEE.u128() - 1u128, NATIVE_DENOM),
    );
    assert_error(
        res,
        ContractError::WrongFee {
            expected: coin(CREATE_AUCTION_FEE.u128(), NATIVE_DENOM),
        }
        .to_string(),
    );

    // creating valid auction succeeds
    let res = create_standard_auction(
        &mut router,
        &auction_creator,
        &auction,
        collection.as_ref(),
        &token_id.to_string(),
        coin(MIN_RESERVE_PRICE, DUMMY_DENOM),
        DEFAULT_DURATION,
        None,
        coin(CREATE_AUCTION_FEE.u128(), NATIVE_DENOM),
    );
    assert!(res.is_ok());

    // check that fair burn was paid
    let fair_burn_event = res
        .unwrap()
        .events
        .iter()
        .find(|&e| e.ty == "wasm-fair-burn")
        .unwrap()
        .clone();

    let burn_amount = fair_burn_event
        .attributes
        .iter()
        .find(|attr| attr.key == "burn_amount")
        .unwrap()
        .value
        .parse::<u64>()
        .unwrap();

    let dist_amount = fair_burn_event
        .attributes
        .iter()
        .find(|attr| attr.key == "dist_amount")
        .unwrap()
        .value
        .parse::<u64>()
        .unwrap();

    let protocol_fee = Uint128::from(burn_amount + dist_amount);
    assert_eq!(CREATE_AUCTION_FEE, protocol_fee);

    // validate contract escrows NFT on auction creation
    assert_eq!(
        query_owner_of(&router, &collection, &token_id.to_string()),
        auction.to_string()
    );

    // validate auction parameters
    let auction_info = query_auction(
        &router,
        &auction,
        collection.as_ref(),
        &token_id.to_string(),
    );
    assert_eq!(token_id.to_string(), auction_info.token_id);
    assert_eq!(collection, auction_info.collection);
    assert_eq!(auction_creator, auction_info.seller);
    assert_eq!(
        coin(MIN_RESERVE_PRICE, DUMMY_DENOM),
        auction_info.reserve_price
    );

    assert_eq!(DEFAULT_DURATION, auction_info.duration);
    assert_eq!(None, auction_info.end_time);
    assert_eq!(None, auction_info.seller_funds_recipient);
    assert_eq!(None, auction_info.high_bid);
    assert_eq!(None, auction_info.first_bid_time);

    // create duplicate auctions fails
    let res = create_standard_auction(
        &mut router,
        &auction_creator,
        &auction,
        collection.as_ref(),
        &token_id.to_string(),
        coin(MIN_RESERVE_PRICE, DUMMY_DENOM),
        DEFAULT_DURATION,
        None,
        coin(CREATE_AUCTION_FEE.u128(), NATIVE_DENOM),
    );
    assert_error(res, StdError::generic_err("Unauthorized").to_string());
}

#[test]
fn try_update_reserve_price() {
    let vt = standard_minter_template(1);
    let (mut router, creator, bidder) = (vt.router, vt.accts.creator, vt.accts.bidder);
    let fair_burn = setup_fair_burn(&mut router, creator.clone());
    let auction = setup_reserve_auction(&mut router, creator.clone(), fair_burn).unwrap();
    let minter = vt.collection_response_vec[0].minter.clone().unwrap();
    let token_id: u32 = 1;
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();

    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);
    let _block_time = router.block_info().time;

    let auction_creator =
        setup_addtl_account(&mut router, "auction_creator", INITIAL_BALANCE).unwrap();

    fund_account(&mut router, &bidder, coin(INITIAL_BALANCE, DUMMY_DENOM));

    // mint nft for creator
    mint(&mut router, &minter, &creator, &auction_creator);
    approve(
        &mut router,
        &auction_creator,
        &collection,
        &auction,
        token_id,
    );

    // creating valid auction succeeds
    let res = create_standard_auction(
        &mut router,
        &auction_creator,
        &auction,
        collection.as_ref(),
        &token_id.to_string(),
        coin(MIN_RESERVE_PRICE, NATIVE_DENOM),
        DEFAULT_DURATION,
        None,
        coin(CREATE_AUCTION_FEE.u128(), NATIVE_DENOM),
    );
    assert!(res.is_ok());

    let new_reserve_price = coin(MIN_RESERVE_PRICE + 1u128, DUMMY_DENOM);

    // update auction with non-owner fails
    let msg = ExecuteMsg::UpdateReservePrice {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        reserve_price: new_reserve_price.clone(),
    };
    let res = router.execute_contract(creator.clone(), auction.clone(), &msg, &[]);
    assert_error(res, ContractError::Unauthorized {}.to_string());

    // update auction with invalid reserve price fails
    let msg = ExecuteMsg::UpdateReservePrice {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        reserve_price: coin(MIN_RESERVE_PRICE - 1, DUMMY_DENOM),
    };
    let res = router.execute_contract(auction_creator.clone(), auction.clone(), &msg, &[]);
    assert_error(
        res,
        ContractError::InvalidReservePrice {
            min: coin(MIN_RESERVE_PRICE, DUMMY_DENOM),
        }
        .to_string(),
    );

    // update auction with valid reserve price succeeds
    let msg = ExecuteMsg::UpdateReservePrice {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        reserve_price: new_reserve_price.clone(),
    };
    let res = router.execute_contract(auction_creator.clone(), auction.clone(), &msg, &[]);
    assert!(res.is_ok());

    // update reserve price fails after bid has been placed
    let res = place_bid(
        &mut router,
        &auction,
        &bidder,
        collection.as_ref(),
        &token_id.to_string(),
        new_reserve_price.clone(),
    );
    assert!(res.is_ok());
    let msg = ExecuteMsg::UpdateReservePrice {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        reserve_price: new_reserve_price.clone(),
    };
    let res = router.execute_contract(auction_creator.clone(), auction.clone(), &msg, &[]);
    assert_error(res, ContractError::AuctionStarted {}.to_string());

    // validate auction parameters
    let auction_info = query_auction(
        &router,
        &auction,
        collection.as_ref(),
        &token_id.to_string(),
    );
    assert_eq!(new_reserve_price, auction_info.reserve_price);
}

#[test]
fn try_cancel_auction() {
    let vt = standard_minter_template(1);
    let (mut router, creator, bidder) = (vt.router, vt.accts.creator, vt.accts.bidder);
    let fair_burn = setup_fair_burn(&mut router, creator.clone());
    let auction = setup_reserve_auction(&mut router, creator.clone(), fair_burn).unwrap();
    let minter = vt.collection_response_vec[0].minter.clone().unwrap();
    let token_id: u32 = 1;
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();

    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);
    let _block_time = router.block_info().time;

    let auction_creator =
        setup_addtl_account(&mut router, "auction_creator", INITIAL_BALANCE).unwrap();

    fund_account(&mut router, &bidder, coin(INITIAL_BALANCE, DUMMY_DENOM));

    // mint nft for creator
    mint(&mut router, &minter, &creator, &auction_creator);
    approve(
        &mut router,
        &auction_creator,
        &collection,
        &auction,
        token_id,
    );

    // creating valid auction succeeds
    let res = create_standard_auction(
        &mut router,
        &auction_creator,
        &auction,
        collection.as_ref(),
        &token_id.to_string(),
        coin(MIN_RESERVE_PRICE, DUMMY_DENOM),
        DEFAULT_DURATION,
        None,
        coin(CREATE_AUCTION_FEE.u128(), NATIVE_DENOM),
    );
    assert!(res.is_ok());

    // cancel auction with non-owner fails
    let msg = ExecuteMsg::CancelAuction {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
    };
    let res = router.execute_contract(creator.clone(), auction.clone(), &msg, &[]);
    assert_error(res, ContractError::Unauthorized {}.to_string());

    // valid cancel auction succeeds
    let msg = ExecuteMsg::CancelAuction {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
    };
    let res = router.execute_contract(auction_creator.clone(), auction.clone(), &msg, &[]);
    assert!(res.is_ok());

    // duplicate cancel auction fails
    let msg = ExecuteMsg::CancelAuction {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
    };
    let res = router.execute_contract(creator.clone(), auction.clone(), &msg, &[]);
    assert_error(
        res,
        "stargaze_reserve_auction::state::Auction not found".to_string(),
    );

    // cancel auction fails after bid has been placed
    approve(
        &mut router,
        &auction_creator,
        &collection,
        &auction,
        token_id,
    );
    let res = create_standard_auction(
        &mut router,
        &auction_creator,
        &auction,
        collection.as_ref(),
        &token_id.to_string(),
        coin(MIN_RESERVE_PRICE, DUMMY_DENOM),
        DEFAULT_DURATION,
        None,
        coin(CREATE_AUCTION_FEE.u128(), NATIVE_DENOM),
    );
    assert!(res.is_ok());
    let res = place_bid(
        &mut router,
        &auction,
        &bidder,
        collection.as_ref(),
        &token_id.to_string(),
        coin(MIN_RESERVE_PRICE, DUMMY_DENOM),
    );
    assert!(res.is_ok());
    let msg = ExecuteMsg::CancelAuction {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
    };
    let res = router.execute_contract(auction_creator.clone(), auction.clone(), &msg, &[]);
    assert_error(res, ContractError::AuctionStarted {}.to_string());
}

#[test]
fn try_place_bid() {
    let vt = standard_minter_template(1);
    let (mut router, creator, bidder) = (vt.router, vt.accts.creator, vt.accts.bidder);
    let second_bidder = setup_addtl_account(&mut router, "second_bidder", INITIAL_BALANCE).unwrap();
    let fair_burn = setup_fair_burn(&mut router, creator.clone());
    let auction = setup_reserve_auction(&mut router, creator.clone(), fair_burn).unwrap();
    let minter = vt.collection_response_vec[0].minter.clone().unwrap();
    let token_id: u32 = 1;
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();

    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);
    let block_time = router.block_info().time;

    let auction_creator =
        setup_addtl_account(&mut router, "auction_creator", INITIAL_BALANCE).unwrap();

    // mint nft for creator
    mint(&mut router, &minter, &creator, &auction_creator);
    approve(
        &mut router,
        &auction_creator,
        &collection,
        &auction,
        token_id,
    );

    // creating valid auction succeeds
    let res = create_standard_auction(
        &mut router,
        &auction_creator,
        &auction,
        collection.as_ref(),
        &token_id.to_string(),
        coin(MIN_RESERVE_PRICE, DUMMY_DENOM),
        DEFAULT_DURATION,
        None,
        coin(CREATE_AUCTION_FEE.u128(), NATIVE_DENOM),
    );
    assert!(res.is_ok());

    // place bid with owner fails
    fund_account(
        &mut router,
        &auction_creator,
        coin(INITIAL_BALANCE, DUMMY_DENOM),
    );
    setup_block_time(&mut router, block_time.plus_seconds(10).nanos(), None);
    let res = place_bid(
        &mut router,
        &auction,
        &auction_creator,
        collection.as_ref(),
        &token_id.to_string(),
        coin(MIN_RESERVE_PRICE, DUMMY_DENOM),
    );
    assert_error(res, ContractError::SellerShouldNotBid {}.to_string());

    // place bid on non-existent auction fails
    fund_account(&mut router, &bidder, coin(INITIAL_BALANCE, DUMMY_DENOM));
    let res = place_bid(
        &mut router,
        &auction,
        &bidder,
        collection.as_ref(),
        &2.to_string(),
        coin(MIN_RESERVE_PRICE, DUMMY_DENOM),
    );
    assert_error(
        res,
        "stargaze_reserve_auction::state::Auction not found".to_string(),
    );

    // place bid below reserve fails
    let res = place_bid(
        &mut router,
        &auction,
        &bidder,
        collection.as_ref(),
        &token_id.to_string(),
        coin(MIN_RESERVE_PRICE - 1, DUMMY_DENOM),
    );
    assert_error(
        res,
        ContractError::BidTooLow(Uint128::from(MIN_RESERVE_PRICE)).to_string(),
    );

    // place bid above reserve succeeds
    let res = place_bid(
        &mut router,
        &auction,
        &bidder,
        collection.as_ref(),
        &token_id.to_string(),
        coin(MIN_RESERVE_PRICE, DUMMY_DENOM),
    );
    assert!(res.is_ok());

    // place bid below next valid bid fails
    let res = place_bid(
        &mut router,
        &auction,
        &bidder,
        collection.as_ref(),
        &token_id.to_string(),
        coin(
            calc_min_bid_increment(
                MIN_RESERVE_PRICE,
                Decimal::from_str(MIN_BID_INCREMENT_PCT).unwrap(),
                1,
            )
            .u128()
                - 1u128,
            DUMMY_DENOM,
        ),
    );
    assert_error(
        res,
        ContractError::BidTooLow(calc_min_bid_increment(
            MIN_RESERVE_PRICE,
            Decimal::from_str(MIN_BID_INCREMENT_PCT).unwrap(),
            1,
        ))
        .to_string(),
    );

    // place bid above next valid bid succeeds
    let res = place_bid(
        &mut router,
        &auction,
        &bidder,
        collection.as_ref(),
        &token_id.to_string(),
        coin(
            calc_min_bid_increment(
                MIN_RESERVE_PRICE,
                Decimal::from_str(MIN_BID_INCREMENT_PCT).unwrap(),
                1,
            )
            .u128(),
            DUMMY_DENOM,
        ),
    );
    assert!(res.is_ok());

    // place bid at end of auction extends auction
    fund_account(
        &mut router,
        &second_bidder,
        coin(INITIAL_BALANCE, DUMMY_DENOM),
    );
    let auction_info = query_auction(
        &router,
        &auction,
        collection.as_ref(),
        &token_id.to_string(),
    );
    let bid_time = auction_info.end_time.unwrap().minus_seconds(1u64);
    setup_block_time(&mut router, bid_time.nanos(), None);
    let res = place_bid(
        &mut router,
        &auction,
        &second_bidder,
        collection.as_ref(),
        &token_id.to_string(),
        coin(
            calc_min_bid_increment(
                MIN_RESERVE_PRICE,
                Decimal::from_str(MIN_BID_INCREMENT_PCT).unwrap(),
                2,
            )
            .u128(),
            DUMMY_DENOM,
        ),
    );
    assert!(res.is_ok());
    let auction_info = query_auction(
        &router,
        &auction,
        collection.as_ref(),
        &token_id.to_string(),
    );
    assert_eq!(
        auction_info.end_time.unwrap(),
        bid_time.plus_seconds(EXTEND_DURATION)
    );

    // place bid after auction ends fails
    setup_block_time(&mut router, auction_info.end_time.unwrap().nanos(), None);
    let res = place_bid(
        &mut router,
        &auction,
        &bidder,
        collection.as_ref(),
        &token_id.to_string(),
        coin(
            calc_min_bid_increment(
                MIN_RESERVE_PRICE,
                Decimal::from_str(MIN_BID_INCREMENT_PCT).unwrap(),
                3,
            )
            .u128(),
            DUMMY_DENOM,
        ),
    );
    assert_error(res, ContractError::AuctionEnded {}.to_string());
}

#[test]
fn try_settle_auction_with_bids() {
    let vt = standard_minter_template(1);
    let (mut router, creator, bidder) = (vt.router, vt.accts.creator, vt.accts.bidder);
    let fair_burn = setup_fair_burn(&mut router, creator.clone());
    let auction = setup_reserve_auction(&mut router, creator.clone(), fair_burn).unwrap();
    let minter = vt.collection_response_vec[0].minter.clone().unwrap();
    let token_id: u32 = 1;
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();

    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);
    let block_time = router.block_info().time;

    let auction_creator =
        setup_addtl_account(&mut router, "auction_creator", INITIAL_BALANCE).unwrap();
    let second_bidder = setup_addtl_account(&mut router, "second_bidder", INITIAL_BALANCE).unwrap();

    // mint nft for creator
    mint(&mut router, &minter, &creator, &auction_creator);
    approve(
        &mut router,
        &auction_creator,
        &collection,
        &auction,
        token_id,
    );

    // creating valid auction succeeds
    let res = create_standard_auction(
        &mut router,
        &auction_creator,
        &auction,
        collection.as_ref(),
        &token_id.to_string(),
        coin(MIN_RESERVE_PRICE, DUMMY_DENOM),
        DEFAULT_DURATION,
        None,
        coin(CREATE_AUCTION_FEE.u128(), NATIVE_DENOM),
    );
    assert!(res.is_ok());

    // settle auction before auction end fails
    let msg = ExecuteMsg::SettleAuction {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
    };
    let res = router.execute_contract(creator.clone(), auction.clone(), &msg, &[]);
    assert_error(res, ContractError::AuctionNotEnded {}.to_string());

    // place bid above reserve succeeds
    fund_account(&mut router, &bidder, coin(INITIAL_BALANCE, DUMMY_DENOM));
    let res = place_bid(
        &mut router,
        &auction,
        &bidder,
        collection.as_ref(),
        &token_id.to_string(),
        coin(MIN_RESERVE_PRICE, DUMMY_DENOM),
    );
    assert!(res.is_ok());

    let high_bid_amount = calc_min_bid_increment(
        MIN_RESERVE_PRICE,
        Decimal::from_str(MIN_BID_INCREMENT_PCT).unwrap(),
        1,
    )
    .u128();

    // place bid above next valid bid succeeds
    fund_account(
        &mut router,
        &second_bidder,
        coin(INITIAL_BALANCE, DUMMY_DENOM),
    );
    let res = place_bid(
        &mut router,
        &auction,
        &second_bidder,
        collection.as_ref(),
        &token_id.to_string(),
        coin(high_bid_amount, DUMMY_DENOM),
    );
    assert!(res.is_ok());

    setup_block_time(
        &mut router,
        block_time.plus_seconds(DEFAULT_DURATION).nanos(),
        None,
    );

    let msg = ExecuteMsg::SettleAuction {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
    };
    let res = router.execute_contract(creator.clone(), auction.clone(), &msg, &[]);
    assert!(res.is_ok());

    let fair_burn_event = res
        .unwrap()
        .events
        .iter()
        .find(|&e| e.ty == "wasm-fund-fair-burn-pool")
        .unwrap()
        .clone();

    let burn_amount = &fair_burn_event
        .attributes
        .iter()
        .find(|attr| attr.key == "coin_0")
        .unwrap()
        .value;

    fn parse_coin(input: &str) -> Option<Coin> {
        let re = Regex::new(r"(?P<amount>\d+)(?P<denom>.+)").unwrap();
        re.captures(input).map(|cap| {
            let amount = Uint128::from(cap["amount"].parse::<u128>().unwrap());
            let denom = cap["denom"].to_string();
            Coin { denom, amount }
        })
    }

    let burn_coin = parse_coin(burn_amount).unwrap();

    assert_eq!(
        Uint128::from(high_bid_amount) * Decimal::from_str(TRADING_FEE_PCT).unwrap(),
        burn_coin.amount
    );

    // check that royalty was paid
    let collection_info: CollectionInfoResponse = router
        .wrap()
        .query_wasm_smart(collection.clone(), &Sg721QueryMsg::CollectionInfo {})
        .unwrap();
    let royalty_share = collection_info.royalty_info.unwrap().share;
    let royalty_fee = Uint128::from(high_bid_amount) * royalty_share;

    let new_creator_balance = router
        .wrap()
        .query_balance(&creator, DUMMY_DENOM)
        .unwrap()
        .amount;
    assert_eq!(new_creator_balance, royalty_fee);

    // check that seller was paid
    let seller_payment = Uint128::from(high_bid_amount) - burn_coin.amount - royalty_fee;
    let new_auction_creator_balance = router
        .wrap()
        .query_balance(&auction_creator, DUMMY_DENOM)
        .unwrap()
        .amount;
    assert_eq!(new_auction_creator_balance, seller_payment);

    // check that first bidder was fully refunded
    let first_bidder_balance = router
        .wrap()
        .query_balance(&bidder, DUMMY_DENOM)
        .unwrap()
        .amount;
    assert_eq!(first_bidder_balance, Uint128::from(INITIAL_BALANCE));

    // check that second bidder debited and was given NFT
    let second_bidder_balance = router
        .wrap()
        .query_balance(&second_bidder, DUMMY_DENOM)
        .unwrap()
        .amount;
    assert_eq!(
        second_bidder_balance,
        Uint128::from(INITIAL_BALANCE - high_bid_amount)
    );
    assert_eq!(
        query_owner_of(&router, &collection, &token_id.to_string()),
        second_bidder.to_string()
    );
}

#[test]
fn try_settle_auction_with_no_bids() {
    let vt = standard_minter_template(1);
    let (mut router, creator, _bidder) = (vt.router, vt.accts.creator, vt.accts.bidder);
    let fair_burn = setup_fair_burn(&mut router, creator.clone());
    let auction = setup_reserve_auction(&mut router, creator.clone(), fair_burn).unwrap();
    let minter = vt.collection_response_vec[0].minter.clone().unwrap();
    let token_id: u32 = 1;
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();

    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);
    let block_time = router.block_info().time;

    let auction_creator =
        setup_addtl_account(&mut router, "auction_creator", INITIAL_BALANCE).unwrap();

    // mint nft for creator
    mint(&mut router, &minter, &creator, &auction_creator);
    approve(
        &mut router,
        &auction_creator,
        &collection,
        &auction,
        token_id,
    );

    // creating valid auction succeeds
    let res = create_standard_auction(
        &mut router,
        &auction_creator,
        &auction,
        collection.as_ref(),
        &token_id.to_string(),
        coin(MIN_RESERVE_PRICE, DUMMY_DENOM),
        DEFAULT_DURATION,
        None,
        coin(CREATE_AUCTION_FEE.u128(), NATIVE_DENOM),
    );
    assert!(res.is_ok());

    setup_block_time(
        &mut router,
        block_time.plus_seconds(DEFAULT_DURATION).nanos(),
        None,
    );

    // Cannot settle an auction that has no bid
    let msg = ExecuteMsg::SettleAuction {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
    };
    let res = router.execute_contract(creator.clone(), auction.clone(), &msg, &[]);
    assert_error(res, ContractError::AuctionNotEnded {}.to_string());
}

#[test]
fn try_update_min_reserve_prices() {
    let vt = standard_minter_template(1);
    let (mut router, creator, _bidder) = (vt.router, vt.accts.creator, vt.accts.bidder);
    let fair_burn = setup_fair_burn(&mut router, creator.clone());
    let reserve_auction = setup_reserve_auction(&mut router, creator.clone(), fair_burn).unwrap();

    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);

    let min_reserve_price_manager = setup_addtl_account(
        &mut router,
        DUMMY_MIN_RESERVE_PRICE_MANAGER,
        INITIAL_BALANCE,
    )
    .unwrap();

    let wrong_min_reserve_price_manager = setup_addtl_account(
        &mut router,
        "false_min_reserve_price_manager",
        INITIAL_BALANCE,
    )
    .unwrap();

    let coins_response: Vec<Coin> = router
        .wrap()
        .query_wasm_smart(
            reserve_auction.clone(),
            &QueryMsg::MinReservePrices {
                query_options: None,
            },
        )
        .unwrap();

    // Duplicate denom throws error
    let set_min_reserve_prices_msg = ExecuteMsg::SetMinReservePrices {
        min_reserve_prices: vec![coins_response[0].clone()],
    };
    let response = router.execute_contract(
        min_reserve_price_manager.clone(),
        reserve_auction.clone(),
        &set_min_reserve_prices_msg,
        &[],
    );
    assert_eq!(
        response.unwrap_err().root_cause().to_string(),
        "InvalidInput: found duplicate denom"
    );

    // New denoms can only be added by the manager
    let new_coins = vec![coin(3000000, "uosmo"), coin(4000000, "ujuno")];
    let set_min_reserve_prices_msg = ExecuteMsg::SetMinReservePrices {
        min_reserve_prices: new_coins.clone(),
    };
    let response = router.execute_contract(
        wrong_min_reserve_price_manager.clone(),
        reserve_auction.clone(),
        &set_min_reserve_prices_msg,
        &[],
    );
    assert_error(response, ContractError::Unauthorized {}.to_string());

    let new_coins = vec![coin(3000000, "uosmo"), coin(4000000, "ujuno")];
    let set_min_reserve_prices_msg = ExecuteMsg::SetMinReservePrices {
        min_reserve_prices: new_coins.clone(),
    };
    let response = router.execute_contract(
        min_reserve_price_manager.clone(),
        reserve_auction.clone(),
        &set_min_reserve_prices_msg,
        &[],
    );
    assert!(response.is_ok());

    let next_coins_response: Vec<Coin> = router
        .wrap()
        .query_wasm_smart(
            reserve_auction.clone(),
            &QueryMsg::MinReservePrices {
                query_options: None,
            },
        )
        .unwrap();

    let mut expected_coins = coins_response.clone();
    expected_coins.extend(new_coins);
    assert_eq!(next_coins_response.len(), expected_coins.len());

    // Removing non-existent denoms throws error
    let unset_min_reserve_prices_msg = ExecuteMsg::UnsetMinReservePrices {
        denoms: vec!["uusd".to_string()],
    };
    let response = router.execute_contract(
        min_reserve_price_manager.clone(),
        reserve_auction.clone(),
        &unset_min_reserve_prices_msg,
        &[],
    );
    assert_eq!(
        response.unwrap_err().root_cause().to_string(),
        "InvalidInput: denom not found"
    );

    // Removing existent denoms can only be done by the manager
    let remove_coins = vec!["uosmo".to_string(), "ujuno".to_string()];
    let unset_min_reserve_prices_msg = ExecuteMsg::UnsetMinReservePrices {
        denoms: remove_coins,
    };
    let response = router.execute_contract(
        wrong_min_reserve_price_manager.clone(),
        reserve_auction.clone(),
        &unset_min_reserve_prices_msg,
        &[],
    );
    assert_error(response, ContractError::Unauthorized {}.to_string());

    let response = router.execute_contract(
        min_reserve_price_manager.clone(),
        reserve_auction.clone(),
        &unset_min_reserve_prices_msg,
        &[],
    );
    assert!(response.is_ok());

    let next_coins_response: Vec<Coin> = router
        .wrap()
        .query_wasm_smart(
            reserve_auction,
            &QueryMsg::MinReservePrices {
                query_options: None,
            },
        )
        .unwrap();

    assert_eq!(next_coins_response.len(), coins_response.len());
}

#[test]
fn try_update_min_reserve_price_manager() {
    let vt = standard_minter_template(1);
    let (mut router, creator, _bidder) = (vt.router, vt.accts.creator, vt.accts.bidder);
    let fair_burn = setup_fair_burn(&mut router, creator.clone());
    let reserve_auction = setup_reserve_auction(&mut router, creator.clone(), fair_burn).unwrap();

    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);

    let min_reserve_price_manager = setup_addtl_account(
        &mut router,
        DUMMY_MIN_RESERVE_PRICE_MANAGER,
        INITIAL_BALANCE,
    )
    .unwrap();

    let wrong_min_reserve_price_manager = setup_addtl_account(
        &mut router,
        "false_min_reserve_price_manager",
        INITIAL_BALANCE,
    )
    .unwrap();

    let update_min_reserve_price_manager_msg = ExecuteMsg::UpdateMinReservePriceManager {
        manager: "new_manager".to_string(),
    };

    let response = router.execute_contract(
        wrong_min_reserve_price_manager.clone(),
        reserve_auction.clone(),
        &update_min_reserve_price_manager_msg,
        &[],
    );
    assert_error(response, ContractError::Unauthorized {}.to_string());

    let response = router.execute_contract(
        min_reserve_price_manager.clone(),
        reserve_auction.clone(),
        &update_min_reserve_price_manager_msg,
        &[],
    );
    assert!(response.is_ok());

    let new_manager: String = router
        .wrap()
        .query_wasm_smart(
            reserve_auction.clone(),
            &QueryMsg::MinReservePriceManager {},
        )
        .unwrap();
    assert_eq!(new_manager, "new_manager".to_string());
}
