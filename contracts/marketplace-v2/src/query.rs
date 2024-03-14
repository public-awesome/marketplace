use crate::{
    helpers::build_collection_token_index_str,
    msg::{
        AsksByCollectionOffset, AsksByCreatorOffset, AsksByExpirationOffset, AsksByPriceOffset,
        CollectionOffersByCollectionOffset, CollectionOffersByCreatorOffset,
        CollectionOffersByExpirationOffset, CollectionOffersByPriceOffset,
        OffersByCollectionOffset, OffersByCreatorOffset, OffersByExpirationOffset,
        OffersByTokenPriceOffset, QueryMsg,
    },
    state::{
        asks, collection_offers, offers, Ask, CollectionOffer, Config, Denom, Offer, PriceRange,
        TokenId, PRICE_RANGES, SUDO_PARAMS,
    },
};

use cosmwasm_std::{to_json_binary, Addr, Binary, Deps, Env, StdResult};
use cw_storage_plus::Bound;
use sg_index_query::{QueryOptions, QueryOptionsInternal};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    let api = deps.api;

    match msg {
        QueryMsg::Config {} => to_json_binary(&query_sudo_params(deps)?),
        QueryMsg::PriceRange { denom } => to_json_binary(&query_price_range(deps, denom)?),
        QueryMsg::PriceRanges { query_options } => to_json_binary(&query_price_ranges(
            deps,
            query_options.unwrap_or(QueryOptions::default()),
        )?),
        QueryMsg::Ask {
            collection,
            token_id,
        } => to_json_binary(&query_ask(deps, api.addr_validate(&collection)?, token_id)?),
        QueryMsg::AsksByCollection {
            collection,
            query_options,
        } => to_json_binary(&query_asks_by_collection(
            deps,
            api.addr_validate(&collection)?,
            query_options.unwrap_or(QueryOptions::default()),
        )?),
        QueryMsg::AsksByPrice {
            collection,
            denom,
            query_options,
        } => to_json_binary(&query_asks_by_price(
            deps,
            api.addr_validate(&collection)?,
            denom,
            query_options.unwrap_or(QueryOptions::default()),
        )?),
        QueryMsg::AsksByCreator {
            creator,
            query_options,
        } => to_json_binary(&query_asks_by_creator(
            deps,
            api.addr_validate(&creator)?,
            query_options.unwrap_or(QueryOptions::default()),
        )?),
        QueryMsg::AsksByExpiration { query_options } => to_json_binary(&query_asks_by_expiration(
            deps,
            query_options.unwrap_or(QueryOptions::default()),
        )?),
        QueryMsg::Offer {
            collection,
            token_id,
            creator,
        } => to_json_binary(&query_offer(
            deps,
            api.addr_validate(&collection)?,
            token_id,
            api.addr_validate(&creator)?,
        )?),
        QueryMsg::OffersByCollection {
            collection,
            query_options,
        } => to_json_binary(&query_offers_by_collection(
            deps,
            api.addr_validate(&collection)?,
            query_options.unwrap_or(QueryOptions::default()),
        )?),
        QueryMsg::OffersByTokenPrice {
            collection,
            token_id,
            denom,
            query_options,
        } => to_json_binary(&query_offers_by_token_price(
            deps,
            api.addr_validate(&collection)?,
            token_id,
            denom,
            query_options.unwrap_or(QueryOptions::default()),
        )?),
        QueryMsg::OffersByCreator {
            creator,
            query_options,
        } => to_json_binary(&query_offers_by_creator(
            deps,
            api.addr_validate(&creator)?,
            query_options.unwrap_or(QueryOptions::default()),
        )?),
        QueryMsg::OffersByExpiration { query_options } => to_json_binary(
            &query_offers_by_expiration(deps, query_options.unwrap_or(QueryOptions::default()))?,
        ),
        QueryMsg::CollectionOffer {
            collection,
            creator,
        } => to_json_binary(&query_collection_offer(
            deps,
            api.addr_validate(&collection)?,
            api.addr_validate(&creator)?,
        )?),
        QueryMsg::CollectionOffersByCollection {
            collection,
            query_options,
        } => to_json_binary(&query_collection_offers_by_collection(
            deps,
            api.addr_validate(&collection)?,
            query_options.unwrap_or(QueryOptions::default()),
        )?),
        QueryMsg::CollectionOffersByPrice {
            collection,
            denom,
            query_options,
        } => to_json_binary(&query_collection_offers_by_price(
            deps,
            api.addr_validate(&collection)?,
            denom,
            query_options.unwrap_or(QueryOptions::default()),
        )?),
        QueryMsg::CollectionOffersByCreator {
            creator,
            query_options,
        } => to_json_binary(&query_collection_offers_by_creator(
            deps,
            api.addr_validate(&creator)?,
            query_options.unwrap_or(QueryOptions::default()),
        )?),
        QueryMsg::CollectionOffersByExpiration { query_options } => {
            to_json_binary(&query_collection_offers_by_expiration(
                deps,
                query_options.unwrap_or(QueryOptions::default()),
            )?)
        }
    }
}

