use cosmwasm_std::{
    coin, ensure, ensure_eq, has_coins, Addr, Coin, DepsMut, Env, MessageInfo, Response,
};
use cw_utils::{maybe_addr, nonpayable, NativeBalance};
use sg_marketplace_common::{
    address::address_or,
    coin::{transfer_coin, transfer_coins},
    nft::{only_owner, only_tradable, transfer_nft},
    MarketplaceStdError,
};
use std::ops::{Add, Sub};

use crate::{
    error::ContractError,
    events::{
        AskEvent, BidEvent, CollectionBidEvent, CollectionDenomEvent, ConfigEvent, ListingFeeEvent,
        MinExpiryRewardEvent,
    },
    helpers::{finalize_sale, generate_id, only_contract_admin, only_valid_price, validate_expiry},
    msg::ExecuteMsg,
    orders::{Ask, Bid, CollectionBid, MatchingBid, OrderDetails},
    state::{
        asks, bids, collection_bids, Config, Denom, OrderId, TokenId, COLLECTION_DENOMS, CONFIG,
        LISTING_FEES, MIN_EXPIRY_REWARDS, NONCE,
    },
};

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
        ExecuteMsg::UpdateCollectionDenom { collection, denom } => {
            execute_update_collection_denom(deps, env, info, api.addr_validate(&collection)?, denom)
        }
        ExecuteMsg::SetListingFee { fee } => execute_set_listing_fee(deps, env, info, fee),
        ExecuteMsg::RemoveListingFee { denom } => {
            execute_remove_listing_fee(deps, env, info, denom)
        }
        ExecuteMsg::SetMinExpiryReward { min_reward } => {
            execute_set_min_expiry_reward(deps, env, info, min_reward)
        }
        ExecuteMsg::RemoveMinExpiryReward { denom } => {
            execute_remove_min_expiry_reward(deps, env, info, denom)
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
        ExecuteMsg::UpdateAsk { id, details } => {
            execute_update_ask(deps, env, info, id, details.str_to_addr(api)?)
        }
        ExecuteMsg::RemoveAsk {
            id,
            reward_recipient,
        } => execute_remove_ask(deps, env, info, id, reward_recipient),
        ExecuteMsg::AcceptAsk { id, details } => {
            execute_accept_ask(deps, env, info, id, details.str_to_addr(api)?)
        }
        ExecuteMsg::SetBid {
            collection,
            token_id,
            details,
        } => execute_set_bid(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            token_id,
            details.str_to_addr(api)?,
            false,
        ),
        ExecuteMsg::UpdateBid { id, details } => {
            execute_update_bid(deps, env, info, id, details.str_to_addr(api)?)
        }
        ExecuteMsg::RemoveBid {
            id,
            reward_recipient,
        } => execute_remove_bid(deps, env, info, id, reward_recipient),
        ExecuteMsg::AcceptBid { id, details } => {
            execute_accept_bid(deps, env, info, id, details.str_to_addr(api)?)
        }
        ExecuteMsg::SetCollectionBid {
            collection,
            details,
        } => execute_set_collection_bid(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            details.str_to_addr(api)?,
            false,
        ),
        ExecuteMsg::UpdateCollectionBid { id, details } => {
            execute_update_collection_bid(deps, env, info, id, details.str_to_addr(api)?)
        }
        ExecuteMsg::RemoveCollectionBid {
            id,
            reward_recipient,
        } => execute_remove_collection_bid(deps, env, info, id, reward_recipient),
        ExecuteMsg::AcceptCollectionBid {
            id,
            token_id,
            details,
        } => {
            execute_accept_collection_bid(deps, env, info, id, token_id, details.str_to_addr(api)?)
        }
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
        ExecuteMsg::BuySpecificNft {
            collection,
            token_id,
            details,
        } => execute_set_bid(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            token_id,
            details.str_to_addr(api)?,
            true,
        ),
        ExecuteMsg::BuyCollectionNft {
            collection,
            details,
        } => execute_set_collection_bid(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            details.str_to_addr(api)?,
            true,
        ),
    }
}

pub fn execute_update_config(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    config: Config<Addr>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_contract_admin(&deps.querier, &env, &info)?;
    config.save(deps.storage)?;

    let response = Response::new().add_event(
        ConfigEvent {
            ty: "set-config",
            config: &config,
        }
        .into(),
    );

    Ok(response)
}

