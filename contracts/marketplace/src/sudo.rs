use cosmwasm_std::{coin, Addr, BankMsg, Coin, DepsMut, Env, Event};
use cw_utils::maybe_addr;
use sg_marketplace_common::{address::map_validate, coin::bps_to_decimal, query::QueryOptions};
use sg_std::Response;
use stargaze_fair_burn::append_fair_burn_msg;
use std::collections::BTreeMap;

use crate::{
    helpers::ExpiryRange,
    hooks::{prepare_ask_hook, prepare_collection_offer_hook, prepare_offer_hook},
    msg::{HookAction, SudoMsg},
    query::{
        query_asks_by_expiration, query_collection_offers_by_expiration, query_offers_by_expiration,
    },
    state::{
        asks, collection_offers, offers, Denom, PriceRange, ASK_HOOKS, OFFER_HOOKS, PRICE_RANGES,
        SALE_HOOKS, SUDO_PARAMS,
    },
    ContractError,
};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(deps: DepsMut, env: Env, msg: SudoMsg) -> Result<Response, ContractError> {
    let api = deps.api;

    match msg {
        SudoMsg::BeginBlock {} => sudo_begin_block(deps, env),
        SudoMsg::EndBlock {} => sudo_end_block(deps, env),
        SudoMsg::UpdateParams {
            fair_burn,
            listing_fee,
            ask_expiry,
            offer_expiry,
            operators,
            max_asks_removed_per_block,
            max_offers_removed_per_block,
            max_collection_offers_removed_per_block,
            trading_fee_bps,
            max_finders_fee_bps,
            removal_reward_bps,
        } => sudo_update_params(
            deps,
            env,
            maybe_addr(api, fair_burn)?,
            listing_fee,
            ask_expiry,
            offer_expiry,
            operators,
            max_asks_removed_per_block,
            max_offers_removed_per_block,
            max_collection_offers_removed_per_block,
            trading_fee_bps,
            max_finders_fee_bps,
            removal_reward_bps,
        ),
        SudoMsg::AddDenoms { price_ranges } => sudo_add_denoms(deps, price_ranges),
        SudoMsg::RemoveDenoms { denoms } => sudo_remove_denoms(deps, denoms),
        SudoMsg::AddSaleHook { hook } => sudo_add_sale_hook(deps, api.addr_validate(&hook)?),
        SudoMsg::AddAskHook { hook } => sudo_add_ask_hook(deps, env, api.addr_validate(&hook)?),
        SudoMsg::AddOfferHook { hook } => sudo_add_offer_hook(deps, env, api.addr_validate(&hook)?),
        SudoMsg::RemoveSaleHook { hook } => sudo_remove_sale_hook(deps, api.addr_validate(&hook)?),
        SudoMsg::RemoveAskHook { hook } => sudo_remove_ask_hook(deps, api.addr_validate(&hook)?),
        SudoMsg::RemoveOfferHook { hook } => {
            sudo_remove_offer_hook(deps, api.addr_validate(&hook)?)
        }
    }
}

pub fn sudo_begin_block(_deps: DepsMut, _env: Env) -> Result<Response, ContractError> {
    Ok(Response::new())
}

