use cosmwasm_std::{Addr, Uint128};
use cw_multi_test::Executor;
use cw_utils::Duration;
use sg_marketplace::ContractError;
use sg_marketplace::ExpiryRange;
use sg_multi_test::StargazeApp;

use crate::tests::setup::setup_contracts::contract_marketplace;

pub const LISTING_FEE: u128 = 100;
// Governance parameters
pub const TRADING_FEE_BPS: u64 = 200; // 2%
pub const MIN_EXPIRY: u64 = 24 * 60 * 60; // 24 hours (in seconds)
pub const MAX_EXPIRY: u64 = 180 * 24 * 60 * 60; // 6 months (in seconds)
pub const MAX_FINDERS_FEE_BPS: u64 = 1000; // 10%
pub const BID_REMOVAL_REWARD_BPS: u64 = 500; // 5%

pub fn execute_marketplace_setup(
    router: &mut StargazeApp,
    marketplace_admin: Addr,
    trading_fee: u64,
) -> Addr {
    let marketplace_id = router.store_code(contract_marketplace());
    let msg = sg_marketplace::msg::InstantiateMsg {
        operators: vec!["operator1".to_string()],
        trading_fee_bps: trading_fee,
        ask_expiry: ExpiryRange::new(MIN_EXPIRY, MAX_EXPIRY),
        bid_expiry: ExpiryRange::new(MIN_EXPIRY, MAX_EXPIRY),
        sale_hook: None,
        max_finders_fee_bps: MAX_FINDERS_FEE_BPS,
        min_price: Uint128::from(5u128),
        stale_bid_duration: Duration::Time(100),
        bid_removal_reward_bps: BID_REMOVAL_REWARD_BPS,
        listing_fee: Uint128::from(LISTING_FEE),
    };
    router
        .instantiate_contract(
            marketplace_id,
            marketplace_admin,
            &msg,
            &[],
            "Marketplace",
            None,
        )
        .unwrap()
}

pub fn setup_marketplace(
    router: &mut StargazeApp,
    marketplace_admin: Addr,
) -> Result<Addr, ContractError> {
    let marketplace = execute_marketplace_setup(router, marketplace_admin, TRADING_FEE_BPS);
    Ok(marketplace)
}
