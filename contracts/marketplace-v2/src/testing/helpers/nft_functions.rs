use cosmwasm_std::coins;
use cosmwasm_std::{Addr, Empty};
use cw_multi_test::{AppResponse, Executor};
use sg721::ExecuteMsg as Sg721ExecuteMsg;
use sg721_base::msg::CollectionInfoResponse;
use sg_multi_test::StargazeApp;
use sg_std::NATIVE_DENOM;

pub const _MINT_PRICE: u128 = 100_000_000;

// Mints an NFT for a creator
pub fn _mint(router: &mut StargazeApp, creator: &Addr, minter_addr: &Addr) -> String {
    let minter_msg = vending_minter::msg::ExecuteMsg::Mint {};
    let response = router
        .execute_contract(
            creator.clone(),
            minter_addr.clone(),
            &minter_msg,
            &coins(_MINT_PRICE, NATIVE_DENOM),
        )
        .unwrap();

    for event in response.events {
        for attr in event.attributes {
            if attr.key == "token_id" {
                return attr.value;
            }
        }
    }

    panic!("No token_id found in events")
}

pub fn mint_for(
    router: &mut StargazeApp,
    creator: &Addr,
    owner: &Addr,
    minter_addr: &Addr,
    token_id: &str,
) -> AppResponse {
    let mint_for_creator_msg = vending_minter::msg::ExecuteMsg::MintFor {
        token_id: token_id.parse().unwrap(),
        recipient: owner.to_string(),
    };
    let response = router.execute_contract(
        creator.clone(),
        minter_addr.clone(),
        &mint_for_creator_msg,
        &[],
    );
    response.unwrap()
}

pub fn approve(
    router: &mut StargazeApp,
    creator: &Addr,
    collection: &Addr,
    marketplace: &Addr,
    token_id: &str,
) {
    let approve_msg: Sg721ExecuteMsg<CollectionInfoResponse, Empty> = Sg721ExecuteMsg::Approve {
        spender: marketplace.to_string(),
        token_id: token_id.to_string(),
        expires: None,
    };
    let res = router.execute_contract(creator.clone(), collection.clone(), &approve_msg, &[]);
    assert!(res.is_ok());
}

pub fn _transfer(
    router: &mut StargazeApp,
    creator: &Addr,
    recipient: &Addr,
    collection: &Addr,
    token_id: u32,
) {
    let transfer_msg: Sg721ExecuteMsg<Empty, Empty> = Sg721ExecuteMsg::TransferNft {
        recipient: recipient.to_string(),
        token_id: token_id.to_string(),
    };
    let res = router.execute_contract(creator.clone(), collection.clone(), &transfer_msg, &[]);
    assert!(res.is_ok());
}
