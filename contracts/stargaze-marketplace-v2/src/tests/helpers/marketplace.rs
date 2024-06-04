use crate::{msg::ExecuteMsg, orders::OrderDetails};

use cosmwasm_std::{Addr, Coin, Empty};
use cw721_base::msg::ExecuteMsg as Cw721ExecuteMsg;
use cw_multi_test::{App, Executor};

pub fn mint(app: &mut App, creator: &Addr, owner: &Addr, collection: &Addr, token_id: &str) {
    let mint_msg = Cw721ExecuteMsg::<Empty, Empty>::Mint {
        token_id: token_id.to_string(),
        owner: owner.to_string(),
        token_uri: None,
        extension: Empty {},
    };
    let response = app.execute_contract(creator.clone(), collection.clone(), &mint_msg, &[]);
    assert!(response.is_ok());
}

pub fn approve(app: &mut App, owner: &Addr, collection: &Addr, spender: &Addr, token_id: &str) {
    let approve_msg = Cw721ExecuteMsg::<Empty, Empty>::Approve {
        spender: spender.to_string(),
        token_id: token_id.to_string(),
        expires: None,
    };
    let response = app.execute_contract(owner.clone(), collection.clone(), &approve_msg, &[]);
    assert!(response.is_ok());
}

#[allow(clippy::too_many_arguments)]
pub fn mint_and_set_ask(
    app: &mut App,
    creator: &Addr,
    owner: &Addr,
    marketplace: &Addr,
    collection: &Addr,
    token_id: &str,
    send_funds: &[Coin],
    details: OrderDetails<String>,
) {
    mint(app, creator, owner, collection, token_id);
    approve(app, owner, collection, marketplace, token_id);

    let set_ask = ExecuteMsg::SetAsk {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
        details,
    };

    let response = app.execute_contract(owner.clone(), marketplace.clone(), &set_ask, send_funds);

    assert!(response.is_ok());
}
