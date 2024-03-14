use crate::{
    msg::{ExecuteMsg, OrderOptions, QueryMsg, SudoMsg},
    state::{Ask, CollectionOffer, Config, Denom, ExpirationInfo, Offer, PriceRange},
    testing::{
        helpers::marketplace::mint_and_set_ask,
        setup::{
            msg::MarketAccounts,
            setup_accounts::setup_additional_account,
            templates::{marketplace_v2_template, MarketplaceV2Template, TestContracts},
        },
    },
};

use cosmwasm_std::{coin, Addr, Uint128};
use cw721::OwnerOfResponse;
use cw_multi_test::Executor;
use cw_utils::NativeBalance;
use sg721_base::msg::QueryMsg as Sg721QueryMsg;
use sg_std::GENESIS_MINT_START_TIME;
use sg_std::NATIVE_DENOM;
use std::{ops::Sub, vec};
use test_suite::common_setup::{
    msg::MinterTemplateResponse, setup_accounts_and_block::setup_block_time,
};

#[test]
fn try_sudo_begin_block_noop() {
    let MarketplaceV2Template {
        minter_template:
            MinterTemplateResponse {
                mut router,
                accts: MarketAccounts { .. },
                ..
            },
        contracts:
            TestContracts {
                fair_burn: _,
                royalty_registry: _,
                marketplace,
                ..
            },
    } = marketplace_v2_template(10_000);

    let begin_block_msg = SudoMsg::BeginBlock {};
    let response = router.wasm_sudo(marketplace, &begin_block_msg);
    assert!(response.is_ok());
}

