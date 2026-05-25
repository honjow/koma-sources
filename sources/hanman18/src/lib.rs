#![no_std]

extern crate koma_source_sdk;

use koma_source_sdk::host::{self, html_attr, html_close, html_parse, html_select, html_text,
    http_request, log_info};
use koma_source_sdk::json_utils::{
    append_json_escaped, contains_bytes, extract_json_number, extract_json_string, find_subslice,
    write_bytes, write_url_encoded, write_usize,
};
use koma_source_sdk::result::ResultBuffer;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{FetchError, build_get_request, decode_json_body_into, fetch_error_code};

#[link(wasm_import_module = "koma_host")]
extern "C" {
    #[link_name = "html_select_all"]
    fn koma_host_html_select_all(
        descriptor: i32, selector_ptr: *const u8, selector_len: u32,
        out_ptr: *mut u8, out_cap: u32,
    ) -> i32;
}

fn html_select_all(descriptor: i32, selector: &[u8], out: &mut [u8]) -> i32 {
    unsafe {
        koma_host_html_select_all(
            descriptor, selector.as_ptr(), selector.len() as u32,
            out.as_mut_ptr(), out.len() as u32,
        )
    }
}

static mut SELECT_ALL_BUF: [u8; 16000] = [0; 16000];

const BASE_URL: &[u8] = b"https://hanman18.com";
const PAYLOAD_CAP: usize = 1024 * 1024;
const HTTP_OUT_CAP: usize = 2 * 1024 * 1024;
const BODY_CAP: usize = 2 * 1024 * 1024;
const HTTP_REQ_CAP: usize = 2048;
const SCRATCH_CAP: usize = 8192;
const B64_CAP: usize = 512 * 1024;

static mut RESPONSE: ResultBuffer<{ PAYLOAD_CAP + 256 }> = ResultBuffer::new();
static mut PAYLOAD_BUF: [u8; PAYLOAD_CAP] = [0; PAYLOAD_CAP];
static mut HTTP_OUT: [u8; HTTP_OUT_CAP] = [0; HTTP_OUT_CAP];
static mut BODY_BUF: [u8; BODY_CAP] = [0; BODY_CAP];
static mut HTTP_REQ_BUF: [u8; HTTP_REQ_CAP] = [0; HTTP_REQ_CAP];
static mut SCRATCH_A: [u8; SCRATCH_CAP] = [0; SCRATCH_CAP];
static mut SCRATCH_B: [u8; SCRATCH_CAP] = [0; SCRATCH_CAP];
static mut B64_BUF: [u8; B64_CAP] = [0; B64_CAP];

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.hanman18.koma",
    name: "HANMAN18",
    version: "0.1.0",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "HANMAN18 (hanman18.com) manga18 multi-source framework. NSFW.",
    content_rating: "nsfw",
};

const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: true, manga_detail: true, chapters: true, pages: true,
    listings: false, manga_list: true, home: false, filters: false,
    settings: false, image_request: false, credentials: false,
};

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! { loop {} }

fn response_buffer() -> &'static mut ResultBuffer<{ PAYLOAD_CAP + 256 }> {
    unsafe { &mut *core::ptr::addr_of_mut!(RESPONSE) }
}
fn payload_buf() -> &'static mut [u8] { unsafe { &mut *core::ptr::addr_of_mut!(PAYLOAD_BUF) } }
fn http_out() -> &'static mut [u8] { unsafe { &mut *core::ptr::addr_of_mut!(HTTP_OUT) } }
fn body_buf() -> &'static mut [u8] { unsafe { &mut *core::ptr::addr_of_mut!(BODY_BUF) } }
fn http_req_buf() -> &'static mut [u8] { unsafe { &mut *core::ptr::addr_of_mut!(HTTP_REQ_BUF) } }
fn scratch_a() -> &'static mut [u8] { unsafe { &mut *core::ptr::addr_of_mut!(SCRATCH_A) } }
fn scratch_b() -> &'static mut [u8] { unsafe { &mut *core::ptr::addr_of_mut!(SCRATCH_B) } }
fn b64_buf() -> &'static mut [u8] { unsafe { &mut *core::ptr::addr_of_mut!(B64_BUF) } }
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
    if req_ptr == 0 || req_len == 0 { return None; }
    Some(unsafe { core::slice::from_raw_parts(req_ptr as *const u8, req_len as usize) })
}

