import { ExpiryRange } from "./shared-types";

export interface InstantiateMsg {
/**
 * Valid time range for Asks (min, max) in seconds
 */
ask_expiry: ExpiryRange
/**
 * Valid time range for Bids (min, max) in seconds
 */
bid_expiry: ExpiryRange
/**
 * Operators are entites that are responsible for maintaining the active state of Asks. They listen to NFT transfer events, and update the active state of Asks.
 */
operators: string[]
sales_finalized_hook?: (string | null)
/**
 * Fair Burn fee for winning bids 0.25% = 25, 0.5% = 50, 1% = 100, 2.5% = 250
 */
trading_fee_basis_points: number
[k: string]: unknown
}
