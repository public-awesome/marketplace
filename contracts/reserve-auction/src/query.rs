use crate::msg::{
    AuctionKeyOffset, AuctionResponse, AuctionsResponse, CoinsResponse, ConfigResponse,
    HaltManagerResponse, MinReservePriceOffset, QueryMsg,
};
use crate::state::{auctions, Auction, HALT_MANAGER};
use crate::state::{CONFIG, MIN_RESERVE_PRICES};

use cosmwasm_std::{coin, to_binary, Addr, Binary, Coin, Deps, Env, StdResult};
use cw_storage_plus::Bound;
use sg_marketplace_common::query::{unpack_query_options, QueryOptions};

// Query limits
const DEFAULT_QUERY_LIMIT: u32 = 10;
const MAX_QUERY_LIMIT: u32 = 100;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::HaltManager {} => to_binary(&query_halt_manager(deps)?),
        QueryMsg::MinReservePrices { query_options } => to_binary(&query_min_reserve_prices(
            deps,
            query_options.unwrap_or(QueryOptions::default()),
        )?),
        QueryMsg::Auction {
            collection,
            token_id,
        } => to_binary(&query_auction(deps, collection, token_id)?),
        QueryMsg::AuctionsBySeller {
            seller,
            query_options,
        } => to_binary(&query_auctions_by_seller(
            deps,
            seller,
            query_options.unwrap_or(QueryOptions::default()),
        )?),
        QueryMsg::AuctionsByEndTime {
            end_time,
            query_options,
        } => to_binary(&query_auctions_by_end_time(
            deps,
            end_time,
            query_options.unwrap_or(QueryOptions::default()),
        )?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse { config })
}

pub fn query_halt_manager(deps: Deps) -> StdResult<HaltManagerResponse> {
    let halt_manager = HALT_MANAGER.load(deps.storage)?;
    Ok(HaltManagerResponse { halt_manager })
}

pub fn query_min_reserve_prices(
    deps: Deps,
    query_options: QueryOptions<MinReservePriceOffset>,
) -> StdResult<CoinsResponse> {
    let (limit, order, min, max) = unpack_query_options(
        query_options,
        Box::new(|sa| Bound::exclusive(sa.denom)),
        DEFAULT_QUERY_LIMIT,
        MAX_QUERY_LIMIT,
    );

    let coins: Vec<Coin> = MIN_RESERVE_PRICES
        .range(deps.storage, min, max, order)
        .take(limit)
        .map(|item| item.map(|(denom, amount)| coin(amount.u128(), denom)))
        .collect::<StdResult<_>>()?;

    Ok(CoinsResponse { coins })
}

pub fn query_auction(
    deps: Deps,
    collection: String,
    token_id: String,
) -> StdResult<AuctionResponse> {
    let collection = deps.api.addr_validate(&collection)?;
    let auction = auctions().load(deps.storage, (collection, token_id))?;
    Ok(AuctionResponse { auction })
}

pub fn query_auctions_by_seller(
    deps: Deps,
    seller: String,
    query_options: QueryOptions<AuctionKeyOffset>,
) -> StdResult<AuctionsResponse> {
    deps.api.addr_validate(&seller)?;

    let (limit, order, min, max) = unpack_query_options(
        query_options,
        Box::new(|sa| Bound::exclusive((Addr::unchecked(sa.collection), sa.token_id))),
        DEFAULT_QUERY_LIMIT,
        MAX_QUERY_LIMIT,
    );

    let auctions: Vec<Auction> = auctions()
        .idx
        .seller
        .prefix(seller)
        .range(deps.storage, min, max, order)
        .take(limit)
        .map(|item| item.map(|(_, v)| v))
        .collect::<StdResult<_>>()?;

    Ok(AuctionsResponse { auctions })
}

pub fn query_auctions_by_end_time(
    deps: Deps,
    end_time: u64,
    query_options: QueryOptions<AuctionKeyOffset>,
) -> StdResult<AuctionsResponse> {
    let query_options = QueryOptions {
        descending: query_options.descending,
        limit: query_options.limit,
        start_after: Some(
            query_options
                .start_after
                .map_or((end_time, (Addr::unchecked(""), "".to_string())), |sa| {
                    (end_time, (Addr::unchecked(sa.collection), sa.token_id))
                }),
        ),
    };

    let (limit, order, min, max) = unpack_query_options(
        query_options,
        Box::new(Bound::exclusive),
        DEFAULT_QUERY_LIMIT,
        MAX_QUERY_LIMIT,
    );

    let max = max.unwrap_or(Bound::exclusive((
        u64::MAX,
        (Addr::unchecked(""), "".to_string()),
    )));

    let auctions: Vec<Auction> = auctions()
        .idx
        .end_time
        .range(deps.storage, min, Some(max), order)
        .take(limit)
        .map(|item| item.map(|(_, v)| v))
        .collect::<StdResult<_>>()?;

    Ok(AuctionsResponse { auctions })
}
