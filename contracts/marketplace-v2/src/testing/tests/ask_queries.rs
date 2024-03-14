use crate::{
    msg::{AsksByCollectionOffset, AsksByCreatorOffset, AsksByExpirationOffset, AsksByPriceOffset},
    testing::setup::{msg::MarketAccounts, setup_marketplace::JUNO_DENOM},
};
use crate::{
    msg::{OrderOptions, QueryMsg},
    state::{Ask, Config, ExpirationInfo, PriceRange},
    testing::{
        helpers::marketplace::mint_and_set_ask,
        setup::templates::{marketplace_v2_template, MarketplaceV2Template, TestContracts},
    },
};

use cosmwasm_std::coin;
use cosmwasm_std::Addr;
use sg_index_query::{QueryBound, QueryOptions};
use sg_std::{GENESIS_MINT_START_TIME, NATIVE_DENOM};
use test_suite::common_setup::{
    msg::MinterTemplateResponse, setup_accounts_and_block::setup_block_time,
};

#[test]
fn try_query_asks_by_collection() {
    let MarketplaceV2Template {
        minter_template:
            MinterTemplateResponse {
                mut router,
                accts:
                    MarketAccounts {
                        creator,
                        owner,
                        bidder: _,
                    },
                ..
            },
        contracts:
            TestContracts {
                collection,
                minter,
                fair_burn: _,
                royalty_registry: _,
                marketplace,
            },
    } = marketplace_v2_template(10_000);

    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);

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

    let num_nfts: u8 = 4;
    for idx in 1..(num_nfts + 1) {
        let token_id = idx.to_string();
        let ask_price = coin(
            native_denom_price_range.min.u128() + idx as u128,
            NATIVE_DENOM,
        );
        mint_and_set_ask(
            &mut router,
            &creator,
            &owner,
            &minter,
            &marketplace,
            &collection,
            &token_id.to_string(),
            &ask_price,
            &[config.listing_fee.clone()],
            None,
        );
    }

    // Other collection address returns no asks
    let dummy_collection = Addr::unchecked("dummy_collection");
    let asks = router
        .wrap()
        .query_wasm_smart::<Vec<Ask>>(
            &marketplace,
            &QueryMsg::AsksByCollection {
                collection: dummy_collection.to_string(),
                query_options: Some(QueryOptions {
                    descending: Some(true),
                    limit: None,
                    min: Some(QueryBound::Exclusive(AsksByCollectionOffset {
                        token_id: "2".to_string(),
                    })),
                    max: Some(QueryBound::Exclusive(AsksByCollectionOffset {
                        token_id: "5".to_string(),
                    })),
                }),
            },
        )
        .unwrap();
    assert_eq!(asks.len(), 0);

    // Correct number of asks returned for collection
    let asks = router
        .wrap()
        .query_wasm_smart::<Vec<Ask>>(
            &marketplace,
            &QueryMsg::AsksByCollection {
                collection: collection.to_string(),
                query_options: Some(QueryOptions {
                    descending: None,
                    limit: None,
                    min: None,
                    max: None,
                }),
            },
        )
        .unwrap();
    assert_eq!(asks.len(), num_nfts as usize);

    // Query Options work
    let asks = router
        .wrap()
        .query_wasm_smart::<Vec<Ask>>(
            &marketplace,
            &QueryMsg::AsksByCollection {
                collection: collection.to_string(),
                query_options: Some(QueryOptions {
                    descending: Some(true),
                    limit: None,
                    min: Some(QueryBound::Exclusive(AsksByCollectionOffset {
                        token_id: "2".to_string(),
                    })),
                    max: Some(QueryBound::Exclusive(AsksByCollectionOffset {
                        token_id: "5".to_string(),
                    })),
                }),
            },
        )
        .unwrap();
    assert_eq!(asks.len(), 2);
    assert_eq!(asks[0].token_id, "4".to_string());
    assert_eq!(asks[1].token_id, "3".to_string());
}

