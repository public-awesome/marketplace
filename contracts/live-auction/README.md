# Stargaze Reserve (Timed) Auctions

Reserve auctions enable an NFT to receive bids of increasing value for a certain duration of time.

This contract honors royalties.

## State

### `Config`

These parameters will be set on contract instantiation. They can also be updated via Sudo.

```rs
pub struct Config {
    pub create_auction_fee: Uint128,
    pub min_reserve_price: Coin,
    pub min_bid_increment: u64,
    pub min_duration: u64,
    pub extend_duration: u64,
    pub trading_fee: Decimal,
}
```

### `Auction`

```rs
pub struct HighBid {
    pub coin: Coin,
    pub bidder: Addr,
}

pub struct Auction {
    pub token_id: String,
    pub collection: Addr,
    pub seller: Addr,
    pub reserve_price: Coin,
    pub start_time: Timestamp,
    pub end_time: Timestamp,
    pub seller_funds_recipient: Option<Addr>,
    pub high_bid: Option<HighBid>,
    pub first_bid_time: Option<Timestamp>,
}
```

## Messages

### `Instantiate`

```rs
pub struct InstantiateMsg {
    pub create_auction_fee: Uint128,
    pub min_reserve_price: Coin,
    pub min_duration: u64,
    pub min_bid_increment: u64,
    pub extend_duration: u64,
    pub trading_fee_bps: u64,
}
```

### `CreateAuction`

Creates an auction for the given NFT, the NFT is escrowed at the time of contract creation. The timer stars after `start_time`. The contract maintains custody of the NFT until the auction is finished. `Approval` must be given this contract so it can transfer the NFT to itself. `Approval` can be batched to run before `CreateAuction`.

```rs
CreateAuction {
    collection: String,
    token_id: String,
    reserve_price: Coin,
    start_time: Timestamp,
    end_time: Timestamp,
    seller_funds_recipient: Option<String>,
}
```

### `UpdateReservePrice`

Updated the reserve price of an existing auction. This only runs if the auction hasn't started yet.

```rs
UpdateReservePrice {
    collection: String,
    token_id: String,
    reserve_price: Coin,
}
```

### `CancelAuction`

Cancels an existing auction. This only runs if the auction hasn't started yet.

```rs
CancelAuction {
    collection: String,
    token_id: String,
}
```

### `PlaceBid`

Places a bid on the given NFT. Each bid must be a fixed amount greater than the last. The amount for the bid is held in escrow by the contract until either the auction ends or a higher bid is placed. When a higher bid is placed, the previous bid is refunded. If a bid is placed within `extend_duration` of the auction ending, the auction is extended by `extend_duration` seconds.

```rs
PlaceBid {
    collection: String,
    token_id: String,
}
```

### `SettleAuction`

Ends the auction for the given NFT. It sends it to the highest bidder, and transfers the funds from the bid to the seller. Royalties are paid to the creator. Anyone can call this function.

```rs
SettleAuction {
    collection: String,
    token_id: String,
}
```
