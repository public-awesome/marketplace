use crate::{
    error::ContractError,
    events::{AllowDenomsEvent, AskEvent, CollectionOfferEvent, ConfigEvent, OfferEvent},
    helpers::{finalize_sale, only_contract_admin},
    msg::ExecuteMsg,
    orders::{Ask, CollectionOffer, MatchingOffer, Offer, OrderDetails},
    state::{
        asks, collection_offers, offers, AllowDenoms, Config, OrderId, TokenId, ALLOW_DENOMS,
        CONFIG, NONCE,
    },
};

use cosmwasm_std::{ensure, ensure_eq, Addr, DepsMut, Env, Event, MessageInfo, Response};
use cw_utils::{nonpayable, NativeBalance};
use sg_marketplace_common::{
    coin::{transfer_coin, transfer_coins},
    nft::{only_owner, only_tradable, transfer_nft},
    MarketplaceStdError,
};
use std::ops::Sub;

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
        ExecuteMsg::UpdateConfig { config } => {
            execute_update_config(deps, env, info, config.str_to_addr(api)?)
        }
        ExecuteMsg::UpdateAllowDenoms { allow_denoms } => {
            execute_update_allow_denoms(deps, env, info, allow_denoms)
        }
        ExecuteMsg::SetAsk {
            collection,
            token_id,
            details,
        } => execute_set_ask(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            token_id,
            details.str_to_addr(api)?,
            false,
        ),
        ExecuteMsg::SellNft {
            collection,
            token_id,
            details,
        } => execute_set_ask(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            token_id,
            details.str_to_addr(api)?,
            true,
        ),
        ExecuteMsg::RemoveAsk { id } => execute_remove_ask(deps, env, info, id),
        ExecuteMsg::SetOffer {
            collection,
            token_id,
            details,
        } => execute_set_offer(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            token_id,
            details.str_to_addr(api)?,
            false,
        ),
        ExecuteMsg::BuySpecificNft {
            collection,
            token_id,
            details,
        } => execute_set_offer(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            token_id,
            details.str_to_addr(api)?,
            true,
        ),
        ExecuteMsg::RemoveOffer { id } => execute_remove_offer(deps, env, info, id),
        ExecuteMsg::SetCollectionOffer {
            collection,
            details,
        } => execute_set_collection_offer(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            details.str_to_addr(api)?,
            false,
        ),
        ExecuteMsg::BuyCollectionNft {
            collection,
            details,
        } => execute_set_collection_offer(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            details.str_to_addr(api)?,
            true,
        ),
        ExecuteMsg::RemoveCollectionOffer { id } => {
            execute_remove_collection_offer(deps, env, info, id)
        }
    }
}

pub fn execute_update_config(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    config: Config<Addr>,
) -> Result<Response, ContractError> {
    only_contract_admin(&deps.querier, &env, &info)?;

    CONFIG.save(deps.storage, &config)?;

    let response = Response::new().add_event(
        ConfigEvent {
            ty: "set-config",
            config: &config,
        }
        .into(),
    );

    Ok(response)
}

pub fn execute_update_allow_denoms(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    allow_denoms: AllowDenoms,
) -> Result<Response, ContractError> {
    only_contract_admin(&deps.querier, &env, &info)?;

    ALLOW_DENOMS.save(deps.storage, &allow_denoms)?;

    let response = Response::new().add_event(
        AllowDenomsEvent {
            ty: "set-allow-denoms",
            allow_denoms: &allow_denoms,
        }
        .into(),
    );

    Ok(response)
}

