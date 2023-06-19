use cosmwasm_std::{to_binary, Addr, Binary, Deps, Env, StdResult};
use cw_storage_plus::Bound;
use sg_marketplace_common::query::{unpack_query_options, QueryOptions};

use crate::{
    constants::{DEFAULT_QUERY_LIMIT, MAX_QUERY_LIMIT},
    helpers::build_collection_token_index_str,
    msg::QueryMsg,
    state::{
        asks, collection_offers, offers, Ask, CollectionOffer, Denom, Offer, SudoParams, TokenId,
        ASK_HOOKS, OFFER_HOOKS, SALE_HOOKS, SUDO_PARAMS,
    },
};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    let api = deps.api;

    match msg {
        QueryMsg::SudoParams {} => to_binary(&query_sudo_params(deps)?),
        QueryMsg::Ask {
            collection,
            token_id,
        } => to_binary(&query_ask(deps, api.addr_validate(&collection)?, token_id)?),
        QueryMsg::Asks {
            collection,
            query_options,
        } => to_binary(&query_asks(
            deps,
            api.addr_validate(&collection)?,
            query_options.unwrap_or(QueryOptions::default()),
        )?),
        QueryMsg::AsksByPrice {
            collection,
            denom,
            query_options,
        } => to_binary(&query_asks_by_price(
            deps,
            api.addr_validate(&collection)?,
            denom,
            query_options.unwrap_or(QueryOptions::default()),
        )?),
        QueryMsg::AsksBySeller {
            seller,
            query_options,
        } => to_binary(&query_asks_by_seller(
            deps,
            api.addr_validate(&seller)?,
            query_options.unwrap_or(QueryOptions::default()),
        )?),
        QueryMsg::AsksByExpiration { query_options } => to_binary(&query_asks_by_expiration(
            deps,
            query_options.unwrap_or(QueryOptions::default()),
        )?),
        QueryMsg::Offer {
            collection,
            token_id,
            bidder,
        } => to_binary(&query_offer(
            deps,
            api.addr_validate(&collection)?,
            token_id,
            api.addr_validate(&bidder)?,
        )?),
        QueryMsg::OffersByCollection {
            collection,
            query_options,
        } => to_binary(&query_offers_by_collection(
            deps,
            api.addr_validate(&collection)?,
            query_options.unwrap_or(QueryOptions::default()),
        )?),
        QueryMsg::OffersByTokenPrice {
            collection,
            token_id,
            denom,
            query_options,
        } => to_binary(&query_offers_by_token_price(
            deps,
            api.addr_validate(&collection)?,
            token_id,
            denom,
            query_options.unwrap_or(QueryOptions::default()),
        )?),
        QueryMsg::OffersByBidder {
            bidder,
            query_options,
        } => to_binary(&query_offers_by_bidder(
            deps,
            api.addr_validate(&bidder)?,
            query_options.unwrap_or(QueryOptions::default()),
        )?),
        QueryMsg::OffersByExpiration { query_options } => to_binary(&query_offers_by_expiration(
            deps,
            query_options.unwrap_or(QueryOptions::default()),
        )?),
        QueryMsg::CollectionOffer { collection, bidder } => to_binary(&query_collection_offer(
            deps,
            api.addr_validate(&collection)?,
            api.addr_validate(&bidder)?,
        )?),
        QueryMsg::CollectionOffersByPrice {
            collection,
            denom,
            query_options,
        } => to_binary(&query_collection_offers_by_price(
            deps,
            api.addr_validate(&collection)?,
            denom,
            query_options.unwrap_or(QueryOptions::default()),
        )?),
        QueryMsg::CollectionOffersByBidder {
            bidder,
            query_options,
        } => to_binary(&query_collection_offers_by_bidder(
            deps,
            api.addr_validate(&bidder)?,
            query_options.unwrap_or(QueryOptions::default()),
        )?),
        QueryMsg::CollectionOffersByExpiration { query_options } => {
            to_binary(&query_collection_offers_by_expiration(
                deps,
                query_options.unwrap_or(QueryOptions::default()),
            )?)
        }
        QueryMsg::AskHooks {} => to_binary(&ASK_HOOKS.query_hooks(deps)?),
        QueryMsg::BidHooks {} => to_binary(&OFFER_HOOKS.query_hooks(deps)?),
        QueryMsg::SaleHooks {} => to_binary(&SALE_HOOKS.query_hooks(deps)?),
    }
}

