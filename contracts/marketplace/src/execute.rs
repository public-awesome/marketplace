use std::ops::Sub;

use cosmwasm_std::{
    attr, coin, ensure, Addr, Coin, Decimal, DepsMut, Env, Event, MessageInfo, Order, StdResult,
    Timestamp, Uint128,
};
use cw721::OwnerOfResponse;
use cw_utils::{maybe_addr, nonpayable, one_coin, NativeBalance};
use sg_marketplace_common::{
    address::address_or,
    coin::{bps_to_decimal, transfer_coin},
    nft::{has_approval, only_owner, only_tradable, owner_of},
};
use sg_std::{Response, NATIVE_DENOM};
use stargaze_fair_burn::append_fair_burn_msg;

use crate::{
    error::ContractError,
    helpers::{finalize_sale, match_offer, MatchResult},
    hooks::{prepare_ask_hook, prepare_collection_offer_hook, prepare_offer_hook},
    msg::{ExecuteMsg, HookAction},
    state::{
        asks, collection_offers, offers, Ask, CollectionOffer, ExpiringOrder, Offer, SudoParams,
        TokenId, SUDO_PARAMS,
    },
    state_deprecated::{
        ask_key as ask_key_dep, asks as asks_dep, bid_key as bid_key_dep, bids as bids_dep,
        collection_bid_key as collection_bid_key_dep, collection_bids as collection_bids_dep,
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
        ExecuteMsg::SetAsk {
            collection,
            token_id,
            price,
            asset_recipient,
            reserve_for,
            finders_fee_bps,
            expires,
        } => execute_set_ask(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            token_id,
            price,
            maybe_addr(api, asset_recipient)?,
            maybe_addr(api, reserve_for)?,
            finders_fee_bps,
            expires,
        ),
        ExecuteMsg::UpdateAskPrice {
            collection,
            token_id,
            price,
        } => execute_update_ask_price(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            token_id,
            price,
        ),
        ExecuteMsg::RemoveAsk {
            collection,
            token_id,
        } => execute_remove_ask(deps, info, api.addr_validate(&collection)?, token_id),
        ExecuteMsg::RemoveStaleAsk {
            collection,
            token_id,
        } => execute_remove_stale_ask(deps, env, info, api.addr_validate(&collection)?, token_id),
        ExecuteMsg::MigrateAsks { limit } => execute_migrate_asks(deps, env, info, limit),
        ExecuteMsg::SetOffer {
            collection,
            token_id,
            asset_recipient,
            finder,
            finders_fee_bps,
            expires,
        } => execute_set_offer(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            token_id,
            maybe_addr(api, asset_recipient)?,
            maybe_addr(api, finder)?,
            finders_fee_bps,
            expires,
            false,
        ),
        ExecuteMsg::BuyNow {
            collection,
            token_id,
            asset_recipient,
            finder,
        } => execute_set_offer(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            token_id,
            maybe_addr(api, asset_recipient)?,
            maybe_addr(api, finder)?,
            None,
            None,
            true,
        ),
        ExecuteMsg::AcceptOffer {
            collection,
            token_id,
            bidder,
            asset_recipient,
            finder,
        } => execute_accept_offer(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            token_id,
            api.addr_validate(&bidder)?,
            maybe_addr(api, asset_recipient)?,
            maybe_addr(api, finder)?,
        ),
        ExecuteMsg::RemoveOffer {
            collection,
            token_id,
        } => execute_remove_offer(deps, env, info, api.addr_validate(&collection)?, token_id),
        ExecuteMsg::RejectOffer {
            collection,
            token_id,
            bidder,
        } => execute_reject_offer(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            token_id,
            api.addr_validate(&bidder)?,
        ),
        ExecuteMsg::RemoveStaleOffer {
            collection,
            token_id,
            bidder,
        } => execute_remove_stale_offer(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            token_id,
            api.addr_validate(&bidder)?,
        ),
        ExecuteMsg::MigrateOffers { limit } => execute_migrate_offers(deps, env, info, limit),
        ExecuteMsg::SetCollectionOffer {
            collection,
            asset_recipient,
            finders_fee_bps,
            expires,
        } => execute_set_collection_offer(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            maybe_addr(api, asset_recipient)?,
            finders_fee_bps,
            expires,
        ),
        ExecuteMsg::AcceptCollectionOffer {
            collection,
            token_id,
            bidder,
            asset_recipient,
            finder,
        } => execute_accept_collection_offer(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            token_id,
            api.addr_validate(&bidder)?,
            maybe_addr(api, asset_recipient)?,
            maybe_addr(api, finder)?,
        ),
        ExecuteMsg::RemoveCollectionOffer { collection } => {
            execute_remove_collection_offer(deps, env, info, api.addr_validate(&collection)?)
        }
        ExecuteMsg::RemoveStaleCollectionOffer { collection, bidder } => {
            execute_remove_stale_collection_offer(
                deps,
                env,
                info,
                api.addr_validate(&collection)?,
                api.addr_validate(&bidder)?,
            )
        }
        ExecuteMsg::MigrateCollectionOffers { limit } => {
            execute_migrate_collection_offers(deps, env, info, limit)
        }
    }
}

