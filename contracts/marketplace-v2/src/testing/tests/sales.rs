use crate::{
    msg::{ExecuteMsg, QueryMsg},
    state::Config,
    testing::{
        helpers::nft_functions::{approve, mint_for},
        setup::{
            msg::MarketAccounts,
            setup_accounts::setup_additional_account,
            templates::{marketplace_v2_template, MarketplaceV2Template, TestContracts},
        },
    },
};

use cosmwasm_std::{coin, Addr, Decimal};
use cw_multi_test::Executor;
use cw_utils::NativeBalance;
use sg721::RoyaltyInfoResponse;
use sg721_base::msg::{CollectionInfoResponse, QueryMsg as Sg721QueryMsg};
use sg_std::GENESIS_MINT_START_TIME;
use sg_std::NATIVE_DENOM;
use std::ops::{Add, Sub};
use test_suite::common_setup::{
    msg::MinterTemplateResponse, setup_accounts_and_block::setup_block_time,
};

#[test]
fn try_set_ask_sale() {
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

    let bidder2 = setup_additional_account(&mut router, "bidder2").unwrap();

    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);
    let _block_time = router.block_info().time;

    let config: Config<Addr> = router
        .wrap()
        .query_wasm_smart(&marketplace, &QueryMsg::Config {})
        .unwrap();

    let royalty_info: RoyaltyInfoResponse = router
        .wrap()
        .query_wasm_smart::<CollectionInfoResponse>(
            collection.clone(),
            &Sg721QueryMsg::CollectionInfo {},
        )
        .unwrap()
        .royalty_info
        .unwrap();

    let owner_balances_before =
        NativeBalance(router.wrap().query_all_balances(owner.clone()).unwrap());
    let bidder2_balances_before =
        NativeBalance(router.wrap().query_all_balances(bidder2.clone()).unwrap());
    let royalty_balances_before = NativeBalance(
        router
            .wrap()
            .query_all_balances(royalty_info.payment_address.clone())
            .unwrap(),
    );

    let token_id = "1";

    // Create ask with matching offer produces a valid sale

    // * Offer 1 - 10_000_000 native denom (should not match)
    let offer_price_1 = coin(10_000_000, NATIVE_DENOM);
    let set_offer = ExecuteMsg::SetOffer {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        price: offer_price_1.clone(),
        order_options: None,
    };
    let response =
        router.execute_contract(bidder, marketplace.clone(), &set_offer, &[offer_price_1]);
    assert!(response.is_ok());

    // * Offer 2 - 15_000_000 native denom (should_match)
    let offer_price_2 = coin(15_000_000, NATIVE_DENOM);
    let set_offer = ExecuteMsg::SetOffer {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        price: offer_price_2.clone(),
        order_options: None,
    };
    let response = router.execute_contract(
        bidder2.clone(),
        marketplace.clone(),
        &set_offer,
        &[offer_price_2.clone()],
    );
    assert!(response.is_ok());

    mint_for(&mut router, &creator, &owner, &minter, token_id);
    approve(&mut router, &owner, &collection, &marketplace, token_id);

    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        price: coin(5_000_000, NATIVE_DENOM),
        order_options: None,
    };
    let response = router.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &set_ask,
        &[config.listing_fee.clone()],
    );
    assert!(response.is_ok());

    let owner_balances_after =
        NativeBalance(router.wrap().query_all_balances(owner.clone()).unwrap());
    let bidder2_balances_after = NativeBalance(router.wrap().query_all_balances(bidder2).unwrap());
    let royalty_balances_after = NativeBalance(
        router
            .wrap()
            .query_all_balances(royalty_info.payment_address.clone())
            .unwrap(),
    );

    let sale_coin = offer_price_2;
    let fair_burn_amount = sale_coin
        .amount
        .mul_ceil(Decimal::bps(config.trading_fee_bps));
    let royalty_amount = sale_coin.amount.mul_ceil(royalty_info.share);
    let seller_amount = sale_coin.amount.sub(fair_burn_amount).sub(royalty_amount);

    assert_eq!(
        owner_balances_before
            .sub(config.listing_fee)
            .unwrap()
            .add(coin(seller_amount.u128(), NATIVE_DENOM)),
        owner_balances_after
    );
    assert_eq!(
        bidder2_balances_before.sub(sale_coin).unwrap(),
        bidder2_balances_after
    );
    assert_eq!(
        royalty_balances_before.add(coin(royalty_amount.u128(), NATIVE_DENOM)),
        royalty_balances_after
    );
}

