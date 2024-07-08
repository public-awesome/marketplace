use crate::{
    msg::{ExecuteMsg, QueryMsg},
    orders::OrderDetails,
    state::Config,
    tests::{
        helpers::{
            marketplace::{approve, mint},
            utils::find_attrs,
        },
        setup::{
            setup_accounts::{setup_additional_account, TestAccounts},
            setup_contracts::NATIVE_DENOM,
            templates::{test_context, TestContext, TestContracts},
        },
    },
};

use cosmwasm_std::{coin, Addr, Decimal};
use cw_multi_test::Executor;
use cw_utils::NativeBalance;
use std::ops::{Add, Sub};

#[test]
fn try_set_ask_sale() {
    let TestContext {
        mut app,
        contracts:
            TestContracts {
                marketplace,
                collection,
                ..
            },
        accounts:
            TestAccounts {
                creator,
                owner,
                bidder,
                ..
            },
    } = test_context();

    let bidder2 = setup_additional_account(&mut app, "bidder2").unwrap();

    let config: Config<Addr> = app
        .wrap()
        .query_wasm_smart(&marketplace, &QueryMsg::Config {})
        .unwrap();

    let owner_balances_before =
        NativeBalance(app.wrap().query_all_balances(owner.clone()).unwrap());
    let bidder2_balances_before =
        NativeBalance(app.wrap().query_all_balances(bidder2.clone()).unwrap());

    let token_id = "1";

    // Create ask with matching offer produces a valid sale

    // * Offer 1 - 10_000_000 native denom (should not match)
    let offer_price_1 = coin(10_000_000, NATIVE_DENOM);
    let set_offer = ExecuteMsg::SetOffer {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        details: OrderDetails {
            price: offer_price_1.clone(),
            recipient: None,
            finder: None,
        },
    };
    let response = app.execute_contract(bidder, marketplace.clone(), &set_offer, &[offer_price_1]);
    assert!(response.is_ok());

    // * Offer 2 - 15_000_000 native denom (should_match)
    let offer_price_2 = coin(15_000_000, NATIVE_DENOM);
    let set_offer = ExecuteMsg::SetOffer {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        details: OrderDetails {
            price: offer_price_2.clone(),
            recipient: None,
            finder: None,
        },
    };
    let response = app.execute_contract(
        bidder2.clone(),
        marketplace.clone(),
        &set_offer,
        &[offer_price_2.clone()],
    );
    assert!(response.is_ok());

    mint(&mut app, &creator, &owner, &collection, token_id);
    approve(&mut app, &owner, &collection, &marketplace, token_id);

    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        details: OrderDetails {
            price: coin(5_000_000, NATIVE_DENOM),
            recipient: None,
            finder: None,
        },
    };
    let response = app.execute_contract(owner.clone(), marketplace.clone(), &set_ask, &[]);
    assert!(response.is_ok());

    let owner_balances_after = NativeBalance(app.wrap().query_all_balances(owner.clone()).unwrap());
    let bidder2_balances_after = NativeBalance(app.wrap().query_all_balances(bidder2).unwrap());

    let sale_coin = offer_price_2;
    let fair_burn_amount = sale_coin
        .amount
        .mul_ceil(Decimal::bps(config.protocol_fee_bps));
    let seller_amount = sale_coin.amount.sub(fair_burn_amount);

    assert_eq!(
        owner_balances_before.add(coin(seller_amount.u128(), NATIVE_DENOM)),
        owner_balances_after
    );
    assert_eq!(
        bidder2_balances_before.sub(sale_coin).unwrap(),
        bidder2_balances_after
    );
}

