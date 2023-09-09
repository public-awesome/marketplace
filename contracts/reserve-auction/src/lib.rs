//! # Stargaze Reserve Auction (aka Live Auction)
//!
//! This CosmWasm smart contract implements a reserve auction on the Stargaze network. In a reserve auction, an item is not sold unless the highest bid is equal to or greater than a predetermined reserve price. The contract includes several key features such as auction creation, bid placement, auction settlement, and cancellation. The auction also provides the ability to update the reserve price.
//!
//! ## Messages
//!
//! The contract functionality is implemented in the following executable messages.
//!
//! **CreateAuction**: Allows the owner of an NFT to create an auction. The owner sets the reserve price, auction duration, and an optional recipient address for the auction proceeds. Upon creation, the contract verifies that the NFT owner has approved the auction contract to transfer the NFT. The function also handles the creation fee, which is sent to a fair-burn contract if applicable. The auction officially starts when the first bid has been //! placed.
//!
//! **UpdateReservePrice**: Allows the seller to update the reserve price of an auction. This operation is only permissible if the auction has not yet //! started (i.e., no bids have been placed).
//!
//! **CancelAuction**: Allows the seller to cancel an auction. Like updating the reserve price, cancellation is only permissible if the auction has not yet //! started.
//!
//! **PlaceBid**: Allows a participant to place a bid on an NFT. If the participant is placing the first bid, then the bid must be higher than the reserve price. If it is not the first bid, then the bid must be higher than the previous highest bid. If a bid is placed near the end of an auction, the end //! time of the auction may be extended in order to allow for more bidding.
//!
//! **SettleAuction**: Allows anyone to settle an auction after it has ended. The function distributes the winning bid to the seller, transfers the NFT to //! the winning bidder, and burns the platform fee. This message is also invoked within the CosmosSDK's EndBlocker to allow for timely settling of auctions.
//!
//! ## Addresses
//!
//! - `elfagar-1: stars1dnadsd7tx0dmnpp26ms7d66zsp7tduygwjgfjzueh0lg9t5lq5vq9kn47c`

mod error;
pub mod execute;
mod helpers;
pub mod instantiate;
pub mod migrate;
pub mod msg;
pub mod query;
mod state;
pub mod sudo;
mod tests;

pub use crate::error::ContractError;
