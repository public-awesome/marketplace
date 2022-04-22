use crate::msg::{
    AskCountResponse, AsksResponse, BidResponse, BidsResponse, CollectionsResponse,
    CurrentAskResponse, ParamResponse, QueryMsg,
};
use crate::state::{ask_key, asks, bids, SUDO_PARAMS};
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
        QueryMsg::Params {} => to_binary(&query_params(deps)?),
    }
}

pub fn query_params(deps: Deps) -> StdResult<ParamResponse> {
    let config = SUDO_PARAMS.load(deps.storage)?;

    Ok(ParamResponse { params: config })
}
pub fn query_asks(
    deps: Deps,
    collection: Addr,
    start_after: Option<u32>,
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
    start_after: Option<String>,
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
    token_id: u32,
) -> StdResult<CurrentAskResponse> {
    let ask = asks().may_load(deps.storage, ask_key(collection, token_id))?;

    Ok(CurrentAskResponse { ask })
}

pub fn query_bid(
    deps: Deps,
    collection: Addr,
    token_id: u32,
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
    token_id: u32,
    start_after: Option<String>,
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

#[cfg(test)]
mod tests {
    use std::vec;

    use crate::error::ContractError;
    use crate::execute::{execute, instantiate};
    use crate::msg::{ExecuteMsg, InstantiateMsg};
    use crate::state::{bid_key, Ask, Bid};

    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coin, coins, DepsMut, StdError, Timestamp, Uint128};
    use sg_std::NATIVE_DENOM;

    const CREATOR: &str = "creator";
    const COLLECTION: &str = "collection";
    const TOKEN_ID: u32 = 123;
    // Governance parameters
    const TRADING_FEE_PERCENT: u32 = 2; // 2%
    const MIN_EXPIRY: u64 = 24 * 60 * 60; // 24 hours (in seconds)
    const MAX_EXPIRY: u64 = 180 * 24 * 60 * 60; // 6 months (in seconds)

    #[test]
    fn ask_indexed_map() {
        let mut deps = mock_dependencies();
        let collection = Addr::unchecked(COLLECTION);
        let seller = Addr::unchecked("seller");

        let ask = Ask {
            collection: collection.clone(),
            token_id: TOKEN_ID,
            seller: seller.clone(),
            price: Uint128::from(500u128),
            funds_recipient: None,
            expires: Timestamp::from_seconds(0),
            active: true,
        };
        let key = ask_key(collection.clone(), TOKEN_ID);
        let res = asks().save(deps.as_mut().storage, key.clone(), &ask);
        assert!(res.is_ok());

        let ask2 = Ask {
            collection: collection.clone(),
            token_id: TOKEN_ID + 1,
            seller: seller.clone(),
            price: Uint128::from(500u128),
            funds_recipient: None,
            expires: Timestamp::from_seconds(0),
            active: true,
        };
        let key2 = ask_key(collection.clone(), TOKEN_ID + 1);
        let res = asks().save(deps.as_mut().storage, key2, &ask2);
        assert!(res.is_ok());

        let res = asks().load(deps.as_ref().storage, key);
        assert_eq!(res.unwrap(), ask);

        let res = query_asks_by_seller(deps.as_ref(), seller).unwrap();
        assert_eq!(res.asks.len(), 2);
        assert_eq!(res.asks[0], ask);

        let res = query_ask_count(deps.as_ref(), collection).unwrap();
        assert_eq!(res.count, 2);
    }

    #[test]

    fn bid_indexed_map() {
        let mut deps = mock_dependencies();
        let collection = Addr::unchecked(COLLECTION);
        let bidder = Addr::unchecked("bidder");

        let bid = Bid {
            collection: collection.clone(),
            token_id: TOKEN_ID,
            bidder: bidder.clone(),
            price: Uint128::from(500u128),
            expires: Timestamp::from_seconds(0),
        };
        let key = bid_key(collection.clone(), TOKEN_ID, bidder.clone());
        let res = bids().save(deps.as_mut().storage, key.clone(), &bid);
        assert!(res.is_ok());

        let bid2 = Bid {
            collection: collection.clone(),
            token_id: TOKEN_ID + 1,
            bidder: bidder.clone(),
            price: Uint128::from(500u128),
            expires: Timestamp::from_seconds(0),
        };
        let key2 = bid_key(collection, TOKEN_ID + 1, bidder.clone());
        let res = bids().save(deps.as_mut().storage, key2, &bid2);
        assert!(res.is_ok());

        let res = bids().load(deps.as_ref().storage, key);
        assert_eq!(res.unwrap(), bid);

        let res = query_bids_by_bidder(deps.as_ref(), bidder).unwrap();
        assert_eq!(res.bids.len(), 2);
        assert_eq!(res.bids[0], bid);
    }

    fn setup_contract(deps: DepsMut) {
        let msg = InstantiateMsg {
            operators: vec!["operator".to_string()],
            trading_fee_percent: TRADING_FEE_PERCENT,
            min_expiry: MIN_EXPIRY,
            max_expiry: MAX_EXPIRY,
        };
        let info = mock_info(CREATOR, &[]);
        let res = instantiate(deps, mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            operators: vec!["operator".to_string()],
            trading_fee_percent: TRADING_FEE_PERCENT,
            min_expiry: MIN_EXPIRY,
            max_expiry: MAX_EXPIRY,
        };
        let info = mock_info("creator", &coins(1000, NATIVE_DENOM));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn try_set_bid() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let broke = mock_info("broke", &[]);
        let bidder = mock_info("bidder", &coins(1000, NATIVE_DENOM));

        let set_bid_msg = ExecuteMsg::SetBid {
            collection: COLLECTION.to_string(),
            token_id: TOKEN_ID,
            expires: Timestamp::from_seconds(0),
        };

        // Broke bidder calls Set Bid and gets an error
        let err = execute(deps.as_mut(), mock_env(), broke, set_bid_msg).unwrap_err();
        assert_eq!(
            err,
            ContractError::BidPaymentError(cw_utils::PaymentError::NoFunds {})
        );

        let set_bid_msg = ExecuteMsg::SetBid {
            collection: COLLECTION.to_string(),
            token_id: TOKEN_ID,
            expires: mock_env().block.time.plus_seconds(MIN_EXPIRY + 1),
        };

        // Bidder calls SetBid before an Ask is set, so it should fail
        let err = execute(deps.as_mut(), mock_env(), bidder, set_bid_msg).unwrap_err();
        assert_eq!(
            err,
            ContractError::Std(StdError::NotFound {
                kind: "sg_marketplace::state::Ask".to_string()
            })
        );
    }

    #[test]
    fn try_set_ask() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let set_ask = ExecuteMsg::SetAsk {
            collection: COLLECTION.to_string(),
            token_id: TOKEN_ID,
            price: coin(100, NATIVE_DENOM),
            funds_recipient: None,
            expires: Timestamp::from_seconds(
                mock_env().block.time.plus_seconds(MIN_EXPIRY + 1).seconds(),
            ),
        };

        // Reject if not called by the media owner
        let not_allowed = mock_info("random", &[]);
        let err = execute(deps.as_mut(), mock_env(), not_allowed, set_ask);
        assert!(err.is_err());

        // Reject wrong denom
        let set_bad_ask = ExecuteMsg::SetAsk {
            collection: COLLECTION.to_string(),
            token_id: TOKEN_ID,
            price: coin(100, "osmo".to_string()),
            funds_recipient: None,
            expires: Timestamp::from_seconds(
                mock_env().block.time.plus_seconds(MIN_EXPIRY + 1).seconds(),
            ),
        };
        let err = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("creator", &[]),
            set_bad_ask,
        )
        .unwrap_err();
        assert_eq!(err, ContractError::InvalidPrice {});
    }
}
