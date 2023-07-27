## Store WASM Code

This uploads the code for Stargaze Live Auction v1.0.1

The source code is available at https://github.com/public-awesome/marketplace/releases/tag/stargaze_reserve_auction-v1.0.1

This patch follows the release of Reserve Auctions on mainnet to fix a few items outlined below:

- fix usage of seller_funds_recipient address
- add more attributes to certain contract events

### Compile Instructions

```sh
docker run --rm -v "$(pwd)":/code --platform linux/amd64 \
	--mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
	--mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
	cosmwasm/workspace-optimizer:0.12.13
```

This results in the following SHA256 checksum:

```
4e77dfe1830d5a33058502d19ef990773f45acfee7862ebb5355626c75bd0eb1  stargaze_reserve_auction.wasm
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
