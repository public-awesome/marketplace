#![cfg(test)]
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, QueryMsg};
use cosmwasm_std::{Addr, Empty};
use cw721::{Cw721QueryMsg, OwnerOfResponse};
use cw721_base::msg::{ExecuteMsg as Cw721ExecuteMsg, MintMsg};
use cw_multi_test::{BankSudo, Contract, ContractWrapper, Executor, SudoMsg};
use sg_multi_test::StargazeApp;
use sg_std::StargazeMsgWrapper;

fn custom_mock_app() -> StargazeApp {
    StargazeApp::default()
}

pub fn contract_nft_marketplace() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

pub fn contract_sg721() -> Box<dyn Contract<StargazeMsgWrapper>> {
    let contract = ContractWrapper::new(
        sg721::contract::execute,
        sg721::contract::instantiate,
        sg721::contract::query,
    );
    Box::new(contract)
}

#[cfg(test)]
mod tests {
    use crate::msg::{Bid, BidResponse};

    use super::*;
    use cosmwasm_std::{coin, coins, Coin, Decimal};
    use sg721::msg::{InstantiateMsg as Sg721InstantiateMsg, RoyaltyInfoResponse};
    use sg721::state::CollectionInfo;
    use sg_multi_test::StargazeApp;
    use sg_std::NATIVE_DENOM;

    const TOKEN_ID: u32 = 123;
    const CREATION_FEE: u128 = 1_000_000_000;
    const INITIAL_BALANCE: u128 = 2000;

    // Instantiates all needed contracts for testing
    fn setup_contracts(
        router: &mut StargazeApp,
        creator: &Addr,
    ) -> Result<(Addr, Addr), ContractError> {
        // Instantiate marketplace contract
        let marketplace_id = router.store_code(contract_nft_marketplace());
        let msg = crate::msg::InstantiateMsg {};
        let nft_marketplace_addr = router
            .instantiate_contract(
                marketplace_id,
                creator.clone(),
                &msg,
                &[],
                "Marketplace",
                None,
            )
            .unwrap();

        // Setup media contract
        let sg721_id = router.store_code(contract_sg721());
        let msg = Sg721InstantiateMsg {
            name: String::from("Test Coin"),
            symbol: String::from("TEST"),
            minter: creator.to_string(),
            collection_info: CollectionInfo {
                creator: creator.to_string(),
                description: String::from("Stargaze Monkeys"),
                image: "ipfs://bafybeigi3bwpvyvsmnbj46ra4hyffcxdeaj6ntfk5jpic5mx27x6ih2qvq/images/1.png".to_string(),
                external_link: Some("https://example.com/external.html".to_string()),
                royalty_info: Some(RoyaltyInfoResponse {
                    payment_address: creator.to_string(),
                    share: Decimal::percent(10),
                }),
            },
        };
        let nft_contract_addr = router
            .instantiate_contract(
                sg721_id,
                creator.clone(),
                &msg,
                &coins(CREATION_FEE, NATIVE_DENOM),
                "NFT",
                None,
            )
            .unwrap();

        Ok((nft_marketplace_addr, nft_contract_addr))
    }