pub fn execute_update_collection_denom(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    denom: Denom,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_contract_admin(&deps.querier, &env, &info)?;

    COLLECTION_DENOMS.save(deps.storage, collection.clone(), &denom)?;

    let response = Response::new().add_event(
        CollectionDenomEvent {
            ty: "set-collection-denom",
            collection: collection.as_ref(),
            denom: &denom,
        }
        .into(),
    );

    Ok(response)
}

pub fn execute_set_listing_fee(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    fee: Coin,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_contract_admin(&deps.querier, &env, &info)?;

    LISTING_FEES.save(deps.storage, fee.denom.clone(), &fee.amount)?;

    let response = Response::new().add_event(
        ListingFeeEvent {
            ty: "set-listing-fee",
            denom: &fee.denom,
            amount: &Some(fee.amount),
        }
        .into(),
    );

    Ok(response)
}

pub fn execute_remove_listing_fee(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    denom: Denom,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_contract_admin(&deps.querier, &env, &info)?;

    LISTING_FEES.remove(deps.storage, denom.clone());

    let response = Response::new().add_event(
        ListingFeeEvent {
            ty: "remove-listing-fee",
            denom: &denom,
            amount: &None,
        }
        .into(),
    );

    Ok(response)
}

pub fn execute_set_min_expiry_reward(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    min_reward: Coin,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_contract_admin(&deps.querier, &env, &info)?;

    MIN_EXPIRY_REWARDS.save(deps.storage, min_reward.denom.clone(), &min_reward.amount)?;

    let response = Response::new().add_event(
        MinExpiryRewardEvent {
            ty: "set-min-expiry-reward",
            denom: &min_reward.denom,
            amount: &Some(min_reward.amount),
        }
        .into(),
    );

    Ok(response)
}

pub fn execute_remove_min_expiry_reward(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    denom: Denom,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_contract_admin(&deps.querier, &env, &info)?;

    MIN_EXPIRY_REWARDS.remove(deps.storage, denom.clone());

    let response = Response::new().add_event(
        MinExpiryRewardEvent {
            ty: "remove-min-expiry-reward",
            denom: &denom,
            amount: &None,
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
    only_owner(&deps.querier, &info, &collection, &token_id)?;
    only_tradable(&deps.querier, &env.block, &collection)?;

    let config = CONFIG.load(deps.storage)?;
    only_valid_price(deps.storage, &config, &collection, &details.price)?;

    let mut funds = NativeBalance(info.funds.clone());
    funds.normalize();

    // Remove expiry reward from funds
    let expiry_reward = validate_expiry(deps.storage, details.expiry.as_ref())?;
    if let Some(reward) = &expiry_reward {
        funds = funds
            .sub(reward.clone())
            .map_err(|_| ContractError::InsufficientFunds("expiry reward".to_string()))?;
    }

    // Check and collect listing fee
    let mut listing_payment_option: Option<Coin> = None;
    for user_coin in funds.clone().into_vec() {
        if let Some(listing_fee) = LISTING_FEES.may_load(deps.storage, user_coin.denom.clone())? {
            listing_payment_option = Some(coin(listing_fee.u128(), user_coin.denom));
            break;
        }
    }
    let listing_payment = listing_payment_option
        .ok_or(ContractError::InsufficientFunds("listing fee".to_string()))?;
    funds = funds
        .sub(listing_payment.clone())
        .map_err(|_| ContractError::InsufficientFunds("listing fee".to_string()))?;

    // Transfer listing fee to fee manager
    let mut response = transfer_coin(listing_payment, &config.fee_manager, Response::new());

    let ask = Ask::new(info.sender.clone(), collection, token_id, details);

    let match_result = ask.match_with_bid(deps.as_ref())?;

    if let Some(matching_bid) = match_result {
        // If a match is found:
        // * finalize the sale
        // * re-add expiry reward to funds so it can be refunded to the seller
        response = finalize_sale(deps, &env, &ask, &config, &matching_bid, false, response)?;

        if let Some(reward) = &expiry_reward {
            funds = funds.add(reward.clone());
        }
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
                    "expiry_timestamp",
                    "expiry_reward",
                ],
            }
            .into(),
        );

        asks().update(deps.storage, ask.id.clone(), |existing| match existing {
            Some(_) => Err(ContractError::InternalError("ask id collision".to_string())),
            None => Ok(ask.clone()),
        })?;
    };

    // Transfer remaining funds back to user
    if !funds.is_empty() {
        response = transfer_coins(funds.into_vec(), &ask.asset_recipient(), response);
    }

    Ok(response)
}

