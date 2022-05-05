use crate::error::ContractError;
use crate::helpers::map_validate;
use crate::msg::{
    AskHookMsg, BidHookMsg, CollectionBidHookMsg, ExecuteMsg, HookAction, InstantiateMsg,
    SaleHookMsg,
};
use crate::state::{
    ask_key, asks, bid_key, bids, collection_bid_key, collection_bids, Ask, Bid, CollectionBid,
    SaleType, SudoParams, TokenId, ASK_HOOKS, BID_HOOKS, COLLECTION_BID_HOOKS, SALE_HOOKS,
    SUDO_PARAMS,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, to_binary, Addr, BankMsg, Coin, Decimal, Deps, DepsMut, Env, Event, MessageInfo, Reply,
    StdResult, Storage, Timestamp, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw721::{Cw721ExecuteMsg, OwnerOfResponse};
use cw721_base::helpers::Cw721Contract;
use cw_utils::{maybe_addr, must_pay, nonpayable, Expiration};
use sg1::fair_burn;
use sg721::msg::{CollectionInfoResponse, QueryMsg as Sg721QueryMsg};
use sg_std::{Response, SubMsg, NATIVE_DENOM};

// Version info for migration info
const CONTRACT_NAME: &str = "crates.io:sg-marketplace";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    msg.ask_expiry.validate()?;
    msg.bid_expiry.validate()?;

    let params = SudoParams {
        trading_fee_percent: Decimal::percent(msg.trading_fee_bps),
        ask_expiry: msg.ask_expiry,
        bid_expiry: msg.bid_expiry,
        operators: map_validate(deps.api, &msg.operators)?,
        max_finders_fee_percent: Decimal::percent(msg.max_finders_fee_bps),
        min_price: msg.min_price,
        stale_bid_duration: msg.stale_bid_duration,
        bid_removal_reward_percent: Decimal::percent(msg.bid_removal_reward_bps),
    };
    SUDO_PARAMS.save(deps.storage, &params)?;

    if let Some(hook) = msg.sale_hook {
        SALE_HOOKS.add_hook(deps.storage, deps.api.addr_validate(&hook)?)?;
    }

    Ok(Response::new())
}

pub struct AskInfo {
    sale_type: SaleType,
    collection: Addr,
    token_id: TokenId,
    price: Coin,
    funds_recipient: Option<Addr>,
    reserve_for: Option<Addr>,
    finders_fee_bps: Option<u64>,
    expires: Timestamp,
}

pub struct BidInfo {
    collection: Addr,
    token_id: TokenId,
    expires: Timestamp,
    finder: Option<Addr>,
    finders_fee_bps: Option<u64>,
}

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
            sale_type,
            collection,
            token_id,
            price,
            funds_recipient,
            reserve_for,
            finders_fee_bps,
            expires,
        } => execute_set_ask(
            deps,
            env,
            info,
            AskInfo {
                sale_type,
                collection: api.addr_validate(&collection)?,
                token_id,
                price,
                funds_recipient: maybe_addr(api, funds_recipient)?,
                reserve_for: maybe_addr(api, reserve_for)?,
                finders_fee_bps,
                expires,
            },
        ),
        ExecuteMsg::RemoveAsk {
            collection,
            token_id,
        } => execute_remove_ask(deps, info, api.addr_validate(&collection)?, token_id),
        ExecuteMsg::UpdateAskIsActive {
            collection,
            token_id,
        } => execute_update_ask_is_active(deps, info, api.addr_validate(&collection)?, token_id),
        ExecuteMsg::SetBid {
            collection,
            token_id,
            expires,
            finder,
            finders_fee_bps,
        } => execute_set_bid(
            deps,
            env,
            info,
            BidInfo {
                collection: api.addr_validate(&collection)?,
                token_id,
                expires,
                finder: maybe_addr(api, finder)?,
                finders_fee_bps,
            },
        ),
        ExecuteMsg::RemoveBid {
            collection,
            token_id,
        } => execute_remove_bid(deps, env, info, api.addr_validate(&collection)?, token_id),
        ExecuteMsg::AcceptBid {
            collection,
            token_id,
            bidder,
            finder,
        } => execute_accept_bid(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            token_id,
            api.addr_validate(&bidder)?,
            maybe_addr(api, finder)?,
        ),
        ExecuteMsg::UpdateAskPrice {
            collection,
            token_id,
            price,
        } => execute_update_ask_price(deps, info, api.addr_validate(&collection)?, token_id, price),
        ExecuteMsg::SetCollectionBid {
            collection,
            expires,
            finders_fee_bps,
        } => execute_set_collection_bid(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            finders_fee_bps,
            expires,
        ),
        ExecuteMsg::RemoveCollectionBid { collection } => {
            execute_remove_collection_bid(deps, env, info, api.addr_validate(&collection)?)
        }
        ExecuteMsg::AcceptCollectionBid {
            collection,
            token_id,
            bidder,
            finder,
        } => execute_accept_collection_bid(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            token_id,
            api.addr_validate(&bidder)?,
            maybe_addr(api, finder)?,
        ),
        ExecuteMsg::RemoveStaleBid {
            collection,
            token_id,
            bidder,
        } => execute_remove_stale_bid(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            token_id,
            api.addr_validate(&bidder)?,
        ),
        ExecuteMsg::RemoveStaleCollectionBid { collection, bidder } => {
            execute_remove_stale_collection_bid(
                deps,
                env,
                info,
                api.addr_validate(&collection)?,
                api.addr_validate(&bidder)?,
            )
        }
    }
}

