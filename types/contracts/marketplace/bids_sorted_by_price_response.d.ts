import { Bid } from "./shared-types";

export interface BidsSortedByPriceResponse {
bids: Bid[]
[k: string]: unknown
}