pub fn execute_update_ask(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: OrderId,
    details: OrderDetails<Addr>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let mut ask = asks()
        .load(deps.storage, id.clone())
        .map_err(|_| ContractError::InvalidInput(format!("ask not found [{}]", id)))?;

    ensure_eq!(
        info.sender,
        ask.creator,
        MarketplaceStdError::Unauthorized(
            "only the creator of ask can perform this action".to_string()
        )
    );

    only_valid_price(deps.storage, &config, &ask.collection, &details.price)?;

    let mut funds = NativeBalance(info.funds.clone());
    funds.normalize();

    // Add previous expiry reward to message funds
    if let Some(reward) = ask.details.expiry_reward() {
        funds = funds.add(reward.clone());
    }

    // Remove next expiry reward from message funds
    let expiry_reward = validate_expiry(deps.storage, details.expiry.as_ref())?;
    if let Some(reward) = &expiry_reward {
        funds = funds
            .sub(reward.clone())
            .map_err(|_| ContractError::InsufficientFunds("expiry reward".to_string()))?;
    }

    ask.details = details;

    let mut response = Response::new();

    let match_result = ask.match_with_bid(deps.as_ref())?;

    if let Some(matching_bid) = match_result {
        // If a match is found:
        // * finalize the sale
        // * re-add expiry reward to funds so it can be refunded to the seller
        response = finalize_sale(deps, &env, &ask, &config, &matching_bid, false, response)?;

        if let Some(reward) = &expiry_reward {
            funds = funds.add(reward.clone());
        }
    } else {
        // If no match is found continue updating the ask
        ask.save(deps.storage)?;

        response = response.add_event(
            AskEvent {
                ty: "update-ask",
                ask: &ask,
                attr_keys: vec![
                    "id",
                    "collection",
                    "token_id",
                    "price",
                    "recipient",
                    "finder",
                    "expiry_timestamp",
                    "expiry_reward",
                ],
            }
            .into(),
        );
    };

    // Transfer remaining funds back to user
    if !funds.is_empty() {
        response = transfer_coins(funds.into_vec(), &ask.asset_recipient(), response);
    }

    Ok(response)
}

pub fn execute_remove_ask(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: OrderId,
    reward_recipient: Option<String>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let ask = asks()
        .load(deps.storage, id.clone())
        .map_err(|_| ContractError::InvalidInput(format!("ask not found [{}]", id)))?;

    if !ask.details.is_expired(&env) {
        ensure_eq!(
            info.sender,
            ask.creator,
            MarketplaceStdError::Unauthorized(
                "only the creator of ask can perform this action".to_string()
            )
        );
    }

    let mut response = transfer_nft(
        &ask.collection,
        &ask.token_id,
        &ask.asset_recipient(),
        Response::new(),
    );

    ask.remove(deps.storage)?;

    response = response.add_event(
        AskEvent {
            ty: "remove-ask",
            ask: &ask,
            attr_keys: vec!["id", "collection", "token_id"],
        }
        .into(),
    );

    if let Some(reward) = ask.details.expiry_reward() {
        let reward_recipient_addr = address_or(
            maybe_addr(deps.api, reward_recipient)?.as_ref(),
            &ask.asset_recipient(),
        );

        response = transfer_coin(reward.clone(), &reward_recipient_addr, response);
    }

    Ok(response)
}

pub fn execute_accept_ask(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: OrderId,
    details: OrderDetails<Addr>,
) -> Result<Response, ContractError> {
    let mut funds = NativeBalance(info.funds.clone());
    funds.normalize();

    let ask = asks()
        .load(deps.storage, id.clone())
        .map_err(|_| ContractError::InvalidInput(format!("ask not found [{}]", id)))?;

    ensure!(
        has_coins(&[details.price.clone()], &ask.details.price),
        ContractError::InvalidInput("ask price is greater than max input".to_string())
    );

    funds = funds
        .sub(ask.details.price.clone())
        .map_err(|_| ContractError::InsufficientFunds("ask price".to_string()))?;

    let nonce = NONCE.load(deps.storage)?.wrapping_add(1);
    NONCE.save(deps.storage, &nonce)?;

    let bid = Bid::new(
        info.sender.clone(),
        ask.collection.clone(),
        ask.token_id.clone(),
        details,
        env.block.height,
        nonce,
    );

    let config = CONFIG.load(deps.storage)?;
    let mut response = finalize_sale(
        deps,
        &env,
        &ask,
        &config,
        &MatchingBid::Bid(bid),
        true,
        Response::new(),
    )?;

    // Transfer remaining funds back to user
    if !funds.is_empty() {
        response = transfer_coins(funds.into_vec(), &info.sender, response);
    }

    Ok(response)
}