#[test]
fn try_accept_ask_sale() {
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
    let _block_time = router.block_info().time;

    let config: Config<Addr> = router
        .wrap()
        .query_wasm_smart(&marketplace, &QueryMsg::Config {})
        .unwrap();

    let royalty_info: RoyaltyInfoResponse = router
        .wrap()
        .query_wasm_smart::<CollectionInfoResponse>(
            collection.clone(),
            &Sg721QueryMsg::CollectionInfo {},
        )
        .unwrap()
        .royalty_info
        .unwrap();

    let owner_balances_before =
        NativeBalance(router.wrap().query_all_balances(owner.clone()).unwrap());
    let bidder_balances_before =
        NativeBalance(router.wrap().query_all_balances(bidder.clone()).unwrap());
    let royalty_balances_before = NativeBalance(
        router
            .wrap()
            .query_all_balances(royalty_info.payment_address.clone())
            .unwrap(),
    );

    // Create ask with no matching offer
    let token_id = "1";
    mint_for(&mut router, &creator, &owner, &minter, token_id);
    approve(&mut router, &owner, &collection, &marketplace, token_id);
    let ask_price = coin(5_000_000, NATIVE_DENOM);

    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        price: ask_price.clone(),
        order_options: None,
    };
    let response = router.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &set_ask,
        &[config.listing_fee.clone()],
    );
    assert!(response.is_ok());

    // Accept ask directly
    let accept_ask = ExecuteMsg::AcceptAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        order_options: None,
    };
    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &accept_ask,
        &[ask_price.clone()],
    );
    assert!(response.is_ok());

    let owner_balances_after =
        NativeBalance(router.wrap().query_all_balances(owner.clone()).unwrap());
    let bidder_balances_after = NativeBalance(router.wrap().query_all_balances(bidder).unwrap());
    let royalty_balances_after = NativeBalance(
        router
            .wrap()
            .query_all_balances(royalty_info.payment_address.clone())
            .unwrap(),
    );

    let sale_coin = ask_price;
    let fair_burn_amount = sale_coin
        .amount
        .mul_ceil(Decimal::bps(config.trading_fee_bps));
    let royalty_amount = sale_coin.amount.mul_ceil(royalty_info.share);
    let seller_amount = sale_coin.amount.sub(fair_burn_amount).sub(royalty_amount);

    assert_eq!(
        owner_balances_before
            .sub(config.listing_fee)
            .unwrap()
            .add(coin(seller_amount.u128(), NATIVE_DENOM)),
        owner_balances_after
    );
    assert_eq!(
        bidder_balances_before.sub(sale_coin).unwrap(),
        bidder_balances_after
    );
    assert_eq!(
        royalty_balances_before.add(coin(royalty_amount.u128(), NATIVE_DENOM)),
        royalty_balances_after
    );
}

#[test]
fn try_set_offer_sale() {
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
    let _block_time = router.block_info().time;

    let config: Config<Addr> = router
        .wrap()
        .query_wasm_smart(&marketplace, &QueryMsg::Config {})
        .unwrap();

    let royalty_info: RoyaltyInfoResponse = router
        .wrap()
        .query_wasm_smart::<CollectionInfoResponse>(
            collection.clone(),
            &Sg721QueryMsg::CollectionInfo {},
        )
        .unwrap()
        .royalty_info
        .unwrap();

    let owner_balances_before =
        NativeBalance(router.wrap().query_all_balances(owner.clone()).unwrap());
    let bidder_balances_before =
        NativeBalance(router.wrap().query_all_balances(bidder.clone()).unwrap());
    let royalty_balances_before = NativeBalance(
        router
            .wrap()
            .query_all_balances(royalty_info.payment_address.clone())
            .unwrap(),
    );

    // Create ask with no matching offer
    let token_id = "1";
    mint_for(&mut router, &creator, &owner, &minter, token_id);
    approve(&mut router, &owner, &collection, &marketplace, token_id);
    let ask_price = coin(5_000_000, NATIVE_DENOM);

    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        price: ask_price.clone(),
        order_options: None,
    };
    let response = router.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &set_ask,
        &[config.listing_fee.clone()],
    );
    assert!(response.is_ok());

    // Create offer that matches ask
    let offer_price = coin(10_000_000, NATIVE_DENOM);
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
        &[offer_price],
    );
    assert!(response.is_ok());

    let owner_balances_after =
        NativeBalance(router.wrap().query_all_balances(owner.clone()).unwrap());
    let bidder_balances_after = NativeBalance(router.wrap().query_all_balances(bidder).unwrap());
    let royalty_balances_after = NativeBalance(
        router
            .wrap()
            .query_all_balances(royalty_info.payment_address.clone())
            .unwrap(),
    );

    let sale_coin = ask_price;
    let fair_burn_amount = sale_coin
        .amount
        .mul_ceil(Decimal::bps(config.trading_fee_bps));
    let royalty_amount = sale_coin.amount.mul_ceil(royalty_info.share);
    let seller_amount = sale_coin.amount.sub(fair_burn_amount).sub(royalty_amount);

    assert_eq!(
        owner_balances_before
            .sub(config.listing_fee)
            .unwrap()
            .add(coin(seller_amount.u128(), NATIVE_DENOM)),
        owner_balances_after
    );
    assert_eq!(
        bidder_balances_before.sub(sale_coin).unwrap(),
        bidder_balances_after
    );
    assert_eq!(
        royalty_balances_before.add(coin(royalty_amount.u128(), NATIVE_DENOM)),
        royalty_balances_after
    );
}

