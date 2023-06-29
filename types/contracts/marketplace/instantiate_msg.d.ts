import { Duration, ExpiryRange, Uint128 } from "./shared-types";

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
 * Stale bid removal reward
 */
bid_removal_reward_bps: number
/**
 * Listing fee to reduce spam
 */
listing_fee: Uint128
/**
 * Max basis points for the finders fee
 */
max_finders_fee_bps: number
/**
 * Min value for bids and asks
 */
min_price: Uint128
/**
 * Operators are entites that are responsible for maintaining the active state of Asks. They listen to NFT transfer events, and update the active state of Asks.
 */
operators: string[]
/**
 * The address of the airdrop claim contract to detect sales
 */
sale_hook?: (string | null)
/**
 * Duration after expiry when a bid becomes stale (in seconds)
 */
stale_bid_duration: Duration
/**
 * Fair Burn fee for winning bids 0.25% = 25, 0.5% = 50, 1% = 100, 2.5% = 250
 */
trading_fee_bps: number
[k: string]: unknown
}
