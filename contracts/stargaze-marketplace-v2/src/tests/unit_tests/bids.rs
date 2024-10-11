use crate::{
    msg::{ExecuteMsg, QueryMsg},
    orders::{Bid, OrderDetails},
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
fn try_set_bid() {
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

    let token_id = "1";

    // Create bid without sufficient bid funds fails
    let bid_price = coin(1_000_000, NATIVE_DENOM);
    let set_bid = ExecuteMsg::SetBid {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        details: OrderDetails {
            price: bid_price.clone(),
            recipient: None,
            finder: None,
        },
    };
    let response = app.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_bid,
        &[coin(bid_price.amount.u128() - 1u128, NATIVE_DENOM)],
    );
    assert_error(response, ContractError::InsufficientFunds.to_string());

    // Create bid with invalid denom fails
    let bid_price = coin(1_000_000, JUNO_DENOM);
    let set_bid = ExecuteMsg::SetBid {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        details: OrderDetails {
            price: bid_price.clone(),
            recipient: None,
            finder: None,
        },
    };
    let response = app.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_bid,
        &[coin(bid_price.amount.u128() - 1u128, JUNO_DENOM)],
    );
    assert_error(
        response,
        ContractError::InvalidInput("invalid denom".to_string()).to_string(),
    );

    // Create bid with invalid price fails
    let bid_price = coin(0, NATIVE_DENOM);
    let set_bid = ExecuteMsg::SetBid {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        details: OrderDetails {
            price: bid_price.clone(),
            recipient: None,
            finder: None,
        },
    };
    let response =
        app.execute_contract(bidder.clone(), marketplace.clone(), &set_bid, &[bid_price]);
    assert_error(
        response,
        ContractError::InvalidInput("order price must be greater than 0".to_string()).to_string(),
    );

    // Create bid succeeds, even when overpaid
    let recipient = Addr::unchecked("recipient".to_string());
    let finder = Addr::unchecked("finder".to_string());
    let bid_price = coin(1_000_000, NATIVE_DENOM);
    let set_bid = ExecuteMsg::SetBid {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        details: OrderDetails {
            price: bid_price.clone(),
            recipient: Some(recipient.to_string()),
            finder: Some(finder.to_string()),
        },
    };
    let bidder_native_balances_before =
        NativeBalance(app.wrap().query_all_balances(bidder.clone()).unwrap());
    let response = app.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_bid,
        &[coin(bid_price.amount.u128() * 2u128, NATIVE_DENOM)],
    );
    assert!(response.is_ok());

    let bidder_native_balances_after =
        NativeBalance(app.wrap().query_all_balances(bidder.clone()).unwrap());
    assert_eq!(
        bidder_native_balances_before
            .sub(bid_price.clone())
            .unwrap(),
        bidder_native_balances_after
    );

    let bid_id = find_attrs(response.unwrap(), "wasm-set-bid", "id")
        .pop()
        .unwrap();

    let bid = app
        .wrap()
        .query_wasm_smart::<Option<Bid>>(&marketplace, &QueryMsg::Bid(bid_id.clone()))
        .unwrap()
        .unwrap();

    assert_eq!(bid.id, bid_id);
    assert_eq!(bid.creator, bidder);
    assert_eq!(bid.collection, collection);
    assert_eq!(bid.token_id, token_id);
    assert_eq!(bid.details.price, bid_price);
    assert_eq!(bid.details.recipient, Some(recipient));
    assert_eq!(bid.details.finder, Some(finder));
}

#[test]
pub fn try_update_bid() {
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

    let num_bids: u8 = 4;
    let token_id = "1".to_string();
    let mut bid_ids: Vec<String> = vec![];
    for idx in 1..(num_bids + 1) {
        let bid_price = coin(1000000u128 + idx as u128, NATIVE_DENOM);
        let set_bid = ExecuteMsg::SetBid {
            collection: collection.to_string(),
            token_id: token_id.to_string(),
            details: OrderDetails {
                price: bid_price.clone(),
                recipient: None,
                finder: None,
            },
        };
        let response =
            app.execute_contract(bidder.clone(), marketplace.clone(), &set_bid, &[bid_price]);
        assert!(response.is_ok());

        let bid_id = find_attrs(response.unwrap(), "wasm-set-bid", "id")
            .pop()
            .unwrap();
        bid_ids.push(bid_id);
    }

    // Non creator updating bid fails
    let update_bid = ExecuteMsg::UpdateBid {
        id: bid_ids[0].clone(),
        details: OrderDetails {
            price: coin(1000000u128, NATIVE_DENOM),
            recipient: Some(recipient.to_string()),
            finder: Some(finder.to_string()),
        },
    };
    let response = app.execute_contract(owner.clone(), marketplace.clone(), &update_bid, &[]);
    assert_error(
        response,
        MarketplaceStdError::Unauthorized(
            "only the creator of bid can perform this action".to_string(),
        )
        .to_string(),
    );

    // Updating bid succeeds, wallet is refunded
    let new_price = coin(1000000u128, NATIVE_DENOM);
    let update_bid = ExecuteMsg::UpdateBid {
        id: bid_ids[0].clone(),
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
        &update_bid,
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
        accounts: TestAccounts { bidder, .. },
    } = test_context();

    let bidder2 = setup_additional_account(&mut app, "bidder2").unwrap();

    let token_id = "1";
    let price = coin(1000000u128, NATIVE_DENOM);
    let response = app.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &ExecuteMsg::SetBid {
            collection: collection.to_string(),
            token_id: token_id.to_string(),
            details: OrderDetails {
                price: price.clone(),
                recipient: None,
                finder: None,
            },
        },
        &[price],
    );

    let bid_id = find_attrs(response.unwrap(), "wasm-set-bid", "id")
        .pop()
        .unwrap();

    // Removing bid as non creator fails
    let remove_bid = ExecuteMsg::RemoveBid { id: bid_id.clone() };
    let response = app.execute_contract(bidder2.clone(), marketplace.clone(), &remove_bid, &[]);
    assert_error(
        response,
        MarketplaceStdError::Unauthorized(
            "only the creator of bid can perform this action".to_string(),
        )
        .to_string(),
    );

    // Removing bid as creator succeeds
    let response = app.execute_contract(bidder.clone(), marketplace.clone(), &remove_bid, &[]);
    assert!(response.is_ok());

    let bid = app
        .wrap()
        .query_wasm_smart::<Option<Bid>>(&marketplace, &QueryMsg::Bid(bid_id))
        .unwrap();
    assert!(bid.is_none());
}
