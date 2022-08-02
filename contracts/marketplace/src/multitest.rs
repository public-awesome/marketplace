#![cfg(test)]
use crate::error::ContractError;
use crate::helpers::ExpiryRange;
use crate::msg::{
    AskCountResponse, AskOffset, AskResponse, AsksResponse, BidOffset, BidResponse,
    CollectionBidOffset, CollectionOffset, CollectionsResponse, ParamsResponse, SudoMsg,
};
use crate::msg::{
    BidsResponse, CollectionBidResponse, CollectionBidsResponse, ExecuteMsg, QueryMsg,
};
use crate::state::{Bid, SaleType};
use cosmwasm_std::{Addr, Empty, Timestamp};
use cw721::{Cw721QueryMsg, OwnerOfResponse};
use cw721_base::msg::{ExecuteMsg as Cw721ExecuteMsg, MintMsg};
use cw_multi_test::{BankSudo, Contract, ContractWrapper, Executor, SudoMsg as CwSudoMsg};
use sg_controllers::HooksResponse;
use sg_multi_test::StargazeApp;
use sg_std::StargazeMsgWrapper;

use cosmwasm_std::{coin, coins, Coin, Decimal, Uint128};
use cw_utils::{Duration, Expiration};
use sg721::msg::{InstantiateMsg as Sg721InstantiateMsg, RoyaltyInfoResponse};
use sg721::state::CollectionInfo;
use sg_std::NATIVE_DENOM;

const TOKEN_ID: u32 = 123;
const CREATION_FEE: u128 = 1_000_000_000;
const INITIAL_BALANCE: u128 = 2000;

// Governance parameters
const TRADING_FEE_BPS: u64 = 200; // 2%
const MIN_EXPIRY: u64 = 24 * 60 * 60; // 24 hours (in seconds)
const MAX_EXPIRY: u64 = 180 * 24 * 60 * 60; // 6 months (in seconds)
const MAX_FINDERS_FEE_BPS: u64 = 1000; // 10%
const BID_REMOVAL_REWARD_BPS: u64 = 500; // 5%

fn custom_mock_app() -> StargazeApp {
    StargazeApp::default()
}

pub fn contract_marketplace() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        crate::execute::execute,
        crate::execute::instantiate,
        crate::query::query,
    )
    .with_sudo(crate::sudo::sudo)
    .with_reply(crate::execute::reply);
    Box::new(contract)
}

pub fn contract_sg721() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        sg721::contract::execute,
        sg721::contract::instantiate,
        sg721::contract::query,
    );
    Box::new(contract)
}

fn setup_block_time(router: &mut StargazeApp, seconds: u64) {
    let mut block = router.block_info();
    block.time = Timestamp::from_seconds(seconds);
    router.set_block(block);
}

// Instantiates all needed contracts for testing
fn setup_contracts(
    router: &mut StargazeApp,
    creator: &Addr,
) -> Result<(Addr, Addr), ContractError> {
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
    };
    let marketplace = router
        .instantiate_contract(
            marketplace_id,
            creator.clone(),
            &msg,
            &[],
            "Marketplace",
            None,
        )
        .unwrap();
    println!("marketplace: {:?}", marketplace);

    // Setup media contract
    let sg721_id = router.store_code(contract_sg721());
    let msg = Sg721InstantiateMsg {
        name: String::from("Test Coin"),
        symbol: String::from("TEST"),
        minter: creator.to_string(),
        collection_info: CollectionInfo {
            creator: creator.to_string(),
            description: String::from("Stargaze Monkeys"),
            image:
                "ipfs://bafybeigi3bwpvyvsmnbj46ra4hyffcxdeaj6ntfk5jpic5mx27x6ih2qvq/images/1.png"
                    .to_string(),
            external_link: Some("https://example.com/external.html".to_string()),
            royalty_info: Some(RoyaltyInfoResponse {
                payment_address: creator.to_string(),
                share: Decimal::percent(10),
            }),
        },
    };
    let collection = router
        .instantiate_contract(
            sg721_id,
            creator.clone(),
            &msg,
            &coins(CREATION_FEE, NATIVE_DENOM),
            "NFT",
            None,
        )
        .unwrap();
    println!("collection: {:?}", collection);

    Ok((marketplace, collection))
}

fn setup_collection(router: &mut StargazeApp, creator: &Addr) -> Result<Addr, ContractError> {
    // Setup media contract
    let sg721_id = router.store_code(contract_sg721());
    let msg = Sg721InstantiateMsg {
        name: String::from("Test Collection 2"),
        symbol: String::from("TEST 2"),
        minter: creator.to_string(),
        collection_info: CollectionInfo {
            creator: creator.to_string(),
            description: String::from("Stargaze Monkeys 2"),
            image:
                "ipfs://bafybeigi3bwpvyvsmnbj46ra4hyffcxdeaj6ntfk5jpic5mx27x6ih2qvq/images/1.png"
                    .to_string(),
            external_link: Some("https://example.com/external.html".to_string()),
            royalty_info: Some(RoyaltyInfoResponse {
                payment_address: creator.to_string(),
                share: Decimal::percent(10),
            }),
        },
    };
    let collection = router
        .instantiate_contract(
            sg721_id,
            creator.clone(),
            &msg,
            &coins(CREATION_FEE, NATIVE_DENOM),
            "NFT",
            None,
        )
        .unwrap();
    println!("collection 2: {:?}", collection);
    Ok(collection)
}

// Intializes accounts with balances
fn setup_accounts(router: &mut StargazeApp) -> Result<(Addr, Addr, Addr), ContractError> {
    let owner: Addr = Addr::unchecked("owner");
    let bidder: Addr = Addr::unchecked("bidder");
    let creator: Addr = Addr::unchecked("creator");
    let creator_funds: Vec<Coin> = coins(CREATION_FEE, NATIVE_DENOM);
    let funds: Vec<Coin> = coins(INITIAL_BALANCE, NATIVE_DENOM);
    router
        .sudo(CwSudoMsg::Bank({
            BankSudo::Mint {
                to_address: owner.to_string(),
                amount: funds.clone(),
            }
        }))
        .map_err(|err| println!("{:?}", err))
        .ok();
    router
        .sudo(CwSudoMsg::Bank({
            BankSudo::Mint {
                to_address: bidder.to_string(),
                amount: funds.clone(),
            }
        }))
        .map_err(|err| println!("{:?}", err))
        .ok();
    router
        .sudo(CwSudoMsg::Bank({
            BankSudo::Mint {
                to_address: creator.to_string(),
                amount: creator_funds.clone(),
            }
        }))
        .map_err(|err| println!("{:?}", err))
        .ok();

    // Check native balances
    let owner_native_balances = router.wrap().query_all_balances(owner.clone()).unwrap();
    assert_eq!(owner_native_balances, funds);
    let bidder_native_balances = router.wrap().query_all_balances(bidder.clone()).unwrap();
    assert_eq!(bidder_native_balances, funds);
    let creator_native_balances = router.wrap().query_all_balances(creator.clone()).unwrap();
    assert_eq!(creator_native_balances, creator_funds);

    Ok((owner, bidder, creator))
}

fn setup_second_bidder_account(router: &mut StargazeApp) -> Result<Addr, ContractError> {
    let bidder2: Addr = Addr::unchecked("bidder2");
    let funds: Vec<Coin> = coins(CREATION_FEE + INITIAL_BALANCE, NATIVE_DENOM);
    router
        .sudo(CwSudoMsg::Bank({
            BankSudo::Mint {
                to_address: bidder2.to_string(),
                amount: funds.clone(),
            }
        }))
        .map_err(|err| println!("{:?}", err))
        .ok();

    // Check native balances
    let bidder_native_balances = router.wrap().query_all_balances(bidder2.clone()).unwrap();
    assert_eq!(bidder_native_balances, funds);

    Ok(bidder2)
}

// Mints an NFT for a creator
fn mint(router: &mut StargazeApp, creator: &Addr, collection: &Addr, token_id: u32) {
    let mint_for_creator_msg = Cw721ExecuteMsg::Mint(MintMsg {
        token_id: token_id.to_string(),
        owner: creator.clone().to_string(),
        token_uri: Some("https://starships.example.com/Starship/Enterprise.json".into()),
        extension: Empty {},
    });
    let res = router.execute_contract(
        creator.clone(),
        collection.clone(),
        &mint_for_creator_msg,
        &[],
    );
    assert!(res.is_ok());
}

fn mint_for(
    router: &mut StargazeApp,
    owner: &Addr,
    creator: &Addr,
    collection: &Addr,
    token_id: u32,
) {
    let mint_for_creator_msg = Cw721ExecuteMsg::Mint(MintMsg {
        token_id: token_id.to_string(),
        owner: owner.to_string(),
        token_uri: Some("https://starships.example.com/Starship/Enterprise.json".into()),
        extension: Empty {},
    });
    let res = router.execute_contract(
        creator.clone(),
        collection.clone(),
        &mint_for_creator_msg,
        &[],
    );
    assert!(res.is_ok());
}

fn approve(
    router: &mut StargazeApp,
    creator: &Addr,
    collection: &Addr,
    marketplace: &Addr,
    token_id: u32,
) {
    let approve_msg = Cw721ExecuteMsg::<Empty>::Approve {
        spender: marketplace.to_string(),
        token_id: token_id.to_string(),
        expires: None,
    };
    let res = router.execute_contract(creator.clone(), collection.clone(), &approve_msg, &[]);
    assert!(res.is_ok());
}

fn transfer(
    router: &mut StargazeApp,
    creator: &Addr,
    recipient: &Addr,
    collection: &Addr,
    token_id: u32,
) {
    let transfer_msg = Cw721ExecuteMsg::<Empty>::TransferNft {
        recipient: recipient.to_string(),
        token_id: token_id.to_string(),
    };
    let res = router.execute_contract(creator.clone(), collection.clone(), &transfer_msg, &[]);
    assert!(res.is_ok());
}

fn burn(router: &mut StargazeApp, creator: &Addr, collection: &Addr, token_id: u32) {
    let transfer_msg = Cw721ExecuteMsg::<Empty>::Burn {
        token_id: token_id.to_string(),
    };
    let res = router.execute_contract(creator.clone(), collection.clone(), &transfer_msg, &[]);
    assert!(res.is_ok());
}

