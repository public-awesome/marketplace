export interface InstantiateMsg {
admin: string
max_expiry: number
min_expiry: number
trading_fee_percent: number
[k: string]: unknown
}
