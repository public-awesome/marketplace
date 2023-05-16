use cosmwasm_schema::cw_serde;
use cosmwasm_std::{coin, ensure, Addr, Coin, Decimal, Storage, Timestamp, Uint128};
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
    pub max_duration: u64,
    pub extend_duration: u64,
    pub create_auction_fee: Uint128,
    pub max_auctions_to_settle_per_block: u64,
}

impl Config {
    pub fn min_reserve_price_coin(&self) -> Coin {
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
        ensure!(
            !self.min_reserve_price.is_zero(),
            ContractError::InvalidConfig("min_reserve_price must be greater than zero".to_string(),)
        );
        ensure!(
            !self.min_bid_increment_pct.is_zero(),
            ContractError::InvalidConfig(
                "min_bid_increment_pct must be greater than zero".to_string(),
            )
        );
        ensure!(
            self.min_bid_increment_pct < Decimal::percent(10000),
            ContractError::InvalidConfig(
                "min_bid_increment_pct must be less than 100%".to_string(),
            )
        );
        ensure!(
            self.min_duration > 0,
            ContractError::InvalidConfig("min_duration must be greater than zero".to_string(),)
        );
        ensure!(
            self.extend_duration > 0,
            ContractError::InvalidConfig("extend_duration must be greater than zero".to_string(),)
        );
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
    pub duration: u64, // in seconds
    pub end_time: Option<Timestamp>,
    pub seller_funds_recipient: Option<Addr>,
    pub high_bid: Option<HighBid>,
    pub first_bid_time: Option<Timestamp>,
}

impl Auction {
    pub fn min_bid_coin(&self, min_bid_increment_pct: Decimal) -> Coin {
        let amount = match &self.high_bid {
            Some(high_bid) => {
                let next_min_bid = high_bid.coin.amount
                    * (Decimal::percent(10000) + min_bid_increment_pct)
                    / Uint128::from(100u128);
                max(next_min_bid, high_bid.coin.amount + Uint128::one())
            }
            None => self.reserve_price.amount,
        };
        coin(amount.u128(), NATIVE_DENOM)
    }
}

pub type AuctionKey = (Addr, String);

#[index_list(Auction)]
pub struct AuctionIndexes<'a> {
    pub seller: MultiIndex<'a, String, Auction, AuctionKey>,
    pub end_time: MultiIndex<'a, u64, Auction, AuctionKey>,
}

pub fn auctions<'a>() -> IndexedMap<'a, AuctionKey, Auction, AuctionIndexes<'a>> {
    let indexes = AuctionIndexes {
        seller: MultiIndex::new(|_pk: &[u8], a: &Auction| a.seller.to_string(), "a", "a__s"),
        end_time: MultiIndex::new(
            |_pk: &[u8], a: &Auction| a.end_time.map_or(u64::MAX, |et| et.seconds()),
            "a",
            "a__et",
        ),
    };
    IndexedMap::new("a", indexes)
}
