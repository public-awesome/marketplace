use base_factory_new::state::Extension;
use base_minter_new::msg::ExecuteMsg as BaseMinterNewExecuteMsg;
use base_minter_old::msg::ExecuteMsg as BaseMinterOldExecuteMsg;
use cosm_orc::orchestrator::{Coin as OrcCoin, Denom as OrcDenom};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, Decimal, Empty, Timestamp, Uint128};
use sg2::msg::{CollectionParams, CreateMinterMsg, Sg2ExecuteMsg};
use sg721::{CollectionInfo, RoyaltyInfoResponse};
use sg721_base_old::msg::ExecuteMsg as Sg721BaseOldExecuteMsg;
use sg_marketplace::{msg::ExecuteMsg as MarketplaceExecuteMsg, state::SaleType};
use std::str::FromStr;
use test_context::test_context;

use crate::helpers::{
    chain::Chain,
    constants::{
        CREATE_MINTER_FEE, LISTING_FEE, MINT_FEE_BPS, MIN_MINT_FEE, NAME_BASE_FACTORY_NEW,
        NAME_BASE_FACTORY_OLD, NAME_BASE_MINTER_NEW, NAME_BASE_MINTER_OLD, NAME_MARKETPLACE,
        NAME_SG721_BASE_NEW, NAME_SG721_BASE_OLD,
    },
    helper::latest_block_time,
    instantiate::{instantiate_base_factory_new, instantiate_base_factory_old},
};

#[cw_serde]
pub struct RoyaltyInfo {
    pub payment_address: String,
    pub share: Decimal,
    pub updated_at: Option<Timestamp>,
}

