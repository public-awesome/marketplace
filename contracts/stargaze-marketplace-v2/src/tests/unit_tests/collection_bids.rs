use crate::{
    msg::{ExecuteMsg, QueryMsg},
    orders::{CollectionBid, Expiry, OrderDetails},
    tests::{
        helpers::utils::{assert_error, find_attrs},
        setup::{
            setup_accounts::{setup_additional_account, TestAccounts},
            setup_contracts::{JUNO_DENOM, MIN_EXPIRY_REWARD, NATIVE_DENOM},
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
            expiry: None,
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
    assert_error(
        response,
        ContractError::InsufficientFunds("collection bid price".to_string()).to_string(),
    );

    // Create collection_bid with invalid denom fails
    let collection_bid_price = coin(1_000_000, JUNO_DENOM);
    let set_collection_bid = ExecuteMsg::SetCollectionBid {
        collection: collection.to_string(),
        details: OrderDetails {
            price: collection_bid_price.clone(),
            recipient: None,
            finder: None,
            expiry: None,
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
            expiry: None,
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
            expiry: None,
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

    // Create expiring bid with reward below min fails
    let expiry_reward = coin(MIN_EXPIRY_REWARD - 1u128, NATIVE_DENOM);
    let set_bid = ExecuteMsg::SetCollectionBid {
        collection: collection.to_string(),
        details: OrderDetails {
            price: collection_bid_price.clone(),
            recipient: None,
            finder: None,
            expiry: Some(Expiry {
                timestamp: app.block_info().time.plus_seconds(100),
                reward: expiry_reward.clone(),
            }),
        },
    };
    let response = app.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_bid,
        &[
            coin(MIN_EXPIRY_REWARD - 1u128, NATIVE_DENOM),
            collection_bid_price.clone(),
        ],
    );
    assert_error(
        response,
        ContractError::InvalidInput(
            "expiry reward must be greater than or equal to min expiry reward".to_string(),
        )
        .to_string(),
    );

    // Create expiring bid with insufficient funds fails
    let expiry_reward = coin(MIN_EXPIRY_REWARD, NATIVE_DENOM);
    let set_bid = ExecuteMsg::SetCollectionBid {
        collection: collection.to_string(),
        details: OrderDetails {
            price: collection_bid_price.clone(),
            recipient: None,
            finder: None,
            expiry: Some(Expiry {
                timestamp: app.block_info().time.plus_seconds(100),
                reward: expiry_reward.clone(),
            }),
        },
    };
    let response = app.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_bid,
        &[
            coin(MIN_EXPIRY_REWARD - 1u128, NATIVE_DENOM),
            collection_bid_price.clone(),
        ],
    );
    assert_error(
        response,
        ContractError::InsufficientFunds("expiry reward".to_string()).to_string(),
    );

    // Create expiring bid with enough funds succeeds
    let expiry_reward = coin(MIN_EXPIRY_REWARD, NATIVE_DENOM);
    let set_bid = ExecuteMsg::SetCollectionBid {
        collection: collection.to_string(),
        details: OrderDetails {
            price: collection_bid_price.clone(),
            recipient: None,
            finder: None,
            expiry: Some(Expiry {
                timestamp: app.block_info().time.plus_seconds(100),
                reward: expiry_reward.clone(),
            }),
        },
    };
    let response = app.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_bid,
        &[
            coin(MIN_EXPIRY_REWARD, NATIVE_DENOM),
            collection_bid_price.clone(),
        ],
    );
    assert!(response.is_ok());
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
                expiry: None,
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
            expiry: None,
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
            expiry: None,
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

    // Setting expiry with reward below min fails
    let collection_bid = app
        .wrap()
        .query_wasm_smart::<Option<CollectionBid>>(
            &marketplace,
            &QueryMsg::CollectionBid(collection_bid_ids[1].clone()),
        )
        .unwrap()
        .unwrap();
    let collection_bid_price = collection_bid.details.price.clone();

    let expiry_reward = coin(MIN_EXPIRY_REWARD - 1u128, NATIVE_DENOM);
    let update_collection_bid = ExecuteMsg::UpdateCollectionBid {
        id: collection_bid_ids[1].clone(),
        details: OrderDetails {
            price: collection_bid_price.clone(),
            recipient: None,
            finder: None,
            expiry: Some(Expiry {
                timestamp: app.block_info().time.plus_seconds(100),
                reward: expiry_reward.clone(),
            }),
        },
    };
    let response = app.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &update_collection_bid,
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
    let update_collection_bid = ExecuteMsg::UpdateCollectionBid {
        id: collection_bid_ids[1].clone(),
        details: OrderDetails {
            price: collection_bid_price.clone(),
            recipient: None,
            finder: None,
            expiry: Some(Expiry {
                timestamp: app.block_info().time.plus_seconds(100),
                reward: expiry_reward.clone(),
            }),
        },
    };
    let response = app.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &update_collection_bid,
        &[coin(MIN_EXPIRY_REWARD - 1u128, NATIVE_DENOM)],
    );
    assert_error(
        response,
        ContractError::InsufficientFunds("expiry reward".to_string()).to_string(),
    );

    // Setting expiry with sufficient funds succeeds
    let bidder_balances_0 = NativeBalance(app.wrap().query_all_balances(bidder.clone()).unwrap());
    let expiry_reward = coin(MIN_EXPIRY_REWARD, NATIVE_DENOM);
    let update_collection_bid = ExecuteMsg::UpdateCollectionBid {
        id: collection_bid_ids[1].clone(),
        details: OrderDetails {
            price: collection_bid_price.clone(),
            recipient: None,
            finder: None,
            expiry: Some(Expiry {
                timestamp: app.block_info().time.plus_seconds(100),
                reward: expiry_reward.clone(),
            }),
        },
    };
    let response = app.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &update_collection_bid,
        &[expiry_reward.clone()],
    );
    assert!(response.is_ok());

    let bidder_balances_1 = NativeBalance(app.wrap().query_all_balances(bidder.clone()).unwrap());
    assert_eq!(
        bidder_balances_0.sub(expiry_reward).unwrap(),
        bidder_balances_1
    );

    // Increasing expiry reward succeeds
    let expiry_reward = coin(MIN_EXPIRY_REWARD + 1_000, NATIVE_DENOM);
    let update_collection_bid = ExecuteMsg::UpdateCollectionBid {
        id: collection_bid_ids[1].clone(),
        details: OrderDetails {
            price: collection_bid_price.clone(),
            recipient: None,
            finder: None,
            expiry: Some(Expiry {
                timestamp: app.block_info().time.plus_seconds(100),
                reward: expiry_reward.clone(),
            }),
        },
    };
    let response = app.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &update_collection_bid,
        &[coin(1_000, NATIVE_DENOM).clone()],
    );
    assert!(response.is_ok());

    let bidder_balances_2 = NativeBalance(app.wrap().query_all_balances(bidder.clone()).unwrap());
    assert_eq!(
        bidder_balances_1.sub(coin(1_000, NATIVE_DENOM)).unwrap(),
        bidder_balances_2
    );

    // Lowering expiry reward succeeds
    let expiry_reward = coin(MIN_EXPIRY_REWARD, NATIVE_DENOM);
    let update_collection_bid = ExecuteMsg::UpdateCollectionBid {
        id: collection_bid_ids[1].clone(),
        details: OrderDetails {
            price: collection_bid_price.clone(),
            recipient: None,
            finder: None,
            expiry: Some(Expiry {
                timestamp: app.block_info().time.plus_seconds(100),
                reward: expiry_reward.clone(),
            }),
        },
    };
    let response = app.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &update_collection_bid,
        &[],
    );
    assert!(response.is_ok());

    let bidder_balances_3 = NativeBalance(app.wrap().query_all_balances(bidder.clone()).unwrap());
    assert_eq!(
        bidder_balances_2.add(coin(1_000, NATIVE_DENOM)),
        bidder_balances_3
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
                expiry: None,
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
        reward_recipient: None,
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

#[test]
pub fn try_remove_expired_collection_bid() {
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

    let bidder_2 = setup_additional_account(&mut app, "bidder2").unwrap();

    let expiry_timestamp = app.block_info().time.plus_seconds(100);
    let collection_bid_price = coin(1000000u128, NATIVE_DENOM);
    let set_collection_bid = ExecuteMsg::SetCollectionBid {
        collection: collection.to_string(),
        details: OrderDetails {
            price: collection_bid_price.clone(),
            recipient: None,
            finder: None,
            expiry: Some(Expiry {
                timestamp: expiry_timestamp,
                reward: coin(MIN_EXPIRY_REWARD, NATIVE_DENOM),
            }),
        },
    };
    let response = app.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_collection_bid,
        &[collection_bid_price, coin(MIN_EXPIRY_REWARD, NATIVE_DENOM)],
    );
    assert!(response.is_ok());

    let collection_bid_id = find_attrs(response.unwrap(), "wasm-set-collection-bid", "id")
        .pop()
        .unwrap();

    // Removing bid before expiry fails
    let reward_recipient = Addr::unchecked("reward_recipient");
    let remove_collection_bid = ExecuteMsg::RemoveCollectionBid {
        id: collection_bid_id.clone(),
        reward_recipient: Some(reward_recipient.to_string()),
    };
    let response = app.execute_contract(
        bidder_2.clone(),
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

    // Removing bid after expiry succeeds, and reward is sent to reward recipient
    app.update_block(|block| {
        block.time = expiry_timestamp.plus_seconds(100);
    });
    let response = app.execute_contract(
        bidder_2.clone(),
        marketplace.clone(),
        &remove_collection_bid,
        &[],
    );
    assert!(response.is_ok());

    let reward_balance = app.wrap().query_all_balances(reward_recipient).unwrap();
    assert_eq!(reward_balance, vec![coin(MIN_EXPIRY_REWARD, NATIVE_DENOM)]);
}
