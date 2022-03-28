use cosmwasm_std::{Addr, Coin, Uint128};
use cw_storage_plus::Map;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Ask {
    pub price: Coin,
    pub funds_recipient: Option<Addr>,
}

// Mapping from (collection, token_id, bidder) to bid amount
pub const TOKEN_BIDS: Map<(&Addr, &str, &Addr), Uint128> = Map::new("b");

// Mapping from (collection, token_id) to the current ask for the token
pub const TOKEN_ASKS: Map<(&Addr, &str), Ask> = Map::new("a");
