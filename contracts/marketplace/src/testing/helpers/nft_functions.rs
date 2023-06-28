use std::collections::HashSet;
use std::iter::FromIterator;

use cosmwasm_std::coins;
use cosmwasm_std::{Addr, Empty};
use cw721::TokensResponse;
use cw_multi_test::Executor;
use sg721::ExecuteMsg as Sg721ExecuteMsg;
use sg721_base::msg::CollectionInfoResponse;
use sg_multi_test::StargazeApp;
use sg_std::NATIVE_DENOM;

pub const MINT_PRICE: u128 = 100_000_000;

// Mints an NFT for a creator
pub fn mint(router: &mut StargazeApp, creator: &Addr, minter_addr: &Addr) {
    let minter_msg = vending_minter::msg::ExecuteMsg::Mint {};
    let res = router.execute_contract(
        creator.clone(),
        minter_addr.clone(),
        &minter_msg,
        &coins(MINT_PRICE, NATIVE_DENOM),
    );
    assert!(res.is_ok());
}

pub fn mint_for(
    router: &mut StargazeApp,
    owner: &Addr,
    creator: &Addr,
    collection: &Addr,
    token_id: u32,
) {
    let mint_for_creator_msg = vending_minter::msg::ExecuteMsg::MintFor {
        token_id,
        recipient: creator.to_string(),
    };
    let res = router.execute_contract(
        owner.clone(),
        collection.clone(),
        &mint_for_creator_msg,
        &[],
    );
    println!("res is {:?}", res);
    assert!(res.is_ok());
}

pub fn approve(
    router: &mut StargazeApp,
    creator: &Addr,
    collection: &Addr,
    marketplace: &Addr,
    token_id: u32,
) {
    let approve_msg: Sg721ExecuteMsg<CollectionInfoResponse, Empty> = Sg721ExecuteMsg::Approve {
        spender: marketplace.to_string(),
        token_id: token_id.to_string(),
        expires: None,
    };
    let res = router.execute_contract(creator.clone(), collection.clone(), &approve_msg, &[]);
    assert!(res.is_ok());
}

pub fn revoke_approval(
    router: &mut StargazeApp,
    creator: &Addr,
    collection: &Addr,
    marketplace: &Addr,
    token_id: u32,
) {
    let revoke_approval: Sg721ExecuteMsg<CollectionInfoResponse, Empty> = Sg721ExecuteMsg::Revoke {
        spender: marketplace.to_string(),
        token_id: token_id.to_string(),
    };
    let res = router.execute_contract(creator.clone(), collection.clone(), &revoke_approval, &[]);
    assert!(res.is_ok());
}

pub fn transfer(
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

pub fn burn(router: &mut StargazeApp, creator: &Addr, collection: &Addr, token_id: u32) {
    let transfer_msg: Sg721ExecuteMsg<Empty, Empty> = Sg721ExecuteMsg::Burn {
        token_id: token_id.to_string(),
    };
    let res = router.execute_contract(creator.clone(), collection.clone(), &transfer_msg, &[]);
    assert!(res.is_ok());
}

pub fn get_next_token_id_and_map(
    router: &mut StargazeApp,
    incoming_hash: &HashSet<String>,
    collection: Addr,
) -> (HashSet<std::string::String>, u32) {
    let query_msg = sg721_base::msg::QueryMsg::AllTokens {
        start_after: None,
        limit: None,
    };
    let res: TokensResponse = router
        .wrap()
        .query_wasm_smart(collection, &query_msg)
        .unwrap();
    let tokens_hash: HashSet<String> = HashSet::from_iter(res.tokens.iter().cloned());
    let difference = tokens_hash.difference(incoming_hash);
    let nft_hash = tokens_hash.clone();
    let token_id: Option<&String> = difference.into_iter().next();
    let token_id_unwrapped = token_id.unwrap().parse::<u32>().unwrap();
    (nft_hash, token_id_unwrapped)
}