/// A seller may set an Ask on their NFT to list it on Marketplace
pub fn execute_set_ask(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    ask_info: AskInfo,
) -> Result<Response, ContractError> {
    let AskInfo {
        sale_type,
        collection,
        token_id,
        price,
        funds_recipient,
        reserve_for,
        finders_fee_bps,
        expires,
    } = ask_info;

    nonpayable(&info)?;
    price_validate(deps.storage, &price)?;
    only_owner(deps.as_ref(), &info, collection.clone(), token_id)?;

    // Check if this contract is approved to transfer the token
    Cw721Contract(collection.clone()).approval(
        &deps.querier,
        token_id.to_string(),
        env.contract.address.to_string(),
        None,
    )?;

    let params = SUDO_PARAMS.load(deps.storage)?;
    params.ask_expiry.is_valid(&env.block, expires)?;

    if let Some(fee) = finders_fee_bps {
        if Decimal::percent(fee) > params.max_finders_fee_percent {
            return Err(ContractError::InvalidFindersFeeBps(fee));
        };
    }

    let seller = info.sender;
    let ask = Ask {
        sale_type,
        collection: collection.clone(),
        token_id,
        seller: seller.clone(),
        price: price.amount,
        funds_recipient,
        reserve_for,
        finders_fee_bps,
        expires_at: expires,
        is_active: true,
    };
    store_ask(deps.storage, &ask)?;

    let hook = prepare_ask_hook(deps.as_ref(), &ask, HookAction::Create)?;

    let event = Event::new("set-ask")
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("seller", seller)
        .add_attribute("price", price.to_string())
        .add_attribute("expires", expires.to_string());

    Ok(Response::new().add_submessages(hook).add_event(event))
}

/// Removes the ask on a particular NFT
pub fn execute_remove_ask(
    deps: DepsMut,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_owner(deps.as_ref(), &info, collection.clone(), token_id)?;

    let key = ask_key(collection.clone(), token_id);
    let ask = asks().load(deps.storage, key.clone())?;
    asks().remove(deps.storage, key)?;

    let hook = prepare_ask_hook(deps.as_ref(), &ask, HookAction::Delete)?;

    let event = Event::new("remove-ask")
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id.to_string());

    Ok(Response::new().add_event(event).add_submessages(hook))
}

