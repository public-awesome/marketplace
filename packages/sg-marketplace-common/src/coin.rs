pub use crate::errors::MarketplaceStdError;

use cosmwasm_std::{Addr, BankMsg, Coin, Response, SubMsg};

/// Invoke `transfer_coin` to build a `SubMsg` to transfer a single coin to an address.
pub fn transfer_coin(send_coin: Coin, to: &Addr, response: Response) -> Response {
    transfer_coins(vec![send_coin], to, response)
}

/// Invoke `transfer_coin` to build a `SubMsg` to transfer a vector of coins to an address.
pub fn transfer_coins(funds: Vec<Coin>, to: &Addr, response: Response) -> Response {
    response.add_submessage(SubMsg::new(BankMsg::Send {
        to_address: to.to_string(),
        amount: funds,
    }))
}

/// Invoke `checked_transfer_coin` to build a `SubMsg` to transfer a single coin to an address.
/// If no funds are sent, an error is thrown.
pub fn checked_transfer_coin(
    send_coin: Coin,
    to: &Addr,
    response: Response,
) -> Result<Response, MarketplaceStdError> {
    checked_transfer_coins(vec![send_coin], to, response)
}

/// Invoke `checked_transfer_coin` to build a `SubMsg` to transfer a vector of coins to an address.
/// If no funds are sent, an error is thrown.
pub fn checked_transfer_coins(
    funds: Vec<Coin>,
    to: &Addr,
    response: Response,
) -> Result<Response, MarketplaceStdError> {
    for item in &funds {
        if item.amount.is_zero() {
            return Err(MarketplaceStdError::ZeroAmountBankSend);
        }
    }
    Ok(transfer_coins(funds, to, response))
}
