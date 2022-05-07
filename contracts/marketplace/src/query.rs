use crate::msg::{
    AskCountResponse, AskOffset, AskResponse, AsksResponse, BidOffset, BidResponse, Bidder,
    BidsResponse, Collection, CollectionBidOffset, CollectionBidResponse, CollectionBidsResponse,
    CollectionOffset, CollectionsResponse, ParamsResponse, QueryMsg,
};
use crate::state::{
    ask_key, asks, bid_key, bids, collection_bid_key, collection_bids, BidKey, CollectionBid,
    TokenId, ASK_HOOKS, BID_HOOKS, SALE_HOOKS, SUDO_PARAMS,
};
use crate::ContractError;
use cosmwasm_std::{entry_point, to_binary, Addr, Binary, Deps, Env, Order, StdError, StdResult};
use cw_storage_plus::{Bound, PrefixBound};
use cw_utils::maybe_addr;

// Query limits
const DEFAULT_QUERY_LIMIT: u32 = 10;
const MAX_QUERY_LIMIT: u32 = 30;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    let api = deps.api;

    match msg {
        QueryMsg::Collections { start_after, limit } => {
            to_binary(&query_collections(deps, start_after, limit)?)
        }
        QueryMsg::Ask {
            collection,
            token_id,
        } => to_binary(&query_ask(deps, api.addr_validate(&collection)?, token_id)?),
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
        QueryMsg::AsksBySeller {
            seller,
            start_after,
            limit,
        } => to_binary(&query_asks_by_seller(
            deps,
            api.addr_validate(&seller)?,
            start_after,
            limit,
        )?),
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
        QueryMsg::BidsByBidder {
            bidder,
            start_after,
            limit,
        } => to_binary(&query_bids_by_bidder(
            deps,
            api.addr_validate(&bidder)?,
            start_after,
            limit,
        )?),
        QueryMsg::BidsSortedByPrice {
            collection,
            start_after,
            limit,
        } => to_binary(&query_bids_sorted_by_price(
            deps,
            api.addr_validate(&collection)?,
            start_after,
            limit,
        )?),
        QueryMsg::ReverseBidsSortedByPrice {
            collection,
            start_before,
            limit,
        } => to_binary(&reverse_query_bids_sorted_by_price(
            deps,
            api.addr_validate(&collection)?,
            start_before,
            limit,
        )?),
        QueryMsg::BidsByBidderSortedByExpiration {
            bidder,
            start_after,
            limit,
        } => to_binary(&query_bids_by_bidder_sorted_by_expiry(
            deps,
            api.addr_validate(&bidder)?,
            start_after,
            limit,
        )?),
        QueryMsg::CollectionBid { collection, bidder } => to_binary(&query_collection_bid(
            deps,
            api.addr_validate(&collection)?,
            api.addr_validate(&bidder)?,
        )?),
        QueryMsg::CollectionBidsSortedByPrice {
            collection,
            start_after,
            limit,
        } => to_binary(&query_collection_bids_sorted_by_price(
            deps,
            api.addr_validate(&collection)?,
            start_after,
            limit,
        )?),
        QueryMsg::ReverseCollectionBidsSortedByPrice {
            collection,
            start_before,
            limit,
        } => to_binary(&reverse_query_collection_bids_sorted_by_price(
            deps,
            api.addr_validate(&collection)?,
            start_before,
            limit,
        )?),
        QueryMsg::CollectionBidsByBidder { bidder } => to_binary(&query_collection_bids_by_bidder(
            deps,
            api.addr_validate(&bidder)?,
        )?),
        QueryMsg::CollectionBidsSortedByExpiration {
            collection,
            start_after,
            limit,
        } => to_binary(&query_collection_bids_by_bidder_sorted_by_expiry(
            deps,
            api.addr_validate(&collection)?,
            start_after,
            limit,
        )?),
        QueryMsg::AskHooks {} => to_binary(&ASK_HOOKS.query_hooks(deps)?),
        QueryMsg::BidHooks {} => to_binary(&BID_HOOKS.query_hooks(deps)?),
        QueryMsg::SaleHooks {} => to_binary(&SALE_HOOKS.query_hooks(deps)?),
        QueryMsg::Params {} => to_binary(&query_params(deps)?),
    }
}

pub fn query_collections(
    deps: Deps,
    start_after: Option<Collection>,
    limit: Option<u32>,
) -> StdResult<CollectionsResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;
    let start_addr = maybe_addr(deps.api, start_after)?;

    let collections = asks()
        .prefix_range(
            deps.storage,
            start_addr.map(PrefixBound::exclusive),
            None,
            Order::Ascending,
        )
        .take(limit)
        .map(|item| item.map(|(key, _)| key.0))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(CollectionsResponse { collections })
}

pub fn query_asks(
    deps: Deps,
    collection: Addr,
    start_after: Option<TokenId>,
    limit: Option<u32>,
) -> StdResult<AsksResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;

    let asks = asks()
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
        .collect::<StdResult<Vec<_>>>()?;

    Ok(AsksResponse { asks })
}

