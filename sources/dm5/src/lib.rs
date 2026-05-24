#![no_std]

extern crate koma_source_sdk;

use koma_source_sdk::host::{self, http_request, log_info};
use koma_source_sdk::json_utils::{
    append_json_escaped, contains_bytes, extract_json_number, extract_json_string, find_subslice,
    write_bytes, write_url_encoded, write_usize,
};
use koma_source_sdk::result::ResultBuffer;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};

const BASE_URL: &[u8] = b"https://www.dm5.cn";
const PAYLOAD_CAP: usize = 1024 * 1024;
const HTTP_OUT_CAP: usize = 2 * 1024 * 1024;
const BODY_CAP: usize = 2 * 1024 * 1024;
const HTTP_REQ_CAP: usize = 2048;
const SCRATCH_CAP: usize = 8192;

static mut RESPONSE: ResultBuffer<{ PAYLOAD_CAP + 256 }> = ResultBuffer::new();
static mut PAYLOAD_BUF: [u8; PAYLOAD_CAP] = [0; PAYLOAD_CAP];
static mut HTTP_OUT: [u8; HTTP_OUT_CAP] = [0; HTTP_OUT_CAP];
static mut BODY_BUF: [u8; BODY_CAP] = [0; BODY_CAP];
static mut HTTP_REQ_BUF: [u8; HTTP_REQ_CAP] = [0; HTTP_REQ_CAP];
static mut SCRATCH_A: [u8; SCRATCH_CAP] = [0; SCRATCH_CAP];
static mut SCRATCH_B: [u8; SCRATCH_CAP] = [0; SCRATCH_CAP];

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.dm5.koma",
    name: "DM5",
    version: "0.1.0",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "DM5 HTML scraping source.",
    content_rating: "nsfw",
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
    settings: true,
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
fn body_buf() -> &'static mut [u8] {
    unsafe { &mut *core::ptr::addr_of_mut!(BODY_BUF) }
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
fn payload_slice(len: usize) -> &'static [u8] {
    unsafe { core::slice::from_raw_parts(PAYLOAD_BUF.as_ptr(), len) }
}

fn write_error(operation: &str, code: &str, message: &str) -> u32 {
    response_buffer().write_error(operation, code, message)
}
fn write_success_payload(operation: &str, len: usize) -> u32 {
    response_buffer().write_success(operation, payload_slice(len))
}
fn read_request<'a>(req_ptr: u32, req_len: u32) -> Option<&'a [u8]> {
    if req_ptr == 0 || req_len == 0 {
        return None;
    }
    Some(unsafe { core::slice::from_raw_parts(req_ptr as *const u8, req_len as usize) })
}

fn trim_ascii(bytes: &[u8]) -> &[u8] {
    let mut start = 0usize;
    let mut end = bytes.len();
    while start < end && matches!(bytes[start], b' ' | b'\t' | b'\n' | b'\r') {
        start += 1;
    }
    while end > start && matches!(bytes[end - 1], b' ' | b'\t' | b'\n' | b'\r') {
        end -= 1;
    }
    &bytes[start..end]
}