#[test]
fn try_accept_offer_sale() {
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
    let _block_time = router.block_info().time;

    let config: Config<Addr> = router
        .wrap()
        .query_wasm_smart(&marketplace, &QueryMsg::Config {})
        .unwrap();

    let royalty_info: RoyaltyInfoResponse = router
        .wrap()
        .query_wasm_smart::<CollectionInfoResponse>(
            collection.clone(),
            &Sg721QueryMsg::CollectionInfo {},
        )
        .unwrap()
        .royalty_info
        .unwrap();

    let owner_balances_before =
        NativeBalance(router.wrap().query_all_balances(owner.clone()).unwrap());
    let bidder_balances_before =
        NativeBalance(router.wrap().query_all_balances(bidder.clone()).unwrap());
    let royalty_balances_before = NativeBalance(
        router
            .wrap()
            .query_all_balances(royalty_info.payment_address.clone())
            .unwrap(),
    );

    let token_id = "1";

    // Create ask with matching offer produces a valid sale
    let offer_price = coin(10_000_000, NATIVE_DENOM);
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

    mint_for(&mut router, &creator, &owner, &minter, token_id);
    approve(&mut router, &owner, &collection, &marketplace, token_id);

    let accept_offer = ExecuteMsg::AcceptOffer {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        creator: bidder.to_string(),
        order_options: None,
    };
    let response = router.execute_contract(owner.clone(), marketplace.clone(), &accept_offer, &[]);
    assert!(response.is_ok());

    let owner_balances_after =
        NativeBalance(router.wrap().query_all_balances(owner.clone()).unwrap());
    let bidder_balances_after = NativeBalance(router.wrap().query_all_balances(bidder).unwrap());
    let royalty_balances_after = NativeBalance(
        router
            .wrap()
            .query_all_balances(royalty_info.payment_address.clone())
            .unwrap(),
    );

    let sale_coin = offer_price;
    let fair_burn_amount = sale_coin
        .amount
        .mul_ceil(Decimal::bps(config.trading_fee_bps));
    let royalty_amount = sale_coin.amount.mul_ceil(royalty_info.share);
    let seller_amount = sale_coin.amount.sub(fair_burn_amount).sub(royalty_amount);

    assert_eq!(
        owner_balances_before.add(coin(seller_amount.u128(), NATIVE_DENOM)),
        owner_balances_after
    );
    assert_eq!(
        bidder_balances_before.sub(sale_coin).unwrap(),
        bidder_balances_after
    );
    assert_eq!(
        royalty_balances_before.add(coin(royalty_amount.u128(), NATIVE_DENOM)),
        royalty_balances_after
    );
}

