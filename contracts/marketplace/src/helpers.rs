use cosmwasm_std::{to_binary, Addr, Api, StdResult, WasmMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sg_std::CosmosMsg;

use crate::msg::ExecuteMsg;

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
