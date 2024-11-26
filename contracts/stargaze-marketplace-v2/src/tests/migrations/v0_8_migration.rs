use crate::{
    helpers::generate_id,
    migrate::{migrate, MigrateMsg, V0_7Config, V0_7CONFIG},
    orders::Expiry,
    state::{asks, bids, collection_bids, Denom, OrderId, TokenId, CONFIG},
    tests::setup::{
        setup_accounts::TestAccounts,
        templates::{test_context, TestContext, TestContracts},
    },
};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    testing::{mock_dependencies, mock_env},
    Order, StdResult, Timestamp,
};
use cosmwasm_std::{Addr, Coin};
use cw_address_like::AddressLike;
use cw_storage_plus::{Index, IndexList, IndexedMap, MultiIndex};
use sg_marketplace_common::constants::NATIVE_DENOM;

#[test]
fn try_handle_v0_8_migration() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let fee_manager = Addr::unchecked("fee_manager");
    let royalty_registry = Addr::unchecked("royalty_registry");

    V0_7CONFIG
        .save(
            &mut deps.storage,
            &V0_7Config {
                fee_manager: fee_manager.clone(),
                royalty_registry: royalty_registry.clone(),
                protocol_fee_bps: 200,
                max_royalty_fee_bps: 1000,
                maker_reward_bps: 4000,
                taker_reward_bps: 1000,
                default_denom: NATIVE_DENOM.to_string(),
            },
        )
        .unwrap();

    let response = migrate(
        deps.as_mut(),
        env,
        MigrateMsg {
            max_asks_removed_per_block: 10,
            max_bids_removed_per_block: 10,
            max_collection_bids_removed_per_block: 10,
        },
    );
    assert!(response.is_ok());

    let config = CONFIG.load(&deps.storage).unwrap();
    assert_eq!(config.fee_manager, fee_manager);
    assert_eq!(config.royalty_registry, royalty_registry);
    assert_eq!(config.protocol_fee_bps, 200);
    assert_eq!(config.max_royalty_fee_bps, 1000);
    assert_eq!(config.maker_reward_bps, 4000);
    assert_eq!(config.taker_reward_bps, 1000);
    assert_eq!(config.default_denom, NATIVE_DENOM.to_string());
    assert_eq!(config.max_asks_removed_per_block, 10);
    assert_eq!(config.max_bids_removed_per_block, 10);
    assert_eq!(config.max_collection_bids_removed_per_block, 10);
}

#[cw_serde]
pub struct OrderDetails<T: AddressLike> {
    pub price: Coin,
    pub recipient: Option<T>,
    pub finder: Option<T>,
}

#[cw_serde]
pub struct V0_7Ask {
    pub id: String,
    pub creator: Addr,
    pub collection: Addr,
    pub token_id: TokenId,
    pub details: OrderDetails<Addr>,
}

/// Defines indices for accessing Asks
pub struct V0_7AskIndices<'a> {
    // Index Asks by collection and denom price
    pub collection_denom_price: MultiIndex<'a, (Addr, Denom, u128), V0_7Ask, OrderId>,
    // Index Asks by creator and collection
    pub creator_collection: MultiIndex<'a, (Addr, Addr), V0_7Ask, OrderId>,
}

impl<'a> IndexList<V0_7Ask> for V0_7AskIndices<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<V0_7Ask>> + '_> {
        let v: Vec<&dyn Index<V0_7Ask>> =
            vec![&self.collection_denom_price, &self.creator_collection];
        Box::new(v.into_iter())
    }
}

pub fn v0_7_asks<'a>() -> IndexedMap<'a, OrderId, V0_7Ask, V0_7AskIndices<'a>> {
    let indexes: V0_7AskIndices = V0_7AskIndices {
        collection_denom_price: MultiIndex::new(
            |_pk: &[u8], a: &V0_7Ask| {
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
            |_pk: &[u8], a: &V0_7Ask| (a.creator.clone(), a.collection.clone()),
            "a",
            "a_c",
        ),
    };
    IndexedMap::new("a", indexes)
}

