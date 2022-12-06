use crate::error::ContractError;
use crate::testing::setup::constants::{CREATION_FEE, SECOND_BIDDER_INITIAL_BALANCE};
use cosmwasm_std::{coins, Addr, Coin};
use cw_multi_test::{BankSudo, SudoMsg as CwSudoMsg};
use sg_multi_test::StargazeApp;
use sg_std::NATIVE_DENOM;

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
