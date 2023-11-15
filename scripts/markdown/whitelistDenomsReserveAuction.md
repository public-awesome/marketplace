# Whitelist Atom and stStars for use in Live Auctions

## Description

This proposal seeks to introduce ATOM and stStars as accepted denominations for minimum reserve prices in Stargaze's live auctions. By incorporating these two prominent assets, we aim to enhance the adoption and usability of the auction marketplace.

## Key Details

### Fair Burn

These alternative denoms will be subject to the fair burn fee. For now the fair burn fee amount accrued on these assets will be deposited into the fair burn pool.

### Auction Minimums

The minimum amount for an ATOM auction will be .1 ATOM. The minimum amount for an stStars auction will be 1 stStars.

### IBC Channels

**ATOM**

starsd q ibc-transfer denom-trace ibc/9DF365E2C0EF4EA02FA771F638BB9C0C830EFCD354629BDC017F79B348B4E989

- {"denom_trace":{"path":"transfer/channel-239","base_denom":"uatom"}}

**stStars**

starsd q ibc-transfer denom-trace ibc/55967CD055E19BF374A2556456C5760DAFDCF1D86DD85FAD08DBA806964DB2C4

- {"denom_trace":{"path":"transfer/channel-106","base_denom":"ustrd"}}
