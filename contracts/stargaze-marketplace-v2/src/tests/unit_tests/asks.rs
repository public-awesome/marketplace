use crate::{
    helpers::generate_id,
    msg::{ExecuteMsg, QueryMsg},
    orders::{Ask, Expiry, OrderDetails},
    tests::{
        helpers::{
            marketplace::{approve, mint, mint_and_set_ask},
            utils::assert_error,
        },
        setup::{
            setup_accounts::{setup_additional_account, TestAccounts},
            setup_contracts::{JUNO_DENOM, LISTING_FEE, MIN_EXPIRY_REWARD, NATIVE_DENOM},
            templates::{test_context, TestContext, TestContracts},
        },
    },
    ContractError,
};

use cosmwasm_std::{coin, Addr};
use cw_multi_test::Executor;
use cw_utils::NativeBalance;
use sg_marketplace_common::MarketplaceStdError;
use std::ops::{Add, Sub};

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
            expiry: None,
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
            expiry: None,
        },
    };
    let response = app.execute_contract(owner.clone(), marketplace.clone(), &set_ask, &[]);
    assert_error(
        response,
        ContractError::InvalidInput("invalid denom".to_string()).to_string(),
    );

    // Create ask with invalid price fails
    approve(&mut app, &owner, &collection, &marketplace, &token_ids[0]);
    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_ids[0].clone(),
        details: OrderDetails {
            price: coin(0, NATIVE_DENOM),
            recipient: None,
            finder: None,
            expiry: None,
        },
    };
    let response = app.execute_contract(owner.clone(), marketplace.clone(), &set_ask, &[]);
    assert_error(
        response,
        ContractError::InvalidInput("order price must be greater than 0".to_string()).to_string(),
    );

    // Create ask without paying listing fee fails
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
            expiry: None,
        },
    };
    let response = app.execute_contract(owner.clone(), marketplace.clone(), &set_ask, &[]);
    assert_error(
        response,
        ContractError::InsufficientFunds("listing fee".to_string()).to_string(),
    );

    // Create ask with invalid listing fee denom fails
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
            expiry: None,
        },
    };
    let response = app.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &set_ask,
        &[coin(LISTING_FEE, JUNO_DENOM)],
    );
    assert_error(
        response,
        ContractError::InsufficientFunds("listing fee".to_string()).to_string(),
    );

    // Create ask with invalid listing fee amount fails
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
            expiry: None,
        },
    };
    let response = app.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &set_ask,
        &[coin(LISTING_FEE - 1u128, NATIVE_DENOM)],
    );
    assert_error(
        response,
        ContractError::InsufficientFunds("listing fee".to_string()).to_string(),
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
            expiry: None,
        },
    };
    let response = app.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &set_ask,
        &[coin(LISTING_FEE, NATIVE_DENOM)],
    );
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

    // Create expiring ask with reward below min fails
    approve(&mut app, &owner, &collection, &marketplace, &token_ids[1]);
    let expiry_reward = coin(MIN_EXPIRY_REWARD - 1u128, NATIVE_DENOM);
    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_ids[1].clone(),
        details: OrderDetails {
            price: price.clone(),
            recipient: None,
            finder: None,
            expiry: Some(Expiry {
                timestamp: app.block_info().time.plus_seconds(100),
                reward: expiry_reward.clone(),
            }),
        },
    };
    let response = app.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &set_ask,
        &[
            coin(LISTING_FEE, NATIVE_DENOM),
            coin(MIN_EXPIRY_REWARD - 1u128, NATIVE_DENOM),
        ],
    );
    assert_error(
        response,
        ContractError::InvalidInput(
            "expiry reward must be greater than or equal to min expiry reward".to_string(),
        )
        .to_string(),
    );

    // Create expiring ask with insufficient funds fails
    let expiry_reward = coin(MIN_EXPIRY_REWARD, NATIVE_DENOM);
    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_ids[1].clone(),
        details: OrderDetails {
            price: price.clone(),
            recipient: None,
            finder: None,
            expiry: Some(Expiry {
                timestamp: app.block_info().time.plus_seconds(100),
                reward: expiry_reward.clone(),
            }),
        },
    };
    let response = app.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &set_ask,
        &[
            coin(LISTING_FEE, NATIVE_DENOM),
            coin(MIN_EXPIRY_REWARD - 1u128, NATIVE_DENOM),
        ],
    );
    assert_error(
        response,
        ContractError::InsufficientFunds("listing fee".to_string()).to_string(),
    );

    // Create expiring ask with enough funds succeeds
    let expiry_reward = coin(MIN_EXPIRY_REWARD, NATIVE_DENOM);
    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_ids[1].clone(),
        details: OrderDetails {
            price: price.clone(),
            recipient: None,
            finder: None,
            expiry: Some(Expiry {
                timestamp: app.block_info().time.plus_seconds(100),
                reward: expiry_reward.clone(),
            }),
        },
    };
    let response = app.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &set_ask,
        &[
            coin(LISTING_FEE, NATIVE_DENOM),
            coin(MIN_EXPIRY_REWARD, NATIVE_DENOM),
        ],
    );
    assert!(response.is_ok());
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
            OrderDetails {
                price: coin(1000000 + idx as u128, NATIVE_DENOM),
                recipient: Some(recipient.to_string()),
                finder: Some(finder.to_string()),
                expiry: None,
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
            expiry: None,
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
            expiry: None,
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

    // Setting expiry with reward below min fails
    let expiry_reward = coin(MIN_EXPIRY_REWARD - 1u128, NATIVE_DENOM);
    let ask_id = generate_id(vec![collection.as_bytes(), token_ids[0_usize].as_bytes()]);
    let update_ask = ExecuteMsg::UpdateAsk {
        id: ask_id.clone(),
        details: OrderDetails {
            price: new_price.clone(),
            recipient: None,
            finder: None,
            expiry: Some(Expiry {
                timestamp: app.block_info().time.plus_seconds(100),
                reward: expiry_reward.clone(),
            }),
        },
    };
    let response = app.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &update_ask,
        &[coin(MIN_EXPIRY_REWARD - 1u128, NATIVE_DENOM)],
    );
    assert_error(
        response,
        ContractError::InvalidInput(
            "expiry reward must be greater than or equal to min expiry reward".to_string(),
        )
        .to_string(),
    );

    // Setting expiry with insufficient funds fails
    let expiry_reward = coin(MIN_EXPIRY_REWARD, NATIVE_DENOM);
    let ask_id = generate_id(vec![collection.as_bytes(), token_ids[0_usize].as_bytes()]);
    let update_ask = ExecuteMsg::UpdateAsk {
        id: ask_id.clone(),
        details: OrderDetails {
            price: new_price.clone(),
            recipient: None,
            finder: None,
            expiry: Some(Expiry {
                timestamp: app.block_info().time.plus_seconds(100),
                reward: expiry_reward.clone(),
            }),
        },
    };
    let response = app.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &update_ask,
        &[coin(MIN_EXPIRY_REWARD - 1u128, NATIVE_DENOM)],
    );
    assert_error(
        response,
        ContractError::InsufficientFunds("expiry reward".to_string()).to_string(),
    );

    // Setting expiry with sufficient funds succeeds
    let owner_balances_0 = NativeBalance(app.wrap().query_all_balances(owner.clone()).unwrap());
    let expiry_reward = coin(MIN_EXPIRY_REWARD, NATIVE_DENOM);
    let ask_id = generate_id(vec![collection.as_bytes(), token_ids[0_usize].as_bytes()]);
    let update_ask = ExecuteMsg::UpdateAsk {
        id: ask_id.clone(),
        details: OrderDetails {
            price: new_price.clone(),
            recipient: None,
            finder: None,
            expiry: Some(Expiry {
                timestamp: app.block_info().time.plus_seconds(100),
                reward: expiry_reward.clone(),
            }),
        },
    };
    let response = app.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &update_ask,
        &[expiry_reward.clone()],
    );
    assert!(response.is_ok());

    let owner_balances_1 = NativeBalance(app.wrap().query_all_balances(owner.clone()).unwrap());
    assert_eq!(
        owner_balances_0.sub(expiry_reward).unwrap(),
        owner_balances_1
    );

    // Increasing expiry reward succeeds
    let expiry_reward = coin(MIN_EXPIRY_REWARD + 1_000, NATIVE_DENOM);
    let ask_id = generate_id(vec![collection.as_bytes(), token_ids[0_usize].as_bytes()]);
    let update_ask = ExecuteMsg::UpdateAsk {
        id: ask_id.clone(),
        details: OrderDetails {
            price: new_price.clone(),
            recipient: None,
            finder: None,
            expiry: Some(Expiry {
                timestamp: app.block_info().time.plus_seconds(100),
                reward: expiry_reward.clone(),
            }),
        },
    };
    let response = app.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &update_ask,
        &[coin(1_000, NATIVE_DENOM).clone()],
    );
    assert!(response.is_ok());

    let owner_balances_2 = NativeBalance(app.wrap().query_all_balances(owner.clone()).unwrap());
    assert_eq!(
        owner_balances_1.sub(coin(1_000, NATIVE_DENOM)).unwrap(),
        owner_balances_2
    );

    // Lowering expiry reward succeeds
    let expiry_reward = coin(MIN_EXPIRY_REWARD, NATIVE_DENOM);
    let ask_id = generate_id(vec![collection.as_bytes(), token_ids[0_usize].as_bytes()]);
    let update_ask = ExecuteMsg::UpdateAsk {
        id: ask_id.clone(),
        details: OrderDetails {
            price: new_price.clone(),
            recipient: None,
            finder: None,
            expiry: Some(Expiry {
                timestamp: app.block_info().time.plus_seconds(100),
                reward: expiry_reward.clone(),
            }),
        },
    };
    let response = app.execute_contract(owner.clone(), marketplace.clone(), &update_ask, &[]);
    assert!(response.is_ok());

    let owner_balances_3 = NativeBalance(app.wrap().query_all_balances(owner.clone()).unwrap());
    assert_eq!(
        owner_balances_2.add(coin(1_000, NATIVE_DENOM)),
        owner_balances_3
    );
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
            OrderDetails {
                price: coin(1000000 + idx as u128, NATIVE_DENOM),
                recipient: None,
                finder: None,
                expiry: None,
            },
        );
        token_ids.push(token_id.clone());
    }

    // Removing ask as non creator fails
    let ask_id = generate_id(vec![collection.as_bytes(), token_ids[0_usize].as_bytes()]);
    let remove_ask = ExecuteMsg::RemoveAsk {
        id: ask_id.clone(),
        reward_recipient: None,
    };
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

