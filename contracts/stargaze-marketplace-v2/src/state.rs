use crate::helpers::build_collection_token_index_str;
use crate::orders::{Bid, CollectionBid};
use crate::ContractError;
use crate::{constants::MAX_BASIS_POINTS, orders::Ask};

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{ensure, Addr, Api, Storage, Uint128};
use cw_address_like::AddressLike;
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};

pub type OrderId = String;
pub type TokenId = String;
pub type Denom = String;

#[cw_serde]
pub struct Config<T: AddressLike> {
    /// The address of the address that will receive the protocol fees
    pub fee_manager: T,
    /// The address of the royalty registry contract
    pub royalty_registry: T,
    /// Protocol fee
    pub protocol_fee_bps: u64,
    /// Protocol fee for trades in non-native denoms
    pub non_native_protocol_fee_bps: u64,
    /// Max value for the royalty fee
    pub max_royalty_fee_bps: u64,
    /// The reward paid out to the market maker. Reward is a percentage of the protocol fee
    pub maker_reward_bps: u64,
    /// The reward paid out to the market taker. Reward is a percentage of the protocol fee
    pub taker_reward_bps: u64,
    /// The default denom for all collections on the marketplace
    pub default_denom: Denom,
}

impl Config<String> {
    pub fn str_to_addr(self, api: &dyn Api) -> Result<Config<Addr>, ContractError> {
        Ok(Config {
            fee_manager: api.addr_validate(&self.fee_manager)?,
            royalty_registry: api.addr_validate(&self.royalty_registry)?,
            protocol_fee_bps: self.protocol_fee_bps,
            non_native_protocol_fee_bps: self.non_native_protocol_fee_bps,
            max_royalty_fee_bps: self.max_royalty_fee_bps,
            maker_reward_bps: self.maker_reward_bps,
            taker_reward_bps: self.taker_reward_bps,
            default_denom: self.default_denom,
        })
    }
}

impl Config<Addr> {
    pub fn save(&self, storage: &mut dyn Storage) -> Result<(), ContractError> {
        ensure!(
            self.protocol_fee_bps < MAX_BASIS_POINTS,
            ContractError::InvalidInput("trade_fee_bps must be less than 1".to_string())
        );
        ensure!(
            self.non_native_protocol_fee_bps < MAX_BASIS_POINTS,
            ContractError::InvalidInput("non_native_trade_fee_bps must be less than 1".to_string())
        );
        ensure!(
            (self.maker_reward_bps + self.taker_reward_bps) < MAX_BASIS_POINTS,
            ContractError::InvalidInput(
                "taker and maker reward bps must be less than 1 combined".to_string()
            )
        );

        CONFIG.save(storage, self)?;
        Ok(())
    }
}

pub const CONFIG: Item<Config<Addr>> = Item::new("C");

pub const COLLECTION_DENOMS: Map<Addr, Denom> = Map::new("D");

pub const LISTING_FEES: Map<Denom, Uint128> = Map::new("L");

pub const NONCE: Item<u64> = Item::new("N");

/// Defines indices for accessing Asks
pub struct AskIndices<'a> {
    // Index Asks by collection and denom price
    pub collection_denom_price: MultiIndex<'a, (Addr, Denom, u128), Ask, OrderId>,
    // Index Asks by creator and collection
    pub creator_collection: MultiIndex<'a, (Addr, Addr), Ask, OrderId>,
}

impl<'a> IndexList<Ask> for AskIndices<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Ask>> + '_> {
        let v: Vec<&dyn Index<Ask>> = vec![&self.collection_denom_price, &self.creator_collection];
        Box::new(v.into_iter())
    }
}

pub fn asks<'a>() -> IndexedMap<'a, OrderId, Ask, AskIndices<'a>> {
    let indexes: AskIndices = AskIndices {
        collection_denom_price: MultiIndex::new(
            |_pk: &[u8], a: &Ask| {
                (
                    a.collection.clone(),
                    a.details.price.denom.clone(),
                    a.details.price.amount.u128(),
                )
            },
            "a",
            "a_p",
        ),
        creator_collection: MultiIndex::new(
            |_pk: &[u8], a: &Ask| (a.creator.clone(), a.collection.clone()),
            "a",
            "a_c",
        ),
    };
    IndexedMap::new("a", indexes)
}

/// Defines incides for accessing bids
pub struct BidIndices<'a> {
    // Index bids for a token id, sorted by denom price (infinity router dependency)
    pub token_denom_price: MultiIndex<'a, (TokenId, Denom, u128), Bid, OrderId>,
    // Index bids by creator and collection
    pub creator_collection: MultiIndex<'a, (Addr, Addr), Bid, OrderId>,
}

impl<'a> IndexList<Bid> for BidIndices<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Bid>> + '_> {
        let v: Vec<&dyn Index<Bid>> = vec![&self.token_denom_price, &self.creator_collection];
        Box::new(v.into_iter())
    }
}

pub fn bids<'a>() -> IndexedMap<'a, OrderId, Bid, BidIndices<'a>> {
    let indexes = BidIndices {
        token_denom_price: MultiIndex::new(
            |_pk: &[u8], o: &Bid| {
                (
                    build_collection_token_index_str(o.collection.as_ref(), &o.token_id),
                    o.details.price.denom.clone(),
                    o.details.price.amount.u128(),
                )
            },
            "o",
            "o_p",
        ),
        creator_collection: MultiIndex::new(
            |_pk: &[u8], o: &Bid| (o.creator.clone(), o.collection.clone()),
            "o",
            "o_c",
        ),
    };
    IndexedMap::new("o", indexes)
}

/// Defines incides for accessing collection bids
pub struct CollectionBidIndices<'a> {
    // Index collection bids by collection and price
    pub collection_denom_price: MultiIndex<'a, (Addr, Denom, u128), CollectionBid, OrderId>,
    // Index collection bids by creator
    pub creator_collection: MultiIndex<'a, (Addr, Addr), CollectionBid, OrderId>,
}

impl<'a> IndexList<CollectionBid> for CollectionBidIndices<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<CollectionBid>> + '_> {
        let v: Vec<&dyn Index<CollectionBid>> =
            vec![&self.collection_denom_price, &self.creator_collection];
        Box::new(v.into_iter())
    }
}

pub fn collection_bids<'a>() -> IndexedMap<'a, OrderId, CollectionBid, CollectionBidIndices<'a>> {
    let indexes = CollectionBidIndices {
        collection_denom_price: MultiIndex::new(
            |_pk: &[u8], co: &CollectionBid| {
                (
                    co.collection.clone(),
                    co.details.price.denom.clone(),
                    co.details.price.amount.u128(),
                )
            },
            "c",
            "c_p",
        ),
        creator_collection: MultiIndex::new(
            |_pk: &[u8], co: &CollectionBid| (co.creator.clone(), co.collection.clone()),
            "c",
            "c_c",
        ),
    };
    IndexedMap::new("c", indexes)
}
