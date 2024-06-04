use crate::coin::transfer_coins;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{coin, Addr, Coin, Decimal, Response, StdError};

pub struct Fee {
    pub label: String,
    pub recipient: Addr,
    pub share: Decimal,
}

#[cw_serde]
pub struct Payment {
    pub label: String,
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

    pub fn add_fee(&mut self, label: String, share: Decimal, recipient: Addr) {
        self.fees.push(Fee {
            label,
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
                label: fee.label.clone(),
                recipient: fee.recipient.clone(),
                funds: fee_coin,
            });
        }

        if !seller_amount.is_zero() {
            self.payments.push(Payment {
                label: "seller".to_string(),
                recipient: self.seller_recipient.clone(),
                funds: coin(seller_amount.u128(), &denom),
            });
        }

        Ok(())
    }

    pub fn find_payment(&self, label: String) -> Option<&Payment> {
        self.payments.iter().find(|p| p.label == label)
    }

    pub fn payout(&self, mut response: Response) -> Response {
        for payment in self.payments.iter() {
            response = transfer_coins(vec![payment.funds.clone()], &payment.recipient, response);
        }

        response
    }
}
