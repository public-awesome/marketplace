use cosmwasm_std::Addr;
use cw_multi_test::Executor;
use sg_multi_test::StargazeApp;

use super::setup_contracts::contract_fair_burn;

pub const FAIR_BURN_FEE_BPS: u64 = 5000;

pub fn setup_fair_burn(router: &mut StargazeApp, creator: Addr) -> Addr {
    let fair_burn_id = router.store_code(contract_fair_burn());
    let msg = sg_fair_burn::msg::InstantiateMsg {
        fee_bps: FAIR_BURN_FEE_BPS,
    };
    router
        .instantiate_contract(fair_burn_id, creator, &msg, &[], "FairBurn", None)
        .unwrap()
}
