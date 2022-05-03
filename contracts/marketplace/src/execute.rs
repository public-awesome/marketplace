use crate::error::ContractError;
use crate::helpers::map_validate;
use crate::msg::{AskCreatedHookMsg, AskFilledHookMsg, ExecuteMsg, InstantiateMsg};
use crate::state::{
    ask_key, asks, bid_key, bids, collection_bid_key, collection_bids, Ask, Bid, CollectionBid,
    SudoParams, TokenId, ASK_CREATED_HOOKS, ASK_FILLED_HOOKS, SUDO_PARAMS,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, to_binary, Addr, BankMsg, Coin, Decimal, Deps, DepsMut, Env, Event, MessageInfo, Order,
    Reply, StdResult, Storage, Timestamp, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw721::{Cw721ExecuteMsg, OwnerOfResponse};
use cw721_base::helpers::Cw721Contract;
use cw_utils::{maybe_addr, must_pay, nonpayable};
use sg1::fair_burn;
use sg721::msg::{CollectionInfoResponse, QueryMsg as Sg721QueryMsg};
use sg_std::{Response, SubMsg, NATIVE_DENOM};

// Version info for migration info
const CONTRACT_NAME: &str = "crates.io:sg-marketplace";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const REPLY_ASK_FILLED_HOOK: u64 = 1;
const REPLY_ASK_CREATED_HOOK: u64 = 2;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let params = SudoParams {
        trading_fee_basis_points: Decimal::percent(msg.trading_fee_basis_points),
        ask_expiry: msg.ask_expiry,
        bid_expiry: msg.bid_expiry,
        operators: map_validate(deps.api, &msg.operators)?,
    };
    SUDO_PARAMS.save(deps.storage, &params)?;

    if let Some(hook) = msg.ask_filled_hook {
        ASK_FILLED_HOOKS.add_hook(deps.storage, deps.api.addr_validate(&hook)?)?;
    }

    Ok(Response::new())
}

/// To mitigate clippy::too_many_arguments warning
pub struct ExecuteEnv<'a> {
    deps: DepsMut<'a>,
    env: Env,
    info: MessageInfo,
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
            collection,
            token_id,
            price,
            funds_recipient,
            reserve_for,
            expires,
        } => execute_set_ask(
            ExecuteEnv { deps, env, info },
            api.addr_validate(&collection)?,
            token_id,
            price,
            maybe_addr(api, funds_recipient)?,
            maybe_addr(api, reserve_for)?,
            expires,
        ),
        ExecuteMsg::RemoveAsk {
            collection,
            token_id,
        } => execute_remove_ask(deps, info, api.addr_validate(&collection)?, token_id),
        ExecuteMsg::UpdateAskState {
            collection,
            token_id,
            active,
        } => execute_update_ask_state(
            deps,
            info,
            api.addr_validate(&collection)?,
            token_id,
            active,
        ),
        ExecuteMsg::SetBid {
            collection,
            token_id,
            expires,
        } => execute_set_bid(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            token_id,
            expires,
        ),
        ExecuteMsg::RemoveBid {
            collection,
            token_id,
        } => execute_remove_bid(deps, env, info, api.addr_validate(&collection)?, token_id),
        ExecuteMsg::AcceptBid {
            collection,
            token_id,
            bidder,
        } => execute_accept_bid(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            token_id,
            api.addr_validate(&bidder)?,
        ),
        ExecuteMsg::UpdateAsk {
            collection,
            token_id,
            price,
        } => execute_update_ask(deps, info, api.addr_validate(&collection)?, token_id, price),
        ExecuteMsg::SetCollectionBid {
            collection,
            expires,
        } => execute_set_collection_bid(deps, env, info, api.addr_validate(&collection)?, expires),
        ExecuteMsg::AcceptCollectionBid {
            collection,
            token_id,
            bidder,
        } => execute_accept_collection_bid(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            token_id,
            api.addr_validate(&bidder)?,
        ),
    }
}