#[test]
fn try_set_accept_bid() {
    let mut router = custom_mock_app();

    // Setup intial accounts
    let (owner, bidder, creator) = setup_accounts(&mut router).unwrap();

    // Instantiate and configure contracts
    let (marketplace, collection) = setup_contracts(&mut router, &creator).unwrap();

    // Mint NFT for creator
    mint(&mut router, &creator, &collection, TOKEN_ID);
    approve(&mut router, &creator, &collection, &marketplace, TOKEN_ID);

    // Should error with expiry lower than min
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        price: coin(110, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY - 1),
        finders_fee_bps: Some(0),
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &set_ask, &[]);
    assert!(res.is_err());

    // An asking price is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::Auction,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        price: coin(110, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finders_fee_bps: Some(0),
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &set_ask, &[]);
    assert!(res.is_ok());

    // Transfer nft from creator to owner. Creates a stale ask that needs to be updated
    transfer(&mut router, &creator, &owner, &collection, TOKEN_ID);

    // Should error on non-admin trying to update active state
    let update_ask_state = ExecuteMsg::SyncAsk {
        collection: collection.to_string(),
        token_id: TOKEN_ID,
    };
    router
        .execute_contract(creator.clone(), marketplace.clone(), &update_ask_state, &[])
        .unwrap_err();

    // Should not error on admin updating active state to false
    let res = router.execute_contract(
        Addr::unchecked("operator1"),
        marketplace.clone(),
        &update_ask_state,
        &[],
    );
    assert!(res.is_ok());

    // Should error when ask is unchanged
    router
        .execute_contract(
            Addr::unchecked("operator1"),
            marketplace.clone(),
            &update_ask_state,
            &[],
        )
        .unwrap_err();

    let ask_msg = QueryMsg::Ask {
        collection: collection.to_string(),
        token_id: TOKEN_ID,
    };
    let res: AskResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &ask_msg)
        .unwrap();
    assert!(!res.ask.unwrap().is_active);

    // Reset active state
    transfer(&mut router, &owner, &creator, &collection, TOKEN_ID);
    // after transfer, needs another approval
    approve(&mut router, &creator, &collection, &marketplace, TOKEN_ID);
    let update_ask_state = ExecuteMsg::SyncAsk {
        collection: collection.to_string(),
        token_id: TOKEN_ID,
    };
    let res = router.execute_contract(
        Addr::unchecked("operator1"),
        marketplace.clone(),
        &update_ask_state,
        &[],
    );
    assert!(res.is_ok());
    let ask_msg = QueryMsg::Ask {
        collection: collection.to_string(),
        token_id: TOKEN_ID,
    };
    let res: AskResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &ask_msg)
        .unwrap();
    assert!(res.ask.unwrap().is_active);

    // Bidder makes bid
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        finders_fee_bps: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };
    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_bid_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    // Check contract has been paid
    let contract_balances = router
        .wrap()
        .query_all_balances(marketplace.clone())
        .unwrap();
    assert_eq!(contract_balances, coins(100, NATIVE_DENOM));

    // Check creator hasn't been paid yet
    let creator_native_balances = router.wrap().query_all_balances(creator.clone()).unwrap();
    assert_eq!(creator_native_balances, vec![]);

    // Creator accepts bid
    let accept_bid_msg = ExecuteMsg::AcceptBid {
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        bidder: bidder.to_string(),
        finder: None,
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &accept_bid_msg, &[]);
    assert!(res.is_ok());

    // Check money is transfered
    let creator_native_balances = router.wrap().query_all_balances(creator).unwrap();
    // 100  - 2 (fee)
    assert_eq!(creator_native_balances, coins(100 - 2, NATIVE_DENOM));
    let bidder_native_balances = router.wrap().query_all_balances(bidder.clone()).unwrap();
    assert_eq!(
        bidder_native_balances,
        coins(INITIAL_BALANCE - 100, NATIVE_DENOM)
    );

    // Check NFT is transferred
    let query_owner_msg = Cw721QueryMsg::OwnerOf {
        token_id: TOKEN_ID.to_string(),
        include_expired: None,
    };
    let res: OwnerOfResponse = router
        .wrap()
        .query_wasm_smart(collection, &query_owner_msg)
        .unwrap();
    assert_eq!(res.owner, bidder.to_string());

    // Check contract has zero balance
    let contract_balances = router.wrap().query_all_balances(marketplace).unwrap();
    assert_eq!(contract_balances, []);
}

#[test]
fn try_set_accept_bid_no_ask() {
    let mut router = custom_mock_app();

    // Setup intial accounts
    let (_owner, bidder, creator) = setup_accounts(&mut router).unwrap();

    // Instantiate and configure contracts
    let (marketplace, collection) = setup_contracts(&mut router, &creator).unwrap();

    // Mint NFT for creator
    mint(&mut router, &creator, &collection, TOKEN_ID);
    approve(&mut router, &creator, &collection, &marketplace, TOKEN_ID);

    // Bidder makes bid
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::Auction,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        finders_fee_bps: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };
    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_bid_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    // Check contract has been paid
    let contract_balances = router
        .wrap()
        .query_all_balances(marketplace.clone())
        .unwrap();
    assert_eq!(contract_balances, coins(100, NATIVE_DENOM));

    // Check creator hasn't been paid yet
    let creator_native_balances = router.wrap().query_all_balances(creator.clone()).unwrap();
    assert_eq!(creator_native_balances, vec![]);

    // Creator accepts bid
    let accept_bid_msg = ExecuteMsg::AcceptBid {
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        bidder: bidder.to_string(),
        finder: None,
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &accept_bid_msg, &[]);
    assert!(res.is_ok());

    // Check money is transfered
    let creator_native_balances = router.wrap().query_all_balances(creator).unwrap();
    // 100  - 2 (fee)
    assert_eq!(creator_native_balances, coins(100 - 2, NATIVE_DENOM));
    let bidder_native_balances = router.wrap().query_all_balances(bidder.clone()).unwrap();
    assert_eq!(
        bidder_native_balances,
        coins(INITIAL_BALANCE - 100, NATIVE_DENOM)
    );

    // Check NFT is transferred
    let query_owner_msg = Cw721QueryMsg::OwnerOf {
        token_id: TOKEN_ID.to_string(),
        include_expired: None,
    };
    let res: OwnerOfResponse = router
        .wrap()
        .query_wasm_smart(collection, &query_owner_msg)
        .unwrap();
    assert_eq!(res.owner, bidder.to_string());

    // Check contract has zero balance
    let contract_balances = router.wrap().query_all_balances(marketplace).unwrap();
    assert_eq!(contract_balances, []);
}

#[test]
fn try_set_accept_bid_high_fees() {
    // Setup initial accounts
    // Instantiate and configure contracts
    // Setup bid with high finders fee, network fee, royalty share so seller gets nothing
    // Should throw error
    let mut router = custom_mock_app();
    let (owner, bidder, creator) = setup_accounts(&mut router).unwrap();
    let (marketplace, _) = setup_contracts(&mut router, &creator).unwrap();
    let creator_funds: Vec<Coin> = coins(CREATION_FEE, NATIVE_DENOM);

    router
        .sudo(CwSudoMsg::Bank({
            BankSudo::Mint {
                to_address: creator.to_string(),
                amount: creator_funds,
            }
        }))
        .map_err(|err| println!("{:?}", err))
        .ok();

    let sg721_id = router.store_code(contract_sg721());
    let msg = Sg721InstantiateMsg {
        name: String::from("Test Coin"),
        symbol: String::from("TEST"),
        minter: creator.to_string(),
        collection_info: CollectionInfo {
            creator: creator.to_string(),
            description: String::from("Stargaze Monkeys"),
            image:
                "ipfs://bafybeigi3bwpvyvsmnbj46ra4hyffcxdeaj6ntfk5jpic5mx27x6ih2qvq/images/1.png"
                    .to_string(),
            external_link: Some("https://example.com/external.html".to_string()),
            royalty_info: Some(RoyaltyInfoResponse {
                payment_address: creator.to_string(),
                share: Decimal::percent(100),
            }),
        },
    };
    let collection = router
        .instantiate_contract(
            sg721_id,
            creator.clone(),
            &msg,
            &coins(CREATION_FEE, NATIVE_DENOM),
            "NFT",
            None,
        )
        .unwrap();
    println!("collection: {:?}", collection);

    // Mint NFT for creator
    mint(&mut router, &creator, &collection, TOKEN_ID);
    approve(&mut router, &creator, &collection, &marketplace, TOKEN_ID);

    // Bidder makes bid
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::Auction,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        finders_fee_bps: Some(10),
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finder: Some(owner.to_string()),
    };
    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_bid_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    let accept_bid_msg = ExecuteMsg::AcceptBid {
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        bidder: bidder.to_string(),
        finder: None,
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &accept_bid_msg, &[]);
    assert_eq!(
        res.unwrap_err().source().unwrap().to_string(),
        "Generic error: Fees exceed payment".to_string()
    );
}

#[test]
fn try_update_ask() {
    let mut router = custom_mock_app();

    // Setup intial accounts
    let (_owner, _, creator) = setup_accounts(&mut router).unwrap();

    // Instantiate and configure contracts
    let (marketplace, collection) = setup_contracts(&mut router, &creator).unwrap();

    // Mint NFT for creator
    mint(&mut router, &creator, &collection, TOKEN_ID);
    approve(&mut router, &creator, &collection, &marketplace, TOKEN_ID);

    // An asking price is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        price: coin(110, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finders_fee_bps: Some(0),
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &set_ask, &[]);
    assert!(res.is_ok());

    let update_ask = ExecuteMsg::UpdateAskPrice {
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        price: coin(200, NATIVE_DENOM),
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &update_ask, &[]);
    assert!(res.is_ok());

    let update_ask = ExecuteMsg::UpdateAskPrice {
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        price: coin(200, "bobo"),
    };
    router
        .execute_contract(creator.clone(), marketplace.clone(), &update_ask, &[])
        .unwrap_err();

    let update_ask = ExecuteMsg::UpdateAskPrice {
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        price: coin(0, NATIVE_DENOM),
    };
    router
        .execute_contract(creator.clone(), marketplace.clone(), &update_ask, &[])
        .unwrap_err();

    // can not update ask price for expired ask
    let time = router.block_info().time;
    setup_block_time(&mut router, time.plus_seconds(MIN_EXPIRY + 2).seconds());
    let update_ask = ExecuteMsg::UpdateAskPrice {
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        price: coin(150, NATIVE_DENOM),
    };
    router
        .execute_contract(creator.clone(), marketplace.clone(), &update_ask, &[])
        .unwrap_err();
    // reset time to original
    setup_block_time(&mut router, time.seconds());

    // confirm ask removed
    let remove_ask_msg = ExecuteMsg::RemoveAsk {
        collection: collection.to_string(),
        token_id: TOKEN_ID,
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &remove_ask_msg, &[]);
    assert!(res.is_ok());
    let res: AskResponse = router
        .wrap()
        .query_wasm_smart(
            marketplace.to_string(),
            &QueryMsg::Ask {
                collection: collection.to_string(),
                token_id: TOKEN_ID,
            },
        )
        .unwrap();
    assert_eq!(res.ask, None);
}

