use crate::{
    msg::{PriceOffset, QueryMsg},
    orders::{Ask, Expiry, OrderDetails},
    tests::{
        helpers::marketplace::mint_and_set_ask,
        setup::{
            setup_accounts::TestAccounts,
            setup_contracts::{MIN_EXPIRY_REWARD, NATIVE_DENOM},
            templates::{test_context, TestContext, TestContracts},
        },
    },
};

use cosmwasm_std::coin;
use cosmwasm_std::Addr;
use sg_index_query::{QueryBound, QueryOptions};

#[test]
fn try_query_asks_by_collection() {
    let TestContext {
        mut app,
        contracts:
            TestContracts {
                marketplace,
                collection,
                ..
            },
        accounts: TestAccounts { creator, owner, .. },
    } = test_context();

    let num_nfts: u8 = 4;
    for idx in 1..(num_nfts + 1) {
        let token_id = idx.to_string();
        let price = coin(1000000u128 + idx as u128, NATIVE_DENOM);
        mint_and_set_ask(
            &mut app,
            &creator,
            &owner,
            &marketplace,
            &collection,
            &token_id.to_string(),
            OrderDetails {
                price,
                recipient: None,
                finder: None,
                expiry: None,
            },
        );
    }

    // Other collection address returns no asks
    let dummy_collection = Addr::unchecked("dummy_collection");
    let asks = app
        .wrap()
        .query_wasm_smart::<Vec<Ask>>(
            &marketplace,
            &QueryMsg::AsksByCollectionDenom {
                collection: dummy_collection.to_string(),
                denom: NATIVE_DENOM.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(asks.len(), 0);

    // Correct number of asks returned for collection
    let asks = app
        .wrap()
        .query_wasm_smart::<Vec<Ask>>(
            &marketplace,
            &QueryMsg::AsksByCollectionDenom {
                collection: collection.to_string(),
                denom: NATIVE_DENOM.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(asks.len(), num_nfts as usize);

    // Query Options work
    let asks = app
        .wrap()
        .query_wasm_smart::<Vec<Ask>>(
            &marketplace,
            &QueryMsg::AsksByCollectionDenom {
                collection: collection.to_string(),
                denom: NATIVE_DENOM.to_string(),
                query_options: Some(QueryOptions {
                    descending: Some(true),
                    limit: None,
                    min: Some(QueryBound::Exclusive(PriceOffset {
                        id: "".to_string(),
                        amount: 1000003u128,
                    })),
                    max: Some(QueryBound::Exclusive(PriceOffset {
                        id: "".to_string(),
                        amount: 1000005u128,
                    })),
                }),
            },
        )
        .unwrap();
    assert_eq!(asks.len(), 2);
    assert_eq!(asks[0].details.price.amount.u128(), 1000004u128);
    assert_eq!(asks[1].details.price.amount.u128(), 1000003u128);
}

#[test]
fn try_query_asks_by_creator() {
    let TestContext {
        mut app,
        contracts:
            TestContracts {
                marketplace,
                collection,
                ..
            },
        accounts: TestAccounts { creator, owner, .. },
    } = test_context();

    let num_nfts: u8 = 4;
    for idx in 1..(num_nfts + 1) {
        let token_id = idx.to_string();
        let price = coin(1000000u128 + idx as u128, NATIVE_DENOM);
        mint_and_set_ask(
            &mut app,
            &creator,
            &owner,
            &marketplace,
            &collection,
            &token_id.to_string(),
            OrderDetails {
                price,
                recipient: None,
                finder: None,
                expiry: None,
            },
        );
    }

    // Other creator address returns no asks
    let dummy_creator = Addr::unchecked("dummy_creator");
    let asks = app
        .wrap()
        .query_wasm_smart::<Vec<Ask>>(
            &marketplace,
            &QueryMsg::AsksByCreatorCollection {
                creator: dummy_creator.to_string(),
                collection: collection.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(asks.len(), 0);

    // Correct number of asks returned for correct creator
    let asks = app
        .wrap()
        .query_wasm_smart::<Vec<Ask>>(
            &marketplace,
            &QueryMsg::AsksByCreatorCollection {
                creator: owner.to_string(),
                collection: collection.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(asks.len(), num_nfts as usize);

    // Query Options work
    let asks = app
        .wrap()
        .query_wasm_smart::<Vec<Ask>>(
            &marketplace,
            &QueryMsg::AsksByCreatorCollection {
                creator: owner.to_string(),
                collection: collection.to_string(),
                query_options: Some(QueryOptions {
                    descending: Some(true),
                    limit: Some(2),
                    min: None,
                    max: None,
                }),
            },
        )
        .unwrap();

    assert_eq!(asks.len(), 2);
}

#[test]
fn try_query_asks_by_expiration_timestamp() {
    let TestContext {
        mut app,
        contracts:
            TestContracts {
                marketplace,
                collection,
                ..
            },
        accounts: TestAccounts { creator, owner, .. },
    } = test_context();

    let price = coin(1000000u128, NATIVE_DENOM);
    let expiry_reward = coin(MIN_EXPIRY_REWARD, NATIVE_DENOM);

    let num_nfts: u8 = 4;
    for idx in 1..(num_nfts + 1) {
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

    // Correct number of asks returned
    let asks = app
        .wrap()
        .query_wasm_smart::<Vec<Ask>>(
            &marketplace,
            &QueryMsg::AsksByExpiryTimestamp {
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(asks.len(), num_nfts as usize);

    // Query Options work
    let asks = app
        .wrap()
        .query_wasm_smart::<Vec<Ask>>(
            &marketplace,
            &QueryMsg::AsksByExpiryTimestamp {
                query_options: Some(QueryOptions {
                    descending: Some(true),
                    limit: Some(2),
                    min: None,
                    max: None,
                }),
            },
        )
        .unwrap();

    assert_eq!(asks.len(), 2);
}