/// An owner may set an Ask on their media. A bid is automatically fulfilled if it meets the asking price.
pub fn execute_set_ask(
    env: ExecuteEnv,
    collection: Addr,
    token_id: TokenId,
    price: Coin,
    funds_recipient: Option<Addr>,
    reserve_for: Option<Addr>,
    expires: Timestamp,
) -> Result<Response, ContractError> {
    let ExecuteEnv { deps, info, env } = env;
    nonpayable(&info)?;
    price_validate(&price)?;

    let params = SUDO_PARAMS.load(deps.storage)?;
    params.ask_expiry.is_valid(&env.block, expires)?;

    // Only the media onwer can call this
    let owner_of_response = only_owner(deps.as_ref(), &info, collection.clone(), token_id)?;
    // Check that approval has been set for marketplace contract
    if owner_of_response
        .approvals
        .iter()
        .map(|x| x.spender == env.contract.address)
        .len()
        != 1
    {
        return Err(ContractError::NeedsApproval {});
    }

    let seller = info.sender;
    asks().save(
        deps.storage,
        ask_key(collection.clone(), token_id),
        &Ask {
            collection: collection.clone(),
            token_id,
            seller: seller.clone(),
            price: price.amount,
            funds_recipient: funds_recipient.clone(),
            reserve_for,
            expires,
            active: true,
        },
    )?;

    let msg = AskCreatedHookMsg {
        collection: collection.to_string(),
        token_id,
        seller: seller.to_string(),
        funds_recipient: funds_recipient
            .unwrap_or_else(|| seller.clone())
            .to_string(),
        price: price.clone(),
    };

    // Include hook submessages, i.e: listing rewards
    let submsgs = ASK_CREATED_HOOKS.prepare_hooks(deps.storage, |h| {
        let execute = WasmMsg::Execute {
            contract_addr: h.to_string(),
            msg: msg.clone().into_binary()?,
            funds: vec![],
        };
        Ok(SubMsg::reply_on_error(execute, REPLY_ASK_CREATED_HOOK))
    })?;

    let event = Event::new("set-ask")
        .add_attribute("collection", collection)
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("seller", seller)
        .add_attribute("price", price.to_string());

    Ok(Response::new().add_submessages(submsgs).add_event(event))
}

/// Removes the ask on a particular media
pub fn execute_remove_ask(
    deps: DepsMut,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_owner(deps.as_ref(), &info, collection.clone(), token_id)?;

    asks().remove(deps.storage, (collection.clone(), token_id))?;

    let bids_to_remove = bids()
        .idx
        .collection_token_id
        .prefix((collection.clone(), token_id))
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| item.map(|(_, b)| b))
        .collect::<StdResult<Vec<_>>>()?;

    let mut msgs: Vec<BankMsg> = vec![];
    for bid in bids_to_remove.iter() {
        msgs.push(remove_and_refund_bid(deps.storage, bid.clone())?)
    }

    let event = Event::new("remove-ask")
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id.to_string());

    Ok(Response::new().add_messages(msgs).add_event(event))
}

/// Updates the the active state of the ask.
/// This is a privileged operation called by an operator to update the active state of an Ask
/// when an NFT transfer happens.
pub fn execute_update_ask_state(
    deps: DepsMut,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
    active: bool,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let params = SUDO_PARAMS.load(deps.storage)?;
    if !params
        .operators
        .iter()
        .any(|a| a.as_ref() == info.sender.as_ref())
    {
        return Err(ContractError::Unauthorized {});
    }

    let mut ask = asks().load(deps.storage, ask_key(collection.clone(), token_id))?;
    ask.active = active;
    asks().save(deps.storage, ask_key(collection.clone(), token_id), &ask)?;

    let event = Event::new("update-ask-state")
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("active", active.to_string());

    Ok(Response::new().add_event(event))
}

/// Updates the ask price on a particular NFT
pub fn execute_update_ask(
    deps: DepsMut,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
    price: Coin,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_owner(deps.as_ref(), &info, collection.clone(), token_id)?;
    price_validate(&price)?;

    let mut ask = asks().load(deps.storage, ask_key(collection.clone(), token_id))?;
    ask.price = price.amount;
    asks().save(deps.storage, ask_key(collection.clone(), token_id), &ask)?;

    let event = Event::new("update-ask")
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("price", price.to_string());

    Ok(Response::new().add_event(event))
}

