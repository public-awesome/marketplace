use crate::{
    constants::BLACKLIST_MANAGER,
    orders::{Ask, MatchingBid},
    state::{Config, TokenId, BLACKLIST, COLLECTION_DENOMS, IS_PAUSED},
    ContractError,
};

use blake2::{Blake2s256, Digest};
use cosmwasm_std::{
    ensure, ensure_eq, to_json_binary, Addr, Coin, Decimal, Deps, DepsMut, Env, Event, MessageInfo,
    QuerierWrapper, Response, StdError, StdResult, Storage, Uint128, WasmMsg,
};
use sg_marketplace_common::constants::NATIVE_DENOM;
use sg_marketplace_common::{
    nft::transfer_nft, royalties::fetch_or_set_royalties, sale::NftSaleProcessor,
    MarketplaceStdError,
};
use stargaze_vip_minter::msg::ExecuteMsg as LoyaltyProgramExecuteMsg;
use stargaze_vip_minter::msg::QueryMsg as LoyaltyProgramQueryMsg;
use stargaze_vip_minter::msg::TierResponse;
use std::{cmp::min, ops::Sub};
pub fn build_collection_token_index_str(collection: &str, token_id: &TokenId) -> String {
    let string_list = [collection.to_string(), token_id.clone()];
    string_list.join("/")
}

pub fn generate_id(components: Vec<&[u8]>) -> String {
    let mut hasher = Blake2s256::new();
    for component in components {
        hasher.update(component);
    }
    format!("{:x}", hasher.finalize())
}

pub fn only_contract_admin(
    querier: &QuerierWrapper,
    env: &Env,
    info: &MessageInfo,
) -> Result<(), ContractError> {
    let contract_info_resp = querier.query_wasm_contract_info(&env.contract.address)?;

    if contract_info_resp.admin.is_none() {
        Err(MarketplaceStdError::Unauthorized(
            "contract admin unset".to_string(),
        ))?;
    }

    ensure_eq!(
        info.sender,
        contract_info_resp.admin.unwrap(),
        MarketplaceStdError::Unauthorized(
            "only the admin of contract can perform this action".to_string(),
        )
    );

    Ok(())
}

// only_valid_price checks non zero amounts and enforces being in the collection denom or optional denom
pub fn only_valid_price(
    storage: &dyn Storage,
    config: &Config<Addr>,
    collection: &Addr,
    price: &Coin,
    denom: Option<&str>,
) -> Result<(), ContractError> {
    ensure!(
        price.amount > Uint128::zero(),
        ContractError::InvalidInput("order price must be greater than 0".to_string())
    );

    if let Some(denom) = denom {
        ensure_eq!(
            denom,
            price.denom,
            ContractError::InvalidInput("invalid denom".to_string())
        );
    } else {
        let query_result = COLLECTION_DENOMS.may_load(storage, collection.clone())?;
        let collection_denom = query_result.unwrap_or(config.default_denom.clone());
        ensure_eq!(
            collection_denom,
            price.denom,
            ContractError::InvalidInput("invalid denom".to_string())
        );
    }

    Ok(())
}

/// Validates that a token is not blacklisted
pub fn only_not_blacklisted(
    storage: &dyn Storage,
    collection: &Addr,
    token_id: &TokenId,
) -> Result<(), ContractError> {
    let key = build_collection_token_index_str(collection.as_ref(), token_id);
    if BLACKLIST.has(storage, key) {
        return Err(ContractError::TokenBlacklisted(
            collection.to_string(),
            token_id.clone(),
        ));
    }
    Ok(())
}

/// Ensures the contract is not paused
pub fn ensure_not_paused(storage: &dyn Storage) -> Result<(), ContractError> {
    let paused = IS_PAUSED.may_load(storage)?.unwrap_or(false);
    if paused {
        return Err(ContractError::ContractPaused);
    }
    Ok(())
}

/// Validates that the sender is the blacklist manager
pub fn only_blacklist_manager(info: &MessageInfo) -> Result<(), ContractError> {
    ensure_eq!(
        info.sender.as_str(),
        BLACKLIST_MANAGER,
        MarketplaceStdError::Unauthorized(
            "only the blacklist manager can perform this action".to_string()
        )
    );
    Ok(())
}

#[derive(Debug)]
pub struct ProtocolFees {
    pub protocol_fee: Decimal,
    pub maker_reward: Decimal,
    pub taker_reward: Decimal,
}

