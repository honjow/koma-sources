#![no_std]

extern crate koma_source_sdk;

mod chapter_duplicates;
mod chapter_number;
mod access_signal;
mod cdn_selection;

use koma_source_sdk::host::{
    self, HtmlDescriptor, html_attr, html_close, html_parse, html_select, html_text, http_request,
    log_info,
};
use koma_source_sdk::json_utils::{
    append_json_escaped, contains_bytes, extract_json_number, extract_json_string, find_subslice,
    json_object_member, request_page_number, write_bytes, write_url_encoded, write_usize,
};
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{build_get_request, parse_status_code};

#[derive(Copy, Clone)]
enum FetchError { Network, Http(u16), Access(u16, access_signal::AccessSignal) }

fn fetch_error_code(error: FetchError) -> (&'static str, &'static str) {
    match error {
        FetchError::Network => ("network_error", "connection or timeout failure"),
        FetchError::Access(_, access_signal::AccessSignal::Challenge) => ("challenge_required", "website verification required"),
        FetchError::Access(_, access_signal::AccessSignal::Authentication) => ("auth_required", "website authentication required"),
        FetchError::Http(401 | 403) => ("permission_denied", "source requires access verification"),
        FetchError::Http(404) => ("not_found", "resource not found"),
        FetchError::Http(429) => ("rate_limited", "rate limited by server"),
        FetchError::Http(400..=499) => ("client_error", "client error (4xx)"),
        FetchError::Http(_) => ("server_error", "server error (5xx)"),
    }
}

// Preserve only the observed HTTP status; transport failures have no HTTP status.
fn write_fetch_error(operation: &str, error: FetchError) -> u32 {
    let (code, message) = fetch_error_code(error);
    let status = match error {
        FetchError::Network => return write_error(operation, code, message),
        FetchError::Http(status) | FetchError::Access(status, _) => status,
    };
    let mut envelope = [0u8; 512];
    let mut cursor = 0;
    let ok = write_bytes(&mut envelope, &mut cursor, br#"{"type":"response","version":1,"ok":false,"operation":""#)
        && append_json_escaped(&mut envelope, &mut cursor, operation.as_bytes())
        && write_bytes(&mut envelope, &mut cursor, br#"","error":{"code":""#)
        && append_json_escaped(&mut envelope, &mut cursor, code.as_bytes())
        && write_bytes(&mut envelope, &mut cursor, br#"","message":""#)
        && append_json_escaped(&mut envelope, &mut cursor, message.as_bytes())
        && write_bytes(&mut envelope, &mut cursor, br#"","status":"#)
        && write_usize(&mut envelope, &mut cursor, status as usize)
        && write_bytes(&mut envelope, &mut cursor, br#"},"hostHints":{"network":false},"warnings":[]}"#);
    if !ok { return write_error(operation, "internal_error", "error response overflow"); }
    response_buffer().write(&envelope[..cursor], false)
}

// Additional host import not yet in SDK
#[link(wasm_import_module = "koma_host")]
extern "C" {
    #[link_name = "html_select_all"]
    fn koma_host_html_select_all(
        descriptor: i32,
        selector_ptr: *const u8,
        selector_len: u32,
        out_ptr: *mut u8,
        out_cap: u32,
    ) -> i32;
}

/// Select all matching elements. Returns count; descriptors written to `out` (max out.len()/4).
fn html_select_all(descriptor: i32, selector: &[u8], out: &mut [u8]) -> i32 {
    unsafe {
        koma_host_html_select_all(
            descriptor,
            selector.as_ptr(),
            selector.len() as u32,
            out.as_mut_ptr(),
            out.len() as u32,
        )
    }
}

/// Buffer to hold descriptors returned by html_select_all (up to 4000 results)
static mut SELECT_ALL_BUF: [u8; 16000] = [0; 16000]; // 4000 * 4 bytes

const SITE_BASE: &[u8] = b"https://www.baozimh.com";
const READER_BASE: &[u8] = b"https://www.twmanga.com";
koma_source_sdk::koma_source_buffers! {
    payload: 1024 * 1024,
    http_out: 2 * 1024 * 1024,
    body: 2 * 1024 * 1024,
    http_req: 1024,
    scratch: 1024,
}
koma_source_sdk::koma_source_helpers!();

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "online.baozimh.koma",
    name: "包子漫畫 (Baozimh)",
    version: "0.1.10",
    api_version: "0.2",
    language: "zh-Hant",
    author: "Koma",
    description: "Baozimh (baozimh.com) HTML scraping source.",
    content_rating: "unknown",
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


fn fetch_html(url_bytes: &[u8]) -> Result<usize, FetchError> {
    let req_len = build_get_request(http_req_buf(), url_bytes, None, &[]).ok_or(FetchError::Network)?;
    // Retry transport-level failures (DNS/connect/TLS/timeout) up to 3 times.
    // HTTP 4xx/5xx come back as Ok and are classified below.
    let mut resp_len = 0usize;
    let mut transport_failed = true;
    for attempt in 0..3u8 {
        let req_slice = &http_req_buf()[..req_len];
        match http_request(req_slice, http_out()) {
            Ok(n) => { resp_len = n; transport_failed = false; break; }
            Err(_) => {
                if attempt < 2 { log_info(b"baozimh: http transport error, retrying"); }
            }
        }
    }
    if transport_failed { return Err(FetchError::Network); }
    let resp = &http_out()[..resp_len];
    // Host ok describes transport success, including HTTP refusal responses.
    let status = extract_json_number(resp, b"status")
        .or_else(|| extract_json_number(resp, b"statusCode"))
        .map(parse_status_code).unwrap_or(0);
    match status {
        401 | 403 => {
            let error = match access_signal::from_host_response(status, resp, body_buf()) {
                Some(signal) => FetchError::Access(status, signal),
                None => FetchError::Http(status),
            };
            return Err(error);
        }
        400..=599 => return Err(FetchError::Http(status)),
        200..=299 => {},
        _ => return Err(FetchError::Network),
    }
    if !contains_bytes(resp, br#""ok":true"#) {
        return Err(FetchError::Network);
    }
    let body_marker = b"\"bodyText\":\"";
    let body_start_idx = find_subslice(resp, body_marker).ok_or(FetchError::Network)? + body_marker.len();
    let html_dst = body_buf();
    let mut out_cursor = 0usize;
    let mut i = body_start_idx;
    while i < resp.len() {
        let b = resp[i];
        if b == b'\\' && i + 1 < resp.len() {
            let next = resp[i + 1];
            let unescaped: u8 = match next {
                b'"' => b'"',
                b'\\' => b'\\',
                b'/' => b'/',
                b'n' => b'\n',
                b'r' => b'\r',
                b't' => b'\t',
                b'b' => 0x08,
                b'f' => 0x0c,
                b'u' => {
                    if i + 5 >= resp.len() {
                        return Err(FetchError::Network);
                    }
                    let mut code: u32 = 0;
                    let mut k = 0;
                    while k < 4 {
                        let h = resp[i + 2 + k];
                        let v = match h {
                            b'0'..=b'9' => (h - b'0') as u32,
                            b'a'..=b'f' => 10 + (h - b'a') as u32,
                            b'A'..=b'F' => 10 + (h - b'A') as u32,
                            _ => return Err(FetchError::Network),
                        };
                        code = (code << 4) | v;
                        k += 1;
                    }
                    if code < 0x80 {
                        if out_cursor >= html_dst.len() {
                            return Err(FetchError::Network);
                        }
                        html_dst[out_cursor] = code as u8;
                        out_cursor += 1;
                    } else if code < 0x800 {
                        if out_cursor + 2 > html_dst.len() {
                            return Err(FetchError::Network);
                        }
                        html_dst[out_cursor] = 0xC0 | (code >> 6) as u8;
                        html_dst[out_cursor + 1] = 0x80 | (code & 0x3F) as u8;
                        out_cursor += 2;
                    } else {
                        if out_cursor + 3 > html_dst.len() {
                            return Err(FetchError::Network);
                        }
                        html_dst[out_cursor] = 0xE0 | (code >> 12) as u8;
                        html_dst[out_cursor + 1] = 0x80 | ((code >> 6) & 0x3F) as u8;
                        html_dst[out_cursor + 2] = 0x80 | (code & 0x3F) as u8;
                        out_cursor += 3;
                    }
                    i += 6;
                    continue;
                }
                _ => next,
            };
            if out_cursor >= html_dst.len() {
                return Err(FetchError::Network);
            }
            html_dst[out_cursor] = unescaped;
            out_cursor += 1;
            i += 2;
            continue;
        }
        if b == b'"' {
            return Ok(out_cursor);
        }
        if out_cursor >= html_dst.len() {
            return Err(FetchError::Network);
        }
        html_dst[out_cursor] = b;
        out_cursor += 1;
        i += 1;
    }
    Err(FetchError::Network)
}

struct OwnedDescriptor(HtmlDescriptor);

impl Drop for OwnedDescriptor {
    fn drop(&mut self) {
        let _ = html_close(self.0);
    }
}

fn attr_into<'a>(
    desc: HtmlDescriptor,
    name: &[u8],
    out: &'a mut [u8],
) -> Option<&'a [u8]> {
    let len = html_attr(desc, name, out).ok()?;
    Some(&out[..len])
}

fn text_into<'a>(desc: HtmlDescriptor, out: &'a mut [u8]) -> Option<&'a [u8]> {
    let len = html_text(desc, out).ok()?;
    Some(&out[..len])
}

// A tag is complete only when Text did not fill its actual output buffer.
// Preserve the previous tag-specific ASCII whitespace trimming exactly.
fn detail_tag_text<'a>(descriptor: HtmlDescriptor, output: &'a mut [u8]) -> Result<&'a [u8], ()> {
    let len = html_text(descriptor, output).map_err(|_| ())?;
    if len >= output.len() { return Err(()); }
    let mut start = 0;
    let mut end = len;
    while start < end && matches!(output[start], b' ' | b'\n' | b'\r' | b'\t') { start += 1; }
    while end > start && matches!(output[end - 1], b' ' | b'\n' | b'\r' | b'\t') { end -= 1; }
    Ok(&output[start..end])
}

