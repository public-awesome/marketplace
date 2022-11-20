// use crate::msg::{AskResponse, BidResponse, ExecuteMsg, QueryMsg};
// use crate::tests_folder::multitest::{
//     approve, listing_funds, mint, setup_contracts,
//     setup_second_bidder_account,
// };
// use crate::tests_folder::setup_accounts_and_block::setup_accounts;
// use crate::tests_folder::setup_contracts::custom_mock_app;
// use crate::tests_folder::setup_accounts_and_block::CREATOR_INITIAL_BALANCE;
// use crate::tests_folder::multitest::{LISTING_FEE, MIN_EXPIRY, TOKEN_ID};
// use crate::state::SaleType;
// use cosmwasm_std::{coin, coins, Uint128};
// use cw_multi_test::Executor;
// use sg_std::NATIVE_DENOM;

// use crate::error::ContractError;
// use crate::execute::migrate;
// use crate::helpers::ExpiryRange;
// use crate::msg::{
//     AskCountResponse, AskOffset, AsksResponse, BidOffset,
//     CollectionBidOffset, CollectionOffset, CollectionsResponse, ParamsResponse, SudoMsg,
// };
// use crate::msg::{
//     BidsResponse, CollectionBidResponse, CollectionBidsResponse,
// };
// use crate::state::{Bid, SudoParams, SUDO_PARAMS};
// use crate::tests_folder::setup_accounts_and_block::{
//     setup_block_time, INITIAL_BALANCE,
// };
// use crate::tests_folder::setup_contracts::{contract_marketplace};
// use crate::tests_folder::setup_minter::{
//     configure_minter, MinterCollectionResponse, MINT_FEE_FAIR_BURN, MINT_PRICE,
// };
// use cosmwasm_std::testing::{mock_dependencies, mock_env};
// use cosmwasm_std::{Addr, Empty, Timestamp};
// use cw721::{Cw721QueryMsg, OwnerOfResponse, TokensResponse};
// use cw_multi_test::{BankSudo, SudoMsg as CwSudoMsg};
// use sg2::msg::CollectionParams;
// use sg721_base::msg::CollectionInfoResponse;
// use sg_controllers::HooksResponse;
// use sg_multi_test::StargazeApp;
// use sg_std::GENESIS_MINT_START_TIME;

// use cw_utils::{Duration, Expiration};
// use sg721::ExecuteMsg as Sg721ExecuteMsg;
// use std::collections::HashSet;
// use std::iter::FromIterator;

// use crate::tests_folder::mock_collection_params::{
//     mock_collection_params_1, mock_collection_two,
// };
// use crate::tests_folder::setup_minter::CREATION_FEE;

// #[test]
// fn set_auction_bids() {
//     let mut router = custom_mock_app();
//     let (_, bidder, creator) = setup_accounts(&mut router).unwrap();
//     add_funds_for_incremental_fee(&mut router, &creator, CREATION_FEE, 1u128).unwrap();
//     let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
//     let collection_params_1 = mock_collection_params_1(Some(start_time));
//     let collection_params_2 = mock_collection_two(Some(start_time.plus_seconds(1)));
//     let setup_params = SetupContractsParams {
//         minter_admin: creator.clone(),
//         collection_params_vec: vec![collection_params_1, collection_params_2],
//         num_tokens: 2,
//         router: &mut router,
//     };
//     let (marketplace, minter_collections) = setup_contracts(setup_params).unwrap();
//     let minter_addr = minter_collections[0].minter.clone();
//     let collection = minter_collections[0].collection.clone();
//     // An asking price is made by the creator
//     let set_ask = ExecuteMsg::SetAsk {
//         sale_type: SaleType::Auction,
//         collection: collection.to_string(),
//         token_id: TOKEN_ID,
//         price: coin(150, NATIVE_DENOM),
//         funds_recipient: None,
//         reserve_for: None,
//         expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
//         finders_fee_bps: Some(0),
//     };

//     let res = router.execute_contract(
//         creator.clone(),
//         marketplace.clone(),
//         &set_ask,
//         &listing_funds(LISTING_FEE).unwrap(),
//     );
//     assert!(res.is_ok());

