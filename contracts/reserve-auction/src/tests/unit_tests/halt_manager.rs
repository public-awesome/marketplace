use crate::msg::{QueryMsg, SudoMsg};
use crate::state::{Auction, HaltManager};
use crate::tests::helpers::auction_functions::{create_standard_auction, place_bid};
use crate::tests::helpers::constants::{
    CREATE_AUCTION_FEE, DEFAULT_DURATION, HALT_BUFFER_DURATION, HALT_DURATION_THRESHOLD,
    HALT_POSTPONE_DURATION, MIN_RESERVE_PRICE,
};
use crate::tests::helpers::nft_functions::{approve, mint};
use crate::tests::setup::setup_accounts::{setup_addtl_account, INITIAL_BALANCE};
use crate::tests::setup::setup_fair_burn::setup_fair_burn;
use crate::tests::setup::setup_royalty_registry::setup_royalty_registry;
use crate::tests::setup::{
    setup_auctions::setup_reserve_auction, setup_minters::standard_minter_template,
};
use cosmwasm_std::{coin, Timestamp};
use sg_marketplace_common::query::QueryOptions;
use sg_std::{GENESIS_MINT_START_TIME, NATIVE_DENOM};
use test_suite::common_setup::setup_accounts_and_block::setup_block_time;

#[test]
fn try_halt_detection() {
    let vt = standard_minter_template(1000);
    let (mut router, creator, _) = (vt.router, vt.accts.creator, vt.accts.bidder);
    let fair_burn = setup_fair_burn(&mut router, creator.clone());
    let royalty_registry = setup_royalty_registry(&mut router, creator.clone());

    let genesis_start_timestamp = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    setup_block_time(&mut router, genesis_start_timestamp.nanos(), None);

    // Test that halt manager is instantiated with contract
    let reserve_auction =
        setup_reserve_auction(&mut router, creator, fair_burn, royalty_registry).unwrap();
    let halt_manager: HaltManager = router
        .wrap()
        .query_wasm_smart(reserve_auction.clone(), &QueryMsg::HaltManager {})
        .unwrap();
    assert_eq!(halt_manager.prev_block_time, 0);
    assert!(halt_manager.halt_windows.is_empty());

    // Test that prev block time is recorded
    let six_minutes = 60 * 6;
    let next_block_timestamp = genesis_start_timestamp.plus_seconds(six_minutes);
    setup_block_time(&mut router, next_block_timestamp.nanos(), None);
    let begin_block_msg = SudoMsg::BeginBlock {};
    router
        .wasm_sudo(reserve_auction.clone(), &begin_block_msg)
        .unwrap();
    let halt_manager: HaltManager = router
        .wrap()
        .query_wasm_smart(reserve_auction.clone(), &QueryMsg::HaltManager {})
        .unwrap();
    assert_eq!(halt_manager.prev_block_time, next_block_timestamp.seconds());
    assert!(halt_manager.halt_windows.is_empty());

    // Test that prev block time is updated
    let six_minutes = 60 * 6;
    let next_block_timestamp = next_block_timestamp.plus_seconds(six_minutes);
    setup_block_time(&mut router, next_block_timestamp.nanos(), None);
    let begin_block_msg = SudoMsg::BeginBlock {};
    router
        .wasm_sudo(reserve_auction.clone(), &begin_block_msg)
        .unwrap();
    let halt_manager: HaltManager = router
        .wrap()
        .query_wasm_smart(reserve_auction.clone(), &QueryMsg::HaltManager {})
        .unwrap();
    assert_eq!(halt_manager.prev_block_time, next_block_timestamp.seconds());
    assert!(halt_manager.halt_windows.is_empty(),);

    // Test that halt info is set after downtime threshold
    let next_block_timestamp = next_block_timestamp.plus_seconds(HALT_DURATION_THRESHOLD);
    setup_block_time(&mut router, next_block_timestamp.nanos(), None);
    let begin_block_msg = SudoMsg::BeginBlock {};
    router
        .wasm_sudo(reserve_auction.clone(), &begin_block_msg)
        .unwrap();
    let halt_manager: HaltManager = router
        .wrap()
        .query_wasm_smart(reserve_auction.clone(), &QueryMsg::HaltManager {})
        .unwrap();
    assert_eq!(halt_manager.prev_block_time, next_block_timestamp.seconds());
    assert_eq!(halt_manager.halt_windows.len(), 1);

    // Test that additional halt infos can be set
    let next_block_timestamp = next_block_timestamp.plus_seconds(HALT_DURATION_THRESHOLD);
    setup_block_time(&mut router, next_block_timestamp.nanos(), None);
    let begin_block_msg = SudoMsg::BeginBlock {};
    router
        .wasm_sudo(reserve_auction.clone(), &begin_block_msg)
        .unwrap();
    let halt_manager: HaltManager = router
        .wrap()
        .query_wasm_smart(reserve_auction, &QueryMsg::HaltManager {})
        .unwrap();
    assert_eq!(halt_manager.prev_block_time, next_block_timestamp.seconds());
    assert_eq!(halt_manager.halt_windows.len(), 2);
}

