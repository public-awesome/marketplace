use crate::{
    orders::{Ask, MatchingBid},
    state::{Config, TokenId, COLLECTION_DENOMS},
    ContractError,
};

use blake2::{Blake2s256, Digest};
use cosmwasm_std::{
    ensure, ensure_eq, Addr, Coin, Decimal, DepsMut, Env, Event, MessageInfo, QuerierWrapper,
    Response, Storage, Uint128,
};
use sg_marketplace_common::{
    nft::transfer_nft, royalties::fetch_or_set_royalties, sale::NftSaleProcessor,
    MarketplaceStdError,
};
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

pub fn only_valid_price(
    storage: &dyn Storage,
    config: &Config<Addr>,
    collection: &Addr,
    price: &Coin,
) -> Result<(), ContractError> {
    ensure!(
        price.amount > Uint128::zero(),
        ContractError::InvalidInput("order price must be greater than 0".to_string())
    );

    let query_result = COLLECTION_DENOMS.may_load(storage, collection.clone())?;
    let collection_denom = query_result.unwrap_or(config.default_denom.clone());
    ensure_eq!(
        collection_denom,
        price.denom,
        ContractError::InvalidInput("invalid denom".to_string())
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
) -> Result<ProtocolFees, ContractError> {
    let mut protocol_fees = ProtocolFees {
        protocol_fee: Decimal::bps(config.protocol_fee_bps),
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

#[allow(clippy::too_many_arguments)]
pub fn finalize_sale(
    deps: DepsMut,
    env: &Env,
    ask: &Ask,
    config: &Config<Addr>,
    matching_bid: &MatchingBid,
    ask_before_bid: bool,
    action: &str,
    response: Response,
) -> Result<Response, ContractError> {
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

    let protocol_fees = divide_protocol_fees(config, maker.is_some(), taker.is_some())?;

    if protocol_fees.protocol_fee > Decimal::zero() {
        nft_sale_processor.add_fee(
            "protocol".to_string(),
            protocol_fees.protocol_fee,
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
        .add_attribute("action", action.to_string());

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
            protocol_fee_bps: 200,
            max_royalty_fee_bps: 500,
            maker_reward_bps: 4000,
            taker_reward_bps: 1000,
            default_denom: "ustars".to_string(),
        };

        let result = divide_protocol_fees(&config, true, true).unwrap();

        assert_eq!(result.protocol_fee, Decimal::from_str("0.01").unwrap());
        assert_eq!(result.maker_reward, Decimal::from_str("0.008").unwrap());
        assert_eq!(result.taker_reward, Decimal::from_str("0.002").unwrap());
    }
}