pub fn query_sudo_params(deps: Deps) -> StdResult<Config<Addr>> {
    SUDO_PARAMS.load(deps.storage)
}

pub fn query_price_range(deps: Deps, denom: Denom) -> StdResult<PriceRange> {
    PRICE_RANGES.load(deps.storage, denom)
}

pub fn query_price_ranges(
    deps: Deps,
    query_options: QueryOptions<String>,
) -> StdResult<Vec<(Denom, PriceRange)>> {
    let QueryOptionsInternal {
        limit,
        order,
        min,
        max,
    } = query_options.unpack(&(|offset| offset.to_string()), None, None);

    let denom_price_ranges = PRICE_RANGES
        .range(deps.storage, min, max, order)
        .take(limit)
        .collect::<StdResult<Vec<(Denom, PriceRange)>>>()?;

    Ok(denom_price_ranges)
}

pub fn query_ask(deps: Deps, collection: Addr, token_id: TokenId) -> StdResult<Option<Ask>> {
    let ask = asks().may_load(deps.storage, Ask::build_key(&collection, &token_id))?;
    Ok(ask)
}

pub fn query_asks_by_collection(
    deps: Deps,
    collection: Addr,
    query_options: QueryOptions<AsksByCollectionOffset>,
) -> StdResult<Vec<Ask>> {
    let QueryOptionsInternal {
        limit,
        order,
        min,
        max,
    } = query_options.unpack(&(|offset| offset.token_id.to_string()), None, None);

    let asks = asks()
        .prefix(collection.to_string())
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
    query_options: QueryOptions<AsksByPriceOffset>,
) -> StdResult<Vec<Ask>> {
    let QueryOptionsInternal {
        limit,
        order,
        min,
        max,
    } = query_options.unpack(
        &(|offset| {
            (
                offset.amount,
                (collection.to_string(), offset.token_id.clone()),
            )
        }),
        None,
        None,
    );

    let results = asks()
        .idx
        .collection_denom_price
        .sub_prefix((collection, denom))
        .range_raw(deps.storage, min, max, order)
        .take(limit)
        .map(|res| res.map(|(_, ask)| ask))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(results)
}

pub fn query_asks_by_creator(
    deps: Deps,
    creator: Addr,
    query_options: QueryOptions<AsksByCreatorOffset>,
) -> StdResult<Vec<Ask>> {
    let QueryOptionsInternal {
        limit,
        order,
        min,
        max,
    } = query_options.unpack(
        &(|offset| (offset.collection.clone(), offset.token_id.clone())),
        None,
        None,
    );

    let asks = asks()
        .idx
        .creator
        .prefix(creator)
        .range(deps.storage, min, max, order)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(asks)
}

