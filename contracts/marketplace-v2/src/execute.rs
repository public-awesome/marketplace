use crate::{
    error::ContractError,
    events::AskEvent,
    helpers::{
        finalize_sale, only_order_creator, only_owner_or_seller, reconcile_funds,
        validate_expiration_info, validate_order_options, validate_price,
    },
    msg::{ExecuteMsg, OrderOptions, UpdateVal},
    orders::{MatchingOffer, RewardPayout},
    state::{
        asks, collection_offers, offers, Ask, CollectionOffer, ExpirationInfo, KeyString, Offer,
        TokenId, SUDO_PARAMS,
    },
};

use cosmwasm_std::{ensure, to_json_binary, Addr, Coin, DepsMut, Env, MessageInfo, WasmMsg};
use cw_utils::{nonpayable, NativeBalance};
use sg_marketplace_common::{
    coin::transfer_coin,
    nft::{only_owner, only_tradable, transfer_nft},
};
use sg_std::Response;
use stargaze_fair_burn::msg::ExecuteMsg as FairBurnExecuteMsg;
use std::ops::AddAssign;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let api = deps.api;

    match msg {
        ExecuteMsg::SetAsk {
            collection,
            token_id,
            price,
            order_options,
        } => execute_set_ask(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            token_id,
            price,
            order_options.unwrap_or_default(),
        ),
        ExecuteMsg::UpdateAsk {
            collection,
            token_id,
            asset_recipient,
            finders_fee_bps,
            expiration_info,
        } => execute_update_ask(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            token_id,
            asset_recipient,
            finders_fee_bps,
            expiration_info,
        ),
        ExecuteMsg::AcceptAsk {
            collection,
            token_id,
            order_options,
        } => execute_accept_ask(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            token_id,
            order_options.unwrap_or_default(),
        ),
        ExecuteMsg::RemoveAsk {
            collection,
            token_id,
        } => execute_remove_ask(deps, info, api.addr_validate(&collection)?, token_id),
        ExecuteMsg::RemoveExpiredAsk {
            collection,
            token_id,
        } => execute_remove_expired_ask(deps, env, info, api.addr_validate(&collection)?, token_id),
        ExecuteMsg::SetOffer {
            collection,
            token_id,
            price,
            order_options,
        } => execute_set_offer(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            token_id,
            price,
            order_options.unwrap_or_default(),
        ),
        ExecuteMsg::UpdateOffer {
            collection,
            token_id,
            asset_recipient,
            finders_fee_bps,
            expiration_info,
        } => execute_update_offer(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            token_id,
            asset_recipient,
            finders_fee_bps,
            expiration_info,
        ),
        ExecuteMsg::AcceptOffer {
            collection,
            token_id,
            creator,
            order_options,
        } => execute_accept_offer(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            token_id,
            api.addr_validate(&creator)?,
            order_options.unwrap_or_default(),
        ),
        ExecuteMsg::RemoveOffer {
            collection,
            token_id,
        } => execute_remove_offer(deps, env, info, api.addr_validate(&collection)?, token_id),
        ExecuteMsg::RejectOffer {
            collection,
            token_id,
            creator,
        } => execute_reject_offer(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            token_id,
            api.addr_validate(&creator)?,
        ),
        ExecuteMsg::RemoveExpiredOffer {
            collection,
            token_id,
            creator,
        } => execute_remove_expired_offer(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            token_id,
            api.addr_validate(&creator)?,
        ),
        ExecuteMsg::SetCollectionOffer {
            collection,
            price,
            order_options,
        } => execute_set_collection_offer(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            price,
            order_options.unwrap_or_default(),
        ),
        ExecuteMsg::UpdateCollectionOffer {
            collection,
            asset_recipient,
            finders_fee_bps,
            expiration_info,
        } => execute_update_collection_offer(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            asset_recipient,
            finders_fee_bps,
            expiration_info,
        ),
        ExecuteMsg::AcceptCollectionOffer {
            collection,
            token_id,
            creator,
            order_options,
        } => execute_accept_collection_offer(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            token_id,
            api.addr_validate(&creator)?,
            order_options.unwrap_or_default(),
        ),
        ExecuteMsg::RemoveCollectionOffer { collection } => {
            execute_remove_collection_offer(deps, env, info, api.addr_validate(&collection)?)
        }
        ExecuteMsg::RemoveExpiredCollectionOffer {
            collection,
            creator,
        } => execute_remove_expired_collection_offer(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            api.addr_validate(&creator)?,
        ),
    }
}

