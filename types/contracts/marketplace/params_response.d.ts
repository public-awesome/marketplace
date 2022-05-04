import { Addr, Duration, ExpiryRange, Uint128 } from "./shared-types";

/**
 * A fixed-point decimal value with 18 fractional digits, i.e. Decimal(1_000_000_000_000_000_000) == 1.0
 *
 * The greatest possible value that can be represented is 340282366920938463463.374607431768211455 (which is (2^128 - 1) / 10^18)
 */
export type Decimal = string;

export interface ParamsResponse {
  params: SudoParams;
  [k: string]: unknown;
}
export interface SudoParams {
  /**
   * Valid time range for Asks (min, max) in seconds
   */
  ask_expiry: ExpiryRange;
  /**
   * Valid time range for Bids (min, max) in seconds
   */
  bid_expiry: ExpiryRange;
  /**
   * Max value for the finders fee
   */
  max_finders_fee_percent: Decimal;
  /**
   * Min value for a bid
   */
  min_price: Uint128;
  /**
   * Operators are entites that are responsible for maintaining the active state of Asks They listen to NFT transfer events, and update the active state of Asks
   */
  operators: Addr[];
  /**
   * Fair Burn fee for winning bids
   */
  trading_fee_percent: Decimal;
  [k: string]: unknown;
}