pub fn sudo_end_block(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let sudo_params = SUDO_PARAMS.load(deps.storage)?;

    let mut response = Response::new();
    let mut payout_map: BTreeMap<String, Vec<Coin>> = BTreeMap::new();
    let fair_burn_key = "fair-burn";

    // Remove expired asks
    let mut event = Event::new("remove-exipired-asks");
    let expired_asks = query_asks_by_expiration(
        deps.as_ref(),
        QueryOptions {
            descending: Some(false),
            start_after: Some((env.block.time.seconds() + 1, "".to_string(), "".to_string())),
            limit: Some(sudo_params.max_asks_removed_per_block),
        },
    )?;

    for ask in expired_asks {
        asks().remove(deps.storage, ask.key())?;

        if let Some(paid_removal_fee) = &ask.paid_removal_fee {
            payout_map
                .entry(fair_burn_key.to_string())
                .or_insert(vec![])
                .push(paid_removal_fee.clone());
        }

        response =
            response.add_submessages(prepare_ask_hook(deps.as_ref(), &ask, HookAction::Delete)?);
        event = event.add_attribute("ask", ask.key_to_str());
    }

    // Remove expired offers
    let mut event = Event::new("remove-exipired-offers");
    let expired_offers = query_offers_by_expiration(
        deps.as_ref(),
        QueryOptions {
            descending: Some(false),
            start_after: Some((
                env.block.time.seconds() + 1,
                "".to_string(),
                "".to_string(),
                "".to_string(),
            )),
            limit: Some(sudo_params.max_offers_removed_per_block),
        },
    )?;

    for offer in expired_offers {
        offers().remove(deps.storage, offer.key())?;

        let denom = offer.price.denom.clone();

        // Calculate removal fee and refund
        let removal_fee = offer
            .price
            .amount
            .mul_ceil(sudo_params.removal_reward_percent);
        payout_map
            .entry(fair_burn_key.to_string())
            .or_insert(vec![])
            .push(coin(removal_fee.u128(), &denom));

        let bidder_refund = offer.price.amount - removal_fee;
        payout_map
            .entry(offer.bidder.to_string())
            .or_insert(vec![])
            .push(coin(bidder_refund.u128(), &denom));

        response = response.add_submessages(prepare_offer_hook(
            deps.as_ref(),
            &offer,
            HookAction::Delete,
        )?);
        event = event.add_attribute("offer", offer.key_to_str());
    }

    // Remove expired collection offers
    let mut event = Event::new("remove-exipired-collection-offers");
    let expired_collection_offers = query_collection_offers_by_expiration(
        deps.as_ref(),
        QueryOptions {
            descending: Some(false),
            start_after: Some((env.block.time.seconds() + 1, "".to_string(), "".to_string())),
            limit: Some(sudo_params.max_offers_removed_per_block),
        },
    )?;

    for collection_offer in expired_collection_offers {
        collection_offers().remove(deps.storage, collection_offer.key())?;

        let denom = collection_offer.price.denom.clone();

        // Calculate removal fee and refund
        let removal_fee = collection_offer
            .price
            .amount
            .mul_ceil(sudo_params.removal_reward_percent);
        payout_map
            .entry(fair_burn_key.to_string())
            .or_insert(vec![])
            .push(coin(removal_fee.u128(), &denom));

        let bidder_refund = collection_offer.price.amount - removal_fee;
        payout_map
            .entry(collection_offer.bidder.to_string())
            .or_insert(vec![])
            .push(coin(bidder_refund.u128(), &denom));

        response = response.add_submessages(prepare_collection_offer_hook(
            deps.as_ref(),
            &collection_offer,
            HookAction::Delete,
        )?);
        event = event.add_attribute("collection-offer", collection_offer.key_to_str());
    }

    // Transfer all funds
    for (recipient, funds) in payout_map {
        if recipient == fair_burn_key {
            response = append_fair_burn_msg(&sudo_params.fair_burn, funds, None, response);
        } else {
            response = response.add_message(BankMsg::Send {
                to_address: recipient,
                amount: funds,
            });
        }
    }

    Ok(response)
}

