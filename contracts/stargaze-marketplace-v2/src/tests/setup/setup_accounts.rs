use crate::error::ContractError;
use crate::tests::setup::setup_contracts::{ATOM_DENOM, JUNO_DENOM, NATIVE_DENOM};

use cosmwasm_std::{coin, coins, Addr, Coin};
use cw_multi_test::{App, SudoMsg as CwSudoMsg};
use cw_multi_test::{BankSudo, SudoMsg};

// all amounts in ustars
pub const INITIAL_BALANCE: u128 = 5_000_000_000;

pub struct TestAccounts {
    pub creator: Addr,
    pub owner: Addr,
    pub bidder: Addr,
    pub fee_manager: Addr,
}

// initializes accounts with balances
pub fn setup_accounts(app: &mut App) -> Result<TestAccounts, ContractError> {
    let creator: Addr = Addr::unchecked("creator");
    let owner: Addr = Addr::unchecked("owner");
    let bidder: Addr = Addr::unchecked("bidder");
    let fee_manager: Addr = Addr::unchecked("fee_manager");
    let funds: Vec<Coin> = vec![
        coin(INITIAL_BALANCE, ATOM_DENOM),
        coin(INITIAL_BALANCE, JUNO_DENOM),
        coin(INITIAL_BALANCE, NATIVE_DENOM),
    ];
    app.sudo(SudoMsg::Bank({
        BankSudo::Mint {
            to_address: owner.to_string(),
            amount: funds.clone(),
        }
    }))
    .map_err(|err| println!("{:?}", err))
    .ok();
    app.sudo(SudoMsg::Bank({
        BankSudo::Mint {
            to_address: bidder.to_string(),
            amount: funds.clone(),
        }
    }))
    .map_err(|err| println!("{:?}", err))
    .ok();
    app.sudo(SudoMsg::Bank({
        BankSudo::Mint {
            to_address: creator.to_string(),
            amount: funds.clone(),
        }
    }))
    .map_err(|err| println!("{:?}", err))
    .ok();

    // Check native balances
    let owner_native_balances = app.wrap().query_all_balances(owner.clone()).unwrap();
    assert_eq!(owner_native_balances, funds);
    let bidder_native_balances = app.wrap().query_all_balances(bidder.clone()).unwrap();
    assert_eq!(bidder_native_balances, funds);
    let creator_native_balances = app.wrap().query_all_balances(creator.clone()).unwrap();
    assert_eq!(creator_native_balances, funds);

    Ok(TestAccounts {
        creator,
        owner,
        bidder,
        fee_manager,
    })
}

pub fn setup_additional_account(app: &mut App, addr_input: &str) -> Result<Addr, ContractError> {
    let addr: Addr = Addr::unchecked(addr_input);
    let funds: Vec<Coin> = coins(INITIAL_BALANCE, NATIVE_DENOM);
    app.sudo(CwSudoMsg::Bank({
        BankSudo::Mint {
            to_address: addr.to_string(),
            amount: funds.clone(),
        }
    }))
    .map_err(|err| println!("{:?}", err))
    .ok();

    let addr_native_balances = app.wrap().query_all_balances(addr.clone()).unwrap();
    assert_eq!(addr_native_balances, funds);

    Ok(addr)
}