#[test]
fn try_accept_ask_sale() {
    let TestContext {
        mut app,
        contracts:
            TestContracts {
                marketplace,
                collection,
                ..
            },
        accounts:
            TestAccounts {
                creator,
                owner,
                bidder,
                ..
            },
    } = test_context();

    let config: Config<Addr> = app
        .wrap()
        .query_wasm_smart(&marketplace, &QueryMsg::Config {})
        .unwrap();

    let owner_balances_before =
        NativeBalance(app.wrap().query_all_balances(owner.clone()).unwrap());
    let bidder_balances_before =
        NativeBalance(app.wrap().query_all_balances(bidder.clone()).unwrap());

    // Create ask with no matching offer
    let token_id = "1";
    mint(&mut app, &creator, &owner, &collection, token_id);
    approve(&mut app, &owner, &collection, &marketplace, token_id);
    let ask_price = coin(5_000_000, NATIVE_DENOM);

    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        details: OrderDetails {
            price: ask_price.clone(),
            recipient: None,
            finder: None,
        },
    };
    let response = app.execute_contract(owner.clone(), marketplace.clone(), &set_ask, &[]);
    assert!(response.is_ok());
    let ask_id = find_attrs(response.unwrap(), "wasm-set-ask", "id")
        .pop()
        .unwrap();

    // Accept ask directly
    let accept_ask = ExecuteMsg::AcceptAsk {
        id: ask_id,
        max_input: ask_price.clone(),
        recipient: None,
        finder: None,
    };
    let response = app.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &accept_ask,
        &[ask_price.clone()],
    );
    assert!(response.is_ok());

    let owner_balances_after = NativeBalance(app.wrap().query_all_balances(owner.clone()).unwrap());
    let bidder_balances_after = NativeBalance(app.wrap().query_all_balances(bidder).unwrap());

    let sale_coin = ask_price;
    let fair_burn_amount = sale_coin
        .amount
        .mul_ceil(Decimal::bps(config.protocol_fee_bps));
    let seller_amount = sale_coin.amount.sub(fair_burn_amount);

    assert_eq!(
        owner_balances_before.add(coin(seller_amount.u128(), NATIVE_DENOM)),
        owner_balances_after
    );
    assert_eq!(
        bidder_balances_before.sub(sale_coin).unwrap(),
        bidder_balances_after
    );
}

#[test]
fn try_set_offer_sale() {
    let TestContext {
        mut app,
        contracts:
            TestContracts {
                marketplace,
                collection,
                ..
            },
        accounts:
            TestAccounts {
                creator,
                owner,
                bidder,
                ..
            },
    } = test_context();

    let config: Config<Addr> = app
        .wrap()
        .query_wasm_smart(&marketplace, &QueryMsg::Config {})
        .unwrap();

    let owner_balances_before =
        NativeBalance(app.wrap().query_all_balances(owner.clone()).unwrap());
    let bidder_balances_before =
        NativeBalance(app.wrap().query_all_balances(bidder.clone()).unwrap());

    // Create ask with no matching offer
    let token_id = "1";
    mint(&mut app, &creator, &owner, &collection, token_id);
    approve(&mut app, &owner, &collection, &marketplace, token_id);
    let ask_price = coin(5_000_000, NATIVE_DENOM);

    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        details: OrderDetails {
            price: ask_price.clone(),
            recipient: None,
            finder: None,
        },
    };
    let response = app.execute_contract(owner.clone(), marketplace.clone(), &set_ask, &[]);
    assert!(response.is_ok());

    // Create offer that matches ask
    let offer_price = coin(10_000_000, NATIVE_DENOM);
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

    let owner_balances_after = NativeBalance(app.wrap().query_all_balances(owner.clone()).unwrap());
    let bidder_balances_after = NativeBalance(app.wrap().query_all_balances(bidder).unwrap());

    let sale_coin = ask_price;
    let fair_burn_amount = sale_coin
        .amount
        .mul_ceil(Decimal::bps(config.protocol_fee_bps));
    let seller_amount = sale_coin.amount.sub(fair_burn_amount);

    assert_eq!(
        owner_balances_before.add(coin(seller_amount.u128(), NATIVE_DENOM)),
        owner_balances_after
    );
    assert_eq!(
        bidder_balances_before.sub(sale_coin).unwrap(),
        bidder_balances_after
    );
}

