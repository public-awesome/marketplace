use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin};
use cw_address_like::AddressLike;
use sg_index_query::QueryOptions;

use crate::state::{
    Ask, CollectionOffer, Config, Denom, ExpirationInfo, Offer, PriceRange, TokenId,
};

#[cw_serde]
pub enum UpdateVal<T> {
    Set(T),
    Unset,
}

#[cw_serde]
pub struct OrderOptions<T: AddressLike> {
    pub asset_recipient: Option<T>,
    pub finder: Option<T>,
    pub finders_fee_bps: Option<u64>,
    pub expiration_info: Option<ExpirationInfo>,
}

impl<T: AddressLike> Default for OrderOptions<T> {
    fn default() -> Self {
        OrderOptions {
            asset_recipient: None,
            finder: None,
            finders_fee_bps: None,
            expiration_info: None,
        }
    }
}

#[cw_serde]
pub struct InstantiateMsg {
    /// The initial configuration for the contract
    pub config: Config<String>,
    /// Min/max values for offers and asks
    pub price_ranges: Vec<(Denom, PriceRange)>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// List an NFT on the marketplace by creating a new ask
    SetAsk {
        collection: String,
        token_id: TokenId,
        price: Coin,
        order_options: Option<OrderOptions<String>>,
    },
    /// Update the price of an existing ask
    UpdateAsk {
        collection: String,
        token_id: TokenId,
        asset_recipient: Option<UpdateVal<String>>,
        finders_fee_bps: Option<UpdateVal<u64>>,
        expiration_info: Option<UpdateVal<ExpirationInfo>>,
    },
    /// Buy an NFT from the marketplace
    AcceptAsk {
        collection: String,
        token_id: TokenId,
        order_options: Option<OrderOptions<String>>,
    },
    /// Remove an existing ask from the marketplace
    RemoveAsk {
        collection: String,
        token_id: TokenId,
    },
    /// Privileged operation to remove stale or invalid asks.
    RemoveExpiredAsk {
        collection: String,
        token_id: TokenId,
    },
    /// Create an offer for an NFT
    SetOffer {
        collection: String,
        token_id: TokenId,
        price: Coin,
        order_options: Option<OrderOptions<String>>,
    },
    /// Update the price of an existing offer
    UpdateOffer {
        collection: String,
        token_id: TokenId,
        asset_recipient: Option<UpdateVal<String>>,
        finders_fee_bps: Option<UpdateVal<u64>>,
        expiration_info: Option<UpdateVal<ExpirationInfo>>,
    },
    /// Accept a offer on an existing ask
    AcceptOffer {
        collection: String,
        token_id: TokenId,
        creator: String,
        order_options: Option<OrderOptions<String>>,
    },
    /// Remove an existing offer from an ask
    RemoveOffer {
        collection: String,
        token_id: TokenId,
    },
    /// Reject a offer on an existing ask
    RejectOffer {
        collection: String,
        token_id: TokenId,
        creator: String,
    },
    /// Remove an existing offer from an ask
    RemoveExpiredOffer {
        collection: String,
        token_id: TokenId,
        creator: String,
    },
    /// Place an offer (limit order) across an entire collection
    SetCollectionOffer {
        collection: String,
        price: Coin,
        order_options: Option<OrderOptions<String>>,
    },
    /// Update the price of an existing offer
    UpdateCollectionOffer {
        collection: String,
        asset_recipient: Option<UpdateVal<String>>,
        finders_fee_bps: Option<UpdateVal<u64>>,
        expiration_info: Option<UpdateVal<ExpirationInfo>>,
    },
    /// Accept a collection offer
    AcceptCollectionOffer {
        collection: String,
        token_id: TokenId,
        creator: String,
        order_options: Option<OrderOptions<String>>,
    },
    /// Remove an offer across an entire collection
    RemoveCollectionOffer { collection: String },
    /// Remove an offer across an entire collection
    RemoveExpiredCollectionOffer { collection: String, creator: String },
}

#[allow(clippy::large_enum_variant)]
#[cw_serde]
pub enum SudoMsg {
    /// BeginBlock Is called by x/cron module BeginBlocker
    BeginBlock {},
    /// EndBlock Is called by x/cron module EndBlocker
    EndBlock {},
    /// Update the contract parameters
    /// Can only be called by governance
    UpdateParams {
        fair_burn: Option<String>,
        listing_fee: Option<Coin>,
        min_removal_reward: Option<Coin>,
        trading_fee_bps: Option<u64>,
        max_royalty_fee_bps: Option<u64>,
        max_finders_fee_bps: Option<u64>,
        min_expiration_seconds: Option<u64>,
        max_asks_removed_per_block: Option<u32>,
        max_offers_removed_per_block: Option<u32>,
        max_collection_offers_removed_per_block: Option<u32>,
    },
    AddDenoms {
        price_ranges: Vec<(Denom, PriceRange)>,
    },
    RemoveDenoms {
        denoms: Vec<Denom>,
    },
}

