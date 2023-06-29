import { Bid } from "./shared-types";

export interface BidsResponse {
bids: Bid[]
[k: string]: unknown
}
