import { CliUx } from '@oclif/core'
import {
  CosmWasmClient,
  SigningCosmWasmClient,
} from '@cosmjs/cosmwasm-stargate'
import { Coin, DirectSecp256k1HdWallet } from '@cosmjs/proto-signing'
import { GasPrice } from '@cosmjs/stargate'
import storage, { STORAGE_MAP, StringMap } from '../confStorage'
import { isValidHttpUrl } from '../types/httpUrl'
import { fromRawAmount } from './unitConversions'

export const getChainConfig = async () => {}

export const getCosmWasmClient = async () => {
  const uri = storage.get(STORAGE_MAP.NODE) as string
  if (!isValidHttpUrl(uri)) {
    throw new Error('Invalid RPC endpoint')
  }
  return await CosmWasmClient.connect(uri)
}

export const getGasPrice = () => {
  return GasPrice.fromString(`"0.0025ustars`)
}

export const getSigningCosmWasmClient = async (prefix = 'stars') => {
  const defaultMnemonic = storage.get(STORAGE_MAP.DEFAULT_MNEMONIC) as string
  const mnemonicMap = storage.get(STORAGE_MAP.MNEMONICS) as StringMap
  const mnemonic = mnemonicMap[defaultMnemonic]
  const uri = storage.get(STORAGE_MAP.NODE) as string
  const wallet = await DirectSecp256k1HdWallet.fromMnemonic(mnemonic, {
    prefix,
  })

  if (!isValidHttpUrl(uri)) {
    throw new Error('Invalid RPC endpoint')
  }
  const [firstAccount] = await wallet.getAccounts()
  const gasPrice = getGasPrice()
  const client = await SigningCosmWasmClient.connectWithSigner(uri, wallet, {
    gasPrice,
  })

  return {
    address: firstAccount.address,
    client,
    gasPrice,
  }
}

export const initCliClient = async (logger: any) => {
  const initialResponse = await getSigningCosmWasmClient()
  const network = await initialResponse.client.getChainId()

  const { address, client, gasPrice } = await getSigningCosmWasmClient()
  const DENOM = 'ustars'

  const balance = await client.getBalance(address, DENOM)

  logger(`
Client
--------
network: ${network}
address: ${address}
balance: ${fromRawAmount(balance.amount)}${balance.denom}
gasPrice: ${gasPrice}
`)

  return { address, client, balance, gasPrice }
}

const printMessage = (logger: any, msg: any) => {
  let isEncoded = false
  try {
    msg[0].typeUrl
    isEncoded = true
  } catch (e: unknown) {}
  if (isEncoded) {
    logger(msg)
  } else {
    logger(`${JSON.stringify(msg, null, 2)}\n`)
  }
}

export const getConfirmation = (logger: any, msg: any, cost?: Coin) => {
  logger('Please confirm the settings for your transaction')
  logger('--------------------------------------------------')
  printMessage(logger, msg)
  if (cost) {
    logger('\nCOST')
    logger('--------------------------------------------------')
    logger(`${JSON.stringify(cost, null, 2)}\n`)
  }
  return CliUx.ux.confirm('Ready to submit the transaction? (y/n)')
}
