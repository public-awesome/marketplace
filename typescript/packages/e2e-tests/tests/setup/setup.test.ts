import chainConfig from '../../configs/chain_config.json'
import { pollConnection } from '../utils/sleep'
import context from './context'
import axios from 'axios'
import _ from 'lodash'

const SETUP_TIMEOUT_SECS = 60

beforeAll(async () => {
  await pollConnection()
  let { client, address } = await context.getSigningCosmWasmClient(0)
  await context.uploadContracts(client, address)
}, SETUP_TIMEOUT_SECS * 1000)

describe('connection', () => {
  test(
    'connection established',
    async () => {
      let result = await axios.get(chainConfig.grpc_endpoint)
      expect(result.status).toBe(200)
    },
    SETUP_TIMEOUT_SECS * 1000,
  )
})
