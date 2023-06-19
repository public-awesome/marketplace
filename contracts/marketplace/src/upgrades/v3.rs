use cosmwasm_schema::cw_serde;
use cosmwasm_std::{coin, Decimal, DepsMut, Env, Uint128};
use sg_std::{Response, NATIVE_DENOM};

use crate::{
    state::{SudoParams, SUDO_PARAMS},
    state_deprecated::SUDO_PARAMS as SUDO_PARAMS_DEP,
    ContractError,
};

#[cw_serde]
pub struct MigrateMsgV3 {
    pub fair_burn: String,
    pub removal_reward_bps: u64,
    pub max_asks_removed_per_block: u32,
    pub max_offers_removed_per_block: u32,
    pub max_collection_offers_removed_per_block: u32,
}

pub fn migrate(
    deps: DepsMut,
    _env: Env,
    msg: MigrateMsgV3,
    response: Response,
) -> Result<Response, ContractError> {
    // Load and clear previous sudo params
    let prev_sudo_params = SUDO_PARAMS_DEP.load(deps.storage)?;
    SUDO_PARAMS_DEP.remove(deps.storage);

    // Save new sudo params
    SUDO_PARAMS.save(
        deps.storage,
        &SudoParams {
            fair_burn: deps.api.addr_validate(&msg.fair_burn)?,
            listing_fee: coin(prev_sudo_params.listing_fee.u128(), NATIVE_DENOM),
            ask_expiry: prev_sudo_params.ask_expiry,
            offer_expiry: prev_sudo_params.bid_expiry,
            operators: prev_sudo_params.operators,
            max_asks_removed_per_block: msg.max_asks_removed_per_block,
            max_offers_removed_per_block: msg.max_offers_removed_per_block,
            max_collection_offers_removed_per_block: msg.max_collection_offers_removed_per_block,
            trading_fee_percent: prev_sudo_params.trading_fee_percent / Uint128::from(100u64),
            max_finders_fee_percent: prev_sudo_params.max_finders_fee_percent
                / Uint128::from(100u64),
            removal_reward_percent: Decimal::percent(msg.removal_reward_bps)
                / Uint128::from(100u64),
        },
    )?;

    Ok(response)
}
