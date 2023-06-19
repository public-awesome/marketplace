use cosmwasm_std::{Coin, Decimal, StdError, Uint128};
use cw_utils::PaymentError;
use sg_controllers::HookError;
use sg_marketplace_common::MarketplaceCommonError;
use thiserror::Error;

use crate::helpers::ExpiryRangeError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    MarketplaceCommonError(#[from] MarketplaceCommonError),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Invalid fair burn: {0}")]
    InvalidFairBurn(String),

    #[error("UnauthorizedOwner")]
    UnauthorizedOwner {},

    #[error("UnauthorizedOperator")]
    UnauthorizedOperator {},

    #[error("InvalidPrice")]
    InvalidPrice {},

    #[error("InvalidDuration")]
    InvalidDuration {},

    #[error("AskExpired")]
    AskExpired {},

    #[error("AskNotActive")]
    AskNotActive {},

    #[error("AskUnchanged")]
    AskUnchanged {},

    #[error("AskNotFound")]
    AskNotFound {},

    #[error("InvalidListing/StaleListing")]
    InvalidListing {},

    #[error("BidExpired")]
    BidExpired {},

    #[error("BidNotStale")]
    BidNotStale {},

    #[error("InvalidFinder: {0}")]
    InvalidFinder(String),

    #[error("PriceTooSmall: {0}")]
    PriceTooSmall(Uint128),

    #[error("InvalidFunds: expected {expected}")]
    InvalidFunds { expected: Coin },

    #[error("PriceTooHigh: {0}")]
    PriceTooHigh(Uint128),

    #[error("InvalidListingFee: {0}")]
    InvalidListingFee(Uint128),

    #[error("Token reserved")]
    TokenReserved {},

    #[error("Invalid finders fee bps: {0}")]
    InvalidFindersFeePercent(Decimal),

    #[error("Invalid trading fee bps: {0}")]
    InvalidTradingFeeBps(u64),

    #[error("Invalid bid removal reward bps: {0}")]
    InvalidBidRemovalRewardBps(u64),

    #[error("{0}")]
    BidPaymentError(#[from] PaymentError),

    #[error("{0}")]
    Hook(#[from] HookError),

    #[error("{0}")]
    ExpiryRange(#[from] ExpiryRangeError),

    #[error("Invalid reserve_for address: {reason}")]
    InvalidReserveAddress { reason: String },

    #[error("Given operator address already registered as an operator")]
    OperatorAlreadyRegistered {},

    #[error("Given operator address is not registered as an operator")]
    OperatorNotRegistered {},

    #[error("InvalidContractVersion")]
    InvalidContractVersion {},

    #[error("Item not for sale")]
    ItemNotForSale {},
}
