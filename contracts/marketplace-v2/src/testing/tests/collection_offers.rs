use crate::{
    msg::{ExecuteMsg, OrderOptions, QueryMsg, UpdateVal},
    state::{CollectionOffer, Config, ExpirationInfo, KeyString, PriceRange},
    testing::{
        helpers::{nft_functions::mint_for, utils::assert_error},
        setup::{
            msg::MarketAccounts,
            setup_accounts::setup_additional_account,
            setup_marketplace::{ATOM_DENOM, JUNO_DENOM},
            templates::{marketplace_v2_template, MarketplaceV2Template, TestContracts},
        },
    },
    ContractError,
};

use cosmwasm_std::coin;
use cosmwasm_std::{Addr, Uint128};
use cw_multi_test::Executor;
use cw_utils::NativeBalance;
use sg_std::GENESIS_MINT_START_TIME;
use sg_std::NATIVE_DENOM;
use std::ops::{Add, Sub};
use test_suite::common_setup::{
    msg::MinterTemplateResponse, setup_accounts_and_block::setup_block_time,
};

#[test]
fn try_set_simple_collection_offer() {
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
    let _block_time = router.block_info().time;

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

    let token_id = "1";
    mint_for(&mut router, &creator, &owner, &minter, token_id);

    // Create collection offer without sufficient offer funds fails
    let collection_offer_price = coin(2_000_000, NATIVE_DENOM);
    let set_collection_offer = ExecuteMsg::SetCollectionOffer {
        collection: collection.to_string(),
        price: collection_offer_price.clone(),
        order_options: None,
    };
    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_collection_offer,
        &[config.listing_fee.clone()],
    );
    assert_error(
        response,
        ContractError::InsufficientFunds {
            expected: collection_offer_price,
        }
        .to_string(),
    );

    // Create collection offer with invalid denom fails
    let collection_offer_price = coin(2_000_000, JUNO_DENOM);
    let set_collection_offer = ExecuteMsg::SetCollectionOffer {
        collection: collection.to_string(),
        price: collection_offer_price.clone(),
        order_options: None,
    };
    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_collection_offer,
        &[collection_offer_price],
    );
    assert_error(
        response,
        ContractError::InvalidInput("invalid denom".to_string()).to_string(),
    );

    // Create collection offer with price too low fails
    let set_collection_offer = ExecuteMsg::SetCollectionOffer {
        collection: collection.to_string(),
        price: coin(native_denom_price_range.min.u128() - 1u128, NATIVE_DENOM),
        order_options: None,
    };
    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_collection_offer,
        &[config.listing_fee.clone()],
    );
    assert_error(
        response,
        ContractError::InvalidInput("price too low 99999 < 100000".to_string()).to_string(),
    );

    // Create ask with price too high fails
    let set_collection_offer = ExecuteMsg::SetCollectionOffer {
        collection: collection.to_string(),
        price: coin(native_denom_price_range.max.u128() + 1u128, NATIVE_DENOM),
        order_options: None,
    };
    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_collection_offer,
        &[config.listing_fee],
    );
    assert_error(
        response,
        ContractError::InvalidInput("price too high 100000000000001 > 100000000000000".to_string())
            .to_string(),
    );

    // Create simple offer succeeds
    let price = coin(native_denom_price_range.min.u128(), NATIVE_DENOM);
    let set_collection_offer = ExecuteMsg::SetCollectionOffer {
        collection: collection.to_string(),
        price: price.clone(),
        order_options: None,
    };
    let bidder_native_balances_before =
        NativeBalance(router.wrap().query_all_balances(bidder.clone()).unwrap());
    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_collection_offer,
        &[price.clone()],
    );
    assert!(response.is_ok());

    let bidder_native_balances_after =
        NativeBalance(router.wrap().query_all_balances(bidder.clone()).unwrap());
    assert_eq!(
        bidder_native_balances_before.sub(price.clone()).unwrap(),
        bidder_native_balances_after
    );

    let collection_offer = router
        .wrap()
        .query_wasm_smart::<Option<CollectionOffer>>(
            &marketplace,
            &QueryMsg::CollectionOffer {
                collection: collection.to_string(),
                creator: bidder.to_string(),
            },
        )
        .unwrap()
        .unwrap();
    assert_eq!(collection_offer.order_info.price, price);
    assert_eq!(collection_offer.order_info.creator, bidder);
    assert_eq!(collection_offer.order_info.asset_recipient, None);
    assert_eq!(collection_offer.order_info.finders_fee_bps, None);
    assert_eq!(collection_offer.order_info.expiration_info, None);

    // Create duplicate offer fails
    let set_collection_offer = ExecuteMsg::SetCollectionOffer {
        collection: collection.to_string(),
        price: price.clone(),
        order_options: None,
    };
    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_collection_offer,
        &[price.clone()],
    );
    assert_error(
        response,
        ContractError::EntityExists(format!(
            "collection_offer {}",
            CollectionOffer::build_key(&collection, &bidder).to_string()
        ))
        .to_string(),
    );

    // Overpay listing fee succeeds
    let owner_native_balances_before =
        NativeBalance(router.wrap().query_all_balances(owner.clone()).unwrap());
    let set_collection_offer = ExecuteMsg::SetCollectionOffer {
        collection: collection.to_string(),
        price: price.clone(),
        order_options: None,
    };
    let response = router.execute_contract(
        owner.clone(),
        marketplace,
        &set_collection_offer,
        &[coin(price.amount.u128() * 2u128, NATIVE_DENOM)],
    );
    assert!(response.is_ok());

    let owner_native_balances_after =
        NativeBalance(router.wrap().query_all_balances(owner.clone()).unwrap());
    assert_eq!(
        owner_native_balances_before.sub(price).unwrap(),
        owner_native_balances_after
    );
}

