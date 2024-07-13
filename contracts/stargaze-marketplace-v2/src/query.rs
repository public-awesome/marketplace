use crate::{
    helpers::build_collection_token_index_str,
    msg::{PriceOffset, QueryMsg},
    orders::{Ask, CollectionBid, Bid},
    state::{
        asks, bids, collection_bids, AllowDenoms, Config, Denom, OrderId, ALLOW_DENOMS, CONFIG,
    },
};

use cosmwasm_std::{to_json_binary, Addr, Binary, Deps, Env, StdResult};
use sg_index_query::{QueryOptions, QueryOptionsInternal};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    let api = deps.api;

    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::AllowDenoms {} => to_json_binary(&query_allow_denoms(deps)?),
        QueryMsg::Ask(id) => to_json_binary(&query_asks(deps, vec![id])?.pop()),
        QueryMsg::Asks(ids) => to_json_binary(&query_asks(deps, ids)?),
        QueryMsg::AsksByCollectionDenom {
            collection,
            denom,
            query_options,
        } => to_json_binary(&query_asks_by_collection_denom(
            deps,
            api.addr_validate(&collection)?,
            denom,
            query_options.unwrap_or(QueryOptions::default()),
        )?),
        QueryMsg::AsksByCreatorCollection {
            creator,
            collection,
            query_options,
        } => to_json_binary(&query_asks_by_creator_collection(
            deps,
            api.addr_validate(&creator)?,
            api.addr_validate(&collection)?,
            query_options.unwrap_or(QueryOptions::default()),
        )?),
        QueryMsg::Bid(id) => to_json_binary(&query_bids(deps, vec![id])?.pop()),
        QueryMsg::Bids(ids) => to_json_binary(&query_bids(deps, ids)?),
        QueryMsg::BidsByTokenPrice {
            collection,
            token_id,
            denom,
            query_options,
        } => to_json_binary(&query_bids_by_token_price(
            deps,
            api.addr_validate(&collection)?,
            token_id,
            denom,
            query_options.unwrap_or(QueryOptions::default()),
        )?),
        QueryMsg::BidsByCreatorCollection {
            creator,
            collection,
            query_options,
        } => to_json_binary(&query_bids_by_creator_collection(
            deps,
            api.addr_validate(&creator)?,
            api.addr_validate(&collection)?,
            query_options.unwrap_or(QueryOptions::default()),
        )?),
        QueryMsg::CollectionBid(id) => {
            to_json_binary(&query_collection_bids(deps, vec![id])?.pop())
        }
        QueryMsg::CollectionBids(ids) => to_json_binary(&query_collection_bids(deps, ids)?),
        QueryMsg::CollectionBidsByPrice {
            collection,
            denom,
            query_options,
        } => to_json_binary(&query_collection_bids_by_price(
            deps,
            api.addr_validate(&collection)?,
            denom,
            query_options.unwrap_or(QueryOptions::default()),
        )?),
        QueryMsg::CollectionBidsByCreatorCollection {
            creator,
            collection,
            query_options,
        } => to_json_binary(&query_collection_bids_by_creator_collection(
            deps,
            api.addr_validate(&creator)?,
            api.addr_validate(&collection)?,
            query_options.unwrap_or(QueryOptions::default()),
        )?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<Config<Addr>> {
    CONFIG.load(deps.storage)
}

pub fn query_allow_denoms(deps: Deps) -> StdResult<AllowDenoms> {
    ALLOW_DENOMS.load(deps.storage)
}

pub fn query_asks(deps: Deps, ids: Vec<OrderId>) -> StdResult<Vec<Ask>> {
    let mut retval = vec![];

    for id in ids {
        let ask = asks().may_load(deps.storage, id)?;
        if let Some(ask) = ask {
            retval.push(ask);
        }
    }

    Ok(retval)
}

pub fn query_asks_by_collection_denom(
    deps: Deps,
    collection: Addr,
    denom: Denom,
    query_options: QueryOptions<PriceOffset>,
) -> StdResult<Vec<Ask>> {
    let QueryOptionsInternal {
        limit,
        order,
        min,
        max,
    } = query_options.unpack(&(|offset| (offset.amount, offset.id.clone())), None, None);

    let results = asks()
        .idx
        .collection_denom_price
        .sub_prefix((collection, denom))
        .range(deps.storage, min, max, order)
        .take(limit)
        .map(|res| res.map(|(_, ask)| ask))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(results)
}

pub fn query_asks_by_creator_collection(
    deps: Deps,
    creator: Addr,
    collection: Addr,
    query_options: QueryOptions<OrderId>,
) -> StdResult<Vec<Ask>> {
    let QueryOptionsInternal {
        limit,
        order,
        min,
        max,
    } = query_options.unpack(&(|offset| offset.clone()), None, None);

    let results = asks()
        .idx
        .creator_collection
        .prefix((creator, collection))
        .range(deps.storage, min, max, order)
        .take(limit)
        .map(|res| res.map(|(_, ask)| ask))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(results)
}

pub fn query_bids(deps: Deps, ids: Vec<OrderId>) -> StdResult<Vec<Bid>> {
    let mut retval = vec![];

    for id in ids {
        let bid = bids().may_load(deps.storage, id)?;
        if let Some(bid) = bid {
            retval.push(bid);
        }
    }

    Ok(retval)
}

pub fn query_bids_by_token_price(
    deps: Deps,
    collection: Addr,
    token_id: String,
    denom: Denom,
    query_options: QueryOptions<PriceOffset>,
) -> StdResult<Vec<Bid>> {
    let QueryOptionsInternal {
        limit,
        order,
        min,
        max,
    } = query_options.unpack(&(|offset| (offset.amount, offset.id.clone())), None, None);

    let results = bids()
        .idx
        .token_denom_price
        .sub_prefix((
            build_collection_token_index_str(collection.as_ref(), &token_id),
            denom,
        ))
        .range(deps.storage, min, max, order)
        .take(limit)
        .map(|res| res.map(|(_, bid)| bid))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(results)
}

pub fn query_bids_by_creator_collection(
    deps: Deps,
    creator: Addr,
    collection: Addr,
    query_options: QueryOptions<OrderId>,
) -> StdResult<Vec<Bid>> {
    let QueryOptionsInternal {
        limit,
        order,
        min,
        max,
    } = query_options.unpack(&(|offset| offset.clone()), None, None);

    let results = bids()
        .idx
        .creator_collection
        .prefix((creator, collection))
        .range(deps.storage, min, max, order)
        .take(limit)
        .map(|res| res.map(|(_, bid)| bid))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(results)
}

pub fn query_collection_bids(deps: Deps, ids: Vec<OrderId>) -> StdResult<Vec<CollectionBid>> {
    let mut retval = vec![];

    for id in ids {
        let collection_bid = collection_bids().may_load(deps.storage, id)?;
        if let Some(collection_bid) = collection_bid {
            retval.push(collection_bid);
        }
    }

    Ok(retval)
}

pub fn query_collection_bids_by_price(
    deps: Deps,
    collection: Addr,
    denom: Denom,
    query_options: QueryOptions<PriceOffset>,
) -> StdResult<Vec<CollectionBid>> {
    let QueryOptionsInternal {
        limit,
        order,
        min,
        max,
    } = query_options.unpack(&(|offset| (offset.amount, offset.id.clone())), None, None);

    let results = collection_bids()
        .idx
        .collection_denom_price
        .sub_prefix((collection, denom))
        .range(deps.storage, min, max, order)
        .take(limit)
        .map(|res| res.map(|(_, collection_bid)| collection_bid))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(results)
}

pub fn query_collection_bids_by_creator_collection(
    deps: Deps,
    creator: Addr,
    collection: Addr,
    query_options: QueryOptions<OrderId>,
) -> StdResult<Vec<CollectionBid>> {
    let QueryOptionsInternal {
        limit,
        order,
        min,
        max,
    } = query_options.unpack(&(|offset| offset.clone()), None, None);

    let results = collection_bids()
        .idx
        .creator_collection
        .prefix((creator, collection))
        .range(deps.storage, min, max, order)
        .take(limit)
        .map(|res| res.map(|(_, collection_bid)| collection_bid))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(results)
}
