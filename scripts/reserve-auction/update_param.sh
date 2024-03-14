RESERVE_AUCTION="stars1dnadsd7tx0dmnpp26ms7d66zsp7tduygwjgfjzueh0lg9t5lq5vq9kn47c"
MSG=$(cat <<EOF
{
  "update_params": {
      "min_bid_increment_percent" : "0.01"
  }
}
EOF
)

TITLE="Update Reserve Auction"
DESCRIPTION="Update Reserve Auction"

FROM="hot-wallet"
DEPOSIT="10000000000ustars"
CHAIN_ID="elgafar-1"
NODE="https://rpc.elgafar-1.stargaze-apis.com:443"


starsd tx gov submit-proposal sudo-contract "$RESERVE_AUCTION" "$MSG" \
    --title "$TITLE" \
    --description "$(echo "$DESCRIPTION" | base64 --decode)" \
    --from "$FROM" \
    --deposit "$DEPOSIT" \
    --chain-id "$CHAIN_ID" \
    --node "$NODE" \
    --gas-prices 1ustars \
    --gas auto \
    --gas-adjustment 1.5