/// Updates the the active state of the ask.
/// This is a privileged operation called by an operator to update the active state of an Ask
/// when an NFT transfer happens.
pub fn execute_update_ask_is_active(
    deps: DepsMut,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_operator(deps.storage, &info)?;

    let mut ask = asks().load(deps.storage, ask_key(collection.clone(), token_id))?;
    let res = only_owner(deps.as_ref(), &info, collection.clone(), token_id)?;
    let new_is_active = res.owner == ask.seller;
    if new_is_active == ask.is_active {
        return Err(ContractError::AskUnchanged {});
    }
    ask.is_active = new_is_active;
    asks().save(deps.storage, ask_key(collection.clone(), token_id), &ask)?;

    let hook = prepare_ask_hook(deps.as_ref(), &ask, HookAction::Update)?;

    let event = Event::new("update-ask-state")
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("is_active", ask.is_active.to_string());

    Ok(Response::new().add_event(event).add_submessages(hook))
}

/// Updates the ask price on a particular NFT
pub fn execute_update_ask_price(
    deps: DepsMut,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
    price: Coin,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_owner(deps.as_ref(), &info, collection.clone(), token_id)?;
    price_validate(deps.storage, &price)?;

    let mut ask = asks().load(deps.storage, ask_key(collection.clone(), token_id))?;
    ask.price = price.amount;
    asks().save(deps.storage, ask_key(collection.clone(), token_id), &ask)?;

    let hook = prepare_ask_hook(deps.as_ref(), &ask, HookAction::Update)?;

    let event = Event::new("update-ask")
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("price", price.to_string());

    Ok(Response::new().add_event(event).add_submessages(hook))
}

/// Places a bid on a listed or unlisted NFT. The bid is escrowed in the contract.
pub fn execute_set_bid(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    bid_info: BidInfo,
) -> Result<Response, ContractError> {
    let BidInfo {
        collection,
        token_id,
        finders_fee_bps,
        expires,
        finder,
    } = bid_info;

    let params = SUDO_PARAMS.load(deps.storage)?;
    let bid_price = must_pay(&info, NATIVE_DENOM)?;
    if bid_price < params.min_price {
        return Err(ContractError::PriceTooSmall(bid_price));
    }
    params.bid_expiry.is_valid(&env.block, expires)?;
    let bidder = info.sender;

    let mut res = Response::new();

    let existing_bid =
        bids().may_load(deps.storage, (collection.clone(), token_id, bidder.clone()))?;

    if let Some(existing_bid) = existing_bid {
        bids().remove(deps.storage, (collection.clone(), token_id, bidder.clone()))?;
        let refund_bidder = BankMsg::Send {
            to_address: bidder.to_string(),
            amount: vec![coin(existing_bid.price.u128(), NATIVE_DENOM)],
        };
        res = res.add_message(refund_bidder)
    }

    let ask = asks().may_load(deps.storage, ask_key(collection.clone(), token_id))?;
    let bid: Option<Bid> = match ask {
        Some(ask) => {
            if ask.expires_at <= env.block.time {
                return Err(ContractError::AskExpired {});
            }
            if !ask.is_active {
                return Err(ContractError::AskNotActive {});
            }
            if let Some(reserved_for) = ask.clone().reserve_for {
                if reserved_for != bidder {
                    return Err(ContractError::TokenReserved {});
                }
            }
            let inner_bid: Option<Bid> = match ask.sale_type {
                SaleType::FixedPrice => {
                    if ask.price != bid_price {
                        return Err(ContractError::InvalidPrice {});
                    } else {
                        asks().remove(deps.storage, ask_key(collection.clone(), token_id))?;
                        finalize_sale(
                            deps.as_ref(),
                            ask,
                            bid_price,
                            bidder.clone(),
                            finder,
                            &mut res,
                        )?;
                    }
                    None
                }
                SaleType::Auction => {
                    let bid = Bid::new(
                        collection.clone(),
                        token_id,
                        bidder.clone(),
                        bid_price,
                        finders_fee_bps,
                        expires,
                    );
                    store_bid(deps.storage, &bid)?;
                    Some(bid)
                }
            };
            inner_bid
        }
        None => {
            let bid = Bid::new(
                collection.clone(),
                token_id,
                bidder.clone(),
                bid_price,
                finders_fee_bps,
                expires,
            );
            store_bid(deps.storage, &bid)?;
            Some(bid)
        }
    };

    let hook = if let Some(bid) = bid {
        prepare_bid_hook(deps.as_ref(), &bid, HookAction::Create)?
    } else {
        vec![]
    };

    let event = Event::new("set-bid")
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("bidder", bidder)
        .add_attribute("bid_price", bid_price.to_string())
        .add_attribute("expires", expires.to_string());

    Ok(res.add_submessages(hook).add_event(event))
}

