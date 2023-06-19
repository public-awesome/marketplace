use cosmwasm_std::{Addr, BankMsg, Coin, Decimal, Uint128};
use sg_std::SubMsg;

pub use crate::errors::MarketplaceCommonError;

pub fn transfer_coin(send_coin: Coin, to: &Addr) -> SubMsg {
    SubMsg::new(BankMsg::Send {
        to_address: to.to_string(),
        amount: vec![send_coin],
    })
}

pub fn checked_transfer_coin(send_coin: Coin, to: &Addr) -> Result<SubMsg, MarketplaceCommonError> {
    if send_coin.amount.is_zero() {
        return Err(MarketplaceCommonError::ZeroAmountBankSend);
    }
    Ok(transfer_coin(send_coin, to))
}

pub fn decimal_to_bps(decimal: Decimal) -> u128 {
    (decimal.atomics() / Uint128::from(100_000_000_000_000u128)).u128()
}

pub fn bps_to_decimal(bps: u64) -> Decimal {
    Decimal::percent(bps) / Uint128::from(100u128)
}
