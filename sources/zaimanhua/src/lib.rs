#![no_std]

extern crate koma_source_sdk;

use koma_source_sdk::host::{self, http_request, log_info};
use koma_source_sdk::json_utils::{
    append_json_escaped, append_json_unescaped_then_escaped, contains_bytes, extract_json_number,
    extract_json_string, find_subslice, write_bytes, write_url_encoded, write_usize,
};
use koma_source_sdk::result::ResultBuffer;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};

const API_URL: &[u8] = b"https://v4api.zaimanhua.com/app/v1";
const MOBILE_BASE: &[u8] = b"https://m.zaimanhua.com";
const DEFAULT_PAGE_SIZE: usize = 20;

const PAYLOAD_CAP: usize = 128 * 1024;
const HTTP_OUT_CAP: usize = 512 * 1024;
const JSON_BUF_CAP: usize = 256 * 1024;
const HTTP_REQ_CAP: usize = 2048;
const SCRATCH_CAP: usize = 4096;

static mut RESPONSE: ResultBuffer<{ PAYLOAD_CAP + 256 }> = ResultBuffer::new();
static mut PAYLOAD_BUF: [u8; PAYLOAD_CAP] = [0; PAYLOAD_CAP];
static mut HTTP_OUT: [u8; HTTP_OUT_CAP] = [0; HTTP_OUT_CAP];
static mut JSON_BUF: [u8; JSON_BUF_CAP] = [0; JSON_BUF_CAP];
static mut HTTP_REQ_BUF: [u8; HTTP_REQ_CAP] = [0; HTTP_REQ_CAP];
static mut SCRATCH_A: [u8; SCRATCH_CAP] = [0; SCRATCH_CAP];
static mut SCRATCH_B: [u8; SCRATCH_CAP] = [0; SCRATCH_CAP];

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.zaimanhua.koma",
    name: "再漫画",
    version: "0.1.0",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "再漫画 manga source (zaimanhua.com)",
    content_rating: "safe",
};

const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: true,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: true,
    manga_list: true,
    home: true,
    filters: true,
    settings: false,
    image_request: true,
    credentials: false,
};

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}

fn response_buffer() -> &'static mut ResultBuffer<{ PAYLOAD_CAP + 256 }> {
    unsafe { &mut *core::ptr::addr_of_mut!(RESPONSE) }
}
fn payload_buf() -> &'static mut [u8] {
    unsafe { &mut *core::ptr::addr_of_mut!(PAYLOAD_BUF) }
}
fn http_out() -> &'static mut [u8] {
    unsafe { &mut *core::ptr::addr_of_mut!(HTTP_OUT) }
}
fn http_req_buf() -> &'static mut [u8] {
    unsafe { &mut *core::ptr::addr_of_mut!(HTTP_REQ_BUF) }
}
fn scratch_a() -> &'static mut [u8] {
    unsafe { &mut *core::ptr::addr_of_mut!(SCRATCH_A) }
}
fn scratch_b() -> &'static mut [u8] {
    unsafe { &mut *core::ptr::addr_of_mut!(SCRATCH_B) }
}
fn json_buf() -> &'static mut [u8] {
    unsafe { &mut *core::ptr::addr_of_mut!(JSON_BUF) }
}

fn payload_slice(len: usize) -> &'static [u8] {
    unsafe { core::slice::from_raw_parts(PAYLOAD_BUF.as_ptr(), len) }
}

fn write_error(operation: &str, code: &str, message: &str) -> u32 {
    response_buffer().write_error(operation, code, message)
}

fn write_success_payload(operation: &str, len: usize) -> u32 {
    response_buffer().write_success(operation, payload_slice(len))
}

fn write_u64_value(dst: &mut [u8], cursor: &mut usize, mut value: u64) -> bool {
    let mut buf = [0u8; 20];
    let mut pos = buf.len();
    if value == 0 {
        pos -= 1;
        buf[pos] = b'0';
    } else {
        while value > 0 {
            pos -= 1;
            buf[pos] = b'0' + (value % 10) as u8;
            value /= 10;
        }
    }
    write_bytes(dst, cursor, &buf[pos..])
}

fn read_request<'a>(req_ptr: u32, req_len: u32) -> Option<&'a [u8]> {
    if req_ptr == 0 || req_len == 0 {
        return None;
    }
    Some(unsafe { core::slice::from_raw_parts(req_ptr as *const u8, req_len as usize) })
}

// --- HTTP helpers ---

fn build_get_request_with_platform(
    dst: &mut [u8],
    url: &[u8],
    platform: Option<&[u8]>,
) -> Option<usize> {
    let mut cursor = 0usize;
    let prefix = br#"{"version":1,"method":"GET","url":""#;
    let headers = br#"","headers":{"User-Agent":"koma-source-dev/0.1""#;
    let suffix = br#"},"timeoutMs":15000,"responseKind":"bodyText"}"#;
    write_bytes(dst, &mut cursor, prefix).then_some(())?;
    append_json_escaped(dst, &mut cursor, url).then_some(())?;
    write_bytes(dst, &mut cursor, headers).then_some(())?;
    if let Some(p) = platform {
        write_bytes(dst, &mut cursor, br#","Platform":""#).then_some(())?;
        append_json_escaped(dst, &mut cursor, p).then_some(())?;
        write_bytes(dst, &mut cursor, b"\"").then_some(())?;
    }
    write_bytes(dst, &mut cursor, suffix).then_some(())?;
    Some(cursor)
}

#[derive(Copy, Clone)]
enum FetchError {
    Network,
    NotFound,
    ParseError,
    ServerError,
}

fn fetch_error_code(e: FetchError) -> (&'static str, &'static str) {
    match e {
        FetchError::Network => ("network_error", "connection or timeout failure"),
        FetchError::NotFound => ("not_found", "resource not found"),
        FetchError::ParseError => ("parse_error", "failed to parse response"),
        FetchError::ServerError => ("server_error", "server error"),
    }
}

fn fetch_json_with_platform(
    url_bytes: &[u8],
    platform: Option<&[u8]>,
) -> Result<&'static [u8], FetchError> {
    let req_len = build_get_request_with_platform(http_req_buf(), url_bytes, platform)
        .ok_or(FetchError::Network)?;

    let mut resp_len = 0usize;
    let mut transport_failed = true;
    for attempt in 0..3u8 {
        let req_slice = &http_req_buf()[..req_len];
        match http_request(req_slice, http_out()) {
            Ok(n) => {
                resp_len = n;
                transport_failed = false;
                break;
            }
            Err(_) => {
                if attempt < 2 {
                    log_info(b"zaimanhua: http transport error, retrying");
                }
            }
        }
    }
    if transport_failed {
        return Err(FetchError::Network);
    }

    let resp = &http_out()[..resp_len];
    if !contains_bytes(resp, br#""ok":true"#) {
        log_info(b"zaimanhua: http response not ok");
        return Err(FetchError::ServerError);
    }

    let body_marker = b"\"bodyText\":\"";
    let body_start =
        find_subslice(resp, body_marker).ok_or(FetchError::Network)? + body_marker.len();
    let out = json_buf();
    let mut out_cursor = 0usize;
    let mut i = body_start;
    while i < resp.len() {
        let b = resp[i];
        if b == b'\\' && i + 1 < resp.len() {
            let next = resp[i + 1];
            let unescaped = match next {
                b'"' => b'"',
                b'\\' => b'\\',
                b'/' => b'/',
                b'n' => b'\n',
                b'r' => b'\r',
                b't' => b'\t',
                _ => next,
            };
            if out_cursor >= out.len() {
                return Err(FetchError::Network);
            }
            out[out_cursor] = unescaped;
            out_cursor += 1;
            i += 2;
            continue;
        }
        if b == b'"' {
            break;
        }
        if out_cursor >= out.len() {
            return Err(FetchError::Network);
        }
        out[out_cursor] = b;
        out_cursor += 1;
        i += 1;
    }
    Ok(unsafe { core::slice::from_raw_parts(JSON_BUF.as_ptr(), out_cursor) })
}

