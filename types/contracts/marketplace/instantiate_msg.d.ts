import { ExpiryRange } from "./shared-types";

export interface InstantiateMsg {
/**
 * Valid time range for Asks (min, max) in seconds
 */
ask_expiry: ExpiryRange
ask_filled_hook?: (string | null)
/**
 * Valid time range for Bids (min, max) in seconds
 */
bid_expiry: ExpiryRange
/**
 * Max basis points for the finders fee
 */
max_finders_fee_bps: number
/**
 * Operators are entites that are responsible for maintaining the active state of Asks. They listen to NFT transfer events, and update the active state of Asks.
 */
operators: string[]
/**
 * Fair Burn fee for winning bids 0.25% = 25, 0.5% = 50, 1% = 100, 2.5% = 250
 */
trading_fee_bps: number
[k: string]: unknown
}