pub fn execute_set_bid(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
    details: OrderDetails<Addr>,
    buy_now: bool,
) -> Result<Response, ContractError> {
    only_tradable(&deps.querier, &env.block, &collection)?;

    let config = CONFIG.load(deps.storage)?;
    only_valid_price(deps.storage, &config, &collection, &details.price)?;

    let mut funds = NativeBalance(info.funds.clone());
    funds.normalize();

    let nonce = NONCE.load(deps.storage)?.wrapping_add(1);
    NONCE.save(deps.storage, &nonce)?;

    let bid = Bid::new(
        info.sender.clone(),
        collection,
        token_id,
        details,
        env.block.height,
        nonce,
    );

    let matching_ask = bid.match_with_ask(deps.as_ref())?;

    let mut response = Response::new();

    if let Some(ask) = matching_ask {
        // If a matching ask is found perform the sale
        funds = funds
            .sub(ask.details.price.clone())
            .map_err(|_| ContractError::InsufficientFunds("ask price".to_string()))?;

        let config: Config<Addr> = CONFIG.load(deps.storage)?;
        response = finalize_sale(
            deps,
            &env,
            &ask,
            &config,
            &MatchingBid::Bid(bid),
            true,
            response,
        )?;
    } else if buy_now {
        // If no match is found and buy_now is true, abort transaction
        Err(ContractError::NoMatchFound)?;
    } else {
        // If no match is found. Bid creation should:
        // * remove bid price from funds
        // * remove expiry reward from funds if any
        // * store the bid
        // * emit event
        funds = funds
            .sub(bid.details.price.clone())
            .map_err(|_| ContractError::InsufficientFunds("bid price".to_string()))?;

        let expiry_reward = validate_expiry(deps.storage, bid.details.expiry.as_ref())?;
        if let Some(reward) = &expiry_reward {
            funds = funds
                .sub(reward.clone())
                .map_err(|_| ContractError::InsufficientFunds("expiry reward".to_string()))?;
        }

        response = response.add_event(
            BidEvent {
                ty: "set-bid",
                bid: &bid,
                attr_keys: vec![
                    "id",
                    "creator",
                    "collection",
                    "token_id",
                    "price",
                    "recipient",
                    "finder",
                    "expiry_timestamp",
                    "expiry_reward",
                ],
            }
            .into(),
        );

        bids().update(deps.storage, bid.id.clone(), |existing| match existing {
            Some(_) => Err(ContractError::InternalError("bid id collision".to_string())),
            None => Ok(bid),
        })?;
    }

    // Transfer remaining funds back to user
    if !funds.is_empty() {
        response = transfer_coins(funds.into_vec(), &info.sender, response);
    }

    Ok(response)
}

