import { Addr } from "./shared-types";
/**
 * A fixed-point decimal value with 18 fractional digits, i.e. Decimal(1_000_000_000_000_000_000) == 1.0
 * 
 * The greatest possible value that can be represented is 340282366920938463463.374607431768211455 (which is (2^128 - 1) / 10^18)
 */
export type Decimal = string

export interface ParamsResponse {
params: SudoParams
[k: string]: unknown
}
export interface SudoParams {
/**
 * Valid time range for Asks (min, max) in seconds
 */
ask_expiry: [number, number]
/**
 * Valid time range for Bids (min, max) in seconds
 */
bid_expiry: [number, number]
/**
 * Operators are entites that are responsible for maintaining the active state of Asks They listen to NFT transfer events, and update the active state of Asks
 */
operators: Addr[]
/**
 * Fair Burn fee for winning bids 0.25% = 25, 0.5% = 50, 1% = 100, 2.5% = 250
 */
trading_fee_basis_points: Decimal
[k: string]: unknown
}
