import { Addr } from "./shared-types";

export interface ParamResponse {
params: SudoParams
[k: string]: unknown
}
export interface SudoParams {
max_expiry: number
min_expiry: number
operators: Addr[]
trading_fee_percent: number
[k: string]: unknown
}