pub fn execute_set_ask(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
    details: OrderDetails<Addr>,
    sell_now: bool,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_owner(&deps.querier, &info, &collection, &token_id)?;
    only_tradable(&deps.querier, &env.block, &collection)?;

    let allow_denoms = ALLOW_DENOMS.load(deps.storage)?;
    ensure!(
        allow_denoms.contains(&details.price.denom),
        ContractError::InvalidInput("invalid denom".to_string())
    );

    let ask = Ask::new(info.sender.clone(), collection, token_id, details);

    let config = CONFIG.load(deps.storage)?;

    let mut response = Response::new();

    let match_result = ask.match_with_offer(deps.as_ref())?;

    if let Some(matching_offer) = match_result {
        // If a match is found finalize the sale
        response = finalize_sale(deps, &env, &ask, &config, &matching_offer, false, response)?;
    } else if sell_now {
        // If no match is found and sell_now is true, abort transaction
        Err(ContractError::NoMatchFound)?;
    } else {
        // If no match is found continue creating the ask.
        // Ask creation should:
        // * escrow the nft
        // * store the ask

        response = transfer_nft(
            &ask.collection,
            &ask.token_id,
            &env.contract.address,
            response,
        );

        response = response.add_event(
            AskEvent {
                ty: "set-ask",
                ask: &ask,
                attr_keys: vec![
                    "id",
                    "creator",
                    "collection",
                    "token_id",
                    "price",
                    "recipient",
                    "finder",
                ],
            }
            .into(),
        );

        asks().update(deps.storage, ask.id.clone(), |existing| match existing {
            Some(_) => Err(ContractError::InternalError("ask id collision".to_string())),
            None => Ok(ask),
        })?;
    };

    Ok(response)
}

pub fn execute_remove_ask(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    id: OrderId,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let ask = asks()
        .load(deps.storage, id.clone())
        .map_err(|_| ContractError::InvalidInput(format!("ask not found [{}]", id)))?;

    ensure_eq!(
        info.sender,
        ask.creator,
        MarketplaceStdError::Unauthorized(
            "only the creator of ask can perform this action".to_string()
        )
    );

    let mut response = transfer_nft(
        &ask.collection,
        &ask.token_id,
        &ask.asset_recipient(),
        Response::new(),
    );

    ask.remove(deps.storage)?;

    response = response.add_event(Event::new("remove-ask".to_string()).add_attribute("id", id));

    Ok(response)
}

pub fn execute_set_offer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
    details: OrderDetails<Addr>,
    buy_now: bool,
) -> Result<Response, ContractError> {
    only_tradable(&deps.querier, &env.block, &collection)?;

    let allow_denoms = ALLOW_DENOMS.load(deps.storage)?;
    ensure!(
        allow_denoms.contains(&details.price.denom),
        ContractError::InvalidInput("invalid denom".to_string())
    );

    let mut funds = NativeBalance(info.funds.clone());
    funds.normalize();

    let nonce = NONCE.load(deps.storage)?.wrapping_add(1);
    NONCE.save(deps.storage, &nonce)?;

    let offer = Offer::new(
        info.sender.clone(),
        collection,
        token_id,
        details,
        env.block.height,
        nonce,
    );

    let matching_ask = offer.match_with_ask(deps.as_ref())?;

    let mut response = Response::new();

    if let Some(ask) = matching_ask {
        // If a matching ask is found perform the sale
        funds = funds
            .sub(ask.details.price.clone())
            .map_err(|_| ContractError::InsufficientFunds)?;

        let config: Config<Addr> = CONFIG.load(deps.storage)?;
        response = finalize_sale(
            deps,
            &env,
            &ask,
            &config,
            &MatchingOffer::Offer(offer),
            true,
            response,
        )?;
    } else if buy_now {
        // If no match is found and buy_now is true, abort transaction
        Err(ContractError::NoMatchFound)?;
    } else {
        // If no match is found. Offer creation should:
        // * store the offer
        // * emit event

        funds = funds
            .sub(offer.details.price.clone())
            .map_err(|_| ContractError::InsufficientFunds)?;

        response = response.add_event(
            OfferEvent {
                ty: "set-offer",
                offer: &offer,
                attr_keys: vec![
                    "id",
                    "creator",
                    "collection",
                    "token_id",
                    "price",
                    "recipient",
                    "finder",
                ],
            }
            .into(),
        );

        offers().update(deps.storage, offer.id.clone(), |existing| match existing {
            Some(_) => Err(ContractError::InternalError(
                "offer id collision".to_string(),
            )),
            None => Ok(offer),
        })?;
    }

    // Transfer remaining funds back to user
    if !funds.is_empty() {
        response = transfer_coins(funds.into_vec(), &info.sender, response);
    }

    Ok(response)
}