#[test]
fn try_query_asks() {
    let mut router = custom_mock_app();

    // Setup intial accounts
    let (_owner, _, creator) = setup_accounts(&mut router).unwrap();

    // Instantiate and configure contracts
    let (marketplace, collection) = setup_contracts(&mut router, &creator).unwrap();

    // Mint NFT for creator
    mint(&mut router, &creator, &collection, TOKEN_ID);
    approve(&mut router, &creator, &collection, &marketplace, TOKEN_ID);

    // test before ask is made, without using pagination
    let query_asks_msg = QueryMsg::Asks {
        collection: collection.to_string(),
        include_inactive: Some(true),
        start_after: None,
        limit: None,
    };
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_asks_msg)
        .unwrap();
    assert_eq!(res.asks, vec![]);

    // An asking price is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        price: coin(110, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finders_fee_bps: Some(0),
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &set_ask, &[]);
    assert!(res.is_ok());

    // test after ask is made
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_asks_msg)
        .unwrap();
    assert_eq!(res.asks[0].token_id, TOKEN_ID);
    assert_eq!(res.asks[0].price.u128(), 110);

    // test pagination, starting when tokens exist
    let query_asks_msg = QueryMsg::Asks {
        collection: collection.to_string(),
        include_inactive: Some(true),
        start_after: Some(TOKEN_ID - 1),
        limit: None,
    };
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_asks_msg)
        .unwrap();
    assert_eq!(res.asks[0].token_id, TOKEN_ID);

    // test pagination, starting when token don't exist
    let query_asks_msg = QueryMsg::Asks {
        collection: collection.to_string(),
        include_inactive: Some(true),
        start_after: Some(TOKEN_ID),
        limit: None,
    };
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_asks_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 0);

    // test listed collections query
    let res: CollectionsResponse = router
        .wrap()
        .query_wasm_smart(
            marketplace,
            &QueryMsg::Collections {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(res.collections[0], "contract1");
}

#[test]
fn try_query_sorted_asks() {
    let mut router = custom_mock_app();

    // Setup intial accounts
    let (_owner, _, creator) = setup_accounts(&mut router).unwrap();

    // Instantiate and configure contracts
    let (marketplace, collection) = setup_contracts(&mut router, &creator).unwrap();

    // Mint NFTs for creator
    mint(&mut router, &creator, &collection, TOKEN_ID);
    approve(&mut router, &creator, &collection, &marketplace, TOKEN_ID);
    mint(&mut router, &creator, &collection, TOKEN_ID + 1);
    approve(
        &mut router,
        &creator,
        &collection,
        &marketplace,
        TOKEN_ID + 1,
    );
    mint(&mut router, &creator, &collection, TOKEN_ID + 2);
    approve(
        &mut router,
        &creator,
        &collection,
        &marketplace,
        TOKEN_ID + 2,
    );

    // An asking price is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        price: coin(110, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finders_fee_bps: Some(0),
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &set_ask, &[]);
    assert!(res.is_ok());
    // An asking price is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID + 1,
        price: coin(109, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finders_fee_bps: Some(0),
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &set_ask, &[]);
    assert!(res.is_ok());
    // An asking price is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID + 2,
        price: coin(111, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finders_fee_bps: Some(0),
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &set_ask, &[]);
    assert!(res.is_ok());

    let query_asks_msg = QueryMsg::AsksSortedByPrice {
        collection: collection.to_string(),
        include_inactive: Some(true),
        start_after: None,
        limit: None,
    };
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_asks_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 3);
    assert_eq!(res.asks[0].price.u128(), 109u128);
    assert_eq!(res.asks[1].price.u128(), 110u128);
    assert_eq!(res.asks[2].price.u128(), 111u128);

    let start_after = AskOffset::new(res.asks[0].price, res.asks[0].token_id);
    let query_msg = QueryMsg::AsksSortedByPrice {
        collection: collection.to_string(),
        include_inactive: Some(true),
        start_after: Some(start_after),
        limit: None,
    };

    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 2);
    assert_eq!(res.asks[0].price.u128(), 110u128);
    assert_eq!(res.asks[1].price.u128(), 111u128);

    let reverse_query_asks_msg = QueryMsg::ReverseAsksSortedByPrice {
        collection: collection.to_string(),
        include_inactive: Some(true),
        start_before: None,
        limit: None,
    };

    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &reverse_query_asks_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 3);
    assert_eq!(res.asks[0].price.u128(), 111u128);
    assert_eq!(res.asks[1].price.u128(), 110u128);
    assert_eq!(res.asks[2].price.u128(), 109u128);

    let start_before = AskOffset::new(res.asks[0].price, res.asks[0].token_id);
    let reverse_query_asks_start_before_first_desc_msg = QueryMsg::ReverseAsksSortedByPrice {
        collection: collection.to_string(),
        include_inactive: Some(true),
        start_before: Some(start_before),
        limit: None,
    };

    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(
            marketplace.clone(),
            &reverse_query_asks_start_before_first_desc_msg,
        )
        .unwrap();
    assert_eq!(res.asks.len(), 2);
    assert_eq!(res.asks[0].price.u128(), 110u128);
    assert_eq!(res.asks[1].price.u128(), 109u128);

    let res: AskCountResponse = router
        .wrap()
        .query_wasm_smart(
            marketplace.clone(),
            &QueryMsg::AskCount {
                collection: collection.to_string(),
            },
        )
        .unwrap();
    assert_eq!(res.count, 3);
}

#[test]
fn try_query_asks_by_seller() {
    let mut router = custom_mock_app();

    // Setup intial accounts
    let (owner, _, creator) = setup_accounts(&mut router).unwrap();

    let owner2: Addr = Addr::unchecked("owner2");
    router
        .sudo(CwSudoMsg::Bank({
            BankSudo::Mint {
                to_address: owner2.to_string(),
                amount: coins(CREATION_FEE, NATIVE_DENOM),
            }
        }))
        .map_err(|err| println!("{:?}", err))
        .ok();

    // Instantiate and configure contracts
    let (marketplace, collection) = setup_contracts(&mut router, &creator).unwrap();

    // Mint NFT for creator
    mint_for(&mut router, &owner, &creator, &collection, TOKEN_ID);
    approve(&mut router, &owner, &collection, &marketplace, TOKEN_ID);
    mint_for(&mut router, &owner2, &creator, &collection, TOKEN_ID + 1);
    approve(
        &mut router,
        &owner2,
        &collection,
        &marketplace,
        TOKEN_ID + 1,
    );
    mint_for(&mut router, &owner2, &creator, &collection, TOKEN_ID + 2);
    approve(
        &mut router,
        &owner2,
        &collection,
        &marketplace,
        TOKEN_ID + 2,
    );

    // Owner1 lists their token for sale
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        price: coin(110, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finders_fee_bps: Some(0),
    };
    let res = router.execute_contract(owner.clone(), marketplace.clone(), &set_ask, &[]);
    assert!(res.is_ok());

    // Owner2 lists their token for sale
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID + 1,
        price: coin(109, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finders_fee_bps: Some(0),
    };
    let res = router.execute_contract(owner2.clone(), marketplace.clone(), &set_ask, &[]);
    assert!(res.is_ok());

    // Owner2 lists another token for sale
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID + 2,
        price: coin(111, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finders_fee_bps: Some(0),
    };
    let res = router.execute_contract(owner2.clone(), marketplace.clone(), &set_ask, &[]);
    assert!(res.is_ok());

    let res: AskCountResponse = router
        .wrap()
        .query_wasm_smart(
            marketplace.clone(),
            &QueryMsg::AskCount {
                collection: collection.to_string(),
            },
        )
        .unwrap();
    assert_eq!(res.count, 3);

    // owner1 should only have 1 token
    let query_asks_msg = QueryMsg::AsksBySeller {
        seller: owner.to_string(),
        include_inactive: Some(true),
        start_after: None,
        limit: None,
    };
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.to_string(), &query_asks_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 1);

    // owner2 should have 2 token
    let query_asks_msg = QueryMsg::AsksBySeller {
        seller: owner2.to_string(),
        include_inactive: Some(true),
        start_after: None,
        limit: None,
    };
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.to_string(), &query_asks_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 2);

    // owner2 should have 0 tokens when paginated by a non-existing collection
    let query_asks_msg = QueryMsg::AsksBySeller {
        seller: owner2.to_string(),
        include_inactive: Some(true),
        start_after: Some(CollectionOffset::new(
            "non-existing-collection".to_string(),
            TOKEN_ID,
        )),
        limit: None,
    };
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.to_string(), &query_asks_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 0);

    // owner2 should have 2 tokens when paginated by a existing collection
    let query_asks_msg = QueryMsg::AsksBySeller {
        seller: owner2.to_string(),
        include_inactive: Some(true),
        start_after: Some(CollectionOffset::new(collection.to_string(), 0)),
        limit: None,
    };
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.to_string(), &query_asks_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 2);

    // owner2 should have 1 token when paginated by a existing collection starting after a token
    let query_asks_msg = QueryMsg::AsksBySeller {
        seller: owner2.to_string(),
        include_inactive: Some(true),
        start_after: Some(CollectionOffset::new(collection.to_string(), TOKEN_ID + 1)),
        limit: None,
    };
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.to_string(), &query_asks_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 1);
}

#[test]
fn try_query_sorted_bids() {
    let mut router = custom_mock_app();

    // Setup intial accounts
    let (_owner, bidder, creator) = setup_accounts(&mut router).unwrap();

    // Instantiate and configure contracts
    let (marketplace, collection) = setup_contracts(&mut router, &creator).unwrap();

    // Mint NFT for creator
    mint(&mut router, &creator, &collection, TOKEN_ID);
    approve(&mut router, &creator, &collection, &marketplace, TOKEN_ID);
    mint(&mut router, &creator, &collection, TOKEN_ID + 1);
    approve(
        &mut router,
        &creator,
        &collection,
        &marketplace,
        TOKEN_ID + 1,
    );
    mint(&mut router, &creator, &collection, TOKEN_ID + 2);
    approve(
        &mut router,
        &creator,
        &collection,
        &marketplace,
        TOKEN_ID + 2,
    );

    // An asking price is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::Auction,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        price: coin(110, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finders_fee_bps: Some(0),
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &set_ask, &[]);
    assert!(res.is_ok());
    // An asking price is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::Auction,
        collection: collection.to_string(),
        token_id: TOKEN_ID + 1,
        price: coin(109, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finders_fee_bps: Some(0),
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &set_ask, &[]);
    assert!(res.is_ok());
    // An asking price is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::Auction,
        collection: collection.to_string(),
        token_id: TOKEN_ID + 2,
        price: coin(111, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finders_fee_bps: Some(0),
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &set_ask, &[]);
    assert!(res.is_ok());

    // Bidder makes bid
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        finders_fee_bps: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };
    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_bid_msg,
        &coins(50, NATIVE_DENOM),
    );
    assert!(res.is_ok());
    // Bidder makes bid
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID + 1,
        finders_fee_bps: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };
    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_bid_msg,
        &coins(70, NATIVE_DENOM),
    );
    assert!(res.is_ok());
    // Bidder makes bid
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID + 2,
        finders_fee_bps: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };
    router
        .execute_contract(
            bidder.clone(),
            marketplace.clone(),
            &set_bid_msg,
            &coins(1, NATIVE_DENOM),
        )
        .unwrap_err();
    let res = router.execute_contract(
        bidder,
        marketplace.clone(),
        &set_bid_msg,
        &coins(60, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    let query_bids_msg = QueryMsg::BidsSortedByPrice {
        collection: collection.to_string(),
        limit: None,
        start_after: None,
    };
    let res: BidsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_bids_msg)
        .unwrap();
    assert_eq!(res.bids.len(), 3);
    assert_eq!(res.bids[0].price.u128(), 50u128);
    assert_eq!(res.bids[1].price.u128(), 60u128);
    assert_eq!(res.bids[2].price.u128(), 70u128);

    // test adding another bid to an existing ask
    let bidder2: Addr = Addr::unchecked("bidder2");
    let funds: Vec<Coin> = coins(INITIAL_BALANCE, NATIVE_DENOM);
    router
        .sudo(CwSudoMsg::Bank({
            BankSudo::Mint {
                to_address: bidder2.to_string(),
                amount: funds,
            }
        }))
        .map_err(|err| println!("{:?}", err))
        .ok();

    // Bidder makes bid
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        finders_fee_bps: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };
    let res = router.execute_contract(
        bidder2,
        marketplace.clone(),
        &set_bid_msg,
        &coins(40, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    let res: BidsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_bids_msg)
        .unwrap();
    assert_eq!(res.bids.len(), 4);
    assert_eq!(res.bids[0].price.u128(), 40u128);
    assert_eq!(res.bids[1].price.u128(), 50u128);
    assert_eq!(res.bids[2].price.u128(), 60u128);
    assert_eq!(res.bids[3].price.u128(), 70u128);

    // test start_after query
    let start_after = BidOffset {
        price: res.bids[2].price,
        token_id: res.bids[2].token_id,
        bidder: res.bids[2].bidder.clone(),
    };
    let query_start_after_bids_msg = QueryMsg::BidsSortedByPrice {
        collection: collection.to_string(),
        limit: None,
        start_after: Some(start_after),
    };
    let res: BidsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_start_after_bids_msg)
        .unwrap();
    assert_eq!(res.bids.len(), 1);
    assert_eq!(res.bids[0].price.u128(), 70u128);

    // test reverse bids query
    let reverse_query_bids_msg = QueryMsg::ReverseBidsSortedByPrice {
        collection: collection.to_string(),
        limit: None,
        start_before: None,
    };
    let res: BidsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &reverse_query_bids_msg)
        .unwrap();
    assert_eq!(res.bids.len(), 4);
    assert_eq!(res.bids[0].price.u128(), 70u128);
    assert_eq!(res.bids[1].price.u128(), 60u128);
    assert_eq!(res.bids[2].price.u128(), 50u128);
    assert_eq!(res.bids[3].price.u128(), 40u128);

    // test start_before reverse bids query
    let start_before = BidOffset {
        price: res.bids[1].price,
        token_id: res.bids[1].token_id,
        bidder: res.bids[1].bidder.clone(),
    };
    let reverse_query_start_before_bids_msg = QueryMsg::ReverseBidsSortedByPrice {
        collection: collection.to_string(),
        limit: None,
        start_before: Some(start_before),
    };
    let res: BidsResponse = router
        .wrap()
        .query_wasm_smart(marketplace, &reverse_query_start_before_bids_msg)
        .unwrap();
    assert_eq!(res.bids.len(), 2);
    assert_eq!(res.bids[0].price.u128(), 50u128);
    assert_eq!(res.bids[1].price.u128(), 40u128);
}

