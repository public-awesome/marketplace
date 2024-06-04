use cosmwasm_schema::cw_serde;
use cosmwasm_std::{DepsMut, Env, Response};
use cw2::set_contract_version;

use crate::{
    constants::{CONTRACT_NAME, CONTRACT_VERSION},
    ContractError,
};

#[cw_serde]
pub struct MigrateMsg {}

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let response = Response::new();

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(response)
}
