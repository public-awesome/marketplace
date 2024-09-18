use crate::tests::setup::{
    setup_accounts::setup_accounts,
    setup_contracts::{setup_cw721, setup_marketplace, setup_royalty_registry},
};

use cosmwasm_std::Addr;
use cw_multi_test::App;

use super::setup_accounts::TestAccounts;

pub struct TestContracts {
    pub collection: Addr,
    pub marketplace: Addr,
    #[allow(dead_code)]
    pub royalty_registry: Addr,
}

pub struct TestContext {
    pub app: App,
    pub accounts: TestAccounts,
    pub contracts: TestContracts,
}

pub fn test_context() -> TestContext {
    let mut app = App::default();
    let accounts = setup_accounts(&mut app).unwrap();

    let royalty_registry = setup_royalty_registry(&mut app, &accounts.creator).unwrap();

    let marketplace = setup_marketplace(
        &mut app,
        accounts.fee_manager.clone(),
        royalty_registry.clone(),
        accounts.creator.clone(),
    )
    .unwrap();

    let collection = setup_cw721(&mut app, &accounts.creator).unwrap();

    TestContext {
        app,
        accounts,
        contracts: TestContracts {
            collection,
            royalty_registry,
            marketplace,
        },
    }
}