#[test]
pub fn try_set_complex_collection_offer() {
    let MarketplaceV2Template {
        minter_template:
            MinterTemplateResponse {
                mut router,
                accts:
                    MarketAccounts {
                        creator: _,
                        owner: _,
                        bidder,
                    },
                ..
            },
        contracts:
            TestContracts {
                collection,
                minter: _,
                fair_burn: _,
                royalty_registry: _,
                marketplace,
            },
    } = marketplace_v2_template(10_000);

    let asset_recipient = setup_additional_account(&mut router, "asset_recipient").unwrap();
    let finder = setup_additional_account(&mut router, "finder").unwrap();

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

    // Create collection offer with finder sender fails
    let set_collection_offer = ExecuteMsg::SetCollectionOffer {
        collection: collection.to_string(),
        price: coin(native_denom_price_range.min.u128(), NATIVE_DENOM),
        order_options: Some(OrderOptions {
            asset_recipient: None,
            finder: Some(bidder.to_string()),
            finders_fee_bps: None,
            expiration_info: None,
        }),
    };
    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_collection_offer,
        &[coin(native_denom_price_range.min.u128(), NATIVE_DENOM)],
    );
    assert_error(
        response,
        ContractError::InvalidInput("finder should not be sender".to_string()).to_string(),
    );

    // Create collection offer with finders fee above 100% fails
    let set_collection_offer = ExecuteMsg::SetCollectionOffer {
        collection: collection.to_string(),
        price: coin(native_denom_price_range.min.u128(), NATIVE_DENOM),
        order_options: Some(OrderOptions {
            asset_recipient: None,
            finder: Some(finder.to_string()),
            finders_fee_bps: Some(10001),
            expiration_info: None,
        }),
    };
    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_collection_offer,
        &[coin(
            config.listing_fee.amount.u128(),
            &config.listing_fee.denom,
        )],
    );
    assert_error(
        response,
        ContractError::InvalidInput("finders_fee_bps is above 100%".to_string()).to_string(),
    );

    // Create collection offer with expiration info fails when no fee is paid
    let set_collection_offer = ExecuteMsg::SetCollectionOffer {
        collection: collection.to_string(),
        price: coin(native_denom_price_range.min.u128(), NATIVE_DENOM),
        order_options: Some(OrderOptions {
            asset_recipient: None,
            finder: Some(finder.to_string()),
            finders_fee_bps: Some(1000),
            expiration_info: Some(ExpirationInfo {
                expiration: block_time.plus_seconds(config.min_expiration_seconds),
                removal_reward: coin(config.min_removal_reward.amount.u128(), JUNO_DENOM),
            }),
        }),
    };
    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_collection_offer,
        &[coin(
            config.listing_fee.amount.u128(),
            &config.listing_fee.denom,
        )],
    );
    assert_error(
        response,
        ContractError::InvalidInput(format!(
            "removal reward must be at least {}",
            config.min_removal_reward
        ))
        .to_string(),
    );

    // Create collection offer with expiration too soon fails
    let set_collection_offer = ExecuteMsg::SetCollectionOffer {
        collection: collection.to_string(),
        price: coin(native_denom_price_range.min.u128(), NATIVE_DENOM),
        order_options: Some(OrderOptions {
            asset_recipient: None,
            finder: Some(finder.to_string()),
            finders_fee_bps: Some(1000),
            expiration_info: Some(ExpirationInfo {
                expiration: block_time.plus_seconds(config.min_expiration_seconds - 1),
                removal_reward: coin(config.min_removal_reward.amount.u128(), NATIVE_DENOM),
            }),
        }),
    };
    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_collection_offer,
        &[coin(
            config.listing_fee.amount.u128(),
            &config.listing_fee.denom,
        )],
    );
    assert_error(
        response,
        ContractError::InvalidInput("expiration is below minimum".to_string()).to_string(),
    );

    // Create collection offer with all valid parameters succeeds
    let price = coin(native_denom_price_range.min.u128(), NATIVE_DENOM);
    let finders_fee_bps = 1000;
    let expiration_info = ExpirationInfo {
        expiration: block_time.plus_seconds(config.min_expiration_seconds),
        removal_reward: coin(config.min_removal_reward.amount.u128(), NATIVE_DENOM),
    };
    let set_collection_offer = ExecuteMsg::SetCollectionOffer {
        collection: collection.to_string(),
        price: price.clone(),
        order_options: Some(OrderOptions {
            asset_recipient: Some(asset_recipient.to_string()),
            finder: Some(finder.to_string()),
            finders_fee_bps: Some(finders_fee_bps),
            expiration_info: Some(expiration_info.clone()),
        }),
    };

    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_collection_offer,
        &[coin(
            (config.listing_fee.amount + config.min_removal_reward.amount).u128(),
            &config.listing_fee.denom,
        )],
    );
    assert!(response.is_ok());

    let collection_offer = router
        .wrap()
        .query_wasm_smart::<Option<CollectionOffer>>(
            &marketplace,
            &QueryMsg::CollectionOffer {
                collection: collection.to_string(),
                creator: bidder.to_string(),
            },
        )
        .unwrap()
        .unwrap();
    assert_eq!(collection_offer.order_info.price, price);
    assert_eq!(collection_offer.order_info.creator, bidder);
    assert_eq!(
        collection_offer.order_info.asset_recipient,
        Some(asset_recipient.clone())
    );
    assert_eq!(
        collection_offer.order_info.finders_fee_bps,
        Some(finders_fee_bps)
    );
    assert_eq!(
        collection_offer.order_info.expiration_info,
        Some(expiration_info)
    );

    // Create collection offer with high removal reward succeeds
    router
        .execute_contract(
            bidder.clone(),
            marketplace.clone(),
            &ExecuteMsg::RemoveCollectionOffer {
                collection: collection.to_string(),
            },
            &[],
        )
        .unwrap();

    let removal_reward_amount = config.min_removal_reward.amount * Uint128::from(10u8);
    let overpay_fees = vec![
        price.clone(),
        coin(
            removal_reward_amount.u128(),
            &config.min_removal_reward.denom,
        ),
        coin(config.listing_fee.amount.u128(), ATOM_DENOM),
    ];

    let expiration_info = ExpirationInfo {
        expiration: block_time.plus_seconds(config.min_expiration_seconds),
        removal_reward: coin(
            removal_reward_amount.u128(),
            &config.min_removal_reward.denom,
        ),
    };

    let bidder_native_balances_before =
        NativeBalance(router.wrap().query_all_balances(bidder.clone()).unwrap());
    let set_collection_offer = ExecuteMsg::SetCollectionOffer {
        collection: collection.to_string(),
        price: price.clone(),
        order_options: Some(OrderOptions {
            asset_recipient: Some(asset_recipient.to_string()),
            finder: Some(finder.to_string()),
            finders_fee_bps: Some(finders_fee_bps),
            expiration_info: Some(expiration_info.clone()),
        }),
    };
    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_collection_offer,
        &overpay_fees,
    );
    assert!(response.is_ok());

    let bidder_native_balances_after =
        NativeBalance(router.wrap().query_all_balances(bidder.clone()).unwrap());
    assert_eq!(
        bidder_native_balances_before
            .sub(price.clone())
            .unwrap()
            .sub(coin(removal_reward_amount.u128(), NATIVE_DENOM))
            .unwrap(),
        bidder_native_balances_after
    );

    let collection_offer = router
        .wrap()
        .query_wasm_smart::<Option<CollectionOffer>>(
            &marketplace,
            &QueryMsg::CollectionOffer {
                collection: collection.to_string(),
                creator: bidder.to_string(),
            },
        )
        .unwrap()
        .unwrap();
    assert_eq!(collection_offer.order_info.price, price);
    assert_eq!(collection_offer.order_info.creator, bidder);
    assert_eq!(
        collection_offer.order_info.asset_recipient,
        Some(asset_recipient)
    );
    assert_eq!(
        collection_offer.order_info.finders_fee_bps,
        Some(finders_fee_bps)
    );
    assert_eq!(
        collection_offer.order_info.expiration_info,
        Some(expiration_info)
    );
}

