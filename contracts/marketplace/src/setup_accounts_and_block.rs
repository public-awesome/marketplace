use sg721_base::ContractError;
use sg_std::NATIVE_DENOM;
use cosmwasm_std::{coins, Addr, Timestamp, Coin};
use cw_multi_test::{BankSudo, SudoMsg};
use sg_multi_test::StargazeApp;

pub const TOKEN_ID: u32 = 123;
pub const CREATOR_INITIAL_BALANCE: u128 = 5000000000;
pub const LISTING_FEE: u128 = 0;
pub const INITIAL_BALANCE: u128 = 5000000000;

// // Set blockchain time to after mint by default
// pub fn setup_block_time(router: &mut StargazeApp, nanos: u64, height: Option<u64>) {
//     let mut block = router.block_info();
//     block.time = Timestamp::from_nanos(nanos);
//     if let Some(h) = height {
//         block.height = h;
//     }
//     router.set_block(block);
// }

pub fn setup_block_time(router: &mut StargazeApp, seconds: u64) {
    let mut block = router.block_info();
    block.time = Timestamp::from_seconds(seconds);
    router.set_block(block);
}

// initializes accounts with balances
pub fn setup_accounts(router: &mut StargazeApp) -> Result<(Addr, Addr, Addr), ContractError> {
    let owner: Addr = Addr::unchecked("owner");
    let bidder: Addr = Addr::unchecked("bidder");
    let creator: Addr = Addr::unchecked("creator");
    let creator_funds: Vec<Coin> = coins(2* INITIAL_BALANCE, NATIVE_DENOM);
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