fn fetch_json(url_bytes: &[u8]) -> Result<&'static [u8], FetchError> {
    fetch_json_with_platform(url_bytes, None)
}

// --- Parsing helpers ---

fn parse_status_byte(status: &[u8]) -> &'static [u8] {
    // "连载中" = e8 bf 9e e8 bd bd e4 b8 ad
    if contains_bytes(
        status,
        &[0xe8, 0xbf, 0x9e, 0xe8, 0xbd, 0xbd, 0xe4, 0xb8, 0xad],
    ) {
        b"ongoing"
    // "已完结" = e5 b7 b2 e5 ae 8c e7 bb 93
    } else if contains_bytes(
        status,
        &[0xe5, 0xb7, 0xb2, 0xe5, 0xae, 0x8c, 0xe7, 0xbb, 0x93],
    ) {
        b"completed"
    } else {
        b"unknown"
    }
}

/// Extract the id field: try "comic_id" first (non-zero), then "id"
fn extract_item_id(obj: &[u8]) -> Option<&[u8]> {
    let comic_id = extract_json_number(obj, b"comic_id");
    if let Some(cid) = comic_id {
        if cid != b"0" {
            return Some(cid);
        }
    }
    extract_json_number(obj, b"id")
}

/// Format authors: replace "/" with ", "
fn format_authors_to_payload(payload: &mut [u8], cursor: &mut usize, authors: &[u8]) -> bool {
    let mut i = 0usize;
    let mut first = true;
    while i < authors.len() {
        // skip leading slashes
        while i < authors.len() && authors[i] == b'/' {
            i += 1;
        }
        if i >= authors.len() {
            break;
        }
        // find end of segment
        let start = i;
        while i < authors.len() && authors[i] != b'/' {
            i += 1;
        }
        let segment = &authors[start..i];
        if !segment.is_empty() {
            if !first {
                if !write_bytes(payload, cursor, b",") {
                    return false;
                }
            }
            if !write_bytes(payload, cursor, b"\"")
                || !append_json_unescaped_then_escaped(payload, cursor, segment)
                || !write_bytes(payload, cursor, b"\"")
            {
                return false;
            }
            first = false;
        }
    }
    true
}

// --- Operations ---

fn run_search(req: &[u8]) -> u32 {
    let query = match extract_json_string(req, b"query") {
        Some(q) => q,
        None => return write_error("search", "invalid_request", "missing query"),
    };

    // Check if query is a pure number for ID lookup
    let is_pure_number = !query.is_empty() && query.iter().all(|&b| b >= b'0' && b <= b'9');

    // Extract page from request
    let page_bytes = extract_json_number(req, b"page");
    let page_num = if let Some(pb) = page_bytes {
        let mut n = 0usize;
        for &b in pb {
            n = n * 10 + (b - b'0') as usize;
        }
        if n == 0 {
            1
        } else {
            n
        }
    } else {
        1
    };

    // If pure number and first page, try ID lookup first
    if is_pure_number && page_num == 1 {
        let url_buf = scratch_a();
        let mut url_cursor = 0usize;
        let ok = write_bytes(url_buf, &mut url_cursor, API_URL)
            && write_bytes(url_buf, &mut url_cursor, b"/comic/detail/")
            && write_bytes(url_buf, &mut url_cursor, query)
            && write_bytes(url_buf, &mut url_cursor, b"?_v=2.2.5");
        if ok {
            let url_bytes = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_cursor) };
            if let Ok(api_json) = fetch_json(url_bytes) {
                // Try to extract manga detail from response
                if let Some(manga_obj) = extract_data_inner_object(api_json) {
                    let payload = payload_buf();
                    let mut c = 0usize;
                    if write_bytes(payload, &mut c, br#"{"items":["#) {
                        let id = query;
                        let title = extract_json_string(manga_obj, b"title").unwrap_or(b"Unknown");
                        let cover = extract_json_string(manga_obj, b"cover").unwrap_or(b"");
                        let status = extract_json_status(manga_obj);
                        let authors = extract_authors_list(manga_obj);
                        let types = extract_types_list(manga_obj);

                        let ok2 = write_bytes(payload, &mut c, br#"{"id":""#)
                            && append_json_escaped(payload, &mut c, id)
                            && write_bytes(payload, &mut c, br#"","title":""#)
                            && append_json_escaped(payload, &mut c, title)
                            && write_bytes(payload, &mut c, br#"","cover":{"kind":"url","url":""#)
                            && append_json_escaped(payload, &mut c, cover)
                            && write_bytes(payload, &mut c, br#""},"authors":["#);
                        if ok2 {
                            if let Some(auth) = authors {
                                let _ = write_bytes(payload, &mut c, b"\"")
                                    && append_json_escaped(payload, &mut c, auth)
                                    && write_bytes(payload, &mut c, b"\"");
                            }
                        }
                        let ok3 = write_bytes(payload, &mut c, br#"],"status":""#)
                            && append_json_escaped(payload, &mut c, status)
                            && write_bytes(
                                payload,
                                &mut c,
                                br#"","contentRating":"safe","sourceTags":["#,
                            );
                        if ok3 {
                            if let Some(tags) = types {
                                // tags is already formatted as JSON array content
                                let _ = write_bytes(payload, &mut c, tags);
                            } else {
                                let _ = write_bytes(payload, &mut c, br#""zaimanhua""#);
                            }
                        }
                        if write_bytes(
                            payload,
                            &mut c,
                            br#"]},"page":{"nextCursor":null,"hasMore":false}}"#,
                        ) {
                            return write_success_payload("search", c);
                        }
                    }
                }
            }
        }
        // ID lookup failed, fall through to normal search
    }

    // Normal search: /search/index?keyword=...&source=0&size=20&page=N
    let url_buf = scratch_a();
    let mut url_cursor = 0usize;
    let ok = write_bytes(url_buf, &mut url_cursor, API_URL)
        && write_bytes(url_buf, &mut url_cursor, b"/search/index?source=0&size=")
        && write_usize(url_buf, &mut url_cursor, DEFAULT_PAGE_SIZE)
        && write_bytes(url_buf, &mut url_cursor, b"&keyword=")
        && write_url_encoded(url_buf, &mut url_cursor, query)
        && write_bytes(url_buf, &mut url_cursor, b"&page=")
        && write_usize(url_buf, &mut url_cursor, page_num);
    if !ok {
        return write_error("search", "internal_error", "url overflow");
    }
    let url_bytes = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_cursor) };

    let api_json = match fetch_json(url_bytes) {
        Ok(v) => v,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("search", c, m);
        }
    };

    // Parse response: {"errno":0,"data":{"list":[...],"total":N,"page":N,"size":N}}
    // Or data might have "comicList" instead of "list"
    let data_obj = match extract_json_data_object(api_json) {
        Some(d) => d,
        None => return write_error("search", "parse_error", "no data object"),
    };

    let items_array = extract_json_array_content(data_obj, b"list")
        .or_else(|| extract_json_array_content(data_obj, b"comicList"));

    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("search", "internal_error", "overflow");
    }

    let mut written = 0usize;
    if let Some(items_data) = items_array {
        let mut raw_pos = 0usize;
        while let Some(obj) = next_raw_object(items_data, &mut raw_pos) {
            if written >= DEFAULT_PAGE_SIZE {
                break;
            }
            let id = match extract_item_id(obj) {
                Some(v) => v,
                None => continue,
            };
            // "name" (genre) or "title" (search/ranking)
            let title = extract_json_string(obj, b"title")
                .or_else(|| extract_json_string(obj, b"name"))
                .unwrap_or(b"Unknown");
            let cover = extract_json_string(obj, b"cover").unwrap_or(b"");
            let status_str = extract_json_string(obj, b"status").unwrap_or(b"");
            let status = parse_status_byte(status_str);
            let authors_str = extract_json_string(obj, b"authors");
            let types_str = extract_json_string(obj, b"types");

            if written > 0 {
                if !write_bytes(payload, &mut c, b",") {
                    break;
                }
            }
            let ok = write_bytes(payload, &mut c, br#"{"id":""#)
                && append_json_escaped(payload, &mut c, id)
                && write_bytes(payload, &mut c, br#"","title":""#)
                && append_json_escaped(payload, &mut c, title)
                && write_bytes(payload, &mut c, br#"","cover":{"kind":"url","url":""#)
                && append_json_escaped(payload, &mut c, cover)
                && write_bytes(payload, &mut c, br#""},"authors":["#);
            if !ok {
                break;
            }
            if let Some(auth) = authors_str {
                let _ = format_authors_to_payload(payload, &mut c, auth);
            }
            let ok2 = write_bytes(payload, &mut c, br#"],"status":""#)
                && append_json_escaped(payload, &mut c, status)
                && write_bytes(
                    payload,
                    &mut c,
                    br#"","contentRating":"safe","sourceTags":["#,
                );
            if !ok2 {
                break;
            }
            if let Some(ts) = types_str {
                // Write types as individual tags
                let mut ti = 0usize;
                let mut first_tag = true;
                while ti < ts.len() {
                    while ti < ts.len() && ts[ti] == b'/' {
                        ti += 1;
                    }
                    if ti >= ts.len() {
                        break;
                    }
                    let start = ti;
                    while ti < ts.len() && ts[ti] != b'/' {
                        ti += 1;
                    }
                    let tag = &ts[start..ti];
                    if !tag.is_empty() {
                        if !first_tag {
                            if !write_bytes(payload, &mut c, b",") {
                                break;
                            }
                        }
                        let _ = write_bytes(payload, &mut c, b"\"")
                            && append_json_unescaped_then_escaped(payload, &mut c, tag)
                            && write_bytes(payload, &mut c, b"\"");
                        first_tag = false;
                    }
                }
            } else {
                let _ = write_bytes(payload, &mut c, br#""zaimanhua""#);
            }
            if !write_bytes(payload, &mut c, b"]}") {
                break;
            }
            written += 1;
        }
    }

    // Pagination
    let total = extract_json_number(data_obj, b"total")
        .or_else(|| extract_json_number(data_obj, b"totalNum"))
        .and_then(|t| {
            let mut n = 0usize;
            for &b in t {
                n = n * 10 + (b - b'0') as usize;
            }
            Some(n)
        })
        .unwrap_or(0);
    let has_more = page_num * DEFAULT_PAGE_SIZE < total;

    if !write_bytes(payload, &mut c, br#"],"page":{"nextCursor":""#) {
        return write_error("search", "internal_error", "overflow");
    }
    if has_more {
        if !write_usize(payload, &mut c, page_num + 1) {
            return write_error("search", "internal_error", "overflow");
        }
    }
    if !write_bytes(payload, &mut c, br#"","hasMore":"#) {
        return write_error("search", "internal_error", "overflow");
    }
    let has_more_str: &[u8] = if has_more { b"true" } else { b"false" };
    if !write_bytes(payload, &mut c, has_more_str) {
        return write_error("search", "internal_error", "overflow");
    }
    if !write_bytes(payload, &mut c, b"}}") {
        return write_error("search", "internal_error", "overflow");
    }
    write_success_payload("search", c)
}

/// Extract the inner "data":{...} -> then the inner "data":{...} for manga detail
fn extract_data_inner_object(api_json: &[u8]) -> Option<&[u8]> {
    // {"errno":0,"data":{"comicInfo":{...}}} or {"errno":0,"data":{"data":{...}}}
    let data_marker = b"\"data\":{";
    let idx = find_subslice(api_json, data_marker)?;
    let inner_start = idx + b"\"data\":".len();
    // Parse the outer object { ... }
    let obj = &api_json[inner_start..];
    // Now find the inner "data" or "comicInfo" key
    let inner_data =
        find_subslice(obj, b"\"data\":{").or_else(|| find_subslice(obj, b"\"comicInfo\":{"));
    if let Some(inner_idx) = inner_data {
        let key_len = if contains_bytes(&obj[inner_idx..inner_idx + 20], b"\"comicInfo\"") {
            b"\"comicInfo\":".len()
        } else {
            b"\"data\":".len()
        };
        let obj_start = inner_idx + key_len - 1; // include the {
                                                 // Find matching }
        let mut depth = 0i32;
        let mut pos = obj_start;
        while pos < obj.len() {
            match obj[pos] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(&obj[obj_start..pos + 1]);
                    }
                }
                b'"' => {
                    pos += 1;
                    while pos < obj.len() && obj[pos] != b'"' {
                        if obj[pos] == b'\\' {
                            pos += 1;
                        }
                        pos += 1;
                    }
                }
                _ => {}
            }
            pos += 1;
        }
    }
    None
}

fn extract_json_data_object(api_json: &[u8]) -> Option<&[u8]> {
    // Find "data":{...} at top level
    let data_marker = b"\"data\":{";
    let idx = find_subslice(api_json, data_marker)?;
    let obj_start = idx + b"\"data\":".len() - 1;
    let mut depth = 0i32;
    let mut pos = obj_start;
    while pos < api_json.len() {
        match api_json[pos] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&api_json[obj_start..pos + 1]);
                }
            }
            b'"' => {
                pos += 1;
                while pos < api_json.len() && api_json[pos] != b'"' {
                    if api_json[pos] == b'\\' {
                        pos += 1;
                    }
                    pos += 1;
                }
            }
            _ => {}
        }
        pos += 1;
    }
    None
}

