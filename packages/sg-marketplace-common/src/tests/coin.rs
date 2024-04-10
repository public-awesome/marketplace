use crate::{
    coin::{checked_transfer_coin, checked_transfer_coins, transfer_coin, transfer_coins},
    constants::NATIVE_DENOM,
    MarketplaceStdError,
};

use cosmwasm_std::{coin, Addr, Response};

#[test]
fn try_transfer_coin() {
    let recipient = Addr::unchecked("recipient");

    let funds = vec![coin(100u128, NATIVE_DENOM)];
    let response = transfer_coin(funds[0].clone(), &recipient, Response::new());

    match &response.messages[0].msg {
        cosmwasm_std::CosmosMsg::Bank(bank_msg) => match bank_msg {
            cosmwasm_std::BankMsg::Send { to_address, amount } => {
                assert_eq!(to_address, &recipient.to_string());
                assert_eq!(amount, &funds);
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
    let response = transfer_coins(funds.clone(), &recipient, Response::new());
    match &response.messages[0].msg {
        cosmwasm_std::CosmosMsg::Bank(bank_msg) => match bank_msg {
            cosmwasm_std::BankMsg::Send { to_address, amount } => {
                assert_eq!(to_address, &recipient.to_string());
                assert_eq!(amount, &funds);
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
        Err(MarketplaceStdError::ZeroAmountBankSend),
        checked_transfer_coin(coin(0u128, NATIVE_DENOM), &recipient, Response::new())
    );

    let funds = vec![coin(1000u128, NATIVE_DENOM)];
    let response = checked_transfer_coin(funds[0].clone(), &recipient, Response::new()).unwrap();
    match &response.messages[0].msg {
        cosmwasm_std::CosmosMsg::Bank(bank_msg) => match bank_msg {
            cosmwasm_std::BankMsg::Send { to_address, amount } => {
                assert_eq!(to_address, &recipient.to_string());
                assert_eq!(amount, &funds);
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
        Err(MarketplaceStdError::ZeroAmountBankSend),
        checked_transfer_coins(vec![coin(0u128, NATIVE_DENOM)], &recipient, Response::new())
    );

    let funds = vec![coin(1000u128, NATIVE_DENOM), coin(1000u128, "uosmo")];
    let response = checked_transfer_coins(funds.clone(), &recipient, Response::new()).unwrap();
    match &response.messages[0].msg {
        cosmwasm_std::CosmosMsg::Bank(bank_msg) => match bank_msg {
            cosmwasm_std::BankMsg::Send { to_address, amount } => {
                assert_eq!(to_address, &recipient.to_string());
                assert_eq!(amount, &funds);
            }
            _ => panic!("Unexpected bank message type"),
        },
        _ => panic!("Unexpected message type"),
    }
}
