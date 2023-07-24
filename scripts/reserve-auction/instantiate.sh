CODE_ID=75
MSG=$(cat <<EOF
{
    "fair_burn": "stars1pm9sf7ftn6fmxcsx9cn9cng2r42vfhg7wh58h975cv9tdpcjjrdsgd0lzt",
    "trading_fee_percent": "0.02",
    "min_bid_increment_percent": "0.01",
    "min_duration": 60,
    "max_duration": 5184000,
    "extend_duration": 900,
    "create_auction_fee": {"denom": "ustars", "amount": "5000000"},
    "max_auctions_to_settle_per_block": 25,
    "halt_duration_threshold": 300,
    "halt_buffer_duration": 600,
    "halt_postpone_duration": 1800,
    "min_reserve_prices": [{"denom": "ustars", "amount": "1000000"}]
}
EOF
)

FROM="hot-wallet"
CHAIN_ID="stargaze-1"
NODE="https://rpc.stargaze-apis.com:443"

starsd tx wasm instantiate $CODE_ID  "$MSG"  \
  --label "stargaze-live-auction" \
  --from "$FROM" \
  --chain-id "$CHAIN_ID" \
  --node "$NODE" \
  --gas-prices 1ustars \
  --gas-adjustment 1.7 \
  --gas auto \
  --no-admin \
  -b block \
  -o json | jq .
