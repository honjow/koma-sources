#!/usr/bin/env bash
# build.sh — Build all sources into .koma packages and generate index.json
# Usage: ./build.sh [--source <name>]
# Requires: cargo (with wasm32-unknown-unknown), zip, jq, koma-source-dev (in PATH or $DEV_RUNNER)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SOURCES_DIR="${KOMA_SOURCES_SRC:-$(dirname "$SCRIPT_DIR")/Koma/tools/sources}"
OUTPUT_DIR="$SCRIPT_DIR/dist"
DEV_RUNNER="${DEV_RUNNER:-$(dirname "$SCRIPT_DIR")/Koma/tools/koma-source-dev/target/release/koma-source-dev}"

# Source registry: directory name → wasm crate name
declare -A SOURCE_MAP=(
  ["baozimh"]="first-real-source"
  ["mangadex"]="mangadex"
)

# Optional: nsfw flags not in source_info
declare -A NSFW_MAP=(
  ["baozimh"]="false"
  ["mangadex"]="true"
)

REPO_URL="${KOMA_REPO_URL:-https://github.com/honjow/koma-sources}"

ONLY_SOURCE=""
VERSION_TAG=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --source) ONLY_SOURCE="$2"; shift 2 ;;
    --tag) VERSION_TAG="$2"; shift 2 ;;
    *) shift ;;
  esac
done

log() { echo "$@" >&2; }

build_source() {
  local name="$1"
  local crate_dir="${SOURCE_MAP[$name]}"
  local src_path="$SOURCES_DIR/$crate_dir"
  
  if [[ ! -d "$src_path" ]]; then
    log "ERROR: source directory not found: $src_path"
    return 1
  fi

  log "▸ Building $name ($src_path)..."
  
  # Compile to wasm
  (cd "$src_path" && cargo build --release --target wasm32-unknown-unknown 2>&1 | tail -1) >&2
  
  # Find the wasm file
  local wasm_file
  wasm_file=$(find "$src_path/target/wasm32-unknown-unknown/release" -name "*.wasm" -not -name "*.d" | head -1)
  if [[ -z "$wasm_file" ]]; then
    log "ERROR: no wasm output found for $name"
    return 1
  fi

  # Extract source info via dev runner
  local info
  info=$("$DEV_RUNNER" info "$wasm_file" 2>/dev/null)
  
  local src_id src_name src_version src_lang src_author src_desc src_content_rating
  src_id=$(echo "$info" | jq -r '.data.sourceInfo.id')
  src_name=$(echo "$info" | jq -r '.data.sourceInfo.name')
  src_version=$(echo "$info" | jq -r '.data.sourceInfo.version')
  src_lang=$(echo "$info" | jq -r '.data.sourceInfo.language')
  src_author=$(echo "$info" | jq -r '.data.sourceInfo.author')
  src_desc=$(echo "$info" | jq -r '.data.sourceInfo.description')
  src_content_rating=$(echo "$info" | jq -r '.data.sourceInfo.contentRating')
  
  local nsfw="${NSFW_MAP[$name]:-false}"
  
  # Create manifest.json
  local pkg_dir="$OUTPUT_DIR/pkg/$name"
  mkdir -p "$pkg_dir"
  
  jq -n \
    --arg id "$src_id" \
    --arg name "$src_name" \
    --arg version "$src_version" \
    --arg lang "$src_lang" \
    --argjson nsfw "$nsfw" \
    --arg author "$src_author" \
    --arg description "$src_desc" \
    --arg contentRating "$src_content_rating" \
    --arg minAppVersion "0.1.0" \
    '{id: $id, name: $name, version: $version, lang: $lang, nsfw: $nsfw, author: $author, description: $description, contentRating: $contentRating, minAppVersion: $minAppVersion}' \
    > "$pkg_dir/manifest.json"

  # Copy wasm
  cp "$wasm_file" "$pkg_dir/source.wasm"
  
  # Copy icon if exists
  if [[ -f "$src_path/icon.png" ]]; then
    cp "$src_path/icon.png" "$pkg_dir/icon.png"
  fi

  # Package as .koma (zip)
  local koma_file="$OUTPUT_DIR/sources/$name/${name}-${src_version}.koma"
  mkdir -p "$(dirname "$koma_file")"
  rm -f "$koma_file"
  (cd "$pkg_dir" && zip -q "$koma_file" manifest.json source.wasm icon.png 2>/dev/null || \
   cd "$pkg_dir" && zip -q "$koma_file" manifest.json source.wasm)
  
  local size
  size=$(stat -c%s "$koma_file" 2>/dev/null || stat -f%z "$koma_file")
  log "  ✓ ${name}-${src_version}.koma ($(numfmt --to=iec "$size" 2>/dev/null || echo "${size}B"))"
  
  # Copy icon to dist
  local icon_path=""
  if [[ -f "$pkg_dir/icon.png" ]]; then
    icon_path="sources/$name/icon.png"
    cp "$pkg_dir/icon.png" "$OUTPUT_DIR/sources/$name/icon.png"
  fi
  
  # Determine pkg download URL
  local pkg_filename="${name}-${src_version}.koma"
  local pkg_url
  if [[ -n "$VERSION_TAG" ]]; then
    pkg_url="${REPO_URL}/releases/download/${VERSION_TAG}/${pkg_filename}"
  else
    pkg_url="${pkg_filename}"
  fi

  # Output index entry JSON to stdout (only line on stdout)
  jq -n \
    --arg id "$src_id" \
    --arg name "$src_name" \
    --arg version "$src_version" \
    --arg lang "$src_lang" \
    --argjson nsfw "$nsfw" \
    --arg author "$src_author" \
    --arg description "$src_desc" \
    --arg contentRating "$src_content_rating" \
    --arg pkg "$pkg_url" \
    --arg icon "$icon_path" \
    --arg minAppVersion "0.1.0" \
    '{id: $id, name: $name, version: $version, lang: $lang, nsfw: $nsfw, author: $author, description: $description, contentRating: $contentRating, pkg: $pkg, icon: $icon, minAppVersion: $minAppVersion}'
}

# Main
rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR/sources"

INDEX_ENTRIES=()

for name in $(echo "${!SOURCE_MAP[@]}" | tr ' ' '\n' | sort); do
  if [[ -n "$ONLY_SOURCE" && "$name" != "$ONLY_SOURCE" ]]; then
    continue
  fi
  json=$(build_source "$name")
  INDEX_ENTRIES+=("$json")
done

# Generate index.json
log ""
log "▸ Generating index.json..."
printf '%s\n' "${INDEX_ENTRIES[@]}" | jq -s '.' > "$OUTPUT_DIR/index.json"
count=$(jq 'length' "$OUTPUT_DIR/index.json")
log "  ✓ index.json ($count sources)"

# Clean up pkg staging
rm -rf "$OUTPUT_DIR/pkg"

log ""
log "Done. Output in $OUTPUT_DIR/"
