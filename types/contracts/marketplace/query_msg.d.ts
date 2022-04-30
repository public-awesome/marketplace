import { Uint128 } from "./shared-types";

export type QueryMsg = ({
current_ask: {
collection: string
token_id: number
[k: string]: unknown
}
} | {
asks: {
collection: string
limit?: (number | null)
start_after?: (number | null)
[k: string]: unknown
}
} | {
asks_sorted_by_price: {
collection: string
limit?: (number | null)
start_after?: (Offset | null)
[k: string]: unknown
}
} | {
reverse_asks_sorted_by_price: {
collection: string
limit?: (number | null)
start_before?: (Offset | null)
[k: string]: unknown
}
} | {
ask_count: {
collection: string
[k: string]: unknown
}
} | {
asks_by_seller: {
seller: string
[k: string]: unknown
}
} | {
listed_collections: {
limit?: (number | null)
start_after?: (string | null)
[k: string]: unknown
}
} | {
bid: {
bidder: string
collection: string
token_id: number
[k: string]: unknown
}
} | {
bids_by_bidder: {
bidder: string
[k: string]: unknown
}
} | {
bids: {
collection: string
limit?: (number | null)
start_after?: (string | null)
token_id: number
[k: string]: unknown
}
} | {
bids_sorted_by_price: {
collection: string
limit?: (number | null)
order_asc: boolean
[k: string]: unknown
}
} | {
params: {
[k: string]: unknown
}
} | {
collection_bid: {
bidder: string
collection: string
[k: string]: unknown
}
} | {
collection_bids_by_bidder: {
bidder: string
[k: string]: unknown
}
} | {
collection_bids_sorted_by_price: {
collection: string
limit?: (number | null)
order_asc: boolean
[k: string]: unknown
}
} | {
sale_finalized_hooks: {
[k: string]: unknown
}
} | {
ask_hooks: {
[k: string]: unknown
}
})

/**
 * Offsets for pagination
 */
export interface Offset {
price: Uint128
token_id: number
[k: string]: unknown
}
