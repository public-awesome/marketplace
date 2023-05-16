use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Decimal, Storage, Timestamp, Uint128};
use cw_storage_macro::index_list;
use cw_storage_plus::{IndexedMap, Item, MultiIndex};
use sg_std::NATIVE_DENOM;
use std::cmp::max;

use crate::ContractError;

#[cw_serde]
pub struct Config {
    pub marketplace: Addr,
    pub min_reserve_price: Uint128,
    pub min_bid_increment_pct: Decimal,
    pub min_duration: u64,
    pub extend_duration: u64,
    pub create_auction_fee: Uint128,
    pub max_auctions_to_settle_per_block: u64,
}

impl Config {
    pub fn coin_min_reserve_price(&self) -> Coin {
        Coin {
            denom: NATIVE_DENOM.to_string(),
            amount: self.min_reserve_price,
        }
    }

    pub fn save(&self, storage: &mut dyn Storage) -> Result<(), ContractError> {
        self.validate()?;
        CONFIG.save(storage, self)?;
        Ok(())
    }

    fn validate(&self) -> Result<(), ContractError> {
        if self.min_reserve_price.is_zero() {
            return Err(ContractError::InvalidConfig(
                "min_reserve_price must be greater than zero".to_string(),
            ));
        }
        if self.min_bid_increment_pct.is_zero() {
            return Err(ContractError::InvalidConfig(
                "min_bid_increment_pct must be greater than zero".to_string(),
            ));
        }
        if self.min_bid_increment_pct >= Decimal::percent(10000) {
            return Err(ContractError::InvalidConfig(
                "min_bid_increment_pct must be less than 100%".to_string(),
            ));
        }
        if self.min_duration == 0 {
            return Err(ContractError::InvalidConfig(
                "min_duration must be greater than zero".to_string(),
            ));
        }
        if self.extend_duration == 0 {
            return Err(ContractError::InvalidConfig(
                "extend_duration must be greater than zero".to_string(),
            ));
        }
        Ok(())
    }
}

pub const CONFIG: Item<Config> = Item::new("config");

#[cw_serde]
pub struct HighBid {
    pub coin: Coin,
    pub bidder: Addr,
}

#[cw_serde]
pub struct Auction {
    pub collection: Addr,
    pub token_id: String,
    pub seller: Addr,
    pub reserve_price: Coin,
    pub start_time: Timestamp,
    pub end_time: Timestamp,
    pub seller_funds_recipient: Option<Addr>,
    pub high_bid: Option<HighBid>,
    pub first_bid_time: Option<Timestamp>,
}

impl Auction {
    pub fn min_bid(&self, min_bid_increment_pct: Decimal) -> Uint128 {
        if let Some(_high_bid) = &self.high_bid {
            let next_min_bid = _high_bid.coin.amount
                * (Decimal::percent(10000) + min_bid_increment_pct)
                / Uint128::from(100u128);
            max(next_min_bid, _high_bid.coin.amount + Uint128::one())
        } else {
            self.reserve_price.amount
        }
    }
}

pub type TokenId = String;
pub type Collection = Addr;
pub type AuctionKey = (Collection, TokenId);

#[index_list(Auction)]
pub struct AuctionIndexes<'a> {
    pub seller: MultiIndex<'a, String, Auction, AuctionKey>,
    pub end_time: MultiIndex<'a, u64, Auction, AuctionKey>,
}

pub fn auctions<'a>() -> IndexedMap<'a, AuctionKey, Auction, AuctionIndexes<'a>> {
    let indexes = AuctionIndexes {
        seller: MultiIndex::new(
            |_pk: &[u8], d: &Auction| d.seller.to_string(),
            "auctions",
            "auctions__seller",
        ),
        end_time: MultiIndex::new(
            |_pk: &[u8], d: &Auction| d.end_time.seconds(),
            "auctions",
            "auctions__end_time",
        ),
    };
    IndexedMap::new("auctions", indexes)
}
