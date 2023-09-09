RESERVE_AUCTION="stars1dnadsd7tx0dmnpp26ms7d66zsp7tduygwjgfjzueh0lg9t5lq5vq9kn47c"
NEW_CODE_ID=123
MSG=$(cat <<EOF
{

}
EOF
)

TITLE="Stargaze Live Auction v1.0.1" 
MARKDOWN="scripts/markdown/migrateContractReserveAuctionV1.0.1.md"
DESCRIPTION=$(cat "$MARKDOWN" | base64 | tr -d '\n')

FROM="hot-wallet"
DEPOSIT="10000000000ustars"

CHAIN_ID="stargaze-1"
NODE="https://rpc.stargaze-apis.com:443"

starsd tx gov submit-proposal migrate-contract "$RESERVE_AUCTION" $NEW_CODE_ID "$MSG" \
    --title "$TITLE" \
    --description "$(echo "$DESCRIPTION" | base64 --decode)" \
    --from "$FROM" \
    --deposit "$DEPOSIT" \
    --chain-id "$CHAIN_ID" \
    --node "$NODE" \
    --gas-prices 1ustars \
    --gas auto \
    --gas-adjustment 1.5