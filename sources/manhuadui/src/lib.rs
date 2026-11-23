#![no_std]

extern crate koma_source_sdk;

use koma_source_sdk::host::{
    self, html_attr, html_close, html_parse, html_select, html_text, http_request,
};
use koma_source_sdk::json_utils::{
    append_json_escaped, extract_json_number,
    extract_json_string, find_subslice, json_object_member, request_page_number, write_bytes, write_url_encoded, write_usize,
};
use koma_source_sdk::source::{SourceCapabilities, SourceInfo, SourceError};
use koma_source_sdk::chapters::{Chapter, ChapterPage, ChapterPlacement, ChapterResponse, GroupedChapters};
use koma_source_sdk::plain_text::decode_json_content;
use koma_source_sdk::{build_get_request, decode_json_body_into, fetch_error_code, FetchError};

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

static mut SELECT_ALL_BUF: [u8; 16000] = [0; 16000];

const BASE_URL: &[u8] = b"https://www.ykmh.net";
// Request the mobile catalog directly so the visible verification document can serve the same GET.
const BROWSE_URL: &[u8] = b"https://m.ykmh.net";
koma_source_sdk::koma_source_buffers! {
    payload: 1024 * 1024,
    http_out: 2 * 1024 * 1024,
    body: 2 * 1024 * 1024,
    http_req: 2048,
    scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.manhuadui.koma",
    name: "优酷漫画",
    version: "0.1.8",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "优酷漫画的中文连载漫画，保留同一作品的不同版本。本源提供原站公开免费章节。",
    content_rating: "nsfw",
};

const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: true,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: true,
    manga_list: true,
    home: false,
    filters: false,
    settings: false,
    image_request: true,
    credentials: false,
};

fn strip_prefix_ascii<'a>(bytes: &'a [u8], prefix: &[u8]) -> &'a [u8] {
    if bytes.len() >= prefix.len() && &bytes[..prefix.len()] == prefix {
        trim_ascii(&bytes[prefix.len()..])
    } else {
        bytes
    }
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

// Fixed diagnostic fields only: never emit URLs, queries, headers or body contents.
fn diagnostic_error(stage: &[u8], code: &str) {
    let mut out = [0u8; 160];
    let mut c = 0;
    if write_bytes(&mut out, &mut c, b"manhuadui stage=")
        && write_bytes(&mut out, &mut c, stage)
        && write_bytes(&mut out, &mut c, b" code=")
        && write_bytes(&mut out, &mut c, code.as_bytes()) {
        host::log_info(&out[..c]);
    }
}

fn fetch_body(url: &[u8]) -> Result<usize, (&'static str, &'static str)> {
    host::log_info(b"manhuadui stage=fetch_begin");
    let req_len = match build_get_request(http_req_buf(), url, None, &[]) {
        Some(n) => n,
        None => {
            diagnostic_error(b"request_build", "network_error");
            return Err(fetch_error_code(FetchError::Network));
        }
    };
    let resp_len = match http_request(&http_req_buf()[..req_len], http_out()) {
        Ok(n) => n,
        Err(e) => {
            let result = match e {
                -1 => "return_-1",
                -2 => "return_-2",
                -3 => "return_-3",
                _ => "return_other",
            };
            diagnostic_error(b"http_return", result);
            diagnostic_error(b"http_request", "network_error");
            return Err(fetch_error_code(FetchError::Network));
        }
    };
    let response = unsafe { core::slice::from_raw_parts(HTTP_OUT.as_ptr(), resp_len) };
    let status = extract_json_number(response, b"status")
        .or_else(|| extract_json_number(response, b"statusCode"))
        .map(koma_source_sdk::parse_status_code).unwrap_or(0);
    let mut status_log = [0u8; 64];
    let mut c = 0;
    if write_bytes(&mut status_log, &mut c, b"manhuadui stage=http_response status=")
        && write_usize(&mut status_log, &mut c, status as usize) {
        host::log_info(&status_log[..c]);
    }
    // Whitelist fixed transport codes; an arbitrary host string must not reach logs.
    if status == 0 {
        let code = match extract_json_string(response, b"code") {
            Some(b"transport_unavailable") => "transport_unavailable",
            Some(b"invalid_response") => "invalid_response",
            Some(b"transport_invalid_response") => "transport_invalid_response",
            Some(b"invalid_request") => "invalid_request",
            Some(b"request_too_large") => "request_too_large",
            Some(b"timeout") => "timeout",
            Some(b"network_error") => "network_error",
            _ => "unknown",
        };
        diagnostic_error(b"transport_envelope", code);
    }
    // A positive host signal distinguishes the site's verification page from
    // an ordinary access denial. Each public list/search operation makes one GET.
    if matches!(status, 401 | 403 | 503) &&
        extract_json_string(response, b"accessSignal") == Some(b"challenge_required") {
        diagnostic_error(b"http_status", "challenge_required");
        return Err(("challenge_required", "website verification required"));
    }
    let error = match status {
        200..=299 => None,
        401 | 403 => Some(("permission_denied", "source denied this public request")),
        404 => Some(fetch_error_code(FetchError::NotFound)),
        429 => Some(fetch_error_code(FetchError::RateLimit)),
        400..=499 => Some(fetch_error_code(FetchError::ClientError)),
        500..=599 => Some(fetch_error_code(FetchError::ServerError)),
        _ => Some(fetch_error_code(FetchError::Network)),
    };
    if let Some(e) = error {
        diagnostic_error(b"http_status", e.0);
        return Err(e);
    }
    match decode_json_body_into(response, body_buf()) {
        Ok(n) => {
            host::log_info(b"manhuadui stage=body_decode_ok");
            Ok(n)
        }
        Err(e) => {
            let error = fetch_error_code(e);
            diagnostic_error(b"body_decode", error.0);
            Err(error)
        }
    }
}

