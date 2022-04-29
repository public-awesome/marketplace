export type SudoMsg = ({
update_params: {
ask_expiry?: ([number, number] | null)
bid_expiry?: ([number, number] | null)
operators?: (string[] | null)
trading_fee?: (number | null)
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
