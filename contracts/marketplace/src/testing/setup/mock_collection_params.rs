use cosmwasm_std::{Decimal, Timestamp};
use sg721::{CollectionInfo, RoyaltyInfoResponse};

use sg2::msg::CollectionParams;
use sg2::tests::mock_collection_params;

pub fn mock_collection_params_1(start_trading_time: Option<Timestamp>) -> CollectionParams {
    CollectionParams {
        info: CollectionInfo {
            start_trading_time,
            ..mock_collection_params().info
        },
        ..mock_collection_params()
    }
}

pub fn mock_curator_payment_address(start_trading_time: Option<Timestamp>) -> CollectionParams {
    CollectionParams {
        info: CollectionInfo {
            royalty_info: Some(RoyaltyInfoResponse {
                payment_address: "curator".to_string(),
                share: Decimal::percent(10),
            }),
            start_trading_time,
            ..mock_collection_params().info
        },
        ..mock_collection_params()
    }
}

pub fn mock_collection_params_high_fee(start_trading_time: Option<Timestamp>) -> CollectionParams {
    CollectionParams {
        info: CollectionInfo {
            royalty_info: Some(RoyaltyInfoResponse {
                payment_address: "creator".to_string(),
                share: Decimal::percent(100),
            }),
            start_trading_time,
            ..mock_collection_params().info
        },
        ..mock_collection_params()
    }
}

pub fn mock_collection_two(start_trading_time: Option<Timestamp>) -> CollectionParams {
    CollectionParams {
        info: CollectionInfo {
            start_trading_time,
            ..mock_collection_params().info
        },
        ..mock_collection_params()
    }
}