pub fn divide_protocol_fees(
    config: &Config<Addr>,
    maker_exists: bool,
    taker_exists: bool,
    is_native: bool,
) -> Result<ProtocolFees, ContractError> {
    let fee_bps = if is_native {
        config.protocol_fee_bps
    } else {
        config.non_native_protocol_fee_bps
    };

    let mut protocol_fees = ProtocolFees {
        protocol_fee: Decimal::bps(fee_bps),
        maker_reward: Decimal::zero(),
        taker_reward: Decimal::zero(),
    };

    if protocol_fees.protocol_fee == Decimal::zero() {
        return Ok(protocol_fees);
    }

    if maker_exists && config.maker_reward_bps > 0 {
        protocol_fees.maker_reward = Decimal::bps(config.protocol_fee_bps)
            .checked_mul(Decimal::bps(config.maker_reward_bps))?;
        protocol_fees.protocol_fee = protocol_fees.protocol_fee.sub(protocol_fees.maker_reward);
    }

    if taker_exists && config.taker_reward_bps > 0 {
        protocol_fees.taker_reward = Decimal::bps(config.protocol_fee_bps)
            .checked_mul(Decimal::bps(config.taker_reward_bps))?;
        protocol_fees.protocol_fee = protocol_fees.protocol_fee.sub(protocol_fees.taker_reward);
    }

    Ok(protocol_fees)
}

pub fn calculate_loyalty_bonus_bps(
    deps: Deps,
    env: &Env,
    participant: String,
    config: &Config<Addr>,
    mut response: Response,
) -> StdResult<(Decimal, Response)> {
    let loyalty_registry = match &config.loyalty_registry {
        Some(addr) => addr,
        None => return Ok((Decimal::zero(), response)),
    };

    let loyalty_bonuses_bps = match &config.loyalty_bonuses_bps {
        Some(bonuses) => bonuses,
        None => return Ok((Decimal::zero(), response)),
    };

    let participant_info: TierResponse = deps.querier.query_wasm_smart(
        loyalty_registry,
        &LoyaltyProgramQueryMsg::Tier {
            address: participant.clone(),
        },
    )?;

    if let (Some(threshold), Some(last_update)) = (
        config.loyalty_update_threshold_secs,
        participant_info.last_update_time,
    ) {
        // participant_info.tier can't be None while last_update is Some, checking just for safety
        if participant_info.tier.is_some()
            && env
                .block
                .time
                .seconds()
                .saturating_sub(last_update.seconds())
                > threshold
        {
            response = response.add_message(WasmMsg::Execute {
                contract_addr: loyalty_registry.to_string(),
                msg: to_json_binary(&LoyaltyProgramExecuteMsg::Update {
                    address: participant,
                })?,
                funds: vec![],
            });
        }
    }

    let tier = match participant_info.tier {
        Some(tier) => {
            let max_tier = loyalty_bonuses_bps.len() as u64;
            min(tier, max_tier)
        }
        None => return Ok((Decimal::zero(), response)),
    };

    let bonus_bps = loyalty_bonuses_bps.get(tier as usize).copied().unwrap_or(0);

    let bonus_decimal = Decimal::bps(bonus_bps);
    Ok((bonus_decimal, response))
}

