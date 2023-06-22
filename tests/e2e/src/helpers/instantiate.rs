use base_factory_new::msg::InstantiateMsg as BaseFactoryNewInstantiateMsg;
use base_factory_new::state::BaseMinterParams as BaseMinterNewParams;
use base_factory_old::msg::InstantiateMsg as BaseFactoryOldInstantiateMsg;
use base_factory_old::state::BaseMinterParams as BaseMinterOldParams;
use cosm_orc::orchestrator::cosm_orc::CosmOrc;
use cosm_orc::orchestrator::error::ProcessError;
use cosm_orc::orchestrator::{CosmosgRPC, InstantiateResponse};
use cosmwasm_std::{coin, Uint128};
use cw_utils::Duration;
use sg_marketplace::msg::InstantiateMsg as MarketplaceInstantiateMsg;
use sg_marketplace::ExpiryRange;
use sg_std::NATIVE_DENOM;
use stargaze_fair_burn::msg::InstantiateMsg as FairBurnInstantiateMsg;

use crate::helpers::chain::SigningAccount;
use crate::helpers::constants::NAME_FAIR_BURN;

use super::constants::{
    CREATE_MINTER_FEE, LISTING_FEE, MINT_FEE_BPS, MIN_MINT_FEE, NAME_BASE_FACTORY_NEW,
    NAME_BASE_FACTORY_OLD, NAME_BASE_MINTER_NEW, NAME_BASE_MINTER_OLD, NAME_MARKETPLACE,
    NAME_SG721_BASE_NEW, NAME_SG721_BASE_OLD,
};

pub fn instantiate_fair_burn(
    orc: &mut CosmOrc<CosmosgRPC>,
    user: &SigningAccount,
) -> Result<InstantiateResponse, ProcessError> {
    orc.instantiate(
        NAME_FAIR_BURN,
        &format!("{}_inst", NAME_FAIR_BURN,),
        &FairBurnInstantiateMsg { fee_bps: 5000 },
        &user.key,
        Some(user.account.address.parse().unwrap()),
        vec![],
    )
}

pub fn instantiate_marketplace(
    orc: &mut CosmOrc<CosmosgRPC>,
    user: &SigningAccount,
) -> Result<InstantiateResponse, ProcessError> {
    orc.instantiate(
        NAME_MARKETPLACE,
        &format!("{}_inst", NAME_MARKETPLACE,),
        &MarketplaceInstantiateMsg {
            trading_fee_bps: 200u64,
            ask_expiry: ExpiryRange {
                min: 10u64,
                max: 10_000u64,
            },
            bid_expiry: ExpiryRange {
                min: 10u64,
                max: 10_000u64,
            },
            operators: vec![],
            sale_hook: None,
            max_finders_fee_bps: 1000u64,
            min_price: Uint128::from(10u128),
            stale_bid_duration: Duration::Time(100u64),
            bid_removal_reward_bps: 100u64,
            listing_fee: Uint128::from(LISTING_FEE),
        },
        &user.key,
        Some(user.account.address.parse().unwrap()),
        vec![],
    )
}

pub fn instantiate_base_factory_old(
    orc: &mut CosmOrc<CosmosgRPC>,
    user: &SigningAccount,
) -> Result<InstantiateResponse, ProcessError> {
    orc.instantiate(
        NAME_BASE_FACTORY_OLD,
        &format!("{}_inst", NAME_BASE_FACTORY_OLD,),
        &BaseFactoryOldInstantiateMsg {
            params: BaseMinterOldParams {
                code_id: orc.contract_map.code_id(NAME_BASE_MINTER_OLD).unwrap(),
                allowed_sg721_code_ids: vec![orc
                    .contract_map
                    .code_id(NAME_SG721_BASE_OLD)
                    .unwrap()],
                frozen: false,
                creation_fee: coin(CREATE_MINTER_FEE, NATIVE_DENOM),
                min_mint_price: coin(MIN_MINT_FEE, NATIVE_DENOM),
                mint_fee_bps: MINT_FEE_BPS,
                max_trading_offset_secs: 3600,
                extension: None,
            },
        },
        &user.key,
        Some(user.account.address.parse().unwrap()),
        vec![],
    )
}

pub fn instantiate_base_factory_new(
    orc: &mut CosmOrc<CosmosgRPC>,
    user: &SigningAccount,
) -> Result<InstantiateResponse, ProcessError> {
    orc.instantiate(
        NAME_BASE_FACTORY_NEW,
        &format!("{}_inst", NAME_BASE_FACTORY_NEW,),
        &BaseFactoryNewInstantiateMsg {
            params: BaseMinterNewParams {
                code_id: orc.contract_map.code_id(NAME_BASE_MINTER_NEW).unwrap(),
                allowed_sg721_code_ids: vec![orc
                    .contract_map
                    .code_id(NAME_SG721_BASE_NEW)
                    .unwrap()],
                frozen: false,
                creation_fee: coin(CREATE_MINTER_FEE, NATIVE_DENOM),
                min_mint_price: coin(MIN_MINT_FEE, NATIVE_DENOM),
                mint_fee_bps: MINT_FEE_BPS,
                max_trading_offset_secs: 3600,
                max_royalty_bps: 5000,
                max_royalty_increase_rate_bps: 1000,
                extension: None,
            },
        },
        &user.key,
        Some(user.account.address.parse().unwrap()),
        vec![],
    )
}
