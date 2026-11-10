# Writing a Koma Source

A complete guide to creating manga sources for [Koma](https://github.com/honjow/Koma).

Sources are compiled to WebAssembly (`wasm32-unknown-unknown`, `#![no_std]`) and run inside the Koma app. The host provides HTTP, HTML parsing, and logging — your source handles site-specific logic.

## 站点规则归书源包

域名、镜像、请求额度、站点Cookie形状、历史章节ID和站点兼容参数写在源实现或本源的 `metadata.json`。宿主对已安装包通用执行，禁止在App按源名、包名、域名或包版本加白名单、黑名单及特例。需要宿主执行的站点参数使用 [Koma源运行时ABI](https://github.com/honjow/Koma/blob/master/docs/source-runtime-abi.md#站点规则声明) 的 `requestLimits` 和 `websitePolicy`。声明不授予登录、账号或跨域会话权限。

镜像设置是站点选择，不只是API基址。提供镜像选择的源必须在 `websitePolicy.homepageChoices` 中为同一设置声明每个取值对应的可浏览主页，并让当前镜像同时决定API、公开封面、图片CDN、阅读页和普通WebView入口。任何仍回落到默认域的公开资源都视为镜像未完成；宿主会按该声明打开右上角WebView，未声明时只能回退包默认主页。验证必须在直连网络下分别选择默认站与镜像：WebView应落到所选站点，列表和详情封面应来自所选镜像自己的资源域并完整解码。

发布前按最低支持版本的正式宿主核对该包所需能力。仅新版宿主能完成的行为必须提高
源登记的最低 Koma 版本，打包脚本将同一值写入包清单和订阅索引。最低版本必须对应
确实具备该能力的宿主。正式版本尚未确定时，候选包不进入公开索引。

## Quick Start
### 验证页 UA

需要网页验证的源默认使用系统 WebView UA。若真机上确认 Cloudflare managed challenge 在系统 UA 下反复挑战或拿不到 `cf_clearance`，可在 `websitePolicy` 声明 `webSessionAndroidAgent: true`：宿主保留原生 UA 的 Chrome/WebKit 版本和移动指纹，只把括号内的 OS token 换成 `; Android 10`，与华为系统浏览器的行为一致。不要声明桌面 Chrome UA，它会和移动指纹矛盾。该声明默认关闭，启用前要在真机对照验证页加载、cookie/clearance 状态和原请求结果；只改声明、没做设备验证不能记为修复。

### WebView fetch bridge

原生网络栈被站点拒绝、但验证后的 WebView 会话能正常读取文档时，可声明 `webFetchBridge: true`。宿主在验证完成后保留一个隐藏的 WebView 会话，同源 GET 用页面内 fetch 读取，把 HTML 作为 200 bodyText 回放给源。只支持同源 GET、无 body；每个新 URL 需要一次页面内 fetch，但不再弹验证页。启用前要在真机验证列表、封面、详情、章节和阅读图片，不能只声明就发布。


```bash
# 1. Create a new source from template
cd koma-sources
./build.sh --scaffold mysource
# This creates sources/mysource/ with Cargo.toml + src/lib.rs

# 2. Edit source metadata in sources/mysource/src/lib.rs
# 3. Implement the run_* functions
# 4. `--scaffold` already adds name|crate_dir|false|0.1.0 to SOURCE_ENTRIES
#    and adds the crate to workspace Cargo.toml; raise minAppVersion only when needed.

# 6. Add exactly one real local source icon: icon.png/jpg/jpeg/gif/webp/ico
# 7. Build
./build.sh --source mysource

# 8. Test
./target/release/koma-source-dev test-all \
  target/wasm32-unknown-unknown/release/koma_mysource_source.wasm
```

A source identity icon is required, not a manga cover or placeholder. Add exactly
one regular local `icon.png`, `icon.jpg`, `icon.jpeg`, `icon.gif`, `icon.webp` or
`icon.ico`. `build.sh` fails before compilation if the resource is missing,
empty, undecodable, duplicated, larger than 1 MiB or more than 1,048,576 total
pixels. Install Python 3 and Pillow for actual decoding. New manifests require
`icon` to equal the root filename; generated indexes must publish its actual
bytes using the same extension.

素材制作规范：优先合适的真实官网身份原图，不得留下额外底板、外部遮罩或
阴影；允许为移除这些缺陷而二次处理不合适的移植素材，但必须保留真实身份及
出处、处理前后证据。不能用占位、仓库星标或别站标识补缺。App 原有圆角显示
裁剪属于独立层，必须保留；源资源要求不能被误用来取消它。新生产包要求不
自动卸载旧设备来源，也不删除旧备份。

素材制作与验收流程：选取素材前，主代理和执行子代理各自实际读取本段；
出可安装候选前逐个打开原素材和最终素材，在浅、深背景核对真实身份、
额外底板、外部遮罩、阴影及留白。来自指定上游或官网也不豁免这项检查，
原字节未改、尺寸、解码及哈希通过都不能代替视觉验收。先按画面真实
边界裁除外影和多余留白；旧外圆角遮罩裁缺的边角应恢复，由 App 统一
做显示圆角。保留图案、文字与画面本有的填色，不按颜色把画面抠成
透明。低清才做必要的锐化或恢复，无法可靠恢复时才使用受严格约束的
重绘，并如实记为重绘。单张返工不影响已经可用的稿件。处理依据和
前后证据记在该源 ASSET_SOURCES.md。主代理还须从
最终候选包中打开实际图标亲审，核对列表中的真实显示；未完成或有缺陷
就保留未验收状态，不能把执行代理的结论直接当成交付。

## 移植前的源码对照

1. 固定上游扩展版本。从入口追到实际请求和结果，列出分类、筛选、搜索、
   分页、目录、阅读、登录和设置，以及默认值、适用条件和停止条件。
2. 在该源 README 按行为记下上游实现、Koma 调用位置与源内消费位置。
   逐项核对请求形状、分支、默认值和结果语义。比如分页要对照宿主传入的
   `args.page.cursor` 与源的实际读取处。通用栏目和提示使用界面语言；
   没有独立首页的参考不能把热门首屏包装成首页。
3. 实现后沿同一调用链复核。删改或适配写明代码位置及依据；漏项标为
   未完成。交付验证沿用项目现行规则。

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

### Strict request and transport boundaries

For new ports, the opt-in SDK source_protocol module validates complete request
syntax and direct selected fields before HTTP. Its direct/raw_string/text helpers
distinguish absent from non-string or duplicate identities (including escaped key
aliases); legacy json_utils extraction helpers do not provide that guarantee.
text reuses plain_text::decode_json_content: decode a JSON search string before
URL encoding it, rather than encoding its literal escape bytes. feature reads
only the actual hostFeatures array. Existing sources are not silently switched.

source_protocol::body preserves HTTP refusal/rate status first, then checks the
Host's top-level ok completion, and reuses decode_json_body_into. A 200 response
whose body transfer failed (ok:false, response_body/network_error) remains a
network error, not a site parse error. Site HTML parsing happens only after a
complete body. This centralizes the direct/duplicate/type checks already used
by strict catalogue sources without changing the legacy SDK helper behavior.

## JSON Request Formats

The App sends an envelope with an `args` object. The examples below show its contents.
For a pageable listing the first cursor is `null`; subsequent calls use the
returned cursor or, for numbered pages, a decimal string such as `"2"`.

| Operation | Request JSON |
|-----------|-------------|
| search | `{"query":"one piece","page":{"cursor":null,"limit":20},"filters":{}}` |
| get_manga | `{"mangaId":"manga:slug"}` |
| get_chapters | `{"mangaId":"manga:slug"}` |
| get_pages | `{"chapterId":"chapter:slug:001"}` |
| get_image_request | `{"pageId":"manga:slug","url":"https://..."}` |
| get_listings | `{}` |
| get_manga_list | `{"listingId":"popular","page":{"cursor":null,"limit":20},"filters":{}}` |
| get_home | `{}` |
| get_filters | `{}` |

Read identities from the actual `args` object. For image requests the App sends the work ID as `pageId` for a cover and the stable page ID for a reader page. When a listing already knows a public cover URL and all its GET headers, include
the complete `cover.headers` in that listing response, as described in the host
ABI. Reuse the listing's mirror or other already known inputs. Do not re-read a
session or fetch a detail page per cover. Keep the old resolver for older hosts
and for covers that actually need a ticket, session, or special transport.

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

Selected element descriptors refer to nodes in their original parsed document. Closing an element releases only that descriptor; closing its document invalidates all selected descriptors from that document.

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
The host writes a JSON envelope to `http_out`, not a raw HTTP status line or headers:

```json
{"ok":true,"status":200,"bodyText":"response body","networkPerformed":true}
```

`ok` means the transport completed; an HTTP 404 or 503 can also have `ok:true`.
`decode_json_body` / `decode_json_body_into` check HTTP status before decoding
`bodyText`. The official field is `status`; `statusCode` is accepted as a legacy
alias, with `status` taking priority if both are present. HTTP 2xx continues to
body decoding, 404/429/other 4xx/5xx uses the existing `FetchError` categories,
and other status codes (including 3xx) produce `FetchError::Network`; the helper
does not follow redirects or retry. Transport `ok:false` produces an error.
Missing both status fields retains the legacy behavior: `ok:true` decodes the
body, and `ok:false` is a network error. This compatibility case is not proof of
HTTP success. Successful body string decoding and resource budgets are unchanged.

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

Output: `dist/sources/<name>/<name>-<version>.koma` (zip with `manifest.json`, `source.wasm`, and exactly one required package-owned root icon). The generated manifest carries `author`, `description`, and `contentRating` from `SOURCE_INFO`; its `minAppVersion` comes from that source's `SOURCE_ENTRIES` row. Keep these source-level fields accurate because Koma preserves them in the installed source details. `minAppVersion` must use canonical `MAJOR.MINOR.PATCH` text, for example `0.1.0`.

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
      "lang": "en",
      "nsfw": false,
      "author": "Example author",
      "description": "A concise source description.",
      "contentRating": "safe",
      "minAppVersion": "0.1.0",
      "pkg": "mysource-0.1.0.koma",
      "icon": "sources/mysource/icon.png"
    }
  ]
}
```

`pkg` must be a safe relative `.koma` path resolved from the index document's directory. For a GitHub Release, `index.json` and `mysource-0.1.0.koma` are sibling release assets, so the flat filename above resolves correctly. Koma then fetches the package and lists the available source for download.