/// Removes a bid made by the bidder. Bidders can only remove their own bids
pub fn execute_remove_bid(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let bidder = info.sender;

    // Check bid exists for bidder
    let bid = bids().load(
        deps.storage,
        bid_key(collection.clone(), token_id, bidder.clone()),
    )?;

    let hook = prepare_bid_hook(deps.as_ref(), &bid, HookAction::Delete)?;

    let event = Event::new("remove-bid")
        .add_attribute("collection", collection)
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("bidder", bidder);

    let res = Response::new()
        .add_message(remove_and_refund_bid(deps.storage, bid)?)
        .add_event(event)
        .add_submessages(hook);

    Ok(res)
}

fn remove_and_refund_bid(store: &mut dyn Storage, bid: Bid) -> Result<BankMsg, ContractError> {
    // Remove bid
    bids().remove(store, (bid.collection, bid.token_id, bid.bidder.clone()))?;

    // Refund bidder
    let msg = BankMsg::Send {
        to_address: bid.bidder.to_string(),
        amount: vec![coin(bid.price.u128(), NATIVE_DENOM)],
    };

    Ok(msg)
}

/// Seller can accept a bid which transfers funds as well as the token. The bid may or may not be associated with an ask.
pub fn execute_accept_bid(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
    bidder: Addr,
    finder: Option<Addr>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_owner(deps.as_ref(), &info, collection.clone(), token_id)?;

    let bid = bids().load(deps.storage, (collection.clone(), token_id, bidder.clone()))?;
    if bid.expires_at <= env.block.time {
        return Err(ContractError::BidExpired {});
    }

    let ask = match asks().may_load(deps.storage, ask_key(collection.clone(), token_id))? {
        Some(existing_ask) => {
            if existing_ask.expires_at <= env.block.time {
                return Err(ContractError::AskExpired {});
            }
            if !existing_ask.is_active {
                return Err(ContractError::AskNotActive {});
            }
            asks().remove(deps.storage, ask_key(collection.clone(), token_id))?;
            existing_ask
        }
        None => {
            // Create a temporary Ask
            Ask {
                sale_type: SaleType::Auction,
                collection: collection.clone(),
                token_id,
                price: bid.price,
                expires_at: bid.expires_at,
                is_active: true,
                seller: info.sender,
                funds_recipient: None,
                reserve_for: None,
                finders_fee_bps: bid.finders_fee_bps,
            }
        }
    };

    // Remove accepted bid
    bids().remove(deps.storage, (collection.clone(), token_id, bidder.clone()))?;

    let mut res = Response::new();

    // Transfer funds and NFT
    finalize_sale(
        deps.as_ref(),
        ask,
        bid.price,
        bidder.clone(),
        finder,
        &mut res,
    )?;

    let event = Event::new("accept-bid")
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("bidder", bidder)
        .add_attribute("price", bid.price.to_string());

    Ok(res.add_event(event))
}

/// Place a collection bid (limit order) across an entire collection
pub fn execute_set_collection_bid(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    finders_fee_bps: Option<u64>,
    expires: Timestamp,
) -> Result<Response, ContractError> {
    let params = SUDO_PARAMS.load(deps.storage)?;
    let price = must_pay(&info, NATIVE_DENOM)?;
    if price < params.min_price {
        return Err(ContractError::PriceTooSmall(price));
    }
    params.bid_expiry.is_valid(&env.block, expires)?;

    let bidder = info.sender;
    let mut res = Response::new();

    // Check bidder has existing bid, if so remove existing bid
    let existing_bid = collection_bids().may_load(
        deps.storage,
        collection_bid_key(collection.clone(), bidder.clone()),
    )?;
    if let Some(existing_bid) = existing_bid {
        res = res.add_message(remove_and_refund_collection_bid(
            deps.storage,
            existing_bid,
        )?);
    }

    let collection_bid = CollectionBid {
        collection: collection.clone(),
        bidder: bidder.clone(),
        price,
        finders_fee_bps,
        expires_at: expires,
    };
    collection_bids().save(
        deps.storage,
        collection_bid_key(collection.clone(), bidder.clone()),
        &collection_bid,
    )?;

    let hook = prepare_collection_bid_hook(deps.as_ref(), &collection_bid, HookAction::Create)?;

    let event = Event::new("set-collection-bid")
        .add_attribute("collection", collection.to_string())
        .add_attribute("bidder", bidder)
        .add_attribute("bid_price", price.to_string())
        .add_attribute("expires", expires.to_string());

    Ok(res.add_event(event).add_submessages(hook))
}

