use crate::msg::ExecuteMsg;
use crate::multitest::{
    approve, custom_mock_app, listing_funds, mint, setup_accounts, setup_contracts,
};
use crate::multitest::{LISTING_FEE, MIN_EXPIRY, TOKEN_ID};
use crate::state::SaleType;
use cosmwasm_std::{coin, coins};

use cw_multi_test::Executor;
use sg_std::NATIVE_DENOM;

#[test]
fn try_set_bid_fixed_price() {
    let mut router = custom_mock_app();
    // Setup intial accounts
    let (_owner, bidder, creator) = setup_accounts(&mut router).unwrap();
    // Instantiate and configure contracts
    let (marketplace, collection) = setup_contracts(&mut router, &creator).unwrap();

    // Mint NFT for creator
    mint(&mut router, &creator, &collection, TOKEN_ID);
    approve(&mut router, &creator, &collection, &marketplace, TOKEN_ID);

    // An asking price is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        price: coin(150, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finders_fee_bps: Some(0),
    };
    // router.execute_contract(sender, contract_addr, msg, send_funds)
    let res = router.execute_contract(
        creator.clone(),
        marketplace.clone(),
        &set_ask,
        &listing_funds(LISTING_FEE).unwrap(),
    );
    assert!(res.is_ok());

    // Bidder makes bid
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        finders_fee_bps: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };

    // Bidder makes bid higher than the asking price
    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_bid_msg,
        &coins(200, NATIVE_DENOM),
    );
    assert!(res.is_err());

    // Bidder makes bid higher lower the asking price
    let res = router.execute_contract(
        bidder,
        marketplace.clone(),
        &set_bid_msg,
        &coins(50, NATIVE_DENOM),
    );
    assert!(res.is_err());
}
