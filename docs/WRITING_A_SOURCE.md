# Writing a Koma Source

This guide walks through creating a new manga source for Koma.

## Quick Start

1. Copy the template:
   ```bash
   cp -r template sources/my-source
   ```

2. Edit `sources/my-source/Cargo.toml`:
   ```toml
   [package]
   name = "koma_my_source"
   ```

3. Add to workspace `Cargo.toml`:
   ```toml
   members = [
     # ...
     "sources/my-source",
   ]
   ```

4. Edit `sources/my-source/src/lib.rs` — update constants and implement operations.

5. Build & test:
   ```bash
   cargo build --release --target wasm32-unknown-unknown -p koma_my_source
   ./target/release/koma-source-dev run \
     --op search --request '{"query":"test"}' \
     target/wasm32-unknown-unknown/release/koma_my_source.wasm
   ```

## Architecture

A Koma source is a WASM module (compiled from Rust `#![no_std]`) that exports a fixed set of functions. The host (Koma app or dev runner) calls these functions, passing JSON requests and receiving JSON responses.

```
Host (Koma app)          WASM Module (your source)
─────────────────        ─────────────────────────
call koma_source_info()  → returns source metadata
call koma_source_search  → you fetch HTML/JSON via host, parse, return results
     ↑                     ↓
   koma_host.http_request  (host provides HTTP)
   koma_host.html_parse    (host provides HTML parsing)
   koma_host.html_select
   koma_host.html_attr
   koma_host.html_text
   koma_host.log           (host provides logging)
```

## Exports (your source must provide)

| Export | Required | Description |
|--------|----------|-------------|
| `koma_source_info()` | ✓ | Return source metadata + capabilities |
| `koma_source_init(ptr, len)` | ✓ | Initialize with manifest data |
| `koma_source_search(ptr, len)` | ✓ | Search for manga |
| `koma_source_get_manga(ptr, len)` | ✓ | Get manga details |
| `koma_source_get_chapters(ptr, len)` | ✓ | Get chapter list |
| `koma_source_get_pages(ptr, len)` | ✓ | Get page URLs for a chapter |
| `koma_source_get_image_request(ptr, len)` | | Modify image request (headers, URL) |
| `koma_source_get_listings(ptr, len)` | | Browse by listing (popular, latest) |
| `koma_source_get_manga_list(ptr, len)` | | Browse with filters/pagination |
| `koma_source_get_filters(ptr, len)` | | Return available filter options |
| `koma_source_get_settings(ptr, len)` | | Return source settings |
| `koma_source_get_home(ptr, len)` | | Home page sections |
| `koma_source_alloc(size)` | ✓ | Allocate memory for host to write into |
| `koma_source_free(ptr)` | ✓ | Free a result buffer |

Set `SourceCapabilities` flags to match what you implement.

## Host Imports (provided by the host)

| Import | Signature | Description |
|--------|-----------|-------------|
| `koma_host.http_request` | `(req_ptr, req_len, out_ptr, out_cap) -> i32` | Make HTTP request, returns response length |
| `koma_host.html_parse` | `(html_ptr, html_len) -> i32` | Parse HTML, returns document descriptor |
| `koma_host.html_select` | `(desc, sel_ptr, sel_len) -> i32` | CSS select first match, returns descriptor |
| `koma_host.html_select_all` | `(desc, sel_ptr, sel_len, out_ptr, out_cap) -> i32` | CSS select all, returns count |
| `koma_host.html_attr` | `(desc, attr_ptr, attr_len, out_ptr, out_cap) -> i32` | Get attribute value |
| `koma_host.html_text` | `(desc, out_ptr, out_cap) -> i32` | Get text content |
| `koma_host.html_close` | `(desc) -> i32` | Free HTML document |
| `koma_host.log` | `(level, ptr, len)` | Log message (0=debug, 1=info, 2=warn, 3=error) |
| `koma_host.check_cancel` | `() -> i32` | Check if operation was cancelled |

## SDK Utilities

The `koma_source_sdk` crate provides:

- **`host`** — Wrappers for host imports (`http_request`, `log_info`, etc.)
- **`json_utils`** — No-alloc JSON building/parsing (`write_bytes`, `append_json_escaped`, `extract_json_string`, `extract_json_number`, `write_url_encoded`, `JsonArrayIter`, etc.)
- **`result`** — `ResultBuffer` for writing success/error responses
- **`source`** — `SourceInfo` and `SourceCapabilities` structs

## Development Workflow

```bash
# Build dev runner (once)
cargo build --release -p koma-source-dev

# Build your source
cargo build --release --target wasm32-unknown-unknown -p koma_my_source

# Test individual operations
DEV=./target/release/koma-source-dev
WASM=./target/wasm32-unknown-unknown/release/koma_my_source.wasm

$DEV info $WASM                                          # check metadata
$DEV run --op search --request '{"query":"one piece"}' $WASM   # search
$DEV run --op get_manga --request '{"mangaId":"..."}' $WASM    # manga detail
$DEV run --op get_chapters --request '{"mangaId":"..."}' $WASM # chapters

# Run full test suite (chains search → manga → chapters → pages)
$DEV test-all $WASM
```

## Tips

- **No std library** — You're in `#![no_std]`. Use the SDK's `write_bytes`, `append_json_escaped` etc. to build JSON manually. No `String`, `Vec`, or `format!`.
- **Static buffers** — Allocate fixed-size `static mut` arrays. 256KB payload buffer is usually enough.
- **HTML sources** — Use `host::html_parse` + `host::html_select` + `host::html_attr` / `html_text` for scraping. See `sources/baozimh` for a full example.
- **JSON API sources** — Use `host::http_request` and parse with `extract_json_string` / `JsonArrayIter`. See `sources/mangadex` for a full example.
- **Error handling** — Always return meaningful error codes via `write_error()`. The SDK's `FetchError` enum helps classify HTTP failures.
- **Test early** — Use `koma-source-dev run` after implementing each operation. The dev runner provides real HTTP and HTML parsing.
