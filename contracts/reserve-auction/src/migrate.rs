use crate::instantiate::{CONTRACT_NAME, CONTRACT_VERSION};
use crate::state::{Config, CONFIG};
use crate::ContractError;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Decimal, DepsMut, Env};
use cw_storage_plus::Item;
use sg_std::Response;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrationMsg) -> Result<Response, ContractError> {
    let api = deps.api;

    pub const OLD_CONFIG: Item<OldConfig> = Item::new("config");

    let old_config = OLD_CONFIG.load(deps.storage)?;
    OLD_CONFIG.remove(deps.storage);

    let new_config = Config {
        fair_burn: api.addr_validate(&msg.fair_burn)?,
        marketplace: old_config.marketplace,
        min_bid_increment_pct: old_config.min_bid_increment_pct,
        min_duration: old_config.min_duration,
        max_duration: old_config.max_duration,
        extend_duration: old_config.extend_duration,
        create_auction_fee: old_config.create_auction_fee,
        max_auctions_to_settle_per_block: old_config.max_auctions_to_settle_per_block,
    };
    CONFIG.save(deps.storage, &new_config)?;

    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new())
}

#[cw_serde]
pub struct MigrationMsg {
    pub fair_burn: String,
}

#[cw_serde]
pub struct OldConfig {
    pub marketplace: Addr,
    pub min_bid_increment_pct: Decimal,
    pub min_duration: u64,
    pub max_duration: u64,
    pub extend_duration: u64,
    pub create_auction_fee: Coin,
    pub max_auctions_to_settle_per_block: u64,
}