#[test]
fn try_query_bids() {
    let mut router = custom_mock_app();
    // Setup intial accounts
    let (_owner, bidder, creator) = setup_accounts(&mut router).unwrap();
    // Instantiate and configure contracts
    let (marketplace, collection) = setup_contracts(&mut router, &creator).unwrap();
    // Mint NFT for creator
    mint(&mut router, &creator, &collection, TOKEN_ID);
    approve(&mut router, &creator, &collection, &marketplace, TOKEN_ID);

    // An asking price is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::Auction,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        price: coin(110, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finders_fee_bps: Some(0),
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &set_ask, &[]);
    assert!(res.is_ok());

    // test before bid is made
    let query_bids_msg = QueryMsg::Bids {
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        start_after: None,
        limit: None,
    };
    let res: BidsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_bids_msg)
        .unwrap();
    assert_eq!(res.bids, vec![]);

    // Bidder makes bids
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        finders_fee_bps: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 10),
        finder: None,
    };
    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_bid_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::Auction,
        collection: collection.to_string(),
        token_id: TOKEN_ID + 1,
        finders_fee_bps: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };
    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_bid_msg,
        &coins(105, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    let res: BidsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_bids_msg)
        .unwrap();
    assert_eq!(res.bids[0].token_id, TOKEN_ID);
    assert_eq!(res.bids[0].price.u128(), 100u128);
    let query_bids_msg = QueryMsg::Bids {
        collection: collection.to_string(),
        token_id: TOKEN_ID + 1,
        start_after: None,
        limit: None,
    };
    let res: BidsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_bids_msg)
        .unwrap();
    assert_eq!(res.bids[0].token_id, TOKEN_ID + 1);
    assert_eq!(res.bids[0].price.u128(), 105u128);

    let query_bids_msg = QueryMsg::BidsByBidder {
        bidder: bidder.to_string(),
        start_after: Some(CollectionOffset::new(collection.to_string(), TOKEN_ID - 1)),
        limit: None,
    };
    let res: BidsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_bids_msg)
        .unwrap();
    assert_eq!(res.bids.len(), 2);
    let query_bids_msg = QueryMsg::BidsByBidderSortedByExpiration {
        bidder: bidder.to_string(),
        start_after: Some(CollectionOffset::new(collection.to_string(), TOKEN_ID - 1)),
        limit: None,
    };
    let res: BidsResponse = router
        .wrap()
        .query_wasm_smart(marketplace, &query_bids_msg)
        .unwrap();
    assert_eq!(res.bids.len(), 2);
    assert_eq!(
        res.bids[0].expires_at.seconds(),
        router
            .block_info()
            .time
            .plus_seconds(MIN_EXPIRY + 1)
            .seconds()
    );
    assert_eq!(
        res.bids[1].expires_at.seconds(),
        router
            .block_info()
            .time
            .plus_seconds(MIN_EXPIRY + 10)
            .seconds()
    );
}

#[test]
fn auto_accept_bid() {
    let mut router = custom_mock_app();

    // Setup intial accounts
    let (_owner, bidder, creator) = setup_accounts(&mut router).unwrap();

    // Instantiate and configure contracts
    let (marketplace, collection) = setup_contracts(&mut router, &creator).unwrap();

    // Mint NFT for creator
    mint(&mut router, &creator, &collection, TOKEN_ID);

    // An ask is made by the creator, but fails because NFT is not authorized
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        price: coin(100, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finders_fee_bps: Some(0),
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &set_ask, &[]);
    assert!(res.is_err());

    // Creator Authorizes NFT
    approve(&mut router, &creator, &collection, &marketplace, TOKEN_ID);

    // Now set_ask succeeds
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &set_ask, &[]);
    assert!(res.is_ok());

    // Bidder makes bid with a random token in the same amount as the ask
    router
        .sudo(CwSudoMsg::Bank({
            BankSudo::Mint {
                to_address: bidder.to_string(),
                amount: coins(1000, "random"),
            }
        }))
        .map_err(|err| println!("{:?}", err))
        .ok();
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        finders_fee_bps: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };
    router
        .execute_contract(
            bidder.clone(),
            marketplace.clone(),
            &set_bid_msg,
            &coins(100, "random"),
        )
        .unwrap_err();

    // Bidder makes bid that meets the ask criteria
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        finders_fee_bps: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };
    let res = router
        .execute_contract(
            bidder.clone(),
            marketplace,
            &set_bid_msg,
            &coins(100, NATIVE_DENOM),
        )
        .unwrap();

    // Bid is accepted, sale has been finalized
    assert_eq!(res.events[3].ty, "wasm-finalize-sale");

    // Check money is transfered
    let creator_native_balances = router.wrap().query_all_balances(creator).unwrap();
    // 100  - 2 (fee)
    assert_eq!(creator_native_balances, coins(100 - 2, NATIVE_DENOM));
    let bidder_native_balances = router.wrap().query_all_balances(bidder.clone()).unwrap();
    assert_eq!(
        bidder_native_balances,
        vec![
            coin(1000, "random"),
            coin(INITIAL_BALANCE - 100, NATIVE_DENOM),
        ]
    );

    // Check NFT is transferred
    let query_owner_msg = Cw721QueryMsg::OwnerOf {
        token_id: TOKEN_ID.to_string(),
        include_expired: None,
    };
    let res: OwnerOfResponse = router
        .wrap()
        .query_wasm_smart(collection, &query_owner_msg)
        .unwrap();
    assert_eq!(res.owner, bidder.to_string());
}

#[test]
fn try_reserved_ask() {
    let mut router = custom_mock_app();

    // Setup intial accounts
    let (owner, bidder, creator) = setup_accounts(&mut router).unwrap();

    // Instantiate and configure contracts
    let (marketplace, collection) = setup_contracts(&mut router, &creator).unwrap();

    // Mint NFT for creator
    mint(&mut router, &creator, &collection, TOKEN_ID);
    approve(&mut router, &creator, &collection, &marketplace, TOKEN_ID);

    // An ask is made by the owner
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        price: coin(100, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: Some(bidder.to_string()),
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finders_fee_bps: Some(0),
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &set_ask, &[]);
    assert!(res.is_ok());

    // Non-bidder makes bid that meets the ask price
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        finders_fee_bps: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };
    let err = router
        .execute_contract(
            owner,
            marketplace.clone(),
            &set_bid_msg,
            &coins(100, NATIVE_DENOM),
        )
        .unwrap_err();
    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::TokenReserved {}
    );

    // Bidder makes bid that meets ask price
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        finders_fee_bps: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };
    let res = router.execute_contract(
        bidder.clone(),
        marketplace,
        &set_bid_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    // Check NFT is transferred to bidder (with reserved address)
    let query_owner_msg = Cw721QueryMsg::OwnerOf {
        token_id: TOKEN_ID.to_string(),
        include_expired: None,
    };
    let res: OwnerOfResponse = router
        .wrap()
        .query_wasm_smart(collection, &query_owner_msg)
        .unwrap();
    assert_eq!(res.owner, bidder.to_string());
}

#[test]
fn try_ask_with_finders_fee() {
    let mut router = custom_mock_app();

    // Setup intial accounts
    let (_, bidder, creator) = setup_accounts(&mut router).unwrap();

    // Instantiate and configure contracts
    let (marketplace, collection) = setup_contracts(&mut router, &creator).unwrap();

    // Mint NFT for creator
    mint(&mut router, &creator, &collection, TOKEN_ID);
    approve(&mut router, &creator, &collection, &marketplace, TOKEN_ID);

    // An ask is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        price: coin(100, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finders_fee_bps: Some(500), // 5%
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &set_ask, &[]);
    assert!(res.is_ok());

    let finder = Addr::unchecked("finder".to_string());

    // Bidder makes bid that meets ask price
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        finders_fee_bps: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finder: Some(finder.to_string()),
    };
    let res = router.execute_contract(
        bidder.clone(),
        marketplace,
        &set_bid_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    // Check money is transfered
    let creator_balances = router.wrap().query_all_balances(creator).unwrap();
    // 100  - 2 (network fee) - 5 (finders fee)
    assert_eq!(creator_balances, coins(100 - 2 - 5, NATIVE_DENOM));
    let bidder_balances = router.wrap().query_all_balances(bidder).unwrap();
    assert_eq!(
        bidder_balances,
        vec![coin(INITIAL_BALANCE - 100, NATIVE_DENOM),]
    );
    let finder_balances = router.wrap().query_all_balances(finder).unwrap();
    assert_eq!(finder_balances, coins(5, NATIVE_DENOM));
}

#[test]
fn remove_bid_refund() {
    let mut router = custom_mock_app();

    // Setup intial accounts
    let (_owner, bidder, creator) = setup_accounts(&mut router).unwrap();

    // Instantiate and configure contracts
    let (marketplace, collection) = setup_contracts(&mut router, &creator).unwrap();

    // Mint NFT for creator
    mint(&mut router, &creator, &collection, TOKEN_ID);
    approve(&mut router, &creator, &collection, &marketplace, TOKEN_ID);

    // An asking price is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::Auction,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        price: coin(110, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finders_fee_bps: Some(0),
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &set_ask, &[]);
    assert!(res.is_ok());

    // Bidder makes bid
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        finders_fee_bps: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };

    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_bid_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    // Bidder sent bid money
    let bidder_native_balances = router.wrap().query_all_balances(bidder.clone()).unwrap();
    assert_eq!(
        bidder_native_balances,
        coins(INITIAL_BALANCE - 100, NATIVE_DENOM)
    );

    // Contract has been paid
    let contract_balances = router
        .wrap()
        .query_all_balances(marketplace.clone())
        .unwrap();
    assert_eq!(contract_balances, coins(100, NATIVE_DENOM));

    // Bidder removes bid
    let remove_bid_msg = ExecuteMsg::RemoveBid {
        collection: collection.to_string(),
        token_id: TOKEN_ID,
    };
    let res = router.execute_contract(bidder.clone(), marketplace, &remove_bid_msg, &[]);
    assert!(res.is_ok());

    // Bidder has money back
    let bidder_native_balances = router.wrap().query_all_balances(bidder).unwrap();
    assert_eq!(bidder_native_balances, coins(INITIAL_BALANCE, NATIVE_DENOM));
}

