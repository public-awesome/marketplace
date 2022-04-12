use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, MultiIndex};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Ask {
    pub collection: Addr,
    pub token_id: u32,
    pub seller: Addr,
    pub price: Uint128,
    pub funds_recipient: Option<Addr>,
}

pub type AskKey = (Addr, u32);

/// Defines incides for accessing Asks
pub struct AskIndicies<'a> {
    pub collection: MultiIndex<'a, Addr, Ask, AskKey>,
    pub seller: MultiIndex<'a, Addr, Ask, AskKey>,
}

impl<'a> IndexList<Ask> for AskIndicies<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Ask>> + '_> {
        let v: Vec<&dyn Index<Ask>> = vec![&self.collection, &self.seller];
        Box::new(v.into_iter())
    }
}

pub fn asks<'a>() -> IndexedMap<'a, AskKey, Ask, AskIndicies<'a>> {
    let indexes = AskIndicies {
        collection: MultiIndex::new(|d: &Ask| d.collection.clone(), "asks", "asks__collection"),
        seller: MultiIndex::new(|d: &Ask| d.seller.clone(), "asks", "asks__seller"),
    };
    IndexedMap::new("asks", indexes)
}

// // Mapping from (collection, token_id, bidder) to bid amount
// pub const TOKEN_BIDS: Map<(&Addr, u32, &Addr), Bid> = Map::new("b");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Bid {
    pub collection: Addr,
    pub token_id: u32,
    pub bidder: Addr,
    pub price: Uint128,
}

/// (collection, token_id, bidder) uniquely identifies a bid
pub type BidKey = (Addr, u32, Addr);

/// Defines incides for accessing bids
pub struct BidIndicies<'a> {
    pub collection: MultiIndex<'a, Addr, Bid, BidKey>,
    pub bidder: MultiIndex<'a, Addr, Bid, BidKey>,
}

impl<'a> IndexList<Bid> for BidIndicies<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Bid>> + '_> {
        let v: Vec<&dyn Index<Bid>> = vec![&self.collection, &self.bidder];
        Box::new(v.into_iter())
    }
}

pub fn bids<'a>() -> IndexedMap<'a, BidKey, Bid, BidIndicies<'a>> {
    let indexes = BidIndicies {
        collection: MultiIndex::new(|d: &Bid| d.collection.clone(), "bids", "bids__collection"),
        bidder: MultiIndex::new(|d: &Bid| d.bidder.clone(), "bids", "bids__bidder"),
    };
    IndexedMap::new("bids", indexes)
}