/// A creator may set an Ask on their NFT to list it on Marketplace
pub fn execute_set_ask(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
    price: Coin,
    order_options_str: OrderOptions<String>,
) -> Result<Response, ContractError> {
    only_owner(&deps.querier, &info, &collection, &token_id)?;
    only_tradable(&deps.querier, &env.block, &collection)?;
    validate_price(deps.storage, &price)?;

    let sudo_params = SUDO_PARAMS.load(deps.storage)?;
    let order_options =
        validate_order_options(deps.api, &info, &env.block, &sudo_params, order_options_str)?;

    let mut response = Response::new();
    let mut funds_due = NativeBalance(vec![]);

    if !sudo_params.listing_fee.amount.is_zero() {
        funds_due.add_assign(sudo_params.listing_fee.clone());
        response = response.add_message(WasmMsg::Execute {
            contract_addr: sudo_params.fair_burn.to_string(),
            msg: to_json_binary(&FairBurnExecuteMsg::FairBurn { recipient: None })?,
            funds: vec![sudo_params.listing_fee.clone()],
        });
    }

    let mut ask = Ask::new(
        collection.clone(),
        token_id.clone(),
        price,
        info.sender.clone(),
        order_options.asset_recipient,
        order_options.finders_fee_bps,
        None,
    );

    // Ensure the ask does not exist
    let ask_key = ask.key();
    ensure!(
        asks().may_load(deps.storage, ask_key.clone())?.is_none(),
        ContractError::EntityExists(format!("ask {}", &ask_key.to_string()))
    );

    // Check for a matching offer
    let match_result = ask.match_with_offer(deps.as_ref(), &env);

    if let Ok(Some(matching_offer)) = match_result {
        // If a match is found:
        // * finalize the sale
        // * remove the offer
        response = finalize_sale(
            deps.as_ref(),
            &env,
            &ask,
            &matching_offer,
            &sudo_params,
            order_options.finder.as_ref(),
            false,
            response,
        )?;

        match matching_offer {
            MatchingOffer::Offer(offer) => {
                response = offer.remove(deps.storage, false, RewardPayout::Return, response)?;
            }
            MatchingOffer::CollectionOffer(collection_offer) => {
                response =
                    collection_offer.remove(deps.storage, false, RewardPayout::Return, response)?;
            }
        }
    } else {
        // If no match is found, or there is an error in matching, continue creating the ask.
        // Ask creation should:
        // * escrow the nft
        // * take removal fee if applicable
        // * store the ask
        // * emit 'set-ask' event

        // Escrow the NFT
        response = transfer_nft(
            &ask.collection,
            &ask.token_id,
            &env.contract.address,
            response,
        );

        // Take removal fee if expiration is set
        if let Some(expiration_info) = order_options.expiration_info {
            funds_due.add_assign(expiration_info.removal_reward.clone());
            ask.order_info.expiration_info = Some(expiration_info);
        }

        ask.save(deps.storage)?;

        response = response.add_event(
            AskEvent {
                ty: "set-ask",
                ask: &ask,
            }
            .into(),
        )
    }

    response = reconcile_funds(&info, funds_due, response)?;

    Ok(response)
}

