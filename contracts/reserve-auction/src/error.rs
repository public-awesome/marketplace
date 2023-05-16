use cosmwasm_std::{Coin, StdError, Uint128};
use cw_utils::PaymentError;
use sg_marketplace_common::MarketplaceCommonError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    PaymentError(#[from] PaymentError),

    #[error("{0}")]
    MarketplaceCommonError(#[from] MarketplaceCommonError),

    #[error("InvalidConfig: {0}")]
    InvalidConfig(String),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("InvalidStartTime")]
    InvalidStartTime {},

    #[error("InvalidEndTime")]
    InvalidEndTime {},

    #[error("AuctionStarted")]
    AuctionStarted {},

    #[error("AuctionNotStarted")]
    AuctionNotStarted {},

    #[error("AuctionNotEnded")]
    AuctionNotEnded {},

    #[error("AuctionEnded")]
    AuctionEnded {},

    #[error("WrongFee: {given} != {required}")]
    WrongFee { given: Uint128, required: Uint128 },

    #[error("InvalidReservePrice: {min}")]
    InvalidReservePrice { min: Coin },

    #[error("ReserveNotMet: {min}")]
    ReserveNotMet { min: Coin },

    #[error("BidTooLow: {0}")]
    BidTooLow(Uint128),

    #[error("SellerShouldNotBid")]
    SellerShouldNotBid {},

    #[error("AuctionAlreadyExists collection: {collection} token_id: {token_id}")]
    AuctionAlreadyExists {
        collection: String,
        token_id: String,
    },
}
