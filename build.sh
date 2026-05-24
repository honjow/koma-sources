#!/usr/bin/env bash
# build.sh — Build all sources into .koma packages and generate index.json
# Usage: ./build.sh [--source <name>] [--scaffold <name>]
# Requires: cargo (with wasm32-unknown-unknown), zip, jq, koma-source-dev (in PATH or $DEV_RUNNER)
set -euo pipefail

# Prefer rustup toolchain (system cargo may lack wasm32-unknown-unknown target)
_real_home="${REAL_HOME:-$(getent passwd "$(id -un)" | cut -d: -f6)}"
if [[ -d "$_real_home/.rustup" && -d "$_real_home/.cargo/bin" ]]; then
  export RUSTUP_HOME="$_real_home/.rustup"
  export CARGO_HOME="$_real_home/.cargo"
  export PATH="$_real_home/.cargo/bin:$PATH"
fi
unset _real_home

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SOURCES_DIR="$SCRIPT_DIR/sources"
OUTPUT_DIR="$SCRIPT_DIR/dist"
DEV_RUNNER="${DEV_RUNNER:-$SCRIPT_DIR/target/release/koma-source-dev}"

# Source registry: directory name → crate directory name
declare -A SOURCE_MAP=(
  ["baozimh"]="baozimh"
  ["mangadex"]="mangadex"
  ["mangabz"]="mangabz"
  ["manhuaren"]="manhuaren"
  ["zaimanhua"]="zaimanhua"
  ["happymh"]="happymh"
  ["manhuagui"]="manhuagui"
  ["terrahistoricus"]="terrahistoricus"
  ["noyacg"]="noyacg"
  ["komiic"]="komiic"
  ["dm5"]="dm5"
  ["zerobyw"]="zerobyw"
)

# Optional: nsfw flags not in source_info
declare -A NSFW_MAP=(
  ["baozimh"]="false"
  ["mangadex"]="true"
  ["mangabz"]="false"
  ["manhuaren"]="false"
  ["zaimanhua"]="false"
  ["happymh"]="false"
  ["manhuagui"]="true"
  ["terrahistoricus"]="false"
  ["noyacg"]="true"
  ["komiic"]="true"
  ["dm5"]="true"
  ["zerobyw"]="true"
)

REPO_URL="${KOMA_REPO_URL:-https://github.com/honjow/koma-sources}"

ONLY_SOURCE=""
VERSION_TAG=""
SCAFFOLD_NAME=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --source) ONLY_SOURCE="$2"; shift 2 ;;
    --tag) VERSION_TAG="$2"; shift 2 ;;
    --scaffold) SCAFFOLD_NAME="$2"; shift 2 ;;
    *) shift ;;
  esac
done

log() { echo "$@" >&2; }

if [[ -n "$SCAFFOLD_NAME" ]]; then
  "$SCRIPT_DIR/scripts/scaffold-source.sh" "$SCAFFOLD_NAME"
  exit 0
fi

build_source() {
  local name="$1"
  local crate_dir="${SOURCE_MAP[$name]}"
  local src_path="$SOURCES_DIR/$crate_dir"
  
  if [[ ! -d "$src_path" ]]; then
    log "ERROR: source directory not found: $src_path"
    return 1
  fi

  log "▸ Packaging $name..."
  
  # Find the wasm file (workspace builds output to root target/)
  local crate_name
  crate_name=$(grep '^name' "$src_path/Cargo.toml" | head -1 | sed 's/.*= *"\(.*\)"/\1/')
  local wasm_file="$SCRIPT_DIR/target/wasm32-unknown-unknown/release/${crate_name}.wasm"
  if [[ ! -f "$wasm_file" ]]; then
    # fallback: try crate-local target
    wasm_file="$src_path/target/wasm32-unknown-unknown/release/${crate_name}.wasm"
  fi
  if [[ ! -f "$wasm_file" ]]; then
    log "ERROR: no wasm output found for $name (expected $wasm_file)"
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
  
  # Determine pkg download URL/path
  local pkg_filename="${name}-${src_version}.koma"
  local pkg_url
  if [[ -n "$VERSION_TAG" ]]; then
    pkg_url="${REPO_URL}/releases/download/${VERSION_TAG}/${pkg_filename}"
  else
    pkg_url="sources/${name}/${pkg_filename}"
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

validate_index_packages() {
  local index_file="$OUTPUT_DIR/index.json"
  local failures=0

  while IFS= read -r pkg_path; do
    if [[ -z "$pkg_path" || "$pkg_path" == "null" ]]; then
      log "ERROR: index entry has empty pkg"
      failures=$((failures + 1))
      continue
    fi

    if [[ "$pkg_path" == /* || "$pkg_path" =~ ^[A-Za-z][A-Za-z0-9+.-]*: ]]; then
      continue
    fi

    if [[ "$pkg_path" == ../* || "$pkg_path" == */../* ]]; then
      log "ERROR: pkg path escapes dist: $pkg_path"
      failures=$((failures + 1))
      continue
    fi

    if [[ ! -f "$OUTPUT_DIR/$pkg_path" ]]; then
      log "ERROR: pkg path does not exist under dist: $pkg_path"
      failures=$((failures + 1))
    fi
  done < <(jq -r '.[].pkg' "$index_file")

  if [[ "$failures" -gt 0 ]]; then
    return 1
  fi

  log "  ✓ package paths resolve under dist"
}

# Main
log "▸ Building dev runner..."
cargo build --release -p koma-source-dev 2>&1 | tail -1 >&2

log "▸ Building all WASM sources..."
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS="${CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS:+$CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS }-C target-feature=-reference-types -C link-arg=--strip-all" \
cargo build --release --target wasm32-unknown-unknown \
  $(for name in "${!SOURCE_MAP[@]}"; do
    dir="${SOURCE_MAP[$name]}"
    crate=$(grep '^name' "$SOURCES_DIR/$dir/Cargo.toml" | head -1 | sed 's/.*= *"\(.*\)"/\1/')
    echo "-p $crate"
  done) 2>&1 | tail -1 >&2

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

validate_index_packages

log ""
log "Done. Output in $OUTPUT_DIR/"
