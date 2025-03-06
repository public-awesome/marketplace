use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Decimal};
use sg_marketplace_common::query::QueryOptions;

use crate::state::{Auction, Config, HaltManager};

#[cw_serde]
pub struct InstantiateMsg {
    /// The address of the fair burn contract
    pub fair_burn: String,
    /// The number of basis points that is fair burned on each settled auction
    pub trading_fee_percent: Decimal,
    /// Each bid must be some number of basis points greater than the previous bid
    pub min_bid_increment_percent: Decimal,
    /// The minimum duration of an auction
    pub min_duration: u64,
    /// The maximum duration of an auction
    pub max_duration: u64,
    /// When a bid is placed near the end of an auction,
    /// extend the auction by this duration
    pub extend_duration: u64,
    /// The fee that must be paid when creating an auction
    pub create_auction_fee: Coin,
    /// The maximum number of auctions that can be processed in each block
    pub max_auctions_to_settle_per_block: u64,
    /// If the time between blocks exceeds the halt_duration_threshold,
    /// then it is determined that a halt has occurred.
    pub halt_duration_threshold: u64,
    /// The amount of time, in seconds, added to the end of a halt period
    /// and used to determine a halt window. If an auction ends
    /// within a halt window it cannot be settled, it must be
    /// postponed.
    pub halt_buffer_duration: u64,
    /// The amount of time, in seconds, that should be added to an auction
    /// that needs to be postponed.
    pub halt_postpone_duration: u64,
    /// The minimum reserve prices for the various denoms. Denoms
    /// no defined are not supported.
    pub min_reserve_prices: Vec<Coin>,
    pub royalty_registry: String,
    pub max_royalty_fee_bps: u64,
}

#[cw_serde]
pub enum ExecuteMsg {
    CreateAuction {
        collection: String,
        token_id: String,
        reserve_price: Coin,
        duration: u64,
        seller_funds_recipient: Option<String>,
    },
    UpdateReservePrice {
        collection: String,
        token_id: String,
        reserve_price: Coin,
    },
    CancelAuction {
        collection: String,
        token_id: String,
    },
    PlaceBid {
        collection: String,
        token_id: String,
    },
    SettleAuction {
        collection: String,
        token_id: String,
    },
}

#[cw_serde]
#[derive(Default)]
pub struct MinReservePriceOffset {
    pub denom: String,
}

#[cw_serde]
#[derive(Default)]
pub struct AuctionKeyOffset {
    pub collection: String,
    pub token_id: String,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Config)]
    Config {},
    #[returns(HaltManager)]
    HaltManager {},
    #[returns(Vec<Coin>)]
    MinReservePrices {
        query_options: Option<QueryOptions<MinReservePriceOffset>>,
    },
    #[returns(Option<Auction>)]
    Auction {
        collection: String,
        token_id: String,
    },
    #[returns(Vec<Auction>)]
    AuctionsBySeller {
        seller: String,
        query_options: Option<QueryOptions<AuctionKeyOffset>>,
    },
    #[returns(Vec<Auction>)]
    AuctionsByEndTime {
        end_time: u64,
        query_options: Option<QueryOptions<AuctionKeyOffset>>,
    },
}

#[allow(clippy::large_enum_variant)]
#[cw_serde]
pub enum SudoMsg {
    BeginBlock {}, // Is called by x/cron module BeginBlocker
    EndBlock {},   // Is called by x/cron module EndBlocker
    UpdateParams {
        fair_burn: Option<String>,
        trading_fee_percent: Option<Decimal>,
        min_bid_increment_percent: Option<Decimal>,
        min_duration: Option<u64>,
        extend_duration: Option<u64>,
        create_auction_fee: Option<Coin>,
        max_auctions_to_settle_per_block: Option<u64>,
        halt_duration_threshold: Option<u64>,
        halt_buffer_duration: Option<u64>,
        halt_postpone_duration: Option<u64>,
        royalty_registry: Option<String>,
        max_royalty_fee_bps: Option<u64>,
    },
    SetMinReservePrices {
        min_reserve_prices: Vec<Coin>,
    },
    UnsetMinReservePrices {
        denoms: Vec<String>,
    },
}
