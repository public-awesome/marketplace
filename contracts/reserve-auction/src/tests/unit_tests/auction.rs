use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::tests::helpers::constants::{
    CREATE_AUCTION_FEE, EXTEND_DURATION, MIN_BID_INCREMENT_BPS, MIN_DURATION, MIN_RESERVE_PRICE,
};
use crate::tests::setup::setup_accounts::{setup_addtl_account, INITIAL_BALANCE};
use crate::tests::{
    helpers::{
        auction_functions::{
            auction_contract, create_standard_auction, instantiate_auction, place_bid,
            query_auction,
        },
        nft_functions::{approve, mint, query_owner_of},
        utils::{assert_error, calc_min_bid_increment},
    },
    setup::{
        setup_auctions::setup_reserve_auction,
        setup_marketplace::{setup_marketplace, TRADING_FEE_BPS},
        setup_minters::standard_minter_template,
    },
};
use crate::ContractError;
use cosmwasm_std::{coin, Decimal, StdError, Uint128};
use cw_multi_test::Executor;
use sg721_base::msg::{CollectionInfoResponse, QueryMsg as Sg721QueryMsg};
use sg_multi_test::StargazeApp;
use sg_std::{GENESIS_MINT_START_TIME, NATIVE_DENOM};
use test_suite::common_setup::setup_accounts_and_block::setup_block_time;

#[test]
fn try_instantiate() {
    let mut app = StargazeApp::default();
    let auction_id = app.store_code(auction_contract());
    let vt = standard_minter_template(1);
    let (mut router, creator, _) = (vt.router, vt.accts.creator, vt.accts.bidder);
    let marketplace = setup_marketplace(&mut router, creator.clone()).unwrap();

    let msg = InstantiateMsg {
        marketplace: marketplace.to_string(),
        min_reserve_price: coin(1u128, NATIVE_DENOM),
        min_duration: 120,
        min_bid_increment_bps: 10,
        extend_duration: 60,
        create_auction_fee: Uint128::new(1),
        max_auctions_to_settle_per_block: 200,
    };
    let auction_addr = instantiate_auction(&mut app, auction_id, msg.clone());

    let res: ConfigResponse = app
        .wrap()
        .query_wasm_smart(auction_addr, &QueryMsg::Config {})
        .unwrap();

    assert_eq!(&res.config.create_auction_fee, &msg.create_auction_fee);
    assert_eq!(&res.config.min_reserve_price, &msg.min_reserve_price);
    assert_eq!(&res.config.min_duration, &msg.min_duration);
    assert_eq!(
        &res.config.min_bid_increment_pct,
        &Decimal::percent(msg.min_bid_increment_bps)
    );
    assert_eq!(&res.config.extend_duration, &msg.extend_duration);
}

#[test]
fn try_create_auction() {
    let vt = standard_minter_template(1);
    let (mut router, creator, _) = (vt.router, vt.accts.creator, vt.accts.bidder);
    let marketplace = setup_marketplace(&mut router, creator.clone()).unwrap();
    let auction = setup_reserve_auction(&mut router, creator.clone(), marketplace).unwrap();
    let minter = vt.collection_response_vec[0].minter.clone().unwrap();
    let token_id: u32 = 1;
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();

    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);
    let block_time = router.block_info().time;

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
        block_time,
        block_time.plus_seconds(MIN_DURATION),
        MIN_RESERVE_PRICE,
        None,
        CREATE_AUCTION_FEE.u128(),
    );
    assert_error(res, StdError::generic_err("Unauthorized").to_string());

    // create auction without approval fails
    let res = create_standard_auction(
        &mut router,
        &auction_creator,
        &auction,
        collection.as_ref(),
        &token_id.to_string(),
        block_time,
        block_time.plus_seconds(MIN_DURATION),
        MIN_RESERVE_PRICE,
        None,
        CREATE_AUCTION_FEE.u128(),
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
        block_time,
        block_time.plus_seconds(MIN_DURATION),
        MIN_RESERVE_PRICE - 1,
        None,
        CREATE_AUCTION_FEE.u128(),
    );
    assert_error(
        res,
        ContractError::InvalidReservePrice {
            min: coin(MIN_RESERVE_PRICE, NATIVE_DENOM),
        }
        .to_string(),
    );

    // create auction with invalid start time fails
    let res = create_standard_auction(
        &mut router,
        &auction_creator,
        &auction,
        collection.as_ref(),
        &token_id.to_string(),
        block_time.minus_seconds(1),
        block_time.plus_seconds(MIN_DURATION),
        MIN_RESERVE_PRICE,
        None,
        CREATE_AUCTION_FEE.u128(),
    );
    assert_error(res, ContractError::InvalidStartTime {}.to_string());

    // create auction with invalid end time fails
    let res = create_standard_auction(
        &mut router,
        &auction_creator,
        &auction,
        collection.as_ref(),
        &token_id.to_string(),
        block_time,
        block_time.plus_seconds(MIN_DURATION - 1),
        MIN_RESERVE_PRICE,
        None,
        CREATE_AUCTION_FEE.u128(),
    );
    assert_error(res, ContractError::InvalidEndTime {}.to_string());

    // create auction with invalid create auction fee fails
    let res = create_standard_auction(
        &mut router,
        &auction_creator,
        &auction,
        collection.as_ref(),
        &token_id.to_string(),
        block_time,
        block_time.plus_seconds(MIN_DURATION - 1),
        MIN_RESERVE_PRICE,
        None,
        CREATE_AUCTION_FEE.u128() - 1u128,
    );
    assert_error(
        res,
        ContractError::WrongFee {
            given: Uint128::from(CREATE_AUCTION_FEE.u128() - 1u128),
            required: CREATE_AUCTION_FEE,
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
        block_time,
        block_time.plus_seconds(MIN_DURATION),
        MIN_RESERVE_PRICE,
        None,
        CREATE_AUCTION_FEE.u128(),
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
        coin(MIN_RESERVE_PRICE, NATIVE_DENOM),
        auction_info.reserve_price
    );
    assert_eq!(block_time, auction_info.start_time);
    assert_eq!(block_time.plus_seconds(MIN_DURATION), auction_info.end_time);
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
        block_time,
        block_time.plus_seconds(MIN_DURATION),
        MIN_RESERVE_PRICE,
        None,
        CREATE_AUCTION_FEE.u128(),
    );
    assert_error(res, StdError::generic_err("Unauthorized").to_string());
}

