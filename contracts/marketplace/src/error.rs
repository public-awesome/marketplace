use cosmwasm_std::{StdError, Uint128};
use cw_utils::PaymentError;
use sg_controllers::HookError;
use thiserror::Error;

use crate::helpers::ExpiryRangeError;

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

    #[error("PriceTooSmall: {0}")]
    PriceTooSmall(Uint128),

    #[error("Contract needs approval")]
    NeedsApproval {},

    #[error("Token reserved")]
    TokenReserved {},

    #[error("Unrecognised reply id: {0}")]
    UnrecognisedReply(u64),

    #[error("Invalid finders fee bps: {0}")]
    InvalidFindersFeeBps(u64),

    #[error("{0}")]
    BidPaymentError(#[from] PaymentError),

    #[error("{0}")]
    Hook(#[from] HookError),

    #[error("{0}")]
    ExpiryRange(#[from] ExpiryRangeError),
}