/// A creator may update the Ask on their NFT
pub fn execute_update_ask(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
    asset_recipient: Option<UpdateVal<String>>,
    finders_fee_bps: Option<UpdateVal<u64>>,
    expiration_info: Option<UpdateVal<ExpirationInfo>>,
) -> Result<Response, ContractError> {
    let api = deps.api;

    let sudo_params = SUDO_PARAMS.load(deps.storage)?;

    let mut response = Response::new();
    let mut funds_due = NativeBalance(vec![]);

    let ask_key = Ask::build_key(&collection, &token_id);
    let mut ask = asks()
        .load(deps.storage, ask_key.clone())
        .map_err(|e| ContractError::EntityNotFound(e.to_string()))?;

    only_order_creator(&info, &ask.order_info)?;

    // Update asset recipient if set
    match asset_recipient {
        Some(UpdateVal::Set(asset_recipient_val)) => {
            ask.order_info.asset_recipient = Some(api.addr_validate(&asset_recipient_val)?);
        }
        Some(UpdateVal::Unset) => {
            ask.order_info.asset_recipient = None;
        }
        None => {}
    }

    // Update finders fee percent if set
    match finders_fee_bps {
        Some(UpdateVal::Set(finders_fee_bps_val)) => {
            ask.order_info.finders_fee_bps = Some(finders_fee_bps_val);
        }
        Some(UpdateVal::Unset) => {
            ask.order_info.finders_fee_bps = None;
        }
        None => {}
    }

    // Update the expiration on the ask if set
    if let Some(expiration_info_update) = expiration_info {
        // If a removal reward was paid, we need to refund it
        if let Some(old_expiration_info) = ask.order_info.expiration_info {
            if !old_expiration_info.removal_reward.amount.is_zero() {
                response = transfer_coin(
                    old_expiration_info.removal_reward,
                    &ask.order_info.creator,
                    response,
                )
            }
        }

        match expiration_info_update {
            UpdateVal::Set(new_expiration_info) => {
                validate_expiration_info(&env.block, &sudo_params, &new_expiration_info)?;
                funds_due.add_assign(new_expiration_info.removal_reward.clone());
                ask.order_info.expiration_info = Some(new_expiration_info);
            }
            UpdateVal::Unset => {
                ask.order_info.expiration_info = None;
            }
        }
    }

    ask.save(deps.storage)?;

    response = response.add_event(
        AskEvent {
            ty: "update-ask",
            ask: &ask,
        }
        .into(),
    );

    response = reconcile_funds(&info, funds_due, response)?;

    Ok(response)
}

/// Accepts an ask on a listed NFT (ie. Buy Now).
pub fn execute_accept_ask(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
    order_options_str: OrderOptions<String>,
) -> Result<Response, ContractError> {
    only_tradable(&deps.querier, &env.block, &collection)?;

    let sudo_params = SUDO_PARAMS.load(deps.storage)?;
    let order_options =
        validate_order_options(deps.api, &info, &env.block, &sudo_params, order_options_str)?;

    let mut response = Response::new();
    let mut funds_due = NativeBalance(vec![]);

    let ask_key = Ask::build_key(&collection, &token_id);
    let ask = asks()
        .load(deps.storage, ask_key)
        .map_err(|e| ContractError::EntityNotFound(e.to_string()))?;

    funds_due.add_assign(ask.order_info.price.clone());

    let offer = Offer::new(
        collection,
        token_id,
        ask.order_info.price.clone(),
        info.sender.clone(),
        order_options.asset_recipient,
        order_options.finders_fee_bps,
        None,
    );

    response = finalize_sale(
        deps.as_ref(),
        &env,
        &ask,
        &MatchingOffer::Offer(offer.clone()),
        &sudo_params,
        order_options.finder.as_ref(),
        true,
        response,
    )?;

    // Remove the ask
    response = ask.remove(deps.storage, false, RewardPayout::Return, response)?;

    // If there is an existing offer remove it
    let existing_offer_option = offers().may_load(deps.storage, offer.key())?;
    if let Some(existing_offer) = existing_offer_option {
        response = existing_offer.remove(deps.storage, true, RewardPayout::Return, response)?;
    }

    response = reconcile_funds(&info, funds_due, response)?;

    Ok(response)
}

/// Removes the ask on a particular NFT, only the creator can invoke this operation.
pub fn execute_remove_ask(
    deps: DepsMut,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let ask_key = Ask::build_key(&collection, &token_id);
    let ask = asks().load(deps.storage, ask_key)?;

    only_order_creator(&info, &ask.order_info)?;

    let response = ask.remove(deps.storage, true, RewardPayout::Return, Response::new())?;

    Ok(response)
}

