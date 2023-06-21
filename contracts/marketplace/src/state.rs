use std::fmt;

use crate::helpers::{build_collection_token_index_str, ExpiryRange};
use crate::ContractError;
use crate::{constants::MAX_BASIS_POINTS, helpers::price_validate};

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    attr, coin, ensure, Addr, Attribute, BlockInfo, Coin, Decimal, Event, Storage, Timestamp,
    Uint128,
};
use cw_storage_macro::index_list;
use cw_storage_plus::{IndexedMap, Item, Map, MultiIndex};
use sg_controllers::Hooks;
use sg_marketplace_common::address::address_or;
use sg_marketplace_common::coin::decimal_to_bps;

pub type TokenId = String;
pub type Denom = String;

#[cw_serde]
pub struct SudoParams {
    /// The address of the fair burn contract
    pub fair_burn: Addr,
    /// Listing fee to reduce spam
    pub listing_fee: Coin,
    /// Valid time range for Asks
    /// (min, max) in seconds
    pub ask_expiry: ExpiryRange,
    /// Valid time range for offers
    /// (min, max) in seconds
    pub offer_expiry: ExpiryRange,
    /// Operators are entites that are responsible for maintaining the active state of Asks
    /// They listen to NFT transfer events, and update the active state of Asks
    pub operators: Vec<Addr>,
    /// The maximum number of asks that can be removed per block
    pub max_asks_removed_per_block: u32,
    /// The maximum number of offers that can be removed per block
    pub max_offers_removed_per_block: u32,
    /// The maximum number of collection offers that can be removed per block
    pub max_collection_offers_removed_per_block: u32,
    /// Fair Burn fee
    pub trading_fee_percent: Decimal,
    /// Max value for the finders fee
    pub max_finders_fee_percent: Decimal,
    /// Stale offer / ask removal reward
    pub removal_reward_percent: Decimal,
}

impl SudoParams {
    pub fn save(&self, storage: &mut dyn Storage) -> Result<(), ContractError> {
        SUDO_PARAMS.save(storage, self)?;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), ContractError> {
        let max_finders_fee_bps = decimal_to_bps(self.max_finders_fee_percent) as u64;
        ensure!(
            max_finders_fee_bps <= MAX_BASIS_POINTS,
            ContractError::InvalidFindersFeePercent(Decimal::percent(max_finders_fee_bps))
        );

        let trading_fee_bps = decimal_to_bps(self.trading_fee_percent) as u64;
        ensure!(
            trading_fee_bps <= MAX_BASIS_POINTS,
            ContractError::InvalidTradingFeeBps(trading_fee_bps)
        );

        let removal_reward_bps = decimal_to_bps(self.removal_reward_percent) as u64;
        ensure!(
            removal_reward_bps <= MAX_BASIS_POINTS,
            ContractError::InvalidBidRemovalRewardBps(removal_reward_bps)
        );

        self.ask_expiry.validate()?;
        self.offer_expiry.validate()?;

        Ok(())
    }
}

pub const SUDO_PARAMS: Item<SudoParams> = Item::new("sp");

// A map of acceptable denoms to their minimum trade prices.
// Denoms not found in the Map are not accepted.
#[cw_serde]
pub struct PriceRange {
    pub min: Uint128,
    pub max: Uint128,
}
pub const PRICE_RANGES: Map<Denom, PriceRange> = Map::new("pr");

impl fmt::Display for PriceRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{\"min\":{},\"max\":{}}}", self.min, self.max)
    }
}

pub const ASK_HOOKS: Hooks = Hooks::new("ah");
pub const OFFER_HOOKS: Hooks = Hooks::new("oh");
pub const SALE_HOOKS: Hooks = Hooks::new("sh");
pub const COLLECTION_OFFER_HOOKS: Hooks = Hooks::new("coh");

pub trait ExpiringOrder {
    fn expires(&self) -> Option<Timestamp>;

    fn is_expired(&self, block: &BlockInfo) -> bool {
        if let Some(expires) = self.expires() {
            return expires <= block.time;
        }
        false
    }
}

/// Primary key for asks: (collection, token_id)
pub type AskKey = (Addr, TokenId);

/// Represents an ask on the marketplace
#[cw_serde]
pub struct Ask {
    pub collection: Addr,
    pub token_id: TokenId,
    pub seller: Addr,
    pub price: Coin,
    pub asset_recipient: Option<Addr>,
    pub reserve_for: Option<Addr>,
    pub finders_fee_percent: Option<Decimal>,
    pub expires: Option<Timestamp>,
    pub paid_removal_fee: Option<Coin>,
}

