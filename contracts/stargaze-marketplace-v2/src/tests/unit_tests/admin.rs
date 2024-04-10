use crate::{
    msg::{ExecuteMsg, QueryMsg},
    state::{AllowDenoms, Config},
    tests::{
        helpers::utils::assert_error,
        setup::{
            setup_accounts::TestAccounts,
            templates::{test_context, TestContext, TestContracts},
        },
    },
};

use cosmwasm_std::Addr;
use cw_multi_test::Executor;
use sg_marketplace_common::MarketplaceStdError;
use std::vec;

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
fn try_admin_update_allow_denoms() {
    let TestContext {
        mut app,
        contracts: TestContracts { marketplace, .. },
        accounts: TestAccounts { creator, owner, .. },
    } = test_context();

    let new_denom_setting = AllowDenoms::Excludes(vec!["ujuno".to_string()]);
    let update_allow_denoms = ExecuteMsg::UpdateAllowDenoms {
        allow_denoms: new_denom_setting.clone(),
    };

    // None admin cannot update config
    let response = app.execute_contract(owner, marketplace.clone(), &update_allow_denoms, &[]);
    assert_error(
        response,
        MarketplaceStdError::Unauthorized(
            "only the admin of contract can perform this action".to_string(),
        )
        .to_string(),
    );

    let response = app.execute_contract(creator, marketplace.clone(), &update_allow_denoms, &[]);
    assert!(response.is_ok());
    let allow_denoms: AllowDenoms = app
        .wrap()
        .query_wasm_smart(&marketplace, &QueryMsg::AllowDenoms {})
        .unwrap();

    assert_eq!(allow_denoms, new_denom_setting);
}