/// Iterate JSON objects from raw array content (content between `[` and `]`)
fn next_raw_object<'a>(data: &'a [u8], pos: &mut usize) -> Option<&'a [u8]> {
    while *pos < data.len() {
        match data[*pos] {
            b' ' | b'\t' | b'\n' | b'\r' | b',' => *pos += 1,
            b']' => return None,
            b'{' => break,
            _ => return None,
        }
    }
    if *pos >= data.len() {
        return None;
    }
    let start = *pos;
    let mut depth = 0i32;
    let mut in_string = false;
    while *pos < data.len() {
        let b = data[*pos];
        if in_string {
            if b == b'\\' {
                *pos += 1;
            } else if b == b'"' {
                in_string = false;
            }
        } else {
            match b {
                b'"' => in_string = true,
                b'{' | b'[' => depth += 1,
                b'}' | b']' => {
                    depth -= 1;
                    if depth == 0 {
                        *pos += 1;
                        return Some(&data[start..*pos]);
                    }
                }
                _ => {}
            }
        }
        *pos += 1;
    }
    None
}

fn extract_json_array_content<'a>(data: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let mut pattern = [0u8; 64];
    let needed = key.len() + 3;
    if needed > pattern.len() {
        return None;
    }
    pattern[0] = b'"';
    pattern[1..1 + key.len()].copy_from_slice(key);
    pattern[1 + key.len()] = b'"';
    pattern[2 + key.len()] = b':';
    let start = find_subslice(data, &pattern[..needed])? + needed;
    // Skip whitespace
    let mut i = start;
    while i < data.len()
        && (data[i] == b' ' || data[i] == b'\n' || data[i] == b'\r' || data[i] == b'\t')
    {
        i += 1;
    }
    if i >= data.len() || data[i] != b'[' {
        return None;
    }
    let arr_start = i + 1;
    let mut depth = 1i32;
    i += 1;
    while i < data.len() && depth > 0 {
        match data[i] {
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
            }
            b'"' => {
                i += 1;
                while i < data.len() && data[i] != b'"' {
                    if data[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    Some(&data[arr_start..i - 1])
}

fn extract_json_bool(data: &[u8], key: &[u8]) -> Option<bool> {
    let mut pattern = [0u8; 64];
    let needed = key.len() + 3;
    if needed > pattern.len() {
        return None;
    }
    pattern[0] = b'"';
    pattern[1..1 + key.len()].copy_from_slice(key);
    pattern[1 + key.len()] = b'"';
    pattern[2 + key.len()] = b':';
    let mut i = find_subslice(data, &pattern[..needed])? + needed;
    while i < data.len()
        && (data[i] == b' ' || data[i] == b'\n' || data[i] == b'\r' || data[i] == b'\t')
    {
        i += 1;
    }
    if i + 4 <= data.len() && &data[i..i + 4] == b"true" {
        Some(true)
    } else if i + 5 <= data.len() && &data[i..i + 5] == b"false" {
        Some(false)
    } else {
        None
    }
}

fn json_array_content_has_value(data: &[u8]) -> bool {
    let mut i = 0usize;
    while i < data.len() {
        match data[i] {
            b' ' | b'\t' | b'\n' | b'\r' | b',' => i += 1,
            _ => return true,
        }
    }
    false
}

fn extract_json_status(obj: &[u8]) -> &'static [u8] {
    // status is an array of {tag_name: "..."}
    let status_marker = b"\"status\":[";
    if let Some(idx) = find_subslice(obj, status_marker) {
        let rest = &obj[idx + b"\"status\":[".len()..];
        // Find first tag_name
        if let Some(name) = extract_json_string(rest, b"tag_name") {
            return parse_status_byte(name);
        }
    }
    b"unknown"
}

fn extract_authors_list<'a>(obj: &'a [u8]) -> Option<&'a [u8]> {
    // authors is an array of {tag_name: "..."}
    let marker = b"\"authors\":[";
    let idx = find_subslice(obj, marker)?;
    let rest = &obj[idx + marker.len()..];
    let mut result_start = None;
    let mut result_end = 0usize;
    // Find first tag_name
    let pos = 0usize;
    while pos < rest.len() {
        if let Some(name_start) = find_subslice(&rest[pos..], b"\"tag_name\":\"") {
            let actual_start = pos + name_start + b"\"tag_name\":\"".len();
            let mut end = actual_start;
            while end < rest.len() && rest[end] != b'"' {
                if rest[end] == b'\\' {
                    end += 1;
                }
                end += 1;
            }
            result_start = Some(actual_start);
            result_end = end;
            break;
        } else {
            break;
        }
    }
    result_start.map(|s| &rest[s..result_end])
}

