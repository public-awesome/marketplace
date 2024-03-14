import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate'
import chainConfig from '../../configs/chain_config.json'
import testAccounts from '../../configs/test_accounts.json'
import { getSigningClient } from '../utils/client'
import { InstantiateMsg as RoyaltyRegistryInstantiateMsg } from '@stargazezone/core-types/lib/RoyaltyRegistry.types'
import { InstantiateMsg as VendingFactoryInstantiateMsg } from '@stargazezone/launchpad/src/VendingFactory.types'
import { InstantiateMsg as MarketplaceV2InstantiateMsg } from '@stargazezone/marketplace-types/lib/MarketplaceV2.types'
import { InstantiateMsg as ReserveAuctionInstantiateMsg } from '@stargazezone/marketplace-types/lib/ReserveAuction.types'
import fs from 'fs'
import _, { initial } from 'lodash'
import path from 'path'

export const CONTRACT_MAP = {
  // core artifacts
  FAIR_BURN: 'stargaze_fair_burn',
  ROYALTY_REGISTRY: 'stargaze_royalty_registry',

  // launchpad artifacts
  VENDING_MINTER: 'vending_minter',
  VENDING_FACTORY: 'vending_factory',
  SG721_BASE: 'sg721_base',

  // marketplace artifacts
  MARKETPLACE: 'sg_marketplace',
  MARKETPLACE_V2: 'stargaze_marketplace_v2',
  RESERVE_AUCTION: 'stargaze_reserve_auction',
}

export type TestUser = {
  name: string
  address: string
  client: SigningCosmWasmClient
}

export type TestUserMap = { [name: string]: TestUser }

export default class Context {
  private codeIds: { [key: string]: number } = {}
  private contracts: { [key: string]: string[] } = {}
  private testCachePath: string = path.join(__dirname, '../../tmp/test_cache.json')
  private testUserMap: TestUserMap = {}

  private initializeTestUsers = async () => {
    for (let i = 0; i < testAccounts.length; i++) {
      const mnemonic = testAccounts[i].mnemonic
      const signingClient = await getSigningClient(mnemonic)
      const testAccount = testAccounts[i]
      this.testUserMap[testAccount.name] = {
        name: testAccount.name,
        address: testAccounts[i].address,
        client: signingClient.client,
      }
    }
  }

  private hydrateContext = async () => {
    let testCache = JSON.parse(fs.readFileSync(this.testCachePath, 'utf8'))
    this.codeIds = testCache.codeIds
    this.contracts = testCache.contracts
  }

  private uploadContracts = async () => {
    let { client, address: sender } = this.getTestUser('user1')

    let fileNames = fs.readdirSync(chainConfig.artifacts_path)
    let wasmFileNames = _.filter(fileNames, (fileName) => _.endsWith(fileName, '.wasm'))

    for (const idx in wasmFileNames) {
      let wasmFileName = wasmFileNames[idx]
      let wasmFilePath = path.join(chainConfig.artifacts_path, wasmFileName)
      let wasmFile = fs.readFileSync(wasmFilePath, { encoding: null })
      let uploadResult = await client.upload(sender, wasmFile, 'auto')
      let codeIdKey = wasmFileName.replace('-aarch64', '').replace('.wasm', '')
      this.codeIds[codeIdKey] = uploadResult.codeId
      console.log(`Uploaded ${codeIdKey} contract with codeId ${uploadResult.codeId}`)
    }
  }

  private instantiateContract = async (
    client: SigningCosmWasmClient,
    sender: string,
    contractKey: string,
    msg: any,
  ) => {
    let instantiateResult = await client.instantiate(sender, this.codeIds[contractKey], msg, contractKey, 'auto')
    this.addContractAddress(contractKey, instantiateResult.contractAddress)
    console.log(`Instantiated ${contractKey} contract with address ${instantiateResult.contractAddress}`)
    return instantiateResult
  }

