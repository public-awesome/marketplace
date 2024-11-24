use crate::{
    msg::{ExecuteMsg, PriceOffset, QueryMsg},
    orders::{CollectionBid, OrderDetails},
    tests::{
        helpers::utils::find_attrs,
        setup::{
            setup_accounts::{setup_additional_account, TestAccounts},
            setup_contracts::{ATOM_DENOM, NATIVE_DENOM},
            templates::{test_context, TestContext, TestContracts},
        },
    },
};

use cosmwasm_std::coin;
use cosmwasm_std::Addr;
use cw_multi_test::Executor;
use sg_index_query::{QueryBound, QueryOptions};

#[test]
fn try_query_collection_bids_by_collection() {
    let TestContext {
        mut app,
        contracts:
            TestContracts {
                marketplace,
                collection,
                ..
            },
        accounts: TestAccounts { .. },
    } = test_context();

    let num_collection_bids: u8 = 4;
    let mut collection_bid_ids: Vec<String> = vec![];
    for idx in 1..(num_collection_bids + 1) {
        let collection_bidder =
            setup_additional_account(&mut app, &format!("collection-bidder-{}", idx)).unwrap();

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
            collection_bidder.clone(),
            marketplace.clone(),
            &set_collection_bid,
            &[collection_bid_price.clone()],
        );
        assert!(response.is_ok());
        let bid_id = find_attrs(response.unwrap(), "wasm-set-collection-bid", "id")
            .pop()
            .unwrap();
        collection_bid_ids.push(bid_id);
    }

    let collection_bids = app
        .wrap()
        .query_wasm_smart::<Vec<CollectionBid>>(
            &marketplace,
            &QueryMsg::CollectionBids(collection_bid_ids.clone()),
        )
        .unwrap();
    assert_eq!(collection_bids.len(), num_collection_bids as usize);

    for (idx, bid) in collection_bids.iter().enumerate() {
        assert_eq!(bid.id, collection_bid_ids[idx]);
    }
}

#[test]
fn try_query_collection_bids_by_token_price() {
    let TestContext {
        mut app,
        contracts:
            TestContracts {
                marketplace,
                collection,
                ..
            },
        accounts: TestAccounts { .. },
    } = test_context();

    let num_collection_bids: u8 = 4;
    for idx in 1..(num_collection_bids + 1) {
        let collection_bidder =
            setup_additional_account(&mut app, &format!("collection-bidder-{}", idx)).unwrap();
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
            collection_bidder.clone(),
            marketplace.clone(),
            &set_collection_bid,
            &[collection_bid_price.clone()],
        );
        assert!(response.is_ok());
    }

    // Other collection address returns no collection bids
    let dummy_collection = Addr::unchecked("dummy_collection");
    let collection_bids = app
        .wrap()
        .query_wasm_smart::<Vec<CollectionBid>>(
            &marketplace,
            &QueryMsg::CollectionBidsByPrice {
                collection: dummy_collection.to_string(),
                denom: NATIVE_DENOM.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(collection_bids.len(), 0);

    // Other denoms returns no collection bids
    let collection_bids = app
        .wrap()
        .query_wasm_smart::<Vec<CollectionBid>>(
            &marketplace,
            &QueryMsg::CollectionBidsByPrice {
                collection: collection.to_string(),
                denom: ATOM_DENOM.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(collection_bids.len(), 0);

    // Correct number of collection bids returned for correct collection and denom
    let collection_bids = app
        .wrap()
        .query_wasm_smart::<Vec<CollectionBid>>(
            &marketplace,
            &QueryMsg::CollectionBidsByPrice {
                collection: collection.to_string(),
                denom: NATIVE_DENOM.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(collection_bids.len(), num_collection_bids as usize);

    // Query Options work
    let qo_collection_bids = app
        .wrap()
        .query_wasm_smart::<Vec<CollectionBid>>(
            &marketplace,
            &QueryMsg::CollectionBidsByPrice {
                collection: collection.to_string(),
                denom: NATIVE_DENOM.to_string(),
                query_options: Some(QueryOptions {
                    descending: Some(true),
                    limit: None,
                    min: Some(QueryBound::Exclusive(PriceOffset {
                        id: collection_bids[0].id.clone(),
                        amount: collection_bids[0].details.price.amount.u128(),
                    })),
                    max: Some(QueryBound::Exclusive(PriceOffset {
                        id: collection_bids[3].id.clone(),
                        amount: collection_bids[3].details.price.amount.u128(),
                    })),
                }),
            },
        )
        .unwrap();

    assert_eq!(qo_collection_bids.len(), 2);

    for (idx, bid) in qo_collection_bids.iter().enumerate() {
        let bid_idx = 2 - idx;
        assert_eq!(bid.id, collection_bids[bid_idx].id);
        assert_eq!(
            bid.details.price.amount.u128(),
            collection_bids[bid_idx].details.price.amount.u128()
        );
    }
}

#[test]
fn try_query_collection_bids_by_creator() {
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

    let num_collection_bids: u8 = 4;
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
            &[collection_bid_price.clone()],
        );
        assert!(response.is_ok());
    }

    // Other creator address returns no collection_bids
    let dummy_creator = Addr::unchecked("dummy_creator");
    let collection_bids = app
        .wrap()
        .query_wasm_smart::<Vec<CollectionBid>>(
            &marketplace,
            &QueryMsg::CollectionBidsByCreatorCollection {
                creator: dummy_creator.to_string(),
                collection: collection.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(collection_bids.len(), 0);

    // Correct number of asks returned for correct creator
    let collection_bids = app
        .wrap()
        .query_wasm_smart::<Vec<CollectionBid>>(
            &marketplace,
            &QueryMsg::CollectionBidsByCreatorCollection {
                creator: bidder.to_string(),
                collection: collection.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(collection_bids.len(), num_collection_bids as usize);

    // Query Options work
    let qo_collection_bids = app
        .wrap()
        .query_wasm_smart::<Vec<CollectionBid>>(
            &marketplace,
            &QueryMsg::CollectionBidsByCreatorCollection {
                creator: bidder.to_string(),
                collection: collection.to_string(),
                query_options: Some(QueryOptions {
                    descending: Some(true),
                    limit: Some(2),
                    min: Some(QueryBound::Exclusive(collection_bids[0].id.clone())),
                    max: Some(QueryBound::Exclusive(collection_bids[3].id.clone())),
                }),
            },
        )
        .unwrap();
    assert_eq!(qo_collection_bids.len(), 2);
    assert_eq!(
        qo_collection_bids[0].creator,
        Addr::unchecked(bidder.to_string())
    );
    assert_eq!(qo_collection_bids[0].collection, collection);
}
