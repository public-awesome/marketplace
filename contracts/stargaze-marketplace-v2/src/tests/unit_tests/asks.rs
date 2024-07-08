use crate::{
    helpers::generate_id,
    msg::{ExecuteMsg, QueryMsg},
    orders::{Ask, OrderDetails},
    tests::{
        helpers::{
            marketplace::{approve, mint, mint_and_set_ask},
            utils::assert_error,
        },
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
use sg_marketplace_common::MarketplaceStdError;

#[test]
fn try_set_ask() {
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

    let num_nfts: u8 = 4;
    let mut token_ids: Vec<String> = vec![];
    for idx in 1..(num_nfts + 1) {
        let token_id = idx.to_string();
        mint(&mut app, &creator, &owner, &collection, &token_id);
        token_ids.push(token_id.clone());
    }

    // Create ask unowned token fails
    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_ids[0].clone(),
        details: OrderDetails {
            price: coin(1_000_000, NATIVE_DENOM),
            recipient: None,
            finder: None,
        },
    };
    let response = app.execute_contract(bidder, marketplace.clone(), &set_ask, &[]);
    assert_error(response, "Unauthorized: sender is not owner".to_string());

    // Create ask without token approval fails
    let response = app.execute_contract(owner.clone(), marketplace.clone(), &set_ask, &[]);
    assert!(response.is_err());

    // Create ask with invalid denom fails
    approve(&mut app, &owner, &collection, &marketplace, &token_ids[0]);
    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_ids[0].clone(),
        details: OrderDetails {
            price: coin(1_000_000, JUNO_DENOM),
            recipient: None,
            finder: None,
        },
    };
    let response = app.execute_contract(owner.clone(), marketplace.clone(), &set_ask, &[]);
    assert_error(
        response,
        ContractError::InvalidInput("invalid denom".to_string()).to_string(),
    );

    // Create ask succeeds
    let recipient = Addr::unchecked("recipient".to_string());
    let finder = Addr::unchecked("finder".to_string());
    let price = coin(1_000_000, NATIVE_DENOM);
    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_ids[0].clone(),
        details: OrderDetails {
            price: price.clone(),
            recipient: Some(recipient.to_string()),
            finder: Some(finder.to_string()),
        },
    };
    let response = app.execute_contract(owner.clone(), marketplace.clone(), &set_ask, &[]);
    assert!(response.is_ok());

    let ask_id = generate_id(vec![collection.as_bytes(), token_ids[0].as_bytes()]);
    let ask: Ask = app
        .wrap()
        .query_wasm_smart(&marketplace, &QueryMsg::Ask(ask_id))
        .unwrap();
    assert_eq!(ask.creator, owner);
    assert_eq!(ask.collection, collection);
    assert_eq!(ask.token_id, token_ids[0]);
    assert_eq!(ask.details.price, price);
    assert_eq!(ask.details.recipient, Some(recipient));
    assert_eq!(ask.details.finder, Some(finder));

    // Create duplicate ask fails
    let response = app.execute_contract(owner.clone(), marketplace.clone(), &set_ask, &[]);
    assert_error(response, "Unauthorized: sender is not owner".to_string());
}

#[test]
pub fn try_update_ask() {
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

    let recipient = setup_additional_account(&mut app, "recipient").unwrap();
    let finder = setup_additional_account(&mut app, "finder").unwrap();

    let num_nfts: u8 = 4;
    let mut token_ids: Vec<String> = vec![];
    for idx in 1..(num_nfts + 1) {
        let token_id = idx.to_string();
        mint_and_set_ask(
            &mut app,
            &creator,
            &owner,
            &marketplace,
            &collection,
            &token_id,
            &[],
            OrderDetails {
                price: coin(1000000 + idx as u128, NATIVE_DENOM),
                recipient: Some(recipient.to_string()),
                finder: Some(finder.to_string()),
            },
        );
        token_ids.push(token_id.clone());
    }

    // Non creator updating ask fails
    let ask_id = generate_id(vec![collection.as_bytes(), token_ids[0_usize].as_bytes()]);
    let update_ask = ExecuteMsg::UpdateAsk {
        id: ask_id.clone(),
        details: OrderDetails {
            price: coin(1000000, NATIVE_DENOM).clone(),
            recipient: None,
            finder: None,
        },
    };
    let response = app.execute_contract(bidder.clone(), marketplace.clone(), &update_ask, &[]);
    assert_error(
        response,
        MarketplaceStdError::Unauthorized(
            "only the creator of ask can perform this action".to_string(),
        )
        .to_string(),
    );

    // Setting recipient and finder to None succeeds
    let new_price = coin(1000001u128, NATIVE_DENOM);
    let ask_id = generate_id(vec![collection.as_bytes(), token_ids[0_usize].as_bytes()]);
    let update_ask = ExecuteMsg::UpdateAsk {
        id: ask_id.clone(),
        details: OrderDetails {
            price: new_price.clone(),
            recipient: None,
            finder: None,
        },
    };
    let response = app.execute_contract(owner.clone(), marketplace.clone(), &update_ask, &[]);
    assert!(response.is_ok());

    let ask: Ask = app
        .wrap()
        .query_wasm_smart(&marketplace, &QueryMsg::Ask(ask_id))
        .unwrap();

    assert_eq!(ask.details.price, new_price);
    assert_eq!(ask.details.recipient, None);
    assert_eq!(ask.details.finder, None);
}

#[test]
pub fn try_remove_ask() {
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

    let num_nfts: u8 = 4;
    let mut token_ids: Vec<String> = vec![];
    for idx in 1..(num_nfts + 1) {
        let token_id = idx.to_string();
        mint_and_set_ask(
            &mut app,
            &creator,
            &owner,
            &marketplace,
            &collection,
            &token_id,
            &[],
            OrderDetails {
                price: coin(1000000 + idx as u128, NATIVE_DENOM),
                recipient: None,
                finder: None,
            },
        );
        token_ids.push(token_id.clone());
    }

    // Removing ask as non creator fails
    let ask_id = generate_id(vec![collection.as_bytes(), token_ids[0_usize].as_bytes()]);
    let remove_ask = ExecuteMsg::RemoveAsk { id: ask_id.clone() };
    let response = app.execute_contract(bidder.clone(), marketplace.clone(), &remove_ask, &[]);
    assert_error(
        response,
        MarketplaceStdError::Unauthorized(
            "only the creator of ask can perform this action".to_string(),
        )
        .to_string(),
    );

    // Removing ask as creator succeeds
    let response = app.execute_contract(owner.clone(), marketplace.clone(), &remove_ask, &[]);
    assert!(response.is_ok());

    let ask = app
        .wrap()
        .query_wasm_smart::<Option<Ask>>(&marketplace, &QueryMsg::Ask(ask_id))
        .unwrap();
    assert!(ask.is_none());
}
