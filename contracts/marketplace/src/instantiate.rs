use cosmwasm_std::{ensure, Decimal, DepsMut, Env, MessageInfo, Uint128};
use cw2::set_contract_version;
use sg_marketplace_common::address::map_validate;
use sg_std::Response;

use crate::{
    constants::{CONTRACT_NAME, CONTRACT_VERSION},
    error::ContractError,
    msg::InstantiateMsg,
    state::{PriceRange, SudoParams, PRICE_RANGES, SALE_HOOKS},
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

    let sudo_params = SudoParams {
        fair_burn: deps.api.addr_validate(&msg.fair_burn)?,
        listing_fee: msg.listing_fee,
        ask_expiry: msg.ask_expiry,
        offer_expiry: msg.offer_expiry,
        operators: map_validate(deps.api, &msg.operators)?,
        max_asks_removed_per_block: msg.max_asks_removed_per_block,
        max_offers_removed_per_block: msg.max_offers_removed_per_block,
        max_collection_offers_removed_per_block: msg.max_collection_offers_removed_per_block,
        trading_fee_percent: Decimal::percent(msg.trading_fee_bps) / Uint128::from(100u64),
        max_finders_fee_percent: Decimal::percent(msg.max_finders_fee_bps) / Uint128::from(100u64),
        removal_reward_percent: Decimal::percent(msg.removal_reward_bps) / Uint128::from(100u64),
    };
    sudo_params.validate()?;
    sudo_params.save(deps.storage)?;

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

    if let Some(hook) = msg.sale_hook {
        SALE_HOOKS.add_hook(deps.storage, deps.api.addr_validate(&hook)?)?;
    }

    Ok(Response::new())
}
