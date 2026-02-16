use crate::{
    constants::BLACKLIST_MANAGER,
    msg::{ExecuteMsg, QueryMsg},
    orders::{Ask, Bid, CollectionBid, OrderDetails},
    tests::{
        helpers::{
            marketplace::{approve, mint, mint_and_set_ask},
            utils::{assert_error, find_attrs},
        },
        setup::{
            setup_accounts::TestAccounts,
            setup_contracts::{LISTING_FEE, NATIVE_DENOM},
            templates::{test_context, TestContext, TestContracts},
        },
    },
    ContractError,
};

use cosmwasm_std::{coin, Addr};
use cw_multi_test::{BankSudo, Executor, SudoMsg};

fn fund_blacklist_manager(app: &mut cw_multi_test::App) -> Addr {
    let manager = Addr::unchecked(BLACKLIST_MANAGER);
    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        to_address: manager.to_string(),
        amount: vec![coin(5_000_000_000, NATIVE_DENOM)],
    }))
    .unwrap();
    manager
}

// ─── Pause / Resume ───

#[test]
fn try_pause_resume_by_manager() {
    let TestContext {
        mut app,
        contracts: TestContracts { marketplace, .. },
        ..
    } = test_context();

    let manager = fund_blacklist_manager(&mut app);

    // Initially not paused
    let paused: bool = app
        .wrap()
        .query_wasm_smart(&marketplace, &QueryMsg::Paused {})
        .unwrap();
    assert!(!paused);

    // Manager can pause
    let res = app.execute_contract(manager.clone(), marketplace.clone(), &ExecuteMsg::Pause {}, &[]);
    assert!(res.is_ok());

    let paused: bool = app
        .wrap()
        .query_wasm_smart(&marketplace, &QueryMsg::Paused {})
        .unwrap();
    assert!(paused);

    // Manager can resume
    let res = app.execute_contract(manager, marketplace.clone(), &ExecuteMsg::Resume {}, &[]);
    assert!(res.is_ok());

    let paused: bool = app
        .wrap()
        .query_wasm_smart(&marketplace, &QueryMsg::Paused {})
        .unwrap();
    assert!(!paused);
}

#[test]
fn try_pause_by_non_manager_fails() {
    let TestContext {
        mut app,
        contracts: TestContracts { marketplace, .. },
        accounts: TestAccounts { owner, .. },
    } = test_context();

    let res = app.execute_contract(owner, marketplace.clone(), &ExecuteMsg::Pause {}, &[]);
    assert!(res.is_err());
}

// ─── Trade ops blocked when paused ───

#[test]
fn try_set_ask_when_paused() {
    let TestContext {
        mut app,
        contracts:
            TestContracts {
                marketplace,
                collection,
                ..
            },
        accounts:
            TestAccounts {
                creator, owner, ..
            },
    } = test_context();

    let manager = fund_blacklist_manager(&mut app);

    mint(&mut app, &creator, &owner, &collection, "1");
    approve(&mut app, &owner, &collection, &marketplace, "1");

    // Pause
    app.execute_contract(manager, marketplace.clone(), &ExecuteMsg::Pause {}, &[])
        .unwrap();

    // SetAsk should fail
    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: "1".to_string(),
        details: OrderDetails {
            price: coin(1_000_000, NATIVE_DENOM),
            recipient: None,
            finder: None,
        },
    };
    let res = app.execute_contract(
        owner,
        marketplace.clone(),
        &set_ask,
        &[coin(LISTING_FEE, NATIVE_DENOM)],
    );
    assert_error(res, ContractError::ContractPaused.to_string());
}

#[test]
fn try_set_bid_when_paused() {
    let TestContext {
        mut app,
        contracts:
            TestContracts {
                marketplace,
                collection,
                ..
            },
        accounts: TestAccounts { bidder, .. },
    } = test_context();

    let manager = fund_blacklist_manager(&mut app);

    // Pause
    app.execute_contract(manager, marketplace.clone(), &ExecuteMsg::Pause {}, &[])
        .unwrap();

    let set_bid = ExecuteMsg::SetBid {
        collection: collection.to_string(),
        token_id: "1".to_string(),
        details: OrderDetails {
            price: coin(1_000_000, NATIVE_DENOM),
            recipient: None,
            finder: None,
        },
    };
    let res = app.execute_contract(
        bidder,
        marketplace.clone(),
        &set_bid,
        &[coin(1_000_000, NATIVE_DENOM)],
    );
    assert_error(res, ContractError::ContractPaused.to_string());
}

