set -eux

CONTRACT_ADDRESS=stars1vvdkcn393ddyd47v9g3qv6mvne59d0ykzy9wre3ga0c58dtdg4ksm776jg

MSG=$(cat <<EOF
{
  "set_min_reserve_prices": {
    "min_reserve_prices": [
        {
            "denom": "ibc/9DF365E2C0EF4EA02FA771F638BB9C0C830EFCD354629BDC017F79B348B4E989",
            "amount": "100000"
        },
        {
            "denom": "ibc/55967CD055E19BF374A2556456C5760DAFDCF1D86DD85FAD08DBA806964DB2C4",
            "amount": "1000000"
        }
    ]
  }
}
EOF
)

TITLE="Whitelist ATOM and stStars for use in Live Auctions" 
MARKDOWN="scripts/markdown/whitelistDenomsReserveAuction.md"
DESCRIPTION=$(cat "$MARKDOWN" | base64 | tr -d '\n')

FROM="hot-wallet"
DEPOSIT="10000000000ustars"

CHAIN_ID="stargaze-1"
NODE="https://rpc.stargaze-apis.com:443"

starsd tx gov submit-proposal sudo-contract "$CONTRACT_ADDRESS" "$MSG" \
 --title "$TITLE" \
 --description "$(echo "$DESCRIPTION" | base64 --decode)" \
 --from "$FROM" \
 --deposit "$DEPOSIT" \
 --chain-id "$CHAIN_ID" \
 --node "$NODE" \
 --gas-prices 1ustars \
 --gas auto \
 --gas-adjustment 1.5 \
 --dry-run