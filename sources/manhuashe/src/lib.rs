#![no_std]

extern crate koma_source_sdk;

use koma_source_sdk::host::{self, html_attr, html_close, html_parse, html_select, html_text, http_request, log_info};
use koma_source_sdk::json_utils::{
    append_json_escaped, contains_bytes, extract_json_number, extract_json_string, find_subslice,
    write_bytes, write_url_encoded, write_usize,
};
use koma_source_sdk::result::ResultBuffer;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};

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

const BASE_URL: &[u8] = b"https://www.311s.com";
koma_source_sdk::koma_source_buffers! {
    payload: 1024 * 1024,
    http_out: 2 * 1024 * 1024,
    body: 2 * 1024 * 1024,
    http_req: 2048,
    scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.manhuashe.koma",
    name: "漫画社 (Manhuashe)",
    version: "0.1.0",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "Manhuashe (311s.com) HTML scraping source.",
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

#[derive(Copy, Clone)]
enum FetchError {
    Network,
    NotFound,
    RateLimit,
    ClientError,
    ServerError,
}

fn fetch_body(url: &[u8]) -> Result<usize, FetchError> {
    let req_len = build_get_request(http_req_buf(), url).ok_or(FetchError::Network)?;
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
                    log_info(b"manhuashe: http transport error, retrying");
                }
            }
        }
    }
    if transport_failed {
        return Err(FetchError::Network);
    }
    decode_json_body(&http_out()[..resp_len])
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

/// Strip the domain prefix from an href to get a relative path as the manga ID.
/// e.g. "https://www.311s.com/comic/xxx" → "/comic/xxx"
/// e.g. "/comic/xxx" → "/comic/xxx"
fn manga_path_from_href(href: &[u8]) -> Option<&[u8]> {
    if href.is_empty() {
        return None;
    }
    // If it starts with '/', it's already relative
    if href[0] == b'/' {
        return Some(href);
    }
    // Strip the domain prefix: find the third '/' (after https://)
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

/// Parse the comic list page (used by both popular and search)
fn parse_comic_list(html: &[u8]) -> u32 {
    let document = match html_parse(html) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error("parse", "parse_error", "html_parse failed"),
    };

    let select_buf = unsafe { &mut *core::ptr::addr_of_mut!(SELECT_ALL_BUF) };
    let count = html_select_all(document.0.raw(), b"div.comic-list > div.comic-item", select_buf);

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
            select_buf[offset],
            select_buf[offset + 1],
            select_buf[offset + 2],
            select_buf[offset + 3],
        ]);
        if desc_raw < 0 {
            let _ = html_close(unsafe { core::mem::transmute::<i32, host::HtmlDescriptor>(desc_raw) });
            continue;
        }

        let item_desc: host::HtmlDescriptor = unsafe { core::mem::transmute(desc_raw) };

        // Get title from h3 a
        let title_result = html_select(item_desc, b"h3 a");
        let mut title_buf_a = [0u8; 512];
        let mut href_buf_a = [0u8; 512];
        let mut title_text = None;
        let mut manga_href = None;

        if let Ok(a_desc) = title_result {
            let a_owned = OwnedDescriptor(a_desc);
            title_text = text_into(a_owned.0, &mut title_buf_a).map(|s| trim_ascii(s));
            manga_href = attr_into(a_owned.0, b"href", &mut href_buf_a);
        }

        // Get cover from img
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
                && write_bytes(payload, &mut c, br#""},"authors":[],"status":"unknown","contentRating":"safe","description":"","sourceTags":["manhuashe"]}"#);
            if !ok {
                let _ = html_close(item_desc);
                break;
            }
            written += 1;
        }

        let _ = html_close(item_desc);
    }

    // Check pagination: div.pagination > a.next href != a.on href
    let has_more = contains_bytes(html, b"class=\"next\"");

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
        && write_bytes(url_buf, &mut url_cursor, b"/search/")
        && write_url_encoded(url_buf, &mut url_cursor, query)
        && write_bytes(url_buf, &mut url_cursor, b"/")
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

    parse_comic_list(html)
}

