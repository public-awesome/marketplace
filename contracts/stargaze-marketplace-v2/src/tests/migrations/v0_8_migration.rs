use crate::{
    helpers::generate_id,
    orders::Expiry,
    state::{asks, Denom, OrderId, TokenId},
    tests::setup::{
        setup_accounts::TestAccounts,
        templates::{test_context, TestContext, TestContracts},
    },
};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{testing::mock_dependencies, Order, StdResult, Timestamp};
use cosmwasm_std::{Addr, Coin};
use cw_address_like::AddressLike;
use cw_storage_plus::{Index, IndexList, IndexedMap, MultiIndex};
use sg_marketplace_common::constants::NATIVE_DENOM;

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
