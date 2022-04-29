import { Bid } from "./shared-types";

export interface CollectionBidsByBidderResponse {
bids: Bid[]
[k: string]: unknown
}
