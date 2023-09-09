use cosmwasm_std::{coin, Addr, BankMsg, Decimal, Uint128, WasmMsg};
use sg721::RoyaltyInfo;
use sg_std::{CosmosMsg, Response, NATIVE_DENOM};

use crate::{
    coin::bps_to_decimal,
    sale::{payout_nft_sale_fees, TokenPayment},
};

#[test]
fn try_payout_nft_sale_fees() {
    let sale_price = Uint128::from(10_000u64);
    let sale_coin = coin(sale_price.u128(), NATIVE_DENOM);
    let trading_fee_percent = bps_to_decimal(200);
    let fair_burn = Addr::unchecked("fair-burn");
    let finder = Addr::unchecked("finder");
    let seller = Addr::unchecked("seller");

    let finders_fee_bps = 300u64;
    let royalty_info = RoyaltyInfo {
        payment_address: Addr::unchecked("royalty"),
        share: Decimal::percent(5u64),
    };

    // Calculate correct fees with no finders fee and no royalties
    let (token_payments, _response) = payout_nft_sale_fees(
        &sale_coin,
        &seller,
        &fair_burn,
        None,
        None,
        trading_fee_percent,
        None,
        None,
        Response::new(),
    )
    .unwrap();

    assert_eq!(
        token_payments,
        vec![
            TokenPayment {
                label: "fair-burn".to_string(),
                coin: coin(200u128, NATIVE_DENOM),
                recipient: fair_burn.clone(),
            },
            TokenPayment {
                label: "seller".to_string(),
                coin: coin(9_800u128, NATIVE_DENOM),
                recipient: seller.clone(),
            },
        ]
    );

    let (token_payments, response) = payout_nft_sale_fees(
        &sale_coin,
        &seller,
        &fair_burn,
        None,
        Some(finder.clone()).as_ref(),
        trading_fee_percent,
        Some(bps_to_decimal(finders_fee_bps)),
        Some(royalty_info.clone()),
        Response::new(),
    )
    .unwrap();

    assert_eq!(
        token_payments,
        vec![
            TokenPayment {
                label: "fair-burn".to_string(),
                coin: coin(200u128, NATIVE_DENOM),
                recipient: fair_burn.clone(),
            },
            TokenPayment {
                label: "finder".to_string(),
                coin: coin(300u128, NATIVE_DENOM),
                recipient: finder,
            },
            TokenPayment {
                label: "royalty".to_string(),
                coin: coin(500u128, NATIVE_DENOM),
                recipient: royalty_info.payment_address,
            },
            TokenPayment {
                label: "seller".to_string(),
                coin: coin(9_000u128, NATIVE_DENOM),
                recipient: seller,
            },
        ]
    );

    match response.messages[0].msg.clone() {
        CosmosMsg::Wasm(WasmMsg::Execute { contract_addr, .. }) => {
            assert_eq!(contract_addr, fair_burn.to_string());
        }
        _ => panic!("Unexpected message type"),
    }

    match response.messages[1].msg.clone() {
        CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
            assert_eq!(to_address, "finder");
            assert_eq!(amount[0], token_payments[1].coin);
        }
        _ => panic!("Unexpected message type"),
    }

    match response.messages[2].msg.clone() {
        CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
            assert_eq!(to_address, "royalty");
            assert_eq!(amount[0], token_payments[2].coin);
        }
        _ => panic!("Unexpected message type"),
    }

    match response.messages[3].msg.clone() {
        CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
            assert_eq!(to_address, "seller");
            assert_eq!(amount[0], token_payments[3].coin);
        }
        _ => panic!("Unexpected message type"),
    }
}
