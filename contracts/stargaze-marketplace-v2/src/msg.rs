use crate::{
    orders::{Ask, Bid, CollectionBid, OrderDetails},
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
        details: OrderDetails<String>,
    },
    SetBid {
        collection: String,
        token_id: TokenId,
        details: OrderDetails<String>,
    },
    RemoveBid {
        id: OrderId,
    },
    UpdateBid {
        id: OrderId,
        details: OrderDetails<String>,
    },
    AcceptBid {
        id: OrderId,
        details: OrderDetails<String>,
    },
    SetCollectionBid {
        collection: String,
        details: OrderDetails<String>,
    },
    RemoveCollectionBid {
        id: OrderId,
    },
    UpdateCollectionBid {
        id: OrderId,
        details: OrderDetails<String>,
    },
    AcceptCollectionBid {
        id: OrderId,
        token_id: TokenId,
        details: OrderDetails<String>,
    },
    SellNft {
        collection: String,
        token_id: TokenId,
        details: OrderDetails<String>,
    },
    BuySpecificNft {
        collection: String,
        token_id: TokenId,
        details: OrderDetails<String>,
    },
    BuyCollectionNft {
        collection: String,
        details: OrderDetails<String>,
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
    #[returns(Option<Bid>)]
    Bid(String),
    #[returns(Vec<Bid>)]
    Bids(Vec<String>),
    #[returns(Vec<Bid>)]
    BidsByTokenPrice {
        collection: String,
        token_id: TokenId,
        denom: Denom,
        query_options: Option<QueryOptions<PriceOffset>>,
    },
    #[returns(Vec<Bid>)]
    BidsByCreatorCollection {
        creator: String,
        collection: String,
        query_options: Option<QueryOptions<String>>,
    },
    #[returns(Option<CollectionBid>)]
    CollectionBid(String),
    #[returns(Vec<CollectionBid>)]
    CollectionBids(Vec<String>),
    #[returns(Vec<CollectionBid>)]
    CollectionBidsByPrice {
        collection: String,
        denom: Denom,
        query_options: Option<QueryOptions<PriceOffset>>,
    },
    #[returns(Vec<CollectionBid>)]
    CollectionBidsByCreatorCollection {
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