#[allow(clippy::too_many_arguments)]
pub fn finalize_sale(
    deps: DepsMut,
    env: &Env,
    ask: &Ask,
    config: &Config<Addr>,
    matching_bid: &MatchingBid,
    ask_before_bid: bool,
    action: &str,
    mut response: Response,
) -> Result<Response, ContractError> {
    // Check if token is blacklisted before proceeding with any sale
    only_not_blacklisted(deps.storage, &ask.collection, &ask.token_id)?;

    let (nft_recipient, bid_details) = match &matching_bid {
        MatchingBid::Bid(bid) => (bid.asset_recipient(), &bid.details),
        MatchingBid::CollectionBid(collection_bid) => {
            (collection_bid.asset_recipient(), &collection_bid.details)
        }
    };

    let (sale_price, maker, taker) = if ask_before_bid {
        (&ask.details.price, &ask.details.finder, &bid_details.finder)
    } else {
        (&bid_details.price, &bid_details.finder, &ask.details.finder)
    };

    let seller_recipient = ask.asset_recipient();
    let mut nft_sale_processor =
        NftSaleProcessor::new(sale_price.clone(), seller_recipient.clone());

    let is_native = sale_price.denom == NATIVE_DENOM;
    let protocol_fees = divide_protocol_fees(config, maker.is_some(), taker.is_some(), is_native)?;

    let (loyalty_bonus_bps, updated_response) = calculate_loyalty_bonus_bps(
        deps.as_ref(),
        env,
        seller_recipient.to_string(),
        config,
        response,
    )?;
    response = updated_response;

    // Loyalty bonus is deducted from the protocol fee and is sent to the seller
    let mut protocol_fee = protocol_fees.protocol_fee;
    if loyalty_bonus_bps > Decimal::zero() {
        let loyalty_fee = protocol_fee
            .checked_mul(loyalty_bonus_bps)
            .map_err(|_| StdError::generic_err("Loyalty bonus calculation failed".to_string()))?;
        protocol_fee = protocol_fee
            .checked_sub(loyalty_fee)
            .map_err(|_| StdError::generic_err("Loyalty bonus exceeds protocol fee"))?;
        nft_sale_processor.add_fee(
            "loyalty_bonus".to_string(),
            loyalty_fee,
            seller_recipient.clone(),
        );
    }

    if protocol_fees.protocol_fee > Decimal::zero() {
        nft_sale_processor.add_fee(
            "protocol".to_string(),
            protocol_fee,
            config.fee_manager.clone(),
        );
    }
    if protocol_fees.maker_reward > Decimal::zero() {
        nft_sale_processor.add_fee(
            "maker".to_string(),
            protocol_fees.maker_reward,
            maker.clone().unwrap().clone(),
        );
    }
    if protocol_fees.taker_reward > Decimal::zero() {
        nft_sale_processor.add_fee(
            "taker".to_string(),
            protocol_fees.taker_reward,
            taker.clone().unwrap().clone(),
        );
    }

    let (royalty_entry_option, mut response) = fetch_or_set_royalties(
        deps.as_ref(),
        &config.royalty_registry,
        &ask.collection,
        Some(&env.contract.address),
        response,
    )?;

    if let Some(royalty_entry) = royalty_entry_option {
        nft_sale_processor.add_fee(
            "royalty".to_string(),
            min(
                royalty_entry.share,
                Decimal::bps(config.max_royalty_fee_bps),
            ),
            royalty_entry.recipient,
        );
    }

    nft_sale_processor.build_payments()?;
    response = nft_sale_processor.payout(response);

    // Transfer NFT to buyer
    response = transfer_nft(&ask.collection, &ask.token_id, &nft_recipient, response);

    // Remove orders
    ask.remove(deps.storage)?;
    match &matching_bid {
        MatchingBid::Bid(bid) => {
            bid.remove(deps.storage)?;
        }
        MatchingBid::CollectionBid(collection_bid) => {
            collection_bid.remove(deps.storage)?;
        }
    }

    let mut sale_event = Event::new("finalize-sale")
        .add_attribute("collection", ask.collection.to_string())
        .add_attribute("token_id", ask.token_id.to_string())
        .add_attribute("denom", sale_price.denom.to_string())
        .add_attribute("price", sale_price.amount.to_string())
        .add_attribute("seller_recipient", seller_recipient.to_string())
        .add_attribute("nft_recipient", nft_recipient.to_string())
        .add_attribute("ask", ask.id.to_string())
        .add_attribute("marketplace_action", action.to_string());

    match &matching_bid {
        MatchingBid::Bid(bid) => {
            sale_event = sale_event.add_attribute("bid", bid.id.to_string());
        }
        MatchingBid::CollectionBid(collection_bid) => {
            sale_event = sale_event.add_attribute("collection_bid", collection_bid.id.to_string());
        }
    }

    for payment in nft_sale_processor.payments.iter() {
        sale_event = sale_event.add_attribute(&payment.label, payment.funds.amount.to_string());
    }

    response = response.add_event(sale_event);

    Ok(response)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn try_maker_and_taker_fees() {
        let config = Config {
            fee_manager: Addr::unchecked("fee_manager"),
            royalty_registry: Addr::unchecked("royalty_registry"),
            loyalty_registry: None,
            loyalty_bonuses_bps: None,
            loyalty_update_threshold_secs: None,
            protocol_fee_bps: 200,
            non_native_protocol_fee_bps: 400,
            max_royalty_fee_bps: 500,
            maker_reward_bps: 4000,
            taker_reward_bps: 1000,
            default_denom: "ustars".to_string(),
        };

        let result = divide_protocol_fees(&config, true, true, true).unwrap();

        assert_eq!(result.protocol_fee, Decimal::from_str("0.01").unwrap());
        assert_eq!(result.maker_reward, Decimal::from_str("0.008").unwrap());
        assert_eq!(result.taker_reward, Decimal::from_str("0.002").unwrap());

        let result = divide_protocol_fees(&config, true, true, false).unwrap();

        assert_eq!(result.protocol_fee, Decimal::from_str("0.03").unwrap());
        assert_eq!(result.maker_reward, Decimal::from_str("0.008").unwrap());
        assert_eq!(result.taker_reward, Decimal::from_str("0.002").unwrap());
    }
}
