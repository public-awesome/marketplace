use crate::{constants::NATIVE_DENOM, sale::NftSaleProcessor};

use cosmwasm_std::{coin, Addr, BankMsg, CosmosMsg, Decimal, Response, Uint128};

pub struct RoyaltyInfo {
    pub payment_address: Addr,
    pub share: Decimal,
}

#[test]
fn try_payout_nft_sale_fees() {
    let sale_price = Uint128::from(10_000u64);
    let sale_coin = coin(sale_price.u128(), NATIVE_DENOM);
    let trading_fee_percent = Decimal::bps(200);
    let fair_burn = Addr::unchecked("fair-burn");
    let finder = Addr::unchecked("finder");
    let seller = Addr::unchecked("seller");

    let finders_fee_bps = 300u64;
    let royalty_info = RoyaltyInfo {
        payment_address: Addr::unchecked("royalty"),
        share: Decimal::percent(5u64),
    };

    let mut nft_sale_processor = NftSaleProcessor::new(sale_coin.clone(), seller.clone());
    nft_sale_processor.add_fee(
        "fair-burn".to_string(),
        trading_fee_percent,
        fair_burn.clone(),
    );
    nft_sale_processor.build_payments().unwrap();
    let response = nft_sale_processor.payout(Response::new());

    for event in response.events.iter() {
        if event.ty == "wasm-fair-burn-fee" {
            assert_eq!(event.attributes[0].key, "coin");
            assert_eq!(
                event.attributes[0].value,
                coin(200u128, NATIVE_DENOM).to_string()
            );
            assert_eq!(event.attributes[1].key, "recipient");
            assert_eq!(event.attributes[2].value, fair_burn.to_string());
        }
    }

    let mut nft_sale_processor = NftSaleProcessor::new(sale_coin, seller);
    nft_sale_processor.add_fee(
        "fair-burn".to_string(),
        trading_fee_percent,
        fair_burn.clone(),
    );
    nft_sale_processor.add_fee(
        "royalty".to_string(),
        royalty_info.share,
        royalty_info.payment_address.clone(),
    );
    nft_sale_processor.add_fee(
        "finder".to_string(),
        Decimal::bps(finders_fee_bps),
        finder.clone(),
    );
    nft_sale_processor.build_payments().unwrap();
    let response = nft_sale_processor.payout(Response::new());

    let assert_fair_burn_payment = coin(200u128, NATIVE_DENOM);
    let assert_finder_payment = coin(300u128, NATIVE_DENOM);
    let assert_royalty_payment = coin(500u128, NATIVE_DENOM);
    let assert_seller_payment = coin(9000u128, NATIVE_DENOM);
    for event in response.events.iter() {
        if event.ty == "fair-burn-fee" {
            assert_eq!(event.attributes[0].key, "coin");
            assert_eq!(
                event.attributes[0].value,
                coin(200u128, NATIVE_DENOM).to_string()
            );
            assert_eq!(event.attributes[1].key, "recipient");
            assert_eq!(event.attributes[1].value, fair_burn.to_string());
        } else if event.ty == "wasm-finder-fee" {
            assert_eq!(event.attributes[0].key, "coin");
            assert_eq!(
                event.attributes[0].value,
                coin(300u128, NATIVE_DENOM).to_string()
            );
            assert_eq!(event.attributes[1].key, "recipient");
            assert_eq!(event.attributes[1].value, finder.to_string());
        } else if event.ty == "wasm-royalty-fee" {
            assert_eq!(event.attributes[0].key, "coin");
            assert_eq!(
                event.attributes[0].value,
                coin(500u128, NATIVE_DENOM).to_string()
            );
            assert_eq!(event.attributes[1].key, "recipient");
            assert_eq!(
                event.attributes[1].value,
                royalty_info.payment_address.to_string()
            );
        }
    }

    match response.messages[0].msg.clone() {
        CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
            assert_eq!(to_address, "fair-burn");
            assert_eq!(amount[0], assert_fair_burn_payment);
        }
        _ => panic!("Unexpected message type"),
    }

    match response.messages[1].msg.clone() {
        CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
            assert_eq!(to_address, "royalty");
            assert_eq!(amount[0], assert_royalty_payment);
        }
        _ => panic!("Unexpected message type"),
    }

    match response.messages[2].msg.clone() {
        CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
            assert_eq!(to_address, "finder");
            assert_eq!(amount[0], assert_finder_payment);
        }
        _ => panic!("Unexpected message type"),
    }

    match response.messages[3].msg.clone() {
        CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
            assert_eq!(to_address, "seller");
            assert_eq!(amount[0], assert_seller_payment);
        }
        _ => panic!("Unexpected message type"),
    }
}