struct OwnedDescriptor(host::HtmlDescriptor);

impl Drop for OwnedDescriptor {
    fn drop(&mut self) {
        let _ = html_close(self.0);
    }
}

fn attr_into<'a>(desc: host::HtmlDescriptor, name: &[u8], out: &'a mut [u8]) -> Option<&'a [u8]> {
    let len = html_attr(desc, name, out).ok()?;
    Some(&out[..len])
}

fn text_into<'a>(desc: host::HtmlDescriptor, out: &'a mut [u8]) -> Option<&'a [u8]> {
    let len = html_text(desc, out).ok()?;
    Some(&out[..len])
}

fn path_from_href(href: &[u8]) -> Option<&[u8]> {
    if href.is_empty() {
        return None;
    }
    if href[0] == b'/' {
        return Some(href);
    }
    let mut slash_count = 0u8;
    let mut i = 0usize;
    while i < href.len() {
        if href[i] == b'/' {
            slash_count += 1;
            if slash_count == 3 {
                return Some(&href[i..]);
            }
        }
        i += 1;
    }
    None
}

fn strip_prefix<'a>(bytes: &'a [u8], prefix: &[u8]) -> Option<&'a [u8]> {
    if bytes.len() >= prefix.len() && &bytes[..prefix.len()] == prefix {
        Some(&bytes[prefix.len()..])
    } else {
        None
    }
}

fn strip_query_fragment(bytes: &[u8]) -> &[u8] {
    let mut end = bytes.len();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'?' || bytes[i] == b'#' {
            end = i;
            break;
        }
        i += 1;
    }
    &bytes[..end]
}

fn manga_slug_from_path(path: &[u8]) -> Option<&[u8]> {
    let path = strip_query_fragment(path);
    let mut end = path.len();
    while end > 0 && path[end - 1] == b'/' {
        end -= 1;
    }
    let mut start = end;
    while start > 0 && path[start - 1] != b'/' {
        start -= 1;
    }
    if start < end {
        Some(&path[start..end])
    } else {
        None
    }
}

fn write_manga_url(dst: &mut [u8], c: &mut usize, slug: &[u8]) -> bool {
    write_bytes(dst, c, BASE_URL)
        && write_bytes(dst, c, b"/manhua/")
        && append_json_escaped(dst, c, slug)
        && write_bytes(dst, c, b"/")
}

// Website phone.js redirects Android documents from www to m. Keep public
// source links and old IDs unchanged, but request the same m origin used by
// the already-accepted browse verification flow and fixed upstream SinMH.
fn write_manga_fetch_url(dst: &mut [u8], c: &mut usize, slug: &[u8]) -> bool {
    write_bytes(dst, c, BROWSE_URL) && write_bytes(dst, c, b"/manhua/")
        && append_json_escaped(dst, c, slug) && write_bytes(dst, c, b"/")
}
fn write_path_url(dst: &mut [u8], c: &mut usize, path: &[u8]) -> bool {
    if path.starts_with(b"http://") || path.starts_with(b"https://") {
        append_json_escaped(dst, c, path)
    } else if path.starts_with(b"//") {
        write_bytes(dst, c, b"https:") && append_json_escaped(dst, c, path)
    } else if path.starts_with(b"/") {
        write_bytes(dst, c, BROWSE_URL) && append_json_escaped(dst, c, path)
    } else {
        write_bytes(dst, c, BROWSE_URL)
            && write_bytes(dst, c, b"/")
            && append_json_escaped(dst, c, path)
    }
}

fn write_json_url(dst: &mut [u8], c: &mut usize, url: &[u8]) -> bool {
    if url.starts_with(b"http://") || url.starts_with(b"https://") {
        append_json_escaped(dst, c, url)
    } else if url.starts_with(b"//") {
        write_bytes(dst, c, b"http:") && append_json_escaped(dst, c, url)
    } else if url.starts_with(b"/") {
        write_bytes(dst, c, BASE_URL) && append_json_escaped(dst, c, url)
    } else {
        append_json_escaped(dst, c, url)
    }
}

