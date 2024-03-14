CODE_ID=2755
MSG=$(cat <<EOF
{
    "sudo_params": {
        "fair_burn": "stars177jd3r8aul2dgt9pj77x8zem3au46ee2cj4srxwqdw4lkpd7tsqquz2r2d",
        "royalty_registry": "stars1crgx0f70fzksa57hq87wtl8f04h0qyk5la0hk0fu8dyhl67ju80qaxzr5z",
        "listing_fee": {
            "amount": "5000000",
            "denom": "ustars"
        },
        "min_ask_removal_reward": {
            "amount": "4000000",
            "denom": "ustars"
        },
        "min_offer_removal_reward": {
            "amount": "3000000",
            "denom": "ustars"
        },
        "trading_fee_percent": "0.02",
        "max_royalty_fee_percent": "0.1",
        "max_finders_fee_percent": "0.08",
        "min_expiration_seconds": 60,
        "order_removal_lookahead_secs": 8,
        "max_asks_removed_per_block": 30,
        "max_offers_removed_per_block": 10,
        "max_collection_offers_removed_per_block": 20
    },
    "price_ranges": [
        [
            "ustars",
            {
                "min": "500000",
                "max": "1000000000000000"
            }
        ]
    ]
}
EOF
)

LABEL="stargaze-marketplace-v2"

ADMIN="stars19mmkdpvem2xvrddt8nukf5kfpjwfslrsu7ugt5"
FROM="hot-wallet"

CHAIN_ID="elgafar-1"
NODE="https://rpc.elgafar-1.stargaze-apis.com:443"

starsd tx wasm instantiate $CODE_ID  "$MSG" \
  --label "$LABEL" \
  --admin "$ADMIN" \
  --from "$FROM" \
  --chain-id "$CHAIN_ID" \
  --node "$NODE" \
  --gas-prices 0.1ustars \
  --gas-adjustment 1.7 \
  --gas auto \
  -b block \
  -o json | jq .
