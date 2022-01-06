use cosmwasm_std::StdError;
use http::uri::InvalidUri;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("UnknownReplyId")]
    UnknownReplyId { id: u64 },

    #[error("InvalidReplyData")]
    InvalidReplyData {},

    #[error("InvalidTokenUri")]
    InvalidTokenUri { error: InvalidUri },
}

impl From<InvalidUri> for ContractError {
    fn from(e: InvalidUri) -> ContractError {
        ContractError::InvalidTokenUri { error: e }
    }
}
