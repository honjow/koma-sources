//! # Example Demo Source
//!
//! A minimal, fully functional Koma source that demonstrates:
//! - Using SDK macros (koma_source_buffers!, koma_source_helpers!, koma_source_exports!)
//! - JSON API source pattern (fetch JSON, parse with extract_json_string / JsonArrayIter)
//! - HTML scraping pattern (html_parse → html_select → html_attr → html_text → html_close)
//! - Building JSON responses manually in no_std
//!
//! This source uses the free JSONPlaceholder API (https://jsonplaceholder.typicode.com)
//! to provide fake manga data. It's meant as a learning reference, not a real source.

#![no_std]

use koma_source_sdk::host;
use koma_source_sdk::json_utils::{
    append_json_escaped, extract_json_string, write_bytes, write_usize, JsonArrayIter,
};
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};

// ── Source metadata ──────────────────────────────────────────────
// These are displayed in the Koma app source list.

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.example.demo",
    name: "Demo Source",
    language: "en",
    version: "0.1.0",
    api_version: "0.2",
    description: "A demo source using JSONPlaceholder API. Not a real manga source.",
    author: "Koma Contributors",
    content_rating: "safe",
};

const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: true,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: false,
    manga_list: false,
    filters: false,
    settings: false,
    home: false,
    credentials: false,
    image_request: false,
};

// ── Buffers ──────────────────────────────────────────────────────
// These define static memory for the WASM module.
// - payload: where you build the output JSON
// - http_out: where the host writes HTTP response body
// - body: decoded body (after stripping HTTP status line)
// - http_req: where you build the HTTP request (URL + headers)
// - scratch: temporary working buffers (scratch_a, scratch_b)

koma_source_sdk::koma_source_buffers! {
    payload: 64 * 1024,
    http_out: 256 * 1024,
    body: 256 * 1024,
    http_req: 2048,
    scratch: 4096,
}

// ── Helpers ──────────────────────────────────────────────────────
// The macro generates: write_error, write_success_payload, read_request,
// trim_ascii, decode_json_body, fetch_get

koma_source_sdk::koma_source_helpers!();

// ── Search ───────────────────────────────────────────────────────
// Input:  {"query":"one piece","page":1,"limit":25}
// Output: {"items":[...],"page":{"hasMore":false}}
//
// We fake it by fetching posts from JSONPlaceholder and wrapping them
// as "manga" results.

