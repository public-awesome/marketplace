use crate::instantiate::{CONTRACT_NAME, CONTRACT_VERSION};
use crate::state::{Config, CONFIG};
use crate::ContractError;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Decimal, DepsMut, Env};
use cw_storage_plus::Item;
use sg_marketplace_common::coin::bps_to_decimal;
use sg_std::Response;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrationMsg) -> Result<Response, ContractError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let old_config: OldConfig = OLD_CONFIG.load(deps.storage)?;
    OLD_CONFIG.remove(deps.storage);

    CONFIG.save(
        deps.storage,
        &Config {
            fair_burn: old_config.fair_burn,
            trading_fee_pct: bps_to_decimal(msg.trading_fee_bps),
            min_bid_increment_pct: old_config.min_bid_increment_pct,
            min_duration: old_config.min_duration,
            max_duration: old_config.max_duration,
            extend_duration: old_config.extend_duration,
            create_auction_fee: old_config.create_auction_fee,
            max_auctions_to_settle_per_block: old_config.max_auctions_to_settle_per_block,
            halt_duration_threshold: old_config.halt_duration_threshold,
            halt_buffer_duration: old_config.halt_buffer_duration,
            halt_postpone_duration: old_config.halt_postpone_duration,
        },
    )?;

    Ok(Response::new())
}

#[cw_serde]
pub struct MigrationMsg {
    trading_fee_bps: u64,
}

#[cw_serde]
pub struct OldConfig {
    pub fair_burn: Addr,
    pub marketplace: Addr,
    pub min_bid_increment_pct: Decimal,
    pub min_duration: u64,
    pub max_duration: u64,
    pub extend_duration: u64,
    pub create_auction_fee: Coin,
    pub max_auctions_to_settle_per_block: u64,
    pub halt_duration_threshold: u64, // in seconds
    pub halt_buffer_duration: u64,    // in seconds
    pub halt_postpone_duration: u64,  // in seconds
}

pub const OLD_CONFIG: Item<OldConfig> = Item::new("c");
