use cosmwasm_std::Addr;
use sg2::msg::CollectionParams;
use sg_multi_test::StargazeApp;

pub struct SetupContractsParams<'a> {
    pub minter_admin: Addr,
    pub collection_params_vec: Vec<CollectionParams>,
    pub num_tokens: u32,
    pub router: &'a mut StargazeApp,
}

pub struct MinterSetupParams<'a> {
    pub router: &'a mut StargazeApp,
    pub minter_admin: Addr,
    pub num_tokens: u32,
    pub collection_params: CollectionParams,
    pub splits_addr: Option<String>,
    pub minter_code_id: u64,
    pub factory_code_id: u64,
    pub sg721_code_id: u64,
}
pub struct MinterCollectionResponse {
    pub minter: Addr,
    pub collection: Addr,
}
