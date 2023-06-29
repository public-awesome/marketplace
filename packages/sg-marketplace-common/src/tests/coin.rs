use cosmwasm_std::{coin, Addr, Decimal};
use sg_std::NATIVE_DENOM;

use crate::{
    coin::{
        bps_to_decimal, checked_transfer_coin, checked_transfer_coins, decimal_to_bps,
        transfer_coin, transfer_coins,
    },
    MarketplaceCommonError,
};

#[test]
fn try_transfer_coin() {
    let recipient = Addr::unchecked("recipient");

    let funds = vec![coin(100u128, NATIVE_DENOM)];
    let submsg = transfer_coin(funds[0].clone(), &recipient);
    match submsg.msg {
        cosmwasm_std::CosmosMsg::Bank(bank_msg) => match bank_msg {
            cosmwasm_std::BankMsg::Send { to_address, amount } => {
                assert_eq!(to_address, recipient);
                assert_eq!(amount, funds);
            }
            _ => panic!("Unexpected bank message type"),
        },
        _ => panic!("Unexpected message type"),
    }
}

#[test]
fn try_transfer_coins() {
    let recipient = Addr::unchecked("recipient");

    let funds = vec![coin(100u128, NATIVE_DENOM), coin(100u128, "uosmo")];
    let submsg = transfer_coins(funds.clone(), &recipient);
    match submsg.msg {
        cosmwasm_std::CosmosMsg::Bank(bank_msg) => match bank_msg {
            cosmwasm_std::BankMsg::Send { to_address, amount } => {
                assert_eq!(to_address, recipient);
                assert_eq!(amount, funds);
            }
            _ => panic!("Unexpected bank message type"),
        },
        _ => panic!("Unexpected message type"),
    }
}

#[test]
fn try_checked_transfer_coin() {
    let recipient = Addr::unchecked("recipient");

    assert_eq!(
        Err(MarketplaceCommonError::ZeroAmountBankSend),
        checked_transfer_coin(coin(0u128, NATIVE_DENOM), &recipient)
    );

    let funds = vec![coin(1000u128, NATIVE_DENOM)];
    let submsg = checked_transfer_coin(funds[0].clone(), &recipient).unwrap();
    match submsg.msg {
        cosmwasm_std::CosmosMsg::Bank(bank_msg) => match bank_msg {
            cosmwasm_std::BankMsg::Send { to_address, amount } => {
                assert_eq!(to_address, recipient);
                assert_eq!(amount, funds);
            }
            _ => panic!("Unexpected bank message type"),
        },
        _ => panic!("Unexpected message type"),
    }
}

#[test]
fn try_checked_transfer_coins() {
    let recipient = Addr::unchecked("recipient");

    assert_eq!(
        Err(MarketplaceCommonError::ZeroAmountBankSend),
        checked_transfer_coins(vec![coin(0u128, NATIVE_DENOM)], &recipient)
    );

    let funds = vec![coin(1000u128, NATIVE_DENOM), coin(1000u128, "uosmo")];
    let submsg = checked_transfer_coins(funds.clone(), &recipient).unwrap();
    match submsg.msg {
        cosmwasm_std::CosmosMsg::Bank(bank_msg) => match bank_msg {
            cosmwasm_std::BankMsg::Send { to_address, amount } => {
                assert_eq!(to_address, recipient);
                assert_eq!(amount, funds);
            }
            _ => panic!("Unexpected bank message type"),
        },
        _ => panic!("Unexpected message type"),
    }
}

#[test]
fn try_decimal_to_bps() {
    let d = Decimal::one();
    assert_eq!(decimal_to_bps(d), 10_000u128);
}

#[test]
fn try_bps_to_decimal() {
    let bps = 10_000u64;
    assert_eq!(bps_to_decimal(bps), Decimal::one());
}
