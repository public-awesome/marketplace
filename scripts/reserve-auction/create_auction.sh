RESERVE_AUCTION_ADDRESS=stars1pl77jpe2rrmtppv5tvs7d0g6xjq6smqxd879snksf2jwmuvxl6qs0jtvyt

# # ------------------------------
# # Approve NFT
# # ------------------------------
MSG=$(cat <<EOF
{
  "approve": {
    "spender": "stars1pl77jpe2rrmtppv5tvs7d0g6xjq6smqxd879snksf2jwmuvxl6qs0jtvyt",
    "token_id": "4840",
    "expires": {
      "at_time": "2685586146961000000"
    }
  }
}

EOF
)

starsd tx wasm execute stars10hm2p3ll26zkzwmm202mfdmqy0x0qaxjtqcu6y9cl45razea84hs62p5zn "$MSG" \
  --from testnet-1 --gas-prices 0.1ustars --gas-adjustment 1.7 --gas auto \
  -b async -y -o json | jq .

# # ------------------------------
# # Approve NFT
# # ------------------------------
# MSG=$(cat <<EOF
# {
#   "approve": {
#     "spender": "stars1pl77jpe2rrmtppv5tvs7d0g6xjq6smqxd879snksf2jwmuvxl6qs0jtvyt",
#     "token_id": "4995",
#     "expires": {
#       "at_time": "2685586146961000000"
#     }
#   }
# }

# EOF
# )

# starsd tx wasm execute stars10hm2p3ll26zkzwmm202mfdmqy0x0qaxjtqcu6y9cl45razea84hs62p5zn "$MSG" \
#   --from testnet-2 --gas-prices 0.1ustars --gas-adjustment 1.7 --gas auto --sequence 24 \
#   -b async -y -o json | jq .

# # ------------------------------
# # Approve NFT
# # ------------------------------
# MSG=$(cat <<EOF
# {
#   "approve": {
#     "spender": "stars1pl77jpe2rrmtppv5tvs7d0g6xjq6smqxd879snksf2jwmuvxl6qs0jtvyt",
#     "token_id": "5023",
#     "expires": {
#       "at_time": "2685586146961000000"
#     }
#   }
# }

# EOF
# )

# starsd tx wasm execute stars10hm2p3ll26zkzwmm202mfdmqy0x0qaxjtqcu6y9cl45razea84hs62p5zn "$MSG" \
#   --from testnet-2 --gas-prices 0.1ustars --gas-adjustment 1.7 --gas auto --sequence 25 \
#   -b async -y -o json | jq .

# # ------------------------------
# # Approve NFT
# # ------------------------------
# MSG=$(cat <<EOF
# {
#   "approve": {
#     "spender": "stars1pl77jpe2rrmtppv5tvs7d0g6xjq6smqxd879snksf2jwmuvxl6qs0jtvyt",
#     "token_id": "5066",
#     "expires": {
#       "at_time": "2685586146961000000"
#     }
#   }
# }

# EOF
# )

# starsd tx wasm execute stars10hm2p3ll26zkzwmm202mfdmqy0x0qaxjtqcu6y9cl45razea84hs62p5zn "$MSG" \
#   --from testnet-2 --gas-prices 0.1ustars --gas-adjustment 1.7 --gas auto --sequence 26 \
#   -b async -y -o json | jq .





# # ------------------------------
# # Create Auction
# # ------------------------------
# MSG=$(cat <<EOF
# {
#   "create_auction": {
#     "collection": "stars10hm2p3ll26zkzwmm202mfdmqy0x0qaxjtqcu6y9cl45razea84hs62p5zn",
#     "duration": 60,
#     "reserve_price": {
#       "amount": "100000000",
#       "denom": "ustars"
#     },
#     "token_id": "4840"
#   }
# }

# EOF
# )

# starsd tx wasm execute stars1pl77jpe2rrmtppv5tvs7d0g6xjq6smqxd879snksf2jwmuvxl6qs0jtvyt "$MSG" --amount 50ustars \
#   --from testnet-2 --gas-prices 0.1ustars --gas-adjustment 1.7 --gas auto --sequence 27 \
#   -b async -y -o json | jq .

# # ------------------------------
# # Create Auction
# # ------------------------------
# MSG=$(cat <<EOF
# {
#   "create_auction": {
#     "collection": "stars10hm2p3ll26zkzwmm202mfdmqy0x0qaxjtqcu6y9cl45razea84hs62p5zn",
#     "duration": 60,
#     "reserve_price": {
#       "amount": "100000000",
#       "denom": "ustars"
#     },
#     "token_id": "4995"
#   }
# }

# EOF
# )

# starsd tx wasm execute stars1pl77jpe2rrmtppv5tvs7d0g6xjq6smqxd879snksf2jwmuvxl6qs0jtvyt "$MSG" --amount 50ustars \
#   --from testnet-2 --gas-prices 0.1ustars --gas-adjustment 1.7 --gas auto --sequence 28 \
#   -b async -y -o json | jq .

# # ------------------------------
# # Create Auction
# # ------------------------------
# MSG=$(cat <<EOF
# {
#   "create_auction": {
#     "collection": "stars10hm2p3ll26zkzwmm202mfdmqy0x0qaxjtqcu6y9cl45razea84hs62p5zn",
#     "duration": 60,
#     "reserve_price": {
#       "amount": "100000000",
#       "denom": "ustars"
#     },
#     "token_id": "5023"
#   }
# }

# EOF
# )

# starsd tx wasm execute stars1pl77jpe2rrmtppv5tvs7d0g6xjq6smqxd879snksf2jwmuvxl6qs0jtvyt "$MSG" --amount 50ustars \
#   --from testnet-2 --gas-prices 0.1ustars --gas-adjustment 1.7 --gas auto --sequence 29 \
#   -b async -y -o json | jq .