fn write_manga_item(
    payload: &mut [u8],
    c: &mut usize,
    slug: &[u8],
    title: &[u8],
    cover: &[u8],
    written: usize,
) -> bool {
    (written == 0 || write_bytes(payload, c, b","))
        && write_bytes(payload, c, br#"{"id":"ykmh:"#)
        && append_json_escaped(payload, c, slug)
        && write_bytes(payload, c, br#"","title":""#)
        && append_json_escaped(payload, c, title)
        && write_bytes(payload, c, br#"","cover":{"kind":"url","url":""#)
        && write_json_url(payload, c, cover)
        && write_bytes(payload, c, br#""},"authors":[],"status":"unknown","contentRating":"nsfw","description":"","sourceTags":["manhuadui"]}"#)
}

fn parse_list(html: &[u8], operation: &str, page: usize) -> u32 {
    host::log_info(b"manhuadui stage=parse_list_begin");
    let document = match html_parse(html) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => {
            diagnostic_error(b"list_html_parse", "parse_error");
            return write_error(operation, "parse_error", "html_parse failed");
        },
    };

    let select_buf = unsafe { &mut *core::ptr::addr_of_mut!(SELECT_ALL_BUF) };
    // The mobile list and search endpoints use different card layouts.
    let card_selector: &[u8] = if operation == "search" {
        b"li.list-comic, div.itemBox"
    } else {
        b"li.list-comic"
    };
    let count = html_select_all(document.0.raw(), card_selector, select_buf);

    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        diagnostic_error(b"list_payload", "internal_error");
        return write_error(operation, "internal_error", "payload overflow");
    }

    let total = if count > 0 { count as usize } else { 0 };
    let total = if total > 500 { 500 } else { total };
    let mut written = 0usize;

    for i in 0..total {
        let offset = i * 4;
        if offset + 4 > select_buf.len() {
            break;
        }
        let desc_raw = i32::from_le_bytes([
            select_buf[offset],
            select_buf[offset + 1],
            select_buf[offset + 2],
            select_buf[offset + 3],
        ]);
        if desc_raw < 0 {
            continue;
        }
        let item_desc: host::HtmlDescriptor = unsafe { core::mem::transmute(desc_raw) };

        let mut title_buf = [0u8; 512];
        let mut href_buf = [0u8; 512];
        let mut cover_buf = [0u8; 1024];

        let mut title = None;
        if let Ok(desc) = html_select(item_desc, b"a.txtA, .itemTxt > a.title") {
            let owned = OwnedDescriptor(desc);
            title = text_into(owned.0, &mut title_buf).map(trim_ascii);
        }
        if title.is_none() {
            if let Ok(desc) = html_select(item_desc, b"h3 > a") {
                let owned = OwnedDescriptor(desc);
                title = text_into(owned.0, &mut title_buf).map(trim_ascii);
            }
        }
        if title.is_none() {
            if let Ok(desc) = html_select(item_desc, b"p > a") {
                let owned = OwnedDescriptor(desc);
                title = text_into(owned.0, &mut title_buf).map(trim_ascii);
                if title.is_none() {
                    title = attr_into(owned.0, b"title", &mut title_buf).map(trim_ascii);
                }
            }
        }

        let mut href = None;
        if let Ok(desc) = html_select(item_desc, b"a.comic_img, a.ImgA, .itemTxt > a.title") {
            let owned = OwnedDescriptor(desc);
            href = attr_into(owned.0, b"href", &mut href_buf);
        }
        if href.is_none() {
            if let Ok(desc) = html_select(item_desc, b"a.image-link") {
                let owned = OwnedDescriptor(desc);
                href = attr_into(owned.0, b"href", &mut href_buf);
            }
        }
        if href.is_none() {
            if let Ok(desc) = html_select(item_desc, b"p > a") {
                let owned = OwnedDescriptor(desc);
                href = attr_into(owned.0, b"href", &mut href_buf);
            }
        }
        let cover = if let Ok(desc) = html_select(item_desc, b"img") {
            let owned = OwnedDescriptor(desc);
            attr_into(owned.0, b"src", &mut cover_buf).unwrap_or(b"")
        } else {
            b""
        };

        if let (Some(t), Some(h)) = (title, href) {
            if let Some(path) = path_from_href(h) {
                let slug = match manga_slug_from_path(path) {
                    Some(v) => v,
                    None => {
                        let _ = html_close(item_desc);
                        continue;
                    }
                };
                if !write_manga_item(payload, &mut c, slug, t, cover, written) {
                    let _ = html_close(item_desc);
                    break;
                }
                written += 1;
            }
        }

        let _ = html_close(item_desc);
    }

    let has_more = written > 0 && html_select(document.0, b".pagination .next:not(.disabled) > a").map(|d| {
        let _ = html_close(d);
        true
    }).unwrap_or(false);
    if !write_bytes(payload, &mut c, br#"],"page":{"nextCursor":"#)
        || !if has_more {
            write_bytes(payload, &mut c, b"\"")
                && write_usize(payload, &mut c, page.saturating_add(1))
                && write_bytes(payload, &mut c, b"\"")
        } else {
            write_bytes(payload, &mut c, b"null")
        }
        || !write_bytes(payload, &mut c, br#","hasMore":"#)
        || !write_bytes(
            payload,
            &mut c,
            if has_more {
                b"true" as &[u8]
            } else {
                b"false" as &[u8]
            },
        )
        || !write_bytes(payload, &mut c, b"}}")
    {
        diagnostic_error(b"list_payload", "internal_error");
        return write_error(operation, "internal_error", "payload overflow");
    }

    host::log_info(b"manhuadui stage=parse_list_ok");
    write_success_payload(operation, c)
}

// Keep decoded query separate from URL and HTTP buffers and off the WAMR stack.
static mut SEARCH_QUERY: [u8; 8192] = [0; 8192];
fn run_search(req: &[u8]) -> u32 {
    let query = match extract_json_string(req, b"query") {
        Some(q) => q,
        None => { diagnostic_error(b"search_request", "invalid_request"); return write_error("search", "invalid_request", "missing query"); },
    };
    let decoded = unsafe { &mut *core::ptr::addr_of_mut!(SEARCH_QUERY) };
    let query = match koma_source_sdk::plain_text::decode_json_content(query, decoded) {
        Ok(v) => v,
        Err(_) => return write_error("search", "invalid_request", "invalid query"),
    };
    let page = request_page_number(req);

    let url_buf = scratch_a();
    let mut c = 0usize;
    if !(write_bytes(url_buf, &mut c, BROWSE_URL)
        && write_bytes(url_buf, &mut c, b"/search/?keywords=")
        && write_url_encoded(url_buf, &mut c, query)
        && write_bytes(url_buf, &mut c, b"&page=")
        && write_usize(url_buf, &mut c, page))
    {
        diagnostic_error(b"search_url", "internal_error");
        return write_error("search", "internal_error", "url overflow");
    }
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), c) };

    let html_len = match fetch_body(url) {
        Ok(n) => n,
        Err(e) => {
            let (code, message) = e;
            return write_error("search", code, message);
        }
    };
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };
    parse_list(html, "search", page)
}