#[test_context(Chain)]
#[test]
#[ignore]
fn test_royalty_info(chain: &mut Chain) {
    let denom = chain.cfg.orc_cfg.chain_cfg.denom.clone();
    let _prefix = chain.cfg.orc_cfg.chain_cfg.prefix.clone();

    let master_account = chain.cfg.users[0].clone();
    let buyer_account = chain.cfg.users[1].clone();
    let royalty_account = chain.cfg.users[2].clone();

    // Instantiate the factories
    let response = instantiate_base_factory_old(&mut chain.orc, &master_account).unwrap();
    let response = instantiate_base_factory_new(&mut chain.orc, &master_account).unwrap();

    // Instantiate the minters
    let response = chain
        .orc
        .execute(
            NAME_BASE_FACTORY_OLD,
            "instantiate-minter-old",
            &Sg2ExecuteMsg::<Extension>::CreateMinter(CreateMinterMsg {
                init_msg: None,
                collection_params: CollectionParams {
                    code_id: chain.orc.contract_map.code_id(NAME_SG721_BASE_OLD).unwrap(),
                    name: "Test Name".to_string(),
                    symbol: "SYM".to_string(),
                    info: CollectionInfo {
                        creator: master_account.account.address.clone(),
                        description: "Description".to_string(),
                        image: "https://google.com".to_string(),
                        external_link: None,
                        explicit_content: None,
                        start_trading_time: Some(Timestamp::from_seconds(0)),
                        royalty_info: Some(RoyaltyInfoResponse {
                            payment_address: royalty_account.account.address.clone(),
                            share: Decimal::percent(500) / Uint128::from(100u128),
                        }),
                    },
                },
            }),
            &master_account.key,
            vec![OrcCoin {
                amount: CREATE_MINTER_FEE,
                denom: OrcDenom::from_str(&denom.clone()).unwrap(),
            }],
        )
        .unwrap();
    let mut tags = response
        .res
        .find_event_tags("instantiate".to_string(), "_contract_address".to_string());
    assert_eq!(tags.len(), 2);
    chain
        .orc
        .contract_map
        .add_address(NAME_SG721_BASE_OLD, tags.pop().unwrap().value.clone())
        .unwrap();
    chain
        .orc
        .contract_map
        .add_address(NAME_BASE_MINTER_OLD, tags.pop().unwrap().value.clone())
        .unwrap();

    let response = chain
        .orc
        .execute(
            NAME_BASE_FACTORY_NEW,
            "instantiate-minter-new",
            &Sg2ExecuteMsg::CreateMinter::<Option<Empty>>(CreateMinterMsg {
                init_msg: None,
                collection_params: CollectionParams {
                    code_id: chain.orc.contract_map.code_id(NAME_SG721_BASE_NEW).unwrap(),
                    name: "Test Name".to_string(),
                    symbol: "SYM".to_string(),
                    info: CollectionInfo {
                        creator: master_account.account.address,
                        description: "Description".to_string(),
                        image: "https://google.com".to_string(),
                        external_link: None,
                        explicit_content: None,
                        start_trading_time: Some(Timestamp::from_seconds(0)),
                        royalty_info: Some(RoyaltyInfoResponse {
                            payment_address: royalty_account.account.address.clone(),
                            share: Decimal::percent(500) / Uint128::from(100u128),
                        }),
                    },
                },
            }),
            &master_account.key,
            vec![OrcCoin {
                amount: CREATE_MINTER_FEE,
                denom: OrcDenom::from_str(&denom.clone()).unwrap(),
            }],
        )
        .unwrap();
    let mut tags = response
        .res
        .find_event_tags("instantiate".to_string(), "_contract_address".to_string());
    assert_eq!(tags.len(), 2);
    chain
        .orc
        .contract_map
        .add_address(NAME_SG721_BASE_NEW, tags.pop().unwrap().value.clone())
        .unwrap();
    chain
        .orc
        .contract_map
        .add_address(NAME_BASE_MINTER_NEW, tags.pop().unwrap().value.clone())
        .unwrap();

    // Mint NFTs
    let mut token_ids: Vec<String> = vec![];
    let response = chain.orc.execute(
        NAME_BASE_MINTER_OLD,
        "minter-old-mint",
        &BaseMinterOldExecuteMsg::Mint {
            token_uri:
                "ipfs://bafybeigi3bwpvyvsmnbj46ra4hyffcxdeaj6ntfk5jpic5mx27x6ih2qvq/images/1.png"
                    .to_string(),
        },
        &master_account.key,
        vec![OrcCoin {
            amount: (Uint128::from(MIN_MINT_FEE) * Decimal::percent(MINT_FEE_BPS)
                / Uint128::from(100u128))
            .u128(),
            denom: OrcDenom::from_str(&denom.clone()).unwrap(),
        }],
    ).unwrap();
    token_ids.push(
        response
            .res
            .find_event_tags("wasm".to_string(), "token_id".to_string())
            .remove(0)
            .value
            .clone(),
    );

    let response = chain.orc.execute(
        NAME_BASE_MINTER_NEW,
        "minter-new-mint",
        &BaseMinterNewExecuteMsg::Mint {
            token_uri:
                "ipfs://bafybeigi3bwpvyvsmnbj46ra4hyffcxdeaj6ntfk5jpic5mx27x6ih2qvq/images/1.png"
                    .to_string(),
        },
        &master_account.key,
        vec![OrcCoin {
            amount: (Uint128::from(MIN_MINT_FEE) * Decimal::percent(MINT_FEE_BPS)
                / Uint128::from(100u128))
            .u128(),
            denom: OrcDenom::from_str(&denom.clone()).unwrap(),
        }],
    ).unwrap();
    token_ids.push(
        response
            .res
            .find_event_tags("wasm".to_string(), "token_id".to_string())
            .remove(0)
            .value
            .clone(),
    );

    // Approve NFTs
    let response = chain
        .orc
        .execute(
            NAME_SG721_BASE_OLD,
            "sg721-old-approve",
            &Sg721BaseOldExecuteMsg::<Option<Empty>, Empty>::Approve {
                spender: chain.orc.contract_map.address(NAME_MARKETPLACE).unwrap(),
                token_id: token_ids[0].clone(),
                expires: None,
            },
            &master_account.key,
            vec![],
        )
        .unwrap();
    let response = chain
        .orc
        .execute(
            NAME_SG721_BASE_NEW,
            "sg721-new-approve",
            &Sg721BaseOldExecuteMsg::<Option<Empty>, Empty>::Approve {
                spender: chain.orc.contract_map.address(NAME_MARKETPLACE).unwrap(),
                token_id: token_ids[1].clone(),
                expires: None,
            },
            &master_account.key,
            vec![],
        )
        .unwrap();

    // Create Asks
    let block_time = latest_block_time(&chain.orc);
    let response = chain
        .orc
        .execute(
            NAME_MARKETPLACE,
            "marketplace-ask-old",
            &MarketplaceExecuteMsg::SetAsk {
                sale_type: SaleType::FixedPrice,
                collection: chain.orc.contract_map.address(NAME_SG721_BASE_OLD).unwrap(),
                token_id: token_ids[0].parse::<u32>().unwrap(),
                price: Coin {
                    denom: denom.clone(),
                    amount: Uint128::from(1_000u128),
                },
                funds_recipient: None,
                reserve_for: None,
                finders_fee_bps: None,
                expires: block_time.plus_seconds(100),
            },
            &master_account.key,
            vec![OrcCoin {
                amount: LISTING_FEE,
                denom: OrcDenom::from_str(&denom.clone()).unwrap(),
            }],
        )
        .unwrap();
    let response = chain
        .orc
        .execute(
            NAME_MARKETPLACE,
            "marketplace-ask-new",
            &MarketplaceExecuteMsg::SetAsk {
                sale_type: SaleType::FixedPrice,
                collection: chain.orc.contract_map.address(NAME_SG721_BASE_NEW).unwrap(),
                token_id: token_ids[1].parse::<u32>().unwrap(),
                price: Coin {
                    denom: denom.clone(),
                    amount: Uint128::from(1_000u128),
                },
                funds_recipient: None,
                reserve_for: None,
                finders_fee_bps: None,
                expires: block_time.plus_seconds(100),
            },
            &master_account.key,
            vec![OrcCoin {
                amount: LISTING_FEE,
                denom: OrcDenom::from_str(&denom.clone()).unwrap(),
            }],
        )
        .unwrap();

    // Buy Now
    let response = chain
        .orc
        .execute(
            NAME_MARKETPLACE,
            "marketplace-buy-now-old",
            &MarketplaceExecuteMsg::BuyNow {
                collection: chain.orc.contract_map.address(NAME_SG721_BASE_OLD).unwrap(),
                token_id: token_ids[0].parse::<u32>().unwrap(),
                expires: block_time.plus_seconds(100),
                finder: None,
                finders_fee_bps: None,
            },
            &buyer_account.key,
            vec![OrcCoin {
                amount: 1_000u128,
                denom: OrcDenom::from_str(&denom.clone()).unwrap(),
            }],
        )
        .unwrap();
    let response = chain
        .orc
        .execute(
            NAME_MARKETPLACE,
            "marketplace-buy-now-new",
            &MarketplaceExecuteMsg::BuyNow {
                collection: chain.orc.contract_map.address(NAME_SG721_BASE_NEW).unwrap(),
                token_id: token_ids[1].parse::<u32>().unwrap(),
                expires: block_time.plus_seconds(100),
                finder: None,
                finders_fee_bps: None,
            },
            &buyer_account.key,
            vec![OrcCoin {
                amount: 1_000u128,
                denom: OrcDenom::from_str(&denom.clone()).unwrap(),
            }],
        )
        .unwrap();
}
