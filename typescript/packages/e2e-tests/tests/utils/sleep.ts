import chainConfig from '../../configs/chain_config.json'
import axios, { AxiosResponse } from 'axios'

export const sleep = (ms: number) =>
  new Promise((resolve) => setTimeout(resolve, ms))

export const pollConnection = async () => {
  console.log('Polling connection...')

  while (true) {
    await sleep(4000)

    try {
      let result = await axios.get(chainConfig.grpc_endpoint)
      console.log('Connection established')
      break
    } catch (err) {
      console.log(`Failed to establish connection: Retrying...`)
    }
  }
}
