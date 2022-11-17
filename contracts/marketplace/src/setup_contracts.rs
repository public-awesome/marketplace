use cosmwasm_std::{Timestamp};
use cw_multi_test::{Contract, ContractWrapper};

use sg_multi_test::StargazeApp;
use sg_std::StargazeMsgWrapper;


pub fn custom_mock_app() -> StargazeApp {
    StargazeApp::default()
}

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

pub fn contract_sg721() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        sg721_base::entry::execute,
        sg721_base::entry::instantiate,
        sg721_base::entry::query,
    );
    Box::new(contract)
}

pub fn contract_factory() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        vending_factory::contract::execute,
        vending_factory::contract::instantiate,
        vending_factory::contract::query,
    );
    Box::new(contract)
}

pub fn contract_minter() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        vending_minter::contract::execute,
        vending_minter::contract::instantiate,
        vending_minter::contract::query,
    )
    .with_reply(vending_minter::contract::reply);
    Box::new(contract)
}


pub fn setup_block_time(router: &mut StargazeApp, seconds: u64) {
    let mut block = router.block_info();
    block.time = Timestamp::from_seconds(seconds);
    router.set_block(block);
}
