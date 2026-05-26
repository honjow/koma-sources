# Writing a Koma Source

A complete guide to creating manga sources for [Koma](https://github.com/honjow/Koma).

Sources are compiled to WebAssembly (`wasm32-unknown-unknown`, `#![no_std]`) and run inside the Koma app. The host provides HTTP, HTML parsing, and logging — your source handles site-specific logic.

## Quick Start

```bash
# 1. Create a new source from template
cd koma-sources
./build.sh --scaffold mysource
# This creates sources/mysource/ with Cargo.toml + src/lib.rs

# 2. Edit source metadata in sources/mysource/src/lib.rs
# 3. Implement the run_* functions
# 4. Register in build.sh (add to SOURCE_MAP and NSFW_MAP)
# 5. Add "sources/mysource" to workspace Cargo.toml members

# 6. Build
./build.sh --source mysource

# 7. Test
./target/release/koma-source-dev test-all \
  target/wasm32-unknown-unknown/release/koma_mysource_source.wasm
```

## Architecture

```
Koma App (host)              Your WASM source
─────────────────            ──────────────────
koma_source_search()    →   run_search(req)        →  you call host::http_request()
koma_source_get_manga() →   run_get_manga(req)     →  you call host::html_parse()
koma_source_get_chapters()→ run_get_chapters(req)  →  host provides HTML parsing
koma_source_get_pages() →   run_get_pages(req)     →  host provides HTTP
         ↑                           ↓
    reads JSON result          writes JSON to payload_buf
```

The host calls your exported functions, you use host imports to fetch data, and you write JSON results to a static buffer. No allocator needed.

## Template Structure

Copy `template/` to `sources/<name>/`. The template uses three SDK macros that eliminate ~180 lines of boilerplate:

```rust
#![no_std]
use koma_source_sdk::host;
use koma_source_sdk::json_utils::{append_json_escaped, extract_json_string, write_bytes};
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};

// 1. Source metadata
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.example.mysource",   // unique reverse-domain ID
    name: "My Source",             // display name
    language: "en",                // ISO 639-1
    version: "0.1.0",
    api_version: "0.2",           // always "0.2"
    description: "A manga source.",
    author: "Your Name",
    content_rating: "safe",       // "safe" | "suggestive" | "nsfw" | "unknown"
};

const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: true,        // text search
    manga_detail: true,  // manga detail page
    chapters: true,      // chapter list
    pages: true,         // page image URLs
    listings: false,     // browse by listing (popular, latest)
    manga_list: false,   // browse with filters/pagination
    filters: false,      // filter options
    settings: false,     // source settings (cookies, quality)
    home: false,         // home page sections
    credentials: false,  // requires login
    image_request: false,// modify image request headers
};

// 2. Static buffers — adjust sizes for your source
koma_source_sdk::koma_source_buffers! {
    payload: 128 * 1024,   // output JSON
    http_out: 512 * 1024,  // HTTP response body
    body: 512 * 1024,      // decoded JSON body
    http_req: 2048,        // HTTP request (URL + headers)
    scratch: 8192,         // temporary buffers (scratch_a, scratch_b)
}

// 3. Helper functions
koma_source_sdk::koma_source_helpers!();

// 4. Implement operations
fn run_search(req: &[u8]) -> u32 { /* ... */ }
fn run_get_manga(req: &[u8]) -> u32 { /* ... */ }
fn run_get_chapters(req: &[u8]) -> u32 { /* ... */ }
fn run_get_pages(req: &[u8]) -> u32 { /* ... */ }
fn run_get_image_request(req: &[u8]) -> u32 { /* ... */ }
// Optional operations (return not_implemented if unsupported):
fn run_get_listings(_req: &[u8]) -> u32 { write_error("get_listings", "not_implemented", "") }
fn run_get_manga_list(_req: &[u8]) -> u32 { write_error("get_manga_list", "not_implemented", "") }
fn run_get_home(_req: &[u8]) -> u32 { write_error("get_home", "not_implemented", "") }
fn run_get_filters(_req: &[u8]) -> u32 { write_error("get_filters", "not_implemented", "") }
fn run_get_settings(_req: &[u8]) -> u32 { write_error("get_settings", "not_implemented", "") }

// 5. WASM exports — auto-generated
koma_source_sdk::koma_source_exports!("mysource");
```

### What the macros provide

After `koma_source_buffers!`, these buffer accessors are available:
- `response_buffer()` — result buffer for writing responses
- `payload_buf()` — mutable `&[u8]` for building output JSON
- `http_out()` — mutable `&[u8]` where host writes HTTP responses
- `body_buf()` — mutable `&[u8]` for decoded body
- `http_req_buf()` — mutable `&[u8]` for building HTTP requests
- `scratch_a()`, `scratch_b()` — temporary working buffers
- `payload_slice(len)` — immutable view of payload_buf

After `koma_source_helpers!`, these functions are available:
- `write_error(op, code, msg) -> u32` — write error response
- `write_success_payload(op, len) -> u32` — write success from payload_buf[0..len]
- `read_request(ptr, len) -> Option<&[u8]>` — parse WASM ABI request
- `trim_ascii(bytes) -> &[u8]` — trim whitespace
- `decode_json_body(resp) -> Result<usize, FetchError>` — strip HTTP headers into body_buf
- `fetch_get(url, referer) -> Result<usize, FetchError>` — HTTP GET + decode in one call

After `koma_source_exports!("name")`, all `koma_source_*` WASM exports are generated.

## JSON Request Formats

The host sends JSON requests to your source:

| Operation | Request JSON |
|-----------|-------------|
| search | `{"query":"one piece","page":1,"limit":25}` |
| get_manga | `{"mangaId":"manga:slug"}` |
| get_chapters | `{"mangaId":"manga:slug"}` |
| get_pages | `{"chapterId":"chapter:slug:001"}` |
| get_image_request | `{"url":"https://..."}` |
| get_listings | `{}` |
| get_manga_list | `{"listingId":"popular","page":1,"limit":25}` |
| get_home | `{}` |
| get_filters | `{}` |

Parse with `extract_json_string(req, b"mangaId")`.

## JSON Response Formats

You build response JSON in `payload_buf()` using `write_bytes` and `append_json_escaped`.

### search

```json
{
  "items": [
    {
      "id": "manga:slug",
      "title": "Title",
      "subtitle": "Alt title (optional)",
      "cover": {"kind": "url", "url": "https://..."},
      "authors": ["Author"],
      "status": "ongoing",
      "contentRating": "safe",
      "sourceTags": ["tag1"]
    }
  ],
  "page": {"hasMore": false}
}
```

### get_manga

```json
{
  "manga": {
    "id": "manga:slug",
    "title": "Title",
    "alternateTitles": ["Alt Title"],
    "description": "Description text",
    "cover": {"kind": "url", "url": "https://..."},
    "authors": ["Author"],
    "artists": ["Artist"],
    "status": "ongoing",
    "contentRating": "safe",
    "language": "zh",
    "tags": ["Action", "Comedy"],
    "links": [{"kind": "source", "url": "https://..."}]
  }
}
```

### get_chapters

```json
{
  "items": [
    {
      "id": "chapter:slug:001",
      "mangaId": "manga:slug",
      "title": "Chapter 1",
      "chapterNumber": "1",
      "volumeNumber": null,
      "language": "zh",
      "publishedAt": null,
      "updatedAt": null,
      "pageCount": 20
    }
  ],
  "page": {"hasMore": false}
}
```

### get_pages

```json
{
  "pages": [
    {
      "id": "page:0",
      "index": 0,
      "image": {"kind": "url", "url": "https://..."}
    }
  ]
}
```

**Important**: Page images use nested `image.url`, not a flat `url` field.

### get_image_request

```json
{"url": "https://...", "headers": {"Referer": "https://..."}}
```

Used to add custom headers (e.g. Referer) to image requests. Most sources can pass through unchanged.

### get_listings

```json
{"listings": [{"id": "popular", "name": "Popular"}, {"id": "latest", "name": "Latest"}]}
```

### get_manga_list

Same format as search response. Request includes `listingId` and `page`/`limit`.

### Cover / Image kinds

- URL: `{"kind": "url", "url": "https://..."}`
- None: `{"kind": "none"}`
- Placeholder: `{"kind": "placeholder", "label": "...", "width": 800, "height": 1200}`

## Building JSON Responses

Since there's no allocator, you build JSON by writing bytes into `payload_buf()`:

```rust
fn run_search(req: &[u8]) -> u32 {
    let query = extract_json_string(req, b"query").unwrap_or(b"");
    // ... fetch and parse data ...

    let p = payload_buf();
    let mut c = 0usize;  // cursor

    // Each write_bytes/append_json_escaped returns false on overflow
    let ok = write_bytes(p, &mut c, br#"{"items":[{"id":"manga:1","title":""#)
        && append_json_escaped(p, &mut c, title_bytes)  // escapes " \ etc.
        && write_bytes(p, &mut c, br#"","cover":{"kind":"none"}}],"page":{"hasMore":false}}"#);

    if !ok {
        return write_error("search", "internal_error", "buffer overflow");
    }
    write_success_payload("search", c)
}
```

### Key JSON utilities

| Function | Purpose |
|----------|---------|
| `write_bytes(dst, &mut cursor, src)` | Write raw bytes, advance cursor |
| `append_json_escaped(dst, &mut cursor, src)` | Write bytes with JSON escaping (handles `"`, `\`, control chars) |
| `write_usize(dst, &mut cursor, val)` | Write a number as ASCII digits |
| `write_url_encoded(dst, &mut cursor, src)` | URL-encode bytes |
| `extract_json_string(data, key)` | Find `"key":"value"` in JSON, returns the value bytes |
| `extract_json_number(data, key)` | Find `"key":123` in JSON |
| `JsonArrayIter::new(data, key)` | Iterate `"key":[{...},{...}]` |

## HTTP Requests

### Simple GET

```rust
use koma_source_sdk::host::http_request;

// Build HTTP request (URL + headers)
let req_len = koma_source_sdk::build_get_request(
    http_req_buf(),
    b"https://example.com/api/manga/1",
    None,           // referer
    &[],            // extra headers
).unwrap();

// Execute request — host writes response to http_out
let resp_len = http_request(&http_req_buf()[..req_len], http_out()).unwrap();

// Strip HTTP status line + headers, decode body to body_buf
let body_len = koma_source_sdk::decode_json_body_into(
    &http_out()[..resp_len],
    body_buf()
).unwrap();

let json_body = &body_buf()[..body_len];
```

### One-shot GET (helper)

```rust
// fetch_get does build_get_request + http_request + decode_json_body in one call
let body_len = fetch_get(b"https://example.com/api/search?q=test", None).unwrap();
let json_body = &body_buf()[..body_len];
```

### POST request

```rust
let req_len = koma_source_sdk::build_post_request(
    http_req_buf(),
    b"https://example.com/api/search",
    None,                          // referer
    b"application/json",           // content-type
    br#"{"query":"test","page":1}"#,  // body
).unwrap();

let resp_len = host::http_request(&http_req_buf()[..req_len], http_out()).unwrap();
```

### Custom headers

```rust
// build_get_request accepts extra headers as key:value pairs
let req_len = koma_source_sdk::build_get_request(
    http_req_buf(),
    url,
    Some(b"https://example.com"),   // referer
    &[
        b"X-Api-Key: your-key-here",
        b"Accept: application/json",
    ],
).unwrap();
```

## HTML Scraping

For sites without a JSON API:

```rust
use koma_source_sdk::host;

fn run_search(req: &[u8]) -> u32 {
    let query = extract_json_string(req, b"query").unwrap_or(b"");
    let url = b"https://example.com/search?q=test";

    let req_len = koma_source_sdk::build_get_request(http_req_buf(), url, None, &[]).unwrap();
    let resp_len = host::http_request(&http_req_buf()[..req_len], http_out()).unwrap();
    let html = &http_out()[..resp_len];  // raw HTTP response (includes headers)

    // Parse HTML — host provides HTML parser
    let doc = match host::html_parse(html) {
        Ok(d) => d,
        Err(_) => return write_error("search", "parse_error", "html_parse failed"),
    };

    // Select first match
    if let Ok(item) = host::html_select(doc, b"div.result-item") {
        let title = host::html_text(item, scratch_a()).unwrap_or(b"");
        let link = host::html_attr(item, b"href", scratch_b()).unwrap_or(b"");
        // ... build JSON ...
    }

    // IMPORTANT: close documents to free memory
    host::html_close(doc).ok();

    // ... return result ...
}
```

### HTML API reference

| Function | Returns | Description |
|----------|---------|-------------|
| `html_parse(html) -> Result<HtmlDescriptor, i32>` | Document handle | Parse raw HTML (including HTTP headers — host handles stripping) |
| `html_select(doc, selector) -> Result<HtmlDescriptor, i32>` | Element handle | CSS select first matching element |
| `html_select_all(doc, selector, out, cap) -> i32` | Count | Select all matches, write descriptors to out buffer |
| `html_attr(element, attr, out, cap) -> Result<usize, i32>` | Written length | Get attribute value |
| `html_text(element, out, cap) -> Result<usize, i32>` | Written length | Get text content |
| `html_close(descriptor) -> Result<(), i32>` | — | Free descriptor. **Always close documents when done.** |

## Source Settings

For sources that need user-provided values (cookies, tokens, quality preferences):

```rust
// In SOURCE_CAPS:
settings: true,
credentials: true,  // if auth is needed

// Read a setting at runtime:
fn get_setting_or(key: &[u8], default: &[u8]) -> &[u8] {
    let mut buf = scratch_a();
    match host::get_setting(key, buf) {
        Some(val) => val,
        None => default,
    }
}

// Usage:
let cookies = get_setting_or(b"cookies", b"");
let quality = get_setting_or(b"image_quality", b"high");
```

## Testing

### Individual operations

```bash
DEV=./target/release/koma-source-dev
WASM=./target/wasm32-unknown-unknown/release/koma_mysource_source.wasm

# Check metadata
$DEV info $WASM

# Search (2>/dev/null for clean JSON output)
$DEV run --op search --request '{"query":"one piece"}' $WASM 2>/dev/null

# Chain: search → manga → chapters → pages
$DEV run --op get_manga --request '{"mangaId":"manga:SLUG"}' $WASM 2>/dev/null
$DEV run --op get_chapters --request '{"mangaId":"manga:SLUG"}' $WASM 2>/dev/null
$DEV run --op get_pages --request '{"chapterId":"chapter:SLUG:001"}' $WASM 2>/dev/null
```

### Full test suite

```bash
$DEV test-all $WASM
```

This automatically chains search → get_manga → get_chapters → get_pages and reports pass/fail.

### Web UI

```bash
$DEV serve target/wasm32-unknown-unknown/release --port 3010
# Open http://localhost:3010 in browser
```

## Common Pitfalls

### No std library
No `String`, `Vec`, `format!`, `println!`. Use `write_bytes`, `append_json_escaped`, and static buffers for everything.

### Non-ASCII byte literals
`br#"中文"#` causes compile errors in `no_std`. Use UTF-8 hex arrays with a comment:
```rust
// "最新" in UTF-8
const LABEL_LATEST: &[u8] = &[0xe6, 0x9c, 0x80, 0xe6, 0x96, 0xb0];
```

### Buffer overflow
Always check return values of `write_bytes` / `append_json_escaped`. They return `false` when the buffer is full.

### HTTP response format
The host writes the full HTTP response (status line + headers + body) to `http_out`. Use `decode_json_body` or `decode_json_body_into` to strip headers and get just the body.

### HTML descriptor leaks
Always call `host::html_close(doc)` when done with a parsed document. The host has limited descriptor slots.

### Dev runner output
HTTP request logs go to stderr. Use `2>/dev/null` when parsing JSON programmatically:
```bash
$DEV run --op search --request '{"query":"test"}' $WASM 2>/dev/null
```

### Dev runner needs .wasm files
The `serve` and `run` commands need raw `.wasm` files from `target/wasm32-unknown-unknown/release/`, NOT `.koma` zip packages.

### Chinese/JSON text in output
When writing Chinese text to JSON output, use `append_json_escaped` — it handles the `\uXXXX` escaping correctly.

## Reference Sources

| Source | Type | Lines | Good for learning |
|--------|------|-------|-------------------|
| `example-demo` | JSON API | ~260 | **Start here** — minimal working source with SDK macros |
| `terrahistoricus` | JSON API | ~350 | Clean JSON API with pagination |
| `baozimh` | HTML scraping | ~1300 | Full HTML scraping reference |
| `mangadex` | JSON API | ~1200 | Complex API with auth, covers, dedup |

## Distribution

### Building packages

```bash
./build.sh              # Build all registered sources
./build.sh --source foo # Build single source
```

Output: `dist/sources/<name>/<name>-<version>.koma` (zip with manifest.json + source.wasm).

### Third-party source repos

Create your own repo with the same structure. Users add your release index URL in Koma settings.

Minimum:
1. A `sources/your-source/` crate depending on `koma_source_sdk`
2. A `build.sh` that produces `.koma` packages + `index.json`
3. GitHub releases with downloadable assets

The `index.json` format:
```json
{
  "sources": [
    {
      "id": "com.example.mysource",
      "name": "My Source",
      "version": "0.1.0",
      "url": "https://github.com/you/koma-sources/releases/download/v0.1.0/mysource-0.1.0.koma"
    }
  ]
}
```

App fetches the index URL and lists available sources for download.
