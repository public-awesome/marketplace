use crate::{
    bank_send, calculate_nft_sale_fees, has_approval, load_collection_royalties, only_owner,
    owner_of, payout_nft_sale_fees, transfer_nft, TokenPayment, TransactionFees,
};

use cosmwasm_std::{
    coin,
    testing::{mock_dependencies, mock_info},
    to_binary, Addr, BankMsg, ContractResult, Decimal, Querier, QuerierResult, QuerierWrapper,
    StdError, SystemResult, Uint128,
};
use cw721::OwnerOfResponse;
use mockall::*;
use sg721::{RoyaltyInfo, RoyaltyInfoResponse};
use sg721_base::msg::CollectionInfoResponse;
use sg_std::{CosmosMsg, Response, StargazeMsg, StargazeMsgWrapper, NATIVE_DENOM};
use test_suite::common_setup::templates::base_minter_with_sg721;

#[test]
fn try_transfer_nft() {
    let collection = Addr::unchecked("collection");
    let recipient = Addr::unchecked("recipient");
    transfer_nft(&collection, "1", &recipient);
}

#[test]
fn try_bank_send() {
    let recipient = Addr::unchecked("recipient");
    bank_send(coin(100u128, NATIVE_DENOM), &recipient);
}

#[test]
fn try_owner_of() {
    let bmt = base_minter_with_sg721(1);
    let collection = bmt.collection_response_vec[0].collection.clone().unwrap();
    let creator = bmt.accts.creator;
    let token_id = 1;

    mock! {
        QuerierStruct {}
        impl Querier for QuerierStruct {
            fn raw_query(&self, bin_request: &[u8]) -> QuerierResult;
        }
    }

    let return_value = SystemResult::Ok(ContractResult::Ok(
        to_binary(&OwnerOfResponse {
            owner: creator.clone().to_string(),
            approvals: vec![],
        })
        .unwrap(),
    ));

    let mut mock = MockQuerierStruct::new();
    mock.expect_raw_query()
        .with(predicate::always())
        .times(1)
        .returning(move |_| return_value.clone());

    let querier_wrapper = QuerierWrapper::new(&mock);

    assert_eq!(
        Ok(OwnerOfResponse {
            owner: creator.to_string(),
            approvals: vec![]
        }),
        owner_of(&querier_wrapper, &collection, &token_id.to_string())
    );
}

#[test]
fn try_only_owner() {
    let bmt = base_minter_with_sg721(1);
    let collection = bmt.collection_response_vec[0].collection.clone().unwrap();

    let creator = bmt.accts.creator;
    let buyer = bmt.accts.buyer;

    let token_id = 1;

    mock! {
        QuerierStruct {}
        impl Querier for QuerierStruct {
            fn raw_query(&self, bin_request: &[u8]) -> QuerierResult;
        }
    }

    let return_value = SystemResult::Ok(ContractResult::Ok(
        to_binary(&OwnerOfResponse {
            owner: creator.clone().to_string(),
            approvals: vec![],
        })
        .unwrap(),
    ));

    let mut mock = MockQuerierStruct::new();
    mock.expect_raw_query()
        .with(predicate::always())
        .times(2)
        .returning(move |_| return_value.clone());

    let querier_wrapper = QuerierWrapper::new(&mock);

    let info = mock_info(&buyer.to_string(), &[]);
    assert_eq!(
        Err(StdError::generic_err("Unauthorized")),
        only_owner(&querier_wrapper, &info, &collection, &token_id.to_string())
    );

    let info = mock_info(&creator.to_string(), &[]);
    assert_eq!(
        Ok(()),
        only_owner(&querier_wrapper, &info, &collection, &token_id.to_string())
    );
}

#[test]
fn try_has_approval() {
    let bmt = base_minter_with_sg721(1);
    let collection = bmt.collection_response_vec[0].collection.clone().unwrap();
    let minter = bmt.collection_response_vec[0].minter.clone().unwrap();

    let token_id = 1;

    mock! {
        QuerierStruct {}
        impl Querier for QuerierStruct {
            fn raw_query(&self, bin_request: &[u8]) -> QuerierResult;
        }
    }

    let return_value = SystemResult::Ok(ContractResult::Err(
        StdError::generic_err("Approval not found").to_string(),
    ));

    let mut mock = MockQuerierStruct::new();
    mock.expect_raw_query()
        .with(predicate::always())
        .times(1)
        .returning(move |_| return_value.clone());

    let querier_wrapper = QuerierWrapper::new(&mock);

    assert_eq!(
        Err(StdError::generic_err(
            "Querier contract error: Generic error: Approval not found"
        )),
        has_approval(
            &querier_wrapper,
            &minter,
            &collection,
            &token_id.to_string(),
            None
        )
    );
}

