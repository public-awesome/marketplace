use crate::{
    msg::{AuctionResponse, ExecuteMsg, InstantiateMsg, QueryMsg},
    state::Auction,
};
use anyhow::Error;
use cosmwasm_std::{coin, Addr};
use cw_multi_test::{AppResponse, Contract, ContractWrapper, Executor};
use sg_multi_test::StargazeApp;
use sg_std::{StargazeMsgWrapper, NATIVE_DENOM};

pub fn auction_contract() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        crate::execute::execute,
        crate::instantiate::instantiate,
        crate::query::query,
    )
    .with_sudo(crate::sudo::sudo);
    Box::new(contract)
}

const ADMIN: &str = "ADMIN";

pub fn instantiate_auction(app: &mut StargazeApp, code_id: u64, msg: InstantiateMsg) -> Addr {
    app.instantiate_contract(code_id, Addr::unchecked(ADMIN), &msg, &[], "auction", None)
        .unwrap()
}

#[allow(clippy::too_many_arguments)]
pub fn create_standard_auction(
    router: &mut StargazeApp,
    creator: &Addr,
    auction: &Addr,
    collection: &str,
    token_id: &str,
    reserve_price: u128,
    duration: u64,
    seller_funds_recipient: Option<String>,
    creation_fee: u128,
) -> Result<AppResponse, Error> {
    let msg = ExecuteMsg::CreateAuction {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        reserve_price: coin(reserve_price, NATIVE_DENOM),
        duration,
        seller_funds_recipient,
    };
    router.execute_contract(
        creator.clone(),
        auction.clone(),
        &msg,
        &[coin(creation_fee, NATIVE_DENOM)],
    )
}

pub fn place_bid(
    router: &mut StargazeApp,
    reserve_auction: &Addr,
    bidder: &Addr,
    collection: &str,
    token_id: &str,
    bid_amount: u128,
) -> Result<AppResponse, Error> {
    let msg = ExecuteMsg::PlaceBid {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
    };
    router.execute_contract(
        bidder.clone(),
        reserve_auction.clone(),
        &msg,
        &[coin(bid_amount, NATIVE_DENOM)],
    )
}

pub fn query_auction(
    router: &StargazeApp,
    reserve_auction: &Addr,
    collection: &str,
    token_id: &str,
) -> Auction {
    let res: AuctionResponse = router
        .wrap()
        .query_wasm_smart(
            reserve_auction,
            &QueryMsg::Auction {
                collection: collection.to_string(),
                token_id: token_id.to_string(),
            },
        )
        .unwrap();
    res.auction
}
