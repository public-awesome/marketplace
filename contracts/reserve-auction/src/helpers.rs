use crate::msg::{ExecuteMsg, QueryMsg};
use crate::state::{auctions, Auction, Config};
use crate::ContractError;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, CustomQuery, Deps, DepsMut, Event, Querier, QuerierWrapper,
    StdResult, Timestamp, WasmMsg, WasmQuery,
};
use sg_marketplace_common::{
    calculate_nft_sale_fees, load_collection_royalties, load_marketplace_params,
    payout_nft_sale_fees, transfer_nft,
};
use sg_std::Response;

#[cw_serde]
pub struct ReserveAuctionContract(pub Addr);

impl ReserveAuctionContract {
    pub fn addr(&self) -> Addr {
        self.0.clone()
    }

    pub fn call<T: Into<ExecuteMsg>>(&self, msg: T) -> StdResult<CosmosMsg> {
        let msg = to_binary(&msg.into())?;
        Ok(WasmMsg::Execute {
            contract_addr: self.addr().into(),
            msg,
            funds: vec![],
        }
        .into())
    }

    /// Get Auction
    pub fn auction<Q, T, CQ>(
        &self,
        querier: &Q,
        collection: String,
        token_id: String,
    ) -> StdResult<Auction>
    where
        Q: Querier,
        T: Into<String>,
        CQ: CustomQuery,
    {
        let msg = QueryMsg::Auction {
            collection,
            token_id,
        };
        let query = WasmQuery::Smart {
            contract_addr: self.addr().into(),
            msg: to_binary(&msg)?,
        }
        .into();
        let res: Auction = QuerierWrapper::<CQ>::new(querier).query(&query)?;
        Ok(res)
    }

    /// Get Config
    pub fn config<Q, T, CQ>(&self, querier: &Q) -> StdResult<Config>
    where
        Q: Querier,
        T: Into<String>,
        CQ: CustomQuery,
    {
        let msg = QueryMsg::Config {};
        let query = WasmQuery::Smart {
            contract_addr: self.addr().into(),
            msg: to_binary(&msg)?,
        }
        .into();
        let res: Config = QuerierWrapper::<CQ>::new(querier).query(&query)?;
        Ok(res)
    }
}

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

pub fn settle_auction(
    deps: DepsMut,
    block_time: Timestamp,
    config: &Config,
    auction: Auction,
    response: Response,
) -> Result<Response, ContractError> {
    let mut response = response;

    // make sure auction has started
    if block_time < auction.start_time {
        return Err(ContractError::AuctionNotStarted {});
    }
    // make sure auction has ended
    if block_time < auction.end_time {
        return Err(ContractError::AuctionNotEnded {});
    }

    // Remove auction from storage
    auctions().remove(
        deps.storage,
        (auction.collection.clone(), auction.token_id.to_string()),
    )?;

    if let Some(_high_bid) = auction.high_bid {
        let marketplace_params = load_marketplace_params(&deps.querier, &config.marketplace)?;
        let royalty_info = load_collection_royalties(&deps.querier, deps.api, &auction.collection)?;

        let tx_fees = calculate_nft_sale_fees(
            _high_bid.coin.amount,
            marketplace_params.trading_fee_percent,
            auction.seller,
            None,
            None,
            royalty_info,
        )?;

        response = payout_nft_sale_fees(response, tx_fees, None)?;

        // transfer token to highest bidder
        let transfer_msg = transfer_nft(
            &auction.collection.clone(),
            &auction.token_id,
            &_high_bid.bidder.clone(),
        );
        response = response.add_submessage(transfer_msg);

        response = response.add_event(
            Event::new("settle-auction")
                .add_attribute("collection", auction.collection.to_string())
                .add_attribute("token_id", auction.token_id.to_string())
                .add_attribute("bidder", _high_bid.bidder.to_string())
                .add_attribute("bid_amount", _high_bid.coin.amount.to_string()),
        );
    } else {
        // no bids, return NFT to seller
        let transfer_msg = transfer_nft(
            &auction.collection.clone(),
            &auction.token_id,
            &&auction.seller.clone(),
        );
        response = response.add_submessage(transfer_msg);

        response = response.add_event(
            Event::new("settle-auction")
                .add_attribute("collection", auction.collection.to_string())
                .add_attribute("token_id", auction.token_id.to_string()),
        );
    };

    Ok(response)
}
