/**
 * A human readable address.
 *
 * In Cosmos, this is typically bech32 encoded. But for multi-chain smart contracts no assumptions should be made other than being UTF-8 encoded and of reasonable length.
 *
 * This type represents a validated address. It can be created in the following ways 1. Use `Addr::unchecked(input)` 2. Use `let checked: Addr = deps.api.addr_validate(input)?` 3. Use `let checked: Addr = deps.api.addr_humanize(canonical_addr)?` 4. Deserialize from JSON. This must only be done from JSON that was validated before such as a contract's state. `Addr` must not be used in messages sent by the user because this would result in unvalidated instances.
 *
 * This type is immutable. If you really need to mutate it (Really? Are you sure?), create a mutable copy using `let mut mutable = Addr::to_string()` and operate on that `String` instance.
 */
export type Addr = string;
/**
 * A point in time in nanosecond precision.
 *
 * This type can represent times from 1970-01-01T00:00:00Z to 2554-07-21T23:34:33Z.
 *
 * ## Examples
 *
 * ``` # use cosmwasm_std::Timestamp; let ts = Timestamp::from_nanos(1_000_000_202); assert_eq!(ts.nanos(), 1_000_000_202); assert_eq!(ts.seconds(), 1); assert_eq!(ts.subsec_nanos(), 202);
 *
 * let ts = ts.plus_seconds(2); assert_eq!(ts.nanos(), 3_000_000_202); assert_eq!(ts.seconds(), 3); assert_eq!(ts.subsec_nanos(), 202); ```
 */
export type Timestamp = Uint64;
/**
 * A thin wrapper around u64 that is using strings for JSON encoding/decoding, such that the full u64 range can be used for clients that convert JSON numbers to floats, like JavaScript and jq.
 *
 * # Examples
 *
 * Use `from` to create instances of this and `u64` to get the value out:
 *
 * ``` # use cosmwasm_std::Uint64; let a = Uint64::from(42u64); assert_eq!(a.u64(), 42);
 *
 * let b = Uint64::from(70u32); assert_eq!(b.u64(), 70); ```
 */
export type Uint64 = string;
/**
 * A thin wrapper around u128 that is using strings for JSON encoding/decoding, such that the full u128 range can be used for clients that convert JSON numbers to floats, like JavaScript and jq.
 *
 * # Examples
 *
 * Use `from` to create instances of this and `u128` to get the value out:
 *
 * ``` # use cosmwasm_std::Uint128; let a = Uint128::from(123u128); assert_eq!(a.u128(), 123);
 *
 * let b = Uint128::from(42u64); assert_eq!(b.u128(), 42);
 *
 * let c = Uint128::from(70u32); assert_eq!(c.u128(), 70); ```
 */
export type Uint128 = string;
export type SaleType = ("fixed_price" | "auction");
/**
 * Represents an ask on the marketplace
 */
export interface Ask {
    [k: string]: unknown;
    collection: Addr;
    expires: Timestamp;
    finders_fee_bps?: (number | null);
    funds_recipient?: (Addr | null);
    is_active: boolean;
    price: Uint128;
    reserve_for?: (Addr | null);
    sale_type: SaleType;
    seller: Addr;
    token_id: number;
}
/**
 * Offset for ask pagination
 */
export interface AskOffset {
    [k: string]: unknown;
    price: Uint128;
    token_id: number;
}
/**
 * Offset for bid pagination
 */
export interface BidOffset {
    [k: string]: unknown;
    bidder: Addr;
    price: Uint128;
    token_id: number;
}
/**
 * Represents a bid (offer) on the marketplace
 */
export interface Bid {
    [k: string]: unknown;
    bidder: Addr;
    collection: Addr;
    expires: Timestamp;
    finders_fee_bps?: (number | null);
    price: Uint128;
    token_id: number;
}
/**
 * Offset for collection bid pagination
 */
export interface CollectionBidOffset {
    [k: string]: unknown;
    bidder: string;
    collection: string;
    price: Uint128;
}
/**
 * Offset for collection pagination
 */
export interface CollectionOffset {
    [k: string]: unknown;
    collection: string;
    token_id: number;
}
export interface Coin {
    [k: string]: unknown;
    amount: Uint128;
    denom: string;
}
/**
 * Duration is a delta of time. You can add it to a BlockInfo or Expiration to move that further in the future. Note that an height-based Duration and a time-based Expiration cannot be combined
 */
export type Duration = ({
    height: number
    } | {
    time: number
    });
export interface ExpiryRange {
    [k: string]: unknown;
    max: number;
    min: number;
}
