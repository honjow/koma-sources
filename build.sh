#!/usr/bin/env bash
# build.sh — Build all sources into .koma packages and generate index.json
# Usage: ./build.sh [--source <name>] [--scaffold <name>]
# Requires: cargo (with wasm32-unknown-unknown), zip, jq, koma-source-dev (in PATH or $DEV_RUNNER)
set -euo pipefail

# Prefer rustup toolchain (system cargo may lack wasm32-unknown-unknown target).
# macOS does not provide getent, so fall back to HOME there.
_real_home="${REAL_HOME:-${HOME:-}}"
if [[ -z "$_real_home" ]] && command -v getent >/dev/null 2>&1; then
  _real_home="$(getent passwd "$(id -un)" | cut -d: -f6)"
fi
if [[ -n "$_real_home" && -d "$_real_home/.rustup" && -d "$_real_home/.cargo/bin" ]]; then
  export RUSTUP_HOME="$_real_home/.rustup"
  export CARGO_HOME="$_real_home/.cargo"
  export PATH="$_real_home/.cargo/bin:$PATH"
fi
unset _real_home
if ! command -v cargo >/dev/null 2>&1 && [[ -d /opt/homebrew/opt/rustup/bin ]]; then
  export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SOURCES_DIR="$SCRIPT_DIR/sources"
OUTPUT_DIR="$SCRIPT_DIR/dist"
DEV_RUNNER="${DEV_RUNNER:-$SCRIPT_DIR/target/release/koma-source-dev}"

# Source registry: source name | crate directory | nsfw.
# Keep this Bash 3 compatible for macOS /bin/bash.
SOURCE_ENTRIES='
baozimh|baozimh|false
mangadex|mangadex|true
mangabz|mangabz|false
manhuaren|manhuaren|false
zaimanhua|zaimanhua|false
happymh|happymh|false
manhuagui|manhuagui|true
terrahistoricus|terrahistoricus|false
noyacg|noyacg|true
komiic|komiic|true
dm5|dm5|true
zerobyw|zerobyw|true
manhuashe|manhuashe|false
dongmanmanhua|dongmanmanhua|false
manhuawu|manhuawu|false
iqiyi|iqiyi|false
jiuermanhua|jiuermanhua|false
zazhimi|zazhimi|false
mh1234|mh1234|false
hanman18|hanman18|true
example-demo|example-demo|false
'

source_names() {
  printf '%s\n' "$SOURCE_ENTRIES" | while IFS='|' read -r name _dir _nsfw; do
    [[ -n "$name" ]] && printf '%s\n' "$name"
  done
}

source_crate_dir() {
  local target="$1"
  printf '%s\n' "$SOURCE_ENTRIES" | while IFS='|' read -r name dir _nsfw; do
    if [[ "$name" == "$target" ]]; then
      printf '%s\n' "$dir"
      return 0
    fi
  done
}

source_nsfw() {
  local target="$1"
  printf '%s\n' "$SOURCE_ENTRIES" | while IFS='|' read -r name _dir nsfw; do
    if [[ "$name" == "$target" ]]; then
      printf '%s\n' "$nsfw"
      return 0
    fi
  done
}

selected_source_names() {
  if [[ -n "$ONLY_SOURCE" ]]; then
    printf '%s\n' "$ONLY_SOURCE"
  else
    source_names
  fi
}

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
  local crate_dir
  crate_dir="$(source_crate_dir "$name")"
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
  
  local nsfw
  nsfw="$(source_nsfw "$name")"
  nsfw="${nsfw:-false}"
  
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
  $(for name in $(selected_source_names); do
    dir="$(source_crate_dir "$name")"
    crate=$(grep '^name' "$SOURCES_DIR/$dir/Cargo.toml" | head -1 | sed 's/.*= *"\(.*\)"/\1/')
    echo "-p $crate"
  done) 2>&1 | tail -1 >&2

rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR/sources"

INDEX_ENTRIES=()

for name in $(source_names | sort); do
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