#[test]
fn try_handle_v0_7_asks() {
    let TestContext {
        contracts: TestContracts { .. },
        accounts: TestAccounts { creator, .. },
        ..
    } = test_context();

    let collection = Addr::unchecked("collection");
    let token_id = "1".to_string();

    let v0_7_ask = V0_7Ask {
        id: generate_id(vec![collection.as_bytes(), token_id.as_bytes()]),
        creator,
        collection,
        token_id,
        details: OrderDetails::<Addr> {
            price: Coin::new(100, NATIVE_DENOM),
            recipient: None,
            finder: None,
        },
    };

    let mut deps = mock_dependencies();

    // Store legacy v0_7 ask
    v0_7_asks()
        .save(&mut deps.storage, v0_7_ask.id.clone(), &v0_7_ask)
        .unwrap();

    let ask = v0_7_asks()
        .load(&deps.storage, v0_7_ask.id.clone())
        .unwrap();

    assert_eq!(ask.id, v0_7_ask.id);
    assert_eq!(ask.creator, v0_7_ask.creator);
    assert_eq!(ask.collection, v0_7_ask.collection);
    assert_eq!(ask.token_id, v0_7_ask.token_id);
    assert_eq!(ask.details.price, v0_7_ask.details.price);

    // Load v0_7 ask in v0_8 format
    let mut ask = asks().load(&deps.storage, v0_7_ask.id.clone()).unwrap();
    assert_eq!(ask.id, v0_7_ask.id);
    assert_eq!(ask.creator, v0_7_ask.creator);
    assert_eq!(ask.collection, v0_7_ask.collection);
    assert_eq!(ask.token_id, v0_7_ask.token_id);
    assert_eq!(ask.details.price, v0_7_ask.details.price);
    assert_eq!(ask.details.expiry, None);

    // Ensure the expiry index is empty
    let results = asks()
        .idx
        .expiry_timestamp
        .range(&deps.storage, None, None, Order::Ascending)
        .take(1)
        .map(|res| res.map(|(_, ask)| ask))
        .collect::<StdResult<Vec<_>>>()
        .unwrap();

    assert_eq!(results.len(), 0);

    // Update v0_8 ask with expiry
    ask.details.expiry = Some(Expiry {
        timestamp: Timestamp::from_seconds(100),
        reward: Coin::new(30, NATIVE_DENOM),
    });
    asks()
        .save(&mut deps.storage, ask.id.clone(), &ask)
        .unwrap();

    // Load updated v0_8 ask
    let ask = asks().load(&deps.storage, v0_7_ask.id.clone()).unwrap();
    assert_eq!(
        ask.details.expiry,
        Some(Expiry {
            timestamp: Timestamp::from_seconds(100),
            reward: Coin::new(30, NATIVE_DENOM),
        })
    );

    // Check the expiry index for the ask
    let results = asks()
        .idx
        .expiry_timestamp
        .range(&deps.storage, None, None, Order::Ascending)
        .take(1)
        .map(|res| res.map(|(_, ask)| ask))
        .collect::<StdResult<Vec<_>>>()
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, ask.id);
}

#[cw_serde]
pub struct V0_7Bid {
    pub id: String,
    pub creator: Addr,
    pub collection: Addr,
    pub token_id: TokenId,
    pub details: OrderDetails<Addr>,
}

/// Defines indices for accessing Bids
pub struct V0_7BidIndices<'a> {
    // Index Bids by collection and denom price
    pub collection_denom_price: MultiIndex<'a, (Addr, Denom, u128), V0_7Bid, OrderId>,
    // Index Bids by creator and collection
    pub creator_collection: MultiIndex<'a, (Addr, Addr), V0_7Bid, OrderId>,
}

impl<'a> IndexList<V0_7Bid> for V0_7BidIndices<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<V0_7Bid>> + '_> {
        let v: Vec<&dyn Index<V0_7Bid>> =
            vec![&self.collection_denom_price, &self.creator_collection];
        Box::new(v.into_iter())
    }
}

