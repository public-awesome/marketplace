use crate::{
    msg::{ExecuteMsg, QueryMsg, SudoMsg},
    orders::{Ask, Bid, CollectionBid, Expiry, OrderDetails},
    state::Config,
    tests::{
        helpers::marketplace::mint_and_set_ask,
        setup::{
            setup_accounts::{setup_additional_account, TestAccounts},
            setup_contracts::{MIN_EXPIRY_REWARD, NATIVE_DENOM},
            templates::{test_context, TestContext, TestContracts},
        },
    },
};

use cosmwasm_std::{coin, to_json_binary, Addr};
use cw_multi_test::{Executor, SudoMsg as CwSudoMsg, WasmSudo};
use cw_utils::NativeBalance;
use sg_index_query::QueryOptions;
use std::ops::Add;

#[test]
fn try_sudo_end_block() {
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

    let bidder_2 = setup_additional_account(&mut app, "bidder2").unwrap();

    let price = coin(1000000u128, NATIVE_DENOM);
    let expiry_reward = coin(MIN_EXPIRY_REWARD, NATIVE_DENOM);

    let config = app
        .wrap()
        .query_wasm_smart::<Config<String>>(&marketplace, &QueryMsg::Config {})
        .unwrap();

    let num_orders = config.max_asks_removed_per_block + 1;

    for idx in 1..(num_orders + 1) {
        let token_id = idx.to_string();
        let expiry_timestamp = app.block_info().time.plus_seconds(100 + idx as u64);
        mint_and_set_ask(
            &mut app,
            &creator,
            &owner,
            &marketplace,
            &collection,
            &token_id.to_string(),
            OrderDetails {
                price: price.clone(),
                recipient: None,
                finder: None,
                expiry: Some(Expiry {
                    timestamp: expiry_timestamp,
                    reward: expiry_reward.clone(),
                }),
            },
        );
    }
    let asks = app
        .wrap()
        .query_wasm_smart::<Vec<Ask>>(
            &marketplace,
            &QueryMsg::AsksByExpiryTimestamp {
                query_options: Some(QueryOptions {
                    limit: Some(20_u32),
                    descending: None,
                    min: None,
                    max: None,
                }),
            },
        )
        .unwrap();
    assert_eq!(asks.len() as u32, num_orders);

    let token_id = 1000.to_string();
    for idx in 1..(num_orders + 1) {
        let expiry_timestamp = app.block_info().time.plus_seconds(100 + idx as u64);
        let set_bid = ExecuteMsg::SetBid {
            collection: collection.to_string(),
            token_id: token_id.to_string(),
            details: OrderDetails {
                price: price.clone(),
                recipient: None,
                finder: None,
                expiry: Some(Expiry {
                    timestamp: expiry_timestamp,
                    reward: expiry_reward.clone(),
                }),
            },
        };
        let response = app.execute_contract(
            bidder.clone(),
            marketplace.clone(),
            &set_bid,
            &[price.clone(), expiry_reward.clone()],
        );
        assert!(response.is_ok());
    }
    let bids = app
        .wrap()
        .query_wasm_smart::<Vec<Bid>>(
            &marketplace,
            &QueryMsg::BidsByExpiryTimestamp {
                query_options: Some(QueryOptions {
                    limit: Some(20_u32),
                    descending: None,
                    min: None,
                    max: None,
                }),
            },
        )
        .unwrap();
    assert_eq!(bids.len() as u32, num_orders);

    for idx in 1..(num_orders + 1) {
        let collection = Addr::unchecked(format!("collection-{}", idx));
        let expiry_timestamp = app.block_info().time.plus_seconds(100 + idx as u64);
        let set_collection_bid = ExecuteMsg::SetCollectionBid {
            collection: collection.to_string(),
            details: OrderDetails {
                price: price.clone(),
                recipient: None,
                finder: None,
                expiry: Some(Expiry {
                    timestamp: expiry_timestamp,
                    reward: expiry_reward.clone(),
                }),
            },
        };
        let response = app.execute_contract(
            bidder_2.clone(),
            marketplace.clone(),
            &set_collection_bid,
            &[price.clone(), expiry_reward.clone()],
        );
        assert!(response.is_ok());
    }
    let collection_bids = app
        .wrap()
        .query_wasm_smart::<Vec<CollectionBid>>(
            &marketplace,
            &QueryMsg::CollectionBidsByExpiryTimestamp {
                query_options: Some(QueryOptions {
                    limit: Some(20_u32),
                    descending: None,
                    min: None,
                    max: None,
                }),
            },
        )
        .unwrap();
    assert_eq!(collection_bids.len() as u32, num_orders);

    app.update_block(|block| {
        block.time = block.time.plus_seconds(110);
    });

    let fee_manager_balance_before = NativeBalance(
        app.wrap()
            .query_all_balances(config.fee_manager.clone())
            .unwrap(),
    );

    let response = app.sudo(CwSudoMsg::Wasm(WasmSudo {
        contract_addr: marketplace.clone(),
        msg: to_json_binary(&SudoMsg::EndBlock {}).unwrap(),
    }));
    assert!(response.is_ok());

    let fee_manager_balance_after = NativeBalance(
        app.wrap()
            .query_all_balances(config.fee_manager.clone())
            .unwrap(),
    );

    let asks = app
        .wrap()
        .query_wasm_smart::<Vec<Ask>>(
            &marketplace,
            &QueryMsg::AsksByExpiryTimestamp {
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(asks.len(), 1);

    let bids = app
        .wrap()
        .query_wasm_smart::<Vec<Bid>>(
            &marketplace,
            &QueryMsg::BidsByExpiryTimestamp {
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(bids.len(), 1);

    let collection_bids = app
        .wrap()
        .query_wasm_smart::<Vec<CollectionBid>>(
            &marketplace,
            &QueryMsg::CollectionBidsByExpiryTimestamp {
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(collection_bids.len(), 1);

    assert_eq!(
        fee_manager_balance_before.add(coin(
            MIN_EXPIRY_REWARD * 3 * (num_orders - 1) as u128,
            NATIVE_DENOM
        )),
        fee_manager_balance_after
    );
}