/// Operation that anyone can invoke to remove an expired ask on a particular NFT.
pub fn execute_remove_expired_ask(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let ask_key = Ask::build_key(&collection, &token_id);
    let ask = asks().load(deps.storage, ask_key.clone())?;

    ensure!(
        ask.order_info.is_expired(&env.block),
        ContractError::EntityNotExpired(format!("ask {}", ask_key.to_string()))
    );

    let response = ask.remove(
        deps.storage,
        true,
        RewardPayout::Other(info.sender),
        Response::new(),
    )?;

    Ok(response)
}

/// Places an offer on a listed or unlisted NFT. The offer is escrowed in the contract.
pub fn execute_set_offer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
    price: Coin,
    order_options_str: OrderOptions<String>,
) -> Result<Response, ContractError> {
    only_tradable(&deps.querier, &env.block, &collection)?;
    validate_price(deps.storage, &price)?;

    let sudo_params = SUDO_PARAMS.load(deps.storage)?;
    let order_options =
        validate_order_options(deps.api, &info, &env.block, &sudo_params, order_options_str)?;

    let mut response = Response::new();
    let mut funds_due = NativeBalance(vec![]);

    let mut offer = Offer::new(
        collection.clone(),
        token_id.clone(),
        price,
        info.sender.clone(),
        order_options.asset_recipient,
        order_options.finders_fee_bps,
        None,
    );

    // Ensure the offer does not exist
    ensure!(
        offers().may_load(deps.storage, offer.key())?.is_none(),
        ContractError::EntityExists(format!("offer {}", offer.key().to_string()))
    );

    let matching_ask = offer.match_with_ask(deps.as_ref(), &env)?;

    if let Some(ask) = matching_ask {
        // If a matching ask is found:
        // * ensure at least ask price is paid
        // * perform the sale
        // * remove the ask
        funds_due.add_assign(ask.order_info.price.clone());

        response = finalize_sale(
            deps.as_ref(),
            &env,
            &ask,
            &MatchingOffer::Offer(offer),
            &sudo_params,
            order_options.finder.as_ref(),
            true,
            response,
        )?;

        response = ask.remove(deps.storage, false, RewardPayout::Return, response)?;
    } else {
        // If no match is found. Offer creation should:
        // * ensure offer price is paid
        // * take removal fee if applicable
        // * store the offer
        // * emit event and hook
        funds_due.add_assign(offer.order_info.price.clone());

        if let Some(expiration_info) = order_options.expiration_info {
            funds_due.add_assign(expiration_info.removal_reward.clone());
            offer.order_info.expiration_info = Some(expiration_info);
        }

        offers().save(deps.storage, offer.key(), &offer)?;
    }

    response = reconcile_funds(&info, funds_due, response)?;

    Ok(response)
}

/// A buyer may update their offer
pub fn execute_update_offer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
    asset_recipient: Option<UpdateVal<String>>,
    finders_fee_bps: Option<UpdateVal<u64>>,
    expiration_info: Option<UpdateVal<ExpirationInfo>>,
) -> Result<Response, ContractError> {
    let api = deps.api;
    let sudo_params = SUDO_PARAMS.load(deps.storage)?;

    let mut response = Response::new();
    let mut funds_due = NativeBalance(vec![]);

    let offer_key = Offer::build_key(&collection, &token_id, &info.sender);
    let mut offer = offers()
        .load(deps.storage, offer_key.clone())
        .map_err(|e| ContractError::EntityNotFound(e.to_string()))?;

    // Update asset recipient if set
    match asset_recipient {
        Some(UpdateVal::Set(asset_recipient_val)) => {
            offer.order_info.asset_recipient = Some(api.addr_validate(&asset_recipient_val)?);
        }
        Some(UpdateVal::Unset) => {
            offer.order_info.asset_recipient = None;
        }
        None => {}
    }

    // Update finders fee percent if set
    match finders_fee_bps {
        Some(UpdateVal::Set(finders_fee_bps_val)) => {
            offer.order_info.finders_fee_bps = Some(finders_fee_bps_val);
        }
        Some(UpdateVal::Unset) => {
            offer.order_info.finders_fee_bps = None;
        }
        None => {}
    }

    // Update the expiration on the ask if set
    if let Some(expiration_info_update) = expiration_info {
        // If a removal reward was paid, we need to refund it
        if let Some(old_expiration_info) = offer.order_info.expiration_info {
            if !old_expiration_info.removal_reward.amount.is_zero() {
                response = transfer_coin(
                    old_expiration_info.removal_reward,
                    &offer.order_info.creator,
                    response,
                )
            }
        }

        match expiration_info_update {
            UpdateVal::Set(new_expiration_info) => {
                validate_expiration_info(&env.block, &sudo_params, &new_expiration_info)?;
                funds_due.add_assign(new_expiration_info.removal_reward.clone());
                offer.order_info.expiration_info = Some(new_expiration_info);
            }
            UpdateVal::Unset => {
                offer.order_info.expiration_info = None;
            }
        }
    }

    offer.save(deps.storage)?;

    response = reconcile_funds(&info, funds_due, response)?;

    Ok(response)
}

