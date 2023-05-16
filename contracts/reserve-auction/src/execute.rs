#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use sg1::fair_burn;

use crate::error::ContractError;
use crate::helpers::{only_no_auction, settle_auction};
use crate::msg::ExecuteMsg;
use crate::state::CONFIG;
use crate::state::{auctions, Auction, HighBid};
use cosmwasm_std::{
    coin, ensure, ensure_eq, has_coins, Addr, Coin, DepsMut, Env, Event, MessageInfo, Timestamp,
    Uint128,
};
use cw_utils::{maybe_addr, must_pay, nonpayable};
use sg_marketplace_common::{has_approval, only_owner, transfer_nft, transfer_token};
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
            duration,
            seller_funds_recipient,
        } => execute_create_auction(
            deps,
            info,
            env,
            api.addr_validate(&collection)?,
            &token_id,
            duration,
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
            env,
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
    duration: u64,
    reserve_price: Coin,
    seller_funds_recipient: Option<Addr>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Only NFT owner can create auction for an NFT
    only_owner(&deps.querier, &info, &collection, token_id)?;

    // NFT owner must have approved the reserve auction contract to transfer the NFT
    has_approval(
        &deps.querier,
        &env.contract.address,
        &collection,
        token_id,
        Some(false),
    )?;

    // Cannot create a duplicate auction for an NFT
    only_no_auction(deps.as_ref(), &collection, token_id)?;

    let mut response = Response::new();

    // If create auction fee is greater than zero, then pay the fee
    if config.create_auction_fee > Uint128::zero() {
        let fee = must_pay(&info, NATIVE_DENOM)?;
        ensure_eq!(
            fee,
            config.create_auction_fee,
            ContractError::WrongFee {
                required: config.create_auction_fee,
                given: fee,
            }
        );
        fair_burn(fee.u128(), None, &mut response);
    } else {
        nonpayable(&info)?;
    }

    // Ensure that the reserve price is greater than the minimum reserve price
    let min_reserve_price = config.min_reserve_price_coin();
    ensure!(
        has_coins(&[reserve_price.clone()], &min_reserve_price),
        ContractError::InvalidReservePrice {
            min: min_reserve_price,
        }
    );

    // Ensure that the duration is within the min and max duration
    ensure!(
        duration >= config.min_duration && duration <= config.max_duration,
        ContractError::InvalidDuration {
            min: config.min_duration,
            max: config.max_duration,
            found: duration
        }
    );

    let auction = Auction {
        token_id: token_id.to_string(),
        collection: collection.clone(),
        seller: info.sender,
        reserve_price,
        duration,
        end_time: None,
        seller_funds_recipient,
        high_bid: None,
        first_bid_time: None,
    };

    auction.save(deps.storage)?;

    let mut event = Event::new("create-auction")
        .add_attribute("collection", auction.collection.to_string())
        .add_attribute("token_id", auction.token_id.to_string())
        .add_attribute("seller", auction.seller.to_string())
        .add_attribute("reserve_price", auction.reserve_price.to_string());
    if auction.seller_funds_recipient.is_some() {
        event = event.add_attribute(
            "seller_funds_recipient",
            auction.seller_funds_recipient.unwrap().to_string(),
        );
    }

    response = response.add_event(event).add_submessage(transfer_nft(
        &collection,
        token_id,
        &env.contract.address,
    ));

    Ok(response)
}

pub fn execute_update_reserve_price(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    collection: Addr,
    token_id: String,
    reserve_price: Coin,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;
    let config = CONFIG.load(deps.storage)?;
    let mut auction = auctions().load(deps.storage, (collection, token_id))?;

    // Ensure caller is the seller
    ensure_eq!(auction.seller, info.sender, ContractError::Unauthorized {});

    // Ensure auction hasn't started
    ensure!(
        auction.first_bid_time.is_none(),
        ContractError::AuctionStarted {}
    );

    // Ensure that the reserve price is greater than the minimum reserve price
    let min_reserve_price = config.min_reserve_price_coin();
    ensure!(
        has_coins(&[reserve_price.clone()], &min_reserve_price),
        ContractError::InvalidReservePrice {
            min: min_reserve_price,
        }
    );

    // Update reserve price
    auction.reserve_price = reserve_price;
    auction.save(deps.storage)?;

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

    // Ensure caller is the seller
    ensure_eq!(auction.seller, info.sender, ContractError::Unauthorized {});

    // Ensure auction hasn't started
    ensure!(
        auction.first_bid_time.is_none(),
        ContractError::AuctionStarted {}
    );

    // Remove auction from storage
    auction.remove(deps.storage)?;

    let event = Event::new("cancel-auction")
        .add_attribute("collection", auction.collection.to_string())
        .add_attribute("token_id", auction.token_id.to_string());

    let response = Response::new()
        .add_event(event)
        .add_submessage(transfer_nft(&collection, token_id, &auction.seller));

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

    let mut auction = auctions().load(deps.storage, (collection, token_id.to_string()))?;

    let bid_amount = must_pay(&info, NATIVE_DENOM)?;

    let mut response = Response::new();
    let block_time = env.block.time;

    // Ensure seller is not the bidder
    ensure!(
        auction.seller != info.sender,
        ContractError::SellerShouldNotBid {}
    );

    // Ensure minimum bid amount is met
    let min_bid = auction.min_bid_coin(config.min_bid_increment_pct);
    ensure!(
        has_coins(&info.funds, &min_bid),
        ContractError::BidTooLow(min_bid.amount)
    );

    // Ensure auction has not ended
    ensure!(
        auction.end_time.is_none() || auction.end_time.unwrap().seconds() > block_time.seconds(),
        ContractError::AuctionEnded {}
    );

    match auction.first_bid_time {
        // If this is the first bid, set the first_bid_time and end_time
        None => {
            auction.first_bid_time = Some(block_time);
            auction.end_time = Some(block_time.plus_seconds(auction.duration));
        }
        // If this is not the first bid, refund previous bidder and extend end_time if necessary
        Some(_) => {
            // Refund previous bidder
            let high_bid = auction.high_bid.unwrap();
            response = response.add_submessage(transfer_token(high_bid.coin, &high_bid.bidder));

            let time_remaining = auction.end_time.unwrap().seconds() - block_time.seconds();
            if time_remaining <= config.extend_duration {
                auction.end_time = Some(block_time.plus_seconds(config.extend_duration));
            }
        }
    };

    auction.high_bid = Some(HighBid {
        bidder: info.sender,
        coin: coin(bid_amount.u128(), NATIVE_DENOM),
    });

    auction.save(deps.storage)?;

    let high_bid = &auction.high_bid.unwrap();
    let event = Event::new("place-bid")
        .add_attribute("collection", auction.collection.to_string())
        .add_attribute("token_id", auction.token_id)
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
    let auction = auctions().load(deps.storage, (collection, token_id.to_string()))?;

    let response = Response::new();

    settle_auction(deps, block_time, &config, auction, response)
}
