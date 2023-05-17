use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum MarketplaceCommonError {
    #[error("Invalid fair burn : {0}")]
    InvalidFairBurn(String),

    #[error("Zero amount bank send")]
    ZeroAmountBankSend,
}
