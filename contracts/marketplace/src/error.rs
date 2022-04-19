use cosmwasm_std::{Coin, StdError};
use cw_utils::PaymentError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Invalid royalties")]
    InvalidRoyalties {},

    #[error("No royalties exist for token_id")]
    NoRoyaltiesForTokenId {},

    #[error("Funds sent don't match bid amount")]
    IncorrectBidFunds {},

    #[error("Bid not found")]
    BidNotFound {},

    #[error("Contract needs approval")]
    NeedsApproval {},

    #[error("IncorrectPaymentAmount {0} != {1}")]
    IncorrectPaymentAmount(Coin, Coin),

    #[error("{0}")]
    BidPaymentError(#[from] PaymentError),
}
