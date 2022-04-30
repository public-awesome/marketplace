use crate::msg::ExecuteMsg;
use cosmwasm_std::{to_binary, Addr, Api, BlockInfo, StdError, StdResult, Timestamp, WasmMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sg_std::CosmosMsg;
use thiserror::Error;

/// MarketplaceContract is a wrapper around Addr that provides a lot of helpers
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MarketplaceContract(pub Addr);

impl MarketplaceContract {
    pub fn addr(&self) -> Addr {
        self.0.clone()
    }

    pub fn call<T: Into<ExecuteMsg>>(&self, msg: T) -> StdResult<CosmosMsg> {
        let msg = to_binary(&msg.into())?;
        Ok(WasmMsg::Execute {
            contract_addr: self.addr().into(),
            msg,
            funds: vec![],
        }
        .into())
    }
}

pub fn map_validate(api: &dyn Api, addresses: &[String]) -> StdResult<Vec<Addr>> {
    addresses
        .iter()
        .map(|addr| api.addr_validate(addr))
        .collect()
}

#[derive(Error, Debug, PartialEq)]
pub enum ExpiryRangeError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Invalid expiration range")]
    InvalidExpirationRange {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ExpiryRange(pub u64, pub u64);

impl ExpiryRange {
    pub fn new(range: (u64, u64)) -> Self {
        ExpiryRange(range.0, range.1)
    }

    /// Validates if given expires time is within the allowable range
    pub fn is_valid(&self, block: &BlockInfo, expires: Timestamp) -> Result<(), ExpiryRangeError> {
        let now = block.time;
        if !(expires > now.plus_seconds(self.0) && expires <= now.plus_seconds(self.1)) {
            return Err(ExpiryRangeError::InvalidExpirationRange {});
        }

        Ok(())
    }
}