/// Only governance can update contract params
pub fn sudo_update_params(
    deps: DepsMut,
    _env: Env,
    fair_burn: Option<Addr>,
    listing_fee: Option<Coin>,
    ask_expiry: Option<ExpiryRange>,
    offer_expiry: Option<ExpiryRange>,
    operators: Option<Vec<String>>,
    max_asks_removed_per_block: Option<u32>,
    max_offers_removed_per_block: Option<u32>,
    max_collection_offers_removed_per_block: Option<u32>,
    trading_fee_bps: Option<u64>,
    max_finders_fee_bps: Option<u64>,
    removal_reward_bps: Option<u64>,
) -> Result<Response, ContractError> {
    let mut sudo_params = SUDO_PARAMS.load(deps.storage)?;
    let mut event = Event::new("sudo-update-params");

    if let Some(fair_burn) = fair_burn {
        event = event.add_attribute("fair_burn", fair_burn.to_string());
        sudo_params.fair_burn = fair_burn;
    }

    if let Some(listing_fee) = listing_fee {
        event = event.add_attribute("listing_fee", listing_fee.to_string());
        sudo_params.listing_fee = listing_fee;
    }

    if let Some(ask_expiry) = ask_expiry {
        ask_expiry.validate()?;
        event = event.add_attribute("ask_expiry", ask_expiry.to_string());
        sudo_params.ask_expiry = ask_expiry;
    }

    if let Some(offer_expiry) = offer_expiry {
        offer_expiry.validate()?;
        event = event.add_attribute("offer_expiry", offer_expiry.to_string());
        sudo_params.offer_expiry = offer_expiry;
    }

    if let Some(operators) = operators {
        event = event.add_attribute("operators", operators.join(","));
        sudo_params.operators = map_validate(deps.api, operators.as_slice())?;
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

    if let Some(trading_fee_bps) = trading_fee_bps {
        event = event.add_attribute(
            "trading_fee_percent",
            sudo_params.trading_fee_percent.to_string(),
        );
        sudo_params.trading_fee_percent = bps_to_decimal(trading_fee_bps);
    }

    if let Some(max_finders_fee_bps) = max_finders_fee_bps {
        event = event.add_attribute(
            "max_finders_fee_percent",
            sudo_params.max_finders_fee_percent.to_string(),
        );
        sudo_params.max_finders_fee_percent = bps_to_decimal(max_finders_fee_bps);
    }

    if let Some(removal_reward_bps) = removal_reward_bps {
        event = event.add_attribute(
            "removal_reward_percent",
            sudo_params.removal_reward_percent.to_string(),
        );
        sudo_params.removal_reward_percent = bps_to_decimal(removal_reward_bps);
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

pub fn sudo_add_sale_hook(deps: DepsMut, hook: Addr) -> Result<Response, ContractError> {
    SALE_HOOKS.add_hook(deps.storage, hook.clone())?;

    let res = Response::new()
        .add_attribute("action", "add_sale_hook")
        .add_attribute("hook", hook);
    Ok(res)
}

pub fn sudo_add_ask_hook(deps: DepsMut, _env: Env, hook: Addr) -> Result<Response, ContractError> {
    ASK_HOOKS.add_hook(deps.storage, hook.clone())?;

    let res = Response::new()
        .add_attribute("action", "add_ask_hook")
        .add_attribute("hook", hook);
    Ok(res)
}

pub fn sudo_add_offer_hook(
    deps: DepsMut,
    _env: Env,
    hook: Addr,
) -> Result<Response, ContractError> {
    OFFER_HOOKS.add_hook(deps.storage, hook.clone())?;

    let res = Response::new()
        .add_attribute("action", "add_bid_hook")
        .add_attribute("hook", hook);
    Ok(res)
}

pub fn sudo_remove_sale_hook(deps: DepsMut, hook: Addr) -> Result<Response, ContractError> {
    SALE_HOOKS.remove_hook(deps.storage, hook.clone())?;

    let res = Response::new()
        .add_attribute("action", "remove_sale_hook")
        .add_attribute("hook", hook);
    Ok(res)
}

pub fn sudo_remove_ask_hook(deps: DepsMut, hook: Addr) -> Result<Response, ContractError> {
    ASK_HOOKS.remove_hook(deps.storage, hook.clone())?;

    let res = Response::new()
        .add_attribute("action", "remove_ask_hook")
        .add_attribute("hook", hook);
    Ok(res)
}

pub fn sudo_remove_offer_hook(deps: DepsMut, hook: Addr) -> Result<Response, ContractError> {
    OFFER_HOOKS.remove_hook(deps.storage, hook.clone())?;

    let res = Response::new()
        .add_attribute("action", "remove_bid_hook")
        .add_attribute("hook", hook);
    Ok(res)
}
