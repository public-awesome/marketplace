import { ExpiryRange, Uint128 } from "./shared-types";

export type SudoMsg = ({
update_params: {
ask_expiry?: (ExpiryRange | null)
bid_expiry?: (ExpiryRange | null)
bid_removal_reward_bps?: (number | null)
max_finders_fee_bps?: (number | null)
min_price?: (Uint128 | null)
operators?: (string[] | null)
stale_bid_duration?: (number | null)
trading_fee_bps?: (number | null)
[k: string]: unknown
}
} | {
add_ask_hook: {
hook: string
[k: string]: unknown
}
} | {
add_bid_hook: {
hook: string
[k: string]: unknown
}
} | {
remove_ask_hook: {
hook: string
[k: string]: unknown
}
} | {
remove_bid_hook: {
hook: string
[k: string]: unknown
}
} | {
add_sale_hook: {
hook: string
[k: string]: unknown
}
} | {
remove_sale_hook: {
hook: string
[k: string]: unknown
}
})
