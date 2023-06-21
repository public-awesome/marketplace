use cosmwasm_schema::cw_serde;
use cosmwasm_std::{DepsMut, Env, StdError};
use cw2::set_contract_version;
use semver::Version;
use sg_std::Response;

use crate::{
    constants::{CONTRACT_NAME, CONTRACT_VERSION},
    upgrades::v3,
    ContractError,
};

#[cw_serde]
pub struct MigrateMsg {
    v3: v3::MigrateMsgV3,
}

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(mut deps: DepsMut, env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    let current_version = cw2::get_contract_version(deps.storage)?;
    if current_version.contract != CONTRACT_NAME {
        return Err(StdError::generic_err("Cannot upgrade to a different contract").into());
    }
    let version: Version = current_version
        .version
        .parse()
        .map_err(|_| StdError::generic_err("Invalid contract version"))?;
    let new_version: Version = CONTRACT_VERSION
        .parse()
        .map_err(|_| StdError::generic_err("Invalid contract version"))?;

    if version >= new_version {
        return Err(StdError::generic_err("Must upgrade to a greater version").into());
    }

    let mut response = Response::new();

    if version < Version::new(3, 0, 0) {
        response = v3::migrate(deps.branch(), env, msg.v3, response)?;
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(response)
}
