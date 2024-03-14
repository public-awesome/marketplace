use crate::{
    msg::{
        CollectionOffersByCollectionOffset, CollectionOffersByCreatorOffset,
        CollectionOffersByExpirationOffset, CollectionOffersByPriceOffset, ExecuteMsg,
        OrderOptions, QueryMsg,
    },
    state::{CollectionOffer, Config, ExpirationInfo, PriceRange},
    testing::setup::{
        msg::MarketAccounts,
        setup_accounts::setup_additional_account,
        setup_marketplace::ATOM_DENOM,
        templates::{marketplace_v2_template, MarketplaceV2Template, TestContracts},
    },
};

use cosmwasm_std::coin;
use cosmwasm_std::Addr;
use cw_multi_test::Executor;
use sg_index_query::{QueryBound, QueryOptions};
use sg_std::{GENESIS_MINT_START_TIME, NATIVE_DENOM};
use test_suite::common_setup::{
    msg::MinterTemplateResponse, setup_accounts_and_block::setup_block_time,
};

#[test]
fn try_query_collection_offers_by_collection() {
    let MarketplaceV2Template {
        minter_template:
            MinterTemplateResponse {
                mut router,
                accts: MarketAccounts { .. },
                ..
            },
        contracts:
            TestContracts {
                collection,
                marketplace,
                ..
            },
    } = marketplace_v2_template(10_000);

    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);
    let _block_time = router.block_info().time;

    let native_denom_price_range: PriceRange = router
        .wrap()
        .query_wasm_smart(
            &marketplace,
            &QueryMsg::PriceRange {
                denom: NATIVE_DENOM.to_string(),
            },
        )
        .unwrap();

    let num_orders: u8 = 4;
    for idx in 1..(num_orders + 1) {
        let collection_bidder =
            setup_additional_account(&mut router, &format!("collection-bidder-{}", idx)).unwrap();
        let collection_offer_price = coin(
            native_denom_price_range.min.u128() + idx as u128,
            NATIVE_DENOM,
        );
        let set_collection_offer = ExecuteMsg::SetCollectionOffer {
            collection: collection.to_string(),
            price: collection_offer_price.clone(),
            order_options: None,
        };
        let response = router.execute_contract(
            collection_bidder.clone(),
            marketplace.clone(),
            &set_collection_offer,
            &[collection_offer_price.clone()],
        );
        assert!(response.is_ok());
    }

    // Other collection address returns no collection offers
    let dummy_collection = Addr::unchecked("dummy_collection");
    let collection_offers = router
        .wrap()
        .query_wasm_smart::<Vec<CollectionOffer>>(
            &marketplace,
            &QueryMsg::CollectionOffersByCollection {
                collection: dummy_collection.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(collection_offers.len(), 0);

    // Correct number of offers returned for collection
    let collection_offers = router
        .wrap()
        .query_wasm_smart::<Vec<CollectionOffer>>(
            &marketplace,
            &QueryMsg::CollectionOffersByCollection {
                collection: collection.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(collection_offers.len(), num_orders as usize);

    // Query Options work
    let collection_offers = router
        .wrap()
        .query_wasm_smart::<Vec<CollectionOffer>>(
            &marketplace,
            &QueryMsg::CollectionOffersByCollection {
                collection: collection.to_string(),
                query_options: Some(QueryOptions {
                    descending: Some(true),
                    limit: None,
                    min: Some(QueryBound::Exclusive(CollectionOffersByCollectionOffset {
                        creator: "collection-bidder-2".to_string(),
                    })),
                    max: Some(QueryBound::Inclusive(CollectionOffersByCollectionOffset {
                        creator: "collection-bidder-4".to_string(),
                    })),
                }),
            },
        )
        .unwrap();
    assert_eq!(collection_offers.len(), 2);
    assert_eq!(
        collection_offers[0].order_info.creator,
        "collection-bidder-4".to_string()
    );
    assert_eq!(
        collection_offers[1].order_info.creator,
        "collection-bidder-3".to_string()
    );
}

#[test]
fn try_query_collection_offers_by_token_price() {
    let MarketplaceV2Template {
        minter_template:
            MinterTemplateResponse {
                mut router,
                accts: MarketAccounts { .. },
                ..
            },
        contracts:
            TestContracts {
                collection,
                marketplace,
                ..
            },
    } = marketplace_v2_template(10_000);

    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);
    let _block_time = router.block_info().time;

    let native_denom_price_range: PriceRange = router
        .wrap()
        .query_wasm_smart(
            &marketplace,
            &QueryMsg::PriceRange {
                denom: NATIVE_DENOM.to_string(),
            },
        )
        .unwrap();

    let num_orders: u8 = 4;
    for idx in 1..(num_orders + 1) {
        let collection_bidder =
            setup_additional_account(&mut router, &format!("collection-bidder-{}", idx)).unwrap();
        let collection_offer_price = coin(
            native_denom_price_range.min.u128() + idx as u128,
            NATIVE_DENOM,
        );
        let set_collection_offer = ExecuteMsg::SetCollectionOffer {
            collection: collection.to_string(),
            price: collection_offer_price.clone(),
            order_options: None,
        };
        let response = router.execute_contract(
            collection_bidder.clone(),
            marketplace.clone(),
            &set_collection_offer,
            &[collection_offer_price.clone()],
        );
        assert!(response.is_ok());
    }

    // Other collection address returns no collection offers
    let dummy_collection = Addr::unchecked("dummy_collection");
    let collection_offers = router
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
    let collection_offers = router
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
    let collection_offers = router
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
    assert_eq!(collection_offers.len(), num_orders as usize);

    // Query Options work
    let collection_offers = router
        .wrap()
        .query_wasm_smart::<Vec<CollectionOffer>>(
            &marketplace,
            &QueryMsg::CollectionOffersByPrice {
                collection: collection.to_string(),
                denom: NATIVE_DENOM.to_string(),
                query_options: Some(QueryOptions {
                    descending: Some(true),
                    limit: None,
                    min: Some(QueryBound::Exclusive(CollectionOffersByPriceOffset {
                        amount: native_denom_price_range.min.u128() + 2,
                        creator: "".to_string(),
                    })),
                    max: Some(QueryBound::Exclusive(CollectionOffersByPriceOffset {
                        amount: native_denom_price_range.min.u128() + 5,
                        creator: "".to_string(),
                    })),
                }),
            },
        )
        .unwrap();
    assert_eq!(collection_offers.len(), 3);
    assert_eq!(
        collection_offers[0].order_info.price,
        coin(native_denom_price_range.min.u128() + 4, NATIVE_DENOM)
    );
    assert_eq!(
        collection_offers[1].order_info.price,
        coin(native_denom_price_range.min.u128() + 3, NATIVE_DENOM)
    );
    assert_eq!(
        collection_offers[2].order_info.price,
        coin(native_denom_price_range.min.u128() + 2, NATIVE_DENOM)
    );
}

#[test]
fn try_query_collection_offers_by_creator() {
    let MarketplaceV2Template {
        minter_template:
            MinterTemplateResponse {
                mut router,
                accts: MarketAccounts { .. },
                ..
            },
        contracts:
            TestContracts {
                collection,
                marketplace,
                ..
            },
    } = marketplace_v2_template(10_000);

    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);
    let _block_time = router.block_info().time;

    let native_denom_price_range: PriceRange = router
        .wrap()
        .query_wasm_smart(
            &marketplace,
            &QueryMsg::PriceRange {
                denom: NATIVE_DENOM.to_string(),
            },
        )
        .unwrap();

    let num_orders: u8 = 4;
    for idx in 1..(num_orders + 1) {
        let collection_bidder =
            setup_additional_account(&mut router, &format!("collection-bidder-{}", idx)).unwrap();
        let collection_offer_price = coin(
            native_denom_price_range.min.u128() + idx as u128,
            NATIVE_DENOM,
        );
        let set_collection_offer = ExecuteMsg::SetCollectionOffer {
            collection: collection.to_string(),
            price: collection_offer_price.clone(),
            order_options: None,
        };
        let response = router.execute_contract(
            collection_bidder.clone(),
            marketplace.clone(),
            &set_collection_offer,
            &[collection_offer_price.clone()],
        );
        assert!(response.is_ok());
    }

    // Other creator address returns no collection_offers
    let dummy_creator = Addr::unchecked("dummy_creator");
    let collection_offers = router
        .wrap()
        .query_wasm_smart::<Vec<CollectionOffer>>(
            &marketplace,
            &QueryMsg::CollectionOffersByCreator {
                creator: dummy_creator.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(collection_offers.len(), 0);

    // Correct number of asks returned for correct creator
    let collection_offers = router
        .wrap()
        .query_wasm_smart::<Vec<CollectionOffer>>(
            &marketplace,
            &QueryMsg::CollectionOffersByCreator {
                creator: "collection-bidder-1".to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(collection_offers.len(), 1_usize);

    // Query Options work
    let collection_offers = router
        .wrap()
        .query_wasm_smart::<Vec<CollectionOffer>>(
            &marketplace,
            &QueryMsg::CollectionOffersByCreator {
                creator: "collection-bidder-2".to_string(),
                query_options: Some(QueryOptions {
                    descending: Some(true),
                    limit: Some(2),
                    min: Some(QueryBound::Inclusive(CollectionOffersByCreatorOffset {
                        collection: "".to_string(),
                    })),
                    max: Some(QueryBound::Inclusive(CollectionOffersByCreatorOffset {
                        collection: collection.to_string(),
                    })),
                }),
            },
        )
        .unwrap();
    assert_eq!(collection_offers.len(), 1);
    assert_eq!(
        collection_offers[0].order_info.creator,
        Addr::unchecked("collection-bidder-2".to_string())
    );
    assert_eq!(collection_offers[0].collection, collection);
}

#[test]
fn try_query_collection_offers_by_expiration() {
    let MarketplaceV2Template {
        minter_template:
            MinterTemplateResponse {
                mut router,
                accts: MarketAccounts { .. },
                ..
            },
        contracts:
            TestContracts {
                collection,
                marketplace,
                ..
            },
    } = marketplace_v2_template(10_000);

    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);
    let block_time = router.block_info().time;

    let config: Config<Addr> = router
        .wrap()
        .query_wasm_smart(&marketplace, &QueryMsg::Config {})
        .unwrap();

    let native_denom_price_range: PriceRange = router
        .wrap()
        .query_wasm_smart(
            &marketplace,
            &QueryMsg::PriceRange {
                denom: NATIVE_DENOM.to_string(),
            },
        )
        .unwrap();

    let num_orders: u8 = 4;
    for idx in 1..(num_orders + 1) {
        let collection_bidder =
            setup_additional_account(&mut router, &format!("collection-bidder-{}", idx)).unwrap();
        let collection_offer_price = coin(
            native_denom_price_range.min.u128() + idx as u128,
            NATIVE_DENOM,
        );
        let set_collection_offer = ExecuteMsg::SetCollectionOffer {
            collection: collection.to_string(),
            price: collection_offer_price.clone(),
            order_options: Some(OrderOptions {
                asset_recipient: None,
                finder: None,
                finders_fee_bps: None,
                expiration_info: Some(ExpirationInfo {
                    expiration: block_time
                        .plus_seconds(config.min_expiration_seconds)
                        .plus_seconds(idx as u64),
                    removal_reward: config.min_removal_reward.clone(),
                }),
            }),
        };
        let response = router.execute_contract(
            collection_bidder.clone(),
            marketplace.clone(),
            &set_collection_offer,
            &[
                collection_offer_price.clone(),
                config.min_removal_reward.clone(),
            ],
        );
        assert!(response.is_ok());
    }

    // Correct number of collection offers returned for correct query
    let collection_offers = router
        .wrap()
        .query_wasm_smart::<Vec<CollectionOffer>>(
            &marketplace,
            &QueryMsg::CollectionOffersByExpiration {
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(collection_offers.len(), num_orders as usize);

    // Query Options work
    let collection_offers = router
        .wrap()
        .query_wasm_smart::<Vec<CollectionOffer>>(
            &marketplace,
            &QueryMsg::CollectionOffersByExpiration {
                query_options: Some(QueryOptions {
                    descending: Some(true),
                    limit: Some(2),
                    min: Some(QueryBound::Exclusive(CollectionOffersByExpirationOffset {
                        collection: "".to_string(),
                        creator: "".to_string(),
                        expiration: block_time
                            .plus_seconds(config.min_expiration_seconds)
                            .plus_seconds(3u64)
                            .seconds(),
                    })),
                    max: Some(QueryBound::Exclusive(CollectionOffersByExpirationOffset {
                        collection: "".to_string(),
                        creator: "".to_string(),
                        expiration: block_time
                            .plus_seconds(config.min_expiration_seconds)
                            .plus_seconds(5u64)
                            .seconds(),
                    })),
                }),
            },
        )
        .unwrap();

    assert_eq!(collection_offers.len(), 2);
    assert_eq!(
        collection_offers[0].order_info.creator,
        Addr::unchecked("collection-bidder-4")
    );
    assert_eq!(
        collection_offers[1].order_info.creator,
        Addr::unchecked("collection-bidder-3")
    );
}
