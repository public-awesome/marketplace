use crate::{
    constants::MAX_BASIS_POINTS,
    msg::OrderOptions,
    orders::MatchingOffer,
    state::{asks, Ask, Config, ExpirationInfo, OrderInfo, TokenId, PRICE_RANGES},
    ContractError,
};

use cosmwasm_std::{
    ensure, ensure_eq, has_coins, Addr, Api, BlockInfo, Coin, Decimal, Deps, Env, Event,
    MessageInfo, Storage,
};
use cw_utils::{maybe_addr, NativeBalance};
use sg_marketplace_common::{
    coin::transfer_coins,
    nft::{owner_of, transfer_nft},
    sale::{FeeType, NftSaleProcessor},
    MarketplaceStdError,
};
use sg_std::Response;
use stargaze_royalty_registry::fetch_or_set_royalties;
use std::{cmp::min, ops::Sub};

#[allow(clippy::field_reassign_with_default)]
pub fn validate_order_options(
    api: &dyn Api,
    info: &MessageInfo,
    block_info: &BlockInfo,
    config: &Config<Addr>,
    order_options_str: OrderOptions<String>,
) -> Result<OrderOptions<Addr>, ContractError> {
    let order_options_addr = OrderOptions {
        asset_recipient: maybe_addr(api, order_options_str.asset_recipient)?,
        finder: maybe_addr(api, order_options_str.finder)?,
        finders_fee_bps: order_options_str.finders_fee_bps,
        expiration_info: order_options_str.expiration_info,
    };

    if let Some(finder) = &order_options_addr.finder {
        ensure!(
            finder != info.sender,
            ContractError::InvalidInput("finder should not be sender".to_string())
        );
    }

    if let Some(finders_fee_bps) = order_options_str.finders_fee_bps {
        ensure!(
            finders_fee_bps <= MAX_BASIS_POINTS,
            ContractError::InvalidInput("finders_fee_bps is above 100%".to_string())
        );
    }

    if let Some(expiration_info) = &order_options_addr.expiration_info {
        validate_expiration_info(block_info, config, expiration_info)?;
    }

    Ok(order_options_addr)
}

pub fn validate_expiration_info(
    block_info: &BlockInfo,
    config: &Config<Addr>,
    expiration_info: &ExpirationInfo,
) -> Result<(), ContractError> {
    ensure!(
        expiration_info.expiration >= block_info.time.plus_seconds(config.min_expiration_seconds),
        ContractError::InvalidInput("expiration is below minimum".to_string())
    );

    ensure!(
        has_coins(
            &[expiration_info.removal_reward.clone()],
            &config.min_removal_reward
        ),
        ContractError::InvalidInput(format!(
            "removal reward must be at least {}",
            &config.min_removal_reward
        ))
    );

    Ok(())
}

pub fn validate_price(store: &dyn Storage, price: &Coin) -> Result<(), ContractError> {
    let price_range = PRICE_RANGES.may_load(store, price.denom.clone())?;

    ensure!(
        price_range.is_some(),
        ContractError::InvalidInput("invalid denom".to_string())
    );

    let price_range = price_range.unwrap();
    ensure!(
        price.amount >= price_range.min,
        ContractError::InvalidInput(format!(
            "price too low {} < {}",
            price.amount, price_range.min
        ))
    );
    ensure!(
        price.amount <= price_range.max,
        ContractError::InvalidInput(format!(
            "price too high {} > {}",
            price.amount, price_range.max
        ))
    );
    Ok(())
}

/// `reconcile_funds` reconciles the funds due to the contract with the funds sent by the user.
/// If the user sent more funds than due, the excess is returned to the user.
pub fn reconcile_funds(
    info: &MessageInfo,
    mut funds_due: NativeBalance,
    mut response: Response,
) -> Result<Response, ContractError> {
    funds_due.normalize();

    let mut funds_user = NativeBalance(info.funds.clone());
    funds_user.normalize();

    // Deduct funds due from user funds
    for funds in funds_due.into_vec() {
        funds_user = funds_user
            .sub(funds.clone())
            .map_err(|_| ContractError::InsufficientFunds { expected: funds })?;
    }

    // Transfer remaining funds back to user
    if !funds_user.is_empty() {
        response = transfer_coins(funds_user.into_vec(), &info.sender, response);
    }

    Ok(response)
}

