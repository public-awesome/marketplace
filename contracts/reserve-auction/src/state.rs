use cosmwasm_schema::cw_serde;
use cosmwasm_std::{coin, ensure, Addr, Coin, Decimal, Storage, Timestamp, Uint128};
use cw_storage_macro::index_list;
use cw_storage_plus::{IndexedMap, Item, Map, MultiIndex};
use sg_marketplace_common::address::address_or;

use crate::ContractError;

#[cw_serde]
pub struct Config {
    pub fair_burn: Addr,
    pub trading_fee_percent: Decimal,
    pub min_bid_increment_percent: Decimal,
    pub min_duration: u64,
    pub max_duration: u64,
    pub extend_duration: u64,
    pub create_auction_fee: Coin,
    pub max_auctions_to_settle_per_block: u64,
    pub halt_duration_threshold: u64, // in seconds
    pub halt_buffer_duration: u64,    // in seconds
    pub halt_postpone_duration: u64,  // in seconds
    pub royalty_registry: Addr,
    pub max_royalty_fee_bps: u64,
}

impl Config {
    pub fn save(&self, storage: &mut dyn Storage) -> Result<(), ContractError> {
        self.validate()?;
        CONFIG.save(storage, self)?;
        Ok(())
    }

    fn validate(&self) -> Result<(), ContractError> {
        ensure!(
            !self.min_bid_increment_percent.is_zero(),
            ContractError::InvalidConfig(
                "min_bid_increment_percent must be greater than zero".to_string(),
            )
        );
        ensure!(
            self.min_bid_increment_percent < Decimal::one(),
            ContractError::InvalidConfig(
                "min_bid_increment_percent must be less than 100%".to_string(),
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

pub const CONFIG: Item<Config> = Item::new("cfg");

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

    pub fn funds_recipient(&self) -> Addr {
        address_or(self.seller_funds_recipient.as_ref(), &self.seller)
    }

    pub fn min_bid_coin(&self, min_bid_increment_percent: Decimal) -> Coin {
        let amount = match &self.high_bid {
            Some(high_bid) => high_bid
                .coin
                .amount
                .mul_ceil(Decimal::one() + min_bid_increment_percent),
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
pub struct HaltWindow {
    pub start_time: u64, // in seconds
    pub end_time: u64,   // in seconds
}

#[cw_serde]
pub struct HaltManager {
    pub prev_block_time: u64, // in seconds
    pub halt_windows: Vec<HaltWindow>,
}

pub const HALT_MANAGER: Item<HaltManager> = Item::new("hm");

impl HaltManager {
    pub fn is_within_halt_window(&self, block_time: u64) -> bool {
        for halt_info in &self.halt_windows {
            if block_time > halt_info.start_time && block_time < halt_info.end_time {
                return true;
            }
        }
        false
    }

    pub fn find_stale_halt_info(
        &mut self,
        earliest_auction_end_time: Option<Timestamp>,
    ) -> Option<HaltWindow> {
        if self.halt_windows.is_empty() {
            return None;
        }
        let halt_info = self.halt_windows.first().unwrap();
        if earliest_auction_end_time.is_none()
            || earliest_auction_end_time.unwrap().seconds() > halt_info.end_time
        {
            return Some(self.halt_windows.remove(0));
        }
        None
    }
}
