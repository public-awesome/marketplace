use crate::{
    orders::{Ask, CollectionOffer, Offer, OrderDetails},
    state::{AllowDenoms, Config, Denom, OrderId, TokenId},
};

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
use sg_index_query::QueryOptions;

#[cw_serde]
pub struct InstantiateMsg {
    /// The initial configuration for the contract
    pub config: Config<String>,
    /// The initial allowed denoms for the contract
    pub allow_denoms: AllowDenoms,
}

#[cw_serde]
pub enum ExecuteMsg {
    // Admin messages
    UpdateConfig {
        config: Config<String>,
    },
    UpdateAllowDenoms {
        allow_denoms: AllowDenoms,
    },
    // Marketplace messages
    SetAsk {
        collection: String,
        token_id: TokenId,
        details: OrderDetails<String>,
    },
    RemoveAsk {
        id: OrderId,
    },
    UpdateAsk {
        id: OrderId,
        details: OrderDetails<String>,
    },
    AcceptAsk {
        id: OrderId,
        asset_recipient: Option<String>,
        finder: Option<String>,
    },
    SetOffer {
        collection: String,
        token_id: TokenId,
        details: OrderDetails<String>,
    },
    RemoveOffer {
        id: OrderId,
    },
    UpdateOffer {
        id: OrderId,
        details: OrderDetails<String>,
    },
    AcceptOffer {
        id: OrderId,
        asset_recipient: Option<String>,
        finder: Option<String>,
    },
    SetCollectionOffer {
        collection: String,
        details: OrderDetails<String>,
    },
    RemoveCollectionOffer {
        id: OrderId,
    },
    UpdateCollectionOffer {
        id: OrderId,
        details: OrderDetails<String>,
    },
    AcceptCollectionOffer {
        id: OrderId,
        token_id: TokenId,
        asset_recipient: Option<String>,
        finder: Option<String>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Config<Addr>)]
    Config {},
    #[returns(AllowDenoms)]
    AllowDenoms {},
    #[returns(Option<Ask>)]
    Ask(String),
    #[returns(Vec<Ask>)]
    Asks(Vec<String>),
    #[returns(Vec<Ask>)]
    AsksByCollectionDenom {
        collection: String,
        denom: Denom,
        query_options: Option<QueryOptions<PriceOffset>>,
    },
    #[returns(Vec<Ask>)]
    AsksByCreatorCollection {
        creator: String,
        collection: String,
        query_options: Option<QueryOptions<String>>,
    },
    #[returns(Option<Offer>)]
    Offer(String),
    #[returns(Vec<Offer>)]
    Offers(Vec<String>),
    #[returns(Vec<Offer>)]
    OffersByTokenPrice {
        collection: String,
        token_id: TokenId,
        denom: Denom,
        query_options: Option<QueryOptions<PriceOffset>>,
    },
    #[returns(Vec<Offer>)]
    OffersByCreatorCollection {
        creator: String,
        collection: String,
        query_options: Option<QueryOptions<String>>,
    },
    #[returns(Option<CollectionOffer>)]
    CollectionOffer(String),
    #[returns(Vec<CollectionOffer>)]
    CollectionOffers(Vec<String>),
    #[returns(Vec<CollectionOffer>)]
    CollectionOffersByPrice {
        collection: String,
        denom: Denom,
        query_options: Option<QueryOptions<PriceOffset>>,
    },
    #[returns(Vec<CollectionOffer>)]
    CollectionOffersByCreatorCollection {
        creator: String,
        collection: String,
        query_options: Option<QueryOptions<String>>,
    },
}

#[cw_serde]
pub struct PriceOffset {
    pub id: OrderId,
    pub amount: u128,
}
