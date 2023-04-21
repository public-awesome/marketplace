use cosmwasm_std::{Addr, Empty};
use cw721::OwnerOfResponse;
use cw_multi_test::Executor;
use sg721::ExecuteMsg as Sg721ExecuteMsg;
use sg721_base::msg::CollectionInfoResponse;
use sg_multi_test::StargazeApp;

pub const _MINT_PRICE: u128 = 100_000_000;

pub fn mint(router: &mut StargazeApp, minter_addr: &Addr, creator: &Addr, recipient: &Addr) -> u32 {
    let minter_msg = vending_minter::msg::ExecuteMsg::MintTo {
        recipient: recipient.to_string(),
    };
    let res = router.execute_contract(creator.clone(), minter_addr.clone(), &minter_msg, &[]);
    assert!(res.is_ok());

    let mint_event = res
        .unwrap()
        .events
        .iter()
        .find(|&e| e.ty == "wasm")
        .unwrap()
        .clone();

    let token_id = mint_event
        .attributes
        .iter()
        .find(|attr| attr.key == "token_id")
        .unwrap()
        .value
        .parse::<u32>()
        .unwrap();

    token_id
}

pub fn approve(
    router: &mut StargazeApp,
    creator: &Addr,
    collection: &Addr,
    auction: &Addr,
    token_id: u32,
) {
    let approve_msg: Sg721ExecuteMsg<CollectionInfoResponse, Empty> = Sg721ExecuteMsg::Approve {
        spender: auction.to_string(),
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

pub fn query_owner_of(router: &StargazeApp, collection: &Addr, token_id: &str) -> String {
    let res: OwnerOfResponse = router
        .wrap()
        .query_wasm_smart(
            collection,
            &sg721_base::msg::QueryMsg::OwnerOf {
                token_id: token_id.to_string(),
                include_expired: None,
            },
        )
        .unwrap();
    res.owner
}