fn build_get_request(dst: &mut [u8], url: &[u8], referer: Option<&[u8]>) -> Option<usize> {
    let mut cursor = 0usize;
    write_bytes(dst, &mut cursor, br#"{"version":1,"method":"GET","url":""#).then_some(())?;
    append_json_escaped(dst, &mut cursor, url).then_some(())?;
    write_bytes(dst, &mut cursor, br#"","headers":{"User-Agent":"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/139.0.0.0 Safari/537.36","Accept-Language":"zh-TW""#).then_some(())?;
    if let Some(r) = referer {
        write_bytes(dst, &mut cursor, br#","Referer":""#).then_some(())?;
        append_json_escaped(dst, &mut cursor, r).then_some(())?;
        write_bytes(dst, &mut cursor, b"\"").then_some(())?;
    }
    write_bytes(
        dst,
        &mut cursor,
        br#"},"timeoutMs":15000,"responseKind":"bodyText"}"#,
    )
    .then_some(())?;
    Some(cursor)
}

#[derive(Copy, Clone)]
enum FetchError {
    Network,
    NotFound,
    RateLimit,
    ClientError,
    ServerError,
}

fn parse_status_code(bytes: &[u8]) -> u16 {
    let mut n = 0u16;
    for &b in bytes {
        if b >= b'0' && b <= b'9' {
            n = n * 10 + (b - b'0') as u16;
        }
    }
    n
}

fn fetch_error_code(e: FetchError) -> (&'static str, &'static str) {
    match e {
        FetchError::Network => ("network_error", "connection or timeout failure"),
        FetchError::NotFound => ("not_found", "resource not found"),
        FetchError::RateLimit => ("rate_limited", "rate limited by server"),
        FetchError::ClientError => ("client_error", "client error (4xx)"),
        FetchError::ServerError => ("server_error", "server error (5xx)"),
    }
}

fn decode_json_body(resp: &[u8]) -> Result<usize, FetchError> {
    if !contains_bytes(resp, br#""ok":true"#) {
        let err = if let Some(code_bytes) = extract_json_number(resp, b"statusCode") {
            match parse_status_code(code_bytes) {
                404 => FetchError::NotFound,
                429 => FetchError::RateLimit,
                400..=499 => FetchError::ClientError,
                500..=599 => FetchError::ServerError,
                _ => FetchError::Network,
            }
        } else {
            FetchError::Network
        };
        return Err(err);
    }
    let marker = b"\"bodyText\":\"";
    let mut i = find_subslice(resp, marker).ok_or(FetchError::Network)? + marker.len();
    let dst = body_buf();
    let mut out = 0usize;
    while i < resp.len() {
        let b = resp[i];
        if b == b'\\' && i + 1 < resp.len() {
            let next = resp[i + 1];
            match next {
                b'"' => {
                    if out >= dst.len() {
                        return Err(FetchError::Network);
                    }
                    dst[out] = b'"';
                    out += 1;
                    i += 2;
                }
                b'\\' => {
                    if out >= dst.len() {
                        return Err(FetchError::Network);
                    }
                    dst[out] = b'\\';
                    out += 1;
                    i += 2;
                }
                b'/' => {
                    if out >= dst.len() {
                        return Err(FetchError::Network);
                    }
                    dst[out] = b'/';
                    out += 1;
                    i += 2;
                }
                b'n' | b'r' | b't' => {
                    if out >= dst.len() {
                        return Err(FetchError::Network);
                    }
                    dst[out] = if next == b'n' {
                        b'\n'
                    } else if next == b'r' {
                        b'\r'
                    } else {
                        b'\t'
                    };
                    out += 1;
                    i += 2;
                }
                b'u' => {
                    if i + 5 >= resp.len() || out + 3 > dst.len() {
                        return Err(FetchError::Network);
                    }
                    let mut code = 0u32;
                    let mut k = 0usize;
                    while k < 4 {
                        let h = resp[i + 2 + k];
                        let v = match h {
                            b'0'..=b'9' => (h - b'0') as u32,
                            b'a'..=b'f' => (h - b'a' + 10) as u32,
                            b'A'..=b'F' => (h - b'A' + 10) as u32,
                            _ => return Err(FetchError::Network),
                        };
                        code = (code << 4) | v;
                        k += 1;
                    }
                    if code < 0x80 {
                        dst[out] = code as u8;
                        out += 1;
                    } else if code < 0x800 {
                        dst[out] = 0xC0 | (code >> 6) as u8;
                        dst[out + 1] = 0x80 | (code & 0x3F) as u8;
                        out += 2;
                    } else {
                        dst[out] = 0xE0 | (code >> 12) as u8;
                        dst[out + 1] = 0x80 | ((code >> 6) & 0x3F) as u8;
                        dst[out + 2] = 0x80 | (code & 0x3F) as u8;
                        out += 3;
                    }
                    i += 6;
                }
                _ => {
                    if out >= dst.len() {
                        return Err(FetchError::Network);
                    }
                    dst[out] = next;
                    out += 1;
                    i += 2;
                }
            }
            continue;
        }
        if b == b'"' {
            return Ok(out);
        }
        if out >= dst.len() {
            return Err(FetchError::Network);
        }
        dst[out] = b;
        out += 1;
        i += 1;
    }
    Err(FetchError::Network)
}

fn fetch_body(url: &[u8], referer: Option<&[u8]>) -> Result<usize, FetchError> {
    let req_len = build_get_request(http_req_buf(), url, referer).ok_or(FetchError::Network)?;
    let mut resp_len = 0usize;
    let mut transport_failed = true;
    for attempt in 0..3u8 {
        match http_request(&http_req_buf()[..req_len], http_out()) {
            Ok(n) => {
                resp_len = n;
                transport_failed = false;
                break;
            }
            Err(_) => {
                if attempt < 2 {
                    log_info(b"DM5: http transport error, retrying");
                }
            }
        }
    }
    if transport_failed {
        return Err(FetchError::Network);
    }
    decode_json_body(&http_out()[..resp_len])
}

fn make_url(parts: &[&[u8]]) -> Option<usize> {
    let buf = scratch_a();
    let mut c = 0usize;
    for part in parts {
        write_bytes(buf, &mut c, part).then_some(())?;
    }
    Some(c)
}

fn manga_id_from_href(href: &[u8]) -> Option<&[u8]> {
    if href.len() < 10 || href[0] != b'/' || !href[1..].starts_with(b"manhua-") {
        return None;
    }
    let mut end = 1usize;
    while end < href.len() && href[end] != b'/' && href[end] != b'?' && href[end] != b'#' {
        end += 1;
    }
    Some(&href[1..end])
}

fn chapter_id_from_href(href: &[u8]) -> Option<&[u8]> {
    if href.len() < 4 || href[0] != b'/' || href[1] != b'm' {
        return None;
    }
    let mut end = 2usize;
    while end < href.len() && href[end] >= b'0' && href[end] <= b'9' {
        end += 1;
    }
    if end == 2 {
        None
    } else {
        Some(&href[2..end])
    }
}

fn attr_value<'a>(tag: &'a [u8], attr: &[u8]) -> Option<&'a [u8]> {
    let pos = find_subslice(tag, attr)?;
    let mut i = pos + attr.len();
    while i < tag.len() && matches!(tag[i], b' ' | b'\t' | b'\n' | b'\r') {
        i += 1;
    }
    if i >= tag.len() || tag[i] != b'=' {
        return None;
    }
    i += 1;
    while i < tag.len() && matches!(tag[i], b' ' | b'\t' | b'\n' | b'\r') {
        i += 1;
    }
    if i >= tag.len() {
        return None;
    }
    let quote = tag[i];
    if quote != b'"' && quote != b'\'' {
        return None;
    }
    i += 1;
    let start = i;
    while i < tag.len() && tag[i] != quote {
        i += 1;
    }
    if i >= tag.len() {
        None
    } else {
        Some(&tag[start..i])
    }
}

fn strip_tags_to<'a>(src: &[u8], out: &'a mut [u8]) -> &'a [u8] {
    let mut c = 0usize;
    let mut in_tag = false;
    for &b in src {
        if b == b'<' {
            in_tag = true;
            continue;
        }
        if b == b'>' {
            in_tag = false;
            continue;
        }
        if !in_tag && c < out.len() {
            out[c] = b;
            c += 1;
        }
    }
    trim_ascii(&out[..c])
}

