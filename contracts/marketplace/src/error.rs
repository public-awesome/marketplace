use cosmwasm_std::StdError;
use cw_utils::PaymentError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Invalid roylties")]
    InvalidRoyalties {},

    #[error("No roylities exist for token_id")]
    NoRoyaltiesForTokenId {},

    #[error("Invalid bid")]
    InvalidBid {},

    #[error("Invalid bid, amount too low")]
    InvalidBidTooLow {},

    #[error("Funds sent don't match bid amount")]
    IncorrectBidFunds {},

    #[error("Bid not found")]
    BidNotFound {},

    #[error("Contract needs approval")]
    NeedsApproval {},

    #[error("{0}")]
    PaymentError(#[from] PaymentError),
}