//     // Bidder makes bid
//     let set_bid_msg = ExecuteMsg::SetBid {
//         sale_type: SaleType::Auction,
//         collection: collection.to_string(),
//         token_id: TOKEN_ID,
//         finders_fee_bps: None,
//         expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
//         finder: None,
//     };

//     // Bidder makes bid lower than the asking price
//     let res = router.execute_contract(
//         bidder.clone(),
//         marketplace.clone(),
//         &set_bid_msg,
//         &coins(100, NATIVE_DENOM),
//     );
//     assert!(res.is_err());

//     // Bidder makes bid higher than the asking price
//     let res = router.execute_contract(
//         bidder.clone(),
//         marketplace.clone(),
//         &set_bid_msg,
//         &coins(200, NATIVE_DENOM),
//     );
//     assert!(res.is_ok());

//     let ask_query = QueryMsg::Ask {
//         collection: collection.to_string(),
//         token_id: TOKEN_ID,
//     };

//     // ask should be returned
//     let res: AskResponse = router
//         .wrap()
//         .query_wasm_smart(marketplace.clone(), &ask_query)
//         .unwrap();
//     assert_ne!(res.ask, None);

//     let bid_query = QueryMsg::Bid {
//         collection: collection.to_string(),
//         token_id: TOKEN_ID,
//         bidder: bidder.to_string(),
//     };

//     // bid should be returned
//     let res: BidResponse = router
//         .wrap()
//         .query_wasm_smart(marketplace.clone(), &bid_query)
//         .unwrap();
//     assert_ne!(res.bid, None);
//     let bid = res.bid.unwrap();
//     assert_eq!(bid.price, Uint128::from(200u128));

//     let bidder2 = setup_second_bidder_account(&mut router).unwrap();

//     // Bidder 2 makes bid equal to the asking price
//     let set_bid_msg = ExecuteMsg::SetBid {
//         sale_type: SaleType::FixedPrice,
//         collection: collection.to_string(),
//         token_id: TOKEN_ID,
//         finders_fee_bps: None,
//         expires: router.block_info().time.plus_seconds(MIN_EXPIRY + 1),
//         finder: None,
//     };

//     let res = router.execute_contract(
//         bidder2.clone(),
//         marketplace.clone(),
//         &set_bid_msg,
//         &coins(150, NATIVE_DENOM),
//     );
//     assert!(res.is_ok());

//     // bid should be returned for bidder 1
//     let res: BidResponse = router
//         .wrap()
//         .query_wasm_smart(marketplace.clone(), &bid_query)
//         .unwrap();
//     assert_ne!(res.bid, None);
//     let bid = res.bid.unwrap();
//     assert_eq!(bid.price, Uint128::from(200u128));

//     let bid_query = QueryMsg::Bid {
//         collection: collection.to_string(),
//         token_id: TOKEN_ID,
//         bidder: bidder2.to_string(),
//     };

//     // bid should  be returned for bidder 2
//     let res: BidResponse = router
//         .wrap()
//         .query_wasm_smart(marketplace.clone(), &bid_query)
//         .unwrap();
//     assert_ne!(res.bid, None);
//     let bid = res.bid.unwrap();
//     assert_eq!(bid.price, Uint128::from(150u128));

//     // Creator accepts bid
//     let accept_bid_msg = ExecuteMsg::AcceptBid {
//         collection: collection.to_string(),
//         token_id: TOKEN_ID,
//         bidder: bidder.to_string(),
//         finder: None,
//     };
//     let res = router.execute_contract(creator.clone(), marketplace.clone(), &accept_bid_msg, &[]);
//     assert!(res.is_ok());
//     // ask should have been removed
//     let res: AskResponse = router
//         .wrap()
//         .query_wasm_smart(marketplace.clone(), &ask_query)
//         .unwrap();
//     assert_eq!(res.ask, None);

//     // Check creator has been paid
//     let creator_native_balances = router.wrap().query_all_balances(creator.clone()).unwrap();
//     assert_eq!(
//         creator_native_balances,
//         coins(CREATOR_INITIAL_BALANCE + 200 - 4, NATIVE_DENOM)
//     );

//     // Check contract has second bid balance
//     let contract_balances = router.wrap().query_all_balances(marketplace).unwrap();
//     assert_eq!(contract_balances, coins(150, NATIVE_DENOM));
// }
