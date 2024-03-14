use cosmwasm_std::{ensure, DepsMut, Env, MessageInfo};
use cw2::set_contract_version;
use sg_std::Response;

use crate::{
    constants::{CONTRACT_NAME, CONTRACT_VERSION},
    error::ContractError,
    msg::InstantiateMsg,
    state::{Config, PriceRange, PRICE_RANGES},
};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        fair_burn: deps.api.addr_validate(&msg.config.fair_burn)?,
        royalty_registry: deps.api.addr_validate(&msg.config.royalty_registry)?,
        listing_fee: msg.config.listing_fee,
        min_removal_reward: msg.config.min_removal_reward,
        trading_fee_bps: msg.config.trading_fee_bps,
        max_royalty_fee_bps: msg.config.max_royalty_fee_bps,
        max_finders_fee_bps: msg.config.max_finders_fee_bps,
        min_expiration_seconds: msg.config.min_expiration_seconds,
        order_removal_lookahead_secs: msg.config.order_removal_lookahead_secs,
        max_asks_removed_per_block: msg.config.max_asks_removed_per_block,
        max_offers_removed_per_block: msg.config.max_offers_removed_per_block,
        max_collection_offers_removed_per_block: msg.config.max_collection_offers_removed_per_block,
    };
    config.validate()?;
    config.save(deps.storage)?;

    for (denom, price_range) in msg.price_ranges {
        PRICE_RANGES.update(
            deps.storage,
            denom,
            |existing_price_range| -> Result<PriceRange, ContractError> {
                ensure!(
                    existing_price_range.is_none(),
                    ContractError::InvalidInput("duplicate denom in price_ranges".to_string())
                );
                Ok(price_range)
            },
        )?;
    }

    Ok(Response::new())
}
