import { CosmWasmClient } from '@cosmjs/cosmwasm-stargate'
import Context, { CONTRACT_MAP } from '../setup/context'
import { getQueryClient } from '../utils/client'
import { getFutureTimestamp } from '../utils/datetime'
import { approveNft, createMinter, mintNft } from '../utils/nft'
import { contracts } from '@stargazezone/marketplace-types'
import { MarketplaceV2QueryClient as MarketplaceV2QueryClientType } from '@stargazezone/marketplace-types/lib/MarketplaceV2.client'
import {
  Ask,
  CollectionOffer,
  Offer,
  SudoParamsForString,
} from '@stargazezone/marketplace-types/lib/MarketplaceV2.types'
import _ from 'lodash'

const { MarketplaceV2Client, MarketplaceV2QueryClient } = contracts.MarketplaceV2

describe('MarketplaceV2', () => {
  const creatorName = 'user1'
  const askerName = 'user2'
  const sellerAssetRecipientName = 'user3'
  const buyerName = 'user4'
  const buyerAssetRecipientName = 'user5'

  let context: Context
  let collectionAddress: string
  let queryClient: CosmWasmClient
  let marketplaceV2Address: string
  let marketplaceV2QueryClient: MarketplaceV2QueryClientType
  let sudoParams: SudoParamsForString

  beforeAll(async () => {
    context = new Context()
    await context.initialize(true)
    collectionAddress = await createMinter(context)

    queryClient = await getQueryClient()
    marketplaceV2Address = context.getContractAddress(CONTRACT_MAP.MARKETPLACE_V2)
    marketplaceV2QueryClient = new MarketplaceV2QueryClient(queryClient, marketplaceV2Address)
    sudoParams = await marketplaceV2QueryClient.sudoParams()
  })

  test('ask lifecycle', async () => {
    const asker = context.getTestUser(askerName)
    const askerAssetRecipient = context.getTestUser(sellerAssetRecipientName)

    const askerMarketplaceV2Client = new MarketplaceV2Client(asker.client, asker.address, marketplaceV2Address)

    let tokenId = await mintNft(context, asker.client, asker.address, asker.address)
    await approveNft(asker.client, asker.address, collectionAddress, tokenId, marketplaceV2Address)

    let price = { amount: '100000000', denom: 'ustars' }
    await askerMarketplaceV2Client.setAsk(
      {
        collection: collectionAddress,
        tokenId,
        price,
      },
      'auto',
      'set-ask',
      [sudoParams.listing_fee],
    )

    let ask = (await marketplaceV2QueryClient.ask({ collection: collectionAddress, tokenId })) as Ask
    expect(ask).toBeDefined()
    expect(ask.collection).toEqual(collectionAddress)
    expect(ask.token_id).toEqual(tokenId)
    expect(ask.order_info.price).toEqual(ask.order_info.price)
    expect(ask.order_info.creator).toEqual(asker.address)
    expect(ask.order_info.asset_recipient).toBeNull()
    expect(ask.order_info.expiration_info).toBeNull()
    expect(ask.order_info.finders_fee_percent).toBeNull()

    let expirationInfo = {
      expiration: getFutureTimestamp(120),
      removal_reward: sudoParams.min_ask_removal_reward,
    }
    await askerMarketplaceV2Client.updateAsk(
      {
        collection: collectionAddress,
        tokenId,
        assetRecipient: { set: askerAssetRecipient.address },
        findersFeePercent: { set: '0.01' },
        expirationInfo: {
          set: expirationInfo,
        },
      },
      'auto',
      'update-ask',
      [sudoParams.min_ask_removal_reward],
    )
    ask = (await marketplaceV2QueryClient.ask({ collection: collectionAddress, tokenId })) as Ask
    expect(ask).toBeDefined()
    expect(ask.collection).toEqual(collectionAddress)
    expect(ask.token_id).toEqual(tokenId)
    expect(ask.order_info.price).toEqual(ask.order_info.price)
    expect(ask.order_info.creator).toEqual(asker.address)
    expect(ask.order_info.asset_recipient).toEqual(askerAssetRecipient.address)
    expect(ask.order_info.expiration_info).toEqual(expirationInfo)
    expect(ask.order_info.finders_fee_percent).toEqual('0.01')

    await askerMarketplaceV2Client.removeAsk(
      {
        collection: collectionAddress,
        tokenId,
      },
      'auto',
      'remove-ask',
      [],
    )
    let nullAsk = await marketplaceV2QueryClient.ask({ collection: collectionAddress, tokenId })
    expect(nullAsk).toBeNull()
  })

  test('offer lifecycle', async () => {
    const buyer = context.getTestUser(buyerName)
    const buyerAssetRecipient = context.getTestUser(buyerAssetRecipientName)

    const buyerMarketplaceV2Client = new MarketplaceV2Client(buyer.client, buyer.address, marketplaceV2Address)

    let tokenId = '2'
    let price = { amount: '100000000', denom: 'ustars' }
    await buyerMarketplaceV2Client.setOffer(
      {
        collection: collectionAddress,
        tokenId,
        price,
      },
      'auto',
      'set-offer',
      [price],
    )

    let offer = (await marketplaceV2QueryClient.offer({
      collection: collectionAddress,
      tokenId,
      creator: buyer.address,
    })) as Offer
    expect(offer).toBeDefined()
    expect(offer.collection).toEqual(collectionAddress)
    expect(offer.token_id).toEqual(tokenId)
    expect(offer.order_info.price).toEqual(offer.order_info.price)
    expect(offer.order_info.creator).toEqual(buyer.address)
    expect(offer.order_info.asset_recipient).toBeNull()
    expect(offer.order_info.expiration_info).toBeNull()
    expect(offer.order_info.finders_fee_percent).toBeNull()

    let expirationInfo = {
      expiration: getFutureTimestamp(120),
      removal_reward: sudoParams.min_offer_removal_reward,
    }
    await buyerMarketplaceV2Client.updateOffer(
      {
        collection: collectionAddress,
        tokenId,
        assetRecipient: { set: buyerAssetRecipient.address },
        findersFeePercent: { set: '0.01' },
        expirationInfo: {
          set: expirationInfo,
        },
      },
      'auto',
      'update-offer',
      [sudoParams.min_offer_removal_reward],
    )
    offer = (await marketplaceV2QueryClient.offer({
      collection: collectionAddress,
      tokenId,
      creator: buyer.address,
    })) as Offer
    expect(offer).toBeDefined()
    expect(offer.collection).toEqual(collectionAddress)
    expect(offer.token_id).toEqual(tokenId)
    expect(offer.order_info.price).toEqual(offer.order_info.price)
    expect(offer.order_info.creator).toEqual(buyer.address)
    expect(offer.order_info.asset_recipient).toEqual(buyerAssetRecipient.address)
    expect(offer.order_info.expiration_info).toEqual(expirationInfo)
    expect(offer.order_info.finders_fee_percent).toEqual('0.01')

    await buyerMarketplaceV2Client.removeOffer(
      {
        collection: collectionAddress,
        tokenId,
      },
      'auto',
      'remove-offer',
      [],
    )
    let nullOffer = await marketplaceV2QueryClient.offer({
      collection: collectionAddress,
      tokenId,
      creator: buyer.address,
    })
    expect(nullOffer).toBeNull()
  })

  test('collection offer lifecycle', async () => {
    const buyer = context.getTestUser(buyerName)
    const buyerAssetRecipient = context.getTestUser(buyerAssetRecipientName)

    const buyerMarketplaceV2Client = new MarketplaceV2Client(buyer.client, buyer.address, marketplaceV2Address)

    let price = { amount: '100000000', denom: 'ustars' }
    await buyerMarketplaceV2Client.setCollectionOffer(
      {
        collection: collectionAddress,
        price,
      },
      'auto',
      'set-collection-offer',
      [price],
    )

    let collectionOffer = (await marketplaceV2QueryClient.collectionOffer({
      collection: collectionAddress,
      creator: buyer.address,
    })) as CollectionOffer
    expect(collectionOffer).toBeDefined()
    expect(collectionOffer.collection).toEqual(collectionAddress)
    expect(collectionOffer.order_info.price).toEqual(collectionOffer.order_info.price)
    expect(collectionOffer.order_info.creator).toEqual(buyer.address)
    expect(collectionOffer.order_info.asset_recipient).toBeNull()
    expect(collectionOffer.order_info.expiration_info).toBeNull()
    expect(collectionOffer.order_info.finders_fee_percent).toBeNull()

    let expirationInfo = {
      expiration: getFutureTimestamp(120),
      removal_reward: sudoParams.min_offer_removal_reward,
    }
    await buyerMarketplaceV2Client.updateCollectionOffer(
      {
        collection: collectionAddress,
        assetRecipient: { set: buyerAssetRecipient.address },
        findersFeePercent: { set: '0.01' },
        expirationInfo: {
          set: expirationInfo,
        },
      },
      'auto',
      'update-collection-offer',
      [sudoParams.min_offer_removal_reward],
    )
    collectionOffer = (await marketplaceV2QueryClient.collectionOffer({
      collection: collectionAddress,
      creator: buyer.address,
    })) as CollectionOffer
    expect(collectionOffer).toBeDefined()
    expect(collectionOffer.collection).toEqual(collectionAddress)
    expect(collectionOffer.order_info.price).toEqual(collectionOffer.order_info.price)
    expect(collectionOffer.order_info.creator).toEqual(buyer.address)
    expect(collectionOffer.order_info.asset_recipient).toEqual(buyerAssetRecipient.address)
    expect(collectionOffer.order_info.expiration_info).toEqual(expirationInfo)
    expect(collectionOffer.order_info.finders_fee_percent).toEqual('0.01')

    await buyerMarketplaceV2Client.removeCollectionOffer(
      {
        collection: collectionAddress,
      },
      'auto',
      'remove-collection-offer',
      [],
    )
    let nullCollectionOffer = (await marketplaceV2QueryClient.collectionOffer({
      collection: collectionAddress,
      creator: buyer.address,
    })) as CollectionOffer
    expect(nullCollectionOffer).toBeNull()
  })

  test('process sale transaction', async () => {
    const asker = context.getTestUser(askerName)
    const buyer = context.getTestUser(buyerName)

    const askerMarketplaceV2Client = new MarketplaceV2Client(asker.client, asker.address, marketplaceV2Address)
    const buyerMarketplaceV2Client = new MarketplaceV2Client(buyer.client, buyer.address, marketplaceV2Address)

    let tokenId = await mintNft(context, asker.client, asker.address, asker.address)
    await approveNft(asker.client, asker.address, collectionAddress, tokenId, marketplaceV2Address)

    let price = { amount: '100000000', denom: 'ustars' }
    await askerMarketplaceV2Client.setAsk(
      {
        collection: collectionAddress,
        tokenId,
        price,
      },
      'auto',
      'set-ask',
      [sudoParams.listing_fee],
    )

    await buyerMarketplaceV2Client.acceptAsk(
      {
        collection: collectionAddress,
        tokenId,
      },
      'auto',
      'accept-ask',
      [price],
    )

    let nullAsk = await marketplaceV2QueryClient.ask({ collection: collectionAddress, tokenId })
    expect(nullAsk).toBeNull()

    let ownerOfResponse = await queryClient.queryContractSmart(collectionAddress, {
      owner_of: {
        token_id: tokenId,
      },
    })
    expect(ownerOfResponse.owner).toEqual(buyer.address)
  })
})
