import { CosmWasmClient, SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate'
import { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing'
import { GasPrice, QueryClient } from '@cosmjs/stargate'
import chainConfig from '../../configs/chain_config.json'
import testAccounts from '../../configs/test_accounts.json'
import { getClient, getSigningClient } from '../utils/client'
import { isValidHttpUrl } from '../utils/url'
import { InstantiateMsg as BaseFactoryInstantiateMsg } from '@stargazezone/launchpad/src/BaseFactory.types'
import { InstantiateMsg as ReserveAuctionInstantiateMsg } from '@stargazezone/marketplace-types/src/ReserveAuction.types'
import fs from 'fs'
import _ from 'lodash'
import path from 'path'

export const CONTRACT_MAP = {
  FAIR_BURN: 'stargaze_fair_burn',
  MARKETPLACE: 'sg_marketplace',
  MARKETPLACE_V1: 'sg_marketplace_v1',
  RESERVE_AUCTION: 'stargaze_reserve_auction',
  BASE_MINTER: 'base_minter',
  BASE_FACTORY: 'base_factory',
  SG721_BASE: 'sg721_base',
}

export default class Context {
  codeIds: { [key: string]: number } = {}
  contracts: { [key: string]: string[] } = {}
  testCachePath: string = path.join(__dirname, '../../tmp/test_cache.json')

  writeContext = () => {
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

  hydrateContext = async () => {
    let testCache = JSON.parse(fs.readFileSync(this.testCachePath, 'utf8'))
    this.codeIds = testCache.codeIds
    this.contracts = testCache.contracts
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

    // Instantiate stargaze reserve auction
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
  }
}
