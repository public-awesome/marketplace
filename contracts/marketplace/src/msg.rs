use crate::state::Ask;
use cosmwasm_std::Coin;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    SetBid {
        collection: String,
        token_id: String,
    },
    RemoveBid {
        collection: String,
        token_id: String,
    },
    SetAsk {
        collection: String,
        token_id: String,
        amount: Coin,
    },
    RemoveAsk {
        collection: String,
        token_id: String,
    },
    AcceptBid {
        collection: String,
        token_id: String,
        bidder: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    CurrentAsk {
        collection: String,
        token_id: String,
    },
    Bid {
        collection: String,
        token_id: String,
        bidder: String,
    },
    Bids {
        collection: String,
        token_id: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CurrentAskResponse {
    pub ask: Option<Ask>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BidResponse {
    pub bid_info: Option<BidInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BidsResponse {
    pub bid_infos: Vec<BidInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BidInfo {
    pub price: Coin,
    pub bidder: String,
}
