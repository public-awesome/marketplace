import { Coin } from "./shared-types";

export interface AskCreatedHooksResponse {
collection: string
funds_recipient: string
price: Coin
seller: string
token_id: number
[k: string]: unknown
}
