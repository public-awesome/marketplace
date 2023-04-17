use crate::{
    helpers::ExpiryRange,
    state::{Ask, Bid, CollectionBid, SaleType, SudoParams, TokenId},
};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_binary, Addr, Binary, Coin, StdResult, Timestamp, Uint128};
use cw_utils::Duration;
use sg_controllers::HooksResponse;

#[cw_serde]
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
    /// The address of the airdrop claim contract to detect sales
    pub sale_hook: Option<String>,
    /// Max basis points for the finders fee
    pub max_finders_fee_bps: u64,
    /// Min value for bids and asks
    pub min_price: Uint128,
    /// Duration after expiry when a bid becomes stale (in seconds)
    pub stale_bid_duration: Duration,
    /// Stale bid removal reward
    pub bid_removal_reward_bps: u64,
    /// Listing fee to reduce spam
    pub listing_fee: Uint128,
}

#[cw_serde]
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
        sale_type: SaleType,
        finder: Option<String>,
        finders_fee_bps: Option<u64>,
    },
    /// Place multiple bids
    SetBids {
        collection: String,
        token_ids: Vec<TokenId>,
        expires: Timestamp,
        finder: Option<String>,
        finders_fee_bps: Option<u64>,
    },
    BuyNow {
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
    /// Privileged operation to change the active state of an ask when an NFT is transferred
    SyncAsk {
        collection: String,
        token_id: TokenId,
    },
    /// Privileged operation to remove stale or invalid asks.
    RemoveStaleAsk {
        collection: String,
        token_id: TokenId,
    },
    /// Privileged operation to remove stale bids
    RemoveStaleBid {
        collection: String,
        token_id: TokenId,
        bidder: String,
    },
    /// Privileged operation to remove stale collection bids
    RemoveStaleCollectionBid { collection: String, bidder: String },
}

#[cw_serde]
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
        listing_fee: Option<Uint128>,
    },
    /// Add a new operator
    AddOperator { operator: String },
    /// Remove operator
    RemoveOperator { operator: String },
    /// Add a new hook to be informed of all asks
    AddAskHook { hook: String },
    /// Add a new hook to be informed of all bids
    AddBidHook { hook: String },
    /// Remove a ask hook
    RemoveAskHook { hook: String },
    /// Remove a bid hook
    RemoveBidHook { hook: String },
    /// Add a new hook to be informed of all trades
    AddSaleHook { hook: String },
    /// Remove a trade hook
    RemoveSaleHook { hook: String },
}

pub type Collection = String;
pub type Bidder = String;
pub type Seller = String;

/// Offset for ask pagination
#[cw_serde]
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
#[cw_serde]
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
#[cw_serde]
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
#[cw_serde]
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

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// List of collections that have asks on them
    #[returns(CollectionsResponse)]
    Collections {
        start_after: Option<Collection>,
        limit: Option<u32>,
    },
    /// Get the current ask for specific NFT
    #[returns(AsksResponse)]
    Ask {
        collection: Collection,
        token_id: TokenId,
    },
    /// Get all asks for a collection
    #[returns(AsksResponse)]
    Asks {
        collection: Collection,
        include_inactive: Option<bool>,
        start_after: Option<TokenId>,
        limit: Option<u32>,
    },
    /// Get all asks for a collection in reverse
    #[returns(AsksResponse)]
    ReverseAsks {
        collection: Collection,
        include_inactive: Option<bool>,
        start_before: Option<TokenId>,
        limit: Option<u32>,
    },
    /// Get all asks for a collection, sorted by price
    #[returns(AsksResponse)]
    AsksSortedByPrice {
        collection: Collection,
        include_inactive: Option<bool>,
        start_after: Option<AskOffset>,
        limit: Option<u32>,
    },
    /// Get all asks for a collection, sorted by price in reverse
    #[returns(AsksResponse)]
    ReverseAsksSortedByPrice {
        collection: Collection,
        include_inactive: Option<bool>,
        start_before: Option<AskOffset>,
        limit: Option<u32>,
    },
    /// Count of all asks
    #[returns(AskCountResponse)]
    AskCount { collection: Collection },
    /// Get all asks by seller
    #[returns(AsksResponse)]
    AsksBySeller {
        seller: Seller,
        include_inactive: Option<bool>,
        start_after: Option<CollectionOffset>,
        limit: Option<u32>,
    },
    /// Get data for a specific bid
    #[returns(BidResponse)]
    Bid {
        collection: Collection,
        token_id: TokenId,
        bidder: Bidder,
    },
    /// Get all bids by a bidder
    #[returns(BidsResponse)]
    BidsByBidder {
        bidder: Bidder,
        start_after: Option<CollectionOffset>,
        limit: Option<u32>,
    },
    /// Get all bids by a bidder, sorted by expiration
    #[returns(BidsResponse)]
    BidsByBidderSortedByExpiration {
        bidder: Bidder,
        start_after: Option<CollectionOffset>,
        limit: Option<u32>,
    },
    /// Get all bids for a specific NFT
    #[returns(BidsResponse)]
    Bids {
        collection: Collection,
        token_id: TokenId,
        start_after: Option<Bidder>,
        limit: Option<u32>,
    },
    /// Get all bids for a collection, sorted by price
    #[returns(BidsResponse)]
    BidsSortedByPrice {
        collection: Collection,
        start_after: Option<BidOffset>,
        limit: Option<u32>,
    },
    /// Get all bids for a collection, sorted by price in reverse
    #[returns(BidsResponse)]
    ReverseBidsSortedByPrice {
        collection: Collection,
        start_before: Option<BidOffset>,
        limit: Option<u32>,
    },
    /// Get data for a specific collection bid
    #[returns(CollectionBidResponse)]
    CollectionBid {
        collection: Collection,
        bidder: Bidder,
    },
    /// Get all collection bids by a bidder
    #[returns(CollectionBidResponse)]
    CollectionBidsByBidder {
        bidder: Bidder,
        start_after: Option<CollectionOffset>,
        limit: Option<u32>,
    },
    /// Get all collection bids by a bidder, sorted by expiration
    #[returns(CollectionBidResponse)]
    CollectionBidsByBidderSortedByExpiration {
        bidder: Collection,
        start_after: Option<CollectionBidOffset>,
        limit: Option<u32>,
    },
    /// Get all collection bids for a collection sorted by price
    #[returns(CollectionBidResponse)]
    CollectionBidsSortedByPrice {
        collection: Collection,
        start_after: Option<CollectionBidOffset>,
        limit: Option<u32>,
    },
    /// Get all collection bids for a collection sorted by price in reverse
    #[returns(CollectionBidResponse)]
    ReverseCollectionBidsSortedByPrice {
        collection: Collection,
        start_before: Option<CollectionBidOffset>,
        limit: Option<u32>,
    },
    /// Show all registered ask hooks
    #[returns(HooksResponse)]
    AskHooks {},
    /// Show all registered bid hooks
    #[returns(HooksResponse)]
    BidHooks {},
    /// Show all registered sale hooks
    #[returns(HooksResponse)]
    SaleHooks {},
    /// Get the config for the contract
    #[returns(ParamsResponse)]
    Params {},
}

