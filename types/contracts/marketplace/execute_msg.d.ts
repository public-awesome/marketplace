import { Uint128 } from "./shared-types";

export type ExecuteMsg = ({
set_bid: {
collection: string
expires: number
token_id: number
[k: string]: unknown
}
} | {
remove_bid: {
collection: string
token_id: number
[k: string]: unknown
}
} | {
set_ask: {
collection: string
expires: number
funds_recipient?: (string | null)
price: Coin
token_id: number
[k: string]: unknown
}
} | {
remove_ask: {
collection: string
token_id: number
[k: string]: unknown
}
} | {
accept_bid: {
bidder: string
collection: string
token_id: number
[k: string]: unknown
}
})

export interface Coin {
amount: Uint128
denom: string
[k: string]: unknown
}
