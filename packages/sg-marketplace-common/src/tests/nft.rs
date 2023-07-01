use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    to_binary, Addr, Binary, ContractResult, Decimal, Querier, QuerierResult, QuerierWrapper,
    StdError, SystemError, SystemResult, Timestamp,
};
use cw721::OwnerOfResponse;
use mockall::*;
use sg721::{RoyaltyInfo, RoyaltyInfoResponse};
use sg721_base::msg::CollectionInfoResponse;
use test_suite::common_setup::templates::base_minter_with_sg721;

use crate::nft::{
    has_approval, load_collection_royalties, only_owner, only_tradable, owner_of, transfer_nft,
};

#[test]
fn try_transfer_nft() {
    let collection = Addr::unchecked("collection");
    let recipient = Addr::unchecked("recipient");
    transfer_nft(&collection, "1", &recipient);
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
            owner: creator.to_string(),
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
            owner: creator.to_string(),
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

    let info = mock_info(buyer.as_ref(), &[]);
    assert_eq!(
        Err(StdError::generic_err("Unauthorized")),
        only_owner(&querier_wrapper, &info, &collection, &token_id.to_string())
    );

    let info = mock_info(creator.as_ref(), &[]);
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
fn try_only_tradable() {
    mock! {
        QuerierStruct {}
        impl Querier for QuerierStruct {
            fn raw_query(&self, bin_request: &[u8]) -> QuerierResult;
        }
    }

    let collection = Addr::unchecked("collection");

    let env = mock_env();
    let mut mock = MockQuerierStruct::new();

    // CollectionInfo query error response is ok
    let return_value = SystemResult::Err(SystemError::InvalidResponse {
        error: "error".to_string(),
        response: Binary::from_base64("").unwrap(),
    });
    mock.expect_raw_query()
        .with(predicate::always())
        .times(1)
        .returning(move |_| return_value.clone());

    let querier_wrapper = QuerierWrapper::new(&mock);

    assert_eq!(
        Ok(()),
        only_tradable(&querier_wrapper, &env.block, &collection)
    );

    // No start trading time set is ok
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
    mock.expect_raw_query()
        .with(predicate::always())
        .times(1)
        .returning(move |_| return_value.clone());

    let querier_wrapper = QuerierWrapper::new(&mock);

    assert_eq!(
        Ok(()),
        only_tradable(&querier_wrapper, &env.block, &collection)
    );

    // Start trading time in the past is ok
    let return_value = SystemResult::Ok(ContractResult::Ok(
        to_binary(&CollectionInfoResponse {
            creator: "creator".to_string(),
            description: "description".to_string(),
            image: "image".to_string(),
            external_link: None,
            explicit_content: None,
            start_trading_time: Some(Timestamp::from_seconds(0)),
            royalty_info: Some(RoyaltyInfoResponse {
                payment_address: "payment_address".to_string(),
                share: Decimal::percent(200),
            }),
        })
        .unwrap(),
    ));
    mock.expect_raw_query()
        .with(predicate::always())
        .times(1)
        .returning(move |_| return_value.clone());

    let querier_wrapper = QuerierWrapper::new(&mock);

    assert_eq!(
        Ok(()),
        only_tradable(&querier_wrapper, &env.block, &collection)
    );

    // Start trading time in the future throws an error
    let return_value = SystemResult::Ok(ContractResult::Ok(
        to_binary(&CollectionInfoResponse {
            creator: "creator".to_string(),
            description: "description".to_string(),
            image: "image".to_string(),
            external_link: None,
            explicit_content: None,
            start_trading_time: Some(env.block.time.plus_seconds(1)),
            royalty_info: Some(RoyaltyInfoResponse {
                payment_address: "payment_address".to_string(),
                share: Decimal::percent(200),
            }),
        })
        .unwrap(),
    ));
    mock.expect_raw_query()
        .with(predicate::always())
        .times(1)
        .returning(move |_| return_value.clone());

    let querier_wrapper = QuerierWrapper::new(&mock);

    assert_eq!(
        Err(crate::MarketplaceStdError::CollectionNotTradable {}),
        only_tradable(&querier_wrapper, &env.block, &collection)
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
