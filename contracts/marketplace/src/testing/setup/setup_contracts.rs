use cw_multi_test::{Contract, ContractWrapper};
use sg_std::StargazeMsgWrapper;

pub fn contract_fair_burn() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        stargaze_fair_burn::contract::execute,
        stargaze_fair_burn::contract::instantiate,
        stargaze_fair_burn::contract::query,
    );
    Box::new(contract)
}

pub fn contract_marketplace() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        crate::execute::execute,
        crate::instantiate::instantiate,
        crate::query::query,
    )
    .with_sudo(crate::sudo::sudo)
    .with_reply(crate::reply::reply)
    .with_migrate(crate::migrate::migrate);
    Box::new(contract)
}