fn run_get_manga_list(req: &[u8]) -> u32 {
    let page = extract_json_number(req, b"page")
        .and_then(parse_usize)
        .unwrap_or(1);

    let url_buf = scratch_a();
    let mut url_cursor = 0usize;
    if !(write_bytes(url_buf, &mut url_cursor, BASE_URL)
        && write_bytes(url_buf, &mut url_cursor, b"/category/order/hits/page/")
        && write_usize(url_buf, &mut url_cursor, page))
    {
        return write_error("get_manga_list", "internal_error", "url overflow");
    }
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_cursor) };

    let html_len = match fetch_body(url) {
        Ok(n) => n,
        Err(e) => { let (c, m) = fetch_error_code(e); return write_error("get_manga_list", c, m); }
    };
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };

    parse_comic_list(html)
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

    // title: div.comic-meta-info > h1
    let mut title_buf = [0u8; 512];
    let title = if let Ok(desc) = html_select(document.0, b"div.comic-meta-info h1") {
        let owned = OwnedDescriptor(desc);
        text_into(owned.0, &mut title_buf).map(|s| trim_ascii(s))
    } else {
        None
    };

    // cover: div.comic-cover-large > img src
    let mut cover_buf = [0u8; 1024];
    let cover = if let Ok(desc) = html_select(document.0, b"div.comic-cover-large img") {
        let owned = OwnedDescriptor(desc);
        attr_into(owned.0, b"src", &mut cover_buf)
    } else {
        None
    };

    // author: from comic-stats - find the stat-item containing "作者："
    let mut author_buf = [0u8; 256];
    let mut author_text: &[u8] = b"";
    if let Ok(desc) = html_select(document.0, b"div.comic-stats") {
        let owned = OwnedDescriptor(desc);
        let mut stats_buf = [0u8; 4096];
        if let Some(stats_html) = text_into(owned.0, &mut stats_buf) {
            // Find "作者：" in the text and extract the value after it
            if let Some(pos) = find_subslice(stats_html, "作者：".as_bytes()) {
                let start = pos + "作者：".as_bytes().len();
                let mut end = start;
                while end < stats_html.len() && stats_html[end] != b'\n' && stats_html[end] != b'\r' {
                    end += 1;
                }
                if end > start {
                    let val = trim_ascii(&stats_html[start..end]);
                    let copy_len = val.len().min(author_buf.len());
                    author_buf[..copy_len].copy_from_slice(&val[..copy_len]);
                    author_text = &author_buf[..copy_len];
                }
            }
        }
    }

    // description: div.comic-description > p text
    let mut desc_buf = [0u8; 4096];
    let desc = if let Ok(desc_sel) = html_select(document.0, b"div.comic-description p") {
        let owned = OwnedDescriptor(desc_sel);
        text_into(owned.0, &mut desc_buf).map(|s| trim_ascii(s))
    } else {
        None
    };

    // genre tags: div.comic-meta-info > div.comic-tags > span text
    let genre_buf = scratch_a();
    let mut genre_cursor = 0usize;
    let select_buf = unsafe { &mut *core::ptr::addr_of_mut!(SELECT_ALL_BUF) };
    if let Ok(tags_container) = html_select(document.0, b"div.comic-meta-info div.comic-tags") {
        let tag_count = html_select_all(tags_container.raw(), b"span", select_buf);
        let mut tag_idx = 0usize;
        for i in 0..tag_count as usize {
            let off = i * 4;
            if off + 4 > select_buf.len() { break; }
            let tag_desc_raw = i32::from_le_bytes([
                select_buf[off], select_buf[off + 1], select_buf[off + 2], select_buf[off + 3],
            ]);
            if tag_desc_raw < 0 { continue; }
            let tag_desc: host::HtmlDescriptor = unsafe { core::mem::transmute(tag_desc_raw) };
            let mut tag_text_buf = [0u8; 128];
            if let Some(t) = text_into(tag_desc, &mut tag_text_buf).map(|s| trim_ascii(s)) {
                if tag_idx > 0 {
                    write_bytes(genre_buf, &mut genre_cursor, b",");
                }
                append_json_escaped(genre_buf, &mut genre_cursor, t);
                tag_idx += 1;
            }
            let _ = html_close(tag_desc);
        }
        let _ = html_close(tags_container);
    }
    let genre_json = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), genre_cursor) };

    // status: check the last comic-tags span for 连载/完结
    let status = if contains_bytes(html, "完结".as_bytes()) {
        b"completed" as &[u8]
    } else if contains_bytes(html, "连载".as_bytes()) {
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
        && if !author_text.is_empty() { append_json_escaped(payload, &mut c, author_text) } else { write_bytes(payload, &mut c, b"") }
        && write_bytes(payload, &mut c, br#"],"artists":[],"status":""#)
        && write_bytes(payload, &mut c, status)
        && write_bytes(payload, &mut c, br#","contentRating":"safe","language":"zh","tags":["#)
        && write_bytes(payload, &mut c, genre_json)
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

    // Build URL
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

    // Parse #chapter-list > div.chapter-item > a
    let select_buf = unsafe { &mut *core::ptr::addr_of_mut!(SELECT_ALL_BUF) };
    let chapter_list = match html_select(document.0, b"#chapter-list") {
        Ok(d) => d,
        Err(_) => return write_error("get_chapters", "parse_error", "chapter-list not found"),
    };

    let count = html_select_all(chapter_list.raw(), b"div.chapter-item a", select_buf);
    let _ = html_close(chapter_list);

    // We need to reverse the chapter order
    let total = if count > 0 { count as usize } else { 0 };
    let total = if total > 2000 { 2000 } else { total };

    let payload = payload_buf();
    let mut c = 0usize;

    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("get_chapters", "internal_error", "payload overflow");
    }

    let mut written = 0usize;
    // Iterate in reverse
    for i in (0..total).rev() {
        let offset = i * 4;
        if offset + 4 > select_buf.len() { break; }
        let desc_raw = i32::from_le_bytes([
            select_buf[offset],
            select_buf[offset + 1],
            select_buf[offset + 2],
            select_buf[offset + 3],
        ]);
        if desc_raw < 0 { continue; }

        let a_desc: host::HtmlDescriptor = unsafe { core::mem::transmute(desc_raw) };
        let mut name_buf = [0u8; 256];
        let mut href_buf = [0u8; 512];

        let name = text_into(a_desc, &mut name_buf).map(|s| trim_ascii(s));
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
    // We need to find the second colon to split manga_path and chapter_path
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

    // Parse div.comic-content > img
    let select_buf = unsafe { &mut *core::ptr::addr_of_mut!(SELECT_ALL_BUF) };
    let count = html_select_all(document.0.raw(), b"div.comic-content img", select_buf);

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

    for i in 0..total {
        let offset = i * 4;
        if offset + 4 > select_buf.len() { break; }
        let desc_raw = i32::from_le_bytes([
            select_buf[offset],
            select_buf[offset + 1],
            select_buf[offset + 2],
            select_buf[offset + 3],
        ]);
        if desc_raw < 0 { continue; }

        let img_desc: host::HtmlDescriptor = unsafe { core::mem::transmute(desc_raw) };
        let mut src_buf = [0u8; 1024];
        let src = attr_into(img_desc, b"src", &mut src_buf);
        let _ = html_close(img_desc);

        if let Some(img_url) = src {
            if i > 0 {
                if !write_bytes(payload, &mut c, b",") {
                    break;
                }
            }
            let page_ok = write_bytes(payload, &mut c, br#"{"id":"page:"#)
                && write_usize(payload, &mut c, i)
                && write_bytes(payload, &mut c, br#"","index":"#)
                && write_usize(payload, &mut c, i)
                && write_bytes(payload, &mut c, br#","image":{"kind":"url","url":""#)
                && append_json_escaped(payload, &mut c, img_url)
                && write_bytes(payload, &mut c, b"}}");

            if !page_ok {
                break;
            }
        }
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
    log_info(b"manhuashe source init");
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
    log_info(b"manhuashe search");
    run_search(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga", "invalid_request", "empty request"),
    };
    log_info(b"manhuashe get_manga");
    run_get_manga(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_chapters(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_chapters", "invalid_request", "empty request"),
    };
    log_info(b"manhuashe get_chapters");
    run_get_chapters(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_pages(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_pages", "invalid_request", "empty request"),
    };
    log_info(b"manhuashe get_pages");
    run_get_pages(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga_list(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga_list", "invalid_request", "empty request"),
    };
    log_info(b"manhuashe get_manga_list");
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