#[test]
pub fn try_update_collection_offer() {
    let MarketplaceV2Template {
        minter_template:
            MinterTemplateResponse {
                mut router,
                accts:
                    MarketAccounts {
                        creator: _,
                        owner: _,
                        bidder,
                    },
                ..
            },
        contracts:
            TestContracts {
                collection,
                minter: _,
                fair_burn: _,
                royalty_registry: _,
                marketplace,
            },
    } = marketplace_v2_template(10_000);

    let asset_recipient2 = setup_additional_account(&mut router, "asset_recipient2").unwrap();

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

    let price = coin(native_denom_price_range.min.u128(), NATIVE_DENOM);
    router
        .execute_contract(
            bidder.clone(),
            marketplace.clone(),
            &ExecuteMsg::SetCollectionOffer {
                collection: collection.to_string(),
                price: price.clone(),
                order_options: None,
            },
            &[price],
        )
        .unwrap();

    // Setting asset_recipient and finders_fee_bps succeeds
    let update_collection_offer = ExecuteMsg::UpdateCollectionOffer {
        collection: collection.to_string(),
        asset_recipient: Some(UpdateVal::Set(asset_recipient2.to_string())),
        finders_fee_bps: Some(UpdateVal::Set(2000)),
        expiration_info: None,
    };
    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &update_collection_offer,
        &[],
    );
    assert!(response.is_ok());

    let collection_offer = router
        .wrap()
        .query_wasm_smart::<Option<CollectionOffer>>(
            &marketplace,
            &QueryMsg::CollectionOffer {
                collection: collection.to_string(),
                creator: bidder.to_string(),
            },
        )
        .unwrap()
        .unwrap();
    assert_eq!(
        collection_offer.order_info.asset_recipient,
        Some(asset_recipient2)
    );
    assert_eq!(collection_offer.order_info.finders_fee_bps, Some(2000));

    // Setting expiration_info without fee fails
    let expiration_info = ExpirationInfo {
        expiration: block_time.plus_seconds(config.min_expiration_seconds),
        removal_reward: coin(config.min_removal_reward.amount.u128(), NATIVE_DENOM),
    };
    let update_collection_offer = ExecuteMsg::UpdateCollectionOffer {
        collection: collection.to_string(),
        asset_recipient: None,
        finders_fee_bps: None,
        expiration_info: Some(UpdateVal::Set(expiration_info.clone())),
    };

    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &update_collection_offer,
        &[],
    );
    assert_error(
        response,
        ContractError::InsufficientFunds {
            expected: coin(config.min_removal_reward.amount.u128(), NATIVE_DENOM),
        }
        .to_string(),
    );

    // Setting expiration_info with fee succeeds
    let update_collection_offer = ExecuteMsg::UpdateCollectionOffer {
        collection: collection.to_string(),
        asset_recipient: None,
        finders_fee_bps: None,
        expiration_info: Some(UpdateVal::Set(expiration_info)),
    };

    let bidder_native_balances_before =
        NativeBalance(router.wrap().query_all_balances(bidder.clone()).unwrap());
    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &update_collection_offer,
        &[config.min_removal_reward.clone()],
    );
    assert!(response.is_ok());

    let bidder_native_balances_after =
        NativeBalance(router.wrap().query_all_balances(bidder.clone()).unwrap());

    assert_eq!(
        bidder_native_balances_before
            .sub(config.min_removal_reward.clone())
            .unwrap(),
        bidder_native_balances_after
    );

    // Updating expiration_info refunds previous paid removal reward
    let expiration_info = ExpirationInfo {
        expiration: block_time.plus_seconds(config.min_expiration_seconds),
        removal_reward: coin(
            config.min_removal_reward.amount.u128() * 2u128,
            NATIVE_DENOM,
        ),
    };
    let update_collection_offer = ExecuteMsg::UpdateCollectionOffer {
        collection: collection.to_string(),
        asset_recipient: None,
        finders_fee_bps: None,
        expiration_info: Some(UpdateVal::Set(expiration_info.clone())),
    };

    let bidder_native_balances_before =
        NativeBalance(router.wrap().query_all_balances(bidder.clone()).unwrap());
    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &update_collection_offer,
        &[expiration_info.removal_reward.clone()],
    );
    assert!(response.is_ok());

    let bidder_native_balances_after =
        NativeBalance(router.wrap().query_all_balances(bidder.clone()).unwrap());

    assert_eq!(
        bidder_native_balances_before
            .sub(config.min_removal_reward)
            .unwrap(),
        bidder_native_balances_after
    );

    // Removing expiration_info refunds removal reward
    let update_collection_offer = ExecuteMsg::UpdateCollectionOffer {
        collection: collection.to_string(),
        asset_recipient: None,
        finders_fee_bps: None,
        expiration_info: Some(UpdateVal::Unset),
    };

    let bidder_native_balances_before =
        NativeBalance(router.wrap().query_all_balances(bidder.clone()).unwrap());
    let response = router.execute_contract(
        bidder.clone(),
        marketplace,
        &update_collection_offer,
        &[expiration_info.removal_reward.clone()],
    );
    assert!(response.is_ok());

    let bidder_native_balances_after =
        NativeBalance(router.wrap().query_all_balances(bidder).unwrap());

    assert_eq!(
        bidder_native_balances_before.add(expiration_info.removal_reward),
        bidder_native_balances_after
    );
}

