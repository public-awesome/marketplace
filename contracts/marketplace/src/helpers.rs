use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    ensure, has_coins, Addr, BlockInfo, Coin, Decimal, Deps, Event, StdError, Storage, Timestamp,
};
use sg_marketplace_common::{
    nft::{load_collection_royalties, transfer_nft},
    sale::payout_nft_sale_fees,
};
use sg_std::Response;
use std::fmt;
use thiserror::Error;

use crate::{
    hooks::prepare_sale_hook,
    state::{asks, Ask, ExpiringOrder, Offer, SudoParams, TokenId, PRICE_RANGES},
    ContractError,
};

#[derive(Error, Debug, PartialEq)]
pub enum ExpiryRangeError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Invalid expiry: {0}")]
    InvalidExpiry(String),

    #[error("Invalid expiration range: {0}")]
    InvalidExpirationRange(String),
}

#[cw_serde]
pub struct ExpiryRange {
    pub min: u64,
    pub max: u64,
}

impl fmt::Display for ExpiryRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{\"min\":{},\"max\":{}}}", self.min, self.max)
    }
}

impl ExpiryRange {
    pub fn new(min: u64, max: u64) -> Self {
        ExpiryRange { min, max }
    }

    /// Validates if given expires time is within the allowable range
    pub fn is_valid(&self, block: &BlockInfo, expires: Timestamp) -> Result<(), ExpiryRangeError> {
        let now = block.time;
        if !(expires > now.plus_seconds(self.min) && expires <= now.plus_seconds(self.max)) {
            return Err(ExpiryRangeError::InvalidExpiry(
                "expiration time outside of valid range".to_string(),
            ));
        }

        Ok(())
    }

    pub fn validate(&self) -> Result<(), ExpiryRangeError> {
        if self.min > self.max {
            return Err(ExpiryRangeError::InvalidExpirationRange(
                "range min > max".to_string(),
            ));
        }

        Ok(())
    }
}

pub enum MatchResult {
    Match(Ask),
    NotMatch(String),
}

impl fmt::Display for MatchResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MatchResult::Match(_ask) => write!(f, "match"),
            MatchResult::NotMatch(reason) => write!(f, "{}", reason),
        }
    }
}

pub fn price_validate(
    store: &dyn Storage,
    price: &Coin,
    is_ask: bool,
) -> Result<(), ContractError> {
    let price_range = PRICE_RANGES.may_load(store, price.denom.clone())?;
    ensure!(
        price_range.is_some(),
        ContractError::InvalidInput("invalid denom".to_string())
    );
    let price_range = price_range.unwrap();
    ensure!(
        price.amount >= price_range.min,
        ContractError::InvalidInput("price too low".to_string())
    );
    if is_ask {
        ensure!(
            price.amount <= price_range.max,
            ContractError::InvalidInput("price too high".to_string())
        );
    }
    Ok(())
}

pub fn match_offer(
    deps: Deps,
    block: &BlockInfo,
    offer: &Offer,
) -> Result<MatchResult, ContractError> {
    let ask_key = Ask::build_key(&offer.collection, &offer.token_id);
    let ask_option = asks().may_load(deps.storage, ask_key.clone())?;

    if ask_option.is_none() {
        return Ok(MatchResult::NotMatch("ask_not_found".to_string()));
    }

    let ask = ask_option.unwrap();
    if ask.is_expired(block) {
        return Ok(MatchResult::NotMatch("ask_expired".to_string()));
    }
    if let Some(reserve_for) = &ask.reserve_for {
        if reserve_for != &offer.bidder {
            return Ok(MatchResult::NotMatch("ask_reserved".to_string()));
        }
    }
    if !has_coins(&[offer.price.clone()], &ask.price) {
        return Ok(MatchResult::NotMatch("offer_insufficient".to_string()));
    }

    Ok(MatchResult::Match(ask))
}

/// Transfers funds and NFT, updates bid
pub fn finalize_sale(
    deps: Deps,
    collection: &Addr,
    token_id: &String,
    seller: &Addr,
    seller_recipient: &Addr,
    buyer: &Addr,
    buyer_recipient: &Addr,
    price: &Coin,
    sudo_params: &SudoParams,
    finder: Option<&Addr>,
    finders_fee_percent: Option<Decimal>,
    response: Response,
) -> Result<Response, ContractError> {
    let royalty_info = load_collection_royalties(&deps.querier, deps.api, collection)?;

    let (token_payments, mut response) = payout_nft_sale_fees(
        price,
        &seller_recipient,
        &sudo_params.fair_burn,
        None,
        finder,
        sudo_params.trading_fee_percent,
        finders_fee_percent,
        royalty_info,
        response,
    )?;

    // Add royalty event
    let royalty_payment = token_payments.iter().find(|tp| tp.label == "royalty");
    if let Some(royalty_payment) = royalty_payment {
        let event = Event::new("royalty-payout")
            .add_attribute("collection", collection.to_string())
            .add_attribute("amount", royalty_payment.coin.amount.to_string())
            .add_attribute("denom", royalty_payment.coin.denom.to_string())
            .add_attribute("recipient", royalty_payment.recipient.to_string());
        response = response.add_event(event);
    }

    // Transfer NFT to buyer
    response = response.add_submessage(transfer_nft(collection, token_id, buyer_recipient));

    // Prepare hook
    response = response.add_submessages(prepare_sale_hook(
        deps, collection, token_id, price, seller, buyer,
    )?);

    let event = Event::new("finalize-sale")
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id.to_string())
        .add_attribute("seller", seller.to_string())
        .add_attribute("buyer", buyer.to_string())
        .add_attribute("price", price.to_string());
    response = response.add_event(event);

    Ok(response)
}

pub fn build_collection_token_index_str(collection: &str, token_id: &TokenId) -> String {
    let string_list = vec![collection.to_string(), token_id.clone()];
    let collection_token = string_list.join("/");
    collection_token
}
