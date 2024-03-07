use cosmwasm_std::{coin, ensure, Addr, Coin, Deps, DepsMut, Event, Storage, Timestamp};
use sg_marketplace_common::{
    nft::{load_collection_royalties, transfer_nft},
    sale::payout_nft_sale_fees,
};
use sg_std::Response;

use crate::state::{auctions, Auction, Config, HaltManager, MIN_RESERVE_PRICES};
use crate::ContractError;

pub fn only_no_auction(deps: Deps, collection: &Addr, token_id: &str) -> Result<(), ContractError> {
    if auctions()
        .may_load(deps.storage, (collection.clone(), token_id.to_string()))?
        .is_some()
    {
        return Err(ContractError::AuctionAlreadyExists {
            collection: String::from(collection),
            token_id: token_id.to_string(),
        });
    }
    Ok(())
}

pub fn validate_reserve_price(
    storage: &dyn Storage,
    check_reserve_price: &Coin,
) -> Result<(), ContractError> {
    let minimum_amount = MIN_RESERVE_PRICES.may_load(storage, check_reserve_price.denom.clone())?;

    ensure!(
        minimum_amount.is_some(),
        ContractError::InvalidInput("invalid reserve price denom".to_string(),)
    );

    ensure!(
        check_reserve_price.amount >= minimum_amount.unwrap(),
        ContractError::InvalidReservePrice {
            min: coin(
                minimum_amount.unwrap().u128(),
                check_reserve_price.denom.clone(),
            ),
        }
    );

    Ok(())
}

pub fn settle_auction(
    deps: DepsMut,
    block_time: Timestamp,
    mut auction: Auction,
    config: &Config,
    halt_manager: &HaltManager,
    mut response: Response,
) -> Result<Response, ContractError> {
    // Ensure auction has ended
    ensure!(
        auction.end_time.is_some() && auction.end_time.unwrap() <= block_time,
        ContractError::AuctionNotEnded {}
    );

    // If auction is set to end within a halt window, then postpone it instead
    let auction_end_time = auction.end_time.unwrap();
    if halt_manager.is_within_halt_window(auction_end_time.seconds()) {
        let new_auction_end_time = block_time.plus_seconds(config.halt_postpone_duration);
        auction.end_time = Some(new_auction_end_time);
        auctions().save(
            deps.storage,
            (auction.collection.clone(), auction.token_id.clone()),
            &auction,
        )?;
        response = response.add_event(
            Event::new("postpone-auction")
                .add_attribute("collection", auction.collection.to_string())
                .add_attribute("token_id", auction.token_id)
                .add_attribute("auction_end_time", new_auction_end_time.to_string()),
        );
        return Ok(response);
    }

    // Remove auction from storage
    auctions().remove(
        deps.storage,
        (auction.collection.clone(), auction.token_id.clone()),
    )?;

    // High bid must exist if end time exists
    let high_bid = auction.high_bid.as_ref().unwrap();

    let royalty_info = load_collection_royalties(&deps.querier, deps.api, &auction.collection)?;

    (_, response) = payout_nft_sale_fees(
        &high_bid.coin,
        &auction.funds_recipient(),
        &config.fair_burn,
        None,
        None,
        config.trading_fee_percent,
        None,
        royalty_info,
        response,
    )?;

    // Transfer NFT to highest bidder
    response = response.add_submessage(transfer_nft(
        &auction.collection,
        &auction.token_id,
        &high_bid.bidder,
    ));

    response = response.add_event(
        Event::new("settle-auction")
            .add_attribute("collection", auction.collection.to_string())
            .add_attribute("token_id", auction.token_id)
            .add_attribute("seller", auction.seller)
            .add_attribute("bidder", high_bid.bidder.to_string())
            .add_attribute("bid_amount", high_bid.coin.amount.to_string())
            .add_attribute("bid_denom", high_bid.coin.denom.to_string()),
    );

    Ok(response)
}