pub fn execute_update_bid(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: OrderId,
    details: OrderDetails<Addr>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let mut bid = bids()
        .load(deps.storage, id.clone())
        .map_err(|_| ContractError::InvalidInput(format!("bid not found [{}]", id)))?;

    ensure_eq!(
        info.sender,
        bid.creator,
        MarketplaceStdError::Unauthorized(
            "only the creator of bid can perform this action".to_string()
        )
    );

    only_valid_price(deps.storage, &config, &bid.collection, &details.price)?;

    let mut funds = NativeBalance(info.funds.clone());
    funds.normalize();

    // Add the previous price to the funds in context
    funds = funds.add(bid.details.price.clone());

    // Add previous expiry reward to message funds
    if let Some(reward) = bid.details.expiry_reward() {
        funds = funds.add(reward.clone());
    }

    // Remove next expiry reward from message funds
    let expiry_reward = validate_expiry(deps.storage, details.expiry.as_ref())?;
    if let Some(reward) = &expiry_reward {
        funds = funds
            .sub(reward.clone())
            .map_err(|_| ContractError::InsufficientFunds("expiry reward".to_string()))?;
    }

    bid.details = details;

    let mut response = Response::new();

    let match_result = bid.match_with_ask(deps.as_ref())?;

    if let Some(ask) = match_result {
        // If a match is found:
        // * re-add expiry reward to funds so it can be refunded to the bidder
        // * deduct ask price from funds
        // * finalize the sale
        if let Some(reward) = bid.details.expiry_reward() {
            funds = funds.add(reward.clone());
        }

        funds = funds
            .sub(ask.details.price.clone())
            .map_err(|_| ContractError::InsufficientFunds("ask price".to_string()))?;

        let config: Config<Addr> = CONFIG.load(deps.storage)?;

        response = finalize_sale(
            deps,
            &env,
            &ask,
            &config,
            &MatchingBid::Bid(bid),
            true,
            response,
        )?;
    } else {
        // If no match is found update the bid
        funds = funds
            .sub(bid.details.price.clone())
            .map_err(|_| ContractError::InsufficientFunds("bid price".to_string()))?;

        bid.save(deps.storage)?;

        response = response.add_event(
            BidEvent {
                ty: "update-bid",
                bid: &bid,
                attr_keys: vec![
                    "id",
                    "collection",
                    "token_id",
                    "price",
                    "creator",
                    "recipient",
                    "finder",
                    "expiry_timestamp",
                    "expiry_reward",
                ],
            }
            .into(),
        );
    };

    // Transfer remaining funds back to user
    if !funds.is_empty() {
        response = transfer_coins(funds.into_vec(), &info.sender, response);
    }

    Ok(response)
}

pub fn execute_remove_bid(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: OrderId,
    reward_recipient: Option<String>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let bid = bids()
        .load(deps.storage, id.clone())
        .map_err(|_| ContractError::InvalidInput(format!("bid not found [{}]", id)))?;

    if !bid.details.is_expired(&env) {
        ensure_eq!(
            info.sender,
            bid.creator,
            MarketplaceStdError::Unauthorized(
                "only the creator of bid can perform this action".to_string()
            )
        )
    }

    let refund = bid.details.price.clone();

    bid.remove(deps.storage)?;

    let mut response = transfer_coin(refund, &info.sender, Response::new());

    response = response.add_event(
        BidEvent {
            ty: "remove-bid",
            bid: &bid,
            attr_keys: vec!["id", "collection", "token_id"],
        }
        .into(),
    );

    if let Some(reward) = bid.details.expiry_reward() {
        let reward_recipient_addr = address_or(
            maybe_addr(deps.api, reward_recipient)?.as_ref(),
            &bid.asset_recipient(),
        );

        response = transfer_coin(reward.clone(), &reward_recipient_addr, response);
    }

    Ok(response)
}

pub fn execute_accept_bid(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: OrderId,
    details: OrderDetails<Addr>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let bid: Bid = bids()
        .load(deps.storage, id.clone())
        .map_err(|_| ContractError::InvalidInput(format!("bid not found [{}]", id)))?;

    ensure!(
        has_coins(&[bid.details.price.clone()], &details.price),
        ContractError::InvalidInput("min output is greater than bid price".to_string())
    );

    let ask_id = generate_id(vec![bid.collection.as_bytes(), bid.token_id.as_bytes()]);
    let ask_option = asks().may_load(deps.storage, ask_id.clone())?;

    let mut response = Response::new();

    // Check if the sender is the owner of the NFT, or if the creator of a valid ask
    let ask = if let Some(ask) = ask_option {
        ensure_eq!(
            info.sender,
            ask.creator,
            MarketplaceStdError::Unauthorized("sender is not creator of ask".to_string())
        );

        // For pre-existing asks, refund the expiry reward to the seller if any
        if let Some(reward) = ask.details.expiry_reward() {
            response = transfer_coin(reward.clone(), &ask.asset_recipient(), response);
        }

        ask
    } else {
        only_owner(&deps.querier, &info, &bid.collection, &bid.token_id)?;

        Ask::new(
            info.sender.clone(),
            bid.collection.clone(),
            bid.token_id.clone(),
            details,
        )
    };

    let config: Config<Addr> = CONFIG.load(deps.storage)?;

    let response = finalize_sale(
        deps,
        &env,
        &ask,
        &config,
        &MatchingBid::Bid(bid),
        false,
        response,
    )?;

    Ok(response)
}