#[test]
fn try_set_collection_bid_when_paused() {
    let TestContext {
        mut app,
        contracts:
            TestContracts {
                marketplace,
                collection,
                ..
            },
        accounts: TestAccounts { bidder, .. },
    } = test_context();

    let manager = fund_blacklist_manager(&mut app);

    // Pause
    app.execute_contract(manager, marketplace.clone(), &ExecuteMsg::Pause {}, &[])
        .unwrap();

    let set_collection_bid = ExecuteMsg::SetCollectionBid {
        collection: collection.to_string(),
        details: OrderDetails {
            price: coin(1_000_000, NATIVE_DENOM),
            recipient: None,
            finder: None,
        },
    };
    let res = app.execute_contract(
        bidder,
        marketplace.clone(),
        &set_collection_bid,
        &[coin(1_000_000, NATIVE_DENOM)],
    );
    assert_error(res, ContractError::ContractPaused.to_string());
}

// ─── Withdrawals work when paused ───

#[test]
fn try_remove_ask_when_paused() {
    let TestContext {
        mut app,
        contracts:
            TestContracts {
                marketplace,
                collection,
                ..
            },
        accounts:
            TestAccounts {
                creator, owner, ..
            },
    } = test_context();

    let manager = fund_blacklist_manager(&mut app);

    // Create ask before pausing
    mint_and_set_ask(
        &mut app,
        &creator,
        &owner,
        &marketplace,
        &collection,
        "1",
        OrderDetails {
            price: coin(1_000_000, NATIVE_DENOM),
            recipient: None,
            finder: None,
        },
    );

    // Get ask ID
    let asks_result: Vec<Ask> = app
        .wrap()
        .query_wasm_smart(
            &marketplace,
            &QueryMsg::AsksByCreatorCollection {
                creator: owner.to_string(),
                collection: collection.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    let ask_id = asks_result[0].id.clone();

    // Pause
    app.execute_contract(manager, marketplace.clone(), &ExecuteMsg::Pause {}, &[])
        .unwrap();

    // Owner can still remove ask when paused
    let res = app.execute_contract(
        owner,
        marketplace.clone(),
        &ExecuteMsg::RemoveAsk { id: ask_id },
        &[],
    );
    assert!(res.is_ok());
}

#[test]
fn try_remove_bid_when_paused() {
    let TestContext {
        mut app,
        contracts:
            TestContracts {
                marketplace,
                collection,
                ..
            },
        accounts: TestAccounts { bidder, .. },
    } = test_context();

    let manager = fund_blacklist_manager(&mut app);

    // Create bid before pausing
    let bid_price = coin(1_000_000, NATIVE_DENOM);
    let res = app.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &ExecuteMsg::SetBid {
            collection: collection.to_string(),
            token_id: "1".to_string(),
            details: OrderDetails {
                price: bid_price.clone(),
                recipient: None,
                finder: None,
            },
        },
        &[bid_price],
    );
    let bid_id = find_attrs(res.unwrap(), "wasm-set-bid", "id")
        .pop()
        .unwrap();

    // Pause
    app.execute_contract(manager, marketplace.clone(), &ExecuteMsg::Pause {}, &[])
        .unwrap();

    // Bidder can still remove bid when paused
    let res = app.execute_contract(
        bidder,
        marketplace.clone(),
        &ExecuteMsg::RemoveBid { id: bid_id },
        &[],
    );
    assert!(res.is_ok());
}

#[test]
fn try_remove_collection_bid_when_paused() {
    let TestContext {
        mut app,
        contracts:
            TestContracts {
                marketplace,
                collection,
                ..
            },
        accounts: TestAccounts { bidder, .. },
    } = test_context();

    let manager = fund_blacklist_manager(&mut app);

    // Create collection bid before pausing
    let bid_price = coin(1_000_000, NATIVE_DENOM);
    let res = app.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &ExecuteMsg::SetCollectionBid {
            collection: collection.to_string(),
            details: OrderDetails {
                price: bid_price.clone(),
                recipient: None,
                finder: None,
            },
        },
        &[bid_price],
    );
    let bid_id = find_attrs(res.unwrap(), "wasm-set-collection-bid", "id")
        .pop()
        .unwrap();

    // Pause
    app.execute_contract(manager, marketplace.clone(), &ExecuteMsg::Pause {}, &[])
        .unwrap();

    // Bidder can still remove collection bid when paused
    let res = app.execute_contract(
        bidder,
        marketplace.clone(),
        &ExecuteMsg::RemoveCollectionBid { id: bid_id },
        &[],
    );
    assert!(res.is_ok());
}

