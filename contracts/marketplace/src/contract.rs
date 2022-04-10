use crate::error::ContractError;
use crate::msg::{
    AskInfo, AsksResponse, BidInfo, BidResponse, BidsResponse, CollectionsResponse,
    CurrentAskResponse, ExecuteMsg, InstantiateMsg, QueryMsg,
};
use crate::state::{asks, Ask, TOKEN_ASKS, TOKEN_BIDS};
use cosmwasm_std::{
    coin, entry_point, to_binary, Addr, BankMsg, Binary, Coin, Decimal, Deps, DepsMut, Env,
    MessageInfo, Order, StdResult, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw721::{Cw721ExecuteMsg, Cw721QueryMsg, OwnerOfResponse};
use cw_storage_plus::{Bound, PrefixBound};
use cw_utils::{maybe_addr, must_pay};
use sg721::msg::{CollectionInfoResponse, QueryMsg as Sg721QueryMsg};
use sg_std::{fair_burn, CosmosMsg, Response, NATIVE_DENOM};

// Version info for migration info
const CONTRACT_NAME: &str = "crates.io:sg-marketplace";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// Query limits
const DEFAULT_QUERY_LIMIT: u32 = 10;
const MAX_QUERY_LIMIT: u32 = 30;

// Governance parameters
const TRADING_FEE_PERCENT: u32 = 2;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::new().add_attribute("action", "instantiate"))
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
        ExecuteMsg::SetBid {
            collection,
            token_id,
        } => execute_set_bid(deps, info, api.addr_validate(&collection)?, token_id),
        ExecuteMsg::RemoveBid {
            collection,
            token_id,
        } => execute_remove_bid(deps, env, info, api.addr_validate(&collection)?, token_id),
        ExecuteMsg::SetAsk {
            collection,
            token_id,
            price,
            funds_recipient,
        } => execute_set_ask(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            token_id,
            price,
            funds_recipient.map(|addr| api.addr_validate(&addr).unwrap()),
        ),
        ExecuteMsg::RemoveAsk {
            collection,
            token_id,
        } => execute_remove_ask(deps, info, api.addr_validate(&collection)?, token_id),
        ExecuteMsg::AcceptBid {
            collection,
            token_id,
            bidder,
        } => execute_accept_bid(
            deps,
            info,
            api.addr_validate(&collection)?,
            token_id,
            api.addr_validate(&bidder)?,
        ),
    }
}

