set -eux

CONTRACT=artifacts/stargaze_reserve_auction.wasm

TITLE="Stargaze Live Auction v1.0.1" 
MARKDOWN="scripts/markdown/storeWasmCodeReserveAuctionv1.0.1.md"
DESCRIPTION=$(cat "$MARKDOWN" | base64 | tr -d '\n')
SOURCE="https://github.com/public-awesome/marketplace/releases/tag/stargaze_reserve_auction-v1.0.1"
BUILDER="cosmwasm/workspace-optimizer:0.15.1"
HASH="4e77dfe1830d5a33058502d19ef990773f45acfee7862ebb5355626c75bd0eb1"

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