#[test]
fn try_load_collection_royalties() {
    mock! {
        QuerierStruct {}
        impl Querier for QuerierStruct {
            fn raw_query(&self, bin_request: &[u8]) -> QuerierResult;
        }
    }

    let return_value = SystemResult::Ok(ContractResult::Ok(
        to_binary(&CollectionInfoResponse {
            creator: "creator".to_string(),
            description: "description".to_string(),
            image: "image".to_string(),
            external_link: None,
            explicit_content: None,
            start_trading_time: None,
            royalty_info: Some(RoyaltyInfoResponse {
                payment_address: "payment_address".to_string(),
                share: Decimal::percent(200),
            }),
        })
        .unwrap(),
    ));

    let mut mock = MockQuerierStruct::new();
    mock.expect_raw_query()
        .with(predicate::always())
        .times(1)
        .returning(move |_| return_value.clone());

    let querier_wrapper = QuerierWrapper::new(&mock);
    let deps = mock_dependencies();

    assert_eq!(
        Ok(Some(RoyaltyInfo {
            payment_address: Addr::unchecked("payment_address"),
            share: Decimal::percent(200),
        })),
        load_collection_royalties(&querier_wrapper, &deps.api, &Addr::unchecked("collection"))
    );
}

#[test]
fn try_calculate_nft_sale_fees() {
    let sale_price = Uint128::from(10_000u64);
    let trading_fee_percent = Decimal::percent(200u64);
    let seller = Addr::unchecked("seller");
    let finder = Addr::unchecked("finder");

    let finders_fee_bps = 300u64;
    let royalty_info = RoyaltyInfo {
        payment_address: Addr::unchecked("royalty"),
        share: Decimal::percent(5u64),
    };

    // Calculate correct fees with finders fee and royalties
    let fees = calculate_nft_sale_fees(
        sale_price,
        trading_fee_percent,
        seller.clone(),
        Some(finder.clone()),
        Some(finders_fee_bps),
        Some(royalty_info),
    )
    .unwrap();

    assert_eq!(
        fees,
        TransactionFees {
            fair_burn_fee: Uint128::from(200u128),
            finders_fee: Some(TokenPayment {
                coin: coin(300u128, NATIVE_DENOM),
                recipient: Addr::unchecked("finder"),
            }),
            royalty_fee: Some(TokenPayment {
                coin: coin(500u128, NATIVE_DENOM),
                recipient: Addr::unchecked("royalty"),
            }),
            seller_payment: TokenPayment {
                coin: coin(9_000u128, NATIVE_DENOM),
                recipient: Addr::unchecked("seller"),
            },
        }
    );

    // Calculate correct fees with no finders fee and no royalties
    let fees = calculate_nft_sale_fees(
        sale_price,
        trading_fee_percent,
        seller,
        Some(finder),
        None,
        None,
    )
    .unwrap();

    assert_eq!(
        fees,
        TransactionFees {
            fair_burn_fee: Uint128::from(200u128),
            finders_fee: None,
            royalty_fee: None,
            seller_payment: TokenPayment {
                coin: coin(9_800u128, NATIVE_DENOM),
                recipient: Addr::unchecked("seller"),
            },
        }
    );
}

#[test]
fn try_payout_nft_sale_fees() {
    let tx_fees = TransactionFees {
        fair_burn_fee: Uint128::from(200u128),
        finders_fee: Some(TokenPayment {
            coin: coin(300u128, NATIVE_DENOM),
            recipient: Addr::unchecked("finder"),
        }),
        royalty_fee: Some(TokenPayment {
            coin: coin(500u128, NATIVE_DENOM),
            recipient: Addr::unchecked("royalty"),
        }),
        seller_payment: TokenPayment {
            coin: coin(9_000u128, NATIVE_DENOM),
            recipient: Addr::unchecked("seller"),
        },
    };

    let developer = Addr::unchecked("developer");
    let response = Response::new();
    let response = payout_nft_sale_fees(response, tx_fees.clone(), Some(developer)).unwrap();

    match response.messages[0].msg.clone() {
        CosmosMsg::Bank(BankMsg::Send { to_address, .. }) => {
            assert_eq!(to_address, "developer");
        }
        _ => panic!("Unexpected message type"),
    }

    match response.messages[1].msg.clone() {
        CosmosMsg::Bank(BankMsg::Burn { .. }) => {}
        _ => panic!("Unexpected message type"),
    }

    match response.messages[2].msg.clone() {
        CosmosMsg::Custom(StargazeMsgWrapper { msg_data, .. }) => match msg_data {
            StargazeMsg::FundFairburnPool { .. } => {}
            _ => panic!("Unexpected message type"),
        },
        _ => panic!("Unexpected message type"),
    }

    match response.messages[3].msg.clone() {
        CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
            assert_eq!(to_address, "finder");
            assert_eq!(amount[0], tx_fees.finders_fee.unwrap().coin);
        }
        _ => panic!("Unexpected message type"),
    }

    match response.messages[4].msg.clone() {
        CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
            assert_eq!(to_address, "royalty");
            assert_eq!(amount[0], tx_fees.royalty_fee.unwrap().coin);
        }
        _ => panic!("Unexpected message type"),
    }

    match response.messages[5].msg.clone() {
        CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
            assert_eq!(to_address, "seller");
            assert_eq!(amount[0], tx_fees.seller_payment.coin);
        }
        _ => panic!("Unexpected message type"),
    }
}
