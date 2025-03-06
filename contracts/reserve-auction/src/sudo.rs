use crate::error::ContractError;
use crate::helpers::settle_auction;
use crate::msg::SudoMsg;
use crate::state::{auctions, Auction, HaltWindow, CONFIG, HALT_MANAGER, MIN_RESERVE_PRICES};

use cosmwasm_std::{Addr, Coin, Decimal, DepsMut, Env, Event, Order, StdResult};
use cw_storage_plus::Bound;
use sg_std::Response;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(deps: DepsMut, env: Env, msg: SudoMsg) -> Result<Response, ContractError> {
    match msg {
        SudoMsg::BeginBlock {} => sudo_begin_block(deps, env),
        SudoMsg::EndBlock {} => sudo_end_block(deps, env),
        SudoMsg::UpdateParams {
            fair_burn,
            trading_fee_percent,
            min_bid_increment_percent,
            min_duration,
            extend_duration,
            create_auction_fee,
            max_auctions_to_settle_per_block,
            halt_duration_threshold,
            halt_buffer_duration,
            halt_postpone_duration,
            royalty_registry,
            max_royalty_fee_bps,
        } => sudo_update_params(
            deps,
            env,
            fair_burn,
            trading_fee_percent,
            min_bid_increment_percent,
            min_duration,
            extend_duration,
            create_auction_fee,
            max_auctions_to_settle_per_block,
            halt_duration_threshold,
            halt_buffer_duration,
            halt_postpone_duration,
            royalty_registry,
            max_royalty_fee_bps,
        ),
        SudoMsg::SetMinReservePrices { min_reserve_prices } => {
            sudo_set_min_reserve_prices(deps, min_reserve_prices)
        }
        SudoMsg::UnsetMinReservePrices { denoms } => sudo_unset_min_reserve_prices(deps, denoms),
    }
}

pub fn sudo_begin_block(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let mut response = Response::new();

    let config = CONFIG.load(deps.storage)?;
    let mut halt_manager = HALT_MANAGER.load(deps.storage)?;

    let current_block_time = env.block.time.seconds();
    let seconds_since_last_block = current_block_time - halt_manager.prev_block_time;

    if halt_manager.prev_block_time > 0
        && seconds_since_last_block >= config.halt_duration_threshold
    {
        let halt_window = HaltWindow {
            start_time: halt_manager.prev_block_time,
            end_time: current_block_time + config.halt_buffer_duration,
        };
        response = response
            .add_event(Event::new("halt-detected"))
            .add_attribute("start_time", halt_window.start_time.to_string())
            .add_attribute("end_time", halt_window.end_time.to_string());
        halt_manager.halt_windows.push(halt_window);
    }

    halt_manager.prev_block_time = current_block_time;
    HALT_MANAGER.save(deps.storage, &halt_manager)?;

    Ok(response)
}

pub fn sudo_end_block(mut deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut halt_manager = HALT_MANAGER.load(deps.storage)?;

    let mut response = Response::new();

    // Settle auctions normally
    let limit = config.max_auctions_to_settle_per_block as usize;
    let order = Order::Ascending;
    let max = Some(Bound::exclusive((
        env.block.time.seconds() + 1,
        (Addr::unchecked(""), "".to_string()),
    )));

    let auctions = auctions()
        .idx
        .end_time
        .range(deps.storage, None, max, order)
        .take(limit)
        .map(|item| item.map(|(_, v)| v))
        .collect::<StdResult<Vec<Auction>>>()?;

    let earliest_auction_end_time = auctions.first().map(|a| a.end_time.unwrap());

    response =
        response.add_event(Event::new("sudo-end-block").add_attribute("action", "settle-auctions"));

    for auction in auctions {
        response = settle_auction(
            deps.branch(),
            env.clone(),
            auction,
            &config,
            &halt_manager,
            response,
        )?;
    }

    // Try and clear a halt info if necessary
    let halt_info = halt_manager.find_stale_halt_info(earliest_auction_end_time);
    if halt_info.is_some() {
        HALT_MANAGER.save(deps.storage, &halt_manager)?;
    }

    Ok(response)
}

