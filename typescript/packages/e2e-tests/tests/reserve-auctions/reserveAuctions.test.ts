import Context, { CONTRACT_MAP } from '../setup/context'
import { getClient, getSigningClient } from '../utils/client'
import { approveNft, createMinter, mintNft } from '../utils/nft'
import { contracts } from '@stargazezone/marketplace-types'
import { ReserveAuctionQueryClient as ReserveAuctionQueryClientType } from '@stargazezone/marketplace-types/build/src/ReserveAuction.client'

const { ReserveAuctionClient, ReserveAuctionQueryClient } = contracts.ReserveAuction

describe('ReserveAuctions', () => {
  const creator = 0
  const seller = 1
  const sellerAssetRecipient = 2
  const buyer = 3
  const buyerAssetRecipient = 4

  let context: Context
  let collectionAddress: string
  let tokenId: string
  let reserveAuctionAddress: string
  let reserveAuctionQueryClient: ReserveAuctionQueryClientType

  beforeAll(async () => {
    context = new Context()
    await context.hydrateContext()
    await createMinter(context)

    let { address: recipient } = await getSigningClient(seller)
    ;[collectionAddress, tokenId] = await mintNft(context, recipient)

    reserveAuctionAddress = context.getContractAddress(CONTRACT_MAP.RESERVE_AUCTION)
    await approveNft(context, seller, collectionAddress, tokenId, reserveAuctionAddress)

    let queryClient = await getClient()
    reserveAuctionQueryClient = new ReserveAuctionQueryClient(queryClient, reserveAuctionAddress)
  })

  describe('Auctions', () => {
    test('create auction', async () => {
      let { client, address: sender } = await getSigningClient(seller)
      let reserveAuctionClient = new ReserveAuctionClient(client, sender, reserveAuctionAddress)

      let { address: assetRecipient } = await getSigningClient(sellerAssetRecipient)
      let config = await reserveAuctionQueryClient.config()

      let price = { amount: '100000000', denom: 'ustars' }
      let duration = 600
      await reserveAuctionClient.createAuction(
        {
          collection: collectionAddress,
          duration,
          reservePrice: price,
          sellerFundsRecipient: assetRecipient,
          tokenId,
        },
        'auto',
        'create-auction',
        [config.create_auction_fee],
      )

      let auction = await reserveAuctionQueryClient.auction({ collection: collectionAddress, tokenId })
      expect(auction?.collection).toEqual(collectionAddress)
      expect(auction?.token_id).toEqual(tokenId)
      expect(auction?.seller_funds_recipient).toEqual(assetRecipient)
      expect(auction?.reserve_price.amount).toEqual(price.amount)
      expect(auction?.reserve_price.denom).toEqual(price.denom)
      expect(auction?.duration).toEqual(duration)
    })
  })
})
