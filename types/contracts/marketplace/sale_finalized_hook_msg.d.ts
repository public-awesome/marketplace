import { Coin } from "./shared-types";

export interface SaleFinalizedHookMsg {
buyer: string
collection: string
price: Coin
seller: string
token_id: number
[k: string]: unknown
}
