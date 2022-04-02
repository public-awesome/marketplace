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
        token_id: u32,
    },
    RemoveBid {
        collection: String,
        token_id: u32,
    },
    SetAsk {
        collection: String,
        token_id: u32,
        price: Coin,
        funds_recipient: Option<String>,
    },
    RemoveAsk {
        collection: String,
        token_id: u32,
    },
    AcceptBid {
        collection: String,
        token_id: u32,
        bidder: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    CurrentAsk {
        collection: String,
        token_id: u32,
    },
    Bid {
        collection: String,
        token_id: u32,
        bidder: String,
    },
    Bids {
        collection: String,
        token_id: u32,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[serde(rename = "all_listed_nfts")]
    AllListedNFTs {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[serde(rename = "all_listed_nfts_in_collection")]
    AllListedNFTsInCollection {
        collection: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Bid {
    pub price: Coin,
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
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Nft {
    pub collection: String,
    pub token_id: u32,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ListedNftsResponse {
    pub nfts: Vec<Nft>,
}
