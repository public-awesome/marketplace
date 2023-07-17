set -eux

CONTRACT=artifacts/stargaze_reserve_auction.wasm

TITLE="Stargaze Reserve Auction v1.2.0" 
DESCRIPTION=$(cat scripts/markdown/stargaze_reserve_auction-v1.2.0.md | jq -Rsa | tr -d '"')
SOURCE="https://github.com/public-awesome/core/releases/tag/stargaze_reserve_auction-v1.2.0"
BUILDER="cosmwasm/workspace-optimizer:0.12.13"
HASH="f2674f0df06a37254563b81aefc0dd184874c996a4efaeaaec7199e390128153"

FROM="hot-wallet"
DEPOSIT="10000000000ustars"

RUN_AS="stars19mmkdpvem2xvrddt8nukf5kfpjwfslrsu7ugt5"
ANY_OF_ADDRS="stars19mmkdpvem2xvrddt8nukf5kfpjwfslrsu7ugt5,stars1r5ecq7zn6hwh5e68e79ume8rp9ht7kjz352drk"

CHAIN_ID="elgafar-1"
NODE="https://rpc.elgafar-1.stargaze-apis.com:443"

starsd tx gov submit-proposal wasm-store "$CONTRACT" \
 --title "$TITLE" \
 --description "$DESCRIPTION" \
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
