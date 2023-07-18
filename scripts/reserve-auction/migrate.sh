RESERVE_AUCTION_ADDRESS=stars1pl77jpe2rrmtppv5tvs7d0g6xjq6smqxd879snksf2jwmuvxl6qs0jtvyt
# MSG=$(cat <<EOF
# {
#   "fair_burn": "stars1mp4dg9mst3hxn5xvcd9zllyx6gguu5jsp5tyt9nsfrtghhwj2akqudhls8"
# }

# EOF
# )

# starsd tx wasm migrate $RESERVE_AUCTION_ADDRESS 2355 "$MSG" \
#   --from testnet-1 --gas-prices 0.1ustars --gas-adjustment 1.7 --gas auto \
#   -b block -o json | jq .

MSG=$(cat <<EOF
{
  "config": {}
}

EOF
)

starsd query wasm contract-state smart $RESERVE_AUCTION_ADDRESS "$MSG" -o json | jq .