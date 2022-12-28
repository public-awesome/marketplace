use crate::testing::setup::msg::MarketAccounts;
use crate::testing::setup::setup_accounts::setup_accounts;
use cosmwasm_std::{Addr, Timestamp};
use sg2::tests::{
    mock_collection_params_1, mock_collection_params_high_fee, mock_collection_two,
    mock_curator_payment_address,
};
use sg_std::GENESIS_MINT_START_TIME;
use test_suite::common_setup::{
    contract_boxes::custom_mock_app,
    msg::{MinterCollectionResponse, VendingTemplateResponse},
    setup_minter::{
        common::minter_params::minter_params_token,
        vending_minter::setup::{configure_minter, vending_minter_code_ids},
    },
};

use crate::testing::setup::msg::MarketplaceTemplateResponse;
use crate::testing::setup::setup_marketplace::setup_marketplace;

pub fn standard_minter_template(num_tokens: u32) -> VendingTemplateResponse<MarketAccounts> {
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
    VendingTemplateResponse {
        router: app,
        collection_response_vec: minter_collection_response,
        accts: MarketAccounts {
            owner,
            bidder,
            creator,
        },
    }
}

pub fn minter_template_high_fee(num_tokens: u32) -> VendingTemplateResponse<MarketAccounts> {
    let mut app = custom_mock_app();
    let (owner, bidder, creator) = setup_accounts(&mut app).unwrap();
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    let collection_params = mock_collection_params_high_fee(Some(start_time));
    let minter_params = minter_params_token(num_tokens);
    let code_ids = vending_minter_code_ids(&mut app);
    let minter_collection_response: Vec<MinterCollectionResponse> = configure_minter(
        &mut app,
        creator.clone(),
        vec![collection_params],
        vec![minter_params],
        code_ids,
    );
    VendingTemplateResponse {
        router: app,
        collection_response_vec: minter_collection_response,
        accts: MarketAccounts {
            owner,
            bidder,
            creator,
        },
    }
}

pub fn minter_template_owner_admin(num_tokens: u32) -> VendingTemplateResponse<MarketAccounts> {
    let mut app = custom_mock_app();
    let (owner, bidder, creator) = setup_accounts(&mut app).unwrap();
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    let collection_params = mock_collection_params_1(Some(start_time));
    let minter_params = minter_params_token(num_tokens);
    let code_ids = vending_minter_code_ids(&mut app);
    let minter_collection_response: Vec<MinterCollectionResponse> = configure_minter(
        &mut app,
        owner.clone(),
        vec![collection_params],
        vec![minter_params],
        code_ids,
    );
    VendingTemplateResponse {
        router: app,
        collection_response_vec: minter_collection_response,
        accts: MarketAccounts {
            owner,
            bidder,
            creator,
        },
    }
}

pub fn minter_with_curator(num_tokens: u32) -> VendingTemplateResponse<MarketAccounts> {
    let mut app = custom_mock_app();
    let (owner, bidder, creator) = setup_accounts(&mut app).unwrap();
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    let collection_params = mock_curator_payment_address(Some(start_time));
    let minter_params = minter_params_token(num_tokens);
    let code_ids = vending_minter_code_ids(&mut app);
    let minter_collection_response: Vec<MinterCollectionResponse> = configure_minter(
        &mut app,
        creator.clone(),
        vec![collection_params],
        vec![minter_params],
        code_ids,
    );
    VendingTemplateResponse {
        router: app,
        collection_response_vec: minter_collection_response,
        accts: MarketAccounts {
            owner,
            bidder,
            creator,
        },
    }
}

pub fn minter_two_collections(num_tokens: u32) -> VendingTemplateResponse<MarketAccounts> {
    let mut app = custom_mock_app();
    let (owner, bidder, creator) = setup_accounts(&mut app).unwrap();
    let start_time = Timestamp::from_nanos(GENESIS_MINT_START_TIME);
    let collection_params = mock_collection_params_1(Some(start_time));
    let collection_params_two = mock_collection_two(Some(start_time));
    let minter_params = minter_params_token(num_tokens);
    let minter_params_two = minter_params_token(num_tokens);
    let code_ids = vending_minter_code_ids(&mut app);
    let minter_collection_response: Vec<MinterCollectionResponse> = configure_minter(
        &mut app,
        creator.clone(),
        vec![collection_params, collection_params_two],
        vec![minter_params, minter_params_two],
        code_ids,
    );
    VendingTemplateResponse {
        router: app,
        collection_response_vec: minter_collection_response,
        accts: MarketAccounts {
            owner,
            bidder,
            creator,
        },
    }
}

pub fn minter_two_collections_with_time(
    num_tokens: u32,
    time_one: Timestamp,
    time_two: Timestamp,
) -> VendingTemplateResponse<MarketAccounts> {
    let mut app = custom_mock_app();
    let (owner, bidder, creator) = setup_accounts(&mut app).unwrap();
    let collection_params = mock_collection_params_1(Some(time_one));
    let collection_params_two = mock_collection_two(Some(time_two));
    let minter_params = minter_params_token(num_tokens);
    let minter_params_two = minter_params_token(num_tokens);
    let code_ids = vending_minter_code_ids(&mut app);
    let minter_collection_response: Vec<MinterCollectionResponse> = configure_minter(
        &mut app,
        creator.clone(),
        vec![collection_params, collection_params_two],
        vec![minter_params, minter_params_two],
        code_ids,
    );
    VendingTemplateResponse {
        router: app,
        collection_response_vec: minter_collection_response,
        accts: MarketAccounts {
            owner,
            bidder,
            creator,
        },
    }
}