#[test]
pub fn try_remove_collection_offer() {
    let MarketplaceV2Template {
        minter_template:
            MinterTemplateResponse {
                mut router,
                accts:
                    MarketAccounts {
                        creator: _,
                        owner: _,
                        bidder,
                    },
                ..
            },
        contracts:
            TestContracts {
                collection,
                minter: _,
                fair_burn: _,
                royalty_registry: _,
                marketplace,
            },
    } = marketplace_v2_template(10_000);

    let bidder2 = setup_additional_account(&mut router, "bidder2").unwrap();

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

    let price = coin(native_denom_price_range.min.u128(), NATIVE_DENOM);
    router
        .execute_contract(
            bidder.clone(),
            marketplace.clone(),
            &ExecuteMsg::SetCollectionOffer {
                collection: collection.to_string(),
                price: price.clone(),
                order_options: None,
            },
            &[price.clone()],
        )
        .unwrap();

    // Removing collection offer as creator succeeds
    let remove_collection_offer = ExecuteMsg::RemoveCollectionOffer {
        collection: collection.to_string(),
    };
    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &remove_collection_offer,
        &[],
    );
    assert!(response.is_ok());

    let collection_offer = router
        .wrap()
        .query_wasm_smart::<Option<CollectionOffer>>(
            &marketplace,
            &QueryMsg::CollectionOffer {
                collection: collection.to_string(),
                creator: bidder.to_string(),
            },
        )
        .unwrap();
    assert!(collection_offer.is_none());

    // Cannot remove collection offer that is not expired
    router
        .execute_contract(
            bidder.clone(),
            marketplace.clone(),
            &ExecuteMsg::SetCollectionOffer {
                collection: collection.to_string(),
                price: price.clone(),
                order_options: Some(OrderOptions {
                    asset_recipient: None,
                    finder: None,
                    finders_fee_bps: None,
                    expiration_info: Some(ExpirationInfo {
                        expiration: block_time.plus_seconds(config.min_expiration_seconds),
                        removal_reward: coin(config.min_removal_reward.amount.u128(), NATIVE_DENOM),
                    }),
                }),
            },
            &[price, config.min_removal_reward.clone()],
        )
        .unwrap();

    let remove_expired_collection_offer = ExecuteMsg::RemoveExpiredCollectionOffer {
        collection: collection.to_string(),
        creator: bidder.to_string(),
    };
    let response = router.execute_contract(
        bidder2,
        marketplace.clone(),
        &remove_expired_collection_offer,
        &[],
    );

    assert_error(
        response,
        ContractError::EntityNotExpired(format!(
            "collection_offer {}",
            CollectionOffer::build_key(&collection, &bidder).to_string()
        ))
        .to_string(),
    );

    // Anyone can remove collection offer that is expired
    setup_block_time(
        &mut router,
        block_time
            .plus_seconds(config.min_expiration_seconds)
            .nanos(),
        None,
    );
    let remove_expired_collection_offer = ExecuteMsg::RemoveExpiredCollectionOffer {
        collection: collection.to_string(),
        creator: bidder.to_string(),
    };
    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &remove_expired_collection_offer,
        &[],
    );
    assert!(response.is_ok());

    let collection_offer = router
        .wrap()
        .query_wasm_smart::<Option<CollectionOffer>>(
            &marketplace,
            &QueryMsg::CollectionOffer {
                collection: collection.to_string(),
                creator: bidder.to_string(),
            },
        )
        .unwrap();
    assert!(collection_offer.is_none());
}
