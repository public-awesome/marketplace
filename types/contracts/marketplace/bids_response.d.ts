import { Coin } from "./shared-types";

export interface BidsResponse {
bids: BidInfo[]
[k: string]: unknown
}
export interface BidInfo {
price: Coin
token_id: number
[k: string]: unknown
}
