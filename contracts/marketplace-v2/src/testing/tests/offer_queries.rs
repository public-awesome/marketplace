use crate::{
    msg::{
        ExecuteMsg, OffersByCollectionOffset, OffersByCreatorOffset, OffersByExpirationOffset,
        OffersByTokenPriceOffset, OrderOptions, QueryMsg,
    },
    state::{Config, ExpirationInfo, Offer, PriceRange},
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
fn try_query_offers_by_collection() {
    let MarketplaceV2Template {
        minter_template:
            MinterTemplateResponse {
                mut router,
                accts: MarketAccounts { bidder, .. },
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
        let token_id = idx.to_string();
        let offer_price = coin(
            native_denom_price_range.min.u128() + idx as u128,
            NATIVE_DENOM,
        );
        let set_offer = ExecuteMsg::SetOffer {
            collection: collection.to_string(),
            token_id: token_id.to_string(),
            price: offer_price.clone(),
            order_options: None,
        };
        let response = router.execute_contract(
            bidder.clone(),
            marketplace.clone(),
            &set_offer,
            &[offer_price.clone()],
        );
        assert!(response.is_ok());
    }

    // Other collection address returns no offers
    let dummy_collection = Addr::unchecked("dummy_collection");
    let offers = router
        .wrap()
        .query_wasm_smart::<Vec<Offer>>(
            &marketplace,
            &QueryMsg::OffersByCollection {
                collection: dummy_collection.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(offers.len(), 0);

    // Correct number of offers returned for collection
    let offers = router
        .wrap()
        .query_wasm_smart::<Vec<Offer>>(
            &marketplace,
            &QueryMsg::OffersByCollection {
                collection: collection.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(offers.len(), num_orders as usize);

    // Query Options work
    let offers = router
        .wrap()
        .query_wasm_smart::<Vec<Offer>>(
            &marketplace,
            &QueryMsg::OffersByCollection {
                collection: collection.to_string(),
                query_options: Some(QueryOptions {
                    descending: Some(true),
                    limit: None,
                    min: Some(QueryBound::Exclusive(OffersByCollectionOffset {
                        token_id: "3".to_string(),
                        creator: "".to_string(),
                    })),
                    max: Some(QueryBound::Exclusive(OffersByCollectionOffset {
                        token_id: "5".to_string(),
                        creator: "".to_string(),
                    })),
                }),
            },
        )
        .unwrap();
    assert_eq!(offers.len(), 2);
    assert_eq!(offers[0].token_id, "4".to_string());
    assert_eq!(offers[1].token_id, "3".to_string());
}

#[test]
fn try_query_offers_by_token_price() {
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
    let token_id = "1".to_string();
    for idx in 1..(num_orders + 1) {
        let token_bidder =
            setup_additional_account(&mut router, &format!("bidder-{}", idx)).unwrap();

        let offer_price = coin(
            native_denom_price_range.min.u128() + idx as u128,
            NATIVE_DENOM,
        );
        let set_offer = ExecuteMsg::SetOffer {
            collection: collection.to_string(),
            token_id: token_id.to_string(),
            price: offer_price.clone(),
            order_options: None,
        };
        let response = router.execute_contract(
            token_bidder.clone(),
            marketplace.clone(),
            &set_offer,
            &[offer_price.clone()],
        );
        assert!(response.is_ok());
    }

    // Other collection address returns no offers
    let dummy_collection = Addr::unchecked("dummy_collection");
    let offers = router
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

    // Other token_ids address returns no offers
    let offers = router
        .wrap()
        .query_wasm_smart::<Vec<Offer>>(
            &marketplace,
            &QueryMsg::OffersByTokenPrice {
                collection: collection.to_string(),
                token_id: "5".to_string(),
                denom: NATIVE_DENOM.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(offers.len(), 0);

    // Other denoms returns no offers
    let offers = router
        .wrap()
        .query_wasm_smart::<Vec<Offer>>(
            &marketplace,
            &QueryMsg::OffersByTokenPrice {
                collection: collection.to_string(),
                token_id: "5".to_string(),
                denom: ATOM_DENOM.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(offers.len(), 0);

    // Correct number of offers returned for correct collection, token_id, and denom
    let offers = router
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
    assert_eq!(offers.len(), num_orders as usize);

    // Query Options work
    let offers = router
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
                    min: Some(QueryBound::Exclusive(OffersByTokenPriceOffset {
                        amount: native_denom_price_range.min.u128() + 2,
                        creator: "".to_string(),
                    })),
                    max: Some(QueryBound::Exclusive(OffersByTokenPriceOffset {
                        amount: native_denom_price_range.min.u128() + 5,
                        creator: "".to_string(),
                    })),
                }),
            },
        )
        .unwrap();
    assert_eq!(offers.len(), 3);
    assert_eq!(
        offers[0].order_info.price,
        coin(native_denom_price_range.min.u128() + 4, NATIVE_DENOM)
    );
    assert_eq!(
        offers[1].order_info.price,
        coin(native_denom_price_range.min.u128() + 3, NATIVE_DENOM)
    );
    assert_eq!(
        offers[2].order_info.price,
        coin(native_denom_price_range.min.u128() + 2, NATIVE_DENOM)
    );
}

#[test]
fn try_query_offers_by_creator() {
    let MarketplaceV2Template {
        minter_template:
            MinterTemplateResponse {
                mut router,
                accts: MarketAccounts { bidder, .. },
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
        let token_id = idx.to_string();
        let offer_price = coin(
            native_denom_price_range.min.u128() + idx as u128,
            NATIVE_DENOM,
        );
        let set_offer = ExecuteMsg::SetOffer {
            collection: collection.to_string(),
            token_id: token_id.to_string(),
            price: offer_price.clone(),
            order_options: None,
        };
        let response = router.execute_contract(
            bidder.clone(),
            marketplace.clone(),
            &set_offer,
            &[offer_price.clone()],
        );
        assert!(response.is_ok());
    }

    // Other creator address returns no offers
    let dummy_creator = Addr::unchecked("dummy_creator");
    let offers = router
        .wrap()
        .query_wasm_smart::<Vec<Offer>>(
            &marketplace,
            &QueryMsg::OffersByCreator {
                creator: dummy_creator.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(offers.len(), 0);

    // Correct number of offers returned for correct creator
    let offers = router
        .wrap()
        .query_wasm_smart::<Vec<Offer>>(
            &marketplace,
            &QueryMsg::OffersByCreator {
                creator: bidder.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(offers.len(), num_orders as usize);

    // Query Options work
    let offers = router
        .wrap()
        .query_wasm_smart::<Vec<Offer>>(
            &marketplace,
            &QueryMsg::OffersByCreator {
                creator: bidder.to_string(),
                query_options: Some(QueryOptions {
                    descending: Some(true),
                    limit: Some(2),
                    min: Some(QueryBound::Inclusive(OffersByCreatorOffset {
                        collection: "".to_string(),
                        token_id: "".to_string(),
                    })),
                    max: Some(QueryBound::Exclusive(OffersByCreatorOffset {
                        collection: collection.to_string(),
                        token_id: "5".to_string(),
                    })),
                }),
            },
        )
        .unwrap();
    assert_eq!(offers.len(), 2);
    assert_eq!(offers[0].token_id, "4".to_string());
    assert_eq!(offers[1].token_id, "3".to_string());
}

#[test]
fn try_query_offers_by_expiration() {
    let MarketplaceV2Template {
        minter_template:
            MinterTemplateResponse {
                mut router,
                accts: MarketAccounts { bidder, .. },
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
        let token_id = idx.to_string();
        let offer_price = coin(
            native_denom_price_range.min.u128() + idx as u128,
            NATIVE_DENOM,
        );
        let set_offer = ExecuteMsg::SetOffer {
            collection: collection.to_string(),
            token_id: token_id.to_string(),
            price: offer_price.clone(),
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
            bidder.clone(),
            marketplace.clone(),
            &set_offer,
            &[offer_price.clone(), config.min_removal_reward.clone()],
        );
        assert!(response.is_ok());
    }

    // Correct number of offers returned for correct query
    let offers = router
        .wrap()
        .query_wasm_smart::<Vec<Offer>>(
            &marketplace,
            &QueryMsg::OffersByExpiration {
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(offers.len(), num_orders as usize);

    // Query Options work
    let offers = router
        .wrap()
        .query_wasm_smart::<Vec<Offer>>(
            &marketplace,
            &QueryMsg::OffersByExpiration {
                query_options: Some(QueryOptions {
                    descending: Some(true),
                    limit: Some(2),
                    min: Some(QueryBound::Exclusive(OffersByExpirationOffset {
                        collection: "".to_string(),
                        token_id: "".to_string(),
                        creator: "".to_string(),
                        expiration: block_time
                            .plus_seconds(config.min_expiration_seconds)
                            .plus_seconds(3u64)
                            .seconds(),
                    })),
                    max: Some(QueryBound::Exclusive(OffersByExpirationOffset {
                        collection: "".to_string(),
                        token_id: "".to_string(),
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

    assert_eq!(offers.len(), 2);
    assert_eq!(offers[0].token_id, "4".to_string());
    assert_eq!(offers[1].token_id, "3".to_string());
}
