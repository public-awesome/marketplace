use crate::{
    msg::{ExecuteMsg, PriceOffset, QueryMsg},
    orders::{CollectionOffer, OrderDetails},
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
fn try_query_collection_offers_by_collection() {
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

    let num_collection_offers: u8 = 4;
    let mut collection_offer_ids: Vec<String> = vec![];
    for idx in 1..(num_collection_offers + 1) {
        let collection_bidder =
            setup_additional_account(&mut app, &format!("collection-bidder-{}", idx)).unwrap();

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
            collection_bidder.clone(),
            marketplace.clone(),
            &set_collection_offer,
            &[collection_offer_price.clone()],
        );
        assert!(response.is_ok());
        let offer_id = find_attrs(response.unwrap(), "wasm-set-collection-offer", "id")
            .pop()
            .unwrap();
        collection_offer_ids.push(offer_id);
    }

    let collection_offers = app
        .wrap()
        .query_wasm_smart::<Vec<CollectionOffer>>(
            &marketplace,
            &QueryMsg::CollectionOffers(collection_offer_ids.clone()),
        )
        .unwrap();
    assert_eq!(collection_offers.len(), num_collection_offers as usize);

    for (idx, offer) in collection_offers.iter().enumerate() {
        assert_eq!(offer.id, collection_offer_ids[idx]);
    }
}

#[test]
fn try_query_collection_offers_by_token_price() {
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

    let num_collection_offers: u8 = 4;
    for idx in 1..(num_collection_offers + 1) {
        let collection_bidder =
            setup_additional_account(&mut app, &format!("collection-bidder-{}", idx)).unwrap();
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
            collection_bidder.clone(),
            marketplace.clone(),
            &set_collection_offer,
            &[collection_offer_price.clone()],
        );
        assert!(response.is_ok());
    }

    // Other collection address returns no collection offers
    let dummy_collection = Addr::unchecked("dummy_collection");
    let collection_offers = app
        .wrap()
        .query_wasm_smart::<Vec<CollectionOffer>>(
            &marketplace,
            &QueryMsg::CollectionOffersByPrice {
                collection: dummy_collection.to_string(),
                denom: NATIVE_DENOM.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(collection_offers.len(), 0);

    // Other denoms returns no collection offers
    let collection_offers = app
        .wrap()
        .query_wasm_smart::<Vec<CollectionOffer>>(
            &marketplace,
            &QueryMsg::CollectionOffersByPrice {
                collection: collection.to_string(),
                denom: ATOM_DENOM.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(collection_offers.len(), 0);

    // Correct number of collection offers returned for correct collection and denom
    let collection_offers = app
        .wrap()
        .query_wasm_smart::<Vec<CollectionOffer>>(
            &marketplace,
            &QueryMsg::CollectionOffersByPrice {
                collection: collection.to_string(),
                denom: NATIVE_DENOM.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(collection_offers.len(), num_collection_offers as usize);

    // Query Options work
    let qo_collection_offers = app
        .wrap()
        .query_wasm_smart::<Vec<CollectionOffer>>(
            &marketplace,
            &QueryMsg::CollectionOffersByPrice {
                collection: collection.to_string(),
                denom: NATIVE_DENOM.to_string(),
                query_options: Some(QueryOptions {
                    descending: Some(true),
                    limit: None,
                    min: Some(QueryBound::Exclusive(PriceOffset {
                        id: collection_offers[0].id.clone(),
                        amount: collection_offers[0].details.price.amount.u128(),
                    })),
                    max: Some(QueryBound::Exclusive(PriceOffset {
                        id: collection_offers[3].id.clone(),
                        amount: collection_offers[3].details.price.amount.u128(),
                    })),
                }),
            },
        )
        .unwrap();

    assert_eq!(qo_collection_offers.len(), 2);

    for (idx, offer) in qo_collection_offers.iter().enumerate() {
        let offer_idx = 2 - idx;
        assert_eq!(offer.id, collection_offers[offer_idx].id);
        assert_eq!(
            offer.details.price.amount.u128(),
            collection_offers[offer_idx].details.price.amount.u128()
        );
    }
}

#[test]
fn try_query_collection_offers_by_creator() {
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

    let num_collection_offers: u8 = 4;
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
            &[collection_offer_price.clone()],
        );
        assert!(response.is_ok());
    }

    // Other creator address returns no collection_offers
    let dummy_creator = Addr::unchecked("dummy_creator");
    let collection_offers = app
        .wrap()
        .query_wasm_smart::<Vec<CollectionOffer>>(
            &marketplace,
            &QueryMsg::CollectionOffersByCreatorCollection {
                creator: dummy_creator.to_string(),
                collection: collection.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(collection_offers.len(), 0);

    // Correct number of asks returned for correct creator
    let collection_offers = app
        .wrap()
        .query_wasm_smart::<Vec<CollectionOffer>>(
            &marketplace,
            &QueryMsg::CollectionOffersByCreatorCollection {
                creator: bidder.to_string(),
                collection: collection.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(collection_offers.len(), num_collection_offers as usize);

    // Query Options work
    let qo_collection_offers = app
        .wrap()
        .query_wasm_smart::<Vec<CollectionOffer>>(
            &marketplace,
            &QueryMsg::CollectionOffersByCreatorCollection {
                creator: bidder.to_string(),
                collection: collection.to_string(),
                query_options: Some(QueryOptions {
                    descending: Some(true),
                    limit: Some(2),
                    min: Some(QueryBound::Exclusive(collection_offers[0].id.clone())),
                    max: Some(QueryBound::Exclusive(collection_offers[3].id.clone())),
                }),
            },
        )
        .unwrap();
    assert_eq!(qo_collection_offers.len(), 2);
    assert_eq!(
        qo_collection_offers[0].creator,
        Addr::unchecked(bidder.to_string())
    );
    assert_eq!(qo_collection_offers[0].collection, collection);
}
