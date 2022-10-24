use cw_storage_macro::index_list;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Addr;
use cw_storage_plus::{Item, MultiIndex};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct State {
    pub count: i32,
    pub owner: Addr,
}

pub const STATE: Item<State> = Item::new("state");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Auction {
    pub seller: Addr,
    pub reserve_price: u128,
    pub payment_address: Addr,
    pub highest_bid: u64,
    pub highest_bidder: Addr,
    pub duration: u64,
    pub start_time: u64,
    pub first_bid_time: u64,
}

pub type TokenId = String;

pub type AuctionKey = (Addr, TokenId);

#[index_list(Auction)]
pub struct AuctionIndexes<'a> {
    pub seller: MultiIndex<'a, Addr, Auction, AuctionKey>,
    pub highest_bid: MultiIndex<'a, u64, Auction, AuctionKey>,
    pub duration: MultiIndex<'a, u64, Auction, AuctionKey>,
    pub start_time: MultiIndex<'a, u64, Auction, AuctionKey>,
    pub first_bid_time: MultiIndex<'a, u64, Auction, AuctionKey>,
}
