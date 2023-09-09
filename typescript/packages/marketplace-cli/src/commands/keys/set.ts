import { Command, Flags } from '@oclif/core'
import { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing'
import storage, { STORAGE_MAP } from '../../lib/confStorage'

export default class KeysSet extends Command {
  static description =
    'set the keypair that is to be used in subsequent commands'

  static examples = ['<%= config.bin %> <%= command.id %>']

  static flags = {
    name: Flags.string({
      char: 'n',
      description: 'namespace of the key that is being set',
      required: true,
    }),
  }

  static args = []

  public async run(): Promise<void> {
    const { args, flags } = await this.parse(KeysSet)

    const mnemonicMap = storage.getMap(STORAGE_MAP.MNEMONICS)
    if (!mnemonicMap[flags.name]) {
      throw new Error(`Namespace not found: ${flags.name}`)
    }

    storage.set(STORAGE_MAP.DEFAULT_MNEMONIC, flags.name)

    const wallet = await DirectSecp256k1HdWallet.fromMnemonic(
      mnemonicMap[flags.name],
      { prefix: 'stars' },
    )
    const [firstAccount] = await wallet.getAccounts()
    this.log(`Default account set to: ${firstAccount.address}`)
  }
}
