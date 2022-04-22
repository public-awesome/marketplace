export interface InstantiateMsg {
admins: string[]
admins_mutable: boolean
max_expiry: number
min_expiry: number
trading_fee_percent: number
[k: string]: unknown
}