fn set_ask(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    sudo_params: &SudoParams,
    funds_normalized: NativeBalance,
    mut ask: Ask,
    mut response: Response,
) -> Result<Response, ContractError> {
    // Validate that the Ask:
    // * has valid properties
    // * is for an NFT owned by the sender
    // * is for an NFT that has passed the start trade time threshhold
    // * is for an NFT that has the contract address approved for transfers
    only_owner(&deps.querier, &info, &ask.collection, &ask.token_id)?;
    only_tradable(deps.as_ref(), &env.block, &ask.collection)?;
    has_approval(
        &deps.querier,
        &env.contract.address,
        &ask.collection,
        &ask.token_id,
        Some(false),
    )?;

    // If existing Ask has a paid removal fee, return to sender
    let existing_ask = asks().may_load(deps.storage, ask.key())?;
    if let Some(existing_ask) = existing_ask {
        if let Some(paid_removal_fee) = existing_ask.paid_removal_fee {
            response =
                response.add_submessage(transfer_coin(paid_removal_fee, &existing_ask.seller));
        }
    }

    // Validate removal fee is paid, if applicable
    if let Some(removal_fee) = ask.removal_fee(sudo_params.removal_reward_percent) {
        ensure!(
            funds_normalized.has(&removal_fee),
            ContractError::InvalidFunds {
                expected: removal_fee,
            }
        );
        funds_normalized.sub(removal_fee.clone())?;
        ask.paid_removal_fee = Some(removal_fee);
    }

    ask.save(deps.storage)?;

    Ok(response)
}

/// A seller may set an Ask on their NFT to list it on Marketplace
pub fn execute_set_ask(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
    price: Coin,
    asset_recipient: Option<Addr>,
    reserve_for: Option<Addr>,
    finders_fee_bps: Option<u64>,
    expires: Option<Timestamp>,
) -> Result<Response, ContractError> {
    let mut response = Response::new();
    let sudo_params = SUDO_PARAMS.load(deps.storage)?;

    let ask = Ask {
        collection,
        token_id,
        seller: info.sender.clone(),
        price,
        asset_recipient,
        reserve_for,
        finders_fee_percent: finders_fee_bps
            .map(|bps| Decimal::percent(bps) / Uint128::from(100u64)),
        expires,
        paid_removal_fee: None,
    };
    ask.validate(deps.storage, &env.block, &sudo_params)?;

    // Ensure listing fee is paid
    let mut funds_normalized = NativeBalance(info.funds.clone());
    funds_normalized.normalize();

    // Pay listing fee
    if !sudo_params.listing_fee.amount.is_zero() {
        ensure!(
            funds_normalized.has(&sudo_params.listing_fee),
            ContractError::InvalidFunds {
                expected: sudo_params.listing_fee,
            }
        );
        funds_normalized = funds_normalized.sub_saturating(sudo_params.listing_fee.clone())?;
        response = append_fair_burn_msg(
            &sudo_params.fair_burn,
            vec![sudo_params.listing_fee.clone()],
            None,
            response,
        );
    }

    let event = ask.create_event(
        "set-ask",
        vec![
            "collection",
            "token_id",
            "seller",
            "price",
            "asset_recipient",
            "reserve_for",
            "finders_fee_percent",
            "expires",
            "paid_removal_fee",
        ],
    );

    let ask_hook_message = prepare_ask_hook(deps.as_ref(), &ask, HookAction::Create)?;

    response = set_ask(
        deps,
        env,
        info,
        &sudo_params,
        funds_normalized,
        ask,
        response,
    )?;

    response = response.add_event(event).add_submessages(ask_hook_message);

    Ok(response)
}

