#![no_std]

extern crate koma_source_sdk;

use koma_source_sdk::host::{
    self, html_attr, html_close, html_parse, html_select, html_text, http_request, log_info,
};
use koma_source_sdk::json_utils::{
    append_json_escaped, append_json_unescaped_then_escaped, extract_json_number,
    extract_json_string, find_subslice, write_bytes, write_url_encoded, write_usize,
};
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

const BASE_URL: &[u8] = b"https://m.wmh1234.com";
const REFERER: &[u8] = b"https://m.wmh1234.com/";
koma_source_sdk::koma_source_buffers! {
    payload: 1024 * 1024,
    http_out: 2 * 1024 * 1024,
    body: 2 * 1024 * 1024,
    http_req: 2048,
    scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.mh1234.koma",
    name: "漫画1234",
    version: "0.1.0",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "漫画1234 HTML scraping source.",
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
    image_request: true,
    credentials: false,
};


fn strip_prefix<'a>(bytes: &'a [u8], prefix: &[u8]) -> &'a [u8] {
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
    let req_len = build_get_request(http_req_buf(), url, Some(REFERER), &[])
        .ok_or(FetchError::Network)?;
    let resp_len =
        http_request(&http_req_buf()[..req_len], http_out()).map_err(|_| FetchError::Network)?;
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
    Some(trim_ascii(&out[..len]))
}

fn select_text<'a>(
    root: host::HtmlDescriptor,
    selector: &[u8],
    buf: &'a mut [u8],
) -> Option<&'a [u8]> {
    let desc = html_select(root, selector).ok()?;
    let owned = OwnedDescriptor(desc);
    text_into(owned.0, buf)
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

fn descriptor_at(buf: &[u8], index: usize) -> Option<host::HtmlDescriptor> {
    let offset = index.checked_mul(4)?;
    if offset + 4 > buf.len() {
        return None;
    }
    let raw = i32::from_le_bytes([
        buf[offset],
        buf[offset + 1],
        buf[offset + 2],
        buf[offset + 3],
    ]);
    if raw < 0 {
        return None;
    }
    Some(unsafe { core::mem::transmute(raw) })
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
        write_bytes(dst, c, b"https:") && append_json_escaped(dst, c, url)
    } else if url.starts_with(b"/") {
        write_bytes(dst, c, BASE_URL) && append_json_escaped(dst, c, url)
    } else {
        append_json_escaped(dst, c, url)
    }
}

fn extract_attr_after<'a>(html: &'a [u8], marker: &[u8], attr: &[u8]) -> Option<&'a [u8]> {
    let mut pos = find_subslice(html, marker)? + marker.len();
    let rest = &html[pos..];
    pos += find_subslice(rest, b"<img")?;
    let rest = &html[pos..];
    let attr_pos = find_subslice(rest, attr)? + attr.len();
    let start = pos + attr_pos;
    let mut end = start;
    while end < html.len() && html[end] != b'"' {
        end += 1;
    }
    if end > start { Some(&html[start..end]) } else { None }
}

fn extract_meta_text(html: &[u8], index: usize) -> Option<&[u8]> {
    let mut pos = find_subslice(html, b"comic-hero__meta")?;
    let mut seen = 0usize;
    loop {
        let rest = &html[pos..];
        let span = find_subslice(rest, b"class=\"meta-item\"")?;
        pos += span;
        if seen == index {
            let rest = &html[pos..];
            let svg_end = find_subslice(rest, b"</svg>")? + b"</svg>".len();
            let start = pos + svg_end;
            let end = start + find_subslice(&html[start..], b"</span>")?;
            return Some(trim_ascii(&html[start..end]));
        }
        seen += 1;
        pos += b"class=\"meta-item\"".len();
    }
}

