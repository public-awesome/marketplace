use cosmwasm_std::{coin, Addr, Uint128};
use cw_multi_test::Executor;
use sg_multi_test::StargazeApp;
use sg_std::NATIVE_DENOM;

use crate::error::ContractError;
use crate::msg::InstantiateMsg;
use crate::state::{Config, PriceRange};
use crate::testing::setup::setup_contracts::contract_marketplace;

pub const ATOM_DENOM: &str = "uatom";
pub const JUNO_DENOM: &str = "ujuno";

pub fn setup_marketplace(
    router: &mut StargazeApp,
    fair_burn: Addr,
    royalty_registry: Addr,
    marketplace_admin: Addr,
) -> Result<Addr, ContractError> {
    let marketplace_id = router.store_code(contract_marketplace());
    let msg = InstantiateMsg {
        config: Config {
            fair_burn: fair_burn.to_string(),
            royalty_registry: royalty_registry.to_string(),
            listing_fee: coin(1_000_000, NATIVE_DENOM),
            min_removal_reward: coin(4_000_000, NATIVE_DENOM),
            trading_fee_bps: 200,
            max_royalty_fee_bps: 1000,
            max_finders_fee_bps: 400,
            min_expiration_seconds: 600,
            order_removal_lookahead_secs: 10,
            max_asks_removed_per_block: 50,
            max_offers_removed_per_block: 20,
            max_collection_offers_removed_per_block: 30,
        },
        price_ranges: vec![
            (
                NATIVE_DENOM.to_string(),
                PriceRange {
                    min: Uint128::from(100_000u128),
                    max: Uint128::from(100_000_000_000_000u128),
                },
            ),
            (
                ATOM_DENOM.to_string(),
                PriceRange {
                    min: Uint128::from(100_000u128),
                    max: Uint128::from(100_000_000_000_000u128),
                },
            ),
        ],
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