fn write_card_item(
    payload: &mut [u8],
    c: &mut usize,
    id: &[u8],
    title: &[u8],
    cover: &[u8],
    tag: &[u8],
) -> bool {
    write_bytes(payload, c, br#"{"id":"manga:"#)
        && append_json_escaped(payload, c, id)
        && write_bytes(payload, c, br#"","title":""#)
        && append_json_escaped(payload, c, title)
        && write_bytes(payload, c, br#"","cover":{"kind":"url","url":""#)
        && append_json_escaped(payload, c, cover)
        && write_bytes(payload, c, br#""},"authors":[],"status":"unknown","contentRating":"nsfw","description":"","sourceTags":["#)
        && write_bytes(payload, c, b"\"")
        && append_json_escaped(payload, c, tag)
        && write_bytes(payload, c, br#""]}"#)
}

fn parse_cards(html: &[u8], payload: &mut [u8], c: &mut usize, max: usize, tag: &[u8]) -> usize {
    let mut pos = 0usize;
    let mut written = 0usize;
    let mut last_id = b"" as &[u8];
    while written < max {
        let rel = match find_subslice(&html[pos..], b"<a ") {
            Some(v) => v,
            None => break,
        };
        let a_start = pos + rel;
        let a_end = match find_subslice(&html[a_start..], b">") {
            Some(v) => a_start + v + 1,
            None => break,
        };
        pos = a_end;
        let a_tag = &html[a_start..a_end];
        let href = match attr_value(a_tag, b"href") {
            Some(v) => v,
            None => continue,
        };
        let id = match manga_id_from_href(href) {
            Some(v) => v,
            None => continue,
        };
        if id == last_id {
            continue;
        }
        let title = match attr_value(a_tag, b"title") {
            Some(v) => v,
            None => continue,
        };
        let mut cover = b"" as &[u8];
        let search_start = a_start.saturating_sub(700);
        let before = &html[search_start..a_start];
        if let Some(img_rel) = find_last_subslice(before, b"<img") {
            let img_start = search_start + img_rel;
            let img_end = find_subslice(&html[img_start..], b">")
                .map(|v| img_start + v + 1)
                .unwrap_or(a_start);
            cover = attr_value(&html[img_start..img_end], b"src").unwrap_or(b"");
        } else if let Some(style_rel) = find_last_subslice(before, b"background-image: url(") {
            let start = search_start + style_rel + b"background-image: url(".len();
            if let Some(end_rel) = find_subslice(&html[start..a_start], b")") {
                cover = trim_ascii(&html[start..start + end_rel]);
            }
        }
        if written > 0 && !write_bytes(payload, c, b",") {
            break;
        }
        if !write_card_item(payload, c, id, title, cover, tag) {
            break;
        }
        last_id = id;
        written += 1;
    }
    written
}

fn find_last_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    let mut result = None;
    let mut pos = 0usize;
    while pos + needle.len() <= haystack.len() {
        if &haystack[pos..pos + needle.len()] == needle {
            result = Some(pos);
        }
        pos += 1;
    }
    result
}

fn write_manga_page_from_url(operation: &str, url: &[u8], max: usize) -> u32 {
    let html_len = match fetch_body(url, None) {
        Ok(n) => n,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error(operation, c, m);
        }
    };
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };
    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error(operation, "internal_error", "payload overflow");
    }
    let written = parse_cards(html, payload, &mut c, max, b"dm5");
    let has_more = contains_bytes(html, b">>") || contains_bytes(html, b">&gt;<");
    let ok = write_bytes(payload, &mut c, br#"],"page":{"nextCursor":"#)
        && if has_more {
            write_bytes(payload, &mut c, b"\"next\"")
        } else {
            write_bytes(payload, &mut c, b"null")
        }
        && write_bytes(payload, &mut c, br#","hasMore":"#)
        && write_bytes(
            payload,
            &mut c,
            if has_more && written > 0 {
                b"true"
            } else {
                b"false"
            },
        )
        && write_bytes(payload, &mut c, b"}}");
    if !ok {
        return write_error(operation, "internal_error", "payload overflow");
    }
    write_success_payload(operation, c)
}

fn run_search(req: &[u8]) -> u32 {
    let query = extract_json_string(req, b"query").unwrap_or(b"");
    if query.is_empty() {
        return run_get_listings(req);
    }
    if query.starts_with(b"id:") {
        let id = &query[3..];
        let url_len = match make_url(&[BASE_URL, b"/", id, b"/"]) {
            Some(n) => n,
            None => return write_error("search", "internal_error", "url overflow"),
        };
        let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_len) };
        return search_single_manga(url, id);
    }
    let mut c = 0usize;
    let url_buf = scratch_a();
    if !(write_bytes(url_buf, &mut c, BASE_URL)
        && write_bytes(url_buf, &mut c, b"/search?title=")
        && write_url_encoded(url_buf, &mut c, query)
        && write_bytes(url_buf, &mut c, b"&language=1&page=")
        && write_bytes(
            url_buf,
            &mut c,
            extract_json_number(req, b"page").unwrap_or(b"1"),
        ))
    {
        return write_error("search", "internal_error", "url overflow");
    }
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), c) };
    write_manga_page_from_url("search", url, 100)
}

