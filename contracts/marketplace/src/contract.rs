use crate::error::ContractError;
use crate::msg::{
    BidInfo, BidResponse, BidsResponse, CurrentAskResponse, ExecuteMsg, InstantiateMsg, QueryMsg,
};
use crate::state::{Ask, Bid, TOKEN_ASKS, TOKEN_BIDS};
use cosmwasm_std::{
    coin, entry_point, to_binary, Addr, BankMsg, Binary, Coin, Decimal, Deps, DepsMut, Env,
    MessageInfo, Order, StdResult, WasmMsg,
};
use cw2::set_contract_version;
use cw721::{Cw721ExecuteMsg, Cw721QueryMsg, OwnerOfResponse};
use cw_storage_plus::Bound;
use cw_utils::{maybe_addr, must_pay};
use sg721::msg::{CollectionInfoResponse, QueryMsg as Sg721QueryMsg};
use sg_std::{CosmosMsg, Response, NATIVE_DENOM};

// Version info for migration info
const CONTRACT_NAME: &str = "crates.io:sg-marketplace";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// Query limits
const DEFAULT_QUERY_LIMIT: u32 = 10;
const MAX_QUERY_LIMIT: u32 = 30;

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
        } => execute_set_bid(deps, info, api.addr_validate(&collection)?, &token_id),
        ExecuteMsg::RemoveBid {
            collection,
            token_id,
        } => execute_remove_bid(deps, env, info, api.addr_validate(&collection)?, &token_id),
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
            &token_id,
            price,
            funds_recipient.map(|addr| api.addr_validate(&addr).unwrap()),
        ),
        ExecuteMsg::RemoveAsk {
            collection,
            token_id,
        } => execute_remove_ask(deps, info, api.addr_validate(&collection)?, &token_id),
        ExecuteMsg::AcceptBid {
            collection,
            token_id,
            bidder,
        } => execute_accept_bid(
            deps,
            info,
            api.addr_validate(&collection)?,
            &token_id,
            api.addr_validate(&bidder)?,
        ),
    }
}

/// Anyone may place a bid on a minted token. By placing a bid, the bidder sends a native Coin to the market contract.
pub fn execute_set_bid(
    deps: DepsMut,
    info: MessageInfo,
    collection: Addr,
    token_id: &str,
) -> Result<Response, ContractError> {
    // Make sure a bid amount was sent
    let bid_price = must_pay(&info, NATIVE_DENOM)?;

    let mut res = Response::new();

    // Check bidder has existing bid, if so remove existing bid
    if let Some(existing_bid) =
        TOKEN_BIDS.may_load(deps.storage, (&collection, token_id, &info.sender))?
    {
        TOKEN_BIDS.remove(deps.storage, (&collection, token_id, &info.sender));
        let exec_refund_bidder = BankMsg::Send {
            to_address: existing_bid.bidder.to_string(),
            amount: vec![existing_bid.price],
        };
        res = res.add_message(exec_refund_bidder)
    };

    match TOKEN_ASKS.may_load(deps.storage, (&collection, token_id))? {
        Some(ask) => {
            // Check if bid meets ask criteria and finalize sale if so
            if ask.price.amount == bid_price {
                TOKEN_ASKS.remove(deps.storage, (&collection, token_id));
                // Include messages needed to finalize nft transfer and payout
                let msgs: Vec<CosmosMsg> = finalize_sale(
                    deps,
                    collection.clone(),
                    token_id,
                    info.sender.clone(),
                    ask.funds_recipient.unwrap_or(info.sender.clone()),
                    ask.price,
                )?;

                res = res
                    .add_attribute("action", "sale_finalized")
                    .add_messages(msgs);
            }
        }
        None => {
            TOKEN_BIDS.save(
                deps.storage,
                (&collection, token_id, &info.sender),
                &Bid {
                    price: coin(bid_price.u128(), NATIVE_DENOM),
                    bidder: info.sender.clone(),
                },
            )?;
            res = res.add_attribute("action", "set_bid");
        }
    }

    Ok(res
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id)
        .add_attribute("bidder", info.sender)
        .add_attribute("bid_price", bid_price.to_string()))
}