#[test]
pub fn try_remove_expired_ask() {
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

    let expiry_timestamp = app.block_info().time.plus_seconds(100);
    let token_id = "1".to_string();
    mint_and_set_ask(
        &mut app,
        &creator,
        &owner,
        &marketplace,
        &collection,
        &token_id,
        OrderDetails {
            price: coin(1000000u128, NATIVE_DENOM),
            recipient: None,
            finder: None,
            expiry: Some(Expiry {
                timestamp: expiry_timestamp,
                reward: coin(MIN_EXPIRY_REWARD, NATIVE_DENOM),
            }),
        },
    );

    // Removing ask before expiry fails
    let reward_recipient = Addr::unchecked("reward_recipient");
    let ask_id = generate_id(vec![collection.as_bytes(), token_id.as_bytes()]);
    let remove_ask = ExecuteMsg::RemoveAsk {
        id: ask_id.clone(),
        reward_recipient: Some(reward_recipient.to_string()),
    };
    let response = app.execute_contract(bidder.clone(), marketplace.clone(), &remove_ask, &[]);
    assert_error(
        response,
        MarketplaceStdError::Unauthorized(
            "only the creator of ask can perform this action".to_string(),
        )
        .to_string(),
    );

    // Removing ask after expiry succeeds, and reward is sent to reward recipient
    app.update_block(|block| {
        block.time = expiry_timestamp.plus_seconds(100);
    });
    let response = app.execute_contract(bidder.clone(), marketplace.clone(), &remove_ask, &[]);
    assert!(response.is_ok());

    let reward_balance = app.wrap().query_all_balances(reward_recipient).unwrap();
    assert_eq!(reward_balance, vec![coin(MIN_EXPIRY_REWARD, NATIVE_DENOM)]);
}