// ─── Manager can remove others' orders ───

#[test]
fn try_manager_remove_others_bid() {
    let TestContext {
        mut app,
        contracts:
            TestContracts {
                marketplace,
                collection,
                ..
            },
        accounts: TestAccounts { bidder, .. },
    } = test_context();

    let manager = fund_blacklist_manager(&mut app);

    // Create bid
    let bid_price = coin(1_000_000, NATIVE_DENOM);
    let res = app.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &ExecuteMsg::SetBid {
            collection: collection.to_string(),
            token_id: "1".to_string(),
            details: OrderDetails {
                price: bid_price.clone(),
                recipient: None,
                finder: None,
            },
        },
        &[bid_price.clone()],
    );
    let bid_id = find_attrs(res.unwrap(), "wasm-set-bid", "id")
        .pop()
        .unwrap();

    let bidder_balance_before = app.wrap().query_balance(bidder.clone(), NATIVE_DENOM).unwrap();

    // Manager removes bid; funds go to original bidder
    let res = app.execute_contract(
        manager,
        marketplace.clone(),
        &ExecuteMsg::RemoveBid { id: bid_id.clone() },
        &[],
    );
    assert!(res.is_ok());

    let bidder_balance_after = app.wrap().query_balance(bidder, NATIVE_DENOM).unwrap();
    assert_eq!(
        bidder_balance_after.amount,
        bidder_balance_before.amount + bid_price.amount
    );

    // Bid is gone
    let bid: Option<Bid> = app
        .wrap()
        .query_wasm_smart(&marketplace, &QueryMsg::Bid(bid_id))
        .unwrap();
    assert!(bid.is_none());
}

