use crate::{
    helpers::generate_id,
    msg::PriceOffset,
    query::{
        query_asks_by_collection_denom, query_collection_offers_by_price,
        query_offers_by_token_price,
    },
    state::{asks, collection_offers, offers, TokenId},
    ContractError,
};

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{attr, has_coins, Addr, Api, Attribute, Coin, Deps, StdResult, Storage};
use cw_address_like::AddressLike;
use cw_utils::maybe_addr;
use sg_index_query::{QueryBound, QueryOptions};
use sg_marketplace_common::address::address_or;

#[cw_serde]
pub struct OrderDetails<T: AddressLike> {
    pub price: Coin,
    pub asset_recipient: Option<T>,
    pub finder: Option<T>,
}

impl OrderDetails<String> {
    pub fn str_to_addr(self, api: &dyn Api) -> StdResult<OrderDetails<Addr>> {
        Ok(OrderDetails {
            price: self.price,
            asset_recipient: maybe_addr(api, self.asset_recipient)?,
            finder: maybe_addr(api, self.finder)?,
        })
    }
}

pub enum MatchingOffer {
    Offer(Offer),
    CollectionOffer(CollectionOffer),
}

#[cw_serde]
pub struct Ask {
    pub id: String,
    pub creator: Addr,
    pub collection: Addr,
    pub token_id: TokenId,
    pub details: OrderDetails<Addr>,
}

impl Ask {
    pub fn new(
        creator: Addr,
        collection: Addr,
        token_id: TokenId,
        details: OrderDetails<Addr>,
    ) -> Self {
        Self {
            id: generate_id(vec![collection.as_bytes(), token_id.as_bytes()]),
            creator,
            collection,
            token_id,
            details,
        }
    }

    pub fn asset_recipient(&self) -> Addr {
        address_or(self.details.asset_recipient.as_ref(), &self.creator)
    }

    pub fn save(&self, storage: &mut dyn Storage) -> Result<(), ContractError> {
        asks().save(storage, self.id.clone(), self)?;
        Ok(())
    }

    pub fn remove(&self, storage: &mut dyn Storage) -> Result<(), ContractError> {
        asks().remove(storage, self.id.clone())?;
        Ok(())
    }

    pub fn match_with_offer(&self, deps: Deps) -> Result<Option<MatchingOffer>, ContractError> {
        let top_offer = query_offers_by_token_price(
            deps,
            self.collection.clone(),
            self.token_id.clone(),
            self.details.price.denom.clone(),
            QueryOptions {
                descending: Some(true),
                limit: Some(1),
                min: Some(QueryBound::Inclusive(PriceOffset {
                    id: "".to_string(),
                    amount: self.details.price.amount.u128(),
                })),
                max: None,
            },
        )?
        .pop();

        let top_collection_offer = query_collection_offers_by_price(
            deps,
            self.collection.clone(),
            self.details.price.denom.clone(),
            QueryOptions {
                descending: Some(true),
                limit: Some(1),
                min: Some(QueryBound::Inclusive(PriceOffset {
                    id: "".to_string(),
                    amount: self.details.price.amount.u128(),
                })),
                max: None,
            },
        )?
        .pop();

        let result = match (top_offer, top_collection_offer) {
            (Some(offer), Some(collection_offer)) => {
                if offer.details.price.amount >= collection_offer.details.price.amount {
                    Some(MatchingOffer::Offer(offer))
                } else {
                    Some(MatchingOffer::CollectionOffer(collection_offer))
                }
            }
            (Some(offer), None) => Some(MatchingOffer::Offer(offer)),
            (None, Some(collection_offer)) => {
                Some(MatchingOffer::CollectionOffer(collection_offer))
            }
            (None, None) => None,
        };

        Ok(result)
    }

    pub fn get_event_attrs(&self, attr_keys: Vec<&str>) -> Vec<Attribute> {
        let mut attributes = vec![];
        for attr_key in attr_keys {
            let attr = match attr_key {
                "id" => Some(attr("id", self.id.to_string())),
                "creator" => Some(attr("creator", self.creator.to_string())),
                "collection" => Some(attr("collection", self.collection.to_string())),
                "token_id" => Some(attr("token_id", self.token_id.to_string())),
                "price" => Some(attr("price", self.details.price.to_string())),
                "asset_recipient" => self
                    .details
                    .asset_recipient
                    .as_ref()
                    .map(|asset_recipient| attr("asset_recipient", asset_recipient.to_string())),
                "finder" => self
                    .details
                    .finder
                    .as_ref()
                    .map(|finder| attr("finder", finder.to_string())),
                &_ => {
                    unreachable!("Invalid attr_key: {}", attr_key)
                }
            };
            if let Some(value) = attr {
                attributes.push(value);
            }
        }
        attributes
    }
}

#[cw_serde]
pub struct Offer {
    pub id: String,
    pub creator: Addr,
    pub collection: Addr,
    pub token_id: TokenId,
    pub details: OrderDetails<Addr>,
}

