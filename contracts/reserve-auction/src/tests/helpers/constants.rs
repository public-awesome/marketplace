use cosmwasm_std::Uint128;

// test params
pub const CREATE_AUCTION_FEE: Uint128 = Uint128::new(1000000);
pub const MIN_RESERVE_PRICE: u128 = 1000000;
pub const MIN_DURATION: u64 = 60;
pub const MAX_DURATION: u64 = 60 * 60 * 24 * 7;
pub const DEFAULT_DURATION: u64 = 60 * 60;
pub const MIN_BID_INCREMENT_BPS: u64 = 2500;
pub const EXTEND_DURATION: u64 = 500;
pub const MAX_AUCTIONS_TO_SETTLE_PER_BLOCK: u64 = 200;
