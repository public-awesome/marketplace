import Context, { CONTRACT_MAP } from '../setup/context'
import { getClient, getSigningClient } from '../utils/client'
import { approveNft, createMinter, mintNft } from '../utils/nft'
import { contracts } from '@stargazezone/marketplace-types'
import type { MarketplaceQueryClient as MarketplaceQueryClientType } from '@stargazezone/marketplace-types/build/src/Marketplace.client'

const { MarketplaceClient, MarketplaceQueryClient } = contracts.Marketplace

describe('MarketplaceV3', () => {
  const creator = 0
  const seller = 1
  const sellerAssetRecipient = 2
  const buyer = 3
  const buyerAssetRecipient = 4

  let context: Context
  let collectionAddress: string
  let tokenId: string
  let marketplaceAddress: string
  let marketplaceQueryClient: MarketplaceQueryClientType

  beforeAll(async () => {
    context = new Context()
    await context.hydrateContext()
    await createMinter(context)

    let { address: recipient } = await getSigningClient(seller)
    ;[collectionAddress, tokenId] = await mintNft(context, recipient)

    marketplaceAddress = context.getContractAddress(CONTRACT_MAP.MARKETPLACE)
    await approveNft(context, seller, collectionAddress, tokenId, marketplaceAddress)

    let queryClient = await getClient()
    marketplaceQueryClient = new MarketplaceQueryClient(queryClient, marketplaceAddress)
  })

  describe('Asks', () => {
    test('set ask', async () => {
      let { client, address: sender } = await getSigningClient(seller)
      let marketplaceClient = new MarketplaceClient(client, sender, marketplaceAddress)

      let { address: assetRecipient } = await getSigningClient(sellerAssetRecipient)
      let sudoParams = await marketplaceQueryClient.sudoParams()

      let price = { amount: '100000000', denom: 'ustars' }
      await marketplaceClient.setAsk(
        {
          collection: collectionAddress,
          tokenId,
          assetRecipient,
          price,
          findersFeeBps: 300,
        },
        'auto',
        'set-ask',
        [sudoParams.listing_fee],
      )

      let ask = await marketplaceQueryClient.ask({ collection: collectionAddress, tokenId })
      expect(ask?.collection).toEqual(collectionAddress)
      expect(ask?.token_id).toEqual(tokenId)
      expect(ask?.asset_recipient).toEqual(assetRecipient)
      expect(ask?.price.amount).toEqual(price.amount)
      expect(ask?.price.denom).toEqual(price.denom)
    })

    test('update ask price', async () => {
      let { client, address: sender } = await getSigningClient(seller)
      let marketplaceClient = new MarketplaceClient(client, sender, marketplaceAddress)

      let price = { amount: '200000000', denom: 'ustars' }
      await marketplaceClient.updateAskPrice(
        {
          collection: collectionAddress,
          tokenId,
          price,
        },
        'auto',
        'update-ask-price',
        [],
      )

      let ask = await marketplaceQueryClient.ask({ collection: collectionAddress, tokenId })
      expect(ask?.collection).toEqual(collectionAddress)
      expect(ask?.token_id).toEqual(tokenId)
      expect(ask?.price.amount).toEqual(price.amount)
      expect(ask?.price.denom).toEqual(price.denom)
    })

    test('remove ask', async () => {
      let { client, address: sender } = await getSigningClient(seller)
      let marketplaceClient = new MarketplaceClient(client, sender, marketplaceAddress)

      await marketplaceClient.removeAsk(
        {
          collection: collectionAddress,
          tokenId,
        },
        'auto',
        'remove-ask',
        [],
      )

      let ask = await marketplaceQueryClient.ask({ collection: collectionAddress, tokenId })
      expect(ask).toBeNull()
    })
  })

  describe('Offers', () => {
    test('set offer', async () => {
      let { client, address: sender } = await getSigningClient(buyer)
      let marketplaceClient = new MarketplaceClient(client, sender, marketplaceAddress)

      let { address: assetRecipient } = await getSigningClient(buyerAssetRecipient)

      let price = { amount: '100000000', denom: 'ustars' }
      await marketplaceClient.setOffer(
        {
          collection: collectionAddress,
          tokenId,
          assetRecipient,
          findersFeeBps: 200,
        },
        'auto',
        'set-offer',
        [price],
      )

      let offer = await marketplaceQueryClient.offer({ collection: collectionAddress, tokenId, bidder: sender })
      expect(offer?.collection).toEqual(collectionAddress)
      expect(offer?.token_id).toEqual(tokenId)
      expect(offer?.asset_recipient).toEqual(assetRecipient)
      expect(offer?.price.amount).toEqual(price.amount)
      expect(offer?.price.denom).toEqual(price.denom)
    })

    test('remove offer', async () => {
      let { client, address: sender } = await getSigningClient(buyer)
      let marketplaceClient = new MarketplaceClient(client, sender, marketplaceAddress)

      await marketplaceClient.removeOffer(
        {
          collection: collectionAddress,
          tokenId,
        },
        'auto',
        'remove-offer',
      )

      let offer = await marketplaceQueryClient.offer({ collection: collectionAddress, tokenId, bidder: sender })
      expect(offer).toBeNull()
    })
  })

  describe('Collection Offers', () => {
    test('set collection offer', async () => {
      let { client, address: sender } = await getSigningClient(buyer)
      let marketplaceClient = new MarketplaceClient(client, sender, marketplaceAddress)

      let { address: assetRecipient } = await getSigningClient(buyerAssetRecipient)

      let price = { amount: '100000000', denom: 'ustars' }
      await marketplaceClient.setCollectionOffer(
        {
          collection: collectionAddress,
          assetRecipient,
          findersFeeBps: 200,
        },
        'auto',
        'set-collection-offer',
        [price],
      )

      let collectionOffer = await marketplaceQueryClient.collectionOffer({
        collection: collectionAddress,
        bidder: sender,
      })
      expect(collectionOffer?.collection).toEqual(collectionAddress)
      expect(collectionOffer?.asset_recipient).toEqual(assetRecipient)
      expect(collectionOffer?.price.amount).toEqual(price.amount)
      expect(collectionOffer?.price.denom).toEqual(price.denom)
    })

    test('remove collection offer', async () => {
      let { client, address: sender } = await getSigningClient(buyer)
      let marketplaceClient = new MarketplaceClient(client, sender, marketplaceAddress)

      await marketplaceClient.removeCollectionOffer(
        {
          collection: collectionAddress,
        },
        'auto',
        'remove-collection-offer',
      )

      let collectionOffer = await marketplaceQueryClient.collectionOffer({
        collection: collectionAddress,
        bidder: sender,
      })
      expect(collectionOffer).toBeNull()
    })
  })
})