pub fn query_sudo_params(deps: Deps) -> StdResult<SudoParams> {
    Ok(SUDO_PARAMS.load(deps.storage)?)
}

pub fn query_ask(deps: Deps, collection: Addr, token_id: TokenId) -> StdResult<Option<Ask>> {
    let ask = asks().may_load(deps.storage, Ask::build_key(&collection, &token_id))?;
    Ok(ask)
}

pub fn query_asks(
    deps: Deps,
    collection: Addr,
    query_options: QueryOptions<TokenId>,
) -> StdResult<Vec<Ask>> {
    let (limit, order, min, max) = unpack_query_options(
        query_options,
        Box::new(Bound::exclusive),
        DEFAULT_QUERY_LIMIT,
        MAX_QUERY_LIMIT,
    );

    let asks = asks()
        .prefix(collection)
        .range(deps.storage, min, max, order)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(asks)
}

pub fn query_asks_by_price(
    deps: Deps,
    collection: Addr,
    denom: Denom,
    query_options: QueryOptions<u128>,
) -> StdResult<Vec<Ask>> {
    let collection_clone = collection.clone();
    let denom_clone = denom.clone();
    let (limit, order, min, max) = unpack_query_options(
        query_options,
        Box::new(|price| Bound::exclusive((price, (collection_clone, denom_clone)))),
        DEFAULT_QUERY_LIMIT,
        MAX_QUERY_LIMIT,
    );

    let asks = asks()
        .idx
        .collection_denom_price
        .sub_prefix((collection, denom))
        .range(deps.storage, min, max, order)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(asks)
}

pub fn query_asks_by_seller(
    deps: Deps,
    seller: Addr,
    query_options: QueryOptions<(String, TokenId)>,
) -> StdResult<Vec<Ask>> {
    let (limit, order, min, max) = unpack_query_options(
        query_options,
        Box::new(|sa| Bound::exclusive((Addr::unchecked(sa.0), sa.1))),
        DEFAULT_QUERY_LIMIT,
        MAX_QUERY_LIMIT,
    );

    let asks = asks()
        .idx
        .seller
        .prefix(seller)
        .range(deps.storage, min, max, order)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(asks)
}

pub fn query_asks_by_expiration(
    deps: Deps,
    query_options: QueryOptions<(u64, String, TokenId)>,
) -> StdResult<Vec<Ask>> {
    let (limit, order, min, max) = unpack_query_options(
        query_options,
        Box::new(|sa| Bound::exclusive((sa.0, (Addr::unchecked(sa.1), sa.2)))),
        DEFAULT_QUERY_LIMIT,
        MAX_QUERY_LIMIT,
    );

    let max = max.map(|_| Bound::exclusive((u64::MAX, (Addr::unchecked(""), "".to_string()))));

    let asks = asks()
        .idx
        .expiration
        .range(deps.storage, min, max, order)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(asks)
}

pub fn query_offer(
    deps: Deps,
    collection: Addr,
    token_id: TokenId,
    bidder: Addr,
) -> StdResult<Option<Offer>> {
    let offer = offers().may_load(
        deps.storage,
        Offer::build_key(&collection, &token_id, &bidder),
    )?;
    Ok(offer)
}

pub fn query_offers_by_collection(
    deps: Deps,
    collection: Addr,
    query_options: QueryOptions<(TokenId, String)>,
) -> StdResult<Vec<Offer>> {
    let (limit, order, min, max) = unpack_query_options(
        query_options,
        Box::new(|sa| Bound::exclusive((sa.0, Addr::unchecked(sa.1)))),
        DEFAULT_QUERY_LIMIT,
        MAX_QUERY_LIMIT,
    );

    let offers = offers()
        .sub_prefix(collection)
        .range(deps.storage, min, max, order)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(offers)
}

