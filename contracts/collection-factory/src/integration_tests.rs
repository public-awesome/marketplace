#![cfg(test)]
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, QueryMsg};

use cosmwasm_std::Empty;
use cw_multi_test::{App, BankSudo, Contract, ContractWrapper, Executor, SudoMsg};

fn mock_app() -> App {
    App::default()
}

pub fn contract_factory() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    )
    .with_reply(crate::contract::reply);
    Box::new(contract)
}

pub fn contract_cw721() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        sg721::contract::execute,
        sg721::contract::instantiate,
        sg721::contract::query,
    );
    Box::new(contract)
}

#[cfg(test)]
mod tests {
    use crate::msg::CollectionsResponse;

    use super::*;
    use cosmwasm_std::{coins, Addr, Coin};
    use sg721::state::CollectionInfo;

    const NATIVE_TOKEN_DENOM: &str = "ustars";
    const INITIAL_BALANCE: u128 = 2000;

    // Upload contract code and instantiate factory contract
    fn setup_factory_contract(router: &mut App, creator: &Addr) -> Result<Addr, ContractError> {
        // Upload contract code
        let _cw721_id = router.store_code(contract_cw721());
        let factory_id = router.store_code(contract_factory());

        // Instantiate factory contract
        let msg = crate::msg::InstantiateMsg {};
        let factory_addr = router
            .instantiate_contract(factory_id, creator.clone(), &msg, &[], "Factory", None)
            .unwrap();

        Ok(factory_addr)
    }

    // Add a creator account with initial balances
    fn setup_creator_account(router: &mut App) -> Result<Addr, ContractError> {
        let creator: Addr = Addr::unchecked("creator");
        let funds: Vec<Coin> = coins(INITIAL_BALANCE, NATIVE_TOKEN_DENOM);
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
        let creator_native_balances = router.wrap().query_all_balances(creator.clone()).unwrap();
        assert_eq!(creator_native_balances, funds);

        Ok(creator)
    }

    #[test]
    fn init_collection() {
        let mut router = mock_app();
        let creator = setup_creator_account(&mut router).unwrap();
        let factory_addr = setup_factory_contract(&mut router, &creator).unwrap();

        // Init a new collection
        let msg = ExecuteMsg::InitCollection {
            code_id: 1,
            name: "Collection".to_string(),
            symbol: "SYM".to_string(),
            collection_info: CollectionInfo {
                contract_uri: String::from("https://bafyreibvxty5gjyeedk7or7tahyrzgbrwjkolpairjap3bmegvcjdipt74.ipfs.dweb.link/metadata.json"),
                creator: creator.clone(),
                royalties: None,
            },
        };
        let res = router.execute_contract(creator.clone(), factory_addr.clone(), &msg, &[]);
        assert!(res.is_ok());

        // Query collections for creator
        let res: CollectionsResponse = router
            .wrap()
            .query_wasm_smart(factory_addr, &QueryMsg::Collections { creator })
            .unwrap();
        assert_eq!(res.collections.len(), 1);
    }
}