/// Updates the ask price on a particular NFT
pub fn execute_update_ask_price(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
    price: Coin,
) -> Result<Response, ContractError> {
    let mut response = Response::new();
    let sudo_params = SUDO_PARAMS.load(deps.storage)?;

    let ask_key = Ask::build_key(&collection, &token_id);
    let mut ask = asks().load(deps.storage, ask_key.clone())?;

    ensure!(!ask.is_expired(&env.block), ContractError::AskExpired {});

    ask.price = price;
    ask.validate(deps.storage, &env.block, &sudo_params)?;

    let mut funds_normalized = NativeBalance(info.funds.clone());
    funds_normalized.normalize();

    let event = ask.create_event(
        "update-ask",
        vec!["collection", "token_id", "price", "paid_removal_fee"],
    );

    let ask_hook_message = prepare_ask_hook(deps.as_ref(), &ask, HookAction::Update)?;

    response = set_ask(
        deps.branch(),
        env,
        info,
        &sudo_params,
        funds_normalized,
        ask,
        response,
    )?;

    response = response.add_event(event).add_submessages(ask_hook_message);

    Ok(response)
}

fn remove_ask(
    deps: DepsMut,
    _info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
    mut response: Response,
) -> Result<Response, ContractError> {
    let ask_key = Ask::build_key(&collection, &token_id);
    let ask = asks().load(deps.storage, ask_key.clone())?;
    asks().remove(deps.storage, ask_key)?;

    // Refund paid removal fee
    if let Some(paid_removal_fee) = &ask.paid_removal_fee {
        response = response.add_submessage(transfer_coin(paid_removal_fee.clone(), &ask.seller));
    }

    response = response.add_event(ask.create_event("remove-ask", vec!["collection", "token_id"]));

    // Prepare ask delete hook
    response = response.add_submessages(prepare_ask_hook(deps.as_ref(), &ask, HookAction::Delete)?);

    Ok(response)
}

/// Removes the ask on a particular NFT
pub fn execute_remove_ask(
    deps: DepsMut,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_owner(&deps.querier, &info, &collection, &token_id)?;

    let mut response = Response::new();

    response = remove_ask(deps, info, collection, token_id, response)?;

    Ok(response)
}

/// Privileged operation to remove a stale ask. Operators can call this to remove asks that are still in the
/// state after they have expired or a token is no longer existing.
pub fn execute_remove_stale_ask(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let sudo_params = SUDO_PARAMS.load(deps.storage)?;
    ensure!(
        sudo_params.operators.contains(&info.sender),
        ContractError::UnauthorizedOperator {}
    );

    let ask_key = Ask::build_key(&collection, &token_id);
    let ask = asks().load(deps.storage, ask_key)?;

    let is_stale = match owner_of(&deps.querier, &collection, &token_id) {
        Ok(OwnerOfResponse { owner, approvals }) => {
            let seller_is_owner = ask.seller.to_string() == owner;
            let contract_is_approved = approvals
                .iter()
                .find(|approval| approval.spender == env.contract.address.as_str())
                .is_some();
            let ask_is_expired = ask.is_expired(&env.block);

            !seller_is_owner || !contract_is_approved || ask_is_expired
        }
        Err(_) => true,
    };

    ensure!(is_stale, ContractError::AskUnchanged {});

    let mut response = Response::new();
    response = remove_ask(deps, info, collection, token_id, response)?;

    Ok(response)
}

