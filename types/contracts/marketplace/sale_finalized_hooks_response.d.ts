import { Coin } from "./shared-types";

export interface SaleFinalizedHooksResponse {
buyer: string
collection: string
price: Coin
seller: string
token_id: number
[k: string]: unknown
}
