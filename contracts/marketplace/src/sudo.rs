use crate::error::ContractError;
use crate::helpers::ExpiryRange;
use crate::msg::SudoMsg;
use crate::state::{ASK_HOOKS, BID_HOOKS, SALE_HOOKS, SUDO_PARAMS};
use cosmwasm_std::{entry_point, Addr, Decimal, DepsMut, Env, Uint128};
use cw_utils::Duration;
use sg_std::Response;

pub struct ParamInfo {
    trading_fee_bps: Option<u64>,
    ask_expiry: Option<ExpiryRange>,
    bid_expiry: Option<ExpiryRange>,
    operators: Option<Vec<String>>,
    max_finders_fee_bps: Option<u64>,
    min_price: Option<Uint128>,
    stale_bid_duration: Option<u64>,
    bid_removal_reward_bps: Option<u64>,
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(deps: DepsMut, env: Env, msg: SudoMsg) -> Result<Response, ContractError> {
    let api = deps.api;

    match msg {
        SudoMsg::UpdateParams {
            trading_fee_bps,
            ask_expiry,
            bid_expiry,
            operators,
            max_finders_fee_bps,
            min_price,
            stale_bid_duration,
            bid_removal_reward_bps,
        } => sudo_update_params(
            deps,
            env,
            ParamInfo {
                trading_fee_bps,
                ask_expiry,
                bid_expiry,
                operators,
                max_finders_fee_bps,
                min_price,
                stale_bid_duration,
                bid_removal_reward_bps,
            },
        ),
        SudoMsg::AddOperator { operator } => sudo_add_operator(deps, api.addr_validate(&operator)?),
        SudoMsg::RemoveOperator { operator } => {
            sudo_remove_operator(deps, api.addr_validate(&operator)?)
        }
        SudoMsg::AddSaleHook { hook } => sudo_add_sale_hook(deps, api.addr_validate(&hook)?),
        SudoMsg::AddAskHook { hook } => sudo_add_ask_hook(deps, env, api.addr_validate(&hook)?),
        SudoMsg::AddBidHook { hook } => sudo_add_bid_hook(deps, env, api.addr_validate(&hook)?),
        SudoMsg::RemoveSaleHook { hook } => sudo_remove_sale_hook(deps, api.addr_validate(&hook)?),
        SudoMsg::RemoveAskHook { hook } => sudo_remove_ask_hook(deps, api.addr_validate(&hook)?),
        SudoMsg::RemoveBidHook { hook } => sudo_remove_bid_hook(deps, api.addr_validate(&hook)?),
    }
}

/// Only governance can update contract params
pub fn sudo_update_params(
    deps: DepsMut,
    _env: Env,
    param_info: ParamInfo,
) -> Result<Response, ContractError> {
    let ParamInfo {
        trading_fee_bps,
        ask_expiry,
        bid_expiry,
        operators: _operators,
        max_finders_fee_bps,
        min_price,
        stale_bid_duration,
        bid_removal_reward_bps,
    } = param_info;

    ask_expiry.as_ref().map(|a| a.validate()).transpose()?;
    bid_expiry.as_ref().map(|b| b.validate()).transpose()?;

    let mut params = SUDO_PARAMS.load(deps.storage)?;

    params.trading_fee_percent = trading_fee_bps
        .map(Decimal::percent)
        .unwrap_or(params.trading_fee_percent);

    params.ask_expiry = ask_expiry.unwrap_or(params.ask_expiry);
    params.bid_expiry = bid_expiry.unwrap_or(params.bid_expiry);

    params.max_finders_fee_percent = max_finders_fee_bps
        .map(Decimal::percent)
        .unwrap_or(params.max_finders_fee_percent);

    params.min_price = min_price.unwrap_or(params.min_price);

    params.stale_bid_duration = stale_bid_duration
        .map(Duration::Time)
        .unwrap_or(params.stale_bid_duration);

    params.bid_removal_reward_percent = bid_removal_reward_bps
        .map(Decimal::percent)
        .unwrap_or(params.bid_removal_reward_percent);

    SUDO_PARAMS.save(deps.storage, &params)?;

    Ok(Response::new().add_attribute("action", "update_params"))
}

pub fn sudo_add_operator(deps: DepsMut, operator: Addr) -> Result<Response, ContractError> {
    let mut params = SUDO_PARAMS.load(deps.storage)?;
    if !params.operators.iter().any(|o| o == &operator) {
        params.operators.push(operator.clone());
    } else {
        return Err(ContractError::OperatorAlreadyRegistered {});
    }
    SUDO_PARAMS.save(deps.storage, &params)?;
    let res = Response::new()
        .add_attribute("action", "add_operator")
        .add_attribute("operator", operator);
    Ok(res)
}

pub fn sudo_remove_operator(deps: DepsMut, operator: Addr) -> Result<Response, ContractError> {
    let mut params = SUDO_PARAMS.load(deps.storage)?;
    if let Some(i) = params.operators.iter().position(|o| o == &operator) {
        params.operators.remove(i);
    } else {
        return Err(ContractError::OperatorNotRegistered {});
    }
    SUDO_PARAMS.save(deps.storage, &params)?;
    let res = Response::new()
        .add_attribute("action", "remove_operator")
        .add_attribute("operator", operator);
    Ok(res)
}

pub fn sudo_add_sale_hook(deps: DepsMut, hook: Addr) -> Result<Response, ContractError> {
    SALE_HOOKS.add_hook(deps.storage, hook.clone())?;

    let res = Response::new()
        .add_attribute("action", "add_sale_hook")
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

pub fn sudo_add_bid_hook(deps: DepsMut, _env: Env, hook: Addr) -> Result<Response, ContractError> {
    BID_HOOKS.add_hook(deps.storage, hook.clone())?;

    let res = Response::new()
        .add_attribute("action", "add_bid_hook")
        .add_attribute("hook", hook);
    Ok(res)
}

pub fn sudo_remove_sale_hook(deps: DepsMut, hook: Addr) -> Result<Response, ContractError> {
    SALE_HOOKS.remove_hook(deps.storage, hook.clone())?;

    let res = Response::new()
        .add_attribute("action", "remove_sale_hook")
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

pub fn sudo_remove_bid_hook(deps: DepsMut, hook: Addr) -> Result<Response, ContractError> {
    BID_HOOKS.remove_hook(deps.storage, hook.clone())?;

    let res = Response::new()
        .add_attribute("action", "remove_bid_hook")
        .add_attribute("hook", hook);
    Ok(res)
}
