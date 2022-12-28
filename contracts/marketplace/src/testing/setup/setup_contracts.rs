use cw_multi_test::{Contract, ContractWrapper};
use sg_std::StargazeMsgWrapper;

pub fn contract_marketplace() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        crate::execute::execute,
        crate::execute::instantiate,
        crate::query::query,
    )
    .with_sudo(crate::sudo::sudo)
    .with_reply(crate::execute::reply)
    .with_migrate(crate::execute::migrate);
    Box::new(contract)
}