/// Removes a bid made by the bidder. Bidders can only remove their own bids
pub fn execute_remove_bid(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    collection: Addr,
    token_id: &str,
) -> Result<Response, ContractError> {
    // Check bid exists for bidder
    let bid = TOKEN_BIDS.load(deps.storage, (&collection, token_id, &info.sender))?;

    // Remove bid
    TOKEN_BIDS.remove(deps.storage, (&collection, token_id, &info.sender));

    // Refund bidder
    let exec_refund_bidder = BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![bid.price],
    };

    Ok(Response::new()
        .add_attribute("action", "remove_bid")
        .add_attribute("collection", collection)
        .add_attribute("token_id", token_id)
        .add_attribute("bidder", info.sender.to_string())
        .add_message(exec_refund_bidder))
}

/// An owner may set an Ask on their media. A bid is automatically fulfilled if it meets the asking price.
pub fn execute_set_ask(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    token_id: &str,
    price: Coin,
    funds_recipient: Option<Addr>,
) -> Result<Response, ContractError> {
    // Only the media onwer can call this
    let owner_of_response = check_only_owner(deps.as_ref(), &info, collection.clone(), &token_id)?;
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
    TOKEN_ASKS.save(
        deps.storage,
        (&collection, token_id),
        &Ask {
            price: price.clone(),
            funds_recipient,
        },
    )?;
    Ok(Response::new()
        .add_attribute("action", "set_ask")
        .add_attribute("collection", collection)
        .add_attribute("token_id", token_id)
        .add_attribute("price", price.to_string()))
}

/// Removes the ask on a particular media
pub fn execute_remove_ask(
    deps: DepsMut,
    info: MessageInfo,
    collection: Addr,
    token_id: &str,
) -> Result<Response, ContractError> {
    check_only_owner(deps.as_ref(), &info, collection.clone(), token_id)?;

    TOKEN_ASKS.remove(deps.storage, (&collection, token_id));

    Ok(Response::new()
        .add_attribute("action", "remove_ask")
        .add_attribute("collection", collection)
        .add_attribute("token_id", token_id))
}

/// Owner can accept a bid which transfers funds as well as the token
pub fn execute_accept_bid(
    deps: DepsMut,
    info: MessageInfo,
    collection: Addr,
    token_id: &str,
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
        info.sender.clone(),
        ask.funds_recipient.unwrap_or(info.sender),
        bid.price.clone(),
    )?;

    Ok(Response::new()
        .add_attribute("action", "accept_bid")
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id)
        .add_attribute("bidder", bid.bidder)
        .add_messages(msgs))
}

/// Checks to enfore only nft owner can call
pub fn check_only_owner(
    deps: Deps,
    info: &MessageInfo,
    collection: Addr,
    token_id: &str,
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
pub fn finalize_sale(
    deps: DepsMut,
    collection: Addr,
    token_id: &str,
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
pub fn payout(
    deps: Deps,
    collection: Addr,
    payment: Coin,
    payment_recipient: Addr,
) -> StdResult<Vec<CosmosMsg>> {
    // Will hold payment msgs
    let mut msgs: Vec<CosmosMsg> = vec![];

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
                amount: payment.amount * (Decimal::one() - royalty.share),
                denom: payment.denom,
            }],
        };
        msgs.append(&mut vec![owner_share_msg.into()]);
    } else {
        // If token doesn't support royalties, pay owner in full
        let owner_share_msg = BankMsg::Send {
            to_address: payment_recipient.to_string(),
            amount: vec![Coin {
                amount: payment.amount,
                denom: payment.denom,
            }],
        };
        msgs.append(&mut vec![owner_share_msg.into()]);
    }

    Ok(msgs)
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
            &token_id,
        )?),
        QueryMsg::Bid {
            collection,
            token_id,
            bidder,
        } => to_binary(&query_bid(
            deps,
            api.addr_validate(&collection)?,
            &token_id,
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
            &token_id,
            start_after,
            limit,
        )?),
    }
}

pub fn query_current_ask(
    deps: Deps,
    collection: Addr,
    token_id: &str,
) -> StdResult<CurrentAskResponse> {
    let ask = TOKEN_ASKS.may_load(deps.storage, (&collection, token_id))?;
    Ok(CurrentAskResponse { ask })
}