/// Remove an existing collection bid (limit order)
pub fn execute_remove_collection_bid(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    collection: Addr,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let bidder = info.sender;

    // Check bidder has existing bid, if so remove existing bid
    let collection_bid = collection_bids().load(
        deps.storage,
        collection_bid_key(collection.clone(), bidder.clone()),
    )?;

    let hook = prepare_collection_bid_hook(deps.as_ref(), &collection_bid, HookAction::Delete)?;

    let event = Event::new("remove-collection-bid")
        .add_attribute("collection", collection.to_string())
        .add_attribute("bidder", bidder);

    let res = Response::new()
        .add_message(remove_and_refund_collection_bid(
            deps.storage,
            collection_bid,
        )?)
        .add_event(event)
        .add_submessages(hook);

    Ok(res)
}

fn remove_and_refund_collection_bid(
    store: &mut dyn Storage,
    collection_bid: CollectionBid,
) -> Result<BankMsg, ContractError> {
    // Remove bid
    collection_bids().remove(
        store,
        (collection_bid.collection, collection_bid.bidder.clone()),
    )?;

    // Refund bidder
    let msg = BankMsg::Send {
        to_address: collection_bid.bidder.to_string(),
        amount: vec![coin(collection_bid.price.u128(), NATIVE_DENOM)],
    };

    Ok(msg)
}

/// Owner/seller of an item in a collection can accept a collection bid which transfers funds as well as a token
pub fn execute_accept_collection_bid(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
    bidder: Addr,
    finder: Option<Addr>,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_owner(deps.as_ref(), &info, collection.clone(), token_id)?;

    let bid = collection_bids().load(
        deps.storage,
        collection_bid_key(collection.clone(), bidder.clone()),
    )?;
    if bid.expires_at <= env.block.time {
        return Err(ContractError::BidExpired {});
    }

    collection_bids().remove(
        deps.storage,
        collection_bid_key(collection.clone(), bidder.clone()),
    )?;

    let mut res = Response::new();

    // Create a temporary Ask
    let ask = Ask {
        sale_type: SaleType::Auction,
        collection: collection.clone(),
        token_id,
        price: bid.price,
        expires_at: bid.expires_at,
        is_active: true,
        seller: info.sender.clone(),
        funds_recipient: None,
        reserve_for: None,
        finders_fee_bps: bid.finders_fee_bps,
    };

    // Transfer funds and NFT
    finalize_sale(
        deps.as_ref(),
        ask,
        bid.price,
        bidder.clone(),
        finder,
        &mut res,
    )?;

    let event = Event::new("accept-collection-bid")
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("bidder", bidder)
        .add_attribute("seller", info.sender.to_string())
        .add_attribute("price", bid.price.to_string());

    Ok(res.add_event(event))
}

