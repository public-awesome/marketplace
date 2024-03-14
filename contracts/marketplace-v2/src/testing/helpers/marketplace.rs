use crate::{
    msg::{ExecuteMsg, OrderOptions},
    testing::helpers::nft_functions::{approve, mint_for},
};

use cosmwasm_std::{Addr, Coin};
use cw_multi_test::Executor;
use sg_multi_test::StargazeApp;

#[allow(clippy::too_many_arguments)]
pub fn mint_and_set_ask(
    router: &mut StargazeApp,
    creator: &Addr,
    owner: &Addr,
    minter: &Addr,
    marketplace: &Addr,
    collection: &Addr,
    token_id: &str,
    price: &Coin,
    send_funds: &[Coin],
    order_options: Option<OrderOptions<String>>,
) {
    mint_for(router, creator, owner, minter, token_id);
    approve(router, owner, collection, marketplace, token_id);

    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        price: price.clone(),
        order_options,
    };

    let response =
        router.execute_contract(owner.clone(), marketplace.clone(), &set_ask, send_funds);

    assert!(response.is_ok());
}