fn run_get_manga_list(req: &[u8]) -> u32 {
    let page = request_page_number(req);

    let url_buf = scratch_a();
    let mut c = 0usize;
    if !(write_bytes(url_buf, &mut c, BROWSE_URL)
        && write_bytes(url_buf, &mut c, b"/list/click/?page=")
        && write_usize(url_buf, &mut c, page))
    {
        diagnostic_error(b"list_url", "internal_error");
        return write_error("get_manga_list", "internal_error", "url overflow");
    }
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), c) };

    let html_len = match fetch_body(url) {
        Ok(n) => n,
        Err(e) => {
            let (code, message) = e;
            return write_error("get_manga_list", code, message);
        }
    };
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };
    parse_list(html, "get_manga_list", page)
}

fn select_text<'a>(
    root: host::HtmlDescriptor,
    selector: &[u8],
    buf: &'a mut [u8],
) -> Option<&'a [u8]> {
    let desc = html_select(root, selector).ok()?;
    let owned = OwnedDescriptor(desc);
    text_into(owned.0, buf).map(trim_ascii)
}

fn select_attr<'a>(
    root: host::HtmlDescriptor,
    selector: &[u8],
    attr: &[u8],
    buf: &'a mut [u8],
) -> Option<&'a [u8]> {
    let desc = html_select(root, selector).ok()?;
    let owned = OwnedDescriptor(desc);
    attr_into(owned.0, attr, buf)
}

