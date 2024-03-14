use crate::{
    msg::{AsksByPriceOffset, CollectionOffersByPriceOffset, OffersByTokenPriceOffset},
    query::{query_asks_by_price, query_collection_offers_by_price, query_offers_by_token_price},
    state::{
        asks, collection_offers, offers, Ask, AskKey, CollectionOffer, CollectionOfferKey,
        ExpirationInfo, Offer, OfferKey, OrderInfo, TokenId,
    },
    ContractError,
};

use cosmwasm_std::{
    attr, has_coins, Addr, Attribute, BlockInfo, Coin, Deps, Env, Storage, Uint128,
};
use cw_utils::NativeBalance;
use sg_index_query::{QueryBound, QueryOptions};
use sg_marketplace_common::{
    address::address_or,
    coin::{transfer_coin, transfer_coins},
    nft::transfer_nft,
};
use sg_std::Response;
use std::ops::AddAssign;

pub enum MatchingOffer {
    Offer(Offer),
    CollectionOffer(CollectionOffer),
}

pub enum RewardPayout {
    Contract,
    Return,
    Other(Addr),
}

impl OrderInfo {
    pub fn asset_recipient(&self) -> Addr {
        address_or(self.asset_recipient.as_ref(), &self.creator)
    }

    pub fn is_expired(&self, block_info: &BlockInfo) -> bool {
        if let Some(expiration_info) = &self.expiration_info {
            expiration_info.expiration <= block_info.time
        } else {
            false
        }
    }
}

impl Ask {
    pub fn new(
        collection: Addr,
        token_id: TokenId,
        price: Coin,
        creator: Addr,
        asset_recipient: Option<Addr>,
        finders_fee_bps: Option<u64>,
        expiration_info: Option<ExpirationInfo>,
    ) -> Self {
        Ask {
            collection,
            token_id,
            order_info: OrderInfo {
                price,
                creator,
                asset_recipient,
                finders_fee_bps,
                expiration_info,
            },
        }
    }

    pub fn build_key(collection: &Addr, token_id: &TokenId) -> AskKey {
        (collection.to_string(), token_id.clone())
    }

    pub fn key(&self) -> AskKey {
        Self::build_key(&self.collection, &self.token_id)
    }

    pub fn save(&self, storage: &mut dyn Storage) -> Result<(), ContractError> {
        asks().save(storage, self.key(), self)?;
        Ok(())
    }

    pub fn remove(
        &self,
        storage: &mut dyn Storage,
        refund_nft: bool,
        reward_payout: RewardPayout,
        mut response: Response,
    ) -> Result<Response, ContractError> {
        asks().remove(storage, self.key())?;

        let return_address = self.order_info.asset_recipient();

        if refund_nft {
            response = transfer_nft(&self.collection, &self.token_id, &return_address, response);
        }

        if let Some(expiration_info) = &self.order_info.expiration_info {
            if expiration_info.removal_reward.amount > Uint128::zero() {
                let reward = expiration_info.removal_reward.clone();
                match &reward_payout {
                    RewardPayout::Contract => {}
                    RewardPayout::Return => {
                        response = transfer_coin(reward, &return_address, response);
                    }
                    RewardPayout::Other(recipient) => {
                        response = transfer_coin(reward, recipient, response);
                    }
                }
            }
        }

        Ok(response)
    }