fn search_single_manga(url: &[u8], id: &[u8]) -> u32 {
    let html_len = match fetch_body(url, None) {
        Ok(n) => n,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("search", c, m);
        }
    };
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };
    let mut title_buf = [0u8; 256];
    let title =
        extract_between_text(html, b"<p class=\"title\"", b"</p>", &mut title_buf).unwrap_or(id);
    let cover = extract_attr_after(html, b"class=\"cover\"", b"src").unwrap_or(b"");
    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"items":["#)
        && write_card_item(payload, &mut c, id, title, cover, b"dm5")
        && write_bytes(
            payload,
            &mut c,
            br#"],"page":{"nextCursor":null,"hasMore":false}}"#,
        );
    if !ok {
        return write_error("search", "internal_error", "payload overflow");
    }
    write_success_payload("search", c)
}

fn extract_attr_after<'a>(html: &'a [u8], marker: &[u8], attr: &[u8]) -> Option<&'a [u8]> {
    let p = find_subslice(html, marker)?;
    let tag_start = find_back(html, p, b'<').unwrap_or(p);
    let tag_end = find_subslice(&html[p..], b">")? + p + 1;
    attr_value(&html[tag_start..tag_end], attr)
}

fn find_back(bytes: &[u8], mut pos: usize, needle: u8) -> Option<usize> {
    while pos > 0 {
        if bytes[pos] == needle {
            return Some(pos);
        }
        pos -= 1;
    }
    None
}

fn extract_between_text<'a>(
    html: &[u8],
    marker: &[u8],
    end_marker: &[u8],
    out: &'a mut [u8],
) -> Option<&'a [u8]> {
    let marker_pos = find_subslice(html, marker)?;
    let gt = find_subslice(&html[marker_pos..], b">")? + marker_pos + 1;
    let end = find_subslice(&html[gt..], end_marker)? + gt;
    Some(strip_tags_to(&html[gt..end], out))
}

