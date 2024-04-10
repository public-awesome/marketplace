//! # sg-marketplace-common
//!
//! The `sg-marketplace-common` common package is used to manage functionality shared between marketplace contracts. The package is divided into the following modules:
//!
//! - `mod address`: functionality related to the `cosmwasm_std::Addr` type
//! - `mod coin`: functionality related to the `cosmwasm_std::Coin` type
//! - `mod nft`: functionality related to NFT data
//! - `mod query`: functionality related to querying smart contracts
//! - `mod sale`: functionality related to NFT sales

pub mod address;
pub mod coin;
pub mod constants;
mod errors;
pub mod nft;
pub mod royalties;
pub mod sale;
mod tests;
mod types;

pub use crate::errors::MarketplaceStdError;