pub fn query_asks_by_expiration(
    deps: Deps,
    query_options: QueryOptions<AsksByExpirationOffset>,
) -> StdResult<Vec<Ask>> {
    let QueryOptionsInternal {
        limit,
        order,
        min,
        max,
    } = query_options.unpack(
        &(|offset| {
            (
                offset.expiration,
                (offset.collection.clone(), offset.token_id.clone()),
            )
        }),
        None,
        None,
    );

    let max = Some(max.unwrap_or(Bound::exclusive((
        u64::MAX,
        ("".to_string(), "".to_string()),
    ))));

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
    query_options: QueryOptions<OffersByCollectionOffset>,
) -> StdResult<Vec<Offer>> {
    let QueryOptionsInternal {
        limit,
        order,
        min,
        max,
    } = query_options.unpack(
        &(|offset| {
            (
                offset.token_id.clone(),
                Addr::unchecked(offset.creator.clone()),
            )
        }),
        None,
        None,
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
    query_options: QueryOptions<OffersByTokenPriceOffset>,
) -> StdResult<Vec<Offer>> {
    let QueryOptionsInternal {
        limit,
        order,
        min,
        max,
    } = query_options.unpack(
        &(|offset| {
            (
                offset.amount,
                (
                    collection.clone(),
                    token_id.clone(),
                    Addr::unchecked(offset.creator.clone()),
                ),
            )
        }),
        None,
        None,
    );

    let collection_token_index_str =
        build_collection_token_index_str(collection.as_ref(), &token_id);

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

pub fn query_offers_by_creator(
    deps: Deps,
    creator: Addr,
    query_options: QueryOptions<OffersByCreatorOffset>,
) -> StdResult<Vec<Offer>> {
    let QueryOptionsInternal {
        limit,
        order,
        min,
        max,
    } = query_options.unpack(
        &(|offset| {
            (
                Addr::unchecked(offset.collection.clone()),
                offset.token_id.clone(),
                creator.clone(),
            )
        }),
        None,
        None,
    );

    let offers = offers()
        .idx
        .creator
        .prefix(creator)
        .range(deps.storage, min, max, order)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(offers)
}

pub fn query_offers_by_expiration(
    deps: Deps,
    query_options: QueryOptions<OffersByExpirationOffset>,
) -> StdResult<Vec<Offer>> {
    let QueryOptionsInternal {
        limit,
        order,
        min,
        max,
    } = query_options.unpack(
        &(|offset| {
            (
                offset.expiration,
                (
                    Addr::unchecked(offset.collection.clone()),
                    offset.token_id.clone(),
                    Addr::unchecked(offset.creator.clone()),
                ),
            )
        }),
        None,
        None,
    );

    let max = Some(max.unwrap_or(Bound::exclusive((
        u64::MAX,
        (Addr::unchecked(""), "".to_string(), Addr::unchecked("")),
    ))));

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

pub fn query_collection_offers_by_collection(
    deps: Deps,
    collection: Addr,
    query_options: QueryOptions<CollectionOffersByCollectionOffset>,
) -> StdResult<Vec<CollectionOffer>> {
    let QueryOptionsInternal {
        limit,
        order,
        min,
        max,
    } = query_options.unpack(
        &(|offset| Addr::unchecked(offset.creator.clone())),
        None,
        None,
    );

    let offers = collection_offers()
        .prefix(collection)
        .range(deps.storage, min, max, order)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(offers)
}

pub fn query_collection_offers_by_price(
    deps: Deps,
    collection: Addr,
    denom: Denom,
    query_options: QueryOptions<CollectionOffersByPriceOffset>,
) -> StdResult<Vec<CollectionOffer>> {
    let QueryOptionsInternal {
        limit,
        order,
        min,
        max,
    } = query_options.unpack(
        &(|offset| {
            (
                offset.amount,
                (collection.clone(), Addr::unchecked(offset.creator.clone())),
            )
        }),
        None,
        None,
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

pub fn query_collection_offers_by_creator(
    deps: Deps,
    creator: Addr,
    query_options: QueryOptions<CollectionOffersByCreatorOffset>,
) -> StdResult<Vec<CollectionOffer>> {
    let QueryOptionsInternal {
        limit,
        order,
        min,
        max,
    } = query_options.unpack(
        &(|offset| (Addr::unchecked(offset.collection.clone()), creator.clone())),
        None,
        None,
    );

    let offers = collection_offers()
        .idx
        .creator
        .prefix(creator)
        .range(deps.storage, min, max, order)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(offers)
}

pub fn query_collection_offers_by_expiration(
    deps: Deps,
    query_options: QueryOptions<CollectionOffersByExpirationOffset>,
) -> StdResult<Vec<CollectionOffer>> {
    let QueryOptionsInternal {
        limit,
        order,
        min,
        max,
    } = query_options.unpack(
        &(|offset| {
            (
                offset.expiration,
                (
                    Addr::unchecked(offset.collection.clone()),
                    Addr::unchecked(offset.creator.clone()),
                ),
            )
        }),
        None,
        None,
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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::state::OrderInfo;

    use cosmwasm_std::{coin, testing::mock_dependencies};
    use sg_index_query::QueryBound;
    use sg_std::NATIVE_DENOM;

    #[test]
    fn test_query_offers_by_token_price() {
        let mut deps = mock_dependencies();
        // let mut env = mock_env();

        let creator = Addr::unchecked("creator");
        let collection = Addr::unchecked("collection");
        let token_id = "1".to_string();

        let offer = Offer {
            collection: collection.clone(),
            token_id: token_id.to_string(),
            order_info: OrderInfo {
                creator: creator.clone(),
                price: coin(100, NATIVE_DENOM),
                asset_recipient: None,
                finders_fee_bps: None,
                expiration_info: None,
            },
        };
        offer.save(&mut deps.storage).unwrap();

        let result = query_offers_by_token_price(
            deps.as_ref(),
            collection.clone(),
            token_id.clone(),
            NATIVE_DENOM.to_string(),
            QueryOptions {
                limit: Some(1),
                descending: Some(true),
                min: Some(QueryBound::Inclusive(OffersByTokenPriceOffset {
                    creator: "".to_string(),
                    amount: 100u128,
                })),
                max: None,
            },
        )
        .unwrap();
        assert!(result.len() == 1);

        let result = query_offers_by_token_price(
            deps.as_ref(),
            collection,
            token_id,
            NATIVE_DENOM.to_string(),
            QueryOptions {
                limit: Some(1),
                descending: Some(true),
                min: Some(QueryBound::Exclusive(OffersByTokenPriceOffset {
                    creator: "".to_string(),
                    amount: 101u128,
                })),
                max: None,
            },
        )
        .unwrap();
        assert!(result.is_empty());
    }
}
