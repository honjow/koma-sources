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

// Additional host import for html_select_all
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

/// Buffer to hold descriptors returned by html_select_all (up to 4000 results)
static mut SELECT_ALL_BUF: [u8; 16000] = [0; 16000]; // 4000 * 4 bytes

const BASE_URL: &[u8] = b"https://www.iqiyi.com";
koma_source_sdk::koma_source_buffers! {
    payload: 1024 * 1024,
    http_out: 2 * 1024 * 1024,
    body: 2 * 1024 * 1024,
    http_req: 2048,
    scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.iqiyi.koma",
    name: "爱奇艺叭嗒 (Iqiyi)",
    version: "0.1.0",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "Iqiyi Bada Manhua (iqiyi.com/manhua) HTML scraping source.",
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


fn read_request<'a>(req_ptr: u32, req_len: u32) -> Option<&'a [u8]> {
    if req_ptr == 0 || req_len == 0 {
        return None;
    }
    Some(unsafe { core::slice::from_raw_parts(req_ptr as *const u8, req_len as usize) })
}

fn fetch_body(url: &[u8]) -> Result<usize, FetchError> {
    let req_len = build_get_request(http_req_buf(), url, None, &[]).ok_or(FetchError::Network)?;
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
                    log_info(b"iqiyi: http transport error, retrying");
                }
            }
        }
    }
    if transport_failed {
        return Err(FetchError::Network);
    }
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

/// Extract relative path from an href, stripping the domain prefix.
fn manga_path_from_href(href: &[u8]) -> Option<&[u8]> {
    if href.is_empty() {
        return None;
    }
    if href[0] == b'/' {
        return Some(href);
    }
    // Strip domain: find the third '/'
    let mut slash_count = 0u8;
    let mut idx = 0usize;
    while idx < href.len() {
        if href[idx] == b'/' {
            slash_count += 1;
            if slash_count == 3 {
                return Some(&href[idx..]);
            }
        }
        idx += 1;
    }
    None
}

/// Parse usize from decimal bytes
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

// ============================================================
// Operations
// ============================================================

