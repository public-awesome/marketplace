use crate::error::ContractError;
use crate::ExpiryRange;
use cosmwasm_std::{Addr, Uint128};
use cw_multi_test::Executor;
use cw_utils::Duration;
use sg_multi_test::StargazeApp;

use crate::testing::setup::setup_contracts::contract_marketplace;

pub const LISTING_FEE: u128 = 0;
// Governance parameters
pub const TRADING_FEE_BPS: u64 = 200; // 2%
pub const MIN_EXPIRY: u64 = 24 * 60 * 60; // 24 hours (in seconds)
pub const MAX_EXPIRY: u64 = 180 * 24 * 60 * 60; // 6 months (in seconds)
pub const MAX_FINDERS_FEE_BPS: u64 = 1000; // 10%
pub const BID_REMOVAL_REWARD_BPS: u64 = 500; // 5%

pub fn setup_marketplace(
    router: &mut StargazeApp,
    marketplace_admin: Addr,
) -> Result<Addr, ContractError> {
    let marketplace_id = router.store_code(contract_marketplace());
    let msg = crate::msg::InstantiateMsg {
        operators: vec!["operator1".to_string()],
        trading_fee_bps: TRADING_FEE_BPS,
        ask_expiry: ExpiryRange::new(MIN_EXPIRY, MAX_EXPIRY),
        bid_expiry: ExpiryRange::new(MIN_EXPIRY, MAX_EXPIRY),
        sale_hook: None,
        max_finders_fee_bps: MAX_FINDERS_FEE_BPS,
        min_price: Uint128::from(5u128),
        stale_bid_duration: Duration::Time(100),
        bid_removal_reward_bps: BID_REMOVAL_REWARD_BPS,
        listing_fee: Uint128::from(LISTING_FEE),
    };
    let marketplace = router
        .instantiate_contract(
            marketplace_id,
            marketplace_admin,
            &msg,
            &[],
            "Marketplace",
            None,
        )
        .unwrap();
    Ok(marketplace)
}

pub fn setup_marketplace_and_collections_with_params(
    router: &mut StargazeApp,
    marketplace_admin: Addr,
    instantiate_msg: crate::msg::InstantiateMsg,
) -> Result<Addr, ContractError> {
    // Instantiate marketplace contract
    let marketplace_id = router.store_code(contract_marketplace());
    let marketplace = router
        .instantiate_contract(
            marketplace_id,
            marketplace_admin,
            &instantiate_msg,
            &[],
            "Marketplace",
            None,
        )
        .unwrap();
    Ok(marketplace)
}
