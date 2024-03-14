use crate::testing::setup::{
    msg::MarketAccounts,
    setup_accounts::setup_accounts,
    setup_contracts::{setup_fair_burn, setup_royalty_registry},
    setup_marketplace::setup_marketplace,
};

use cosmwasm_std::{Addr, Timestamp};
use sg2::tests::mock_collection_params_1;
use sg_std::GENESIS_MINT_START_TIME;
use test_suite::common_setup::{
    contract_boxes::custom_mock_app,
    msg::{MinterCollectionResponse, MinterTemplateResponse},
    setup_minter::{
        common::minter_params::minter_params_token,
        vending_minter::setup::{configure_minter, vending_minter_code_ids},
    },
};

pub fn standard_minter_template(num_tokens: u32) -> MinterTemplateResponse<MarketAccounts> {
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
        accts: MarketAccounts {
            owner,
            bidder,
            creator,
        },
    }
}

pub struct TestContracts {
    pub collection: Addr,
    pub minter: Addr,
    pub marketplace: Addr,
    pub fair_burn: Addr,
    pub royalty_registry: Addr,
}

pub struct MarketplaceV2Template {
    pub minter_template: MinterTemplateResponse<MarketAccounts>,
    pub contracts: TestContracts,
}

pub fn marketplace_v2_template(num_tokens: u32) -> MarketplaceV2Template {
    let mut vt = standard_minter_template(num_tokens);
    let fair_burn = setup_fair_burn(&mut vt.router, &vt.accts.creator).unwrap();
    let royalty_registry = setup_royalty_registry(&mut vt.router, &vt.accts.creator).unwrap();
    let marketplace = setup_marketplace(
        &mut vt.router,
        fair_burn.clone(),
        royalty_registry.clone(),
        vt.accts.creator.clone(),
    )
    .unwrap();

    let collection = vt.collection_response_vec[0].collection.clone().unwrap();
    let minter = vt.collection_response_vec[0].minter.clone().unwrap();

    MarketplaceV2Template {
        minter_template: vt,
        contracts: TestContracts {
            collection,
            minter,
            fair_burn,
            royalty_registry,
            marketplace,
        },
    }
}