#[test]
fn try_update_reserve_price() {
    let vt = standard_minter_template(1);
    let (mut router, creator, bidder) = (vt.router, vt.accts.creator, vt.accts.bidder);
    let marketplace = setup_marketplace(&mut router, creator.clone()).unwrap();
    let auction = setup_reserve_auction(&mut router, creator.clone(), marketplace).unwrap();
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
        block_time,
        block_time.plus_seconds(MIN_DURATION),
        MIN_RESERVE_PRICE,
        None,
        CREATE_AUCTION_FEE.u128(),
    );
    assert!(res.is_ok());

    let new_reserve_price = coin(MIN_RESERVE_PRICE + 1u128, NATIVE_DENOM);

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
        reserve_price: coin(MIN_RESERVE_PRICE - 1, NATIVE_DENOM),
    };
    let res = router.execute_contract(auction_creator.clone(), auction.clone(), &msg, &[]);
    assert_error(
        res,
        ContractError::InvalidReservePrice {
            min: coin(MIN_RESERVE_PRICE, NATIVE_DENOM),
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
        new_reserve_price.amount.u128(),
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
    let marketplace = setup_marketplace(&mut router, creator.clone()).unwrap();
    let auction = setup_reserve_auction(&mut router, creator.clone(), marketplace).unwrap();
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
        block_time,
        block_time.plus_seconds(MIN_DURATION),
        MIN_RESERVE_PRICE,
        None,
        CREATE_AUCTION_FEE.u128(),
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
    assert_error(res, "reserve_auction::state::Auction not found".to_string());

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
        block_time,
        block_time.plus_seconds(MIN_DURATION),
        MIN_RESERVE_PRICE,
        None,
        CREATE_AUCTION_FEE.u128(),
    );
    assert!(res.is_ok());
    let res = place_bid(
        &mut router,
        &auction,
        &bidder,
        collection.as_ref(),
        &token_id.to_string(),
        MIN_RESERVE_PRICE,
    );
    assert!(res.is_ok());
    let msg = ExecuteMsg::CancelAuction {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
    };
    let res = router.execute_contract(creator.clone(), auction.clone(), &msg, &[]);
    assert_error(res, ContractError::AuctionStarted {}.to_string());
}