pub fn query_bid(
    deps: Deps,
    collection: Addr,
    token_id: &str,
    bidder: Addr,
) -> StdResult<BidResponse> {
    let bid_info = TOKEN_BIDS
        .may_load(deps.storage, (&collection, token_id, &bidder))?
        .map(|b| BidInfo {
            price: b.price,
            bidder: b.bidder.to_string(),
        });

    Ok(BidResponse { bid_info })
}

pub fn query_bids(
    deps: Deps,
    collection: Addr,
    token_id: &str,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<BidsResponse> {
    let limit = limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize;
    let start_addr = maybe_addr(deps.api, start_after)?;
    let start = start_addr.as_ref().map(Bound::exclusive);

    let bid_infos: StdResult<Vec<BidInfo>> = TOKEN_BIDS
        .prefix((&collection, token_id))
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_k, v) = item?;
            Ok(BidInfo {
                price: v.price,
                bidder: v.bidder.to_string(),
            })
        })
        .collect();

    Ok(BidsResponse {
        bid_infos: bid_infos?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coin, coins, from_binary};
    use sg_std::NATIVE_DENOM;

    const CREATOR: &str = "creator";
    const COLLECTION: &str = "collection";
    const TOKEN_ID: &str = "123";

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
    fn set_and_remove_bid() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let broke = mock_info("broke", &[]);
        let bidder = mock_info("bidder", &coins(1000, NATIVE_DENOM));
        let random_addr = mock_info("random", &coins(1000, NATIVE_DENOM));

        let set_bid_msg = ExecuteMsg::SetBid {
            collection: COLLECTION.to_string(),
            token_id: TOKEN_ID.to_string(),
        };

        // Broke bidder calls Set Bid and gets an error
        let err = execute(deps.as_mut(), mock_env(), broke, set_bid_msg).unwrap_err();
        assert_eq!(
            err,
            ContractError::BidPaymentError(cw_utils::PaymentError::NoFunds {})
        );

        let set_bid_msg = ExecuteMsg::SetBid {
            collection: COLLECTION.to_string(),
            token_id: TOKEN_ID.to_string(),
        };

        // Bidder calls Set Bid successfully
        let res = execute(deps.as_mut(), mock_env(), bidder.clone(), set_bid_msg);
        assert!(res.is_ok());

        // Query for bid
        let query_bid_msg = QueryMsg::Bid {
            collection: COLLECTION.to_string(),
            token_id: TOKEN_ID.to_string(),
            bidder: bidder.sender.to_string(),
        };

        let q = query(deps.as_ref(), mock_env(), query_bid_msg).unwrap();
        let value: BidResponse = from_binary(&q).unwrap();
        let bid_info = BidInfo {
            price: coin(1000, NATIVE_DENOM),
            bidder: bidder.sender.to_string(),
        };
        assert_eq!(
            value,
            BidResponse {
                bid_info: Some(bid_info)
            }
        );

        // Query for list of bids
        let bids_query_msg = QueryMsg::Bids {
            collection: COLLECTION.to_string(),
            token_id: TOKEN_ID.to_string(),
            start_after: None,
            limit: None,
        };
        let q = query(deps.as_ref(), mock_env(), bids_query_msg).unwrap();
        let value: BidsResponse = from_binary(&q).unwrap();
        assert_eq!(value.bid_infos.len(), 1);

        // Remove bid
        let remove_bid_msg = ExecuteMsg::RemoveBid {
            collection: COLLECTION.to_string(),
            token_id: TOKEN_ID.to_string(),
        };

        // Random address can't remove bid
        let res = execute(
            deps.as_mut(),
            mock_env(),
            random_addr,
            remove_bid_msg.clone(),
        );
        assert!(res.is_err());

        // Bidder can remove bid
        let res = execute(deps.as_mut(), mock_env(), bidder, remove_bid_msg).unwrap();

        // Check Bank msg was added for refund
        assert_eq!(1, res.messages.len());
    }

    #[test]
    fn set_ask() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let set_ask = ExecuteMsg::SetAsk {
            collection: COLLECTION.to_string(),
            token_id: TOKEN_ID.to_string(),
            price: coin(100, NATIVE_DENOM),
            funds_recipient: None,
        };

        // Reject if not called by the media owner
        let not_allowed = mock_info("random", &[]);
        let err = execute(deps.as_mut(), mock_env(), not_allowed, set_ask);
        assert!(err.is_err());
    }
}
