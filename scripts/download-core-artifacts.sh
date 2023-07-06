#!/bin/bash

# This script is used to download launchpad wasm files from github
# and place them in the proper location for the integration tests.

contracts=(
    "stargaze_fair_burn"
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
    url=https://github.com/public-awesome/core/releases/download/${contract}-v${version}/${contract}.wasm
    
    if [ -z "$version" ]; then
        echo "Version not found for $contract"
        exit 1
    fi

    echo "Downloading $url"
    curl -L --output "./artifacts/${contract}.wasm" "${url}"
done
