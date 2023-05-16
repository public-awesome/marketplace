use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum MarketplaceCommonError {
    #[error("Invalid fair burn : {0}")]
    InvalidFairBurn(String),

    #[error("Invalid bank transfer: {0}")]
    InvalidBankTransfer(String),
}
