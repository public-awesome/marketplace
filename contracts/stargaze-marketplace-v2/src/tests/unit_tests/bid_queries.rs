use crate::{
    msg::{ExecuteMsg, PriceOffset, QueryMsg},
    orders::{Bid, OrderDetails},
    tests::{
        helpers::utils::find_attrs,
        setup::{
            setup_accounts::TestAccounts,
            setup_contracts::{JUNO_DENOM, NATIVE_DENOM},
            templates::{test_context, TestContext, TestContracts},
        },
    },
};

use cosmwasm_std::coin;
use cosmwasm_std::Addr;
use cw_multi_test::Executor;
use sg_index_query::{QueryBound, QueryOptions};

#[test]
fn try_query_bids() {
    let TestContext {
        mut app,
        contracts:
            TestContracts {
                marketplace,
                collection,
                ..
            },
        accounts: TestAccounts { bidder, .. },
    } = test_context();

    let num_bids: u8 = 4;
    let token_id = "1".to_string();
    let mut bid_ids: Vec<String> = vec![];
    for idx in 1..(num_bids + 1) {
        let bid_price = coin(1000000u128 + idx as u128, NATIVE_DENOM);
        let set_bid = ExecuteMsg::SetBid {
            collection: collection.to_string(),
            token_id: token_id.to_string(),
            details: OrderDetails {
                price: bid_price.clone(),
                recipient: None,
                finder: None,
                expiry: None,
            },
        };
        let response =
            app.execute_contract(bidder.clone(), marketplace.clone(), &set_bid, &[bid_price]);
        assert!(response.is_ok());

        let bid_id = find_attrs(response.unwrap(), "wasm-set-bid", "id")
            .pop()
            .unwrap();
        bid_ids.push(bid_id);
    }

    let bids = app
        .wrap()
        .query_wasm_smart::<Vec<Bid>>(&marketplace, &QueryMsg::Bids(bid_ids.clone()))
        .unwrap();
    assert_eq!(bids.len(), num_bids as usize);

    for (idx, bid) in bids.iter().enumerate() {
        assert_eq!(bid.id, bid_ids[idx]);
    }
}

