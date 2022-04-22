use cosmwasm_std::StdError;
use cw1_whitelist::ContractError as WhitelistError;
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

    #[error("Funds sent don't match bid amount")]
    IncorrectBidFunds {},

    #[error("InvalidExpiration")]
    InvalidExpiration {},

    #[error("InvalidPrice")]
    InvalidPrice {},

    #[error("AskExpired")]
    AskExpired {},

    #[error("AskNotActive")]
    AskNotActive {},

    #[error("BidExpired")]
    BidExpired {},

    #[error("Bid not found")]
    BidNotFound {},

    #[error("Contract needs approval")]
    NeedsApproval {},

    #[error("Invalid Ask Price #{denom} #{amount}")]
    InvalidAskPrice { denom: String, amount: u128 },

    #[error("{0}")]
    WhitelistError(#[from] WhitelistError),

    #[error("{0}")]
    BidPaymentError(#[from] PaymentError),
}
