use crate::error::ContractError;
use crate::helpers::map_validate;
use crate::msg::{
    AskHookMsg, BidHookMsg, CollectionBidHookMsg, ExecuteMsg, HookAction, InstantiateMsg,
    SaleHookMsg,
};
use crate::state::{
    ask_key, asks, bid_key, bids, collection_bid_key, collection_bids, Ask, Bid, CollectionBid,
    Order, SaleType, SudoParams, TokenId, ASK_HOOKS, BID_HOOKS, COLLECTION_BID_HOOKS, SALE_HOOKS,
    SUDO_PARAMS,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, to_binary, Addr, BankMsg, Coin, Decimal, Deps, DepsMut, Empty, Env, Event, MessageInfo,
    Reply, StdError, StdResult, Storage, Timestamp, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw721::{Cw721ExecuteMsg, OwnerOfResponse};
use cw721_base::helpers::Cw721Contract;
use cw_utils::{may_pay, maybe_addr, must_pay, nonpayable, Duration, Expiration};
use semver::Version;
use sg1::fair_burn;
use sg721::msg::{CollectionInfoResponse, QueryMsg as Sg721QueryMsg};
use sg_std::{Response, SubMsg, NATIVE_DENOM};

// Version info for migration info
const CONTRACT_NAME: &str = "crates.io:sg-marketplace";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// bps fee can not exceed 100%
const MAX_FEE_BPS: u64 = 10000;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    if msg.max_finders_fee_bps > MAX_FEE_BPS {
        return Err(ContractError::InvalidFindersFeeBps(msg.max_finders_fee_bps));
    }
    if msg.trading_fee_bps > MAX_FEE_BPS {
        return Err(ContractError::InvalidTradingFeeBps(msg.trading_fee_bps));
    }
    if msg.bid_removal_reward_bps > MAX_FEE_BPS {
        return Err(ContractError::InvalidBidRemovalRewardBps(
            msg.bid_removal_reward_bps,
        ));
    }

    msg.ask_expiry.validate()?;
    msg.bid_expiry.validate()?;

    match msg.stale_bid_duration {
        Duration::Height(_) => return Err(ContractError::InvalidDuration {}),
        Duration::Time(_) => {}
    };

    let params = SudoParams {
        trading_fee_percent: Decimal::percent(msg.trading_fee_bps),
        ask_expiry: msg.ask_expiry,
        bid_expiry: msg.bid_expiry,
        operators: map_validate(deps.api, &msg.operators)?,
        max_finders_fee_percent: Decimal::percent(msg.max_finders_fee_bps),
        min_price: msg.min_price,
        stale_bid_duration: msg.stale_bid_duration,
        bid_removal_reward_percent: Decimal::percent(msg.bid_removal_reward_bps),
        listing_fee: msg.listing_fee,
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
        ExecuteMsg::SetBid {
            collection,
            token_id,
            expires,
            finder,
            finders_fee_bps,
            sale_type,
        } => execute_set_bid(
            deps,
            env,
            info,
            sale_type,
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
        } => execute_update_ask_price(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            token_id,
            price,
        ),
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
        ExecuteMsg::SyncAsk {
            collection,
            token_id,
        } => execute_sync_ask(deps, env, info, api.addr_validate(&collection)?, token_id),
        ExecuteMsg::RemoveStaleAsk {
            collection,
            token_id,
        } => execute_remove_stale_ask(deps, env, info, api.addr_validate(&collection)?, token_id),
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

    price_validate(deps.storage, &price)?;
    only_owner(deps.as_ref(), &info, &collection, token_id)?;

    // Check if this contract is approved to transfer the token
    Cw721Contract(collection.clone()).approval(
        &deps.querier,
        token_id.to_string(),
        env.contract.address.to_string(),
        None,
    )?;

    let params = SUDO_PARAMS.load(deps.storage)?;
    params.ask_expiry.is_valid(&env.block, expires)?;

    // Check if msg has correct listing fee
    let listing_fee = may_pay(&info, NATIVE_DENOM)?;
    if listing_fee != params.listing_fee {
        return Err(ContractError::InvalidListingFee(listing_fee));
    }

    if let Some(fee) = finders_fee_bps {
        if Decimal::percent(fee) > params.max_finders_fee_percent {
            return Err(ContractError::InvalidFindersFeeBps(fee));
        };
    }

    if let Some(address) = reserve_for.clone() {
        if address == info.sender {
            return Err(ContractError::InvalidReserveAddress {
                reason: "cannot reserve to the same address".to_string(),
            });
        }
        if sale_type != SaleType::FixedPrice {
            return Err(ContractError::InvalidReserveAddress {
                reason: "can only reserve for fixed_price sales".to_string(),
            });
        }
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

    // Append fair_burn msg
    let mut res = Response::new();
    if listing_fee > Uint128::zero() {
        fair_burn(listing_fee.u128(), None, &mut res);
    }

    let hook = prepare_ask_hook(deps.as_ref(), &ask, HookAction::Create)?;

    let event = Event::new("set-ask")
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("seller", seller)
        .add_attribute("price", price.to_string())
        .add_attribute("expires", expires.to_string());

    Ok(res.add_submessages(hook).add_event(event))
}

/// Removes the ask on a particular NFT
pub fn execute_remove_ask(
    deps: DepsMut,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_owner(deps.as_ref(), &info, &collection, token_id)?;

    let key = ask_key(&collection, token_id);
    let ask = asks().load(deps.storage, key.clone())?;
    asks().remove(deps.storage, key)?;

    let hook = prepare_ask_hook(deps.as_ref(), &ask, HookAction::Delete)?;

    let event = Event::new("remove-ask")
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id.to_string());

    Ok(Response::new().add_event(event).add_submessages(hook))
}

/// Updates the ask price on a particular NFT
pub fn execute_update_ask_price(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
    price: Coin,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_owner(deps.as_ref(), &info, &collection, token_id)?;
    price_validate(deps.storage, &price)?;

    let key = ask_key(&collection, token_id);

    let mut ask = asks().load(deps.storage, key.clone())?;
    if !ask.is_active {
        return Err(ContractError::AskNotActive {});
    }
    if ask.is_expired(&env.block) {
        return Err(ContractError::AskExpired {});
    }

    ask.price = price.amount;
    asks().save(deps.storage, key, &ask)?;

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
    sale_type: SaleType,
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

    if let Some(finder) = finder.clone() {
        if info.sender == finder {
            return Err(ContractError::InvalidFinder(
                "bidder cannot be finder".to_string(),
            ));
        }
    }

    // check bid finders_fee_bps is not over max
    if let Some(fee) = finders_fee_bps {
        if Decimal::percent(fee) > params.max_finders_fee_percent {
            return Err(ContractError::InvalidFindersFeeBps(fee));
        }
    }
    let bid_price = must_pay(&info, NATIVE_DENOM)?;
    if bid_price < params.min_price {
        return Err(ContractError::PriceTooSmall(bid_price));
    }
    params.bid_expiry.is_valid(&env.block, expires)?;
    if let Some(finders_fee_bps) = finders_fee_bps {
        if Decimal::percent(finders_fee_bps) > params.max_finders_fee_percent {
            return Err(ContractError::InvalidFindersFeeBps(finders_fee_bps));
        }
    }

    let bidder = info.sender;
    let mut res = Response::new();
    let bid_key = bid_key(&collection, token_id, &bidder);
    let ask_key = ask_key(&collection, token_id);

    if let Some(existing_bid) = bids().may_load(deps.storage, bid_key.clone())? {
        bids().remove(deps.storage, bid_key)?;
        let refund_bidder = BankMsg::Send {
            to_address: bidder.to_string(),
            amount: vec![coin(existing_bid.price.u128(), NATIVE_DENOM)],
        };
        res = res.add_message(refund_bidder)
    }

    let existing_ask = asks().may_load(deps.storage, ask_key.clone())?;

    // if the bid is placed for fixed price only but there is no ask
    // return an error
    if sale_type == SaleType::FixedPrice && existing_ask.is_none() {
        return Err(ContractError::AskNotFound {});
    }

    if let Some(ask) = existing_ask.clone() {
        if ask.is_expired(&env.block) {
            return Err(ContractError::AskExpired {});
        }
        if !ask.is_active {
            return Err(ContractError::AskNotActive {});
        }
        if let Some(reserved_for) = ask.reserve_for {
            if reserved_for != bidder {
                return Err(ContractError::TokenReserved {});
            }
        }
    }

    let save_bid = |store| -> StdResult<_> {
        let bid = Bid::new(
            collection.clone(),
            token_id,
            bidder.clone(),
            bid_price,
            finders_fee_bps,
            expires,
        );
        store_bid(store, &bid)?;
        Ok(Some(bid))
    };

    let bid = match existing_ask {
        Some(ask) => match ask.sale_type {
            SaleType::FixedPrice => {
                if ask.price != bid_price {
                    return Err(ContractError::InvalidPrice {});
                }
                asks().remove(deps.storage, ask_key)?;
                finalize_sale(
                    deps.as_ref(),
                    ask,
                    bid_price,
                    bidder.clone(),
                    finder,
                    &mut res,
                )?;
                None
            }
            SaleType::Auction => save_bid(deps.storage)?,
        },
        None => save_bid(deps.storage)?,
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

    let key = bid_key(&collection, token_id, &bidder);
    let bid = bids().load(deps.storage, key.clone())?;
    bids().remove(deps.storage, key)?;

    let refund_bidder_msg = BankMsg::Send {
        to_address: bid.bidder.to_string(),
        amount: vec![coin(bid.price.u128(), NATIVE_DENOM)],
    };

    let hook = prepare_bid_hook(deps.as_ref(), &bid, HookAction::Delete)?;

    let event = Event::new("remove-bid")
        .add_attribute("collection", collection)
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("bidder", bidder);

    let res = Response::new()
        .add_message(refund_bidder_msg)
        .add_event(event)
        .add_submessages(hook);

    Ok(res)
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
    only_owner(deps.as_ref(), &info, &collection, token_id)?;

    let bid_key = bid_key(&collection, token_id, &bidder);
    let ask_key = ask_key(&collection, token_id);

    let bid = bids().load(deps.storage, bid_key.clone())?;
    if bid.is_expired(&env.block) {
        return Err(ContractError::BidExpired {});
    }

    let ask = if let Some(existing_ask) = asks().may_load(deps.storage, ask_key.clone())? {
        if existing_ask.is_expired(&env.block) {
            return Err(ContractError::AskExpired {});
        }
        if !existing_ask.is_active {
            return Err(ContractError::AskNotActive {});
        }
        asks().remove(deps.storage, ask_key)?;
        existing_ask
    } else {
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
    };

    // Remove accepted bid
    bids().remove(deps.storage, bid_key)?;

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
    // check bid finders_fee_bps is not over max
    if let Some(fee) = finders_fee_bps {
        if Decimal::percent(fee) > params.max_finders_fee_percent {
            return Err(ContractError::InvalidFindersFeeBps(fee));
        }
    }

    let bidder = info.sender;
    let mut res = Response::new();

    let key = collection_bid_key(&collection, &bidder);

    let existing_bid = collection_bids().may_load(deps.storage, key.clone())?;
    if let Some(bid) = existing_bid {
        collection_bids().remove(deps.storage, key.clone())?;
        let refund_bidder_msg = BankMsg::Send {
            to_address: bid.bidder.to_string(),
            amount: vec![coin(bid.price.u128(), NATIVE_DENOM)],
        };
        res = res.add_message(refund_bidder_msg);
    }

    let collection_bid = CollectionBid {
        collection: collection.clone(),
        bidder: bidder.clone(),
        price,
        finders_fee_bps,
        expires_at: expires,
    };
    collection_bids().save(deps.storage, key, &collection_bid)?;

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

    let key = collection_bid_key(&collection, &bidder);

    let collection_bid = collection_bids().load(deps.storage, key.clone())?;
    collection_bids().remove(deps.storage, key)?;

    let refund_bidder_msg = BankMsg::Send {
        to_address: collection_bid.bidder.to_string(),
        amount: vec![coin(collection_bid.price.u128(), NATIVE_DENOM)],
    };

    let hook = prepare_collection_bid_hook(deps.as_ref(), &collection_bid, HookAction::Delete)?;

    let event = Event::new("remove-collection-bid")
        .add_attribute("collection", collection.to_string())
        .add_attribute("bidder", bidder);

    let res = Response::new()
        .add_message(refund_bidder_msg)
        .add_event(event)
        .add_submessages(hook);

    Ok(res)
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
    only_owner(deps.as_ref(), &info, &collection, token_id)?;

    let bid_key = collection_bid_key(&collection, &bidder);
    let ask_key = ask_key(&collection, token_id);

    let bid = collection_bids().load(deps.storage, bid_key.clone())?;
    if bid.is_expired(&env.block) {
        return Err(ContractError::BidExpired {});
    }
    collection_bids().remove(deps.storage, bid_key)?;

    let ask = if let Some(existing_ask) = asks().may_load(deps.storage, ask_key.clone())? {
        if existing_ask.is_expired(&env.block) {
            return Err(ContractError::AskExpired {});
        }
        if !existing_ask.is_active {
            return Err(ContractError::AskNotActive {});
        }
        asks().remove(deps.storage, ask_key)?;
        existing_ask
    } else {
        // Create a temporary Ask
        Ask {
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
        }
    };

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

    let event = Event::new("accept-collection-bid")
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("bidder", bidder)
        .add_attribute("seller", info.sender.to_string())
        .add_attribute("price", bid.price.to_string());

    Ok(res.add_event(event))
}

/// Synchronizes the active state of an ask based on token ownership.
/// This is a privileged operation called by an operator to update an ask when a transfer happens.
pub fn execute_sync_ask(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_operator(deps.storage, &info)?;

    let key = ask_key(&collection, token_id);

    let mut ask = asks().load(deps.storage, key.clone())?;

    // Check if marketplace still holds approval
    // An approval will be removed when
    // 1 - There is a transfer
    // 2 - The approval expired (approvals can have different expiration times)
    let res = Cw721Contract(collection.clone()).approval(
        &deps.querier,
        token_id.to_string(),
        env.contract.address.to_string(),
        None,
    );
    if res.is_ok() == ask.is_active {
        return Err(ContractError::AskUnchanged {});
    }
    ask.is_active = res.is_ok();
    asks().save(deps.storage, key, &ask)?;

    let hook = prepare_ask_hook(deps.as_ref(), &ask, HookAction::Update)?;

    let event = Event::new("update-ask-state")
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("is_active", ask.is_active.to_string());

    Ok(Response::new().add_event(event).add_submessages(hook))
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
    only_operator(deps.storage, &info)?;

    let key = ask_key(&collection, token_id);
    let ask = asks().load(deps.storage, key.clone())?;

    let res =
        Cw721Contract(collection.clone()).owner_of(&deps.querier, token_id.to_string(), false);
    let has_owner = res.is_ok();
    let expired = ask.is_expired(&env.block);

    // it has an owner and ask is still valid
    if has_owner && !expired {
        return Err(ContractError::AskUnchanged {});
    }

    asks().remove(deps.storage, key)?;
    let hook = prepare_ask_hook(deps.as_ref(), &ask, HookAction::Delete)?;

    let event = Event::new("remove-ask")
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("operator", info.sender.to_string())
        .add_attribute("expired", expired.to_string())
        .add_attribute("has_owner", has_owner.to_string());

    Ok(Response::new().add_event(event).add_submessages(hook))
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

    let bid_key = bid_key(&collection, token_id, &bidder);
    let bid = bids().load(deps.storage, bid_key.clone())?;

    let params = SUDO_PARAMS.load(deps.storage)?;
    let stale_time = (Expiration::AtTime(bid.expires_at) + params.stale_bid_duration)?;
    if !stale_time.is_expired(&env.block) {
        return Err(ContractError::BidNotStale {});
    }

    // bid is stale, refund bidder and reward operator
    bids().remove(deps.storage, bid_key)?;

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

    let key = collection_bid_key(&collection, &bidder);
    let collection_bid = collection_bids().load(deps.storage, key.clone())?;

    let params = SUDO_PARAMS.load(deps.storage)?;
    let stale_time = (Expiration::AtTime(collection_bid.expires_at) + params.stale_bid_duration)?;
    if !stale_time.is_expired(&env.block) {
        return Err(ContractError::BidNotStale {});
    }

    // collection bid is stale, refund bidder and reward operator
    collection_bids().remove(deps.storage, key)?;

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

    res.messages
        .append(&mut prepare_sale_hook(deps, &ask, buyer.clone())?);

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
            if payment < (network_fee + Uint128::from(finders_fee) + amount.amount) {
                return Err(StdError::generic_err("Fees exceed payment"));
            }
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
            if payment < (network_fee + Uint128::from(finders_fee)) {
                return Err(StdError::generic_err("Fees exceed payment"));
            }
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
        bid_key(&bid.collection, bid.token_id, &bid.bidder),
        bid,
    )
}

fn store_ask(store: &mut dyn Storage, ask: &Ask) -> StdResult<()> {
    asks().save(store, ask_key(&ask.collection, ask.token_id), ask)
}

/// Checks to enfore only NFT owner can call
fn only_owner(
    deps: Deps,
    info: &MessageInfo,
    collection: &Addr,
    token_id: u32,
) -> Result<OwnerOfResponse, ContractError> {
    let res =
        Cw721Contract(collection.clone()).owner_of(&deps.querier, token_id.to_string(), false)?;
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

fn prepare_ask_hook(deps: Deps, ask: &Ask, action: HookAction) -> StdResult<Vec<SubMsg>> {
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

fn prepare_sale_hook(deps: Deps, ask: &Ask, buyer: Addr) -> StdResult<Vec<SubMsg>> {
    let submsgs = SALE_HOOKS.prepare_hooks(deps.storage, |h| {
        let msg = SaleHookMsg {
            collection: ask.collection.to_string(),
            token_id: ask.token_id,
            price: coin(ask.price.clone().u128(), NATIVE_DENOM),
            seller: ask.seller.to_string(),
            buyer: buyer.to_string(),
        };
        let execute = WasmMsg::Execute {
            contract_addr: h.to_string(),
            msg: msg.into_binary()?,
            funds: vec![],
        };
        Ok(SubMsg::reply_on_error(execute, HookReply::Sale as u64))
    })?;

    Ok(submsgs)
}

fn prepare_bid_hook(deps: Deps, bid: &Bid, action: HookAction) -> StdResult<Vec<SubMsg>> {
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
) -> StdResult<Vec<SubMsg>> {
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: Empty) -> Result<Response, ContractError> {
    let current_version = cw2::get_contract_version(deps.storage)?;
    if current_version.contract != CONTRACT_NAME {
        return Err(StdError::generic_err("Cannot upgrade to a different contract").into());
    }
    let version: Version = current_version
        .version
        .parse()
        .map_err(|_| StdError::generic_err("Invalid contract version"))?;
    let new_version: Version = CONTRACT_VERSION
        .parse()
        .map_err(|_| StdError::generic_err("Invalid contract version"))?;

    if version > new_version {
        return Err(StdError::generic_err("Cannot upgrade to a previous contract version").into());
    }
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::new())
}
