use crate::ContractError;
use crate::{msg::InstantiateMsg, state::Config};

use cosmwasm_std::{Addr, Decimal, Empty};
use cw721_base::InstantiateMsg as Cw721InstantiateMsg;
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use stargaze_royalty_registry::msg::InstantiateMsg as RoyaltyRegistryInstantiateMsg;
use stargaze_royalty_registry::state::Config as RoyaltyRegistryConfig;

pub const NATIVE_DENOM: &str = "ustars";
pub const ATOM_DENOM: &str = "uatom";
pub const JUNO_DENOM: &str = "ujuno";

pub fn contract_cw721() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw721_base::entry::execute,
        cw721_base::entry::instantiate,
        cw721_base::entry::query,
    );
    Box::new(contract)
}

pub fn setup_cw721(app: &mut App, creator: &Addr) -> Result<Addr, ContractError> {
    let code_id = app.store_code(contract_cw721());
    let royalty_registry = app
        .instantiate_contract(
            code_id,
            creator.clone(),
            &Cw721InstantiateMsg {
                name: "Test Collection".to_string(),
                symbol: "TC".to_string(),
                minter: creator.to_string(),
            },
            &[],
            "CW721",
            None,
        )
        .unwrap();
    Ok(royalty_registry)
}

pub fn contract_royalty_registry() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        stargaze_royalty_registry::execute::execute,
        stargaze_royalty_registry::instantiate::instantiate,
        stargaze_royalty_registry::query::query,
    );
    Box::new(contract)
}

pub fn contract_marketplace() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::execute::execute,
        crate::instantiate::instantiate,
        crate::query::query,
    )
    .with_migrate(crate::migrate::migrate);
    Box::new(contract)
}

pub fn setup_royalty_registry(app: &mut App, creator: &Addr) -> Result<Addr, ContractError> {
    let code_id = app.store_code(contract_royalty_registry());
    let royalty_registry = app
        .instantiate_contract(
            code_id,
            creator.clone(),
            &RoyaltyRegistryInstantiateMsg {
                config: RoyaltyRegistryConfig {
                    update_wait_period: 24 * 60 * 60,
                    max_share_delta: Decimal::percent(10),
                },
            },
            &[],
            "FairBurn",
            None,
        )
        .unwrap();
    Ok(royalty_registry)
}

pub fn setup_marketplace(
    app: &mut App,
    fee_manager: Addr,
    royalty_registry: Addr,
    marketplace_admin: Addr,
) -> Result<Addr, ContractError> {
    let marketplace_id = app.store_code(contract_marketplace());
    let msg = InstantiateMsg {
        config: Config {
            fee_manager: fee_manager.to_string(),
            royalty_registry: royalty_registry.to_string(),
            protocol_fee_bps: 200,
            max_royalty_fee_bps: 1000,
            maker_reward_bps: 4000,
            taker_reward_bps: 1000,
        },
        allow_denoms: crate::state::AllowDenoms::Includes(vec![
            NATIVE_DENOM.to_string(),
            ATOM_DENOM.to_string(),
        ]),
    };
    let marketplace = app
        .instantiate_contract(
            marketplace_id,
            marketplace_admin.clone(),
            &msg,
            &[],
            "Marketplace",
            Some(marketplace_admin.to_string()),
        )
        .unwrap();
    Ok(marketplace)
}
