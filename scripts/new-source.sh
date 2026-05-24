#!/usr/bin/env bash
# new-source.sh — Scaffold a new Koma source
# Usage: ./scripts/new-source.sh <name> "<Display Name>" <lang>
# Example: ./scripts/new-source.sh nhentai "NHentai" en
set -euo pipefail

if [[ $# -lt 3 ]]; then
  echo "Usage: $0 <name> <display-name> <lang>"
  echo "Example: $0 nhentai \"NHentai\" en"
  exit 1
fi

NAME="$1"
DISPLAY_NAME="$2"
LANG="$3"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
SOURCE_DIR="$ROOT_DIR/sources/$NAME"
CRATE_NAME="koma_${NAME}_source"

if [[ -d "$SOURCE_DIR" ]]; then
  echo "ERROR: sources/$NAME already exists" >&2
  exit 1
fi

echo "▸ Creating sources/$NAME..."
cp -r "$ROOT_DIR/template" "$SOURCE_DIR"

# Update Cargo.toml
sed -i "s/koma_example_source/$CRATE_NAME/" "$SOURCE_DIR/Cargo.toml"

# Update lib.rs constants
sed -i "s|com.example.koma|com.${NAME}.koma|" "$SOURCE_DIR/src/lib.rs"
sed -i "s|Example Source|${DISPLAY_NAME}|" "$SOURCE_DIR/src/lib.rs"
sed -i "s|An example source template.|${DISPLAY_NAME} source for Koma.|" "$SOURCE_DIR/src/lib.rs"
sed -i "s|language: \"en\"|language: \"${LANG}\"|" "$SOURCE_DIR/src/lib.rs"
sed -i "s|example source init|${NAME} source init|" "$SOURCE_DIR/src/lib.rs"

# Add to workspace
if ! grep -q "\"sources/$NAME\"" "$ROOT_DIR/Cargo.toml"; then
  sed -i "/\"sources\/mangadex\"/a\\  \"sources/$NAME\"," "$ROOT_DIR/Cargo.toml"
fi

# Add to build.sh SOURCE_MAP
if ! grep -q "\"$NAME\"" "$ROOT_DIR/build.sh"; then
  sed -i "/\[\"mangadex\"\]=\"mangadex\"/a\\  [\"$NAME\"]=\"$NAME\"" "$ROOT_DIR/build.sh"
fi

echo "✓ Created sources/$NAME (crate: $CRATE_NAME)"
echo ""
echo "Next steps:"
echo "  1. Edit sources/$NAME/src/lib.rs — implement your operations"
echo "  2. cargo build --release --target wasm32-unknown-unknown -p $CRATE_NAME"
echo "  3. ./target/release/koma-source-dev test-all target/wasm32-unknown-unknown/release/${CRATE_NAME}.wasm"
