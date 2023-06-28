import { CosmWasmClient, SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate'
import { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing'
import { GasPrice } from '@cosmjs/stargate'
import chainConfig from '../../configs/chain_config.json'
import testAccounts from '../../configs/test_accounts.json'
import { isValidHttpUrl } from '../utils/url'
import _ from 'lodash'

export const getClient = () => {
  return CosmWasmClient.connect(chainConfig.grpc_endpoint)
}

export const getSigningClient = async (testAccountIdx: number = 0) => {
  const { mnemonic } = testAccounts[testAccountIdx]
  const { prefix, grpc_endpoint, gas_prices, denom } = chainConfig
  const wallet = await DirectSecp256k1HdWallet.fromMnemonic(mnemonic, {
    prefix,
  })

  if (!isValidHttpUrl(grpc_endpoint)) {
    throw new Error('Invalid RPC endpoint')
  }

  const [{ address }] = await wallet.getAccounts()
  const gasPrice = GasPrice.fromString(`${gas_prices}${denom}`)
  const client = await SigningCosmWasmClient.connectWithSigner(grpc_endpoint, wallet, { gasPrice })

  return { address, client, gasPrice }
}