pub fn execute_set_collection_bid(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    details: OrderDetails<Addr>,
    buy_now: bool,
) -> Result<Response, ContractError> {
    only_tradable(&deps.querier, &env.block, &collection)?;

    let config = CONFIG.load(deps.storage)?;
    only_valid_price(deps.storage, &config, &collection, &details.price)?;

    let mut funds = NativeBalance(info.funds.clone());
    funds.normalize();

    let nonce = NONCE.load(deps.storage)?.wrapping_add(1);
    NONCE.save(deps.storage, &nonce)?;

    let collection_bid = CollectionBid::new(
        info.sender.clone(),
        collection,
        details,
        env.block.height,
        nonce,
    );

    let matching_ask = collection_bid.match_with_ask(deps.as_ref())?;

    let mut response = Response::new();

    if let Some(ask) = matching_ask {
        // If a matching ask is found perform the sale
        funds = funds
            .sub(ask.details.price.clone())
            .map_err(|_| ContractError::InsufficientFunds("ask price".to_string()))?;

        let config: Config<Addr> = CONFIG.load(deps.storage)?;
        response = finalize_sale(
            deps,
            &env,
            &ask,
            &config,
            &MatchingBid::CollectionBid(collection_bid),
            true,
            response,
        )?;
    } else if buy_now {
        // If no match is found and buy_now is true, abort transaction
        Err(ContractError::NoMatchFound)?;
    } else {
        // If no match is found. Bid creation should:
        // * deduct the bid price from funds
        // * deduct the expiry reward from funds if any
        // * emit event
        // * store the bid
        funds = funds
            .sub(collection_bid.details.price.clone())
            .map_err(|_| ContractError::InsufficientFunds("collection bid price".to_string()))?;

        if let Some(reward) = collection_bid.details.expiry_reward() {
            funds = funds
                .sub(reward.clone())
                .map_err(|_| ContractError::InsufficientFunds("expiry reward".to_string()))?;
        }

        response = response.add_event(
            CollectionBidEvent {
                ty: "set-collection-bid",
                collection_bid: &collection_bid,
                attr_keys: vec![
                    "id",
                    "creator",
                    "collection",
                    "price",
                    "recipient",
                    "finder",
                    "expiry_timestamp",
                    "expiry_reward",
                ],
            }
            .into(),
        );

        collection_bids().update(
            deps.storage,
            collection_bid.id.clone(),
            |existing| match existing {
                Some(_) => Err(ContractError::InternalError(
                    "collection bid id collision".to_string(),
                )),
                None => Ok(collection_bid),
            },
        )?;
    }

    // Transfer remaining funds back to user
    if !funds.is_empty() {
        response = transfer_coins(funds.into_vec(), &info.sender, response);
    }

    Ok(response)
}

pub fn execute_update_collection_bid(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: OrderId,
    details: OrderDetails<Addr>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let mut collection_bid = collection_bids()
        .load(deps.storage, id.clone())
        .map_err(|_| ContractError::InvalidInput(format!("collection bid not found [{}]", id)))?;

    ensure_eq!(
        info.sender,
        collection_bid.creator,
        MarketplaceStdError::Unauthorized(
            "only the creator of collection bid can perform this action".to_string()
        )
    );

    only_valid_price(
        deps.storage,
        &config,
        &collection_bid.collection,
        &details.price,
    )?;

    let mut funds = NativeBalance(info.funds.clone());
    funds.normalize();

    // Add the previous price to the funds in context
    funds = funds.add(collection_bid.details.price.clone());

    // Add previous expiry reward to message funds
    if let Some(reward) = collection_bid.details.expiry_reward() {
        funds = funds.add(reward.clone());
    }

    // Remove next expiry reward from message funds
    let expiry_reward = validate_expiry(deps.storage, details.expiry.as_ref())?;
    if let Some(reward) = &expiry_reward {
        funds = funds
            .sub(reward.clone())
            .map_err(|_| ContractError::InsufficientFunds("expiry reward".to_string()))?;
    }

    collection_bid.details = details;

    let mut response = Response::new();

    let match_result = collection_bid.match_with_ask(deps.as_ref())?;

    if let Some(ask) = match_result {
        // If a match is found:
        // * re-add expiry reward to funds so it can be refunded to the bidder
        // * deduct ask price from funds
        // * finalize the sale
        if let Some(reward) = collection_bid.details.expiry_reward() {
            funds = funds.add(reward.clone());
        }

        funds = funds
            .sub(ask.details.price.clone())
            .map_err(|_| ContractError::InsufficientFunds("ask price".to_string()))?;

        let config: Config<Addr> = CONFIG.load(deps.storage)?;

        response = finalize_sale(
            deps,
            &env,
            &ask,
            &config,
            &MatchingBid::CollectionBid(collection_bid),
            true,
            response,
        )?;
    } else {
        // If no match is found update the bid
        funds = funds
            .sub(collection_bid.details.price.clone())
            .map_err(|_| ContractError::InsufficientFunds("collection bid price".to_string()))?;

        collection_bid.save(deps.storage)?;

        response = response.add_event(
            CollectionBidEvent {
                ty: "update-collection-bid",
                collection_bid: &collection_bid,
                attr_keys: vec![
                    "id",
                    "collection",
                    "price",
                    "creator",
                    "recipient",
                    "finder",
                    "expiry_timestamp",
                    "expiry_reward",
                ],
            }
            .into(),
        );
    };

    // Transfer remaining funds back to user
    if !funds.is_empty() {
        response = transfer_coins(funds.into_vec(), &info.sender, response);
    }

    Ok(response)
}