/// Creator can accept an offer, whether the token is listed for sale or not.
pub fn execute_accept_offer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
    creator: Addr,
    order_options_str: OrderOptions<String>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_tradable(&deps.querier, &env.block, &collection)?;
    only_owner_or_seller(deps.as_ref(), &env, &info, &collection, &token_id)?;

    let sudo_params = SUDO_PARAMS.load(deps.storage)?;
    let order_options =
        validate_order_options(deps.api, &info, &env.block, &sudo_params, order_options_str)?;

    let mut response = Response::new();

    let offer_key = Offer::build_key(&collection, &token_id, &creator);
    let offer = offers()
        .load(deps.storage, offer_key.clone())
        .map_err(|e| ContractError::EntityNotFound(e.to_string()))?;

    let ask = Ask::new(
        collection.clone(),
        token_id.clone(),
        offer.order_info.price.clone(),
        info.sender,
        order_options.asset_recipient,
        order_options.finders_fee_bps,
        None,
    );

    response = finalize_sale(
        deps.as_ref(),
        &env,
        &ask,
        &MatchingOffer::Offer(offer.clone()),
        &sudo_params,
        order_options.finder.as_ref(),
        false,
        response,
    )?;

    response = offer.remove(deps.storage, false, RewardPayout::Return, response)?;

    // Remove the ask if it exists, refund removal fee if necessary
    let ask_option = asks().may_load(deps.storage, ask.key())?;
    if let Some(ask) = ask_option {
        response = ask.remove(deps.storage, false, RewardPayout::Return, response)?;
    }

    Ok(response)
}

/// Removes an offer made by the creator. Creators can only remove their own offers
pub fn execute_remove_offer(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let offer_key = Offer::build_key(&collection, &token_id, &info.sender);
    let offer = offers()
        .load(deps.storage, offer_key.clone())
        .map_err(|e| ContractError::EntityNotFound(e.to_string()))?;

    let response = offer.remove(deps.storage, true, RewardPayout::Return, Response::new())?;

    Ok(response)
}

/// Removes an offer made by the creator. Only NFT owners can reject an offer made by another creator.
pub fn execute_reject_offer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
    creator: Addr,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_owner_or_seller(deps.as_ref(), &env, &info, &collection, &token_id)?;

    let offer_key = Offer::build_key(&collection, &token_id, &creator);
    let offer = offers()
        .load(deps.storage, offer_key.clone())
        .map_err(|_| ContractError::EntityNotFound(format!("offer {}", offer_key.to_string())))?;

    let response = offer.remove(deps.storage, true, RewardPayout::Return, Response::new())?;

    Ok(response)
}

pub fn execute_remove_expired_offer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
    creator: Addr,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let offer_key = Offer::build_key(&collection, &token_id, &creator);
    let offer = offers()
        .load(deps.storage, offer_key.clone())
        .map_err(|_| ContractError::EntityNotFound(format!("offer {}", offer_key.to_string())))?;

    ensure!(
        offer.order_info.is_expired(&env.block),
        ContractError::EntityNotExpired(format!("offer {}", offer_key.to_string()))
    );

    let response = offer.remove(
        deps.storage,
        true,
        RewardPayout::Other(info.sender),
        Response::new(),
    )?;

    Ok(response)
}

