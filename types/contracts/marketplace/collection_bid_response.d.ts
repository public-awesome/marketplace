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
expires: Timestamp
price: Uint128
[k: string]: unknown
}