pub fn only_order_creator(info: &MessageInfo, order_info: &OrderInfo) -> Result<(), ContractError> {
    ensure_eq!(
        info.sender,
        order_info.creator,
        MarketplaceStdError::Unauthorized(
            "only the creator of order can perform this action".to_string()
        )
    );
    Ok(())
}

pub fn only_owner_or_seller(
    deps: Deps,
    env: &Env,
    info: &MessageInfo,
    collection: &Addr,
    token_id: &TokenId,
) -> Result<(), MarketplaceStdError> {
    let owner_of_response = owner_of(&deps.querier, collection, token_id)?;

    if owner_of_response.owner == info.sender {
        return Ok(());
    }

    if owner_of_response.owner == env.contract.address {
        let ask_key = Ask::build_key(collection, token_id);
        let ask_option = asks().may_load(deps.storage, ask_key)?;

        if let Some(ask) = ask_option {
            if ask.order_info.creator == info.sender {
                return Ok(());
            }
        }
    }

    Err(MarketplaceStdError::Unauthorized(
        "sender is not owner or seller".to_string(),
    ))
}

/// Processes a sale, transferring funds to correct destinations
pub fn finalize_sale(
    deps: Deps,
    env: &Env,
    ask: &Ask,
    matching_offer: &MatchingOffer,
    config: &Config<Addr>,
    finder: Option<&Addr>,
    ask_before_offer: bool,
    response: Response,
) -> Result<Response, ContractError> {
    let (offer_price, offer_order_info) = match &matching_offer {
        MatchingOffer::Offer(offer) => (offer.order_info.price.clone(), offer.order_info.clone()),
        MatchingOffer::CollectionOffer(collection_offer) => (
            collection_offer.order_info.price.clone(),
            collection_offer.order_info.clone(),
        ),
    };

    let (sale_price, finders_fee_bps) = if ask_before_offer {
        (ask.order_info.price.clone(), ask.order_info.finders_fee_bps)
    } else {
        (offer_price, offer_order_info.finders_fee_bps)
    };

    let (royalty_entry_option, mut response) = fetch_or_set_royalties(
        deps,
        &config.royalty_registry,
        &ask.collection,
        Some(&env.contract.address),
        response,
    )?;

    let mut nft_sale_processor =
        NftSaleProcessor::new(sale_price.clone(), ask.order_info.asset_recipient());

    nft_sale_processor.add_fee(
        FeeType::FairBurn,
        Decimal::bps(config.trading_fee_bps),
        config.fair_burn.clone(),
    );

    if let Some(royalty_entry) = royalty_entry_option {
        nft_sale_processor.add_fee(
            FeeType::Royalty,
            min(
                royalty_entry.share,
                Decimal::bps(config.max_royalty_fee_bps),
            ),
            royalty_entry.recipient,
        );
    }

    if let (Some(finder), Some(finders_fee_bps)) = (finder, finders_fee_bps) {
        nft_sale_processor.add_fee(
            FeeType::Finder,
            min(
                Decimal::bps(finders_fee_bps),
                Decimal::bps(config.max_finders_fee_bps),
            ),
            finder.clone(),
        );
    }

    nft_sale_processor.build_payments()?;
    response = nft_sale_processor.payout(response);

    // Transfer NFT to buyer
    response = transfer_nft(
        &ask.collection,
        &ask.token_id,
        &offer_order_info.asset_recipient(),
        response,
    );

    response = response.add_event(
        Event::new("finalize-sale")
            .add_attribute("collection", ask.collection.to_string())
            .add_attribute("token_id", ask.token_id.to_string())
            .add_attribute("seller", ask.order_info.creator.to_string())
            .add_attribute("buyer", offer_order_info.creator.to_string())
            .add_attribute("price", sale_price.to_string()),
    );

    Ok(response)
}

pub fn build_collection_token_index_str(collection: &str, token_id: &TokenId) -> String {
    let string_list = vec![collection.to_string(), token_id.clone()];
    string_list.join("/")
}