pub fn execute_migrate_asks(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    limit: u64,
) -> Result<Response, ContractError> {
    let response = Response::new();

    let prev_asks = asks_dep()
        .range(deps.storage, None, None, Order::Ascending)
        .take(limit as usize)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<_>>>()?;

    let mut event = Event::new("migrate-asks");

    for ask in prev_asks {
        asks_dep().remove(deps.storage, ask_key_dep(&ask.collection, ask.token_id))?;

        let new_ask = Ask {
            collection: ask.collection,
            token_id: ask.token_id.to_string(),
            seller: ask.seller,
            price: coin(ask.price.u128(), NATIVE_DENOM),
            asset_recipient: ask.funds_recipient,
            reserve_for: ask.reserve_for,
            finders_fee_percent: ask.finders_fee_bps.map(bps_to_decimal),
            expires: Some(ask.expires_at),
            paid_removal_fee: None,
        };

        // If new ask already exists, skip it
        if asks().has(deps.storage, new_ask.key()) {
            continue;
        }

        asks().save(deps.storage, new_ask.key(), &new_ask)?;
        event = event.add_attribute("migrate-ask", new_ask.key_to_str());
    }

    Ok(response.add_event(event))
}

/// Places a bid on a listed or unlisted NFT. The bid is escrowed in the contract.
pub fn execute_set_offer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
    asset_recipient: Option<Addr>,
    finder: Option<Addr>,
    finders_fee_bps: Option<u64>,
    expires: Option<Timestamp>,
    buy_now: bool,
) -> Result<Response, ContractError> {
    let sudo_params = SUDO_PARAMS.load(deps.storage)?;

    // Validate finder address
    if let Some(finder) = &finder {
        ensure!(
            finder != &info.sender,
            ContractError::InvalidFinder("finder cannot be bidder".to_string())
        );
    }

    let offer_coin = one_coin(&info)?;
    let offer = Offer {
        collection,
        token_id,
        bidder: info.sender,
        price: offer_coin,
        asset_recipient,
        finders_fee_percent: finders_fee_bps.map(bps_to_decimal),
        expires,
    };
    offer.validate(deps.storage, &env.block, &sudo_params)?;

    let mut response = Response::new();
    if let Some(existing_offer) = offers().may_load(deps.storage, offer.key().clone())? {
        offers().remove(deps.storage, offer.key())?;
        response =
            response.add_submessage(transfer_coin(existing_offer.price, &existing_offer.bidder));
    }

    let match_result = match_offer(deps.as_ref(), &env.block, &offer)?;
    response = response
        .add_event(Event::new("match").add_attribute("match-result", match_result.to_string()));

    match match_result {
        MatchResult::Match(ask) => {
            // Found a matching Ask,
            // Remove the ask
            asks().remove(deps.storage, ask.key())?;

            // Ensure seller still owns NFT
            let owner = owner_of(&deps.querier, &ask.collection, &ask.token_id)?.owner;
            ensure!(
                owner == ask.seller.to_string(),
                ContractError::InvalidListing {}
            );

            response = finalize_sale(
                deps.as_ref(),
                &offer.collection,
                &offer.token_id,
                &ask.seller,
                &ask.asset_recipient(),
                &offer.bidder,
                &offer.asset_recipient(),
                &ask.price,
                &sudo_params,
                finder.as_ref(),
                ask.finders_fee_percent,
                response,
            )?;

            let bidder_refund = offer.price.amount - ask.price.amount;
            if !bidder_refund.is_zero() {
                response = response.add_submessage(transfer_coin(
                    coin(bidder_refund.u128(), offer.price.denom),
                    &offer.bidder,
                ));
            }
        }
        MatchResult::NotMatch(_reason) => {
            // Offer has not matching ask

            // If buy_now is true, then throw an error when no matching ask is found
            ensure!(!buy_now, ContractError::ItemNotForSale {});

            // Store the offer and add offer hook to response
            offers().save(deps.storage, offer.key(), &offer)?;

            response = response.add_submessages(prepare_offer_hook(
                deps.as_ref(),
                &offer,
                HookAction::Create,
            )?);

            response = response.add_event(offer.create_event(
                "set-offer",
                vec![
                    "collection",
                    "token_id",
                    "bidder",
                    "price",
                    "asset_recipient",
                    "finders_fee_percent",
                    "expires",
                ],
            ));
        }
    }

    Ok(response)
}

