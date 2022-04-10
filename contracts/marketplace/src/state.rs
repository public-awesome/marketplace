use cosmwasm_std::{Addr, StdResult, Storage, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex, UniqueIndex};
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

pub type Bid = Uint128;

// Mapping from (collection, token_id, bidder) to bid amount
pub const TOKEN_BIDS: Map<(&Addr, u32, &Addr), Bid> = Map::new("b");

// Mapping from (collection, token_id) to the current ask for the token
pub const TOKEN_ASKS: Map<(&Addr, u32), Ask> = Map::new("a");

// (collection, token_id) -> Ask
// (collection) -> [Ask]
// pub const TOKEN_ASKS: Map<(&Addr, u32), Ask> = Map::new("a");
// (seller, collection, token_id) -> Ask
// (seller, collection) -> [Ask]
// (seller) -> [Ask]

/// Primary key for Asks
pub const ASK_COUNT: Item<u64> = Item::new("ask_count");

pub fn ask_count(storage: &dyn Storage) -> StdResult<u64> {
    Ok(ASK_COUNT.may_load(storage)?.unwrap_or_default())
}

pub fn increment_asks(storage: &mut dyn Storage) -> StdResult<u64> {
    let val = ask_count(storage)? + 1;
    ASK_COUNT.save(storage, &val)?;
    Ok(val)
}

pub fn decrement_asks(storage: &mut dyn Storage) -> StdResult<u64> {
    let val = ask_count(storage)? - 1;
    ASK_COUNT.save(storage, &val)?;
    Ok(val)
}

// TODO: do we need both collection and collection_token?
// Can't we just have collection_token and prefix iterate over the collection?
/// Defines incides for accessing Asks
pub struct AskIndicies<'a> {
    // (collection) -> [Ask]
    pub collection: MultiIndex<'a, Addr, Ask, u64>,
    // (collection, token_id) -> Ask
    pub collection_token: UniqueIndex<'a, (Addr, u32), Ask, u64>,
    // (seller) -> [Ask]
    pub seller: MultiIndex<'a, Addr, Ask, u64>,
}

impl<'a> IndexList<Ask> for AskIndicies<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Ask>> + '_> {
        let v: Vec<&dyn Index<Ask>> = vec![&self.collection, &self.collection_token, &self.seller];
        Box::new(v.into_iter())
    }
}

pub fn asks<'a>() -> IndexedMap<'a, u64, Ask, AskIndicies<'a>> {
    let indexes = AskIndicies {
        collection: MultiIndex::new(|d: &Ask| d.collection.clone(), "asks", "asks__collection"),
        collection_token: UniqueIndex::new(
            |d: &Ask| (d.collection.clone(), d.token_id),
            "asks__collection_token",
        ),
        seller: MultiIndex::new(|d: &Ask| d.seller.clone(), "asks", "asks__seller"),
    };
    IndexedMap::new("asks", indexes)
}