fn build_url_with_path(path: &[u8]) -> Option<usize> {
    let url_buf = scratch_a();
    let mut c = 0usize;
    if path.starts_with(b"http://") || path.starts_with(b"https://") {
        append_json_escaped(url_buf, &mut c, path).then_some(c)
    } else {
        (write_bytes(url_buf, &mut c, BASE_URL) && write_bytes(url_buf, &mut c, path)).then_some(c)
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
        && write_bytes(payload, c, br#""},"authors":[],"status":"unknown","contentRating":"safe","description":"","sourceTags":["mh1234"]}"#)
}

fn parse_list(html: &[u8], operation: &str) -> u32 {
    let document = match html_parse(html) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error(operation, "parse_error", "html_parse failed"),
    };

    let select_buf = unsafe { &mut *core::ptr::addr_of_mut!(SELECT_ALL_BUF) };
    let count = html_select_all(document.0.raw(), b".comic-card", select_buf);
    let total = if count > 0 { count as usize } else { 0 };
    let total = if total > 500 { 500 } else { total };

    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error(operation, "internal_error", "payload overflow");
    }

    let mut written = 0usize;
    for i in 0..total {
        let Some(item_desc) = descriptor_at(select_buf, i) else {
            continue;
        };

        let mut title_buf = [0u8; 512];
        let mut href_buf = [0u8; 1024];
        let mut cover_buf = [0u8; 1024];

        let title = select_text(item_desc, b"a.comic-card__link .comic-card__title", &mut title_buf);
        let href = select_attr(item_desc, b"a.comic-card__link", b"href", &mut href_buf);
        let mut cover = select_attr(item_desc, b"img.comic-card__image", b"data-src", &mut cover_buf);
        if cover.is_none() {
            cover = select_attr(item_desc, b"img.comic-card__image", b"src", &mut cover_buf);
        }

        if let (Some(t), Some(h)) = (title, href) {
            if let Some(path) = path_from_href(h) {
                if !write_manga_item(payload, &mut c, path, t, cover.unwrap_or(b""), written) {
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
        || !write_bytes(
            payload,
            &mut c,
            if written > 0 { b"true" as &[u8] } else { b"false" as &[u8] },
        )
        || !write_bytes(payload, &mut c, b"}}")
    {
        return write_error(operation, "internal_error", "payload overflow");
    }

    write_success_payload(operation, c)
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
        && write_bytes(url_buf, &mut c, b"/search/")
        && write_url_encoded(url_buf, &mut c, query)
        && if page > 1 {
            write_bytes(url_buf, &mut c, b"/page/") && write_usize(url_buf, &mut c, page)
        } else {
            true
        })
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
    parse_list(html, "search")
}

fn run_get_manga_list(req: &[u8]) -> u32 {
    let page = extract_json_number(req, b"page").and_then(parse_usize).unwrap_or(1);

    let url_buf = scratch_a();
    let mut c = 0usize;
    if !(write_bytes(url_buf, &mut c, BASE_URL)
        && write_bytes(url_buf, &mut c, b"/category/order/hits")
        && if page > 1 {
            write_bytes(url_buf, &mut c, b"/page/") && write_usize(url_buf, &mut c, page)
        } else {
            true
        })
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
    parse_list(html, "get_manga_list")
}

fn fetch_manga_page(operation: &str, id: &[u8]) -> Result<usize, u32> {
    let Some(url_len) = build_url_with_path(id) else {
        return Err(write_error(operation, "internal_error", "url overflow"));
    };
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_len) };
    match fetch_body(url) {
        Ok(n) => Ok(n),
        Err(e) => {
            let (code, message) = fetch_error_code(e);
            Err(write_error(operation, code, message))
        }
    }
}

fn run_get_manga(req: &[u8]) -> u32 {
    let manga_id = match extract_json_string(req, b"mangaId") {
        Some(v) => v,
        None => return write_error("get_manga", "invalid_request", "missing mangaId"),
    };

    let html_len = match fetch_manga_page("get_manga", manga_id) {
        Ok(n) => n,
        Err(r) => return r,
    };
    let url_len = match build_url_with_path(manga_id) {
        Some(n) => n,
        None => return write_error("get_manga", "internal_error", "url overflow"),
    };
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_len) };
    let document = match html_parse(html) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error("get_manga", "parse_error", "html_parse failed"),
    };

    let mut title_buf = [0u8; 512];
    let mut cover_buf = [0u8; 1024];
    let mut author_buf = [0u8; 512];
    let mut genre_buf = [0u8; 512];
    let mut status_buf = [0u8; 512];
    let mut desc_buf = [0u8; 4096];

    let mut title = select_text(document.0, b".comic-hero__title", &mut title_buf);
    if title.is_none() {
        title = select_text(document.0, b"h1", &mut title_buf);
    }
    let mut cover = None;
    if let Ok(cover_desc) = html_select(document.0, b".comic-hero__cover") {
        let owned = OwnedDescriptor(cover_desc);
        cover = select_attr(owned.0, b"img", b"data-src", &mut cover_buf);
        if cover.is_none() {
            cover = select_attr(owned.0, b"img", b"src", &mut cover_buf);
        }
    }
    if cover.map(|v| v.is_empty()).unwrap_or(true) {
        cover = extract_attr_after(html, b"class=\"comic-hero__cover\"", b"data-src=\"")
            .or_else(|| extract_attr_after(html, b"class=\"comic-hero__cover\"", b"src=\""));
    }

    let select_buf = unsafe { &mut *core::ptr::addr_of_mut!(SELECT_ALL_BUF) };
    let meta_parent = html_select(document.0, b".comic-hero__meta").ok();
    let meta_count = if let Some(parent) = meta_parent {
        html_select_all(parent.raw(), b".meta-item", select_buf)
    } else {
        0
    };
    let meta_total = if meta_count > 0 { meta_count as usize } else { 0 };
    let mut author_len = 0usize;
    let mut genre_len = 0usize;
    if meta_total > 0 {
        if let Some(desc) = descriptor_at(select_buf, 0) {
            if let Some(t) = text_into(desc, &mut author_buf) {
                author_len = t.len();
            }
            let _ = html_close(desc);
        }
    }
    if meta_total > 1 {
        if let Some(desc) = descriptor_at(select_buf, 1) {
            if let Some(t) = text_into(desc, &mut genre_buf) {
                genre_len = t.len();
            }
            let _ = html_close(desc);
        }
    }
    if let Some(parent) = meta_parent {
        let _ = html_close(parent);
    }
    let mut author = strip_prefix(trim_ascii(&author_buf[..author_len]), "作者:".as_bytes());
    let mut genre = strip_prefix(trim_ascii(&genre_buf[..genre_len]), "类型:".as_bytes());
    if author.is_empty() {
        author = extract_meta_text(html, 0).unwrap_or(b"");
    }
    if genre.is_empty() {
        genre = extract_meta_text(html, 1).unwrap_or(b"");
    }

    let stat_count = html_select_all(document.0.raw(), b".stat-item", select_buf);
    let stat_total = if stat_count > 0 { stat_count as usize } else { 0 };
    let stat_total = if stat_total > 16 { 16 } else { stat_total };
    let mut status_len = 0usize;
    for i in 0..stat_total {
        let Some(desc) = descriptor_at(select_buf, i) else {
            continue;
        };
        let mut item_buf = [0u8; 512];
        let item_text = text_into(desc, &mut item_buf).unwrap_or(b"");
        if find_subslice(item_text, "状态".as_bytes()).is_some() {
            if let Some(t) = select_text(desc, b".stat-value", &mut status_buf) {
                status_len = t.len();
            } else {
                let n = item_text.len();
                status_buf[..n].copy_from_slice(item_text);
                status_len = n;
            }
        }
        let _ = html_close(desc);
    }
    let status_raw = trim_ascii(&status_buf[..status_len]);
    let status = if find_subslice(status_raw, "完结".as_bytes()).is_some() {
        b"completed" as &[u8]
    } else if find_subslice(status_raw, "连载".as_bytes()).is_some() {
        b"ongoing" as &[u8]
    } else {
        b"unknown" as &[u8]
    };

    let desc_raw = select_text(document.0, b"#comicDesc", &mut desc_buf).unwrap_or(b"");
    let desc = strip_prefix(desc_raw, "介绍:".as_bytes());

    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"manga":{"id":""#)
        && append_json_escaped(payload, &mut c, manga_id)
        && write_bytes(payload, &mut c, br#"","title":""#)
        && append_json_escaped(payload, &mut c, title.unwrap_or(manga_id))
        && write_bytes(payload, &mut c, br#"","alternateTitles":[],"description":""#)
        && append_json_escaped(payload, &mut c, desc)
        && write_bytes(payload, &mut c, br#"","cover":{"kind":"url","url":""#)
        && write_json_url(payload, &mut c, cover.unwrap_or(b""))
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
        && write_bytes(payload, &mut c, br#"","contentRating":"safe","language":"zh","tags":["#)
        && if !genre.is_empty() {
            write_bytes(payload, &mut c, b"\"")
                && append_json_escaped(payload, &mut c, genre)
                && write_bytes(payload, &mut c, b"\"")
        } else {
            true
        }
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

    let html_len = match fetch_manga_page("get_chapters", manga_id) {
        Ok(n) => n,
        Err(r) => return r,
    };
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };
    let document = match html_parse(html) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error("get_chapters", "parse_error", "html_parse failed"),
    };

    let select_buf = unsafe { &mut *core::ptr::addr_of_mut!(SELECT_ALL_BUF) };
    let count = html_select_all(document.0.raw(), b".chapter-list a.chapter-item", select_buf);
    let total = if count > 0 { count as usize } else { 0 };
    let total = if total > 4000 { 4000 } else { total };

    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("get_chapters", "internal_error", "payload overflow");
    }

    let mut written = 0usize;
    for i in (0..total).rev() {
        let Some(a_desc) = descriptor_at(select_buf, i) else {
            continue;
        };
        let mut name_buf = [0u8; 512];
        let mut href_buf = [0u8; 1024];
        let mut fallback_name_buf = [0u8; 512];
        let name = match select_text(a_desc, b".chapter-title", &mut name_buf) {
            Some(v) => Some(v),
            None => text_into(a_desc, &mut fallback_name_buf),
        };
        let href = attr_into(a_desc, b"href", &mut href_buf);
        let _ = html_close(a_desc);

        if let (Some(ch_name), Some(ch_href)) = (name, href) {
            if let Some(path) = path_from_href(ch_href) {
                if !path.starts_with(b"/comic/") {
                    continue;
                }
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

fn run_get_pages(req: &[u8]) -> u32 {
    let chapter_id = match extract_json_string(req, b"chapterId") {
        Some(v) => v,
        None => return write_error("get_pages", "invalid_request", "missing chapterId"),
    };

    let Some(url_len) = build_url_with_path(chapter_id) else {
        return write_error("get_pages", "internal_error", "url overflow");
    };
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_len) };
    let html_len = match fetch_body(url) {
        Ok(n) => n,
        Err(e) => {
            let (code, message) = fetch_error_code(e);
            return write_error("get_pages", code, message);
        }
    };
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };
    let document = match html_parse(html) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error("get_pages", "parse_error", "html_parse failed"),
    };

    let select_buf = unsafe { &mut *core::ptr::addr_of_mut!(SELECT_ALL_BUF) };
    let count = html_select_all(document.0.raw(), b"img.reader-image", select_buf);
    let total = if count > 0 { count as usize } else { 0 };
    let total = if total > 2000 { 2000 } else { total };

    let payload = payload_buf();
    let mut c = 0usize;
    if !(write_bytes(payload, &mut c, br#"{"chapterId":""#)
        && append_json_escaped(payload, &mut c, chapter_id)
        && write_bytes(payload, &mut c, br#"","pages":["#))
    {
        return write_error("get_pages", "internal_error", "payload overflow");
    }

    let mut written = 0usize;
    for i in 0..total {
        let Some(img_desc) = descriptor_at(select_buf, i) else {
            continue;
        };
        let mut image_buf = [0u8; 2048];
        let mut image = attr_into(img_desc, b"data-src", &mut image_buf);
        if image.is_none() {
            image = attr_into(img_desc, b"src", &mut image_buf);
        }
        let _ = html_close(img_desc);

        if let Some(img) = image {
            if img.is_empty() {
                continue;
            }
            if written > 0 && !write_bytes(payload, &mut c, b",") {
                break;
            }
            let ok = write_bytes(payload, &mut c, br#"{"id":"page:"#)
                && write_usize(payload, &mut c, written)
                && write_bytes(payload, &mut c, br#"","index":"#)
                && write_usize(payload, &mut c, written)
                && write_bytes(payload, &mut c, br#","image":{"kind":"url","url":""#)
                && write_json_url(payload, &mut c, img)
                && write_bytes(payload, &mut c, br#""}}"#);
            if !ok {
                break;
            }
            written += 1;
        }
    }

    if !write_bytes(payload, &mut c, b"]}") {
        return write_error("get_pages", "internal_error", "payload overflow");
    }
    write_success_payload("get_pages", c)
}

fn run_get_image_request(req: &[u8]) -> u32 {
    let url = extract_json_string(req, b"url")
        .or_else(|| extract_json_string(req, b"imageUrl"))
        .unwrap_or(b"");
    if url.is_empty() {
        return write_error("get_image_request", "invalid_request", "missing url");
    }

    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"url":""#)
        && append_json_unescaped_then_escaped(payload, &mut c, url)
        && write_bytes(
            payload,
            &mut c,
            br#"","headers":{"Referer":"https://m.wmh1234.com/"}}"#,
        );
    if !ok {
        return write_error("get_image_request", "internal_error", "payload overflow");
    }
    write_success_payload("get_image_request", c)
}

#[no_mangle]
pub extern "C" fn koma_source_init(_manifest_ptr: u32, manifest_len: u32) -> i32 {
    log_info(b"mh1234 source init");
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
    log_info(b"mh1234 search");
    run_search(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga_list(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga_list", "invalid_request", "empty request"),
    };
    log_info(b"mh1234 get_manga_list");
    run_get_manga_list(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga", "invalid_request", "empty request"),
    };
    log_info(b"mh1234 get_manga");
    run_get_manga(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_chapters(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_chapters", "invalid_request", "empty request"),
    };
    log_info(b"mh1234 get_chapters");
    run_get_chapters(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_pages(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_pages", "invalid_request", "empty request"),
    };
    log_info(b"mh1234 get_pages");
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
pub extern "C" fn koma_source_get_image_request(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_image_request", "invalid_request", "empty request"),
    };
    log_info(b"mh1234 get_image_request");
    run_get_image_request(req)
}

#[no_mangle]
pub extern "C" fn koma_source_free(result_ptr: u32) {
    response_buffer().free(result_ptr)
}
