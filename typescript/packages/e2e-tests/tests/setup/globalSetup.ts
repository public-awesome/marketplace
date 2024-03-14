import { pollConnection } from '../utils/sleep'
import Context from './context'

const main = async () => {
  console.log('\nRunning global setup')
  await pollConnection()
  const context = new Context()
  await context.initialize(false)
}

export default main
