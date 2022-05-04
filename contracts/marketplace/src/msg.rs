use crate::{
    helpers::ExpiryRange,
    state::{Ask, Bid, CollectionBid, SaleType, SudoParams, TokenId},
};
use cosmwasm_std::{to_binary, Addr, Binary, Coin, StdResult, Timestamp, Uint128, WasmMsg};
use cw_utils::Duration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sg_std::CosmosMsg;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// Fair Burn fee for winning bids
    /// 0.25% = 25, 0.5% = 50, 1% = 100, 2.5% = 250
    pub trading_fee_bps: u64,
    /// Valid time range for Asks
    /// (min, max) in seconds
    pub ask_expiry: ExpiryRange,
    /// Valid time range for Bids
    /// (min, max) in seconds
    pub bid_expiry: ExpiryRange,
    /// Operators are entites that are responsible for maintaining the active state of Asks.
    /// They listen to NFT transfer events, and update the active state of Asks.
    pub operators: Vec<String>,
    pub sale_hook: Option<String>,
    /// Max basis points for the finders fee
    pub max_finders_fee_bps: u64,
    /// Min value for bids and asks
    pub min_price: Uint128,
    /// Duration after expiry when a bid becomes stale (in seconds)
    pub stale_bid_duration: Duration,
    /// Stale bid removal reward
    pub bid_removal_reward_bps: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// List an NFT on the marketplace by creating a new ask
    SetAsk {
        sale_type: SaleType,
        collection: String,
        token_id: TokenId,
        price: Coin,
        funds_recipient: Option<String>,
        reserve_for: Option<String>,
        finders_fee_bps: Option<u64>,
        expires: Timestamp,
    },
    /// Remove an existing ask from the marketplace
    RemoveAsk {
        collection: String,
        token_id: TokenId,
    },
    /// Priviledged operation to change the active state of an ask when an NFT is transferred
    UpdateAskIsActive {
        collection: String,
        token_id: TokenId,
        is_active: bool,
    },
    /// Update the price of an existing ask
    UpdateAskPrice {
        collection: String,
        token_id: TokenId,
        price: Coin,
    },
    /// Place a bid on an existing ask
    SetBid {
        collection: String,
        token_id: TokenId,
        expires: Timestamp,
        finder: Option<String>,
        finders_fee_bps: Option<u64>,
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
        finder: Option<String>,
    },
    /// Place a bid (limit order) across an entire collection
    SetCollectionBid {
        collection: String,
        expires: Timestamp,
        finders_fee_bps: Option<u64>,
    },
    /// Remove a bid (limit order) across an entire collection
    RemoveCollectionBid { collection: String },
    /// Accept a collection bid
    AcceptCollectionBid {
        collection: String,
        token_id: TokenId,
        bidder: String,
        finder: Option<String>,
    },
    /// Privileged operation to remove stale bids
    RemoveStaleBid {
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
        trading_fee_bps: Option<u64>,
        ask_expiry: Option<ExpiryRange>,
        bid_expiry: Option<ExpiryRange>,
        operators: Option<Vec<String>>,
        max_finders_fee_bps: Option<u64>,
        min_price: Option<Uint128>,
        stale_bid_duration: Option<u64>,
        bid_removal_reward_bps: Option<u64>,
    },
    /// Add a new hook to be informed of all asks
    AddAskCreatedHook { hook: String },
    /// Add a new hook to be informed of all bids
    AddBidCreatedHook { hook: String },
    /// Remove a ask hook
    RemoveAskCreatedHook { hook: String },
    /// Remove a bid hook
    RemoveBidCreatedHook { hook: String },
    /// Add a new hook to be informed of all trades
    AddSaleHook { hook: String },
    /// Remove a trade hook
    RemoveSaleHook { hook: String },
}

pub type Collection = String;
pub type Bidder = String;
pub type Seller = String;

/// Offset for ask pagination
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AskOffset {
    pub price: Uint128,
    pub token_id: TokenId,
}

impl AskOffset {
    pub fn new(price: Uint128, token_id: TokenId) -> Self {
        AskOffset { price, token_id }
    }
}

/// Offset for bid pagination
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BidOffset {
    pub price: Uint128,
    pub token_id: TokenId,
    pub bidder: Addr,
}

impl BidOffset {
    pub fn new(price: Uint128, token_id: TokenId, bidder: Addr) -> Self {
        BidOffset {
            price,
            token_id,
            bidder,
        }
    }
}
/// Offset for collection pagination
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CollectionOffset {
    pub collection: String,
    pub token_id: TokenId,
}

impl CollectionOffset {
    pub fn new(collection: String, token_id: TokenId) -> Self {
        CollectionOffset {
            collection,
            token_id,
        }
    }
}

/// Offset for collection bid pagination
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CollectionBidOffset {
    pub price: Uint128,
    pub collection: Collection,
    pub bidder: Bidder,
}

