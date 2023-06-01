use crate::tests::helpers::constants::{
    CREATE_AUCTION_FEE, EXTEND_DURATION, MAX_AUCTIONS_TO_SETTLE_PER_BLOCK, MAX_DURATION,
    MIN_BID_INCREMENT_BPS, MIN_DURATION, MIN_RESERVE_PRICE,
};
use crate::{msg::InstantiateMsg, ContractError};

use cosmwasm_std::{coin, Addr};
use cw_multi_test::Executor;
use sg_multi_test::StargazeApp;
use sg_std::NATIVE_DENOM;

use super::setup_contracts::*;

pub const DUMMY_DENOM: &str =
    "ibc/773B5B5E24EC48005205A2EB35E6C0743EE47C9147E94BD5A4E0CBB63082314D";

pub fn setup_reserve_auction(
    router: &mut StargazeApp,
    auction_admin: Addr,
    fair_burn: Addr,
    marketplace: Addr,
) -> Result<Addr, ContractError> {
    let reserve_auction_id = router.store_code(contract_reserve_auction());
    let msg = InstantiateMsg {
        fair_burn: fair_burn.to_string(),
        marketplace: marketplace.to_string(),
        min_duration: MIN_DURATION,
        max_duration: MAX_DURATION,
        min_bid_increment_bps: MIN_BID_INCREMENT_BPS,
        extend_duration: EXTEND_DURATION,
        create_auction_fee: coin(CREATE_AUCTION_FEE.u128(), NATIVE_DENOM),
        max_auctions_to_settle_per_block: MAX_AUCTIONS_TO_SETTLE_PER_BLOCK,
        min_reserve_prices: vec![
            coin(MIN_RESERVE_PRICE, NATIVE_DENOM),
            coin(MIN_RESERVE_PRICE, DUMMY_DENOM),
        ],
    };
    let auction = router
        .instantiate_contract(
            reserve_auction_id,
            auction_admin,
            &msg,
            &[],
            "Reserve-Auction",
            None,
        )
        .unwrap();
    Ok(auction)
}
