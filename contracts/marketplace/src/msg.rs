use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_binary, Binary, Coin, StdResult, Timestamp};
use sg_marketplace_common::query::QueryOptions;

use crate::{
    helpers::ExpiryRange,
    state::{Ask, CollectionOffer, Denom, Offer, PriceRange, SudoParams, TokenId},
};

#[cw_serde]
pub struct InstantiateMsg {
    /// The address of the fair burn contract
    pub fair_burn: String,
    /// Listing fee to reduce spam
    pub listing_fee: Coin,
    /// Valid time range for Asks
    /// (min, max) in seconds
    pub ask_expiry: ExpiryRange,
    /// Valid time range for offers
    /// (min, max) in seconds
    pub offer_expiry: ExpiryRange,
    /// Operators are entites that are responsible for maintaining the active state of Asks.
    /// They listen to NFT transfer events, and update the active state of Asks.
    pub operators: Vec<String>,
    /// The maximum number of asks that can be removed per block
    pub max_asks_removed_per_block: u32,
    /// The maximum number of offers that can be removed per block
    pub max_offers_removed_per_block: u32,
    /// The maximum number of collection offers that can be removed per block
    pub max_collection_offers_removed_per_block: u32,
    /// Fair Burn fee
    /// 0.25% = 25, 0.5% = 50, 1% = 100, 2.5% = 250
    pub trading_fee_bps: u64,
    /// Max basis points for the finders fee
    pub max_finders_fee_bps: u64,
    /// Expired offer / ask removal reward
    pub removal_reward_bps: u64,
    /// Min/max values for offers and asks
    pub price_ranges: Vec<(Denom, PriceRange)>,
    /// The address of the airdrop claim contract to detect sales
    pub sale_hook: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// List an NFT on the marketplace by creating a new ask
    SetAsk {
        collection: String,
        token_id: TokenId,
        price: Coin,
        asset_recipient: Option<String>,
        reserve_for: Option<String>,
        finders_fee_bps: Option<u64>,
        expires: Option<Timestamp>,
    },
    /// Update the price of an existing ask
    UpdateAskPrice {
        collection: String,
        token_id: TokenId,
        price: Coin,
    },
    /// Remove an existing ask from the marketplace
    RemoveAsk {
        collection: String,
        token_id: TokenId,
    },
    /// Privileged operation to remove stale or invalid asks.
    RemoveStaleAsk {
        collection: String,
        token_id: TokenId,
    },
    /// Migrate ask to v3 Ask
    MigrateAsks { limit: u64 },
    /// Create an offer for an NFT
    SetOffer {
        collection: String,
        token_id: TokenId,
        asset_recipient: Option<String>,
        finder: Option<String>,
        finders_fee_bps: Option<u64>,
        expires: Option<Timestamp>,
    },
    /// Buy an NFT from the marketplace
    BuyNow {
        collection: String,
        token_id: TokenId,
        asset_recipient: Option<String>,
        finder: Option<String>,
    },
    /// Accept a offer on an existing ask
    AcceptOffer {
        collection: String,
        token_id: TokenId,
        bidder: String,
        asset_recipient: Option<String>,
        finder: Option<String>,
    },
    /// Remove an existing offer from an ask
    RemoveOffer {
        collection: String,
        token_id: TokenId,
    },
    /// Reject a offer on an existing ask
    RejectOffer {
        collection: String,
        token_id: TokenId,
        bidder: String,
    },
    /// Remove an existing offer from an ask
    RemoveStaleOffer {
        collection: String,
        token_id: TokenId,
        bidder: String,
    },
    /// Migrate Bids to V3 Offers
    MigrateOffers { limit: u64 },
    /// Place an offer (limit order) across an entire collection
    SetCollectionOffer {
        collection: String,
        asset_recipient: Option<String>,
        finders_fee_bps: Option<u64>,
        expires: Option<Timestamp>,
    },
    /// Accept a collection offer
    AcceptCollectionOffer {
        collection: String,
        token_id: TokenId,
        bidder: String,
        asset_recipient: Option<String>,
        finder: Option<String>,
    },
    /// Remove a offer (limit order) across an entire collection
    RemoveCollectionOffer { collection: String },
    /// Remove a offer (limit order) across an entire collection
    RemoveStaleCollectionOffer { collection: String, bidder: String },
    /// Migrate CollectionBids to V3 CollectionOffers
    MigrateCollectionOffers { limit: u64 },
}