#[test]
fn try_accept_offer_sale() {
    let TestContext {
        mut app,
        contracts:
            TestContracts {
                marketplace,
                collection,
                ..
            },
        accounts:
            TestAccounts {
                creator,
                owner,
                bidder,
                ..
            },
    } = test_context();

    let config: Config<Addr> = app
        .wrap()
        .query_wasm_smart(&marketplace, &QueryMsg::Config {})
        .unwrap();

    let owner_balances_before =
        NativeBalance(app.wrap().query_all_balances(owner.clone()).unwrap());
    let bidder_balances_before =
        NativeBalance(app.wrap().query_all_balances(bidder.clone()).unwrap());

    let token_id = "1";

    // Create ask with matching offer produces a valid sale
    let offer_price = coin(10_000_000, NATIVE_DENOM);
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
        &[offer_price.clone()],
    );
    assert!(response.is_ok());
    let offer_id = find_attrs(response.unwrap(), "wasm-set-offer", "id")
        .pop()
        .unwrap();

    mint(&mut app, &creator, &owner, &collection, token_id);
    approve(&mut app, &owner, &collection, &marketplace, token_id);

    let accept_offer = ExecuteMsg::AcceptOffer {
        id: offer_id,
        min_output: offer_price.clone(),
        recipient: None,
        finder: None,
    };
    let response = app.execute_contract(owner.clone(), marketplace.clone(), &accept_offer, &[]);
    assert!(response.is_ok());

    let owner_balances_after = NativeBalance(app.wrap().query_all_balances(owner.clone()).unwrap());
    let bidder_balances_after = NativeBalance(app.wrap().query_all_balances(bidder).unwrap());

    let sale_coin = offer_price;
    let fair_burn_amount = sale_coin
        .amount
        .mul_ceil(Decimal::bps(config.protocol_fee_bps));
    let seller_amount = sale_coin.amount.sub(fair_burn_amount);

    assert_eq!(
        owner_balances_before.add(coin(seller_amount.u128(), NATIVE_DENOM)),
        owner_balances_after
    );
    assert_eq!(
        bidder_balances_before.sub(sale_coin).unwrap(),
        bidder_balances_after
    );
}

