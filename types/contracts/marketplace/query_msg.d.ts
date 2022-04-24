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
[k: string]: unknown
}
} | {
hooks: {
[k: string]: unknown
}
})
