#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use crate::msg::{AuctionResponse, AuctionsResponse, ConfigResponse, QueryMsg, QueryOptions};
use crate::state::{auctions, Auction, AuctionKey};
use crate::state::{CONFIG, EXPIRING_AUCTIONS};
use cosmwasm_std::{to_binary, Addr, Binary, Deps, Env, Order, StdResult};
use cw_storage_plus::{Bound, PrimaryKey};

// Query limits
const DEFAULT_QUERY_LIMIT: u32 = 10;
const MAX_QUERY_LIMIT: u32 = 100;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
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
        QueryMsg::AuctionsByEndTime { query_options } => to_binary(&query_auctions_by_end_time(
            deps,
            query_options.unwrap_or(QueryOptions::default()),
        )?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse { config })
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
    query_options: QueryOptions<(String, String)>,
) -> StdResult<AuctionsResponse> {
    deps.api.addr_validate(&seller)?;

    let (limit, order, min, max) = unpack_query_options(query_options, |sa| {
        Bound::exclusive((Addr::unchecked(sa.0), sa.1))
    });

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
    query_options: QueryOptions<u64>,
) -> StdResult<AuctionsResponse> {
    let mut query_options = query_options;
    if query_options.start_after.is_none() {
        query_options.start_after = Some(0u64);
    }

    let (limit, order, min, max) = unpack_query_options(query_options, Bound::exclusive);

    let auction_keys: Vec<AuctionKey> = EXPIRING_AUCTIONS
        .range(deps.storage, min, max, order)
        .take(limit)
        .map(|item| item.map(|(_, v)| v))
        .collect::<StdResult<_>>()?;

    let auctions = auction_keys
        .iter()
        .map(|k| auctions().load(deps.storage, k.clone()))
        .collect::<StdResult<_>>()?;

    Ok(AuctionsResponse { auctions })
}

pub fn unpack_query_options<'a, T: PrimaryKey<'a>, U>(
    query_options: QueryOptions<U>,
    start_after_fn: fn(U) -> Bound<'a, T>,
) -> (usize, Order, Option<Bound<'a, T>>, Option<Bound<'a, T>>) {
    let limit = query_options
        .limit
        .unwrap_or(DEFAULT_QUERY_LIMIT)
        .min(MAX_QUERY_LIMIT) as usize;

    let mut order = Order::Ascending;
    if let Some(_descending) = query_options.descending {
        if _descending {
            order = Order::Descending;
        }
    };

    let (mut min, mut max) = (None, None);
    let mut bound = None;
    if let Some(_start_after) = query_options.start_after {
        bound = Some(start_after_fn(_start_after));
    };
    match order {
        Order::Ascending => min = bound,
        Order::Descending => max = bound,
    };

    (limit, order, min, max)
}
