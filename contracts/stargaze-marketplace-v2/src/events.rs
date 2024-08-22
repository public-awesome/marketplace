use crate::{
    orders::{Ask, Bid, CollectionBid},
    state::Config,
};

use cosmwasm_std::{attr, Addr, Event};
use std::vec;

pub struct ConfigEvent<'a> {
    pub ty: &'a str,
    pub config: &'a Config<Addr>,
}

impl<'a> From<ConfigEvent<'a>> for Event {
    fn from(ce: ConfigEvent) -> Self {
        Event::new(ce.ty.to_string()).add_attributes(vec![
            attr("fee_manager", ce.config.fee_manager.to_string()),
            attr("royalty_registry", ce.config.royalty_registry.to_string()),
            attr("protocol_fee_bps", ce.config.protocol_fee_bps.to_string()),
            attr(
                "max_royalty_fee_bps",
                ce.config.max_royalty_fee_bps.to_string(),
            ),
            attr("maker_reward_bps", ce.config.maker_reward_bps.to_string()),
            attr("taker_reward_bps", ce.config.taker_reward_bps.to_string()),
            attr("default_denom", ce.config.default_denom.to_string()),
        ])
    }
}

pub struct CollectionDenomEvent<'a> {
    pub ty: &'a str,
    pub collection: &'a str,
    pub denom: &'a str,
}

impl<'a> From<CollectionDenomEvent<'a>> for Event {
    fn from(cde: CollectionDenomEvent) -> Self {
        Event::new(cde.ty.to_string()).add_attributes(vec![
            attr("collection", cde.collection.to_string()),
            attr("denom", cde.denom.to_string()),
        ])
    }
}

pub struct AskEvent<'a> {
    pub ty: &'a str,
    pub ask: &'a Ask,
    pub attr_keys: Vec<&'a str>,
}

impl<'a> From<AskEvent<'a>> for Event {
    fn from(ae: AskEvent) -> Self {
        Event::new(ae.ty.to_string()).add_attributes(ae.ask.get_event_attrs(ae.attr_keys))
    }
}

pub struct BidEvent<'a> {
    pub ty: &'a str,
    pub bid: &'a Bid,
    pub attr_keys: Vec<&'a str>,
}

impl<'a> From<BidEvent<'a>> for Event {
    fn from(oe: BidEvent) -> Self {
        Event::new(oe.ty.to_string()).add_attributes(oe.bid.get_event_attrs(oe.attr_keys))
    }
}

pub struct CollectionBidEvent<'a> {
    pub ty: &'a str,
    pub collection_bid: &'a CollectionBid,
    pub attr_keys: Vec<&'a str>,
}

impl<'a> From<CollectionBidEvent<'a>> for Event {
    fn from(coe: CollectionBidEvent) -> Self {
        Event::new(coe.ty.to_string())
            .add_attributes(coe.collection_bid.get_event_attrs(coe.attr_keys))
    }
}