fn slug_from_comic_path(path: &[u8], out: &mut [u8]) -> Option<usize> {
    let needle = b"/comic/";
    let idx = find_subslice(path, needle)? + needle.len();
    let mut len = 0usize;
    let mut i = idx;
    while i < path.len() {
        let b = path[i];
        if b == b'?' || b == b'#' || b == b'/' || b == b' ' {
            break;
        }
        if len >= out.len() {
            return None;
        }
        out[len] = b;
        len += 1;
        i += 1;
    }
    if len == 0 { None } else { Some(len) }
}

fn run_search(req: &[u8]) -> u32 {
    let query = match extract_json_string(req, b"query") {
        Some(q) => q,
        None => return write_error("search", "invalid_request", "missing query"),
    };
    let url_buf = scratch_a();
    let mut url_cursor = 0usize;
    if !write_bytes(url_buf, &mut url_cursor, SITE_BASE) {
        return write_error("search", "internal_error", "url buffer overflow");
    }
    if !write_bytes(url_buf, &mut url_cursor, b"/search?q=") {
        return write_error("search", "internal_error", "url buffer overflow");
    }
    if !write_url_encoded(url_buf, &mut url_cursor, query) {
        return write_error("search", "internal_error", "url buffer overflow");
    }
    let url_bytes = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_cursor) };

    let html_len = match fetch_html(url_bytes) {
        Ok(len) => len,
        Err(e) => return write_fetch_error("search", e),
    };
    let html_bytes = &body_buf()[..html_len];

    let document = match html_parse(html_bytes) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error("search", "parse_error", "html_parse failed"),
    };

    // Use html_select_all to get all comics-card poster links
    let select_buf = unsafe { &mut *core::ptr::addr_of_mut!(SELECT_ALL_BUF) };
    let count = html_select_all(document.0.raw(), b"a.comics-card__poster", select_buf);

    let payload = payload_buf();
    let mut c = 0usize;

    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("search", "internal_error", "payload overflow");
    }

    let mut written = 0usize;
    let max_items = if count > 0 { count as usize } else { 0 };
    let max_items = if max_items > 500 { 500 } else { max_items };

    for i in 0..max_items {
        let offset = i * 4;
        if offset + 4 > select_buf.len() { break; }
        let desc = i32::from_le_bytes([
            select_buf[offset],
            select_buf[offset + 1],
            select_buf[offset + 2],
            select_buf[offset + 3],
        ]);
        if desc < 0 { continue; }

        let scratch_href = scratch_a();
        let scratch_title = scratch_b();
        let hd: HtmlDescriptor = unsafe { core::mem::transmute(desc) };
        let href_bytes = attr_into(hd, b"href", scratch_href);
        let title_bytes = attr_into(hd, b"title", scratch_title);

        if let (Some(href), Some(title)) = (href_bytes, title_bytes) {
            let mut slug_buf = [0u8; 128];
            if let Some(slug_len) = slug_from_comic_path(href, &mut slug_buf) {
                let slug = &slug_buf[..slug_len];
                if written > 0 {
                    if !write_bytes(payload, &mut c, b",") {
                        return write_error("search", "internal_error", "payload overflow");
                    }
                }
                let ok = write_bytes(payload, &mut c, br#"{"id":"manga:"#)
                    && append_json_escaped(payload, &mut c, slug)
                    && write_bytes(payload, &mut c, br#"","title":""#)
                    && append_json_escaped(payload, &mut c, title)
                    && write_bytes(payload, &mut c, br#"","cover":{"kind":"url","url":"https://static-tw.baozimh.com/cover/"#)
                    && append_json_escaped(payload, &mut c, slug)
                    && write_bytes(payload, &mut c, br#".jpg"},"authors":[],"status":"unknown","contentRating":"unknown","description":"","sourceTags":["baozimh"]}"#);
                if !ok {
                    return write_error("search", "internal_error", "payload overflow");
                }
                written += 1;
            }
        }
        // Close the descriptor
        // Safety: HtmlDescriptor is repr(transparent) over i32
        let _ = html_close(unsafe { core::mem::transmute::<i32, HtmlDescriptor>(desc) });
    }

    if !write_bytes(payload, &mut c, br#"],"page":{"nextCursor":null,"hasMore":false}}"#) {
        return write_error("search", "internal_error", "payload overflow");
    }

    write_success_payload("search", c)
}

fn run_get_manga(req: &[u8]) -> u32 {
    let manga_id = match extract_json_string(req, b"mangaId") {
        Some(v) => v,
        None => return write_error("get_manga", "invalid_request", "missing mangaId"),
    };
    let prefix = b"manga:";
    if manga_id.len() <= prefix.len() || &manga_id[..prefix.len()] != prefix {
        return write_error("get_manga", "invalid_request", "unexpected mangaId");
    }
    let slug = &manga_id[prefix.len()..];

    let url_buf = scratch_a();
    let mut url_cursor = 0usize;
    if !(write_bytes(url_buf, &mut url_cursor, SITE_BASE)
        && write_bytes(url_buf, &mut url_cursor, b"/comic/")
        && write_bytes(url_buf, &mut url_cursor, slug))
    {
        return write_error("get_manga", "internal_error", "url overflow");
    }
    let url_bytes = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_cursor) };

    let html_len = match fetch_html(url_bytes) {
        Ok(n) => n,
        Err(e) => return write_fetch_error("get_manga", e),
    };
    let html_bytes = &body_buf()[..html_len];

    let document = match html_parse(html_bytes) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error("get_manga", "parse_error", "html_parse failed"),
    };

    let title_desc = html_select(document.0, b"h1.comics-detail__title");
    let author_desc = html_select(document.0, b"h2.comics-detail__author");
    let desc_desc = html_select(document.0, b"p.comics-detail__desc");
    let alt_desc = html_select(document.0, b"p.comics-detail__other-name");

    let mut title_buf = [0u8; 256];
    let mut author_buf = [0u8; 256];
    let mut desc_buf = [0u8; 1024];
    let mut alt_buf = [0u8; 512];

    let title_text = if let Ok(d) = title_desc {
        let owned = OwnedDescriptor(d);
        text_into(owned.0, &mut title_buf).map(|s| trim_ascii(s))
    } else {
        None
    };
    let author_text = if let Ok(d) = author_desc {
        let owned = OwnedDescriptor(d);
        text_into(owned.0, &mut author_buf).map(|s| trim_ascii(s))
    } else {
        None
    };
    let desc_text = if let Ok(d) = desc_desc {
        let owned = OwnedDescriptor(d);
        text_into(owned.0, &mut desc_buf).map(|s| trim_ascii(s))
    } else {
        None
    };
    let alt_text = if let Ok(d) = alt_desc {
        let owned = OwnedDescriptor(d);
        text_into(owned.0, &mut alt_buf).map(|s| trim_ascii(s))
    } else {
        None
    };

    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"manga":{"id":"manga:"#)
        && append_json_escaped(payload, &mut c, slug)
        && write_bytes(payload, &mut c, br#"","title":""#)
        && append_json_escaped(payload, &mut c, title_text.unwrap_or(slug))
        && write_bytes(payload, &mut c, br#"","alternateTitles":["#);
    if !ok {
        return write_error("get_manga", "internal_error", "payload overflow");
    }
    if let Some(alt) = alt_text {
        if !alt.is_empty() {
            let ok_alt = write_bytes(payload, &mut c, b"\"")
                && append_json_escaped(payload, &mut c, alt)
                && write_bytes(payload, &mut c, b"\"");
            if !ok_alt {
                return write_error("get_manga", "internal_error", "payload overflow");
            }
        }
    }
    let ok = write_bytes(payload, &mut c, br#"],"description":""#)
        && append_json_escaped(payload, &mut c, desc_text.unwrap_or(&[]))
        && write_bytes(payload, &mut c, br#"","cover":{"kind":"url","url":"https://static-tw.baozimh.com/cover/"#)
        && append_json_escaped(payload, &mut c, slug)
        && write_bytes(payload, &mut c, br#".jpg"},"authors":["#);

    if !ok {
        return write_error("get_manga", "internal_error", "payload overflow");
    }
    if let Some(author) = author_text {
        if !author.is_empty() {
            let ok2 = write_bytes(payload, &mut c, b"\"")
                && append_json_escaped(payload, &mut c, author)
                && write_bytes(payload, &mut c, b"\"");
            if !ok2 {
                return write_error("get_manga", "internal_error", "payload overflow");
            }
        }
    }
    // Preserve the existing [0]=status, [1]=region, [2+]=genre contract,
    // but allocate/own all matching descriptors rather than silently taking 20.
    let tag_count = html_select_all(document.0.raw(), b"span.tag", &mut scratch_a()[..0]);
    if tag_count < 0 { return write_error("get_manga", "parse_error", "tag selection failed"); }
    let total = tag_count as usize;
    let descriptor_bytes = match total.checked_mul(4) {
        Some(n) if n <= body_buf().len() => n,
        _ => return write_error("get_manga", "source_error", "tags exceed current descriptor capacity"),
    };
    let handles = &mut body_buf()[..descriptor_bytes];
    let selected = if total == 0 { 0 } else {
        html_select_all(document.0.raw(), b"span.tag", handles)
    };
    if selected < 0 { return write_error("get_manga", "parse_error", "tag selection failed"); }
    let tags = PageDescriptors { bytes: handles, count: (selected as usize).min(total) };
    if selected as usize != total {
        return write_error("get_manga", "parse_error", "tag selection changed");
    }
    let status_str: &[u8] = if total > 0 {
        let descriptor = match tags.get(0) {
            Some(handle) => handle,
            None => return write_error("get_manga", "parse_error", "invalid tag descriptor"),
        };
        let text = match detail_tag_text(descriptor, &mut http_out()[..NATIVE_RESULT_CAP]) {
            Ok(text) => text,
            Err(()) => return write_error("get_manga", "parse_error", "tag text unavailable or incomplete"),
        };
        if text == "連載中".as_bytes() || text == "连载中".as_bytes() { b"ongoing" }
        else if text == "已完結".as_bytes() || text == "已完结".as_bytes() { b"completed" }
        else { b"unknown" }
    } else { b"unknown" };

    // Write: ],"artists":[],"status":"...","contentRating":"unknown","language":"zh-Hant","tags":[
    let ok3 = write_bytes(payload, &mut c, br#"],"artists":[],"status":""#)
        && write_bytes(payload, &mut c, status_str)
        && write_bytes(payload, &mut c, br#"","contentRating":"unknown","language":"zh-Hant","tags":["#);
    if !ok3 {
        return write_error("get_manga", "internal_error", "payload overflow");
    }

    // Stream all genre texts within the existing complete-response budget.
    let mut tag_written = 0usize;
    for ti in 2..total {
        let descriptor = match tags.get(ti) {
            Some(handle) => handle,
            None => return write_error("get_manga", "parse_error", "invalid tag descriptor"),
        };
        let text = match detail_tag_text(descriptor, &mut http_out()[..NATIVE_RESULT_CAP]) {
            Ok(text) => text,
            Err(()) => return write_error("get_manga", "parse_error", "tag text unavailable or incomplete"),
        };
        if text.is_empty() { continue; }
        if tag_written > 0 && !write_bytes(payload, &mut c, b",") {
            return write_error("get_manga", "internal_error", "payload overflow");
        }
        if !(write_bytes(payload, &mut c, br#"""#)
            && append_json_escaped(payload, &mut c, text)
            && write_bytes(payload, &mut c, br#"""#)) {
            return write_error("get_manga", "internal_error", "payload overflow");
        }
        tag_written += 1;
    }

    if !write_bytes(payload, &mut c, br#"],"links":[{"kind":"source","url":"https://www.baozimh.com/comic/"#) {
        return write_error("get_manga", "internal_error", "payload overflow");
    }
    if !append_json_escaped(payload, &mut c, slug) {
        return write_error("get_manga", "internal_error", "payload overflow");
    }
    if !write_bytes(payload, &mut c, br#""}]}}"#) {
        return write_error("get_manga", "internal_error", "payload overflow");
    }

    let result = write_success_payload("get_manga", c);
    if result == 0 { return write_error("get_manga", "internal_error", "response overflow"); }
    let envelope_len = unsafe {
        core::ptr::read_unaligned((result as *const u8).add(8).cast::<u32>())
    };
    if envelope_len as usize > NATIVE_RESULT_CAP {
        return write_error("get_manga", "source_error", "detail cannot fit current response capacity");
    }
    result
}

fn run_get_chapters(req: &[u8]) -> u32 {
    let manga_id = match extract_json_string(req, b"mangaId") {
        Some(v) => v,
        None => return write_error("get_chapters", "invalid_request", "missing mangaId"),
    };
    let prefix = b"manga:";
    if manga_id.len() <= prefix.len() || &manga_id[..prefix.len()] != prefix {
        return write_error("get_chapters", "invalid_request", "unexpected mangaId");
    }
    let slug = &manga_id[prefix.len()..];

    let url_buf = scratch_a();
    let mut url_cursor = 0usize;
    if !(write_bytes(url_buf, &mut url_cursor, SITE_BASE)
        && write_bytes(url_buf, &mut url_cursor, b"/comic/")
        && write_bytes(url_buf, &mut url_cursor, slug))
    {
        return write_error("get_chapters", "internal_error", "url overflow");
    }
    let url_bytes = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_cursor) };

    let html_len = match fetch_html(url_bytes) {
        Ok(n) => n,
        Err(e) => return write_fetch_error("get_chapters", e),
    };
    let html_bytes = &body_buf()[..html_len];

    let document = match html_parse(html_bytes) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error("get_chapters", "parse_error", "html_parse failed"),
    };

    // Same production SelectAll ownership contract as the pages consumer.
    let count = html_select_all(document.0.raw(), b"a.comics-chapters__item", &mut scratch_a()[..0]);
    if count < 0 {
        return write_error("get_chapters", "parse_error", "chapter selection failed");
    }
    let total_items = count as usize;
    let descriptor_bytes = match total_items.checked_mul(4) {
        Some(bytes) if bytes <= BODY_CAP => bytes,
        _ => return write_error("get_chapters", "source_error", "chapters cannot fit current response capacity"),
    };
    let handles = &mut body_buf()[..descriptor_bytes];
    let selected = if total_items == 0 { 0 } else {
        html_select_all(document.0.raw(), b"a.comics-chapters__item", handles)
    };
    if selected < 0 {
        return write_error("get_chapters", "parse_error", "chapter selection failed");
    }
    let descriptors = PageDescriptors {
        bytes: handles,
        count: (selected as usize).min(descriptor_bytes / 4),
    };
    if selected as usize != total_items {
        return write_error("get_chapters", "parse_error", "chapter selection changed");
    }

    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("get_chapters", "internal_error", "payload overflow");
    }

    let mut written = 0usize;
    let mut seen_len = 0usize;

    for i in 0..total_items {
        let hd = match descriptors.get(i) {
            Some(value) => value,
            None => return write_error("get_chapters", "parse_error", "missing chapter descriptor"),
        };
        let mut title_buf = [0u8; 256];
        let title_capacity = title_buf.len();
        let raw_title = text_into(hd, &mut title_buf);
        let title = raw_title.map(trim_ascii);
        let mut href_buf = [0u8; 256];
        let href_capacity = href_buf.len();
        let href = attr_into(hd, b"href", &mut href_buf);

        let mut chapter_slot_buf = [0u8; 16];
        let chapter_slot = if let Some(h) = href {
            extract_query_param(h, b"chapter_slot", &mut chapter_slot_buf)
        } else {
            None
        };
        let slot = chapter_slot.unwrap_or(b"0");

        // Latest/full-list DOM groups may repeat the very same link. An ID alone
        // is insufficient: section links or conflicting titles must stay visible.
        // A saturated field may have been truncated by the existing host buffer.
        let complete_href = chapter_duplicates::complete_fact(href, href_capacity);
        let complete_title = chapter_duplicates::complete_fact(raw_title, title_capacity).map(trim_ascii);
        if chapter_duplicates::remember_exact_duplicate(http_out(), &mut seen_len, complete_href, chapter_slot, complete_title) {
            continue;
        }

        if written > 0 {
            if !write_bytes(payload, &mut c, b",") { return write_error("get_chapters", "internal_error", "payload overflow"); }
        }
        let ok = write_bytes(payload, &mut c, br#"{"id":"chapter:"#)
            && append_json_escaped(payload, &mut c, slug)
            && write_bytes(payload, &mut c, b":")
            && append_json_escaped(payload, &mut c, slot)
            && write_bytes(payload, &mut c, br#"","mangaId":"manga:"#)
            && append_json_escaped(payload, &mut c, slug)
            && write_bytes(payload, &mut c, br#"","title":""#)
            && append_json_escaped(payload, &mut c, title.unwrap_or(b"Chapter"))
            && write_bytes(payload, &mut c, br#"","chapterNumber":"#)
            && if let Some(number) = complete_title.and_then(chapter_number::title_chapter_number) {
                write_bytes(payload, &mut c, b"\"")
                    && append_json_escaped(payload, &mut c, number)
                    && write_bytes(payload, &mut c, b"\"")
            } else {
                write_bytes(payload, &mut c, b"null")
            }
            && write_bytes(payload, &mut c, br#","volumeNumber":null,"publishedAt":null,"updatedAt":null,"pageCount":null}"#);
        if !ok { return write_error("get_chapters", "internal_error", "payload overflow"); }
        written += 1;
    }

    if !write_bytes(payload, &mut c, br#"],"page":{"nextCursor":null,"hasMore":false}}"#) {
        return write_error("get_chapters", "internal_error", "payload overflow");
    }
    let result = write_success_payload("get_chapters", c);
    if result == 0 { return write_error("get_chapters", "internal_error", "response overflow"); }
    let envelope_len = unsafe {
        core::ptr::read_unaligned((result as *const u8).add(8).cast::<u32>())
    };
    if envelope_len as usize > NATIVE_RESULT_CAP {
        return write_error("get_chapters", "source_error", "chapters cannot fit current response capacity");
    }
    result
}

// Own exactly the child handles actually written by SelectAll, including
// unconsumed entries when the operation must return early. The root outlives it.
struct PageDescriptors<'a> {
    bytes: &'a [u8],
    count: usize,
}

impl PageDescriptors<'_> {
    fn get(&self, index: usize) -> Option<HtmlDescriptor> {
        if index >= self.count { return None; }
        let offset = index * 4;
        let raw = i32::from_le_bytes([
            self.bytes[offset], self.bytes[offset + 1],
            self.bytes[offset + 2], self.bytes[offset + 3],
        ]);
        if raw < 0 { return None; }
        Some(unsafe { core::mem::transmute(raw) })
    }
}

impl Drop for PageDescriptors<'_> {
    fn drop(&mut self) {
        for index in 0..self.count {
            if let Some(handle) = self.get(index) {
                let _ = html_close(handle);
            }
        }
    }
}

// Current Native ReadEnvelope limits the complete JSON envelope, not a URI.
const NATIVE_RESULT_CAP: usize = 1024 * 1024;

fn write_page_ordinal(dst: &mut [u8], cursor: &mut usize, ordinal: usize) -> bool {
    if ordinal >= 10000 {
        return write_usize(dst, cursor, ordinal);
    }
    let digits = [
        b'0' + ((ordinal / 1000) % 10) as u8,
        b'0' + ((ordinal / 100) % 10) as u8,
        b'0' + ((ordinal / 10) % 10) as u8,
        b'0' + (ordinal % 10) as u8,
    ];
    write_bytes(dst, cursor, &digits)
}

fn log_cdn_result(probes: usize, outcome: &[u8]) {
    let mut message = [0u8; 96];
    let mut length = 0;
    if write_bytes(&mut message, &mut length, b"baozimh cdn probes=")
        && write_usize(&mut message, &mut length, probes)
        && write_bytes(&mut message, &mut length, b" outcome=")
        && write_bytes(&mut message, &mut length, outcome) {
        log_info(&message[..length]);
    }
}

// HEAD probes are deliberately not fetch_html: no GET/body or transport retry.
fn probe_image_head(parts: &[&[u8]]) -> Result<bool, ()> {
    if host::check_cancel() { return Err(()); }
    let request = payload_buf();
    let mut cursor = 0;
    let mut ok = write_bytes(request, &mut cursor, br#"{"version":1,"method":"HEAD","url":""#);
    for part in parts { ok = ok && append_json_escaped(request, &mut cursor, part); }
    ok = ok && write_bytes(request, &mut cursor, br#"","headers":{},"timeoutMs":1500,"responseKind":"bodyText"}"#);
    if !ok { return Ok(false); }
    let mut response = [0u8; 4096];
    let size = http_request(&request[..cursor], &mut response);
    if host::check_cancel() { return Err(()); }
    let size = match size { Ok(size) => size, Err(_) => return Ok(false) };
    let response = &response[..size];
    let status = json_object_member(response, b"status")
        .or_else(|| json_object_member(response, b"statusCode")).map(parse_status_code);
    Ok(json_object_member(response, b"ok") == Some(b"true".as_slice())
        && status == Some(200)
        && json_object_member(response, b"bodyText") == Some(br#""""#.as_slice()))
}

fn run_get_pages(req: &[u8]) -> u32 {
    let chapter_id = match extract_json_string(req, b"chapterId") {
        Some(v) => v,
        None => return write_error("get_pages", "invalid_request", "missing chapterId"),
    };
    let prefix = b"chapter:";
    if chapter_id.len() <= prefix.len() || &chapter_id[..prefix.len()] != prefix {
        return write_error("get_pages", "invalid_request", "unexpected chapterId");
    }
    let rest = &chapter_id[prefix.len()..];
    let colon_idx = match find_subslice(rest, b":") {
        Some(i) => i,
        None => return write_error("get_pages", "invalid_request", "chapterId missing slot"),
    };
    let slug = &rest[..colon_idx];
    let slot = &rest[colon_idx + 1..];

    let url_buf = scratch_a();
    let mut url_cursor = 0usize;
    if !(write_bytes(url_buf, &mut url_cursor, READER_BASE)
        && write_bytes(url_buf, &mut url_cursor, b"/comic/chapter/")
        && write_bytes(url_buf, &mut url_cursor, slug)
        && write_bytes(url_buf, &mut url_cursor, b"/0_")
        && write_bytes(url_buf, &mut url_cursor, slot)
        && write_bytes(url_buf, &mut url_cursor, b".html"))
    {
        return write_error("get_pages", "internal_error", "url overflow");
    }
    let url_bytes = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_cursor) };

    let html_len = match fetch_html(url_bytes) {
        Ok(n) => n,
        Err(e) => return write_fetch_error("get_pages", e),
    };
    let html_bytes = &body_buf()[..html_len];

    let document = match html_parse(html_bytes) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error("get_pages", "parse_error", "html_parse failed"),
    };
    // The count-only pass allocates no child handles. Native SelectAll returns
    // the full count even when the caller provides zero output capacity.
    let count = html_select_all(document.0.raw(), b"amp-img.comic-contain__item", &mut scratch_a()[..0]);
    if count < 0 {
        return write_error("get_pages", "parse_error", "image selection failed");
    }
    let total = count as usize;
    let descriptor_bytes = match total.checked_mul(4) {
        Some(bytes) if bytes <= BODY_CAP => bytes,
        _ => return write_error("get_pages", "source_error", "chapter cannot fit current response capacity"),
    };
    // Lexbor owns its DOM/attribute storage after parsing; the original HTML
    // body is no longer needed. Reuse it, without adding a fixed page limit.
    let handles = &mut body_buf()[..descriptor_bytes];
    let selected = if total == 0 { 0 } else {
        html_select_all(document.0.raw(), b"amp-img.comic-contain__item", handles)
    };
    if selected < 0 {
        // Native SelectAll error paths allocate no descriptors.
        return write_error("get_pages", "parse_error", "image selection failed");
    }
    let descriptors = PageDescriptors {
        bytes: handles,
        // Only these entries were written/allocated by the actual second call.
        count: (selected as usize).min(descriptor_bytes / 4),
    };
    if selected as usize != total {
        return write_error("get_pages", "parse_error", "image selection changed");
    }

    // Keep the original first URI alive while later attributes use the other
    // half of the existing HTTP buffer. HEAD replies use their own small buffer.
    let (first_uri_buffer, page_uri_buffer) = http_out().split_at_mut(NATIVE_RESULT_CAP);
    let first_uri = if total == 0 { &[] as &[u8] } else {
        let hd = match descriptors.get(0) {
            Some(value) => value,
            None => return write_error("get_pages", "parse_error", "missing image descriptor"),
        };
        match attr_into(hd, b"src", first_uri_buffer) {
            Some(value) => value,
            None => return write_error("get_pages", "parse_error", "missing image src"),
        }
    };
    if host::check_cancel() { return write_error("get_pages", "cancelled", "cancelled"); }
    let mut probes = 0;
    let mut head_succeeded = false;
    let selection = match cdn_selection::choose(first_uri, slug, |parts| {
        probes += 1;
        let result = probe_image_head(parts);
        if matches!(result, Ok(true)) { head_succeeded = true; }
        result
    }) {
        Ok(value) => value,
        Err(()) => {
            log_cdn_result(probes, b"cancelled");
            return write_error("get_pages", "cancelled", "cancelled");
        }
    };
    log_cdn_result(probes, if selection.is_some() { b"fallback" }
        else if probes == 0 { b"skipped" }
        else if head_succeeded { b"original" }
        else { b"original_all_failed" });

    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"chapterId":"chapter:"#)
        && append_json_escaped(payload, &mut c, slug)
        && write_bytes(payload, &mut c, b":")
        && append_json_escaped(payload, &mut c, slot)
        && write_bytes(payload, &mut c, br#"","pages":["#);
    if !ok {
        return write_error("get_pages", "internal_error", "payload overflow");
    }

    let mut written = 0usize;

    for i in 0..total {
        if host::check_cancel() { return write_error("get_pages", "cancelled", "cancelled"); }
        let hd = match descriptors.get(i) {
            Some(value) => value,
            None => return write_error("get_pages", "parse_error", "missing image descriptor"),
        };
        // The old HTTP envelope has been consumed; reuse that full buffer.
        let img_src = match attr_into(hd, b"src", page_uri_buffer) {
            Some(value) => value,
            None => return write_error("get_pages", "parse_error", "missing image src"),
        };
        if img_src.len() == NATIVE_RESULT_CAP {
            // Attr reports copied bytes, not original length. A saturated 1MiB
            // URI cannot fit the same-sized complete result envelope.
            return write_error("get_pages", "source_error", "image URI cannot fit current response capacity");
        }
        {
            if written > 0 {
                if !write_bytes(payload, &mut c, b",") {
                    return write_error("get_pages", "internal_error", "payload overflow");
                }
            }
            let ok2 = write_bytes(payload, &mut c, br#"{"id":"page:"#)
                && append_json_escaped(payload, &mut c, slug)
                && write_bytes(payload, &mut c, b":")
                && append_json_escaped(payload, &mut c, slot)
                && write_bytes(payload, &mut c, b":")
                && write_page_ordinal(payload, &mut c, written)
                && write_bytes(payload, &mut c, br#"","index":"#)
                && write_usize(payload, &mut c, written)
                && write_bytes(payload, &mut c, br#","image":{"kind":"url","url":""#)
                && match selection.as_ref().and_then(|selected| selected.rewrite(img_src)) {
                    Some(rewritten) => append_json_escaped(payload, &mut c, rewritten.prefix)
                        && append_json_escaped(payload, &mut c, rewritten.suffix)
                        && append_json_escaped(payload, &mut c, rewritten.tail),
                    None => append_json_escaped(payload, &mut c, img_src),
                }
                && write_bytes(payload, &mut c, br#""}}"#);
            if !ok2 {
                return write_error("get_pages", "internal_error", "payload overflow");
            }
            written += 1;
        }
    }

    if !write_bytes(payload, &mut c, b"]}") {
        return write_error("get_pages", "internal_error", "payload overflow");
    }
    let result = write_success_payload("get_pages", c);
    if result == 0 {
        return write_error("get_pages", "internal_error", "response overflow");
    }
    // The ABI header length is the full JSON envelope (excluding the 16-byte
    // header), not just data. Match the current Native ReadEnvelope boundary.
    let envelope_len = unsafe {
        core::ptr::read_unaligned((result as *const u8).add(8).cast::<u32>())
    };
    if envelope_len as usize > NATIVE_RESULT_CAP {
        return write_error("get_pages", "source_error", "chapter cannot fit current response capacity");
    }
    result
}

fn run_get_listings(_req: &[u8]) -> u32 {
    // Listing descriptors are local metadata, not the first network result page.
    let listings = r#"{"listings":[{"id":"popular","name":"熱門","kind":"popular"}]}"#.as_bytes();
    payload_buf()[..listings.len()].copy_from_slice(listings);
    write_success_payload("get_listings", listings.len())
}

fn run_get_filters(_req: &[u8]) -> u32 {
    // Return filter definitions for baozimh classify page
    // Filters: region, state, type (genre)
    // Using &str (not byte string) because labels contain non-ASCII (Chinese)
    const FILTERS_JSON: &str = "{\"filters\":[{\"id\":\"region\",\"name\":\"地區\",\"kind\":\"select\",\"options\":[{\"value\":\"all\",\"label\":\"全部\"},{\"value\":\"cn\",\"label\":\"國漫\"},{\"value\":\"jp\",\"label\":\"日本\"},{\"value\":\"kr\",\"label\":\"韓國\"},{\"value\":\"en\",\"label\":\"歐美\"}],\"default\":\"all\"},{\"id\":\"state\",\"name\":\"狀態\",\"kind\":\"select\",\"options\":[{\"value\":\"all\",\"label\":\"全部\"},{\"value\":\"serial\",\"label\":\"連載中\"},{\"value\":\"pub\",\"label\":\"已完結\"}],\"default\":\"all\"},{\"id\":\"type\",\"name\":\"類型\",\"kind\":\"select\",\"options\":[{\"value\":\"all\",\"label\":\"全部\"},{\"value\":\"lianai\",\"label\":\"戀愛\"},{\"value\":\"chunai\",\"label\":\"純愛\"},{\"value\":\"gufeng\",\"label\":\"古風\"},{\"value\":\"yineng\",\"label\":\"異能\"},{\"value\":\"xuanyi\",\"label\":\"懸疑\"},{\"value\":\"juqing\",\"label\":\"劇情\"},{\"value\":\"kehuan\",\"label\":\"科幻\"},{\"value\":\"qihuan\",\"label\":\"奇幻\"},{\"value\":\"xuanhuan\",\"label\":\"玄幻\"},{\"value\":\"chuanyue\",\"label\":\"穿越\"},{\"value\":\"mouxian\",\"label\":\"冒險\"},{\"value\":\"tuili\",\"label\":\"推理\"},{\"value\":\"wuxia\",\"label\":\"武俠\"},{\"value\":\"gedou\",\"label\":\"格鬥\"},{\"value\":\"zhanzheng\",\"label\":\"戰爭\"},{\"value\":\"rexie\",\"label\":\"熱血\"},{\"value\":\"gaoxiao\",\"label\":\"搞笑\"},{\"value\":\"danuzhu\",\"label\":\"大女主\"},{\"value\":\"dushi\",\"label\":\"都市\"},{\"value\":\"zongcai\",\"label\":\"總裁\"},{\"value\":\"hougong\",\"label\":\"後宮\"},{\"value\":\"richang\",\"label\":\"日常\"},{\"value\":\"hanman\",\"label\":\"韓漫\"},{\"value\":\"shaonian\",\"label\":\"少年\"},{\"value\":\"qita\",\"label\":\"其它\"}],\"default\":\"all\"}]}";
    let filters_bytes = FILTERS_JSON.as_bytes();

    let payload = payload_buf();
    let flen = filters_bytes.len();
    if flen > payload.len() {
        return write_error("get_filters", "internal_error", "payload overflow");
    }
    payload[..flen].copy_from_slice(filters_bytes);
    write_success_payload("get_filters", flen)
}

fn run_get_manga_list(req: &[u8]) -> u32 {
    // Do not confuse the envelope's type=request with the manga genre filter.
    let args = json_object_member(req, b"args").unwrap_or(req);
    let filters = json_object_member(args, b"filters").unwrap_or(args);
    let settings = json_object_member(req, b"settings").unwrap_or(b"{}");
    let region_filter = extract_json_string(filters, b"region").unwrap_or(b"all");
    let state_filter = extract_json_string(filters, b"state").unwrap_or(b"all");
    let region_default = extract_json_string(settings, b"defaultRegion").unwrap_or(b"all");
    let state_default = extract_json_string(settings, b"defaultState").unwrap_or(b"all");
    let region = if region_filter == b"all" {
        match region_default { b"cn" | b"jp" | b"kr" | b"en" => region_default, _ => b"all" }
    } else { region_filter };
    let state = if state_filter == b"all" {
        match state_default { b"serial" | b"pub" => state_default, _ => b"all" }
    } else { state_filter };
    let manga_type = extract_json_string(filters, b"type").unwrap_or(b"all");
    let page_num = request_page_number(req);
    if page_num > 1000 {
        return write_error("get_manga_list", "invalid_request", "page limit");
    }

    // Build URL: /classify?type=X&region=Y&state=Z&filter=*&page=N
    let url_buf = scratch_a();
    let mut url_cursor = 0usize;
    let ok = write_bytes(url_buf, &mut url_cursor, SITE_BASE)
        && write_bytes(url_buf, &mut url_cursor, b"/classify?type=")
        && write_bytes(url_buf, &mut url_cursor, manga_type)
        && write_bytes(url_buf, &mut url_cursor, b"&region=")
        && write_bytes(url_buf, &mut url_cursor, region)
        && write_bytes(url_buf, &mut url_cursor, b"&state=")
        && write_bytes(url_buf, &mut url_cursor, state)
        && write_bytes(url_buf, &mut url_cursor, b"&filter=%2a&page=")
        && write_usize(url_buf, &mut url_cursor, page_num);
    if !ok {
        return write_error("get_manga_list", "internal_error", "url overflow");
    }
    let url_bytes = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_cursor) };

    let html_len = match fetch_html(url_bytes) {
        Ok(len) => len,
        Err(e) => return write_fetch_error("get_manga_list", e),
    };
    let html_bytes = &body_buf()[..html_len];

    let document = match html_parse(html_bytes) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error("get_manga_list", "parse_error", "html_parse failed"),
    };

    let select_buf = unsafe { &mut *core::ptr::addr_of_mut!(SELECT_ALL_BUF) };
    let count = html_select_all(document.0.raw(), b"a.comics-card__poster", select_buf);

    let payload = payload_buf();
    let mut c = 0usize;

    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("get_manga_list", "internal_error", "payload overflow");
    }

    let max_items = if count > 0 { (count as usize).min(500) } else { 0 };
    let mut written = 0usize;

    for i in 0..max_items {
        let offset = i * 4;
        if offset + 4 > select_buf.len() { break; }
        let desc = i32::from_le_bytes([
            select_buf[offset], select_buf[offset+1], select_buf[offset+2], select_buf[offset+3],
        ]);
        if desc < 0 { continue; }

        let hd: HtmlDescriptor = unsafe { core::mem::transmute(desc) };
        let scratch_href = scratch_a();
        let scratch_title = scratch_b();
        let href_bytes = attr_into(hd, b"href", scratch_href);
        let title_bytes = attr_into(hd, b"title", scratch_title);

        if let (Some(href), Some(title)) = (href_bytes, title_bytes) {
            let mut slug_buf = [0u8; 128];
            if let Some(slug_len) = slug_from_comic_path(href, &mut slug_buf) {
                let slug = &slug_buf[..slug_len];
                if written > 0 {
                    if !write_bytes(payload, &mut c, b",") { break; }
                }
                let ok = write_bytes(payload, &mut c, br#"{"id":"manga:"#)
                    && append_json_escaped(payload, &mut c, slug)
                    && write_bytes(payload, &mut c, br#"","title":""#)
                    && append_json_escaped(payload, &mut c, title)
                    && write_bytes(payload, &mut c, br#"","cover":{"kind":"url","url":"https://static-tw.baozimh.com/cover/"#)
                    && append_json_escaped(payload, &mut c, slug)
                    && write_bytes(payload, &mut c, br#".jpg"},"authors":[],"status":"unknown","contentRating":"unknown","sourceTags":["baozimh"]}"#);
                if !ok { break; }
                written += 1;
            }
        }
        let _ = html_close(hd);
    }

    // A terminal page must not advertise a cursor: the App treats any cursor
    // as a next page even when hasMore is false.
    let has_more = written >= 36;
    if !write_bytes(payload, &mut c, br#"],"page":{"nextCursor":"#) {
        return write_error("get_manga_list", "internal_error", "payload overflow");
    }
    let ok = if has_more {
        write_bytes(payload, &mut c, b"\"")
            && write_usize(payload, &mut c, page_num + 1)
            && write_bytes(payload, &mut c, br#"","hasMore":true}}"#)
    } else {
        write_bytes(payload, &mut c, br#"null,"hasMore":false}}"#)
    };
    if !ok {
        return write_error("get_manga_list", "internal_error", "payload overflow");
    }

    write_success_payload("get_manga_list", c)
}

fn run_modify_image_request(req: &[u8]) -> u32 {
    // baozimh images are freely accessible, no modification needed.
    // Just extract the URL and return an unmodified request
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
        return write_error("get_image_request", "internal_error", "payload overflow");
    }
    write_success_payload("get_image_request", c)
}

// --- get_home: multi-section home page ---
// Uses /classify with different filters to build sections, avoiding the heavy homepage HTML.

enum HomeSectionError {
    Fetch(FetchError),
    Parse,
    Selection,
    Overflow,
}

fn write_home_section_error(error: HomeSectionError) -> u32 {
    match error {
        HomeSectionError::Fetch(error) => write_fetch_error("get_home", error),
        HomeSectionError::Parse => write_error("get_home", "parse_error", "html_parse failed"),
        HomeSectionError::Selection => write_error("get_home", "parse_error", "card selection failed"),
        HomeSectionError::Overflow => write_error("get_home", "internal_error", "overflow"),
    }
}

fn parse_cards_into_section(payload: &mut [u8], c: &mut usize, section_id: &[u8], section_title: &[u8], url: &[u8], max_items: usize) -> Result<(), HomeSectionError> {
    let ok = write_bytes(payload, c, br#"{"id":""#)
        && append_json_escaped(payload, c, section_id)
        && write_bytes(payload, c, br#"","title":""#)
        && append_json_escaped(payload, c, section_title)
        && write_bytes(payload, c, br#"","items":["#);
    if !ok { return Err(HomeSectionError::Overflow); }

    let mut item_count = 0usize;

    let html_len = fetch_html(url).map_err(HomeSectionError::Fetch)?;
    let html_bytes = &body_buf()[..html_len];
    let doc_desc = html_parse(html_bytes).map_err(|_| HomeSectionError::Parse)?;
    {
            let doc = OwnedDescriptor(doc_desc);
            let select_buf = unsafe { &mut *core::ptr::addr_of_mut!(SELECT_ALL_BUF) };
            // Home deliberately returns only the existing summary size. Ask the
            // host to allocate only those handles, and own every written handle
            // through all normal and error exits before closing the document.
            let descriptor_bytes = max_items.checked_mul(4)
                .filter(|bytes| *bytes <= select_buf.len())
                .ok_or(HomeSectionError::Overflow)?;
            let count = html_select_all(doc.0.raw(), b"a.comics-card__poster",
                &mut select_buf[..descriptor_bytes]);
            if count < 0 { return Err(HomeSectionError::Selection); }
            let descriptors = PageDescriptors {
                bytes: &select_buf[..descriptor_bytes],
                count: (count as usize).min(max_items),
            };

            for i in 0..descriptors.count {
                let hd = descriptors.get(i).ok_or(HomeSectionError::Selection)?;
                let scratch_href = scratch_a();
                let scratch_title = scratch_b();
                let href_bytes = attr_into(hd, b"href", scratch_href);
                let title_bytes = attr_into(hd, b"title", scratch_title);

                if let (Some(href), Some(title)) = (href_bytes, title_bytes) {
                    let mut slug_buf = [0u8; 128];
                    if let Some(slug_len) = slug_from_comic_path(href, &mut slug_buf) {
                        let slug = &slug_buf[..slug_len];
                        if item_count > 0 {
                            if !write_bytes(payload, c, b",") { return Err(HomeSectionError::Overflow); }
                        }
                        let ok2 = write_bytes(payload, c, br#"{"id":"manga:"#)
                            && append_json_escaped(payload, c, slug)
                            && write_bytes(payload, c, br#"","title":""#)
                            && append_json_escaped(payload, c, title)
                            && write_bytes(payload, c, br#"","cover":{"kind":"url","url":"https://static-tw.baozimh.com/cover/"#)
                            && append_json_escaped(payload, c, slug)
                            && write_bytes(payload, c, br#".jpg"}}"#);
                        if !ok2 { return Err(HomeSectionError::Overflow); }
                        item_count += 1;
                    }
                }
            }
    }

    write_bytes(payload, c, b"]}").then_some(()).ok_or(HomeSectionError::Overflow)
}

fn run_get_home(_req: &[u8]) -> u32 {
    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"sections":["#) {
        return write_error("get_home", "internal_error", "overflow");
    }

    // Section 1: 熱門 (default classify page)
    if let Err(error) = parse_cards_into_section(payload, &mut c,
        b"home:popular", "熱門漫畫".as_bytes(),
        b"https://www.baozimh.com/classify", 12) {
        return write_home_section_error(error);
    }

    // Section 2: 國漫推薦
    if !write_bytes(payload, &mut c, b",") {
        return write_error("get_home", "internal_error", "overflow");
    }
    if let Err(error) = parse_cards_into_section(payload, &mut c,
        b"home:cn", "國漫推薦".as_bytes(),
        b"https://www.baozimh.com/classify?type=all&region=cn", 12) {
        return write_home_section_error(error);
    }

    // Section 3: 日本漫畫
    if !write_bytes(payload, &mut c, b",") {
        return write_error("get_home", "internal_error", "overflow");
    }
    if let Err(error) = parse_cards_into_section(payload, &mut c,
        b"home:jp", "日本漫畫".as_bytes(),
        b"https://www.baozimh.com/classify?type=all&region=jp", 12) {
        return write_home_section_error(error);
    }

    // Section 4: 韓國漫畫
    if !write_bytes(payload, &mut c, b",") {
        return write_error("get_home", "internal_error", "overflow");
    }
    if let Err(error) = parse_cards_into_section(payload, &mut c,
        b"home:kr", "韓國漫畫".as_bytes(),
        b"https://www.baozimh.com/classify?type=all&region=kr", 12) {
        return write_home_section_error(error);
    }

    if !write_bytes(payload, &mut c, b"]}") {
        return write_error("get_home", "internal_error", "overflow");
    }
    write_success_payload("get_home", c)
}

// --- get_settings: source-level user preferences ---

fn run_get_settings(_req: &[u8]) -> u32 {
    const SETTINGS_JSON: &str = "{\"settings\":[{\"id\":\"defaultRegion\",\"name\":\"預設地區\",\"kind\":\"select\",\"options\":[{\"value\":\"all\",\"label\":\"全部\"},{\"value\":\"cn\",\"label\":\"國漫\"},{\"value\":\"jp\",\"label\":\"日本\"},{\"value\":\"kr\",\"label\":\"韓國\"},{\"value\":\"en\",\"label\":\"歐美\"}],\"default\":\"all\"},{\"id\":\"defaultState\",\"name\":\"預設狀態\",\"kind\":\"select\",\"options\":[{\"value\":\"all\",\"label\":\"全部\"},{\"value\":\"serial\",\"label\":\"連載中\"},{\"value\":\"pub\",\"label\":\"已完結\"}],\"default\":\"all\"}]}";

    let settings_bytes = SETTINGS_JSON.as_bytes();
    let payload = payload_buf();
    let flen = settings_bytes.len();
    if flen > payload.len() {
        return write_error("get_settings", "internal_error", "payload overflow");
    }
    payload[..flen].copy_from_slice(settings_bytes);
    write_success_payload("get_settings", flen)
}

fn extract_query_param<'a>(url: &[u8], key: &[u8], out: &'a mut [u8]) -> Option<&'a [u8]> {
    let mut pattern_buf = [0u8; 32];
    if key.len() + 1 > pattern_buf.len() {
        return None;
    }
    pattern_buf[..key.len()].copy_from_slice(key);
    pattern_buf[key.len()] = b'=';
    let pattern = &pattern_buf[..key.len() + 1];
    let start = find_subslice(url, pattern)? + pattern.len();
    let mut len = 0usize;
    let mut i = start;
    while i < url.len() {
        let b = url[i];
        if b == b'&' || b == b'#' {
            break;
        }
        if len >= out.len() {
            return None;
        }
        out[len] = b;
        len += 1;
        i += 1;
    }
    Some(&out[..len])
}

#[no_mangle]
pub extern "C" fn koma_source_init(_manifest_ptr: u32, manifest_len: u32) -> i32 {
    log_info(b"baozimh source init");
    if host::check_cancel() {
        return -2;
    }
    if manifest_len > 0 { 0 } else { -1 }
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
    log_info(b"baozimh search");
    run_search(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga", "invalid_request", "empty request"),
    };
    log_info(b"baozimh get_manga");
    run_get_manga(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_chapters(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_chapters", "invalid_request", "empty request"),
    };
    log_info(b"baozimh get_chapters");
    run_get_chapters(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_pages(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_pages", "invalid_request", "empty request"),
    };
    log_info(b"baozimh get_pages");
    run_get_pages(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_listings(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_listings", "invalid_request", "empty request"),
    };
    run_get_listings(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga_list(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga_list", "invalid_request", "empty request"),
    };
    log_info(b"baozimh get_manga_list");
    run_get_manga_list(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_filters(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_filters", "invalid_request", "empty request"),
    };
    log_info(b"baozimh get_filters");
    run_get_filters(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_image_request(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_image_request", "invalid_request", "empty request"),
    };
    log_info(b"baozimh modify_image_request");
    run_modify_image_request(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_home(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_home", "invalid_request", "empty request"),
    };
    log_info(b"baozimh get_home");
    run_get_home(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_settings(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_settings", "invalid_request", "empty request"),
    };
    log_info(b"baozimh get_settings");
    run_get_settings(req)
}

#[no_mangle]
pub extern "C" fn koma_source_free(result_ptr: u32) {
    response_buffer().free(result_ptr)
}
