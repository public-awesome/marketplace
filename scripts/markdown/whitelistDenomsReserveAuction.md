# Whitelist additional denoms for use in Live Auctions

## Description

This proposal seeks to introduce additional denominations for use in Stargaze's live auctions. By incorporating these alternative assets we aim to enhance the adoption and usability of the auction marketplace.

## Key details

### Whitelisted denominations

**ATOM**

- denom: ibc/9DF365E2C0EF4EA02FA771F638BB9C0C830EFCD354629BDC017F79B348B4E989
- origin chain id: cosmoshub-4
- origin denom: uatom
- trace: transfer/channel-239
- minimum auction reserve price: 0.005 uatom

**stATOM**

- denom: ibc/FED316EA6AA1F52581F61D5D4B38F2A09042D5EA1DABA07B8A23C1EE3C0C4651
- origin chain id: stride-1
- origin denom: stuatom
- trace: transfer/channel-106
- minimum auction reserve price: 0.005 stuatom

**stSTARS**

- denom: ibc/7A58490427EF0092E2BFFB4BEEBA38E29B09E9B98557DFC78335B43F15CF2676
- origin chain id: stride-1
- origin denom: stustars
- trace: transfer/channel-106
- minimum auction reserve price: 1 stustars

**wETH.axl**

- denom: ibc/08CF01F857C36D3C91C3427AA2EACFAFC07971E7AC40B6C433A9982B333F2567
- origin chain id: axelar-dojo-1
- origin denom: weth-wei
- trace: transfer/channel-50
- minimum auction reserve price: 0.00001 weth-wei

**Noble USDC**

- denom: ibc/4A1C18CA7F50544760CF306189B810CE4C1CB156C7FC870143D401FE7280E591
- origin chain id: noble-1
- origin denom: uusdc
- trace: transfer/channel-204
- minimum auction reserve price: 0.05 uusdc

### Fair Burn

These alternative denoms will be subject to the fair burn fee. For now the fair burn fee amount accrued on these assets will be managed by the Liquidity DAO.

### Auction minimums

Each whitelisted denom will have a minimum auction reserve price. This is the minimum amount that an auction can be created for. The minimum auction reserve price is set to ensure that the auction is worth the time and effort of the platform.
