use cw_multi_test::{Contract, ContractWrapper};
use sg_std::StargazeMsgWrapper;

pub fn reserve_auction_contract() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        crate::execute::execute,
        crate::instantiate::instantiate,
        crate::query::query,
    )
    .with_sudo(crate::sudo::sudo);
    Box::new(contract)
}

pub fn contract_marketplace() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        sg_marketplace::execute::execute,
        sg_marketplace::execute::instantiate,
        sg_marketplace::query::query,
    )
    .with_sudo(sg_marketplace::sudo::sudo)
    .with_reply(sg_marketplace::execute::reply)
    .with_migrate(sg_marketplace::execute::migrate);
    Box::new(contract)
}
