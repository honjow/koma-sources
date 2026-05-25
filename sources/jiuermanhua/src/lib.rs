#![no_std]

extern crate koma_source_sdk;

use koma_source_sdk::host::{
    self, html_attr, html_close, html_parse, html_select, html_text, http_request, log_info,
};
use koma_source_sdk::json_utils::{
    append_json_escaped, append_json_unescaped_then_escaped, extract_json_number,
    extract_json_string, find_subslice, write_bytes, write_url_encoded, write_usize,
};
use koma_source_sdk::result::ResultBuffer;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
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

const BASE_URL: &[u8] = b"http://www.92mh.com";
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
    id: "com.jiuermanhua.koma",
    name: "92漫画",
    version: "0.1.0",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "92漫画 HTML scraping source.",
    content_rating: "safe",
};

const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: true,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: false,
    manga_list: true,
    home: false,
    filters: false,
    settings: false,
    image_request: false,
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
    if any { Some(n) } else { None }
}

fn fetch_body(url: &[u8]) -> Result<usize, FetchError> {
    let req_len = build_get_request(http_req_buf(), url, None, &[]).ok_or(FetchError::Network)?;
    let resp_len = http_request(&http_req_buf()[..req_len], http_out()).map_err(|_| FetchError::Network)?;
    decode_json_body_into(&http_out()[..resp_len], body_buf())
}

struct OwnedDescriptor(host::HtmlDescriptor);

impl Drop for OwnedDescriptor {
    fn drop(&mut self) {
        let _ = html_close(self.0);
    }
}

fn attr_into<'a>(
    desc: host::HtmlDescriptor,
    name: &[u8],
    out: &'a mut [u8],
) -> Option<&'a [u8]> {
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
    id: &[u8],
    title: &[u8],
    cover: &[u8],
    written: usize,
) -> bool {
    (written == 0 || write_bytes(payload, c, b","))
        && write_bytes(payload, c, br#"{"id":""#)
        && append_json_escaped(payload, c, id)
        && write_bytes(payload, c, br#"","title":""#)
        && append_json_escaped(payload, c, title)
        && write_bytes(payload, c, br#"","cover":{"kind":"url","url":""#)
        && write_json_url(payload, c, cover)
        && write_bytes(payload, c, br#""},"authors":[],"status":"unknown","contentRating":"safe","description":"","sourceTags":["jiuermanhua"]}"#)
}

fn parse_list(html: &[u8]) -> u32 {
    let document = match html_parse(html) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error("parse", "parse_error", "html_parse failed"),
    };

    let select_buf = unsafe { &mut *core::ptr::addr_of_mut!(SELECT_ALL_BUF) };
    let count = html_select_all(document.0.raw(), b"li.list-comic", select_buf);

    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("parse", "internal_error", "payload overflow");
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
        if let Ok(desc) = html_select(item_desc, b"h3 > a") {
            let owned = OwnedDescriptor(desc);
            title = text_into(owned.0, &mut title_buf).map(trim_ascii);
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
        if let Ok(desc) = html_select(item_desc, b"a.comic_img") {
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
                if !write_manga_item(payload, &mut c, path, t, cover, written) {
                    let _ = html_close(item_desc);
                    break;
                }
                written += 1;
            }
        }

        let _ = html_close(item_desc);
    }

    if !write_bytes(payload, &mut c, br#"],"page":{"nextCursor":"#)
        || !if written > 0 {
            write_bytes(payload, &mut c, b"\"next\"")
        } else {
            write_bytes(payload, &mut c, b"null")
        }
        || !write_bytes(payload, &mut c, br#","hasMore":"#)
        || !write_bytes(payload, &mut c, if written > 0 { b"true" as &[u8] } else { b"false" as &[u8] })
        || !write_bytes(payload, &mut c, b"}}")
    {
        return write_error("parse", "internal_error", "payload overflow");
    }

    write_success_payload("parse", c)
}

fn run_search(req: &[u8]) -> u32 {
    let query = match extract_json_string(req, b"query") {
        Some(q) => q,
        None => return write_error("search", "invalid_request", "missing query"),
    };
    let page = extract_json_number(req, b"page").and_then(parse_usize).unwrap_or(1);

    let url_buf = scratch_a();
    let mut c = 0usize;
    if !(write_bytes(url_buf, &mut c, BASE_URL)
        && write_bytes(url_buf, &mut c, b"/search/?keywords=")
        && write_url_encoded(url_buf, &mut c, query)
        && write_bytes(url_buf, &mut c, b"&page=")
        && write_usize(url_buf, &mut c, page))
    {
        return write_error("search", "internal_error", "url overflow");
    }
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), c) };

    let html_len = match fetch_body(url) {
        Ok(n) => n,
        Err(e) => {
            let (code, message) = fetch_error_code(e);
            return write_error("search", code, message);
        }
    };
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };
    parse_list(html)
}

