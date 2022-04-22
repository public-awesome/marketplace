export interface ParamResponse {
params: SudoParams
[k: string]: unknown
}
export interface SudoParams {
max_expiry: number
min_expiry: number
trading_fee_percent: number
[k: string]: unknown
}
