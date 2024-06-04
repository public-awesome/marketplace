use crate::types::{CollectionInfoResponse, RoyaltyEntry, Sg721QueryMsg};

use cosmwasm_std::{
    to_json_binary, Addr, Decimal, Deps, QuerierWrapper, Response, StdResult, WasmMsg,
};
use stargaze_royalty_registry::msg::{
    ExecuteMsg as RoyaltyRegistryExecuteMsg, QueryMsg as RoyaltyRegistryQueryMsg,
    RoyaltyPaymentResponse,
};
use std::str::FromStr;

pub fn fetch_royalty_entry(
    querier: &QuerierWrapper,
    royalty_registry: &Addr,
    collection: &Addr,
    protocol: Option<&Addr>,
) -> StdResult<Option<RoyaltyEntry>> {
    let royalty_payment_response = querier.query_wasm_smart::<RoyaltyPaymentResponse>(
        royalty_registry,
        &RoyaltyRegistryQueryMsg::RoyaltyPayment {
            collection: collection.to_string(),
            protocol: protocol.map(|p| p.to_string()),
        },
    )?;

    if let Some(royalty_protocol) = royalty_payment_response.royalty_protocol {
        return Ok(Some(royalty_protocol.royalty_entry.into()));
    }

    if let Some(royalty_default) = royalty_payment_response.royalty_default {
        return Ok(Some(royalty_default.royalty_entry.into()));
    }

    Ok(None)
}

pub fn fetch_or_set_royalties(
    deps: Deps,
    royalty_registry: &Addr,
    collection: &Addr,
    protocol: Option<&Addr>,
    mut response: Response,
) -> StdResult<(Option<RoyaltyEntry>, Response)> {
    let royalty_entry = fetch_royalty_entry(&deps.querier, royalty_registry, collection, protocol)?;
    if let Some(royalty_entry) = royalty_entry {
        return Ok((Some(royalty_entry), response));
    }

    let collection_info = deps
        .querier
        .query_wasm_smart::<CollectionInfoResponse>(collection, &Sg721QueryMsg::CollectionInfo {});

    match collection_info {
        Ok(collection_info) => {
            if collection_info.royalty_info.is_none() {
                return Ok((None, response));
            }

            let royalty_info_response = collection_info.royalty_info.unwrap();
            let royalty_entry = RoyaltyEntry {
                recipient: deps
                    .api
                    .addr_validate(&royalty_info_response.payment_address)?,
                share: Decimal::from_str(&royalty_info_response.share.to_string()).unwrap(),
                updated: None,
            };

            response = response.add_message(WasmMsg::Execute {
                contract_addr: royalty_registry.to_string(),
                msg: to_json_binary(&RoyaltyRegistryExecuteMsg::InitializeCollectionRoyalty {
                    collection: collection.to_string(),
                })
                .unwrap(),
                funds: vec![],
            });

            Ok((Some(royalty_entry), response))
        }
        Err(_) => Ok((None, response)),
    }
}
