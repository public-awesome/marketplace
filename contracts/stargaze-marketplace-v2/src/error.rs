use cosmwasm_std::{OverflowError, StdError};
use cw_utils::PaymentError;
use sg_marketplace_common::MarketplaceStdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    OverflowError(#[from] OverflowError),

    #[error("{0}")]
    PaymentError(#[from] PaymentError),

    #[error("{0}")]
    MarketplaceStdError(#[from] MarketplaceStdError),

    #[error("NoMatchFound")]
    NoMatchFound,

    #[error("InvalidInput: {0}")]
    InvalidInput(String),

    #[error("InsufficientFunds: {0}")]
    InsufficientFunds(String),

    #[error("InternalError: {0}")]
    InternalError(String),
}
