use crate::state::Ask;

use cosmwasm_std::Event;
use std::vec;

pub struct AskEvent<'a> {
    pub ty: &'a str,
    pub ask: &'a Ask,
}

impl<'a> From<AskEvent<'a>> for Event {
    fn from(ae: AskEvent) -> Self {
        Event::new(ae.ty.to_string()).add_attributes(ae.ask.get_event_attrs(vec![
            "collection",
            "token_id",
            "creator",
            "price",
            "asset_recipient",
            "finders_fee_bps",
            "expiration",
            "removal_reward",
        ]))
    }
}

// pub struct UpdatePairEvent<'a> {
//     pub ty: &'a str,
//     pub pair: &'a Pair,
// }

// impl<'a> From<UpdatePairEvent<'a>> for Event {
//     fn from(pe: UpdatePairEvent) -> Self {
//         Event::new(pe.ty.to_string()).add_attributes(pe.pair.get_event_attrs(vec![
//             "pair_type",
//             "swap_fee_percent",
//             "reinvest_tokens",
//             "reinvest_nfts",
//             "bonding_curve",
//             "spot_price",
//             "delta",
//             "is_active",
//             "asset_recipient",
//         ]))
//     }
// }

// pub struct NftTransferEvent<'a> {
//     pub ty: &'a str,
//     pub pair: &'a Pair,
//     pub token_ids: &'a Vec<String>,
// }

// impl<'a> From<NftTransferEvent<'a>> for Event {
//     fn from(nte: NftTransferEvent) -> Self {
//         Event::new(nte.ty.to_string())
//             .add_attributes(nte.pair.get_event_attrs(vec!["total_nfts"]))
//             .add_attributes(nte.token_ids.iter().map(|token_id| ("token_id", token_id)))
//     }
// }

// pub struct TokenTransferEvent<'a> {
//     pub ty: &'a str,
//     pub funds: &'a Coin,
// }

// impl<'a> From<TokenTransferEvent<'a>> for Event {
//     fn from(tte: TokenTransferEvent) -> Self {
//         Event::new(tte.ty.to_string()).add_attribute("funds", tte.funds.to_string())
//     }
// }

// pub struct SwapEvent<'a> {
//     pub ty: &'a str,
//     pub pair: &'a Pair,
//     pub token_id: &'a str,
//     pub sender_recipient: &'a Addr,
//     pub quote_summary: &'a QuoteSummary,
// }

// impl<'a> From<SwapEvent<'a>> for Event {
//     fn from(se: SwapEvent) -> Self {
//         let mut event = Event::new(se.ty.to_string())
//             .add_attributes(se.pair.get_event_attrs(vec!["spot_price", "is_active"]));

//         event = event.add_attributes(vec![
//             attr("token_id", se.token_id),
//             attr("sender_recipient", se.sender_recipient),
//             attr("fair_burn_fee", se.quote_summary.fair_burn.amount),
//             attr("seller_amount", se.quote_summary.seller_amount),
//         ]);

//         if let Some(royalty) = se.quote_summary.royalty.as_ref() {
//             event = event.add_attribute("royalty_fee", royalty.amount);
//         }
//         if let Some(swap) = se.quote_summary.swap.as_ref() {
//             event = event.add_attribute("swap_fee", swap.amount);
//         }

//         event
//     }
// }

// pub struct PairInternalEvent<'a> {
//     pub pair: &'a Pair,
// }

// impl<'a> From<PairInternalEvent<'a>> for Event {
//     fn from(pie: PairInternalEvent) -> Self {
//         Event::new("pair-internal".to_string()).add_attributes(pie.pair.get_event_attrs(vec![
//             "total_tokens",
//             "sell_to_pair_quote",
//             "buy_from_pair_quote",
//         ]))
//     }
// }
