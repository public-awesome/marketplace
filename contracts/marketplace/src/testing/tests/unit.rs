use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{coin, coins, Addr, DepsMut, Timestamp, Uint128};
use sg_marketplace_common::query::QueryOptions;
use sg_std::NATIVE_DENOM;
use std::vec;

use crate::state::{offers, Offer, PriceRange};
use crate::testing::setup::setup_marketplace::{
    MAX_ENTRY_REMOVAL_PER_BLOCK, MAX_FIXED_PRICE_ASK_AMOUNT,
};
use crate::{
    execute::execute,
    helpers::ExpiryRange,
    instantiate::instantiate,
    msg::{ExecuteMsg, InstantiateMsg},
    query::{query_asks_by_seller, query_offers_by_bidder},
    state::{asks, Ask},
    testing::setup::setup_marketplace::{
        MAX_EXPIRY, MAX_FINDERS_FEE_BPS, MIN_EXPIRY, REMOVAL_REWARD_BPS, TRADING_FEE_BPS,
    },
    ContractError,
};

const CREATOR: &str = "creator";
const COLLECTION: &str = "collection";
const TOKEN_ID: u32 = 123;

#[test]
fn ask_indexed_map() {
    let mut deps = mock_dependencies();
    let collection = Addr::unchecked(COLLECTION);
    let seller = Addr::unchecked("seller");

    let ask = Ask {
        collection: collection.clone(),
        token_id: TOKEN_ID.to_string(),
        seller: seller.clone(),
        price: coin(500u128, NATIVE_DENOM),
        asset_recipient: None,
        reserve_for: None,
        finders_fee_percent: None,
        expires: Some(Timestamp::from_seconds(0)),
        paid_removal_fee: None,
    };
    let key = Ask::build_key(&collection, &TOKEN_ID.to_string());
    let response = asks().save(deps.as_mut().storage, key.clone(), &ask);
    assert!(response.is_ok());

    let ask2 = Ask {
        collection: collection.clone(),
        token_id: (TOKEN_ID + 1).to_string(),
        seller: seller.clone(),
        price: coin(500u128, NATIVE_DENOM),
        asset_recipient: None,
        reserve_for: None,
        expires: Some(Timestamp::from_seconds(0)),
        finders_fee_percent: None,
        paid_removal_fee: None,
    };
    let key2 = Ask::build_key(&collection, &(TOKEN_ID + 1).to_string());
    let response = asks().save(deps.as_mut().storage, key2, &ask2);
    assert!(response.is_ok());

    let response = asks().load(deps.as_ref().storage, key);
    assert_eq!(response.unwrap(), ask);

    let asks = query_asks_by_seller(deps.as_ref(), seller, QueryOptions::default()).unwrap();
    assert_eq!(asks.len(), 2);
    assert_eq!(asks[0], ask);
}

#[test]
fn offer_indexed_map() {
    let mut deps = mock_dependencies();
    let collection = Addr::unchecked(COLLECTION);
    let bidder = Addr::unchecked("bidder");

    let offer = Offer {
        collection: collection.clone(),
        token_id: TOKEN_ID.to_string(),
        bidder: bidder.clone(),
        price: coin(500u128, NATIVE_DENOM),
        asset_recipient: None,
        finders_fee_percent: None,
        expires: Some(Timestamp::from_seconds(0)),
    };
    let key = Offer::build_key(&collection, &TOKEN_ID.to_string(), &bidder);
    let response = offers().save(deps.as_mut().storage, key.clone(), &offer);
    assert!(response.is_ok());

    let offer2 = Offer {
        collection: collection.clone(),
        token_id: TOKEN_ID.to_string(),
        bidder: bidder.clone(),
        price: coin(500u128, NATIVE_DENOM),
        asset_recipient: None,
        finders_fee_percent: None,
        expires: Some(Timestamp::from_seconds(0)),
    };
    let key2 = Offer::build_key(&collection, &(TOKEN_ID + 1).to_string(), &bidder);
    let response = offers().save(deps.as_mut().storage, key2, &offer2);
    assert!(response.is_ok());

    let response = offers().load(deps.as_ref().storage, key);
    assert_eq!(response.unwrap(), offer);

    let offers = query_offers_by_bidder(deps.as_ref(), bidder, QueryOptions::default()).unwrap();
    assert_eq!(offers.len(), 2);
    assert_eq!(offers[0], offer);
}

