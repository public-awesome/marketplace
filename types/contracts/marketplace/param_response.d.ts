import { Addr } from "./shared-types";

export interface ParamResponse {
params: SudoParams
[k: string]: unknown
}
export interface SudoParams {
ask_expiry: [number, number]
bid_expiry: [number, number]
operators: Addr[]
trading_fee_percent: number
[k: string]: unknown
}