/// Privileged operation to remove a stale bid. Operators can call this to remove and refund bids that are still in the
/// state after they have expired. As a reward they get a governance-determined percentage of the bid price.
pub fn execute_remove_stale_bid(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
    bidder: Addr,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let operator = only_operator(deps.storage, &info)?;

    let bid = bids().load(
        deps.storage,
        bid_key(collection.clone(), token_id, bidder.clone()),
    )?;

    let params = SUDO_PARAMS.load(deps.storage)?;
    let stale_time = (Expiration::AtTime(bid.expires_at) + params.stale_bid_duration)?;
    if !stale_time.is_expired(&env.block) {
        return Err(ContractError::BidNotStale {});
    }

    let params = SUDO_PARAMS.load(deps.storage)?;

    // bid is stale, refund bidder and reward operator
    bids().remove(
        deps.storage,
        (bid.clone().collection, bid.token_id, bid.bidder.clone()),
    )?;

    let reward = bid.price * params.bid_removal_reward_percent / Uint128::from(100u128);

    let bidder_msg = BankMsg::Send {
        to_address: bid.bidder.to_string(),
        amount: vec![coin((bid.price - reward).u128(), NATIVE_DENOM)],
    };
    let operator_msg = BankMsg::Send {
        to_address: operator.to_string(),
        amount: vec![coin(reward.u128(), NATIVE_DENOM)],
    };

    let hook = prepare_bid_hook(deps.as_ref(), &bid, HookAction::Delete)?;

    let event = Event::new("remove-stale-bid")
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("bidder", bidder.to_string())
        .add_attribute("operator", operator.to_string())
        .add_attribute("reward", reward.to_string());

    Ok(Response::new()
        .add_event(event)
        .add_message(bidder_msg)
        .add_message(operator_msg)
        .add_submessages(hook))
}

/// Privileged operation to remove a stale colllection bid. Operators can call this to remove and refund bids that are still in the
/// state after they have expired. As a reward they get a governance-determined percentage of the bid price.
pub fn execute_remove_stale_collection_bid(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    bidder: Addr,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let operator = only_operator(deps.storage, &info)?;

    let collection_bid = collection_bids().load(
        deps.storage,
        collection_bid_key(collection.clone(), bidder.clone()),
    )?;

    let params = SUDO_PARAMS.load(deps.storage)?;
    let stale_time = (Expiration::AtTime(collection_bid.expires_at) + params.stale_bid_duration)?;
    if !stale_time.is_expired(&env.block) {
        return Err(ContractError::BidNotStale {});
    }

    let params = SUDO_PARAMS.load(deps.storage)?;

    // collection bid is stale, refund bidder and reward operator
    collection_bids().remove(
        deps.storage,
        collection_bid_key(
            collection_bid.clone().collection,
            collection_bid.bidder.clone(),
        ),
    )?;

    let reward = collection_bid.price * params.bid_removal_reward_percent / Uint128::from(100u128);

    let bidder_msg = BankMsg::Send {
        to_address: collection_bid.bidder.to_string(),
        amount: vec![coin((collection_bid.price - reward).u128(), NATIVE_DENOM)],
    };
    let operator_msg = BankMsg::Send {
        to_address: operator.to_string(),
        amount: vec![coin(reward.u128(), NATIVE_DENOM)],
    };

    let hook = prepare_collection_bid_hook(deps.as_ref(), &collection_bid, HookAction::Delete)?;

    let event = Event::new("remove-stale-collection-bid")
        .add_attribute("collection", collection.to_string())
        .add_attribute("bidder", bidder.to_string())
        .add_attribute("operator", operator.to_string())
        .add_attribute("reward", reward.to_string());

    Ok(Response::new()
        .add_event(event)
        .add_message(bidder_msg)
        .add_message(operator_msg)
        .add_submessages(hook))
}

/// Transfers funds and NFT, updates bid
fn finalize_sale(
    deps: Deps,
    ask: Ask,
    price: Uint128,
    buyer: Addr,
    finder: Option<Addr>,
    res: &mut Response,
) -> StdResult<()> {
    payout(
        deps,
        ask.collection.clone(),
        price,
        ask.funds_recipient
            .clone()
            .unwrap_or_else(|| ask.seller.clone()),
        finder,
        ask.finders_fee_bps,
        res,
    )?;

    let cw721_transfer_msg = Cw721ExecuteMsg::TransferNft {
        token_id: ask.token_id.to_string(),
        recipient: buyer.to_string(),
    };

    let exec_cw721_transfer = WasmMsg::Execute {
        contract_addr: ask.collection.to_string(),
        msg: to_binary(&cw721_transfer_msg)?,
        funds: vec![],
    };
    res.messages.push(SubMsg::new(exec_cw721_transfer));

    let msg = SaleHookMsg {
        collection: ask.collection.to_string(),
        token_id: ask.token_id,
        price: coin(price.clone().u128(), NATIVE_DENOM),
        seller: ask.seller.to_string(),
        buyer: buyer.to_string(),
    };
    let mut submsgs = SALE_HOOKS.prepare_hooks(deps.storage, |h| {
        let execute = WasmMsg::Execute {
            contract_addr: h.to_string(),
            msg: msg.clone().into_binary()?,
            funds: vec![],
        };
        Ok(SubMsg::reply_on_error(execute, HookReply::Sale as u64))
    })?;
    res.messages.append(&mut submsgs);

    let event = Event::new("finalize-sale")
        .add_attribute("collection", ask.collection.to_string())
        .add_attribute("token_id", ask.token_id.to_string())
        .add_attribute("seller", ask.seller.to_string())
        .add_attribute("buyer", buyer.to_string())
        .add_attribute("price", price.to_string());
    res.events.push(event);

    Ok(())
}

