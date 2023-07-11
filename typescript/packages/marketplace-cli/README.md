# Stargaze Marketplace CLI

## Getting Started

Prerequisites

- yarn v1.22
- node v16

Instructions

1. Run `yarn`
2. To see which commands are available run `./bin/dev help` or `./bin/dev <command> help`
3. To run commands run `./bin/dev <command> <args>`

## Notes

- Command scripts can be found in ...
- Commands with many args (like init commands) take in config files that can be found in ...

## Uploading content to nft.storage

```sh-session
yarn run ipfs-car -- --pack ./tmp/images --output ./tmp/images.car
./bin/dev nft-storage migrate-metadata <CID>
yarn run ipfs-car -- --pack ./tmp/metadata --output ./tmp/metadata.car
```
