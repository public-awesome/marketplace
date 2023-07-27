## Store WASM Code

This uploads the code for Stargaze Live Auction v1.0.0

The source code is available at https://github.com/public-awesome/core/releases/tag/stargaze_reserve_auction-v1.0.0

This CosmWasm smart contract implements a reserve auction on the Stargaze network. In a reserve auction, an item is not sold unless the highest bid is equal to or greater than a predetermined reserve price. The contract includes several key features such as auction creation, bid placement, auction settlement, and cancellation.

To integrate with the Stargaze Reserve Auction contract please refer to the following documentation https://crates.io/crates/stargaze-reserve-auction

### Compile Instructions

```sh
docker run --rm -v "$(pwd)":/code --platform linux/amd64 \
	--mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
	--mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
	cosmwasm/workspace-optimizer:0.12.13
```

This results in the following SHA256 checksum:

```
f7cdf509a1889e21399c33eee9e68ac328abd4a456142db080d760d15135fe56  stargaze_reserve_auction.wasm
```

### Verify On-chain Contract

```sh
starsd q gov proposal $id --output json \
| jq -r '.content.wasm_byte_code' \
| base64 -d \
| gzip -dc \
| sha256sum

```

### Verify Local Contract

```
sha256sum artifacts/stargaze_reserve_auction.wasm
```
