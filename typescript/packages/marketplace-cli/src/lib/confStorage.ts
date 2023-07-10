import { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing'
import Conf from 'conf'
import _ from 'lodash'

export type StringMap = Record<string, string>

const config = new Conf<string | StringMap | undefined>()

export const STORAGE_MAP = {
  MNEMONICS: 'MNEMONICS',
  DEFAULT_MNEMONIC: 'DEFAULT_MNEMONIC',
  NODE: 'NODE',
}

type StorageKeys = keyof typeof STORAGE_MAP

class Storage {
  get = (key: string) => {
    return config.get(key)
  }

  set = (key: string, value: string) => {
    return config.set(key, value)
  }

  getMap = (key: string): StringMap => {
    const map = config.get(key, {})
    if (!_.isObject(map)) {
      throw new Error('Invalid call of _getMap')
    }
    return map as StringMap
  }

  getMnemonic = (name: string) => {
    const mnemonicMap = this.getMap(STORAGE_MAP.MNEMONICS)
    return mnemonicMap[name]
  }

  setMnemonic = (name: string, mnemonic: string) => {
    const mnemonicMap = this.getMap(STORAGE_MAP.MNEMONICS)
    if (mnemonicMap[name]) {
      throw new Error(`name ${name} already in use`)
    }
    mnemonicMap[name] = mnemonic
    config.set(STORAGE_MAP.MNEMONICS, mnemonicMap)

    if (!config.get(STORAGE_MAP.DEFAULT_MNEMONIC)) {
      config.set(STORAGE_MAP.DEFAULT_MNEMONIC, name)
    }
  }
}

const storage = new Storage()

export default storage
