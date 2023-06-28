mod errors;
mod tests;

pub use crate::errors::MarketplaceCommonError;

use cosmwasm_std::{
    coin, ensure, to_binary, Addr, Api, BankMsg, BlockInfo, Coin, Decimal, Deps, Empty,
    MessageInfo, QuerierWrapper, StdError, StdResult, Uint128, WasmMsg,
};
use cw721::{ApprovalResponse, Cw721ExecuteMsg, OwnerOfResponse};
use cw721_base::helpers::Cw721Contract;
use sg721::RoyaltyInfo;
use sg721_base::msg::{CollectionInfoResponse, QueryMsg as Sg721QueryMsg};
use sg_std::{Response, SubMsg};
use stargaze_fair_burn::append_fair_burn_msg;
use std::marker::PhantomData;

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

pub fn transfer_coin(send_coin: Coin, to: &Addr) -> SubMsg {
    SubMsg::new(BankMsg::Send {
        to_address: to.to_string(),
        amount: vec![send_coin],
    })
}

pub fn checked_transfer_coin(send_coin: Coin, to: &Addr) -> Result<SubMsg, MarketplaceCommonError> {
    if send_coin.amount.is_zero() {
        return Err(MarketplaceCommonError::ZeroAmountBankSend);
    }
    Ok(transfer_coin(send_coin, to))
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

#[derive(Debug, PartialEq, Clone)]
pub struct TokenPayment {
    pub coin: Coin,
    pub recipient: Addr,
}

#[derive(Debug, PartialEq, Clone)]
pub struct TransactionFees {
    pub fair_burn_fee: Coin,
    pub seller_payment: TokenPayment,
    pub finders_fee: Option<TokenPayment>,
    pub royalty_fee: Option<TokenPayment>,
}

/// Calculate fees for an NFT sale
pub fn calculate_nft_sale_fees(
    sale_coin: &Coin,
    trading_fee_percent: Decimal,
    seller: Addr,
    finder: Option<Addr>,
    finders_fee_bps: Option<u64>,
    royalty_info: Option<RoyaltyInfo>,
) -> StdResult<TransactionFees> {
    let denom = sale_coin.denom.clone();

    // Calculate Fair Burn
    let fair_burn_fee = coin(
        sale_coin
            .amount
            .mul_ceil(trading_fee_percent / Uint128::from(100u128))
            .u128(),
        &denom,
    );

    let mut seller_payment = sale_coin.amount.checked_sub(fair_burn_fee.amount)?;

    // Calculate finders fee
    let mut finders_fee: Option<TokenPayment> = None;
    if let Some(_finder) = finder {
        let finders_fee_bps = finders_fee_bps.unwrap_or(0);
        let finders_fee_amount =
            sale_coin.amount * Decimal::percent(finders_fee_bps) / Uint128::from(100u128);

        if !finders_fee_amount.is_zero() {
            finders_fee = Some(TokenPayment {
                coin: coin(finders_fee_amount.u128(), &denom),
                recipient: _finder,
            });
            seller_payment = seller_payment.checked_sub(finders_fee_amount)?;
        }
    };

    // Calculate royalty
    let mut royalty_fee: Option<TokenPayment> = None;
    if let Some(royalty_info) = royalty_info {
        let royalty_fee_amount = sale_coin.amount * royalty_info.share;
        if !royalty_fee_amount.is_zero() {
            royalty_fee = Some(TokenPayment {
                coin: coin(royalty_fee_amount.u128(), &denom),
                recipient: royalty_info.payment_address,
            });
            seller_payment = seller_payment.checked_sub(royalty_fee_amount)?;
        }
    };

    // Pay seller
    let seller_payment = TokenPayment {
        coin: coin(seller_payment.u128(), &denom),
        recipient: seller,
    };

    Ok(TransactionFees {
        fair_burn_fee,
        seller_payment,
        finders_fee,
        royalty_fee,
    })
}

pub fn payout_nft_sale_fees(
    fair_burn: &Addr,
    tx_fees: TransactionFees,
    recipient: Option<Addr>,
    mut response: Response,
) -> Result<Response, MarketplaceCommonError> {
    ensure!(
        !tx_fees.fair_burn_fee.amount.is_zero(),
        MarketplaceCommonError::InvalidFairBurn("fair burn fee cannot be 0".to_string(),)
    );

    response = append_fair_burn_msg(
        fair_burn,
        vec![tx_fees.fair_burn_fee],
        recipient.as_ref(),
        response,
    );

    if let Some(finders_fee) = &tx_fees.finders_fee {
        if finders_fee.coin.amount > Uint128::zero() {
            response = response.add_submessage(transfer_coin(
                finders_fee.coin.clone(),
                &finders_fee.recipient,
            ));
        }
    }

    if let Some(royalty_fee) = &tx_fees.royalty_fee {
        if royalty_fee.coin.amount > Uint128::zero() {
            response = response.add_submessage(transfer_coin(
                royalty_fee.coin.clone(),
                &royalty_fee.recipient,
            ));
        }
    }

    response = response.add_submessage(checked_transfer_coin(
        tx_fees.seller_payment.coin,
        &tx_fees.seller_payment.recipient,
    )?);

    Ok(response)
}