#[test]
fn try_manager_remove_others_ask() {
    let TestContext {
        mut app,
        contracts:
            TestContracts {
                marketplace,
                collection,
                ..
            },
        accounts:
            TestAccounts {
                creator, owner, ..
            },
    } = test_context();

    let manager = fund_blacklist_manager(&mut app);

    // Create ask
    mint_and_set_ask(
        &mut app,
        &creator,
        &owner,
        &marketplace,
        &collection,
        "1",
        OrderDetails {
            price: coin(1_000_000, NATIVE_DENOM),
            recipient: None,
            finder: None,
        },
    );

    let asks_result: Vec<Ask> = app
        .wrap()
        .query_wasm_smart(
            &marketplace,
            &QueryMsg::AsksByCreatorCollection {
                creator: owner.to_string(),
                collection: collection.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    let ask_id = asks_result[0].id.clone();

    // Manager removes ask; NFT returns to owner
    let res = app.execute_contract(
        manager,
        marketplace.clone(),
        &ExecuteMsg::RemoveAsk { id: ask_id.clone() },
        &[],
    );
    assert!(res.is_ok());

    // Ask is gone
    let ask: Option<Ask> = app
        .wrap()
        .query_wasm_smart(&marketplace, &QueryMsg::Ask(ask_id))
        .unwrap();
    assert!(ask.is_none());
}

// ─── Bulk operations ───

#[test]
fn try_bulk_remove_bids_by_ids() {
    let TestContext {
        mut app,
        contracts:
            TestContracts {
                marketplace,
                collection,
                ..
            },
        accounts: TestAccounts { bidder, .. },
    } = test_context();

    let manager = fund_blacklist_manager(&mut app);

    // Create multiple bids
    let mut bid_ids = vec![];
    for i in 1..=3 {
        let bid_price = coin(1_000_000u128 + i as u128, NATIVE_DENOM);
        let res = app.execute_contract(
            bidder.clone(),
            marketplace.clone(),
            &ExecuteMsg::SetBid {
                collection: collection.to_string(),
                token_id: i.to_string(),
                details: OrderDetails {
                    price: bid_price.clone(),
                    recipient: None,
                    finder: None,
                },
            },
            &[bid_price],
        );
        let bid_id = find_attrs(res.unwrap(), "wasm-set-bid", "id")
            .pop()
            .unwrap();
        bid_ids.push(bid_id);
    }

    let bidder_balance_before = app.wrap().query_balance(bidder.clone(), NATIVE_DENOM).unwrap();

    // Manager bulk removes by IDs
    let res = app.execute_contract(
        manager,
        marketplace.clone(),
        &ExecuteMsg::BulkRemoveBidsByIds {
            ids: bid_ids.clone(),
        },
        &[],
    );
    assert!(res.is_ok());

    // Check per-item events
    let response = res.unwrap();
    let remove_bid_events: Vec<_> = response
        .events
        .iter()
        .filter(|e| e.ty == "wasm-remove-bid")
        .collect();
    assert_eq!(remove_bid_events.len(), 3);

    // Bidder gets all funds back
    let bidder_balance_after = app.wrap().query_balance(bidder, NATIVE_DENOM).unwrap();
    let total_refund = 1_000_001u128 + 1_000_002 + 1_000_003;
    assert_eq!(
        bidder_balance_after.amount,
        bidder_balance_before.amount + cosmwasm_std::Uint128::new(total_refund)
    );

    // All bids are gone
    for bid_id in bid_ids {
        let bid: Option<Bid> = app
            .wrap()
            .query_wasm_smart(&marketplace, &QueryMsg::Bid(bid_id))
            .unwrap();
        assert!(bid.is_none());
    }
}

#[test]
fn try_bulk_remove_asks_by_ids() {
    let TestContext {
        mut app,
        contracts:
            TestContracts {
                marketplace,
                collection,
                ..
            },
        accounts:
            TestAccounts {
                creator, owner, ..
            },
    } = test_context();

    let manager = fund_blacklist_manager(&mut app);

    // Create multiple asks
    let mut ask_ids = vec![];
    for i in 1..=3 {
        let token_id = i.to_string();
        mint_and_set_ask(
            &mut app,
            &creator,
            &owner,
            &marketplace,
            &collection,
            &token_id,
            OrderDetails {
                price: coin(1_000_000, NATIVE_DENOM),
                recipient: None,
                finder: None,
            },
        );
    }

    let asks_result: Vec<Ask> = app
        .wrap()
        .query_wasm_smart(
            &marketplace,
            &QueryMsg::AsksByCreatorCollection {
                creator: owner.to_string(),
                collection: collection.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    for ask in &asks_result {
        ask_ids.push(ask.id.clone());
    }

    // Manager bulk removes asks by IDs
    let res = app.execute_contract(
        manager,
        marketplace.clone(),
        &ExecuteMsg::BulkRemoveAsksByIds {
            ids: ask_ids.clone(),
        },
        &[],
    );
    assert!(res.is_ok());

    // Check per-item events
    let response = res.unwrap();
    let remove_ask_events: Vec<_> = response
        .events
        .iter()
        .filter(|e| e.ty == "wasm-remove-ask")
        .collect();
    assert_eq!(remove_ask_events.len(), 3);

    // All asks are gone
    for ask_id in ask_ids {
        let ask: Option<Ask> = app
            .wrap()
            .query_wasm_smart(&marketplace, &QueryMsg::Ask(ask_id))
            .unwrap();
        assert!(ask.is_none());
    }
}

#[test]
fn try_bulk_remove_collection_bids_by_ids() {
    let TestContext {
        mut app,
        contracts:
            TestContracts {
                marketplace,
                collection,
                ..
            },
        accounts: TestAccounts { bidder, .. },
    } = test_context();

    let manager = fund_blacklist_manager(&mut app);

    // Create multiple collection bids
    let mut bid_ids = vec![];
    for i in 1..=3 {
        let bid_price = coin(1_000_000u128 + i as u128, NATIVE_DENOM);
        let res = app.execute_contract(
            bidder.clone(),
            marketplace.clone(),
            &ExecuteMsg::SetCollectionBid {
                collection: collection.to_string(),
                details: OrderDetails {
                    price: bid_price.clone(),
                    recipient: None,
                    finder: None,
                },
            },
            &[bid_price],
        );
        let bid_id = find_attrs(res.unwrap(), "wasm-set-collection-bid", "id")
            .pop()
            .unwrap();
        bid_ids.push(bid_id);
    }

    let bidder_balance_before = app.wrap().query_balance(bidder.clone(), NATIVE_DENOM).unwrap();

    // Manager bulk removes by IDs
    let res = app.execute_contract(
        manager,
        marketplace.clone(),
        &ExecuteMsg::BulkRemoveCollectionBidsByIds {
            ids: bid_ids.clone(),
        },
        &[],
    );
    assert!(res.is_ok());

    // Check per-item events
    let response = res.unwrap();
    let remove_events: Vec<_> = response
        .events
        .iter()
        .filter(|e| e.ty == "wasm-remove-collection-bid")
        .collect();
    assert_eq!(remove_events.len(), 3);

    // Bidder gets all funds back
    let bidder_balance_after = app.wrap().query_balance(bidder, NATIVE_DENOM).unwrap();
    let total_refund = 1_000_001u128 + 1_000_002 + 1_000_003;
    assert_eq!(
        bidder_balance_after.amount,
        bidder_balance_before.amount + cosmwasm_std::Uint128::new(total_refund)
    );

    // All bids are gone
    for bid_id in bid_ids {
        let bid: Option<CollectionBid> = app
            .wrap()
            .query_wasm_smart(&marketplace, &QueryMsg::CollectionBid(bid_id))
            .unwrap();
        assert!(bid.is_none());
    }
}

#[test]
fn try_bulk_remove_bids_non_manager_fails() {
    let TestContext {
        mut app,
        contracts:
            TestContracts {
                marketplace,
                collection,
                ..
            },
        accounts: TestAccounts { bidder, .. },
    } = test_context();

    // Create a bid
    let bid_price = coin(1_000_000, NATIVE_DENOM);
    let res = app.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &ExecuteMsg::SetBid {
            collection: collection.to_string(),
            token_id: "1".to_string(),
            details: OrderDetails {
                price: bid_price.clone(),
                recipient: None,
                finder: None,
            },
        },
        &[bid_price],
    );
    let bid_id = find_attrs(res.unwrap(), "wasm-set-bid", "id")
        .pop()
        .unwrap();

    // Non-manager trying to bulk remove fails
    let res = app.execute_contract(
        bidder,
        marketplace.clone(),
        &ExecuteMsg::BulkRemoveBidsByIds {
            ids: vec![bid_id],
        },
        &[],
    );
    assert!(res.is_err());
}

#[test]
fn try_accept_ask_when_paused() {
    let TestContext {
        mut app,
        contracts:
            TestContracts {
                marketplace,
                collection,
                ..
            },
        accounts:
            TestAccounts {
                creator,
                owner,
                bidder,
                ..
            },
    } = test_context();

    let manager = fund_blacklist_manager(&mut app);

    // Create ask before pausing
    mint_and_set_ask(
        &mut app,
        &creator,
        &owner,
        &marketplace,
        &collection,
        "1",
        OrderDetails {
            price: coin(1_000_000, NATIVE_DENOM),
            recipient: None,
            finder: None,
        },
    );

    let asks_result: Vec<Ask> = app
        .wrap()
        .query_wasm_smart(
            &marketplace,
            &QueryMsg::AsksByCreatorCollection {
                creator: owner.to_string(),
                collection: collection.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    let ask_id = asks_result[0].id.clone();

    // Pause
    app.execute_contract(manager, marketplace.clone(), &ExecuteMsg::Pause {}, &[])
        .unwrap();

    // AcceptAsk should fail when paused
    let res = app.execute_contract(
        bidder,
        marketplace.clone(),
        &ExecuteMsg::AcceptAsk {
            id: ask_id,
            details: OrderDetails {
                price: coin(1_000_000, NATIVE_DENOM),
                recipient: None,
                finder: None,
            },
        },
        &[coin(1_000_000, NATIVE_DENOM)],
    );
    assert_error(res, ContractError::ContractPaused.to_string());
}
