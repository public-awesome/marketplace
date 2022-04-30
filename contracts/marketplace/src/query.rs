use crate::msg::{
    AskCountResponse, AsksResponse, BidResponse, Bidder, BidsResponse, Collection,
    CollectionBidResponse, CollectionBidsResponse, CollectionsResponse, CurrentAskResponse, Offset,
    ParamsResponse, QueryMsg,
};
use crate::state::{
    ask_key, asks, bids, collection_bid_key, collection_bids, TokenId, ASK_HOOKS,
    SALE_FINALIZED_HOOKS, SUDO_PARAMS,
};
use cosmwasm_std::{entry_point, to_binary, Addr, Binary, Deps, Env, Order, StdResult};
use cw_storage_plus::{Bound, PrefixBound};
use cw_utils::maybe_addr;

// Query limits
const DEFAULT_QUERY_LIMIT: u32 = 10;
const MAX_QUERY_LIMIT: u32 = 30;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    let api = deps.api;

    match msg {
        QueryMsg::CurrentAsk {
            collection,
            token_id,
        } => to_binary(&query_current_ask(
            deps,
            api.addr_validate(&collection)?,
            token_id,
        )?),
        QueryMsg::Asks {
            collection,
            start_after,
            limit,
        } => to_binary(&query_asks(
            deps,
            api.addr_validate(&collection)?,
            start_after,
            limit,
        )?),
        QueryMsg::AsksSortedByPrice {
            collection,
            start_after,
            limit,
        } => to_binary(&query_asks_sorted_by_price(
            deps,
            api.addr_validate(&collection)?,
            start_after,
            limit,
        )?),
        QueryMsg::ReverseAsksSortedByPrice {
            collection,
            start_before,
            limit,
        } => to_binary(&reverse_query_asks_sorted_by_price(
            deps,
            api.addr_validate(&collection)?,
            start_before,
            limit,
        )?),
        QueryMsg::ListedCollections { start_after, limit } => {
            to_binary(&query_listed_collections(deps, start_after, limit)?)
        }
        QueryMsg::AsksBySeller { seller } => {
            to_binary(&query_asks_by_seller(deps, api.addr_validate(&seller)?)?)
        }
        QueryMsg::AskCount { collection } => {
            to_binary(&query_ask_count(deps, api.addr_validate(&collection)?)?)
        }
        QueryMsg::Bid {
            collection,
            token_id,
            bidder,
        } => to_binary(&query_bid(
            deps,
            api.addr_validate(&collection)?,
            token_id,
            api.addr_validate(&bidder)?,
        )?),
        QueryMsg::Bids {
            collection,
            token_id,
            start_after,
            limit,
        } => to_binary(&query_bids(
            deps,
            api.addr_validate(&collection)?,
            token_id,
            start_after,
            limit,
        )?),
        QueryMsg::BidsByBidder { bidder } => {
            to_binary(&query_bids_by_bidder(deps, api.addr_validate(&bidder)?)?)
        }
        QueryMsg::BidsSortedByPrice {
            collection,
            limit,
            order_asc,
        } => to_binary(&query_bids_sorted_by_price(
            deps,
            api.addr_validate(&collection)?,
            limit,
            order_asc,
        )?),
        QueryMsg::Params {} => to_binary(&query_params(deps)?),
        QueryMsg::CollectionBid { collection, bidder } => to_binary(&query_collection_bid(
            deps,
            api.addr_validate(&collection)?,
            api.addr_validate(&bidder)?,
        )?),
        QueryMsg::CollectionBidsSortedByPrice {
            collection,
            limit,
            order_asc,
        } => to_binary(&query_collection_bids_sorted_by_price(
            deps,
            api.addr_validate(&collection)?,
            limit,
            order_asc,
        )?),
        QueryMsg::CollectionBidsByBidder { bidder } => to_binary(&query_collection_bids_by_bidder(
            deps,
            api.addr_validate(&bidder)?,
        )?),
        QueryMsg::SaleFinalizedHooks {} => to_binary(&SALE_FINALIZED_HOOKS.query_hooks(deps)?),
        QueryMsg::AskHooks {} => to_binary(&ASK_HOOKS.query_hooks(deps)?),
    }
}

pub fn query_params(deps: Deps) -> StdResult<ParamsResponse> {
    let config = SUDO_PARAMS.load(deps.storage)?;

    Ok(ParamsResponse { params: config })
}

pub fn query_asks(
    deps: Deps,
    collection: Addr,
    start_after: Option<TokenId>,
    limit: Option<u32>,
) -> StdResult<AsksResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;

    let asks: StdResult<Vec<_>> = asks()
        .idx
        .collection
        .prefix(collection.clone())
        .range(
            deps.storage,
            Some(Bound::exclusive((
                collection,
                start_after.unwrap_or_default(),
            ))),
            None,
            Order::Ascending,
        )
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect();

    Ok(AsksResponse { asks: asks? })
}

