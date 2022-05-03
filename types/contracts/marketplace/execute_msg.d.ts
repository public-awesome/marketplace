import { Coin, Timestamp } from "./shared-types";

export type ExecuteMsg = ({
set_ask: {
collection: string
expires: Timestamp
finders_fee_basis_points?: (number | null)
funds_recipient?: (string | null)
price: Coin
reserve_for?: (string | null)
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
update_ask_is_active: {
collection: string
is_active: boolean
token_id: number
[k: string]: unknown
}
} | {
update_ask_price: {
collection: string
price: Coin
token_id: number
[k: string]: unknown
}
} | {
set_bid: {
collection: string
expires: Timestamp
finder?: (string | null)
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
finder?: (string | null)
token_id: number
[k: string]: unknown
}
} | {
set_collection_bid: {
collection: string
expires: Timestamp
[k: string]: unknown
}
} | {
accept_collection_bid: {
bidder: string
collection: string
finder?: (string | null)
token_id: number
[k: string]: unknown
}
})
