use crate::state::{Ask, Bid};
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
        bid: Bid,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Returns the current asking price for a token
    CurrentAsk {
        collection: String,
        token_id: String,
    },
    /// Returns the bid for a token / bidder
    Bid {
        collection: String,
        token_id: String,
        bidder: String,
    },
    /// Returns list of bids for token
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
    pub bid: Option<Bid>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BidsResponse {
    pub bids: Vec<Bid>,
}
