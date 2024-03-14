use crate::{
    msg::{ExecuteMsg, OrderOptions, QueryMsg, UpdateVal},
    state::{Ask, Config, ExpirationInfo, KeyString, PriceRange},
    testing::{
        helpers::{
            marketplace::mint_and_set_ask,
            nft_functions::{approve, mint_for},
            utils::assert_error,
        },
        setup::{
            msg::MarketAccounts,
            setup_accounts::setup_additional_account,
            setup_marketplace::{ATOM_DENOM, JUNO_DENOM},
            templates::{marketplace_v2_template, MarketplaceV2Template, TestContracts},
        },
    },
    ContractError,
};

use cosmwasm_std::{coin, StdError};
use cosmwasm_std::{Addr, Uint128};
use cw_multi_test::Executor;
use cw_utils::NativeBalance;
use sg_marketplace_common::MarketplaceStdError;
use sg_std::GENESIS_MINT_START_TIME;
use sg_std::NATIVE_DENOM;
use std::ops::{Add, Sub};
use test_suite::common_setup::{
    msg::MinterTemplateResponse, setup_accounts_and_block::setup_block_time,
};

#[test]
fn try_set_simple_ask() {
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

    // Create ask unowned token fails
    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        price: coin(1_000_000, NATIVE_DENOM),
        order_options: None,
    };
    let response = router.execute_contract(
        bidder,
        marketplace.clone(),
        &set_ask,
        &[config.listing_fee.clone()],
    );
    assert_error(response, StdError::generic_err("Unauthorized").to_string());

    // Create ask without token approval fails
    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        price: coin(1_000_000, NATIVE_DENOM),
        order_options: None,
    };
    let response = router.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &set_ask,
        &[config.listing_fee.clone()],
    );
    assert!(response.is_err());

    // Create ask with invalid denom fails
    approve(&mut router, &owner, &collection, &marketplace, token_id);
    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        price: coin(1_000_000, JUNO_DENOM),
        order_options: None,
    };
    let response = router.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &set_ask,
        &[config.listing_fee.clone()],
    );
    assert_error(
        response,
        ContractError::InvalidInput("invalid denom".to_string()).to_string(),
    );

    // Create ask with price too low fails
    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        price: coin(native_denom_price_range.min.u128() - 1u128, NATIVE_DENOM),
        order_options: None,
    };
    let response = router.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &set_ask,
        &[config.listing_fee.clone()],
    );
    assert_error(
        response,
        ContractError::InvalidInput("price too low 99999 < 100000".to_string()).to_string(),
    );

    // Create ask with price too high fails
    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        price: coin(native_denom_price_range.max.u128() + 1u128, NATIVE_DENOM),
        order_options: None,
    };
    let response = router.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &set_ask,
        &[config.listing_fee.clone()],
    );
    assert_error(
        response,
        ContractError::InvalidInput("price too high 100000000000001 > 100000000000000".to_string())
            .to_string(),
    );

    // Create ask without listing fee fails
    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        price: coin(native_denom_price_range.min.u128(), NATIVE_DENOM),
        order_options: None,
    };
    let response = router.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &set_ask,
        &[coin(
            config.listing_fee.amount.u128() - 1u128,
            &config.listing_fee.denom,
        )],
    );
    assert_error(
        response,
        ContractError::InsufficientFunds {
            expected: coin(1000000u128, NATIVE_DENOM),
        }
        .to_string(),
    );

    // Create simple ask succeeds
    let price = coin(native_denom_price_range.min.u128(), NATIVE_DENOM);
    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        price: price.clone(),
        order_options: None,
    };
    let owner_native_balances_before =
        NativeBalance(router.wrap().query_all_balances(owner.clone()).unwrap());
    let response = router.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &set_ask,
        &[config.listing_fee.clone()],
    );
    assert!(response.is_ok());

    let owner_native_balances_after =
        NativeBalance(router.wrap().query_all_balances(owner.clone()).unwrap());
    assert_eq!(
        owner_native_balances_before
            .sub(config.listing_fee.clone())
            .unwrap(),
        owner_native_balances_after
    );

    let ask: Ask = router
        .wrap()
        .query_wasm_smart(
            &marketplace,
            &QueryMsg::Ask {
                collection: collection.to_string(),
                token_id: token_id.to_string(),
            },
        )
        .unwrap();
    assert_eq!(ask.order_info.price, price);
    assert_eq!(ask.order_info.creator, owner);
    assert_eq!(ask.order_info.asset_recipient, None);
    assert_eq!(ask.order_info.finders_fee_bps, None);
    assert_eq!(ask.order_info.expiration_info, None);

    // Create duplicate ask fails
    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        price: price.clone(),
        order_options: None,
    };
    let response = router.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &set_ask,
        &[config.listing_fee.clone()],
    );
    assert_error(response, "Generic error: Unauthorized".to_string());

    // Overpay listing fee succeeds
    let token_id = "2";
    mint_for(&mut router, &creator, &owner, &minter, token_id);
    approve(&mut router, &owner, &collection, &marketplace, token_id);

    let overpay_fees = vec![
        coin(
            config.listing_fee.amount.u128() * 10u128,
            &config.listing_fee.denom,
        ),
        coin(config.listing_fee.amount.u128(), ATOM_DENOM),
    ];
    let owner_native_balances_before =
        NativeBalance(router.wrap().query_all_balances(owner.clone()).unwrap());
    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        price,
        order_options: None,
    };
    let response =
        router.execute_contract(owner.clone(), marketplace.clone(), &set_ask, &overpay_fees);
    assert!(response.is_ok());

    let owner_native_balances_after =
        NativeBalance(router.wrap().query_all_balances(owner.clone()).unwrap());
    assert_eq!(
        owner_native_balances_before
            .sub(config.listing_fee)
            .unwrap(),
        owner_native_balances_after
    );
}