impl ExpiringOrder for Ask {
    fn expires(&self) -> Option<Timestamp> {
        self.expires
    }
}

impl Ask {
    pub fn build_key(collection: &Addr, token_id: &TokenId) -> AskKey {
        (collection.clone(), token_id.clone())
    }

    pub fn key(&self) -> AskKey {
        Self::build_key(&self.collection, &self.token_id)
    }

    pub fn key_to_str(&self) -> String {
        format!("{}-{}", self.collection, self.token_id)
    }

    pub fn save(&self, storage: &mut dyn Storage) -> Result<(), ContractError> {
        asks().save(storage, self.key(), self)?;
        Ok(())
    }

    pub fn validate(
        &self,
        storage: &dyn Storage,
        block: &BlockInfo,
        sudo_params: &SudoParams,
    ) -> Result<(), ContractError> {
        if let Some(expires) = self.expires {
            sudo_params.ask_expiry.is_valid(block, expires)?;
        }
        if let Some(finders_fee_percent) = self.finders_fee_percent {
            ensure!(
                finders_fee_percent <= sudo_params.max_finders_fee_percent,
                ContractError::InvalidFindersFeePercent(finders_fee_percent)
            );
        }
        if let Some(reserve_for) = &self.reserve_for {
            ensure!(
                reserve_for != &self.seller,
                ContractError::InvalidReserveAddress {
                    reason: "cannot reserve to the same address".to_string(),
                }
            );
        }
        price_validate(storage, &self.price, true)?;

        Ok(())
    }

    pub fn removal_fee(&self, removal_fee_percent: Decimal) -> Option<Coin> {
        self.expires.map(|_| {
            coin(
                self.price.amount.mul_ceil(removal_fee_percent).u128(),
                self.price.denom.clone(),
            )
        })
    }

    pub fn asset_recipient(&self) -> Addr {
        address_or(self.asset_recipient.as_ref(), &self.seller)
    }

    pub fn create_event(&self, event_type: &str, attr_keys: Vec<&str>) -> Event {
        let mut attributes: Vec<Attribute> = vec![];
        for attr_keys in attr_keys {
            let attribute_option = match attr_keys {
                "collection" => Some(attr("collection", self.collection.to_string())),
                "token_id" => Some(attr("token_id", self.token_id.to_string())),
                "seller" => Some(attr("seller", self.seller.to_string())),
                "price" => Some(attr("price", self.price.to_string())),
                "asset_recipient" => self
                    .asset_recipient
                    .as_ref()
                    .map(|addr| attr("funds_recipient", addr.to_string())),
                "reserve_for" => self
                    .reserve_for
                    .as_ref()
                    .map(|addr| attr("reserve_for", addr.to_string())),
                "finders_fee_percent" => self
                    .finders_fee_percent
                    .map(|addr| attr("finders_fee_percent", addr.to_string())),
                "expires" => self.expires.map(|addr| attr("expires", addr.to_string())),
                "paid_removal_fee" => self
                    .paid_removal_fee
                    .as_ref()
                    .map(|addr| attr("paid_removal_fee", addr.to_string())),
                _ => unimplemented!("Invalid attribute key: {}", attr_keys),
            };
            if let Some(attribute) = attribute_option {
                attributes.push(attribute);
            }
        }
        Event::new(event_type).add_attributes(attributes)
    }
}

/// Defines indices for accessing Asks
#[index_list(Ask)]
pub struct AskIndices<'a> {
    // Index Asks by collection and denom price
    pub collection_denom_price: MultiIndex<'a, (Addr, Denom, u128), Ask, AskKey>,
    // Index Asks by seller
    pub seller: MultiIndex<'a, Addr, Ask, AskKey>,
    // Index asks by expiration (seconds), necessary to remove in EndBlocker
    pub expiration: MultiIndex<'a, u64, Ask, AskKey>,
}

pub fn asks<'a>() -> IndexedMap<'a, AskKey, Ask, AskIndices<'a>> {
    let indexes: AskIndices = AskIndices {
        collection_denom_price: MultiIndex::new(
            |_pk: &[u8], a: &Ask| {
                (
                    a.collection.clone(),
                    a.price.denom.clone(),
                    a.price.amount.u128(),
                )
            },
            "a",
            "a_cp",
        ),
        seller: MultiIndex::new(|_pk: &[u8], a: &Ask| a.seller.clone(), "a", "a_s"),
        expiration: MultiIndex::new(
            |_pk: &[u8], a: &Ask| a.expires.map_or(u64::MAX, |ea| ea.seconds()),
            "a",
            "a_ea",
        ),
    };
    IndexedMap::new("a", indexes)
}

