use cosmwasm_std::{StdError, Uint128};
use cw_utils::PaymentError;
use sg_controllers::HookError;
use thiserror::Error;

use crate::helpers::ExpiryRangeError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

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

    #[error("PriceTooHigh: {0}")]
    PriceTooHigh(Uint128),

    #[error("InvalidListingFee: {0}")]
    InvalidListingFee(Uint128),

    #[error("Token reserved")]
    TokenReserved {},

    #[error("Invalid finders fee bps: {0}")]
    InvalidFindersFeeBps(u64),

    #[error("Invalid finders fee bps: {0}")]
    InvalidTradingFeeBps(u64),

    #[error("Invalid finders fee bps: {0}")]
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

    #[error("Collection not tradable yet")]
    CollectionNotTradable {},

    #[error("Item not for sale")]
    ItemNotForSale {},
}
