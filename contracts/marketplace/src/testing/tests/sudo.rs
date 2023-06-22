use cosmwasm_std::coin;
use cosmwasm_std::Addr;
use sg_marketplace_common::coin::bps_to_decimal;
use sg_std::GENESIS_MINT_START_TIME;
use sg_std::NATIVE_DENOM;
use test_suite::common_setup::setup_accounts_and_block::setup_block_time;

use crate::helpers::ExpiryRangeError;
use crate::{
    helpers::ExpiryRange,
    msg::{QueryMsg, SudoMsg},
    state::SudoParams,
    testing::setup::{
        setup_marketplace::{setup_fair_burn, setup_marketplace, LISTING_FEE},
        templates::standard_minter_template,
    },
};

#[test]
fn try_sudo_update_params() {
    let vt = standard_minter_template(1);
    let (mut router, owner) = (vt.router, vt.accts.owner);
    let fair_burn = setup_fair_burn(&mut router, &owner).unwrap();
    let marketplace = setup_marketplace(&mut router, owner, fair_burn).unwrap();
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);

    // Invalid expiry range (min > max) throws error
    let update_params_msg = SudoMsg::UpdateParams {
        fair_burn: None,
        listing_fee: Some(coin(LISTING_FEE, NATIVE_DENOM)),
        ask_expiry: Some(ExpiryRange::new(100, 2)),
        offer_expiry: None,
        operators: Some(vec!["operator1".to_string()]),
        max_asks_removed_per_block: None,
        max_offers_removed_per_block: None,
        max_collection_offers_removed_per_block: None,
        trading_fee_bps: Some(5),
        max_finders_fee_bps: None,
        removal_reward_bps: None,
    };
    let response = router.wasm_sudo(marketplace.clone(), &update_params_msg);
    assert_eq!(
        response.unwrap_err().to_string(),
        ExpiryRangeError::InvalidExpirationRange("range min > max".to_string()).to_string()
    );

    // Invalid operators list is deduped
    let update_params_msg = SudoMsg::UpdateParams {
        fair_burn: None,
        listing_fee: None,
        ask_expiry: None,
        offer_expiry: None,
        operators: Some(vec![
            "operator3".to_string(),
            "operator1".to_string(),
            "operator2".to_string(),
            "operator1".to_string(),
            "operator4".to_string(),
        ]),
        max_asks_removed_per_block: None,
        max_offers_removed_per_block: None,
        max_collection_offers_removed_per_block: None,
        trading_fee_bps: None,
        max_finders_fee_bps: None,
        removal_reward_bps: None,
    };
    let response = router.wasm_sudo(marketplace.clone(), &update_params_msg);
    assert!(response.is_ok());

    let query_params_msg = QueryMsg::SudoParams {};
    let sudo_params: SudoParams = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_params_msg)
        .unwrap();
    assert_eq!(
        sudo_params.operators,
        vec![
            Addr::unchecked("operator1".to_string()),
            Addr::unchecked("operator2".to_string()),
            Addr::unchecked("operator3".to_string()),
            Addr::unchecked("operator4".to_string())
        ]
    );

    // Validate sudo params can be updated
    let update_params_msg = SudoMsg::UpdateParams {
        fair_burn: Some("fair-burn".to_string()),
        listing_fee: Some(coin(LISTING_FEE + 1, NATIVE_DENOM)),
        ask_expiry: Some(ExpiryRange::new(1, 2)),
        offer_expiry: Some(ExpiryRange::new(3, 4)),
        operators: Some(vec!["operator1".to_string()]),
        max_asks_removed_per_block: Some(10),
        max_offers_removed_per_block: Some(20),
        max_collection_offers_removed_per_block: Some(30),
        trading_fee_bps: Some(40),
        max_finders_fee_bps: Some(50),
        removal_reward_bps: Some(60),
    };
    let response = router.wasm_sudo(marketplace.clone(), &update_params_msg);
    assert!(response.is_ok());

    let sudo_params: SudoParams = router
        .wrap()
        .query_wasm_smart(marketplace, &query_params_msg)
        .unwrap();
    assert_eq!(sudo_params.fair_burn, Addr::unchecked("fair-burn"));
    assert_eq!(sudo_params.listing_fee, coin(LISTING_FEE + 1, NATIVE_DENOM));
    assert_eq!(sudo_params.ask_expiry, ExpiryRange::new(1, 2));
    assert_eq!(sudo_params.offer_expiry, ExpiryRange::new(3, 4));
    assert_eq!(sudo_params.operators, vec!["operator1".to_string()]);
    assert_eq!(sudo_params.max_asks_removed_per_block, 10);
    assert_eq!(sudo_params.max_offers_removed_per_block, 20);
    assert_eq!(sudo_params.max_collection_offers_removed_per_block, 30);
    assert_eq!(sudo_params.trading_fee_percent, bps_to_decimal(40));
    assert_eq!(sudo_params.max_finders_fee_percent, bps_to_decimal(50));
    assert_eq!(sudo_params.removal_reward_percent, bps_to_decimal(60));
}
