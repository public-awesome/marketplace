export interface InstantiateMsg {
/**
 * Valid time range for Asks (min, max) in seconds
 */
ask_expiry: [number, number]
/**
 * Valid time range for Bids (min, max) in seconds
 */
bid_expiry: [number, number]
/**
 * Operators are entites that are responsible for maintaining the active state of Asks. They listen to NFT transfer events, and update the active state of Asks.
 */
operators: string[]
sales_finalized_hook?: (string | null)
/**
 * Fair Burn fee for winning bids i.e. 125 = 0.125%, 250 = 0.25%, 500 = 0.5%, 1000 = 1%, 2500 = 2.5%
 */
trading_fee: number
[k: string]: unknown
}
