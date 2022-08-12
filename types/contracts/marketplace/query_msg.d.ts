import { AskOffset, BidOffset, CollectionBidOffset, CollectionOffset } from "./shared-types";

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
include_inactive?: (boolean | null)
limit?: (number | null)
start_after?: (number | null)
[k: string]: unknown
}
} | {
reverse_asks: {
collection: string
include_inactive?: (boolean | null)
limit?: (number | null)
start_before?: (number | null)
[k: string]: unknown
}
} | {
asks_sorted_by_price: {
collection: string
include_inactive?: (boolean | null)
limit?: (number | null)
start_after?: (AskOffset | null)
[k: string]: unknown
}
} | {
reverse_asks_sorted_by_price: {
collection: string
include_inactive?: (boolean | null)
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
include_inactive?: (boolean | null)
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
bids_by_bidder_sorted_by_expiration: {
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
reverse_bids_sorted_by_price: {
collection: string
limit?: (number | null)
start_before?: (BidOffset | null)
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
limit?: (number | null)
start_after?: (CollectionOffset | null)
[k: string]: unknown
}
} | {
collection_bids_by_bidder_sorted_by_expiration: {
bidder: string
limit?: (number | null)
start_after?: (CollectionBidOffset | null)
[k: string]: unknown
}
} | {
collection_bids_sorted_by_price: {
collection: string
limit?: (number | null)
start_after?: (CollectionBidOffset | null)
[k: string]: unknown
}
} | {
reverse_collection_bids_sorted_by_price: {
collection: string
limit?: (number | null)
start_before?: (CollectionBidOffset | null)
[k: string]: unknown
}
} | {
ask_hooks: {
[k: string]: unknown
}
} | {
bid_hooks: {
[k: string]: unknown
}
} | {
sale_hooks: {
[k: string]: unknown
}
} | {
params: {
[k: string]: unknown
}
})
