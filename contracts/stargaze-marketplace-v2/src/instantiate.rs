use crate::{
    constants::{CONTRACT_NAME, CONTRACT_VERSION},
    error::ContractError,
    events::ConfigEvent,
    msg::InstantiateMsg,
    state::NONCE,
};

use cosmwasm_std::{DepsMut, Env, Event, MessageInfo, Response};
use cw2::set_contract_version;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Map the Config Strings to Addrs and then save it
    let config = msg.config.str_to_addr(deps.api)?;
    config.save(deps.storage)?;

    NONCE.save(deps.storage, &0)?;

    let response = Response::new()
        .add_event(
            Event::new("instantiate".to_string())
                .add_attribute("contract_name", CONTRACT_NAME)
                .add_attribute("contract_version", CONTRACT_VERSION),
        )
        .add_event(
            ConfigEvent {
                ty: "set-config",
                config: &config,
            }
            .into(),
        );

    Ok(response)
}