fn setup_contract(deps: DepsMut) {
    let msg = InstantiateMsg {
        fair_burn: "fair_burn".to_string(),
        listing_fee: coin(5u128, NATIVE_DENOM),
        ask_expiry: ExpiryRange::new(MIN_EXPIRY, MAX_EXPIRY),
        offer_expiry: ExpiryRange::new(MIN_EXPIRY, MAX_EXPIRY),
        operators: vec!["operator".to_string()],
        max_asks_removed_per_block: MAX_ENTRY_REMOVAL_PER_BLOCK,
        max_offers_removed_per_block: MAX_ENTRY_REMOVAL_PER_BLOCK,
        max_collection_offers_removed_per_block: MAX_ENTRY_REMOVAL_PER_BLOCK,
        trading_fee_bps: TRADING_FEE_BPS,
        max_finders_fee_bps: MAX_FINDERS_FEE_BPS,
        removal_reward_bps: REMOVAL_REWARD_BPS,
        price_ranges: vec![(
            NATIVE_DENOM.to_string(),
            PriceRange {
                min: Uint128::from(5u128),
                max: Uint128::from(MAX_FIXED_PRICE_ASK_AMOUNT),
            },
        )],
        sale_hook: None,
    };
    let info = mock_info(CREATOR, &[]);
    let res = instantiate(deps, mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
}

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies();

    let msg = InstantiateMsg {
        fair_burn: "fair_burn".to_string(),
        listing_fee: coin(5u128, NATIVE_DENOM),
        ask_expiry: ExpiryRange::new(MIN_EXPIRY, MAX_EXPIRY),
        offer_expiry: ExpiryRange::new(MIN_EXPIRY, MAX_EXPIRY),
        operators: vec!["operator".to_string()],
        max_asks_removed_per_block: MAX_ENTRY_REMOVAL_PER_BLOCK,
        max_offers_removed_per_block: MAX_ENTRY_REMOVAL_PER_BLOCK,
        max_collection_offers_removed_per_block: MAX_ENTRY_REMOVAL_PER_BLOCK,
        trading_fee_bps: TRADING_FEE_BPS,
        max_finders_fee_bps: MAX_FINDERS_FEE_BPS,
        removal_reward_bps: REMOVAL_REWARD_BPS,
        price_ranges: vec![(
            NATIVE_DENOM.to_string(),
            PriceRange {
                min: Uint128::from(5u128),
                max: Uint128::from(MAX_FIXED_PRICE_ASK_AMOUNT),
            },
        )],
        sale_hook: None,
    };
    let info = mock_info("creator", &coins(1000, NATIVE_DENOM));

    // we can just call .unwrap() to assert this was a success
    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
}

#[test]
fn bad_fees_initialization() {
    let mut deps = mock_dependencies();

    // throw error if trading fee bps > 100%
    let msg = InstantiateMsg {
        fair_burn: "fair_burn".to_string(),
        listing_fee: coin(5u128, NATIVE_DENOM),
        ask_expiry: ExpiryRange::new(MIN_EXPIRY, MAX_EXPIRY),
        offer_expiry: ExpiryRange::new(MIN_EXPIRY, MAX_EXPIRY),
        operators: vec!["operator".to_string()],
        max_asks_removed_per_block: MAX_ENTRY_REMOVAL_PER_BLOCK,
        max_offers_removed_per_block: MAX_ENTRY_REMOVAL_PER_BLOCK,
        max_collection_offers_removed_per_block: MAX_ENTRY_REMOVAL_PER_BLOCK,
        trading_fee_bps: 10001,
        max_finders_fee_bps: MAX_FINDERS_FEE_BPS,
        removal_reward_bps: REMOVAL_REWARD_BPS,
        price_ranges: vec![(
            NATIVE_DENOM.to_string(),
            PriceRange {
                min: Uint128::from(5u128),
                max: Uint128::from(MAX_FIXED_PRICE_ASK_AMOUNT),
            },
        )],
        sale_hook: None,
    };
    let info = mock_info("creator", &coins(1000, NATIVE_DENOM));
    let res = instantiate(deps.as_mut(), mock_env(), info, msg);
    assert!(res.is_err());

    // throw error if bid removal reward bps > 100%
    let msg = InstantiateMsg {
        fair_burn: "fair_burn".to_string(),
        listing_fee: coin(5u128, NATIVE_DENOM),
        ask_expiry: ExpiryRange::new(MIN_EXPIRY, MAX_EXPIRY),
        offer_expiry: ExpiryRange::new(MIN_EXPIRY, MAX_EXPIRY),
        operators: vec!["operator".to_string()],
        max_asks_removed_per_block: MAX_ENTRY_REMOVAL_PER_BLOCK,
        max_offers_removed_per_block: MAX_ENTRY_REMOVAL_PER_BLOCK,
        max_collection_offers_removed_per_block: MAX_ENTRY_REMOVAL_PER_BLOCK,
        trading_fee_bps: TRADING_FEE_BPS,
        max_finders_fee_bps: MAX_FINDERS_FEE_BPS,
        removal_reward_bps: 10001,
        price_ranges: vec![(
            NATIVE_DENOM.to_string(),
            PriceRange {
                min: Uint128::from(5u128),
                max: Uint128::from(MAX_FIXED_PRICE_ASK_AMOUNT),
            },
        )],
        sale_hook: None,
    };
    let info = mock_info("creator", &coins(1000, NATIVE_DENOM));
    let res = instantiate(deps.as_mut(), mock_env(), info, msg);
    assert!(res.is_err());

    // throw error if finders fee bps > 100%
    let msg = InstantiateMsg {
        fair_burn: "fair_burn".to_string(),
        listing_fee: coin(5u128, NATIVE_DENOM),
        ask_expiry: ExpiryRange::new(MIN_EXPIRY, MAX_EXPIRY),
        offer_expiry: ExpiryRange::new(MIN_EXPIRY, MAX_EXPIRY),
        operators: vec!["operator".to_string()],
        max_asks_removed_per_block: MAX_ENTRY_REMOVAL_PER_BLOCK,
        max_offers_removed_per_block: MAX_ENTRY_REMOVAL_PER_BLOCK,
        max_collection_offers_removed_per_block: MAX_ENTRY_REMOVAL_PER_BLOCK,
        trading_fee_bps: TRADING_FEE_BPS,
        max_finders_fee_bps: 10001,
        removal_reward_bps: REMOVAL_REWARD_BPS,
        price_ranges: vec![(
            NATIVE_DENOM.to_string(),
            PriceRange {
                min: Uint128::from(5u128),
                max: Uint128::from(MAX_FIXED_PRICE_ASK_AMOUNT),
            },
        )],
        sale_hook: None,
    };
    let info = mock_info("creator", &coins(1000, NATIVE_DENOM));
    let res = instantiate(deps.as_mut(), mock_env(), info, msg);
    assert!(res.is_err());
}