#[test]
pub fn try_set_complex_ask() {
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

    let token_id = "1";
    mint_for(&mut router, &creator, &owner, &minter, token_id);
    approve(&mut router, &owner, &collection, &marketplace, token_id);

    // Create ask with finder sender fails
    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        price: coin(native_denom_price_range.min.u128(), NATIVE_DENOM),
        order_options: Some(OrderOptions {
            asset_recipient: None,
            finder: Some(owner.to_string()),
            finders_fee_bps: None,
            expiration_info: None,
        }),
    };
    let response = router.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &set_ask,
        &[coin(
            config.listing_fee.amount.u128(),
            &config.listing_fee.denom,
        )],
    );
    assert_error(
        response,
        ContractError::InvalidInput("finder should not be sender".to_string()).to_string(),
    );

    // Create ask with finders fee above 100% fails
    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        price: coin(native_denom_price_range.min.u128(), NATIVE_DENOM),
        order_options: Some(OrderOptions {
            asset_recipient: None,
            finder: Some(finder.to_string()),
            finders_fee_bps: Some(10001),
            expiration_info: None,
        }),
    };
    let response = router.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &set_ask,
        &[coin(
            config.listing_fee.amount.u128(),
            &config.listing_fee.denom,
        )],
    );
    assert_error(
        response,
        ContractError::InvalidInput("finders_fee_bps is above 100%".to_string()).to_string(),
    );

    // Create ask with expiration info fails when no fee is paid
    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
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
        owner.clone(),
        marketplace.clone(),
        &set_ask,
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

    // Create ask with expiration too soon fails
    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        price: coin(native_denom_price_range.min.u128(), NATIVE_DENOM),
        order_options: Some(OrderOptions {
            asset_recipient: None,
            finder: Some(finder.to_string()),
            finders_fee_bps: Some(100),
            expiration_info: Some(ExpirationInfo {
                expiration: block_time.plus_seconds(config.min_expiration_seconds - 1),
                removal_reward: coin(config.min_removal_reward.amount.u128(), NATIVE_DENOM),
            }),
        }),
    };
    let response = router.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &set_ask,
        &[coin(
            config.listing_fee.amount.u128(),
            &config.listing_fee.denom,
        )],
    );
    assert_error(
        response,
        ContractError::InvalidInput("expiration is below minimum".to_string()).to_string(),
    );

    // Create ask with all valid parameters succeeds
    let price = coin(native_denom_price_range.min.u128(), NATIVE_DENOM);
    let finders_fee_bps = 100;
    let expiration_info = ExpirationInfo {
        expiration: block_time.plus_seconds(config.min_expiration_seconds),
        removal_reward: coin(config.min_removal_reward.amount.u128(), NATIVE_DENOM),
    };
    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        price: price.clone(),
        order_options: Some(OrderOptions {
            asset_recipient: Some(asset_recipient.to_string()),
            finder: Some(finder.to_string()),
            finders_fee_bps: Some(finders_fee_bps),
            expiration_info: Some(expiration_info.clone()),
        }),
    };

    let response = router.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &set_ask,
        &[coin(
            (config.listing_fee.amount + config.min_removal_reward.amount).u128(),
            &config.listing_fee.denom,
        )],
    );
    assert!(response.is_ok());

    let ask: Ask = router
        .wrap()
        .query_wasm_smart(
            &marketplace,
            &QueryMsg::Ask {
                collection: collection.to_string(),
                token_id: token_id.to_string(),
            },
        )
        .unwrap();
    assert_eq!(ask.order_info.price, price);
    assert_eq!(ask.order_info.creator, owner);
    assert_eq!(
        ask.order_info.asset_recipient,
        Some(asset_recipient.clone())
    );
    assert_eq!(ask.order_info.finders_fee_bps, Some(finders_fee_bps));
    assert_eq!(ask.order_info.expiration_info, Some(expiration_info));

    // Create ask with high removal reward succeeds
    let token_id = "2";
    mint_for(&mut router, &creator, &owner, &minter, token_id);
    approve(&mut router, &owner, &collection, &marketplace, token_id);

    let removal_reward_amount = config.min_removal_reward.amount * Uint128::from(10u8);
    let overpay_fees = vec![
        coin(
            config.listing_fee.amount.u128() * 10u128 + removal_reward_amount.u128(),
            &config.listing_fee.denom,
        ),
        coin(config.listing_fee.amount.u128(), ATOM_DENOM),
    ];

    let expiration_info = ExpirationInfo {
        expiration: block_time.plus_seconds(config.min_expiration_seconds),
        removal_reward: coin(removal_reward_amount.u128(), NATIVE_DENOM),
    };

    let owner_native_balances_before =
        NativeBalance(router.wrap().query_all_balances(owner.clone()).unwrap());
    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        price: price.clone(),
        order_options: Some(OrderOptions {
            asset_recipient: Some(asset_recipient.to_string()),
            finder: Some(finder.to_string()),
            finders_fee_bps: Some(finders_fee_bps),
            expiration_info: Some(expiration_info.clone()),
        }),
    };
    let response =
        router.execute_contract(owner.clone(), marketplace.clone(), &set_ask, &overpay_fees);
    assert!(response.is_ok());

    let owner_native_balances_after =
        NativeBalance(router.wrap().query_all_balances(owner.clone()).unwrap());
    assert_eq!(
        owner_native_balances_before
            .sub(config.listing_fee)
            .unwrap()
            .sub(coin(removal_reward_amount.u128(), NATIVE_DENOM))
            .unwrap(),
        owner_native_balances_after
    );

    let ask: Ask = router
        .wrap()
        .query_wasm_smart(
            &marketplace,
            &QueryMsg::Ask {
                collection: collection.to_string(),
                token_id: token_id.to_string(),
            },
        )
        .unwrap();
    assert_eq!(ask.order_info.price, price);
    assert_eq!(ask.order_info.creator, owner);
    assert_eq!(ask.order_info.asset_recipient, Some(asset_recipient));
    assert_eq!(ask.order_info.finders_fee_bps, Some(finders_fee_bps));
    assert_eq!(ask.order_info.expiration_info, Some(expiration_info));
}

