use cosmwasm_std::{Addr, Timestamp};
use test_suite::common_setup::msg::VendingTemplateResponse;

pub struct MarketAccounts {
    pub owner: Addr,
    pub bidder: Addr,
    pub creator: Addr,
}

pub struct MarketplaceTemplateResponse {
    pub marketplace: Addr,
    pub vending_minter_response: VendingTemplateResponse<MarketAccounts>,
    pub minter_addr: Addr,
    pub collection: Addr,
    pub start_time: Timestamp,
}