#[cw_serde]
pub struct AskResponse {
    pub ask: Option<Ask>,
}

#[cw_serde]
pub struct AsksResponse {
    pub asks: Vec<Ask>,
}

#[cw_serde]
pub struct AskCountResponse {
    pub count: u32,
}

#[cw_serde]
pub struct CollectionsResponse {
    pub collections: Vec<Addr>,
}

#[cw_serde]
pub struct BidResponse {
    pub bid: Option<Bid>,
}

#[cw_serde]
pub struct BidsResponse {
    pub bids: Vec<Bid>,
}

#[cw_serde]
pub struct ParamsResponse {
    pub params: SudoParams,
}

#[cw_serde]
pub struct CollectionBidResponse {
    pub bid: Option<CollectionBid>,
}

#[cw_serde]
pub struct CollectionBidsResponse {
    pub bids: Vec<CollectionBid>,
}

#[cw_serde]
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
}

// This is just a helper to properly serialize the above message
#[cw_serde]
pub enum SaleExecuteMsg {
    SaleHook(SaleHookMsg),
}

#[cw_serde]
pub enum HookAction {
    Create,
    Update,
    Delete,
}

#[cw_serde]
pub struct AskHookMsg {
    pub ask: Ask,
}

impl AskHookMsg {
    pub fn new(ask: Ask) -> Self {
        AskHookMsg { ask }
    }

    /// serializes the message
    pub fn into_binary(self, action: HookAction) -> StdResult<Binary> {
        let msg = match action {
            HookAction::Create => AskHookExecuteMsg::AskCreatedHook(self),
            HookAction::Update => AskHookExecuteMsg::AskUpdatedHook(self),
            HookAction::Delete => AskHookExecuteMsg::AskDeletedHook(self),
        };
        to_binary(&msg)
    }
}

// This is just a helper to properly serialize the above message
#[cw_serde]
pub enum AskHookExecuteMsg {
    AskCreatedHook(AskHookMsg),
    AskUpdatedHook(AskHookMsg),
    AskDeletedHook(AskHookMsg),
}

#[cw_serde]
pub struct BidHookMsg {
    pub bid: Bid,
}

impl BidHookMsg {
    pub fn new(bid: Bid) -> Self {
        BidHookMsg { bid }
    }

    /// serializes the message
    pub fn into_binary(self, action: HookAction) -> StdResult<Binary> {
        let msg = match action {
            HookAction::Create => BidExecuteMsg::BidCreatedHook(self),
            HookAction::Update => BidExecuteMsg::BidUpdatedHook(self),
            HookAction::Delete => BidExecuteMsg::BidDeletedHook(self),
        };
        to_binary(&msg)
    }
}

// This is just a helper to properly serialize the above message
#[cw_serde]
pub enum BidExecuteMsg {
    BidCreatedHook(BidHookMsg),
    BidUpdatedHook(BidHookMsg),
    BidDeletedHook(BidHookMsg),
}

#[cw_serde]
pub struct CollectionBidHookMsg {
    pub collection_bid: CollectionBid,
}

impl CollectionBidHookMsg {
    pub fn new(collection_bid: CollectionBid) -> Self {
        CollectionBidHookMsg { collection_bid }
    }

    /// serializes the message
    pub fn into_binary(self, action: HookAction) -> StdResult<Binary> {
        let msg = match action {
            HookAction::Create => CollectionBidExecuteMsg::CollectionBidCreatedHook(self),
            HookAction::Update => CollectionBidExecuteMsg::CollectionBidUpdatedHook(self),
            HookAction::Delete => CollectionBidExecuteMsg::CollectionBidDeletedHook(self),
        };
        to_binary(&msg)
    }
}

// This is just a helper to properly serialize the above message
#[cw_serde]
pub enum CollectionBidExecuteMsg {
    CollectionBidCreatedHook(CollectionBidHookMsg),
    CollectionBidUpdatedHook(CollectionBidHookMsg),
    CollectionBidDeletedHook(CollectionBidHookMsg),
}