# # ------------------------------
# # Create Auction
# # ------------------------------
# MSG=$(cat <<EOF
# {
#   "create_auction": {
#     "collection": "stars10hm2p3ll26zkzwmm202mfdmqy0x0qaxjtqcu6y9cl45razea84hs62p5zn",
#     "duration": 60,
#     "reserve_price": {
#       "amount": "100000000",
#       "denom": "ustars"
#     },
#     "token_id": "5066"
#   }
# }

# EOF
# )

# starsd tx wasm execute stars1pl77jpe2rrmtppv5tvs7d0g6xjq6smqxd879snksf2jwmuvxl6qs0jtvyt "$MSG" --amount 50ustars \
#   --from testnet-2 --gas-prices 0.1ustars --gas-adjustment 1.7 --gas auto --sequence 30 \
#   -b async -y -o json | jq .






# # ------------------------------
# # Place Bid
# # ------------------------------
# MSG=$(cat <<EOF
# {
#   "place_bid": {
#     "collection": "stars10hm2p3ll26zkzwmm202mfdmqy0x0qaxjtqcu6y9cl45razea84hs62p5zn",
#     "token_id": "4840"
#   }
# }

# EOF
# )

# starsd tx wasm execute stars1pl77jpe2rrmtppv5tvs7d0g6xjq6smqxd879snksf2jwmuvxl6qs0jtvyt "$MSG" --amount 100020000ustars \
#   --from testnet-1 --gas-prices 0.1ustars --gas-adjustment 1.7 --gas auto --sequence 38 \
#   -b async -y -o json | jq .

# # ------------------------------
# # Place Bid
# # ------------------------------
# MSG=$(cat <<EOF
# {
#   "place_bid": {
#     "collection": "stars10hm2p3ll26zkzwmm202mfdmqy0x0qaxjtqcu6y9cl45razea84hs62p5zn",
#     "token_id": "4995"
#   }
# }

# EOF
# )

# starsd tx wasm execute stars1pl77jpe2rrmtppv5tvs7d0g6xjq6smqxd879snksf2jwmuvxl6qs0jtvyt "$MSG" --amount 100020000ustars \
#   --from testnet-1 --gas-prices 0.1ustars --gas-adjustment 1.7 --gas auto --sequence 39 \
#   -b async -y -o json | jq .

# # ------------------------------
# # Place Bid
# # ------------------------------
# MSG=$(cat <<EOF
# {
#   "place_bid": {
#     "collection": "stars10hm2p3ll26zkzwmm202mfdmqy0x0qaxjtqcu6y9cl45razea84hs62p5zn",
#     "token_id": "5023"
#   }
# }

# EOF
# )

# starsd tx wasm execute stars1pl77jpe2rrmtppv5tvs7d0g6xjq6smqxd879snksf2jwmuvxl6qs0jtvyt "$MSG" --amount 100020000ustars \
#   --from testnet-1 --gas-prices 0.1ustars --gas-adjustment 1.7 --gas auto --sequence 40 \
#   -b async -y -o json | jq .

# # ------------------------------
# # Place Bid
# # ------------------------------
# MSG=$(cat <<EOF
# {
#   "place_bid": {
#     "collection": "stars10hm2p3ll26zkzwmm202mfdmqy0x0qaxjtqcu6y9cl45razea84hs62p5zn",
#     "token_id": "5066"
#   }
# }

# EOF
# )

# starsd tx wasm execute stars1pl77jpe2rrmtppv5tvs7d0g6xjq6smqxd879snksf2jwmuvxl6qs0jtvyt "$MSG" --amount 100020000ustars \
#   --from testnet-1 --gas-prices 0.1ustars --gas-adjustment 1.7 --gas auto --sequence 41 \
#   -b async -y -o json | jq .






# ------------------------------
# Query Auction
# ------------------------------
MSG=$(cat <<EOF
{
  "auction": {
    "collection": "stars10hm2p3ll26zkzwmm202mfdmqy0x0qaxjtqcu6y9cl45razea84hs62p5zn",
    "token_id": "4840"
  }
}

EOF
)

starsd query wasm contract-state smart $RESERVE_AUCTION_ADDRESS "$MSG" \
-o json | jq .

# ------------------------------
# Query Auction
# ------------------------------
MSG=$(cat <<EOF
{
  "auction": {
    "collection": "stars10hm2p3ll26zkzwmm202mfdmqy0x0qaxjtqcu6y9cl45razea84hs62p5zn",
    "token_id": "4995"
  }
}

EOF
)

starsd query wasm contract-state smart $RESERVE_AUCTION_ADDRESS "$MSG" \
-o json | jq .


# ------------------------------
# Query Auction
# ------------------------------
MSG=$(cat <<EOF
{
  "auction": {
    "collection": "stars10hm2p3ll26zkzwmm202mfdmqy0x0qaxjtqcu6y9cl45razea84hs62p5zn",
    "token_id": "5023"
  }
}

EOF
)

starsd query wasm contract-state smart $RESERVE_AUCTION_ADDRESS "$MSG" \
-o json | jq .


# ------------------------------
# Query Auction
# ------------------------------
MSG=$(cat <<EOF
{
  "auction": {
    "collection": "stars10hm2p3ll26zkzwmm202mfdmqy0x0qaxjtqcu6y9cl45razea84hs62p5zn",
    "token_id": "5066"
  }
}

EOF
)

starsd query wasm contract-state smart $RESERVE_AUCTION_ADDRESS "$MSG" \
-o json | jq .









# # 4995
# # 5023
# # 5066