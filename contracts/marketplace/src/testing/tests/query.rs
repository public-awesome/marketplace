use crate::msg::CollectionsResponse;
use crate::msg::{ExecuteMsg, QueryMsg};
use crate::state_deprecated::SaleType;
use crate::testing::helpers::funds::{add_funds_for_incremental_fee, listing_funds};
use crate::testing::helpers::nft_functions::{approve, mint};
use crate::testing::setup::setup_accounts::CREATION_FEE;
use crate::testing::setup::setup_marketplace::{setup_marketplace, LISTING_FEE, MIN_EXPIRY};
use cosmwasm_std::Timestamp;
use cw_multi_test::Executor;
use sg_std::GENESIS_MINT_START_TIME;

use cosmwasm_std::coin;
use test_suite::common_setup::setup_accounts_and_block::setup_block_time;

use crate::testing::setup::templates::minter_two_collections;
use sg_std::NATIVE_DENOM;

#[test]
fn collections() {
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    let vt = minter_two_collections(1);
    let (mut router, owner, creator) = (vt.router, vt.accts.owner, vt.accts.creator);
    add_funds_for_incremental_fee(&mut router, &creator, CREATION_FEE, 1u128).unwrap();
    let marketplace = setup_marketplace(&mut router, owner).unwrap();
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
        sale_type: SaleType::FixedPrice,
        collection: collection1.to_string(),
        token_id,
        price: coin(110, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 1),
        finders_fee_bps: Some(0),
    };
    let res = router.execute_contract(
        creator.clone(),
        marketplace.clone(),
        &set_ask,
        &listing_funds(LISTING_FEE).unwrap(),
    );
    assert!(res.is_ok());

    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection2.to_string(),
        token_id,
        price: coin(110, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 1),
        finders_fee_bps: Some(0),
    };
    let res = router.execute_contract(
        creator.clone(),
        marketplace.clone(),
        &set_ask,
        &listing_funds(LISTING_FEE).unwrap(),
    );
    assert!(res.is_ok());

    // check collections query
    let res: CollectionsResponse = router
        .wrap()
        .query_wasm_smart(
            marketplace,
            &QueryMsg::Collections {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(res.collections[0], collection1);
    assert_eq!(res.collections[1], collection2);
}
