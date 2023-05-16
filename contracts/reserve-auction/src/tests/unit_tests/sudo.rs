use crate::msg::{AuctionsResponse, ConfigResponse, QueryMsg, QueryOptions, SudoMsg};
use crate::tests::helpers::auction_functions::place_bid;
use crate::tests::helpers::constants::{
    CREATE_AUCTION_FEE, DEFAULT_DURATION, MAX_AUCTIONS_TO_SETTLE_PER_BLOCK, MIN_BID_INCREMENT_BPS,
    MIN_DURATION, MIN_RESERVE_PRICE,
};
use crate::tests::setup::setup_accounts::{setup_addtl_account, INITIAL_BALANCE};
use crate::tests::{
    helpers::{
        auction_functions::create_standard_auction,
        nft_functions::{approve, mint, query_owner_of},
    },
    setup::{
        setup_auctions::setup_reserve_auction,
        setup_marketplace::{setup_marketplace, TRADING_FEE_BPS},
        setup_minters::standard_minter_template,
    },
};
use cosmwasm_std::{Decimal, Uint128};
use sg721_base::msg::{CollectionInfoResponse, QueryMsg as Sg721QueryMsg};
use sg_std::{GENESIS_MINT_START_TIME, NATIVE_DENOM};
use test_suite::common_setup::setup_accounts_and_block::setup_block_time;

#[test]
fn try_sudo_begin_block() {
    let vt = standard_minter_template(1000);
    let (mut router, creator, _) = (vt.router, vt.accts.creator, vt.accts.bidder);
    let marketplace = setup_marketplace(&mut router, creator.clone()).unwrap();
    let reserve_auction = setup_reserve_auction(&mut router, creator, marketplace).unwrap();

    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);

    // Test begin block no-op
    let begin_block_msg = SudoMsg::BeginBlock {};
    let response = router.wasm_sudo(reserve_auction, &begin_block_msg);
    assert!(response.is_ok());

    response
        .unwrap()
        .events
        .iter()
        .find(|&e| e.ty == "wasm-sudo-begin-block")
        .unwrap();
}

