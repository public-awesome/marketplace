use crate::{
    msg::{ExecuteMsg, QueryMsg},
    orders::{Offer, OrderDetails},
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
use std::ops::Sub;

#[test]
fn try_set_offer() {
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

    // Create offer without sufficient offer funds fails
    let offer_price = coin(1_000_000, NATIVE_DENOM);
    let set_offer = ExecuteMsg::SetOffer {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        details: OrderDetails {
            price: offer_price.clone(),
            recipient: None,
            finder: None,
        },
    };
    let response = app.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_offer,
        &[coin(offer_price.amount.u128() - 1u128, NATIVE_DENOM)],
    );
    assert_error(response, ContractError::InsufficientFunds.to_string());

    // Create offer with invalid denom fails
    let offer_price = coin(1_000_000, JUNO_DENOM);
    let set_offer = ExecuteMsg::SetOffer {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        details: OrderDetails {
            price: offer_price.clone(),
            recipient: None,
            finder: None,
        },
    };
    let response = app.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_offer,
        &[coin(offer_price.amount.u128() - 1u128, JUNO_DENOM)],
    );
    assert_error(
        response,
        ContractError::InvalidInput("invalid denom".to_string()).to_string(),
    );

    // Create offer succeeds, even when overpaid
    let recipient = Addr::unchecked("recipient".to_string());
    let finder = Addr::unchecked("finder".to_string());
    let offer_price = coin(1_000_000, NATIVE_DENOM);
    let set_offer = ExecuteMsg::SetOffer {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        details: OrderDetails {
            price: offer_price.clone(),
            recipient: Some(recipient.to_string()),
            finder: Some(finder.to_string()),
        },
    };
    let bidder_native_balances_before =
        NativeBalance(app.wrap().query_all_balances(bidder.clone()).unwrap());
    let response = app.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_offer,
        &[coin(offer_price.amount.u128() * 2u128, NATIVE_DENOM)],
    );
    assert!(response.is_ok());

    let bidder_native_balances_after =
        NativeBalance(app.wrap().query_all_balances(bidder.clone()).unwrap());
    assert_eq!(
        bidder_native_balances_before
            .sub(offer_price.clone())
            .unwrap(),
        bidder_native_balances_after
    );

    let offer_id = find_attrs(response.unwrap(), "wasm-set-offer", "id")
        .pop()
        .unwrap();

    let offer = app
        .wrap()
        .query_wasm_smart::<Option<Offer>>(&marketplace, &QueryMsg::Offer(offer_id.clone()))
        .unwrap()
        .unwrap();

    assert_eq!(offer.id, offer_id);
    assert_eq!(offer.creator, bidder);
    assert_eq!(offer.collection, collection);
    assert_eq!(offer.token_id, token_id);
    assert_eq!(offer.details.price, offer_price);
    assert_eq!(offer.details.recipient, Some(recipient));
    assert_eq!(offer.details.finder, Some(finder));
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
        accounts: TestAccounts { bidder, .. },
    } = test_context();

    let bidder2 = setup_additional_account(&mut app, "bidder2").unwrap();

    let token_id = "1";
    let price = coin(1000000u128, NATIVE_DENOM);
    let response = app.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &ExecuteMsg::SetOffer {
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

    let offer_id = find_attrs(response.unwrap(), "wasm-set-offer", "id")
        .pop()
        .unwrap();

    // Removing offer as non creator fails
    let remove_offer = ExecuteMsg::RemoveOffer {
        id: offer_id.clone(),
    };
    let response = app.execute_contract(bidder2.clone(), marketplace.clone(), &remove_offer, &[]);
    assert_error(
        response,
        MarketplaceStdError::Unauthorized(
            "only the creator of offer can perform this action".to_string(),
        )
        .to_string(),
    );

    // Removing offer as creator succeeds
    let response = app.execute_contract(bidder.clone(), marketplace.clone(), &remove_offer, &[]);
    assert!(response.is_ok());

    let offer = app
        .wrap()
        .query_wasm_smart::<Option<Offer>>(&marketplace, &QueryMsg::Offer(offer_id))
        .unwrap();
    assert!(offer.is_none());
}
