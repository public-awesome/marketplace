use crate::{
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    state::Auction,
};
use anyhow::Error;
use cosmwasm_std::{Addr, Coin};
use cw_multi_test::{AppResponse, Contract, ContractWrapper, Executor};
use sg_multi_test::StargazeApp;
use sg_std::StargazeMsgWrapper;

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
    reserve_price: Coin,
    duration: u64,
    seller_funds_recipient: Option<String>,
    funds: Coin,
) -> Result<AppResponse, Error> {
    let msg = ExecuteMsg::CreateAuction {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        reserve_price,
        duration,
        seller_funds_recipient,
    };
    router.execute_contract(creator.clone(), auction.clone(), &msg, &[funds])
}

pub fn place_bid(
    router: &mut StargazeApp,
    reserve_auction: &Addr,
    bidder: &Addr,
    collection: &str,
    token_id: &str,
    bid_coin: Coin,
) -> Result<AppResponse, Error> {
    let msg = ExecuteMsg::PlaceBid {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
    };
    router.execute_contract(bidder.clone(), reserve_auction.clone(), &msg, &[bid_coin])
}

pub fn query_auction(
    router: &StargazeApp,
    reserve_auction: &Addr,
    collection: &str,
    token_id: &str,
) -> Auction {
    let auction: Option<Auction> = router
        .wrap()
        .query_wasm_smart(
            reserve_auction,
            &QueryMsg::Auction {
                collection: collection.to_string(),
                token_id: token_id.to_string(),
            },
        )
        .unwrap();
    auction.unwrap()
}
