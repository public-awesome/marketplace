use crate::error::ContractError;
use crate::helpers::map_validate;
use crate::msg::SudoMsg;
use crate::state::{ASK_HOOKS, SALE_FINALIZED_HOOKS, SUDO_PARAMS};
use cosmwasm_std::{entry_point, Addr, DepsMut, Env};
use sg_std::Response;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(deps: DepsMut, env: Env, msg: SudoMsg) -> Result<Response, ContractError> {
    let api = deps.api;

    match msg {
        SudoMsg::UpdateParams {
            trading_fee_percent,
            ask_expiry,
            bid_expiry,
            operators,
        } => sudo_update_params(
            deps,
            env,
            trading_fee_percent,
            ask_expiry,
            bid_expiry,
            operators,
        ),
        SudoMsg::AddSaleFinalizedHook { hook } => {
            sudo_add_sale_finalized_hook(deps, api.addr_validate(&hook)?)
        }
        SudoMsg::AddAskHook { hook } => sudo_add_ask_hook(deps, env, api.addr_validate(&hook)?),
        SudoMsg::RemoveSaleFinalizedHook { hook } => {
            sudo_remove_sale_finalized_hook(deps, api.addr_validate(&hook)?)
        }
        SudoMsg::RemoveAskHook { hook } => sudo_remove_ask_hook(deps, api.addr_validate(&hook)?),
    }
}

/// Only governance can update contract params
pub fn sudo_update_params(
    deps: DepsMut,
    _env: Env,
    trading_fee_percent: Option<u32>,
    ask_expiry: Option<(u64, u64)>,
    bid_expiry: Option<(u64, u64)>,
    operators: Option<Vec<String>>,
) -> Result<Response, ContractError> {
    let mut params = SUDO_PARAMS.load(deps.storage)?;

    params.trading_fee_percent = trading_fee_percent.unwrap_or(params.trading_fee_percent);
    params.ask_expiry = ask_expiry.unwrap_or(params.ask_expiry);
    params.bid_expiry = bid_expiry.unwrap_or(params.bid_expiry);
    if let Some(operators) = operators {
        params.operators = map_validate(deps.api, &operators)?;
    }
    SUDO_PARAMS.save(deps.storage, &params)?;

    Ok(Response::new().add_attribute("action", "update_params"))
}

pub fn sudo_add_sale_finalized_hook(deps: DepsMut, hook: Addr) -> Result<Response, ContractError> {
    SALE_FINALIZED_HOOKS.add_hook(deps.storage, hook.clone())?;

    let res = Response::new()
        .add_attribute("action", "add_sale_finalized_hook")
        .add_attribute("hook", hook);
    Ok(res)
}

pub fn sudo_add_ask_hook(deps: DepsMut, _env: Env, hook: Addr) -> Result<Response, ContractError> {
    ASK_HOOKS.add_hook(deps.storage, hook.clone())?;

    let res = Response::new()
        .add_attribute("action", "add_ask_hook")
        .add_attribute("hook", hook);
    Ok(res)
}

pub fn sudo_remove_sale_finalized_hook(
    deps: DepsMut,
    hook: Addr,
) -> Result<Response, ContractError> {
    SALE_FINALIZED_HOOKS.remove_hook(deps.storage, hook.clone())?;

    let res = Response::new()
        .add_attribute("action", "remove_sale_finalized_hook")
        .add_attribute("hook", hook);
    Ok(res)
}

pub fn sudo_remove_ask_hook(deps: DepsMut, hook: Addr) -> Result<Response, ContractError> {
    ASK_HOOKS.remove_hook(deps.storage, hook.clone())?;

    let res = Response::new()
        .add_attribute("action", "remove_ask_hook")
        .add_attribute("hook", hook);
    Ok(res)
}