fn run_get_manga(req: &[u8]) -> u32 {
    let manga_id = match extract_json_string(req, b"mangaId") {
        Some(v) => v,
        None => return write_error("get_manga", "invalid_request", "missing mangaId"),
    };
    if manga_id.len() <= 6 || &manga_id[..6] != b"manga:" {
        return write_error("get_manga", "invalid_request", "unexpected mangaId");
    }
    let id = &manga_id[6..];
    let url_len = match make_url(&[BASE_URL, b"/", id, b"/"]) {
        Some(n) => n,
        None => return write_error("get_manga", "internal_error", "url overflow"),
    };
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_len) };
    let html_len = match fetch_body(url, None) {
        Ok(n) => n,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_manga", c, m);
        }
    };
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };
    let mut title_buf = [0u8; 256];
    let mut desc_buf = [0u8; 2048];
    let title = extract_js_string(html, b"DM5_COMIC_MNAME=\"").unwrap_or_else(|| {
        extract_between_text(html, b"<p class=\"title\"", b"</p>", &mut title_buf).unwrap_or(id)
    });
    let desc =
        extract_between_text(html, b"<p class=\"content\"", b"</p>", &mut desc_buf).unwrap_or(b"");
    let cover = extract_attr_after(html, b"class=\"cover\"", b"src").unwrap_or(b"");
    let status = if contains_bytes(html, "已完结".as_bytes())
        || contains_bytes(html, "已完結".as_bytes())
    {
        b"completed" as &[u8]
    } else if contains_bytes(html, "连载中".as_bytes()) || contains_bytes(html, "連載中".as_bytes())
    {
        b"ongoing" as &[u8]
    } else {
        b"unknown" as &[u8]
    };
    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"manga":{"id":"manga:"#)
        && append_json_escaped(payload, &mut c, id)
        && write_bytes(payload, &mut c, br#"","title":""#)
        && append_json_escaped(payload, &mut c, title)
        && write_bytes(payload, &mut c, br#"","alternateTitles":[],"description":""#)
        && append_json_escaped(payload, &mut c, desc)
        && write_bytes(payload, &mut c, br#"","cover":{"kind":"url","url":""#)
        && append_json_escaped(payload, &mut c, cover)
        && write_bytes(payload, &mut c, br#""},"authors":[],"artists":[],"status":""#)
        && write_bytes(payload, &mut c, status)
        && write_bytes(payload, &mut c, br#"","contentRating":"nsfw","language":"zh","tags":[],"links":[{"kind":"source","url":""#)
        && append_json_escaped(payload, &mut c, url)
        && write_bytes(payload, &mut c, br#""}]}}"#);
    if !ok {
        return write_error("get_manga", "internal_error", "payload overflow");
    }
    write_success_payload("get_manga", c)
}

fn run_get_chapters(req: &[u8]) -> u32 {
    let manga_id = match extract_json_string(req, b"mangaId") {
        Some(v) => v,
        None => return write_error("get_chapters", "invalid_request", "missing mangaId"),
    };
    if manga_id.len() <= 6 || &manga_id[..6] != b"manga:" {
        return write_error("get_chapters", "invalid_request", "unexpected mangaId");
    }
    let id = &manga_id[6..];
    let url_len = match make_url(&[BASE_URL, b"/", id, b"/"]) {
        Some(n) => n,
        None => return write_error("get_chapters", "internal_error", "url overflow"),
    };
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_len) };
    let html_len = match fetch_body(url, None) {
        Ok(n) => n,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_chapters", c, m);
        }
    };
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };
    let list_start = match find_subslice(html, b"id=\"chapterlistload\"") {
        Some(v) => v,
        None => return write_error("get_chapters", "parse_error", "chapter list not found"),
    };
    let list_end = find_subslice(&html[list_start..], b"</div>")
        .map(|v| list_start + v)
        .unwrap_or(html.len());
    let list = &html[list_start..list_end];
    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("get_chapters", "internal_error", "payload overflow");
    }
    let mut pos = 0usize;
    let mut written = 0usize;
    while let Some(rel) = find_subslice(&list[pos..], b"<a ") {
        let a_start = pos + rel;
        let a_end = match find_subslice(&list[a_start..], b"</a>") {
            Some(v) => a_start + v + 4,
            None => break,
        };
        let block = &list[a_start..a_end];
        pos = a_end;
        let tag_end = match find_subslice(block, b">") {
            Some(v) => v + 1,
            None => continue,
        };
        let href = match attr_value(&block[..tag_end], b"href") {
            Some(v) => v,
            None => continue,
        };
        let cid = match chapter_id_from_href(href) {
            Some(v) => v,
            None => continue,
        };
        let mut title_buf = [0u8; 256];
        let title = strip_tags_to(
            &block[tag_end..block.len().saturating_sub(4)],
            &mut title_buf,
        );
        let page_count = extract_page_count(title);
        if written > 0 && !write_bytes(payload, &mut c, b",") {
            break;
        }
        let ok = write_bytes(payload, &mut c, br#"{"id":"chapter:"#)
            && append_json_escaped(payload, &mut c, id)
            && write_bytes(payload, &mut c, b":")
            && append_json_escaped(payload, &mut c, cid)
            && write_bytes(payload, &mut c, b":")
            && write_usize(payload, &mut c, page_count)
            && write_bytes(payload, &mut c, br#"","mangaId":"manga:"#)
            && append_json_escaped(payload, &mut c, id)
            && write_bytes(payload, &mut c, br#"","title":""#)
            && append_json_escaped(payload, &mut c, title)
            && write_bytes(payload, &mut c, br#"","chapterNumber":""#)
            && append_json_escaped(payload, &mut c, first_number(title).unwrap_or(cid))
            && write_bytes(payload, &mut c, br#"","volumeNumber":null,"language":"zh","publishedAt":null,"updatedAt":null,"pageCount":"#)
            && if page_count > 0 { write_usize(payload, &mut c, page_count) } else { write_bytes(payload, &mut c, b"null") }
            && write_bytes(payload, &mut c, b"}");
        if !ok {
            break;
        }
        written += 1;
        if written >= 600 {
            break;
        }
    }
    if !write_bytes(
        payload,
        &mut c,
        br#"],"page":{"nextCursor":null,"hasMore":false}}"#,
    ) {
        return write_error("get_chapters", "internal_error", "payload overflow");
    }
    write_success_payload("get_chapters", c)
}

fn extract_page_count(title: &[u8]) -> usize {
    let p_marker = "P".as_bytes();
    let p = match find_subslice(title, p_marker) {
        Some(v) => v,
        None => return 0,
    };
    let mut start = p;
    while start > 0 && title[start - 1] >= b'0' && title[start - 1] <= b'9' {
        start -= 1;
    }
    let mut n = 0usize;
    let mut i = start;
    while i < p {
        n = n * 10 + (title[i] - b'0') as usize;
        i += 1;
    }
    n
}

fn first_number(bytes: &[u8]) -> Option<&[u8]> {
    let mut i = 0usize;
    while i < bytes.len() && !(bytes[i] >= b'0' && bytes[i] <= b'9') {
        i += 1;
    }
    if i >= bytes.len() {
        return None;
    }
    let start = i;
    while i < bytes.len() && ((bytes[i] >= b'0' && bytes[i] <= b'9') || bytes[i] == b'.') {
        i += 1;
    }
    Some(&bytes[start..i])
}

fn run_get_pages(req: &[u8]) -> u32 {
    let chapter_id = match extract_json_string(req, b"chapterId") {
        Some(v) => v,
        None => return write_error("get_pages", "invalid_request", "missing chapterId"),
    };
    if chapter_id.len() <= 8 || &chapter_id[..8] != b"chapter:" {
        return write_error("get_pages", "invalid_request", "unexpected chapterId");
    }
    let rest = &chapter_id[8..];
    let sep = match find_subslice(rest, b":") {
        Some(v) => v,
        None => return write_error("get_pages", "invalid_request", "chapterId missing cid"),
    };
    let manga = &rest[..sep];
    let tail = &rest[sep + 1..];
    let sep2 = find_subslice(tail, b":");
    let cid = if let Some(v) = sep2 { &tail[..v] } else { tail };
    let chapter_url_len = match make_url(&[BASE_URL, b"/m", cid, b"/"]) {
        Some(n) => n,
        None => return write_error("get_pages", "internal_error", "url overflow"),
    };
    let chapter_url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), chapter_url_len) };
    let html_len = match fetch_body(chapter_url, Some(BASE_URL)) {
        Ok(n) => n,
        Err(e) => {
            let (code, msg) = fetch_error_code(e);
            return write_error("get_pages", code, msg);
        }
    };
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };
    let mid_raw = extract_js_number(html, b"DM5_MID=").unwrap_or(b"");
    let dt_raw = extract_js_string(html, b"DM5_VIEWSIGN_DT=\"").unwrap_or(b"");
    let sign_raw = extract_js_string(html, b"DM5_VIEWSIGN=\"").unwrap_or(b"");
    if mid_raw.is_empty() || dt_raw.is_empty() || sign_raw.is_empty() {
        return write_error("get_pages", "parse_error", "missing DM5 chapter signature");
    }
    let mut mid_store = [0u8; 32];
    let mut dt_store = [0u8; 64];
    let mut sign_store = [0u8; 64];
    if mid_raw.len() > mid_store.len()
        || dt_raw.len() > dt_store.len()
        || sign_raw.len() > sign_store.len()
    {
        return write_error("get_pages", "parse_error", "chapter signature too long");
    }
    mid_store[..mid_raw.len()].copy_from_slice(mid_raw);
    dt_store[..dt_raw.len()].copy_from_slice(dt_raw);
    sign_store[..sign_raw.len()].copy_from_slice(sign_raw);
    let mid = &mid_store[..mid_raw.len()];
    let dt = &dt_store[..dt_raw.len()];
    let sign = &sign_store[..sign_raw.len()];
    let encoded_page_count = sep2.and_then(|v| parse_usize(&tail[v + 1..]));
    let page_count = extract_js_number(html, b"DM5_IMAGE_COUNT=")
        .and_then(parse_usize)
        .or_else(|| extract_json_number(req, b"pageCount").and_then(parse_usize))
        .or(encoded_page_count)
        .unwrap_or(300)
        .min(500);
    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"chapterId":"chapter:"#)
        && append_json_escaped(payload, &mut c, manga)
        && write_bytes(payload, &mut c, b":")
        && append_json_escaped(payload, &mut c, cid)
        && write_bytes(payload, &mut c, b":")
        && write_usize(payload, &mut c, page_count)
        && write_bytes(payload, &mut c, br#"","pages":["#);
    if !ok {
        return write_error("get_pages", "internal_error", "payload overflow");
    }
    let mut page = 1usize;
    let mut written = 0usize;
    while page <= page_count {
        let got = match fetch_chapter_images(
            manga,
            cid,
            mid,
            dt,
            sign,
            page,
            payload,
            &mut c,
            &mut written,
        ) {
            Ok(n) => n,
            Err(e) => {
                if written == 0 {
                    let (code, msg) = fetch_error_code(e);
                    return write_error("get_pages", code, msg);
                }
                break;
            }
        };
        if got == 0 {
            break;
        }
        page += got;
        if written >= page_count {
            break;
        }
    }
    if !write_bytes(payload, &mut c, b"]}") {
        return write_error("get_pages", "internal_error", "payload overflow");
    }
    write_success_payload("get_pages", c)
}

fn parse_usize(bytes: &[u8]) -> Option<usize> {
    let mut n = 0usize;
    let mut any = false;
    for &b in bytes {
        if b < b'0' || b > b'9' {
            return None;
        }
        n = n * 10 + (b - b'0') as usize;
        any = true;
    }
    if any {
        Some(n)
    } else {
        None
    }
}

fn fetch_chapter_images(
    manga: &[u8],
    cid: &[u8],
    mid: &[u8],
    dt: &[u8],
    sign: &[u8],
    page: usize,
    payload: &mut [u8],
    c: &mut usize,
    written: &mut usize,
) -> Result<usize, FetchError> {
    let url_buf = scratch_a();
    let mut u = 0usize;
    if !(write_bytes(url_buf, &mut u, BASE_URL)
        && write_bytes(url_buf, &mut u, b"/m")
        && write_bytes(url_buf, &mut u, cid)
        && write_bytes(url_buf, &mut u, b"/chapterfun.ashx?cid=")
        && write_bytes(url_buf, &mut u, cid)
        && write_bytes(url_buf, &mut u, b"&page=")
        && write_usize(url_buf, &mut u, page)
        && write_bytes(url_buf, &mut u, b"&key=&language=1&gtk=6&_cid=")
        && write_bytes(url_buf, &mut u, cid)
        && write_bytes(url_buf, &mut u, b"&_mid=")
        && write_bytes(url_buf, &mut u, mid)
        && write_bytes(url_buf, &mut u, b"&_dt=")
        && write_url_encoded(url_buf, &mut u, dt)
        && write_bytes(url_buf, &mut u, b"&_sign=")
        && write_bytes(url_buf, &mut u, sign))
    {
        return Err(FetchError::Network);
    }
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), u) };
    let referer_buf = scratch_b();
    let mut r = 0usize;
    if !(write_bytes(referer_buf, &mut r, BASE_URL)
        && write_bytes(referer_buf, &mut r, b"/m")
        && write_bytes(referer_buf, &mut r, cid)
        && write_bytes(referer_buf, &mut r, b"/"))
    {
        return Err(FetchError::Network);
    }
    let referer = unsafe { core::slice::from_raw_parts(SCRATCH_B.as_ptr(), r) };
    let js_len = fetch_body(url, Some(referer))?;
    let js = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), js_len) };
    unpack_and_write_images(js, manga, cid, payload, c, written)
}

