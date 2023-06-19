use cosmwasm_std::{
    to_binary, Addr, Api, BlockInfo, Deps, Empty, MessageInfo, QuerierWrapper, StdError, StdResult,
    WasmMsg,
};
use cw721::{ApprovalResponse, Cw721ExecuteMsg, OwnerOfResponse};
use cw721_base::helpers::Cw721Contract;
use sg721::RoyaltyInfo;
use sg721_base::msg::{CollectionInfoResponse, QueryMsg as Sg721QueryMsg};
use sg_std::SubMsg;
use std::marker::PhantomData;

pub use crate::errors::MarketplaceCommonError;

pub fn transfer_nft(collection: &Addr, token_id: &str, recipient: &Addr) -> SubMsg {
    SubMsg::new(WasmMsg::Execute {
        contract_addr: collection.to_string(),
        msg: to_binary(&Cw721ExecuteMsg::TransferNft {
            token_id: token_id.to_string(),
            recipient: recipient.to_string(),
        })
        .unwrap(),
        funds: vec![],
    })
}

pub fn owner_of(
    querier: &QuerierWrapper,
    collection: &Addr,
    token_id: &str,
) -> StdResult<OwnerOfResponse> {
    Cw721Contract::<Empty, Empty>(collection.clone(), PhantomData, PhantomData)
        .owner_of(querier, token_id, false)
}

pub fn only_owner(
    querier: &QuerierWrapper,
    info: &MessageInfo,
    collection: &Addr,
    token_id: &str,
) -> StdResult<()> {
    let owner_of_response = owner_of(querier, collection, token_id)?;
    if owner_of_response.owner != info.sender {
        return Err(StdError::generic_err("Unauthorized"));
    }
    Ok(())
}

pub fn has_approval(
    querier: &QuerierWrapper,
    spender: &Addr,
    collection: &Addr,
    token_id: &str,
    include_expired: Option<bool>,
) -> StdResult<ApprovalResponse> {
    Cw721Contract::<Empty, Empty>(collection.clone(), PhantomData, PhantomData).approval(
        querier,
        token_id,
        spender.as_str(),
        include_expired,
    )
}

/// Checks that the collection is tradable
pub fn only_tradable(
    deps: Deps,
    block: &BlockInfo,
    collection: &Addr,
) -> Result<bool, MarketplaceCommonError> {
    let res: Result<CollectionInfoResponse, StdError> = deps
        .querier
        .query_wasm_smart(collection.clone(), &Sg721QueryMsg::CollectionInfo {});

    match res {
        Ok(collection_info) => match collection_info.start_trading_time {
            Some(start_trading_time) => {
                if start_trading_time > block.time {
                    Err(MarketplaceCommonError::CollectionNotTradable {})
                } else {
                    Ok(true)
                }
            }
            // not set by collection, so tradable
            None => Ok(true),
        },
        // not supported by collection
        Err(_) => Ok(true),
    }
}

/// Load the collection royalties as defined on the NFT collection contract
pub fn load_collection_royalties(
    querier: &QuerierWrapper,
    api: &dyn Api,
    collection_addr: &Addr,
) -> StdResult<Option<RoyaltyInfo>> {
    let collection_info: CollectionInfoResponse =
        querier.query_wasm_smart(collection_addr, &Sg721QueryMsg::CollectionInfo {})?;

    let royalty_info: Option<RoyaltyInfo> = match collection_info.royalty_info {
        Some(royalty_info_response) => Some(RoyaltyInfo {
            share: royalty_info_response.share,
            payment_address: api.addr_validate(&royalty_info_response.payment_address)?,
        }),
        None => None,
    };

    Ok(royalty_info)
}