#[test]
fn try_set_collection_offer_sale() {
    let TestContext {
        mut app,
        contracts:
            TestContracts {
                marketplace,
                collection,
                ..
            },
        accounts:
            TestAccounts {
                creator,
                owner,
                bidder,
                ..
            },
    } = test_context();

    let config: Config<Addr> = app
        .wrap()
        .query_wasm_smart(&marketplace, &QueryMsg::Config {})
        .unwrap();

    let owner_balances_before =
        NativeBalance(app.wrap().query_all_balances(owner.clone()).unwrap());
    let bidder_balances_before =
        NativeBalance(app.wrap().query_all_balances(bidder.clone()).unwrap());

    // Create ask with no matching offer
    let token_id = "1";
    mint(&mut app, &creator, &owner, &collection, token_id);
    approve(&mut app, &owner, &collection, &marketplace, token_id);
    let ask_price = coin(5_000_000, NATIVE_DENOM);

    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        details: OrderDetails {
            price: ask_price.clone(),
            recipient: None,
            finder: None,
        },
    };
    let response = app.execute_contract(owner.clone(), marketplace.clone(), &set_ask, &[]);
    assert!(response.is_ok());

    // Create offer that matches ask
    let offer_price = coin(10_000_000, NATIVE_DENOM);
    let set_offer = ExecuteMsg::SetCollectionOffer {
        collection: collection.to_string(),
        details: OrderDetails {
            price: ask_price.clone(),
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

    let owner_balances_after = NativeBalance(app.wrap().query_all_balances(owner.clone()).unwrap());
    let bidder_balances_after = NativeBalance(app.wrap().query_all_balances(bidder).unwrap());

    let sale_coin = ask_price;
    let fair_burn_amount = sale_coin
        .amount
        .mul_ceil(Decimal::bps(config.protocol_fee_bps));
    let seller_amount = sale_coin.amount.sub(fair_burn_amount);

    assert_eq!(
        owner_balances_before.add(coin(seller_amount.u128(), NATIVE_DENOM)),
        owner_balances_after
    );
    assert_eq!(
        bidder_balances_before.sub(sale_coin).unwrap(),
        bidder_balances_after
    );
}

#[test]
fn try_accept_collection_offer_sale() {
    let TestContext {
        mut app,
        contracts:
            TestContracts {
                marketplace,
                collection,
                ..
            },
        accounts:
            TestAccounts {
                creator,
                owner,
                bidder,
                ..
            },
    } = test_context();

    let config: Config<Addr> = app
        .wrap()
        .query_wasm_smart(&marketplace, &QueryMsg::Config {})
        .unwrap();

    let owner_balances_before =
        NativeBalance(app.wrap().query_all_balances(owner.clone()).unwrap());
    let bidder_balances_before =
        NativeBalance(app.wrap().query_all_balances(bidder.clone()).unwrap());

    let token_id = "1";

    // Create ask with matching offer produces a valid sale
    let offer_price = coin(10_000_000, NATIVE_DENOM);
    let set_offer = ExecuteMsg::SetCollectionOffer {
        collection: collection.to_string(),
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
        &[offer_price.clone()],
    );
    assert!(response.is_ok());
    let collection_offer_id = find_attrs(response.unwrap(), "wasm-set-collection-offer", "id")
        .pop()
        .unwrap();

    mint(&mut app, &creator, &owner, &collection, token_id);
    approve(&mut app, &owner, &collection, &marketplace, token_id);

    // Create an Ask to test accepting an offer while the NFT is escrowed
    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        details: OrderDetails {
            price: coin(20_000_000, NATIVE_DENOM),
            recipient: None,
            finder: None,
        },
    };
    let response = app.execute_contract(owner.clone(), marketplace.clone(), &set_ask, &[]);
    assert!(response.is_ok());

    let accept_collection_offer = ExecuteMsg::AcceptCollectionOffer {
        id: collection_offer_id,
        token_id: token_id.to_string(),
        min_output: offer_price.clone(),
        recipient: None,
        finder: None,
    };
    let response = app.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &accept_collection_offer,
        &[],
    );
    assert!(response.is_ok());

    let owner_balances_after = NativeBalance(app.wrap().query_all_balances(owner.clone()).unwrap());
    let bidder_balances_after = NativeBalance(app.wrap().query_all_balances(bidder).unwrap());

    let sale_coin = offer_price;
    let fair_burn_amount = sale_coin
        .amount
        .mul_ceil(Decimal::bps(config.protocol_fee_bps));
    let seller_amount = sale_coin.amount.sub(fair_burn_amount);

    assert_eq!(
        owner_balances_before.add(coin(seller_amount.u128(), NATIVE_DENOM)),
        owner_balances_after
    );
    assert_eq!(
        bidder_balances_before.sub(sale_coin).unwrap(),
        bidder_balances_after
    );
}

