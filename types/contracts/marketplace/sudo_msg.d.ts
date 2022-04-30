import { ExpiryRange } from "./shared-types";

export type SudoMsg = ({
update_params: {
ask_expiry?: (ExpiryRange | null)
bid_expiry?: (ExpiryRange | null)
operators?: (string[] | null)
trading_fee_basis_points?: (number | null)
[k: string]: unknown
}
} | {
add_sale_finalized_hook: {
hook: string
[k: string]: unknown
}
} | {
add_ask_hook: {
hook: string
[k: string]: unknown
}
} | {
remove_sale_finalized_hook: {
hook: string
[k: string]: unknown
}
} | {
remove_ask_hook: {
hook: string
[k: string]: unknown
}
})