/// Seller can accept a bid which transfers funds as well as the token. The bid may or may not be associated with an ask.
pub fn execute_accept_offer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
    bidder: Addr,
    asset_recipient: Option<Addr>,
    finder: Option<Addr>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    // Validate that only the owner of an NFT can accept a bid,
    // and that the collection is tradable.
    only_owner(&deps.querier, &info, &collection, &token_id)?;
    only_tradable(deps.as_ref(), &env.block, &collection)?;

    let sudo_params = SUDO_PARAMS.load(deps.storage)?;

    let mut response = Response::new();

    // Remove the ask if it exists, issue refund if necessary.
    let ask_key = Ask::build_key(&collection, &token_id);
    let ask_option = asks().may_load(deps.storage, ask_key.clone())?;
    if let Some(ask) = ask_option {
        if let Some(paid_removal_fee) = ask.paid_removal_fee {
            response = response.add_submessage(transfer_coin(paid_removal_fee, &ask.seller));
        }
        asks().remove(deps.storage, ask_key)?;
    }

    let offer_key = Offer::build_key(&collection, &token_id, &bidder);

    let offer = offers().load(deps.storage, offer_key.clone())?;
    if offer.is_expired(&env.block) {
        return Err(ContractError::BidExpired {});
    }

    // Remove accepted offer
    offers().remove(deps.storage, offer_key)?;

    // Transfer funds and NFT
    let seller_recipient = address_or(asset_recipient.as_ref(), &info.sender);
    response = finalize_sale(
        deps.as_ref(),
        &collection,
        &token_id,
        &info.sender,
        &seller_recipient,
        &offer.bidder,
        &offer.asset_recipient(),
        &offer.price,
        &sudo_params,
        finder.as_ref(),
        offer.finders_fee_percent,
        response,
    )?;

    response = response.add_event(offer.create_event(
        "accept-offer",
        vec!["collection", "token_id", "bidder", "price"],
    ));

    Ok(response)
}

fn remove_offer(
    deps: DepsMut,
    offer: Offer,
    mut response: Response,
) -> Result<Response, ContractError> {
    offers().remove(deps.storage, offer.key())?;

    // Refund bidder
    response = response.add_submessage(transfer_coin(offer.price.clone(), &offer.bidder));

    response = response.add_submessages(prepare_offer_hook(
        deps.as_ref(),
        &offer,
        HookAction::Delete,
    )?);

    response = response
        .add_event(offer.create_event("remove-offer", vec!["collection", "token_id", "bidder"]));

    Ok(response)
}

/// Removes a bid made by the bidder. Bidders can only remove their own bids
pub fn execute_remove_offer(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let offer_key = Offer::build_key(&collection, &token_id, &info.sender);
    let offer = offers().load(deps.storage, offer_key.clone())?;

    let mut response = Response::new();

    response = remove_offer(deps, offer, response)?;

    Ok(response)
}

/// Removes a bid made by the bidder. Only NFT owners can remove a bid made by another bidder
pub fn execute_reject_offer(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
    bidder: Addr,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_owner(&deps.querier, &info, &collection, &token_id)?;

    let offer_key = Offer::build_key(&collection, &token_id, &bidder);
    let offer = offers().load(deps.storage, offer_key.clone())?;

    let mut response = Response::new();

    response = remove_offer(deps, offer, response)?;

    Ok(response)
}

pub fn execute_remove_stale_offer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
    bidder: Addr,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let sudo_params = SUDO_PARAMS.load(deps.storage)?;
    ensure!(
        sudo_params.operators.contains(&info.sender),
        ContractError::UnauthorizedOperator {}
    );

    let offer_key = Offer::build_key(&collection, &token_id, &bidder);
    let offer = offers().load(deps.storage, offer_key.clone())?;

    // Ensure offer is expired
    ensure!(offer.is_expired(&env.block), ContractError::BidNotStale {});

    let mut response = Response::new();

    response = remove_offer(deps, offer, response)?;

    Ok(response)
}

