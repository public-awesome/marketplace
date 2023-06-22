ADMIN=stars10w5eulj60qp3cfqa0hkmke78qdy2feq6x9xdmd
KEY=$(starsd keys show $ADMIN | jq -r .name)
MARKETPLACE_CODE_ID=2607

MSG=$(cat <<EOF
{
	"operators": [],
	"trading_fee_bps": 200,
	"ask_expiry": {
		"min": 86400,
		"max": 15552000
	},
	"bid_expiry": {
		"min": 86400,
		"max": 15552000
	},
	"max_finders_fee_bps": 1000,
	"min_price": "5",
    "stale_bid_duration": {
            "time": 1000
    },
	"bid_removal_reward_bps": 500,
	"listing_fee": "0"
}
EOF
)
echo $MSG


starsd tx wasm instantiate $MARKETPLACE_CODE_ID "$MSG" --label "marketplace" \
  --no-admin --gas-prices 0.025ustars --gas 500000 --gas-adjustment 1.9 \
  --from $KEY -y -b block -o json | jq .
