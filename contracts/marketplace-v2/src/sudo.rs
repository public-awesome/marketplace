use crate::{
    msg::{
        AsksByExpirationOffset, CollectionOffersByExpirationOffset, OffersByExpirationOffset,
        SudoMsg,
    },
    orders::RewardPayout,
    query::{
        query_asks_by_expiration, query_collection_offers_by_expiration, query_offers_by_expiration,
    },
    state::{Denom, PriceRange, PRICE_RANGES, SUDO_PARAMS},
    ContractError,
};

use cosmwasm_std::{Coin, DepsMut, Env, Event};
use cw_utils::NativeBalance;
use sg_index_query::{QueryBound, QueryOptions};
use sg_std::Response;
use stargaze_fair_burn::append_fair_burn_msg;
use std::ops::AddAssign;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(deps: DepsMut, env: Env, msg: SudoMsg) -> Result<Response, ContractError> {
    let _api = deps.api;

    match msg {
        SudoMsg::BeginBlock {} => sudo_begin_block(deps, env),
        SudoMsg::EndBlock {} => sudo_end_block(deps, env),
        SudoMsg::UpdateParams {
            fair_burn,
            listing_fee,
            min_removal_reward,
            trading_fee_bps,
            max_royalty_fee_bps,
            max_finders_fee_bps,
            min_expiration_seconds,
            max_asks_removed_per_block,
            max_offers_removed_per_block,
            max_collection_offers_removed_per_block,
        } => sudo_update_params(
            deps,
            env,
            fair_burn,
            listing_fee,
            min_removal_reward,
            trading_fee_bps,
            max_royalty_fee_bps,
            max_finders_fee_bps,
            min_expiration_seconds,
            max_asks_removed_per_block,
            max_offers_removed_per_block,
            max_collection_offers_removed_per_block,
        ),
        SudoMsg::AddDenoms { price_ranges } => sudo_add_denoms(deps, price_ranges),
        SudoMsg::RemoveDenoms { denoms } => sudo_remove_denoms(deps, denoms),
    }
}

pub fn sudo_begin_block(_deps: DepsMut, _env: Env) -> Result<Response, ContractError> {
    Ok(Response::new())
}

pub fn sudo_end_block(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let sudo_params = SUDO_PARAMS.load(deps.storage)?;

    let mut response = Response::new();
    let mut funds_fair_burn = NativeBalance::default();

    // Fetch asks to remove
    let expired_asks = query_asks_by_expiration(
        deps.as_ref(),
        QueryOptions {
            descending: Some(false),
            limit: Some(sudo_params.max_asks_removed_per_block),
            min: None,
            max: Some(QueryBound::Exclusive(AsksByExpirationOffset {
                expiration: env
                    .block
                    .time
                    .plus_seconds(sudo_params.order_removal_lookahead_secs)
                    .plus_seconds(1)
                    .seconds(),
                collection: "".to_string(),
                token_id: "".to_string(),
            })),
        },
    )?;

    for ask in expired_asks {
        if let Some(expiration_info) = &ask.order_info.expiration_info {
            funds_fair_burn.add_assign(expiration_info.removal_reward.clone());
        }

        response = ask.remove(deps.storage, true, RewardPayout::Contract, response)?;
    }

    // Remove expired offers
    let expired_offers = query_offers_by_expiration(
        deps.as_ref(),
        QueryOptions {
            descending: Some(false),
            limit: Some(sudo_params.max_offers_removed_per_block),
            min: None,
            max: Some(QueryBound::Exclusive(OffersByExpirationOffset {
                expiration: env
                    .block
                    .time
                    .plus_seconds(sudo_params.order_removal_lookahead_secs)
                    .plus_seconds(1)
                    .seconds(),
                collection: "".to_string(),
                token_id: "".to_string(),
                creator: "".to_string(),
            })),
        },
    )?;

    for offer in expired_offers {
        if let Some(expiration_info) = &offer.order_info.expiration_info {
            funds_fair_burn.add_assign(expiration_info.removal_reward.clone());
        }

        response = offer.remove(deps.storage, true, RewardPayout::Contract, response)?;
    }

    // Remove expired collection offers
    let expired_collection_offers = query_collection_offers_by_expiration(
        deps.as_ref(),
        QueryOptions {
            descending: Some(false),
            limit: Some(sudo_params.max_offers_removed_per_block),
            min: None,
            max: Some(QueryBound::Exclusive(CollectionOffersByExpirationOffset {
                expiration: env
                    .block
                    .time
                    .plus_seconds(sudo_params.order_removal_lookahead_secs)
                    .plus_seconds(1)
                    .seconds(),
                collection: "".to_string(),
                creator: "".to_string(),
            })),
        },
    )?;

    for collection_offer in expired_collection_offers {
        if let Some(expiration_info) = &collection_offer.order_info.expiration_info {
            funds_fair_burn.add_assign(expiration_info.removal_reward.clone());
        }

        response = collection_offer.remove(deps.storage, true, RewardPayout::Contract, response)?;
    }

    funds_fair_burn.normalize();
    if !funds_fair_burn.is_empty() {
        response = append_fair_burn_msg(
            &sudo_params.fair_burn,
            funds_fair_burn.into_vec(),
            None,
            response,
        );
    }

    Ok(response)
}

