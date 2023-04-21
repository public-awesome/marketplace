use std::marker::PhantomData;

use cosmwasm_std::{
    coin, to_binary, Addr, BankMsg, Coin, Deps, Empty, Event, MessageInfo, StdError, StdResult,
    Uint128, WasmMsg,
};
use cw721::{ApprovalResponse, Cw721ExecuteMsg, OwnerOfResponse};
use cw721_base::helpers::Cw721Contract;
use sg721_base::msg::{CollectionInfoResponse, QueryMsg as Sg721QueryMsg};
use sg_std::{Response, SubMsg, NATIVE_DENOM};

pub fn transfer_nft(collection: Addr, token_id: &str, receipient: Addr) -> StdResult<SubMsg> {
    Ok(SubMsg::new(WasmMsg::Execute {
        contract_addr: collection.to_string(),
        msg: to_binary(&Cw721ExecuteMsg::TransferNft {
            token_id: token_id.to_string(),
            recipient: receipient.to_string(),
        })?,
        funds: vec![],
    }))
}

pub fn bank_send(amount: Coin, to: Addr) -> StdResult<SubMsg> {
    Ok(SubMsg::new(BankMsg::Send {
        to_address: to.to_string(),
        amount: vec![amount],
    }))
}

pub fn only_owner(
    deps: Deps,
    info: &MessageInfo,
    collection: &Addr,
    token_id: &str,
) -> StdResult<OwnerOfResponse> {
    let res = Cw721Contract::<Empty, Empty>(collection.clone(), PhantomData, PhantomData)
        .owner_of(&deps.querier, token_id, false)?;
    if res.owner != info.sender {
        return Err(StdError::generic_err("Unauthorized"));
    }

    Ok(res)
}

pub fn owner_of(deps: Deps, collection: &Addr, token_id: &str) -> StdResult<Addr> {
    let res = Cw721Contract::<Empty, Empty>(collection.clone(), PhantomData, PhantomData)
        .owner_of(&deps.querier, token_id, false)?;

    Ok(Addr::unchecked(res.owner))
}

pub fn has_approval(
    deps: Deps,
    spender: &Addr,
    collection: &Addr,
    token_id: &str,
) -> StdResult<ApprovalResponse> {
    let res = Cw721Contract::<Empty, Empty>(collection.clone(), PhantomData, PhantomData)
        .approval(&deps.querier, token_id, spender.as_str(), None)?;

    Ok(res)
}

// royalty payout looks at collection info, determines if a royalty is paid out, and if so, returns the amount
// TODO move to sg721 package
pub fn royalty_payout(
    deps: Deps,
    collection: &Addr,
    payment: Uint128,
    res: &mut Response,
) -> StdResult<Uint128> {
    let collection_info: CollectionInfoResponse = deps
        .querier
        .query_wasm_smart(collection.clone(), &Sg721QueryMsg::CollectionInfo {})?;
    match collection_info.royalty_info {
        // If token supports royalties, payout shares to royalty recipient
        Some(royalty) => {
            let amount = coin((payment * royalty.share).u128(), NATIVE_DENOM);
            res.messages.push(SubMsg::new(BankMsg::Send {
                to_address: royalty.payment_address.to_string(),
                amount: vec![amount.clone()],
            }));

            let event = Event::new("royalty-payout")
                .add_attribute("collection", collection.to_string())
                .add_attribute("amount", amount.to_string())
                .add_attribute("recipient", royalty.payment_address);
            res.events.push(event);

            Ok(amount.amount)
        }
        None => Ok(Uint128::zero()),
    }
}