#[test]
fn try_set_bid() {
    let mut deps = mock_dependencies();
    setup_contract(deps.as_mut());

    let broke = mock_info("broke", &[]);
    let bidder = mock_info("bidder", &coins(1000, NATIVE_DENOM));

    let set_offer_msg = ExecuteMsg::SetOffer {
        collection: COLLECTION.to_string(),
        token_id: TOKEN_ID.to_string(),
        asset_recipient: None,
        finders_fee_bps: None,
        finder: None,
        expires: Some(Timestamp::from_seconds(0)),
    };

    // Broke bidder calls Set Bid and gets an error
    let err = execute(deps.as_mut(), mock_env(), broke, set_offer_msg).unwrap_err();
    assert_eq!(
        err,
        ContractError::BidPaymentError(cw_utils::PaymentError::NoFunds {})
    );

    let set_offer_msg = ExecuteMsg::SetOffer {
        collection: COLLECTION.to_string(),
        token_id: TOKEN_ID.to_string(),
        asset_recipient: None,
        finders_fee_bps: None,
        finder: None,
        expires: Some(mock_env().block.time.plus_seconds(MIN_EXPIRY + 1)),
    };

    // Bidder calls SetBid before an Ask is set, still succeeds
    let response = execute(deps.as_mut(), mock_env(), bidder, set_offer_msg);
    assert!(response.is_ok());
}

#[test]
fn try_set_ask() {
    let mut deps = mock_dependencies();
    setup_contract(deps.as_mut());

    let set_ask = ExecuteMsg::SetAsk {
        collection: COLLECTION.to_string(),
        token_id: TOKEN_ID.to_string(),
        price: coin(100, NATIVE_DENOM),
        asset_recipient: None,
        reserve_for: None,
        finders_fee_bps: Some(0),
        expires: None,
    };

    // Reject if not called by the media owner
    let not_allowed = mock_info("random", &[]);
    let err = execute(deps.as_mut(), mock_env(), not_allowed, set_ask);
    assert!(err.is_err());

    // Reject unsupported denom
    let set_bad_ask = ExecuteMsg::SetAsk {
        collection: COLLECTION.to_string(),
        token_id: TOKEN_ID.to_string(),
        price: coin(100, "osmo".to_string()),
        asset_recipient: None,
        reserve_for: None,
        finders_fee_bps: Some(0),
        expires: None,
    };
    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("creator", &[]),
        set_bad_ask,
    )
    .unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidInput("invalid denom".to_string())
    );
}