pub fn execute_remove_offer(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    id: OrderId,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let offer = offers()
        .load(deps.storage, id.clone())
        .map_err(|_| ContractError::InvalidInput(format!("offer not found [{}]", id)))?;

    ensure_eq!(
        info.sender,
        offer.creator,
        MarketplaceStdError::Unauthorized(
            "only the creator of offer can perform this action".to_string()
        )
    );

    let refund = offer.details.price.clone();

    offer.remove(deps.storage)?;

    let mut response = transfer_coin(refund, &info.sender, Response::new());

    response = response.add_event(Event::new("remove-offer".to_string()).add_attribute("id", id));

    Ok(response)
}

pub fn execute_set_collection_offer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    details: OrderDetails<Addr>,
    buy_now: bool,
) -> Result<Response, ContractError> {
    only_tradable(&deps.querier, &env.block, &collection)?;

    let allow_denoms = ALLOW_DENOMS.load(deps.storage)?;
    ensure!(
        allow_denoms.contains(&details.price.denom),
        ContractError::InvalidInput("invalid denom".to_string())
    );

    let mut funds = NativeBalance(info.funds.clone());
    funds.normalize();

    let nonce = NONCE.load(deps.storage)?.wrapping_add(1);
    NONCE.save(deps.storage, &nonce)?;

    let collection_offer = CollectionOffer::new(
        info.sender.clone(),
        collection,
        details,
        env.block.height,
        nonce,
    );

    let matching_ask = collection_offer.match_with_ask(deps.as_ref())?;

    let mut response = Response::new();

    if let Some(ask) = matching_ask {
        // If a matching ask is found perform the sale
        funds = funds
            .sub(ask.details.price.clone())
            .map_err(|_| ContractError::InsufficientFunds)?;

        let config: Config<Addr> = CONFIG.load(deps.storage)?;
        response = finalize_sale(
            deps,
            &env,
            &ask,
            &config,
            &MatchingOffer::CollectionOffer(collection_offer),
            true,
            response,
        )?;
    } else if buy_now {
        // If no match is found and buy_now is true, abort transaction
        Err(ContractError::NoMatchFound)?;
    } else {
        // If no match is found. Offer creation should store the offer
        funds = funds
            .sub(collection_offer.details.price.clone())
            .map_err(|_| ContractError::InsufficientFunds)?;

        response = response.add_event(
            CollectionOfferEvent {
                ty: "set-collection-offer",
                collection_offer: &collection_offer,
                attr_keys: vec![
                    "id",
                    "creator",
                    "collection",
                    "price",
                    "recipient",
                    "finder",
                ],
            }
            .into(),
        );

        collection_offers().update(deps.storage, collection_offer.id.clone(), |existing| {
            match existing {
                Some(_) => Err(ContractError::InternalError(
                    "collection offer id collision".to_string(),
                )),
                None => Ok(collection_offer),
            }
        })?;
    }

    // Transfer remaining funds back to user
    if !funds.is_empty() {
        response = transfer_coins(funds.into_vec(), &info.sender, response);
    }

    Ok(response)
}

pub fn execute_remove_collection_offer(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    id: OrderId,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let collection_offer = collection_offers()
        .load(deps.storage, id.clone())
        .map_err(|_| ContractError::InvalidInput(format!("collection offer not found [{}]", id)))?;

    ensure_eq!(
        info.sender,
        collection_offer.creator,
        MarketplaceStdError::Unauthorized(
            "only the creator of collection offer can perform this action".to_string()
        )
    );

    let refund = collection_offer.details.price.clone();

    collection_offer.remove(deps.storage)?;

    let mut response = transfer_coin(refund, &info.sender, Response::new());

    response = response
        .add_event(Event::new("remove-collection-offer".to_string()).add_attribute("id", id));

    Ok(response)
}