#[cw_serde]
pub enum SudoMsg {
    /// BeginBlock Is called by x/cron module BeginBlocker
    BeginBlock {},
    /// EndBlock Is called by x/cron module EndBlocker
    EndBlock {},
    /// Update the contract parameters
    /// Can only be called by governance
    UpdateParams {
        fair_burn: Option<String>,
        listing_fee: Option<Coin>,
        ask_expiry: Option<ExpiryRange>,
        offer_expiry: Option<ExpiryRange>,
        operators: Option<Vec<String>>,
        max_asks_removed_per_block: Option<u32>,
        max_offers_removed_per_block: Option<u32>,
        max_collection_offers_removed_per_block: Option<u32>,
        trading_fee_bps: Option<u64>,
        max_finders_fee_bps: Option<u64>,
        removal_reward_bps: Option<u64>,
    },
    AddDenoms {
        price_ranges: Vec<(Denom, PriceRange)>,
    },
    RemoveDenoms {
        denoms: Vec<Denom>,
    },
    /// Add a new hook to be informed of all asks
    AddAskHook {
        hook: String,
    },
    /// Add a new hook to be informed of all bids
    AddOfferHook {
        hook: String,
    },
    /// Remove a ask hook
    RemoveAskHook {
        hook: String,
    },
    /// Remove a bid hook
    RemoveOfferHook {
        hook: String,
    },
    /// Add a new hook to be informed of all trades
    AddSaleHook {
        hook: String,
    },
    /// Remove a trade hook
    RemoveSaleHook {
        hook: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get the config for the contract
    #[returns(SudoParams)]
    SudoParams {},
    /// Get the current ask for specific NFT
    #[returns(Option<Ask>)]
    Ask {
        collection: String,
        token_id: TokenId,
    },
    /// Get all asks for a collection
    #[returns(Vec<Ask>)]
    Asks {
        collection: String,
        query_options: Option<QueryOptions<TokenId>>,
    },
    /// Get all asks for a collection, sorted by price
    #[returns(Vec<Ask>)]
    AsksByPrice {
        collection: String,
        denom: Denom,
        query_options: Option<QueryOptions<(u128, TokenId)>>,
    },
    /// Get all asks by seller
    #[returns(Vec<Ask>)]
    AsksBySeller {
        seller: String,
        query_options: Option<QueryOptions<(String, TokenId)>>,
    },
    /// Get all asks sorted by the expiration time
    #[returns(Vec<Ask>)]
    AsksByExpiration {
        query_options: Option<QueryOptions<(u64, String, TokenId)>>,
    },
    /// Get data for a specific offer
    #[returns(Offer)]
    Offer {
        collection: String,
        token_id: TokenId,
        bidder: String,
    },
    /// Get all offers on a collection
    #[returns(Vec<Offer>)]
    OffersByCollection {
        collection: String,
        query_options: Option<QueryOptions<(TokenId, String)>>,
    },
    /// Get all offers on a collection token, sorted by price
    #[returns(Vec<Offer>)]
    OffersByTokenPrice {
        collection: String,
        token_id: TokenId,
        denom: Denom,
        query_options: Option<QueryOptions<(u128, String)>>,
    },
    /// Get all offers made by a bidder
    #[returns(Vec<Offer>)]
    OffersByBidder {
        bidder: String,
        query_options: Option<QueryOptions<(String, TokenId)>>,
    },
    /// Get all offers sorted by the expiration time
    #[returns(Vec<Offer>)]
    OffersByExpiration {
        query_options: Option<QueryOptions<(u64, String, TokenId, String)>>,
    },
    /// Get data for a specific collection offer
    #[returns(Option<CollectionOffer>)]
    CollectionOffer { collection: String, bidder: String },
    /// Get all collection offers for a collection, sorted by price
    #[returns(Vec<CollectionOffer>)]
    CollectionOffersByPrice {
        collection: String,
        denom: Denom,
        query_options: Option<QueryOptions<(u128, String)>>,
    },
    /// Get all collection offers made by a bidder
    #[returns(Vec<CollectionOffer>)]
    CollectionOffersByBidder {
        bidder: String,
        query_options: Option<QueryOptions<String>>,
    },
    /// Get all collection offers sorted by the expiration time
    #[returns(Vec<CollectionOffer>)]
    CollectionOffersByExpiration {
        query_options: Option<QueryOptions<(u64, String, String)>>,
    },
    /// Show all registered ask hooks
    #[returns(Vec<String>)]
    AskHooks {},
    /// Show all registered bid hooks
    #[returns(Vec<String>)]
    OfferHooks {},
    /// Show all registered sale hooks
    #[returns(Vec<String>)]
    SaleHooks {},
}

#[cw_serde]
pub struct SaleHookMsg {
    pub collection: String,
    pub token_id: String,
    pub price: Coin,
    pub seller: String,
    pub buyer: String,
}

impl SaleHookMsg {
    pub fn new(
        collection: String,
        token_id: String,
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
pub struct OfferHookMsg {
    pub offer: Offer,
}

impl OfferHookMsg {
    pub fn new(offer: Offer) -> Self {
        OfferHookMsg { offer }
    }

    /// serializes the message
    pub fn into_binary(self, action: HookAction) -> StdResult<Binary> {
        let msg = match action {
            HookAction::Create => OfferExecuteMsg::OfferCreatedHook(self),
            HookAction::Update => OfferExecuteMsg::OfferUpdatedHook(self),
            HookAction::Delete => OfferExecuteMsg::OfferDeletedHook(self),
        };
        to_binary(&msg)
    }
}

// This is just a helper to properly serialize the above message
#[cw_serde]
pub enum OfferExecuteMsg {
    OfferCreatedHook(OfferHookMsg),
    OfferUpdatedHook(OfferHookMsg),
    OfferDeletedHook(OfferHookMsg),
}

#[cw_serde]
pub struct CollectionOfferHookMsg {
    pub collection_offer: CollectionOffer,
}

impl CollectionOfferHookMsg {
    pub fn new(collection_offer: CollectionOffer) -> Self {
        CollectionOfferHookMsg { collection_offer }
    }

    /// serializes the message
    pub fn into_binary(self, action: HookAction) -> StdResult<Binary> {
        let msg = match action {
            HookAction::Create => CollectionOfferExecuteMsg::CollectionOfferCreatedHook(self),
            HookAction::Update => CollectionOfferExecuteMsg::CollectionOfferUpdatedHook(self),
            HookAction::Delete => CollectionOfferExecuteMsg::CollectionOfferDeletedHook(self),
        };
        to_binary(&msg)
    }
}

// This is just a helper to properly serialize the above message
#[cw_serde]
pub enum CollectionOfferExecuteMsg {
    CollectionOfferCreatedHook(CollectionOfferHookMsg),
    CollectionOfferUpdatedHook(CollectionOfferHookMsg),
    CollectionOfferDeletedHook(CollectionOfferHookMsg),
}