fn run_search(req: &[u8]) -> u32 {
    let _query = extract_json_string(req, b"query").unwrap_or(b"");

    // Fetch from a public JSON API
    let url = b"https://jsonplaceholder.typicode.com/posts?_limit=3";
    let req_len = match koma_source_sdk::build_get_request(http_req_buf(), url, None, &[]) {
        Some(n) => n,
        None => return write_error("search", "internal_error", "build request failed"),
    };

    let resp_len = match host::http_request(&http_req_buf()[..req_len], http_out()) {
        Ok(n) => n,
        Err(_) => return write_error("search", "network_error", "HTTP request failed"),
    };

    // Decode HTTP response → body_buf (strips HTTP headers)
    let body_len = match koma_source_sdk::decode_json_body_into(&http_out()[..resp_len], body_buf())
    {
        Ok(n) => n,
        Err(_) => return write_error("search", "parse_error", "invalid JSON response"),
    };

    let body = &body_buf()[..body_len];

    // Build output JSON — iterate JSONPlaceholder posts array
    let payload = payload_buf();
    let mut c = 0usize;

    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("search", "internal_error", "overflow");
    }

    // JSONPlaceholder returns a top-level array: [{"id":1,"title":"...","body":"..."}, ...]
    // Parse by iterating through array elements
    let mut idx = 1; // skip opening [
    let mut first = true;

    while idx < body.len() {
        // Skip whitespace and commas
        while idx < body.len() && matches!(body[idx], b' ' | b',' | b'\n' | b'\r' | b'\t') {
            idx += 1;
        }
        if idx >= body.len() || body[idx] == b']' {
            break;
        }
        if body[idx] != b'{' {
            idx += 1;
            continue;
        }

        // Find the end of this object (matching })
        let obj_start = idx;
        let mut depth = 0i32;
        let mut obj_end = idx;
        while obj_end < body.len() {
            if body[obj_end] == b'{' {
                depth += 1;
            } else if body[obj_end] == b'}' {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            obj_end += 1;
        }
        if depth != 0 {
            break;
        }
        let obj = &body[obj_start..=obj_end];

        let id = extract_json_string(obj, b"id").unwrap_or(b"0");
        let title = extract_json_string(obj, b"title").unwrap_or(b"Untitled");

        if !first {
            if !write_bytes(payload, &mut c, b",") {
                break;
            }
        }
        first = false;

        let ok = write_bytes(payload, &mut c, br#"{"id":"demo:"#)
            && append_json_escaped(payload, &mut c, id)
            && write_bytes(payload, &mut c, br#"","title":""#)
            && append_json_escaped(payload, &mut c, title)
            && write_bytes(
                payload,
                &mut c,
                br#"","cover":{"kind":"none"},"authors":[],"status":"unknown","contentRating":"safe","sourceTags":["demo"]}"#,
            );

        if !ok {
            break;
        }

        idx = obj_end + 1;
    }

    if !write_bytes(payload, &mut c, br#"],"page":{"hasMore":false}}"#) {
        return write_error("search", "internal_error", "overflow");
    }

    write_success_payload("search", c)
}

// ── Get manga detail ─────────────────────────────────────────────
// Input:  {"mangaId":"demo:1"}
// Output: {"manga":{"id":"...","title":"...","description":"...",...}}

fn run_get_manga(req: &[u8]) -> u32 {
    let manga_id = extract_json_string(req, b"mangaId").unwrap_or(b"");
    // Extract the numeric part from "demo:N"
    let num_part = if manga_id.starts_with(b"demo:") {
        &manga_id[5..]
    } else {
        manga_id
    };

    let url_scratch = scratch_a();
    let mut uc = 0usize;
    if !write_bytes(url_scratch, &mut uc, b"https://jsonplaceholder.typicode.com/posts/")
        || !append_json_escaped(url_scratch, &mut uc, num_part)
    {
        return write_error("get_manga", "internal_error", "url overflow");
    }
    let url = &url_scratch[..uc];

    let req_len = match koma_source_sdk::build_get_request(http_req_buf(), url, None, &[]) {
        Some(n) => n,
        None => return write_error("get_manga", "internal_error", "build request failed"),
    };

    let resp_len = match host::http_request(&http_req_buf()[..req_len], http_out()) {
        Ok(n) => n,
        Err(_) => return write_error("get_manga", "network_error", "HTTP request failed"),
    };

    let body_len = match koma_source_sdk::decode_json_body_into(&http_out()[..resp_len], body_buf())
    {
        Ok(n) => n,
        Err(_) => return write_error("get_manga", "parse_error", "invalid JSON"),
    };

    let body = &body_buf()[..body_len];
    let title = extract_json_string(body, b"title").unwrap_or(b"Unknown");
    let desc = extract_json_string(body, b"body").unwrap_or(b"");

    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"manga":{"id":"demo:"#)
        && append_json_escaped(payload, &mut c, num_part)
        && write_bytes(payload, &mut c, br#"","title":""#)
        && append_json_escaped(payload, &mut c, title)
        && write_bytes(payload, &mut c, br#"","description":""#)
        && append_json_escaped(payload, &mut c, desc)
        && write_bytes(
            payload,
            &mut c,
            br#"","cover":{"kind":"none"},"authors":["Demo Author"],"status":"unknown","contentRating":"safe","tags":["demo"],"links":[]}}"#,
        );

    if !ok {
        return write_error("get_manga", "internal_error", "overflow");
    }
    write_success_payload("get_manga", c)
}

// ── Get chapters ─────────────────────────────────────────────────
// Input:  {"mangaId":"demo:1"}
// Output: {"items":[{"id":"ch:demo:1:1","title":"Chapter 1",...}],...}
//
// We generate 3 fake chapters for any manga.

fn run_get_chapters(req: &[u8]) -> u32 {
    let manga_id = extract_json_string(req, b"mangaId").unwrap_or(b"");

    let payload = payload_buf();
    let mut c = 0usize;

    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("get_chapters", "internal_error", "overflow");
    }

    for i in 1..=3 {
        if i > 1 {
            if !write_bytes(payload, &mut c, b",") {
                return write_error("get_chapters", "internal_error", "overflow");
            }
        }
        let ok = write_bytes(payload, &mut c, br#"{"id":"ch:"#)
            && append_json_escaped(payload, &mut c, manga_id)
            && write_bytes(payload, &mut c, b":")
            && write_usize(payload, &mut c, i)
            && write_bytes(payload, &mut c, br#"","mangaId":""#)
            && append_json_escaped(payload, &mut c, manga_id)
            && write_bytes(payload, &mut c, br#"","title":"Chapter "#)
            && write_usize(payload, &mut c, i)
            && write_bytes(payload, &mut c, br#"","chapterNumber":""#)
            && write_usize(payload, &mut c, i)
            && write_bytes(payload, &mut c, br#"","language":"en","pageCount":1}"#);

        if !ok {
            return write_error("get_chapters", "internal_error", "overflow");
        }
    }

    if !write_bytes(payload, &mut c, br#"],"page":{"hasMore":false}}"#) {
        return write_error("get_chapters", "internal_error", "overflow");
    }

    write_success_payload("get_chapters", c)
}

// ── Get pages ────────────────────────────────────────────────────
// Input:  {"chapterId":"ch:demo:1:1"}
// Output: {"pages":[{"id":"page:0","index":0,"image":{"kind":"url","url":"..."}}]}

fn run_get_pages(_req: &[u8]) -> u32 {
    // Return a placeholder image for each page
    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(
        payload,
        &mut c,
        br#"{"pages":[{"id":"page:0","index":0,"image":{"kind":"placeholder","label":"demo-page","width":800,"height":1200}}]}"#,
    );

    if !ok {
        return write_error("get_pages", "internal_error", "overflow");
    }
    write_success_payload("get_pages", c)
}

// Optional operations — return not_implemented

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

fn run_get_image_request(_req: &[u8]) -> u32 {
    write_error("get_image_request", "not_implemented", "not supported")
}

// ── Panic handler (required for no_std) ──────────────────────────
// ── WASM exports ─────────────────────────────────────────────────
// Generates: koma_source_info, koma_source_init, koma_source_search,
// koma_source_get_manga, koma_source_get_chapters, koma_source_get_pages,
// koma_source_get_listings, koma_source_get_manga_list, koma_source_get_home,
// koma_source_get_filters, koma_source_get_settings, koma_source_get_image_request,
// koma_source_free

koma_source_sdk::koma_source_exports!("example-demo");

// ── Utility ──────────────────────────────────────────────────────

fn find_subslice_from(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    if haystack.len() < needle.len() {
        return None;
    }
    for i in 0..=haystack.len() - needle.len() {
        if haystack[i..].starts_with(needle) {
            return Some(i);
        }
    }
    None
}
