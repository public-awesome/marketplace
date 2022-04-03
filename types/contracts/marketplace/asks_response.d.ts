import { Addr, Coin } from "./shared-types";

export interface AsksResponse {
asks: AskInfo[]
[k: string]: unknown
}
export interface AskInfo {
funds_recipient?: (Addr | null)
price: Coin
token_id: number
[k: string]: unknown
}