impl CollectionBidOffset {
    pub fn new(price: Uint128, collection: String, bidder: Bidder) -> Self {
        CollectionBidOffset {
            price,
            collection,
            bidder,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// List of collections that have asks on them
    /// Return type: `CollectionsResponse`
    Collections {
        start_after: Option<Collection>,
        limit: Option<u32>,
    },
    /// Get the current ask for specific NFT
    /// Return type: `CurrentAskResponse`
    Ask {
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
    /// Get all asks for a collection, sorted by price
    /// Return type: `AsksResponse`
    AsksSortedByPrice {
        collection: Collection,
        start_after: Option<AskOffset>,
        limit: Option<u32>,
    },
    /// Get all asks for a collection, sorted by price in reverse
    /// Return type: `AsksResponse`
    ReverseAsksSortedByPrice {
        collection: Collection,
        start_before: Option<AskOffset>,
        limit: Option<u32>,
    },
    /// Count of all asks
    /// Return type: `AskCountResponse`
    AskCount { collection: Collection },
    /// Get all asks by seller
    /// Return type: `AsksResponse`
    AsksBySeller {
        seller: Seller,
        start_after: Option<CollectionOffset>,
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
    BidsByBidder {
        bidder: Bidder,
        start_after: Option<CollectionOffset>,
        limit: Option<u32>,
    },
    /// Get all bids for a specific NFT
    /// Return type: `BidsResponse`
    Bids {
        collection: Collection,
        token_id: TokenId,
        start_after: Option<Bidder>,
        limit: Option<u32>,
    },
    /// Get all bids for a collection, sorted by price
    /// Return type: `BidsResponse`
    BidsSortedByPrice {
        collection: Collection,
        start_after: Option<BidOffset>,
        limit: Option<u32>,
    },
    /// Get all bids for a collection, sorted by price in reverse
    /// Return type: `BidsResponse`
    ReverseBidsSortedByPrice {
        collection: Collection,
        start_before: Option<BidOffset>,
        limit: Option<u32>,
    },
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
        start_after: Option<CollectionBidOffset>,
        limit: Option<u32>,
    },
    /// Get all collection bids for a collection sorted by price in reverse
    /// Return type: `CollectionBidsResponse`
    ReverseCollectionBidsSortedByPrice {
        collection: Collection,
        start_before: Option<CollectionBidOffset>,
        limit: Option<u32>,
    },
    /// Show all registered ask hooks
    /// Return type: `HooksResponse`
    AskCreatedHooks {},
    /// Show all registered bid hooks
    /// Return type: `HooksResponse`
    BidCreatedHooks {},
    /// Show all registered sale hooks
    /// Return type: `HooksResponse`
    SaleHooks {},
    /// Get the config for the contract
    /// Return type: `ParamsResponse`
    Params {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AskResponse {
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
pub struct SaleHookMsg {
    pub collection: String,
    pub token_id: u32,
    pub price: Coin,
    pub seller: String,
    pub buyer: String,
}

impl SaleHookMsg {
    pub fn new(
        collection: String,
        token_id: u32,
        price: Coin,
        seller: String,
        buyer: String,
    ) -> Self {
        SaleHookMsg {
            collection,
            token_id,
            price,
            seller,
            buyer,
        }
    }

    /// serializes the message
    pub fn into_binary(self) -> StdResult<Binary> {
        let msg = SaleExecuteMsg::SaleHook(self);
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
pub enum SaleExecuteMsg {
    SaleHook(SaleHookMsg),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct AskCreatedHookMsg {
    pub collection: String,
    pub token_id: u32,
    pub seller: String,
    pub funds_recipient: String,
    pub price: Coin,
}

impl AskCreatedHookMsg {
    pub fn new(
        collection: String,
        token_id: u32,
        seller: String,
        funds_recipient: String,
        price: Coin,
    ) -> Self {
        AskCreatedHookMsg {
            collection,
            token_id,
            seller,
            funds_recipient,
            price,
        }
    }

    /// serializes the message
    pub fn into_binary(self) -> StdResult<Binary> {
        let msg = AskCreatedExecuteMsg::AskCreatedHook(self);
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
pub enum AskCreatedExecuteMsg {
    AskCreatedHook(AskCreatedHookMsg),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct BidCreatedHookMsg {
    pub collection: String,
    pub token_id: TokenId,
    pub bidder: String,
    pub price: Uint128,
}

impl BidCreatedHookMsg {
    pub fn new(collection: String, token_id: TokenId, bidder: String, price: Uint128) -> Self {
        BidCreatedHookMsg {
            collection,
            token_id,
            bidder,
            price,
        }
    }

    /// serializes the message
    pub fn into_binary(self) -> StdResult<Binary> {
        let msg = BidCreatedExecuteMsg::BidCreatedHook(self);
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
pub enum BidCreatedExecuteMsg {
    BidCreatedHook(BidCreatedHookMsg),
}
