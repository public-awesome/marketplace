import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate'
import { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing'
import { GasPrice } from '@cosmjs/stargate'
import chainConfig from '../../configs/chain_config.json'
import testAccounts from '../../configs/test_accounts.json'
import { isValidHttpUrl } from '../utils/url'
import fs from 'fs'
import _ from 'lodash'
import path from 'path'

class Context {
  codeIds: { [key: string]: number } = {}
  contracts: { [key: string]: string[] } = {}

  getSigningCosmWasmClient = async (testAccountIdx: number = 0) => {
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
    const client = await SigningCosmWasmClient.connectWithSigner(
      grpc_endpoint,
      wallet,
      { gasPrice },
    )

    return { address, client, gasPrice }
  }

  uploadContracts = async (client: SigningCosmWasmClient, sender: string) => {
    let fileNames = fs.readdirSync(chainConfig.artifacts_path)
    let wasmFileNames = _.filter(fileNames, (fileName) =>
      _.endsWith(fileName, '.wasm'),
    )

    for (const idx in wasmFileNames) {
      let wasmFileName = wasmFileNames[idx]
      let wasmFilePath = path.join(chainConfig.artifacts_path, wasmFileName)
      let wasmFile = fs.readFileSync(wasmFilePath, { encoding: null })
      let uploadResult = await client.upload(sender, wasmFile, 'auto')
      let codeIdKey = wasmFileName.replace('-aarch64', '').replace('.wasm', '')
      this.codeIds[codeIdKey] = uploadResult.codeId
      console.log(
        `Uploaded ${codeIdKey} contract with codeId ${uploadResult.codeId}`,
      )
    }
  }
}

export default new Context()
