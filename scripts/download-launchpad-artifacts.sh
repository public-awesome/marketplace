#!/bin/bash

# This script is used to download launchpad wasm files from github
# and place them in the proper location for the integration tests.

contracts=(
    "base_factory"
    "base_minter"
    "sg721_base"
)

# Check if Cargo.toml file exists
if [ ! -f Cargo.toml ]; then
    echo "Cargo.toml file not found!"
    exit 1
fi

for contract in "${contracts[@]}"; do
    contract_tmp=$(echo $contract | tr '_' '-')

    # If file exists then skip
    if [ -f "./artifacts/${contract}.wasm" ]; then
        echo "Skipping ${contract}.wasm"
        continue
    fi

    version=$(grep "^$contract_tmp" Cargo.toml | awk -F'"' '{print $2}')
    
    if [ -z "$version" ]; then
        echo "Version not found for $contract"
        exit 1
    fi

    echo "Downloading https://github.com/public-awesome/launchpad/releases/download/v${version}/${contract}.wasm"
    curl -L --output "./artifacts/${contract}.wasm" "https://github.com/public-awesome/launchpad/releases/download/v${version}/${contract}.wasm"
done
