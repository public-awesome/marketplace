use cosmwasm_std::{Addr, Decimal, Timestamp, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex};
use cw_utils::Duration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sg_controllers::Hooks;

use crate::helpers::ExpiryRange;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SudoParams {
    /// Fair Burn fee for winning bids
    pub trading_fee_percent: Decimal,
    /// Valid time range for Asks
    /// (min, max) in seconds
    pub ask_expiry: ExpiryRange,
    /// Valid time range for Bids
    /// (min, max) in seconds
    pub bid_expiry: ExpiryRange,
    /// Operators are entites that are responsible for maintaining the active state of Asks
    /// They listen to NFT transfer events, and update the active state of Asks
    pub operators: Vec<Addr>,
    /// Max value for the finders fee
    pub max_finders_fee_percent: Decimal,
    /// Min value for a bid
    pub min_price: Uint128,
    /// Duration after expiry when a bid becomes stale
    pub stale_bid_duration: Duration,
    /// Stale bid removal reward
    pub bid_removal_reward_percent: Decimal,
}

pub const SUDO_PARAMS: Item<SudoParams> = Item::new("sudo-params");

pub const ASK_HOOKS: Hooks = Hooks::new("ask-hooks");
pub const BID_HOOKS: Hooks = Hooks::new("bid-created-hooks");
pub const SALE_HOOKS: Hooks = Hooks::new("sale-hooks");

pub type TokenId = u32;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SaleType {
    FixedPrice,
    Auction,
}

/// Represents an ask on the marketplace
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Ask {
    pub sale_type: SaleType,
    pub collection: Addr,
    pub token_id: TokenId,
    pub seller: Addr,
    pub price: Uint128,
    pub funds_recipient: Option<Addr>,
    pub reserve_for: Option<Addr>,
    pub finders_fee_bps: Option<u64>,
    pub expires: Timestamp,
    pub is_active: bool,
}

/// Primary key for asks: (collection, token_id)
pub type AskKey = (Addr, TokenId);
/// Convenience ask key constructor
pub fn ask_key(collection: Addr, token_id: TokenId) -> AskKey {
    (collection, token_id)
}

/// Defines indices for accessing Asks
pub struct AskIndicies<'a> {
    pub collection: MultiIndex<'a, Addr, Ask, AskKey>,
    pub collection_price: MultiIndex<'a, (Addr, u128), Ask, AskKey>,
    pub seller: MultiIndex<'a, Addr, Ask, AskKey>,
}

impl<'a> IndexList<Ask> for AskIndicies<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Ask>> + '_> {
        let v: Vec<&dyn Index<Ask>> = vec![&self.collection, &self.collection_price, &self.seller];
        Box::new(v.into_iter())
    }
}

pub fn asks<'a>() -> IndexedMap<'a, AskKey, Ask, AskIndicies<'a>> {
    let indexes = AskIndicies {
        collection: MultiIndex::new(|d: &Ask| d.collection.clone(), "asks", "asks__collection"),
        collection_price: MultiIndex::new(
            |d: &Ask| (d.collection.clone(), d.price.u128()),
            "asks",
            "asks__collection_price",
        ),
        seller: MultiIndex::new(|d: &Ask| d.seller.clone(), "asks", "asks__seller"),
    };
    IndexedMap::new("asks", indexes)
}

/// Represents a bid (offer) on the marketplace
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Bid {
    pub collection: Addr,
    pub token_id: TokenId,
    pub bidder: Addr,
    pub price: Uint128,
    pub finders_fee_bps: Option<u64>,
    pub expires: Timestamp,
}

impl Bid {
    pub fn new(
        collection: Addr,
        token_id: TokenId,
        bidder: Addr,
        price: Uint128,
        finders_fee_bps: Option<u64>,
        expires: Timestamp,
    ) -> Self {
        Bid {
            collection,
            token_id,
            bidder,
            price,
            finders_fee_bps,
            expires,
        }
    }
}

/// Primary key for bids: (collection, token_id, bidder)
pub type BidKey = (Addr, TokenId, Addr);
/// Convenience bid key constructor
pub fn bid_key(collection: Addr, token_id: TokenId, bidder: Addr) -> BidKey {
    (collection, token_id, bidder)
}

/// Defines incides for accessing bids
pub struct BidIndicies<'a> {
    pub collection: MultiIndex<'a, Addr, Bid, BidKey>,
    pub collection_token_id: MultiIndex<'a, (Addr, TokenId), Bid, BidKey>,
    pub collection_price: MultiIndex<'a, (Addr, u128), Bid, BidKey>,
    pub bidder: MultiIndex<'a, Addr, Bid, BidKey>,
}

impl<'a> IndexList<Bid> for BidIndicies<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Bid>> + '_> {
        let v: Vec<&dyn Index<Bid>> = vec![
            &self.collection,
            &self.collection_token_id,
            &self.collection_price,
            &self.bidder,
        ];
        Box::new(v.into_iter())
    }
}

pub fn bids<'a>() -> IndexedMap<'a, BidKey, Bid, BidIndicies<'a>> {
    let indexes = BidIndicies {
        collection: MultiIndex::new(|d: &Bid| d.collection.clone(), "bids", "bids__collection"),
        collection_token_id: MultiIndex::new(
            |d: &Bid| (d.collection.clone(), d.token_id),
            "bids",
            "bids__collection_token_id",
        ),
        collection_price: MultiIndex::new(
            |d: &Bid| (d.collection.clone(), d.price.u128()),
            "bids",
            "bids__collection_price",
        ),
        bidder: MultiIndex::new(|d: &Bid| d.bidder.clone(), "bids", "bids__bidder"),
    };
    IndexedMap::new("bids", indexes)
}

/// Represents a bid (offer) across an entire collection in the marketplace
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CollectionBid {
    pub collection: Addr,
    pub bidder: Addr,
    pub price: Uint128,
    pub finders_fee_bps: Option<u64>,
    pub expires: Timestamp,
}

/// Primary key for bids: (collection, token_id, bidder)
pub type CollectionBidKey = (Addr, Addr);
/// Convenience collection bid key constructor
pub fn collection_bid_key(collection: Addr, bidder: Addr) -> CollectionBidKey {
    (collection, bidder)
}

/// Defines incides for accessing collection bids
pub struct CollectionBidIndicies<'a> {
    pub collection: MultiIndex<'a, Addr, CollectionBid, CollectionBidKey>,
    pub collection_price: MultiIndex<'a, (Addr, u128), CollectionBid, CollectionBidKey>,
    pub bidder: MultiIndex<'a, Addr, CollectionBid, CollectionBidKey>,
}

impl<'a> IndexList<CollectionBid> for CollectionBidIndicies<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<CollectionBid>> + '_> {
        let v: Vec<&dyn Index<CollectionBid>> =
            vec![&self.collection, &self.collection_price, &self.bidder];
        Box::new(v.into_iter())
    }
}

pub fn collection_bids<'a>(
) -> IndexedMap<'a, CollectionBidKey, CollectionBid, CollectionBidIndicies<'a>> {
    let indexes = CollectionBidIndicies {
        collection: MultiIndex::new(
            |d: &CollectionBid| d.collection.clone(),
            "col_bids",
            "col_bids__collection",
        ),
        collection_price: MultiIndex::new(
            |d: &CollectionBid| (d.collection.clone(), d.price.u128()),
            "col_bids",
            "col_bids__collection_price",
        ),
        bidder: MultiIndex::new(
            |d: &CollectionBid| d.bidder.clone(),
            "col_bids",
            "col_bids__bidder",
        ),
    };
    IndexedMap::new("col_bids", indexes)
}
