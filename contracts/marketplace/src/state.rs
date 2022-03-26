use cosmwasm_std::{Addr, Coin};
use cw_storage_plus::Map;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Bid {
    pub amount: Coin,
    pub bidder: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Ask {
    pub amount: Coin,
}

// Mapping from (collection, token_id, bidder) to bid
pub const TOKEN_BIDS: Map<(&Addr, &str, &Addr), Bid> = Map::new("token_bidders");

// Mapping from (collection, token_id) to the current ask for the token
pub const TOKEN_ASKS: Map<(&Addr, &str), Ask> = Map::new("token_asks");
