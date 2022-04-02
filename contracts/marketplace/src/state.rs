use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::Map;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Ask {
    pub price: Uint128,
    pub funds_recipient: Option<Addr>,
}

pub type Bid = Uint128;

// Mapping from (collection, token_id, bidder) to bid amount
pub const TOKEN_BIDS: Map<(&Addr, u32, &Addr), Bid> = Map::new("b");

// Mapping from (collection, token_id) to the current ask for the token
pub const TOKEN_ASKS: Map<(&Addr, u32), Ask> = Map::new("a");
