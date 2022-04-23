use crate::state::{Ask, Bid, SudoParams, TokenId};
use cosmwasm_std::{Addr, Coin, Timestamp};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub trading_fee_percent: u32,
    pub ask_expiry: (u64, u64),
    pub bid_expiry: (u64, u64),
    /// Operators are entites that are responsible for maintaining the active state of Asks.
    /// They listen to NFT transfer events, and update the active state of Asks.
    pub operators: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// List an NFT on the marketplace by creating a new ask
    SetAsk {
        collection: String,
        token_id: TokenId,
        price: Coin,
        funds_recipient: Option<String>,
        expires: Timestamp,
    },
    /// Remove an existing ask from the marketplace
    RemoveAsk {
        collection: String,
        token_id: TokenId,
    },
    /// Admin operation to change the active state of an ask when an NFT is transferred
    UpdateAskState {
        collection: String,
        token_id: TokenId,
        active: bool,
    },
    /// Update the price of an existing ask
    UpdateAsk {
        collection: String,
        token_id: TokenId,
        price: Coin,
    },
    /// Place a bid on an existing ask
    SetBid {
        collection: String,
        token_id: TokenId,
        expires: Timestamp,
    },
    /// Remove an existing bid from an ask
    RemoveBid {
        collection: String,
        token_id: TokenId,
    },
    /// Accept a bid on an existing ask
    AcceptBid {
        collection: String,
        token_id: TokenId,
        bidder: String,
    },
    /// Place a bid (limit order) across an entire collection
    SetCollectionBid {
        collection: String,
        expires: Timestamp,
    },
    /// Accept a collection bid
    AcceptCollectionBid {
        collection: String,
        token_id: TokenId,
        bidder: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SudoMsg {
    /// Update the contract parameters
    /// Can only be called by governance
    UpdateParams {
        trading_fee_percent: Option<u32>,
        ask_expiry: Option<(u64, u64)>,
        bid_expiry: Option<(u64, u64)>,
        operators: Option<Vec<String>>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Get the current ask for specific NFT
    /// Return type: `CurrentAskResponse`
    CurrentAsk {
        collection: String,
        token_id: TokenId,
    },
    /// Get all asks for a collection
    /// Return type: `AsksResponse`
    Asks {
        collection: String,
        start_after: Option<TokenId>,
        limit: Option<u32>,
    },
    /// Get all asks for a collection sorted by price
    /// Return type: `AsksResponse`
    AsksSortedByPrice {
        collection: String,
        limit: Option<u32>,
    },
    /// Count of all asks
    /// Return type: `AskCountResponse`
    AskCount { collection: String },
    /// Get all asks by seller
    /// Return type: `AsksResponse`
    AsksBySeller { seller: String },
    /// List of collections that have asks on them
    /// Return type: `CollectionsResponse`
    ListedCollections {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Get data for a specific bid
    /// Return type: `BidResponse`
    Bid {
        collection: String,
        token_id: TokenId,
        bidder: String,
    },
    /// Get all bids by a bidder
    /// Return type: `BidsResponse`
    BidsByBidder { bidder: String },
    /// Get all bids for a specific NFT
    /// Return type: `BidsResponse`
    Bids {
        collection: String,
        token_id: TokenId,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Get all bids for a collection sorted by price
    /// Return type: `BidsResponse`
    BidsSortedByPrice {
        collection: String,
        limit: Option<u32>,
    },
    /// Get the config for the contract
    /// Return type: `ParamResponse`
    Params {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CurrentAskResponse {
    pub ask: Option<Ask>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AsksResponse {
    pub asks: Vec<Ask>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AskCountResponse {
    pub count: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CollectionsResponse {
    pub collections: Vec<Addr>,
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
pub struct ParamResponse {
    pub params: SudoParams,
}
