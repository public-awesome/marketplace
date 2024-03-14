use crate::coin::transfer_coins;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{coin, Addr, Coin, Decimal, Event, StdError};
use sg_std::Response;
use stargaze_fair_burn::append_fair_burn_msg;
use std::fmt;

#[cw_serde]
pub enum FeeType {
    FairBurn,
    Royalty,
    Finder,
    Seller,
}

impl fmt::Display for FeeType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            FeeType::FairBurn => write!(f, "fair-burn-fee"),
            FeeType::Royalty => write!(f, "royalty-fee"),
            FeeType::Finder => write!(f, "finder-fee"),
            FeeType::Seller => write!(f, "seller-fee"),
        }
    }
}

pub struct Fee {
    pub fee_type: FeeType,
    pub recipient: Addr,
    pub share: Decimal,
}

#[cw_serde]
pub struct Payment {
    pub fee_type: FeeType,
    pub recipient: Addr,
    pub funds: Coin,
}

pub struct NftSaleProcessor {
    pub sale_coin: Coin,
    pub seller_recipient: Addr,
    pub fees: Vec<Fee>,
    pub payments: Vec<Payment>,
}

impl NftSaleProcessor {
    pub fn new(sale_coin: Coin, seller_recipient: Addr) -> Self {
        Self {
            sale_coin,
            seller_recipient,
            fees: vec![],
            payments: vec![],
        }
    }

    pub fn add_fee(&mut self, fee_type: FeeType, share: Decimal, recipient: Addr) {
        self.fees.push(Fee {
            fee_type,
            share,
            recipient,
        });
    }

    pub fn build_payments(&mut self) -> Result<(), StdError> {
        let mut seller_amount = self.sale_coin.amount;
        let denom = self.sale_coin.denom.clone();

        for fee in &self.fees {
            if fee.share.is_zero() {
                continue;
            }

            let fee_amount = self.sale_coin.amount.mul_ceil(fee.share);
            let fee_coin = coin(fee_amount.u128(), &denom);
            seller_amount = seller_amount.checked_sub(fee_amount)?;

            self.payments.push(Payment {
                fee_type: fee.fee_type.clone(),
                recipient: fee.recipient.clone(),
                funds: fee_coin,
            });
        }

        if !seller_amount.is_zero() {
            self.payments.push(Payment {
                fee_type: FeeType::Seller,
                recipient: self.seller_recipient.clone(),
                funds: coin(seller_amount.u128(), &denom),
            });
        }

        Ok(())
    }

    pub fn find_payment(&self, fee_type: FeeType) -> Option<&Payment> {
        self.payments.iter().find(|p| p.fee_type == fee_type)
    }

    pub fn payout(&self, mut response: Response) -> Response {
        for payment in self.payments.iter() {
            response = match payment.fee_type {
                FeeType::FairBurn => append_fair_burn_msg(
                    &payment.recipient,
                    vec![payment.funds.clone()],
                    None,
                    response,
                ),
                _ => transfer_coins(vec![payment.funds.clone()], &payment.recipient, response),
            };

            response = response.add_event(
                Event::new(payment.fee_type.to_string())
                    .add_attribute("coin", payment.funds.to_string())
                    .add_attribute("recipient", payment.recipient.to_string()),
            );
        }

        response
    }
}
