use super::setup_contracts::contract_royalty_registry;
use cosmwasm_std::{Addr, Decimal};
use cw_multi_test::Executor;
use sg_multi_test::StargazeApp;
use stargaze_royalty_registry::msg::InstantiateMsg as RoyaltyRegistryInstantiateMsg;
use stargaze_royalty_registry::state::Config as RoyaltyRegistryConfig;

pub fn setup_royalty_registry(router: &mut StargazeApp, creator: Addr) -> Addr {
    let code_id = router.store_code(contract_royalty_registry());
    router
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
            "RoyaltyRegistry",
            None,
        )
        .unwrap()
}