fn unpack_and_write_images(
    packed: &[u8],
    manga: &[u8],
    cid: &[u8],
    payload: &mut [u8],
    c: &mut usize,
    written: &mut usize,
) -> Result<usize, FetchError> {
    let payload_start = find_subslice(packed, b"}('").ok_or(FetchError::Network)? + 3;
    let payload_end =
        find_subslice(&packed[payload_start..], b"',").ok_or(FetchError::Network)? + payload_start;
    let after = payload_end + 2;
    let mut comma = after;
    while comma < packed.len() && packed[comma] != b',' {
        comma += 1;
    }
    let base = parse_usize(&packed[after..comma]).ok_or(FetchError::Network)?;
    let dict_marker = b"'";
    let dict_start =
        find_subslice(&packed[comma..], dict_marker).ok_or(FetchError::Network)? + comma + 1;
    let dict_end = find_subslice(&packed[dict_start..], b"'.split('|')")
        .ok_or(FetchError::Network)?
        + dict_start;
    let dict = &packed[dict_start..dict_end];
    let mut starts = [0usize; 128];
    let mut lens = [0usize; 128];
    let mut count = 0usize;
    let mut s = 0usize;
    let mut i = 0usize;
    while i <= dict.len() && count < starts.len() {
        if i == dict.len() || dict[i] == b'|' {
            starts[count] = s;
            lens[count] = i - s;
            count += 1;
            s = i + 1;
        }
        i += 1;
    }
    let out = scratch_b();
    let mut o = 0usize;
    let mut p = payload_start;
    while p < payload_end {
        let b = packed[p];
        if is_word(b) {
            let start = p;
            while p < payload_end && is_word(packed[p]) {
                p += 1;
            }
            let idx = parse_base(&packed[start..p], base);
            if idx < count && lens[idx] > 0 {
                if o + lens[idx] > out.len() {
                    return Err(FetchError::Network);
                }
                out[o..o + lens[idx]].copy_from_slice(&dict[starts[idx]..starts[idx] + lens[idx]]);
                o += lens[idx];
            } else {
                let len = p - start;
                if o + len > out.len() {
                    return Err(FetchError::Network);
                }
                out[o..o + len].copy_from_slice(&packed[start..p]);
                o += len;
            }
        } else if b == b'\\' && p + 1 < payload_end {
            if o >= out.len() {
                return Err(FetchError::Network);
            }
            out[o] = packed[p + 1];
            o += 1;
            p += 2;
        } else {
            if o >= out.len() {
                return Err(FetchError::Network);
            }
            out[o] = b;
            o += 1;
            p += 1;
        }
    }
    let unpacked = &out[..o];
    let pix = extract_js_string(unpacked, b"pix=\"").ok_or(FetchError::Network)?;
    let arr_start = find_subslice(unpacked, b"pvalue=[\"").ok_or(FetchError::Network)? + 9;
    let mut pos = arr_start;
    let mut added = 0usize;
    while pos < unpacked.len() {
        let end = match find_subslice(&unpacked[pos..], b"\"") {
            Some(v) => pos + v,
            None => break,
        };
        let path = &unpacked[pos..end];
        if *written > 0 && !write_bytes(payload, c, b",") {
            return Err(FetchError::Network);
        }
        let ok = write_bytes(payload, c, br#"{"id":"page:"#)
            && append_json_escaped(payload, c, manga)
            && write_bytes(payload, c, b":")
            && append_json_escaped(payload, c, cid)
            && write_bytes(payload, c, b":")
            && write_usize(payload, c, *written)
            && write_bytes(payload, c, br#"","index":"#)
            && write_usize(payload, c, *written)
            && write_bytes(payload, c, br#","image":{"kind":"url","url":""#)
            && append_json_escaped(payload, c, pix)
            && append_json_escaped(payload, c, path)
            && write_bytes(payload, c, br#""}}"#);
        if !ok {
            return Err(FetchError::Network);
        }
        *written += 1;
        added += 1;
        pos = end + 1;
        if pos + 2 <= unpacked.len() && &unpacked[pos..pos + 2] == b",\"" {
            pos += 2;
        } else {
            break;
        }
    }
    Ok(added)
}

fn is_word(b: u8) -> bool {
    (b >= b'0' && b <= b'9') || (b >= b'a' && b <= b'z') || (b >= b'A' && b <= b'Z') || b == b'_'
}

fn parse_base(bytes: &[u8], base: usize) -> usize {
    let mut n = 0usize;
    for &b in bytes {
        let v = match b {
            b'0'..=b'9' => (b - b'0') as usize,
            b'a'..=b'z' => (b - b'a' + 10) as usize,
            b'A'..=b'Z' => (b - b'A' + 36) as usize,
            _ => return usize::MAX,
        };
        if v >= base {
            return usize::MAX;
        }
        n = n * base + v;
    }
    n
}

fn extract_js_string<'a>(data: &'a [u8], marker: &[u8]) -> Option<&'a [u8]> {
    let start = find_subslice(data, marker)? + marker.len();
    let end = find_subslice(&data[start..], b"\"")? + start;
    Some(&data[start..end])
}

fn extract_js_number<'a>(data: &'a [u8], marker: &[u8]) -> Option<&'a [u8]> {
    let start = find_subslice(data, marker)? + marker.len();
    let mut end = start;
    while end < data.len() && data[end] >= b'0' && data[end] <= b'9' {
        end += 1;
    }
    if end > start {
        Some(&data[start..end])
    } else {
        None
    }
}

