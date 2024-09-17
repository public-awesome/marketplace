use crate::{
    msg::{ExecuteMsg, QueryMsg},
    orders::OrderDetails,
    state::Config,
    tests::{
        helpers::{
            marketplace::{approve, mint},
            utils::{assert_error, find_attrs},
        },
        setup::{
            setup_accounts::TestAccounts,
            setup_contracts::{ATOM_DENOM, NATIVE_DENOM},
            templates::{test_context, TestContext, TestContracts},
        },
    },
    ContractError,
};

use cosmwasm_std::{coin, Addr};
use cw_multi_test::Executor;
use sg_marketplace_common::MarketplaceStdError;

#[test]
fn try_admin_update_config() {
    let TestContext {
        mut app,
        contracts: TestContracts { marketplace, .. },
        accounts: TestAccounts { creator, owner, .. },
    } = test_context();

    let config: Config<Addr> = app
        .wrap()
        .query_wasm_smart(&marketplace, &QueryMsg::Config {})
        .unwrap();

    let delta = 1u64;
    let fee_manager = "fee_manager_test".to_string();
    let royalty_registry = "royalty_registry_test".to_string();
    let protocol_fee_bps = config.protocol_fee_bps + delta;
    let max_royalty_fee_bps = config.max_royalty_fee_bps + delta;
    let maker_reward_bps = config.maker_reward_bps + delta;
    let taker_reward_bps = config.taker_reward_bps + delta;

    let update_config_msg = ExecuteMsg::UpdateConfig {
        config: Config {
            fee_manager: fee_manager.clone(),
            royalty_registry: royalty_registry.clone(),
            protocol_fee_bps,
            max_royalty_fee_bps,
            maker_reward_bps,
            taker_reward_bps,
            default_denom: NATIVE_DENOM.to_string(),
        },
    };

    // None admin cannot update config
    let response = app.execute_contract(owner, marketplace.clone(), &update_config_msg, &[]);
    assert_error(
        response,
        MarketplaceStdError::Unauthorized(
            "only the admin of contract can perform this action".to_string(),
        )
        .to_string(),
    );

    let invalid_update_config_msg = ExecuteMsg::UpdateConfig {
        config: Config {
            fee_manager: fee_manager.clone(),
            royalty_registry: royalty_registry.clone(),
            protocol_fee_bps,
            max_royalty_fee_bps,
            maker_reward_bps: 5000,
            taker_reward_bps: 6000,
            default_denom: NATIVE_DENOM.to_string(),
        },
    };
    // None admin cannot update config
    let response = app.execute_contract(
        creator.clone(),
        marketplace.clone(),
        &invalid_update_config_msg,
        &[],
    );
    assert_error(
        response,
        ContractError::InvalidInput(
            "taker and maker reward bps must be less than 1 combined".to_string(),
        )
        .to_string(),
    );
    let response = app.execute_contract(creator, marketplace.clone(), &update_config_msg, &[]);
    assert!(response.is_ok());
    let config: Config<Addr> = app
        .wrap()
        .query_wasm_smart(&marketplace, &QueryMsg::Config {})
        .unwrap();

    assert_eq!(config.fee_manager, fee_manager);
    assert_eq!(config.royalty_registry, royalty_registry);
    assert_eq!(config.protocol_fee_bps, protocol_fee_bps);
    assert_eq!(config.max_royalty_fee_bps, max_royalty_fee_bps);
    assert_eq!(config.maker_reward_bps, maker_reward_bps);
    assert_eq!(config.taker_reward_bps, taker_reward_bps);
}

#[test]
fn try_admin_update_collection_denom() {
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

    // Create bid succeeds
    let recipient = Addr::unchecked("recipient".to_string());
    let finder = Addr::unchecked("finder".to_string());
    let token_id = "1";
    let bid_price = coin(1_000_000, NATIVE_DENOM);
    let set_bid = ExecuteMsg::SetBid {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        details: OrderDetails {
            price: bid_price.clone(),
            recipient: Some(recipient.to_string()),
            finder: Some(finder.to_string()),
        },
    };
    let response = app.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_bid,
        &[bid_price.clone()],
    );
    assert!(response.is_ok());
    let bid_id = find_attrs(response.unwrap(), "wasm-set-bid", "id")
        .pop()
        .unwrap();

    let update_collection_denom = ExecuteMsg::UpdateCollectionDenom {
        collection: collection.to_string(),
        denom: ATOM_DENOM.to_string(),
    };

    // None admin cannot update config
    let response = app.execute_contract(
        owner.clone(),
        marketplace.clone(),
        &update_collection_denom,
        &[],
    );
    assert_error(
        response,
        MarketplaceStdError::Unauthorized(
            "only the admin of contract can perform this action".to_string(),
        )
        .to_string(),
    );

    // Admin can update collection denom
    let response = app.execute_contract(
        creator.clone(),
        marketplace.clone(),
        &update_collection_denom,
        &[],
    );
    assert!(response.is_ok());

    // Accept invalid denom bid succeeds
    mint(&mut app, &creator, &owner, &collection, &token_id);
    approve(&mut app, &owner, &collection, &marketplace, &token_id);
    let accept_bid = ExecuteMsg::AcceptBid {
        id: bid_id,
        details: OrderDetails {
            price: bid_price.clone(),
            recipient: None,
            finder: None,
        },
    };
    let response = app.execute_contract(owner.clone(), marketplace.clone(), &accept_bid, &[]);
    assert!(response.is_ok());

    // Create bid fails with old denom fails
    let token_id = "2";
    let set_bid = ExecuteMsg::SetBid {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        details: OrderDetails {
            price: bid_price.clone(),
            recipient: Some(recipient.to_string()),
            finder: Some(finder.to_string()),
        },
    };
    let response =
        app.execute_contract(bidder.clone(), marketplace.clone(), &set_bid, &[bid_price]);
    assert_error(
        response,
        ContractError::InvalidInput("invalid denom".to_string()).to_string(),
    );

    // Create bid succeeds with new denom
    let bid_price = coin(1_000_000, ATOM_DENOM);
    let set_bid: ExecuteMsg = ExecuteMsg::SetBid {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        details: OrderDetails {
            price: bid_price.clone(),
            recipient: Some(recipient.to_string()),
            finder: Some(finder.to_string()),
        },
    };
    let response = app.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_bid,
        &[bid_price.clone()],
    );
    assert!(response.is_ok());
}
