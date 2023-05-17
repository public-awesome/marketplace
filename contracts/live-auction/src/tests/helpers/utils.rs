use anyhow::Error;
use cosmwasm_std::{Decimal, Uint128};
use cw_multi_test::AppResponse;

pub fn assert_error(res: Result<AppResponse, Error>, expected: String) {
    assert_eq!(res.unwrap_err().source().unwrap().to_string(), expected);
}

pub fn calc_min_bid_increment(
    starting_price: u128,
    min_bid_increment_bps: u64,
    num_bids: u64,
) -> Uint128 {
    let mut price = Uint128::from(starting_price);
    for _ in 0..num_bids {
        price = price * (Decimal::percent(10000) + Decimal::percent(min_bid_increment_bps))
            / Uint128::from(100u128);
    }
    price
}