/// Anyone may place a bid on a listed NFT. By placing a bid, the bidder sends STARS to the market contract.
pub fn execute_set_bid(
    deps: DepsMut,
    info: MessageInfo,
    collection: Addr,
    token_id: u32,
) -> Result<Response, ContractError> {
    // Make sure a bid amount was sent
    let bid_price = must_pay(&info, NATIVE_DENOM)?;
    let bidder = info.sender;
    let mut res = Response::new();

    // Check bidder has existing bid, if so remove existing bid
    if let Some(existing_bid) =
        TOKEN_BIDS.may_load(deps.storage, (&collection, token_id, &bidder))?
    {
        TOKEN_BIDS.remove(deps.storage, (&collection, token_id, &bidder));
        let exec_refund_bidder = BankMsg::Send {
            to_address: bidder.to_string(),
            amount: vec![coin(existing_bid.u128(), NATIVE_DENOM)],
        };
        res = res.add_message(exec_refund_bidder)
    };

    let ask = asks().load(deps.storage, ask_key(collection.clone(), token_id))?;
    if ask.price != bid_price {
        // Bid does not meet ask criteria, store bid
        store_bid(
            deps,
            bidder.clone(),
            collection.clone(),
            token_id,
            bid_price,
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
        let msgs = finalize_sale(
            deps,
            collection.clone(),
            token_id,
            bidder.clone(),
            ask.funds_recipient.unwrap_or(owner),
            coin(ask.price.u128(), NATIVE_DENOM),
        )?;

        res = res
            .add_attribute("action", "sale_finalized")
            .add_messages(msgs);
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
    token_id: u32,
) -> Result<Response, ContractError> {
    let bidder = info.sender;

    // Check bid exists for bidder
    let bid = TOKEN_BIDS.load(deps.storage, (&collection, token_id, &bidder))?;

    // Remove bid
    TOKEN_BIDS.remove(deps.storage, (&collection, token_id, &bidder));

    // Refund bidder
    let exec_refund_bidder = BankMsg::Send {
        to_address: bidder.to_string(),
        amount: vec![coin(bid.u128(), NATIVE_DENOM)],
    };

    Ok(Response::new()
        .add_attribute("action", "remove_bid")
        .add_attribute("collection", collection)
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("bidder", bidder)
        .add_message(exec_refund_bidder))
}

/// An owner may set an Ask on their media. A bid is automatically fulfilled if it meets the asking price.
pub fn execute_set_ask(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    token_id: u32,
    price: Coin,
    funds_recipient: Option<Addr>,
) -> Result<Response, ContractError> {
    // Only the media onwer can call this
    let owner_of_response = check_only_owner(deps.as_ref(), &info, collection.clone(), token_id)?;
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
    token_id: u32,
) -> Result<Response, ContractError> {
    check_only_owner(deps.as_ref(), &info, collection.clone(), token_id)?;

    asks().remove(deps.storage, (collection.clone(), token_id))?;

    Ok(Response::new()
        .add_attribute("action", "remove_ask")
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id.to_string()))
}

/// Owner can accept a bid which transfers funds as well as the token
pub fn execute_accept_bid(
    deps: DepsMut,
    info: MessageInfo,
    collection: Addr,
    token_id: u32,
    bidder: Addr,
) -> Result<Response, ContractError> {
    check_only_owner(deps.as_ref(), &info, collection.clone(), token_id)?;

    // Query current ask
    let ask = TOKEN_ASKS.load(deps.storage, (&collection, token_id))?;
    // Remove ask
    TOKEN_ASKS.remove(deps.storage, (&collection, token_id));

    // Query accepted bid
    let bid = TOKEN_BIDS.load(deps.storage, (&collection, token_id, &bidder))?;
    // Remove accepted bid
    TOKEN_BIDS.remove(deps.storage, (&collection, token_id, &bidder));

    // Transfer funds and NFT
    let msgs = finalize_sale(
        deps,
        collection.clone(),
        token_id,
        bidder.clone(),
        ask.funds_recipient.unwrap_or(info.sender),
        coin(bid.u128(), NATIVE_DENOM),
    )?;

    Ok(Response::new()
        .add_attribute("action", "accept_bid")
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("bidder", bidder)
        .add_messages(msgs))
}

/// Checks to enfore only nft owner can call
fn check_only_owner(
    deps: Deps,
    info: &MessageInfo,
    collection: Addr,
    token_id: u32,
) -> Result<OwnerOfResponse, ContractError> {
    let owner: cw721::OwnerOfResponse = deps.querier.query_wasm_smart(
        collection,
        &Cw721QueryMsg::OwnerOf {
            token_id: token_id.to_string(),
            include_expired: None,
        },
    )?;
    if owner.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    Ok(owner)
}

/// Transfers funds and NFT, updates bid
fn finalize_sale(
    deps: DepsMut,
    collection: Addr,
    token_id: u32,
    recipient: Addr,
    funds_recipient: Addr,
    price: Coin,
) -> StdResult<Vec<CosmosMsg>> {
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

    Ok(msgs)
}

/// Payout a bid
fn payout(
    deps: Deps,
    collection: Addr,
    payment: Coin,
    payment_recipient: Addr,
) -> StdResult<Vec<CosmosMsg>> {
    // Will hold payment msgs
    let mut msgs: Vec<CosmosMsg> = vec![];

    // Append Fair Burn message
    let fee_percent = Decimal::percent(TRADING_FEE_PERCENT as u64);
    let network_fee = payment.amount * fee_percent;
    msgs.append(&mut fair_burn(network_fee.u128()));

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

fn store_bid(
    deps: DepsMut,
    bidder: Addr,
    collection: Addr,
    token_id: u32,
    bid_price: Uint128,
) -> StdResult<()> {
    TOKEN_BIDS.save(deps.storage, (&collection, token_id, &bidder), &bid_price)
}

fn ask_key(collection: Addr, token_id: u32) -> (Addr, u32) {
    (collection, token_id)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    let api = deps.api;

    match msg {
        QueryMsg::CurrentAsk {
            collection,
            token_id,
        } => to_binary(&query_current_ask(
            deps,
            api.addr_validate(&collection)?,
            token_id,
        )?),
        QueryMsg::Bid {
            collection,
            token_id,
            bidder,
        } => to_binary(&query_bid(
            deps,
            api.addr_validate(&collection)?,
            token_id,
            api.addr_validate(&bidder)?,
        )?),
        QueryMsg::Bids {
            collection,
            token_id,
            start_after,
            limit,
        } => to_binary(&query_bids(
            deps,
            api.addr_validate(&collection)?,
            token_id,
            start_after,
            limit,
        )?),
        QueryMsg::Asks {
            collection,
            start_after,
            limit,
        } => to_binary(&query_asks(
            deps,
            api.addr_validate(&collection)?,
            start_after,
            limit,
        )?),
        QueryMsg::ListedCollections { start_after, limit } => {
            to_binary(&query_listed_collections(deps, start_after, limit)?)
        }
    }
}

pub fn query_asks(
    deps: Deps,
    collection: Addr,
    start_after: Option<u32>,
    limit: Option<u32>,
) -> StdResult<AsksResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let asks: StdResult<Vec<_>> = TOKEN_ASKS
        .prefix(&collection)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (token_id, ask) = item?;
            Ok(AskInfo {
                seller: ask.seller,
                token_id,
                price: coin(ask.price.u128(), NATIVE_DENOM),
                funds_recipient: ask.funds_recipient,
            })
        })
        .collect();

    Ok(AsksResponse { asks: asks? })
}

