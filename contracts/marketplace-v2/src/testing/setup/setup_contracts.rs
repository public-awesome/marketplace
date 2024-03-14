use crate::ContractError;

use cosmwasm_std::{Addr, Decimal};
use cw_multi_test::{Contract, ContractWrapper, Executor};
use sg_multi_test::StargazeApp;
use sg_std::StargazeMsgWrapper;
use stargaze_fair_burn::msg::InstantiateMsg as FairBurnInstantiateMsg;
use stargaze_royalty_registry::msg::InstantiateMsg as RoyaltyRegistryInstantiateMsg;
use stargaze_royalty_registry::state::Config as RoyaltyRegistryConfig;

pub fn contract_fair_burn() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        stargaze_fair_burn::contract::execute,
        stargaze_fair_burn::contract::instantiate,
        stargaze_fair_burn::contract::query,
    );
    Box::new(contract)
}

pub fn setup_fair_burn(router: &mut StargazeApp, creator: &Addr) -> Result<Addr, ContractError> {
    let fair_burn_id = router.store_code(contract_fair_burn());
    let fair_burn = router
        .instantiate_contract(
            fair_burn_id,
            creator.clone(),
            &FairBurnInstantiateMsg { fee_bps: 5000 },
            &[],
            "FairBurn",
            None,
        )
        .unwrap();
    Ok(fair_burn)
}

pub fn contract_royalty_registry() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        stargaze_royalty_registry::execute::execute,
        stargaze_royalty_registry::instantiate::instantiate,
        stargaze_royalty_registry::query::query,
    );
    Box::new(contract)
}

pub fn setup_royalty_registry(
    router: &mut StargazeApp,
    creator: &Addr,
) -> Result<Addr, ContractError> {
    let royalty_registry_id = router.store_code(contract_royalty_registry());
    let royalty_registry = router
        .instantiate_contract(
            royalty_registry_id,
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

pub fn contract_marketplace() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        crate::execute::execute,
        crate::instantiate::instantiate,
        crate::query::query,
    )
    .with_sudo(crate::sudo::sudo)
    .with_migrate(crate::migrate::migrate);
    Box::new(contract)
}