#[test]
fn try_set_collection_offer_sale() {
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
    let _block_time = router.block_info().time;

    let config: Config<Addr> = router
        .wrap()
        .query_wasm_smart(&marketplace, &QueryMsg::Config {})
        .unwrap();

    let royalty_info: RoyaltyInfoResponse = router
        .wrap()
        .query_wasm_smart::<CollectionInfoResponse>(
            collection.clone(),
            &Sg721QueryMsg::CollectionInfo {},
        )
        .unwrap()
        .royalty_info
        .unwrap();

    let owner_balances_before =
        NativeBalance(router.wrap().query_all_balances(owner.clone()).unwrap());
    let bidder_balances_before =
        NativeBalance(router.wrap().query_all_balances(bidder.clone()).unwrap());
    let royalty_balances_before = NativeBalance(
        router
            .wrap()
            .query_all_balances(royalty_info.payment_address.clone())
            .unwrap(),
    );

    // Create ask with no matching offer
    let token_id = "1";
    mint_for(&mut router, &creator, &owner, &minter, token_id);
    approve(&mut router, &owner, &collection, &marketplace, token_id);
    let ask_price = coin(5_000_000, NATIVE_DENOM);

    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        price: ask_price.clone(),
        order_options: None,
    };
    let response = router.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &set_ask,
        &[config.listing_fee.clone()],
    );
    assert!(response.is_ok());

    // Create offer that matches ask
    let offer_price = coin(10_000_000, NATIVE_DENOM);
    let set_offer = ExecuteMsg::SetCollectionOffer {
        collection: collection.to_string(),
        price: offer_price.clone(),
        order_options: None,
    };
    let response = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_offer,
        &[offer_price],
    );
    assert!(response.is_ok());

    let owner_balances_after =
        NativeBalance(router.wrap().query_all_balances(owner.clone()).unwrap());
    let bidder_balances_after = NativeBalance(router.wrap().query_all_balances(bidder).unwrap());
    let royalty_balances_after = NativeBalance(
        router
            .wrap()
            .query_all_balances(royalty_info.payment_address.clone())
            .unwrap(),
    );

    let sale_coin = ask_price;
    let fair_burn_amount = sale_coin
        .amount
        .mul_ceil(Decimal::bps(config.trading_fee_bps));
    let royalty_amount = sale_coin.amount.mul_ceil(royalty_info.share);
    let seller_amount = sale_coin.amount.sub(fair_burn_amount).sub(royalty_amount);

    assert_eq!(
        owner_balances_before
            .sub(config.listing_fee)
            .unwrap()
            .add(coin(seller_amount.u128(), NATIVE_DENOM)),
        owner_balances_after
    );
    assert_eq!(
        bidder_balances_before.sub(sale_coin).unwrap(),
        bidder_balances_after
    );
    assert_eq!(
        royalty_balances_before.add(coin(royalty_amount.u128(), NATIVE_DENOM)),
        royalty_balances_after
    );
}

#[test]
fn try_accept_collection_offer_sale() {
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
    let _block_time = router.block_info().time;

    let config: Config<Addr> = router
        .wrap()
        .query_wasm_smart(&marketplace, &QueryMsg::Config {})
        .unwrap();

    let royalty_info: RoyaltyInfoResponse = router
        .wrap()
        .query_wasm_smart::<CollectionInfoResponse>(
            collection.clone(),
            &Sg721QueryMsg::CollectionInfo {},
        )
        .unwrap()
        .royalty_info
        .unwrap();

    let owner_balances_before =
        NativeBalance(router.wrap().query_all_balances(owner.clone()).unwrap());
    let bidder_balances_before =
        NativeBalance(router.wrap().query_all_balances(bidder.clone()).unwrap());
    let royalty_balances_before = NativeBalance(
        router
            .wrap()
            .query_all_balances(royalty_info.payment_address.clone())
            .unwrap(),
    );

    let token_id = "1";

    // Create ask with matching offer produces a valid sale
    let offer_price = coin(10_000_000, NATIVE_DENOM);
    let set_offer = ExecuteMsg::SetCollectionOffer {
        collection: collection.to_string(),
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

    mint_for(&mut router, &creator, &owner, &minter, token_id);
    approve(&mut router, &owner, &collection, &marketplace, token_id);

    // Create an Ask to test accepting an offer while the NFT is escrowed
    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        price: coin(20_000_000, NATIVE_DENOM),
        order_options: None,
    };
    let response = router.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &set_ask,
        &[config.listing_fee.clone()],
    );
    assert!(response.is_ok());

    let accept_collection_offer = ExecuteMsg::AcceptCollectionOffer {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        creator: bidder.to_string(),
        order_options: None,
    };
    let response = router.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &accept_collection_offer,
        &[],
    );
    assert!(response.is_ok());

    let owner_balances_after =
        NativeBalance(router.wrap().query_all_balances(owner.clone()).unwrap());
    let bidder_balances_after = NativeBalance(router.wrap().query_all_balances(bidder).unwrap());
    let royalty_balances_after = NativeBalance(
        router
            .wrap()
            .query_all_balances(royalty_info.payment_address.clone())
            .unwrap(),
    );

    let sale_coin = offer_price;
    let fair_burn_amount = sale_coin
        .amount
        .mul_ceil(Decimal::bps(config.trading_fee_bps));
    let royalty_amount = sale_coin.amount.mul_ceil(royalty_info.share);
    let seller_amount = sale_coin.amount.sub(fair_burn_amount).sub(royalty_amount);

    assert_eq!(
        owner_balances_before
            .sub(config.listing_fee)
            .unwrap()
            .add(coin(seller_amount.u128(), NATIVE_DENOM)),
        owner_balances_after
    );
    assert_eq!(
        bidder_balances_before.sub(sale_coin).unwrap(),
        bidder_balances_after
    );
    assert_eq!(
        royalty_balances_before.add(coin(royalty_amount.u128(), NATIVE_DENOM)),
        royalty_balances_after
    );
}