impl Offer {
    pub fn new(
        creator: Addr,
        collection: Addr,
        token_id: TokenId,
        details: OrderDetails<Addr>,
        height: u64,
        nonce: u64,
    ) -> Self {
        Self {
            id: generate_id(vec![
                collection.as_bytes(),
                token_id.as_bytes(),
                height.to_be_bytes().as_ref(),
                nonce.to_be_bytes().as_ref(),
            ]),
            creator,
            collection,
            token_id,
            details,
        }
    }

    pub fn asset_recipient(&self) -> Addr {
        address_or(self.details.asset_recipient.as_ref(), &self.creator)
    }

    pub fn save(&self, storage: &mut dyn Storage) -> Result<(), ContractError> {
        offers().save(storage, self.id.clone(), self)?;
        Ok(())
    }

    pub fn remove(&self, storage: &mut dyn Storage) -> Result<(), ContractError> {
        offers().remove(storage, self.id.clone())?;
        Ok(())
    }

    pub fn match_with_ask(&self, deps: Deps) -> Result<Option<Ask>, ContractError> {
        let ask_id: String =
            generate_id(vec![self.collection.as_bytes(), self.token_id.as_bytes()]);
        let ask_option = asks().may_load(deps.storage, ask_id)?;

        if let Some(ask) = ask_option {
            if has_coins(&[self.details.price.clone()], &ask.details.price) {
                return Ok(Some(ask));
            }
        };

        Ok(None)
    }

    pub fn get_event_attrs(&self, attr_keys: Vec<&str>) -> Vec<Attribute> {
        let mut attributes = vec![];
        for attr_key in attr_keys {
            let attr = match attr_key {
                "id" => Some(attr("id", self.id.to_string())),
                "creator" => Some(attr("creator", self.creator.to_string())),
                "collection" => Some(attr("collection", self.collection.to_string())),
                "token_id" => Some(attr("token_id", self.token_id.to_string())),
                "price" => Some(attr("price", self.details.price.to_string())),
                "asset_recipient" => self
                    .details
                    .asset_recipient
                    .as_ref()
                    .map(|asset_recipient| attr("asset_recipient", asset_recipient.to_string())),
                "finder" => self
                    .details
                    .finder
                    .as_ref()
                    .map(|finder| attr("finder", finder.to_string())),
                &_ => {
                    unreachable!("Invalid attr_key: {}", attr_key)
                }
            };
            if let Some(value) = attr {
                attributes.push(value);
            }
        }
        attributes
    }
}

#[cw_serde]
pub struct CollectionOffer {
    pub id: String,
    pub creator: Addr,
    pub collection: Addr,
    pub details: OrderDetails<Addr>,
}

impl CollectionOffer {
    pub fn new(
        creator: Addr,
        collection: Addr,
        details: OrderDetails<Addr>,
        height: u64,
        nonce: u64,
    ) -> Self {
        Self {
            id: generate_id(vec![
                collection.as_bytes(),
                height.to_be_bytes().as_ref(),
                nonce.to_be_bytes().as_ref(),
            ]),
            creator,
            collection,
            details,
        }
    }

    pub fn asset_recipient(&self) -> Addr {
        address_or(self.details.asset_recipient.as_ref(), &self.creator)
    }

    pub fn save(&self, storage: &mut dyn Storage) -> Result<(), ContractError> {
        collection_offers().save(storage, self.id.clone(), self)?;
        Ok(())
    }

    pub fn remove(&self, storage: &mut dyn Storage) -> Result<(), ContractError> {
        collection_offers().remove(storage, self.id.clone())?;
        Ok(())
    }

    pub fn match_with_ask(&self, deps: Deps) -> Result<Option<Ask>, ContractError> {
        let top_ask = query_asks_by_collection_denom(
            deps,
            self.collection.clone(),
            self.details.price.denom.clone(),
            QueryOptions {
                descending: Some(false),
                limit: Some(1),
                min: None,
                max: Some(QueryBound::Exclusive(PriceOffset {
                    id: "".to_string(),
                    amount: self.details.price.amount.u128() + 1,
                })),
            },
        )?
        .pop();

        Ok(top_ask)
    }

    pub fn get_event_attrs(&self, attr_keys: Vec<&str>) -> Vec<Attribute> {
        let mut attributes = vec![];
        for attr_key in attr_keys {
            let attr = match attr_key {
                "id" => Some(attr("id", self.id.to_string())),
                "creator" => Some(attr("creator", self.creator.to_string())),
                "collection" => Some(attr("collection", self.collection.to_string())),
                "price" => Some(attr("price", self.details.price.to_string())),
                "asset_recipient" => self
                    .details
                    .asset_recipient
                    .as_ref()
                    .map(|asset_recipient| attr("asset_recipient", asset_recipient.to_string())),
                "finder" => self
                    .details
                    .finder
                    .as_ref()
                    .map(|finder| attr("finder", finder.to_string())),
                &_ => {
                    unreachable!("Invalid attr_key: {}", attr_key)
                }
            };
            if let Some(value) = attr {
                attributes.push(value);
            }
        }
        attributes
    }
}