fn trim_ascii(bytes: &[u8]) -> &[u8] {
    let mut s = 0; let mut e = bytes.len();
    while s < e && matches!(bytes[s], b' ' | b'\t' | b'\n' | b'\r') { s += 1; }
    while e > s && matches!(bytes[e-1], b' ' | b'\t' | b'\n' | b'\r') { e -= 1; }
    &bytes[s..e]
}

fn fetch_body(url: &[u8]) -> Result<usize, FetchError> {
    let req_len = build_get_request(http_req_buf(), url, None, &[]).ok_or(FetchError::Network)?;
    let mut resp_len = 0usize;
    let mut failed = true;
    for attempt in 0..3u8 {
        match http_request(&http_req_buf()[..req_len], http_out()) {
            Ok(n) => { resp_len = n; failed = false; break; }
            Err(_) => { if attempt < 2 { log_info(b"hanman18: retry"); } }
        }
    }
    if failed { return Err(FetchError::Network); }
    decode_json_body_into(&http_out()[..resp_len], body_buf())
}

struct OwnedDescriptor(host::HtmlDescriptor);
impl Drop for OwnedDescriptor {
    fn drop(&mut self) { let _ = html_close(self.0); }
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
    if href.is_empty() { return None; }
    if href[0] == b'/' { return Some(href); }
    let mut sc = 0u8; let mut idx = 0;
    while idx < href.len() {
        if href[idx] == b'/' { sc += 1; if sc == 3 { return Some(&href[idx..]); } }
        idx += 1;
    }
    None
}

fn parse_usize(bytes: &[u8]) -> Option<usize> {
    let mut n = 0; let mut any = false;
    for &b in bytes {
        if b < b'0' || b > b'9' { return None; }
        n = n * 10 + (b - b'0') as usize; any = true;
    }
    if any { Some(n) } else { None }
}

const B64: [i8; 128] = [
    -1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,
    -1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,
    -1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,62,-1,-1,-1,63,
    52,53,54,55,56,57,58,59,60,61,-1,-1,-1,-1,-1,-1,
    -1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9,10,11,12,13,14,
    15,16,17,18,19,20,21,22,23,24,25,-1,-1,-1,-1,-1,
    -1,26,27,28,29,30,31,32,33,34,35,36,37,38,39,40,
    41,42,43,44,45,46,47,48,49,50,51,-1,-1,-1,-1,-1,
];

fn base64_decode(input: &[u8], out: &mut [u8]) -> Option<usize> {
    let mut p = 0; let mut b = [0u8; 4]; let mut bc = 0;
    for &ch in input {
        if ch == b' ' || ch == b'\n' || ch == b'\r' || ch == b'\t' || ch == b'=' { continue; }
        if (ch as usize) >= 128 { return None; }
        let v = B64[ch as usize];
        if v < 0 { return None; }
        b[bc] = v as u8; bc += 1;
        if bc == 4 {
            if p + 3 > out.len() { return None; }
            out[p] = b[0] << 2 | b[1] >> 4;
            out[p+1] = b[1] << 4 | b[2] >> 2;
            out[p+2] = b[2] << 6 | b[3];
            p += 3; bc = 0;
        }
    }
    if bc >= 2 { if p+1 > out.len() { return None; } out[p] = b[0] << 2 | b[1] >> 4; p += 1; }
    if bc >= 3 { if p+1 > out.len() { return None; } out[p] = b[1] << 4 | b[2] >> 2; p += 1; }
    Some(p)
}

// ============================================================
// Operations
// ============================================================