#[test]
fn new_bid_refund() {
    let mut router = custom_mock_app();

    // Setup intial accounts
    let (_owner, bidder, creator) = setup_accounts(&mut router).unwrap();

    // Instantiate and configure contracts
    let (marketplace, collection) = setup_contracts(&mut router, &creator).unwrap();

    // Mint NFT for creator
    mint(&mut router, &creator, &collection, TOKEN_ID);
    approve(&mut router, &creator, &collection, &marketplace, TOKEN_ID);

    // An ask is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::Auction,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        price: coin(200, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finders_fee_bps: Some(0),
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &set_ask, &[]);
    assert!(res.is_ok());

    // Bidder makes bid
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        finders_fee_bps: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };
    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_bid_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    // Bidder sent bid money
    let bidder_native_balances = router.wrap().query_all_balances(bidder.clone()).unwrap();
    assert_eq!(
        bidder_native_balances,
        coins(INITIAL_BALANCE - 100, NATIVE_DENOM)
    );

    // Contract has been paid
    let contract_balances = router
        .wrap()
        .query_all_balances(marketplace.clone())
        .unwrap();
    assert_eq!(contract_balances, coins(100, NATIVE_DENOM));

    // Bidder makes higher bid
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        finders_fee_bps: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };
    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_bid_msg,
        &coins(150, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    // Bidder has money back
    let bidder_native_balances = router.wrap().query_all_balances(bidder.clone()).unwrap();
    assert_eq!(
        bidder_native_balances,
        coins(INITIAL_BALANCE - 150, NATIVE_DENOM)
    );

    // Contract has been paid
    let contract_balances = router
        .wrap()
        .query_all_balances(marketplace.clone())
        .unwrap();
    assert_eq!(contract_balances, coins(150, NATIVE_DENOM));

    // Check new bid has been saved
    let query_bid_msg = QueryMsg::Bid {
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        bidder: bidder.to_string(),
    };
    let bid = Bid {
        collection,
        token_id: TOKEN_ID,
        bidder,
        price: Uint128::from(150u128),
        expires_at: (router.block_info().time.plus_seconds(MIN_EXPIRY + 1)),
        finders_fee_bps: None,
    };

    let res: BidResponse = router
        .wrap()
        .query_wasm_smart(marketplace, &query_bid_msg)
        .unwrap();
    assert_eq!(res.bid, Some(bid));
}

#[test]
fn try_royalties() {
    let mut router = custom_mock_app();

    // Setup intial accounts
    let (curator, bidder, creator) = setup_accounts(&mut router).unwrap();

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
    };
    let marketplace = router
        .instantiate_contract(
            marketplace_id,
            creator.clone(),
            &msg,
            &[],
            "Marketplace",
            None,
        )
        .unwrap();

    // Setup media contract with 10% royalties to a curator
    let sg721_id = router.store_code(contract_sg721());
    let msg = sg721::msg::InstantiateMsg {
        name: String::from("Test Coin"),
        symbol: String::from("TEST"),
        minter: creator.to_string(),
        collection_info: CollectionInfo {
            creator: creator.to_string(),
            description: String::from("Stargaze Monkeys"),
            image: "https://example.com/image.png".to_string(),
            external_link: Some("https://example.com/external.html".to_string()),
            royalty_info: Some(RoyaltyInfoResponse {
                payment_address: curator.to_string(),
                share: Decimal::percent(10),
            }),
        },
    };
    let collection = router
        .instantiate_contract(
            sg721_id,
            creator.clone(),
            &msg,
            &coins(CREATION_FEE, NATIVE_DENOM),
            "NFT",
            None,
        )
        .unwrap();

    // Mint NFT for creator
    mint(&mut router, &creator, &collection, TOKEN_ID);
    approve(&mut router, &creator, &collection, &marketplace, TOKEN_ID);

    // An ask is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        price: coin(100, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finders_fee_bps: Some(0),
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &set_ask, &[]);
    assert!(res.is_ok());

    // Bidder makes bid
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        finders_fee_bps: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };
    let res = router.execute_contract(
        bidder.clone(),
        marketplace,
        &set_bid_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    // Check money is transfered correctly and royalties paid
    let curator_native_balances = router.wrap().query_all_balances(curator).unwrap();
    assert_eq!(
        curator_native_balances,
        coins(INITIAL_BALANCE + 10, NATIVE_DENOM)
    );
    let creator_native_balances = router.wrap().query_all_balances(creator).unwrap();
    // 100 - 10 (royalties) - 2 (fee)
    assert_eq!(creator_native_balances, coins(100 - 10 - 2, NATIVE_DENOM));
    let bidder_native_balances = router.wrap().query_all_balances(bidder.clone()).unwrap();
    assert_eq!(
        bidder_native_balances,
        coins(INITIAL_BALANCE - 100, NATIVE_DENOM)
    );

    // Check NFT is transferred
    let query_owner_msg = Cw721QueryMsg::OwnerOf {
        token_id: TOKEN_ID.to_string(),
        include_expired: None,
    };
    let res: OwnerOfResponse = router
        .wrap()
        .query_wasm_smart(collection, &query_owner_msg)
        .unwrap();
    assert_eq!(res.owner, bidder.to_string());
}

#[test]
fn try_sudo_update_params() {
    let mut router = custom_mock_app();

    // Setup intial accounts
    let (_owner, _, creator) = setup_accounts(&mut router).unwrap();

    // Instantiate and configure contracts
    let (marketplace, _) = setup_contracts(&mut router, &creator).unwrap();

    let update_params_msg = SudoMsg::UpdateParams {
        trading_fee_bps: Some(5),
        ask_expiry: Some(ExpiryRange::new(100, 2)),
        bid_expiry: None,
        operators: Some(vec!["operator".to_string()]),
        max_finders_fee_bps: None,
        min_price: Some(Uint128::from(5u128)),
        stale_bid_duration: None,
        bid_removal_reward_bps: None,
    };
    router
        .wasm_sudo(marketplace.clone(), &update_params_msg)
        .unwrap_err();

    let update_params_msg = SudoMsg::UpdateParams {
        trading_fee_bps: Some(5),
        ask_expiry: Some(ExpiryRange::new(1, 2)),
        bid_expiry: Some(ExpiryRange::new(3, 4)),
        operators: Some(vec!["operator".to_string()]),
        max_finders_fee_bps: None,
        min_price: Some(Uint128::from(5u128)),
        stale_bid_duration: Some(10),
        bid_removal_reward_bps: Some(20),
    };
    let res = router.wasm_sudo(marketplace.clone(), &update_params_msg);
    assert!(res.is_ok());

    let query_params_msg = QueryMsg::Params {};
    let res: ParamsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_params_msg)
        .unwrap();
    assert_eq!(res.params.trading_fee_percent, Decimal::percent(5));
    assert_eq!(res.params.ask_expiry, ExpiryRange::new(1, 2));
    assert_eq!(res.params.bid_expiry, ExpiryRange::new(3, 4));
    assert_eq!(res.params.operators, vec!["operator1".to_string()]);
    assert_eq!(res.params.stale_bid_duration, Duration::Time(10));
    assert_eq!(res.params.bid_removal_reward_percent, Decimal::percent(20));

    let update_params_msg = SudoMsg::UpdateParams {
        trading_fee_bps: None,
        ask_expiry: None,
        bid_expiry: None,
        operators: Some(vec![
            "operator3".to_string(),
            "operator1".to_string(),
            "operator2".to_string(),
            "operator1".to_string(),
            "operator4".to_string(),
        ]),
        max_finders_fee_bps: None,
        min_price: None,
        stale_bid_duration: None,
        bid_removal_reward_bps: None,
    };
    let res = router.wasm_sudo(marketplace.clone(), &update_params_msg);
    assert!(res.is_ok());
    // query params
    let res: ParamsResponse = router
        .wrap()
        .query_wasm_smart(marketplace, &query_params_msg)
        .unwrap();
    assert_eq!(
        res.params.operators,
        vec![Addr::unchecked("operator1".to_string()),]
    );
}

#[test]
fn try_add_remove_sales_hooks() {
    let mut router = custom_mock_app();
    // Setup intial accounts
    let (_owner, _, creator) = setup_accounts(&mut router).unwrap();
    // Instantiate and configure contracts
    let (marketplace, _) = setup_contracts(&mut router, &creator).unwrap();

    let add_hook_msg = SudoMsg::AddSaleHook {
        hook: "hook".to_string(),
    };
    let res = router.wasm_sudo(marketplace.clone(), &add_hook_msg);
    assert!(res.is_ok());

    let query_hooks_msg = QueryMsg::SaleHooks {};
    let res: HooksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_hooks_msg)
        .unwrap();
    assert_eq!(res.hooks, vec!["hook".to_string()]);

    let remove_hook_msg = SudoMsg::RemoveSaleHook {
        hook: "hook".to_string(),
    };
    let res = router.wasm_sudo(marketplace.clone(), &remove_hook_msg);
    assert!(res.is_ok());

    let res: HooksResponse = router
        .wrap()
        .query_wasm_smart(marketplace, &query_hooks_msg)
        .unwrap();
    assert!(res.hooks.is_empty());
}

#[test]
fn try_add_too_many_sales_hooks() {
    let mut router = custom_mock_app();
    // Setup intial accounts
    let (_owner, _, creator) = setup_accounts(&mut router).unwrap();
    // Instantiate and configure contracts
    let (marketplace, _) = setup_contracts(&mut router, &creator).unwrap();

    let add_hook_msg = SudoMsg::AddSaleHook {
        hook: "hook1".to_string(),
    };
    let res = router.wasm_sudo(marketplace.clone(), &add_hook_msg);
    assert!(res.is_ok());

    let add_hook_msg = SudoMsg::AddSaleHook {
        hook: "hook2".to_string(),
    };
    let res = router.wasm_sudo(marketplace.clone(), &add_hook_msg);
    assert!(res.is_ok());

    let add_hook_msg = SudoMsg::AddSaleHook {
        hook: "hook3".to_string(),
    };
    let res = router.wasm_sudo(marketplace.clone(), &add_hook_msg);
    assert!(res.is_ok());

    let add_hook_msg = SudoMsg::AddSaleHook {
        hook: "hook4".to_string(),
    };
    let res = router.wasm_sudo(marketplace.clone(), &add_hook_msg);
    assert!(res.is_ok());

    let add_hook_msg = SudoMsg::AddSaleHook {
        hook: "hook5".to_string(),
    };
    let res = router.wasm_sudo(marketplace.clone(), &add_hook_msg);
    assert!(res.is_ok());

    let add_hook_msg = SudoMsg::AddSaleHook {
        hook: "hook6".to_string(),
    };
    let res = router.wasm_sudo(marketplace, &add_hook_msg);
    assert!(res.is_err());
}