    pub fn match_with_offer(
        &self,
        deps: Deps,
        _env: &Env,
    ) -> Result<Option<MatchingOffer>, ContractError> {
        let top_offer = query_offers_by_token_price(
            deps,
            self.collection.clone(),
            self.token_id.clone(),
            self.order_info.price.denom.clone(),
            QueryOptions {
                descending: Some(true),
                limit: Some(1),
                min: Some(QueryBound::Inclusive(OffersByTokenPriceOffset {
                    creator: "".to_string(),
                    amount: self.order_info.price.amount.u128(),
                })),
                max: None,
            },
        )?
        .pop();

        let top_collection_offer = query_collection_offers_by_price(
            deps,
            self.collection.clone(),
            self.order_info.price.denom.clone(),
            QueryOptions {
                descending: Some(true),
                limit: Some(1),
                min: Some(QueryBound::Inclusive(CollectionOffersByPriceOffset {
                    creator: "".to_string(),
                    amount: self.order_info.price.amount.u128(),
                })),
                max: None,
            },
        )?
        .pop();

        let result = match (top_offer, top_collection_offer) {
            (Some(offer), Some(collection_offer)) => {
                if offer.order_info.price.amount >= collection_offer.order_info.price.amount {
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
                "collection" => Some(attr("collection", self.collection.to_string())),
                "token_id" => Some(attr("token_id", self.token_id.to_string())),
                "creator" => Some(attr("creator", self.order_info.creator.to_string())),
                "price" => Some(attr("price", self.order_info.price.to_string())),
                "asset_recipient" => self
                    .order_info
                    .asset_recipient
                    .as_ref()
                    .map(|asset_recipient| attr("asset_recipient", asset_recipient.to_string())),
                "finders_fee_bps" => self
                    .order_info
                    .finders_fee_bps
                    .as_ref()
                    .map(|finders_fee_bps| attr("finders_fee_bps", finders_fee_bps.to_string())),
                "expiration" => self
                    .order_info
                    .expiration_info
                    .as_ref()
                    .map(|expiration_info| {
                        attr("expiration_info", expiration_info.expiration.to_string())
                    }),
                "removal_reward" => {
                    self.order_info
                        .expiration_info
                        .as_ref()
                        .map(|expiration_info| {
                            attr(
                                "expiration_info",
                                expiration_info.removal_reward.to_string(),
                            )
                        })
                }
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

impl Offer {
    pub fn new(
        collection: Addr,
        token_id: TokenId,
        price: Coin,
        creator: Addr,
        asset_recipient: Option<Addr>,
        finders_fee_bps: Option<u64>,
        expiration_info: Option<ExpirationInfo>,
    ) -> Self {
        Self {
            collection,
            token_id,
            order_info: OrderInfo {
                price,
                creator,
                asset_recipient,
                finders_fee_bps,
                expiration_info,
            },
        }
    }

    pub fn build_key(collection: &Addr, token_id: &TokenId, creator: &Addr) -> OfferKey {
        (collection.clone(), token_id.clone(), creator.clone())
    }

    pub fn key(&self) -> OfferKey {
        Self::build_key(&self.collection, &self.token_id, &self.order_info.creator)
    }

    pub fn save(&self, storage: &mut dyn Storage) -> Result<(), ContractError> {
        offers().save(storage, self.key(), self)?;
        Ok(())
    }

    pub fn remove(
        &self,
        storage: &mut dyn Storage,
        refund_offer: bool,
        reward_payout: RewardPayout,
        mut response: Response,
    ) -> Result<Response, ContractError> {
        offers().remove(storage, self.key())?;

        let return_address = self.order_info.asset_recipient();
        let mut funds_to_return = NativeBalance(vec![]);

        if refund_offer {
            funds_to_return.add_assign(self.order_info.price.clone());
        }

        if let Some(expiration_info) = &self.order_info.expiration_info {
            if expiration_info.removal_reward.amount > Uint128::zero() {
                let reward = expiration_info.removal_reward.clone();
                match &reward_payout {
                    RewardPayout::Contract => {}
                    RewardPayout::Return => {
                        funds_to_return.add_assign(reward);
                    }
                    RewardPayout::Other(recipient) => {
                        response = transfer_coin(reward, recipient, response);
                    }
                }
            }
        }

        if !funds_to_return.is_empty() {
            funds_to_return.normalize();
            response = transfer_coins(funds_to_return.into_vec(), &return_address, response);
        }

        Ok(response)
    }

    pub fn match_with_ask(&self, deps: Deps, _env: &Env) -> Result<Option<Ask>, ContractError> {
        let ask_key = Ask::build_key(&self.collection, &self.token_id);
        let ask_option = asks().may_load(deps.storage, ask_key)?;

        if let Some(ask) = ask_option {
            if has_coins(&[self.order_info.price.clone()], &ask.order_info.price) {
                return Ok(Some(ask));
            }
        };

        Ok(None)
    }
}

impl CollectionOffer {
    pub fn new(
        collection: Addr,
        price: Coin,
        creator: Addr,
        asset_recipient: Option<Addr>,
        finders_fee_bps: Option<u64>,
        expiration_info: Option<ExpirationInfo>,
    ) -> Self {
        Self {
            collection,
            order_info: OrderInfo {
                price,
                creator,
                asset_recipient,
                finders_fee_bps,
                expiration_info,
            },
        }
    }

    pub fn build_key(collection: &Addr, creator: &Addr) -> CollectionOfferKey {
        (collection.clone(), creator.clone())
    }

    pub fn key(&self) -> CollectionOfferKey {
        Self::build_key(&self.collection, &self.order_info.creator)
    }

    pub fn save(&self, storage: &mut dyn Storage) -> Result<(), ContractError> {
        collection_offers().save(storage, self.key(), self)?;
        Ok(())
    }

    pub fn remove(
        &self,
        storage: &mut dyn Storage,
        refund_offer: bool,
        reward_payout: RewardPayout,
        mut response: Response,
    ) -> Result<Response, ContractError> {
        collection_offers().remove(storage, self.key())?;

        let return_address = self.order_info.asset_recipient();
        let mut funds_to_return = NativeBalance(vec![]);

        if refund_offer {
            funds_to_return.add_assign(self.order_info.price.clone());
        }

        if let Some(expiration_info) = &self.order_info.expiration_info {
            if expiration_info.removal_reward.amount > Uint128::zero() {
                let reward = expiration_info.removal_reward.clone();
                match &reward_payout {
                    RewardPayout::Contract => {}
                    RewardPayout::Return => {
                        funds_to_return.add_assign(reward);
                    }
                    RewardPayout::Other(recipient) => {
                        response = transfer_coin(reward, recipient, response);
                    }
                }
            }
        }

        if !funds_to_return.is_empty() {
            funds_to_return.normalize();
            response = transfer_coins(funds_to_return.into_vec(), &return_address, response);
        }

        Ok(response)
    }

    pub fn match_with_ask(&self, deps: Deps, _env: &Env) -> Result<Option<Ask>, ContractError> {
        let top_ask = query_asks_by_price(
            deps,
            self.collection.clone(),
            self.order_info.price.denom.clone(),
            QueryOptions {
                descending: Some(false),
                limit: Some(1),
                min: None,
                max: Some(QueryBound::Exclusive(AsksByPriceOffset {
                    token_id: "".to_string(),
                    amount: self.order_info.price.amount.u128() + 1,
                })),
            },
        )?
        .pop();

        Ok(top_ask)
    }
}
