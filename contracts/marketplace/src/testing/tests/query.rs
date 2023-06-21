use cosmwasm_std::coin;
use cosmwasm_std::Timestamp;
use cw_multi_test::Executor;
use sg_std::GENESIS_MINT_START_TIME;
use sg_std::NATIVE_DENOM;
use test_suite::common_setup::setup_accounts_and_block::setup_block_time;

use crate::{
    msg::ExecuteMsg,
    testing::{
        helpers::{
            funds::{add_funds_for_incremental_fee, listing_funds},
            nft_functions::{approve, mint},
        },
        setup::{
            setup_accounts::CREATION_FEE,
            setup_marketplace::{setup_fair_burn, setup_marketplace, LISTING_FEE},
            templates::minter_two_collections,
        },
    },
};

#[test]
fn collections() {
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    let vt = minter_two_collections(1);
    let (mut router, owner, creator) = (vt.router, vt.accts.owner, vt.accts.creator);
    add_funds_for_incremental_fee(&mut router, &creator, CREATION_FEE, 1u128).unwrap();
    let fair_burn = setup_fair_burn(&mut router, &owner).unwrap();
    let marketplace = setup_marketplace(&mut router, fair_burn, owner).unwrap();
    let minter1 = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection1 = vt.collection_response_vec[0].collection.clone().unwrap();

    let minter2 = vt.collection_response_vec[1].minter.clone().unwrap();
    let collection2 = vt.collection_response_vec[1].collection.clone().unwrap();
    setup_block_time(&mut router, start_time.nanos(), None);

    let token_id = 1;
    // place two asks
    mint(&mut router, &creator, &minter1);
    mint(&mut router, &creator, &minter2);
    approve(&mut router, &creator, &collection1, &marketplace, token_id);
    approve(&mut router, &creator, &collection2, &marketplace, token_id);

    let set_ask = ExecuteMsg::SetAsk {
        collection: collection1.to_string(),
        token_id: token_id.to_string(),
        price: coin(110, NATIVE_DENOM),
        asset_recipient: None,
        reserve_for: None,
        finders_fee_bps: Some(0),
        expires: None,
    };
    let response = router.execute_contract(
        creator.clone(),
        marketplace.clone(),
        &set_ask,
        &listing_funds(LISTING_FEE).unwrap(),
    );
    assert!(response.is_ok());

    let set_ask = ExecuteMsg::SetAsk {
        collection: collection2.to_string(),
        token_id: token_id.to_string(),
        price: coin(110, NATIVE_DENOM),
        asset_recipient: None,
        reserve_for: None,
        finders_fee_bps: Some(0),
        expires: None,
    };
    let response = router.execute_contract(
        creator.clone(),
        marketplace.clone(),
        &set_ask,
        &listing_funds(LISTING_FEE).unwrap(),
    );
    assert!(response.is_ok());
}
