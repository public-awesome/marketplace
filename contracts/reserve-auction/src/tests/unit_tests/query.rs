use crate::msg::{AuctionsResponse, QueryMsg, QueryOptions};
use crate::tests::helpers::constants::{CREATE_AUCTION_FEE, MIN_DURATION, MIN_RESERVE_PRICE};
use crate::tests::setup::setup_accounts::{setup_addtl_account, INITIAL_BALANCE};
use crate::tests::{
    helpers::{
        auction_functions::create_standard_auction,
        nft_functions::{approve, mint},
    },
    setup::{
        setup_auctions::setup_reserve_auction, setup_marketplace::setup_marketplace,
        setup_minters::standard_minter_template,
    },
};
use sg_std::GENESIS_MINT_START_TIME;
use test_suite::common_setup::setup_accounts_and_block::setup_block_time;

#[test]
fn try_query_auctions_by_seller() {
    let vt = standard_minter_template(1000);
    let (mut router, creator, _) = (vt.router, vt.accts.creator, vt.accts.bidder);
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
        let current_creator = if idx % 2 == 0 {
            &auction_creator
        } else {
            &creator
        };

        let token_id = mint(&mut router, &minter, &creator, &current_creator);
        approve(
            &mut router,
            &current_creator,
            &collection,
            &reserve_auction,
            token_id,
        );
        token_ids.push(token_id);

        create_standard_auction(
            &mut router,
            &current_creator,
            &reserve_auction,
            collection.as_ref(),
            &token_id.to_string(),
            block_time,
            block_time.plus_seconds(MIN_DURATION),
            MIN_RESERVE_PRICE,
            None,
            CREATE_AUCTION_FEE.u128(),
        )
        .unwrap();
    }

    let response_1: AuctionsResponse = router
        .wrap()
        .query_wasm_smart(
            reserve_auction.clone(),
            &QueryMsg::AuctionsBySeller {
                seller: auction_creator.to_string(),
                query_options: None,
            },
        )
        .unwrap();

    assert_eq!(response_1.auctions.len(), num_auctions / 2);
    for auction in &response_1.auctions {
        assert_eq!(auction.seller, auction_creator.to_string());
    }

    let limit: u32 = 3;
    let start_after_auction = &response_1.auctions[3].clone();
    let start_after = (
        start_after_auction.collection.to_string(),
        start_after_auction.token_id.clone(),
    );
    let response_2: AuctionsResponse = router
        .wrap()
        .query_wasm_smart(
            reserve_auction,
            &QueryMsg::AuctionsBySeller {
                seller: auction_creator.to_string(),
                query_options: Some(QueryOptions {
                    descending: Some(true),
                    limit: Some(limit),
                    start_after: Some(start_after),
                }),
            },
        )
        .unwrap();

    assert_eq!(response_2.auctions.len(), limit as usize);
    assert_eq!(response_2.auctions[0], response_1.auctions[2]);
    assert_eq!(response_2.auctions[1], response_1.auctions[1]);
    assert_eq!(response_2.auctions[2], response_1.auctions[0]);
}

#[test]
fn try_query_auctions_by_end_time() {
    let vt = standard_minter_template(1000);
    let (mut router, creator, _) = (vt.router, vt.accts.creator, vt.accts.bidder);
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

        let start_time = block_time.plus_seconds(idx);
        create_standard_auction(
            &mut router,
            &auction_creator,
            &reserve_auction,
            collection.as_ref(),
            &token_id.to_string(),
            start_time,
            start_time.plus_seconds(MIN_DURATION),
            MIN_RESERVE_PRICE,
            None,
            CREATE_AUCTION_FEE.u128(),
        )
        .unwrap();
    }

    let response_1: AuctionsResponse = router
        .wrap()
        .query_wasm_smart(
            reserve_auction.clone(),
            &QueryMsg::AuctionsByEndTime {
                end_time: block_time.plus_seconds(MIN_DURATION),
                query_options: None,
            },
        )
        .unwrap();

    assert_eq!(response_1.auctions.len(), num_auctions as usize);

    let skip_num: u64 = 6;
    let response_2: AuctionsResponse = router
        .wrap()
        .query_wasm_smart(
            reserve_auction.clone(),
            &QueryMsg::AuctionsByEndTime {
                end_time: block_time.plus_seconds(skip_num).plus_seconds(MIN_DURATION),
                query_options: Some(QueryOptions {
                    descending: None,
                    limit: None,
                    start_after: None,
                }),
            },
        )
        .unwrap();

    assert_eq!(
        response_2.auctions.len(),
        num_auctions as usize - skip_num as usize
    );

    let limit: u32 = 3;
    let start_after_auction = &response_1.auctions[3].clone();
    let start_after = (
        start_after_auction.end_time.seconds(),
        (
            start_after_auction.collection.to_string(),
            start_after_auction.token_id.clone(),
        ),
    );
    let response_3: AuctionsResponse = router
        .wrap()
        .query_wasm_smart(
            reserve_auction,
            &QueryMsg::AuctionsByEndTime {
                end_time: block_time.plus_seconds(MIN_DURATION),
                query_options: Some(QueryOptions {
                    descending: Some(true),
                    limit: Some(limit),
                    start_after: Some(start_after),
                }),
            },
        )
        .unwrap();

    assert_eq!(response_3.auctions.len(), limit as usize);
    assert_eq!(response_3.auctions[0], response_1.auctions[2]);
    assert_eq!(response_3.auctions[1], response_1.auctions[1]);
    assert_eq!(response_3.auctions[2], response_1.auctions[0]);
}
