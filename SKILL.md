---
name: koma-source-dev
description: "Koma WASM comic source development — SDK API, macros, JSON schemas, patterns, and pitfalls. Load this when developing or reviewing Koma sources."
version: 1.0.0
---

# Koma Source Development (AI Reference)

> Load this skill when developing, reviewing, or debugging Koma WASM comic sources.
> Covers: SDK API, macros, JSON schemas, common patterns, critical rules, build/test commands.

## Architecture

Koma sources are **`#![no_std]` Rust** compiled to **`wasm32-unknown-unknown`** WASM.
Each source is a sandboxed WASM module with no OS access — only host-provided functions.

```
Source (your code) → WASM imports → Koma Runtime (host)
  host::http_request     — send HTTP request
  host::html_parse/select/text/attr/close — parse HTML
  host::get_setting      — read source settings (cookies, tokens)
  host::check_cancel     — check user cancellation
  host::log_info         — debug logging
```

## Source Template (Required Structure)

```rust
#![no_std]
extern crate koma_source_sdk;

use koma_source_sdk::host;
use koma_source_sdk::result::ResultBuffer;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{FetchError, build_get_request, fetch_error_code};
use koma_source_sdk::json_utils::*;

// 1. Buffers — static buffers + accessors (payload_buf, http_out, body_buf, etc.)
koma_source_sdk::koma_source_buffers! {
    payload: 1024 * 1024,
    http_out: 2 * 1024 * 1024,
    body: 2 * 1024 * 1024,
    http_req: 2048,
    scratch: 8192,
}

// 2. Helpers — write_error, write_success_payload, read_request, trim_ascii, decode_json_body, fetch_get, panic_handler
koma_source_sdk::koma_source_helpers!();

// 3. Source metadata
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.example.koma", name: "Example", version: "0.1.0",
    api_version: "0.2", language: "zh", author: "Author",
    description: "Desc", content_rating: "safe",
};

// 4. Capabilities
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: true, manga_detail: true, chapters: true, pages: true,
    listings: false, manga_list: false, home: false, filters: false,
    settings: false, credentials: false, image_request: false,
};

// 5. Operations — implement these 4 at minimum
fn run_search(req: &[u8]) -> u32 { /* ... */ }
fn run_get_manga(req: &[u8]) -> u32 { /* ... */ }
fn run_get_chapters(req: &[u8]) -> u32 { /* ... */ }
fn run_get_pages(req: &[u8]) -> u32 { /* ... */ }

// Optional — stub with not_implemented
fn run_get_home(_req: &[u8]) -> u32 {
    write_error("get_home", "not_implemented", "not supported")
}
// Same for: get_listings, get_manga_list, get_filters, get_settings, get_image_request

// 6. Exports — generates all #[no_mangle] koma_source_* functions
koma_source_sdk::koma_source_exports!("my_source");
```

## SDK API Reference

### Host Functions (`host::*`)

- `http_request(request: &[u8], output: &mut [u8]) -> Result<usize, i32>` — Send HTTP, response → output, returns bytes written
- `html_parse(html: &[u8]) -> Result<HtmlDescriptor, i32>` — Parse HTML into DOM
- `html_select(descriptor, selector: &[u8]) -> Result<HtmlDescriptor, i32>` — CSS query, single element
- `html_attr(descriptor, attr: &[u8], output: &mut [u8]) -> Result<usize, i32>` — Get attribute value
- `html_text(descriptor, output: &mut [u8]) -> Result<usize, i32>` — Get text content
- `html_close(descriptor) -> Result<(), i32>` — **MUST call when done with any descriptor**
- `get_setting(key: &[u8], output: &mut [u8]) -> Option<&[u8]>` — Read source setting
- `log_info(message: &[u8])` — Debug log
- `check_cancel() -> bool` — Check if user cancelled

### html_select_all (custom import, not in base SDK)

```rust
#[link(wasm_import_module = "koma_host")]
extern "C" {
    #[link_name = "html_select_all"]
    fn koma_host_html_select_all(
        descriptor: i32, selector_ptr: *const u8, selector_len: u32,
        out_ptr: *mut u8, out_cap: u32,
    ) -> i32;
}
fn html_select_all(descriptor: i32, selector: &[u8], out: &mut [u8]) -> i32 {
    unsafe { koma_host_html_select_all(descriptor, selector.as_ptr(), selector.len() as u32, out.as_mut_ptr(), out.len() as u32) }
}
// Returns i32 descriptors packed as 4-byte LE. Use i32::from_le_bytes + transmute → HtmlDescriptor.
```