#[allow(clippy::too_many_arguments)]
pub fn sudo_update_params(
    deps: DepsMut,
    _env: Env,
    fair_burn: Option<String>,
    trading_fee_percent: Option<Decimal>,
    min_bid_increment_percent: Option<Decimal>,
    min_duration: Option<u64>,
    extend_duration: Option<u64>,
    create_auction_fee: Option<Coin>,
    max_auctions_to_settle_per_block: Option<u64>,
    halt_duration_threshold: Option<u64>,
    halt_buffer_duration: Option<u64>,
    halt_postpone_duration: Option<u64>,
    royalty_registry: Option<String>,
    max_royalty_fee_bps: Option<u64>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    let mut event = Event::new("sudo-update-params");

    if let Some(fair_burn) = fair_burn {
        config.fair_burn = deps.api.addr_validate(&fair_burn)?;
        event = event.add_attribute("fair_burn", &config.fair_burn);
    }
    if let Some(trading_fee_percent) = trading_fee_percent {
        config.trading_fee_percent = trading_fee_percent;
        event = event.add_attribute(
            "trading_fee_percent",
            config.trading_fee_percent.to_string(),
        );
    }
    if let Some(min_bid_increment_percent) = min_bid_increment_percent {
        config.min_bid_increment_percent = min_bid_increment_percent;
        event = event.add_attribute(
            "min_bid_increment_percent",
            config.min_bid_increment_percent.to_string(),
        );
    }
    if let Some(min_duration) = min_duration {
        config.min_duration = min_duration;
        event = event.add_attribute("min_duration", config.min_duration.to_string());
    }
    if let Some(extend_duration) = extend_duration {
        config.extend_duration = extend_duration;
        event = event.add_attribute("extend_duration", config.extend_duration.to_string());
    }
    if let Some(create_auction_fee) = create_auction_fee {
        config.create_auction_fee = create_auction_fee;
        event = event.add_attribute("create_auction_fee", config.create_auction_fee.to_string());
    }
    if let Some(max_auctions_to_settle_per_block) = max_auctions_to_settle_per_block {
        config.max_auctions_to_settle_per_block = max_auctions_to_settle_per_block;
        event = event.add_attribute(
            "max_auctions_to_settle_per_block",
            config.max_auctions_to_settle_per_block.to_string(),
        );
    }
    if let Some(halt_duration_threshold) = halt_duration_threshold {
        config.halt_duration_threshold = halt_duration_threshold;
        event = event.add_attribute(
            "halt_duration_threshold",
            config.halt_duration_threshold.to_string(),
        );
    }
    if let Some(halt_buffer_duration) = halt_buffer_duration {
        config.halt_buffer_duration = halt_buffer_duration;
        event = event.add_attribute(
            "halt_buffer_duration",
            config.halt_buffer_duration.to_string(),
        );
    }
    if let Some(halt_postpone_duration) = halt_postpone_duration {
        config.halt_postpone_duration = halt_postpone_duration;
        event = event.add_attribute(
            "halt_postpone_duration",
            config.halt_postpone_duration.to_string(),
        );
    }
    if let Some(royalty_registry) = royalty_registry {
        config.royalty_registry = deps.api.addr_validate(&royalty_registry)?;
        event = event.add_attribute("royalty_registry", &config.royalty_registry);
    }
    if let Some(max_royalty_fee_bps) = max_royalty_fee_bps {
        config.max_royalty_fee_bps = max_royalty_fee_bps;
        event = event.add_attribute(
            "max_royalty_fee_bps",
            config.max_royalty_fee_bps.to_string(),
        );
    }

    config.save(deps.storage)?;

    Ok(Response::new().add_event(event))
}

pub fn sudo_set_min_reserve_prices(
    deps: DepsMut,
    min_reserve_prices: Vec<Coin>,
) -> Result<Response, ContractError> {
    let mut response = Response::new();

    for min_reserve_price in min_reserve_prices {
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

pub fn sudo_unset_min_reserve_prices(
    deps: DepsMut,
    denoms: Vec<String>,
) -> Result<Response, ContractError> {
    let mut response = Response::new();

    for denom in denoms {
        if !MIN_RESERVE_PRICES.has(deps.storage, denom.clone()) {
            return Err(ContractError::InvalidInput("denom not found".to_string()));
        }
        MIN_RESERVE_PRICES.remove(deps.storage, denom.clone());
        response =
            response.add_event(Event::new("unset-min-reserve-price").add_attribute("denom", denom));
    }
    Ok(response)
}
