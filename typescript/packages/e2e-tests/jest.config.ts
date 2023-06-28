import type { Config } from 'jest'

const config: Config = {
  preset: 'ts-jest',
  testEnvironment: 'node',
  silent: false,
  globalSetup: '<rootDir>/tests/setup/globalSetup.ts',
  testTimeout: 60 * 1000,
}

export default config