### HTTP Helpers

- `build_get_request(dst, url, referer, headers) -> Option<usize>` — Build GET request bytes
- `build_post_request(dst, url, body, content_type, referer) -> Option<usize>` — Build POST request bytes
- `parse_status_code(bytes) -> Option<u16>` — Extract HTTP status code
- `fetch_error_code(FetchError) -> (&str, &str)` — Map error → (code, message)
- `decode_json_body(resp) -> Result<usize, FetchError>` — Decode JSON body → body_buf

### JSON Utils (`json_utils::*`)

- `extract_json_string(json, key) -> Option<&[u8]>`
- `extract_json_number(json, key) -> Option<&[u8]>`
- `contains_bytes(haystack, needle) -> bool`
- `find_subslice(data, pattern) -> Option<usize>`
- `write_bytes(buf, cursor, data) -> bool`
- `append_json_escaped(buf, cursor, data) -> bool`
- `write_usize(buf, cursor, n) -> bool`
- `write_url_encoded(buf, cursor, data) -> bool`
- `JsonArrayIter::new(data, key)` — iterate JSON array elements

### Types

```rust
pub enum FetchError { Network, NotFound, RateLimit, ClientError, ServerError }
pub struct SourceInfo { id, name, version, api_version, language, author, description, content_rating }
pub struct SourceCapabilities { search, manga_detail, chapters, pages, listings, manga_list, home, filters, settings, credentials, image_request }
```

## JSON Schemas

### search
Request: `{"query":"关键词","page":1}`
Response items:
```json
{"id":"manga:slug","title":"标题","cover":{"kind":"url","url":"https://..."},
 "authors":["作者"],"status":"ongoing","contentRating":"safe","description":"...",
 "sourceTags":["tag"]}
```
Pagination: `"page":{"nextCursor":"next","hasMore":true}`

### get_manga
Request: `{"mangaId":"manga:slug"}`
Response: `{"manga":{"id","title","cover","authors","status","contentRating","description","tags":[],"links":[{"kind":"source","url":"..."}]}}`

### get_chapters
Request: `{"mangaId":"manga:slug"}`
Response items:
```json
{"id":"chapter:slug:ch-id","mangaId":"manga:slug","title":"第1话",
 "chapterNumber":1.0,"volumeNumber":null,"language":"zh"}
```

### get_pages
Request: `{"chapterId":"chapter:slug:ch-id"}`
Response:
```json
{"pages":[{"id":"page:0","image":{"kind":"url","url":"https://..."},"index":0}]}
```
**CRITICAL**: Image URL is `image.url` (nested), NOT top-level `url`.

## Common Patterns

### JSON API fetch pattern
```rust
let url_buf = scratch_a();
let mut c = 0usize;
write_bytes(url_buf, &mut c, b"https://api.site.com/search?q=");
write_url_encoded(url_buf, &mut c, query);
let req_len = build_get_request(http_req_buf(), &url_buf[..c], None, &[]).unwrap();
let resp_len = match host::http_request(&http_req_buf()[..req_len], http_out()) {
    Ok(n) => n,
    Err(_) => return write_error("search", "network", "request failed"),
};
let body_len = match decode_json_body(&http_out()[..resp_len]) {
    Ok(n) => n,
    Err(e) => { let (c, m) = fetch_error_code(e); return write_error("search", c, m); }
};
let body = &body_buf()[..body_len];
// parse body, build output in payload_buf(), write_success_payload("search", cursor)
```

### HTML scraping pattern
```rust
let document = match host::html_parse(&http_out()[..resp_len]) {
    Ok(doc) => doc,
    Err(_) => return write_error("get_manga", "parse_error", "html parse failed"),
};
let mut buf = [0u8; 512];
let title = match host::html_select(document.0, b"h1.title") {
    Ok(desc) => {
        let text = host::html_text(desc.0, &mut buf).ok();
        host::html_close(desc.0).ok(); // ALWAYS close
        text.map(|l| trim_ascii(&buf[..l]))
    }
    Err(_) => None,
};
// ... extract more fields ...
host::html_close(document.0).ok(); // ALWAYS close root
```

### html_select_all iteration
```rust
let count = html_select_all(document.0.raw(), b"div.item", select_buf);
for i in 0..(count as usize) {
    let off = i * 4;
    let raw = i32::from_le_bytes([select_buf[off], select_buf[off+1], select_buf[off+2], select_buf[off+3]]);
    if raw < 0 { continue; }
    let desc: host::HtmlDescriptor = unsafe { core::mem::transmute(raw) };
    // ... use desc ...
    host::html_close(desc).ok(); // ALWAYS close each
}
```

