## Store WASM Code

This uploads the code for Stargaze Reserve Auction v1.0.0

The source code is available at https://github.com/public-awesome/core/releases/tag/stargaze_reserve_auction-v1.0.0

### Compile Instructions

```sh
docker run --rm -v "$(pwd)":/code --platform linux/amd64 \
	--mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
	--mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
	cosmwasm/workspace-optimizer:0.12.13
```

This results in the following SHA256 checksum:

```
f2674f0df06a37254563b81aefc0dd184874c996a4efaeaaec7199e390128153  stargaze_reserve_auction.wasm
```

### Verify On-chain Contract

```sh
starsd q gov proposal $id --output json \\
| jq -r '.content.wasm_byte_code' \\
| base64 -d \\
| gzip -dc \\
| sha256sum

```

### Verify Local Contract

```
sha256sum artifacts/stargaze_reserve_auction.wasm
```
