import { Bid } from "./shared-types";

export interface BidsByBidderSortedByExpirationResponse {
bids: Bid[]
[k: string]: unknown
}