### Output JSON assembly
```rust
let payload = payload_buf();
let mut c = 0usize;
write_bytes(payload, &mut c, br#"{"ok":true,"data":{"items":["#);
let mut written = 0;
for item in items {
    if written > 0 { write_bytes(payload, &mut c, b","); }
    let ok = write_bytes(payload, &mut c, br#"{"id":"manga:"#)
        && append_json_escaped(payload, &mut c, item_id)
        && write_bytes(payload, &mut c, br#"","title":""#)
        && append_json_escaped(payload, &mut c, item_title)
        && write_bytes(payload, &mut c, br#""}"#);
    if !ok { break; }
    written += 1;
}
write_bytes(payload, &mut c, br#"],"page":{"nextCursor":null,"hasMore":false}}"#);
write_success_payload("search", c)
```

## Critical Rules

### ❌ MUST NOT
1. No `std` — `#![no_std]` only. No `String`, `Vec`, `HashMap`, `format!`, `println!`
2. No non-ASCII in byte literals — `br#"中文"#` fails. Use `&[0xe4, 0xb8, 0xad]` with comment `// 中文`
3. No QuickJS/JS eval — cannot embed JS runtime in WASM
4. No file I/O, sockets, threading — only host functions
5. Don't leak HtmlDescriptor — always `html_close()` when done
6. Don't use top-level `url` in page objects — it's nested `image.url`
7. Don't flatten `links` array — use `[{"kind":"source","url":"..."}]`

### ✅ MUST
1. Use all 3 SDK macros: `koma_source_buffers!` → `koma_source_helpers!` → `koma_source_exports!`
2. Implement minimum: `run_search`, `run_get_manga`, `run_get_chapters`, `run_get_pages`
3. Stub unimplemented ops: `fn run_get_home(_req: &[u8]) -> u32 { write_error("get_home", "not_implemented", "not supported") }`
4. Use `write_error` for errors — never panic
5. URL-encode query params with `write_url_encoded`
6. JSON-escape all output strings with `append_json_escaped`
7. `koma_source_exports!` calls ALL 10 `run_*` functions — missing any = compile error

## Build & Test

```bash
# Build single source
REAL_HOME=$HOME bash build.sh --source baozimh

# Build all
REAL_HOME=$HOME bash build.sh

# Test single operation (2>/dev/null for clean JSON)
./target/release/koma-source-dev run --op search \
  --request '{"query":"漫画","page":1}' \
  target/wasm32-unknown-unknown/release/koma_baozimh_source.wasm 2>/dev/null

# Full chain test
./target/release/koma-source-dev test-all \
  target/wasm32-unknown-unknown/release/koma_baozimh_source.wasm

# Dev web UI
./target/release/koma-source-dev serve target/wasm32-unknown-unknown/release --port 3010
```

Use `.wasm` files, NOT `.koma` zip packages.

## Common Errors

- `unexpected closing delimiter` — mismatched `{`/`}` in write_bytes JSON strings
- `#[panic_handler] function required` — ensure `koma_source_helpers!()` is present
- `non-ASCII in byte literal` — replace Chinese text with hex UTF-8 arrays
- `payload overflow` — increase `payload` size in `koma_source_buffers!`
- 0 search results — wrong CSS selectors; `curl` actual page to verify DOM
- 0 pages with URLs — using `url` instead of nested `image.url`
- JSON parse error — missing quotes, unclosed strings, trailing comma before `]`
- `cranelift` error — build individual packages with `-p koma_<name>_source`
- `no rules expected koma_source_sdk` — `koma_source_buffers!` macro closing `}` missing

## New Source Checklist

1. `./build.sh --scaffold mysource` or copy `template/`
2. Edit `sources/mysource/src/lib.rs` — SourceInfo, SourceCaps, implement `run_*` functions
3. Register in `Cargo.toml` members + `build.sh` SOURCE_MAP/NSFW_MAP (scaffold does this)
4. Build: `cargo build --release --target wasm32-unknown-unknown -p koma_mysource_source`
5. Test full chain: search → manga → chapters → pages
6. Package: `bash build.sh --source mysource` → `dist/sources/mysource/`

## Distribution

`.koma` zip = `manifest.json` + `source.wasm`. Built by `build.sh` automatically.