#[test]
fn try_sale_fee_breakdown() {
    let TestContext {
        mut app,
        contracts:
            TestContracts {
                marketplace,
                collection,
                ..
            },
        accounts:
            TestAccounts {
                creator,
                owner,
                bidder,
                fee_manager,
                ..
            },
    } = test_context();

    let config: Config<Addr> = app
        .wrap()
        .query_wasm_smart(&marketplace, &QueryMsg::Config {})
        .unwrap();

    let bidder_balances_before =
        NativeBalance(app.wrap().query_all_balances(bidder.clone()).unwrap());

    // Create ask with a finder
    let maker: Addr = Addr::unchecked("maker".to_string());
    let tokens_recipient: Addr = Addr::unchecked("tokens_recipient".to_string());
    let token_id = "1";
    mint(&mut app, &creator, &owner, &collection, token_id);
    approve(&mut app, &owner, &collection, &marketplace, token_id);
    let ask_price = coin(5_000_000, NATIVE_DENOM);

    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        details: OrderDetails {
            price: ask_price.clone(),
            recipient: Some(tokens_recipient.to_string()),
            finder: Some(maker.to_string()),
        },
    };
    let response = app.execute_contract(owner.clone(), marketplace.clone(), &set_ask, &[]);
    assert!(response.is_ok());

    // Accept ask with a taker
    let taker: Addr = Addr::unchecked("taker".to_string());
    let nft_recipient: Addr = Addr::unchecked("nft_recipient".to_string());
    let accept_ask = ExecuteMsg::AcceptAsk {
        id: ask_id,
        max_input: ask_price.clone(),
        recipient: Some(nft_recipient.to_string()),
        finder: Some(taker.to_string()),
    };
    let response = app.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &buy_collection_nft,
        &[ask_price.clone()],
    );
    assert!(response.is_ok());

    // Fetch balances after sale
    let fee_manager_balances_after =
        NativeBalance(app.wrap().query_all_balances(fee_manager).unwrap());

    let maker_balances_after = NativeBalance(app.wrap().query_all_balances(maker.clone()).unwrap());
    let taker_balances_after = NativeBalance(app.wrap().query_all_balances(taker).unwrap());
    let bidder_balances_after = NativeBalance(app.wrap().query_all_balances(bidder).unwrap());
    let tokens_recipient_balances_after = NativeBalance(
        app.wrap()
            .query_all_balances(tokens_recipient.clone())
            .unwrap(),
    );

    // Calculate expected balances
    let sale_coin = ask_price;
    let protocol_reward_total = sale_coin
        .amount
        .mul_ceil(Decimal::bps(config.protocol_fee_bps));
    let maker_reward = protocol_reward_total.mul_ceil(Decimal::bps(config.maker_reward_bps));
    let taker_reward = protocol_reward_total.mul_ceil(Decimal::bps(config.taker_reward_bps));
    let protocol_reward_final = protocol_reward_total.sub(maker_reward).sub(taker_reward);
    let seller_amount = sale_coin.amount.sub(protocol_reward_total);

    let app_response = response.unwrap();

    // Verify protocol reward
    let protocol_reward_coin = coin(protocol_reward_final.u128(), NATIVE_DENOM);
    assert_eq!(
        fee_manager_balances_after,
        NativeBalance(vec![protocol_reward_coin.clone()])
    );
    let protocol_reward_event = find_attrs(app_response.clone(), "wasm-finalize-sale", "protocol")
        .pop()
        .unwrap();
    assert_eq!(
        protocol_reward_event,
        protocol_reward_coin.amount.to_string()
    );

    // Verify maker reward
    let maker_reward_coin = coin(maker_reward.u128(), NATIVE_DENOM);
    assert_eq!(
        maker_balances_after,
        NativeBalance(vec![maker_reward_coin.clone()])
    );
    let maker_reward_event = find_attrs(app_response.clone(), "wasm-finalize-sale", "maker")
        .pop()
        .unwrap();
    assert_eq!(maker_reward_event, maker_reward_coin.amount.to_string());

    // Verify taker reward
    let taker_reward_coin = coin(taker_reward.u128(), NATIVE_DENOM);
    assert_eq!(
        taker_balances_after,
        NativeBalance(vec![taker_reward_coin.clone()])
    );
    let taker_reward_event = find_attrs(app_response.clone(), "wasm-finalize-sale", "taker")
        .pop()
        .unwrap();
    assert_eq!(taker_reward_event, taker_reward_coin.amount.to_string());

    // Verify seller reward
    let seller_coin = coin(seller_amount.u128(), NATIVE_DENOM);
    assert_eq!(
        tokens_recipient_balances_after,
        NativeBalance(vec![seller_coin.clone()])
    );
    let seller_event = find_attrs(app_response.clone(), "wasm-finalize-sale", "seller")
        .pop()
        .unwrap();
    assert_eq!(seller_event, seller_coin.amount.to_string());

    // Verify bidder paid
    assert_eq!(
        bidder_balances_before.sub(sale_coin.clone()).unwrap(),
        bidder_balances_after
    );
}
