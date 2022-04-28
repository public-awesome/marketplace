export interface InstantiateMsg {
ask_expiry: [number, number]
bid_expiry: [number, number]
/**
 * Operators are entites that are responsible for maintaining the active state of Asks. They listen to NFT transfer events, and update the active state of Asks.
 */
operators: string[]
sales_finalized_hook?: (string | null)
trading_fee_percent: number
[k: string]: unknown
}