pub fn v0_7_bids<'a>() -> IndexedMap<'a, OrderId, V0_7Bid, V0_7BidIndices<'a>> {
    let indexes: V0_7BidIndices = V0_7BidIndices {
        collection_denom_price: MultiIndex::new(
            |_pk: &[u8], o: &V0_7Bid| {
                (
                    o.collection.clone(),
                    o.details.price.denom.clone(),
                    o.details.price.amount.u128(),
                )
            },
            "o",
            "o_p",
        ),
        creator_collection: MultiIndex::new(
            |_pk: &[u8], o: &V0_7Bid| (o.creator.clone(), o.collection.clone()),
            "o",
            "o_c",
        ),
    };
    IndexedMap::new("o", indexes)
}

#[test]
fn try_handle_v0_7_bids() {
    let TestContext {
        contracts: TestContracts { .. },
        accounts: TestAccounts { creator, .. },
        ..
    } = test_context();

    let collection = Addr::unchecked("collection");
    let token_id = "1".to_string();

    let v0_7_bid = V0_7Bid {
        id: generate_id(vec![collection.as_bytes(), token_id.as_bytes()]),
        creator,
        collection,
        token_id,
        details: OrderDetails::<Addr> {
            price: Coin::new(100, NATIVE_DENOM),
            recipient: None,
            finder: None,
        },
    };

    let mut deps = mock_dependencies();

    // Store legacy v0_7 bid
    v0_7_bids()
        .save(&mut deps.storage, v0_7_bid.id.clone(), &v0_7_bid)
        .unwrap();

    let bid = v0_7_bids()
        .load(&deps.storage, v0_7_bid.id.clone())
        .unwrap();

    assert_eq!(bid.id, v0_7_bid.id);
    assert_eq!(bid.creator, v0_7_bid.creator);
    assert_eq!(bid.collection, v0_7_bid.collection);
    assert_eq!(bid.token_id, v0_7_bid.token_id);
    assert_eq!(bid.details.price, v0_7_bid.details.price);

    // Load v0_7 bid in v0_8 format
    let mut bid = bids().load(&deps.storage, v0_7_bid.id.clone()).unwrap();
    assert_eq!(bid.id, v0_7_bid.id);
    assert_eq!(bid.creator, v0_7_bid.creator);
    assert_eq!(bid.collection, v0_7_bid.collection);
    assert_eq!(bid.token_id, v0_7_bid.token_id);
    assert_eq!(bid.details.price, v0_7_bid.details.price);
    assert_eq!(bid.details.expiry, None);

    // Ensure the expiry index is empty
    let results = bids()
        .idx
        .expiry_timestamp
        .range(&deps.storage, None, None, Order::Ascending)
        .take(1)
        .map(|res| res.map(|(_, ask)| ask))
        .collect::<StdResult<Vec<_>>>()
        .unwrap();

    assert_eq!(results.len(), 0);

    // Update v0_8 bid with expiry
    bid.details.expiry = Some(Expiry {
        timestamp: Timestamp::from_seconds(100),
        reward: Coin::new(30, NATIVE_DENOM),
    });
    bids()
        .save(&mut deps.storage, bid.id.clone(), &bid)
        .unwrap();

    // Load updated v0_8 bid
    let bid = bids().load(&deps.storage, v0_7_bid.id.clone()).unwrap();
    assert_eq!(
        bid.details.expiry,
        Some(Expiry {
            timestamp: Timestamp::from_seconds(100),
            reward: Coin::new(30, NATIVE_DENOM),
        })
    );

    // Check the expiry index for the bid
    let results = bids()
        .idx
        .expiry_timestamp
        .range(&deps.storage, None, None, Order::Ascending)
        .take(1)
        .map(|res| res.map(|(_, ask)| ask))
        .collect::<StdResult<Vec<_>>>()
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, bid.id);
}

#[cw_serde]
pub struct V0_7CollectionBid {
    pub id: String,
    pub creator: Addr,
    pub collection: Addr,
    pub details: OrderDetails<Addr>,
}

/// Defines indices for accessing Bids
pub struct V0_7CollectionBidIndices<'a> {
    // Index Bids by collection and denom price
    pub collection_denom_price: MultiIndex<'a, (Addr, Denom, u128), V0_7CollectionBid, OrderId>,
    // Index Bids by creator and collection
    pub creator_collection: MultiIndex<'a, (Addr, Addr), V0_7CollectionBid, OrderId>,
}