#[test]
fn try_postpone_auction() {
    let vt = standard_minter_template(1000);
    let (mut router, creator, bidder) = (vt.router, vt.accts.creator, vt.accts.bidder);
    let fair_burn = setup_fair_burn(&mut router, creator.clone());
    let royalty_registry = setup_royalty_registry(&mut router, creator.clone());
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let minter = vt.collection_response_vec[0].minter.clone().unwrap();

    let auction_creator =
        setup_addtl_account(&mut router, "auction_creator", INITIAL_BALANCE).unwrap();

    let genesis_timestamp = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    let next_block_timestamp = genesis_timestamp.plus_seconds(DEFAULT_DURATION);
    setup_block_time(&mut router, next_block_timestamp.nanos(), None);

    let reserve_auction =
        setup_reserve_auction(&mut router, creator.clone(), fair_burn, royalty_registry).unwrap();

    /*
       Halt Window
       start_time: Genesis + DEFAULT_DURATION (1 hour)
       end_time: Genesis + DEFAULT_DURATION (1 hour) + HALT_DURATION_THRESHOLD (20 min)
    */

    // Auctions that end before halt window
    // End time: Genesis + DEFAULT_DURATION (1 hour)
    let mut end_before_halt_token_ids: Vec<u32> = vec![];
    setup_block_time(&mut router, genesis_timestamp.nanos(), None);
    for _ in 0..3 {
        let token_id = mint(&mut router, &minter, &creator, &auction_creator);
        end_before_halt_token_ids.push(token_id);
        approve(
            &mut router,
            &auction_creator,
            &collection,
            &reserve_auction,
            token_id,
        );
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

    // Auctions that end during halt window
    // End time: Genesis + DEFAULT_DURATION (1 hour) + 1 second
    let next_block_timestamp: Timestamp = genesis_timestamp.plus_seconds(1);
    let mut end_during_halt_token_ids: Vec<u32> = vec![];
    setup_block_time(&mut router, next_block_timestamp.nanos(), None);
    for _ in 0..3 {
        let token_id = mint(&mut router, &minter, &creator, &auction_creator);
        end_during_halt_token_ids.push(token_id);
        approve(
            &mut router,
            &auction_creator,
            &collection,
            &reserve_auction,
            token_id,
        );
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

    // Auctions that end during halt window buffer
    // End time: Genesis + DEFAULT_DURATION (1 hour) + HALT_DURATION_THRESHOLD (20 min) + 1 second
    let next_block_timestamp = genesis_timestamp.plus_seconds(1 + HALT_DURATION_THRESHOLD);
    setup_block_time(&mut router, next_block_timestamp.nanos(), None);
    let mut end_during_halt_buffer_token_ids: Vec<u32> = vec![];
    for _ in 0..3 {
        let token_id = mint(&mut router, &minter, &creator, &auction_creator);
        end_during_halt_buffer_token_ids.push(token_id);
        approve(
            &mut router,
            &auction_creator,
            &collection,
            &reserve_auction,
            token_id,
        );
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

    // Auctions that end after halt window
    // End time: Genesis + DEFAULT_DURATION (1 hour) + HALT_DURATION_THRESHOLD (20 min) + HALT_BUFFER_DURATION (30 min)
    let next_block_timestamp =
        genesis_timestamp.plus_seconds(HALT_DURATION_THRESHOLD + HALT_BUFFER_DURATION);
    setup_block_time(&mut router, next_block_timestamp.nanos(), None);
    let mut end_after_halt_token_ids: Vec<u32> = vec![];
    for _ in 0..3 {
        let token_id = mint(&mut router, &minter, &creator, &auction_creator);
        end_after_halt_token_ids.push(token_id);
        approve(
            &mut router,
            &auction_creator,
            &collection,
            &reserve_auction,
            token_id,
        );
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

    // Fetch auctions
    let auctions: Vec<Auction> = router
        .wrap()
        .query_wasm_smart(
            reserve_auction.clone(),
            &QueryMsg::AuctionsByEndTime {
                end_time: 0u64,
                query_options: Some(QueryOptions {
                    descending: Some(false),
                    limit: Some(100),
                    start_after: None,
                }),
            },
        )
        .unwrap();
    assert_eq!(auctions.len(), 12);

    // Run block before halt, validate that 3 auctions are settled
    let mut next_block_timestamp = genesis_timestamp.plus_seconds(DEFAULT_DURATION);
    setup_block_time(&mut router, next_block_timestamp.nanos(), None);
    router
        .wasm_sudo(reserve_auction.clone(), &SudoMsg::BeginBlock {})
        .unwrap();
    router
        .wasm_sudo(reserve_auction.clone(), &SudoMsg::EndBlock {})
        .unwrap();

    let auctions: Vec<Auction> = router
        .wrap()
        .query_wasm_smart(
            reserve_auction.clone(),
            &QueryMsg::AuctionsByEndTime {
                end_time: 0u64,
                query_options: Some(QueryOptions {
                    descending: Some(false),
                    limit: Some(100),
                    start_after: None,
                }),
            },
        )
        .unwrap();
    assert_eq!(auctions.len(), 9);

    // Run block that defines halt
    // Validate that halt defining block postpones auctions
    next_block_timestamp = next_block_timestamp.plus_seconds(HALT_DURATION_THRESHOLD);
    setup_block_time(&mut router, next_block_timestamp.nanos(), None);
    router
        .wasm_sudo(reserve_auction.clone(), &SudoMsg::BeginBlock {})
        .unwrap();
    let halt_manager: HaltManager = router
        .wrap()
        .query_wasm_smart(reserve_auction.clone(), &QueryMsg::HaltManager {})
        .unwrap();
    assert_eq!(halt_manager.halt_windows.len(), 1);
    router
        .wasm_sudo(reserve_auction.clone(), &SudoMsg::EndBlock {})
        .unwrap();

    for token_id in end_during_halt_token_ids {
        let auction = router
            .wrap()
            .query_wasm_smart::<Option<Auction>>(
                reserve_auction.clone(),
                &QueryMsg::Auction {
                    collection: collection.to_string(),
                    token_id: token_id.to_string(),
                },
            )
            .unwrap()
            .unwrap();
        assert_eq!(
            next_block_timestamp
                .plus_seconds(HALT_POSTPONE_DURATION)
                .seconds(),
            auction.end_time.unwrap().seconds()
        );
    }

    // Run next block that is with the halt window buffer
    // Validate that auctions ending within buffer are postponed
    let six_minutes = 60 * 6;
    next_block_timestamp = next_block_timestamp.plus_seconds(six_minutes);
    setup_block_time(&mut router, next_block_timestamp.nanos(), None);
    router
        .wasm_sudo(reserve_auction.clone(), &SudoMsg::BeginBlock {})
        .unwrap();
    let halt_manager: HaltManager = router
        .wrap()
        .query_wasm_smart(reserve_auction.clone(), &QueryMsg::HaltManager {})
        .unwrap();
    assert_eq!(halt_manager.halt_windows.len(), 1);
    router
        .wasm_sudo(reserve_auction.clone(), &SudoMsg::EndBlock {})
        .unwrap();

    for token_id in end_during_halt_buffer_token_ids {
        let auction = router
            .wrap()
            .query_wasm_smart::<Option<Auction>>(
                reserve_auction.clone(),
                &QueryMsg::Auction {
                    collection: collection.to_string(),
                    token_id: token_id.to_string(),
                },
            )
            .unwrap()
            .unwrap();
        assert_eq!(
            next_block_timestamp
                .plus_seconds(HALT_POSTPONE_DURATION)
                .seconds(),
            auction.end_time.unwrap().seconds()
        );
    }

    // Run a few more blocks, to prevent halts
    for _ in 0..4 {
        next_block_timestamp = next_block_timestamp.plus_seconds(six_minutes);
        setup_block_time(&mut router, next_block_timestamp.nanos(), None);
        router
            .wasm_sudo(reserve_auction.clone(), &SudoMsg::BeginBlock {})
            .unwrap();
    }

    // Run block outside of halt window
    // Validate that auctions ending outside of window are settled normally
    next_block_timestamp = genesis_timestamp
        .plus_seconds(DEFAULT_DURATION + HALT_DURATION_THRESHOLD + HALT_BUFFER_DURATION);
    setup_block_time(&mut router, next_block_timestamp.nanos(), None);
    router
        .wasm_sudo(reserve_auction.clone(), &SudoMsg::BeginBlock {})
        .unwrap();
    let halt_manager: HaltManager = router
        .wrap()
        .query_wasm_smart(reserve_auction.clone(), &QueryMsg::HaltManager {})
        .unwrap();
    assert_eq!(halt_manager.halt_windows.len(), 1);
    router
        .wasm_sudo(reserve_auction.clone(), &SudoMsg::EndBlock {})
        .unwrap();

    for token_id in end_after_halt_token_ids {
        let auction_option = router
            .wrap()
            .query_wasm_smart::<Option<Auction>>(
                reserve_auction.clone(),
                &QueryMsg::Auction {
                    collection: collection.to_string(),
                    token_id: token_id.to_string(),
                },
            )
            .unwrap();
        assert!(auction_option.is_none());
    }

    // Validate that halt infos are cleared
    next_block_timestamp = next_block_timestamp.plus_seconds(1);
    setup_block_time(&mut router, next_block_timestamp.nanos(), None);
    router
        .wasm_sudo(reserve_auction.clone(), &SudoMsg::BeginBlock {})
        .unwrap();
    let halt_manager: HaltManager = router
        .wrap()
        .query_wasm_smart(reserve_auction.clone(), &QueryMsg::HaltManager {})
        .unwrap();
    assert_eq!(halt_manager.halt_windows.len(), 1);
    router
        .wasm_sudo(reserve_auction.clone(), &SudoMsg::EndBlock {})
        .unwrap();
    let halt_manager: HaltManager = router
        .wrap()
        .query_wasm_smart(reserve_auction.clone(), &QueryMsg::HaltManager {})
        .unwrap();
    assert_eq!(halt_manager.halt_windows.len(), 0);
}
