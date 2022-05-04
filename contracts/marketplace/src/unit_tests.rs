#[cfg(test)]
use std::vec;

use crate::error::ContractError;
use crate::execute::{execute, instantiate};
use crate::helpers::ExpiryRange;
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::query::{query_ask_count, query_asks_by_seller, query_bids_by_bidder};
use crate::state::{ask_key, asks, bid_key, bids, Ask, Bid, SaleType};

use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{coin, coins, Addr, DepsMut, Timestamp, Uint128};
use cw_utils::Duration;
use sg_std::NATIVE_DENOM;

const CREATOR: &str = "creator";
const COLLECTION: &str = "collection";
const TOKEN_ID: u32 = 123;

// Governance parameters
const TRADING_FEE_BASIS_POINTS: u64 = 200; // 2%
const MIN_EXPIRY: u64 = 24 * 60 * 60; // 24 hours (in seconds)
const MAX_EXPIRY: u64 = 180 * 24 * 60 * 60; // 6 months (in seconds)
const MAX_FINDERS_FEE_BPS: u64 = 1000; // 10%
const BID_REMOVAL_REWARD_BPS: u64 = 500; // 5%

#[test]
fn ask_indexed_map() {
    let mut deps = mock_dependencies();
    let collection = Addr::unchecked(COLLECTION);
    let seller = Addr::unchecked("seller");

    let ask = Ask {
        sale_type: SaleType::FixedPrice,
        collection: collection.clone(),
        token_id: TOKEN_ID,
        seller: seller.clone(),
        price: Uint128::from(500u128),
        funds_recipient: None,
        reserve_for: None,
        expires: Timestamp::from_seconds(0),
        is_active: true,
        finders_fee_bps: Some(0),
    };
    let key = ask_key(collection.clone(), TOKEN_ID);
    let res = asks().save(deps.as_mut().storage, key.clone(), &ask);
    assert!(res.is_ok());

    let ask2 = Ask {
        sale_type: SaleType::FixedPrice,
        collection: collection.clone(),
        token_id: TOKEN_ID + 1,
        seller: seller.clone(),
        price: Uint128::from(500u128),
        funds_recipient: None,
        reserve_for: None,
        expires: Timestamp::from_seconds(0),
        is_active: true,
        finders_fee_bps: Some(0),
    };
    let key2 = ask_key(collection.clone(), TOKEN_ID + 1);
    let res = asks().save(deps.as_mut().storage, key2, &ask2);
    assert!(res.is_ok());

    let res = asks().load(deps.as_ref().storage, key);
    assert_eq!(res.unwrap(), ask);

    let res = query_asks_by_seller(deps.as_ref(), seller, None, None).unwrap();
    assert_eq!(res.asks.len(), 2);
    assert_eq!(res.asks[0], ask);

    let res = query_ask_count(deps.as_ref(), collection).unwrap();
    assert_eq!(res.count, 2);
}

#[test]

fn bid_indexed_map() {
    let mut deps = mock_dependencies();
    let collection = Addr::unchecked(COLLECTION);
    let bidder = Addr::unchecked("bidder");

    let bid = Bid {
        collection: collection.clone(),
        token_id: TOKEN_ID,
        bidder: bidder.clone(),
        price: Uint128::from(500u128),
        expires: Timestamp::from_seconds(0),
    };
    let key = bid_key(collection.clone(), TOKEN_ID, bidder.clone());
    let res = bids().save(deps.as_mut().storage, key.clone(), &bid);
    assert!(res.is_ok());

    let bid2 = Bid {
        collection: collection.clone(),
        token_id: TOKEN_ID + 1,
        bidder: bidder.clone(),
        price: Uint128::from(500u128),
        expires: Timestamp::from_seconds(0),
    };
    let key2 = bid_key(collection, TOKEN_ID + 1, bidder.clone());
    let res = bids().save(deps.as_mut().storage, key2, &bid2);
    assert!(res.is_ok());

    let res = bids().load(deps.as_ref().storage, key);
    assert_eq!(res.unwrap(), bid);

    let res = query_bids_by_bidder(deps.as_ref(), bidder, None, None).unwrap();
    assert_eq!(res.bids.len(), 2);
    assert_eq!(res.bids[0], bid);
}

fn setup_contract(deps: DepsMut) {
    let msg = InstantiateMsg {
        operators: vec!["operator".to_string()],
        trading_fee_bps: TRADING_FEE_BASIS_POINTS,
        ask_expiry: ExpiryRange::new(MIN_EXPIRY, MAX_EXPIRY),
        bid_expiry: ExpiryRange::new(MIN_EXPIRY, MAX_EXPIRY),
        sale_hook: None,
        max_finders_fee_bps: MAX_FINDERS_FEE_BPS,
        min_price: Uint128::from(5u128),
        stale_bid_duration: Duration::Height(100),
        bid_removal_reward_bps: BID_REMOVAL_REWARD_BPS,
    };
    let info = mock_info(CREATOR, &[]);
    let res = instantiate(deps, mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
}

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies();

    let msg = InstantiateMsg {
        operators: vec!["operator".to_string()],
        trading_fee_bps: TRADING_FEE_BASIS_POINTS,
        ask_expiry: ExpiryRange::new(MIN_EXPIRY, MAX_EXPIRY),
        bid_expiry: ExpiryRange::new(MIN_EXPIRY, MAX_EXPIRY),
        sale_hook: None,
        max_finders_fee_bps: MAX_FINDERS_FEE_BPS,
        min_price: Uint128::from(5u128),
        stale_bid_duration: Duration::Height(100),
        bid_removal_reward_bps: BID_REMOVAL_REWARD_BPS,
    };
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
        expires: Timestamp::from_seconds(0),
        finder: None,
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
        expires: mock_env().block.time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };

    // Bidder calls SetBid before an Ask is set, still succeeds
    let res = execute(deps.as_mut(), mock_env(), bidder, set_bid_msg);
    assert!(res.is_ok());
}

#[test]
fn try_set_ask() {
    let mut deps = mock_dependencies();
    setup_contract(deps.as_mut());

    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: COLLECTION.to_string(),
        token_id: TOKEN_ID,
        price: coin(100, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: Timestamp::from_seconds(
            mock_env().block.time.plus_seconds(MIN_EXPIRY + 1).seconds(),
        ),
        finders_fee_bps: Some(0),
    };

    // Reject if not called by the media owner
    let not_allowed = mock_info("random", &[]);
    let err = execute(deps.as_mut(), mock_env(), not_allowed, set_ask);
    assert!(err.is_err());

    // Reject wrong denom
    let set_bad_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: COLLECTION.to_string(),
        token_id: TOKEN_ID,
        price: coin(100, "osmo".to_string()),
        funds_recipient: None,
        reserve_for: None,
        expires: Timestamp::from_seconds(
            mock_env().block.time.plus_seconds(MIN_EXPIRY + 1).seconds(),
        ),
        finders_fee_bps: Some(0),
    };
    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("creator", &[]),
        set_bad_ask,
    )
    .unwrap_err();
    assert_eq!(err, ContractError::InvalidPrice {});
}
