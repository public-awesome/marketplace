use crate::helpers::BLACKLIST_MANAGER;
use crate::msg::{ExecuteMsg, QueryMsg};
use crate::tests::helpers::auction_functions::{create_standard_auction, place_bid};
use crate::tests::helpers::constants::{
    CREATE_AUCTION_FEE, DEFAULT_DURATION, MIN_RESERVE_PRICE,
};
use crate::tests::helpers::nft_functions::{approve, mint};
use crate::tests::helpers::utils::assert_error;
use crate::tests::setup::setup_accounts::{setup_addtl_account, INITIAL_BALANCE};
use crate::tests::setup::setup_auctions::setup_reserve_auction;
use crate::tests::setup::setup_fair_burn::setup_fair_burn;
use crate::tests::setup::setup_minters::standard_minter_template;
use crate::ContractError;

use cosmwasm_std::{coin, Addr};
use cw_multi_test::{BankSudo, Executor, SudoMsg as CwSudoMsg};
use sg_std::{GENESIS_MINT_START_TIME, NATIVE_DENOM};
use test_suite::common_setup::setup_accounts_and_block::setup_block_time;

fn fund_blacklist_manager(router: &mut sg_multi_test::StargazeApp) -> Addr {
    let manager = Addr::unchecked(BLACKLIST_MANAGER);
    router
        .sudo(CwSudoMsg::Bank(BankSudo::Mint {
            to_address: manager.to_string(),
            amount: vec![coin(5_000_000_000, NATIVE_DENOM)],
        }))
        .unwrap();
    manager
}

// ─── Pause / Resume ───

#[test]
fn try_pause_resume_by_manager() {
    let vt = standard_minter_template(1);
    let (mut router, creator, _) = (vt.router, vt.accts.creator, vt.accts.bidder);
    let fair_burn = setup_fair_burn(&mut router, creator.clone());
    let auction = setup_reserve_auction(&mut router, creator.clone(), fair_burn).unwrap();

    let manager = fund_blacklist_manager(&mut router);

    // Initially not paused
    let paused: bool = router
        .wrap()
        .query_wasm_smart(&auction, &QueryMsg::Paused {})
        .unwrap();
    assert!(!paused);

    // Manager can pause
    let res = router.execute_contract(manager.clone(), auction.clone(), &ExecuteMsg::Pause {}, &[]);
    assert!(res.is_ok());

    let paused: bool = router
        .wrap()
        .query_wasm_smart(&auction, &QueryMsg::Paused {})
        .unwrap();
    assert!(paused);

    // Manager can resume
    let res = router.execute_contract(manager, auction.clone(), &ExecuteMsg::Resume {}, &[]);
    assert!(res.is_ok());

    let paused: bool = router
        .wrap()
        .query_wasm_smart(&auction, &QueryMsg::Paused {})
        .unwrap();
    assert!(!paused);
}

#[test]
fn try_pause_by_non_manager_fails() {
    let vt = standard_minter_template(1);
    let (mut router, creator, _) = (vt.router, vt.accts.creator, vt.accts.bidder);
    let fair_burn = setup_fair_burn(&mut router, creator.clone());
    let auction = setup_reserve_auction(&mut router, creator.clone(), fair_burn).unwrap();

    let res = router.execute_contract(creator, auction.clone(), &ExecuteMsg::Pause {}, &[]);
    assert!(res.is_err());
}

// ─── Trade ops blocked when paused ───

#[test]
fn try_create_auction_when_paused() {
    let vt = standard_minter_template(1);
    let (mut router, creator, _) = (vt.router, vt.accts.creator, vt.accts.bidder);
    let fair_burn = setup_fair_burn(&mut router, creator.clone());
    let auction = setup_reserve_auction(&mut router, creator.clone(), fair_burn).unwrap();
    let minter = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let token_id: u32 = 1;

    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);

    let auction_creator =
        setup_addtl_account(&mut router, "auction_creator", INITIAL_BALANCE).unwrap();
    mint(&mut router, &minter, &creator, &auction_creator);
    approve(&mut router, &auction_creator, &collection, &auction, token_id);

    let manager = fund_blacklist_manager(&mut router);

    // Pause
    router
        .execute_contract(manager, auction.clone(), &ExecuteMsg::Pause {}, &[])
        .unwrap();

    // CreateAuction should fail
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
    assert_error(res, ContractError::ContractPaused {}.to_string());
}