fn run_get_manga(req: &[u8]) -> u32 {
    let manga_id = match extract_json_string(req, b"mangaId") {
        Some(v) => v,
        None => return write_error("get_manga", "invalid_request", "missing mangaId"),
    };
    let slug = match strip_prefix(manga_id, b"ykmh:") {
        Some(v) if !v.is_empty() => v,
        _ => return write_error("get_manga", "invalid_request", "bad mangaId"),
    };

    let url_buf = scratch_a();
    let mut url_len = 0usize;
    if !write_manga_fetch_url(url_buf, &mut url_len, slug) {
        return write_error("get_manga", "internal_error", "url overflow");
    }
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_len) };

    let html_len = match fetch_body(url) {
        Ok(n) => n,
        Err(e) => {
            let (code, message) = e;
            return write_error("get_manga", code, message);
        }
    };
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };
    let document = match html_parse(html) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error("get_manga", "parse_error", "html_parse failed"),
    };

    let mut title_buf = [0u8; 512];
    let mut cover_buf = [0u8; 1024];
    let mut author_buf = [0u8; 512];
    let mut status_buf = [0u8; 256];
    let mut desc_buf = [0u8; 4096];

    let title = match select_text(document.0, b"div.comic_deCon > h1, #comicName", &mut title_buf) {
        Some(t) if !t.is_empty() => t,
        _ => return write_error("get_manga", "parse_error", "work title missing"),
    };
    let cover =
        select_attr(document.0, b"div.comic_i_img > img, #Cover img", b"src", &mut cover_buf).unwrap_or(b"");

    let select_buf = unsafe { &mut *core::ptr::addr_of_mut!(SELECT_ALL_BUF) };
    let info_count = html_select_all(document.0.raw(), b".comic_deCon_liO > li, .Introduct_Sub .txtItme", select_buf);
    let mobile_template = html_select(document.0,b"#comicName").map(|d| {let _ = html_close(d); true}).unwrap_or(false);
    let mut author_len = 0usize;
    let mut status_len = 0usize;
    let info_total = if info_count > 0 {
        info_count as usize
    } else {
        0
    };
    let info_total = if info_total > 8 { 8 } else { info_total };
    for i in 0..info_total {
        let offset = i * 4;
        if offset + 4 > select_buf.len() {
            break;
        }
        let desc_raw = i32::from_le_bytes([
            select_buf[offset],
            select_buf[offset + 1],
            select_buf[offset + 2],
            select_buf[offset + 3],
        ]);
        if desc_raw < 0 {
            continue;
        }
        let li_desc: host::HtmlDescriptor = unsafe { core::mem::transmute(desc_raw) };
        if (!mobile_template && i == 0) || (mobile_template && html_select(li_desc, b".icon01").map(|d| {let _ = html_close(d); true}).unwrap_or(false)) {
            if let Some(t) = text_into(li_desc, &mut author_buf).map(trim_ascii) {
                author_len = t.len();
            }
        } else if (!mobile_template && i == 1) || (mobile_template && html_select(li_desc, b".icon02").map(|d| {let _ = html_close(d); true}).unwrap_or(false)) {
            let mut found_status = false;
            if let Ok(a_desc) = html_select(li_desc, if mobile_template { b":scope" } else { b"a" }) {
                let owned = OwnedDescriptor(a_desc);
                if let Some(t) = text_into(owned.0, &mut status_buf).map(trim_ascii) {
                    status_len = t.len();
                    found_status = true;
                }
            }
            if !found_status {
                if let Some(t) = text_into(li_desc, &mut status_buf).map(trim_ascii) {
                    status_len = t.len();
                }
            }
        }
        let _ = html_close(li_desc);
    }

    let author_raw = trim_ascii(&author_buf[..author_len]);
    let author = strip_prefix_ascii(author_raw, "作者：".as_bytes());

    let status_raw = trim_ascii(&status_buf[..status_len]);
    let status = if find_subslice(status_raw, "已完结".as_bytes()).is_some() {
        b"completed" as &[u8]
    } else if find_subslice(status_raw, "连载中".as_bytes()).is_some() {
        b"ongoing" as &[u8]
    } else {
        b"unknown" as &[u8]
    };
    let desc = select_text(document.0, b"p.comic_deCon_d, #full-des, #simple-des", &mut desc_buf).unwrap_or(b"");

    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"manga":{"id":"ykmh:"#)
        && append_json_escaped(payload, &mut c, slug)
        && write_bytes(payload, &mut c, br#"","title":""#)
        && append_json_escaped(payload, &mut c, title)
        && write_bytes(payload, &mut c, br#"","alternateTitles":[],"description":""#)
        && append_json_escaped(payload, &mut c, desc)
        && write_bytes(payload, &mut c, br#"","cover":{"kind":"url","url":""#)
        && write_json_url(payload, &mut c, cover)
        && write_bytes(payload, &mut c, br#""},"authors":["#)
        && if !author.is_empty() {
            write_bytes(payload, &mut c, b"\"")
                && append_json_escaped(payload, &mut c, author)
                && write_bytes(payload, &mut c, b"\"")
        } else {
            true
        }
        && write_bytes(payload, &mut c, br#"],"artists":[],"status":""#)
        && write_bytes(payload, &mut c, status)
        && write_bytes(payload, &mut c, br#"","contentRating":"nsfw","language":"zh","tags":[],"links":[{"kind":"source","url":""#)
        && write_manga_url(payload, &mut c, slug)
        && write_bytes(payload, &mut c, br#""}]}}"#);

    if !ok {
        return write_error("get_manga", "internal_error", "payload overflow");
    }
    write_success_payload("get_manga", c)
}

// The mobile .chapter-body and desktop .zj_list share chapter-list-* UL IDs.
// 237's actual mobile body had all seven rows newest-first, no active desktop
// sort control, and stale data-sort=asc. A UL ID does not identify a desktop
// template or its current order. Keep the mobile provider array contract separate.
// Desktop raw HTML is asc; rendered JS reverses it and updates the active control
// while leaving data-sort unchanged. Prefer that current control when present.
// Canonical source arrays remain oldest-first, independent of the App display sort.
// No title/number sorting: the provider itself links 41 -> 01 in one sampled group.
struct Selection<'a> { bytes: &'a mut [u8], count: usize }
impl Selection<'_> {
    fn node(&self, i: usize) -> host::HtmlDescriptor {
        let o = i * 4;
        let raw = i32::from_le_bytes(self.bytes[o..o+4].try_into().unwrap());
        unsafe { core::mem::transmute(raw) }
    }
}
impl Drop for Selection<'_> {
    fn drop(&mut self) { for i in 0..self.count { let _ = html_close(self.node(i)); } }
}
fn selection<'a>(root: host::HtmlDescriptor, selector: &[u8], bytes: &'a mut [u8])
    -> Result<Selection<'a>, SourceError> {
    let n = html_select_all(root.raw(), selector, bytes);
    if n < 0 { return Err(SourceError::source_error("catalog selector failed")); }
    let count = (n as usize).min(bytes.len()/4);
    let result = Selection { bytes, count };
    if n as usize != count { return Err(SourceError::source_error("complete catalog exceeds response capacity")); }
    Ok(result)
}
fn removed_by_rightsholder(html: &[u8]) -> bool {
    find_subslice(html, "版权方要求".as_bytes()).is_some()
        && find_subslice(html, "屏蔽删除本漫画所有章节链接".as_bytes()).is_some()
}
static mut CATALOG_IDS: [u8; 2*1024*1024] = [0; 2*1024*1024];
static mut CATALOG_RANGES: [(usize,usize);4000] = [(0,0);4000];

fn chapter_catalog(req: &[u8], manga_id: &[u8], document: host::HtmlDescriptor)
    -> Result<ChapterResponse<'static>, SourceError> {
    let mut group_bytes = [0u8;256];
    let groups = selection(document, b".zj_list, .chapter-body", &mut group_bytes)?;
    if groups.count == 0 { return Err(SourceError::source_error("complete chapter catalog missing")); }
    let mut catalog = GroupedChapters::new(payload_buf(), req)?;
    let mut seen_count = 0;
    let mut seen_len = 0;
    for g in 0..groups.count {
        let group_node = groups.node(g);
        let mut key_buf = [0u8;128];
        let mut key_len = 0;
        if !write_bytes(&mut key_buf, &mut key_len, manga_id)
            || !write_bytes(&mut key_buf, &mut key_len, b":") {
            return Err(SourceError::source_error("group identity too long"));
        }
        let mut list_id = [0u8;64];
        let provider_list_id = select_attr(group_node, b"ul[id^=chapter-list-]", b"id", &mut list_id);
        if let Some(list_id) = provider_list_id {
            if !write_bytes(&mut key_buf, &mut key_len, list_id) {
                return Err(SourceError::source_error("group identity too long"));
            }
        } else if !write_bytes(&mut key_buf, &mut key_len, b"mobile-section-")
            || !write_usize(&mut key_buf, &mut key_len, g) {
            return Err(SourceError::source_error("group identity too long"));
        }
        let mut class_buf = [0u8;128];
        let mobile = attr_into(group_node,b"class",&mut class_buf).map_or(false,|classes|
            classes.split(|b| b.is_ascii_whitespace()).any(|class| class==b"chapter-body"));
        let mut order_buf = [0u8;16];
        let mut fallback_order = [0u8;16];
        let active = select_attr(group_node,b".chapter-sort.active",b"data-order",&mut order_buf);
        let newest_first = match active {
            Some(b"asc") => false,
            Some(b"desc") => true,
            Some(_) => return Err(SourceError::source_error("catalog direction invalid")),
            None if mobile => true,
            None => match select_attr(group_node,b"ul[id^=chapter-list-]",b"data-sort",&mut fallback_order) {
                Some(b"asc") => false,
                Some(b"desc") => true,
                _ => return Err(SourceError::source_error("catalog direction missing")),
            },
        };
        let select_buf = unsafe { &mut *core::ptr::addr_of_mut!(SELECT_ALL_BUF) };
        let rows = selection(group_node, b"li > a", select_buf)?;
        let mut position = 0;
        for i in 0..rows.count {
            let n = if newest_first { rows.count-i-1 } else { i };
            let a = rows.node(n);
            let mut name_buf = [0u8;1024];
            let mut fallback_name = [0u8;1024];
            let mut href_buf = [0u8;1024];
            let name = select_text(a, b"span.list_con_zj", &mut name_buf)
                .or_else(|| text_into(a, &mut fallback_name).map(trim_ascii))
                .filter(|s| !s.is_empty()).ok_or(SourceError::source_error("chapter title missing"))?;
            let href = attr_into(a, b"href", &mut href_buf)
                .ok_or(SourceError::source_error("chapter URL missing"))?;
            let path = path_from_href(href).filter(|p| p.starts_with(b"/manhua/") && p.ends_with(b".html"))
                .ok_or(SourceError::source_error("chapter URL outside public catalog"))?;
            let mut duplicate = false;
            for j in 0..seen_count {
                let (start,len) = unsafe { CATALOG_RANGES[j] };
                if unsafe { &CATALOG_IDS[start..start+len] } == path { duplicate=true; break; }
            }
            if duplicate { continue; }
            if seen_count == 4000 || seen_len+path.len()>2*1024*1024 {
                return Err(SourceError::source_error("complete catalog exceeds response capacity"));
            }
            unsafe { CATALOG_IDS[seen_len..seen_len+path.len()].copy_from_slice(path);
                CATALOG_RANGES[seen_count]=(seen_len,path.len()); }
            seen_len += path.len(); seen_count += 1;
            let mut id = [0u8;1100]; let mut id_len=0;
            if !write_bytes(&mut id, &mut id_len, b"ykmhch:") || !write_bytes(&mut id, &mut id_len,path) {
                return Err(SourceError::source_error("chapter identity too long"));
            }
            let mut chapter = Chapter::new(&id[..id_len],manga_id,name);
            chapter.language=Some(b"zh");
            catalog.push(chapter,ChapterPlacement::InGroup { group:&key_buf[..key_len],position })?;
            position += 1;
        }
    }
    if seen_count == 0 { return Err(SourceError::source_error("complete chapter catalog empty")); }
    catalog.finish(ChapterPage::Complete)
}

fn run_get_chapters(req: &[u8]) -> u32 {
    let manga_id = match extract_json_string(req, b"mangaId") {
        Some(v) => v, None => return write_error("get_chapters", "invalid_request", "missing mangaId"),
    };
    let slug = match strip_prefix(manga_id,b"ykmh:") {
        Some(v) if !v.is_empty() => v, _ => return write_error("get_chapters","invalid_request","bad mangaId"),
    };
    let mut url_len=0;
    if !write_manga_fetch_url(scratch_a(),&mut url_len,slug) {
        return write_error("get_chapters","internal_error","url overflow");
    }
    let url=unsafe {core::slice::from_raw_parts(SCRATCH_A.as_ptr(),url_len)};
    let n=match fetch_body(url) {Ok(n)=>n,Err((c,m))=>return write_error("get_chapters",c,m)};
    let html=unsafe {core::slice::from_raw_parts(BODY_BUF.as_ptr(),n)};
    if removed_by_rightsholder(html) {return write_error("get_chapters","chapter_unavailable","chapters removed by rightsholder");}
    let doc=match html_parse(html) {Ok(d)=>OwnedDescriptor(d),Err(_)=>return write_error("get_chapters","parse_error","html_parse failed")};
    match chapter_catalog(req,manga_id,doc.0) {
        Ok(r)=>r.write(response_buffer()),Err(e)=>write_error("get_chapters",e.code(),e.message()),
    }
}

fn extract_js_string<'a>(data: &'a [u8], start: usize) -> Option<(&'a [u8], usize)> {
    if data.get(start)!=Some(&b'"') {return None;}
    let mut i=start+1;
    while i<data.len() {if data[i]==b'\\' {i+=2;continue;}
        if data[i]==b'"' {return Some((&data[start+1..i],i+1));} i+=1;}
    None
}
fn js_value<'a>(html:&'a [u8],key:&[u8])->Option<&'a [u8]> {
    let p=find_subslice(html,key)?+key.len(); let rest=trim_ascii(&html[p..]);
    Some(trim_ascii(rest.strip_prefix(b"=")?))
}
fn js_number(html:&[u8],key:&[u8])->Option<usize> {
    let v=js_value(html,key)?; let end=v.iter().position(|b|!b.is_ascii_digit())?;
    parse_usize(&v[..end])
}
fn absolute_image(url:&[u8])->bool {url.starts_with(b"http://")||url.starts_with(b"https://")}
fn valid_image_url(url:&[u8])->bool {
    absolute_image(url) && !url.iter().any(|b|b.is_ascii_control()||*b==b' '||*b==b'\\')
        && !url.split(|b|*b==b'/').nth(2).unwrap_or(b"").is_empty()
}
fn config_host<'a>(config:&[u8],out:&'a mut [u8])->Option<&'a [u8]> {
    let p=find_subslice(config,b"resHost")?;
    let d=p+find_subslice(&config[p..],b"domain")?;
    let a=d+find_subslice(&config[d..],b"[")?+1;
    let value=trim_ascii(&config[a..]);
    let (raw,_)=extract_js_string(value,0)?;
    let decoded=decode_json_content(raw,out).ok()?;
    if !valid_image_url(decoded) {return None;} Some(decoded)
}
static mut READER_HTML:[u8;2*1024*1024]=[0;2*1024*1024];

