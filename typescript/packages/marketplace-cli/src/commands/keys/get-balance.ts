import { Command, Flags } from '@oclif/core'
import { getCosmWasmClient } from '../../lib/cosmos/cosmWasmClient'
import { fromRawAmount } from '../../lib/cosmos/unitConversions'

export default class KeysGetBalance extends Command {
  static description = 'gets the balance of an address'

  static examples = ['<%= config.bin %> <%= command.id %>']

  static args = [{ name: 'address' }]

  static flags = {
    denom: Flags.string({
      char: 'd',
      description: 'the denom of the balance to fetch',
      required: false,
    }),
  }

  public async run(): Promise<void> {
    const { args, flags } = await this.parse(KeysGetBalance)

    const client = await getCosmWasmClient()
    const DENOM = 'ustars'
    const balance = await client.getBalance(args.address, DENOM)

    this.log(
      JSON.stringify(
        {
          address: args.address,
          balance: fromRawAmount(balance.amount) + balance.denom,
        },
        null,
        2,
      ),
    )
  }
}
