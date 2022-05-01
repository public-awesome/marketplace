import { Addr, Uint128 } from "./shared-types";

export type QueryMsg = ({
collections: {
limit?: (number | null)
start_after?: (string | null)
[k: string]: unknown
}
} | {
ask: {
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
start_after?: (AskOffset | null)
[k: string]: unknown
}
} | {
reverse_asks_sorted_by_price: {
collection: string
limit?: (number | null)
start_before?: (AskOffset | null)
[k: string]: unknown
}
} | {
ask_count: {
collection: string
[k: string]: unknown
}
} | {
asks_by_seller: {
limit?: (number | null)
seller: string
start_after?: (CollectionOffset | null)
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
limit?: (number | null)
start_after?: (CollectionOffset | null)
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
start_after?: (BidOffset | null)
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
ask_created_hooks: {
[k: string]: unknown
}
} | {
ask_filled_hooks: {
[k: string]: unknown
}
} | {
params: {
[k: string]: unknown
}
})

/**
 * Offset for ask pagination
 */
export interface AskOffset {
price: Uint128
token_id: number
[k: string]: unknown
}
/**
 * Offset for collection pagination
 */
export interface CollectionOffset {
collection: string
token_id: number
[k: string]: unknown
}
/**
 * Offset for bid pagination
 */
export interface BidOffset {
bidder: Addr
price: Uint128
token_id: number
[k: string]: unknown
}
