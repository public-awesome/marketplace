import { CliUx } from '@oclif/core'
import {
  CosmWasmClient,
  SigningCosmWasmClient,
} from '@cosmjs/cosmwasm-stargate'
import { Coin, DirectSecp256k1HdWallet } from '@cosmjs/proto-signing'
import { GasPrice } from '@cosmjs/stargate'
import storage, { STORAGE_MAP, StringMap } from '../confStorage'
import { isValidHttpUrl } from '../helpers/httpUrl'
import { fromRawAmount } from './unitConversions'

export const getCosmWasmClient = async () => {
  const uri = storage.get(STORAGE_MAP.NODE) as string
  if (!isValidHttpUrl(uri)) {
    throw new Error('Invalid RPC endpoint')
  }
  return await CosmWasmClient.connect(uri)
}

export const getSigningCosmWasmClient = async (accountName?: string) => {
  if (!accountName) {
    accountName = storage.get(STORAGE_MAP.DEFAULT_MNEMONIC) as string
  }
  const mnemonicMap = storage.get(STORAGE_MAP.MNEMONICS) as StringMap
  const mnemonic = mnemonicMap[accountName]
  const uri = storage.get(STORAGE_MAP.NODE) as string
  const wallet = await DirectSecp256k1HdWallet.fromMnemonic(mnemonic, {
    prefix: 'stars',
  })

  if (!isValidHttpUrl(uri)) {
    throw new Error('Invalid RPC endpoint')
  }
  const [firstAccount] = await wallet.getAccounts()
  const gasPrice = GasPrice.fromString(
    storage.get(STORAGE_MAP.GAS_PRICE) as string,
  )
  const client = await SigningCosmWasmClient.connectWithSigner(uri, wallet, {
    gasPrice,
  })

  return {
    address: firstAccount.address,
    client,
  }
}

export const initCliClient = async (
  logger: any,
): Promise<{
  address: string
  client: SigningCosmWasmClient
  balance: Coin
}> => {
  const queryClient = await getCosmWasmClient()
  const network = await queryClient.getChainId()

  const { address, client } = await getSigningCosmWasmClient()
  const DENOM = 'ustars'

  const balance = await client.getBalance(address, DENOM)

  logger(`
Client
--------
network: ${network}
address: ${address}
balance: ${fromRawAmount(balance.amount)}${balance.denom}
`)

  return { address, client, balance }
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