#[test]
fn try_query_bids_by_token_price() {
    let TestContext {
        mut app,
        contracts:
            TestContracts {
                marketplace,
                collection,
                ..
            },
        accounts: TestAccounts { bidder, .. },
    } = test_context();

    let num_bids: u8 = 4;
    let token_id = "1".to_string();
    let mut bid_ids: Vec<String> = vec![];
    for idx in 1..(num_bids + 1) {
        let bid_price = coin(1000000u128 + idx as u128, NATIVE_DENOM);
        let set_bid = ExecuteMsg::SetBid {
            collection: collection.to_string(),
            token_id: token_id.to_string(),
            details: OrderDetails {
                price: bid_price.clone(),
                recipient: None,
                finder: None,
                expiry: None,
            },
        };
        let response =
            app.execute_contract(bidder.clone(), marketplace.clone(), &set_bid, &[bid_price]);
        assert!(response.is_ok());

        let bid_id = find_attrs(response.unwrap(), "wasm-set-bid", "id")
            .pop()
            .unwrap();
        bid_ids.push(bid_id);
    }

    // Other collection returns no bids
    let dummy_collection = Addr::unchecked("dummy_collection");
    let bids = app
        .wrap()
        .query_wasm_smart::<Vec<Bid>>(
            &marketplace,
            &QueryMsg::BidsByTokenPrice {
                collection: dummy_collection.to_string(),
                token_id: "1".to_string(),
                denom: NATIVE_DENOM.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(bids.len(), 0);

    // Other token id returns no bids
    let bids = app
        .wrap()
        .query_wasm_smart::<Vec<Bid>>(
            &marketplace,
            &QueryMsg::BidsByTokenPrice {
                collection: collection.to_string(),
                token_id: "2".to_string(),
                denom: NATIVE_DENOM.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(bids.len(), 0);

    // Other denoms returns no bids
    let bids = app
        .wrap()
        .query_wasm_smart::<Vec<Bid>>(
            &marketplace,
            &QueryMsg::BidsByTokenPrice {
                collection: collection.to_string(),
                token_id: "1".to_string(),
                denom: JUNO_DENOM.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(bids.len(), 0);

    // Correct number of bids returned for correct token_id and denom
    let bids = app
        .wrap()
        .query_wasm_smart::<Vec<Bid>>(
            &marketplace,
            &QueryMsg::BidsByTokenPrice {
                collection: collection.to_string(),
                token_id: "1".to_string(),
                denom: NATIVE_DENOM.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(bids.len(), num_bids as usize);

    // Query Options work
    let qo_bids = app
        .wrap()
        .query_wasm_smart::<Vec<Bid>>(
            &marketplace,
            &QueryMsg::BidsByTokenPrice {
                collection: collection.to_string(),
                token_id: "1".to_string(),
                denom: NATIVE_DENOM.to_string(),
                query_options: Some(QueryOptions {
                    descending: Some(true),
                    limit: None,
                    min: Some(QueryBound::Exclusive(PriceOffset {
                        id: bids[0].id.clone(),
                        amount: bids[0].details.price.amount.u128(),
                    })),
                    max: Some(QueryBound::Exclusive(PriceOffset {
                        id: bids[3].id.clone(),
                        amount: bids[3].details.price.amount.u128(),
                    })),
                }),
            },
        )
        .unwrap();

    assert_eq!(qo_bids.len(), 2);

    for (idx, bid) in qo_bids.iter().enumerate() {
        let bid_idx = 2 - idx;
        assert_eq!(bid.id, bids[bid_idx].id);
        assert_eq!(
            bid.details.price.amount.u128(),
            bids[bid_idx].details.price.amount.u128()
        );
    }
}

#[test]
fn try_query_bids_by_creator() {
    let TestContext {
        mut app,
        contracts:
            TestContracts {
                marketplace,
                collection,
                ..
            },
        accounts: TestAccounts { owner, bidder, .. },
    } = test_context();

    let num_bids: u8 = 4;
    let token_id = "1".to_string();
    let mut bid_ids: Vec<String> = vec![];
    for idx in 1..(num_bids + 1) {
        let bid_price = coin(1000000u128 + idx as u128, NATIVE_DENOM);
        let set_bid = ExecuteMsg::SetBid {
            collection: collection.to_string(),
            token_id: token_id.to_string(),
            details: OrderDetails {
                price: bid_price.clone(),
                recipient: None,
                finder: None,
                expiry: None,
            },
        };
        let response =
            app.execute_contract(bidder.clone(), marketplace.clone(), &set_bid, &[bid_price]);
        assert!(response.is_ok());

        let bid_id = find_attrs(response.unwrap(), "wasm-set-bid", "id")
            .pop()
            .unwrap();
        bid_ids.push(bid_id);
    }

    // Other creator address returns no bids
    let bids = app
        .wrap()
        .query_wasm_smart::<Vec<Bid>>(
            &marketplace,
            &QueryMsg::BidsByCreatorCollection {
                creator: owner.to_string(),
                collection: collection.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(bids.len(), 0);

    // Correct number of bids returned for correct creator
    let bids = app
        .wrap()
        .query_wasm_smart::<Vec<Bid>>(
            &marketplace,
            &QueryMsg::BidsByCreatorCollection {
                creator: bidder.to_string(),
                collection: collection.to_string(),
                query_options: None,
            },
        )
        .unwrap();
    assert_eq!(bids.len(), num_bids as usize);

    // Query Options work
    let qo_bids = app
        .wrap()
        .query_wasm_smart::<Vec<Bid>>(
            &marketplace,
            &QueryMsg::BidsByCreatorCollection {
                creator: bidder.to_string(),
                collection: collection.to_string(),
                query_options: Some(QueryOptions {
                    descending: Some(true),
                    limit: Some(2),
                    min: Some(QueryBound::Exclusive(bids[0].id.clone())),
                    max: Some(QueryBound::Exclusive(bids[3].id.clone())),
                }),
            },
        )
        .unwrap();

    assert_eq!(qo_bids.len(), 2);

    for (idx, bid) in qo_bids.iter().enumerate() {
        let bid_idx = 2 - idx;
        assert_eq!(bid.id, bids[bid_idx].id);
        assert_eq!(
            bid.details.price.amount.u128(),
            bids[bid_idx].details.price.amount.u128()
        );
    }
}