#[test]
fn try_sudo_end_block() {
    let vt = standard_minter_template(1000);
    let (mut router, creator, bidder) = (vt.router, vt.accts.creator, vt.accts.bidder);
    let marketplace = setup_marketplace(&mut router, creator.clone()).unwrap();
    let reserve_auction = setup_reserve_auction(&mut router, creator.clone(), marketplace).unwrap();
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
            MIN_RESERVE_PRICE,
            DEFAULT_DURATION,
            None,
            CREATE_AUCTION_FEE.u128(),
        )
        .unwrap();
        place_bid(
            &mut router,
            &reserve_auction,
            &bidder,
            collection.as_ref(),
            &token_id.to_string(),
            MIN_RESERVE_PRICE,
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

    let response_1: AuctionsResponse = router
        .wrap()
        .query_wasm_smart(
            reserve_auction.clone(),
            &QueryMsg::AuctionsByEndTime {
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(response_1.auctions.len(), num_auctions as usize);

    // Test end block removes auctions with bids that have ended
    let num_remove_auctions: u64 = 9;
    let new_block_time = block_time
        .plus_seconds(num_remove_auctions - 1)
        .plus_seconds(DEFAULT_DURATION);

    setup_block_time(&mut router, new_block_time.nanos(), None);
    let response = router.wasm_sudo(reserve_auction.clone(), &end_block_msg);
    assert!(response.is_ok());

    let response_2: AuctionsResponse = router
        .wrap()
        .query_wasm_smart(
            reserve_auction.clone(),
            &QueryMsg::AuctionsByEndTime {
                query_options: Some(QueryOptions {
                    limit: None,
                    start_after: Some((block_time.seconds(), "".to_string(), "".to_string())),
                    descending: None,
                }),
            },
        )
        .unwrap();
    assert_eq!(
        response_2.auctions.len(),
        num_auctions as usize - num_remove_auctions as usize
    );

    // Test end block removes last auction with bid that has ended
    let new_block_time = block_time
        .plus_seconds(num_auctions - 1)
        .plus_seconds(DEFAULT_DURATION);

    setup_block_time(&mut router, new_block_time.nanos(), None);
    let sudo_response = router.wasm_sudo(reserve_auction.clone(), &end_block_msg);
    assert!(sudo_response.is_ok());

    let response_3: AuctionsResponse = router
        .wrap()
        .query_wasm_smart(
            reserve_auction.clone(),
            &QueryMsg::AuctionsByEndTime {
                query_options: Some(QueryOptions {
                    limit: None,
                    start_after: Some((block_time.seconds(), "".to_string(), "".to_string())),
                    descending: None,
                }),
            },
        )
        .unwrap();
    assert_eq!(response_3.auctions.len(), 0);

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

    let trading_fee_percent = Decimal::percent(TRADING_FEE_BPS) / Uint128::from(100u128);
    let protocol_fee = Uint128::from(burn_amount + dist_amount);
    assert_eq!(
        Uint128::from(MIN_RESERVE_PRICE) * trading_fee_percent,
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
    let marketplace = setup_marketplace(&mut router, creator.clone()).unwrap();
    let reserve_auction = setup_reserve_auction(&mut router, creator, marketplace).unwrap();
    let minter = vt.collection_response_vec[0].minter.clone().unwrap();

    let delta: u64 = 1;
    let update_params_msg = SudoMsg::UpdateParams {
        marketplace: Some(minter.to_string()),
        min_reserve_price: Some(Uint128::from(MIN_RESERVE_PRICE + delta as u128)),
        min_duration: Some(MIN_DURATION + delta),
        min_bid_increment_bps: Some(MIN_BID_INCREMENT_BPS + delta),
        extend_duration: Some(MIN_DURATION + delta),
        create_auction_fee: Some(CREATE_AUCTION_FEE + Uint128::from(delta)),
        max_auctions_to_settle_per_block: Some(MAX_AUCTIONS_TO_SETTLE_PER_BLOCK + delta),
    };
    let response = router.wasm_sudo(reserve_auction.clone(), &update_params_msg);

    response
        .unwrap()
        .events
        .iter()
        .find(|&e| e.ty == "wasm-sudo-update-params")
        .unwrap();

    let response: ConfigResponse = router
        .wrap()
        .query_wasm_smart(reserve_auction.clone(), &QueryMsg::Config {})
        .unwrap();
    let config = response.config;

    assert_eq!(config.marketplace, minter);
    assert_eq!(
        config.min_reserve_price,
        Uint128::from(MIN_RESERVE_PRICE + delta as u128)
    );
    assert_eq!(config.min_duration, MIN_DURATION + delta);
    assert_eq!(
        config.min_bid_increment_pct,
        Decimal::percent(MIN_BID_INCREMENT_BPS + delta)
    );
    assert_eq!(config.extend_duration, MIN_DURATION + delta);
    assert_eq!(
        config.create_auction_fee,
        CREATE_AUCTION_FEE + Uint128::from(delta)
    );
    assert_eq!(
        config.max_auctions_to_settle_per_block,
        MAX_AUCTIONS_TO_SETTLE_PER_BLOCK + delta
    );

    let update_params_msg = SudoMsg::UpdateParams {
        marketplace: None,
        min_reserve_price: Some(Uint128::from(0u128)),
        min_duration: None,
        min_bid_increment_bps: None,
        extend_duration: None,
        create_auction_fee: None,
        max_auctions_to_settle_per_block: None,
    };
    let response = router.wasm_sudo(reserve_auction.clone(), &update_params_msg);
    assert_eq!(
        response.unwrap_err().to_string(),
        "InvalidConfig: min_reserve_price must be greater than zero"
    );

    let update_params_msg = SudoMsg::UpdateParams {
        marketplace: None,
        min_reserve_price: None,
        min_duration: Some(0u64),
        min_bid_increment_bps: None,
        extend_duration: None,
        create_auction_fee: None,
        max_auctions_to_settle_per_block: None,
    };
    let response = router.wasm_sudo(reserve_auction.clone(), &update_params_msg);
    assert_eq!(
        response.unwrap_err().to_string(),
        "InvalidConfig: min_duration must be greater than zero"
    );

    let update_params_msg = SudoMsg::UpdateParams {
        marketplace: None,
        min_reserve_price: None,
        min_duration: None,
        min_bid_increment_bps: Some(0u64),
        extend_duration: None,
        create_auction_fee: None,
        max_auctions_to_settle_per_block: None,
    };
    let response = router.wasm_sudo(reserve_auction.clone(), &update_params_msg);
    assert_eq!(
        response.unwrap_err().to_string(),
        "InvalidConfig: min_bid_increment_pct must be greater than zero"
    );

    let update_params_msg = SudoMsg::UpdateParams {
        marketplace: None,
        min_reserve_price: None,
        min_duration: None,
        min_bid_increment_bps: Some(10000u64),
        extend_duration: None,
        create_auction_fee: None,
        max_auctions_to_settle_per_block: None,
    };
    let response = router.wasm_sudo(reserve_auction.clone(), &update_params_msg);
    assert_eq!(
        response.unwrap_err().to_string(),
        "InvalidConfig: min_bid_increment_pct must be less than 100%"
    );

    let update_params_msg = SudoMsg::UpdateParams {
        marketplace: None,
        min_reserve_price: None,
        min_duration: None,
        min_bid_increment_bps: None,
        extend_duration: Some(0u64),
        create_auction_fee: None,
        max_auctions_to_settle_per_block: None,
    };
    let response = router.wasm_sudo(reserve_auction, &update_params_msg);
    assert_eq!(
        response.unwrap_err().to_string(),
        "InvalidConfig: extend_duration must be greater than zero"
    );
}
