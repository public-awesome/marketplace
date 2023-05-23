RESERVE_AUCTION_ELGAFAR_CODE_ID=2170
MSG=$(cat <<EOF
{
  "marketplace": "stars18cszlvm6pze0x9sz32qnjq4vtd45xehqs8dq7cwy8yhq35wfnn3qgzs5gu",
  "min_reserve_price": "100",
  "min_bid_increment_bps": 50,
  "min_duration": 10,
  "max_duration": 31536000,
  "extend_duration": 1800,
  "create_auction_fee": "50",
  "max_auctions_to_settle_per_block": 200
}

EOF
)

starsd tx wasm instantiate $RESERVE_AUCTION_ELGAFAR_CODE_ID  "$MSG"  --label "reserve-auction" --no-admin \
  --from testnet-1 --gas-prices 0.1ustars --gas-adjustment 1.7 --gas auto \
  -b block -o json | jq .