pub fn execute_remove_collection_bid(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: OrderId,
    reward_recipient: Option<String>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let collection_bid = collection_bids()
        .load(deps.storage, id.clone())
        .map_err(|_| ContractError::InvalidInput(format!("collection bid not found [{}]", id)))?;

    if !collection_bid.details.is_expired(&env) {
        ensure_eq!(
            info.sender,
            collection_bid.creator,
            MarketplaceStdError::Unauthorized(
                "only the creator of collection bid can perform this action".to_string()
            )
        );
    }

    let refund = collection_bid.details.price.clone();

    collection_bid.remove(deps.storage)?;

    let mut response = transfer_coin(refund, &info.sender, Response::new());

    response = response.add_event(
        CollectionBidEvent {
            ty: "remove-collection-bid",
            collection_bid: &collection_bid,
            attr_keys: vec!["id", "collection"],
        }
        .into(),
    );

    // Refund the expiry reward to the bidder if any
    if let Some(reward) = collection_bid.details.expiry_reward() {
        let reward_recipient_addr = address_or(
            maybe_addr(deps.api, reward_recipient)?.as_ref(),
            &collection_bid.asset_recipient(),
        );

        response = transfer_coin(reward.clone(), &reward_recipient_addr, response);
    }

    Ok(response)
}

pub fn execute_accept_collection_bid(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: OrderId,
    token_id: TokenId,
    details: OrderDetails<Addr>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let collection_bid = collection_bids()
        .load(deps.storage, id.clone())
        .map_err(|_| ContractError::InvalidInput(format!("collection bid not found [{}]", id)))?;

    ensure!(
        has_coins(&[collection_bid.details.price.clone()], &details.price),
        ContractError::InvalidInput("min output is greater than collection bid price".to_string())
    );

    let ask_id = generate_id(vec![
        collection_bid.collection.as_bytes(),
        token_id.as_bytes(),
    ]);
    let ask_option = asks().may_load(deps.storage, ask_id.clone())?;

    let mut response = Response::new();

    // Check if the sender is the owner of the NFT, or if the creator of a valid ask
    let ask = if let Some(ask) = ask_option {
        ensure_eq!(
            info.sender,
            ask.creator,
            MarketplaceStdError::Unauthorized("sender is not creator of ask".to_string())
        );

        // For pre-existing asks, refund the expiry reward to the seller if any
        if let Some(reward) = ask.details.expiry_reward() {
            response = transfer_coin(reward.clone(), &ask.asset_recipient(), response);
        }

        ask
    } else {
        only_owner(&deps.querier, &info, &collection_bid.collection, &token_id)?;
        Ask::new(
            info.sender.clone(),
            collection_bid.collection.clone(),
            token_id.clone(),
            details,
        )
    };

    let config: Config<Addr> = CONFIG.load(deps.storage)?;

    let response = finalize_sale(
        deps,
        &env,
        &ask,
        &config,
        &MatchingBid::CollectionBid(collection_bid),
        false,
        response,
    )?;

    Ok(response)
}
