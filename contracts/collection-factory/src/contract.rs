#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Order, Reply, Response,
    StdResult, SubMsg, WasmMsg,
};
use cw2::set_contract_version;
use cw_utils::parse_reply_instantiate_data;
use sg721::state::CollectionInfo;

use crate::error::ContractError;
use crate::msg::{CollectionsResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{State, COLLECTIONS, STATE};
use cw721_base::helpers::Cw721Contract;
use cw721_base::{
    ExecuteMsg as Cw721ExecuteMsg, MintMsg, MinterResponse, QueryMsg as Cw721QueryMsg,
};
use http::Uri;
use sg721::msg::QueryMsg as Sg721QueryMsg;
use sg721::msg::{CreatorResponse, InstantiateMsg as SG721InstantiateMsg};
use std::str;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:collection-factory";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// id for sub-message reply
const REPLY_INIT_COLLECTION_ID: u64 = 1;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        owner: info.sender.clone(),
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::InitCollection {
            code_id,
            name,
            symbol,
            collection_info,
        } => execute_init_collection(deps, info, env, code_id, name, symbol, collection_info),
        ExecuteMsg::Mint {
            collection,
            token_uri,
        } => execute_mint(deps, info, env, collection, token_uri),
    }
}

pub fn execute_init_collection(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    code_id: u64,
    name: String,
    symbol: String,
    collection_info: CollectionInfo,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    if info.sender != state.owner {
        return Err(ContractError::Unauthorized {});
    }

    let msg = WasmMsg::Instantiate {
        admin: Some(state.owner.into_string()),
        code_id,
        funds: info.funds,
        msg: to_binary(&SG721InstantiateMsg {
            name: name.to_owned(),
            symbol: symbol.to_owned(),
            minter: info.sender.to_string(),
            collection_info,
        })?,
        label: format!("{}-{}-{}", symbol, name, code_id),
    };

    Ok(Response::new()
        .add_attribute("method", "init_collection")
        .add_submessage(SubMsg::reply_on_success(msg, REPLY_INIT_COLLECTION_ID)))
}

/// Mints a new token with the token id incremented by one
pub fn execute_mint(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    collection: String,
    token_uri: String,
) -> Result<Response, ContractError> {
    // TODO: validate funds against a mint fee
    // https://github.com/public-awesome/contracts/issues/50

    token_uri.parse::<Uri>()?;

    let contract_addr = deps.api.addr_validate(&collection)?;
    let sg721 = Cw721Contract(contract_addr.clone());

    let res: CreatorResponse = deps
        .querier
        .query_wasm_smart(contract_addr.to_string(), &Sg721QueryMsg::Creator {})?;
    if info.sender != res.creator {
        return Err(ContractError::Unauthorized {});
    }

    let token_id = sg721.num_tokens(&deps.querier)? + 1;

    let mint_msg = Cw721ExecuteMsg::Mint(MintMsg {
        token_id: token_id.to_string(),
        owner: info.sender.to_string(),
        token_uri: Some(token_uri),
        extension: Empty {},
    });

    // send mint msg
    let msg = WasmMsg::Execute {
        contract_addr: contract_addr.to_string(),
        msg: to_binary(&mint_msg)?,
        funds: info.funds,
    };

    Ok(Response::new()
        .add_attribute("method", "mint")
        .add_attribute("token_id", token_id.to_string())
        .add_message(msg))
}

/// Handles the reply from the VM after a new collection contract has been created
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, reply: Reply) -> Result<Response, ContractError> {
    if reply.id != REPLY_INIT_COLLECTION_ID {
        return Err(ContractError::UnknownReplyId { id: reply.id });
    }

    // get the contract address from the sub-message reply
    let contract_address = match parse_reply_instantiate_data(reply) {
        Ok(res) => res.contract_address,
        Err(_) => return Err(ContractError::InvalidReplyData {}),
    };
    let contract_addr = deps.api.addr_validate(&contract_address)?;

    let res: CreatorResponse = deps
        .querier
        .query_wasm_smart(contract_address.to_string(), &Sg721QueryMsg::Creator {})?;

    // save creator <> contract in storage
    COLLECTIONS.save(deps.storage, (&res.creator, &contract_addr), &Empty {})?;

    Ok(Response::default().add_attribute("contract_address", contract_address))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Collections { creator } => to_binary(&query_collections(deps, creator)?),
    }
}

fn query_collections(deps: Deps, creator: Addr) -> StdResult<CollectionsResponse> {
    let collections = COLLECTIONS
        .prefix(&creator)
        .range_raw(deps.storage, None, None, Order::Ascending)
        .filter_map(|item| item.map(|k| String::from_utf8(k.0)).ok())
        // unwrap and unchecked are safe here because the addresses have already been validated
        .map(|s| Addr::unchecked(s.unwrap()))
        .collect();

    Ok(CollectionsResponse { collections })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock_querier::mock_dependencies;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{Addr, ContractResult, Reply, SubMsgExecutionResponse};

    fn setup_contract(deps: DepsMut) {
        let msg = InstantiateMsg {};
        let info = mock_info("creator", &[]);
        let res = instantiate(deps, mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);
        setup_contract(deps.as_mut());
    }

    #[test]
    fn exec_init_collection() {
        let mut deps = mock_dependencies(&[]);
        let creator = String::from("creator");
        let collection = String::from("collection0");
        setup_contract(deps.as_mut());

        let info = mock_info(&creator, &[]);

        let msg = ExecuteMsg::InitCollection {
            code_id: 1,
            name: collection.to_string(),
            symbol: "SYM".to_string(),
            collection_info: CollectionInfo {
                contract_uri: String::from("https://bafyreibvxty5gjyeedk7or7tahyrzgbrwjkolpairjap3bmegvcjdipt74.ipfs.dweb.link/metadata.json"),
                creator: Addr::unchecked(creator),
                royalties: None,
            },
        };

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.messages.len(), 1);

        let reply_msg = Reply {
            id: REPLY_INIT_COLLECTION_ID,
            result: ContractResult::Ok(SubMsgExecutionResponse {
                events: vec![],
                // "collection0" in binary
                data: Some(vec![10, 11, 99, 111, 108, 108, 101, 99, 116, 105, 111, 110, 48].into()),
            }),
        };

        // register mock creator info querier
        deps.querier.with_creator(&[(
            &collection,
            &CreatorResponse {
                creator: Addr::unchecked("creator"),
            },
        )]);

        // simulate a reply coming in from the VM
        let _res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

        let res = query_collections(deps.as_ref(), Addr::unchecked("creator")).unwrap();
        assert_eq!(res.collections.len(), 1);
    }
}