pub fn execute_migrate_offers(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    limit: u64,
) -> Result<Response, ContractError> {
    let mut response = Response::new();

    let prev_bids = bids_dep()
        .range(deps.storage, None, None, Order::Ascending)
        .take(limit as usize)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<_>>>()?;

    let mut event = Event::new("migrate-offers");

    for bid in prev_bids {
        bids_dep().remove(
            deps.storage,
            bid_key_dep(&bid.collection, bid.token_id, &bid.bidder),
        )?;
        let new_offer = Offer {
            collection: bid.collection,
            token_id: bid.token_id.to_string(),
            bidder: bid.bidder.clone(),
            price: coin(bid.price.u128(), NATIVE_DENOM),
            asset_recipient: None,
            finders_fee_percent: bid.finders_fee_bps.map(bps_to_decimal),
            expires: Some(bid.expires_at),
        };

        // If new offer already exists, refund previous offer
        if offers().has(deps.storage, new_offer.key()) {
            response = response.add_submessage(transfer_coin(
                coin(bid.price.u128(), NATIVE_DENOM),
                &bid.bidder,
            ));
            continue;
        }

        offers().save(deps.storage, new_offer.key(), &new_offer)?;
        event = event.add_attribute("migrate-offer", new_offer.key_to_str());
    }

    Ok(response.add_event(event))
}

/// Place an offer (limit order) across an entire collection
pub fn execute_set_collection_offer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    asset_recipient: Option<Addr>,
    finders_fee_bps: Option<u64>,
    expires: Option<Timestamp>,
) -> Result<Response, ContractError> {
    let sudo_params = SUDO_PARAMS.load(deps.storage)?;

    let collection_offer_coin = one_coin(&info)?;
    let collection_offer = CollectionOffer {
        collection,
        bidder: info.sender,
        price: collection_offer_coin,
        asset_recipient,
        finders_fee_percent: finders_fee_bps.map(bps_to_decimal),
        expires,
    };
    collection_offer.validate(deps.storage, &env.block, &sudo_params)?;

    let mut response = Response::new();

    let existing_collection_offer =
        collection_offers().may_load(deps.storage, collection_offer.key())?;
    if let Some(existing_collection_offer) = existing_collection_offer {
        collection_offers().remove(deps.storage, collection_offer.key())?;
        response = response.add_submessage(transfer_coin(
            existing_collection_offer.price,
            &existing_collection_offer.bidder,
        ));
    }

    collection_offers().save(deps.storage, collection_offer.key(), &collection_offer)?;

    response = response.add_submessages(prepare_collection_offer_hook(
        deps.as_ref(),
        &collection_offer,
        HookAction::Create,
    )?);

    response = response.add_event(collection_offer.create_event(
        "set-collection-offer",
        vec![
            "collection",
            "bidder",
            "price",
            "asset_recipient",
            "finders_fee_percent",
            "expires",
        ],
    ));

    Ok(response)
}

/// Owner/seller of an item in a collection can accept a collection bid which transfers funds as well as a token
pub fn execute_accept_collection_offer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
    bidder: Addr,
    asset_recipient: Option<Addr>,
    finder: Option<Addr>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_owner(&deps.querier, &info, &collection, &token_id)?;
    only_tradable(deps.as_ref(), &env.block, &collection)?;

    let sudo_params = SUDO_PARAMS.load(deps.storage)?;

    let mut response = Response::new();

    // Remove the ask if it exists, issue refund if necessary.
    let ask_key = Ask::build_key(&collection, &token_id);
    let ask_option = asks().may_load(deps.storage, ask_key.clone())?;
    if let Some(ask) = ask_option {
        if let Some(paid_removal_fee) = ask.paid_removal_fee {
            response = response.add_submessage(transfer_coin(paid_removal_fee, &ask.seller));
        }
        asks().remove(deps.storage, ask_key)?;
    }

    let collection_offer_key = CollectionOffer::build_key(&collection, &bidder);
    let collection_offer = collection_offers().load(deps.storage, collection_offer_key)?;
    if collection_offer.is_expired(&env.block) {
        return Err(ContractError::BidExpired {});
    }

    // Transfer funds and NFT
    let seller_recipient = address_or(asset_recipient.as_ref(), &info.sender);
    response = finalize_sale(
        deps.as_ref(),
        &collection,
        &token_id,
        &info.sender,
        &seller_recipient,
        &collection_offer.bidder,
        &collection_offer.asset_recipient(),
        &collection_offer.price,
        &sudo_params,
        finder.as_ref(),
        collection_offer.finders_fee_percent,
        response,
    )?;

    let event = collection_offer
        .create_event(
            "accept-collection-offer",
            vec!["collection", "bidder", "price"],
        )
        .add_attributes(vec![
            attr("seller", info.sender),
            attr("token_id", token_id),
        ]);

    Ok(response.add_event(event))
}