#[test]
fn try_place_bid() {
    let vt = standard_minter_template(1);
    let (mut router, creator, bidder) = (vt.router, vt.accts.creator, vt.accts.bidder);
    let second_bidder = setup_addtl_account(&mut router, "second_bidder", INITIAL_BALANCE).unwrap();
    let marketplace = setup_marketplace(&mut router, creator.clone()).unwrap();
    let auction = setup_reserve_auction(&mut router, creator.clone(), marketplace).unwrap();
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
    let duration_buffer = 100u64;
    let res = create_standard_auction(
        &mut router,
        &auction_creator,
        &auction,
        collection.as_ref(),
        &token_id.to_string(),
        block_time.plus_seconds(duration_buffer),
        block_time
            .plus_seconds(duration_buffer)
            .plus_seconds(MIN_DURATION),
        MIN_RESERVE_PRICE,
        None,
        CREATE_AUCTION_FEE.u128(),
    );
    assert!(res.is_ok());

    // place bid before auction start fails
    let res = place_bid(
        &mut router,
        &auction,
        &bidder,
        collection.as_ref(),
        &token_id.to_string(),
        MIN_RESERVE_PRICE,
    );
    assert_error(res, ContractError::AuctionNotStarted {}.to_string());

    // place bid with owner fails
    setup_block_time(
        &mut router,
        block_time.plus_seconds(duration_buffer).nanos(),
        None,
    );
    let res = place_bid(
        &mut router,
        &auction,
        &auction_creator,
        collection.as_ref(),
        &token_id.to_string(),
        MIN_RESERVE_PRICE,
    );
    assert_error(res, ContractError::SellerShouldNotBid {}.to_string());

    // place bid on non-existent auction fails
    let res = place_bid(
        &mut router,
        &auction,
        &bidder,
        collection.as_ref(),
        &2.to_string(),
        MIN_RESERVE_PRICE,
    );
    assert_error(res, "reserve_auction::state::Auction not found".to_string());

    // place bid below reserve fails
    let res = place_bid(
        &mut router,
        &auction,
        &bidder,
        collection.as_ref(),
        &token_id.to_string(),
        MIN_RESERVE_PRICE - 1,
    );
    assert_error(
        res,
        ContractError::ReserveNotMet {
            min: coin(MIN_RESERVE_PRICE, NATIVE_DENOM),
        }
        .to_string(),
    );

    // place bid above reserve succeeds
    let res = place_bid(
        &mut router,
        &auction,
        &bidder,
        collection.as_ref(),
        &token_id.to_string(),
        MIN_RESERVE_PRICE,
    );
    assert!(res.is_ok());

    // place bid below next valid bid fails
    let res = place_bid(
        &mut router,
        &auction,
        &bidder,
        collection.as_ref(),
        &token_id.to_string(),
        calc_min_bid_increment(MIN_RESERVE_PRICE, MIN_BID_INCREMENT_BPS, 1).u128() - 1u128,
    );
    assert_error(
        res,
        ContractError::BidTooLow(calc_min_bid_increment(
            MIN_RESERVE_PRICE,
            MIN_BID_INCREMENT_BPS,
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
        calc_min_bid_increment(MIN_RESERVE_PRICE, MIN_BID_INCREMENT_BPS, 1).u128(),
    );
    assert!(res.is_ok());

    // place bid at end of auction extends auction
    let auction_info = query_auction(
        &router,
        &auction,
        collection.as_ref(),
        &token_id.to_string(),
    );
    assert_eq!(
        auction_info.end_time,
        block_time
            .plus_seconds(duration_buffer)
            .plus_seconds(MIN_DURATION)
    );
    let bid_time = auction_info.end_time.minus_seconds(1u64);
    setup_block_time(&mut router, bid_time.nanos(), None);
    let res = place_bid(
        &mut router,
        &auction,
        &second_bidder,
        collection.as_ref(),
        &token_id.to_string(),
        calc_min_bid_increment(MIN_RESERVE_PRICE, MIN_BID_INCREMENT_BPS, 2).u128(),
    );
    assert!(res.is_ok());
    let auction_info = query_auction(
        &router,
        &auction,
        collection.as_ref(),
        &token_id.to_string(),
    );
    assert_eq!(
        auction_info.end_time,
        bid_time.plus_seconds(EXTEND_DURATION)
    );

    // place bid after auction ends fails
    setup_block_time(&mut router, auction_info.end_time.nanos(), None);
    let res = place_bid(
        &mut router,
        &auction,
        &bidder,
        collection.as_ref(),
        &token_id.to_string(),
        calc_min_bid_increment(MIN_RESERVE_PRICE, MIN_BID_INCREMENT_BPS, 3).u128(),
    );
    assert_error(res, ContractError::AuctionEnded {}.to_string());
}

#[test]
fn try_settle_auction_with_bids() {
    let vt = standard_minter_template(1);
    let (mut router, creator, bidder) = (vt.router, vt.accts.creator, vt.accts.bidder);
    let marketplace = setup_marketplace(&mut router, creator.clone()).unwrap();
    let auction = setup_reserve_auction(&mut router, creator.clone(), marketplace).unwrap();
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
    let duration_buffer = 100u64;
    let res = create_standard_auction(
        &mut router,
        &auction_creator,
        &auction,
        collection.as_ref(),
        &token_id.to_string(),
        block_time.plus_seconds(duration_buffer),
        block_time
            .plus_seconds(duration_buffer)
            .plus_seconds(MIN_DURATION),
        MIN_RESERVE_PRICE,
        None,
        CREATE_AUCTION_FEE.u128(),
    );
    assert!(res.is_ok());

    // settle auction before auction start fails
    let msg = ExecuteMsg::SettleAuction {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
    };
    let res = router.execute_contract(creator.clone(), auction.clone(), &msg, &[]);
    assert_error(res, ContractError::AuctionNotStarted {}.to_string());

    setup_block_time(
        &mut router,
        block_time.plus_seconds(duration_buffer).nanos(),
        None,
    );

    // settle auction before auction end fails
    let msg = ExecuteMsg::SettleAuction {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
    };
    let res = router.execute_contract(creator.clone(), auction.clone(), &msg, &[]);
    assert_error(res, ContractError::AuctionNotEnded {}.to_string());

    // place bid above reserve succeeds
    let res = place_bid(
        &mut router,
        &auction,
        &bidder,
        collection.as_ref(),
        &token_id.to_string(),
        MIN_RESERVE_PRICE,
    );
    assert!(res.is_ok());

    let high_bid_amount =
        calc_min_bid_increment(MIN_RESERVE_PRICE, MIN_BID_INCREMENT_BPS, 1).u128();
    // place bid above next valid bid succeeds
    let res = place_bid(
        &mut router,
        &auction,
        &second_bidder,
        collection.as_ref(),
        &token_id.to_string(),
        high_bid_amount,
    );
    println!("{:?}", res);
    assert!(res.is_ok());

    setup_block_time(
        &mut router,
        block_time
            .plus_seconds(duration_buffer)
            .plus_seconds(MIN_DURATION)
            .nanos(),
        None,
    );

    let msg = ExecuteMsg::SettleAuction {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
    };
    let res = router.execute_contract(creator.clone(), auction.clone(), &msg, &[]);
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

    let trading_fee_percent = Decimal::percent(TRADING_FEE_BPS) / Uint128::from(100u128);
    let protocol_fee = Uint128::from(burn_amount + dist_amount);
    assert_eq!(
        Uint128::from(high_bid_amount) * trading_fee_percent,
        protocol_fee
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
        .query_balance(&creator, NATIVE_DENOM)
        .unwrap()
        .amount;
    assert_eq!(
        new_creator_balance,
        Uint128::from(INITIAL_BALANCE) + royalty_fee
    );

    // check that seller was paid
    let seller_payment = Uint128::from(high_bid_amount) - protocol_fee - royalty_fee;
    let new_auction_creator_balance = router
        .wrap()
        .query_balance(&auction_creator, NATIVE_DENOM)
        .unwrap()
        .amount;
    assert_eq!(
        new_auction_creator_balance,
        Uint128::from(INITIAL_BALANCE) - CREATE_AUCTION_FEE + seller_payment
    );

    // check that first bidder was fully refunded
    let first_bidder_balance = router
        .wrap()
        .query_balance(&bidder, NATIVE_DENOM)
        .unwrap()
        .amount;
    assert_eq!(first_bidder_balance, Uint128::from(INITIAL_BALANCE));

    // check that second bidder debited and was given NFT
    let second_bidder_balance = router
        .wrap()
        .query_balance(&second_bidder, NATIVE_DENOM)
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
    let marketplace = setup_marketplace(&mut router, creator.clone()).unwrap();
    let auction = setup_reserve_auction(&mut router, creator.clone(), marketplace).unwrap();
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
    let duration_buffer = 100u64;
    let res = create_standard_auction(
        &mut router,
        &auction_creator,
        &auction,
        collection.as_ref(),
        &token_id.to_string(),
        block_time.plus_seconds(duration_buffer),
        block_time
            .plus_seconds(duration_buffer)
            .plus_seconds(MIN_DURATION),
        MIN_RESERVE_PRICE,
        None,
        CREATE_AUCTION_FEE.u128(),
    );
    assert!(res.is_ok());

    setup_block_time(
        &mut router,
        block_time.plus_seconds(duration_buffer).nanos(),
        None,
    );

    setup_block_time(
        &mut router,
        block_time
            .plus_seconds(duration_buffer)
            .plus_seconds(MIN_DURATION)
            .nanos(),
        None,
    );

    let msg = ExecuteMsg::SettleAuction {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
    };
    let res = router.execute_contract(creator.clone(), auction.clone(), &msg, &[]);
    assert!(res.is_ok());

    // check that seller was given NFT
    assert_eq!(
        query_owner_of(&router, &collection, &token_id.to_string()),
        auction_creator.to_string()
    );

    // check that seller has correct final balance
    let new_auction_creator_balance = router
        .wrap()
        .query_balance(&auction_creator, NATIVE_DENOM)
        .unwrap()
        .amount;
    assert_eq!(
        new_auction_creator_balance,
        Uint128::from(INITIAL_BALANCE) - CREATE_AUCTION_FEE
    );
}
