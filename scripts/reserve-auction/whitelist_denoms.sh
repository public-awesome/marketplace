set -eux

CONTRACT_ADDRESS=stars1vvdkcn393ddyd47v9g3qv6mvne59d0ykzy9wre3ga0c58dtdg4ksm776jg

MSG=$(cat <<EOF
{
  "set_min_reserve_prices": {
    "min_reserve_prices": [
        {
            "denom": "ibc/9DF365E2C0EF4EA02FA771F638BB9C0C830EFCD354629BDC017F79B348B4E989",
            "amount": "5000"
        },
        {
            "denom": "ibc/FED316EA6AA1F52581F61D5D4B38F2A09042D5EA1DABA07B8A23C1EE3C0C4651",
            "amount": "5000"
        },
        {
            "denom": "ibc/7A58490427EF0092E2BFFB4BEEBA38E29B09E9B98557DFC78335B43F15CF2676",
            "amount": "1000000"
        },
        {
            "denom": "ibc/08CF01F857C36D3C91C3427AA2EACFAFC07971E7AC40B6C433A9982B333F2567",
            "amount": "50000000000000"
        },
        {
            "denom": "ibc/4A1C18CA7F50544760CF306189B810CE4C1CB156C7FC870143D401FE7280E591",
            "amount": "50000"
        }
    ]
  }
}
EOF
)

TITLE="Live Auctions: Whitelist Denoms" 
MARKDOWN="scripts/markdown/whitelistDenomsReserveAuction.md"
DESCRIPTION=$(cat "$MARKDOWN" | base64 | tr -d '\n')

FROM="stars19mmkdpvem2xvrddt8nukf5kfpjwfslrsu7ugt5"
KEYRING_BACKEND="test"
DEPOSIT="10000000000ustars"

CHAIN_ID="stargaze-1"
NODE="https://rpc.stargaze-apis.com:443"

starsd tx gov submit-proposal sudo-contract "$CONTRACT_ADDRESS" "$MSG" \
 --title "$TITLE" \
 --description "$(echo "$DESCRIPTION" | base64 --decode)" \
 --from "$FROM" \
 --keyring-backend "$KEYRING_BACKEND" \
 --deposit "$DEPOSIT" \
 --chain-id "$CHAIN_ID" \
 --node "$NODE" \
 --gas-prices 1ustars \
 --gas auto \
 --gas-adjustment 1.5