fn extract_types_list<'a>(obj: &'a [u8]) -> Option<&'a [u8]> {
    // types is an array of {tag_name: "..."}
    let marker = b"\"types\":[";
    let idx = find_subslice(obj, marker)?;
    let arr_rest = &obj[idx + marker.len()..];
    // Parse array of objects and extract tag_name values
    let buf = scratch_b();
    let mut cursor = 0usize;
    let mut raw_pos = 0usize;
    let mut first = true;
    while let Some(item) = next_raw_object(arr_rest, &mut raw_pos) {
        if let Some(name) = extract_json_string(item, b"tag_name") {
            if !first {
                if !write_bytes(buf, &mut cursor, b",") {
                    break;
                }
            }
            if !(write_bytes(buf, &mut cursor, b"\"")
                && append_json_unescaped_then_escaped(buf, &mut cursor, name)
                && write_bytes(buf, &mut cursor, b"\""))
            {
                break;
            }
            first = false;
        }
    }
    if cursor > 0 {
        Some(unsafe { core::slice::from_raw_parts(SCRATCH_B.as_ptr(), cursor) })
    } else {
        None
    }
}

fn run_get_manga(req: &[u8]) -> u32 {
    let manga_id = match extract_json_string(req, b"mangaId") {
        Some(v) => v,
        None => return write_error("get_manga", "invalid_request", "missing mangaId"),
    };

    // API: /comic/detail/{id}?_v=2.2.5
    let url_buf = scratch_a();
    let mut url_cursor = 0usize;
    let ok = write_bytes(url_buf, &mut url_cursor, API_URL)
        && write_bytes(url_buf, &mut url_cursor, b"/comic/detail/")
        && write_bytes(url_buf, &mut url_cursor, manga_id)
        && write_bytes(url_buf, &mut url_cursor, b"?_v=2.2.5");
    if !ok {
        return write_error("get_manga", "internal_error", "url overflow");
    }
    let url_bytes = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_cursor) };

    let api_json = match fetch_json(url_bytes) {
        Ok(v) => v,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_manga", c, m);
        }
    };

    let manga_obj = match extract_data_inner_object(api_json) {
        Some(o) => o,
        None => return write_error("get_manga", "parse_error", "no manga data"),
    };

    let title = extract_json_string(manga_obj, b"title").unwrap_or(b"Unknown");
    let cover = extract_json_string(manga_obj, b"cover").unwrap_or(b"");
    let description = extract_json_string(manga_obj, b"description").unwrap_or(b"");
    let status = extract_json_status(manga_obj);
    let authors = extract_authors_list(manga_obj);
    let types_tags = extract_types_list(manga_obj);

    let payload = payload_buf();
    let mut c = 0usize;

    // Build mobile URL for links
    let link_prefix = b"https://m.zaimanhua.com/pages/comic/detail?id=";

    let ok = write_bytes(payload, &mut c, br#"{"manga":{"id":""#)
        && append_json_escaped(payload, &mut c, manga_id)
        && write_bytes(payload, &mut c, br#"","title":""#)
        && append_json_escaped(payload, &mut c, title)
        && write_bytes(
            payload,
            &mut c,
            br#"","alternateTitles":[],"description":""#,
        )
        && append_json_unescaped_then_escaped(payload, &mut c, description)
        && write_bytes(payload, &mut c, br#"","cover":{"kind":"url","url":""#)
        && append_json_escaped(payload, &mut c, cover)
        && write_bytes(payload, &mut c, br#""},"authors":["#);
    if !ok {
        return write_error("get_manga", "internal_error", "overflow");
    }
    if let Some(auth) = authors {
        let _ = write_bytes(payload, &mut c, b"\"")
            && append_json_escaped(payload, &mut c, auth)
            && write_bytes(payload, &mut c, b"\"");
    }
    let ok2 = write_bytes(payload, &mut c, br#"],"artists":[],"status":""#)
        && append_json_escaped(payload, &mut c, status)
        && write_bytes(
            payload,
            &mut c,
            br#"","contentRating":"safe","language":"zh","tags":["#,
        );
    if !ok2 {
        return write_error("get_manga", "internal_error", "overflow");
    }
    if let Some(tags) = types_tags {
        let _ = write_bytes(payload, &mut c, tags);
    }
    let ok3 = write_bytes(payload, &mut c, br#"],"links":[{"kind":"source","url":""#)
        && append_json_escaped(payload, &mut c, link_prefix)
        && append_json_escaped(payload, &mut c, manga_id)
        && write_bytes(payload, &mut c, br#""}]}}"#);
    if !ok3 {
        return write_error("get_manga", "internal_error", "overflow");
    }
    write_success_payload("get_manga", c)
}

fn run_get_chapters(req: &[u8]) -> u32 {
    let manga_id = match extract_json_string(req, b"mangaId") {
        Some(v) => v,
        None => return write_error("get_chapters", "invalid_request", "missing mangaId"),
    };

    // Try app API first: /comic/detail/{id}?_v=2.2.5 with Platform: pc
    let url_buf = scratch_a();
    let mut url_cursor = 0usize;
    let ok = write_bytes(url_buf, &mut url_cursor, API_URL)
        && write_bytes(url_buf, &mut url_cursor, b"/comic/detail/")
        && write_bytes(url_buf, &mut url_cursor, manga_id)
        && write_bytes(url_buf, &mut url_cursor, b"?_v=2.2.5");
    if !ok {
        return write_error("get_chapters", "internal_error", "url overflow");
    }
    let url_bytes = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_cursor) };

    let api_json = match fetch_json_with_platform(url_bytes, Some(b"pc")) {
        Ok(v) => v,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_chapters", c, m);
        }
    };

    // Check for error message
    if let Some(errmsg) = extract_json_string(api_json, b"errmsg") {
        if !errmsg.is_empty() && errmsg != b"" {
            // Non-empty errmsg (check it's not just empty quotes)
            // Actually extract_json_string returns the content between quotes, so empty string returns Some(b"")
            // Only error if errmsg is non-empty
        }
    }

    let chapter_data = match extract_data_inner_object(api_json) {
        Some(o) => o,
        None => return write_error("get_chapters", "parse_error", "no chapter data"),
    };

    // Check isHideChapter
    let is_hide = extract_json_number(chapter_data, b"isHideChapter")
        .map(|v| v != b"0" && v != b"")
        .unwrap_or(false);

    let can_read = extract_json_string(chapter_data, b"canRead")
        .map(|v| v == b"true")
        .unwrap_or(true);

    // If chapters are hidden but can read, try PC API
    let chapters_json = if is_hide && can_read {
        // PC API: https://manhua.zaimanhua.com/api/v1/comic2/comic/detail?id={mangaId}
        let pc_url_buf = scratch_b();
        let mut pc_cursor = 0usize;
        let ok2 = write_bytes(
            pc_url_buf,
            &mut pc_cursor,
            b"https://manhua.zaimanhua.com/api/v1/comic2/comic/detail?id=",
        ) && write_bytes(pc_url_buf, &mut pc_cursor, manga_id);
        if !ok2 {
            return write_error("get_chapters", "internal_error", "url overflow");
        }
        let pc_url = unsafe { core::slice::from_raw_parts(SCRATCH_B.as_ptr(), pc_cursor) };
        match fetch_json_with_platform(pc_url, Some(b"pc")) {
            Ok(v) => v,
            Err(e) => {
                let (c, m) = fetch_error_code(e);
                return write_error("get_chapters", c, m);
            }
        }
    } else {
        api_json
    };

    // Extract chapter list: chapters or chapterList -> array of groups
    let chapter_data2 = if is_hide && can_read {
        // For PC API response, the structure is different
        match extract_data_inner_object(chapters_json) {
            Some(o) => o,
            None => chapter_data,
        }
    } else {
        chapter_data
    };

    // Find chapters array (key is "chapters" for PC API, "chapters" for app API too)
    let chapters_array = extract_json_array_content(chapter_data2, b"chapters")
        .or_else(|| extract_json_array_content(chapter_data2, b"chapterList"));

    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("get_chapters", "internal_error", "overflow");
    }

    let mut written = 0usize;

    if let Some(groups_data) = chapters_array {
        // Iterate chapter groups
        let mut group_pos = 0usize;
        while let Some(group_obj) = next_raw_object(groups_data, &mut group_pos) {
            let group_title = extract_json_string(group_obj, b"title").unwrap_or(b"");

            // Find "data":[...] inside group (array of chapters)
            let inner_chapters = extract_json_array_content(group_obj, b"data");
            if let Some(chapters_inner) = inner_chapters {
                let mut ch_pos = 0usize;
                while let Some(ch_obj) = next_raw_object(chapters_inner, &mut ch_pos) {
                    let ch_id = extract_json_number(ch_obj, b"chapter_id")
                        .or_else(|| extract_json_string(ch_obj, b"chapter_id"));
                    let ch_name = extract_json_string(ch_obj, b"chapter_title").unwrap_or(b"");
                    let update_time = extract_json_number(ch_obj, b"updatetime");

                    let ch_id_str = match ch_id {
                        Some(id) => id,
                        None => continue,
                    };

                    if written > 0 {
                        if !write_bytes(payload, &mut c, b",") {
                            break;
                        }
                    }
                    // Build chapter URL: mangaId/chapterId
                    let ok = write_bytes(payload, &mut c, br#"{"id":""#)
                        && append_json_escaped(payload, &mut c, manga_id)
                        && write_bytes(payload, &mut c, b"/")
                        && append_json_escaped(payload, &mut c, ch_id_str)
                        && write_bytes(payload, &mut c, br#"","mangaId":""#)
                        && append_json_escaped(payload, &mut c, manga_id)
                        && write_bytes(payload, &mut c, br#"","title":""#)
                        && append_json_escaped(payload, &mut c, ch_name)
                        && write_bytes(payload, &mut c, br#"","chapterNumber":null,"volumeNumber":null,"language":"zh","scanlator":""#)
                        && append_json_escaped(payload, &mut c, group_title)
                        && write_bytes(payload, &mut c, br#"","publishedAt":null,"updatedAt":"#);
                    if !ok {
                        break;
                    }
                    if let Some(ut) = update_time {
                        // Convert unix timestamp to millis
                        let mut secs = 0u64;
                        for &b in ut {
                            secs = secs * 10 + (b - b'0') as u64;
                        }
                        let millis = secs * 1000;
                        let _ = write_u64_value(payload, &mut c, millis);
                    } else {
                        let _ = write_bytes(payload, &mut c, b"null");
                    }
                    if !write_bytes(payload, &mut c, br#","pageCount":null}"#) {
                        break;
                    }
                    written += 1;
                }
            }
        }
    }

    if !write_bytes(
        payload,
        &mut c,
        br#"],"page":{"nextCursor":null,"hasMore":false}}"#,
    ) {
        return write_error("get_chapters", "internal_error", "overflow");
    }
    write_success_payload("get_chapters", c)
}

fn run_get_pages(req: &[u8]) -> u32 {
    let chapter_id = match extract_json_string(req, b"chapterId") {
        Some(v) => v,
        None => return write_error("get_pages", "invalid_request", "missing chapterId"),
    };
    // chapterId is "mangaId/chapterId"
    let _slash_pos = match find_subslice(chapter_id, b"/") {
        Some(p) => p,
        None => return write_error("get_pages", "invalid_request", "bad chapterId format"),
    };

    // API: /comic/chapter/{mangaId}/{chapterId}?_v=2.2.5
    let url_buf = scratch_a();
    let mut url_cursor = 0usize;
    let ok = write_bytes(url_buf, &mut url_cursor, API_URL)
        && write_bytes(url_buf, &mut url_cursor, b"/comic/chapter/")
        && write_bytes(url_buf, &mut url_cursor, chapter_id)
        && write_bytes(url_buf, &mut url_cursor, b"?_v=2.2.5");
    if !ok {
        return write_error("get_pages", "internal_error", "url overflow");
    }
    let url_bytes = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_cursor) };

    let api_json = match fetch_json_with_platform(url_bytes, Some(b"h5")) {
        Ok(v) => v,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_pages", c, m);
        }
    };

    let images_obj = match extract_data_inner_object(api_json) {
        Some(o) => o,
        None => return write_error("get_pages", "parse_error", "no page data"),
    };

    // The API returns canRead as a JSON boolean. Older parsing only handled
    // quoted strings, which let canRead:false chapters through as empty pages.
    let can_read = extract_json_bool(images_obj, b"canRead")
        .or_else(|| extract_json_string(images_obj, b"canRead").map(|v| v != b"false"))
        .unwrap_or(true);
    if !can_read {
        return write_error(
            "get_pages",
            "permission_denied",
            "user cannot read this chapter",
        );
    }

    // Extract page_url_hd array. Fall back to the normal page_url list if the
    // HD list exists but is empty.
    let images_array_hd = extract_json_array_content(images_obj, b"page_url_hd");
    let images_array = match images_array_hd {
        Some(d) if json_array_content_has_value(d) => Some(d),
        _ => extract_json_array_content(images_obj, b"page_url")
            .or_else(|| extract_json_array_content(images_obj, b"images")),
    };
    let images_data = match images_array {
        Some(d) => d,
        None => return write_error("get_pages", "parse_error", "no images array"),
    };

    let payload = payload_buf();
    let mut c = 0usize;
    let ok2 = write_bytes(payload, &mut c, br#"{"chapterId":""#)
        && append_json_escaped(payload, &mut c, chapter_id)
        && write_bytes(payload, &mut c, br#"","pages":["#);
    if !ok2 {
        return write_error("get_pages", "internal_error", "overflow");
    }

    // Parse image URLs from array
    let mut i = 0usize;
    let mut page_idx = 0usize;
    while i < images_data.len() {
        while i < images_data.len()
            && (images_data[i] == b' '
                || images_data[i] == b','
                || images_data[i] == b'\n'
                || images_data[i] == b'\r')
        {
            i += 1;
        }
        if i >= images_data.len() || images_data[i] == b']' {
            break;
        }
        if images_data[i] != b'"' {
            break;
        }
        i += 1;
        let str_start = i;
        while i < images_data.len() && images_data[i] != b'"' {
            if images_data[i] == b'\\' {
                i += 1;
            }
            i += 1;
        }
        let url = &images_data[str_start..i];
        i += 1;

        if page_idx > 0 {
            if !write_bytes(payload, &mut c, b",") {
                break;
            }
        }
        let ok3 = write_bytes(payload, &mut c, br#"{"id":"page:"#)
            && write_usize(payload, &mut c, page_idx)
            && write_bytes(payload, &mut c, br#"","index":"#)
            && write_usize(payload, &mut c, page_idx)
            && write_bytes(payload, &mut c, br#","image":{"kind":"url","url":""#)
            && append_json_unescaped_then_escaped(payload, &mut c, url)
            && write_bytes(payload, &mut c, br#""}}"#);
        if !ok3 {
            break;
        }
        page_idx += 1;
    }

    if !write_bytes(payload, &mut c, b"]}") {
        return write_error("get_pages", "internal_error", "overflow");
    }
    write_success_payload("get_pages", c)
}

fn run_get_listings(_req: &[u8]) -> u32 {
    let payload = payload_buf();
    let mut c = 0usize;
    // Build listings JSON with Chinese labels (UTF-8)
    let ok = write_bytes(payload, &mut c, br#"{"listings":[{"id":"latest","name":""#)
        && write_bytes(payload, &mut c, &[0xe6, 0x9c, 0x80, 0xe6, 0x96, 0xb0, 0xe6, 0x9b, 0xb4, 0xe6, 0x96, 0xb0]) // 最新更新
        && write_bytes(payload, &mut c, br#""},{"id":"popular_rank","name":""#)
        && write_bytes(payload, &mut c, &[0xe4, 0xba, 0xba, 0xe6, 0xb0, 0x94, 0xe6, 0x8e, 0x92, 0xe8, 0xa1, 0x8c]) // 人气排行
        && write_bytes(payload, &mut c, br#""},{"id":"subscribe_rank","name":""#)
        && write_bytes(payload, &mut c, &[0xe8, 0xae, 0xa2, 0xe9, 0x98, 0x85, 0xe6, 0x8e, 0x92, 0xe8, 0xa1, 0x8c]) // 订阅排行
        && write_bytes(payload, &mut c, br#""},{"id":"comment_rank","name":""#)
        && write_bytes(payload, &mut c, &[0xe5, 0x90, 0x90, 0xe6, 0xa7, 0xbd, 0xe6, 0x8e, 0x92, 0xe8, 0xa1, 0x8c]) // 吐槽排行
        && write_bytes(payload, &mut c, br#""}]}"#);
    if !ok {
        return write_error("get_listings", "internal_error", "overflow");
    }
    write_success_payload("get_listings", c)
}

fn run_get_manga_list(req: &[u8]) -> u32 {
    let listing_id = extract_json_string(req, b"listingId").unwrap_or(b"latest");
    let page_bytes = extract_json_number(req, b"page");
    let page_num = if let Some(pb) = page_bytes {
        let mut n = 0usize;
        for &b in pb {
            n = n * 10 + (b - b'0') as usize;
        }
        if n == 0 {
            1
        } else {
            n
        }
    } else {
        1
    };

    let url_buf = scratch_a();
    let mut url_cursor = 0usize;

    if listing_id == b"latest" {
        // /comic/update/list/0/{page}
        let ok = write_bytes(url_buf, &mut url_cursor, API_URL)
            && write_bytes(url_buf, &mut url_cursor, b"/comic/update/list/0/")
            && write_usize(url_buf, &mut url_cursor, page_num);
        if !ok {
            return write_error("get_manga_list", "internal_error", "url overflow");
        }
    } else {
        // Ranking: /comic/rank/list?tag_id=0&rank_type=N&page={page}
        let rank_type = if listing_id == b"subscribe_rank" {
            b"2"
        } else if listing_id == b"comment_rank" {
            b"1"
        } else {
            b"0" // popular
        };
        let ok = write_bytes(url_buf, &mut url_cursor, API_URL)
            && write_bytes(
                url_buf,
                &mut url_cursor,
                b"/comic/rank/list?tag_id=0&rank_type=",
            )
            && write_bytes(url_buf, &mut url_cursor, rank_type)
            && write_bytes(url_buf, &mut url_cursor, b"&page=")
            && write_usize(url_buf, &mut url_cursor, page_num);
        if !ok {
            return write_error("get_manga_list", "internal_error", "url overflow");
        }
    }

    let url_bytes = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_cursor) };
    let api_json = match fetch_json(url_bytes) {
        Ok(v) => v,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_manga_list", c, m);
        }
    };

    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("get_manga_list", "internal_error", "overflow");
    }

    // Response for update list: {"errno":0,"data":[{...},...]}
    // Response for ranking: {"errno":0,"data":{"list":[...],...}}
    let mut written = 0usize;

    // Check if data is an array directly (update list) or object with list (ranking)
    let data_marker = b"\"data\":";
    if let Some(idx) = find_subslice(api_json, data_marker) {
        let after = idx + data_marker.len();
        if after < api_json.len() && api_json[after] == b'[' {
            // Data is a direct array
            let arr_content = &api_json[after + 1..];
            let mut raw_pos = 0usize;
            while let Some(obj) = next_raw_object(arr_content, &mut raw_pos) {
                if written >= DEFAULT_PAGE_SIZE {
                    break;
                }
                if let Some(item_id) = extract_item_id(obj) {
                    let title = extract_json_string(obj, b"title")
                        .or_else(|| extract_json_string(obj, b"name"))
                        .unwrap_or(b"Unknown");
                    let cover = extract_json_string(obj, b"cover").unwrap_or(b"");
                    let status_str = extract_json_string(obj, b"status").unwrap_or(b"");
                    let status = parse_status_byte(status_str);
                    let authors_str = extract_json_string(obj, b"authors");

                    if written > 0 {
                        if !write_bytes(payload, &mut c, b",") {
                            break;
                        }
                    }
                    let ok = write_bytes(payload, &mut c, br#"{"id":""#)
                        && append_json_escaped(payload, &mut c, item_id)
                        && write_bytes(payload, &mut c, br#"","title":""#)
                        && append_json_escaped(payload, &mut c, title)
                        && write_bytes(payload, &mut c, br#"","cover":{"kind":"url","url":""#)
                        && append_json_escaped(payload, &mut c, cover)
                        && write_bytes(payload, &mut c, br#""},"authors":["#);
                    if !ok {
                        break;
                    }
                    if let Some(auth) = authors_str {
                        let _ = format_authors_to_payload(payload, &mut c, auth);
                    }
                    let ok2 = write_bytes(payload, &mut c, br#"],"status":""#)
                        && append_json_escaped(payload, &mut c, status)
                        && write_bytes(
                            payload,
                            &mut c,
                            br#"","contentRating":"safe","sourceTags":["zaimanhua"]}"#,
                        );
                    if !ok2 {
                        break;
                    }
                    written += 1;
                }
            }
        } else if after < api_json.len() && api_json[after] == b'{' {
            // Data is an object with "list" or "comicList"
            let data_obj = &api_json[after..];
            let items_arr = extract_json_array_content(data_obj, b"list")
                .or_else(|| extract_json_array_content(data_obj, b"comicList"));
            if let Some(items_data) = items_arr {
                let mut raw_pos = 0usize;
                while let Some(obj) = next_raw_object(items_data, &mut raw_pos) {
                    if written >= DEFAULT_PAGE_SIZE {
                        break;
                    }
                    if let Some(item_id) = extract_item_id(obj) {
                        let title = extract_json_string(obj, b"title")
                            .or_else(|| extract_json_string(obj, b"name"))
                            .unwrap_or(b"Unknown");
                        let cover = extract_json_string(obj, b"cover").unwrap_or(b"");
                        let status_str = extract_json_string(obj, b"status").unwrap_or(b"");
                        let status = parse_status_byte(status_str);
                        let authors_str = extract_json_string(obj, b"authors");

                        if written > 0 {
                            if !write_bytes(payload, &mut c, b",") {
                                break;
                            }
                        }
                        let ok = write_bytes(payload, &mut c, br#"{"id":""#)
                            && append_json_escaped(payload, &mut c, item_id)
                            && write_bytes(payload, &mut c, br#"","title":""#)
                            && append_json_escaped(payload, &mut c, title)
                            && write_bytes(payload, &mut c, br#"","cover":{"kind":"url","url":""#)
                            && append_json_escaped(payload, &mut c, cover)
                            && write_bytes(payload, &mut c, br#""},"authors":["#);
                        if !ok {
                            break;
                        }
                        if let Some(auth) = authors_str {
                            let _ = format_authors_to_payload(payload, &mut c, auth);
                        }
                        let ok2 = write_bytes(payload, &mut c, br#"],"status":""#)
                            && append_json_escaped(payload, &mut c, status)
                            && write_bytes(
                                payload,
                                &mut c,
                                br#"","contentRating":"safe","sourceTags":["zaimanhua"]}"#,
                            );
                        if !ok2 {
                            break;
                        }
                        written += 1;
                    }
                }
            }
        }
    }

    let has_more = written >= DEFAULT_PAGE_SIZE;
    let ok_close = write_bytes(payload, &mut c, br#"],"page":{"nextCursor":""#)
        && write_usize(payload, &mut c, page_num + 1)
        && write_bytes(payload, &mut c, br#"","hasMore":"#);
    if !ok_close {
        return write_error("get_manga_list", "internal_error", "overflow");
    }
    if has_more {
        if !write_bytes(payload, &mut c, b"true}}") {
            return write_error("get_manga_list", "internal_error", "overflow");
        }
    } else {
        if !write_bytes(payload, &mut c, b"false}}") {
            return write_error("get_manga_list", "internal_error", "overflow");
        }
    }
    write_success_payload("get_manga_list", c)
}

fn run_get_home(_req: &[u8]) -> u32 {
    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"sections":["#) {
        return write_error("get_home", "internal_error", "overflow");
    }

    // Fetch latest updates as home section
    let url_latest = b"https://v4api.zaimanhua.com/app/v1/comic/update/list/0/1";

    let ok = write_bytes(payload, &mut c, b"{\"title\":\"")
        && write_bytes(payload, &mut c, &[0xe6, 0x9c, 0x80, 0xe6, 0x96, 0xb0, 0xe6, 0x9b, 0xb4, 0xe6, 0x96, 0xb0]) // 最新更新
        && write_bytes(payload, &mut c, b"\",\"items\":[");
    if !ok {
        return write_error("get_home", "internal_error", "overflow");
    }

    if let Ok(api_json) = fetch_json(url_latest) {
        let data_marker = b"\"data\":[";
        if let Some(idx) = find_subslice(api_json, data_marker) {
            let arr_start = idx + data_marker.len();
            let arr_content = &api_json[arr_start..];
            let mut raw_pos = 0usize;
            let mut count = 0usize;
            while let Some(obj) = next_raw_object(arr_content, &mut raw_pos) {
                if count >= 10 {
                    break;
                }
                if let Some(item_id) = extract_item_id(obj) {
                    let title = extract_json_string(obj, b"title")
                        .or_else(|| extract_json_string(obj, b"name"))
                        .unwrap_or(b"Unknown");
                    let cover = extract_json_string(obj, b"cover").unwrap_or(b"");

                    if count > 0 {
                        if !write_bytes(payload, &mut c, b",") {
                            break;
                        }
                    }
                    let ok2 = write_bytes(payload, &mut c, br#"{"id":""#)
                        && append_json_escaped(payload, &mut c, item_id)
                        && write_bytes(payload, &mut c, br#"","title":""#)
                        && append_json_escaped(payload, &mut c, title)
                        && write_bytes(payload, &mut c, br#"","cover":{"kind":"url","url":""#)
                        && append_json_escaped(payload, &mut c, cover)
                        && write_bytes(payload, &mut c, br#""}}"#);
                    if !ok2 {
                        break;
                    }
                    count += 1;
                }
            }
        }
    }

    if !write_bytes(payload, &mut c, b"]}") {
        return write_error("get_home", "internal_error", "overflow");
    }

    // Popular ranking section
    let url_popular =
        b"https://v4api.zaimanhua.com/app/v1/comic/rank/list?tag_id=0&rank_type=0&page=1";
    if !write_bytes(payload, &mut c, b",") {
        return write_error("get_home", "internal_error", "overflow");
    }
    let ok = write_bytes(payload, &mut c, b"{\"title\":\"")
        && write_bytes(payload, &mut c, &[0xe4, 0xba, 0xba, 0xe6, 0xb0, 0x94, 0xe6, 0x8e, 0x92, 0xe8, 0xa1, 0x8c]) // 人气排行
        && write_bytes(payload, &mut c, b"\",\"items\":[");
    if !ok {
        return write_error("get_home", "internal_error", "overflow");
    }

    if let Ok(api_json) = fetch_json(url_popular) {
        let data_marker = b"\"data\":{";
        if let Some(idx) = find_subslice(api_json, data_marker) {
            let data_start = idx + b"\"data\":".len() - 1;
            let data_obj = &api_json[data_start..];
            if let Some(items_data) = extract_json_array_content(data_obj, b"list")
                .or_else(|| extract_json_array_content(data_obj, b"comicList"))
            {
                let mut raw_pos = 0usize;
                let mut count = 0usize;
                while let Some(obj) = next_raw_object(items_data, &mut raw_pos) {
                    if count >= 10 {
                        break;
                    }
                    if let Some(item_id) = extract_item_id(obj) {
                        let title = extract_json_string(obj, b"title")
                            .or_else(|| extract_json_string(obj, b"name"))
                            .unwrap_or(b"Unknown");
                        let cover = extract_json_string(obj, b"cover").unwrap_or(b"");

                        if count > 0 {
                            if !write_bytes(payload, &mut c, b",") {
                                break;
                            }
                        }
                        let ok2 = write_bytes(payload, &mut c, br#"{"id":""#)
                            && append_json_escaped(payload, &mut c, item_id)
                            && write_bytes(payload, &mut c, br#"","title":""#)
                            && append_json_escaped(payload, &mut c, title)
                            && write_bytes(payload, &mut c, br#"","cover":{"kind":"url","url":""#)
                            && append_json_escaped(payload, &mut c, cover)
                            && write_bytes(payload, &mut c, br#""}}"#);
                        if !ok2 {
                            break;
                        }
                        count += 1;
                    }
                }
            }
        }
    }

    if !write_bytes(payload, &mut c, b"]}") {
        return write_error("get_home", "internal_error", "overflow");
    }

    if !write_bytes(payload, &mut c, b"]}") {
        return write_error("get_home", "internal_error", "overflow");
    }
    write_success_payload("get_home", c)
}

fn run_get_filters(_req: &[u8]) -> u32 {
    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"filters":[{"id":"sortType","name":""#)
        // 排序
        && write_bytes(payload, &mut c, &[0xe6, 0x8e, 0x92, 0xe5, 0xba, 0x8f])
        && write_bytes(payload, &mut c, br#"","kind":"select","options":[{"value":"1","label":""#)
        // 更新排序
        && write_bytes(payload, &mut c, &[0xe6, 0x9b, 0xb4, 0xe6, 0x96, 0xb0, 0xe6, 0x8e, 0x92, 0xe5, 0xba, 0x8f])
        && write_bytes(payload, &mut c, br#""},{"value":"2","label":""#)
        // 人气排序
        && write_bytes(payload, &mut c, &[0xe4, 0xba, 0xba, 0xe6, 0xb0, 0x94, 0xe6, 0x8e, 0x92, 0xe5, 0xba, 0x8f])
        && write_bytes(payload, &mut c, br#""}],"default":"1"},{"id":"status","name":""#)
        // 进度
        && write_bytes(payload, &mut c, &[0xe8, 0xbf, 0x9b, 0xe5, 0xba, 0xa6])
        && write_bytes(payload, &mut c, br#"","kind":"select","options":[{"value":"0","label":""#)
        // 全部
        && write_bytes(payload, &mut c, &[0xe5, 0x85, 0xa8, 0xe9, 0x83, 0xa8])
        && write_bytes(payload, &mut c, br#""},{"value":"2309","label":""#)
        // 连载中
        && write_bytes(payload, &mut c, &[0xe8, 0xbf, 0x9e, 0xe8, 0xbd, 0xbd, 0xe4, 0xb8, 0xad])
        && write_bytes(payload, &mut c, br#""},{"value":"2310","label":""#)
        // 已完结
        && write_bytes(payload, &mut c, &[0xe5, 0xb7, 0xb2, 0xe5, 0xae, 0x8c, 0xe7, 0xbb, 0x93])
        && write_bytes(payload, &mut c, br#""},{"value":"29205","label":""#)
        // 短篇
        && write_bytes(payload, &mut c, &[0xe7, 0x9f, 0xad, 0xe7, 0xaf, 0x87])
        && write_bytes(payload, &mut c, br#""}],"default":"0"},{"id":"cate","name":""#)
        // 读者群
        && write_bytes(payload, &mut c, &[0xe8, 0xaf, 0xbb, 0xe8, 0x80, 0x85, 0xe7, 0xbe, 0xa4])
        && write_bytes(payload, &mut c, br#"","kind":"select","options":[{"value":"0","label":""#)
        // 全部
        && write_bytes(payload, &mut c, &[0xe5, 0x85, 0xa8, 0xe9, 0x83, 0xa8])
        && write_bytes(payload, &mut c, br#""},{"value":"3262","label":""#)
        // 少年漫画
        && write_bytes(payload, &mut c, &[0xe5, 0xb0, 0x91, 0xe5, 0xb9, 0xb4, 0xe6, 0xbc, 0xab, 0xe7, 0x94, 0xbb])
        && write_bytes(payload, &mut c, br#""},{"value":"3263","label":""#)
        // 少女漫画
        && write_bytes(payload, &mut c, &[0xe5, 0xb0, 0x91, 0xe5, 0xa5, 0xb3, 0xe6, 0xbc, 0xab, 0xe7, 0x94, 0xbb])
        && write_bytes(payload, &mut c, br#""},{"value":"3264","label":""#)
        // 青年漫画
        && write_bytes(payload, &mut c, &[0xe9, 0x9d, 0x92, 0xe5, 0xb9, 0xb4, 0xe6, 0xbc, 0xab, 0xe7, 0x94, 0xbb])
        && write_bytes(payload, &mut c, br#""},{"value":"13626","label":""#)
        // 女青漫画
        && write_bytes(payload, &mut c, &[0xe5, 0xa5, 0xb3, 0xe9, 0x9d, 0x92, 0xe6, 0xbc, 0xab, 0xe7, 0x94, 0xbb])
        && write_bytes(payload, &mut c, br#""}],"default":"0"},{"id":"zone","name":""#)
        // 地区
        && write_bytes(payload, &mut c, &[0xe5, 0x9c, 0xb0, 0xe5, 0x8c, 0xba])
        && write_bytes(payload, &mut c, br#"","kind":"select","options":[{"value":"0","label":""#)
        // 全部
        && write_bytes(payload, &mut c, &[0xe5, 0x85, 0xa8, 0xe9, 0x83, 0xa8])
        && write_bytes(payload, &mut c, br#""},{"value":"2304","label":""#)
        // 日本
        && write_bytes(payload, &mut c, &[0xe6, 0x97, 0xa5, 0xe6, 0x9c, 0xac])
        && write_bytes(payload, &mut c, br#""},{"value":"2305","label":""#)
        // 韩国
        && write_bytes(payload, &mut c, &[0xe9, 0x9f, 0xa9, 0xe5, 0x9b, 0xbd])
        && write_bytes(payload, &mut c, br#""},{"value":"2306","label":""#)
        // 欧美
        && write_bytes(payload, &mut c, &[0xe6, 0xac, 0xa7, 0xe7, 0xbe, 0x8e])
        && write_bytes(payload, &mut c, br#""},{"value":"2307","label":""#)
        // 港台
        && write_bytes(payload, &mut c, &[0xe6, 0xb8, 0xaf, 0xe5, 0x8f, 0xb0])
        && write_bytes(payload, &mut c, br#""},{"value":"2308","label":""#)
        // 内地
        && write_bytes(payload, &mut c, &[0xe5, 0x86, 0x85, 0xe5, 0x9c, 0xb0])
        && write_bytes(payload, &mut c, br#""},{"value":"8435","label":""#)
        // 其他
        && write_bytes(payload, &mut c, &[0xe5, 0x85, 0xb6, 0xe4, 0xbb, 0x96])
        && write_bytes(payload, &mut c, br#""}],"default":"0"}]}"#);
    if !ok {
        return write_error("get_filters", "internal_error", "overflow");
    }
    write_success_payload("get_filters", c)
}

fn run_image_request(req: &[u8]) -> u32 {
    let url = match extract_json_string(req, b"url") {
        Some(u) => u,
        None => return write_error("get_image_request", "invalid_request", "missing url"),
    };

    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"url":""#)
        && append_json_escaped(payload, &mut c, url)
        && write_bytes(
            payload,
            &mut c,
            br#"","headers":{"Referer":"https://manhua.zaimanhua.com/"}}"#,
        );
    if !ok {
        return write_error("get_image_request", "internal_error", "overflow");
    }
    write_success_payload("get_image_request", c)
}

// --- Exports ---

#[no_mangle]
pub extern "C" fn koma_source_init(_manifest_ptr: u32, manifest_len: u32) -> i32 {
    log_info(b"zaimanhua source init");
    if host::check_cancel() {
        return -2;
    }
    if manifest_len > 0 {
        0
    } else {
        -1
    }
}

#[no_mangle]
pub extern "C" fn koma_source_info() -> u32 {
    response_buffer().write_source_metadata(&SOURCE_INFO, &SOURCE_CAPS)
}

#[no_mangle]
pub extern "C" fn koma_source_search(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("search", "invalid_request", "empty"),
    };
    log_info(b"zaimanhua search");
    run_search(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga", "invalid_request", "empty"),
    };
    log_info(b"zaimanhua get_manga");
    run_get_manga(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_chapters(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_chapters", "invalid_request", "empty"),
    };
    log_info(b"zaimanhua get_chapters");
    run_get_chapters(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_pages(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_pages", "invalid_request", "empty"),
    };
    log_info(b"zaimanhua get_pages");
    run_get_pages(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_listings(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_listings", "invalid_request", "empty"),
    };
    log_info(b"zaimanhua get_listings");
    run_get_listings(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga_list(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga_list", "invalid_request", "empty"),
    };
    log_info(b"zaimanhua get_manga_list");
    run_get_manga_list(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_home(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_home", "invalid_request", "empty"),
    };
    log_info(b"zaimanhua get_home");
    run_get_home(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_filters(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_filters", "invalid_request", "empty"),
    };
    log_info(b"zaimanhua get_filters");
    run_get_filters(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_image_request(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_image_request", "invalid_request", "empty"),
    };
    log_info(b"zaimanhua get_image_request");
    run_image_request(req)
}

#[no_mangle]
pub extern "C" fn koma_source_free(result_ptr: u32) {
    response_buffer().free(result_ptr)
}
