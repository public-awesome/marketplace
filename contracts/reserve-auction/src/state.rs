use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Decimal, Timestamp, Uint128};
use cw_storage_macro::index_list;
use cw_storage_plus::{IndexedMap, Item, MultiIndex};
use std::cmp::max;

#[cw_serde]
pub struct Config {
    pub marketplace: Addr,
    pub min_reserve_price: Coin,
    pub min_bid_increment_pct: Decimal,
    pub min_duration: u64,
    pub extend_duration: u64,
    pub create_auction_fee: Uint128,
    pub max_auctions_to_settle_per_block: u64,
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
