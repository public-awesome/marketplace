use crate::{
    nft::{only_owner, only_tradable, transfer_nft},
    MarketplaceStdError,
};

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    testing::{mock_env, mock_info},
    to_json_binary, Addr, Binary, ContractResult, Decimal, Querier, QuerierResult, QuerierWrapper,
    Response, SystemError, SystemResult, Timestamp,
};
use cw721::OwnerOfResponse;
use mockall::{mock, predicate};

#[cw_serde]
pub struct RoyaltyInfoResponse {
    pub payment_address: String,
    pub share: Decimal,
}

#[cw_serde]
pub struct CollectionInfoResponse {
    pub creator: String,
    pub description: String,
    pub image: String,
    pub external_link: Option<String>,
    pub explicit_content: Option<bool>,
    pub start_trading_time: Option<Timestamp>,
    pub royalty_info: Option<RoyaltyInfoResponse>,
}

mock! {
    pub QuerierStruct {}

    impl Querier for QuerierStruct {
        fn raw_query(&self, bin_request: &[u8]) -> QuerierResult;
    }
}

#[test]
fn try_transfer_nft() {
    let collection = Addr::unchecked("collection");
    let recipient = Addr::unchecked("recipient");
    transfer_nft(&collection, "1", &recipient, Response::new());
}

#[test]
fn try_only_owner() {
    let collection = Addr::unchecked("collection");
    let creator = Addr::unchecked("creator");
    let buyer = Addr::unchecked("buyer");

    let token_id = 1;

    let return_value = SystemResult::Ok(ContractResult::Ok(
        to_json_binary(&OwnerOfResponse {
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
        Err(MarketplaceStdError::Unauthorized(
            "sender is not owner".to_string()
        )),
        only_owner(&querier_wrapper, &info, &collection, &token_id.to_string())
    );

    let info = mock_info(creator.as_ref(), &[]);
    assert_eq!(
        Ok(()),
        only_owner(&querier_wrapper, &info, &collection, &token_id.to_string())
    );
}

#[test]
fn try_only_tradable() {
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
        to_json_binary(&CollectionInfoResponse {
            creator: "creator".to_string(),
            description: "description".to_string(),
            image: "image".to_string(),
            external_link: None,
            explicit_content: None,
            start_trading_time: None,
            royalty_info: None,
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
        to_json_binary(&CollectionInfoResponse {
            creator: "creator".to_string(),
            description: "description".to_string(),
            image: "image".to_string(),
            external_link: None,
            explicit_content: None,
            start_trading_time: Some(Timestamp::from_seconds(0)),
            royalty_info: None,
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
        to_json_binary(&CollectionInfoResponse {
            creator: "creator".to_string(),
            description: "description".to_string(),
            image: "image".to_string(),
            external_link: None,
            explicit_content: None,
            start_trading_time: Some(env.block.time.plus_seconds(1)),
            royalty_info: None,
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