    // Intializes accounts with balances
    fn setup_accounts(router: &mut StargazeApp) -> Result<(Addr, Addr, Addr), ContractError> {
        let owner: Addr = Addr::unchecked("owner");
        let bidder: Addr = Addr::unchecked("bidder");
        let creator: Addr = Addr::unchecked("creator");
        let creator_funds: Vec<Coin> = coins(CREATION_FEE, NATIVE_DENOM);
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

    // Mints an NFT for a creator
    fn mint_nft_for_creator(router: &mut StargazeApp, creator: &Addr, nft_contract_addr: &Addr) {
        let mint_for_creator_msg = Cw721ExecuteMsg::Mint(MintMsg {
            token_id: TOKEN_ID.to_string(),
            owner: creator.clone().to_string(),
            token_uri: Some("https://starships.example.com/Starship/Enterprise.json".into()),
            extension: Empty {},
        });
        let res = router.execute_contract(
            creator.clone(),
            nft_contract_addr.clone(),
            &mint_for_creator_msg,
            &[],
        );
        assert!(res.is_ok());
    }

    #[test]
    fn try_set_accept_bid() {
        let mut router = custom_mock_app();

        // Setup intial accounts
        let (_owner, bidder, creator) = setup_accounts(&mut router).unwrap();

        // Instantiate and configure contracts
        let (nft_marketplace_addr, nft_contract_addr) =
            setup_contracts(&mut router, &creator).unwrap();

        // Mint NFT for creator
        mint_nft_for_creator(&mut router, &creator, &nft_contract_addr);

        // Creator Authorizes NFT
        let approve_msg = Cw721ExecuteMsg::<Empty>::Approve {
            spender: nft_marketplace_addr.to_string(),
            token_id: TOKEN_ID.to_string(),
            expires: None,
        };
        let res = router.execute_contract(
            creator.clone(),
            nft_contract_addr.clone(),
            &approve_msg,
            &[],
        );
        assert!(res.is_ok());

        // An asking price is made by the creator
        let set_ask = ExecuteMsg::SetAsk {
            collection: nft_contract_addr.to_string(),
            token_id: TOKEN_ID,
            price: coin(110, NATIVE_DENOM),
            funds_recipient: None,
        };
        let res =
            router.execute_contract(creator.clone(), nft_marketplace_addr.clone(), &set_ask, &[]);
        assert!(res.is_ok());

        // Bidder makes bid
        let set_bid_msg = ExecuteMsg::SetBid {
            collection: nft_contract_addr.to_string(),
            token_id: TOKEN_ID,
        };
        let res = router.execute_contract(
            bidder.clone(),
            nft_marketplace_addr.clone(),
            &set_bid_msg,
            &coins(100, NATIVE_DENOM),
        );
        assert!(res.is_ok());

        // Check contract has been paid
        let contract_balances = router
            .wrap()
            .query_all_balances(nft_marketplace_addr.clone())
            .unwrap();
        assert_eq!(contract_balances, coins(100, NATIVE_DENOM));

        // Check creator hasn't been paid yet
        let creator_native_balances = router.wrap().query_all_balances(creator.clone()).unwrap();
        assert_eq!(creator_native_balances, vec![]);

        // Creator accepts bid
        let accept_bid_msg = ExecuteMsg::AcceptBid {
            collection: nft_contract_addr.to_string(),
            token_id: TOKEN_ID,
            bidder: bidder.to_string(),
        };
        let res = router.execute_contract(
            creator.clone(),
            nft_marketplace_addr.clone(),
            &accept_bid_msg,
            &[],
        );
        assert!(res.is_ok());

        // Check money is transfered
        let creator_native_balances = router.wrap().query_all_balances(creator).unwrap();
        assert_eq!(creator_native_balances, coins(100, NATIVE_DENOM));
        let bidder_native_balances = router.wrap().query_all_balances(bidder.clone()).unwrap();
        assert_eq!(
            bidder_native_balances,
            coins(INITIAL_BALANCE - 100, NATIVE_DENOM)
        );

        // Check NFT is transferred
        let query_owner_msg = Cw721QueryMsg::OwnerOf {
            token_id: TOKEN_ID.to_string(),
            include_expired: None,
        };
        let res: OwnerOfResponse = router
            .wrap()
            .query_wasm_smart(nft_contract_addr, &query_owner_msg)
            .unwrap();
        assert_eq!(res.owner, bidder.to_string());

        // Check contract has zero balance
        let contract_balances = router
            .wrap()
            .query_all_balances(nft_marketplace_addr)
            .unwrap();
        assert_eq!(contract_balances, []);
    }

    #[test]
    fn auto_accept_bid() {
        let mut router = custom_mock_app();

        // Setup intial accounts
        let (_owner, bidder, creator) = setup_accounts(&mut router).unwrap();

        // Instantiate and configure contracts
        let (nft_marketplace_addr, nft_contract_addr) =
            setup_contracts(&mut router, &creator).unwrap();

        // Mint NFT for creator
        mint_nft_for_creator(&mut router, &creator, &nft_contract_addr);

        // An ask is made by the creator, but fails because NFT is not authorized
        let set_ask = ExecuteMsg::SetAsk {
            collection: nft_contract_addr.to_string(),
            token_id: TOKEN_ID,
            price: coin(100, NATIVE_DENOM),
            funds_recipient: None,
        };
        let res =
            router.execute_contract(creator.clone(), nft_marketplace_addr.clone(), &set_ask, &[]);
        assert!(res.is_err());

        // Creator Authorizes NFT
        let approve_msg = Cw721ExecuteMsg::<Empty>::Approve {
            spender: nft_marketplace_addr.to_string(),
            token_id: TOKEN_ID.to_string(),
            expires: None,
        };
        let res = router.execute_contract(
            creator.clone(),
            nft_contract_addr.clone(),
            &approve_msg,
            &[],
        );
        assert!(res.is_ok());

        // Now set_ask succeeds
        let res =
            router.execute_contract(creator.clone(), nft_marketplace_addr.clone(), &set_ask, &[]);
        assert!(res.is_ok());

        // Bidder makes bid with a random token in the same amount as the ask
        router
            .sudo(SudoMsg::Bank({
                BankSudo::Mint {
                    to_address: bidder.to_string(),
                    amount: coins(1000, "random"),
                }
            }))
            .map_err(|err| println!("{:?}", err))
            .ok();
        let set_bid_msg = ExecuteMsg::SetBid {
            collection: nft_contract_addr.to_string(),
            token_id: TOKEN_ID,
        };
        router
            .execute_contract(
                bidder.clone(),
                nft_marketplace_addr.clone(),
                &set_bid_msg,
                &coins(100, "random"),
            )
            .unwrap_err();

        // Bidder makes bid that meets the ask criteria
        let set_bid_msg = ExecuteMsg::SetBid {
            collection: nft_contract_addr.to_string(),
            token_id: TOKEN_ID,
        };
        let res = router
            .execute_contract(
                bidder.clone(),
                nft_marketplace_addr,
                &set_bid_msg,
                &coins(100, NATIVE_DENOM),
            )
            .unwrap();

        // Bid is accepted, sale has been finalized
        assert_eq!("sale_finalized", res.events[1].attributes[1].value);

        // Check money is transfered
        let creator_native_balances = router.wrap().query_all_balances(creator).unwrap();
        assert_eq!(creator_native_balances, coins(100, NATIVE_DENOM));
        let bidder_native_balances = router.wrap().query_all_balances(bidder.clone()).unwrap();
        assert_eq!(
            bidder_native_balances,
            vec![
                coin(1000, "random"),
                coin(INITIAL_BALANCE - 100, NATIVE_DENOM),
            ]
        );

        // Check NFT is transferred
        let query_owner_msg = Cw721QueryMsg::OwnerOf {
            token_id: TOKEN_ID.to_string(),
            include_expired: None,
        };
        let res: OwnerOfResponse = router
            .wrap()
            .query_wasm_smart(nft_contract_addr, &query_owner_msg)
            .unwrap();
        assert_eq!(res.owner, bidder.to_string());
    }

    #[test]
    fn remove_bid_refund() {
        let mut router = custom_mock_app();

        // Setup intial accounts
        let (_owner, bidder, creator) = setup_accounts(&mut router).unwrap();

        // Instantiate and configure contracts
        let (nft_marketplace_addr, nft_contract_addr) =
            setup_contracts(&mut router, &creator).unwrap();

        // Mint NFT for creator
        mint_nft_for_creator(&mut router, &creator, &nft_contract_addr);

        // Creator Authorizes NFT
        let approve_msg = Cw721ExecuteMsg::<Empty>::Approve {
            spender: nft_marketplace_addr.to_string(),
            token_id: TOKEN_ID.to_string(),
            expires: None,
        };
        let res = router.execute_contract(
            creator.clone(),
            nft_contract_addr.clone(),
            &approve_msg,
            &[],
        );
        assert!(res.is_ok());

        // An asking price is made by the creator
        let set_ask = ExecuteMsg::SetAsk {
            collection: nft_contract_addr.to_string(),
            token_id: TOKEN_ID,
            price: coin(110, NATIVE_DENOM),
            funds_recipient: None,
        };
        let res =
            router.execute_contract(creator.clone(), nft_marketplace_addr.clone(), &set_ask, &[]);
        assert!(res.is_ok());

        // Bidder makes bid
        let set_bid_msg = ExecuteMsg::SetBid {
            collection: nft_contract_addr.to_string(),
            token_id: TOKEN_ID,
        };

        let res = router.execute_contract(
            bidder.clone(),
            nft_marketplace_addr.clone(),
            &set_bid_msg,
            &coins(100, NATIVE_DENOM),
        );
        assert!(res.is_ok());

        // Bidder sent bid money
        let bidder_native_balances = router.wrap().query_all_balances(bidder.clone()).unwrap();
        assert_eq!(
            bidder_native_balances,
            coins(INITIAL_BALANCE - 100, NATIVE_DENOM)
        );

        // Contract has been paid
        let contract_balances = router
            .wrap()
            .query_all_balances(nft_marketplace_addr.clone())
            .unwrap();
        assert_eq!(contract_balances, coins(100, NATIVE_DENOM));

        // Bidder removes bid
        let remove_bid_msg = ExecuteMsg::RemoveBid {
            collection: nft_contract_addr.to_string(),
            token_id: TOKEN_ID,
        };
        let res =
            router.execute_contract(bidder.clone(), nft_marketplace_addr, &remove_bid_msg, &[]);
        assert!(res.is_ok());

        // Bidder has money back
        let bidder_native_balances = router.wrap().query_all_balances(bidder).unwrap();
        assert_eq!(bidder_native_balances, coins(INITIAL_BALANCE, NATIVE_DENOM));
    }

    #[test]
    fn new_bid_refund() {
        let mut router = custom_mock_app();

        // Setup intial accounts
        let (_owner, bidder, creator) = setup_accounts(&mut router).unwrap();

        // Instantiate and configure contracts
        let (nft_marketplace_addr, nft_contract_addr) =
            setup_contracts(&mut router, &creator).unwrap();

        // Mint NFT for creator
        mint_nft_for_creator(&mut router, &creator, &nft_contract_addr);

        // Creator Authorizes NFT
        let approve_msg = Cw721ExecuteMsg::<Empty>::Approve {
            spender: nft_marketplace_addr.to_string(),
            token_id: TOKEN_ID.to_string(),
            expires: None,
        };
        let res = router.execute_contract(
            creator.clone(),
            nft_contract_addr.clone(),
            &approve_msg,
            &[],
        );
        assert!(res.is_ok());

        // An ask is made by the creator
        let set_ask = ExecuteMsg::SetAsk {
            collection: nft_contract_addr.to_string(),
            token_id: TOKEN_ID,
            price: coin(200, NATIVE_DENOM),
            funds_recipient: None,
        };
        let res =
            router.execute_contract(creator.clone(), nft_marketplace_addr.clone(), &set_ask, &[]);
        assert!(res.is_ok());

        // Bidder makes bid
        let set_bid_msg = ExecuteMsg::SetBid {
            collection: nft_contract_addr.to_string(),
            token_id: TOKEN_ID,
        };
        let res = router.execute_contract(
            bidder.clone(),
            nft_marketplace_addr.clone(),
            &set_bid_msg,
            &coins(100, NATIVE_DENOM),
        );
        assert!(res.is_ok());

        // Bidder sent bid money
        let bidder_native_balances = router.wrap().query_all_balances(bidder.clone()).unwrap();
        assert_eq!(
            bidder_native_balances,
            coins(INITIAL_BALANCE - 100, NATIVE_DENOM)
        );

        // Contract has been paid
        let contract_balances = router
            .wrap()
            .query_all_balances(nft_marketplace_addr.clone())
            .unwrap();
        assert_eq!(contract_balances, coins(100, NATIVE_DENOM));

        // Bidder makes higher bid
        let set_bid_msg = ExecuteMsg::SetBid {
            collection: nft_contract_addr.to_string(),
            token_id: TOKEN_ID,
        };
        let res = router.execute_contract(
            bidder.clone(),
            nft_marketplace_addr.clone(),
            &set_bid_msg,
            &coins(150, NATIVE_DENOM),
        );
        assert!(res.is_ok());

        // Bidder has money back
        let bidder_native_balances = router.wrap().query_all_balances(bidder.clone()).unwrap();
        assert_eq!(
            bidder_native_balances,
            coins(INITIAL_BALANCE - 150, NATIVE_DENOM)
        );

        // Contract has been paid
        let contract_balances = router
            .wrap()
            .query_all_balances(nft_marketplace_addr.clone())
            .unwrap();
        assert_eq!(contract_balances, coins(150, NATIVE_DENOM));

        // Check new bid has been saved
        let query_bid_msg = QueryMsg::Bid {
            collection: nft_contract_addr.to_string(),
            token_id: TOKEN_ID,
            bidder: bidder.to_string(),
        };
        let bid = coin(150, NATIVE_DENOM);

        let res: BidResponse = router
            .wrap()
            .query_wasm_smart(nft_marketplace_addr, &query_bid_msg)
            .unwrap();
        assert_eq!(Some(Bid { price: bid }), res.bid);
    }

    #[test]
    fn royalties() {
        let mut router = custom_mock_app();

        // Setup intial accounts
        let (curator, bidder, creator) = setup_accounts(&mut router).unwrap();

        // Instantiate marketplace contract
        let marketplace_id = router.store_code(contract_nft_marketplace());
        let msg = crate::msg::InstantiateMsg {};
        let nft_marketplace_addr = router
            .instantiate_contract(
                marketplace_id,
                creator.clone(),
                &msg,
                &[],
                "Marketplace",
                None,
            )
            .unwrap();

        // Setup media contract with 10% royalties to a curator
        let sg721_id = router.store_code(contract_sg721());
        let msg = sg721::msg::InstantiateMsg {
            name: String::from("Test Coin"),
            symbol: String::from("TEST"),
            minter: creator.to_string(),
            collection_info: CollectionInfo {
                creator: creator.to_string(),
                description: String::from("Stargaze Monkeys"),
                image: "https://example.com/image.png".to_string(),
                external_link: Some("https://example.com/external.html".to_string()),
                royalty_info: Some(RoyaltyInfoResponse {
                    payment_address: curator.to_string(),
                    share: Decimal::percent(10),
                }),
            },
        };
        let nft_contract_addr = router
            .instantiate_contract(
                sg721_id,
                creator.clone(),
                &msg,
                &coins(CREATION_FEE, NATIVE_DENOM),
                "NFT",
                None,
            )
            .unwrap();

        // Mint NFT for creator
        let mint_for_creator_msg = Cw721ExecuteMsg::Mint(MintMsg {
            token_id: TOKEN_ID.to_string(),
            owner: creator.to_string(),
            token_uri: Some("https://starships.example.com/Starship/Enterprise.json".into()),
            extension: Empty {},
        });
        let res = router.execute_contract(
            creator.clone(),
            nft_contract_addr.clone(),
            &mint_for_creator_msg,
            &[],
        );
        assert!(res.is_ok());

        // Creator Authorizes NFT
        let approve_msg = Cw721ExecuteMsg::<Empty>::Approve {
            spender: nft_marketplace_addr.to_string(),
            token_id: TOKEN_ID.to_string(),
            expires: None,
        };
        let res = router.execute_contract(
            creator.clone(),
            nft_contract_addr.clone(),
            &approve_msg,
            &[],
        );
        assert!(res.is_ok());

        // An ask is made by the creator
        let set_ask = ExecuteMsg::SetAsk {
            collection: nft_contract_addr.to_string(),
            token_id: TOKEN_ID,
            price: coin(100, NATIVE_DENOM),
            funds_recipient: None,
        };
        let res =
            router.execute_contract(creator.clone(), nft_marketplace_addr.clone(), &set_ask, &[]);
        assert!(res.is_ok());

        // Bidder makes bid
        let set_bid_msg = ExecuteMsg::SetBid {
            collection: nft_contract_addr.to_string(),
            token_id: TOKEN_ID,
        };
        let res = router.execute_contract(
            bidder.clone(),
            nft_marketplace_addr,
            &set_bid_msg,
            &coins(100, NATIVE_DENOM),
        );
        assert!(res.is_ok());

        // Check money is transfered correctly and royalties paid
        let curator_native_balances = router.wrap().query_all_balances(curator).unwrap();
        assert_eq!(
            curator_native_balances,
            coins(INITIAL_BALANCE + 10, NATIVE_DENOM)
        );
        let creator_native_balances = router.wrap().query_all_balances(creator).unwrap();
        assert_eq!(creator_native_balances, coins(90, NATIVE_DENOM));
        let bidder_native_balances = router.wrap().query_all_balances(bidder.clone()).unwrap();
        assert_eq!(
            bidder_native_balances,
            coins(INITIAL_BALANCE - 100, NATIVE_DENOM)
        );

        // Check NFT is transferred
        let query_owner_msg = Cw721QueryMsg::OwnerOf {
            token_id: TOKEN_ID.to_string(),
            include_expired: None,
        };
        let res: OwnerOfResponse = router
            .wrap()
            .query_wasm_smart(nft_contract_addr, &query_owner_msg)
            .unwrap();
        assert_eq!(res.owner, bidder.to_string());
    }
}
