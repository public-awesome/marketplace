import { Command, Flags } from '@oclif/core'
import { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing'
import storage, { STORAGE_MAP } from '../../lib/confStorage'
import { getCosmWasmClient } from '../../lib/cosmos/cosmWasmClient'
import { fromRawAmount } from '../../lib/cosmos/unitConversions'
import _ from 'lodash'

export default class KeysList extends Command {
  static description = 'list all the keypairs that have been created'

  static examples = ['<%= config.bin %> <%= command.id %>']

  run = async () => {
    const { args, flags } = await this.parse(KeysList)
    const DENOM = 'ustars'

    const mnemonics = storage.getMap(STORAGE_MAP.MNEMONICS)

    const client = await getCosmWasmClient()

    const addresses = await Promise.all(
      _.map(mnemonics, async (mnemonic, key) => {
        const wallet = await DirectSecp256k1HdWallet.fromMnemonic(mnemonic, {
          prefix: 'stars',
        })
        const [firstAccount] = await wallet.getAccounts()
        const balance = await client.getBalance(firstAccount.address, DENOM)
        return `${key}: ${firstAccount.address} ${fromRawAmount(
          balance.amount,
        )}${balance.denom}`
      }),
    )

    this.log(`Found ${addresses.length} accounts`)
    this.log(addresses.join('\n'))
  }
}
