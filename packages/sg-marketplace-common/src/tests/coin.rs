use cosmwasm_std::{coin, Addr};
use sg_std::NATIVE_DENOM;

use crate::{
    coin::{checked_transfer_coin, transfer_coin},
    MarketplaceCommonError,
};

#[test]
fn try_transfer_coin() {
    let recipient = Addr::unchecked("recipient");
    transfer_coin(coin(100u128, NATIVE_DENOM), &recipient);
}

#[test]
fn try_checked_transfer_coin() {
    let recipient = Addr::unchecked("recipient");

    assert_eq!(
        Err(MarketplaceCommonError::ZeroAmountBankSend),
        checked_transfer_coin(coin(0u128, NATIVE_DENOM), &recipient)
    );

    let msg = checked_transfer_coin(coin(1000u128, NATIVE_DENOM), &recipient);
    assert!(msg.is_ok());
}
