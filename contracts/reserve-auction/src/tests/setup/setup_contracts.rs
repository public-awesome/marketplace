use cw_multi_test::{Contract, ContractWrapper};
use sg_std::StargazeMsgWrapper;

pub fn contract_fair_burn() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        stargaze_fair_burn::contract::execute,
        stargaze_fair_burn::contract::instantiate,
        stargaze_fair_burn::contract::query,
    )
    .with_sudo(stargaze_fair_burn::contract::sudo);
    Box::new(contract)
}

pub fn contract_royalty_registry() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        stargaze_royalty_registry::execute::execute,
        stargaze_royalty_registry::instantiate::instantiate,
        stargaze_royalty_registry::query::query,
    );
    Box::new(contract)
}

pub fn contract_reserve_auction() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        crate::execute::execute,
        crate::instantiate::instantiate,
        crate::query::query,
    )
    .with_sudo(crate::sudo::sudo);
    Box::new(contract)
}