pub fn query_listed_collections(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<CollectionsResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;
    let start_addr = maybe_addr(deps.api, start_after)?;
    let start = start_addr.as_ref().map(PrefixBound::exclusive);

    let collections: StdResult<Vec<_>> = TOKEN_ASKS
        .prefix_range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| item.map(|(key, _)| key.0))
        .collect();

    Ok(CollectionsResponse {
        collections: collections?,
    })
}

pub fn query_current_ask(
    deps: Deps,
    collection: Addr,
    token_id: u32,
) -> StdResult<CurrentAskResponse> {
    let ask = TOKEN_ASKS.may_load(deps.storage, (&collection, token_id))?;

    Ok(CurrentAskResponse { ask })
}

pub fn query_bid(
    deps: Deps,
    collection: Addr,
    token_id: u32,
    bidder: Addr,
) -> StdResult<BidResponse> {
    let bid = TOKEN_BIDS.may_load(deps.storage, (&collection, token_id, &bidder))?;

    Ok(BidResponse { bid })
}

pub fn query_bids(
    deps: Deps,
    collection: Addr,
    token_id: u32,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<BidsResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;
    let start_addr = maybe_addr(deps.api, start_after)?;
    let start = start_addr.as_ref().map(Bound::exclusive);

    let bids: StdResult<Vec<BidInfo>> = TOKEN_BIDS
        .prefix((&collection, token_id))
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_k, v) = item?;
            Ok(BidInfo {
                token_id,
                price: coin(v.u128(), NATIVE_DENOM),
            })
        })
        .collect();

    Ok(BidsResponse { bids: bids? })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coin, coins};
    use sg_std::NATIVE_DENOM;

    const CREATOR: &str = "creator";
    const COLLECTION: &str = "collection";
    const TOKEN_ID: u32 = 123;

    #[test]
    fn ask_indexed_map() {
        let mut deps = mock_dependencies();
        let collection = Addr::unchecked(COLLECTION);

        let ask = Ask {
            collection: collection.clone(),
            token_id: TOKEN_ID,
            seller: Addr::unchecked("seller"),
            price: Uint128::from(500u128),
            funds_recipient: None,
        };
        let key = ask_key(collection, TOKEN_ID);
        let res = asks().save(deps.as_mut().storage, key.clone(), &ask);
        assert!(res.is_ok());

        let res = asks().load(deps.as_ref().storage, key);
        assert_eq!(res.unwrap(), ask);
    }

    fn setup_contract(deps: DepsMut) {
        let msg = InstantiateMsg {};
        let info = mock_info(CREATOR, &[]);
        let res = instantiate(deps, mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(1000, NATIVE_DENOM));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn try_set_bid() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let broke = mock_info("broke", &[]);
        let bidder = mock_info("bidder", &coins(1000, NATIVE_DENOM));

        let set_bid_msg = ExecuteMsg::SetBid {
            collection: COLLECTION.to_string(),
            token_id: TOKEN_ID,
        };

        // Broke bidder calls Set Bid and gets an error
        let err = execute(deps.as_mut(), mock_env(), broke, set_bid_msg).unwrap_err();
        assert_eq!(
            err,
            ContractError::BidPaymentError(cw_utils::PaymentError::NoFunds {})
        );

        let set_bid_msg = ExecuteMsg::SetBid {
            collection: COLLECTION.to_string(),
            token_id: TOKEN_ID,
        };

        // Bidder calls SetBid before an Ask is set, so it should fail
        let err = execute(deps.as_mut(), mock_env(), bidder, set_bid_msg).unwrap_err();
        assert_eq!(err, ContractError::AskDoesNotExist {});
    }

    #[test]
    fn try_set_ask() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let set_ask = ExecuteMsg::SetAsk {
            collection: COLLECTION.to_string(),
            token_id: TOKEN_ID,
            price: coin(100, NATIVE_DENOM),
            funds_recipient: None,
        };

        // Reject if not called by the media owner
        let not_allowed = mock_info("random", &[]);
        let err = execute(deps.as_mut(), mock_env(), not_allowed, set_ask);
        assert!(err.is_err());
    }
}
