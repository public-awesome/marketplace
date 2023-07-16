import { Command, Flags } from '@oclif/core'
import {
  getCosmWasmClient,
  getSigningCosmWasmClient,
} from '../../lib/cosmos/cosmWasmClient'
import { InvalidInput } from '../../lib/errors'
import { ISOToSeconds, isISODate } from '../../lib/helpers/date'
import { contracts } from '@stargazezone/marketplace-types'
import _ from 'lodash'

const { ReserveAuctionClient, ReserveAuctionQueryClient } =
  contracts.ReserveAuction

export default class ReserveAuctionScheduleEndAuction extends Command {
  static description = 'schedule auctions to end at a certain time'

  static examples = ['<%= config.bin %> <%= command.id %>']

  static args = [
    {
      name: 'contract-address',
      description: 'address of the minter',
      required: true,
    },
    {
      name: 'collection-address',
      description: 'address of the NFT collection',
      required: true,
    },
    {
      name: 'end-time',
      description: 'the time to end the auction',
      required: true,
    },
    {
      name: 'creator',
      description: 'the name of the creator account',
      required: true,
    },
    {
      name: 'bidder',
      description: 'the name of the bidder account',
      required: true,
    },
  ]

  static flags = {
    'token-ids': Flags.string({
      char: 't',
      description: 'the token ids of the NFT',
      required: true,
      multiple: true,
    }),
    'dry-run': Flags.boolean({
      char: 'd',
      description: 'do not execute txs',
    }),
  }

  public async run(): Promise<void> {
    const { args, flags } = await this.parse(ReserveAuctionScheduleEndAuction)
    const logger = this.log.bind(this)

    // Validate and set args
    if (!isISODate(args['end-time'])) {
      throw new InvalidInput('end-time must be a valid ISO date')
    }
    const tokenIds = flags['token-ids']
    const endTime = args['end-time']
    const reservePrice = { amount: '1000000', denom: 'ustars' }

    // Fetch and log reserve auction contract config
    const queryClient = await getCosmWasmClient()
    const reserveAuctionQueryClient = new ReserveAuctionQueryClient(
      queryClient,
      args['contract-address'],
    )
    const config = await reserveAuctionQueryClient.config()
    logger(`Reserve Auction Config: ${JSON.stringify(config, null, 2)}`)

    // Fetch the creator's tokens
    let { address: creator, client: creatorClient } =
      await getSigningCosmWasmClient(args['creator'])
    let tokens = await queryClient.queryContractSmart(
      args['collection-address'],
      {
        tokens: { owner: creator },
      },
    )
    logger(`Creator's Tokens: ${JSON.stringify(tokens, null, 2)}`)

    // Fetch the owner of
    let ownerOfResponse = await queryClient.queryContractSmart(
      args['collection-address'],
      {
        owner_of: { token_id: tokenIds[0] },
      },
    )
    logger(`Owner Of Response: ${JSON.stringify(ownerOfResponse, null, 2)}`)

    // Fetch the creator's auctions
    const auctions = await reserveAuctionQueryClient.auctionsBySeller({
      seller: creator,
    })
    logger(`Creator's Auctions: ${JSON.stringify(auctions, null, 2)}`)

    const auctionEndTimes = auctions.forEach((auction) => {
      if (auction.end_time) {
        let endDatetime = new Date(parseInt(auction.end_time, 10) / 1000000)
        console.log(
          `Auction ${auction.token_id} ends at ${endDatetime.toISOString()}`,
        )
      }
    })

    if (flags['dry-run']) {
      return
    }

    for (const tokenId of tokenIds) {
      let auctionExists = _.some(
        auctions,
        (auction) => auction.token_id === tokenId,
      )
      if (auctionExists) {
        logger(`Auction for ${tokenId} already exists...`)
        continue
      }

      // Approve the contract to transfer the NFTs
      let approveResult = await creatorClient.execute(
        creator,
        args['collection-address'],
        {
          approve: { spender: args['contract-address'], token_id: tokenId },
        },
        'auto',
      )
      logger(`Approve ${tokenId} success...`)

      let creatorReserveAuctionClient = new ReserveAuctionClient(
        creatorClient,
        creator,
        args['contract-address'],
      )

      const now = new Date()
      const duration = ISOToSeconds(endTime) - ISOToSeconds(now.toISOString())

      let createAuctionResult = await creatorReserveAuctionClient.createAuction(
        {
          collection: args['collection-address'],
          tokenId,
          duration,
          reservePrice,
        },
        'auto',
        'create-auction',
        [config.create_auction_fee],
      )
      console.log('Create Auction Success...')

      let { address: bidder, client: bidderClient } =
        await getSigningCosmWasmClient(args['bidder'])

      let bidderReserveAuctionClient = new ReserveAuctionClient(
        bidderClient,
        bidder,
        args['contract-address'],
      )

      let placeBidResult = await bidderReserveAuctionClient.placeBid(
        {
          collection: args['collection-address'],
          tokenId,
        },
        'auto',
        'place-bid',
        [reservePrice],
      )
      console.log('Place Bid Success...')
    }
  }
}