fn run_get_pages(req: &[u8]) -> u32 {
    let chapter_id=match extract_json_string(req,b"chapterId") {Some(v)=>v,None=>return write_error("get_pages","invalid_request","missing chapterId")};
    let path=match chapter_id.strip_prefix(b"ykmhch:") {Some(v) if v.starts_with(b"/manhua/")&&v.ends_with(b".html")=>v,_=>return write_error("get_pages","invalid_request","bad chapterId")};
    let work_end=match path[8..].iter().position(|b|*b==b'/') {Some(i)=>8+i+1,None=>return write_error("get_pages","invalid_request","bad chapter path")};
    // Old bookmarks must also obey a later work-level rightsholder removal.
    let mut u=0;
    if !write_path_url(scratch_a(),&mut u,&path[..work_end]) {return write_error("get_pages","internal_error","URL overflow");}
    let url=unsafe {core::slice::from_raw_parts(SCRATCH_A.as_ptr(),u)};
    let n=match fetch_body(url) {Ok(n)=>n,Err((c,m))=>return write_error("get_pages",c,m)};
    if removed_by_rightsholder(unsafe {core::slice::from_raw_parts(BODY_BUF.as_ptr(),n)}) {
        return write_error("get_pages","chapter_unavailable","chapters removed by rightsholder");
    }
    u=0;
    if !write_path_url(scratch_a(),&mut u,path) {return write_error("get_pages","internal_error","URL overflow");}
    let url=unsafe {core::slice::from_raw_parts(SCRATCH_A.as_ptr(),u)};
    let n=match fetch_body(url) {Ok(n)=>n,Err((c,m))=>return write_error("get_pages",c,m)};
    unsafe {READER_HTML[..n].copy_from_slice(&BODY_BUF[..n]);}
    let html=unsafe {core::slice::from_raw_parts(READER_HTML.as_ptr(),n)};
    if js_number(html,b"var chapterCanRead")!=Some(1) || js_number(html,b"var chapterPrice")!=Some(0) {
        return write_error("get_pages","permission_denied","chapter is not publicly readable for free");
    }
    let raw=match js_value(html,b"var chapterImages").and_then(|v|v.strip_prefix(b"[")) {
        Some(v)=>v,None=>return write_error("get_pages","parse_error","chapterImages array missing"),
    };
    let mut path_buffer=[0u8;2048];
    let chapter_path=match js_value(html,b"var chapterPath").and_then(|v|extract_js_string(v,0)).and_then(|(v,_)|decode_json_content(v,&mut path_buffer).ok()) {
        Some(v)=>v,None=>return write_error("get_pages","parse_error","chapterPath missing"),
    };
    let mut image_buf=[0u8;4096]; let mut host_buf=[0u8;2048]; let mut host_len=0;
    let mut count=0; let mut scan=raw;
    loop {
        scan=trim_ascii(scan);
        if scan.starts_with(b"]") {break;}
        let (image,end)=match extract_js_string(scan,0) {Some(v)=>v,None=>return write_error("get_pages","parse_error","invalid image array")};
        let decoded=match decode_json_content(image,&mut image_buf) {Ok(v)=>v,Err(_)=>return write_error("get_pages","parse_error","invalid image string")};
        if !absolute_image(decoded)&&!decoded.starts_with(b"//")&&host_len==0 {
            let len=match fetch_body(b"https://www.ykmh.net/js/config.js") {Ok(n)=>n,Err((c,m))=>return write_error("get_pages",c,m)};
            let config=unsafe {core::slice::from_raw_parts(BODY_BUF.as_ptr(),len)};
            host_len=match config_host(config,&mut host_buf) {Some(v)=>v.len(),None=>return write_error("get_pages","parse_error","image host missing")};
        }
        count+=1;
        if count>2000 {return write_error("get_pages","source_error","complete page table exceeds response capacity");}
        scan=trim_ascii(&scan[end..]);
        if scan.starts_with(b"]") {break;}
        scan=match scan.strip_prefix(b",") {Some(v)=>v,None=>return write_error("get_pages","parse_error","invalid image separator")};
    }
    if count==0 {return write_error("get_pages","chapter_unavailable","empty page table");}
    let payload=payload_buf();let mut c=0;
    if !(write_bytes(payload,&mut c,br#"{"chapterId":""#)&&append_json_escaped(payload,&mut c,chapter_id)&&write_bytes(payload,&mut c,br#"","pages":["#)) {
        return write_error("get_pages","internal_error","payload overflow");
    }
    let mut scan=raw;
    for i in 0..count {
        scan=trim_ascii(scan);let (image,end)=extract_js_string(scan,0).unwrap();
        let decoded=match decode_json_content(image,&mut image_buf) {Ok(v)=>v,Err(_)=>return write_error("get_pages","parse_error","invalid image string")};
        let mut image_url=[0u8;8192];let mut l=0;
        let ok=if absolute_image(decoded) {write_bytes(&mut image_url,&mut l,decoded)}
            else if decoded.starts_with(b"//") {write_bytes(&mut image_url,&mut l,b"https:")&&write_bytes(&mut image_url,&mut l,decoded)}
            else if decoded.starts_with(b"/") {write_bytes(&mut image_url,&mut l,&host_buf[..host_len])&&write_bytes(&mut image_url,&mut l,decoded)}
            else {write_bytes(&mut image_url,&mut l,&host_buf[..host_len])&&write_bytes(&mut image_url,&mut l,b"/")&&write_bytes(&mut image_url,&mut l,chapter_path)&&write_bytes(&mut image_url,&mut l,decoded)};
        if !ok||!valid_image_url(&image_url[..l]) {return write_error("get_pages","parse_error","invalid image URL");}
        if !((i==0||write_bytes(payload,&mut c,b","))&&write_bytes(payload,&mut c,br#"{"id":"page:"#)&&write_usize(payload,&mut c,i)
            &&write_bytes(payload,&mut c,br#"","index":"#)&&write_usize(payload,&mut c,i)&&write_bytes(payload,&mut c,br#","image":{"kind":"url","url":""#)
            &&append_json_escaped(payload,&mut c,&image_url[..l])&&write_bytes(payload,&mut c,br#""}}"#)) {return write_error("get_pages","internal_error","payload overflow");}
        scan=trim_ascii(&scan[end..]);if i+1<count {scan=&scan[1..];}
    }
    if !write_bytes(payload,&mut c,b"]}") {return write_error("get_pages","internal_error","payload overflow");}
    write_success_payload("get_pages",c)
}

fn run_get_listings(_req: &[u8]) -> u32 {
    response_buffer().write_success("get_listings", r#"{"listings":[{"id":"popular","name":"热门","kind":"popular"}]}"#.as_bytes())
}

fn run_get_home(_req: &[u8]) -> u32 {
    response_buffer().write_success("get_home", br#"{"sections":[]}"#)
}

fn run_get_filters(_req: &[u8]) -> u32 {
    response_buffer().write_success("get_filters", br#"{"filters":[]}"#)
}

fn run_get_settings(_req: &[u8]) -> u32 {
    response_buffer().write_success("get_settings", br#"{"settings":[]}"#)
}

fn run_get_image_request(req: &[u8]) -> u32 {
    let raw = match extract_json_string(req, b"url").or_else(|| extract_json_string(req, b"imageUrl")) {
        Some(v) => v, None => return write_error("get_image_request", "invalid_request", "missing image URL"),
    };
    let mut decoded = [0u8;8192];
    let url = match decode_json_content(raw, &mut decoded) {
        Ok(v) if valid_image_url(v) => v,
        _ => return write_error("get_image_request", "invalid_request", "invalid image URL"),
    };
    // Fixed upstream KeiSource.headersBuilder() supplies baseUrl/ as Referer,
    // and getImageRequest() carries those headers to every page/cover request.
    // Use the existing SDK ordinary-request UA; this does not configure ArkWeb
    // verification UA or expose website cookies. Absolute provider URLs stay exact.
    let n = match build_get_request(http_req_buf(), url, Some(b"https://www.ykmh.net/"), &[]) {
        Some(n) => n, None => return write_error("get_image_request", "invalid_request", "image descriptor overflow"),
    };
    let request = unsafe {core::slice::from_raw_parts(core::ptr::addr_of!(HTTP_REQ_BUF).cast::<u8>(),n)};
    let headers = match json_object_member(request,b"headers") {
        Some(v) => v, None => return write_error("get_image_request", "internal_error", "image headers missing"),
    };
    let payload=payload_buf(); let mut c=0;
    if !(write_bytes(payload,&mut c,br#"{"url":""#) && append_json_escaped(payload,&mut c,url)
        && write_bytes(payload,&mut c,br#"","headers":"#) && write_bytes(payload,&mut c,headers)
        && write_bytes(payload,&mut c,b"}")) {
        return write_error("get_image_request", "internal_error", "image descriptor overflow");
    }
    write_success_payload("get_image_request",c)
}

koma_source_sdk::koma_source_exports!("manhuadui");