/// Place an offer across an entire collection
pub fn execute_set_collection_offer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    price: Coin,
    order_options_str: OrderOptions<String>,
) -> Result<Response, ContractError> {
    only_tradable(&deps.querier, &env.block, &collection)?;
    validate_price(deps.storage, &price)?;

    let sudo_params = SUDO_PARAMS.load(deps.storage)?;
    let order_options =
        validate_order_options(deps.api, &info, &env.block, &sudo_params, order_options_str)?;

    let mut response = Response::new();
    let mut funds_due = NativeBalance(vec![]);

    let mut collection_offer = CollectionOffer::new(
        collection.clone(),
        price,
        info.sender.clone(),
        order_options.asset_recipient,
        order_options.finders_fee_bps,
        None,
    );
    let collection_offer_key = collection_offer.key();

    // Ensure the collection offer does not exist
    ensure!(
        collection_offers()
            .may_load(deps.storage, collection_offer_key.clone())?
            .is_none(),
        ContractError::EntityExists(format!(
            "collection_offer {}",
            collection_offer_key.to_string()
        ))
    );

    let matching_ask = collection_offer.match_with_ask(deps.as_ref(), &env)?;
    if let Some(ask) = matching_ask {
        // If a matching ask is found:
        // * ensure at least ask price is paid
        // * perform the sale
        // * remove the ask
        funds_due.add_assign(ask.order_info.price.clone());

        response = finalize_sale(
            deps.as_ref(),
            &env,
            &ask,
            &MatchingOffer::CollectionOffer(collection_offer),
            &sudo_params,
            order_options.finder.as_ref(),
            true,
            response,
        )?;

        response = ask.remove(deps.storage, false, RewardPayout::Return, response)?;
    } else {
        // If no match is found, collection offer creation should:
        // * ensure offer price is paid
        // * store the collection offer
        // * take removal fee if applicable
        funds_due.add_assign(collection_offer.order_info.price.clone());

        if let Some(expiration_info) = order_options.expiration_info {
            funds_due.add_assign(expiration_info.removal_reward.clone());
            collection_offer.order_info.expiration_info = Some(expiration_info);
        }

        collection_offers().save(deps.storage, collection_offer_key, &collection_offer)?;
    }

    response = reconcile_funds(&info, funds_due, response)?;

    Ok(response)
}

/// A buyer may update their offer on a collection
pub fn execute_update_collection_offer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    asset_recipient: Option<UpdateVal<String>>,
    finders_fee_bps: Option<UpdateVal<u64>>,
    expiration_info: Option<UpdateVal<ExpirationInfo>>,
) -> Result<Response, ContractError> {
    let api = deps.api;
    let sudo_params = SUDO_PARAMS.load(deps.storage)?;

    let mut response = Response::new();
    let mut funds_due = NativeBalance(vec![]);

    let collection_offer_key = CollectionOffer::build_key(&collection, &info.sender);
    let mut collection_offer = collection_offers()
        .load(deps.storage, collection_offer_key.clone())
        .map_err(|e| ContractError::EntityNotFound(e.to_string()))?;

    // Update asset recipient if set
    match asset_recipient {
        Some(UpdateVal::Set(asset_recipient_val)) => {
            collection_offer.order_info.asset_recipient =
                Some(api.addr_validate(&asset_recipient_val)?);
        }
        Some(UpdateVal::Unset) => {
            collection_offer.order_info.asset_recipient = None;
        }
        None => {}
    }

    // Update finders fee percent if set
    match finders_fee_bps {
        Some(UpdateVal::Set(finders_fee_bps_val)) => {
            collection_offer.order_info.finders_fee_bps = Some(finders_fee_bps_val);
        }
        Some(UpdateVal::Unset) => {
            collection_offer.order_info.finders_fee_bps = None;
        }
        None => {}
    }

    // Update the expiration on the ask if set
    if let Some(expiration_info_update) = expiration_info {
        // If a removal reward was paid, we need to refund it
        if let Some(old_expiration_info) = collection_offer.order_info.expiration_info {
            if !old_expiration_info.removal_reward.amount.is_zero() {
                response = transfer_coin(
                    old_expiration_info.removal_reward,
                    &collection_offer.order_info.creator,
                    response,
                )
            }
        }

        match expiration_info_update {
            UpdateVal::Set(new_expiration_info) => {
                validate_expiration_info(&env.block, &sudo_params, &new_expiration_info)?;
                funds_due.add_assign(new_expiration_info.removal_reward.clone());
                collection_offer.order_info.expiration_info = Some(new_expiration_info);
            }
            UpdateVal::Unset => {
                collection_offer.order_info.expiration_info = None;
            }
        }
    }

    collection_offer.save(deps.storage)?;

    response = reconcile_funds(&info, funds_due, response)?;

    Ok(response)
}

