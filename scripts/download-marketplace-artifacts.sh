#!/bin/bash

# This script is used to download launchpad wasm files from github
# and place them in the proper location for the integration tests.

echo "Downloading marketplace v1.4"

# If file exists then skip
if [ -f "./artifacts/sg_marketplace_v1.wasm" ]; then
    echo "Skipping sg_marketplace_v1.wasm"
    exit 0
fi

curl -L --output "./artifacts/sg_marketplace_v1.wasm" "https://github.com/public-awesome/marketplace/releases/download/v1.4.0/sg_marketplace.wasm"
    
