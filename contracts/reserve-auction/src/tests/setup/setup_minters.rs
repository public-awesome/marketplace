use crate::tests::setup::msg::Accounts;
use crate::tests::setup::setup_accounts::setup_accounts;
use cosmwasm_std::Timestamp;
use sg2::msg::CollectionParams;
use sg2::tests::mock_collection_params_1;
use sg721::CollectionInfo;
use sg_std::GENESIS_MINT_START_TIME;
use test_suite::common_setup::{
    contract_boxes::custom_mock_app,
    msg::{MinterCollectionResponse, MinterTemplateResponse},
    setup_minter::{
        common::minter_params::minter_params_token,
        vending_minter::setup::{configure_minter, vending_minter_code_ids},
    },
};

pub fn standard_minter_template(num_tokens: u32) -> MinterTemplateResponse<Accounts> {
    let mut app = custom_mock_app();
    let (owner, bidder, creator) = setup_accounts(&mut app).unwrap();
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    let collection_params = mock_collection_params_1(Some(start_time));
    let minter_params = minter_params_token(num_tokens);
    let code_ids = vending_minter_code_ids(&mut app);
    let minter_collection_response: Vec<MinterCollectionResponse> = configure_minter(
        &mut app,
        creator.clone(),
        vec![collection_params],
        vec![minter_params],
        code_ids,
    );
    MinterTemplateResponse {
        router: app,
        collection_response_vec: minter_collection_response,
        accts: Accounts {
            owner,
            bidder,
            creator,
        },
    }
}

pub fn minter_template_no_royalties(num_tokens: u32) -> MinterTemplateResponse<Accounts> {
    let mut app = custom_mock_app();
    let (owner, bidder, creator) = setup_accounts(&mut app).unwrap();
    let start_trading_time = Some(Timestamp::from_nanos(GENESIS_MINT_START_TIME));
    let collection_params = CollectionParams {
        code_id: 1,
        name: "Collection Name".to_string(),
        symbol: "COL".to_string(),
        info: CollectionInfo {
            creator: "creator".to_string(),
            description: String::from("Stargaze Monkeys"),
            image: "https://example.com/image.png".to_string(),
            external_link: Some("https://example.com/external.html".to_string()),
            start_trading_time,
            explicit_content: Some(false),
            royalty_info: None,
        },
    };
    let minter_params = minter_params_token(num_tokens);
    let code_ids = vending_minter_code_ids(&mut app);
    let minter_collection_response: Vec<MinterCollectionResponse> = configure_minter(
        &mut app,
        creator.clone(),
        vec![collection_params],
        vec![minter_params],
        code_ids,
    );
    MinterTemplateResponse {
        router: app,
        collection_response_vec: minter_collection_response,
        accts: Accounts {
            owner,
            bidder,
            creator,
        },
    }
}
