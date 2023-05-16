#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use crate::error::ContractError;
use crate::helpers::settle_auction;
use crate::msg::SudoMsg;
use crate::state::{auctions, ExpiringAuctionKey, CONFIG, EXPIRING_AUCTIONS};
use cosmwasm_std::{Addr, Decimal, DepsMut, Env, Event, Order, StdResult, Uint128};
use cw_storage_plus::Bound;
use sg_std::Response;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(deps: DepsMut, env: Env, msg: SudoMsg) -> Result<Response, ContractError> {
    match msg {
        SudoMsg::BeginBlock {} => sudo_begin_block(deps, env),
        SudoMsg::EndBlock {} => sudo_end_block(deps, env),
        SudoMsg::UpdateParams {
            marketplace,
            min_reserve_price,
            min_duration,
            min_bid_increment_bps,
            extend_duration,
            create_auction_fee,
            max_auctions_to_settle_per_block,
        } => sudo_update_params(
            deps,
            env,
            marketplace,
            min_reserve_price,
            min_duration,
            min_bid_increment_bps,
            extend_duration,
            create_auction_fee,
            max_auctions_to_settle_per_block,
        ),
    }
}

pub fn sudo_begin_block(_deps: DepsMut, _env: Env) -> Result<Response, ContractError> {
    let event = Event::new("sudo-begin-block");
    Ok(Response::new().add_event(event))
}

pub fn sudo_end_block(mut deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let limit = config.max_auctions_to_settle_per_block as usize;
    let order = Order::Ascending;
    let max = Some(Bound::exclusive((
        env.block.time.seconds() + 1,
        Addr::unchecked(""),
        "".to_string(),
    )));

    let auction_keys: Vec<ExpiringAuctionKey> = EXPIRING_AUCTIONS
        .range(deps.storage, None, max, order)
        .take(limit)
        .map(|item| item.map(|(k, _)| k))
        .collect::<StdResult<_>>()?;

    let event = Event::new("sudo-end-block");
    let mut response = Response::new().add_event(event);

    for (_end_time, collection, token_id) in auction_keys {
        let auction = auctions().load(deps.storage, (collection, token_id))?;
        response = settle_auction(deps.branch(), env.block.time, &config, auction, response)?;
    }

    Ok(response)
}

#[allow(clippy::too_many_arguments)]
pub fn sudo_update_params(
    deps: DepsMut,
    _env: Env,
    marketplace: Option<String>,
    min_reserve_price: Option<Uint128>,
    min_duration: Option<u64>,
    min_bid_increment_bps: Option<u64>,
    extend_duration: Option<u64>,
    create_auction_fee: Option<Uint128>,
    max_auctions_to_settle_per_block: Option<u64>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    let mut event = Event::new("sudo-update-params");

    if let Some(_marketplace) = marketplace {
        config.marketplace = deps.api.addr_validate(&_marketplace)?;
        event = event.add_attribute("marketplace", &config.marketplace);
    }
    if let Some(_min_reserve_price) = min_reserve_price {
        config.min_reserve_price = _min_reserve_price;
        event = event.add_attribute("min_reserve_price", config.min_reserve_price.to_string());
    }
    if let Some(_min_duration) = min_duration {
        config.min_duration = _min_duration;
        event = event.add_attribute("min_duration", config.min_duration.to_string());
    }
    if let Some(_min_bid_increment_bps) = min_bid_increment_bps {
        config.min_bid_increment_pct = Decimal::percent(_min_bid_increment_bps);
        event = event.add_attribute(
            "min_bid_increment_pct",
            config.min_bid_increment_pct.to_string(),
        );
    }
    if let Some(_extend_duration) = extend_duration {
        config.extend_duration = _extend_duration;
        event = event.add_attribute("extend_duration", config.extend_duration.to_string());
    }
    if let Some(_create_auction_fee) = create_auction_fee {
        config.create_auction_fee = _create_auction_fee;
        event = event.add_attribute("create_auction_fee", config.create_auction_fee.to_string());
    }
    if let Some(_max_auctions_to_settle_per_block) = max_auctions_to_settle_per_block {
        config.max_auctions_to_settle_per_block = _max_auctions_to_settle_per_block;
        event = event.add_attribute(
            "max_auctions_to_settle_per_block",
            config.max_auctions_to_settle_per_block.to_string(),
        );
    }

    config.save(deps.storage)?;

    Ok(Response::new().add_event(event))
}
