import Context, { CONTRACT_MAP } from '../setup/context'
import { TestUserMap, getClient, initializeTestUsers } from '../utils/client'
import { approveNft, createMinter, mintNft } from '../utils/nft'
import { sleep } from '../utils/sleep'
import { contracts } from '@stargazezone/marketplace-types'
import { ReserveAuctionQueryClient as ReserveAuctionQueryClientType } from '@stargazezone/marketplace-types/lib/ReserveAuction.client'
import { Auction, Config, HighBid } from '@stargazezone/marketplace-types/src/ReserveAuction.types'
import _ from 'lodash'

const { ReserveAuctionClient, ReserveAuctionQueryClient } = contracts.ReserveAuction

describe('Reserve Auctions', () => {
  const creatorName = 'user1'
  const sellerName = 'user2'
  const sellerAssetRecipientName = 'user3'
  const firstBuyerName = 'user4'
  const secondBuyerName = 'user5'

  let context: Context
  let collectionAddress: string
  let reserveAuctionAddress: string
  let reserveAuctionQueryClient: ReserveAuctionQueryClientType
  let config: Config
  let testUsers: TestUserMap

  beforeAll(async () => {
    context = new Context()
    await context.hydrateContext()
    await createMinter(context)

    reserveAuctionAddress = context.getContractAddress(CONTRACT_MAP.RESERVE_AUCTION)

    let queryClient = await getClient()
    reserveAuctionQueryClient = new ReserveAuctionQueryClient(queryClient, reserveAuctionAddress)
    config = await reserveAuctionQueryClient.config()

    testUsers = await initializeTestUsers()
  })

  test('auction lifecycle', async () => {
    const creator = testUsers[creatorName]
    const seller = testUsers[sellerName]
    const sellerAssetRecipient = testUsers[sellerAssetRecipientName]
    const firstBuyer = testUsers[firstBuyerName]
    const secondBuyer = testUsers[secondBuyerName]

    const sellerReserveAuctionClient = new ReserveAuctionClient(seller.client, seller.address, reserveAuctionAddress)
    const firstBuyerReserveAuctionClient = new ReserveAuctionClient(
      firstBuyer.client,
      firstBuyer.address,
      reserveAuctionAddress,
    )
    const secondBuyerReserveAuctionClient = new ReserveAuctionClient(
      secondBuyer.client,
      secondBuyer.address,
      reserveAuctionAddress,
    )

    let [collectionAddress, tokenId] = await mintNft(context, creator.client, creator.address, seller.address)
    await approveNft(seller.client, seller.address, collectionAddress, tokenId, reserveAuctionAddress)

    let reservePrice = { amount: '100000000', denom: 'ustars' }
    let duration = 20
    await sellerReserveAuctionClient.createAuction(
      {
        collection: collectionAddress,
        duration,
        reservePrice,
        sellerFundsRecipient: sellerAssetRecipient.address,
        tokenId,
      },
      'auto',
      'create-auction',
      [config.create_auction_fee],
    )

    let auction = (await reserveAuctionQueryClient.auction({ collection: collectionAddress, tokenId })) as Auction
    expect(auction).toBeDefined()
    expect(auction.collection).toEqual(collectionAddress)
    expect(auction.token_id).toEqual(tokenId)
    expect(auction.seller_funds_recipient).toEqual(sellerAssetRecipient.address)
    expect(auction.reserve_price).toEqual(reservePrice)
    expect(auction.duration).toEqual(duration)
    expect(auction.high_bid).toBeNull()

    await firstBuyerReserveAuctionClient.placeBid(
      {
        collection: collectionAddress,
        tokenId,
      },
      'auto',
      'place-bid',
      [reservePrice],
    )

    auction = (await reserveAuctionQueryClient.auction({ collection: collectionAddress, tokenId })) as Auction
    let high_bid = auction.high_bid as HighBid
    expect(high_bid).toBeDefined()
    expect(high_bid.bidder).toEqual(firstBuyer.address)
    expect(high_bid.coin).toEqual(reservePrice)

    try {
      await secondBuyerReserveAuctionClient.cancelAuction({
        collection: collectionAddress,
        tokenId,
      })
      expect(true).toBeFalsy()
    } catch (e) {
      expect(e).toBeDefined()
    }

    let bidAmount = (parseInt(high_bid.coin.amount, 10) * (1 + parseFloat(config.min_bid_increment_percent))).toString()
    let bidCoin = { amount: bidAmount, denom: high_bid.coin.denom }
    await secondBuyerReserveAuctionClient.placeBid(
      {
        collection: collectionAddress,
        tokenId,
      },
      'auto',
      'place-bid',
      [bidCoin],
    )

    auction = (await reserveAuctionQueryClient.auction({ collection: collectionAddress, tokenId })) as Auction
    expect(auction?.high_bid?.bidder).toEqual(secondBuyer.address)
    expect(auction?.high_bid?.coin).toEqual(bidCoin)

    let auctionEndMs = Math.ceil(parseInt(auction.end_time as string, 10) / 1000000)
    let nowMs = Math.ceil(new Date().getTime())
    let sleepMs = auctionEndMs - nowMs + 2000
    if (sleepMs > 0) {
      await sleep(sleepMs)
    }

    await sellerReserveAuctionClient.settleAuction(
      {
        collection: collectionAddress,
        tokenId,
      },
      'auto',
      'settle-auction',
    )

    auction = (await reserveAuctionQueryClient.auction({ collection: collectionAddress, tokenId })) as Auction
    expect(auction).toBeNull()
  })
})
