use crate::instantiate::{CONTRACT_NAME, CONTRACT_VERSION};
use crate::ContractError;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{DepsMut, Env};
use sg_std::Response;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrationMsg) -> Result<Response, ContractError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new())
}

#[cw_serde]
pub struct MigrationMsg {}