/// Primary key for offers: (collection, token_id, bidder)
pub type OfferKey = (Addr, TokenId, Addr);

/// Represents an offer on an NFT in the marketplace
#[cw_serde]
pub struct Offer {
    pub collection: Addr,
    pub token_id: TokenId,
    pub bidder: Addr,
    pub price: Coin,
    pub asset_recipient: Option<Addr>,
    pub finders_fee_percent: Option<Decimal>,
    pub expires: Option<Timestamp>,
}

impl Offer {
    pub fn build_key(collection: &Addr, token_id: &TokenId, bidder: &Addr) -> OfferKey {
        (collection.clone(), token_id.clone(), bidder.clone())
    }

    pub fn key(&self) -> OfferKey {
        Self::build_key(&self.collection, &self.token_id, &self.bidder)
    }

    pub fn key_to_str(&self) -> String {
        format!("{}-{}-{}", self.collection, self.token_id, &self.bidder)
    }

    pub fn save(&self, storage: &mut dyn Storage) -> Result<(), ContractError> {
        offers().save(storage, self.key(), self)?;
        Ok(())
    }

    pub fn validate(
        &self,
        storage: &dyn Storage,
        block: &BlockInfo,
        sudo_params: &SudoParams,
    ) -> Result<(), ContractError> {
        price_validate(storage, &self.price, false)?;

        if let Some(finders_fee_percent) = self.finders_fee_percent {
            ensure!(
                finders_fee_percent <= sudo_params.max_finders_fee_percent,
                ContractError::InvalidFindersFeePercent(finders_fee_percent)
            );
        }
        if let Some(expires) = self.expires {
            sudo_params.offer_expiry.is_valid(block, expires)?;
        }

        Ok(())
    }

    pub fn asset_recipient(&self) -> Addr {
        address_or(self.asset_recipient.as_ref(), &self.bidder)
    }

    pub fn create_event(&self, event_type: &str, attr_keys: Vec<&str>) -> Event {
        let mut attributes: Vec<Attribute> = vec![];
        for attr_keys in attr_keys {
            let attribute_option = match attr_keys {
                "collection" => Some(attr("collection", self.collection.to_string())),
                "token_id" => Some(attr("token_id", self.token_id.to_string())),
                "bidder" => Some(attr("bidder", self.bidder.to_string())),
                "price" => Some(attr("price", self.price.to_string())),
                "asset_recipient" => self
                    .asset_recipient
                    .as_ref()
                    .map(|addr| attr("asset_recipient", addr.to_string())),
                "finders_fee_percent" => self
                    .finders_fee_percent
                    .map(|addr| attr("finders_fee_percent", addr.to_string())),
                "expires" => self.expires.map(|addr| attr("expires", addr.to_string())),
                _ => unimplemented!("Invalid attribute key: {}", attr_keys),
            };
            if let Some(attribute) = attribute_option {
                attributes.push(attribute);
            }
        }
        Event::new(event_type).add_attributes(attributes)
    }
}

impl ExpiringOrder for Offer {
    fn expires(&self) -> Option<Timestamp> {
        self.expires
    }
}

/// Defines incides for accessing offers
#[index_list(Offer)]
pub struct OfferIndices<'a> {
    // Index offers for a token id, sorted by denom price (infinity dependency)
    pub token_denom_price: MultiIndex<'a, (String, Denom, u128), Offer, OfferKey>,
    // Index offers by bidder
    pub bidder: MultiIndex<'a, Addr, Offer, OfferKey>,
    // Index offers by expiration
    pub expiration: MultiIndex<'a, u64, Offer, OfferKey>,
}

pub fn offers<'a>() -> IndexedMap<'a, OfferKey, Offer, OfferIndices<'a>> {
    let indexes = OfferIndices {
        token_denom_price: MultiIndex::new(
            |_pk: &[u8], o: &Offer| {
                (
                    build_collection_token_index_str(o.collection.as_ref(), &o.token_id),
                    o.price.denom.clone(),
                    o.price.amount.u128(),
                )
            },
            "o",
            "o_tdp",
        ),
        bidder: MultiIndex::new(|_pk: &[u8], o: &Offer| o.bidder.clone(), "o", "o_b"),
        expiration: MultiIndex::new(
            |_pk: &[u8], o: &Offer| o.expires.map_or(u64::MAX, |ea| ea.seconds()),
            "o",
            "o_ea",
        ),
    };
    IndexedMap::new("o", indexes)
}