#[test]
pub fn try_update_ask() {
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

    let asset_recipient = setup_additional_account(&mut router, "asset_recipient").unwrap();
    let asset_recipient2 = setup_additional_account(&mut router, "asset_recipient2").unwrap();
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

    let token_id = "1";
    mint_and_set_ask(
        &mut router,
        &creator,
        &owner,
        &minter,
        &marketplace,
        &collection,
        token_id,
        &coin(native_denom_price_range.min.u128(), NATIVE_DENOM),
        &[coin(
            config.listing_fee.amount.u128(),
            &config.listing_fee.denom,
        )],
        Some(OrderOptions {
            asset_recipient: Some(asset_recipient.to_string()),
            finder: Some(finder.to_string()),
            finders_fee_bps: Some(100),
            expiration_info: None,
        }),
    );

    // Setting asset_recipient and finders_fee_bps succeeds
    let update_ask = ExecuteMsg::UpdateAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        asset_recipient: Some(UpdateVal::Set(asset_recipient2.to_string())),
        finders_fee_bps: Some(UpdateVal::Set(200)),
        expiration_info: None,
    };
    let response = router.execute_contract(owner.clone(), marketplace.clone(), &update_ask, &[]);
    assert!(response.is_ok());

    let ask: Ask = router
        .wrap()
        .query_wasm_smart(
            &marketplace,
            &QueryMsg::Ask {
                collection: collection.to_string(),
                token_id: token_id.to_string(),
            },
        )
        .unwrap();
    assert_eq!(ask.order_info.asset_recipient, Some(asset_recipient2));
    assert_eq!(ask.order_info.finders_fee_bps, Some(200));

    // Setting expiration_info without fee fails
    let expiration_info = ExpirationInfo {
        expiration: block_time.plus_seconds(config.min_expiration_seconds),
        removal_reward: coin(config.min_removal_reward.amount.u128(), NATIVE_DENOM),
    };
    let update_ask = ExecuteMsg::UpdateAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        asset_recipient: None,
        finders_fee_bps: None,
        expiration_info: Some(UpdateVal::Set(expiration_info.clone())),
    };

    let response = router.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &update_ask,
        &[coin(config.min_removal_reward.amount.u128(), ATOM_DENOM)],
    );
    assert_error(
        response,
        ContractError::InsufficientFunds {
            expected: coin(config.min_removal_reward.amount.u128(), NATIVE_DENOM),
        }
        .to_string(),
    );

    // Setting expiration_info with fee succeeds
    let update_ask = ExecuteMsg::UpdateAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        asset_recipient: None,
        finders_fee_bps: None,
        expiration_info: Some(UpdateVal::Set(expiration_info)),
    };

    let owner_native_balances_before =
        NativeBalance(router.wrap().query_all_balances(owner.clone()).unwrap());
    let response = router.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &update_ask,
        &[config.min_removal_reward.clone()],
    );
    assert!(response.is_ok());

    let owner_native_balances_after =
        NativeBalance(router.wrap().query_all_balances(owner.clone()).unwrap());

    assert_eq!(
        owner_native_balances_before
            .sub(config.min_removal_reward.clone())
            .unwrap(),
        owner_native_balances_after
    );

    // Updating expiration_info refunds previous paid removal reward
    let expiration_info = ExpirationInfo {
        expiration: block_time.plus_seconds(config.min_expiration_seconds),
        removal_reward: coin(
            config.min_removal_reward.amount.u128() * 2u128,
            NATIVE_DENOM,
        ),
    };
    let update_ask = ExecuteMsg::UpdateAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        asset_recipient: None,
        finders_fee_bps: None,
        expiration_info: Some(UpdateVal::Set(expiration_info.clone())),
    };

    let owner_native_balances_before =
        NativeBalance(router.wrap().query_all_balances(owner.clone()).unwrap());
    let response = router.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &update_ask,
        &[expiration_info.removal_reward.clone()],
    );
    assert!(response.is_ok());

    let owner_native_balances_after =
        NativeBalance(router.wrap().query_all_balances(owner.clone()).unwrap());

    assert_eq!(
        owner_native_balances_before
            .sub(config.min_removal_reward)
            .unwrap(),
        owner_native_balances_after
    );

    // Removing expiration_info refunds removal reward
    let update_ask = ExecuteMsg::UpdateAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        asset_recipient: None,
        finders_fee_bps: None,
        expiration_info: Some(UpdateVal::Unset),
    };

    let owner_native_balances_before =
        NativeBalance(router.wrap().query_all_balances(owner.clone()).unwrap());
    let response = router.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &update_ask,
        &[expiration_info.removal_reward.clone()],
    );
    assert!(response.is_ok());

    let owner_native_balances_after =
        NativeBalance(router.wrap().query_all_balances(owner.clone()).unwrap());

    assert_eq!(
        owner_native_balances_before.add(expiration_info.removal_reward),
        owner_native_balances_after
    );
}

