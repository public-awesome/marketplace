import { pollConnection } from '../utils/sleep'
import Context from './context'

const main = async () => {
  console.log('\nRunning global setup')
  await pollConnection()
  const context = new Context()
  await context.uploadContracts()
  await context.instantiateContracts()
  context.writeContext()
}

export default main