/// Payout a bid
fn payout(
    deps: Deps,
    collection: Addr,
    payment: Uint128,
    payment_recipient: Addr,
    finder: Option<Addr>,
    finders_fee_bps: Option<u64>,
    res: &mut Response,
) -> StdResult<()> {
    let params = SUDO_PARAMS.load(deps.storage)?;

    // Append Fair Burn message
    let network_fee = payment * params.trading_fee_percent / Uint128::from(100u128);
    fair_burn(network_fee.u128(), None, res);

    // Check if token supports royalties
    let collection_info: CollectionInfoResponse = deps
        .querier
        .query_wasm_smart(collection.clone(), &Sg721QueryMsg::CollectionInfo {})?;

    let finders_fee = match finder {
        Some(finder) => {
            let finders_fee = finders_fee_bps
                .map(|fee| (payment * Decimal::percent(fee) / Uint128::from(100u128)).u128())
                .unwrap_or(0);
            if finders_fee > 0 {
                res.messages.push(SubMsg::new(BankMsg::Send {
                    to_address: finder.to_string(),
                    amount: vec![coin(finders_fee, NATIVE_DENOM)],
                }));
            }
            finders_fee
        }
        None => 0,
    };

    match collection_info.royalty_info {
        // If token supports royalities, payout shares to royalty recipient
        Some(royalty) => {
            let amount = coin((payment * royalty.share).u128(), NATIVE_DENOM);
            res.messages.push(SubMsg::new(BankMsg::Send {
                to_address: royalty.payment_address.to_string(),
                amount: vec![amount.clone()],
            }));

            let event = Event::new("royalty-payout")
                .add_attribute("collection", collection.to_string())
                .add_attribute("amount", amount.to_string())
                .add_attribute("recipient", royalty.payment_address.to_string());
            res.events.push(event);

            let seller_share_msg = BankMsg::Send {
                to_address: payment_recipient.to_string(),
                amount: vec![coin(
                    (payment * (Decimal::one() - royalty.share) - network_fee).u128() - finders_fee,
                    NATIVE_DENOM.to_string(),
                )],
            };
            res.messages.push(SubMsg::new(seller_share_msg));
        }
        None => {
            // If token doesn't support royalties, pay seller in full
            let seller_share_msg = BankMsg::Send {
                to_address: payment_recipient.to_string(),
                amount: vec![coin(
                    (payment - network_fee).u128() - finders_fee,
                    NATIVE_DENOM.to_string(),
                )],
            };
            res.messages.push(SubMsg::new(seller_share_msg));
        }
    }

    Ok(())
}

fn price_validate(store: &dyn Storage, price: &Coin) -> Result<(), ContractError> {
    if price.amount.is_zero() || price.denom != NATIVE_DENOM {
        return Err(ContractError::InvalidPrice {});
    }

    if price.amount < SUDO_PARAMS.load(store)?.min_price {
        return Err(ContractError::PriceTooSmall(price.amount));
    }

    Ok(())
}

fn store_bid(store: &mut dyn Storage, bid: &Bid) -> StdResult<()> {
    bids().save(
        store,
        bid_key(bid.collection.clone(), bid.token_id, bid.bidder.clone()),
        bid,
    )
}

