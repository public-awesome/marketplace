use crate::state::Config;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{DepsMut, Env, Response};
use cw2::set_contract_version;

use crate::{
    constants::{CONTRACT_NAME, CONTRACT_VERSION},
    ContractError,
};

#[cw_serde]
pub struct MigrateMsg {
    config: Option<Config<String>>,
}

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    // Set new contract version
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let response = Response::new().add_attribute("action", "migrate");

    // Apply config update if present
    if let Some(config) = msg.config {
        let updated_config = config.str_to_addr(deps.api)?;
        updated_config.save(deps.storage)?;
    }

    Ok(response)
}