  private instantiateContracts = async () => {
    let { client, address: sender } = this.getTestUser('user1')

    // Instantiate stargaze_fair_burn
    let instantiateFairBurnResult = await this.instantiateContract(client, sender, CONTRACT_MAP.FAIR_BURN, {
      fee_bps: 5000,
    })

    // Instantiate stargaze_royalty_registry
    let royaltyRegistryInstantiateMsg: RoyaltyRegistryInstantiateMsg = {
      config: {
        max_share_delta: '0.10',
        update_wait_period: 12,
      },
    }
    let instantiateRoyaltyRegistryResult = await this.instantiateContract(
      client,
      sender,
      CONTRACT_MAP.ROYALTY_REGISTRY,
      royaltyRegistryInstantiateMsg,
    )

    // Instantiate vending_factory
    let vendingFactoryInstantiateMsg: VendingFactoryInstantiateMsg = {
      params: {
        allowed_sg721_code_ids: [this.codeIds[CONTRACT_MAP.SG721_BASE]],
        code_id: this.codeIds[CONTRACT_MAP.VENDING_MINTER],
        creation_fee: { amount: '1000000', denom: 'ustars' },
        frozen: false,
        max_trading_offset_secs: 60 * 60,
        min_mint_price: { amount: '1000000', denom: 'ustars' },
        mint_fee_bps: 200,
        extension: {
          airdrop_mint_fee_bps: 200,
          airdrop_mint_price: { amount: '1000000', denom: 'ustars' },
          max_per_address_limit: 10_000,
          max_token_limit: 10_000,
          shuffle_fee: { amount: '1000000', denom: 'ustars' },
        },
      },
    }
    await this.instantiateContract(client, sender, CONTRACT_MAP.VENDING_FACTORY, vendingFactoryInstantiateMsg)

    // Instantiate stargaze_reserve_auction
    let reserveAuctionInstantiateMsg: ReserveAuctionInstantiateMsg = {
      create_auction_fee: { amount: '500000', denom: 'ustars' },
      extend_duration: 5,
      fair_burn: instantiateFairBurnResult.contractAddress,
      max_auctions_to_settle_per_block: 100,
      max_duration: 600,
      min_bid_increment_percent: '0.01',
      min_duration: 1,
      min_reserve_prices: [{ amount: '1000000', denom: 'ustars' }],
      trading_fee_percent: '0.02',
      halt_duration_threshold: 30,
      halt_buffer_duration: 45,
      halt_postpone_duration: 60,
    }
    await this.instantiateContract(client, sender, CONTRACT_MAP.RESERVE_AUCTION, reserveAuctionInstantiateMsg)

    // Instantiate stargaze_marketplace_v2
    let marketplaceV2InstantiateMsg: MarketplaceV2InstantiateMsg = {
      price_ranges: [[chainConfig.denom, { min: '1000000', max: '1000000000' }]],
      sudo_params: {
        fair_burn: instantiateFairBurnResult.contractAddress,
        royalty_registry: instantiateRoyaltyRegistryResult.contractAddress,
        listing_fee: { amount: '1000000', denom: 'ustars' },
        trading_fee_percent: '0.02',
        max_finders_fee_percent: '0.8',
        max_royalty_fee_percent: '0.1',
        min_ask_removal_reward: { amount: '2000000', denom: 'ustars' },
        min_offer_removal_reward: { amount: '1000000', denom: 'ustars' },
        min_expiration_seconds: 1,
        order_removal_lookahead_secs: 7,
        max_asks_removed_per_block: 40,
        max_offers_removed_per_block: 20,
        max_collection_offers_removed_per_block: 30,
      },
    }
    await this.instantiateContract(client, sender, CONTRACT_MAP.MARKETPLACE_V2, marketplaceV2InstantiateMsg)
  }

  private writeContext = () => {
    const dir = path.dirname(this.testCachePath)

    if (!fs.existsSync(dir)) {
      fs.mkdirSync(dir, { recursive: true })
    }

    fs.writeFileSync(
      this.testCachePath,
      JSON.stringify({
        codeIds: this.codeIds,
        contracts: this.contracts,
      }),
    )
  }

  initialize = async (hydrate: boolean) => {
    await this.initializeTestUsers()

    if (hydrate) {
      await this.hydrateContext()
    } else {
      await this.uploadContracts()
      await this.instantiateContracts()
      this.writeContext()
    }
  }

  getTestUser = (userName: string) => {
    return this.testUserMap[userName]
  }

  getCodeId = (codeIdKey: string) => {
    return this.codeIds[codeIdKey]
  }

  getContractAddress = (contractKey: string, index: number = 0) => {
    try {
      return this.contracts[contractKey][index]
    } catch {
      console.log(`error ${contractKey} ${index} ${JSON.stringify(this.contracts)}}`)
    }
    return this.contracts[contractKey][index]
  }

  addContractAddress = (contractKey: string, contractAddress: string) => {
    this.contracts[contractKey] = _.extend([], this.contracts[contractKey], [contractAddress])
  }
}
