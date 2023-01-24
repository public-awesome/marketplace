use crate::error::ContractError;
use crate::execute::migrate;
use crate::helpers::ExpiryRange;
use crate::msg::{ExecuteMsg, QueryMsg};
use crate::msg::{ParamsResponse, SudoMsg};
use crate::state::{SaleType, SudoParams, SUDO_PARAMS};
use crate::testing::helpers::funds::{
    add_funds_for_incremental_fee, listing_funds, MINT_FEE_FAIR_BURN,
};
use crate::testing::helpers::nft_functions::{approve, get_next_token_id_and_map, mint};
use crate::testing::setup::setup_accounts::{
    setup_second_bidder_account, CREATION_FEE, INITIAL_BALANCE, MINT_PRICE,
};
use crate::testing::setup::setup_marketplace::{
    setup_marketplace, BID_REMOVAL_REWARD_BPS, LISTING_FEE, MAX_EXPIRY, MAX_FINDERS_FEE_BPS,
    MIN_EXPIRY, TRADING_FEE_BPS,
};
use cosmwasm_std::testing::{mock_dependencies, mock_env};
use cosmwasm_std::{Addr, Empty, Timestamp};
use cw721::{Cw721QueryMsg, OwnerOfResponse};
use cw_multi_test::Executor;
use sg_std::GENESIS_MINT_START_TIME;

use cosmwasm_std::{coin, coins, Decimal, Uint128};
use cw_utils::Duration;
use std::collections::HashSet;
use test_suite::common_setup::setup_accounts_and_block::setup_block_time;

use crate::testing::setup::templates::{
    minter_two_collections_with_time, standard_minter_template,
};
use cw2::set_contract_version;
use sg_std::NATIVE_DENOM;

