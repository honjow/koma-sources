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
RUNNER_FROM_ENV="${DEV_RUNNER:-}"
DEV_RUNNER="${DEV_RUNNER:-$SCRIPT_DIR/target/release/koma-source-dev}"
ICON_VALIDATOR="$SCRIPT_DIR/scripts/validate-source-icon.py"
RELEASE_ICON_DIR="$OUTPUT_DIR/release-icons"
RELEASE_PACKAGE_DIR="$OUTPUT_DIR/release-packages"

# Source registry: source name | crate directory | nsfw | minimum Koma version.
# Keep this Bash 3 compatible for macOS /bin/bash.
SOURCE_ENTRIES='
comicfury-zh|comicfury-zh|true|0.1.0
xkcd-zh|xkcd-zh|false|0.1.0
weicomic-ronshaonien|weicomic-ronshaonien|false|0.1.1
dbmultiverse-zh|dbmultiverse-zh|false|0.1.0
bns-zh-hant|bns-zh-hant|false|0.1.1
genshin-zh|genshin-zh|false|0.1.0
fgo-zh|fgo-zh|false|0.1.1
baozimh|baozimh|false|0.1.0
mangadex|mangadex|true|0.1.0
mangabz|mangabz|false|0.1.0
manhuaren|manhuaren|false|0.1.0
zaimanhua|zaimanhua|false|0.1.0
happymh|happymh|false|0.1.0
manhuagui|manhuagui|true|0.1.0
terrahistoricus|terrahistoricus|false|0.1.0
noyacg|noyacg|true|0.1.0
komiic|komiic|true|0.1.0
dm5|dm5|true|0.1.0
zerobyw|zerobyw|true|0.1.0
manhuashe|manhuashe|false|0.1.0
dongmanmanhua|dongmanmanhua|false|0.1.0
manhuawu|manhuawu|false|0.1.0
iqiyi|iqiyi|false|0.1.0
jiuermanhua|jiuermanhua|false|0.1.0
zazhimi|zazhimi|false|0.1.0
mh1234|mh1234|false|0.1.0
hanman18|hanman18|true|0.1.0
example-demo|example-demo|false|0.1.0
'

SOURCE_APP_VERSION_RE='^(0|[1-9][0-9]{0,8})\.(0|[1-9][0-9]{0,8})\.(0|[1-9][0-9]{0,8})$'

source_names() {
  printf '%s\n' "$SOURCE_ENTRIES" | while IFS='|' read -r name _dir _nsfw _min_app_version; do
    [[ -n "$name" ]] && printf '%s\n' "$name"
  done
}

source_crate_dir() {
  local target="$1"
  printf '%s\n' "$SOURCE_ENTRIES" | while IFS='|' read -r name dir _nsfw _min_app_version; do
    if [[ "$name" == "$target" ]]; then
      printf '%s\n' "$dir"
      return 0
    fi
  done
}

source_nsfw() {
  local target="$1"
  printf '%s\n' "$SOURCE_ENTRIES" | while IFS='|' read -r name _dir nsfw _min_app_version; do
    if [[ "$name" == "$target" ]]; then
      printf '%s\n' "$nsfw"
      return 0
    fi
  done
}

