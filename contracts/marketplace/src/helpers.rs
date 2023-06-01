use crate::{msg::ExecuteMsg, ContractError};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    coin, to_binary, Addr, Api, BlockInfo, Coin, Decimal, StdError, StdResult, Timestamp, Uint128,
    WasmMsg,
};
use sg1::fair_burn;
use sg721::RoyaltyInfo;
use sg_marketplace_common::{checked_transfer_coin, transfer_coin};
use sg_std::{CosmosMsg, Response, NATIVE_DENOM};
use thiserror::Error;

/// MarketplaceContract is a wrapper around Addr that provides a lot of helpers
#[cw_serde]
pub struct MarketplaceContract(pub Addr);

impl MarketplaceContract {
    pub fn addr(&self) -> Addr {
        self.0.clone()
    }

    pub fn call<T: Into<ExecuteMsg>>(&self, msg: T) -> StdResult<CosmosMsg> {
        let msg = to_binary(&msg.into())?;
        Ok(WasmMsg::Execute {
            contract_addr: self.addr().into(),
            msg,
            funds: vec![],
        }
        .into())
    }
}

pub fn map_validate(api: &dyn Api, addresses: &[String]) -> StdResult<Vec<Addr>> {
    let mut validated_addresses = addresses
        .iter()
        .map(|addr| api.addr_validate(addr))
        .collect::<StdResult<Vec<_>>>()?;
    validated_addresses.sort();
    validated_addresses.dedup();
    Ok(validated_addresses)
}

#[derive(Error, Debug, PartialEq)]
pub enum ExpiryRangeError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Invalid expiration range")]
    InvalidExpirationRange {},

    #[error("Expiry min > max")]
    InvalidExpiry {},
}

#[cw_serde]
pub struct ExpiryRange {
    pub min: u64,
    pub max: u64,
}

impl ExpiryRange {
    pub fn new(min: u64, max: u64) -> Self {
        ExpiryRange { min, max }
    }

    /// Validates if given expires time is within the allowable range
    pub fn is_valid(&self, block: &BlockInfo, expires: Timestamp) -> Result<(), ExpiryRangeError> {
        let now = block.time;
        if !(expires > now.plus_seconds(self.min) && expires <= now.plus_seconds(self.max)) {
            return Err(ExpiryRangeError::InvalidExpirationRange {});
        }

        Ok(())
    }

    pub fn validate(&self) -> Result<(), ExpiryRangeError> {
        if self.min > self.max {
            return Err(ExpiryRangeError::InvalidExpiry {});
        }

        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct TokenPayment {
    pub coin: Coin,
    pub recipient: Addr,
}

#[derive(Debug, PartialEq, Clone)]
pub struct TransactionFees {
    pub fair_burn_fee: Uint128,
    pub seller_payment: TokenPayment,
    pub finders_fee: Option<TokenPayment>,
    pub royalty_fee: Option<TokenPayment>,
}

pub fn calculate_nft_sale_fees(
    sale_price: Uint128,
    trading_fee_percent: Decimal,
    seller: Addr,
    finder: Option<Addr>,
    finders_fee_bps: Option<u64>,
    royalty_info: Option<RoyaltyInfo>,
) -> StdResult<TransactionFees> {
    // Calculate Fair Burn
    let fair_burn_fee = sale_price * trading_fee_percent / Uint128::from(100u128);

    let mut seller_payment = sale_price.checked_sub(fair_burn_fee)?;

    // Calculate finders fee
    let mut finders_fee: Option<TokenPayment> = None;
    if let Some(_finder) = finder {
        let finders_fee_bps = finders_fee_bps.unwrap_or(0);
        let finders_fee_amount =
            (sale_price * Decimal::percent(finders_fee_bps) / Uint128::from(100u128)).u128();

        if finders_fee_amount > 0 {
            finders_fee = Some(TokenPayment {
                coin: coin(finders_fee_amount, NATIVE_DENOM),
                recipient: _finder,
            });
            seller_payment = seller_payment.checked_sub(Uint128::from(finders_fee_amount))?;
        }
    };

    // Calculate royalty
    let mut royalty_fee: Option<TokenPayment> = None;
    if let Some(_royalty_info) = royalty_info {
        let royalty_fee_amount = (sale_price * _royalty_info.share).u128();
        if royalty_fee_amount > 0 {
            royalty_fee = Some(TokenPayment {
                coin: coin(royalty_fee_amount, NATIVE_DENOM),
                recipient: _royalty_info.payment_address,
            });
            seller_payment = seller_payment.checked_sub(Uint128::from(royalty_fee_amount))?;
        }
    };

    // Pay seller
    let seller_payment = TokenPayment {
        coin: coin(seller_payment.u128(), NATIVE_DENOM),
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
    response: Response,
    tx_fees: TransactionFees,
    developer: Option<Addr>,
) -> Result<Response, ContractError> {
    let mut response = response;

    if tx_fees.fair_burn_fee == Uint128::zero() {
        return Err(ContractError::InvalidFairBurn(
            "fair burn fee cannot be 0".to_string(),
        ));
    }
    fair_burn(tx_fees.fair_burn_fee.u128(), developer, &mut response);

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

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::mock_dependencies;

    #[test]
    fn test_map_validate() {
        let deps = mock_dependencies();
        let adddreses = map_validate(
            &deps.api,
            &[
                "operator1".to_string(),
                "operator2".to_string(),
                "operator3".to_string(),
            ],
        )
        .unwrap();
        assert_eq!(3, adddreses.len());

        let adddreses = map_validate(
            &deps.api,
            &[
                "operator1".to_string(),
                "operator2".to_string(),
                "operator3".to_string(),
                "operator3".to_string(),
                "operator1".to_string(),
            ],
        )
        .unwrap();

        assert_eq!(
            adddreses,
            vec![
                Addr::unchecked("operator1".to_string()),
                Addr::unchecked("operator2".to_string()),
                Addr::unchecked("operator3".to_string()),
            ]
        )
    }
}
