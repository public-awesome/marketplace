use cosmwasm_std::{Decimal, Timestamp, Uint128};
use sg721::{CollectionInfo, RoyaltyInfoResponse};

use sg2::msg::CollectionParams;

pub fn mock_collection_params_1() -> CollectionParams {
    CollectionParams {
        code_id: 1,
        name: "Collection Name".to_string(),
        symbol: "COL".to_string(),
        info: CollectionInfo {
            creator: "creator".to_string(),
            description: String::from("Stargaze Monkeys"),
            image: "https://example.com/image.png".to_string(),
            external_link: Some("https://example.com/external.html".to_string()),
            start_trading_time: None,
            explicit_content: Some(false),
            royalty_info: Some(RoyaltyInfoResponse {
                payment_address: "creator".to_string(),
                share: Decimal::percent(10),
            }),
        },
    }
}

// pub fn mock_collection_for_owner() -> CollectionParams {
//     CollectionInfoResponse {
//         creator: "creator".to_string(),
//         description: "Stargaze Monkeys".to_string(),
//         image: "https://example.com/image.png".to_string(),
//         external_link: None,
//         explicit_content: Some(false),
//         start_trading_time: Some(Timestamp::from_nanos(1647637200000000000)),
//         royalty_info: Some(RoyaltyInfoResponse {
//             payment_address: "creator".to_string(),
//             share: Decimal::new(Uint128::new(100000000000000000)),
//         }),
//     }
// }

pub fn mock_collection_params_high_fee() -> CollectionParams {
    CollectionParams {
        code_id: 1,
        name: String::from("Test Coin"),
        symbol: String::from("TEST"),
        info: CollectionInfo {
            creator: "creator".to_string(),
            description: String::from("Stargaze Monkeys"),
            image:
                "ipfs://bafybeigi3bwpvyvsmnbj46ra4hyffcxdeaj6ntfk5jpic5mx27x6ih2qvq/images/1.png"
                    .to_string(),
            external_link: Some("https://example.com/external.html".to_string()),
            royalty_info: Some(RoyaltyInfoResponse {
                payment_address: "creator".to_string(),
                share: Decimal::percent(100),
            }),
            start_trading_time: None,
            explicit_content: None
        },
    }
    
}
