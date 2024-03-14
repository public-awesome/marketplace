use crate::constants::MAX_BASIS_POINTS;
use crate::{helpers::build_collection_token_index_str, ContractError};

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{ensure, Addr, Api, Coin, Storage, Timestamp, Uint128};
use cw_address_like::AddressLike;
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};
use std::fmt;

pub type TokenId = String;
pub type Denom = String;

#[cw_serde]
pub struct Config<T: AddressLike> {
    /// The address of the fair burn contract
    pub fair_burn: T,
    /// The address of the royalty registry contract
    pub royalty_registry: T,
    /// Listing fee to reduce spam
    pub listing_fee: Coin,
    /// Minimum removal reward
    pub min_removal_reward: Coin,
    /// Fair Burn fee
    pub trading_fee_bps: u64,
    /// Max value for the royalty fee
    pub max_royalty_fee_bps: u64,
    /// Max value for the finders fee
    pub max_finders_fee_bps: u64,
    /// Minimum expiry seconds for asks / offers
    pub min_expiration_seconds: u64,
    /// The number of seconds to look ahead for orders to remove
    pub order_removal_lookahead_secs: u64,
    /// The maximum number of asks that can be removed per block
    pub max_asks_removed_per_block: u32,
    /// The maximum number of offers that can be removed per block
    pub max_offers_removed_per_block: u32,
    /// The maximum number of collection offers that can be removed per block
    pub max_collection_offers_removed_per_block: u32,
}

impl Config<String> {
    pub fn str_to_addr(self, api: &dyn Api) -> Result<Config<Addr>, ContractError> {
        Ok(Config {
            fair_burn: api.addr_validate(&self.fair_burn)?,
            royalty_registry: api.addr_validate(&self.royalty_registry)?,
            listing_fee: self.listing_fee,
            min_removal_reward: self.min_removal_reward,
            trading_fee_bps: self.trading_fee_bps,
            max_royalty_fee_bps: self.max_royalty_fee_bps,
            max_finders_fee_bps: self.max_finders_fee_bps,
            min_expiration_seconds: self.min_expiration_seconds,
            order_removal_lookahead_secs: self.order_removal_lookahead_secs,
            max_asks_removed_per_block: self.max_asks_removed_per_block,
            max_offers_removed_per_block: self.max_offers_removed_per_block,
            max_collection_offers_removed_per_block: self.max_collection_offers_removed_per_block,
        })
    }
}

impl Config<Addr> {
    pub fn save(&self, storage: &mut dyn Storage) -> Result<(), ContractError> {
        SUDO_PARAMS.save(storage, self)?;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), ContractError> {
        ensure!(
            self.trading_fee_bps < MAX_BASIS_POINTS,
            ContractError::InvalidInput("trade_fee_bps must be less than 1".to_string())
        );
        ensure!(
            self.max_finders_fee_bps < MAX_BASIS_POINTS,
            ContractError::InvalidInput("max_finders_fee_bps must be less than 1".to_string())
        );

        Ok(())
    }
}

pub const SUDO_PARAMS: Item<Config<Addr>> = Item::new("S");

// A map of acceptable denoms to their minimum trade prices.
// Denoms not found in the Map are not accepted.
#[cw_serde]
pub struct PriceRange {
    pub min: Uint128,
    pub max: Uint128,
}
pub const PRICE_RANGES: Map<Denom, PriceRange> = Map::new("P");

impl fmt::Display for PriceRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{\"min\":{},\"max\":{}}}", self.min, self.max)
    }
}

#[cw_serde]
pub struct ExpirationInfo {
    pub expiration: Timestamp,
    pub removal_reward: Coin,
}

#[cw_serde]
pub struct OrderInfo {
    pub price: Coin,
    pub creator: Addr,
    pub asset_recipient: Option<Addr>,
    pub finders_fee_bps: Option<u64>,
    pub expiration_info: Option<ExpirationInfo>,
}

pub trait KeyString {
    fn to_string(&self) -> String;
}

/// Primary key for asks: (collection, token_id)
pub type AskKey = (String, String);

impl KeyString for AskKey {
    fn to_string(&self) -> String {
        format!("{}-{}", self.0, self.1)
    }
}

/// Represents an ask on the marketplace
#[cw_serde]
pub struct Ask {
    pub collection: Addr,
    pub token_id: String,
    pub order_info: OrderInfo,
}

/// Defines indices for accessing Asks
pub struct AskIndices<'a> {
    // Index Asks by collection and denom price
    pub collection_denom_price: MultiIndex<'a, (Addr, Denom, u128), Ask, AskKey>,
    // Index Asks by creator
    pub creator: MultiIndex<'a, Addr, Ask, AskKey>,
    // Index asks by expiration (seconds), necessary to remove in EndBlocker
    pub expiration: MultiIndex<'a, u64, Ask, AskKey>,
}

impl<'a> IndexList<Ask> for AskIndices<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Ask>> + '_> {
        let v: Vec<&dyn Index<Ask>> = vec![
            &self.collection_denom_price,
            &self.creator,
            &self.expiration,
        ];
        Box::new(v.into_iter())
    }
}

