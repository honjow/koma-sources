//! # Koma Source Template
//!
//! Copy this directory to `sources/<your-source>/` and implement the operations.
//!
//! This template uses SDK macros to eliminate boilerplate. You only need to:
//! 1. Edit `SOURCE_INFO` and `SOURCE_CAPS` below
//! 2. Implement the `run_*` functions for operations you support
//! 3. For operations you don't support, leave the default error return

#![no_std]

use koma_source_sdk::host::{self, http_request, log_info};
use koma_source_sdk::json_utils::{
    append_json_escaped, extract_json_string, write_bytes,
};
use koma_source_sdk::result::ResultBuffer;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};

// ═══════════════════════════════════════════════════════════════════
// Configuration — edit these for your source
// ═══════════════════════════════════════════════════════════════════

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.example.koma",          // Unique reverse-domain ID
    name: "Example Source",           // Display name shown in Koma
    language: "en",                   // ISO 639-1 language code
    version: "0.1.0",                // Semantic version
    api_version: "0.2",              // Koma source API version (use "0.2")
    description: "An example source template.",
    author: "Your Name",
    content_rating: "unknown",       // "safe", "suggestive", "nsfw", or "unknown"
};

const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: true,                    // Text search
    manga_detail: true,              // Manga detail page
    chapters: true,                  // Chapter list
    pages: true,                     // Page image URLs
    listings: false,                 // Browse by listing (popular, latest, etc.)
    manga_list: false,               // Browse with filters/pagination
    filters: false,                  // Filter options for browse
    settings: false,                 // Source settings (e.g. cookies, quality)
    home: false,                     // Home page sections
    credentials: false,              // Requires login/auth
    image_request: true,             // Modify image request headers
};

// ═══════════════════════════════════════════════════════════════════
// Buffers — adjust sizes based on your source's needs
//
// payload:    output JSON (search results, chapter list, etc.)
// http_out:   HTTP response body buffer
// body:       decoded JSON body after stripping HTTP headers
// http_req:   HTTP request buffer (URL + headers)
// scratch:    temporary working buffers (two provided: scratch_a/b)
// ═══════════════════════════════════════════════════════════════════

koma_source_sdk::koma_source_buffers! {
    payload: 128 * 1024,             // 128 KB — search results, chapter lists
    http_out: 512 * 1024,            // 512 KB — HTTP response bodies
    body: 512 * 1024,                // 512 KB — decoded JSON
    http_req: 2048,                  // 2 KB — request URL + headers
    scratch: 8192,                   // 8 KB — temporary strings
}

// ═══════════════════════════════════════════════════════════════════
// Helpers — provided by SDK macros
//
// After koma_source_helpers!(), these become available:
//   write_error(op, code, msg)     → write error response
//   write_success_payload(op, len) → write success response from payload_buf
//   read_request(ptr, len)         → parse WASM ABI request to &[u8]
//   trim_ascii(bytes)              → trim whitespace from byte slice
//   decode_json_body(resp)         → strip HTTP headers, decode to body_buf
//   fetch_get(url, referer)        → HTTP GET + decode JSON in one call
// ═══════════════════════════════════════════════════════════════════

koma_source_sdk::koma_source_helpers!();

// ═══════════════════════════════════════════════════════════════════
// Operations — implement these for your source
//
// Each function receives the request JSON as &[u8] and returns a u32
// result pointer (via write_error / write_success_payload).
//
// Request JSON examples:
//   search:        {"query":"one piece","page":1,"limit":25}
//   get_manga:     {"mangaId":"manga:123"}
//   get_chapters:  {"mangaId":"manga:123"}
//   get_pages:     {"chapterId":"chapter:123:456"}
//   get_image_request: {"url":"https://..."}
//
// Response JSON format:
//   search:        {"items":[...],"page":{"hasMore":false}}
//   get_manga:     {"manga":{...}}
//   get_chapters:  {"items":[...],"page":{"hasMore":false}}
//   get_pages:     {"pages":[...]}
//   get_image_request: {"url":"...","headers":{}}
//
// See docs/WRITING_A_SOURCE.md for full JSON schemas.
// ═══════════════════════════════════════════════════════════════════

