export type SudoMsg = ({
update_params: {
ask_expiry?: ([number, number] | null)
bid_expiry?: ([number, number] | null)
operators?: (string[] | null)
trading_fee_percent?: (number | null)
[k: string]: unknown
}
} | {
add_sale_finalized_hook: {
hook: string
[k: string]: unknown
}
} | {
remove_sale_finalized_hook: {
hook: string
[k: string]: unknown
}
})