source_min_app_version() {
  local target="$1"
  printf '%s\n' "$SOURCE_ENTRIES" | while IFS='|' read -r name _dir _nsfw min_app_version; do
    if [[ "$name" == "$target" ]]; then
      printf '%s\n' "$min_app_version"
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

validate_selected_source_icons() {
  local name crate_dir
  if ! command -v python3 >/dev/null 2>&1; then
    log "ERROR: python3 and Pillow are required to validate source icons"
    return 1
  fi
  for name in $(selected_source_names); do
    crate_dir="$(source_crate_dir "$name")"
    python3 "$ICON_VALIDATOR" --source-dir "$SOURCES_DIR/$crate_dir" >&2
  done
}

validate_selected_source_registry() {
  local name min_app_version
  for name in $(selected_source_names); do
    min_app_version="$(source_min_app_version "$name")"
    if [[ ! "$min_app_version" =~ $SOURCE_APP_VERSION_RE ]]; then
      log "ERROR: source $name has invalid minimum Koma version: $min_app_version"
      return 1
    fi
  done
}

if [[ -n "$SCAFFOLD_NAME" ]]; then
  "$SCRIPT_DIR/scripts/scaffold-source.sh" "$SCAFFOLD_NAME"
  exit 0
fi

log "▸ Validating source registry..."
validate_selected_source_registry
log "▸ Validating source icons..."
validate_selected_source_icons

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
  
  local nsfw min_app_version
  nsfw="$(source_nsfw "$name")"
  nsfw="${nsfw:-false}"
  min_app_version="$(source_min_app_version "$name")"
  
  local localized_metadata
  localized_metadata="$(python3 "$SCRIPT_DIR/scripts/read-source-localized-metadata.py" "$src_path/metadata.json")"

  local icon_name
  icon_name="$(python3 "$ICON_VALIDATOR" --source-dir "$src_path" --print-name)"

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
    --arg minAppVersion "$min_app_version" \
    --arg icon "$icon_name" \
    --argjson localizedMetadata "$localized_metadata" \
    '{id: $id, name: $name, version: $version, lang: $lang, nsfw: $nsfw, author: $author, description: $description, contentRating: $contentRating, minAppVersion: $minAppVersion, icon: $icon} + $localizedMetadata' \
    > "$pkg_dir/manifest.json"

  # Copy wasm
  cp "$wasm_file" "$pkg_dir/source.wasm"
  
  # Required identity asset; preserve the actual filename and original bytes.
  cp "$src_path/$icon_name" "$pkg_dir/$icon_name"
  python3 "$ICON_VALIDATOR" --package-dir "$pkg_dir" >&2

  # Package as .koma (zip)
  local koma_file="$OUTPUT_DIR/sources/$name/${name}-${src_version}.koma"
  mkdir -p "$(dirname "$koma_file")"
  rm -f "$koma_file"
  local package_entries=(manifest.json source.wasm "$icon_name")
  (cd "$pkg_dir" && zip -q "$koma_file" "${package_entries[@]}")
  
  local size
  size=$(stat -c%s "$koma_file" 2>/dev/null || stat -f%z "$koma_file")
  log "  ✓ ${name}-${src_version}.koma ($(numfmt --to=iec "$size" 2>/dev/null || echo "${size}B"))"
  
  # Local/static indexes serve their package identity asset beneath the index
  # document. A tagged GitHub Release has a flat asset namespace instead, so
  # publish a unique filename and keep the index reference relative to its
  # own release download directory.
  local icon_path
  if [[ -n "$VERSION_TAG" ]]; then
    local release_icon_name="${name}-${icon_name}"
    mkdir -p "$RELEASE_ICON_DIR"
    cp "$pkg_dir/$icon_name" "$RELEASE_ICON_DIR/$release_icon_name"
    icon_path="$release_icon_name"
  else
    icon_path="sources/$name/$icon_name"
    cp "$pkg_dir/$icon_name" "$OUTPUT_DIR/sources/$name/$icon_name"
  fi
  cmp -s "$pkg_dir/$icon_name" "$src_path/$icon_name"

  # Determine pkg download URL/path
  local pkg_filename="${name}-${src_version}.koma"
  local pkg_url
  if [[ -n "$VERSION_TAG" ]]; then
    # GitHub Release assets share the index document's flat download directory.
    # Keep this relative so Koma applies the same directory/origin containment
    # policy to packages and icons.
    mkdir -p "$RELEASE_PACKAGE_DIR"
    cp "$koma_file" "$RELEASE_PACKAGE_DIR/$pkg_filename"
    pkg_url="$pkg_filename"
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
    --arg minAppVersion "$min_app_version" \
    --argjson localizedMetadata "$localized_metadata" \
    '{id: $id, name: $name, version: $version, lang: $lang, nsfw: $nsfw, author: $author, description: $description, contentRating: $contentRating, pkg: $pkg, icon: $icon, minAppVersion: $minAppVersion} + $localizedMetadata'
}

