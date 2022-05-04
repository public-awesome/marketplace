import { ExpiryRange, Uint128 } from "./shared-types";

export type SudoMsg = ({
update_params: {
ask_expiry?: (ExpiryRange | null)
bid_expiry?: (ExpiryRange | null)
max_finders_fee_bps?: (number | null)
min_bid_amount?: (Uint128 | null)
operators?: (string[] | null)
trading_fee_bps?: (number | null)
[k: string]: unknown
}
} | {
add_ask_created_hook: {
hook: string
[k: string]: unknown
}
} | {
remove_ask_created_hook: {
hook: string
[k: string]: unknown
}
} | {
add_ask_filled_hook: {
hook: string
[k: string]: unknown
}
} | {
remove_ask_filled_hook: {
hook: string
[k: string]: unknown
}
})