fn run_get_listings(_req: &[u8]) -> u32 {
    let url_len = make_url(&[BASE_URL, b"/manhua-list-p1/"]).unwrap_or(0);
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_len) };
    write_manga_page_from_url("get_listings", url, 100)
}

fn run_get_manga_list(req: &[u8]) -> u32 {
    let listing = extract_json_string(req, b"listingId").unwrap_or(b"popular");
    let page = extract_json_number(req, b"page")
        .and_then(parse_usize)
        .unwrap_or(1);
    let url_buf = scratch_a();
    let mut c = 0usize;
    let ok = write_bytes(url_buf, &mut c, BASE_URL)
        && if listing == b"latest" {
            write_bytes(url_buf, &mut c, b"/manhua-list-s2-p")
        } else {
            write_bytes(url_buf, &mut c, b"/manhua-list-p")
        }
        && write_usize(url_buf, &mut c, page)
        && write_bytes(url_buf, &mut c, b"/");
    if !ok {
        return write_error("get_manga_list", "internal_error", "url overflow");
    }
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), c) };
    write_manga_page_from_url("get_manga_list", url, 100)
}

fn run_get_home(_req: &[u8]) -> u32 {
    const HOME_JSON: &str = r#"{"sections":[{"title":"热门漫画","listingId":"popular"},{"title":"最新更新","listingId":"latest"}]}"#;
    let payload = payload_buf();
    let bytes = HOME_JSON.as_bytes();
    if bytes.len() > payload.len() {
        return write_error("get_home", "internal_error", "payload overflow");
    }
    payload[..bytes.len()].copy_from_slice(bytes);
    write_success_payload("get_home", bytes.len())
}