pub fn query_offers_by_token_price(
    deps: Deps,
    collection: Addr,
    token_id: TokenId,
    denom: Denom,
    query_options: QueryOptions<(u128, String)>,
) -> StdResult<Vec<Offer>> {
    let collection_token_index_str =
        build_collection_token_index_str(&collection.to_string(), &token_id);

    let (limit, order, min, max) = unpack_query_options(
        query_options,
        Box::new(|sa| Bound::exclusive((sa.0, (collection, token_id, Addr::unchecked(sa.1))))),
        DEFAULT_QUERY_LIMIT,
        MAX_QUERY_LIMIT,
    );

    let offers = offers()
        .idx
        .token_denom_price
        .sub_prefix((collection_token_index_str, denom))
        .range(deps.storage, min, max, order)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(offers)
}

pub fn query_offers_by_bidder(
    deps: Deps,
    bidder: Addr,
    query_options: QueryOptions<(String, TokenId)>,
) -> StdResult<Vec<Offer>> {
    let bidder_clone = bidder.clone();
    let (limit, order, min, max) = unpack_query_options(
        query_options,
        Box::new(|sa| Bound::exclusive((Addr::unchecked(sa.0), sa.1, bidder_clone))),
        DEFAULT_QUERY_LIMIT,
        MAX_QUERY_LIMIT,
    );

    let offers = offers()
        .idx
        .bidder
        .prefix(bidder)
        .range(deps.storage, min, max, order)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(offers)
}

pub fn query_offers_by_expiration(
    deps: Deps,
    query_options: QueryOptions<(u64, String, TokenId, String)>,
) -> StdResult<Vec<Offer>> {
    let (limit, order, min, max) = unpack_query_options(
        query_options,
        Box::new(|sa| {
            Bound::exclusive((sa.0, (Addr::unchecked(sa.1), sa.2, Addr::unchecked(sa.3))))
        }),
        DEFAULT_QUERY_LIMIT,
        MAX_QUERY_LIMIT,
    );

    let offers = offers()
        .idx
        .expiration
        .range(deps.storage, min, max, order)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(offers)
}

pub fn query_collection_offer(
    deps: Deps,
    collection: Addr,
    bidder: Addr,
) -> StdResult<Option<CollectionOffer>> {
    let collection_offer = collection_offers().may_load(
        deps.storage,
        CollectionOffer::build_key(&collection, &bidder),
    )?;
    Ok(collection_offer)
}

pub fn query_collection_offers_by_price(
    deps: Deps,
    collection: Addr,
    denom: Denom,
    query_options: QueryOptions<(u128, String)>,
) -> StdResult<Vec<CollectionOffer>> {
    let collection_clone = collection.clone();
    let (limit, order, min, max) = unpack_query_options(
        query_options,
        Box::new(|sa| Bound::exclusive((sa.0, (collection_clone, Addr::unchecked(sa.1))))),
        DEFAULT_QUERY_LIMIT,
        MAX_QUERY_LIMIT,
    );

    let offers = collection_offers()
        .idx
        .collection_denom_price
        .sub_prefix((collection, denom))
        .range(deps.storage, min, max, order)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(offers)
}

pub fn query_collection_offers_by_bidder(
    deps: Deps,
    bidder: Addr,
    query_options: QueryOptions<String>,
) -> StdResult<Vec<CollectionOffer>> {
    let bidder_clone = bidder.clone();
    let (limit, order, min, max) = unpack_query_options(
        query_options,
        Box::new(|sa| Bound::exclusive((Addr::unchecked(sa), bidder_clone))),
        DEFAULT_QUERY_LIMIT,
        MAX_QUERY_LIMIT,
    );

    let offers = collection_offers()
        .idx
        .bidder
        .prefix(bidder)
        .range(deps.storage, min, max, order)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(offers)
}

pub fn query_collection_offers_by_expiration(
    deps: Deps,
    query_options: QueryOptions<(u64, String, String)>,
) -> StdResult<Vec<CollectionOffer>> {
    let (limit, order, min, max) = unpack_query_options(
        query_options,
        Box::new(|sa| Bound::exclusive((sa.0, (Addr::unchecked(sa.1), Addr::unchecked(sa.2))))),
        DEFAULT_QUERY_LIMIT,
        MAX_QUERY_LIMIT,
    );

    let offers = collection_offers()
        .idx
        .expiration
        .range(deps.storage, min, max, order)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(offers)
}