#[test]
fn try_place_bid_when_paused() {
    let vt = standard_minter_template(1);
    let (mut router, creator, bidder) = (vt.router, vt.accts.creator, vt.accts.bidder);
    let fair_burn = setup_fair_burn(&mut router, creator.clone());
    let auction = setup_reserve_auction(&mut router, creator.clone(), fair_burn).unwrap();
    let minter = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let token_id: u32 = 1;

    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);

    let auction_creator =
        setup_addtl_account(&mut router, "auction_creator", INITIAL_BALANCE).unwrap();
    mint(&mut router, &minter, &creator, &auction_creator);
    approve(&mut router, &auction_creator, &collection, &auction, token_id);

    // Create auction before pausing
    create_standard_auction(
        &mut router,
        &auction_creator,
        &auction,
        collection.as_ref(),
        &token_id.to_string(),
        coin(MIN_RESERVE_PRICE, NATIVE_DENOM),
        DEFAULT_DURATION,
        None,
        coin(CREATE_AUCTION_FEE.u128(), NATIVE_DENOM),
    )
    .unwrap();

    let manager = fund_blacklist_manager(&mut router);

    // Pause
    router
        .execute_contract(manager, auction.clone(), &ExecuteMsg::Pause {}, &[])
        .unwrap();

    // PlaceBid should fail
    let res = place_bid(
        &mut router,
        &auction,
        &bidder,
        collection.as_ref(),
        &token_id.to_string(),
        coin(MIN_RESERVE_PRICE, NATIVE_DENOM),
    );
    assert_error(res, ContractError::ContractPaused {}.to_string());
}

#[test]
fn try_update_reserve_price_when_paused() {
    let vt = standard_minter_template(1);
    let (mut router, creator, _) = (vt.router, vt.accts.creator, vt.accts.bidder);
    let fair_burn = setup_fair_burn(&mut router, creator.clone());
    let auction = setup_reserve_auction(&mut router, creator.clone(), fair_burn).unwrap();
    let minter = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let token_id: u32 = 1;

    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);

    let auction_creator =
        setup_addtl_account(&mut router, "auction_creator", INITIAL_BALANCE).unwrap();
    mint(&mut router, &minter, &creator, &auction_creator);
    approve(&mut router, &auction_creator, &collection, &auction, token_id);

    // Create auction before pausing
    create_standard_auction(
        &mut router,
        &auction_creator,
        &auction,
        collection.as_ref(),
        &token_id.to_string(),
        coin(MIN_RESERVE_PRICE, NATIVE_DENOM),
        DEFAULT_DURATION,
        None,
        coin(CREATE_AUCTION_FEE.u128(), NATIVE_DENOM),
    )
    .unwrap();

    let manager = fund_blacklist_manager(&mut router);

    // Pause
    router
        .execute_contract(manager, auction.clone(), &ExecuteMsg::Pause {}, &[])
        .unwrap();

    // UpdateReservePrice should fail
    let res = router.execute_contract(
        auction_creator,
        auction.clone(),
        &ExecuteMsg::UpdateReservePrice {
            collection: collection.to_string(),
            token_id: token_id.to_string(),
            reserve_price: coin(MIN_RESERVE_PRICE * 2, NATIVE_DENOM),
        },
        &[],
    );
    assert_error(res, ContractError::ContractPaused {}.to_string());
}

// ─── Withdrawals work when paused ───

#[test]
fn try_cancel_auction_when_paused() {
    let vt = standard_minter_template(1);
    let (mut router, creator, _) = (vt.router, vt.accts.creator, vt.accts.bidder);
    let fair_burn = setup_fair_burn(&mut router, creator.clone());
    let auction = setup_reserve_auction(&mut router, creator.clone(), fair_burn).unwrap();
    let minter = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let token_id: u32 = 1;

    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);

    let auction_creator =
        setup_addtl_account(&mut router, "auction_creator", INITIAL_BALANCE).unwrap();
    mint(&mut router, &minter, &creator, &auction_creator);
    approve(&mut router, &auction_creator, &collection, &auction, token_id);

    // Create auction before pausing
    create_standard_auction(
        &mut router,
        &auction_creator,
        &auction,
        collection.as_ref(),
        &token_id.to_string(),
        coin(MIN_RESERVE_PRICE, NATIVE_DENOM),
        DEFAULT_DURATION,
        None,
        coin(CREATE_AUCTION_FEE.u128(), NATIVE_DENOM),
    )
    .unwrap();

    let manager = fund_blacklist_manager(&mut router);

    // Pause
    router
        .execute_contract(manager, auction.clone(), &ExecuteMsg::Pause {}, &[])
        .unwrap();

    // CancelAuction should still work (unstarted auction)
    let res = router.execute_contract(
        auction_creator,
        auction.clone(),
        &ExecuteMsg::CancelAuction {
            collection: collection.to_string(),
            token_id: token_id.to_string(),
        },
        &[],
    );
    assert!(res.is_ok());
}