pub fn asks<'a>() -> IndexedMap<'a, AskKey, Ask, AskIndices<'a>> {
    let indexes: AskIndices = AskIndices {
        collection_denom_price: MultiIndex::new(
            |_pk: &[u8], a: &Ask| {
                (
                    a.collection.clone(),
                    a.order_info.price.denom.clone(),
                    a.order_info.price.amount.u128(),
                )
            },
            "A",
            "Ap",
        ),
        creator: MultiIndex::new(
            |_pk: &[u8], a: &Ask| a.order_info.creator.clone(),
            "A",
            "Ac",
        ),
        expiration: MultiIndex::new(
            |_pk: &[u8], a: &Ask| {
                a.order_info
                    .expiration_info
                    .as_ref()
                    .map_or(u64::MAX, |e| e.expiration.seconds())
            },
            "A",
            "Ae",
        ),
    };
    IndexedMap::new("A", indexes)
}

/// Primary key for offers: (collection, token_id, creator)
pub type OfferKey = (Addr, TokenId, Addr);

impl KeyString for OfferKey {
    fn to_string(&self) -> String {
        format!("{}-{}-{}", self.0, self.1, self.2)
    }
}

/// Represents an offer on an NFT in the marketplace
#[cw_serde]
pub struct Offer {
    pub collection: Addr,
    pub token_id: String,
    pub order_info: OrderInfo,
}

/// Defines incides for accessing offers
pub struct OfferIndices<'a> {
    // Index offers for a token id, sorted by denom price (infinity dependency)
    pub token_denom_price: MultiIndex<'a, (TokenId, Denom, u128), Offer, OfferKey>,
    // Index offers by creator
    pub creator: MultiIndex<'a, Addr, Offer, OfferKey>,
    // Index offers by expiration
    pub expiration: MultiIndex<'a, u64, Offer, OfferKey>,
}

impl<'a> IndexList<Offer> for OfferIndices<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Offer>> + '_> {
        let v: Vec<&dyn Index<Offer>> =
            vec![&self.token_denom_price, &self.creator, &self.expiration];
        Box::new(v.into_iter())
    }
}

pub fn offers<'a>() -> IndexedMap<'a, OfferKey, Offer, OfferIndices<'a>> {
    let indexes = OfferIndices {
        token_denom_price: MultiIndex::new(
            |_pk: &[u8], o: &Offer| {
                (
                    build_collection_token_index_str(o.collection.as_ref(), &o.token_id),
                    o.order_info.price.denom.clone(),
                    o.order_info.price.amount.u128(),
                )
            },
            "O",
            "Op",
        ),
        creator: MultiIndex::new(
            |_pk: &[u8], o: &Offer| o.order_info.creator.clone(),
            "O",
            "Oc",
        ),
        expiration: MultiIndex::new(
            |_pk: &[u8], o: &Offer| {
                o.order_info
                    .expiration_info
                    .as_ref()
                    .map_or(u64::MAX, |e| e.expiration.seconds())
            },
            "O",
            "Oe",
        ),
    };
    IndexedMap::new("O", indexes)
}

/// Primary key for collection offers: (collection, creator)
pub type CollectionOfferKey = (Addr, Addr);

impl KeyString for CollectionOfferKey {
    fn to_string(&self) -> String {
        format!("{}-{}", self.0, self.1)
    }
}

/// Represents an offer across an entire collection in the marketplace
#[cw_serde]
pub struct CollectionOffer {
    pub collection: Addr,
    pub order_info: OrderInfo,
}

/// Defines incides for accessing collection offers
pub struct CollectionOfferIndices<'a> {
    // Index collection offers by collection and price
    pub collection_denom_price:
        MultiIndex<'a, (Addr, Denom, u128), CollectionOffer, CollectionOfferKey>,
    // Index collection offers by creator
    pub creator: MultiIndex<'a, Addr, CollectionOffer, CollectionOfferKey>,
    // Index collections by expiration, necessary to remove in endblocker
    pub expiration: MultiIndex<'a, u64, CollectionOffer, CollectionOfferKey>,
}

impl<'a> IndexList<CollectionOffer> for CollectionOfferIndices<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<CollectionOffer>> + '_> {
        let v: Vec<&dyn Index<CollectionOffer>> = vec![
            &self.collection_denom_price,
            &self.creator,
            &self.expiration,
        ];
        Box::new(v.into_iter())
    }
}

pub fn collection_offers<'a>(
) -> IndexedMap<'a, CollectionOfferKey, CollectionOffer, CollectionOfferIndices<'a>> {
    let indexes = CollectionOfferIndices {
        collection_denom_price: MultiIndex::new(
            |_pk: &[u8], co: &CollectionOffer| {
                (
                    co.collection.clone(),
                    co.order_info.price.denom.clone(),
                    co.order_info.price.amount.u128(),
                )
            },
            "C",
            "Cp",
        ),
        creator: MultiIndex::new(
            |_pk: &[u8], co: &CollectionOffer| co.order_info.creator.clone(),
            "C",
            "Cc",
        ),
        expiration: MultiIndex::new(
            |_pk: &[u8], co: &CollectionOffer| {
                co.order_info
                    .expiration_info
                    .as_ref()
                    .map_or(u64::MAX, |e| e.expiration.seconds())
            },
            "C",
            "Ce",
        ),
    };
    IndexedMap::new("C", indexes)
}