fn parse_manga_list(html: &[u8], op: &str) -> u32 {
    let document = match html_parse(html) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error(op, "parse_error", "html_parse failed"),
    };
    let select_buf = unsafe { &mut *core::ptr::addr_of_mut!(SELECT_ALL_BUF) };
    let count = html_select_all(document.0.raw(), b"div.story_item", select_buf);
    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("parse", "internal_error", "payload overflow");
    }
    let max_items = count.min(500) as usize;
    let mut written = 0usize;
    for i in 0..max_items {
        let offset = i * 4;
        if offset + 4 > select_buf.len() { break; }
        let desc_raw = i32::from_le_bytes([
            select_buf[offset], select_buf[offset + 1],
            select_buf[offset + 2], select_buf[offset + 3],
        ]);
        if desc_raw < 0 { continue; }
        let item_desc: host::HtmlDescriptor = unsafe { core::mem::transmute(desc_raw) };
        let mut title_buf_a = [0u8; 512];
        let mut href_buf_a = [0u8; 512];
        let mut title_text = None;
        let mut manga_href = None;
        if let Ok(a_desc) = html_select(item_desc, b"div.mg_name a") {
            let a_owned = OwnedDescriptor(a_desc);
            title_text = text_into(a_owned.0, &mut title_buf_a).map(|s| trim_ascii(s));
            manga_href = attr_into(a_owned.0, b"href", &mut href_buf_a);
        }
        if manga_href.is_none() {
            if let Ok(a_desc) = html_select(item_desc, b"div.story_images a") {
                let a_owned = OwnedDescriptor(a_desc);
                manga_href = attr_into(a_owned.0, b"href", &mut href_buf_a);
                if title_text.is_none() {
                    title_text = attr_into(a_owned.0, b"title", &mut title_buf_a);
                }
            }
        }
        let mut cover_buf_a = [0u8; 1024];
        let mut cover_url: &[u8] = b"";
        if let Ok(img_desc) = html_select(item_desc, b"img") {
            let img_owned = OwnedDescriptor(img_desc);
            if let Some(src) = attr_into(img_owned.0, b"src", &mut cover_buf_a) {
                cover_url = src;
            }
        }
        if let (Some(title), Some(href)) = (title_text, manga_href) {
            let path = match path_from_href(href) {
                Some(p) => p,
                None => { let _ = html_close(item_desc); continue; }
            };
            if written > 0 {
                if !write_bytes(payload, &mut c, b",") { let _ = html_close(item_desc); break; }
            }
            let ok = write_bytes(payload, &mut c, br#"{"id":"manga:"#)
                && append_json_escaped(payload, &mut c, path)
                && write_bytes(payload, &mut c, br#"","title":""#)
                && append_json_escaped(payload, &mut c, title)
                && write_bytes(payload, &mut c, br#"","cover":{"kind":"url","url":""#)
                && append_json_escaped(payload, &mut c, cover_url)
                && write_bytes(payload, &mut c, br#""},"authors":[],"status":"unknown","contentRating":"nsfw","description":"","sourceTags":["hanman18"]}"#);
            if !ok { let _ = html_close(item_desc); break; }
            written += 1;
        }
        let _ = html_close(item_desc);
    }
    let has_more = written > 0;
    if !write_bytes(payload, &mut c, br#"],"page":{"nextCursor":"#)
        || !if has_more { write_bytes(payload, &mut c, b"\"next\"") }
           else { write_bytes(payload, &mut c, b"null") }
        || !write_bytes(payload, &mut c, br#","hasMore":"#)
        || !write_bytes(payload, &mut c, if has_more { b"true" } else { b"false" })
        || !write_bytes(payload, &mut c, b"}}")
    {
        return write_error(op, "internal_error", "payload overflow");
    }
    write_success_payload(op, c)
}

fn run_search(req: &[u8]) -> u32 {
    let query = match extract_json_string(req, b"query") {
        Some(q) => q,
        None => return write_error("search", "invalid_request", "missing query"),
    };
    let page = extract_json_number(req, b"page").and_then(parse_usize).unwrap_or(1);
    let url_buf = scratch_a();
    let mut url_cursor = 0usize;
    if !(write_bytes(url_buf, &mut url_cursor, BASE_URL)
        && write_bytes(url_buf, &mut url_cursor, b"/list-manga/")
        && write_usize(url_buf, &mut url_cursor, page)
        && write_bytes(url_buf, &mut url_cursor, b"?search=")
        && write_url_encoded(url_buf, &mut url_cursor, query))
    {
        return write_error("search", "internal_error", "url overflow");
    }
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_cursor) };
    let html_len = match fetch_body(url) {
        Ok(n) => n,
        Err(e) => { let (c, m) = fetch_error_code(e); return write_error("search", c, m); }
    };
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };
    parse_manga_list(html, "search")
}

fn run_get_manga_list(req: &[u8]) -> u32 {
    let page = extract_json_number(req, b"page").and_then(parse_usize).unwrap_or(1);
    let url_buf = scratch_a();
    let mut url_cursor = 0usize;
    if !(write_bytes(url_buf, &mut url_cursor, BASE_URL)
        && write_bytes(url_buf, &mut url_cursor, b"/list-manga/")
        && write_usize(url_buf, &mut url_cursor, page)
        && write_bytes(url_buf, &mut url_cursor, b"?order_by=views"))
    {
        return write_error("get_manga_list", "internal_error", "url overflow");
    }
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_cursor) };
    let html_len = match fetch_body(url) {
        Ok(n) => n,
        Err(e) => { let (c, m) = fetch_error_code(e); return write_error("get_manga_list", c, m); }
    };
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };
    parse_manga_list(html, "get_manga_list")
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
    let path = &manga_id[prefix.len()..];
    let url_buf = scratch_a();
    let mut url_cursor = 0usize;
    if !(write_bytes(url_buf, &mut url_cursor, BASE_URL)
        && write_bytes(url_buf, &mut url_cursor, path))
    {
        return write_error("get_manga", "internal_error", "url overflow");
    }
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_cursor) };
    let html_len = match fetch_body(url) {
        Ok(n) => n,
        Err(e) => { let (c, m) = fetch_error_code(e); return write_error("get_manga", c, m); }
    };
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };
    let document = match html_parse(html) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error("get_manga", "parse_error", "html_parse failed"),
    };
    let mut title_buf = [0u8; 512];
    let title = if let Ok(desc) = html_select(document.0, b"div.detail_name h1") {
        let owned = OwnedDescriptor(desc);
        text_into(owned.0, &mut title_buf).map(|s| trim_ascii(s))
    } else { None };
    let mut cover_buf = [0u8; 1024];
    let cover = if let Ok(desc) = html_select(document.0, b"div.detail_avatar img") {
        let owned = OwnedDescriptor(desc);
        attr_into(owned.0, b"src", &mut cover_buf)
    } else { None };
    let mut author_buf = [0u8; 256];
    let mut author_len: usize = 0;
    let mut status_bytes: &[u8] = b"unknown";
    let select_buf = unsafe { &mut *core::ptr::addr_of_mut!(SELECT_ALL_BUF) };
    let item_count = html_select_all(document.0.raw(), b"div.detail_listInfo div.item", select_buf);
    let total_items = if item_count > 0 { item_count as usize } else { 0 };
    for i in 0..total_items {
        let offset = i * 4;
        if offset + 4 > select_buf.len() { break; }
        let desc_raw = i32::from_le_bytes([
            select_buf[offset], select_buf[offset + 1],
            select_buf[offset + 2], select_buf[offset + 3],
        ]);
        if desc_raw < 0 { continue; }
        let item_desc: host::HtmlDescriptor = unsafe { core::mem::transmute(desc_raw) };
        if let Ok(label_desc) = html_select(item_desc, b"div.info_label") {
            let label_owned = OwnedDescriptor(label_desc);
            let mut label_buf = [0u8; 128];
            if let Some(label) = text_into(label_owned.0, &mut label_buf) {
                if author_len == 0 && contains_bytes(label, b"Author") {
                    if let Ok(val_desc) = html_select(item_desc, b"div.info_value") {
                        let val_owned = OwnedDescriptor(val_desc);
                        let mut tmp_buf = [0u8; 256];
                        if let Some(t) = text_into(val_owned.0, &mut tmp_buf).map(|s| trim_ascii(s)) {
                            if !contains_bytes(t, b"Updating") && !t.is_empty() {
                                let copy_len = t.len().min(author_buf.len());
                                author_buf[..copy_len].copy_from_slice(&t[..copy_len]);
                                author_len = copy_len;
                            }
                        }
                    }
                } else if contains_bytes(label, b"Status") {
                    if let Ok(val_desc) = html_select(item_desc, b"div.info_value") {
                        let val_owned = OwnedDescriptor(val_desc);
                        let mut val_buf = [0u8; 128];
                        if let Some(val) = text_into(val_owned.0, &mut val_buf) {
                            if contains_bytes(val, b"Completed") {
                                status_bytes = b"completed";
                            } else if contains_bytes(val, b"On Going") || contains_bytes(val, b"Ongoing") {
                                status_bytes = b"ongoing";
                            }
                        }
                    }
                }
            }
        }
        let _ = html_close(item_desc);
    }
    let mut desc_buf = [0u8; 4096];
    let desc = if let Ok(desc_sel) = html_select(document.0, b"div.detail_reviewContent") {
        let owned = OwnedDescriptor(desc_sel);
        text_into(owned.0, &mut desc_buf).map(|s| trim_ascii(s))
    } else { None };
    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"manga":{"id":"manga:"#)
        && append_json_escaped(payload, &mut c, path)
        && write_bytes(payload, &mut c, br#"","title":""#)
        && append_json_escaped(payload, &mut c, title.unwrap_or(path))
        && write_bytes(payload, &mut c, br#"","alternateTitles":[],"description":""#)
        && append_json_escaped(payload, &mut c, desc.unwrap_or(b""))
        && write_bytes(payload, &mut c, br#"","cover":{"kind":"url","url":""#)
        && append_json_escaped(payload, &mut c, cover.unwrap_or(b""))
        && write_bytes(payload, &mut c, br#""},"authors":["#)
        && if author_len > 0 {
            write_bytes(payload, &mut c, b"\"")
                && append_json_escaped(payload, &mut c, &author_buf[..author_len])
                && write_bytes(payload, &mut c, b"\"")
        } else {
            write_bytes(payload, &mut c, b"")
        }
        && write_bytes(payload, &mut c, br#"],"artists":[],"status":""#)
        && write_bytes(payload, &mut c, status_bytes)
        && write_bytes(payload, &mut c, br#"","contentRating":"nsfw","language":"zh","tags":[],"links":[{"kind":"source","url":""#)
        && append_json_escaped(payload, &mut c, url)
        && write_bytes(payload, &mut c, br#""}]}}"#);
    if !ok { return write_error("get_manga", "internal_error", "payload overflow"); }
    write_success_payload("get_manga", c)
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
    let path = &manga_id[prefix.len()..];
    let url_buf = scratch_a();
    let mut url_cursor = 0usize;
    if !(write_bytes(url_buf, &mut url_cursor, BASE_URL)
        && write_bytes(url_buf, &mut url_cursor, path))
    {
        return write_error("get_chapters", "internal_error", "url overflow");
    }
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_cursor) };
    let html_len = match fetch_body(url) {
        Ok(n) => n,
        Err(e) => { let (c, m) = fetch_error_code(e); return write_error("get_chapters", c, m); }
    };
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };
    let document = match html_parse(html) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error("get_chapters", "parse_error", "html_parse failed"),
    };
    let chapter_box = match html_select(document.0, b"div.chapter_box") {
        Ok(d) => d,
        Err(_) => return write_error("get_chapters", "parse_error", "chapter_box not found"),
    };
    let select_buf = unsafe { &mut *core::ptr::addr_of_mut!(SELECT_ALL_BUF) };
    let count = html_select_all(chapter_box.raw(), b"a.chapter_num", select_buf);
    let _ = html_close(chapter_box);
    let total = count.min(2000) as usize;
    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("get_chapters", "internal_error", "payload overflow");
    }
    let mut written = 0usize;
    // Reverse: site lists newest first, we want oldest first
    for i in (0..total).rev() {
        let offset = i * 4;
        if offset + 4 > select_buf.len() { break; }
        let desc_raw = i32::from_le_bytes([
            select_buf[offset], select_buf[offset + 1],
            select_buf[offset + 2], select_buf[offset + 3],
        ]);
        if desc_raw < 0 { continue; }
        let a_desc: host::HtmlDescriptor = unsafe { core::mem::transmute(desc_raw) };
        let mut name_buf = [0u8; 512];
        let mut href_buf = [0u8; 512];
        let name = text_into(a_desc, &mut name_buf).map(|s| trim_ascii(s));
        let href = attr_into(a_desc, b"href", &mut href_buf);
        let _ = html_close(a_desc);
        if let (Some(name), Some(href_val)) = (name, href) {
            let chapter_path = path_from_href(href_val).unwrap_or(href_val);
            if written > 0 {
                if !write_bytes(payload, &mut c, b",") { break; }
            }
            let ok = write_bytes(payload, &mut c, br#"{"id":"chapter:"#)
                && append_json_escaped(payload, &mut c, path)
                && write_bytes(payload, &mut c, b":")
                && append_json_escaped(payload, &mut c, chapter_path)
                && write_bytes(payload, &mut c, br#"","mangaId":"manga:"#)
                && append_json_escaped(payload, &mut c, path)
                && write_bytes(payload, &mut c, br#"","title":""#)
                && append_json_escaped(payload, &mut c, name)
                && write_bytes(payload, &mut c, br#"","chapterNumber":null,"volumeNumber":null,"language":"zh","publishedAt":null,"updatedAt":null,"pageCount":null}"#);
            if !ok { break; }
            written += 1;
        }
    }
    if !write_bytes(payload, &mut c, br#"],"page":{"nextCursor":null,"hasMore":false}}"#) {
        return write_error("get_chapters", "internal_error", "payload overflow");
    }
    write_success_payload("get_chapters", c)
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
    // chapter ID format: chapter:{manga_path}:{chapter_path}
    // Find the second colon to split
    let sep = match find_subslice(rest, b":") {
        Some(v) => v,
        None => return write_error("get_pages", "invalid_request", "chapterId missing path separator"),
    };
    let _manga_path = &rest[..sep];
    let chapter_path = &rest[sep + 1..];
    // Build URL
    let url_buf = scratch_a();
    let mut url_cursor = 0usize;
    if !(write_bytes(url_buf, &mut url_cursor, BASE_URL)
        && write_bytes(url_buf, &mut url_cursor, chapter_path))
    {
        return write_error("get_pages", "internal_error", "url overflow");
    }
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_cursor) };
    let html_len = match fetch_body(url) {
        Ok(n) => n,
        Err(e) => { let (c, m) = fetch_error_code(e); return write_error("get_pages", c, m); }
    };
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };

    // Find slides_p_path = [...] in the HTML
    let marker = b"slides_p_path";
    let marker_start = match find_subslice(html, marker) {
        Some(v) => v,
        None => return write_error("get_pages", "parse_error", "slides_p_path not found"),
    };
    // Find the opening bracket after the marker
    let mut bracket_pos = marker_start + marker.len();
    while bracket_pos < html.len() && html[bracket_pos] != b'[' {
        bracket_pos += 1;
    }
    if bracket_pos >= html.len() {
        return write_error("get_pages", "parse_error", "array bracket not found");
    }
    // Find the closing bracket
    let array_start = bracket_pos;
    let mut array_end = array_start + 1;
    let mut depth = 1u8;
    while array_end < html.len() && depth > 0 {
        if html[array_end] == b'[' { depth += 1; }
        else if html[array_end] == b']' { depth -= 1; }
        array_end += 1;
    }
    let array_data = &html[array_start..array_end];

    // Parse the JSON array of base64 strings manually
    // Split by comma, each element is a quoted base64 string
    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"chapterId":"chapter:"#)
        && append_json_escaped(payload, &mut c, rest)
        && write_bytes(payload, &mut c, br#"","pages":["#);
    if !ok {
        return write_error("get_pages", "internal_error", "payload overflow");
    }

    let mut page_idx = 0usize;
    let mut pos = 1; // skip opening [
    while pos < array_data.len() {
        // Find opening quote
        while pos < array_data.len() && array_data[pos] != b'"' { pos += 1; }
        if pos >= array_data.len() { break; }
        pos += 1; // skip quote
        let val_start = pos;
        // Find closing quote (handle escaped quotes)
        while pos < array_data.len() {
            if array_data[pos] == b'"' { break; }
            pos += 1;
        }
        if pos >= array_data.len() { break; }
        let b64_str = &array_data[val_start..pos];
        pos += 1; // skip closing quote

        if b64_str.is_empty() { continue; }

        // Decode base64 into B64_BUF
        let b64_out = b64_buf();
        let decoded_len = match base64_decode(b64_str, b64_out) {
            Some(n) => n,
            None => continue,
        };
        let decoded = &b64_out[..decoded_len];

        // If URL starts with /, prepend base URL
        let img_url: &[u8] = if !decoded.is_empty() && decoded[0] == b'/' {
            // Write full URL to scratch_b
            let sb = scratch_b();
            let mut sb_c = 0usize;
            if write_bytes(sb, &mut sb_c, BASE_URL)
                && write_bytes(sb, &mut sb_c, decoded)
            {
                unsafe { core::slice::from_raw_parts(SCRATCH_B.as_ptr(), sb_c) }
            } else {
                decoded
            }
        } else {
            decoded
        };

        if page_idx > 0 {
            if !write_bytes(payload, &mut c, b",") { break; }
        }
        let page_ok = write_bytes(payload, &mut c, br#"{"id":"page:"#)
            && write_usize(payload, &mut c, page_idx)
            && write_bytes(payload, &mut c, br#"","index":""#)
            && write_usize(payload, &mut c, page_idx)
            && write_bytes(payload, &mut c, br#"","image":{"kind":"url","url":""#)
            && append_json_escaped(payload, &mut c, img_url)
            && write_bytes(payload, &mut c, br#""}}"#);
        if !page_ok { break; }
        page_idx += 1;
    }

    if !write_bytes(payload, &mut c, b"]}") {
        return write_error("get_pages", "internal_error", "payload overflow");
    }
    write_success_payload("get_pages", c)
}

// ============================================================
// WASM exports
// ============================================================

#[no_mangle]
pub extern "C" fn koma_source_init(_manifest_ptr: u32, manifest_len: u32) -> i32 {
    log_info(b"hanman18 source init");
    if host::check_cancel() { return -2; }
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
    log_info(b"hanman18 search");
    run_search(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga", "invalid_request", "empty request"),
    };
    log_info(b"hanman18 get_manga");
    run_get_manga(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_chapters(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_chapters", "invalid_request", "empty request"),
    };
    log_info(b"hanman18 get_chapters");
    run_get_chapters(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_pages(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_pages", "invalid_request", "empty request"),
    };
    log_info(b"hanman18 get_pages");
    run_get_pages(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga_list(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga_list", "invalid_request", "empty request"),
    };
    log_info(b"hanman18 get_manga_list");
    run_get_manga_list(req)
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