#[test]
fn try_add_remove_bid_hooks() {
    let mut router = custom_mock_app();
    // Setup intial accounts
    let (_owner, _, creator) = setup_accounts(&mut router).unwrap();
    // Instantiate and configure contracts
    let (marketplace, _) = setup_contracts(&mut router, &creator).unwrap();

    let add_bid_hook_msg = SudoMsg::AddBidHook {
        hook: "bid_hook".to_string(),
    };
    let res = router.wasm_sudo(marketplace.clone(), &add_bid_hook_msg);
    assert!(res.is_ok());

    let query_hooks_msg = QueryMsg::BidHooks {};
    let res: HooksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_hooks_msg)
        .unwrap();
    assert_eq!(res.hooks, vec!["bid_hook".to_string()]);

    let remove_hook_msg = SudoMsg::RemoveBidHook {
        hook: "bid_hook".to_string(),
    };
    let res = router.wasm_sudo(marketplace.clone(), &remove_hook_msg);
    assert!(res.is_ok());

    let res: HooksResponse = router
        .wrap()
        .query_wasm_smart(marketplace, &query_hooks_msg)
        .unwrap();
    assert!(res.hooks.is_empty());
}

#[test]
fn try_init_hook() {
    let mut router = custom_mock_app();
    // Setup intial accounts
    let (_owner, _, creator) = setup_accounts(&mut router).unwrap();
    // Instantiate marketplace contract
    let marketplace_id = router.store_code(contract_marketplace());
    let msg = crate::msg::InstantiateMsg {
        operators: vec!["operator1".to_string()],
        trading_fee_bps: TRADING_FEE_BPS,
        ask_expiry: ExpiryRange::new(MIN_EXPIRY, MAX_EXPIRY),
        bid_expiry: ExpiryRange::new(MIN_EXPIRY, MAX_EXPIRY),
        sale_hook: Some("hook".to_string()),
        max_finders_fee_bps: MAX_FINDERS_FEE_BPS,
        min_price: Uint128::from(5u128),
        stale_bid_duration: Duration::Time(100),
        bid_removal_reward_bps: BID_REMOVAL_REWARD_BPS,
    };
    let marketplace = router
        .instantiate_contract(marketplace_id, creator, &msg, &[], "Marketplace", None)
        .unwrap();

    let query_hooks_msg = QueryMsg::SaleHooks {};
    let res: HooksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_hooks_msg)
        .unwrap();
    assert_eq!(res.hooks, vec!["hook".to_string()]);

    let remove_hook_msg = SudoMsg::RemoveSaleHook {
        hook: "hook".to_string(),
    };
    let res = router.wasm_sudo(marketplace.clone(), &remove_hook_msg);
    assert!(res.is_ok());

    let res: HooksResponse = router
        .wrap()
        .query_wasm_smart(marketplace, &query_hooks_msg)
        .unwrap();
    assert!(res.hooks.is_empty());
}

#[test]
fn try_hook_was_run() {
    let mut router = custom_mock_app();
    // Setup intial accounts
    let (_owner, bidder, creator) = setup_accounts(&mut router).unwrap();
    // Instantiate and configure contracts
    let (marketplace, collection) = setup_contracts(&mut router, &creator).unwrap();

    // Add sales hook
    let add_hook_msg = SudoMsg::AddSaleHook {
        hook: "hook".to_string(),
    };
    let _res = router.wasm_sudo(marketplace.clone(), &add_hook_msg);

    // Add listed hook
    let add_ask_hook_msg = SudoMsg::AddAskHook {
        hook: "ask_created_hook".to_string(),
    };
    let _res = router.wasm_sudo(marketplace.clone(), &add_ask_hook_msg);

    // Add bid created hook
    let add_ask_hook_msg = SudoMsg::AddBidHook {
        hook: "bid_created_hook".to_string(),
    };
    let _res = router.wasm_sudo(marketplace.clone(), &add_ask_hook_msg);

    // Mint NFT for creator
    mint(&mut router, &creator, &collection, TOKEN_ID);

    // An ask is made by the creator, but fails because NFT is not authorized
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        price: coin(100, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finders_fee_bps: Some(0),
    };
    // Creator Authorizes NFT
    let approve_msg = Cw721ExecuteMsg::<Empty>::Approve {
        spender: marketplace.to_string(),
        token_id: TOKEN_ID.to_string(),
        expires: None,
    };
    let res = router.execute_contract(creator.clone(), collection.clone(), &approve_msg, &[]);
    assert!(res.is_ok());
    // Now set_ask succeeds
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &set_ask, &[]);
    assert!(res.is_ok());
    assert_eq!(
        "ask-hook-failed",
        res.unwrap().events[3].attributes[1].value
    );
    // Bidder makes bid that meets the ask criteria
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        finders_fee_bps: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };

    // Bid succeeds even though the hook contract cannot be found
    let res = router.execute_contract(
        bidder.clone(),
        marketplace,
        &set_bid_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(res.is_ok());
    assert_eq!(
        "sale-hook-failed",
        res.as_ref().unwrap().events[10].attributes[1].value
    );

    // NFT is still transferred despite a sale finalized hook failing
    let query_owner_msg = Cw721QueryMsg::OwnerOf {
        token_id: TOKEN_ID.to_string(),
        include_expired: None,
    };
    let res: OwnerOfResponse = router
        .wrap()
        .query_wasm_smart(collection, &query_owner_msg)
        .unwrap();
    assert_eq!(res.owner, bidder.to_string());
}

#[test]
fn try_add_remove_listed_hooks() {
    let mut router = custom_mock_app();
    // Setup intial accounts
    let (_owner, _, creator) = setup_accounts(&mut router).unwrap();
    // Instantiate and configure contracts
    let (marketplace, _) = setup_contracts(&mut router, &creator).unwrap();

    let add_hook_msg = SudoMsg::AddAskHook {
        hook: "hook".to_string(),
    };
    let res = router.wasm_sudo(marketplace.clone(), &add_hook_msg);
    assert!(res.is_ok());

    let query_hooks_msg = QueryMsg::AskHooks {};
    let res: HooksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_hooks_msg)
        .unwrap();
    assert_eq!(res.hooks, vec!["hook".to_string()]);

    let remove_hook_msg = SudoMsg::RemoveAskHook {
        hook: "hook".to_string(),
    };
    let res = router.wasm_sudo(marketplace.clone(), &remove_hook_msg);
    assert!(res.is_ok());

    let res: HooksResponse = router
        .wrap()
        .query_wasm_smart(marketplace, &query_hooks_msg)
        .unwrap();
    assert!(res.hooks.is_empty());
}

#[test]
fn try_collection_bids() {
    let mut router = custom_mock_app();

    // Setup intial accounts
    let (_owner, bidder, creator) = setup_accounts(&mut router).unwrap();
    let bidder2 = setup_second_bidder_account(&mut router).unwrap();

    // Instantiate and configure contracts
    let (marketplace, collection) = setup_contracts(&mut router, &creator).unwrap();

    // Mint NFT for creator
    mint(&mut router, &creator, &collection, TOKEN_ID);
    approve(&mut router, &creator, &collection, &marketplace, TOKEN_ID);

    // A collection bid is made by the bidder
    let set_collection_bid = ExecuteMsg::SetCollectionBid {
        collection: collection.to_string(),
        finders_fee_bps: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 10),
    };
    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_collection_bid,
        &coins(150, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    // An invalid collection bid is attempted by the bidder
    let set_collection_bid = ExecuteMsg::SetCollectionBid {
        collection: collection.to_string(),
        finders_fee_bps: Some(10100),
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 10),
    };
    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_collection_bid,
        &coins(151, NATIVE_DENOM),
    );
    assert!(res.is_err());

    // A collection bid is made by bidder2
    let set_collection_bid = ExecuteMsg::SetCollectionBid {
        collection: collection.to_string(),
        finders_fee_bps: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 5),
    };
    let res = router.execute_contract(
        bidder2.clone(),
        marketplace.clone(),
        &set_collection_bid,
        &coins(180, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    // test querying a single collection bid
    let query_collection_bid = QueryMsg::CollectionBid {
        collection: collection.to_string(),
        bidder: bidder.to_string(),
    };
    let res: CollectionBidResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_collection_bid)
        .unwrap();
    assert_eq!(res.bid.unwrap().price.u128(), 150u128);

    // test querying all collection bids by bidder
    let query_collection_bids = QueryMsg::CollectionBidsByBidder {
        bidder: bidder.to_string(),
        start_after: None,
        limit: None,
    };
    let res: CollectionBidsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_collection_bids)
        .unwrap();
    assert_eq!(res.bids[0].price.u128(), 150u128);

    // test querying all sorted collection bids by bidder
    let query_collection_bids_by_price = QueryMsg::CollectionBidsSortedByPrice {
        collection: collection.to_string(),
        start_after: None,
        limit: Some(10),
    };
    let res: CollectionBidsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_collection_bids_by_price)
        .unwrap();
    assert_eq!(res.bids.len(), 2);
    assert_eq!(res.bids[0].price.u128(), 150u128);
    assert_eq!(res.bids[1].price.u128(), 180u128);

    // test start_after
    let start_after = CollectionBidOffset::new(
        res.bids[0].price,
        collection.to_string(),
        res.bids[0].bidder.to_string(),
    );
    let query_sorted_collection_bids = QueryMsg::CollectionBidsSortedByPrice {
        collection: collection.to_string(),
        start_after: Some(start_after),
        limit: Some(10),
    };
    let res: CollectionBidsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_sorted_collection_bids)
        .unwrap();
    assert_eq!(res.bids.len(), 1);
    assert_eq!(res.bids[0].price.u128(), 180u128);

    // test querying all sorted collection bids by bidder sorted by expiration
    // add another collection
    let collection2 = setup_collection(&mut router, &bidder2).unwrap();
    // set another collection bid
    let set_collection_bid = ExecuteMsg::SetCollectionBid {
        collection: collection2.to_string(),
        finders_fee_bps: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 10),
    };
    let res = router.execute_contract(
        bidder2.clone(),
        marketplace.clone(),
        &set_collection_bid,
        &coins(180, NATIVE_DENOM),
    );
    assert!(res.is_ok());
    let query_collection_bids_by_expiration = QueryMsg::CollectionBidsByBidderSortedByExpiration {
        bidder: bidder2.to_string(),
        start_after: None,
        limit: None,
    };
    let res: CollectionBidsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_collection_bids_by_expiration)
        .unwrap();
    assert_eq!(res.bids.len(), 2);
    assert_eq!(
        res.bids[0].expires_at.seconds(),
        router
            .block_info()
            .time
            .plus_seconds(MIN_EXPIRY + 5)
            .seconds()
    );
    assert_eq!(
        res.bids[1].expires_at.seconds(),
        router
            .block_info()
            .time
            .plus_seconds(MIN_EXPIRY + 10)
            .seconds()
    );

    // test querying all sorted collection bids by bidder in reverse
    let reverse_query_sorted_collection_bids = QueryMsg::ReverseCollectionBidsSortedByPrice {
        collection: collection.to_string(),
        start_before: None,
        limit: Some(10),
    };
    let res: CollectionBidsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &reverse_query_sorted_collection_bids)
        .unwrap();
    assert_eq!(res.bids.len(), 2);
    assert_eq!(res.bids[0].price.u128(), 180u128);
    assert_eq!(res.bids[1].price.u128(), 150u128);

    // test start_before
    let start_before = CollectionBidOffset::new(
        res.bids[0].price,
        collection.to_string(),
        res.bids[0].bidder.to_string(),
    );
    let reverse_query_sorted_collection_bids = QueryMsg::ReverseCollectionBidsSortedByPrice {
        collection: collection.to_string(),
        start_before: Some(start_before),
        limit: Some(10),
    };
    let res: CollectionBidsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &reverse_query_sorted_collection_bids)
        .unwrap();
    assert_eq!(res.bids.len(), 1);
    assert_eq!(res.bids[0].price.u128(), 150u128);

    // test removing collection bid
    let remove_collection_bid = ExecuteMsg::RemoveCollectionBid {
        collection: collection.to_string(),
    };
    let res = router.execute_contract(bidder2, marketplace.clone(), &remove_collection_bid, &[]);
    assert!(res.is_ok());
    let query_sorted_collection_bids = QueryMsg::CollectionBidsSortedByPrice {
        collection: collection.to_string(),
        start_after: None,
        limit: Some(10),
    };
    let res: CollectionBidsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_sorted_collection_bids)
        .unwrap();
    assert_eq!(res.bids.len(), 1);
    assert_eq!(res.bids[0].price.u128(), 150u128);

    // A collection bid is accepted
    let accept_collection_bid = ExecuteMsg::AcceptCollectionBid {
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        bidder: bidder.to_string(),
        finder: None,
    };
    let res = router.execute_contract(creator.clone(), marketplace, &accept_collection_bid, &[]);
    assert!(res.is_ok());
}

