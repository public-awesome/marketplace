use crate::{
    msg::{ExecuteMsg, PriceOffset, QueryMsg},
    orders::{Offer, OrderDetails},
    tests::{
        helpers::utils::find_attrs,
        setup::{
            setup_accounts::TestAccounts,
            setup_contracts::{JUNO_DENOM, NATIVE_DENOM},
            templates::{test_context, TestContext, TestContracts},
        },
    },
};

use cosmwasm_std::coin;
use cosmwasm_std::Addr;
use cw_multi_test::Executor;
use sg_index_query::{QueryBound, QueryOptions};

#[test]
fn try_query_offers() {
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

    let num_offers: u8 = 4;
    let token_id = "1".to_string();
    let mut offer_ids: Vec<String> = vec![];
    for idx in 1..(num_offers + 1) {
        let offer_price = coin(1000000u128 + idx as u128, NATIVE_DENOM);
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
            &[offer_price],
        );
        assert!(response.is_ok());

        let offer_id = find_attrs(response.unwrap(), "wasm-set-offer", "id")
            .pop()
            .unwrap();
        offer_ids.push(offer_id);
    }

    let offers = app
        .wrap()
        .query_wasm_smart::<Vec<Offer>>(&marketplace, &QueryMsg::Offers(offer_ids.clone()))
        .unwrap();
    assert_eq!(offers.len(), num_offers as usize);

    for (idx, offer) in offers.iter().enumerate() {
        assert_eq!(offer.id, offer_ids[idx]);
    }
}

#[test]
fn try_query_offers_by_token_price() {
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

    let num_offers: u8 = 4;
    let token_id = "1".to_string();
    let mut offer_ids: Vec<String> = vec![];
    for idx in 1..(num_offers + 1) {
        let offer_price = coin(1000000u128 + idx as u128, NATIVE_DENOM);
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
            &[offer_price],
        );
        assert!(response.is_ok());

        let offer_id = find_attrs(response.unwrap(), "wasm-set-offer", "id")
            .pop()
            .unwrap();
        offer_ids.push(offer_id);
    }

    // Other collection returns no offers
    let dummy_collection = Addr::unchecked("dummy_collection");
    let offers = app
        .wrap()
        .query_wasm_smart::<Vec<Offer>>(
            &marketplace,
            &QueryMsg::OffersByTokenPrice {
                collection: dummy_collection.to_string(),
                token_id: "1".to_string(),
                denom: NATIVE_DENOM.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(offers.len(), 0);

    // Other token id returns no offers
    let offers = app
        .wrap()
        .query_wasm_smart::<Vec<Offer>>(
            &marketplace,
            &QueryMsg::OffersByTokenPrice {
                collection: collection.to_string(),
                token_id: "2".to_string(),
                denom: NATIVE_DENOM.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(offers.len(), 0);

    // Other denoms returns no offers
    let offers = app
        .wrap()
        .query_wasm_smart::<Vec<Offer>>(
            &marketplace,
            &QueryMsg::OffersByTokenPrice {
                collection: collection.to_string(),
                token_id: "1".to_string(),
                denom: JUNO_DENOM.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(offers.len(), 0);

    // Correct number of offers returned for correct token_id and denom
    let offers = app
        .wrap()
        .query_wasm_smart::<Vec<Offer>>(
            &marketplace,
            &QueryMsg::OffersByTokenPrice {
                collection: collection.to_string(),
                token_id: "1".to_string(),
                denom: NATIVE_DENOM.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(offers.len(), num_offers as usize);

    // Query Options work
    let qo_offers = app
        .wrap()
        .query_wasm_smart::<Vec<Offer>>(
            &marketplace,
            &QueryMsg::OffersByTokenPrice {
                collection: collection.to_string(),
                token_id: "1".to_string(),
                denom: NATIVE_DENOM.to_string(),
                query_options: Some(QueryOptions {
                    descending: Some(true),
                    limit: None,
                    min: Some(QueryBound::Exclusive(PriceOffset {
                        id: offers[0].id.clone(),
                        amount: offers[0].details.price.amount.u128(),
                    })),
                    max: Some(QueryBound::Exclusive(PriceOffset {
                        id: offers[3].id.clone(),
                        amount: offers[3].details.price.amount.u128(),
                    })),
                }),
            },
        )
        .unwrap();

    assert_eq!(qo_offers.len(), 2);

    for (idx, offer) in qo_offers.iter().enumerate() {
        let offer_idx = 2 - idx;
        assert_eq!(offer.id, offers[offer_idx].id);
        assert_eq!(
            offer.details.price.amount.u128(),
            offers[offer_idx].details.price.amount.u128()
        );
    }
}

#[test]
fn try_query_offers_by_creator() {
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

    let num_offers: u8 = 4;
    let token_id = "1".to_string();
    let mut offer_ids: Vec<String> = vec![];
    for idx in 1..(num_offers + 1) {
        let offer_price = coin(1000000u128 + idx as u128, NATIVE_DENOM);
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
            &[offer_price],
        );
        assert!(response.is_ok());

        let offer_id = find_attrs(response.unwrap(), "wasm-set-offer", "id")
            .pop()
            .unwrap();
        offer_ids.push(offer_id);
    }

    // Other creator address returns no offers
    let offers = app
        .wrap()
        .query_wasm_smart::<Vec<Offer>>(
            &marketplace,
            &QueryMsg::OffersByCreatorCollection {
                creator: owner.to_string(),
                collection: collection.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(offers.len(), 0);

    // Correct number of offers returned for correct creator
    let offers = app
        .wrap()
        .query_wasm_smart::<Vec<Offer>>(
            &marketplace,
            &QueryMsg::OffersByCreatorCollection {
                creator: bidder.to_string(),
                collection: collection.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(offers.len(), num_offers as usize);

    // Query Options work
    let qo_offers = app
        .wrap()
        .query_wasm_smart::<Vec<Offer>>(
            &marketplace,
            &QueryMsg::OffersByCreatorCollection {
                creator: bidder.to_string(),
                collection: collection.to_string(),
                query_options: Some(QueryOptions {
                    descending: Some(true),
                    limit: Some(2),
                    min: Some(QueryBound::Exclusive(offers[0].id.clone())),
                    max: Some(QueryBound::Exclusive(offers[3].id.clone())),
                }),
            },
        )
        .unwrap();

    assert_eq!(qo_offers.len(), 2);

    for (idx, offer) in qo_offers.iter().enumerate() {
        let offer_idx = 2 - idx;
        assert_eq!(offer.id, offers[offer_idx].id);
        assert_eq!(
            offer.details.price.amount.u128(),
            offers[offer_idx].details.price.amount.u128()
        );
    }
}
