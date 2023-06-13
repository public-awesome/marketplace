use cosmwasm_schema::cw_serde;
use cosmwasm_std::{coin, ensure, Addr, Coin, Decimal, Storage, Timestamp, Uint128};
use cw_storage_macro::index_list;
use cw_storage_plus::{IndexedMap, Item, Map, MultiIndex};
use std::cmp::max;

use crate::ContractError;

#[cw_serde]
pub struct Config {
    pub fair_burn: Addr,
    pub marketplace: Addr,
    pub min_bid_increment_pct: Decimal,
    pub min_duration: u64,
    pub max_duration: u64,
    pub extend_duration: u64,
    pub create_auction_fee: Coin,
    pub max_auctions_to_settle_per_block: u64,
    pub halt_duration_threshold: u64, // in seconds
    pub halt_buffer_duration: u64,    // in seconds
    pub halt_postpone_duration: u64,  // in seconds
}

impl Config {
    pub fn save(&self, storage: &mut dyn Storage) -> Result<(), ContractError> {
        self.validate()?;
        CONFIG.save(storage, self)?;
        Ok(())
    }

    fn validate(&self) -> Result<(), ContractError> {
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

pub const CONFIG: Item<Config> = Item::new("c");

// A map of acceptable denoms to their minimum reserve price.
// Denoms not found in the Map are not accepted.
pub const MIN_RESERVE_PRICES: Map<String, Uint128> = Map::new("mrp");

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
    pub fn denom(&self) -> String {
        self.reserve_price.denom.clone()
    }

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
        coin(amount.u128(), self.denom())
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

#[cw_serde]
pub struct HaltInfo {
    pub start_time: u64,  // in seconds
    pub halt_period: u64, // in seconds
    pub end_time: u64,
}

impl HaltInfo {
    pub fn new(start_time: u64, halt_period: u64, halt_buffer_duration: u64) -> Self {
        Self {
            start_time,
            halt_period,
            end_time: start_time + halt_period + halt_buffer_duration,
        }
    }
}

#[cw_serde]
pub struct HaltManager {
    pub prev_block_time: u64, // in seconds
    pub halt_infos: Vec<HaltInfo>,
}

pub const HALT_MANAGER: Item<HaltManager> = Item::new("hm");

impl HaltManager {
    pub fn is_within_halt_window(&self, block_time: u64) -> bool {
        for halt_info in &self.halt_infos {
            if block_time > halt_info.start_time && block_time < halt_info.end_time {
                return true;
            }
        }
        return false;
    }

    pub fn clear_stale_halt_info(&mut self, block_time: u64) {
        if let Some(halt_info) = self.halt_infos.first() {
            if halt_info.end_time < block_time {
                self.halt_infos.remove(0);
            }
        }
    }
}
