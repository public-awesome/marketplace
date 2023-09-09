import { Command, Flags } from '@oclif/core'
import { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing'
import storage from '../../lib/confStorage'

export default class KeysGenerate extends Command {
  static description =
    'generates keypairs that are compatible with the stargaze network'

  static examples = ['<%= config.bin %> <%= command.id %>']

  static flags = {
    name: Flags.string({
      char: 'n',
      description: 'namespace at which the key will be stored',
      required: true,
    }),
  }

  static args = []

  public async run(): Promise<void> {
    const { args, flags } = await this.parse(KeysGenerate)
    const wallet = await DirectSecp256k1HdWallet.generate(24, {
      prefix: 'stars',
    })
    storage.setMnemonic(flags.name, wallet.mnemonic)
    const [firstAccount] = await wallet.getAccounts()
    this.log(`Generated account: ${flags.name} ${firstAccount.address}`)
  }
}
