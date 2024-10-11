use crate::{
    msg::{ExecuteMsg, QueryMsg},
    orders::{CollectionBid, OrderDetails},
    tests::{
        helpers::utils::{assert_error, find_attrs},
        setup::{
            setup_accounts::{setup_additional_account, TestAccounts},
            setup_contracts::{JUNO_DENOM, NATIVE_DENOM},
            templates::{test_context, TestContext, TestContracts},
        },
    },
    ContractError,
};

use cosmwasm_std::coin;
use cosmwasm_std::Addr;
use cw_multi_test::Executor;
use cw_utils::NativeBalance;
use sg_marketplace_common::MarketplaceStdError;
use std::ops::{Add, Sub};

#[test]
fn try_set_collection_bid() {
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

    // Create bid without sufficient bid funds fails
    let collection_bid_price = coin(1_000_000, NATIVE_DENOM);
    let set_collection_bid = ExecuteMsg::SetCollectionBid {
        collection: collection.to_string(),
        details: OrderDetails {
            price: collection_bid_price.clone(),
            recipient: None,
            finder: None,
        },
    };
    let response = app.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_collection_bid,
        &[coin(
            collection_bid_price.amount.u128() - 1u128,
            NATIVE_DENOM,
        )],
    );
    assert_error(response, ContractError::InsufficientFunds.to_string());

    // Create collection_bid with invalid denom fails
    let collection_bid_price = coin(1_000_000, JUNO_DENOM);
    let set_collection_bid = ExecuteMsg::SetCollectionBid {
        collection: collection.to_string(),
        details: OrderDetails {
            price: collection_bid_price.clone(),
            recipient: None,
            finder: None,
        },
    };
    let response = app.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_collection_bid,
        &[coin(collection_bid_price.amount.u128() - 1u128, JUNO_DENOM)],
    );
    assert_error(
        response,
        ContractError::InvalidInput("invalid denom".to_string()).to_string(),
    );

    // Create collection_bid with invalid price fails
    let collection_bid_price = coin(0, NATIVE_DENOM);
    let set_collection_bid = ExecuteMsg::SetCollectionBid {
        collection: collection.to_string(),
        details: OrderDetails {
            price: collection_bid_price.clone(),
            recipient: None,
            finder: None,
        },
    };
    let response = app.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_collection_bid,
        &[coin(1, NATIVE_DENOM)],
    );
    assert_error(
        response,
        ContractError::InvalidInput("order price must be greater than 0".to_string()).to_string(),
    );

    // Create collection_bid succeeds, even when overpaid
    let recipient = Addr::unchecked("recipient".to_string());
    let finder = Addr::unchecked("finder".to_string());
    let collection_bid_price = coin(1_000_000, NATIVE_DENOM);
    let set_collection_bid = ExecuteMsg::SetCollectionBid {
        collection: collection.to_string(),
        details: OrderDetails {
            price: collection_bid_price.clone(),
            recipient: Some(recipient.to_string()),
            finder: Some(finder.to_string()),
        },
    };
    let bidder_native_balances_before =
        NativeBalance(app.wrap().query_all_balances(bidder.clone()).unwrap());
    let response = app.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_collection_bid,
        &[coin(
            collection_bid_price.amount.u128() * 2u128,
            NATIVE_DENOM,
        )],
    );
    assert!(response.is_ok());

    let bidder_native_balances_after =
        NativeBalance(app.wrap().query_all_balances(bidder.clone()).unwrap());
    assert_eq!(
        bidder_native_balances_before
            .sub(collection_bid_price.clone())
            .unwrap(),
        bidder_native_balances_after
    );

    let collection_bid_id = find_attrs(response.unwrap(), "wasm-set-collection-bid", "id")
        .pop()
        .unwrap();

    let collection_bid = app
        .wrap()
        .query_wasm_smart::<Option<CollectionBid>>(
            &marketplace,
            &QueryMsg::CollectionBid(collection_bid_id.clone()),
        )
        .unwrap()
        .unwrap();

    assert_eq!(collection_bid.id, collection_bid_id);
    assert_eq!(collection_bid.creator, bidder);
    assert_eq!(collection_bid.collection, collection);
    assert_eq!(collection_bid.details.price, collection_bid_price);
    assert_eq!(collection_bid.details.recipient, Some(recipient));
    assert_eq!(collection_bid.details.finder, Some(finder));
}

