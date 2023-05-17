#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use crate::error::ContractError;
use crate::msg::InstantiateMsg;
use crate::state::{Config, CONFIG};
use cosmwasm_std::{Decimal, DepsMut, Env, MessageInfo};
use cw2::set_contract_version;
use sg_std::Response;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:reserve-auction";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let config = Config {
        marketplace: deps.api.addr_validate(&msg.marketplace)?,
        min_reserve_price: msg.min_reserve_price,
        min_bid_increment_pct: Decimal::percent(msg.min_bid_increment_bps),
        min_duration: msg.min_duration,
        extend_duration: msg.extend_duration,
        create_auction_fee: msg.create_auction_fee,
        max_auctions_to_settle_per_block: msg.max_auctions_to_settle_per_block,
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new())
}