pub fn query_asks_sorted_by_price(
    deps: Deps,
    collection: Addr,
    start_after: Option<AskOffset>,
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
    start_before: Option<AskOffset>,
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

pub fn query_asks_by_seller(
    deps: Deps,
    seller: Addr,
    start_after: Option<CollectionOffset>,
    limit: Option<u32>,
) -> StdResult<AsksResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;

    let start = if let Some(start) = start_after {
        let collection = deps.api.addr_validate(&start.collection)?;
        Some(Bound::exclusive(ask_key(collection, start.token_id)))
    } else {
        None
    };

    let asks = asks()
        .idx
        .seller
        .prefix(seller)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(AsksResponse { asks })
}

pub fn query_ask(deps: Deps, collection: Addr, token_id: TokenId) -> StdResult<AskResponse> {
    let ask = asks().may_load(deps.storage, ask_key(collection, token_id))?;

    Ok(AskResponse { ask })
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

pub fn query_bids_by_bidder(
    deps: Deps,
    bidder: Addr,
    start_after: Option<CollectionOffset>,
    limit: Option<u32>,
) -> StdResult<BidsResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;

    let start = if let Some(start) = start_after {
        let collection = deps.api.addr_validate(&start.collection)?;
        Some(Bound::exclusive(bid_key(
            collection,
            start.token_id,
            bidder.clone(),
        )))
    } else {
        None
    };

    let bids = bids()
        .idx
        .bidder
        .prefix(bidder)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
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
    start_after: Option<BidOffset>,
    limit: Option<u32>,
) -> StdResult<BidsResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;

    let start: Option<Bound<(u128, BidKey)>> = start_after.map(|offset| {
        Bound::exclusive((
            offset.price.u128(),
            bid_key(collection.clone(), offset.token_id, offset.bidder),
        ))
    });

    let bids = bids()
        .idx
        .collection_price
        .sub_prefix(collection)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| item.map(|(_, b)| b))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(BidsResponse { bids })
}

pub fn reverse_query_bids_sorted_by_price(
    deps: Deps,
    collection: Addr,
    start_before: Option<BidOffset>,
    limit: Option<u32>,
) -> StdResult<BidsResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;

    let end: Option<Bound<(u128, BidKey)>> = start_before.map(|offset| {
        Bound::exclusive((
            offset.price.u128(),
            bid_key(collection.clone(), offset.token_id, offset.bidder),
        ))
    });

    let bids = bids()
        .idx
        .collection_price
        .sub_prefix(collection)
        .range(deps.storage, None, end, Order::Descending)
        .take(limit)
        .map(|item| item.map(|(_, b)| b))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(BidsResponse { bids })
}

pub fn query_bids_by_bidder_sorted_by_expiry(
    deps: Deps,
    bidder: Addr,
    start_after: Option<CollectionOffset>,
    limit: Option<u32>,
) -> StdResult<BidsResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;

    let start = if let Some(start) = start_after {
        let collection = deps.api.addr_validate(&start.collection)?;
        let bid = query_bid(deps, collection.clone(), start.token_id, bidder.clone())?;
        Some(Bound::exclusive((
            bid.bid.unwrap().expires_at.seconds(),
            bid_key(collection, start.token_id, bidder.clone()),
        )))
    } else {
        None
    };

    let bids = bids()
        .idx
        .bidder_expires_at
        .sub_prefix(bidder)
        .range(deps.storage, start, None, Order::Ascending)
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
    start_after: Option<CollectionBidOffset>,
    limit: Option<u32>,
) -> StdResult<CollectionBidsResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;
    let start = start_after.map(|offset| {
        let bidder = deps.api.addr_validate(&offset.bidder).unwrap();
        Bound::exclusive((offset.price.u128(), (collection.clone(), bidder)))
    });

    let bids = collection_bids()
        .idx
        .collection_price
        .sub_prefix(collection)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| item.map(|(_, b)| b))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(CollectionBidsResponse { bids })
}

pub fn reverse_query_collection_bids_sorted_by_price(
    deps: Deps,
    collection: Addr,
    start_before: Option<CollectionBidOffset>,
    limit: Option<u32>,
) -> StdResult<CollectionBidsResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;
    let end = start_before.map(|offset| {
        let bidder = deps.api.addr_validate(&offset.bidder).unwrap();
        Bound::exclusive((offset.price.u128(), (collection.clone(), bidder)))
    });

    let bids = collection_bids()
        .idx
        .collection_price
        .sub_prefix(collection)
        .range(deps.storage, None, end, Order::Descending)
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

pub fn query_collection_bids_by_bidder_sorted_by_expiry(
    deps: Deps,
    bidder: Addr,
    start_after: Option<CollectionBidOffset>,
    limit: Option<u32>,
) -> StdResult<CollectionBidsResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;

    let start = match start_after {
        Some(offset) => {
            let bidder = deps.api.addr_validate(&offset.bidder)?;
            let collection = deps.api.addr_validate(&offset.collection)?;
            let collection_bid =
                query_collection_bid(deps, collection.clone(), bidder.clone())?.bid;
            let bound = match collection_bid {
                Some(collection_bid) => Some(Bound::exclusive((
                    collection_bid.expires_at.seconds(),
                    (collection, bidder),
                ))),
                None => None,
            };
            bound
        }
        None => None,
    };

    let bids = collection_bids()
        .idx
        .bidder_expires_at
        .sub_prefix(bidder)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| item.map(|(_, b)| b))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(CollectionBidsResponse { bids })
}

pub fn query_params(deps: Deps) -> StdResult<ParamsResponse> {
    let config = SUDO_PARAMS.load(deps.storage)?;

    Ok(ParamsResponse { params: config })
}
