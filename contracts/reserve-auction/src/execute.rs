use std::vec;

use crate::error::ContractError;
use crate::helpers::{only_no_auction, settle_auction, validate_reserve_price};
use crate::msg::ExecuteMsg;
use crate::state::{auctions, Auction, HighBid};
use crate::state::{CONFIG, HALT_MANAGER};
use cosmwasm_std::{
    attr, coin, ensure, ensure_eq, has_coins, Addr, Coin, DepsMut, Env, Event, MessageInfo,
    Timestamp,
};
use cw_utils::{maybe_addr, must_pay, nonpayable};
use sg_marketplace_common::{
    coin::checked_transfer_coin,
    nft::{has_approval, only_owner, only_tradable, transfer_nft},
};
use sg_std::Response;
use stargaze_fair_burn::append_fair_burn_msg;

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

    only_tradable(&deps.querier, &env.block, &collection)?;

    let mut response = Response::new();

    validate_reserve_price(deps.as_ref().storage, &reserve_price)?;

    // Ensure that the duration is within the min and max duration
    ensure!(
        duration >= config.min_duration && duration <= config.max_duration,
        ContractError::InvalidDuration {
            min: config.min_duration,
            max: config.max_duration,
            got: duration
        }
    );

    // Handle create auction fee payment
    if config.create_auction_fee.amount.is_zero() {
        // If create auction fee is zero, ensure no payment is sent
        nonpayable(&info)?;
    } else {
        // If create auction fee is non zero, ensure user has sent the correct amount,
        // and send it to the fair-burn contract
        ensure!(
            has_coins(&info.funds, &config.create_auction_fee),
            ContractError::WrongFee {
                expected: config.create_auction_fee,
            }
        );
        response = append_fair_burn_msg(
            &config.fair_burn,
            vec![config.create_auction_fee],
            None,
            response,
        );
    }

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

    auctions().save(
        deps.storage,
        (auction.collection.clone(), auction.token_id.clone()),
        &auction,
    )?;

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
    let mut auction = auctions().load(deps.storage, (collection, token_id))?;

    // Ensure caller is the seller
    ensure_eq!(auction.seller, info.sender, ContractError::Unauthorized {});

    // Ensure auction hasn't started
    ensure!(
        auction.first_bid_time.is_none(),
        ContractError::AuctionStarted {}
    );

    validate_reserve_price(deps.as_ref().storage, &reserve_price)?;

    // Update reserve price
    auction.reserve_price = reserve_price;
    auctions().save(
        deps.storage,
        (auction.collection.clone(), auction.token_id.clone()),
        &auction,
    )?;

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
    auctions().remove(
        deps.storage,
        (auction.collection.clone(), auction.token_id.clone()),
    )?;

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

    let auction_denom = auction.denom();
    let bid_amount = must_pay(&info, &auction_denom)?;

    let mut response = Response::new();
    let block_time = env.block.time;

    // Ensure seller is not the bidder
    ensure!(
        auction.seller != info.sender,
        ContractError::SellerShouldNotBid {}
    );

    // Ensure minimum bid amount is met
    let min_bid = auction.min_bid_coin(config.min_bid_increment_percent);
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
            let previous_high_bid = auction.high_bid.unwrap();
            response = response.add_submessage(checked_transfer_coin(
                previous_high_bid.coin.clone(),
                &previous_high_bid.bidder,
            )?);

            let time_remaining = auction.end_time.unwrap().seconds() - block_time.seconds();
            if time_remaining < config.extend_duration {
                auction.end_time = Some(block_time.plus_seconds(config.extend_duration));
            }

            let new_bid_amount = coin(bid_amount.u128(), auction_denom.clone());
            response = response.add_event(Event::new("refund-bid").add_attributes(vec![
                attr("collection", auction.collection.to_string()),
                attr("token_id", auction.token_id.clone()),
                attr("seller", auction.seller.to_string()),
                attr("new_bidder", info.sender.to_string()),
                attr("new_bid_amount", new_bid_amount.to_string()),
                attr("previous_bidder", previous_high_bid.bidder),
                attr("previous_bid_amount", previous_high_bid.coin.to_string()),
            ]));
        }
    };

    auction.high_bid = Some(HighBid {
        bidder: info.sender,
        coin: coin(bid_amount.u128(), auction_denom),
    });

    auctions().save(
        deps.storage,
        (auction.collection.clone(), auction.token_id.clone()),
        &auction,
    )?;

    let high_bid = &auction.high_bid.unwrap();
    let event = Event::new("place-bid")
        .add_attribute("collection", auction.collection.to_string())
        .add_attribute("token_id", auction.token_id)
        .add_attribute("seller", auction.seller.to_string())
        .add_attribute("bidder", high_bid.bidder.to_string())
        .add_attribute("bid_amount", high_bid.coin.to_string())
        .add_attribute("auction_end_time", auction.end_time.unwrap().to_string());
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
    let halt_manager = HALT_MANAGER.load(deps.storage)?;
    let auction = auctions().load(deps.storage, (collection, token_id.to_string()))?;

    let response = Response::new();

    settle_auction(deps, block_time, auction, &config, &halt_manager, response)
}