#[test]
pub fn try_update_collection_bid() {
    let TestContext {
        mut app,
        contracts:
            TestContracts {
                marketplace,
                collection,
                ..
            },
        accounts: TestAccounts { owner, bidder, .. },
    } = test_context();

    let recipient = setup_additional_account(&mut app, "recipient").unwrap();
    let finder = setup_additional_account(&mut app, "finder").unwrap();

    let num_collection_bids: u8 = 4;
    let mut collection_bid_ids: Vec<String> = vec![];
    for idx in 1..(num_collection_bids + 1) {
        let collection_bid_price = coin(1000000u128 + idx as u128, NATIVE_DENOM);
        let set_collection_bid = ExecuteMsg::SetCollectionBid {
            collection: collection.to_string(),
            details: OrderDetails {
                price: collection_bid_price.clone(),
                recipient: None,
                finder: None,
            },
        };
        let response = app.execute_contract(
            bidder.clone(),
            marketplace.clone(),
            &set_collection_bid,
            &[collection_bid_price],
        );
        assert!(response.is_ok());

        let collection_bid_id = find_attrs(response.unwrap(), "wasm-set-collection-bid", "id")
            .pop()
            .unwrap();
        collection_bid_ids.push(collection_bid_id);
    }

    // Non creator updating collection_bid fails
    let update_collection_bid = ExecuteMsg::UpdateCollectionBid {
        id: collection_bid_ids[0].clone(),
        details: OrderDetails {
            price: coin(1000000u128, NATIVE_DENOM),
            recipient: Some(recipient.to_string()),
            finder: Some(finder.to_string()),
        },
    };
    let response = app.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &update_collection_bid,
        &[],
    );
    assert_error(
        response,
        MarketplaceStdError::Unauthorized(
            "only the creator of collection bid can perform this action".to_string(),
        )
        .to_string(),
    );

    // Updating collection_bid succeeds, wallet is refunded
    let new_price = coin(1000000u128, NATIVE_DENOM);
    let update_collection_bid = ExecuteMsg::UpdateCollectionBid {
        id: collection_bid_ids[0].clone(),
        details: OrderDetails {
            price: new_price.clone(),
            recipient: None,
            finder: None,
        },
    };

    let bidder_native_balances_before =
        NativeBalance(app.wrap().query_all_balances(bidder.clone()).unwrap());
    let response = app.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &update_collection_bid,
        &[new_price],
    );
    assert!(response.is_ok());
    let bidder_native_balances_after =
        NativeBalance(app.wrap().query_all_balances(bidder.clone()).unwrap());

    assert_eq!(
        bidder_native_balances_before.add(coin(1u128, NATIVE_DENOM).clone()),
        bidder_native_balances_after
    );
}

#[test]
pub fn try_remove_bid() {
    let TestContext {
        mut app,
        contracts:
            TestContracts {
                marketplace,
                collection,
                ..
            },
        accounts: TestAccounts { owner, bidder, .. },
    } = test_context();

    let price = coin(1000000u128, NATIVE_DENOM);
    let response = app.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &ExecuteMsg::SetCollectionBid {
            collection: collection.to_string(),
            details: OrderDetails {
                price: price.clone(),
                recipient: None,
                finder: None,
            },
        },
        &[price],
    );

    let collection_bid_id = find_attrs(response.unwrap(), "wasm-set-collection-bid", "id")
        .pop()
        .unwrap();

    // Removing collection_bid as non creator fails
    let remove_collection_bid = ExecuteMsg::RemoveCollectionBid {
        id: collection_bid_id.clone(),
    };
    let response = app.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &remove_collection_bid,
        &[],
    );
    assert_error(
        response,
        MarketplaceStdError::Unauthorized(
            "only the creator of collection bid can perform this action".to_string(),
        )
        .to_string(),
    );

    // Removing collection_bid as creator succeeds
    let response = app.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &remove_collection_bid,
        &[],
    );
    assert!(response.is_ok());

    let collection_bid = app
        .wrap()
        .query_wasm_smart::<Option<CollectionBid>>(
            &marketplace,
            &QueryMsg::CollectionBid(collection_bid_id),
        )
        .unwrap();
    assert!(collection_bid.is_none());
}