#[test]
fn try_sudo_end_block() {
    let MarketplaceV2Template {
        minter_template:
            MinterTemplateResponse {
                mut router,
                accts:
                    MarketAccounts {
                        creator,
                        owner,
                        bidder,
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

    let _bidder2 = setup_additional_account(&mut router, "bidder2").unwrap();

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

    let owner_balances_before =
        NativeBalance(router.wrap().query_all_balances(owner.clone()).unwrap());
    let bidder_balances_before =
        NativeBalance(router.wrap().query_all_balances(bidder.clone()).unwrap());

    let num_orders: u128 = 4;
    for idx in 1..(num_orders + 1) {
        let ask_price = coin(
            native_denom_price_range.min.u128() + idx + 100,
            NATIVE_DENOM,
        );
        let token_id = idx.to_string();
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
                    expiration: block_time.plus_seconds(config.min_expiration_seconds),
                    removal_reward: config.min_removal_reward.clone(),
                }),
            }),
        );

        let offer_price = coin(native_denom_price_range.min.u128() + idx, NATIVE_DENOM);
        let set_offer = ExecuteMsg::SetOffer {
            collection: collection.to_string(),
            token_id: token_id.to_string(),
            price: offer_price.clone(),
            order_options: Some(OrderOptions {
                asset_recipient: None,
                finder: None,
                finders_fee_bps: None,
                expiration_info: Some(ExpirationInfo {
                    expiration: block_time.plus_seconds(config.min_expiration_seconds),
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

        let collection_bidder =
            setup_additional_account(&mut router, &format!("collection-bidder-{}", idx)).unwrap();
        let collection_offer_price = coin(native_denom_price_range.min.u128() + idx, NATIVE_DENOM);
        let set_offer = ExecuteMsg::SetCollectionOffer {
            collection: collection.to_string(),
            price: collection_offer_price.clone(),
            order_options: Some(OrderOptions {
                asset_recipient: None,
                finder: None,
                finders_fee_bps: None,
                expiration_info: Some(ExpirationInfo {
                    expiration: block_time.plus_seconds(config.min_expiration_seconds),
                    removal_reward: config.min_removal_reward.clone(),
                }),
            }),
        };
        let response = router.execute_contract(
            collection_bidder.clone(),
            marketplace.clone(),
            &set_offer,
            &[
                collection_offer_price.clone(),
                config.min_removal_reward.clone(),
            ],
        );
        assert!(response.is_ok());
    }

    // End block does not remove orders yet
    setup_block_time(
        &mut router,
        block_time
            .plus_seconds(config.min_expiration_seconds)
            .minus_seconds(config.order_removal_lookahead_secs)
            .minus_seconds(1)
            .nanos(),
        None,
    );
    let block_time = router.block_info().time;
    let end_block_msg = SudoMsg::EndBlock {};
    let response = router.wasm_sudo(marketplace.clone(), &end_block_msg);
    assert!(response.is_ok());

    let asks = router
        .wrap()
        .query_wasm_smart::<Vec<Ask>>(
            &marketplace,
            &QueryMsg::AsksByCollection {
                collection: collection.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(asks.len(), num_orders as usize);

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

    // End block removes all orders
    setup_block_time(&mut router, block_time.plus_seconds(1).nanos(), None);
    let _block_time = router.block_info().time;

    let end_block_msg = SudoMsg::EndBlock {};
    let response = router.wasm_sudo(marketplace.clone(), &end_block_msg);
    assert!(response.is_ok());

    let asks = router
        .wrap()
        .query_wasm_smart::<Vec<Ask>>(
            &marketplace,
            &QueryMsg::AsksByCollection {
                collection: collection.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(asks.len(), 0_usize);

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
    assert_eq!(offers.len(), 0_usize);

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
    assert_eq!(collection_offers.len(), 0_usize);

    // Validate that the tokens were returned to the original owners
    let owner_balances_after =
        NativeBalance(router.wrap().query_all_balances(owner.clone()).unwrap());
    assert_eq!(
        owner_balances_before
            .sub(coin(
                (config.listing_fee.amount.u128() + config.min_removal_reward.amount.u128())
                    * num_orders,
                NATIVE_DENOM
            ))
            .unwrap(),
        owner_balances_after
    );

    let bidder_balances_after = NativeBalance(router.wrap().query_all_balances(bidder).unwrap());
    assert_eq!(
        bidder_balances_before
            .sub(coin(
                config.min_removal_reward.amount.u128() * num_orders,
                NATIVE_DENOM
            ))
            .unwrap(),
        bidder_balances_after
    );

    let initial_balance = setup_additional_account(&mut router, "inital-balance").unwrap();
    let initial_balances =
        NativeBalance(router.wrap().query_all_balances(initial_balance).unwrap());
    for idx in 1..(num_orders + 1) {
        let collection_bidder = Addr::unchecked(format!("collection-bidder-{}", idx));
        let collection_bidder_balances_after = NativeBalance(
            router
                .wrap()
                .query_all_balances(collection_bidder.clone())
                .unwrap(),
        );
        assert_eq!(
            initial_balances
                .clone()
                .sub(config.min_removal_reward.clone())
                .unwrap(),
            collection_bidder_balances_after
        );
    }

    // Validate that the NFTs were returned to the seller
    for idx in 1..(num_orders + 1) {
        let token_id = idx.to_string();
        let owner_of_response = router
            .wrap()
            .query_wasm_smart::<OwnerOfResponse>(
                &collection,
                &Sg721QueryMsg::OwnerOf {
                    token_id: token_id.clone(),
                    include_expired: None,
                },
            )
            .unwrap();
        assert_eq!(owner_of_response.owner, owner);
    }
}

#[test]
fn try_sudo_update_params() {
    let MarketplaceV2Template {
        minter_template:
            MinterTemplateResponse {
                mut router,
                accts: MarketAccounts { .. },
                ..
            },
        contracts:
            TestContracts {
                fair_burn: _,
                royalty_registry: _,
                marketplace,
                ..
            },
    } = marketplace_v2_template(10_000);

    let config: Config<Addr> = router
        .wrap()
        .query_wasm_smart(&marketplace, &QueryMsg::Config {})
        .unwrap();

    let delta = 1u64;
    let fair_burn = "fair-burn-test".to_string();
    let listing_fee = coin(
        config.listing_fee.amount.u128() + delta as u128,
        config.listing_fee.denom.clone(),
    );
    let min_removal_reward = coin(
        config.min_removal_reward.amount.u128() + delta as u128,
        config.min_removal_reward.denom.clone(),
    );
    let trading_fee_bps = config.trading_fee_bps + delta;
    let max_royalty_fee_bps = config.max_royalty_fee_bps + delta;
    let max_finders_fee_bps = config.max_finders_fee_bps + delta;
    let min_expiration_seconds = config.min_expiration_seconds + delta;
    let max_asks_removed_per_block = config.max_asks_removed_per_block + delta as u32;
    let max_offers_removed_per_block = config.max_offers_removed_per_block + delta as u32;
    let max_collection_offers_removed_per_block =
        config.max_collection_offers_removed_per_block + delta as u32;

    let end_block_msg = SudoMsg::UpdateParams {
        fair_burn: Some(fair_burn.clone()),
        listing_fee: Some(listing_fee.clone()),
        min_removal_reward: Some(min_removal_reward.clone()),
        trading_fee_bps: Some(trading_fee_bps),
        max_royalty_fee_bps: Some(max_royalty_fee_bps),
        max_finders_fee_bps: Some(max_finders_fee_bps),
        min_expiration_seconds: Some(min_expiration_seconds),
        max_asks_removed_per_block: Some(max_asks_removed_per_block),
        max_offers_removed_per_block: Some(max_offers_removed_per_block),
        max_collection_offers_removed_per_block: Some(max_collection_offers_removed_per_block),
    };
    let response = router.wasm_sudo(marketplace.clone(), &end_block_msg);
    assert!(response.is_ok());

    let config: Config<Addr> = router
        .wrap()
        .query_wasm_smart(&marketplace, &QueryMsg::Config {})
        .unwrap();
    assert_eq!(config.fair_burn, fair_burn);
    assert_eq!(config.listing_fee, listing_fee);
    assert_eq!(config.min_removal_reward, min_removal_reward);
    assert_eq!(config.trading_fee_bps, trading_fee_bps);
    assert_eq!(config.max_royalty_fee_bps, max_royalty_fee_bps);
    assert_eq!(config.max_finders_fee_bps, max_finders_fee_bps);
    assert_eq!(config.min_expiration_seconds, min_expiration_seconds);
    assert_eq!(
        config.max_asks_removed_per_block,
        max_asks_removed_per_block
    );
    assert_eq!(
        config.max_offers_removed_per_block,
        max_offers_removed_per_block
    );
    assert_eq!(
        config.max_collection_offers_removed_per_block,
        max_collection_offers_removed_per_block
    );
}

#[test]
fn try_sudo_add_remove_denoms() {
    let MarketplaceV2Template {
        minter_template:
            MinterTemplateResponse {
                mut router,
                accts: MarketAccounts { .. },
                ..
            },
        contracts:
            TestContracts {
                fair_burn: _,
                royalty_registry: _,
                marketplace,
                ..
            },
    } = marketplace_v2_template(10_000);

    let price_ranges = router
        .wrap()
        .query_wasm_smart::<Vec<(Denom, PriceRange)>>(
            &marketplace,
            &QueryMsg::PriceRanges {
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(price_ranges.len(), 2_usize);

    let new_denom_price_range = (
        "uosmo".to_string(),
        PriceRange {
            min: Uint128::from(100u128),
            max: Uint128::from(10_000u128),
        },
    );

    let add_denoms_msg = SudoMsg::AddDenoms {
        price_ranges: vec![new_denom_price_range.clone()],
    };
    let response = router.wasm_sudo(marketplace.clone(), &add_denoms_msg);
    assert!(response.is_ok());

    let price_ranges = router
        .wrap()
        .query_wasm_smart::<Vec<(Denom, PriceRange)>>(
            &marketplace,
            &QueryMsg::PriceRanges {
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(price_ranges.len(), 3_usize);
    assert!(price_ranges
        .iter()
        .any(|denom_price_range| denom_price_range == &new_denom_price_range));

    let remove_denoms_msg = SudoMsg::RemoveDenoms {
        denoms: vec![new_denom_price_range.0],
    };
    let response = router.wasm_sudo(marketplace.clone(), &remove_denoms_msg);
    assert!(response.is_ok());

    let price_ranges = router
        .wrap()
        .query_wasm_smart::<Vec<(Denom, PriceRange)>>(
            &marketplace,
            &QueryMsg::PriceRanges {
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(price_ranges.len(), 2_usize);
}