#[test]
fn try_sudo_update_params() {
    let vt = standard_minter_template(1);
    let (mut router, owner) = (vt.router, vt.accts.owner);
    let marketplace = setup_marketplace(&mut router, owner).unwrap();
    setup_block_time(&mut router, GENESIS_MINT_START_TIME, None);

    let update_params_msg = SudoMsg::UpdateParams {
        trading_fee_bps: Some(5),
        ask_expiry: Some(ExpiryRange::new(100, 2)),
        bid_expiry: None,
        operators: Some(vec!["operator".to_string()]),
        max_finders_fee_bps: None,
        min_price: Some(Uint128::from(5u128)),
        stale_bid_duration: None,
        bid_removal_reward_bps: None,
        listing_fee: Some(Uint128::from(LISTING_FEE)),
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
        listing_fee: Some(Uint128::from(LISTING_FEE)),
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
    assert_eq!(res.params.min_price, Uint128::from(5u128));
    assert_eq!(res.params.stale_bid_duration, Duration::Time(10));
    assert_eq!(res.params.bid_removal_reward_percent, Decimal::percent(20));
    assert_eq!(res.params.listing_fee, Uint128::from(LISTING_FEE));

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
        listing_fee: None,
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
fn try_start_trading_time() {
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    let vt = minter_two_collections_with_time(2, start_time, start_time.plus_seconds(1));
    let (mut router, owner, bidder, creator) =
        (vt.router, vt.accts.owner, vt.accts.bidder, vt.accts.creator);
    let marketplace = setup_marketplace(&mut router, owner).unwrap();
    let minter_addr = vt.collection_response_vec[0].minter.clone().unwrap();
    let collection = vt.collection_response_vec[0].collection.clone().unwrap();

    let minter_2 = vt.collection_response_vec[1].minter.clone().unwrap();
    let collection_2 = vt.collection_response_vec[1].collection.clone().unwrap();
    setup_block_time(&mut router, start_time.nanos(), None);
    add_funds_for_incremental_fee(&mut router, &creator, CREATION_FEE, 1u128).unwrap();

    // Mint NFT for creator
    mint(&mut router, &creator, &minter_addr);

    // // after transfer, needs another approval
    let nft_hash_minter_1: HashSet<String> = HashSet::from([]);
    let (_, minter_1_token_id_0) =
        get_next_token_id_and_map(&mut router, &nft_hash_minter_1, collection.clone());
    approve(
        &mut router,
        &creator,
        &collection,
        &marketplace,
        minter_1_token_id_0,
    );
    // // Mint NFT for creator
    mint(&mut router, &creator, &minter_2);
    // // after transfer, needs another approval
    let nft_hash_minter_2: HashSet<String> = HashSet::from([]);
    let (_, minter_2_token_id_0) =
        get_next_token_id_and_map(&mut router, &nft_hash_minter_2, collection_2.clone());
    approve(
        &mut router,
        &creator,
        &collection_2,
        &marketplace,
        minter_2_token_id_0,
    );

    let creator_native_balances = router.wrap().query_all_balances(creator.clone()).unwrap();

    // Bidder makes bid on NFT with no ask
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: minter_1_token_id_0,
        finders_fee_bps: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };
    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_bid_msg,
        &coins(100, NATIVE_DENOM),
    );

    assert!(res.is_ok());

    // Bidder makes bid on NFT with no ask to collection 2
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection_2.to_string(),
        token_id: minter_2_token_id_0,
        finders_fee_bps: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 1),
        finder: None,
    };
    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_bid_msg,
        &coins(100, NATIVE_DENOM),
    );

    assert!(res.is_ok());

    let bidder2 = setup_second_bidder_account(&mut router).unwrap();

    // A collection bid is made by the bidder
    let set_collection_bid = ExecuteMsg::SetCollectionBid {
        collection: collection_2.to_string(),
        finders_fee_bps: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 10),
    };
    let res = router.execute_contract(
        bidder2.clone(),
        marketplace.clone(),
        &set_collection_bid,
        &coins(100, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    // An asking price is made by the creator
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection.to_string(),
        token_id: minter_1_token_id_0,
        price: coin(110, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 1),
        finders_fee_bps: Some(0),
    };
    let res = router.execute_contract(
        creator.clone(),
        marketplace.clone(),
        &set_ask,
        &listing_funds(LISTING_FEE).unwrap(),
    );

    assert!(res.is_ok());

    // An asking price is made by the creator to collection 2 (should fail)
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection_2.to_string(),
        token_id: minter_2_token_id_0,
        price: coin(110, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 1),
        finders_fee_bps: Some(0),
    };
    let res = router.execute_contract(
        creator.clone(),
        marketplace.clone(),
        &set_ask,
        &listing_funds(LISTING_FEE).unwrap(),
    );

    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err().source().unwrap().to_string(),
        "Collection not tradable yet".to_string()
    );

    // Check creator hasn't been paid yet
    let (uint128_two, fair_burn_percent) = (
        Uint128::from(2u32),
        Decimal::percent(MINT_FEE_FAIR_BURN / 100),
    );
    let mint_price = Uint128::from(MINT_PRICE);
    let creator_balance_minus_two_fees =
        Uint128::from(INITIAL_BALANCE) - (mint_price * uint128_two * fair_burn_percent);
    assert_eq!(
        creator_native_balances[0],
        coin((creator_balance_minus_two_fees).u128(), NATIVE_DENOM)
    );

    // Creator accepts bid
    let accept_bid_msg = ExecuteMsg::AcceptBid {
        collection: collection.to_string(),
        token_id: minter_1_token_id_0,
        bidder: bidder.to_string(),
        finder: None,
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &accept_bid_msg, &[]);
    assert!(res.is_ok());

    // Check money is transferred
    let creator_native_balances = router.wrap().query_all_balances(creator.clone()).unwrap();
    // 100  - 2 (fee)
    assert_eq!(
        creator_native_balances,
        coins(
            creator_balance_minus_two_fees.u128() + 100 - 2,
            NATIVE_DENOM
        )
    );
    let bidder_native_balances = router.wrap().query_all_balances(bidder.clone()).unwrap();
    assert_eq!(
        bidder_native_balances,
        coins(INITIAL_BALANCE - 200, NATIVE_DENOM)
    );

    // Check NFT is transferred
    let query_owner_msg = Cw721QueryMsg::OwnerOf {
        token_id: minter_1_token_id_0.to_string(),
        include_expired: None,
    };
    let res: OwnerOfResponse = router
        .wrap()
        .query_wasm_smart(collection, &query_owner_msg)
        .unwrap();
    assert_eq!(res.owner, bidder.to_string());

    // Creator tries to accept accept bid on collection 2 (should fail)
    let accept_bid_msg = ExecuteMsg::AcceptCollectionBid {
        collection: collection_2.to_string(),
        token_id: minter_2_token_id_0,
        bidder: bidder2.to_string(),
        finder: None,
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &accept_bid_msg, &[]);
    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err().source().unwrap().to_string(),
        "Collection not tradable yet".to_string()
    );

    // A collection bid is accepted
    let accept_collection_bid = ExecuteMsg::AcceptCollectionBid {
        collection: collection_2.to_string(),
        token_id: minter_2_token_id_0,
        bidder: bidder2.to_string(),
        finder: None,
    };

    let res = router.execute_contract(
        creator.clone(),
        marketplace.clone(),
        &accept_collection_bid,
        &[],
    );
    assert!(res.is_err());
    assert_eq!(
        res.unwrap_err().source().unwrap().to_string(),
        "Collection not tradable yet".to_string()
    );

    // move time to start trading time
    setup_block_time(&mut router, start_time.plus_seconds(1).nanos(), None);

    // Creator tries to accept accept bid on collection 2  should work now
    let accept_bid_msg = ExecuteMsg::AcceptBid {
        collection: collection_2.to_string(),
        token_id: minter_2_token_id_0,
        bidder: bidder.to_string(),
        finder: None,
    };
    let res = router.execute_contract(creator.clone(), marketplace.clone(), &accept_bid_msg, &[]);
    assert!(res.is_ok());

    // Check money is transferred
    let creator_native_balances = router.wrap().query_all_balances(creator).unwrap();
    // 200  - 4  sold 2 items
    assert_eq!(
        creator_native_balances,
        coins(
            creator_balance_minus_two_fees.u128() + 200 - 4,
            NATIVE_DENOM
        )
    );

    // bidder approves marketplace to transfer NFT
    approve(
        &mut router,
        &bidder,
        &collection_2,
        &marketplace,
        minter_2_token_id_0,
    );

    // An asking price is made by the bidder to collection
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection_2.to_string(),
        token_id: minter_2_token_id_0,
        price: coin(110, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 2),
        finders_fee_bps: Some(0),
    };
    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_ask,
        &listing_funds(LISTING_FEE).unwrap(),
    );
    assert!(res.is_ok());

    // A collection bid is accepted
    let accept_collection_bid = ExecuteMsg::AcceptCollectionBid {
        collection: collection_2.to_string(),
        token_id: minter_2_token_id_0,
        bidder: bidder2.to_string(),
        finder: None,
    };

    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &accept_collection_bid,
        &[],
    );

    assert!(res.is_ok());

    approve(
        &mut router,
        &bidder2,
        &collection_2,
        &marketplace,
        minter_2_token_id_0,
    );

    // An asking price is made by the bidder to collection
    let set_ask = ExecuteMsg::SetAsk {
        sale_type: SaleType::FixedPrice,
        collection: collection_2.to_string(),
        token_id: minter_2_token_id_0,
        price: coin(110, NATIVE_DENOM),
        funds_recipient: None,
        reserve_for: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 2),
        finders_fee_bps: Some(0),
    };
    let res = router.execute_contract(
        bidder2.clone(),
        marketplace.clone(),
        &set_ask,
        &listing_funds(LISTING_FEE).unwrap(),
    );
    assert!(res.is_ok());

    // Bidder buys now
    let set_bid_msg = ExecuteMsg::SetBid {
        sale_type: SaleType::FixedPrice,
        collection: collection_2.to_string(),
        token_id: minter_2_token_id_0,
        finders_fee_bps: None,
        expires: start_time.plus_seconds(MIN_EXPIRY + 2),
        finder: None,
    };
    let res = router.execute_contract(
        bidder.clone(),
        marketplace.clone(),
        &set_bid_msg,
        &coins(110, NATIVE_DENOM),
    );
    assert!(res.is_ok());

    // Check NFT is transferred
    let query_owner_msg = Cw721QueryMsg::OwnerOf {
        token_id: minter_2_token_id_0.to_string(),
        include_expired: None,
    };
    let res: OwnerOfResponse = router
        .wrap()
        .query_wasm_smart(collection_2, &query_owner_msg)
        .unwrap();
    assert_eq!(res.owner, bidder.to_string());
}

