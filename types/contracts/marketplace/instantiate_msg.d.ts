export interface InstantiateMsg {
max_expiry: number
min_expiry: number
/**
 * Operators are entites that are responsible for maintaining the active state of Asks. They listen to NFT transfer events, and update the active state of Asks.
 */
operators: string[]
trading_fee_percent: number
[k: string]: unknown
}