fn run_search(req: &[u8]) -> u32 {
    let _query = extract_json_string(req, b"query").unwrap_or(b"");

    // TODO: Build search URL, fetch, parse results
    // Example for an HTML source:
    //   let url = b"https://example.com/search?q=...";
    //   let req_len = koma_source_sdk::build_get_request(http_req_buf(), url, None, &[])
    //       .unwrap();
    //   let resp_len = http_request(&http_req_buf()[..req_len], http_out())
    //       .map_err(|_| 0).unwrap();
    //   let html = &http_out()[..resp_len];
    //   let doc = host::html_parse(html).unwrap();
    //   // ... select elements, build JSON ...
    //   host::html_close(doc).ok();

    // Example for a JSON API source:
    //   let resp_len = fetch_get(url, None).unwrap();
    //   let body = &body_buf()[..resp_len];
    //   // ... parse JSON with extract_json_string, JsonArrayIter ...

    write_error("search", "not_implemented", "TODO: implement search")
}

fn run_get_manga(req: &[u8]) -> u32 {
    let _manga_id = extract_json_string(req, b"mangaId").unwrap_or(b"");

    // TODO: Fetch manga detail and build response JSON
    // Response format: {"manga":{"id":"...","title":"...","description":"...",...}}

    write_error("get_manga", "not_implemented", "TODO: implement get_manga")
}

fn run_get_chapters(req: &[u8]) -> u32 {
    let _manga_id = extract_json_string(req, b"mangaId").unwrap_or(b"");

    // TODO: Fetch chapter list and build response JSON
    // Response format: {"items":[{"id":"...","title":"...","chapterNumber":"1",...}],...}

    write_error("get_chapters", "not_implemented", "TODO: implement get_chapters")
}

fn run_get_pages(req: &[u8]) -> u32 {
    let _chapter_id = extract_json_string(req, b"chapterId").unwrap_or(b"");

    // TODO: Fetch page image URLs and build response JSON
    // Response format: {"pages":[{"id":"page:0","index":0,"image":{"kind":"url","url":"..."}}]}

    write_error("get_pages", "not_implemented", "TODO: implement get_pages")
}

fn run_get_image_request(req: &[u8]) -> u32 {
    // Most sources just pass through the URL unchanged.
    // Override this to add custom headers (e.g. Referer) for image requests.
    let url = match extract_json_string(req, b"url") {
        Some(u) => u,
        None => return write_error("get_image_request", "invalid_request", "missing url"),
    };

    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"url":""#)
        && append_json_escaped(payload, &mut c, url)
        && write_bytes(payload, &mut c, br#"","headers":{}}"#);
    if !ok {
        return write_error("get_image_request", "internal_error", "buffer overflow");
    }
    write_success_payload("get_image_request", c)
}

// Optional operations — implement only if SOURCE_CAPS enables them:

fn run_get_listings(_req: &[u8]) -> u32 {
    write_error("get_listings", "not_implemented", "not supported")
}

fn run_get_manga_list(_req: &[u8]) -> u32 {
    write_error("get_manga_list", "not_implemented", "not supported")
}

fn run_get_home(_req: &[u8]) -> u32 {
    write_error("get_home", "not_implemented", "not supported")
}

fn run_get_filters(_req: &[u8]) -> u32 {
    write_error("get_filters", "not_implemented", "not supported")
}

fn run_get_settings(_req: &[u8]) -> u32 {
    write_error("get_settings", "not_implemented", "not supported")
}

// ═══════════════════════════════════════════════════════════════════
// Panic handler (required for no_std)
// ═══════════════════════════════════════════════════════════════════

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}

// ═══════════════════════════════════════════════════════════════════
// WASM exports — auto-generated by the SDK macro
//
// This generates all koma_source_* export functions that the host
// calls. Each export reads the request, calls the corresponding
// run_* function above, and returns the result pointer.
// ═══════════════════════════════════════════════════════════════════

koma_source_sdk::koma_source_exports!("example");
