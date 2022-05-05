import { Addr, Timestamp, Uint128 } from "./shared-types";

export interface CollectionBidResponse {
bid?: (CollectionBid | null)
[k: string]: unknown
}
/**
 * Represents a bid (offer) across an entire collection in the marketplace
 */
export interface CollectionBid {
bidder: Addr
collection: Addr
expires_at: Timestamp
finders_fee_bps?: (number | null)
price: Uint128
[k: string]: unknown
}