/// Parse the category list page (get_manga_list)
/// Items: li.cartoon-hot-list > a.cartoon-cover[href] > img[src]
///        + a.cartoon-item-tit[title][href]
fn parse_category_list(html: &[u8]) -> u32 {
    let document = match html_parse(html) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error("parse", "parse_error", "html_parse failed"),
    };

    let select_buf = unsafe { &mut *core::ptr::addr_of_mut!(SELECT_ALL_BUF) };
    let count = html_select_all(document.0.raw(), b"li.cartoon-hot-list", select_buf);

    let payload = payload_buf();
    let mut c = 0usize;

    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("parse", "internal_error", "payload overflow");
    }

    let max_items = if count > 0 { count as usize } else { 0 };
    let max_items = if max_items > 500 { 500 } else { max_items };
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

        // title + href from a.cartoon-item-tit
        let mut title_buf_a = [0u8; 512];
        let mut href_buf_a = [0u8; 512];
        let mut title_text = None;
        let mut manga_href = None;

        if let Ok(a_desc) = html_select(item_desc, b"a.cartoon-item-tit") {
            let a_owned = OwnedDescriptor(a_desc);
            title_text = text_into(a_owned.0, &mut title_buf_a).map(|s| trim_ascii(s));
            manga_href = attr_into(a_owned.0, b"href", &mut href_buf_a);
        }

        // fallback: try the cover link
        if manga_href.is_none() {
            if let Ok(a_desc) = html_select(item_desc, b"a.cartoon-cover") {
                let a_owned = OwnedDescriptor(a_desc);
                if title_text.is_none() {
                    // Get title from img alt
                    if let Ok(img_desc) = html_select(a_owned.0, b"img") {
                        let img_owned = OwnedDescriptor(img_desc);
                        title_text = attr_into(img_owned.0, b"alt", &mut title_buf_a);
                    }
                }
                manga_href = attr_into(a_owned.0, b"href", &mut href_buf_a);
            }
        }

        // cover from img
        let mut cover_buf_a = [0u8; 1024];
        let mut cover_url: &[u8] = b"";
        if let Ok(img_desc) = html_select(item_desc, b"img") {
            let img_owned = OwnedDescriptor(img_desc);
            if let Some(src) = attr_into(img_owned.0, b"src", &mut cover_buf_a) {
                cover_url = src;
            }
        }

        if let (Some(title), Some(href)) = (title_text, manga_href) {
            let path = match manga_path_from_href(href) {
                Some(p) => p,
                None => {
                    let _ = html_close(item_desc);
                    continue;
                }
            };

            if written > 0 {
                if !write_bytes(payload, &mut c, b",") {
                    let _ = html_close(item_desc);
                    break;
                }
            }

            let ok = write_bytes(payload, &mut c, br#"{"id":"manga:"#)
                && append_json_escaped(payload, &mut c, path)
                && write_bytes(payload, &mut c, br#"","title":""#)
                && append_json_escaped(payload, &mut c, title)
                && write_bytes(payload, &mut c, br#"","cover":{"kind":"url","url":""#)
                && append_json_escaped(payload, &mut c, cover_url)
                && write_bytes(payload, &mut c, br#""},"authors":[],"status":"unknown","contentRating":"safe","description":"","sourceTags":["iqiyi"]}"#);
            if !ok {
                let _ = html_close(item_desc);
                break;
            }
            written += 1;
        }

        let _ = html_close(item_desc);
    }

    // Check pagination: look for "下一页" link with class a1
    let has_more = contains_bytes(html, b"class=\"a1\"")
        && contains_bytes(html, b"\xe4\xb8\x8b\xe4\xb8\x80\xe9\xa1\xb5");

    if !write_bytes(payload, &mut c, br#"],"page":{"nextCursor":"#)
        || !if has_more && written > 0 {
            write_bytes(payload, &mut c, b"\"next\"")
        } else {
            write_bytes(payload, &mut c, b"null")
        }
        || !write_bytes(payload, &mut c, br#","hasMore":"#)
        || !write_bytes(payload, &mut c, if has_more && written > 0 { b"true" } else { b"false" })
        || !write_bytes(payload, &mut c, b"}}")
    {
        return write_error("parse", "internal_error", "payload overflow");
    }

    write_success_payload("parse", c)
}

/// Parse search results page
/// Items: li.stacksBook > div.stacksBook-con > div.stacksBookCover > a[href] > img[src]
///        + h3.stacksBook-tit > a[title]
fn parse_search_list(html: &[u8]) -> u32 {
    let document = match html_parse(html) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error("parse", "parse_error", "html_parse failed"),
    };

    let select_buf = unsafe { &mut *core::ptr::addr_of_mut!(SELECT_ALL_BUF) };
    let count = html_select_all(document.0.raw(), b"li.stacksBook", select_buf);

    let payload = payload_buf();
    let mut c = 0usize;

    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("parse", "internal_error", "payload overflow");
    }

    let max_items = if count > 0 { count as usize } else { 0 };
    let max_items = if max_items > 500 { 500 } else { max_items };
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

        // Get title and href from h3.stacksBook-tit > a
        let mut title_buf_a = [0u8; 512];
        let mut href_buf_a = [0u8; 512];
        let mut title_text = None;
        let mut manga_href = None;

        if let Ok(h3_desc) = html_select(item_desc, b"h3.stacksBook-tit a") {
            let h3_owned = OwnedDescriptor(h3_desc);
            manga_href = attr_into(h3_owned.0, b"href", &mut href_buf_a);
            // Get title from the "title" attribute (it has the clean title without highlight spans)
            title_text = attr_into(h3_owned.0, b"title", &mut title_buf_a);
            if title_text.is_none() {
                title_text = text_into(h3_owned.0, &mut title_buf_a).map(|s| trim_ascii(s));
            }
        }

        // Get cover image
        let mut cover_buf_a = [0u8; 1024];
        let mut cover_url: &[u8] = b"";
        if let Ok(img_desc) = html_select(item_desc, b"div.stacksBookCover img") {
            let img_owned = OwnedDescriptor(img_desc);
            if let Some(src) = attr_into(img_owned.0, b"src", &mut cover_buf_a) {
                cover_url = src;
            }
        }

        if let (Some(title), Some(href)) = (title_text, manga_href) {
            let path = match manga_path_from_href(href) {
                Some(p) => p,
                None => {
                    let _ = html_close(item_desc);
                    continue;
                }
            };

            if written > 0 {
                if !write_bytes(payload, &mut c, b",") {
                    let _ = html_close(item_desc);
                    break;
                }
            }

            let ok = write_bytes(payload, &mut c, br#"{"id":"manga:"#)
                && append_json_escaped(payload, &mut c, path)
                && write_bytes(payload, &mut c, br#"","title":""#)
                && append_json_escaped(payload, &mut c, title)
                && write_bytes(payload, &mut c, br#"","cover":{"kind":"url","url":""#)
                && append_json_escaped(payload, &mut c, cover_url)
                && write_bytes(payload, &mut c, br#""},"authors":[],"status":"unknown","contentRating":"safe","description":"","sourceTags":["iqiyi"]}"#);
            if !ok {
                let _ = html_close(item_desc);
                break;
            }
            written += 1;
        }

        let _ = html_close(item_desc);
    }

    // Check pagination
    let has_more = contains_bytes(html, b"\xe4\xb8\x8b\xe4\xb8\x80\xe9\xa1\xb5");

    if !write_bytes(payload, &mut c, br#"],"page":{"nextCursor":"#)
        || !if has_more && written > 0 {
            write_bytes(payload, &mut c, b"\"next\"")
        } else {
            write_bytes(payload, &mut c, b"null")
        }
        || !write_bytes(payload, &mut c, br#","hasMore":"#)
        || !write_bytes(payload, &mut c, if has_more && written > 0 { b"true" } else { b"false" })
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
    let page = extract_json_number(req, b"page")
        .and_then(parse_usize)
        .unwrap_or(1);

    let url_buf = scratch_a();
    let mut url_cursor = 0usize;
    if !(write_bytes(url_buf, &mut url_cursor, BASE_URL)
        && write_bytes(url_buf, &mut url_cursor, b"/manhua/search-keyword=")
        && write_url_encoded(url_buf, &mut url_cursor, query)
        && write_bytes(url_buf, &mut url_cursor, b"_")
        && write_usize(url_buf, &mut url_cursor, page))
    {
        return write_error("search", "internal_error", "url overflow");
    }
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_cursor) };

    let html_len = match fetch_body(url) {
        Ok(n) => n,
        Err(e) => { let (c, m) = fetch_error_code(e); return write_error("search", c, m); }
    };
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };

    parse_search_list(html)
}

fn run_get_manga_list(req: &[u8]) -> u32 {
    let page = extract_json_number(req, b"page")
        .and_then(parse_usize)
        .unwrap_or(1);

    let url_buf = scratch_a();
    let mut url_cursor = 0usize;
    if !(write_bytes(url_buf, &mut url_cursor, BASE_URL)
        && write_bytes(url_buf, &mut url_cursor, b"/manhua/category/%E5%85%A8%E9%83%A8_-1_-1_9_")
        && write_usize(url_buf, &mut url_cursor, page)
        && write_bytes(url_buf, &mut url_cursor, b"/"))
    {
        return write_error("get_manga_list", "internal_error", "url overflow");
    }
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_cursor) };

    let html_len = match fetch_body(url) {
        Ok(n) => n,
        Err(e) => { let (c, m) = fetch_error_code(e); return write_error("get_manga_list", c, m); }
    };
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };

    parse_category_list(html)
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

    // Build URL
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

    // title: div.detail-info h1
    let mut title_buf = [0u8; 512];
    let title = if let Ok(desc) = html_select(document.0, b"div.detail-info h1") {
        let owned = OwnedDescriptor(desc);
        text_into(owned.0, &mut title_buf).map(|s| trim_ascii(s))
    } else {
        None
    };

    // cover: div.detail-cover > img src
    let mut cover_buf = [0u8; 1024];
    let cover = if let Ok(desc) = html_select(document.0, b"div.detail-cover img") {
        let owned = OwnedDescriptor(desc);
        attr_into(owned.0, b"src", &mut cover_buf)
    } else {
        None
    };

    // author: p.author > span.author-name
    let mut author_buf = [0u8; 256];
    let mut author_text: &[u8] = b"";
    if let Ok(desc) = html_select(document.0, b"p.author span.author-name") {
        let owned = OwnedDescriptor(desc);
        if let Some(t) = text_into(owned.0, &mut author_buf).map(|s| trim_ascii(s)) {
            author_text = t;
        }
    }

    // description: p.detail-docu
    let mut desc_buf = [0u8; 4096];
    let desc = if let Ok(desc_sel) = html_select(document.0, b"p.detail-docu") {
        let owned = OwnedDescriptor(desc_sel);
        text_into(owned.0, &mut desc_buf).map(|s| trim_ascii(s))
    } else {
        None
    };

    // genre: span.detail-categ text
    let mut genre_buf_a = [0u8; 128];
    let genre = if let Ok(desc) = html_select(document.0, b"span.detail-categ") {
        let owned = OwnedDescriptor(desc);
        text_into(owned.0, &mut genre_buf_a).map(|s| trim_ascii(s))
    } else {
        None
    };

    // status: check for "完结" / "连载" in catalog-title
    let status = if contains_bytes(html, b"\xe5\xae\x8c\xe7\xbb\x93") {
        b"completed" as &[u8]
    } else if contains_bytes(html, b"\xe8\xbf\x9e\xe8\xbd\xbd") {
        b"ongoing" as &[u8]
    } else {
        b"unknown" as &[u8]
    };

    // Build response JSON
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
        && if !author_text.is_empty() {
            write_bytes(payload, &mut c, b"\"")
                && append_json_escaped(payload, &mut c, author_text)
                && write_bytes(payload, &mut c, b"\"")
        } else {
            write_bytes(payload, &mut c, b"")
        }
        && write_bytes(payload, &mut c, br#"],"artists":[],"status":""#)
        && write_bytes(payload, &mut c, status)
        && write_bytes(payload, &mut c, br#"","contentRating":"safe","language":"zh","tags":["#);

    // Write genre as tag if present (JSON string inside array)
    if let Some(g) = genre {
        let _ = write_bytes(payload, &mut c, b"\"")
            && append_json_escaped(payload, &mut c, g)
            && write_bytes(payload, &mut c, b"\"");
    }

    let ok = ok
        && write_bytes(payload, &mut c, br#"],"links":[{"kind":"source","url":""#)
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
    let prefix = b"manga:";
    if manga_id.len() <= prefix.len() || &manga_id[..prefix.len()] != prefix {
        return write_error("get_chapters", "invalid_request", "unexpected mangaId");
    }
    let path = &manga_id[prefix.len()..];

    // Build URL to detail page (chapters are on the detail page)
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

    // Parse chapter list from the detail page
    // Try div.chapter-container ol[data-catalogcont="1"] > li > a
    let select_buf = unsafe { &mut *core::ptr::addr_of_mut!(SELECT_ALL_BUF) };
    
    // First try the initial 20 chapters (ol[data-catalogcont="1"])
    let chapter_list = match html_select(document.0, b"ol[data-catalogcont=\"1\"]") {
        Ok(d) => d,
        Err(_) => {
            // Fallback: try div.chapter-container directly
            match html_select(document.0, b"div.chapter-container") {
                Ok(d) => d,
                Err(_) => return write_error("get_chapters", "parse_error", "chapter container not found"),
            }
        }
    };

    let count = html_select_all(chapter_list.raw(), b"li a", select_buf);
    let _ = html_close(chapter_list);

    let total = if count > 0 { count as usize } else { 0 };
    let total = if total > 2000 { 2000 } else { total };

    let payload = payload_buf();
    let mut c = 0usize;

    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("get_chapters", "internal_error", "payload overflow");
    }

    let mut written = 0usize;
    // Reverse the chapter order (oldest first)
    for i in (0..total).rev() {
        let offset = i * 4;
        if offset + 4 > select_buf.len() { break; }
        let desc_raw = i32::from_le_bytes([
            select_buf[offset], select_buf[offset + 1],
            select_buf[offset + 2], select_buf[offset + 3],
        ]);
        if desc_raw < 0 { continue; }

        let a_desc: host::HtmlDescriptor = unsafe { core::mem::transmute(desc_raw) };
        let mut name_buf = [0u8; 256];
        let mut href_buf = [0u8; 512];

        // Get chapter title from span.itemcata-title if present, otherwise from text
        let name = if let Ok(span_desc) = html_select(a_desc, b"span.itemcata-title") {
            let span_owned = OwnedDescriptor(span_desc);
            text_into(span_owned.0, &mut name_buf).map(|s| trim_ascii(s))
        } else {
            text_into(a_desc, &mut name_buf).map(|s| trim_ascii(s))
        };
        let href = attr_into(a_desc, b"href", &mut href_buf);

        let _ = html_close(a_desc);

        if let (Some(name), Some(href_val)) = (name, href) {
            let chapter_path = manga_path_from_href(href_val).unwrap_or(href_val);

            if written > 0 {
                if !write_bytes(payload, &mut c, b",") {
                    break;
                }
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

    let document = match html_parse(html) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error("get_pages", "parse_error", "html_parse failed"),
    };

    // Parse li.main-item > img
    // Images may use src or data-original for lazy loading
    let select_buf = unsafe { &mut *core::ptr::addr_of_mut!(SELECT_ALL_BUF) };
    let count = html_select_all(document.0.raw(), b"li.main-item img", select_buf);

    let payload = payload_buf();
    let mut c = 0usize;

    let ok = write_bytes(payload, &mut c, br#"{"chapterId":"chapter:"#)
        && append_json_escaped(payload, &mut c, rest)
        && write_bytes(payload, &mut c, br#"","pages":["#);

    if !ok {
        return write_error("get_pages", "internal_error", "payload overflow");
    }

    let total = if count > 0 { count as usize } else { 0 };
    let total = if total > 2000 { 2000 } else { total };
    let mut written = 0usize;

    for i in 0..total {
        let offset = i * 4;
        if offset + 4 > select_buf.len() { break; }
        let desc_raw = i32::from_le_bytes([
            select_buf[offset], select_buf[offset + 1],
            select_buf[offset + 2], select_buf[offset + 3],
        ]);
        if desc_raw < 0 { continue; }

        let img_desc: host::HtmlDescriptor = unsafe { core::mem::transmute(desc_raw) };
        let mut src_buf = [0u8; 1024];
        
        // Try data-original first (lazy loaded), then fall back to src
        let src = if let Some(s) = attr_into(img_desc, b"data-original", &mut src_buf) {
            Some(s)
        } else {
            attr_into(img_desc, b"src", &mut src_buf)
        };
        if let Some(img_url) = src {
            // Filter out non-manhua images (e.g. script/chart images)
            if !contains_bytes(img_url, b"manhua.iqiyipic.com")
                && !contains_bytes(img_url, b"iqiyipic.com")
            {
                continue;
            }

            if written > 0 {
                if !write_bytes(payload, &mut c, b",") {
                    break;
                }
            }
            let page_ok = write_bytes(payload, &mut c, br#"{"id":"page:"#)
                && write_usize(payload, &mut c, written)
                && write_bytes(payload, &mut c, br#"","index":"#)
                && write_usize(payload, &mut c, written)
                && write_bytes(payload, &mut c, br#","image":{"kind":"url","url":""#)
                && append_json_escaped(payload, &mut c, img_url)
                && write_bytes(payload, &mut c, br#""}}"#);

            if !page_ok {
                break;
            }
            written += 1;
        }
        let _ = html_close(img_desc);
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
    log_info(b"iqiyi source init");
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
    log_info(b"iqiyi search");
    run_search(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga", "invalid_request", "empty request"),
    };
    log_info(b"iqiyi get_manga");
    run_get_manga(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_chapters(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_chapters", "invalid_request", "empty request"),
    };
    log_info(b"iqiyi get_chapters");
    run_get_chapters(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_pages(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_pages", "invalid_request", "empty request"),
    };
    log_info(b"iqiyi get_pages");
    run_get_pages(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga_list(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga_list", "invalid_request", "empty request"),
    };
    log_info(b"iqiyi get_manga_list");
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