fn store_ask(store: &mut dyn Storage, ask: &Ask) -> StdResult<()> {
    asks().save(store, ask_key(ask.collection.clone(), ask.token_id), ask)
}

/// Checks to enfore only NFT owner can call
fn only_owner(
    deps: Deps,
    info: &MessageInfo,
    collection: Addr,
    token_id: u32,
) -> Result<OwnerOfResponse, ContractError> {
    let res = Cw721Contract(collection).owner_of(&deps.querier, token_id.to_string(), false)?;
    if res.owner != info.sender {
        return Err(ContractError::UnauthorizedOwner {});
    }

    Ok(res)
}

/// Checks to enforce only privileged operators
fn only_operator(store: &dyn Storage, info: &MessageInfo) -> Result<Addr, ContractError> {
    let params = SUDO_PARAMS.load(store)?;
    if !params
        .operators
        .iter()
        .any(|a| a.as_ref() == info.sender.as_ref())
    {
        return Err(ContractError::UnauthorizedOperator {});
    }

    Ok(info.sender.clone())
}

enum HookReply {
    Ask = 1,
    Sale,
    Bid,
    CollectionBid,
}

impl From<u64> for HookReply {
    fn from(item: u64) -> Self {
        match item {
            1 => HookReply::Ask,
            2 => HookReply::Sale,
            3 => HookReply::Bid,
            4 => HookReply::CollectionBid,
            _ => panic!("invalid reply type"),
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match HookReply::from(msg.id) {
        HookReply::Ask => {
            let res = Response::new()
                .add_attribute("action", "ask-hook-failed")
                .add_attribute("error", msg.result.unwrap_err());
            Ok(res)
        }
        HookReply::Sale => {
            let res = Response::new()
                .add_attribute("action", "sale-hook-failed")
                .add_attribute("error", msg.result.unwrap_err());
            Ok(res)
        }
        HookReply::Bid => {
            let res = Response::new()
                .add_attribute("action", "bid-hook-failed")
                .add_attribute("error", msg.result.unwrap_err());
            Ok(res)
        }
        HookReply::CollectionBid => {
            let res = Response::new()
                .add_attribute("action", "collection-bid-hook-failed")
                .add_attribute("error", msg.result.unwrap_err());
            Ok(res)
        }
    }
}

fn prepare_ask_hook(
    deps: Deps,
    ask: &Ask,
    action: HookAction,
) -> Result<Vec<SubMsg>, ContractError> {
    let submsgs = ASK_HOOKS.prepare_hooks(deps.storage, |h| {
        let msg = AskHookMsg { ask: ask.clone() };
        let execute = WasmMsg::Execute {
            contract_addr: h.to_string(),
            msg: msg.into_binary(action.clone())?,
            funds: vec![],
        };
        Ok(SubMsg::reply_on_error(execute, HookReply::Ask as u64))
    })?;

    Ok(submsgs)
}

fn prepare_bid_hook(
    deps: Deps,
    bid: &Bid,
    action: HookAction,
) -> Result<Vec<SubMsg>, ContractError> {
    let submsgs = BID_HOOKS.prepare_hooks(deps.storage, |h| {
        let msg = BidHookMsg { bid: bid.clone() };
        let execute = WasmMsg::Execute {
            contract_addr: h.to_string(),
            msg: msg.into_binary(action.clone())?,
            funds: vec![],
        };
        Ok(SubMsg::reply_on_error(execute, HookReply::Bid as u64))
    })?;

    Ok(submsgs)
}

fn prepare_collection_bid_hook(
    deps: Deps,
    collection_bid: &CollectionBid,
    action: HookAction,
) -> Result<Vec<SubMsg>, ContractError> {
    let submsgs = COLLECTION_BID_HOOKS.prepare_hooks(deps.storage, |h| {
        let msg = CollectionBidHookMsg {
            collection_bid: collection_bid.clone(),
        };
        let execute = WasmMsg::Execute {
            contract_addr: h.to_string(),
            msg: msg.into_binary(action.clone())?,
            funds: vec![],
        };
        Ok(SubMsg::reply_on_error(
            execute,
            HookReply::CollectionBid as u64,
        ))
    })?;

    Ok(submsgs)
}