#[cw_serde]
pub struct AsksByCollectionOffset {
    pub token_id: TokenId,
}

#[cw_serde]
pub struct AsksByPriceOffset {
    pub token_id: TokenId,
    pub amount: u128,
}

#[cw_serde]
pub struct AsksByCreatorOffset {
    pub collection: String,
    pub token_id: TokenId,
}

#[cw_serde]
pub struct AsksByExpirationOffset {
    pub collection: String,
    pub token_id: TokenId,
    pub expiration: u64,
}

#[cw_serde]
pub struct OffersByCollectionOffset {
    pub token_id: TokenId,
    pub creator: String,
}

#[cw_serde]
pub struct OffersByTokenPriceOffset {
    pub creator: String,
    pub amount: u128,
}

#[cw_serde]
pub struct OffersByCreatorOffset {
    pub collection: String,
    pub token_id: TokenId,
}

#[cw_serde]
pub struct OffersByExpirationOffset {
    pub collection: String,
    pub token_id: TokenId,
    pub creator: String,
    pub expiration: u64,
}

#[cw_serde]
pub struct CollectionOffersByCollectionOffset {
    pub creator: String,
}

#[cw_serde]
pub struct CollectionOffersByPriceOffset {
    pub creator: String,
    pub amount: u128,
}

#[cw_serde]
pub struct CollectionOffersByCreatorOffset {
    pub collection: String,
}

#[cw_serde]
pub struct CollectionOffersByExpirationOffset {
    pub collection: String,
    pub creator: String,
    pub expiration: u64,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get the config for the contract
    #[returns(Config<Addr>)]
    Config {},
    /// Get the config for the contract
    #[returns(PriceRange)]
    PriceRange { denom: Denom },
    /// Get the config for the contract
    #[returns(Vec<(Denom, PriceRange)>)]
    PriceRanges {
        query_options: Option<QueryOptions<String>>,
    },
    /// Get the current ask for specific NFT
    #[returns(Option<Ask>)]
    Ask {
        collection: String,
        token_id: TokenId,
    },
    /// Get all asks for a collection
    #[returns(Vec<Ask>)]
    AsksByCollection {
        collection: String,
        query_options: Option<QueryOptions<AsksByCollectionOffset>>,
    },
    /// Get all asks for a collection, sorted by price
    #[returns(Vec<Ask>)]
    AsksByPrice {
        collection: String,
        denom: Denom,
        query_options: Option<QueryOptions<AsksByPriceOffset>>,
    },
    /// Get all asks by creator
    #[returns(Vec<Ask>)]
    AsksByCreator {
        creator: String,
        query_options: Option<QueryOptions<AsksByCreatorOffset>>,
    },
    /// Get all asks sorted by the expiration time
    #[returns(Vec<Ask>)]
    AsksByExpiration {
        query_options: Option<QueryOptions<AsksByExpirationOffset>>,
    },
    /// Get data for a specific offer
    #[returns(Offer)]
    Offer {
        collection: String,
        token_id: TokenId,
        creator: String,
    },
    /// Get all offers on a collection
    #[returns(Vec<Offer>)]
    OffersByCollection {
        collection: String,
        query_options: Option<QueryOptions<OffersByCollectionOffset>>,
    },
    /// Get all offers on a collection token, sorted by price
    #[returns(Vec<Offer>)]
    OffersByTokenPrice {
        collection: String,
        token_id: TokenId,
        denom: Denom,
        query_options: Option<QueryOptions<OffersByTokenPriceOffset>>,
    },
    /// Get all offers made by a creator
    #[returns(Vec<Offer>)]
    OffersByCreator {
        creator: String,
        query_options: Option<QueryOptions<OffersByCreatorOffset>>,
    },
    /// Get all offers sorted by the expiration time
    #[returns(Vec<Offer>)]
    OffersByExpiration {
        query_options: Option<QueryOptions<OffersByExpirationOffset>>,
    },
    /// Get data for a specific collection offer
    #[returns(Option<CollectionOffer>)]
    CollectionOffer { collection: String, creator: String },
    /// Get data for collection offers sorted by collection
    #[returns(Vec<Offer>)]
    CollectionOffersByCollection {
        collection: String,
        query_options: Option<QueryOptions<CollectionOffersByCollectionOffset>>,
    },
    /// Get all collection offers for a collection, sorted by price
    #[returns(Vec<CollectionOffer>)]
    CollectionOffersByPrice {
        collection: String,
        denom: Denom,
        query_options: Option<QueryOptions<CollectionOffersByPriceOffset>>,
    },
    /// Get all collection offers made by a creator
    #[returns(Vec<CollectionOffer>)]
    CollectionOffersByCreator {
        creator: String,
        query_options: Option<QueryOptions<CollectionOffersByCreatorOffset>>,
    },
    /// Get all collection offers sorted by the expiration time
    #[returns(Vec<CollectionOffer>)]
    CollectionOffersByExpiration {
        query_options: Option<QueryOptions<CollectionOffersByExpirationOffset>>,
    },
}