pub fn query_asks_sorted_by_price(
    deps: Deps,
    collection: Addr,
    start_after: Option<Offset>,
    limit: Option<u32>,
) -> StdResult<AsksResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;

    let start = start_after.map(|offset| {
        Bound::exclusive((
            offset.price.u128(),
            ask_key(collection.clone(), offset.token_id),
        ))
    });

    let asks = asks()
        .idx
        .collection_price
        .sub_prefix(collection)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(AsksResponse { asks })
}

pub fn reverse_query_asks_sorted_by_price(
    deps: Deps,
    collection: Addr,
    start_before: Option<Offset>,
    limit: Option<u32>,
) -> StdResult<AsksResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;

    let end = start_before.map(|offset| {
        Bound::exclusive((
            offset.price.u128(),
            ask_key(collection.clone(), offset.token_id),
        ))
    });

    let asks = asks()
        .idx
        .collection_price
        .sub_prefix(collection)
        .range(deps.storage, None, end, Order::Descending)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(AsksResponse { asks })
}

pub fn query_ask_count(deps: Deps, collection: Addr) -> StdResult<AskCountResponse> {
    let count = asks()
        .idx
        .collection
        .prefix(collection)
        .keys_raw(deps.storage, None, None, Order::Ascending)
        .count() as u32;

    Ok(AskCountResponse { count })
}

pub fn query_asks_by_seller(deps: Deps, seller: Addr) -> StdResult<AsksResponse> {
    let asks: StdResult<Vec<_>> = asks()
        .idx
        .seller
        .prefix(seller)
        .range(deps.storage, None, None, Order::Ascending)
        .map(|res| res.map(|item| item.1))
        .collect();

    Ok(AsksResponse { asks: asks? })
}

pub fn query_listed_collections(
    deps: Deps,
    start_after: Option<Collection>,
    limit: Option<u32>,
) -> StdResult<CollectionsResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;
    let start_addr = maybe_addr(deps.api, start_after)?;

    let collections: StdResult<Vec<_>> = asks()
        .prefix_range(
            deps.storage,
            start_addr.map(PrefixBound::exclusive),
            None,
            Order::Ascending,
        )
        .take(limit)
        .map(|item| item.map(|(key, _)| key.0))
        .collect();

    Ok(CollectionsResponse {
        collections: collections?,
    })
}

pub fn query_current_ask(
    deps: Deps,
    collection: Addr,
    token_id: TokenId,
) -> StdResult<CurrentAskResponse> {
    let ask = asks().may_load(deps.storage, ask_key(collection, token_id))?;

    Ok(CurrentAskResponse { ask })
}

pub fn query_bid(
    deps: Deps,
    collection: Addr,
    token_id: TokenId,
    bidder: Addr,
) -> StdResult<BidResponse> {
    let bid = bids().may_load(deps.storage, (collection, token_id, bidder))?;

    Ok(BidResponse { bid })
}

pub fn query_bids_by_bidder(deps: Deps, bidder: Addr) -> StdResult<BidsResponse> {
    let bids = bids()
        .idx
        .bidder
        .prefix(bidder)
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| item.map(|(_, b)| b))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(BidsResponse { bids })
}

pub fn query_bids(
    deps: Deps,
    collection: Addr,
    token_id: TokenId,
    start_after: Option<Bidder>,
    limit: Option<u32>,
) -> StdResult<BidsResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;
    let start = start_after.map(|s| Bound::ExclusiveRaw(s.into()));

    let bids = bids()
        .idx
        .collection_token_id
        .prefix((collection, token_id))
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| item.map(|(_, b)| b))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(BidsResponse { bids })
}

pub fn query_bids_sorted_by_price(
    deps: Deps,
    collection: Addr,
    limit: Option<u32>,
    order_asc: bool,
) -> StdResult<BidsResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;

    let order = if order_asc {
        Order::Ascending
    } else {
        Order::Descending
    };

    let bids = bids()
        .idx
        .collection_price
        .sub_prefix(collection)
        .range(deps.storage, None, None, order)
        .take(limit)
        .map(|item| item.map(|(_, b)| b))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(BidsResponse { bids })
}

pub fn query_collection_bid(
    deps: Deps,
    collection: Addr,
    bidder: Addr,
) -> StdResult<CollectionBidResponse> {
    let bid = collection_bids().may_load(deps.storage, collection_bid_key(collection, bidder))?;

    Ok(CollectionBidResponse { bid })
}

pub fn query_collection_bids_sorted_by_price(
    deps: Deps,
    collection: Addr,
    limit: Option<u32>,
    order_asc: bool,
) -> StdResult<CollectionBidsResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;

    let order = if order_asc {
        Order::Ascending
    } else {
        Order::Descending
    };

    let bids = collection_bids()
        .idx
        .collection_price
        .sub_prefix(collection)
        .range(deps.storage, None, None, order)
        .take(limit)
        .map(|item| item.map(|(_, b)| b))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(CollectionBidsResponse { bids })
}

pub fn query_collection_bids_by_bidder(
    deps: Deps,
    bidder: Addr,
) -> StdResult<CollectionBidsResponse> {
    let bids = collection_bids()
        .idx
        .bidder
        .prefix(bidder)
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| item.map(|(_, b)| b))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(CollectionBidsResponse { bids })
}