fn run_get_filters(_req: &[u8]) -> u32 {
    const FILTERS: &[u8] = br#"{"filters":[]}"#;
    let payload = payload_buf();
    if FILTERS.len() > payload.len() {
        return write_error("get_filters", "internal_error", "payload overflow");
    }
    payload[..FILTERS.len()].copy_from_slice(FILTERS);
    write_success_payload("get_filters", FILTERS.len())
}

fn run_get_settings(_req: &[u8]) -> u32 {
    const SETTINGS: &[u8] = br#"{"settings":[]}"#;
    let payload = payload_buf();
    if SETTINGS.len() > payload.len() {
        return write_error("get_settings", "internal_error", "payload overflow");
    }
    payload[..SETTINGS.len()].copy_from_slice(SETTINGS);
    write_success_payload("get_settings", SETTINGS.len())
}

fn run_get_image_request(req: &[u8]) -> u32 {
    let url = match extract_json_string(req, b"url") {
        Some(v) => v,
        None => return write_error("get_image_request", "invalid_request", "missing url"),
    };
    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"url":""#)
        && append_json_escaped(payload, &mut c, url)
        && write_bytes(payload, &mut c, br#"","headers":{"Referer":"https://www.dm5.cn/","Accept-Language":"zh-TW","User-Agent":"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/139.0.0.0 Safari/537.36"}}"#);
    if !ok {
        return write_error("get_image_request", "internal_error", "payload overflow");
    }
    write_success_payload("get_image_request", c)
}

#[no_mangle]
pub extern "C" fn koma_source_init(_manifest_ptr: u32, manifest_len: u32) -> i32 {
    log_info(b"DM5 source init");
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
        None => return write_error("search", "invalid_request", "empty request"),
    };
    log_info(b"DM5 search");
    run_search(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga", "invalid_request", "empty request"),
    };
    log_info(b"DM5 get_manga");
    run_get_manga(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_chapters(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_chapters", "invalid_request", "empty request"),
    };
    log_info(b"DM5 get_chapters");
    run_get_chapters(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_pages(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_pages", "invalid_request", "empty request"),
    };
    log_info(b"DM5 get_pages");
    run_get_pages(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_listings(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_listings", "invalid_request", "empty request"),
    };
    log_info(b"DM5 get_listings");
    run_get_listings(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga_list(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga_list", "invalid_request", "empty request"),
    };
    log_info(b"DM5 get_manga_list");
    run_get_manga_list(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_home(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_home", "invalid_request", "empty request"),
    };
    log_info(b"DM5 get_home");
    run_get_home(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_filters(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_filters", "invalid_request", "empty request"),
    };
    log_info(b"DM5 get_filters");
    run_get_filters(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_settings(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_settings", "invalid_request", "empty request"),
    };
    log_info(b"DM5 get_settings");
    run_get_settings(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_image_request(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_image_request", "invalid_request", "empty request"),
    };
    log_info(b"DM5 get_image_request");
    run_get_image_request(req)
}

#[no_mangle]
pub extern "C" fn koma_source_free(result_ptr: u32) {
    response_buffer().free(result_ptr)
}
