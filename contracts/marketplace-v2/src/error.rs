use cosmwasm_std::{Coin, StdError};
use cw_utils::PaymentError;
use sg_marketplace_common::MarketplaceStdError;
use stargaze_royalty_registry::ContractError as RoyaltyRegistryError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    PaymentError(#[from] PaymentError),

    #[error("{0}")]
    MarketplaceStdError(#[from] MarketplaceStdError),

    #[error("{0}")]
    RoyaltyRegistryError(#[from] RoyaltyRegistryError),

    #[error("InternalError: {0}")]
    InternalError(String),

    #[error("InvalidInput: {0}")]
    InvalidInput(String),

    #[error("InsufficientFunds: expected {expected}")]
    InsufficientFunds { expected: Coin },

    #[error("EntityNotFound: {0}")]
    EntityNotFound(String),

    #[error("EntityExists: {0}")]
    EntityExists(String),

    #[error("EntityNotExpired: {0}")]
    EntityNotExpired(String),
}
