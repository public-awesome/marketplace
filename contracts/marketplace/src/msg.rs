use crate::state::{Ask, Bid, CollectionBid, SudoParams, TokenId};
use cosmwasm_std::{to_binary, Addr, Binary, Coin, StdResult, Timestamp, WasmMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sg_std::CosmosMsg;

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
    /// Add a new hook to be informed of all trades
    AddSaleFinalizedHook { hook: String },
    /// Remove a hook
    RemoveSaleFinalizedHook { hook: String },
}

pub type Collection = String;
pub type Bidder = String;
pub type Seller = String;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Get the current ask for specific NFT
    /// Return type: `CurrentAskResponse`
    CurrentAsk {
        collection: Collection,
        token_id: TokenId,
    },
    /// Get all asks for a collection
    /// Return type: `AsksResponse`
    Asks {
        collection: Collection,
        start_after: Option<TokenId>,
        limit: Option<u32>,
    },
    /// Get all asks for a collection sorted by price
    /// Return type: `AsksResponse`
    AsksSortedByPrice {
        collection: Collection,
        limit: Option<u32>,
        order_asc: bool,
    },
    /// Count of all asks
    /// Return type: `AskCountResponse`
    AskCount { collection: Collection },
    /// Get all asks by seller
    /// Return type: `AsksResponse`
    AsksBySeller { seller: Seller },
    /// List of collections that have asks on them
    /// Return type: `CollectionsResponse`
    ListedCollections {
        start_after: Option<Collection>,
        limit: Option<u32>,
    },
    /// Get data for a specific bid
    /// Return type: `BidResponse`
    Bid {
        collection: Collection,
        token_id: TokenId,
        bidder: Bidder,
    },
    /// Get all bids by a bidder
    /// Return type: `BidsResponse`
    BidsByBidder { bidder: Bidder },
    /// Get all bids for a specific NFT
    /// Return type: `BidsResponse`
    Bids {
        collection: Collection,
        token_id: TokenId,
        start_after: Option<Bidder>,
        limit: Option<u32>,
    },
    /// Get all bids for a collection sorted by price
    /// Return type: `BidsResponse`
    BidsSortedByPrice {
        collection: Collection,
        limit: Option<u32>,
        order_asc: bool,
    },
    /// Get the config for the contract
    /// Return type: `ParamsResponse`
    Params {},
    /// Get data for a specific collection bid
    /// Return type: `CollectionBidResponse`
    CollectionBid {
        collection: Collection,
        bidder: Bidder,
    },
    /// Get all collection bids by a bidder
    /// Return type: `CollectionBidsResponse`
    CollectionBidsByBidder { bidder: Bidder },
    /// Get all collection bids for a collection sorted by price
    /// Return type: `CollectionBidsResponse`
    CollectionBidsSortedByPrice {
        collection: Collection,
        limit: Option<u32>,
        order_asc: bool,
    },
    /// Show all registered sale finalized hooks
    /// Return type: `HooksResponse`
    SaleFinalizedHooks {},
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
pub struct ParamsResponse {
    pub params: SudoParams,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CollectionBidResponse {
    pub bid: Option<CollectionBid>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CollectionBidsResponse {
    pub bids: Vec<CollectionBid>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct SaleFinalizedHookMsg {
    pub collection: String,
    pub token_id: u32,
    pub price: Coin,
    pub seller: String,
    pub buyer: String,
}

impl SaleFinalizedHookMsg {
    pub fn new(
        collection: String,
        token_id: u32,
        price: Coin,
        seller: String,
        buyer: String,
    ) -> Self {
        SaleFinalizedHookMsg {
            collection,
            token_id,
            price,
            seller,
            buyer,
        }
    }

    /// serializes the message
    pub fn into_binary(self) -> StdResult<Binary> {
        let msg = SaleFinalizedExecuteMsg::SaleFinalizedHook(self);
        to_binary(&msg)
    }

    /// creates a cosmos_msg sending this struct to the named contract
    pub fn into_cosmos_msg<T: Into<String>>(self, contract_addr: T) -> StdResult<CosmosMsg> {
        let msg = self.into_binary()?;
        let execute = WasmMsg::Execute {
            contract_addr: contract_addr.into(),
            msg,
            funds: vec![],
        };
        Ok(execute.into())
    }
}

// This is just a helper to properly serialize the above message
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum SaleFinalizedExecuteMsg {
    SaleFinalizedHook(SaleFinalizedHookMsg),
}
