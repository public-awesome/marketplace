set -eux

CONTRACT=artifacts/stargaze_reserve_auction.wasm

TITLE="Stargaze Live Auction v1.0.0" 
MARKDOWN="scripts/markdown/stargaze_reserve_auction-v1.0.0.md"
DESCRIPTION=$(cat "$MARKDOWN" | base64 | tr -d '\n')
SOURCE="https://github.com/public-awesome/core/releases/tag/stargaze_reserve_auction-v1.0.0"
BUILDER="cosmwasm/workspace-optimizer:0.12.13"
HASH="f7cdf509a1889e21399c33eee9e68ac328abd4a456142db080d760d15135fe56"

FROM="hot-wallet"
DEPOSIT="10000000000ustars"

RUN_AS="stars19mmkdpvem2xvrddt8nukf5kfpjwfslrsu7ugt5"
ANY_OF_ADDRS="stars19mmkdpvem2xvrddt8nukf5kfpjwfslrsu7ugt5,stars1r5ecq7zn6hwh5e68e79ume8rp9ht7kjz352drk"

CHAIN_ID="stargaze-1"
NODE="https://rpc.stargaze-apis.com:443"

starsd tx gov submit-proposal wasm-store "$CONTRACT" \
 --title "$TITLE" \
 --description "$(echo "$DESCRIPTION" | base64 --decode)" \
 --code-source-url "$SOURCE" \
 --builder "$BUILDER" \
 --code-hash "$HASH" \
 --from "$FROM" \
 --deposit "$DEPOSIT" \
 --run-as "$RUN_AS" \
 --instantiate-anyof-addresses "$ANY_OF_ADDRS" \
 --chain-id "$CHAIN_ID" \
 --node "$NODE" \
 --gas-prices 1ustars \
 --gas auto \
 --gas-adjustment 1.5