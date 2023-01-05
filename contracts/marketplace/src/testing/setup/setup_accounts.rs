use crate::error::ContractError;
use cosmwasm_std::{coins, Addr, Coin};
use cw_multi_test::SudoMsg as CwSudoMsg;
use cw_multi_test::{BankSudo, SudoMsg};
use sg_multi_test::StargazeApp;
use sg_std::NATIVE_DENOM;

// all amounts in ustars
pub const INITIAL_BALANCE: u128 = 5_000_000_000;
pub const CREATION_FEE: u128 = 5_000_000_000;
pub const MINT_PRICE: u128 = 100_000_000;
pub const SECOND_BIDDER_INITIAL_BALANCE: u128 = 2000;

// initializes accounts with balances
pub fn setup_accounts(router: &mut StargazeApp) -> Result<(Addr, Addr, Addr), ContractError> {
    let owner: Addr = Addr::unchecked("owner");
    let bidder: Addr = Addr::unchecked("bidder");
    let creator: Addr = Addr::unchecked("creator");
    let creator_funds: Vec<Coin> = coins(2 * INITIAL_BALANCE, NATIVE_DENOM);
    let funds: Vec<Coin> = coins(INITIAL_BALANCE, NATIVE_DENOM);
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
                amount: creator_funds.clone(),
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
    assert_eq!(creator_native_balances, creator_funds);

    Ok((owner, bidder, creator))
}

pub fn setup_second_bidder_account(router: &mut StargazeApp) -> Result<Addr, ContractError> {
    let bidder2: Addr = Addr::unchecked("bidder2");
    let funds: Vec<Coin> = coins(CREATION_FEE + SECOND_BIDDER_INITIAL_BALANCE, NATIVE_DENOM);
    router
        .sudo(CwSudoMsg::Bank({
            BankSudo::Mint {
                to_address: bidder2.to_string(),
                amount: funds.clone(),
            }
        }))
        .map_err(|err| println!("{:?}", err))
        .ok();

    // Check native balances
    let bidder_native_balances = router.wrap().query_all_balances(bidder2.clone()).unwrap();
    assert_eq!(bidder_native_balances, funds);

    Ok(bidder2)
}
