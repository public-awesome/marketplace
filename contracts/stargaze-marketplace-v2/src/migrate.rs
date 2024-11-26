use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, DepsMut, Env, Response};
use cw2::set_contract_version;
use cw_storage_plus::Item;

use crate::{
    constants::{CONTRACT_NAME, CONTRACT_VERSION},
    state::{Config, Denom, CONFIG},
    ContractError,
};

#[cw_serde]
pub struct V0_7Config {
    /// The address of the address that will receive the protocol fees
    pub fee_manager: Addr,
    /// The address of the royalty registry contract
    pub royalty_registry: Addr,
    /// Protocol fee
    pub protocol_fee_bps: u64,
    /// Max value for the royalty fee
    pub max_royalty_fee_bps: u64,
    /// The reward paid out to the market maker. Reward is a percentage of the protocol fee
    pub maker_reward_bps: u64,
    /// The reward paid out to the market taker. Reward is a percentage of the protocol fee
    pub taker_reward_bps: u64,
    /// The default denom for all collections on the marketplace
    pub default_denom: Denom,
}

pub const V0_7CONFIG: Item<V0_7Config> = Item::new("C");

#[cw_serde]
pub struct MigrateMsg {
    pub max_asks_removed_per_block: u32,
    pub max_bids_removed_per_block: u32,
    pub max_collection_bids_removed_per_block: u32,
}

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    let response = Response::new();

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let v0_7config = V0_7CONFIG.load(deps.storage)?;

    let v0_8config = Config {
        fee_manager: v0_7config.fee_manager,
        royalty_registry: v0_7config.royalty_registry,
        protocol_fee_bps: v0_7config.protocol_fee_bps,
        max_royalty_fee_bps: v0_7config.max_royalty_fee_bps,
        maker_reward_bps: v0_7config.maker_reward_bps,
        taker_reward_bps: v0_7config.taker_reward_bps,
        default_denom: v0_7config.default_denom,
        max_asks_removed_per_block: msg.max_asks_removed_per_block,
        max_bids_removed_per_block: msg.max_bids_removed_per_block,
        max_collection_bids_removed_per_block: msg.max_collection_bids_removed_per_block,
    };

    CONFIG.save(deps.storage, &v0_8config)?;

    Ok(response)
}
