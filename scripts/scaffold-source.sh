#!/usr/bin/env bash
# scaffold-source.sh - Scaffold a new Koma WASM source.
# Usage: ./scripts/scaffold-source.sh <name>
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "Usage: $0 <name>" >&2
  exit 1
fi

NAME="$1"
if [[ ! "$NAME" =~ ^[a-z][a-z0-9_]*$ ]]; then
  echo "ERROR: source name must match ^[a-z][a-z0-9_]*$" >&2
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
SOURCE_DIR="$ROOT_DIR/sources/$NAME"
CRATE_NAME="koma_${NAME}_source"

display_name() {
  local raw="$1"
  local spaced="${raw//_/ }"
  local out="" word first rest
  for word in $spaced; do
    first="${word:0:1}"
    rest="${word:1}"
    out+="${first^^}${rest} "
  done
  printf '%s' "${out% }"
}

NAME_TITLE="$(display_name "$NAME")"

if [[ -e "$SOURCE_DIR" ]]; then
  echo "ERROR: sources/$NAME already exists" >&2
  exit 1
fi

mkdir -p "$SOURCE_DIR/src"

cat > "$SOURCE_DIR/Cargo.toml" <<EOF
[package]
name = "$CRATE_NAME"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
koma_source_sdk = { path = "../../sdk" }

[profile.release]
panic = "abort"
opt-level = "s"
lto = true
strip = true
EOF

cat > "$SOURCE_DIR/src/lib.rs" <<EOF
#![no_std]

use koma_source_sdk::json_utils::{append_json_escaped, extract_json_string, write_bytes};
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};

const BASE_URL: &[u8] = b"https://example.com";

koma_source_sdk::koma_source_buffers! {
    payload: 1024 * 1024,
    http_out: 2 * 1024 * 1024,
    body: 2 * 1024 * 1024,
    http_req: 2048,
    scratch: 8192,
}

koma_source_sdk::koma_source_helpers!();

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.$NAME.koma",
    name: "$NAME_TITLE",
    version: "0.1.0",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "$NAME_TITLE source.",
    content_rating: "safe",
};

const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: true,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: false,
    manga_list: false,
    home: false,
    filters: false,
    settings: false,
    image_request: false,
    credentials: false,
};

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}

fn write_static_success(operation: &str, data: &[u8]) -> u32 {
    let payload = payload_buf();
    if data.len() > payload.len() {
        return write_error(operation, "internal_error", "overflow");
    }
    payload[..data.len()].copy_from_slice(data);
    write_success_payload(operation, data.len())
}

fn run_search(_req: &[u8]) -> u32 {
    write_error("search", "unimplemented", "not yet implemented")
}

fn run_get_manga(req: &[u8]) -> u32 {
    let _manga_id = extract_json_string(req, b"mangaId").unwrap_or(b"");
    write_error("get_manga", "unimplemented", "not yet implemented")
}

fn run_get_chapters(_req: &[u8]) -> u32 {
    write_error("get_chapters", "unimplemented", "not yet implemented")
}

fn run_get_pages(_req: &[u8]) -> u32 {
    write_error("get_pages", "unimplemented", "not yet implemented")
}

fn run_get_listings(_req: &[u8]) -> u32 {
    write_static_success("get_listings", koma_source_sdk::result::empty_listings())
}

fn run_get_manga_list(_req: &[u8]) -> u32 {
    write_static_success("get_manga_list", koma_source_sdk::result::empty_manga_list())
}

fn run_get_home(_req: &[u8]) -> u32 {
    write_static_success("get_home", koma_source_sdk::result::empty_home())
}

fn run_get_filters(_req: &[u8]) -> u32 {
    write_static_success("get_filters", koma_source_sdk::result::empty_filters())
}

fn run_get_settings(_req: &[u8]) -> u32 {
    const SETTINGS: &[u8] = br#"{"settings":[]}"#;
    write_static_success("get_settings", SETTINGS)
}

fn run_get_image_request(req: &[u8]) -> u32 {
    let url = extract_json_string(req, b"url").unwrap_or(b"");
    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"url":""#)
        && append_json_escaped(payload, &mut c, url)
        && write_bytes(payload, &mut c, br#""}"#);
    if !ok {
        return write_error("get_image_request", "internal_error", "overflow");
    }
    write_success_payload("get_image_request", c)
}

koma_source_sdk::koma_source_exports!("$NAME");
EOF

if ! grep -q "\"sources/$NAME\"" "$ROOT_DIR/Cargo.toml"; then
  tmp="$(mktemp)"
  awk -v entry="  \"sources/$NAME\"," '
    BEGIN { inserted = 0 }
    /^  "tools\/koma-source-dev",/ && !inserted {
      print entry
      inserted = 1
    }
    { print }
  ' "$ROOT_DIR/Cargo.toml" > "$tmp"
  mv "$tmp" "$ROOT_DIR/Cargo.toml"
fi

if ! grep -q "\\[\"$NAME\"\\]=\"$NAME\"" "$ROOT_DIR/build.sh"; then
  tmp="$(mktemp)"
  awk -v name="$NAME" '
    BEGIN { in_source = 0; in_nsfw = 0; source_done = 0; nsfw_done = 0 }
    /^declare -A SOURCE_MAP=\(/ { in_source = 1 }
    /^declare -A NSFW_MAP=\(/ { in_nsfw = 1 }
    in_source && /^\)/ && !source_done {
      printf "  [\"%s\"]=\"%s\"\n", name, name
      source_done = 1
      in_source = 0
    }
    in_nsfw && /^\)/ && !nsfw_done {
      printf "  [\"%s\"]=\"false\"\n", name
      nsfw_done = 1
      in_nsfw = 0
    }
    { print }
  ' "$ROOT_DIR/build.sh" > "$tmp"
  mv "$tmp" "$ROOT_DIR/build.sh"
  chmod 755 "$ROOT_DIR/build.sh"
fi

echo "Created sources/$NAME"
echo ""
echo "Next steps:"
echo "  1. Edit sources/$NAME/src/lib.rs and replace BASE_URL plus the unimplemented operations."
echo "  2. Build it: cargo build --release --target wasm32-unknown-unknown -p $CRATE_NAME"
echo "  3. Test it with the dev runner once implemented."