/// Owner of an item in a collection can accept a collection offer
pub fn execute_accept_collection_offer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
    creator: Addr,
    order_options_str: OrderOptions<String>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_tradable(&deps.querier, &env.block, &collection)?;
    only_owner_or_seller(deps.as_ref(), &env, &info, &collection, &token_id)?;

    let sudo_params = SUDO_PARAMS.load(deps.storage)?;
    let order_options =
        validate_order_options(deps.api, &info, &env.block, &sudo_params, order_options_str)?;

    let mut response = Response::new();

    let collection_offer_key = CollectionOffer::build_key(&collection, &creator);
    let collection_offer = collection_offers()
        .load(deps.storage, collection_offer_key.clone())
        .map_err(|_| {
            ContractError::EntityNotFound(format!(
                "collection_offer {}",
                collection_offer_key.to_string()
            ))
        })?;

    let ask = Ask::new(
        collection.clone(),
        token_id.clone(),
        collection_offer.order_info.price.clone(),
        info.sender,
        order_options.asset_recipient,
        order_options.finders_fee_bps,
        None,
    );

    response = finalize_sale(
        deps.as_ref(),
        &env,
        &ask,
        &MatchingOffer::CollectionOffer(collection_offer.clone()),
        &sudo_params,
        order_options.finder.as_ref(),
        false,
        response,
    )?;

    response = collection_offer.remove(deps.storage, false, RewardPayout::Return, response)?;

    // Remove the ask if it exists, refund removal fee if necessary
    let ask_key = Ask::build_key(&collection, &token_id);
    let ask_option = asks().may_load(deps.storage, ask_key)?;
    if let Some(ask) = ask_option {
        response = ask.remove(deps.storage, false, RewardPayout::Return, response)?;
    }

    Ok(response)
}

/// Removes an offer made by the creator. Creators can only remove their own offers
pub fn execute_remove_collection_offer(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    collection: Addr,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let collection_offer_key = CollectionOffer::build_key(&collection, &info.sender);
    let collection_offer = collection_offers()
        .load(deps.storage, collection_offer_key.clone())
        .map_err(|_| {
            ContractError::EntityNotFound(format!(
                "collection_offer {}",
                collection_offer_key.to_string()
            ))
        })?;

    let mut response = Response::new();

    response = collection_offer.remove(deps.storage, true, RewardPayout::Return, response)?;

    Ok(response)
}

/// Remove an existing collection offer
pub fn execute_remove_expired_collection_offer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    creator: Addr,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let collection_offer_key = CollectionOffer::build_key(&collection, &creator);
    let collection_offer = collection_offers()
        .load(deps.storage, collection_offer_key.clone())
        .map_err(|e| ContractError::EntityNotFound(e.to_string()))?;

    ensure!(
        collection_offer.order_info.is_expired(&env.block),
        ContractError::EntityNotExpired(format!(
            "collection_offer {}",
            collection_offer_key.to_string()
        ))
    );

    let response = collection_offer.remove(
        deps.storage,
        true,
        RewardPayout::Other(info.sender),
        Response::new(),
    )?;

    Ok(response)
}
