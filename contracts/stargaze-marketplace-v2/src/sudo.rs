use cosmwasm_std::{ensure, DepsMut, Env, Response};
use cw_utils::NativeBalance;
use sg_index_query::{QueryBound, QueryOptions};
use sg_marketplace_common::coin::transfer_coins;
use std::ops::Add;

use crate::{
    events::{AskEvent, BidEvent, CollectionBidEvent},
    msg::SudoMsg,
    query::{
        query_asks_by_expiry_timestamp, query_bids_by_expiry_timestamp,
        query_collection_bids_by_expiry_timestamp,
    },
    state::{asks, bids, collection_bids, CONFIG},
    ContractError,
};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(deps: DepsMut, env: Env, msg: SudoMsg) -> Result<Response, ContractError> {
    match msg {
        SudoMsg::BeginBlock {} => sudo_begin_block(deps, env),
        SudoMsg::EndBlock {} => sudo_end_block(deps, env),
    }
}

pub fn sudo_begin_block(_deps: DepsMut, _env: Env) -> Result<Response, ContractError> {
    Ok(Response::new())
}

pub fn sudo_end_block(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let mut accrued_fees = NativeBalance(vec![]);

    let mut response = Response::new();

    // Remove expired asks
    let expired_asks = query_asks_by_expiry_timestamp(
        deps.as_ref(),
        QueryOptions {
            limit: Some(config.max_asks_removed_per_block),
            descending: Some(false),
            min: None,
            max: Some(QueryBound::Exclusive((
                env.block.time.seconds() + 1,
                "".to_string(),
            ))),
        },
    )?;

    for ask in expired_asks {
        response = response.add_event(
            AskEvent {
                ty: "remove-ask",
                ask: &ask,
                attr_keys: vec!["id", "collection", "token_id"],
            }
            .into(),
        );

        ensure!(
            ask.details.expiry.is_some(),
            ContractError::InternalError("expiry not set".to_string())
        );

        let expiry = ask.details.expiry.unwrap();
        accrued_fees = accrued_fees.add(expiry.reward.clone());

        asks().remove(deps.storage, ask.id)?;
    }

    // Remove expired bids
    let expired_bids = query_bids_by_expiry_timestamp(
        deps.as_ref(),
        QueryOptions {
            limit: Some(config.max_bids_removed_per_block),
            descending: Some(false),
            min: None,
            max: Some(QueryBound::Exclusive((
                env.block.time.seconds() + 1,
                "".to_string(),
            ))),
        },
    )?;

    for bid in expired_bids {
        response = response.add_event(
            BidEvent {
                ty: "remove-bid",
                bid: &bid,
                attr_keys: vec!["id", "collection", "token_id"],
            }
            .into(),
        );

        ensure!(
            bid.details.expiry.is_some(),
            ContractError::InternalError("expiry not set".to_string())
        );

        let expiry = bid.details.expiry.unwrap();
        accrued_fees = accrued_fees.add(expiry.reward.clone());

        bids().remove(deps.storage, bid.id)?;
    }

    // Remove expired collection bids
    let expired_collection_bids = query_collection_bids_by_expiry_timestamp(
        deps.as_ref(),
        QueryOptions {
            limit: Some(config.max_collection_bids_removed_per_block),
            descending: Some(false),
            min: None,
            max: Some(QueryBound::Exclusive((
                env.block.time.seconds() + 1,
                "".to_string(),
            ))),
        },
    )?;

    for collection_bid in expired_collection_bids {
        response = response.add_event(
            CollectionBidEvent {
                ty: "remove-collection-bid",
                collection_bid: &collection_bid,
                attr_keys: vec!["id", "collection"],
            }
            .into(),
        );

        ensure!(
            collection_bid.details.expiry.is_some(),
            ContractError::InternalError("expiry not set".to_string())
        );

        let expiry = collection_bid.details.expiry.unwrap();
        accrued_fees = accrued_fees.add(expiry.reward.clone());

        collection_bids().remove(deps.storage, collection_bid.id)?;
    }

    // Transfer accrued fees to the fee manager
    if !accrued_fees.is_empty() {
        response = transfer_coins(accrued_fees.into_vec(), &config.fee_manager, response);
    }

    Ok(response)
}
