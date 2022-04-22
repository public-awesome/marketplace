import { Timestamp, Uint128 } from "./shared-types";

export type ExecuteMsg = ({
set_ask: {
collection: string
expires: Timestamp
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
update_ask_state: {
active: boolean
collection: string
token_id: number
[k: string]: unknown
}
} | {
update_ask: {
collection: string
price: Coin
token_id: number
[k: string]: unknown
}
} | {
set_bid: {
collection: string
expires: Timestamp
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
accept_bid: {
bidder: string
collection: string
token_id: number
[k: string]: unknown
}
} | {
freeze: {
[k: string]: unknown
}
} | {
update_admins: {
admins: string[]
[k: string]: unknown
}
})

export interface Coin {
amount: Uint128
denom: string
[k: string]: unknown
}
