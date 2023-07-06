use cosmwasm_schema::cw_serde;
use cosmwasm_std::{coin, Addr, Coin, Decimal, StdError};
use sg721::RoyaltyInfo;
use sg_std::Response;
use stargaze_fair_burn::append_fair_burn_msg;

use crate::coin::transfer_coins;

/// `TokenPayment` is a struct that represents a payment of a single token to a single address.
#[cw_serde]
pub struct TokenPayment {
    pub label: String,
    pub coin: Coin,
    pub recipient: Addr,
}

/// Invoke `payout_nft_sale_fees` to build a vector of `TokenPayment` structs representing the fees
/// paid out in an NFT sale.
#[allow(clippy::too_many_arguments)]
pub fn payout_nft_sale_fees(
    sale_coin: &Coin,
    seller: &Addr,
    fair_burn: &Addr,
    fair_burn_recipient: Option<&Addr>,
    finder: Option<&Addr>,
    trading_fee_percent: Decimal,
    finders_fee_percent: Option<Decimal>,
    royalty_info: Option<RoyaltyInfo>,
    mut response: Response,
) -> Result<(Vec<TokenPayment>, Response), StdError> {
    let mut token_payments: Vec<TokenPayment> = Vec::new();
    let mut seller_amount = sale_coin.amount;
    let denom = sale_coin.denom.clone();

    // Fair Burn
    let fair_burn_fee = coin(
        sale_coin.amount.mul_ceil(trading_fee_percent).u128(),
        &denom,
    );
    token_payments.push(TokenPayment {
        label: "fair-burn".to_string(),
        coin: fair_burn_fee.clone(),
        recipient: fair_burn.clone(),
    });
    seller_amount = seller_amount.checked_sub(fair_burn_fee.amount)?;

    // Finders Fee
    if let Some(finder) = finder {
        let finders_fee_percent = finders_fee_percent.unwrap();
        let finders_fee_amount = sale_coin.amount.mul_ceil(finders_fee_percent);

        if !finders_fee_amount.is_zero() {
            token_payments.push(TokenPayment {
                label: "finder".to_string(),
                coin: coin(finders_fee_amount.u128(), &denom),
                recipient: finder.clone(),
            });
            seller_amount = seller_amount.checked_sub(finders_fee_amount)?;
        }
    }

    // Royalty Fee
    if let Some(royalty_info) = royalty_info {
        let royalty_fee_amount = sale_coin.amount.mul_ceil(royalty_info.share);
        if !royalty_fee_amount.is_zero() {
            token_payments.push(TokenPayment {
                label: "royalty".to_string(),
                coin: coin(royalty_fee_amount.u128(), &denom),
                recipient: royalty_info.payment_address,
            });
            seller_amount = seller_amount.checked_sub(royalty_fee_amount)?;
        }
    }

    // Seller Payment
    if !seller_amount.is_zero() {
        token_payments.push(TokenPayment {
            label: "seller".to_string(),
            coin: coin(seller_amount.u128(), &denom),
            recipient: seller.clone(),
        });
    }

    for token_payment in &token_payments {
        response = if token_payment.label == "fair-burn" {
            append_fair_burn_msg(
                fair_burn,
                vec![token_payment.coin.clone()],
                fair_burn_recipient,
                response,
            )
        } else {
            response.add_submessage(transfer_coins(
                vec![token_payment.coin.clone()],
                &token_payment.recipient,
            ))
        }
    }

    Ok((token_payments, response))
}
