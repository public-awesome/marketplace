import { CosmWasmClient, SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate'
import { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing'
import { GasPrice, QueryClient } from '@cosmjs/stargate'
import chainConfig from '../../configs/chain_config.json'
import testAccounts from '../../configs/test_accounts.json'
import { getClient, getSigningClient } from '../utils/client'
import { isValidHttpUrl } from '../utils/url'
import { InstantiateMsg as BaseFactoryInstantiateMsg } from '@stargazezone/launchpad/src/BaseFactory.types'
import { InstantiateMsg as MarketplaceInstantiateMsg } from '@stargazezone/marketplace-types/src/Marketplace.types'
import { InstantiateMsg as MarketplaceV1InstantiateMsg } from '@stargazezone/marketplace-v1-types/src/Marketplace.types'
import fs from 'fs'
import _ from 'lodash'
import path from 'path'

export const CONTRACT_MAP = {
  FAIR_BURN: 'stargaze_fair_burn',
  MARKETPLACE: 'sg_marketplace',
  MARKETPLACE_V1: 'sg_marketplace_v1',
  BASE_MINTER: 'base_minter',
  BASE_FACTORY: 'base_factory',
  SG721_BASE: 'sg721_base',
}

export default class Context {
  codeIds: { [key: string]: number } = {}
  contracts: { [key: string]: string[] } = {}
  testCachePath: string = path.join(__dirname, '../../tmp/test_cache.json')

  writeContext = () => {
    fs.writeFileSync(
      this.testCachePath,
      JSON.stringify({
        codeIds: this.codeIds,
        contracts: this.contracts,
      }),
    )
  }

  hydrateContext = async () => {
    let testCache = JSON.parse(fs.readFileSync(this.testCachePath, 'utf8'))
    this.codeIds = testCache.codeIds
    this.contracts = testCache.contracts
  }

  getCodeId = (codeIdKey: string) => {
    return this.codeIds[codeIdKey]
  }

  getContractAddress = (contractKey: string, index: number = 0) => {
    return this.contracts[contractKey][index]
  }

  pushContractAddress = (contractKey: string, contractAddress: string) => {
    this.contracts[contractKey] = _.extend([], this.contracts[contractKey], [contractAddress])
  }

  uploadContracts = async () => {
    let { client, address: sender } = await getSigningClient(0)

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

  instantiateContract = async (client: SigningCosmWasmClient, sender: string, contractKey: string, msg: any) => {
    let instantiateResult = await client.instantiate(sender, this.codeIds[contractKey], msg, contractKey, 'auto')
    this.pushContractAddress(contractKey, instantiateResult.contractAddress)
    console.log(`Instantiated ${contractKey} contract with address ${instantiateResult.contractAddress}`)
    return instantiateResult
  }

  instantiateContracts = async () => {
    let { client, address: sender } = await getSigningClient(0)

    // Instantiate stargaze_fair_burn
    let instantiateFairBurnResult = await this.instantiateContract(client, sender, CONTRACT_MAP.FAIR_BURN, {
      fee_bps: 5000,
    })

    // Instantiate base_factory
    let baseFactoryInstantiateMsg: BaseFactoryInstantiateMsg = {
      params: {
        allowed_sg721_code_ids: [this.codeIds[CONTRACT_MAP.SG721_BASE]],
        code_id: this.codeIds[CONTRACT_MAP.BASE_MINTER],
        creation_fee: { amount: '1000000', denom: 'ustars' },
        frozen: false,
        max_trading_offset_secs: 60 * 60,
        min_mint_price: { amount: '1000000', denom: 'ustars' },
        mint_fee_bps: 200,
      },
    }
    await this.instantiateContract(client, sender, CONTRACT_MAP.BASE_FACTORY, baseFactoryInstantiateMsg)

    // Instantiate sg_marketplace_v1
    let marketplaceV1InstantiateMsg: MarketplaceV1InstantiateMsg = {
      ask_expiry: { min: 1, max: 60 * 60 * 24 * 7 },
      bid_expiry: { min: 1, max: 60 * 60 * 24 * 7 },
      bid_removal_reward_bps: 300,
      listing_fee: '2000000',
      max_finders_fee_bps: 500,
      min_price: '2000000',
      operators: [sender],
      stale_bid_duration: { time: 60 * 60 },
      trading_fee_bps: 600,
    }
    await this.instantiateContract(client, sender, CONTRACT_MAP.MARKETPLACE_V1, marketplaceV1InstantiateMsg)

    // Instantiate sg_marketplace
    let marketplaceInstantiateMsg: MarketplaceInstantiateMsg = {
      fair_burn: instantiateFairBurnResult.contractAddress,
      ask_expiry: { min: 1, max: 60 * 60 * 24 * 7 },
      offer_expiry: { min: 1, max: 60 * 60 * 24 * 7 },
      removal_reward_bps: 300,
      listing_fee: { amount: '2000000', denom: 'ustars' },
      max_finders_fee_bps: 500,
      operators: [sender],
      trading_fee_bps: 600,
      max_asks_removed_per_block: 200,
      max_collection_offers_removed_per_block: 200,
      max_offers_removed_per_block: 200,
      price_ranges: [['ustars', { min: '1000000', max: '1000000000000000000' }]],
    }
    await this.instantiateContract(client, sender, CONTRACT_MAP.MARKETPLACE, marketplaceInstantiateMsg)
  }
}
