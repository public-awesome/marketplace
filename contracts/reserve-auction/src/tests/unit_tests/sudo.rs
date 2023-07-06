use std::str::FromStr;

use crate::msg::{QueryMsg, SudoMsg};
use crate::state::{Auction, Config};
use crate::tests::helpers::auction_functions::place_bid;
use crate::tests::helpers::constants::{
    CREATE_AUCTION_FEE, DEFAULT_DURATION, HALT_BUFFER_DURATION, HALT_DURATION_THRESHOLD,
    HALT_POSTPONE_DURATION, MAX_AUCTIONS_TO_SETTLE_PER_BLOCK, MIN_BID_INCREMENT_PCT, MIN_DURATION,
    MIN_RESERVE_PRICE, TRADING_FEE_PCT,
};
use crate::tests::setup::setup_accounts::{setup_addtl_account, INITIAL_BALANCE};
use crate::tests::setup::setup_fair_burn::setup_fair_burn;
use crate::tests::{
    helpers::{
        auction_functions::create_standard_auction,
        nft_functions::{approve, mint, query_owner_of},
    },
    setup::{setup_auctions::setup_reserve_auction, setup_minters::standard_minter_template},
};

use cosmwasm_std::{coin, Coin, Decimal, Uint128};
use sg721_base::msg::{CollectionInfoResponse, QueryMsg as Sg721QueryMsg};
use sg_marketplace_common::coin::bps_to_decimal;
use sg_marketplace_common::query::QueryOptions;
use sg_std::{GENESIS_MINT_START_TIME, NATIVE_DENOM};
use test_suite::common_setup::setup_accounts_and_block::setup_block_time;

#[test]
fn try_sudo_begin_block() {
    let vt = standard_minter_template(1000);
    let (mut router, creator, _) = (vt.router, vt.accts.creator, vt.accts.bidder);
    let fair_burn = setup_fair_burn(&mut router, creator.clone());
    let reserve_auction = setup_reserve_auction(&mut router, creator, fair_burn).unwrap();

    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);

    let begin_block_msg = SudoMsg::BeginBlock {};
    let response = router.wasm_sudo(reserve_auction, &begin_block_msg);
    assert!(response.is_ok());
}

