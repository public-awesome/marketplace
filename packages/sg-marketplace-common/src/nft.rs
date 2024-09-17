use crate::{
    types::{CollectionInfoResponse, Sg721QueryMsg},
    MarketplaceStdError,
};

use cosmwasm_std::{
    to_json_binary, Addr, BlockInfo, MessageInfo, QuerierWrapper, Response, StdError, SubMsg,
    WasmMsg,
};
use cw721::{Cw721ExecuteMsg, Cw721QueryMsg, OwnerOfResponse};

/// Invoke `transfer_nft` to build a `SubMsg` to transfer an NFT to an address.
pub fn transfer_nft(
    collection: &Addr,
    token_id: &str,
    recipient: &Addr,
    response: Response,
) -> Response {
    response.add_submessage(SubMsg::new(WasmMsg::Execute {
        contract_addr: collection.to_string(),
        msg: to_json_binary(&Cw721ExecuteMsg::TransferNft {
            token_id: token_id.to_string(),
            recipient: recipient.to_string(),
        })
        .unwrap(),
        funds: vec![],
    }))
}

/// Invoke `only_owner` to check that the sender is the owner of the NFT.
pub fn only_owner(
    querier: &QuerierWrapper,
    info: &MessageInfo,
    collection: &Addr,
    token_id: &str,
) -> Result<(), MarketplaceStdError> {
    let owner_of_response = querier.query_wasm_smart::<OwnerOfResponse>(
        collection.clone(),
        &Cw721QueryMsg::OwnerOf {
            token_id: token_id.to_string(),
            include_expired: Some(false),
        },
    );

    match owner_of_response {
        Ok(owner_of_response) => {
            if owner_of_response.owner != info.sender {
                return Err(MarketplaceStdError::Unauthorized(
                    "sender is not owner".to_string(),
                ));
            }
            Ok(())
        }
        Err(_) => Ok(()),
    }
}

/// Invoke `only_tradable` to check that the NFT collection start trade time threshold has elapsed.
pub fn only_tradable(
    querier: &QuerierWrapper,
    block: &BlockInfo,
    collection: &Addr,
) -> Result<(), MarketplaceStdError> {
    let response: Result<CollectionInfoResponse, StdError> =
        querier.query_wasm_smart(collection.clone(), &Sg721QueryMsg::CollectionInfo {});

    match response {
        Ok(collection_info) => match collection_info.start_trading_time {
            Some(start_trading_time) => {
                if start_trading_time.nanos() > block.time.nanos() {
                    Err(MarketplaceStdError::CollectionNotTradable {})
                } else {
                    Ok(())
                }
            }
            // not set by collection, so tradable
            None => Ok(()),
        },
        // not supported by collection
        Err(_) => Ok(()),
    }
}
