use crate::{
    orders::{Ask, CollectionOffer, Offer},
    state::{AllowDenoms, Config},
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
        ])
    }
}

pub struct AllowDenomsEvent<'a> {
    pub ty: &'a str,
    pub allow_denoms: &'a AllowDenoms,
}

impl<'a> From<AllowDenomsEvent<'a>> for Event {
    fn from(ade: AllowDenomsEvent) -> Self {
        let mut event = Event::new(ade.ty.to_string());

        let enum_type = match &ade.allow_denoms {
            AllowDenoms::Includes(_) => "includes",
            AllowDenoms::Excludes(_) => "excludes",
        };
        event = event.add_attribute("type", enum_type);

        match &ade.allow_denoms {
            AllowDenoms::Includes(denoms) => {
                for denom in denoms {
                    event = event.add_attribute("denom", denom);
                }
            }
            AllowDenoms::Excludes(denoms) => {
                for denom in denoms {
                    event = event.add_attribute("denom", denom);
                }
            }
        }

        event
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

pub struct OfferEvent<'a> {
    pub ty: &'a str,
    pub offer: &'a Offer,
    pub attr_keys: Vec<&'a str>,
}

impl<'a> From<OfferEvent<'a>> for Event {
    fn from(oe: OfferEvent) -> Self {
        Event::new(oe.ty.to_string()).add_attributes(oe.offer.get_event_attrs(oe.attr_keys))
    }
}

pub struct CollectionOfferEvent<'a> {
    pub ty: &'a str,
    pub collection_offer: &'a CollectionOffer,
    pub attr_keys: Vec<&'a str>,
}

impl<'a> From<CollectionOfferEvent<'a>> for Event {
    fn from(coe: CollectionOfferEvent) -> Self {
        Event::new(coe.ty.to_string())
            .add_attributes(coe.collection_offer.get_event_attrs(coe.attr_keys))
    }
}