fn run_get_manga_list(req: &[u8]) -> u32 {
    let page = extract_json_number(req, b"page").and_then(parse_usize).unwrap_or(1);

    let url_buf = scratch_a();
    let mut c = 0usize;
    if !(write_bytes(url_buf, &mut c, BASE_URL)
        && write_bytes(url_buf, &mut c, b"/list/click/?page=")
        && write_usize(url_buf, &mut c, page))
    {
        return write_error("get_manga_list", "internal_error", "url overflow");
    }
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), c) };

    let html_len = match fetch_body(url) {
        Ok(n) => n,
        Err(e) => {
            let (code, message) = fetch_error_code(e);
            return write_error("get_manga_list", code, message);
        }
    };
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };
    parse_list(html)
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

    let url_buf = scratch_a();
    let mut url_len = 0usize;
    if !(write_bytes(url_buf, &mut url_len, BASE_URL) && write_bytes(url_buf, &mut url_len, manga_id)) {
        return write_error("get_manga", "internal_error", "url overflow");
    }
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_len) };

    let html_len = match fetch_body(url) {
        Ok(n) => n,
        Err(e) => {
            let (code, message) = fetch_error_code(e);
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

    let title = select_text(document.0, b"div.comic_deCon > h1", &mut title_buf).unwrap_or(manga_id);
    let cover = select_attr(document.0, b"div.comic_i_img > img", b"src", &mut cover_buf).unwrap_or(b"");

    let select_buf = unsafe { &mut *core::ptr::addr_of_mut!(SELECT_ALL_BUF) };
    let info_count = html_select_all(document.0.raw(), b".comic_deCon_liO > li", select_buf);
    let mut author_len = 0usize;
    let mut status_len = 0usize;
    let info_total = if info_count > 0 { info_count as usize } else { 0 };
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
        if i == 0 {
            if let Some(t) = text_into(li_desc, &mut author_buf).map(trim_ascii) {
                author_len = t.len();
            }
        } else if i == 1 {
            let mut found_status = false;
            if let Ok(a_desc) = html_select(li_desc, b"a") {
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
    let desc = select_text(document.0, b"p.comic_deCon_d", &mut desc_buf).unwrap_or(b"");

    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"manga":{"id":""#)
        && append_json_escaped(payload, &mut c, manga_id)
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
        && write_bytes(payload, &mut c, br#"","contentRating":"safe","language":"zh","tags":[],"links":[{"kind":"source","url":""#)
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

    let url_buf = scratch_a();
    let mut url_len = 0usize;
    if !(write_bytes(url_buf, &mut url_len, BASE_URL) && write_bytes(url_buf, &mut url_len, manga_id)) {
        return write_error("get_chapters", "internal_error", "url overflow");
    }
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_len) };

    let html_len = match fetch_body(url) {
        Ok(n) => n,
        Err(e) => {
            let (code, message) = fetch_error_code(e);
            return write_error("get_chapters", code, message);
        }
    };
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };
    let document = match html_parse(html) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error("get_chapters", "parse_error", "html_parse failed"),
    };

    let select_buf = unsafe { &mut *core::ptr::addr_of_mut!(SELECT_ALL_BUF) };
    let count = html_select_all(document.0.raw(), b"#chapter-list-1 > li > a", select_buf);
    let total = if count > 0 { count as usize } else { 0 };
    let total = if total > 4000 { 4000 } else { total };

    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("get_chapters", "internal_error", "payload overflow");
    }

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
        let a_desc: host::HtmlDescriptor = unsafe { core::mem::transmute(desc_raw) };
        let mut name_buf = [0u8; 256];
        let mut href_buf = [0u8; 512];

        let name = if let Ok(span_desc) = html_select(a_desc, b"span.list_con_zj") {
            let owned = OwnedDescriptor(span_desc);
            text_into(owned.0, &mut name_buf).map(trim_ascii)
        } else {
            text_into(a_desc, &mut name_buf).map(trim_ascii)
        };
        let href = attr_into(a_desc, b"href", &mut href_buf);
        let _ = html_close(a_desc);

        if let (Some(ch_name), Some(ch_href)) = (name, href) {
            if let Some(path) = path_from_href(ch_href) {
                if written > 0 && !write_bytes(payload, &mut c, b",") {
                    break;
                }
                let ok = write_bytes(payload, &mut c, br#"{"id":""#)
                    && append_json_escaped(payload, &mut c, path)
                    && write_bytes(payload, &mut c, br#"","mangaId":""#)
                    && append_json_escaped(payload, &mut c, manga_id)
                    && write_bytes(payload, &mut c, br#"","title":""#)
                    && append_json_escaped(payload, &mut c, ch_name)
                    && write_bytes(payload, &mut c, br#"","chapterNumber":null,"volumeNumber":null,"language":"zh","publishedAt":null,"updatedAt":null,"pageCount":null}"#);
                if !ok {
                    break;
                }
                written += 1;
            }
        }
    }

    if !write_bytes(payload, &mut c, br#"],"page":{"nextCursor":null,"hasMore":false}}"#) {
        return write_error("get_chapters", "internal_error", "payload overflow");
    }
    write_success_payload("get_chapters", c)
}

fn extract_config_host(config: &[u8], out: &mut [u8]) -> Option<usize> {
    let marker = br#"domain":[""#;
    let start = find_subslice(config, marker)? + marker.len();
    let mut end = start;
    while end < config.len() && config[end] != b'"' {
        end += 1;
    }
    if end == start || end >= config.len() || end - start > out.len() {
        return None;
    }
    out[..end - start].copy_from_slice(&config[start..end]);
    Some(end - start)
}

fn extract_js_string<'a>(data: &'a [u8], start: usize) -> Option<(&'a [u8], usize)> {
    if start >= data.len() || data[start] != b'"' {
        return None;
    }
    let mut i = start + 1;
    while i < data.len() {
        if data[i] == b'\\' {
            i += 2;
            continue;
        }
        if data[i] == b'"' {
            return Some((&data[start + 1..i], i + 1));
        }
        i += 1;
    }
    None
}

fn extract_chapter_path<'a>(html: &'a [u8]) -> Option<&'a [u8]> {
    let marker = b"chapterPath";
    let pos = find_subslice(html, marker)? + marker.len();
    let rest = &html[pos..];
    let quote = find_subslice(rest, b"\"")?;
    let (s, _) = extract_js_string(rest, quote)?;
    Some(s)
}