#[test]
fn try_remove_stale_bid() {
    let mut router = custom_mock_app();

    // Setup intial accounts
    let (_owner, bidder, creator) = setup_accounts(&mut router).unwrap();

    // Instantiate and configure contracts
    let (marketplace, collection) = setup_contracts(&mut router, &creator).unwrap();

    // Mint NFT for creator
    mint(&mut router, &creator, &collection, TOKEN_ID);
    approve(&mut router, &creator, &collection, &marketplace, TOKEN_ID);

    // Bidder makes bid
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::Auction,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        finders_fee_bps: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };
    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_bid_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    let operator = Addr::unchecked("operator1".to_string());

    // Try to remove the bid (not yet stale) as an operator
    let remove_msg = ExecuteMsg::RemoveStaleBid {
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        bidder: bidder.to_string(),
    };
    router
        .execute_contract(operator.clone(), marketplace.clone(), &remove_msg, &[])
        .unwrap_err();

    setup_block_time(&mut router, 10000000000);

    let res = router.execute_contract(operator.clone(), marketplace.clone(), &remove_msg, &[]);
    assert!(res.is_ok());

    let operator_balances = router.wrap().query_all_balances(operator).unwrap();
    assert_eq!(operator_balances, coins(5, NATIVE_DENOM));
}

#[test]
fn try_remove_stale_collection_bid() {
    let mut router = custom_mock_app();

    // Setup intial accounts
    let (_owner, bidder, creator) = setup_accounts(&mut router).unwrap();

    // Instantiate and configure contracts
    let (marketplace, collection) = setup_contracts(&mut router, &creator).unwrap();

    // Mint NFT for creator
    mint(&mut router, &creator, &collection, TOKEN_ID);
    approve(&mut router, &creator, &collection, &marketplace, TOKEN_ID);

    let expiry_time = router
        .block_info()
        .time
        .plus_seconds(MIN_EXPIRY + 1)
        .seconds();

    // Bidder makes collection bid
    let set_col_bid_msg = ExecuteMsg::SetCollectionBid {
        collection: collection.to_string(),
        finders_fee_bps: None,
        expires: Timestamp::from_seconds(expiry_time),
    };
    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_col_bid_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    let operator = Addr::unchecked("operator1".to_string());

    // Try to remove the collection bid (not yet stale) as an operator
    let remove_col_msg = ExecuteMsg::RemoveStaleCollectionBid {
        collection: collection.to_string(),
        bidder: bidder.to_string(),
    };
    router
        .execute_contract(operator.clone(), marketplace.clone(), &remove_col_msg, &[])
        .unwrap_err();

    // make bid stale by adding stale_bid_duration
    setup_block_time(&mut router, expiry_time + 100);

    let res = router.execute_contract(operator.clone(), marketplace.clone(), &remove_col_msg, &[]);
    assert!(res.is_ok());

    let operator_balances = router.wrap().query_all_balances(operator).unwrap();
    assert_eq!(operator_balances, coins(5, NATIVE_DENOM));
}

#[test]
fn try_bid_finders_fee() {
    let mut router = custom_mock_app();

    // Setup intial accounts
    let (_owner, bidder, creator) = setup_accounts(&mut router).unwrap();

    // Instantiate and configure contracts
    let (marketplace, collection) = setup_contracts(&mut router, &creator).unwrap();

    // Mint NFT for creator
    mint(&mut router, &creator, &collection, TOKEN_ID);
    approve(&mut router, &creator, &collection, &marketplace, TOKEN_ID);

    // Bidder makes failed bid with a large finder's fee
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        finders_fee_bps: Some(5000),
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };
    let err = router
        .execute_contract(
            bidder.clone(),
            marketplace.clone(),
            &set_bid_msg,
            &coins(100, NATIVE_DENOM),
        )
        .unwrap_err();
    assert_eq!(
        err.source().unwrap().to_string(),
        ContractError::InvalidFindersFeeBps(5000).to_string()
    );

    // Bidder makes bid with a finder's fee
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::Auction,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        finders_fee_bps: Some(500),
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };
    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_bid_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    let finder = Addr::unchecked("finder".to_string());

    // Token owner accepts the bid with a finder address
    let accept_bid_msg = ExecuteMsg::AcceptBid {
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        bidder: bidder.to_string(),
        finder: Some(finder.to_string()),
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &accept_bid_msg, &[]);
    assert!(res.is_ok());

    let finder_balances = router.wrap().query_all_balances(finder).unwrap();
    assert_eq!(finder_balances, coins(5, NATIVE_DENOM));
}

#[test]
fn try_bidder_cannot_be_finder() {
    let mut router = custom_mock_app();

    // Setup intial accounts
    let (_owner, bidder, creator) = setup_accounts(&mut router).unwrap();

    // Instantiate and configure contracts
    let (marketplace, collection) = setup_contracts(&mut router, &creator).unwrap();

    // Mint NFT for creator
    mint(&mut router, &creator, &collection, TOKEN_ID);
    approve(&mut router, &creator, &collection, &marketplace, TOKEN_ID);

    // Bidder makes bid with a finder's fee
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        finders_fee_bps: Some(500),
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finder: Some(bidder.to_string()),
    };
    router
        .execute_contract(
            bidder,
            marketplace.clone(),
            &set_bid_msg,
            &coins(100, NATIVE_DENOM),
        )
        .unwrap_err();
}

#[test]
fn try_ask_with_filter_inactive() {
    let mut router = custom_mock_app();

    // Setup intial accounts
    let (owner, _, creator) = setup_accounts(&mut router).unwrap();

    // Instantiate and configure contracts
    let (marketplace, collection) = setup_contracts(&mut router, &creator).unwrap();

    // Mint NFT for creator
    mint(&mut router, &creator, &collection, TOKEN_ID);
    approve(&mut router, &creator, &collection, &marketplace, TOKEN_ID);

    // An ask is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        price: coin(100, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finders_fee_bps: Some(500), // 5%
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &set_ask, &[]);
    assert!(res.is_ok());

    // transfer nft from creator to owner. Creates a stale ask that needs to be updated
    transfer(&mut router, &creator, &owner, &collection, TOKEN_ID);

    let update_ask_state = ExecuteMsg::SyncAsk {
        collection: collection.to_string(),
        token_id: TOKEN_ID,
    };
    let res = router.execute_contract(
        Addr::unchecked("operator1"),
        marketplace.clone(),
        &update_ask_state,
        &[],
    );
    assert!(res.is_ok());

    let ask_msg = QueryMsg::Asks {
        collection: collection.to_string(),
        include_inactive: None,
        start_after: None,
        limit: None,
    };
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &ask_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 0);

    let ask_msg = QueryMsg::Asks {
        collection: collection.to_string(),
        include_inactive: Some(false),
        start_after: None,
        limit: None,
    };
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &ask_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 0);

    let ask_msg = QueryMsg::Asks {
        collection: collection.to_string(),
        include_inactive: Some(true),
        start_after: None,
        limit: None,
    };
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &ask_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 1);

    // updating price of inactive ask throws error
    let update_ask = ExecuteMsg::UpdateAskPrice {
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        price: coin(200, NATIVE_DENOM),
    };
    router
        .execute_contract(creator.clone(), marketplace.clone(), &update_ask, &[])
        .unwrap_err();
}

#[test]
fn try_sync_ask() {
    let mut router = custom_mock_app();

    // Setup intial accounts
    let (owner, _, creator) = setup_accounts(&mut router).unwrap();

    // Instantiate and configure contracts
    let (marketplace, collection) = setup_contracts(&mut router, &creator).unwrap();

    // Mint NFT for creator
    mint(&mut router, &creator, &collection, TOKEN_ID);
    approve(&mut router, &creator, &collection, &marketplace, TOKEN_ID);

    // An ask is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        price: coin(100, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finders_fee_bps: Some(500), // 5%
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &set_ask, &[]);
    assert!(res.is_ok());

    // Transfer NFT from creator to owner. Creates a stale ask that needs to be updated
    transfer(&mut router, &creator, &owner, &collection, TOKEN_ID);

    let update_ask_state = ExecuteMsg::SyncAsk {
        collection: collection.to_string(),
        token_id: TOKEN_ID,
    };
    let res = router.execute_contract(
        Addr::unchecked("operator1"),
        marketplace.clone(),
        &update_ask_state,
        &[],
    );
    assert!(res.is_ok());

    let ask_msg = QueryMsg::Asks {
        collection: collection.to_string(),
        include_inactive: Some(false),
        start_after: None,
        limit: None,
    };
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &ask_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 0);

    // transfer nft back
    transfer(&mut router, &owner, &creator, &collection, TOKEN_ID);

    // Transfer Back should have unchanged operation (still not active)
    let res = router.execute_contract(
        Addr::unchecked("operator1"),
        marketplace.clone(),
        &update_ask_state,
        &[],
    );
    assert!(res.is_err());

    let ask_msg = QueryMsg::Asks {
        collection: collection.to_string(),
        include_inactive: Some(false),
        start_after: None,
        limit: None,
    };
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &ask_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 0);

    // Approving again should have a success sync ask after
    approve(&mut router, &creator, &collection, &marketplace, TOKEN_ID);

    // SyncAsk should be ok
    let res = router.execute_contract(
        Addr::unchecked("operator1"),
        marketplace.clone(),
        &update_ask_state,
        &[],
    );
    assert!(res.is_ok());
    let ask_msg = QueryMsg::Asks {
        collection: collection.to_string(),
        include_inactive: Some(false),
        start_after: None,
        limit: None,
    };
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &ask_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 1);

    // Approve for shorter period than ask
    let approve_msg = Cw721ExecuteMsg::<Empty>::Approve {
        spender: marketplace.to_string(),
        token_id: TOKEN_ID.to_string(),
        expires: Some(Expiration::AtTime(
            router.block_info().time.plus_seconds(MIN_EXPIRY - 10),
        )),
    };
    let res = router.execute_contract(creator.clone(), collection.clone(), &approve_msg, &[]);
    assert!(res.is_ok());

    // SyncAsk should fail (Unchanged)
    let res = router.execute_contract(
        Addr::unchecked("operator1"),
        marketplace.clone(),
        &update_ask_state,
        &[],
    );
    assert!(res.is_err());

    let expiry_time = router
        .block_info()
        .time
        .plus_seconds(MIN_EXPIRY - 5)
        .seconds();
    // move clock before ask expire but after approval expiration time
    setup_block_time(&mut router, expiry_time);

    // SyncAsk should succeed as approval is no longer valid
    let res = router.execute_contract(
        Addr::unchecked("operator1"),
        marketplace.clone(),
        &update_ask_state,
        &[],
    );
    assert!(res.is_ok());

    // No more valid asks
    let ask_msg = QueryMsg::Asks {
        collection: collection.to_string(),
        include_inactive: Some(false),
        start_after: None,
        limit: None,
    };
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &ask_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 0);
}

