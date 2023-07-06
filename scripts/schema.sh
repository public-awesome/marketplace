START_DIR=$(pwd)

echo "Generating schema for marketplace contract..."

for d in contracts/*; do
  if [ -d "$d" ]; then
    cd $d
    cargo schema
    rm -rf schema/raw
    cd "$START_DIR"
  fi
done