#!/bin/bash

start_dir=$(pwd)

echo "Generating schema for marketplace contract..."

rm -rf schema

for contract_path in contracts/*; do
  if [ -d "$contract_path" ]; then
    cd "$contract_path"
    filename="$(basename "$contract_path")"
    
    # Mapping old contract names to new schema directory names
    case $filename in
      "marketplace")
        schema_dir="sg-marketplace"
        ;;
      "reserve-auction")
        schema_dir="stargaze-reserve-auction"
        ;;
      "stargaze-marketplace-v2")
        schema_dir="stargaze-marketplace-v2"
        ;;
      *)
        # Default to using the filename as is for any contract not explicitly mapped
        schema_dir=$filename
        ;;
    esac

    cargo run --bin schema --release
    rm -rf schema/raw
    mkdir -p "$start_dir/schema/$schema_dir"
    mv schema/*.json "$start_dir/schema/$schema_dir/$schema_dir.json"
    cd "$start_dir"
  fi
done