/// Anyone may place a bid on a listed NFT. By placing a bid, the bidder sends STARS to the market contract.
pub fn execute_set_bid(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
    expires: Timestamp,
) -> Result<Response, ContractError> {
    let bid_price = must_pay(&info, NATIVE_DENOM)?;
    let bidder = info.sender;
    let params = SUDO_PARAMS.load(deps.storage)?;
    params.bid_expiry.is_valid(&env.block, expires)?;

    // Ask validation
    let ask = asks().load(deps.storage, ask_key(collection.clone(), token_id))?;
    if ask.expires <= env.block.time {
        return Err(ContractError::AskExpired {});
    }
    if !ask.active {
        return Err(ContractError::AskNotActive {});
    }
    if let Some(reserved_for) = ask.clone().reserve_for {
        if reserved_for != bidder {
            return Err(ContractError::TokenReserved {});
        }
    }

    // Check bidder has existing bid, if so remove existing bid
    let mut res = Response::new();
    if let Some(existing_bid) =
        bids().may_load(deps.storage, (collection.clone(), token_id, bidder.clone()))?
    {
        bids().remove(deps.storage, (collection.clone(), token_id, bidder.clone()))?;
        let exec_refund_bidder = BankMsg::Send {
            to_address: bidder.to_string(),
            amount: vec![coin(existing_bid.price.u128(), NATIVE_DENOM)],
        };
        res = res.add_message(exec_refund_bidder)
    };

    if ask.price != bid_price {
        // Bid does not meet ask criteria, store bid
        bids().save(
            deps.storage,
            (collection.clone(), token_id, bidder.clone()),
            &Bid {
                collection: collection.clone(),
                token_id,
                bidder: bidder.clone(),
                price: bid_price,
                expires,
            },
        )?;
    } else {
        // Bid meets ask criteria so fulfill bid
        asks().remove(deps.storage, ask_key(collection.clone(), token_id))?;

        // Include messages needed to finalize nft transfer and payout
        fill_ask(deps, ask, bid_price, bidder.clone(), &mut res)?;
    }

    let event = Event::new("set-bid")
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("bidder", bidder)
        .add_attribute("bid_price", bid_price.to_string());

    Ok(res.add_event(event))
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

    let event = Event::new("remove-bid")
        .add_attribute("collection", collection)
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("bidder", bidder);

    let res = Response::new()
        .add_message(remove_and_refund_bid(deps.storage, bid)?)
        .add_event(event);

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

/// Owner can accept a bid which transfers funds as well as the token
pub fn execute_accept_bid(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
    bidder: Addr,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_owner(deps.as_ref(), &info, collection.clone(), token_id)?;

    // Query current ask
    let ask = asks().load(deps.storage, ask_key(collection.clone(), token_id))?;
    if ask.expires <= env.block.time {
        return Err(ContractError::AskExpired {});
    }
    if !ask.active {
        return Err(ContractError::AskNotActive {});
    }

    // Query accepted bid
    let bid = bids().load(deps.storage, (collection.clone(), token_id, bidder.clone()))?;
    if bid.expires <= env.block.time {
        return Err(ContractError::BidExpired {});
    }

    // Remove ask
    asks().remove(deps.storage, ask_key(collection.clone(), token_id))?;
    // Remove accepted bid
    bids().remove(deps.storage, (collection.clone(), token_id, bidder.clone()))?;

    let mut res = Response::new();

    // Transfer funds and NFT
    fill_ask(deps, ask, bid.price, bidder.clone(), &mut res)?;

    let event = Event::new("accept-bid")
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("bidder", bidder);

    Ok(res.add_event(event))
}

/// Place a collection bid (limit order) across an entire collection
pub fn execute_set_collection_bid(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    expires: Timestamp,
) -> Result<Response, ContractError> {
    let price = must_pay(&info, NATIVE_DENOM)?;

    let params = SUDO_PARAMS.load(deps.storage)?;
    params.bid_expiry.is_valid(&env.block, expires)?;

    let bidder = info.sender;
    let mut res = Response::new();

    // Check bidder has existing bid, if so remove existing bid
    if let Some(existing_bid) = collection_bids().may_load(
        deps.storage,
        collection_bid_key(collection.clone(), bidder.clone()),
    )? {
        collection_bids().remove(deps.storage, (collection.clone(), bidder.clone()))?;
        let exec_refund_bidder = BankMsg::Send {
            to_address: bidder.to_string(),
            amount: vec![coin(existing_bid.price.u128(), NATIVE_DENOM)],
        };
        res = res.add_message(exec_refund_bidder)
    };

    collection_bids().save(
        deps.storage,
        collection_bid_key(collection.clone(), bidder.clone()),
        &CollectionBid {
            collection: collection.clone(),
            bidder: bidder.clone(),
            price,
            expires,
        },
    )?;

    let event = Event::new("set-collection-bid")
        .add_attribute("collection", collection.to_string())
        .add_attribute("bidder", bidder)
        .add_attribute("bid_price", price.to_string());

    Ok(res.add_event(event))
}

/// Owner of an item in a collection can accept a collection bid which transfers funds as well as a token
pub fn execute_accept_collection_bid(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    token_id: TokenId,
    bidder: Addr,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    only_owner(deps.as_ref(), &info, collection.clone(), token_id)?;

    let bid = collection_bids().load(
        deps.storage,
        collection_bid_key(collection.clone(), bidder.clone()),
    )?;
    if bid.expires <= env.block.time {
        return Err(ContractError::BidExpired {});
    }

    collection_bids().remove(
        deps.storage,
        collection_bid_key(collection.clone(), bidder.clone()),
    )?;

    let mut res = Response::new();

    // Create a temporary Ask
    let ask = Ask {
        collection: collection.clone(),
        token_id,
        price: bid.price,
        expires: bid.expires,
        active: true,
        seller: info.sender,
        funds_recipient: None,
        reserve_for: None,
    };

    // Transfer funds and NFT
    fill_ask(deps, ask, bid.price, bidder.clone(), &mut res)?;

    let event = Event::new("accept-collection-bid")
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("bidder", bidder);

    Ok(res.add_event(event))
}

/// Checks to enfore only nft owner can call
fn only_owner(
    deps: Deps,
    info: &MessageInfo,
    collection: Addr,
    token_id: u32,
) -> Result<OwnerOfResponse, ContractError> {
    let res = Cw721Contract(collection).owner_of(&deps.querier, token_id.to_string(), false)?;
    if res.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    Ok(res)
}

/// Transfers funds and NFT, updates bid
fn fill_ask(
    deps: DepsMut,
    ask: Ask,
    price: Uint128,
    buyer: Addr,
    res: &mut Response,
) -> StdResult<()> {
    // Payout bid
    payout(
        deps.as_ref(),
        ask.collection.clone(),
        price,
        ask.funds_recipient
            .clone()
            .unwrap_or_else(|| ask.seller.clone()),
        res,
    )?;

    // Create transfer cw721 msg
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

    let msg = AskFilledHookMsg {
        collection: ask.collection.to_string(),
        token_id: ask.token_id,
        price: coin(price.clone().u128(), NATIVE_DENOM),
        seller: ask.seller.to_string(),
        buyer: buyer.to_string(),
    };
    let mut submsgs = ASK_FILLED_HOOKS.prepare_hooks(deps.storage, |h| {
        let execute = WasmMsg::Execute {
            contract_addr: h.to_string(),
            msg: msg.clone().into_binary()?,
            funds: vec![],
        };
        Ok(SubMsg::reply_on_error(execute, REPLY_ASK_FILLED_HOOK))
    })?;
    res.messages.append(&mut submsgs);

    let event = Event::new("fill-ask")
        .add_attribute("collection", ask.collection.to_string())
        .add_attribute("token_id", ask.token_id.to_string())
        .add_attribute("seller", ask.seller.to_string())
        .add_attribute("buyer", buyer.to_string())
        .add_attribute("price", price.to_string());
    res.events.push(event);

    Ok(())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        REPLY_ASK_FILLED_HOOK => {
            let res = Response::new()
                .add_attribute("action", "ask-filled-hook-failed")
                .add_attribute("error", msg.result.unwrap_err());
            Ok(res)
        }
        REPLY_ASK_CREATED_HOOK => {
            let res = Response::new()
                .add_attribute("action", "ask-created-hook-failed")
                .add_attribute("error", msg.result.unwrap_err());
            Ok(res)
        }
        _ => Err(ContractError::UnrecognisedReply(msg.id)),
    }
}

