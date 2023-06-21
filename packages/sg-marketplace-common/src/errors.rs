use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum MarketplaceCommonError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Invalid fair burn: {0}")]
    InvalidFairBurn(String),

    #[error("Zero amount bank send")]
    ZeroAmountBankSend,

    #[error("Collection not tradable yet")]
    CollectionNotTradable {},
}