validate_index_packages() {
  local index_file="$OUTPUT_DIR/index.json"
  local pkg_path expected_path
  local failures=0

  while IFS= read -r pkg_path; do
    if [[ -z "$pkg_path" || "$pkg_path" == "null" ]]; then
      log "ERROR: index entry has empty pkg"
      failures=$((failures + 1))
      continue
    fi

    if [[ -n "$VERSION_TAG" ]]; then
      if [[ "$pkg_path" == */* || "$pkg_path" != *.koma ]]; then
        log "ERROR: release package path must be a flat unique Koma asset: $pkg_path"
        failures=$((failures + 1))
        continue
      fi
      expected_path="$RELEASE_PACKAGE_DIR/$pkg_path"
    else
      if [[ "$pkg_path" == /* || "$pkg_path" =~ ^[A-Za-z][A-Za-z0-9+.-]*: ||
        "$pkg_path" == ../* || "$pkg_path" == */../* ]]; then
        log "ERROR: pkg path escapes dist: $pkg_path"
        failures=$((failures + 1))
        continue
      fi
      expected_path="$OUTPUT_DIR/$pkg_path"
    fi

    if [[ ! -f "$expected_path" ]]; then
      log "ERROR: index package path does not exist in build output: $pkg_path"
      failures=$((failures + 1))
    fi
  done < <(jq -r '.[].pkg' "$index_file")

  if [[ "$failures" -gt 0 ]]; then
    return 1
  fi

  log "  ✓ package paths resolve under dist"
}

validate_index_icons() {
  local index_file="$OUTPUT_DIR/index.json"
  local icon_path expected_path
  local failures=0

  while IFS= read -r icon_path; do
    if [[ -z "$icon_path" || "$icon_path" == "null" ]]; then
      log "ERROR: index entry requires a non-empty icon"
      failures=$((failures + 1))
      continue
    fi
    if [[ -n "$VERSION_TAG" ]]; then
      if [[ ! "$icon_path" =~ ^[a-zA-Z0-9_-]+-icon\.(png|jpg|jpeg|gif|webp|ico)$ ]]; then
        log "ERROR: release icon path must be a flat unique image asset: $icon_path"
        failures=$((failures + 1))
        continue
      fi
      expected_path="$RELEASE_ICON_DIR/$icon_path"
    else
      if [[ ! "$icon_path" =~ ^sources/[a-zA-Z0-9_-]+/icon\.(png|jpg|jpeg|gif|webp|ico)$ ]]; then
        log "ERROR: static index icon path is invalid: $icon_path"
        failures=$((failures + 1))
        continue
      fi
      expected_path="$OUTPUT_DIR/$icon_path"
    fi
    if [[ ! -f "$expected_path" ]]; then
      log "ERROR: index icon path does not exist in build output: $icon_path"
      failures=$((failures + 1))
    elif ! python3 "$ICON_VALIDATOR" --icon "$expected_path" >&2; then
      failures=$((failures + 1))
    fi
  done < <(jq -r '.[].icon' "$index_file")

  if [[ "$failures" -gt 0 ]]; then
    return 1
  fi

  log "  ✓ icon paths resolve in build output"
}

# Main
if [[ -n "$RUNNER_FROM_ENV" ]]; then
  if [[ ! -f "$DEV_RUNNER" || ! -x "$DEV_RUNNER" ]]; then
    log "ERROR: supplied DEV_RUNNER is not an executable file: $DEV_RUNNER"; exit 1
  fi
else
  log "▸ Building dev runner..."
  cargo build --release -p koma-source-dev 2>&1 | tail -1 >&2
fi

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
validate_index_icons

log ""
log "Done. Output in $OUTPUT_DIR/"
