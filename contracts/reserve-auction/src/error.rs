use cosmwasm_std::{Coin, StdError, Uint128};
use cw_utils::PaymentError;
use sg_marketplace_common::MarketplaceStdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    PaymentError(#[from] PaymentError),

    #[error("{0}")]
    MarketplaceStdError(#[from] MarketplaceStdError),

    #[error("InvalidConfig: {0}")]
    InvalidConfig(String),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("InvalidDuration: min {min}, max {max}, got {got}")]
    InvalidDuration { min: u64, max: u64, got: u64 },

    #[error("InvalidInput: {0}")]
    InvalidInput(String),

    #[error("AuctionStarted")]
    AuctionStarted {},

    #[error("AuctionNotEnded")]
    AuctionNotEnded {},

    #[error("AuctionEnded")]
    AuctionEnded {},

    #[error("WrongFee: expected: {expected}")]
    WrongFee { expected: Coin },

    #[error("InvalidReservePrice: {min}")]
    InvalidReservePrice { min: Coin },

    #[error("BidTooLow: {0}")]
    BidTooLow(Uint128),

    #[error("SellerShouldNotBid")]
    SellerShouldNotBid {},

    #[error("AuctionAlreadyExists collection: {collection} token_id: {token_id}")]
    AuctionAlreadyExists {
        collection: String,
        token_id: String,
    },

    #[error("Auction within halt window: a halt has occurred and the auction cannot be settled")]
    AuctionWithinHaltWindow,
}
