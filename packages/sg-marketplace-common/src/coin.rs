use cosmwasm_std::{Addr, BankMsg, Coin, Decimal, Uint128};
use sg_std::SubMsg;

pub use crate::errors::MarketplaceCommonError;

/// Invoke `transfer_coin` to build a `SubMsg` to transfer a single coin to an address.
pub fn transfer_coin(send_coin: Coin, to: &Addr) -> SubMsg {
    transfer_coins(vec![send_coin], to)
}

/// Invoke `transfer_coin` to build a `SubMsg` to transfer a vector of coins to an address.
pub fn transfer_coins(funds: Vec<Coin>, to: &Addr) -> SubMsg {
    SubMsg::new(BankMsg::Send {
        to_address: to.to_string(),
        amount: funds,
    })
}

/// Invoke `checked_transfer_coin` to build a `SubMsg` to transfer a single coin to an address.
/// If no funds are sent, an error is thrown.
pub fn checked_transfer_coin(send_coin: Coin, to: &Addr) -> Result<SubMsg, MarketplaceCommonError> {
    checked_transfer_coins(vec![send_coin], to)
}

/// Invoke `checked_transfer_coin` to build a `SubMsg` to transfer a vector of coins to an address.
/// If no funds are sent, an error is thrown.
pub fn checked_transfer_coins(
    funds: Vec<Coin>,
    to: &Addr,
) -> Result<SubMsg, MarketplaceCommonError> {
    for item in &funds {
        if item.amount.is_zero() {
            return Err(MarketplaceCommonError::ZeroAmountBankSend);
        }
    }
    Ok(transfer_coins(funds, to))
}

/// Invoke `decimal_to_bps` to transform a `Decimal` to a basis point representation.
pub fn decimal_to_bps(decimal: Decimal) -> u128 {
    (decimal.atomics() / Uint128::from(100_000_000_000_000u128)).u128()
}

/// Invoke `bps_to_decimal` to transform a basis point representation to a `Decimal`.
pub fn bps_to_decimal(bps: u64) -> Decimal {
    Decimal::percent(bps) / Uint128::from(100u128)
}
