use std::str::FromStr;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Timestamp};
use stargaze_royalty_registry::state::RoyaltyEntry as RegistryRoyaltyEntry;

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

#[cw_serde]
pub struct RoyaltyEntry {
    /// The address that will receive the royalty payments
    pub recipient: Addr,
    /// The percentage of sales that should be paid to the recipient
    pub share: Decimal,
    /// The last time the royalty entry was updated
    pub updated: Option<Timestamp>,
}

impl From<RegistryRoyaltyEntry> for RoyaltyEntry {
    fn from(e: RegistryRoyaltyEntry) -> Self {
        Self {
            recipient: Addr::unchecked(e.recipient),
            share: Decimal::from_str(&e.share.to_string()).unwrap(),
            updated: e.updated.map(|t| Timestamp::from_nanos(t.nanos())),
        }
    }
}

#[cw_serde]
pub enum Sg721QueryMsg {
    CollectionInfo {},
}
