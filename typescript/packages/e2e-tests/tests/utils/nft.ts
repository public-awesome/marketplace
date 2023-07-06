import Context, { CONTRACT_MAP } from '../setup/context'
import { getClient, getSigningClient } from './client'
import { getExpirationString } from './datetime'
import { ExecuteMsg as BaseFactoryExecuteMsg } from '@stargazezone/launchpad/src/BaseFactory.types'
import assert from 'assert'
import _ from 'lodash'

export const createMinter = async (context: Context) => {
  const queryClient = await getClient()

  let baseFactoryAddress = context.getContractAddress(CONTRACT_MAP.BASE_FACTORY)
  let { params: factoryParams } = await queryClient.queryContractSmart(baseFactoryAddress, {
    params: {},
  })

  const { client: signingClient, address: sender } = await getSigningClient()
  let msg: BaseFactoryExecuteMsg = {
    create_minter: {
      collection_params: {
        code_id: context.getCodeId(CONTRACT_MAP.SG721_BASE),
        name: 'Test Collection',
        symbol: 'TC',
        info: {
          creator: sender,
          description: 'This is the collection description',
          image: 'ipfs://bafybeiek33kk3js27dhodwadtmrf3p6b64netr6t3xzi3sbfyxovxe36qe/1.png',
          start_trading_time: getExpirationString(0),
          royalty_info: {
            payment_address: sender,
            share: '0.05',
          },
        },
      },
    },
  }
  let executeResult = await signingClient.execute(sender, baseFactoryAddress, msg, 'auto', 'instantiate-base-minter', [
    factoryParams.creation_fee,
  ])

  let instantiateEvents = _.filter(executeResult.events, (event) => {
    return event.type === 'instantiate'
  })
  context.pushContractAddress(CONTRACT_MAP.BASE_MINTER, instantiateEvents[0].attributes[0].value)
  context.pushContractAddress(CONTRACT_MAP.SG721_BASE, instantiateEvents[1].attributes[0].value)

  return executeResult
}

export const mintNft = async (context: Context, recipientAddress: string): Promise<[string, string]> => {
  const queryClient = await getClient()

  let baseFactoryAddress = context.getContractAddress(CONTRACT_MAP.BASE_FACTORY)
  let { params: factoryParams } = await queryClient.queryContractSmart(baseFactoryAddress, {
    params: {},
  })

  let baseMinterAddress = context.getContractAddress(CONTRACT_MAP.BASE_MINTER)
  let minterConfig = await queryClient.queryContractSmart(baseMinterAddress, {
    config: {},
  })
  let collectionAddress = minterConfig.collection_address

  const { client: signingClient, address: sender } = await getSigningClient()
  let mintMsg = { mint: { token_uri: 'ipfs://bafybeiek33kk3js27dhodwadtmrf3p6b64netr6t3xzi3sbfyxovxe36qe/1.png' } }

  let mintPrice = (factoryParams.mint_fee_bps * factoryParams.min_mint_price.amount) / 10000
  let mintExecuteResult = await signingClient.execute(sender, baseMinterAddress, mintMsg, 'auto', 'mint-nft', [
    { amount: mintPrice.toString(), denom: factoryParams.min_mint_price.denom },
  ])

  let tokenId: string = ''
  for (const idx in mintExecuteResult.events) {
    {
      const event = mintExecuteResult.events[idx]
      let tokenIdAttribute = _.find(event.attributes, (attribute) => attribute.key === 'token_id')
      if (tokenIdAttribute) {
        tokenId = tokenIdAttribute.value
        break
      }
    }
  }
  assert(tokenId, 'token_id not found in wasm event attributes')

  // Transfer NFT to recipient
  let transferMsg = { transfer_nft: { recipient: recipientAddress, token_id: tokenId } }
  let transferExecuteResult = await signingClient.execute(
    sender,
    collectionAddress,
    transferMsg,
    'auto',
    'transfer-nft',
  )

  return [collectionAddress, tokenId]
}

export const approveNft = async (
  context: Context,
  clientIdx: number,
  collectionAddress: string,
  tokenId: string,
  approveAddress: string,
) => {
  const { client: signingClient, address: sender } = await getSigningClient(clientIdx)

  let msg = { approve: { spender: approveAddress, token_id: tokenId } }
  let executeResult = await signingClient.execute(sender, collectionAddress, msg, 'auto', 'approve-nft')

  return executeResult
}