impl<'a> IndexList<V0_7CollectionBid> for V0_7CollectionBidIndices<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<V0_7CollectionBid>> + '_> {
        let v: Vec<&dyn Index<V0_7CollectionBid>> =
            vec![&self.collection_denom_price, &self.creator_collection];
        Box::new(v.into_iter())
    }
}

pub fn v0_7_collection_bids<'a>(
) -> IndexedMap<'a, OrderId, V0_7CollectionBid, V0_7CollectionBidIndices<'a>> {
    let indexes: V0_7CollectionBidIndices = V0_7CollectionBidIndices {
        collection_denom_price: MultiIndex::new(
            |_pk: &[u8], co: &V0_7CollectionBid| {
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
            |_pk: &[u8], co: &V0_7CollectionBid| (co.creator.clone(), co.collection.clone()),
            "c",
            "c_c",
        ),
    };
    IndexedMap::new("c", indexes)
}

#[test]
fn try_handle_v0_7_collection_bids() {
    let TestContext {
        contracts: TestContracts { .. },
        accounts: TestAccounts { creator, .. },
        ..
    } = test_context();

    let collection = Addr::unchecked("collection");

    let v0_7_collection_bid = V0_7CollectionBid {
        id: generate_id(vec![collection.as_bytes()]),
        creator,
        collection,
        details: OrderDetails::<Addr> {
            price: Coin::new(100, NATIVE_DENOM),
            recipient: None,
            finder: None,
        },
    };

    let mut deps = mock_dependencies();

    // Store legacy v0_7 collection_bid
    v0_7_collection_bids()
        .save(
            &mut deps.storage,
            v0_7_collection_bid.id.clone(),
            &v0_7_collection_bid,
        )
        .unwrap();

    let collection_bid = v0_7_collection_bids()
        .load(&deps.storage, v0_7_collection_bid.id.clone())
        .unwrap();

    assert_eq!(collection_bid.id, v0_7_collection_bid.id);
    assert_eq!(collection_bid.creator, v0_7_collection_bid.creator);
    assert_eq!(collection_bid.collection, v0_7_collection_bid.collection);
    assert_eq!(
        collection_bid.details.price,
        v0_7_collection_bid.details.price
    );

    // Load v0_7 collection_bid in v0_8 format
    let mut collection_bid = collection_bids()
        .load(&deps.storage, v0_7_collection_bid.id.clone())
        .unwrap();
    assert_eq!(collection_bid.id, v0_7_collection_bid.id);
    assert_eq!(collection_bid.creator, v0_7_collection_bid.creator);
    assert_eq!(collection_bid.collection, v0_7_collection_bid.collection);
    assert_eq!(
        collection_bid.details.price,
        v0_7_collection_bid.details.price
    );
    assert_eq!(collection_bid.details.expiry, None);

    // Ensure the expiry index is empty
    let results = collection_bids()
        .idx
        .expiry_timestamp
        .range(&deps.storage, None, None, Order::Ascending)
        .take(1)
        .map(|res| res.map(|(_, ask)| ask))
        .collect::<StdResult<Vec<_>>>()
        .unwrap();

    assert_eq!(results.len(), 0);

    // Update v0_8 collection_bid with expiry
    collection_bid.details.expiry = Some(Expiry {
        timestamp: Timestamp::from_seconds(100),
        reward: Coin::new(30, NATIVE_DENOM),
    });
    collection_bids()
        .save(
            &mut deps.storage,
            collection_bid.id.clone(),
            &collection_bid,
        )
        .unwrap();

    // Load updated v0_8 collection_bid
    let collection_bid = collection_bids()
        .load(&deps.storage, v0_7_collection_bid.id.clone())
        .unwrap();
    assert_eq!(
        collection_bid.details.expiry,
        Some(Expiry {
            timestamp: Timestamp::from_seconds(100),
            reward: Coin::new(30, NATIVE_DENOM),
        })
    );

    // Check the expiry index for the collection_bid
    let results = collection_bids()
        .idx
        .expiry_timestamp
        .range(&deps.storage, None, None, Order::Ascending)
        .take(1)
        .map(|res| res.map(|(_, ask)| ask))
        .collect::<StdResult<Vec<_>>>()
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, collection_bid.id);
}