fn remove_collection_offer(
    deps: DepsMut,
    collection_offer: CollectionOffer,
    mut response: Response,
) -> Result<Response, ContractError> {
    collection_offers().remove(deps.storage, collection_offer.key())?;

    // Refund bidder
    response = response.add_submessage(transfer_coin(
        collection_offer.price.clone(),
        &collection_offer.bidder,
    ));

    response = response.add_submessages(prepare_collection_offer_hook(
        deps.as_ref(),
        &collection_offer,
        HookAction::Delete,
    )?);

    response = response.add_event(
        collection_offer.create_event("remove-collection-offer", vec!["collection", "bidder"]),
    );

    Ok(response)
}

/// Removes a bid made by the bidder. Bidders can only remove their own bids
pub fn execute_remove_collection_offer(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    collection: Addr,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let collection_offer_key = CollectionOffer::build_key(&collection, &info.sender);
    let collection_offer = collection_offers().load(deps.storage, collection_offer_key.clone())?;

    let mut response = Response::new();

    response = remove_collection_offer(deps, collection_offer, response)?;

    Ok(response)
}

/// Remove an existing collection bid (limit order)
pub fn execute_remove_stale_collection_offer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    bidder: Addr,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let sudo_params = SUDO_PARAMS.load(deps.storage)?;
    ensure!(
        sudo_params.operators.contains(&info.sender),
        ContractError::UnauthorizedOperator {}
    );

    let collection_offer_key = CollectionOffer::build_key(&collection, &bidder);
    let collection_offer = collection_offers().load(deps.storage, collection_offer_key.clone())?;

    // Ensure collection offer is expired
    ensure!(
        collection_offer.is_expired(&env.block),
        ContractError::BidNotStale {}
    );

    let mut response = Response::new();

    response = remove_collection_offer(deps, collection_offer, response)?;

    Ok(response)
}

pub fn execute_migrate_collection_offers(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    limit: u64,
) -> Result<Response, ContractError> {
    let mut response = Response::new();

    let prev_collection_bids = collection_bids_dep()
        .range(deps.storage, None, None, Order::Ascending)
        .take(limit as usize)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<_>>>()?;

    let mut event = Event::new("migrate-collection-offers");

    for collection_bid in prev_collection_bids {
        collection_bids_dep().remove(
            deps.storage,
            collection_bid_key_dep(&collection_bid.collection, &collection_bid.bidder),
        )?;
        let new_collection_offer = CollectionOffer {
            collection: collection_bid.collection,
            bidder: collection_bid.bidder.clone(),
            price: coin(collection_bid.price.u128(), NATIVE_DENOM),
            asset_recipient: None,
            finders_fee_percent: collection_bid.finders_fee_bps.map(bps_to_decimal),
            expires: Some(collection_bid.expires_at),
        };

        // If new collection offer already exists, refund bidder
        if collection_offers().has(deps.storage, new_collection_offer.key()) {
            response = response.add_submessage(transfer_coin(
                coin(collection_bid.price.u128(), NATIVE_DENOM),
                &collection_bid.bidder,
            ));
            continue;
        }

        collection_offers().save(
            deps.storage,
            new_collection_offer.key(),
            &new_collection_offer,
        )?;
        event = event.add_attribute(
            "migrate-collection-offer",
            new_collection_offer.key_to_str(),
        );
    }

    Ok(response.add_event(event))
}
