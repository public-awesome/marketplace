use crate::msg::{ExecuteMsg, QueryMsg};
use crate::state::{auctions, Auction, Config};
use crate::ContractError;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    coin, to_binary, Addr, CosmosMsg, CustomQuery, Deps, DepsMut, Event, Querier, QuerierWrapper,
    StdResult, Timestamp, Uint128, WasmMsg, WasmQuery,
};
use sg1::fair_burn;
use sg_marketplace::msg::{ParamsResponse, QueryMsg as MarketplaceQueryMsg};
use sg_marketplace_common::{bank_send, royalty_payout, transfer_nft};
use sg_std::{Response, SubMsg, NATIVE_DENOM};

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

pub fn load_marketplace_params(
    deps: Deps,
    marketplace_addr: &Addr,
) -> Result<ParamsResponse, ContractError> {
    let marketplace_params: ParamsResponse = deps
        .querier
        .query_wasm_smart(marketplace_addr, &MarketplaceQueryMsg::Params {})?;
    Ok(marketplace_params)
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

    let mut event = Event::new("settle-auction")
        .add_attribute("collection", auction.collection.to_string())
        .add_attribute("token_id", auction.token_id.to_string());

    let sub_msgs: Vec<SubMsg> = if let Some(_high_bid) = auction.high_bid {
        // handle protocol fee
        let marketplace_params = load_marketplace_params(deps.as_ref(), &config.marketplace)?;
        let protocol_fee = _high_bid.coin.amount * marketplace_params.params.trading_fee_percent
            / Uint128::from(100u128);
        fair_burn(protocol_fee.u128(), None, &mut response);

        // handle royalties
        let royalty_payment = royalty_payout(
            deps.as_ref(),
            &auction.collection,
            _high_bid.coin.amount,
            &mut response,
        )?;

        // send funds to seller
        let remaining_funds = _high_bid.coin.amount - protocol_fee - royalty_payment;
        let bank_msg = bank_send(
            coin(remaining_funds.u128(), NATIVE_DENOM),
            auction.seller_funds_recipient.unwrap_or(auction.seller),
        )?;

        // transfer token to highest bidder
        let transfer_msg = transfer_nft(
            auction.collection.clone(),
            &auction.token_id,
            _high_bid.bidder.clone(),
        )?;

        event = event
            .add_attribute("bidder", _high_bid.bidder.to_string())
            .add_attribute("bid_amount", _high_bid.coin.amount.to_string());

        vec![bank_msg, transfer_msg]
    } else {
        // no bids, return NFT to seller
        let transfer_msg = transfer_nft(
            auction.collection.clone(),
            &auction.token_id,
            auction.seller,
        )?;

        event = event
            .add_attribute("bidder", "None".to_string())
            .add_attribute("bid_amount", "None".to_string());

        vec![transfer_msg]
    };

    Ok(response.add_event(event).add_submessages(sub_msgs))
}