#[test]
fn try_sudo_end_block() {
    let vt = standard_minter_template(1000);
    let (mut router, creator, bidder) = (vt.router, vt.accts.creator, vt.accts.bidder);
    let fair_burn = setup_fair_burn(&mut router, creator.clone());
    let reserve_auction = setup_reserve_auction(&mut router, creator.clone(), fair_burn).unwrap();
    let minter = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();

    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);
    let block_time = router.block_info().time;

    let auction_creator =
        setup_addtl_account(&mut router, "auction_creator", INITIAL_BALANCE).unwrap();

    let num_auctions = 10;

    let mut token_ids: Vec<u32> = vec![];
    for idx in 0..num_auctions {
        let token_id = mint(&mut router, &minter, &creator, &auction_creator);
        approve(
            &mut router,
            &auction_creator,
            &collection,
            &reserve_auction,
            token_id,
        );
        token_ids.push(token_id);

        setup_block_time(&mut router, block_time.plus_seconds(idx).nanos(), None);
        create_standard_auction(
            &mut router,
            &auction_creator,
            &reserve_auction,
            collection.as_ref(),
            &token_id.to_string(),
            coin(MIN_RESERVE_PRICE, NATIVE_DENOM),
            DEFAULT_DURATION,
            None,
            coin(CREATE_AUCTION_FEE.u128(), NATIVE_DENOM),
        )
        .unwrap();
        place_bid(
            &mut router,
            &reserve_auction,
            &bidder,
            collection.as_ref(),
            &token_id.to_string(),
            coin(MIN_RESERVE_PRICE, NATIVE_DENOM),
        )
        .unwrap();
    }

    // Test end block no-op when no auctions have ended
    let end_block_msg = SudoMsg::EndBlock {};
    let response = router.wasm_sudo(reserve_auction.clone(), &end_block_msg);
    response
        .unwrap()
        .events
        .iter()
        .find(|&e| e.ty == "wasm-sudo-end-block")
        .unwrap();

    let auctions_1: Vec<Auction> = router
        .wrap()
        .query_wasm_smart(
            reserve_auction.clone(),
            &QueryMsg::AuctionsByEndTime {
                end_time: 0u64,
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(auctions_1.len(), num_auctions as usize);

    // Test end block removes auctions with bids that have ended
    let num_remove_auctions: u64 = 9;
    let new_block_time = block_time
        .plus_seconds(num_remove_auctions - 1)
        .plus_seconds(DEFAULT_DURATION);

    setup_block_time(&mut router, new_block_time.nanos(), None);
    let response = router.wasm_sudo(reserve_auction.clone(), &end_block_msg);
    assert!(response.is_ok());

    let auctions_2: Vec<Auction> = router
        .wrap()
        .query_wasm_smart(
            reserve_auction.clone(),
            &QueryMsg::AuctionsByEndTime {
                end_time: block_time.seconds(),
                query_options: Some(QueryOptions {
                    limit: None,
                    start_after: None,
                    descending: None,
                }),
            },
        )
        .unwrap();
    assert_eq!(
        auctions_2.len(),
        num_auctions as usize - num_remove_auctions as usize
    );

    // Test end block removes last auction with bid that has ended
    let new_block_time = block_time
        .plus_seconds(num_auctions - 1)
        .plus_seconds(DEFAULT_DURATION);

    setup_block_time(&mut router, new_block_time.nanos(), None);
    let sudo_response = router.wasm_sudo(reserve_auction.clone(), &end_block_msg);
    assert!(sudo_response.is_ok());

    let auctions_3: Vec<Auction> = router
        .wrap()
        .query_wasm_smart(
            reserve_auction.clone(),
            &QueryMsg::AuctionsByEndTime {
                end_time: block_time.seconds(),
                query_options: Some(QueryOptions {
                    limit: None,
                    start_after: None,
                    descending: None,
                }),
            },
        )
        .unwrap();
    assert_eq!(auctions_3.len(), 0);

    // check that fair burn was paid
    let fair_burn_event = sudo_response
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
    assert_eq!(
        Uint128::from(MIN_RESERVE_PRICE) * Decimal::from_str(TRADING_FEE_PCT).unwrap(),
        protocol_fee
    );

    // check that royalty was paid
    let collection_info: CollectionInfoResponse = router
        .wrap()
        .query_wasm_smart(collection.clone(), &Sg721QueryMsg::CollectionInfo {})
        .unwrap();
    let royalty_share = collection_info.royalty_info.unwrap().share;
    let royalty_fee =
        Uint128::from(MIN_RESERVE_PRICE) * royalty_share * Uint128::from(num_auctions);

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
    let seller_payment = (Uint128::from(MIN_RESERVE_PRICE) - protocol_fee)
        * Uint128::from(num_auctions)
        - royalty_fee;
    let new_auction_creator_balance = router
        .wrap()
        .query_balance(&auction_creator, NATIVE_DENOM)
        .unwrap()
        .amount;
    assert_eq!(
        new_auction_creator_balance,
        Uint128::from(INITIAL_BALANCE) - (CREATE_AUCTION_FEE * Uint128::from(num_auctions))
            + seller_payment
    );

    // check that bidder was debited tokens and was given NFT
    let bidder_balance = router
        .wrap()
        .query_balance(&bidder, NATIVE_DENOM)
        .unwrap()
        .amount;
    assert_eq!(
        bidder_balance,
        Uint128::from(INITIAL_BALANCE - (MIN_RESERVE_PRICE * num_auctions as u128))
    );
    assert_eq!(
        query_owner_of(&router, &collection, &token_ids.last().unwrap().to_string()),
        bidder.to_string()
    );
}

#[test]
fn try_sudo_update_params() {
    let vt = standard_minter_template(1000);
    let (mut router, creator, _bidder) = (vt.router, vt.accts.creator, vt.accts.bidder);
    let fair_burn = setup_fair_burn(&mut router, creator.clone());
    let reserve_auction = setup_reserve_auction(&mut router, creator, fair_burn).unwrap();
    let minter = vt.collection_response_vec[0].minter.clone().unwrap();

    let delta: u64 = 1;
    let delta_decimal = bps_to_decimal(delta);
    let update_params_msg = SudoMsg::UpdateParams {
        fair_burn: Some(minter.to_string()),
        trading_fee_percent: Some(Decimal::from_str(TRADING_FEE_PCT).unwrap() + delta_decimal),
        min_duration: Some(MIN_DURATION + delta),
        min_bid_increment_percent: Some(
            Decimal::from_str(MIN_BID_INCREMENT_PCT).unwrap() + delta_decimal,
        ),
        extend_duration: Some(MIN_DURATION + delta),
        create_auction_fee: Some(coin(
            CREATE_AUCTION_FEE.u128() + Uint128::from(delta).u128(),
            NATIVE_DENOM,
        )),
        max_auctions_to_settle_per_block: Some(MAX_AUCTIONS_TO_SETTLE_PER_BLOCK + delta),
        halt_duration_threshold: Some(HALT_DURATION_THRESHOLD + delta),
        halt_buffer_duration: Some(HALT_BUFFER_DURATION + delta),
        halt_postpone_duration: Some(HALT_POSTPONE_DURATION + delta),
    };
    let response = router.wasm_sudo(reserve_auction.clone(), &update_params_msg);

    response
        .unwrap()
        .events
        .iter()
        .find(|&e| e.ty == "wasm-sudo-update-params")
        .unwrap();

    let config: Config = router
        .wrap()
        .query_wasm_smart(reserve_auction.clone(), &QueryMsg::Config {})
        .unwrap();

    assert_eq!(config.fair_burn, minter);
    assert_eq!(
        config.trading_fee_percent,
        Decimal::from_str(TRADING_FEE_PCT).unwrap() + delta_decimal
    );
    assert_eq!(config.min_duration, MIN_DURATION + delta);
    assert_eq!(
        config.min_bid_increment_percent,
        Decimal::from_str(MIN_BID_INCREMENT_PCT).unwrap() + delta_decimal
    );
    assert_eq!(config.extend_duration, MIN_DURATION + delta);
    assert_eq!(
        config.create_auction_fee,
        coin(
            CREATE_AUCTION_FEE.u128() + Uint128::from(delta).u128(),
            NATIVE_DENOM
        )
    );
    assert_eq!(
        config.max_auctions_to_settle_per_block,
        MAX_AUCTIONS_TO_SETTLE_PER_BLOCK + delta
    );
    assert_eq!(
        config.halt_duration_threshold,
        HALT_DURATION_THRESHOLD + delta
    );
    assert_eq!(config.halt_buffer_duration, HALT_BUFFER_DURATION + delta);
    assert_eq!(
        config.halt_postpone_duration,
        HALT_POSTPONE_DURATION + delta
    );

    let update_params_msg = SudoMsg::UpdateParams {
        fair_burn: None,
        trading_fee_percent: None,
        min_duration: Some(0u64),
        min_bid_increment_percent: None,
        extend_duration: None,
        create_auction_fee: None,
        max_auctions_to_settle_per_block: None,
        halt_duration_threshold: None,
        halt_buffer_duration: None,
        halt_postpone_duration: None,
    };
    let response = router.wasm_sudo(reserve_auction.clone(), &update_params_msg);
    assert_eq!(
        response.unwrap_err().to_string(),
        "InvalidConfig: min_duration must be greater than zero"
    );

    let update_params_msg = SudoMsg::UpdateParams {
        fair_burn: None,
        trading_fee_percent: None,
        min_duration: None,
        min_bid_increment_percent: Some(Decimal::zero()),
        extend_duration: None,
        create_auction_fee: None,
        max_auctions_to_settle_per_block: None,
        halt_duration_threshold: None,
        halt_buffer_duration: None,
        halt_postpone_duration: None,
    };
    let response = router.wasm_sudo(reserve_auction.clone(), &update_params_msg);
    assert_eq!(
        response.unwrap_err().to_string(),
        "InvalidConfig: min_bid_increment_percent must be greater than zero"
    );

    let update_params_msg = SudoMsg::UpdateParams {
        fair_burn: None,
        trading_fee_percent: None,
        min_duration: None,
        min_bid_increment_percent: Some(Decimal::one()),
        extend_duration: None,
        create_auction_fee: None,
        max_auctions_to_settle_per_block: None,
        halt_duration_threshold: None,
        halt_buffer_duration: None,
        halt_postpone_duration: None,
    };
    let response = router.wasm_sudo(reserve_auction.clone(), &update_params_msg);
    assert_eq!(
        response.unwrap_err().to_string(),
        "InvalidConfig: min_bid_increment_percent must be less than 100%"
    );

    let update_params_msg = SudoMsg::UpdateParams {
        fair_burn: None,
        trading_fee_percent: None,
        min_duration: None,
        min_bid_increment_percent: None,
        extend_duration: Some(0u64),
        create_auction_fee: None,
        max_auctions_to_settle_per_block: None,
        halt_duration_threshold: None,
        halt_buffer_duration: None,
        halt_postpone_duration: None,
    };
    let response = router.wasm_sudo(reserve_auction, &update_params_msg);
    assert_eq!(
        response.unwrap_err().to_string(),
        "InvalidConfig: extend_duration must be greater than zero"
    );
}

#[test]
fn try_sudo_min_reserve_prices() {
    let vt = standard_minter_template(1000);
    let (mut router, creator, _) = (vt.router, vt.accts.creator, vt.accts.bidder);
    let fair_burn = setup_fair_burn(&mut router, creator.clone());
    let reserve_auction = setup_reserve_auction(&mut router, creator, fair_burn).unwrap();

    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);

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
    let set_min_reserve_prices_msg = SudoMsg::SetMinReservePrices {
        min_reserve_prices: vec![coins_response[0].clone()],
    };
    let response = router.wasm_sudo(reserve_auction.clone(), &set_min_reserve_prices_msg);
    assert_eq!(
        response.unwrap_err().to_string(),
        "InvalidInput: found duplicate denom"
    );

    // New denoms can be added
    let new_coins = vec![coin(3000000, "uosmo"), coin(4000000, "ujuno")];
    let set_min_reserve_prices_msg = SudoMsg::SetMinReservePrices {
        min_reserve_prices: new_coins.clone(),
    };
    let response = router.wasm_sudo(reserve_auction.clone(), &set_min_reserve_prices_msg);
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
    let unset_min_reserve_prices_msg = SudoMsg::UnsetMinReservePrices {
        denoms: vec!["uusd".to_string()],
    };
    let response = router.wasm_sudo(reserve_auction.clone(), &unset_min_reserve_prices_msg);
    assert_eq!(
        response.unwrap_err().to_string(),
        "InvalidInput: denom not found"
    );

    // Removing existent denoms is ok
    let remove_coins = vec!["uosmo".to_string(), "ujuno".to_string()];
    let unset_min_reserve_prices_msg = SudoMsg::UnsetMinReservePrices {
        denoms: remove_coins,
    };
    let response = router.wasm_sudo(reserve_auction.clone(), &unset_min_reserve_prices_msg);
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
