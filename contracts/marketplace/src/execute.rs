use crate::error::ContractError;
use crate::helpers::map_validate;
use crate::msg::{ExecuteMsg, InstantiateMsg, SaleFinalizedHookMsg};
use crate::state::{
    ask_key, asks, bid_key, bids, collection_bid_key, collection_bids, Ask, Bid, CollectionBid,
    SudoParams, TokenId, SALE_FINALIZED_HOOKS, SUDO_PARAMS,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, to_binary, Addr, BankMsg, Coin, Decimal, Deps, DepsMut, Env, MessageInfo, Order,
    StdResult, Storage, Timestamp, WasmMsg,
};
use cw2::set_contract_version;
use cw721::{Cw721ExecuteMsg, Cw721QueryMsg, OwnerOfResponse};
use cw721_base::helpers::Cw721Contract;
use cw_utils::{must_pay, nonpayable};
use sg1::fair_burn;
use sg721::msg::{CollectionInfoResponse, QueryMsg as Sg721QueryMsg};
use sg_std::{CosmosMsg, Response, SubMsg, NATIVE_DENOM};

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

    let params = SudoParams {
        trading_fee_percent: msg.trading_fee_percent,
        ask_expiry: msg.ask_expiry,
        bid_expiry: msg.bid_expiry,
        operators: map_validate(deps.api, &msg.operators)?,
    };
    SUDO_PARAMS.save(deps.storage, &params)?;

    Ok(Response::new().add_attribute("action", "instantiate"))
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
            expires,
        } => execute_set_ask(
            ExecuteEnv { deps, env, info },
            api.addr_validate(&collection)?,
            token_id,
            price,
            funds_recipient.map(|addr| api.addr_validate(&addr).unwrap()),
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
    expires: Timestamp,
) -> Result<Response, ContractError> {
    let ExecuteEnv { deps, info, env } = env;
    nonpayable(&info)?;
    price_validate(&price)?;

    if expires <= env.block.time {
        return Err(ContractError::InvalidExpiration {});
    }

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

    asks().save(
        deps.storage,
        ask_key(collection.clone(), token_id),
        &Ask {
            collection: collection.clone(),
            token_id,
            seller: info.sender,
            price: price.amount,
            funds_recipient,
            expires,
            active: true,
        },
    )?;

    Ok(Response::new()
        .add_attribute("action", "set_ask")
        .add_attribute("collection", collection)
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("price", price.to_string()))
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

    Ok(Response::new()
        .add_attribute("action", "remove_ask")
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id.to_string())
        .add_messages(msgs))
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

    Ok(Response::new()
        .add_attribute("action", "update_ask_state")
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("active", active.to_string()))
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

    Ok(Response::new()
        .add_attribute("action", "update_ask")
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("price", price.to_string()))
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

    let params = SUDO_PARAMS.load(deps.storage)?;
    expires_validate(&env, expires, params.bid_expiry)?;

    let bidder = info.sender;
    let mut res = Response::new();

    // Check bidder has existing bid, if so remove existing bid
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

    let ask = asks().load(deps.storage, ask_key(collection.clone(), token_id))?;
    if ask.expires <= env.block.time {
        return Err(ContractError::AskExpired {});
    }
    if !ask.active {
        return Err(ContractError::AskNotActive {});
    }
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
        // Bid meets ask criteria so finalize sale
        asks().remove(deps.storage, ask_key(collection.clone(), token_id))?;

        let cw721_res: cw721::OwnerOfResponse = deps.querier.query_wasm_smart(
            collection.clone(),
            &Cw721QueryMsg::OwnerOf {
                token_id: token_id.to_string(),
                include_expired: None,
            },
        )?;
        let owner = deps.api.addr_validate(&cw721_res.owner)?;

        // Include messages needed to finalize nft transfer and payout
        let (msgs, submsgs) = finalize_sale(
            deps,
            collection.clone(),
            token_id,
            owner.clone(),
            bidder.clone(),
            ask.funds_recipient.unwrap_or(owner),
            coin(ask.price.u128(), NATIVE_DENOM),
        )?;

        res = res
            .add_attribute("action", "sale_finalized")
            .add_messages(msgs)
            .add_submessages(submsgs);
    }

    Ok(res
        .add_attribute("action", "set_bid")
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("bidder", bidder)
        .add_attribute("bid_price", bid_price.to_string()))
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

    Ok(Response::new()
        .add_message(remove_and_refund_bid(deps.storage, bid)?)
        .add_attribute("action", "remove_bid")
        .add_attribute("collection", collection)
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("bidder", bidder))
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

    // Transfer funds and NFT
    let (msgs, submsgs) = finalize_sale(
        deps,
        collection.clone(),
        token_id,
        info.sender.clone(),
        bidder.clone(),
        ask.funds_recipient.unwrap_or(info.sender),
        coin(bid.price.u128(), NATIVE_DENOM),
    )?;

    Ok(Response::new()
        .add_attribute("action", "accept_bid")
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("bidder", bidder)
        .add_messages(msgs)
        .add_submessages(submsgs))
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
    expires_validate(&env, expires, params.bid_expiry)?;

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

    Ok(res
        .add_attribute("action", "set_collection_bid")
        .add_attribute("collection", collection.to_string())
        .add_attribute("bidder", bidder)
        .add_attribute("bid_price", price.to_string()))
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

    // Transfer funds and NFT
    let (msgs, submsgs) = finalize_sale(
        deps,
        collection.clone(),
        token_id,
        info.sender.clone(),
        bidder.clone(),
        info.sender,
        coin(bid.price.u128(), NATIVE_DENOM),
    )?;

    Ok(Response::new()
        .add_attribute("action", "accept_collection_bid")
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("bidder", bidder)
        .add_messages(msgs)
        .add_submessages(submsgs))
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
fn finalize_sale(
    deps: DepsMut,
    collection: Addr,
    token_id: u32,
    seller: Addr,
    recipient: Addr,
    funds_recipient: Addr,
    price: Coin,
) -> StdResult<(Vec<CosmosMsg>, Vec<SubMsg>)> {
    // Payout bid
    let mut msgs: Vec<CosmosMsg> =
        payout(deps.as_ref(), collection.clone(), price, funds_recipient)?;

    // Create transfer cw721 msg
    let cw721_transfer_msg = Cw721ExecuteMsg::TransferNft {
        token_id: token_id.to_string(),
        recipient: recipient.to_string(),
    };

    let exec_cw721_transfer = WasmMsg::Execute {
        contract_addr: collection.to_string(),
        msg: to_binary(&cw721_transfer_msg)?,
        funds: vec![],
    };

    msgs.append(&mut vec![exec_cw721_transfer.into()]);

    let msg = SaleFinalizedHookMsg {
        collection: collection.to_string(),
        token_id,
        seller: seller.to_string(),
        buyer: recipient.to_string(),
    };

    let submsg = SALE_FINALIZED_HOOKS.prepare_hooks(deps.storage, |h| {
        msg.clone().into_cosmos_msg(h).map(SubMsg::new)
    })?;

    Ok((msgs, submsg))
}