#[test]
fn try_add_and_remove_operators() {
    let vt = standard_minter_template(1);
    let (mut router, owner) = (vt.router, vt.accts.owner);
    let marketplace = setup_marketplace(&mut router, owner).unwrap();
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    setup_block_time(&mut router, start_time.nanos(), None);

    let update_params_msg = SudoMsg::UpdateParams {
        trading_fee_bps: Some(5),
        ask_expiry: Some(ExpiryRange::new(1, 2)),
        bid_expiry: Some(ExpiryRange::new(3, 4)),
        operators: Some(vec!["operator2".to_string()]),
        max_finders_fee_bps: Some(1),
        min_price: Some(Uint128::from(5u128)),
        stale_bid_duration: Some(10),
        bid_removal_reward_bps: Some(20),
        listing_fee: Some(Uint128::from(LISTING_FEE)),
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
fn try_migrate() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let old_params = SudoParams {
        operators: vec![Addr::unchecked("operator1")],
        trading_fee_percent: Decimal::percent(TRADING_FEE_BPS),
        ask_expiry: ExpiryRange::new(MIN_EXPIRY, MAX_EXPIRY),
        bid_expiry: ExpiryRange::new(MIN_EXPIRY, MAX_EXPIRY),
        max_finders_fee_percent: Decimal::percent(MAX_FINDERS_FEE_BPS),
        min_price: Uint128::from(5u128),
        stale_bid_duration: Duration::Time(100),
        bid_removal_reward_percent: Decimal::percent(BID_REMOVAL_REWARD_BPS),
        listing_fee: Uint128::from(LISTING_FEE),
    };

    SUDO_PARAMS.save(&mut deps.storage, &old_params).unwrap();

    // should error when different name
    set_contract_version(&mut deps.storage, "crates.io:marketplace", "0.15.0").unwrap();
    let err = migrate(deps.as_mut(), env.clone(), Empty {}).unwrap_err();

    assert_eq!(
        err.to_string(),
        "Generic error: Cannot upgrade to a different contract"
    );

    // should error when version is greater version
    set_contract_version(&mut deps.storage, "crates.io:sg-marketplace", "2.0.0").unwrap();
    let err = migrate(deps.as_mut(), env.clone(), Empty {}).unwrap_err();
    assert_eq!(
        err.to_string(),
        "Generic error: Cannot upgrade to a previous contract version"
    );

    // no op when same version
    set_contract_version(
        &mut deps.storage,
        "crates.io:sg-marketplace",
        env!("CARGO_PKG_VERSION"),
    )
    .unwrap();
    migrate(deps.as_mut(), env.clone(), Empty {}).unwrap();

    set_contract_version(&mut deps.storage, "crates.io:sg-marketplace", "1.0.0").unwrap();
    migrate(deps.as_mut(), env, Empty {}).unwrap();

    let new_params = SUDO_PARAMS.load(&deps.storage).unwrap();

    assert_eq!(new_params.operators, old_params.operators);
    assert_eq!(
        new_params.trading_fee_percent,
        old_params.trading_fee_percent
    );
    assert_eq!(new_params.ask_expiry, old_params.ask_expiry);
    assert_eq!(new_params.bid_expiry, old_params.bid_expiry);
    assert_eq!(
        new_params.max_finders_fee_percent,
        old_params.max_finders_fee_percent
    );
    assert_eq!(new_params.min_price, old_params.min_price);
    assert_eq!(new_params.stale_bid_duration, old_params.stale_bid_duration);
    assert_eq!(
        new_params.bid_removal_reward_percent,
        old_params.bid_removal_reward_percent
    );
    assert_eq!(new_params.listing_fee, old_params.listing_fee);
}
