use crate::state::{auctions, Auction, Config};
use crate::ContractError;
use cosmwasm_std::{ensure, Addr, Deps, DepsMut, Event, QuerierWrapper, StdResult, Timestamp};
use sg_marketplace::msg::{ParamsResponse, QueryMsg as MarketplaceQueryMsg};
use sg_marketplace::state::SudoParams;
use sg_marketplace_common::{
    calculate_nft_sale_fees, load_collection_royalties, payout_nft_sale_fees, transfer_nft,
};
use sg_std::Response;

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

pub fn load_marketplace_params(
    querier: &QuerierWrapper,
    marketplace: &Addr,
) -> StdResult<SudoParams> {
    let marketplace_params: ParamsResponse =
        querier.query_wasm_smart(marketplace, &MarketplaceQueryMsg::Params {})?;
    Ok(marketplace_params.params)
}

pub fn settle_auction(
    deps: DepsMut,
    block_time: Timestamp,
    config: &Config,
    auction: Auction,
    response: Response,
) -> Result<Response, ContractError> {
    let mut response = response;

    // Ensure auction has ended
    ensure!(
        auction.end_time.is_some() && auction.end_time.unwrap() <= block_time,
        ContractError::AuctionNotEnded {}
    );

    // Remove auction from storage
    auctions().remove(
        deps.storage,
        (auction.collection.clone(), auction.token_id.clone()),
    )?;

    // High bid must exist if end time exists
    let high_bid = auction.high_bid.unwrap();

    let marketplace_params = load_marketplace_params(&deps.querier, &config.marketplace)?;
    let royalty_info = load_collection_royalties(&deps.querier, deps.api, &auction.collection)?;

    let tx_fees = calculate_nft_sale_fees(
        high_bid.coin.amount,
        marketplace_params.trading_fee_percent,
        auction.seller,
        None,
        None,
        royalty_info,
    )?;

    response = payout_nft_sale_fees(response, tx_fees, None)?;

    // Transfer NFT to highest bidder
    let transfer_msg = transfer_nft(&auction.collection, &auction.token_id, &high_bid.bidder);
    response = response.add_submessage(transfer_msg);

    response = response.add_event(
        Event::new("settle-auction")
            .add_attribute("collection", auction.collection.to_string())
            .add_attribute("token_id", auction.token_id)
            .add_attribute("bidder", high_bid.bidder.to_string())
            .add_attribute("bid_amount", high_bid.coin.amount.to_string()),
    );

    Ok(response)
}