/// Only governance can update contract params
pub fn sudo_update_params(
    deps: DepsMut,
    _env: Env,
    fair_burn: Option<String>,
    listing_fee: Option<Coin>,
    min_removal_reward: Option<Coin>,
    trading_fee_bps: Option<u64>,
    max_royalty_fee_bps: Option<u64>,
    max_finders_fee_bps: Option<u64>,
    min_expiration_seconds: Option<u64>,
    max_asks_removed_per_block: Option<u32>,
    max_offers_removed_per_block: Option<u32>,
    max_collection_offers_removed_per_block: Option<u32>,
) -> Result<Response, ContractError> {
    let mut sudo_params = SUDO_PARAMS.load(deps.storage)?;
    let mut event = Event::new("sudo-update-params");

    if let Some(fair_burn) = fair_burn {
        event = event.add_attribute("fair_burn", fair_burn.to_string());
        sudo_params.fair_burn = deps.api.addr_validate(&fair_burn)?;
    }

    if let Some(listing_fee) = listing_fee {
        event = event.add_attribute("listing_fee", listing_fee.to_string());
        sudo_params.listing_fee = listing_fee;
    }

    if let Some(min_removal_reward) = min_removal_reward {
        event = event.add_attribute("min_removal_reward", min_removal_reward.to_string());
        sudo_params.min_removal_reward = min_removal_reward;
    }

    if let Some(trading_fee_bps) = trading_fee_bps {
        event = event.add_attribute("trading_fee_bps", sudo_params.trading_fee_bps.to_string());
        sudo_params.trading_fee_bps = trading_fee_bps;
    }

    if let Some(max_royalty_fee_bps) = max_royalty_fee_bps {
        event = event.add_attribute(
            "max_royalty_fee_bps",
            sudo_params.max_royalty_fee_bps.to_string(),
        );
        sudo_params.max_royalty_fee_bps = max_royalty_fee_bps;
    }

    if let Some(max_finders_fee_bps) = max_finders_fee_bps {
        event = event.add_attribute(
            "max_finders_fee_bps",
            sudo_params.max_finders_fee_bps.to_string(),
        );
        sudo_params.max_finders_fee_bps = max_finders_fee_bps;
    }

    if let Some(min_expiration_seconds) = min_expiration_seconds {
        event = event.add_attribute(
            "min_expiration_seconds",
            sudo_params.min_expiration_seconds.to_string(),
        );
        sudo_params.min_expiration_seconds = min_expiration_seconds;
    }

    if let Some(max_asks_removed_per_block) = max_asks_removed_per_block {
        event = event.add_attribute(
            "max_asks_removed_per_block",
            max_asks_removed_per_block.to_string(),
        );
        sudo_params.max_asks_removed_per_block = max_asks_removed_per_block;
    }

    if let Some(max_offers_removed_per_block) = max_offers_removed_per_block {
        event = event.add_attribute(
            "max_offers_removed_per_block",
            max_offers_removed_per_block.to_string(),
        );
        sudo_params.max_offers_removed_per_block = max_offers_removed_per_block;
    }

    if let Some(max_collection_offers_removed_per_block) = max_collection_offers_removed_per_block {
        event = event.add_attribute(
            "max_collection_offers_removed_per_block",
            max_collection_offers_removed_per_block.to_string(),
        );
        sudo_params.max_collection_offers_removed_per_block =
            max_collection_offers_removed_per_block;
    }

    sudo_params.validate()?;
    sudo_params.save(deps.storage)?;

    Ok(Response::new().add_event(event))
}

pub fn sudo_add_denoms(
    deps: DepsMut,
    price_ranges: Vec<(Denom, PriceRange)>,
) -> Result<Response, ContractError> {
    let mut event = Event::new("sudo-add-denoms");

    for (denom, price_range) in price_ranges {
        PRICE_RANGES.save(deps.storage, denom.clone(), &price_range)?;
        event = event.add_attribute(denom, price_range.to_string());
    }

    Ok(Response::new().add_event(event))
}

pub fn sudo_remove_denoms(deps: DepsMut, denoms: Vec<Denom>) -> Result<Response, ContractError> {
    let mut event = Event::new("sudo-remove-denoms");

    for denom in denoms {
        PRICE_RANGES.remove(deps.storage, denom.clone());
        event = event.add_attribute("denom", denom);
    }

    Ok(Response::new().add_event(event))
}