/// Payout a bid
fn payout(
    deps: Deps,
    collection: Addr,
    payment: Uint128,
    payment_recipient: Addr,
    res: &mut Response,
) -> StdResult<()> {
    let config = SUDO_PARAMS.load(deps.storage)?;

    // Append Fair Burn message
    let network_fee = payment * config.trading_fee_basis_points / Uint128::from(100u128);
    fair_burn(network_fee.u128(), None, res);

    // Check if token supports Royalties
    let collection_info: CollectionInfoResponse = deps
        .querier
        .query_wasm_smart(collection, &Sg721QueryMsg::CollectionInfo {})?;

    match collection_info.royalty_info {
        // If token supports royalities, payout shares
        Some(royalty) => {
            let royalty_share_msg = BankMsg::Send {
                to_address: royalty.payment_address.to_string(),
                amount: vec![Coin {
                    amount: payment * royalty.share,
                    denom: NATIVE_DENOM.to_string(),
                }],
            };
            res.messages.push(SubMsg::new(royalty_share_msg));

            let owner_share_msg = BankMsg::Send {
                to_address: payment_recipient.to_string(),
                amount: vec![Coin {
                    amount: payment * (Decimal::one() - royalty.share) - network_fee,
                    denom: NATIVE_DENOM.to_string(),
                }],
            };
            res.messages.push(SubMsg::new(owner_share_msg));
        }
        None => {
            // If token doesn't support royalties, pay owner in full
            let owner_share_msg = BankMsg::Send {
                to_address: payment_recipient.to_string(),
                amount: vec![Coin {
                    amount: payment - network_fee,
                    denom: NATIVE_DENOM.to_string(),
                }],
            };
            res.messages.push(SubMsg::new(owner_share_msg));
        }
    }

    Ok(())
}

fn price_validate(price: &Coin) -> Result<(), ContractError> {
    if price.amount.is_zero() || price.denom != NATIVE_DENOM {
        return Err(ContractError::InvalidPrice {});
    }

    Ok(())
}
