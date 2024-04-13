use crate::{
    msg::{ExecuteMsg, QueryMsg},
    orders::{CollectionOffer, OrderDetails},
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
fn try_set_collection_offer() {
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

    // Create offer without sufficient offer funds fails
    let collection_offer_price = coin(1_000_000, NATIVE_DENOM);
    let set_collection_offer = ExecuteMsg::SetCollectionOffer {
        collection: collection.to_string(),
        details: OrderDetails {
            price: collection_offer_price.clone(),
            actor: None,
            finder: None,
        },
    };
    let response = app.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_collection_offer,
        &[coin(
            collection_offer_price.amount.u128() - 1u128,
            NATIVE_DENOM,
        )],
    );
    assert_error(response, ContractError::InsufficientFunds.to_string());

    // Create collection_offer with invalid denom fails
    let collection_offer_price = coin(1_000_000, JUNO_DENOM);
    let set_collection_offer = ExecuteMsg::SetCollectionOffer {
        collection: collection.to_string(),
        details: OrderDetails {
            price: collection_offer_price.clone(),
            actor: None,
            finder: None,
        },
    };
    let response = app.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_collection_offer,
        &[coin(
            collection_offer_price.amount.u128() - 1u128,
            JUNO_DENOM,
        )],
    );
    assert_error(
        response,
        ContractError::InvalidInput("invalid denom".to_string()).to_string(),
    );

    // Create collection_offer succeeds, even when overpaid
    let actor = Addr::unchecked("actor".to_string());
    let finder = Addr::unchecked("finder".to_string());
    let collection_offer_price = coin(1_000_000, NATIVE_DENOM);
    let set_collection_offer = ExecuteMsg::SetCollectionOffer {
        collection: collection.to_string(),
        details: OrderDetails {
            price: collection_offer_price.clone(),
            actor: Some(actor.to_string()),
            finder: Some(finder.to_string()),
        },
    };
    let bidder_native_balances_before =
        NativeBalance(app.wrap().query_all_balances(bidder.clone()).unwrap());
    let response = app.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_collection_offer,
        &[coin(
            collection_offer_price.amount.u128() * 2u128,
            NATIVE_DENOM,
        )],
    );
    assert!(response.is_ok());

    let bidder_native_balances_after =
        NativeBalance(app.wrap().query_all_balances(bidder.clone()).unwrap());
    assert_eq!(
        bidder_native_balances_before
            .sub(collection_offer_price.clone())
            .unwrap(),
        bidder_native_balances_after
    );

    let collection_offer_id = find_attrs(response.unwrap(), "wasm-set-collection-offer", "id")
        .pop()
        .unwrap();

    let collection_offer = app
        .wrap()
        .query_wasm_smart::<Option<CollectionOffer>>(
            &marketplace,
            &QueryMsg::CollectionOffer(collection_offer_id.clone()),
        )
        .unwrap()
        .unwrap();

    assert_eq!(collection_offer.id, collection_offer_id);
    assert_eq!(collection_offer.creator, bidder);
    assert_eq!(collection_offer.collection, collection);
    assert_eq!(collection_offer.details.price, collection_offer_price);
    assert_eq!(collection_offer.details.actor, Some(actor));
    assert_eq!(collection_offer.details.finder, Some(finder));
}

#[test]
pub fn try_update_collection_offer() {
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

    let actor = setup_additional_account(&mut app, "actor").unwrap();
    let finder = setup_additional_account(&mut app, "finder").unwrap();

    let num_collection_offers: u8 = 4;
    let mut collection_offer_ids: Vec<String> = vec![];
    for idx in 1..(num_collection_offers + 1) {
        let collection_offer_price = coin(1000000u128 + idx as u128, NATIVE_DENOM);
        let set_collection_offer = ExecuteMsg::SetCollectionOffer {
            collection: collection.to_string(),
            details: OrderDetails {
                price: collection_offer_price.clone(),
                actor: None,
                finder: None,
            },
        };
        let response = app.execute_contract(
            bidder.clone(),
            marketplace.clone(),
            &set_collection_offer,
            &[collection_offer_price],
        );
        assert!(response.is_ok());

        let collection_offer_id = find_attrs(response.unwrap(), "wasm-set-collection-offer", "id")
            .pop()
            .unwrap();
        collection_offer_ids.push(collection_offer_id);
    }

    // Non creator updating collection_offer fails
    let update_collection_offer = ExecuteMsg::UpdateCollectionOffer {
        id: collection_offer_ids[0].clone(),
        details: OrderDetails {
            price: coin(1000000u128, NATIVE_DENOM),
            actor: Some(actor.to_string()),
            finder: Some(finder.to_string()),
        },
    };
    let response = app.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &update_collection_offer,
        &[],
    );
    assert_error(
        response,
        MarketplaceStdError::Unauthorized(
            "only the creator of collection offer can perform this action".to_string(),
        )
        .to_string(),
    );

    // Updating collection_offer succeeds, wallet is refunded
    let new_price = coin(1000000u128, NATIVE_DENOM);
    let update_collection_offer = ExecuteMsg::UpdateCollectionOffer {
        id: collection_offer_ids[0].clone(),
        details: OrderDetails {
            price: new_price.clone(),
            actor: None,
            finder: None,
        },
    };

    let bidder_native_balances_before =
        NativeBalance(app.wrap().query_all_balances(bidder.clone()).unwrap());
    let response = app.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &update_collection_offer,
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
pub fn try_remove_offer() {
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
        &ExecuteMsg::SetCollectionOffer {
            collection: collection.to_string(),
            details: OrderDetails {
                price: price.clone(),
                actor: None,
                finder: None,
            },
        },
        &[price],
    );

    let collection_offer_id = find_attrs(response.unwrap(), "wasm-set-collection-offer", "id")
        .pop()
        .unwrap();

    // Removing collection_offer as non creator fails
    let remove_collection_offer = ExecuteMsg::RemoveCollectionOffer {
        id: collection_offer_id.clone(),
    };
    let response = app.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &remove_collection_offer,
        &[],
    );
    assert_error(
        response,
        MarketplaceStdError::Unauthorized(
            "only the creator of collection offer can perform this action".to_string(),
        )
        .to_string(),
    );

    // Removing collection_offer as creator succeeds
    let response = app.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &remove_collection_offer,
        &[],
    );
    assert!(response.is_ok());

    let collection_offer = app
        .wrap()
        .query_wasm_smart::<Option<CollectionOffer>>(
            &marketplace,
            &QueryMsg::CollectionOffer(collection_offer_id),
        )
        .unwrap();
    assert!(collection_offer.is_none());
}