#[test]
fn try_set_ask_reserve_for() {
    let mut router = custom_mock_app();

    // Setup intial accounts
    let (_, bidder, creator) = setup_accounts(&mut router).unwrap();

    // Instantiate and configure contracts
    let (marketplace, collection) = setup_contracts(&mut router, &creator).unwrap();

    // Mint NFT for creator
    mint(&mut router, &creator, &collection, TOKEN_ID);
    approve(&mut router, &creator, &collection, &marketplace, TOKEN_ID);

    // Can't reserve to themselves.
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        price: coin(110, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: Some(creator.clone().to_string()),
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 10),
        finders_fee_bps: Some(0),
    };
    let err = router
        .execute_contract(creator.clone(), marketplace.clone(), &set_ask, &[])
        .unwrap_err();
    assert_eq!(
        err.source().unwrap().to_string(),
        ContractError::InvalidReserveAddress {
            reason: "cannot reserve to the same address".to_owned(),
        }
        .to_string()
    );
    // Can't reserve for auction sale type
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::Auction,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        price: coin(110, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: Some(bidder.to_string()),
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 10),
        finders_fee_bps: Some(0),
    };
    let err = router
        .execute_contract(creator.clone(), marketplace.clone(), &set_ask, &[])
        .unwrap_err();
    assert_eq!(
        err.source().unwrap().to_string(),
        ContractError::InvalidReserveAddress {
            reason: "can only reserve for fixed_price sales".to_owned(),
        }
        .to_string()
    );

    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        price: coin(110, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: Some(bidder.to_string()),
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 10),
        finders_fee_bps: Some(0),
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &set_ask, &[]);
    assert!(res.is_ok());

    let bidder2 = setup_second_bidder_account(&mut router).unwrap();
    // Bidder2 makes bid
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        finders_fee_bps: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };
    let res = router.execute_contract(
        bidder2,
        marketplace.clone(),
        &set_bid_msg,
        &coins(110, NATIVE_DENOM),
    );
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert_eq!(
        err.source().unwrap().to_string(),
        ContractError::TokenReserved {}.to_string()
    );

    // Bidder makes bid
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        finders_fee_bps: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };
    let res = router.execute_contract(
        bidder,
        marketplace.clone(),
        &set_bid_msg,
        &coins(110, NATIVE_DENOM),
    );
    assert!(res.is_ok());
}

#[test]
fn try_remove_stale_ask() {
    let mut router = custom_mock_app();

    // Setup intial accounts
    let (owner, _, creator) = setup_accounts(&mut router).unwrap();

    // Instantiate and configure contracts
    let (marketplace, collection) = setup_contracts(&mut router, &creator).unwrap();

    // Mint NFT for creator
    mint(&mut router, &creator, &collection, TOKEN_ID);
    approve(&mut router, &creator, &collection, &marketplace, TOKEN_ID);

    // An ask is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        price: coin(100, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finders_fee_bps: Some(500), // 5%
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &set_ask, &[]);
    assert!(res.is_ok());

    // trying to remove a valid ask
    let remove_ask = ExecuteMsg::RemoveStaleAsk {
        collection: collection.to_string(),
        token_id: TOKEN_ID,
    };
    let res = router.execute_contract(
        Addr::unchecked("operator1"),
        marketplace.clone(),
        &remove_ask,
        &[],
    );
    assert!(res.is_err());

    let ask_msg = QueryMsg::Asks {
        collection: collection.to_string(),
        include_inactive: Some(false),
        start_after: None,
        limit: None,
    };

    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &ask_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 1);

    // burn the token
    burn(&mut router, &creator, &collection, TOKEN_ID);

    // try again to remove the ask
    let res = router.execute_contract(
        Addr::unchecked("operator1"),
        marketplace.clone(),
        &remove_ask,
        &[],
    );
    assert!(res.is_ok());
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &ask_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 0);

    // Mint NFT for creator
    mint(&mut router, &creator, &collection, TOKEN_ID);
    approve(&mut router, &creator, &collection, &marketplace, TOKEN_ID);

    // set ask again
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &set_ask, &[]);
    assert!(res.is_ok());

    let res = router.execute_contract(
        Addr::unchecked("operator1"),
        marketplace.clone(),
        &remove_ask,
        &[],
    );
    assert!(res.is_err());

    // Transfer NFT from creator to owner. Creates a stale ask that needs to be updated
    transfer(&mut router, &creator, &owner, &collection, TOKEN_ID);
    let res = router.execute_contract(
        Addr::unchecked("operator1"),
        marketplace.clone(),
        &remove_ask,
        &[],
    );
    assert!(res.is_err());

    // transfer nft back
    transfer(&mut router, &owner, &creator, &collection, TOKEN_ID);

    // move time forward
    let time = router.block_info().time;
    setup_block_time(&mut router, time.plus_seconds(MIN_EXPIRY + 2).seconds());

    // remove stale ask
    let res = router.execute_contract(
        Addr::unchecked("operator1"),
        marketplace.clone(),
        &remove_ask,
        &[],
    );
    assert!(res.is_ok());
    let res: AsksResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &ask_msg)
        .unwrap();
    assert_eq!(res.asks.len(), 0);
}

#[test]
fn try_add_and_remove_operators() {
    let mut router = custom_mock_app();
    // Setup intial accounts
    let (_owner, _, creator) = setup_accounts(&mut router).unwrap();
    // Instantiate and configure contracts
    let (marketplace, _) = setup_contracts(&mut router, &creator).unwrap();

    let update_params_msg = SudoMsg::UpdateParams {
        trading_fee_bps: Some(5),
        ask_expiry: Some(ExpiryRange::new(1, 2)),
        bid_expiry: Some(ExpiryRange::new(3, 4)),
        operators: Some(vec!["operator2".to_string()]),
        max_finders_fee_bps: Some(1),
        min_price: Some(Uint128::from(5u128)),
        stale_bid_duration: Some(10),
        bid_removal_reward_bps: Some(20),
    };
    let res = router.wasm_sudo(marketplace.clone(), &update_params_msg);
    assert!(res.is_ok());

    // add single operator
    let add_operator_msg = SudoMsg::AddOperator {
        operator: "operator2".to_string(),
    };

    let res = router.wasm_sudo(marketplace.clone(), &add_operator_msg);
    assert!(res.is_ok());

    let query_params_msg = QueryMsg::Params {};
    let mut res: ParamsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_params_msg)
        .unwrap();
    assert_eq!(res.params.trading_fee_percent, Decimal::percent(5));
    assert_eq!(res.params.ask_expiry, ExpiryRange::new(1, 2));
    res.params.operators.sort();
    assert_eq!(
        res.params.operators,
        vec![
            Addr::unchecked("operator1".to_string()),
            Addr::unchecked("operator2".to_string()),
        ]
    );

    // remove single operator
    let add_operator_msg = SudoMsg::RemoveOperator {
        operator: "operator1".to_string(),
    };

    let res = router.wasm_sudo(marketplace.clone(), &add_operator_msg);
    assert!(res.is_ok());

    let query_params_msg = QueryMsg::Params {};
    let res: ParamsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_params_msg)
        .unwrap();
    assert_eq!(res.params.trading_fee_percent, Decimal::percent(5));
    assert_eq!(res.params.ask_expiry, ExpiryRange::new(1, 2));

    assert_eq!(res.params.trading_fee_percent, Decimal::percent(5));
    assert_eq!(res.params.ask_expiry, ExpiryRange::new(1, 2));
    assert_eq!(res.params.bid_expiry, ExpiryRange::new(3, 4));
    assert_eq!(res.params.operators, vec!["operator2".to_string()]);
    assert_eq!(res.params.stale_bid_duration, Duration::Time(10));
    assert_eq!(res.params.min_price, Uint128::from(5u128));
    assert_eq!(res.params.bid_removal_reward_percent, Decimal::percent(20));
    assert_eq!(
        res.params.operators,
        vec![Addr::unchecked("operator2".to_string()),]
    );

    // remove unexisting operator
    let remove_operator = SudoMsg::RemoveOperator {
        operator: "operator1".to_string(),
    };
    let res = router.wasm_sudo(marketplace.clone(), &remove_operator);
    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err().to_string(),
        ContractError::OperatorNotRegistered {}.to_string()
    );

    // add existing operator
    let add_operator_msg = SudoMsg::AddOperator {
        operator: "operator2".to_string(),
    };
    let res = router.wasm_sudo(marketplace, &add_operator_msg);
    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err().to_string(),
        ContractError::OperatorAlreadyRegistered {}.to_string()
    );
}

#[test]
fn try_bid_sale_type() {
    let mut router = custom_mock_app();

    // Setup intial accounts
    let (_, bidder, creator) = setup_accounts(&mut router).unwrap();

    // Instantiate and configure contracts
    let (marketplace, collection) = setup_contracts(&mut router, &creator).unwrap();

    // Mint NFT for creator
    mint(&mut router, &creator, &collection, TOKEN_ID);
    approve(&mut router, &creator, &collection, &marketplace, TOKEN_ID);

    // An asking price is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        price: coin(100, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finders_fee_bps: Some(0),
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &set_ask, &[]);
    assert!(res.is_ok());

    // Bidder makes bid
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        finders_fee_bps: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };
    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_bid_msg,
        &coins(100, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    // Check creator has been paid yet
    let creator_native_balances = router.wrap().query_all_balances(creator.clone()).unwrap();
    assert_eq!(creator_native_balances, coins(100 - 2, NATIVE_DENOM));

    // Check contract has zero balance
    let contract_balances = router
        .wrap()
        .query_all_balances(marketplace.clone())
        .unwrap();
    assert_eq!(contract_balances, []);

    transfer(&mut router, &bidder, &creator, &collection, TOKEN_ID);

    let bidder2 = setup_second_bidder_account(&mut router).unwrap();

    // Bidder makes bid
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        finders_fee_bps: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };
    let res = router.execute_contract(
        bidder2.clone(),
        marketplace.clone(),
        &set_bid_msg,
        &coins(100, NATIVE_DENOM),
    );

    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err().source().unwrap().to_string(),
        ContractError::AskNotFound {}.to_string()
    );

    // Bidder makes bid with Auction
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::Auction,
        collection: collection.to_string(),
        token_id: TOKEN_ID,
        finders_fee_bps: None,
        expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };
    let res = router.execute_contract(
        bidder2.clone(),
        marketplace.clone(),
        &set_bid_msg,
        &coins(100, NATIVE_DENOM),
    );

    assert!(res.is_ok());

    let query_bids_msg = QueryMsg::BidsByBidder {
        bidder: bidder2.to_string(),
        limit: None,
        start_after: None,
    };
    let res: BidsResponse = router
        .wrap()
        .query_wasm_smart(marketplace.clone(), &query_bids_msg)
        .unwrap();
    assert_eq!(res.bids.len(), 1);
    assert_eq!(res.bids[0].price.u128(), 100u128);
}
