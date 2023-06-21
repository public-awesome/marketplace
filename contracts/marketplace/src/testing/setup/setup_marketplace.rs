use cosmwasm_std::{coin, Addr, Uint128};
use cw_multi_test::Executor;
use sg_multi_test::StargazeApp;
use sg_std::NATIVE_DENOM;
use stargaze_fair_burn::msg::InstantiateMsg as FairBurnInstantiateMsg;

use crate::error::ContractError;
use crate::helpers::ExpiryRange;
use crate::msg::InstantiateMsg;
use crate::state::PriceRange;
use crate::testing::setup::setup_contracts::contract_marketplace;

use super::setup_contracts::contract_fair_burn;

pub const FAIR_BURN_BPS: u64 = 5000;
pub const LISTING_FEE: u128 = 0;
// Governance parameters
pub const TRADING_FEE_BPS: u64 = 200; // 2%
pub const MIN_EXPIRY: u64 = 24 * 60 * 60; // 24 hours (in seconds)
pub const MAX_EXPIRY: u64 = 180 * 24 * 60 * 60; // 6 months (in seconds)
pub const MAX_FINDERS_FEE_BPS: u64 = 1000; // 10%
pub const REMOVAL_REWARD_BPS: u64 = 500; // 5%
pub const MAX_ENTRY_REMOVAL_PER_BLOCK: u32 = 200;
pub const MAX_FIXED_PRICE_ASK_AMOUNT: u128 = 100_000_000_000_000u128;

pub fn setup_fair_burn(router: &mut StargazeApp, creator: &Addr) -> Result<Addr, ContractError> {
    let fair_burn_id = router.store_code(contract_fair_burn());
    let fair_burn = router
        .instantiate_contract(
            fair_burn_id,
            creator.clone(),
            &FairBurnInstantiateMsg {
                fee_bps: FAIR_BURN_BPS,
            },
            &[],
            "FairBurn",
            None,
        )
        .unwrap();
    Ok(fair_burn)
}

pub fn setup_marketplace(
    router: &mut StargazeApp,
    fair_burn: Addr,
    marketplace_admin: Addr,
) -> Result<Addr, ContractError> {
    let marketplace_id = router.store_code(contract_marketplace());
    let msg = InstantiateMsg {
        fair_burn: fair_burn.to_string(),
        listing_fee: coin(LISTING_FEE, NATIVE_DENOM),
        ask_expiry: ExpiryRange::new(MIN_EXPIRY, MAX_EXPIRY),
        offer_expiry: ExpiryRange::new(MIN_EXPIRY, MAX_EXPIRY),
        operators: vec!["operator0".to_string()],
        max_asks_removed_per_block: MAX_ENTRY_REMOVAL_PER_BLOCK,
        max_offers_removed_per_block: MAX_ENTRY_REMOVAL_PER_BLOCK,
        max_collection_offers_removed_per_block: MAX_ENTRY_REMOVAL_PER_BLOCK,
        trading_fee_bps: TRADING_FEE_BPS,
        max_finders_fee_bps: MAX_FINDERS_FEE_BPS,
        removal_reward_bps: REMOVAL_REWARD_BPS,
        price_ranges: vec![(
            NATIVE_DENOM.to_string(),
            PriceRange {
                min: Uint128::from(5u128),
                max: Uint128::from(MAX_FIXED_PRICE_ASK_AMOUNT),
            },
        )],
        sale_hook: None,
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
