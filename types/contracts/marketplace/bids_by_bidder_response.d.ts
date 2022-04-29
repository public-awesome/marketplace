import { Bid } from "./shared-types";

export interface BidsByBidderResponse {
bids: Bid[]
[k: string]: unknown
}
