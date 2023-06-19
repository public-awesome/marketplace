use cosmwasm_std::{Addr, Coin, Deps, StdResult, WasmMsg};
use sg_std::SubMsg;

use crate::msg::{AskHookMsg, CollectionOfferHookMsg, HookAction, OfferHookMsg, SaleHookMsg};
use crate::reply::HookReply;
use crate::state::{
    Ask, CollectionOffer, Offer, ASK_HOOKS, COLLECTION_OFFER_HOOKS, OFFER_HOOKS, SALE_HOOKS,
};

pub fn prepare_ask_hook(deps: Deps, ask: &Ask, action: HookAction) -> StdResult<Vec<SubMsg>> {
    let submsgs = ASK_HOOKS.prepare_hooks(deps.storage, |h| {
        let msg = AskHookMsg { ask: ask.clone() };
        let execute = WasmMsg::Execute {
            contract_addr: h.to_string(),
            msg: msg.into_binary(action.clone())?,
            funds: vec![],
        };
        Ok(SubMsg::reply_on_error(execute, HookReply::Ask as u64))
    })?;

    Ok(submsgs)
}

pub fn prepare_sale_hook(
    deps: Deps,
    collection: &Addr,
    token_id: &String,
    price: &Coin,
    seller: &Addr,
    buyer: &Addr,
) -> StdResult<Vec<SubMsg>> {
    let submsgs = SALE_HOOKS.prepare_hooks(deps.storage, |h| {
        let msg = SaleHookMsg {
            collection: collection.to_string(),
            token_id: token_id.to_string(),
            price: price.clone(),
            seller: seller.to_string(),
            buyer: buyer.to_string(),
        };
        let execute = WasmMsg::Execute {
            contract_addr: h.to_string(),
            msg: msg.into_binary()?,
            funds: vec![],
        };
        Ok(SubMsg::reply_on_error(execute, HookReply::Sale as u64))
    })?;

    Ok(submsgs)
}

pub fn prepare_offer_hook(deps: Deps, offer: &Offer, action: HookAction) -> StdResult<Vec<SubMsg>> {
    let submsgs = OFFER_HOOKS.prepare_hooks(deps.storage, |h| {
        let msg = OfferHookMsg {
            offer: offer.clone(),
        };
        let execute = WasmMsg::Execute {
            contract_addr: h.to_string(),
            msg: msg.into_binary(action.clone())?,
            funds: vec![],
        };
        Ok(SubMsg::reply_on_error(execute, HookReply::Offer as u64))
    })?;

    Ok(submsgs)
}

pub fn prepare_collection_offer_hook(
    deps: Deps,
    collection_offer: &CollectionOffer,
    action: HookAction,
) -> StdResult<Vec<SubMsg>> {
    let submsgs = COLLECTION_OFFER_HOOKS.prepare_hooks(deps.storage, |h| {
        let msg = CollectionOfferHookMsg {
            collection_offer: collection_offer.clone(),
        };
        let execute = WasmMsg::Execute {
            contract_addr: h.to_string(),
            msg: msg.into_binary(action.clone())?,
            funds: vec![],
        };
        Ok(SubMsg::reply_on_error(
            execute,
            HookReply::CollectionOffer as u64,
        ))
    })?;

    Ok(submsgs)
}