/// Payout a bid
fn payout(
    deps: Deps,
    collection: Addr,
    payment: Coin,
    payment_recipient: Addr,
) -> StdResult<Vec<CosmosMsg>> {
    let config = SUDO_PARAMS.load(deps.storage)?;

    // Will hold payment msgs
    let mut msgs: Vec<CosmosMsg> = vec![];

    // Append Fair Burn message
    let fee_percent = Decimal::percent(config.trading_fee_percent as u64);
    let network_fee = payment.amount * fee_percent;
    msgs.append(&mut fair_burn(network_fee.u128(), None));

    // Check if token supports Royalties
    let collection_info: CollectionInfoResponse = deps
        .querier
        .query_wasm_smart(collection, &Sg721QueryMsg::CollectionInfo {})?;

    // If token supports royalities, payout shares
    if let Some(royalty) = collection_info.royalty_info {
        let royalty_share_msg = BankMsg::Send {
            to_address: royalty.payment_address.to_string(),
            amount: vec![Coin {
                amount: payment.amount * royalty.share,
                denom: payment.denom.clone(),
            }],
        };
        msgs.append(&mut vec![royalty_share_msg.into()]);

        let owner_share_msg = BankMsg::Send {
            to_address: payment_recipient.to_string(),
            amount: vec![Coin {
                amount: payment.amount * (Decimal::one() - royalty.share) - network_fee,
                denom: payment.denom,
            }],
        };
        msgs.append(&mut vec![owner_share_msg.into()]);
    } else {
        // If token doesn't support royalties, pay owner in full
        let owner_share_msg = BankMsg::Send {
            to_address: payment_recipient.to_string(),
            amount: vec![Coin {
                amount: payment.amount - network_fee,
                denom: payment.denom,
            }],
        };
        msgs.append(&mut vec![owner_share_msg.into()]);
    }

    Ok(msgs)
}

fn expires_validate(
    env: &Env,
    expires: Timestamp,
    expiry: (u64, u64),
) -> Result<(), ContractError> {
    if expires <= env.block.time.plus_seconds(expiry.0)
        || expires > env.block.time.plus_seconds(expiry.1)
    {
        return Err(ContractError::InvalidExpiration {});
    }

    Ok(())
}

fn price_validate(price: &Coin) -> Result<(), ContractError> {
    if price.amount.is_zero() || price.denom != NATIVE_DENOM {
        return Err(ContractError::InvalidPrice {});
    }

    Ok(())
}