#[test]
fn try_query_asks_by_price() {
    let MarketplaceV2Template {
        minter_template:
            MinterTemplateResponse {
                mut router,
                accts:
                    MarketAccounts {
                        creator,
                        owner,
                        bidder: _,
                    },
                ..
            },
        contracts:
            TestContracts {
                collection,
                minter,
                fair_burn: _,
                royalty_registry: _,
                marketplace,
            },
    } = marketplace_v2_template(10_000);

    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);

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

    let num_nfts: u8 = 4;
    for idx in 1..(num_nfts + 1) {
        let token_id = idx.to_string();
        let ask_price = coin(
            native_denom_price_range.min.u128() + idx as u128,
            NATIVE_DENOM,
        );
        mint_and_set_ask(
            &mut router,
            &creator,
            &owner,
            &minter,
            &marketplace,
            &collection,
            &token_id.to_string(),
            &ask_price,
            &[config.listing_fee.clone()],
            None,
        );
    }

    // Other collection address returns no asks
    let dummy_collection = Addr::unchecked("dummy_collection");
    let asks = router
        .wrap()
        .query_wasm_smart::<Vec<Ask>>(
            &marketplace,
            &QueryMsg::AsksByPrice {
                collection: dummy_collection.to_string(),
                denom: NATIVE_DENOM.to_string(),
                query_options: Some(QueryOptions {
                    descending: Some(true),
                    limit: None,
                    min: Some(QueryBound::Exclusive(AsksByPriceOffset {
                        token_id: "2".to_string(),
                        amount: 0u128,
                    })),
                    max: Some(QueryBound::Exclusive(AsksByPriceOffset {
                        token_id: "5".to_string(),
                        amount: 0u128,
                    })),
                }),
            },
        )
        .unwrap();
    assert_eq!(asks.len(), 0);

    // Other denoms returns no asks
    let asks = router
        .wrap()
        .query_wasm_smart::<Vec<Ask>>(
            &marketplace,
            &QueryMsg::AsksByPrice {
                collection: collection.to_string(),
                denom: JUNO_DENOM.to_string(),
                query_options: Some(QueryOptions {
                    descending: Some(true),
                    limit: None,
                    min: Some(QueryBound::Exclusive(AsksByPriceOffset {
                        token_id: "2".to_string(),
                        amount: 0u128,
                    })),
                    max: Some(QueryBound::Exclusive(AsksByPriceOffset {
                        token_id: "5".to_string(),
                        amount: 0u128,
                    })),
                }),
            },
        )
        .unwrap();
    assert_eq!(asks.len(), 0);

    // Correct number of asks returned for correct collection and denom
    let asks = router
        .wrap()
        .query_wasm_smart::<Vec<Ask>>(
            &marketplace,
            &QueryMsg::AsksByPrice {
                collection: collection.to_string(),
                denom: NATIVE_DENOM.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(asks.len(), num_nfts as usize);

    // Query Options work
    let asks = router
        .wrap()
        .query_wasm_smart::<Vec<Ask>>(
            &marketplace,
            &QueryMsg::AsksByPrice {
                collection: collection.to_string(),
                denom: NATIVE_DENOM.to_string(),
                query_options: Some(QueryOptions {
                    descending: Some(true),
                    limit: None,
                    min: Some(QueryBound::Exclusive(AsksByPriceOffset {
                        amount: native_denom_price_range.min.u128() + 2,
                        token_id: "".to_string(),
                    })),
                    max: Some(QueryBound::Exclusive(AsksByPriceOffset {
                        amount: native_denom_price_range.min.u128() + 5,
                        token_id: "".to_string(),
                    })),
                }),
            },
        )
        .unwrap();
    assert_eq!(asks.len(), 3);
    assert_eq!(asks[0].token_id, "4".to_string());
    assert_eq!(asks[1].token_id, "3".to_string());
    assert_eq!(asks[2].token_id, "2".to_string());
}

#[test]
fn try_query_asks_by_creator() {
    let MarketplaceV2Template {
        minter_template:
            MinterTemplateResponse {
                mut router,
                accts:
                    MarketAccounts {
                        creator,
                        owner,
                        bidder: _,
                    },
                ..
            },
        contracts:
            TestContracts {
                collection,
                minter,
                fair_burn: _,
                royalty_registry: _,
                marketplace,
            },
    } = marketplace_v2_template(10_000);

    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);

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

    let num_nfts: u8 = 4;
    for idx in 1..(num_nfts + 1) {
        let token_id = idx.to_string();
        let ask_price = coin(
            native_denom_price_range.min.u128() + idx as u128,
            NATIVE_DENOM,
        );
        mint_and_set_ask(
            &mut router,
            &creator,
            &owner,
            &minter,
            &marketplace,
            &collection,
            &token_id.to_string(),
            &ask_price,
            &[config.listing_fee.clone()],
            None,
        );
    }

    // Other creator address returns no asks
    let dummy_creator = Addr::unchecked("dummy_creator");
    let asks = router
        .wrap()
        .query_wasm_smart::<Vec<Ask>>(
            &marketplace,
            &QueryMsg::AsksByCreator {
                creator: dummy_creator.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(asks.len(), 0);

    // Correct number of asks returned for correct creator
    let asks = router
        .wrap()
        .query_wasm_smart::<Vec<Ask>>(
            &marketplace,
            &QueryMsg::AsksByCreator {
                creator: owner.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(asks.len(), num_nfts as usize);

    // Query Options work
    let asks = router
        .wrap()
        .query_wasm_smart::<Vec<Ask>>(
            &marketplace,
            &QueryMsg::AsksByCreator {
                creator: owner.to_string(),
                query_options: Some(QueryOptions {
                    descending: Some(true),
                    limit: Some(2),
                    min: Some(QueryBound::Inclusive(AsksByCreatorOffset {
                        collection: "".to_string(),
                        token_id: "".to_string(),
                    })),
                    max: Some(QueryBound::Exclusive(AsksByCreatorOffset {
                        collection: collection.to_string(),
                        token_id: "5".to_string(),
                    })),
                }),
            },
        )
        .unwrap();
    assert_eq!(asks.len(), 2);
    assert_eq!(asks[0].token_id, "4".to_string());
    assert_eq!(asks[1].token_id, "3".to_string());
}

#[test]
fn try_query_asks_by_expiration() {
    let MarketplaceV2Template {
        minter_template:
            MinterTemplateResponse {
                mut router,
                accts:
                    MarketAccounts {
                        creator,
                        owner,
                        bidder: _,
                    },
                ..
            },
        contracts:
            TestContracts {
                collection,
                minter,
                fair_burn: _,
                royalty_registry: _,
                marketplace,
            },
    } = marketplace_v2_template(10_000);

    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);

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

    let block_time = router.block_info().time;

    let num_nfts: u8 = 4;
    for idx in 1..(num_nfts + 1) {
        let token_id = idx.to_string();
        let ask_price = coin(
            native_denom_price_range.min.u128() + idx as u128,
            NATIVE_DENOM,
        );
        mint_and_set_ask(
            &mut router,
            &creator,
            &owner,
            &minter,
            &marketplace,
            &collection,
            &token_id.to_string(),
            &ask_price,
            &[
                config.listing_fee.clone(),
                config.min_removal_reward.clone(),
            ],
            Some(OrderOptions {
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
        );
    }

    // Correct number of asks returned for correct creator
    let asks = router
        .wrap()
        .query_wasm_smart::<Vec<Ask>>(
            &marketplace,
            &QueryMsg::AsksByExpiration {
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(asks.len(), num_nfts as usize);

    // Query Options work
    let asks = router
        .wrap()
        .query_wasm_smart::<Vec<Ask>>(
            &marketplace,
            &QueryMsg::AsksByExpiration {
                query_options: Some(QueryOptions {
                    descending: Some(true),
                    limit: Some(2),
                    min: Some(QueryBound::Exclusive(AsksByExpirationOffset {
                        collection: "".to_string(),
                        token_id: "".to_string(),
                        expiration: block_time
                            .plus_seconds(config.min_expiration_seconds)
                            .plus_seconds(3u64)
                            .seconds(),
                    })),
                    max: Some(QueryBound::Inclusive(AsksByExpirationOffset {
                        expiration: block_time
                            .plus_seconds(config.min_expiration_seconds)
                            .plus_seconds(5u64)
                            .seconds(),
                        collection: "".to_string(),
                        token_id: "".to_string(),
                    })),
                }),
            },
        )
        .unwrap();
    assert_eq!(asks.len(), 2);
    assert_eq!(asks[0].token_id, "4".to_string());
    assert_eq!(asks[1].token_id, "3".to_string());
}
