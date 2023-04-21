#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use sg1::fair_burn;

use crate::error::ContractError;
use crate::helpers::{only_no_auction, settle_auction};
use crate::msg::ExecuteMsg;
use crate::state::CONFIG;
use crate::state::{auctions, Auction, HighBid};
use cosmwasm_std::{coin, has_coins, Addr, Coin, DepsMut, Env, Event, MessageInfo, Timestamp};
use cw_utils::{maybe_addr, must_pay, nonpayable};
use sg_marketplace_common::{bank_send, has_approval, only_owner, transfer_nft};
use sg_std::{Response, NATIVE_DENOM};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let api = deps.api;

    match msg {
        ExecuteMsg::CreateAuction {
            collection,
            token_id,
            reserve_price,
            start_time,
            end_time,
            seller_funds_recipient,
        } => execute_create_auction(
            deps,
            info,
            env,
            api.addr_validate(&collection)?,
            &token_id,
            start_time,
            end_time,
            reserve_price,
            maybe_addr(api, seller_funds_recipient)?,
        ),
        ExecuteMsg::UpdateReservePrice {
            collection,
            token_id,
            reserve_price,
        } => execute_update_reserve_price(
            deps,
            info,
            api.addr_validate(&collection)?,
            token_id,
            reserve_price,
        ),
        ExecuteMsg::CancelAuction {
            collection,
            token_id,
        } => execute_cancel_auction(deps, info, api.addr_validate(&collection)?, &token_id),
        ExecuteMsg::PlaceBid {
            collection,
            token_id,
        } => execute_place_bid(deps, env, info, api.addr_validate(&collection)?, &token_id),
        ExecuteMsg::SettleAuction {
            collection,
            token_id,
        } => execute_settle_auction(
            deps,
            info,
            env.block.time,
            api.addr_validate(&collection)?,
            &token_id,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn execute_create_auction(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    collection: Addr,
    token_id: &str,
    start_time: Timestamp,
    end_time: Timestamp,
    reserve_price: Coin,
    seller_funds_recipient: Option<Addr>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    only_owner(deps.as_ref(), &info, &collection, token_id)?;
    has_approval(deps.as_ref(), &env.contract.address, &collection, token_id)?;
    only_no_auction(deps.as_ref(), &collection, token_id)?;

    let mut response = Response::new();

    let fee = must_pay(&info, NATIVE_DENOM)?;
    if fee != config.create_auction_fee {
        return Err(ContractError::WrongFee {
            required: config.create_auction_fee,
            given: fee,
        });
    }
    fair_burn(fee.u128(), None, &mut response);

    if !has_coins(&[reserve_price.clone()], &config.min_reserve_price) {
        return Err(ContractError::InvalidReservePrice {
            min: config.min_reserve_price,
        });
    }

    if start_time < env.block.time {
        return Err(ContractError::InvalidStartTime {});
    }
    if end_time < start_time.plus_seconds(config.min_duration) {
        return Err(ContractError::InvalidEndTime {});
    }

    let auction = Auction {
        token_id: token_id.to_string(),
        collection: collection.clone(),
        seller: info.sender,
        reserve_price,
        start_time,
        end_time,
        seller_funds_recipient,
        high_bid: None,
        first_bid_time: None,
    };

    auctions().save(
        deps.storage,
        (collection.clone(), token_id.to_string()),
        &auction,
    )?;

    let event = Event::new("create-auction")
        .add_attribute("collection", auction.collection.to_string())
        .add_attribute("token_id", auction.token_id.to_string())
        .add_attribute("seller", auction.seller.to_string())
        .add_attribute("reserve_price", auction.reserve_price.to_string())
        .add_attribute("start_time", auction.start_time.to_string())
        .add_attribute("end_time", auction.end_time.to_string())
        .add_attribute(
            "seller_funds_recipient",
            auction
                .seller_funds_recipient
                .map_or("None".to_string(), |a| a.to_string()),
        );

    response = response.add_event(event).add_submessage(transfer_nft(
        collection,
        token_id,
        env.contract.address,
    )?);

    Ok(response)
}

pub fn execute_update_reserve_price(
    deps: DepsMut,
    info: MessageInfo,
    collection: Addr,
    token_id: String,
    reserve_price: Coin,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let config = CONFIG.load(deps.storage)?;
    let mut auction = auctions().load(deps.storage, (collection.clone(), token_id.clone()))?;

    // make sure caller is the seller
    if auction.seller != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    // make sure auction hasn't started
    if auction.first_bid_time.is_some() {
        return Err(ContractError::AuctionStarted {});
    }

    // make sure min reserve price is met
    if !has_coins(&[reserve_price.clone()], &config.min_reserve_price) {
        return Err(ContractError::InvalidReservePrice {
            min: config.min_reserve_price,
        });
    }

    // update reserve price
    auction.reserve_price = reserve_price;
    auctions().save(deps.storage, (collection, token_id), &auction)?;

    let event = Event::new("update-reserve-price")
        .add_attribute("token_id", auction.token_id.to_string())
        .add_attribute("collection", auction.collection.to_string())
        .add_attribute("reserve_price", auction.reserve_price.to_string());

    Ok(Response::new().add_event(event))
}

pub fn execute_cancel_auction(
    deps: DepsMut,
    info: MessageInfo,
    collection: Addr,
    token_id: &str,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let auction = auctions().load(deps.storage, (collection.clone(), token_id.to_string()))?;

    // make sure auction hasn't started
    if auction.first_bid_time.is_some() {
        return Err(ContractError::AuctionStarted {});
    }

    // make sure caller is the seller or a new owner
    if info.sender != auction.seller {
        return Err(ContractError::Unauthorized {});
    }

    // remove auction from storage
    auctions().remove(deps.storage, (collection.clone(), token_id.to_string()))?;

    let event = Event::new("cancel-auction")
        .add_attribute("collection", auction.collection.to_string())
        .add_attribute("token_id", auction.token_id.to_string());

    let response = Response::new()
        .add_event(event)
        .add_submessage(transfer_nft(collection, token_id, auction.seller)?);

    Ok(response)
}

pub fn execute_place_bid(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    token_id: &str,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let mut auction = auctions().load(deps.storage, (collection.clone(), token_id.to_string()))?;

    let denom = config.min_reserve_price.denom.clone();
    let bid = must_pay(&info, &denom)?;

    let mut response = Response::new();
    let block_time = env.block.time;

    // make sure seller is not bidder
    if auction.seller == info.sender {
        return Err(ContractError::SellerShouldNotBid {});
    }

    // make sure auction has started
    if block_time < auction.start_time {
        return Err(ContractError::AuctionNotStarted {});
    }

    // make sure auction has not ended
    if block_time >= auction.end_time {
        return Err(ContractError::AuctionEnded {});
    }

    match auction.first_bid_time {
        // if this is the first bid, start the auction
        None => {
            // ensure the reserve price has been met
            if !has_coins(&info.funds, &auction.reserve_price) {
                return Err(ContractError::ReserveNotMet {
                    min: auction.reserve_price,
                });
            }
            auction.first_bid_time = Some(block_time);
        }
        // first bid has been placed, and auction has started
        Some(_first_bid_time) => {
            // High bid guaranteed to exist when first_bid_time is set
            let min_bid = auction.min_bid(config.min_bid_increment_pct);
            if bid < min_bid {
                return Err(ContractError::BidTooLow(min_bid));
            }

            // refund previous bidder
            let high_bid = auction.high_bid.unwrap();
            response = response.add_submessage(bank_send(high_bid.coin, high_bid.bidder)?);
        }
    };

    auction.high_bid = Some(HighBid {
        bidder: info.sender,
        coin: coin(bid.u128(), denom),
    });

    let time_remaining = auction.end_time.seconds() - block_time.seconds();
    if time_remaining <= config.extend_duration {
        auction.end_time = block_time.plus_seconds(config.extend_duration);
    }

    auctions().save(deps.storage, (collection, token_id.to_string()), &auction)?;

    let high_bid = &auction.high_bid.unwrap();
    let event = Event::new("place-bid")
        .add_attribute("collection", auction.collection.to_string())
        .add_attribute("token_id", auction.token_id.to_string())
        .add_attribute("bidder", high_bid.bidder.to_string())
        .add_attribute("bid_amount", high_bid.coin.to_string());
    response = response.add_event(event);

    Ok(response)
}

pub fn execute_settle_auction(
    deps: DepsMut,
    info: MessageInfo,
    block_time: Timestamp,
    collection: Addr,
    token_id: &str,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let config = CONFIG.load(deps.storage)?;
    let auction = auctions().load(deps.storage, (collection.clone(), token_id.to_string()))?;

    let response = Response::new();

    settle_auction(deps, block_time, &config, auction, response)
}