/// Primary key for collection offers: (collection, bidder)
pub type CollectionOfferKey = (Addr, Addr);

/// Represents an offer across an entire collection in the marketplace
#[cw_serde]
pub struct CollectionOffer {
    pub collection: Addr,
    pub bidder: Addr,
    pub price: Coin,
    pub asset_recipient: Option<Addr>,
    pub finders_fee_percent: Option<Decimal>,
    pub expires: Option<Timestamp>,
}

impl ExpiringOrder for CollectionOffer {
    fn expires(&self) -> Option<Timestamp> {
        self.expires
    }
}

impl CollectionOffer {
    pub fn build_key(collection: &Addr, bidder: &Addr) -> CollectionOfferKey {
        (collection.clone(), bidder.clone())
    }

    pub fn key(&self) -> CollectionOfferKey {
        Self::build_key(&self.collection, &self.bidder)
    }

    pub fn key_to_str(&self) -> String {
        format!("{}-{}", self.collection, &self.bidder)
    }

    pub fn save(&self, storage: &mut dyn Storage) -> Result<(), ContractError> {
        collection_offers().save(storage, self.key(), self)?;
        Ok(())
    }

    pub fn validate(
        &self,
        storage: &dyn Storage,
        block: &BlockInfo,
        sudo_params: &SudoParams,
    ) -> Result<(), ContractError> {
        price_validate(storage, &self.price, false)?;

        if let Some(expires) = self.expires {
            sudo_params.offer_expiry.is_valid(block, expires)?;
        }
        if let Some(finders_fee_percent) = self.finders_fee_percent {
            ensure!(
                finders_fee_percent <= sudo_params.max_finders_fee_percent,
                ContractError::InvalidFindersFeePercent(finders_fee_percent)
            );
        }

        Ok(())
    }

    pub fn asset_recipient(&self) -> Addr {
        address_or(self.asset_recipient.as_ref(), &self.bidder)
    }

    pub fn create_event(&self, event_type: &str, attr_keys: Vec<&str>) -> Event {
        let mut attributes: Vec<Attribute> = vec![];
        for attr_keys in attr_keys {
            let attribute_option = match attr_keys {
                "collection" => Some(attr("collection", self.collection.to_string())),
                "bidder" => Some(attr("bidder", self.bidder.to_string())),
                "price" => Some(attr("price", self.price.to_string())),
                "asset_recipient" => self
                    .asset_recipient
                    .as_ref()
                    .map(|addr| attr("asset_recipient", addr.to_string())),
                "finders_fee_percent" => self
                    .finders_fee_percent
                    .map(|addr| attr("finders_fee_percent", addr.to_string())),
                "expires" => self.expires.map(|addr| attr("expires", addr.to_string())),
                _ => unimplemented!("Invalid attribute key: {}", attr_keys),
            };
            if let Some(attribute) = attribute_option {
                attributes.push(attribute);
            }
        }
        Event::new(event_type).add_attributes(attributes)
    }
}

/// Defines incides for accessing collection offers
#[index_list(CollectionOffer)]
pub struct CollectionOfferIndices<'a> {
    // Index collection offers by collection and price
    pub collection_denom_price:
        MultiIndex<'a, (Addr, Denom, u128), CollectionOffer, CollectionOfferKey>,
    // Index collection offers by bidder
    pub bidder: MultiIndex<'a, Addr, CollectionOffer, CollectionOfferKey>,
    // Index collections by expiration, necessary to remove in endblocker
    pub expiration: MultiIndex<'a, u64, CollectionOffer, CollectionOfferKey>,
}

pub fn collection_offers<'a>(
) -> IndexedMap<'a, CollectionOfferKey, CollectionOffer, CollectionOfferIndices<'a>> {
    let indexes = CollectionOfferIndices {
        collection_denom_price: MultiIndex::new(
            |_pk: &[u8], co: &CollectionOffer| {
                (
                    co.collection.clone(),
                    co.price.denom.clone(),
                    co.price.amount.u128(),
                )
            },
            "co",
            "co_cdp",
        ),
        bidder: MultiIndex::new(
            |_pk: &[u8], co: &CollectionOffer| co.bidder.clone(),
            "co",
            "co_b",
        ),
        expiration: MultiIndex::new(
            |_pk: &[u8], co: &CollectionOffer| co.expires.map_or(u64::MAX, |ea| ea.seconds()),
            "co",
            "co_ea",
        ),
    };
    IndexedMap::new("co", indexes)
}