fn write_page_url(
    payload: &mut [u8],
    c: &mut usize,
    host: &[u8],
    chapter_path: &[u8],
    image: &[u8],
) -> bool {
    if image.starts_with(b"https://") || image.starts_with(b"http://") {
        append_json_unescaped_then_escaped(payload, c, image)
    } else if image.starts_with(br#"https:\/\/"#) || image.starts_with(br#"http:\/\/"#) {
        append_json_unescaped_then_escaped(payload, c, image)
    } else if image.starts_with(br#"\/"#) {
        append_json_escaped(payload, c, host) && append_json_unescaped_then_escaped(payload, c, image)
    } else if image.starts_with(b"/") {
        append_json_escaped(payload, c, host) && append_json_unescaped_then_escaped(payload, c, image)
    } else {
        append_json_escaped(payload, c, host)
            && write_bytes(payload, c, b"/")
            && append_json_escaped(payload, c, chapter_path)
            && append_json_unescaped_then_escaped(payload, c, image)
    }
}

fn run_get_pages(req: &[u8]) -> u32 {
    let chapter_id = match extract_json_string(req, b"chapterId") {
        Some(v) => v,
        None => return write_error("get_pages", "invalid_request", "missing chapterId"),
    };

    let config_url = b"http://www.92mh.com/js/config.js";
    let config_len = match fetch_body(config_url) {
        Ok(n) => n,
        Err(e) => {
            let (code, message) = fetch_error_code(e);
            return write_error("get_pages", code, message);
        }
    };
    let host_len = {
        let config = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), config_len) };
        match extract_config_host(config, scratch_b()) {
            Some(n) => n,
            None => return write_error("get_pages", "parse_error", "image host not found"),
        }
    };

    let url_buf = scratch_a();
    let mut url_len = 0usize;
    if !(write_bytes(url_buf, &mut url_len, BASE_URL) && write_bytes(url_buf, &mut url_len, chapter_id)) {
        return write_error("get_pages", "internal_error", "url overflow");
    }
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_len) };

    let html_len = match fetch_body(url) {
        Ok(n) => n,
        Err(e) => {
            let (code, message) = fetch_error_code(e);
            return write_error("get_pages", code, message);
        }
    };
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };
    let host = unsafe { core::slice::from_raw_parts(SCRATCH_B.as_ptr(), host_len) };
    let chapter_path = extract_chapter_path(html).unwrap_or(b"");

    let images_marker = b"chapterImages";
    let marker_pos = match find_subslice(html, images_marker) {
        Some(p) => p + images_marker.len(),
        None => return write_error("get_pages", "parse_error", "chapterImages not found"),
    };
    let rest = &html[marker_pos..];
    let array_start = match find_subslice(rest, b"[") {
        Some(p) => marker_pos + p + 1,
        None => return write_error("get_pages", "parse_error", "chapterImages array not found"),
    };

    let payload = payload_buf();
    let mut c = 0usize;
    if !(write_bytes(payload, &mut c, br#"{"chapterId":""#)
        && append_json_escaped(payload, &mut c, chapter_id)
        && write_bytes(payload, &mut c, br#"","pages":["#))
    {
        return write_error("get_pages", "internal_error", "payload overflow");
    }

    let mut pos = array_start;
    let mut written = 0usize;
    while pos < html.len() && html[pos] != b']' && written < 2000 {
        if html[pos] != b'"' {
            pos += 1;
            continue;
        }
        let (image, next_pos) = match extract_js_string(html, pos) {
            Some(v) => v,
            None => break,
        };
        if written > 0 && !write_bytes(payload, &mut c, b",") {
            break;
        }
        let ok = write_bytes(payload, &mut c, br#"{"id":"page:"#)
            && write_usize(payload, &mut c, written)
            && write_bytes(payload, &mut c, br#"","index":"#)
            && write_usize(payload, &mut c, written)
            && write_bytes(payload, &mut c, br#","image":{"kind":"url","url":""#)
            && write_page_url(payload, &mut c, host, chapter_path, image)
            && write_bytes(payload, &mut c, br#""}}"#);
        if !ok {
            break;
        }
        written += 1;
        pos = next_pos;
    }

    if !write_bytes(payload, &mut c, b"]}") {
        return write_error("get_pages", "internal_error", "payload overflow");
    }
    write_success_payload("get_pages", c)
}

#[no_mangle]
pub extern "C" fn koma_source_init(_manifest_ptr: u32, manifest_len: u32) -> i32 {
    log_info(b"jiuermanhua source init");
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
    log_info(b"jiuermanhua search");
    run_search(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga_list(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga_list", "invalid_request", "empty request"),
    };
    log_info(b"jiuermanhua get_manga_list");
    run_get_manga_list(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga", "invalid_request", "empty request"),
    };
    log_info(b"jiuermanhua get_manga");
    run_get_manga(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_chapters(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_chapters", "invalid_request", "empty request"),
    };
    log_info(b"jiuermanhua get_chapters");
    run_get_chapters(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_pages(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_pages", "invalid_request", "empty request"),
    };
    log_info(b"jiuermanhua get_pages");
    run_get_pages(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_listings(_req_ptr: u32, _req_len: u32) -> u32 {
    response_buffer().write_success("get_listings", br#"{"listings":[]}"#)
}

#[no_mangle]
pub extern "C" fn koma_source_get_home(_req_ptr: u32, _req_len: u32) -> u32 {
    response_buffer().write_success("get_home", br#"{"sections":[]}"#)
}

#[no_mangle]
pub extern "C" fn koma_source_get_filters(_req_ptr: u32, _req_len: u32) -> u32 {
    response_buffer().write_success("get_filters", br#"{"filters":[]}"#)
}

#[no_mangle]
pub extern "C" fn koma_source_get_settings(_req_ptr: u32, _req_len: u32) -> u32 {
    response_buffer().write_success("get_settings", br#"{"settings":[]}"#)
}

#[no_mangle]
pub extern "C" fn koma_source_get_image_request(_req_ptr: u32, _req_len: u32) -> u32 {
    response_buffer().write_error("get_image_request", "unimplemented", "not implemented")
}

#[no_mangle]
pub extern "C" fn koma_source_free(result_ptr: u32) {
    response_buffer().free(result_ptr)
}
