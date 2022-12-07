use crate::error::ContractError;
use crate::testing::setup::constants::{
    BID_REMOVAL_REWARD_BPS, LISTING_FEE, MAX_EXPIRY, MAX_FINDERS_FEE_BPS, MIN_EXPIRY,
    TRADING_FEE_BPS,
};
use crate::testing::setup::msg::{MinterCollectionResponse, SetupContractsParams};
use crate::testing::setup::setup_contracts::contract_marketplace;
use crate::testing::setup::setup_minter::configure_minter;
use crate::ExpiryRange;
use cosmwasm_std::{Addr, Uint128};
use cw_multi_test::Executor;
use cw_utils::Duration;

// Instantiates all needed contracts for testing
pub fn setup_marketplace_and_collections(
    params: SetupContractsParams,
) -> Result<(Addr, Vec<MinterCollectionResponse>), ContractError> {
    let router = params.router;
    let collection_params_vec = params.collection_params_vec;
    let num_tokens = params.num_tokens;
    let minter_admin = params.minter_admin;

    let minter_collections: Vec<MinterCollectionResponse> = configure_minter(
        router,
        minter_admin.clone(),
        collection_params_vec,
        num_tokens,
    );
    // Instantiate marketplace contract
    let marketplace_id = router.store_code(contract_marketplace());
    let msg = crate::msg::InstantiateMsg {
        operators: vec!["operator1".to_string()],
        trading_fee_bps: TRADING_FEE_BPS,
        ask_expiry: ExpiryRange::new(MIN_EXPIRY, MAX_EXPIRY),
        bid_expiry: ExpiryRange::new(MIN_EXPIRY, MAX_EXPIRY),
        sale_hook: None,
        max_finders_fee_bps: MAX_FINDERS_FEE_BPS,
        min_price: Uint128::from(5u128),
        stale_bid_duration: Duration::Time(100),
        bid_removal_reward_bps: BID_REMOVAL_REWARD_BPS,
        listing_fee: Uint128::from(LISTING_FEE),
    };
    let marketplace = router
        .instantiate_contract(marketplace_id, minter_admin, &msg, &[], "Marketplace", None)
        .unwrap();
    Ok((marketplace, minter_collections))
}

pub fn setup_marketplace_and_collections_with_params(
    params: SetupContractsParams,
    instantiate_msg: crate::msg::InstantiateMsg,
) -> Result<(Addr, Vec<MinterCollectionResponse>), ContractError> {
    let router = params.router;
    let collection_params_vec = params.collection_params_vec;
    let num_tokens = params.num_tokens;
    let minter_admin = params.minter_admin;
    let minter_collections: Vec<MinterCollectionResponse> = configure_minter(
        router,
        minter_admin.clone(),
        collection_params_vec,
        num_tokens,
    );
    // Instantiate marketplace contract
    let marketplace_id = router.store_code(contract_marketplace());
    let marketplace = router
        .instantiate_contract(
            marketplace_id,
            minter_admin,
            &instantiate_msg,
            &[],
            "Marketplace",
            None,
        )
        .unwrap();
    Ok((marketplace, minter_collections))
}