#[test]
pub fn try_remove_ask() {
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

    let token_id = "1";
    mint_and_set_ask(
        &mut router,
        &creator,
        &owner,
        &minter,
        &marketplace,
        &collection,
        token_id,
        &coin(native_denom_price_range.min.u128(), NATIVE_DENOM),
        &[config.listing_fee.clone()],
        None,
    );

    // Removing ask as non creator fails
    let remove_ask = ExecuteMsg::RemoveAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
    };
    let response = router.execute_contract(bidder.clone(), marketplace.clone(), &remove_ask, &[]);
    assert_error(
        response,
        MarketplaceStdError::Unauthorized(
            "only the creator of order can perform this action".to_string(),
        )
        .to_string(),
    );

    // Removing ask as creator succeeds
    let remove_ask = ExecuteMsg::RemoveAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
    };
    let response = router.execute_contract(owner.clone(), marketplace.clone(), &remove_ask, &[]);
    assert!(response.is_ok());

    let ask = router
        .wrap()
        .query_wasm_smart::<Option<Ask>>(
            &marketplace,
            &QueryMsg::Ask {
                collection: collection.to_string(),
                token_id: token_id.to_string(),
            },
        )
        .unwrap();
    assert!(ask.is_none());

    let token_id = "2";
    mint_and_set_ask(
        &mut router,
        &creator,
        &owner,
        &minter,
        &marketplace,
        &collection,
        token_id,
        &coin(native_denom_price_range.min.u128(), NATIVE_DENOM),
        &[coin(
            config.listing_fee.amount.u128() + config.min_removal_reward.amount.u128(),
            NATIVE_DENOM,
        )],
        Some(OrderOptions {
            asset_recipient: None,
            finder: None,
            finders_fee_bps: None,
            expiration_info: Some(ExpirationInfo {
                expiration: block_time.plus_seconds(config.min_expiration_seconds),
                removal_reward: config.min_removal_reward,
            }),
        }),
    );

    // Cannot remove ask that is not expired
    let remove_ask = ExecuteMsg::RemoveExpiredAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
    };
    let response = router.execute_contract(owner.clone(), marketplace.clone(), &remove_ask, &[]);

    assert_error(
        response,
        ContractError::EntityNotExpired(format!(
            "ask {}",
            Ask::build_key(&collection, &token_id.to_string()).to_string()
        ))
        .to_string(),
    );

    // Anyone can remove ask that is expired
    setup_block_time(
        &mut router,
        block_time
            .plus_seconds(config.min_expiration_seconds)
            .nanos(),
        None,
    );
    let remove_ask = ExecuteMsg::RemoveExpiredAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
    };
    let response = router.execute_contract(bidder, marketplace.clone(), &remove_ask, &[]);
    assert!(response.is_ok());

    let ask = router
        .wrap()
        .query_wasm_smart::<Option<Ask>>(
            &marketplace,
            &QueryMsg::Ask {
                collection: collection.to_string(),
                token_id: token_id.to_string(),
            },
        )
        .unwrap();
    assert!(ask.is_none());
}
