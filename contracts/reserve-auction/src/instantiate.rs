#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use crate::msg::InstantiateMsg;
use crate::state::Config;
use crate::{error::ContractError, state::MIN_RESERVE_PRICES};
use cosmwasm_std::{Decimal, DepsMut, Env, Event, MessageInfo};
use cw2::set_contract_version;
use sg_std::Response;

// version info for migration info
pub const CONTRACT_NAME: &str = "crates.io:sg-reserve-auction";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let config = Config {
        fair_burn: deps.api.addr_validate(&msg.fair_burn)?,
        marketplace: deps.api.addr_validate(&msg.marketplace)?,
        min_bid_increment_pct: Decimal::percent(msg.min_bid_increment_bps),
        min_duration: msg.min_duration,
        max_duration: msg.max_duration,
        extend_duration: msg.extend_duration,
        create_auction_fee: msg.create_auction_fee,
        max_auctions_to_settle_per_block: msg.max_auctions_to_settle_per_block,
    };

    config.save(deps.storage)?;

    let mut response = Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("contract_name", CONTRACT_NAME)
        .add_attribute("contract_version", CONTRACT_VERSION)
        .add_attribute("marketplace", &config.marketplace)
        .add_attribute("min_duration", &config.min_duration.to_string())
        .add_attribute(
            "min_bid_increment_pct",
            &config.min_bid_increment_pct.to_string(),
        )
        .add_attribute("extend_duration", &config.extend_duration.to_string())
        .add_attribute("create_auction_fee", &config.create_auction_fee.to_string())
        .add_attribute(
            "max_auctions_to_settle_per_block",
            &config.max_auctions_to_settle_per_block.to_string(),
        );

    for min_reserve_price in msg.min_reserve_prices {
        if MIN_RESERVE_PRICES.has(deps.storage, min_reserve_price.denom.clone()) {
            return Err(ContractError::InvalidInput(
                "found duplicate denom".to_string(),
            ));
        }
        MIN_RESERVE_PRICES.save(
            deps.storage,
            min_reserve_price.denom.clone(),
            &min_reserve_price.amount,
        )?;
        response = response.add_event(
            Event::new("set-min-reserve-price")
                .add_attribute("denom", min_reserve_price.denom)
                .add_attribute("amount", min_reserve_price.amount),
        );
    }

    Ok(response)
}
