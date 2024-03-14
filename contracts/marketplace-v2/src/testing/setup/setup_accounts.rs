use crate::error::ContractError;
use crate::testing::setup::setup_marketplace::{ATOM_DENOM, JUNO_DENOM};
use cosmwasm_std::{coin, coins, Addr, Coin};
use cw_multi_test::SudoMsg as CwSudoMsg;
use cw_multi_test::{BankSudo, SudoMsg};
use sg_multi_test::StargazeApp;
use sg_std::NATIVE_DENOM;

// all amounts in ustars
pub const INITIAL_BALANCE: u128 = 5_000_000_000;

// initializes accounts with balances
pub fn setup_accounts(router: &mut StargazeApp) -> Result<(Addr, Addr, Addr), ContractError> {
    let owner: Addr = Addr::unchecked("owner");
    let bidder: Addr = Addr::unchecked("bidder");
    let creator: Addr = Addr::unchecked("creator");
    let funds: Vec<Coin> = vec![
        coin(INITIAL_BALANCE, ATOM_DENOM),
        coin(INITIAL_BALANCE, JUNO_DENOM),
        coin(INITIAL_BALANCE, NATIVE_DENOM),
    ];
    router
        .sudo(SudoMsg::Bank({
            BankSudo::Mint {
                to_address: owner.to_string(),
                amount: funds.clone(),
            }
        }))
        .map_err(|err| println!("{:?}", err))
        .ok();
    router
        .sudo(SudoMsg::Bank({
            BankSudo::Mint {
                to_address: bidder.to_string(),
                amount: funds.clone(),
            }
        }))
        .map_err(|err| println!("{:?}", err))
        .ok();
    router
        .sudo(SudoMsg::Bank({
            BankSudo::Mint {
                to_address: creator.to_string(),
                amount: funds.clone(),
            }
        }))
        .map_err(|err| println!("{:?}", err))
        .ok();

    // Check native balances
    let owner_native_balances = router.wrap().query_all_balances(owner.clone()).unwrap();
    assert_eq!(owner_native_balances, funds);
    let bidder_native_balances = router.wrap().query_all_balances(bidder.clone()).unwrap();
    assert_eq!(bidder_native_balances, funds);
    let creator_native_balances = router.wrap().query_all_balances(creator.clone()).unwrap();
    assert_eq!(creator_native_balances, funds);

    Ok((owner, bidder, creator))
}

pub fn setup_additional_account(
    router: &mut StargazeApp,
    addr_input: &str,
) -> Result<Addr, ContractError> {
    let addr: Addr = Addr::unchecked(addr_input);
    let funds: Vec<Coin> = coins(INITIAL_BALANCE, NATIVE_DENOM);
    router
        .sudo(CwSudoMsg::Bank({
            BankSudo::Mint {
                to_address: addr.to_string(),
                amount: funds.clone(),
            }
        }))
        .map_err(|err| println!("{:?}", err))
        .ok();

    let addr_native_balances = router.wrap().query_all_balances(addr.clone()).unwrap();
    assert_eq!(addr_native_balances, funds);

    Ok(addr)
}
