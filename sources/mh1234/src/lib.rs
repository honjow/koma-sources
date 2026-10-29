#![no_std]

extern crate koma_source_sdk;

use koma_source_sdk::host::{
    self, html_attr, html_close, html_parse, html_select, html_text, http_request, log_info,
};
use koma_source_sdk::json_utils::{
    append_json_escaped, extract_json_string, find_subslice, json_object_member, write_bytes,
    write_url_encoded, write_usize,
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
    name: "漫画1234网",
    version: "0.1.7",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "漫画1234公开漫画目录与阅读源。",
    content_rating: "safe",
};

const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: true,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: true,
    manga_list: true,
    home: false,
    filters: true,
    settings: false,
    image_request: true,
    credentials: false,
};

fn number(bytes: &[u8]) -> Option<usize> {
    if bytes.is_empty() {
        return None;
    }
    bytes.iter().try_fold(0usize, |n, b| {
        if !b.is_ascii_digit() {
            return None;
        }
        n.checked_mul(10)?.checked_add((b - b'0') as usize)
    })
}

fn args(req: &[u8]) -> &[u8] {
    json_object_member(req, b"args").unwrap_or(req)
}

fn hex(bytes: &[u8]) -> Option<u32> {
    bytes.iter().try_fold(0u32, |n, b| {
        let d = match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => b - b'a' + 10,
            b'A'..=b'F' => b - b'A' + 10,
            _ => return None,
        };
        Some(n * 16 + d as u32)
    })
}

// Decode request strings once, before URL construction or identifier validation.
fn string_into<'a>(raw: &[u8], dst: &'a mut [u8]) -> Option<&'a [u8]> {
    let mut i = 0;
    let mut c = 0;
    while i < raw.len() {
        if raw[i] != b'\\' {
            if raw[i] < 0x20 || !write_bytes(dst, &mut c, &raw[i..i + 1]) {
                return None;
            }
            i += 1;
            continue;
        }
        i += 1;
        let e = *raw.get(i)?;
        i += 1;
        if e == b'u' {
            let mut code = hex(raw.get(i..i + 4)?)?;
            i += 4;
            if (0xd800..=0xdbff).contains(&code) {
                if raw.get(i..i + 2)? != b"\\u" {
                    return None;
                }
                let low = hex(raw.get(i + 2..i + 6)?)?;
                if !(0xdc00..=0xdfff).contains(&low) {
                    return None;
                }
                code = 0x10000 + ((code - 0xd800) << 10) + low - 0xdc00;
                i += 6;
            }
            let ch = core::char::from_u32(code)?;
            let mut utf8 = [0; 4];
            if !write_bytes(dst, &mut c, ch.encode_utf8(&mut utf8).as_bytes()) {
                return None;
            }
        } else {
            let b = match e {
                b'/' | b'\\' | b'"' => e,
                b'n' => b'\n',
                b'r' => b'\r',
                b't' => b'\t',
                b'b' => 8,
                b'f' => 12,
                _ => return None,
            };
            if !write_bytes(dst, &mut c, &[b]) {
                return None;
            }
        }
    }
    core::str::from_utf8(&dst[..c]).ok()?;
    Some(&dst[..c])
}

fn page_number(req: &[u8]) -> Option<usize> {
    let Some(p) = json_object_member(args(req), b"page") else {
        return Some(1);
    };
    let p = if p.first() == Some(&b'{') {
        let cursor = json_object_member(p, b"cursor").unwrap_or(b"null");
        if cursor == b"null" || cursor == b"\"\"" {
            return Some(1);
        }
        cursor.strip_prefix(b"\"")?.strip_suffix(b"\"")?
    } else {
        p
    };
    let n = number(p)?;
    if n == 0 {
        None
    } else {
        Some(n)
    }
}

fn chapter_number(title: &[u8]) -> Option<usize> {
    let title = core::str::from_utf8(title).ok()?.trim();
    let title = title.strip_prefix('第').unwrap_or(title);
    let mut n = 0usize;
    let mut pending = 0usize;
    let mut any = false;
    for ch in title.chars() {
        if ch.is_ascii_digit() {
            pending = pending
                .checked_mul(10)?
                .checked_add(ch as usize - '0' as usize)?;
            any = true;
        } else {
            let d = match ch {
                '零' | '〇' => Some(0),
                '一' => Some(1),
                '二' | '两' | '兩' => Some(2),
                '三' => Some(3),
                '四' => Some(4),
                '五' => Some(5),
                '六' => Some(6),
                '七' => Some(7),
                '八' => Some(8),
                '九' => Some(9),
                _ => None,
            };
            if let Some(d) = d {
                pending = pending.checked_mul(10)?.checked_add(d)?;
                any = true;
            } else if matches!(ch, '十' | '百' | '千') {
                let unit = match ch {
                    '十' => 10,
                    '百' => 100,
                    _ => 1000,
                };
                n = n.checked_add(pending.max(1).checked_mul(unit)?)?;
                pending = 0;
                any = true;
            } else {
                // An explicit chapter suffix is required; do not number specials by their position.
                return if any && matches!(ch, '话' | '話' | '章' | '回') {
                    n.checked_add(pending)
                } else {
                    None
                };
            }
        }
    }
    None
}

fn fetch_body(url: &[u8]) -> Result<usize, FetchError> {
    let req_len =
        build_get_request(http_req_buf(), url, Some(REFERER), &[]).ok_or(FetchError::Network)?;
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

fn attr_into<'a>(desc: host::HtmlDescriptor, name: &[u8], out: &'a mut [u8]) -> Option<&'a [u8]> {
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

struct DescriptorBatch {
    count: usize,
}
impl DescriptorBatch {
    fn select(root: host::HtmlDescriptor, selector: &[u8]) -> Result<Self, ()> {
        let buf = unsafe { &mut *core::ptr::addr_of_mut!(SELECT_ALL_BUF) };
        let count = html_select_all(root.raw(), selector, buf);
        if count < 0 {
            return Err(());
        }
        let batch = Self {
            count: (count as usize).min(buf.len() / 4),
        };
        if count as usize > buf.len() / 4 {
            drop(batch);
            return Err(());
        }
        Ok(batch)
    }
    fn at(&self, i: usize) -> Option<host::HtmlDescriptor> {
        if i >= self.count {
            return None;
        }
        let buf = unsafe { &*core::ptr::addr_of!(SELECT_ALL_BUF) };
        let a = i.checked_mul(4)?;
        let raw = i32::from_le_bytes(buf[a..a + 4].try_into().ok()?);
        if raw <= 0 {
            None
        } else {
            Some(host::HtmlDescriptor::from_raw(raw))
        }
    }
}
impl Drop for DescriptorBatch {
    fn drop(&mut self) {
        for i in 0..self.count {
            if let Some(d) = self.at(i) {
                let _ = html_close(d);
            }
        }
    }
}

fn path_from_href(href: &[u8]) -> Option<&[u8]> {
    let path = href.strip_prefix(BASE_URL).unwrap_or(href);
    if !path.starts_with(b"/")
        || path.starts_with(b"//")
        || path
            .iter()
            .any(|b| b.is_ascii_whitespace() || matches!(b, b'\\' | b'?' | b'#'))
        || find_subslice(path, b"..").is_some()
    {
        return None;
    }
    Some(path)
}

fn manga_path(path: &[u8]) -> bool {
    path.strip_prefix(b"/comic/")
        .and_then(|s| s.strip_suffix(b".html"))
        .and_then(number)
        .is_some()
}

fn chapter_path(path: &[u8]) -> bool {
    if let Some(token) = path.strip_prefix(b"/go/") {
        return !token.is_empty()
            && token
                .iter()
                .all(|b| b.is_ascii_alphanumeric() || *b == b'_' || *b == b'-');
    }
    // Keep previously emitted /comic/<manga>/<chapter>.html IDs callable.
    let Some(rest) = path
        .strip_prefix(b"/comic/")
        .and_then(|v| v.strip_suffix(b".html"))
    else {
        return false;
    };
    let Some(split) = rest.iter().position(|b| *b == b'/') else {
        return false;
    };
    number(&rest[..split]).is_some() && number(&rest[split + 1..]).is_some()
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

fn build_url_with_path(path: &[u8]) -> Option<usize> {
    let path = path_from_href(path)?;
    let mut c = 0;
    (write_bytes(scratch_a(), &mut c, BASE_URL) && write_bytes(scratch_a(), &mut c, path))
        .then_some(c)
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

fn parse_list(html: &[u8], operation: &str, page: usize, route: &[u8]) -> u32 {
    let document = match html_parse(html) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error(operation, "parse_error", "html_parse failed"),
    };
    if html_select(document.0, b".comic-grid")
        .map(OwnedDescriptor)
        .is_err()
    {
        return write_error(operation, "parse_error", "manga grid missing");
    }
    let payload = payload_buf();
    let mut c = 0;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error(operation, "internal_error", "payload overflow");
    }
    let mut written = 0;
    {
        let batch = match DescriptorBatch::select(document.0, b".comic-card") {
            Ok(b) if b.count <= 500 => b,
            _ => return write_error(operation, "parse_error", "manga list exceeds limit"),
        };
        for i in 0..batch.count {
            if host::check_cancel() {
                return write_error(operation, "cancelled", "host cancelled");
            }
            let Some(d) = batch.at(i) else {
                return write_error(operation, "parse_error", "invalid manga descriptor");
            };
            let mut title_buf = [0; 512];
            let mut href_buf = [0; 1024];
            let mut cover_buf = [0; 2048];
            let Some(title) =
                select_text(d, b".comic-card__title", &mut title_buf).filter(|t| !t.is_empty())
            else {
                return write_error(operation, "parse_error", "manga title missing");
            };
            let Some(href) = select_attr(d, b"a.comic-card__link", b"href", &mut href_buf) else {
                return write_error(operation, "parse_error", "manga link missing");
            };
            let Some(path) = path_from_href(href).filter(|p| manga_path(p)) else {
                return write_error(operation, "parse_error", "invalid manga link");
            };
            let Some(cover) = select_attr(d, b"img.comic-card__image", b"data-src", &mut cover_buf)
                .filter(|s| !s.is_empty())
            else {
                return write_error(operation, "parse_error", "manga cover missing");
            };
            if !write_manga_item(payload, &mut c, path, title, cover, written) {
                return write_error(operation, "internal_error", "payload overflow");
            }
            written += 1;
        }
    }
    let mut has_more = false;
    {
        let batch = match DescriptorBatch::select(document.0, b".pagination-wrapper a") {
            Ok(b) if b.count <= 64 => b,
            _ => return write_error(operation, "parse_error", "invalid pagination"),
        };
        for i in 0..batch.count {
            let d = batch.at(i).unwrap();
            let mut label_buf = [0; 64];
            if text_into(d, &mut label_buf) != Some("下一页".as_bytes()) {
                continue;
            }
            let mut href_buf = [0; 2048];
            let Some(href) = attr_into(d, b"href", &mut href_buf) else {
                return write_error(operation, "parse_error", "next-page link missing");
            };
            let Some(path) = path_from_href(href) else {
                return write_error(operation, "parse_error", "invalid next-page link");
            };
            // Only advance to the next page of the same search/listing route.
            let mut expected = [0; 2048];
            let mut n = 0;
            let Some(next) = page.checked_add(1) else {
                return write_error(operation, "invalid_request", "page overflow");
            };
            if !write_bytes(&mut expected, &mut n, route)
                || !write_bytes(&mut expected, &mut n, b"/page/")
                || !write_usize(&mut expected, &mut n, next)
            {
                return write_error(operation, "internal_error", "pagination overflow");
            }
            has_more = path == &expected[..n];
        }
    }
    if !write_bytes(payload, &mut c, br#"],"page":{"nextCursor":"#)
        || !if has_more {
            write_bytes(payload, &mut c, b"\"")
                && write_usize(payload, &mut c, page + 1)
                && write_bytes(payload, &mut c, b"\"")
        } else {
            write_bytes(payload, &mut c, b"null")
        }
        || !write_bytes(payload, &mut c, br#","hasMore":"#)
        || !write_bytes(payload, &mut c, if has_more { b"true" } else { b"false" })
        || !write_bytes(payload, &mut c, b"}}")
    {
        return write_error(operation, "internal_error", "payload overflow");
    }
    write_success_payload(operation, c)
}

fn fetch_list(operation: &str, route: &[u8], page: usize) -> u32 {
    let mut c = 0;
    if !write_bytes(scratch_a(), &mut c, BASE_URL)
        || !write_bytes(scratch_a(), &mut c, route)
        || !(page == 1
            || (write_bytes(scratch_a(), &mut c, b"/page/")
                && write_usize(scratch_a(), &mut c, page)))
    {
        return write_error(operation, "invalid_request", "url overflow");
    }
    let url =
        unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(SCRATCH_A).cast::<u8>(), c) };
    let html_len = match fetch_body(url) {
        Ok(n) => n,
        Err(e) => {
            let (code, message) = fetch_error_code(e);
            return write_error(operation, code, message);
        }
    };
    let html = unsafe {
        core::slice::from_raw_parts(core::ptr::addr_of!(BODY_BUF).cast::<u8>(), html_len)
    };
    parse_list(html, operation, page, route)
}

// Reference applies category filters only to empty-query discovery; textual searches ignore them.
// Preserve Koma's popular/latest defaults until the user explicitly chooses a sort.
fn filtered_route(req: &[u8], default_sort: &[u8], out: &mut [u8]) -> Result<Option<usize>, ()> {
    let filters = match json_object_member(args(req), b"filters") {
        None | Some(b"null") => return Ok(None),
        Some(v) if v.first() == Some(&b'{') => v,
        _ => return Err(()),
    };
    let genre = extract_json_string(filters, b"genre").unwrap_or(b"0");
    let status = extract_json_string(filters, b"status").unwrap_or(b"0");
    let sort = extract_json_string(filters, b"sort").unwrap_or(default_sort);
    if genre.len() > 10 || number(genre).is_none() || !matches!(status, b"0" | b"1" | b"2") || !matches!(sort, b"id" | b"hits" | b"addtime") { return Err(()); }
    if genre == b"0" && status == b"0" && sort == default_sort { return Ok(None); }
    let mut n = 0;
    // The site's own links omit unselected dimensions: tags/0 is a real
    // empty route, not the absence of a genre (observed status-only case).
    if !write_bytes(out, &mut n, b"/category")
        || (genre != b"0" && (!write_bytes(out, &mut n, b"/tags/") || !write_bytes(out, &mut n, genre)))
        || (status != b"0" && (!write_bytes(out, &mut n, b"/finish/") || !write_bytes(out, &mut n, status)))
        || !write_bytes(out, &mut n, b"/order/") || !write_bytes(out, &mut n, sort) { return Err(()); }
    Ok(Some(n))
}
fn run_search(req: &[u8]) -> u32 {
    let Some(raw) = extract_json_string(args(req), b"query") else { return write_error("search", "invalid_request", "missing query"); };
    let mut query_buf = [0; 1024];
    let Some(query) = string_into(raw, &mut query_buf) else { return write_error("search", "invalid_request", "invalid query"); };
    let Some(page) = page_number(req) else { return write_error("search", "invalid_request", "invalid page cursor"); };
    let mut route = [0; 3072];
    if trim_ascii(query).is_empty() {
        return match filtered_route(req, b"hits", &mut route) {
            Ok(Some(n)) => fetch_list("search", &route[..n], page),
            Ok(None) => fetch_list("search", b"/category/order/hits", page),
            Err(()) => write_error("search", "invalid_request", "invalid filters"),
        };
    }
    let mut n = 0;
    if !write_bytes(&mut route, &mut n, b"/search/") || !write_url_encoded(&mut route, &mut n, query) { return write_error("search", "invalid_request", "query too long"); }
    fetch_list("search", &route[..n], page)
}
fn run_get_manga_list(req: &[u8]) -> u32 {
    let Some(page) = page_number(req) else { return write_error("get_manga_list", "invalid_request", "invalid page cursor"); };
    let listing = extract_json_string(args(req), b"listingId").unwrap_or(b"popular");
    let (route, sort) = match listing {
        b"popular" => (b"/category/order/hits".as_slice(), b"hits".as_slice()),
        b"latest" => (b"/category/order/addtime".as_slice(), b"addtime".as_slice()),
        _ => return write_error("get_manga_list", "invalid_request", "unknown listing"),
    };
    let mut filtered = [0; 256];
    match filtered_route(req, sort, &mut filtered) {
        Ok(Some(n)) => fetch_list("get_manga_list", &filtered[..n], page),
        Ok(None) => fetch_list("get_manga_list", route, page),
        Err(()) => write_error("get_manga_list", "invalid_request", "invalid filters"),
    }
}

fn fetch_manga_page(operation: &str, id: &[u8]) -> Result<usize, u32> {
    let Some(url_len) = build_url_with_path(id) else {
        return Err(write_error(operation, "internal_error", "url overflow"));
    };
    let url = unsafe {
        core::slice::from_raw_parts(core::ptr::addr_of!(SCRATCH_A).cast::<u8>(), url_len)
    };
    match fetch_body(url) {
        Ok(n) => Ok(n),
        Err(e) => {
            let (code, message) = fetch_error_code(e);
            Err(write_error(operation, code, message))
        }
    }
}

fn run_get_manga(req: &[u8]) -> u32 {
    let mut id_buf = [0; 1024];
    let Some(manga_id) = extract_json_string(args(req), b"mangaId")
        .and_then(|v| string_into(v, &mut id_buf))
        .and_then(path_from_href)
        .filter(|v| manga_path(v))
    else {
        return write_error("get_manga", "invalid_request", "invalid mangaId");
    };
    let html_len = match fetch_manga_page("get_manga", manga_id) {
        Ok(n) => n,
        Err(r) => return r,
    };
    let html = unsafe {
        core::slice::from_raw_parts(core::ptr::addr_of!(BODY_BUF).cast::<u8>(), html_len)
    };
    let document = match html_parse(html) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error("get_manga", "parse_error", "html_parse failed"),
    };
    let mut title_buf = [0; 512];
    let mut cover_buf = [0; 2048];
    let mut author_buf = [0; 512];
    let mut genre_buf = [0; 512];
    let mut status_buf = [0; 64];
    let mut desc_buf = [0; 8192];
    let Some(title) =
        select_text(document.0, b"#mintWorkTitle", &mut title_buf).filter(|v| !v.is_empty())
    else {
        return write_error("get_manga", "parse_error", "manga title missing");
    };
    let Some(cover) = select_attr(document.0, b"#mintWorkCover", b"src", &mut cover_buf)
        .filter(|v| !v.is_empty())
    else {
        return write_error("get_manga", "parse_error", "manga cover missing");
    };
    let author = select_text(
        document.0,
        b".mint-work-info p:nth-of-type(1)",
        &mut author_buf,
    )
    .unwrap_or(b"");
    let author = trim_ascii(author.strip_suffix(" 著".as_bytes()).unwrap_or(author));
    let genres = select_text(
        document.0,
        b".mint-work-info p:nth-of-type(2)",
        &mut genre_buf,
    )
    .unwrap_or(b"");
    let status_raw =
        select_text(document.0, b".mint-work-info .mint-tag", &mut status_buf).unwrap_or(b"");
    let genres = trim_ascii(genres.strip_suffix(status_raw).unwrap_or(genres));
    let status = match status_raw {
        v if v == "连载".as_bytes() => b"ongoing" as &[u8],
        v if v == "完结".as_bytes() => b"completed" as &[u8],
        _ => b"unknown" as &[u8],
    };
    let description =
        select_text(document.0, b"#mintIntroPanel > div", &mut desc_buf).unwrap_or(b"");
    let payload = payload_buf();
    let mut c = 0;
    let ok = write_bytes(payload, &mut c, br#"{"manga":{"id":""#)
        && append_json_escaped(payload, &mut c, manga_id)
        && write_bytes(payload, &mut c, br#"","title":""#)
        && append_json_escaped(payload, &mut c, title)
        && write_bytes(
            payload,
            &mut c,
            br#"","alternateTitles":[],"description":""#,
        )
        && append_json_escaped(payload, &mut c, description)
        && write_bytes(payload, &mut c, br#"","cover":{"kind":"url","url":""#)
        && write_json_url(payload, &mut c, cover)
        && write_bytes(payload, &mut c, br#""},"authors":["#)
        && (author.is_empty()
            || (write_bytes(payload, &mut c, b"\"")
                && append_json_escaped(payload, &mut c, author)
                && write_bytes(payload, &mut c, b"\"")))
        && write_bytes(payload, &mut c, br#"],"artists":[],"status":""#)
        && write_bytes(payload, &mut c, status)
        && write_bytes(
            payload,
            &mut c,
            br#"","contentRating":"safe","language":"zh","tags":["#,
        );
    if !ok {
        return write_error("get_manga", "internal_error", "payload overflow");
    }
    let mut tags = 0;
    for tag in genres
        .split(|b| b.is_ascii_whitespace())
        .filter(|v| !v.is_empty())
    {
        if !(tags == 0 || write_bytes(payload, &mut c, b","))
            || !write_bytes(payload, &mut c, b"\"")
            || !append_json_escaped(payload, &mut c, tag)
            || !write_bytes(payload, &mut c, b"\"")
        {
            return write_error("get_manga", "internal_error", "payload overflow");
        }
        tags += 1;
    }
    if !write_bytes(payload, &mut c, br#"],"links":[{"kind":"source","url":""#)
        || !write_json_url(payload, &mut c, manga_id)
        || !write_bytes(payload, &mut c, br#""}]}}"#)
    {
        return write_error("get_manga", "internal_error", "payload overflow");
    }
    write_success_payload("get_manga", c)
}

// Metadata is emitted only when the Host declares support. The ordinal is
// the actual oldest-first source grid order verified against the site's reader
// menu, not a guessed chapter/volume number; duplicate and special titles stay.
fn supports_reading_order(req: &[u8]) -> bool {
    json_object_member(req, b"hostFeatures").map(|v| {
        let v = trim_ascii(v);
        v.starts_with(b"[") && v.ends_with(b"]") && v[1..v.len()-1].split(|c| *c == b',')
            .any(|item| trim_ascii(item) == b"\"chapter_reading_order_v1\"")
    }).unwrap_or(false)
}

fn run_get_chapters(req: &[u8]) -> u32 {
    let mut id_buf = [0; 1024];
    let Some(manga_id) = extract_json_string(args(req), b"mangaId")
        .and_then(|v| string_into(v, &mut id_buf))
        .and_then(path_from_href)
        .filter(|v| manga_path(v))
    else {
        return write_error("get_chapters", "invalid_request", "invalid mangaId");
    };
    let html_len = match fetch_manga_page("get_chapters", manga_id) {
        Ok(n) => n,
        Err(r) => return r,
    };
    let html = unsafe {
        core::slice::from_raw_parts(core::ptr::addr_of!(BODY_BUF).cast::<u8>(), html_len)
    };
    let document = match html_parse(html) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error("get_chapters", "parse_error", "html_parse failed"),
    };
    let batch = match DescriptorBatch::select(document.0, b".mint-chapter-grid a") {
        Ok(b) if b.count > 0 => b,
        _ => {
            return write_error(
                "get_chapters",
                "parse_error",
                "chapter grid missing or exceeds limit",
            )
        }
    };
    let payload = payload_buf();
    let mut c = 0;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("get_chapters", "internal_error", "payload overflow");
    }
    let mut written = 0;
    // The public catalogue is newest first; preserve the source's existing oldest-first contract.
    for i in (0..batch.count).rev() {
        if host::check_cancel() {
            return write_error("get_chapters", "cancelled", "host cancelled");
        }
        let Some(d) = batch.at(i) else {
            return write_error("get_chapters", "parse_error", "invalid chapter descriptor");
        };
        let mut name_buf = [0; 512];
        let mut href_buf = [0; 1024];
        let Some(name) = text_into(d, &mut name_buf).filter(|v| !v.is_empty()) else {
            return write_error("get_chapters", "parse_error", "chapter title missing");
        };
        if find_subslice(name, b"APP").is_some() {
            continue;
        }
        let Some(path) = attr_into(d, b"href", &mut href_buf)
            .and_then(path_from_href)
            .filter(|v| chapter_path(v))
        else {
            return write_error("get_chapters", "parse_error", "invalid chapter link");
        };
        let ok = (written == 0 || write_bytes(payload, &mut c, b",")) &&
            write_bytes(payload, &mut c, br#"{"id":""#) &&
            append_json_escaped(payload, &mut c, path) &&
            write_bytes(payload, &mut c, br#"","mangaId":""#) &&
            append_json_escaped(payload, &mut c, manga_id) &&
            write_bytes(payload, &mut c, br#"","title":""#) &&
            append_json_escaped(payload, &mut c, name) &&
            write_bytes(payload, &mut c, br#"","chapterNumber":"#) &&
            if let Some(n) = chapter_number(name) {
                write_bytes(payload, &mut c, b"\"") && write_usize(payload, &mut c, n) && write_bytes(payload, &mut c, b"\"")
            } else { write_bytes(payload, &mut c, b"null") } &&
            write_bytes(payload, &mut c, br#","volumeNumber":null"#) &&
            (!supports_reading_order(req) || (write_bytes(payload, &mut c, br#","readingGroup":"main","continuationEligible":true,"readingOrder":"#) && write_usize(payload, &mut c, written))) &&
            write_bytes(payload, &mut c, br#","language":"zh","publishedAt":null,"updatedAt":null,"pageCount":null}"#);
        if !ok {
            return write_error("get_chapters", "internal_error", "payload overflow");
        }
        written += 1;
    }
    if written == 0 {
        return write_error("get_chapters", "parse_error", "no readable chapters");
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

// The source's public /go link contains a literal reader redirect, not executable source data.
// Follow at most one observed HTTPS redirect to this reader origin, never arbitrary script or hosts.
fn reader_redirect(html: &[u8]) -> Result<Option<&[u8]>, ()> {
    let head = &html[..html.len().min(10240)];
    let Some(a) = find_subslice(head, b"location.replace(") else {
        return Ok(None);
    };
    let rest = trim_ascii(&head[a + b"location.replace(".len()..]);
    let quote = *rest.first().ok_or(())?;
    if quote != b'"' && quote != b'\'' {
        return Err(());
    }
    let rest = &rest[1..];
    let end = rest.iter().position(|b| *b == quote).ok_or(())?;
    let url = &rest[..end];
    let token = url.strip_prefix(b"https://reader.hqread.cc/r/").ok_or(())?;
    if token.is_empty()
        || !token
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        return Err(());
    }
    Ok(Some(url))
}

fn run_get_pages(req: &[u8]) -> u32 {
    let mut id_buf = [0; 1024];
    let Some(chapter_id) = extract_json_string(args(req), b"chapterId")
        .and_then(|v| string_into(v, &mut id_buf))
        .and_then(path_from_href)
        .filter(|v| chapter_path(v))
    else {
        return write_error("get_pages", "invalid_request", "invalid chapterId");
    };
    let Some(url_len) = build_url_with_path(chapter_id) else {
        return write_error("get_pages", "invalid_request", "invalid chapter URL");
    };
    let url = unsafe {
        core::slice::from_raw_parts(core::ptr::addr_of!(SCRATCH_A).cast::<u8>(), url_len)
    };
    let mut html_len = match fetch_body(url) {
        Ok(n) => n,
        Err(e) => {
            let (code, message) = fetch_error_code(e);
            return write_error("get_pages", code, message);
        }
    };
    let html = unsafe {
        core::slice::from_raw_parts(core::ptr::addr_of!(BODY_BUF).cast::<u8>(), html_len)
    };
    match reader_redirect(html) {
        Ok(Some(url)) => {
            // Copy the literal URL before the next fetch overwrites the body buffer.
            let mut reader_url = [0; 2048];
            if url.len() > reader_url.len() {
                return write_error("get_pages", "parse_error", "reader URL too long");
            }
            reader_url[..url.len()].copy_from_slice(url);
            html_len = match fetch_body(&reader_url[..url.len()]) {
                Ok(n) => n,
                Err(e) => {
                    let (code, message) = fetch_error_code(e);
                    return write_error("get_pages", code, message);
                }
            };
        }
        Ok(None) => {}
        Err(()) => return write_error("get_pages", "parse_error", "unsupported reader redirect"),
    }
    let html = unsafe {
        core::slice::from_raw_parts(core::ptr::addr_of!(BODY_BUF).cast::<u8>(), html_len)
    };
    let document = match html_parse(html) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error("get_pages", "parse_error", "html_parse failed"),
    };

    let batch = match DescriptorBatch::select(document.0, b"img.reader-image") {
        Ok(b) if b.count > 0 && b.count <= 2000 => b,
        _ => {
            return write_error(
                "get_pages",
                "parse_error",
                "reader images missing or exceed limit",
            )
        }
    };
    let payload = payload_buf();
    let mut c = 0usize;
    if !(write_bytes(payload, &mut c, br#"{"chapterId":""#)
        && append_json_escaped(payload, &mut c, chapter_id)
        && write_bytes(payload, &mut c, br#"","pages":["#))
    {
        return write_error("get_pages", "internal_error", "payload overflow");
    }

    for i in 0..batch.count {
        if host::check_cancel() {
            return write_error("get_pages", "cancelled", "host cancelled");
        }
        let Some(d) = batch.at(i) else {
            return write_error("get_pages", "parse_error", "invalid image descriptor");
        };
        let mut image_buf = [0; 2048];
        let Some(image) = attr_into(d, b"data-src", &mut image_buf).filter(|v| !v.is_empty())
        else {
            return write_error("get_pages", "parse_error", "reader image URL missing");
        };
        if !image.starts_with(b"https://wmh1234.wszwhg.net/") {
            return write_error("get_pages", "parse_error", "unsupported image origin");
        }
        let ok = (i == 0 || write_bytes(payload, &mut c, b","))
            && write_bytes(payload, &mut c, br#"{"id":"page:"#)
            && write_usize(payload, &mut c, i)
            && write_bytes(payload, &mut c, br#"","index":"#)
            && write_usize(payload, &mut c, i)
            && write_bytes(payload, &mut c, br#","image":{"kind":"url","url":""#)
            && write_json_url(payload, &mut c, image)
            && write_bytes(payload, &mut c, br#""}}"#);
        if !ok {
            return write_error("get_pages", "internal_error", "payload overflow");
        }
    }

    if !write_bytes(payload, &mut c, b"]}") {
        return write_error("get_pages", "internal_error", "payload overflow");
    }
    write_success_payload("get_pages", c)
}

fn run_get_image_request(req: &[u8]) -> u32 {
    let mut url_buf = [0; 2048];
    let Some(url) = extract_json_string(args(req), b"url")
        .or_else(|| extract_json_string(args(req), b"imageUrl"))
        .and_then(|v| string_into(v, &mut url_buf))
        .filter(|u| u.starts_with(b"https://wmh1234.wszwhg.net/"))
    else {
        return write_error(
            "get_image_request",
            "invalid_request",
            "unsupported image URL",
        );
    };
    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"url":""#)
        && append_json_escaped(payload, &mut c, url)
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
    response_buffer().write_success("get_listings", "{\"listings\":[{\"id\":\"popular\",\"name\":\"热门\"},{\"id\":\"latest\",\"name\":\"最新\"}]}".as_bytes())
}

#[no_mangle]
pub extern "C" fn koma_source_get_home(_req_ptr: u32, _req_len: u32) -> u32 {
    response_buffer().write_success("get_home", br#"{"sections":[]}"#)
}

#[no_mangle]
pub extern "C" fn koma_source_get_filters(req_ptr: u32, req_len: u32) -> u32 {
    let Some(req) = read_request(req_ptr, req_len) else {
        return write_error("get_filters", "invalid_request", "empty request");
    };
    // Older hosts have no scope contract: preserve the 0.1.4 empty schema.
    let supported = json_object_member(req, b"hostFeatures").map(|v| {
        let v = trim_ascii(v);
        v.starts_with(b"[") && v.ends_with(b"]") && v[1..v.len()-1].split(|c| *c == b',')
            .any(|item| trim_ascii(item) == b"\"source_filter_scope_header_v1\"")
    }).unwrap_or(false);
    if !supported { return response_buffer().write_success("get_filters", br#"{"filters":[]}"#); }
    let fetch_supported = json_object_member(req, b"hostFeatures").map(|v| {
        let v=trim_ascii(v); v.starts_with(b"[") && v.ends_with(b"]") && v[1..v.len()-1].split(|c| *c==b',')
            .any(|item| trim_ascii(item)==b"\"source_filter_fetching_v1\"")
    }).unwrap_or(false);
    let fetching = fetch_supported && json_object_member(args(req), b"fetchFilterData") == Some(b"true".as_slice());
    if fetch_supported && !fetching {
        // The stable genre ID keeps a saved value until options arrive on demand.
        return response_buffer().write_success("get_filters", r#"{"supportsFilterFetching":true,"filters":[{"id":"genre","name":"题材","type":"select","scope":"browse","default":"0"},{"id":"status","name":"状态","type":"select","scope":"browse","options":[{"value":"0","label":"全部"},{"value":"1","label":"连载"},{"value":"2","label":"完结"}],"default":"0"},{"id":"sort","name":"排序","type":"select","scope":"browse","options":[{"value":"id","label":"最新"},{"value":"hits","label":"热门"},{"value":"addtime","label":"更新"}]}]}"#.as_bytes());
    }
    let out = payload_buf();
    let mut n = 0;
    let prefix = if fetch_supported { br#"{"supportsFilterFetching":true,"filters":["#.as_slice() } else { br#"{"filters":["#.as_slice() };
    if !write_bytes(out, &mut n, prefix) { return write_error("get_filters", "internal_error", "overflow"); }
    let mut genre_written = false;
    let fetched = fetch_body(b"https://m.wmh1234.com/category/");
    if fetching { if let Err(e) = fetched { let (code,message)=fetch_error_code(e); return write_error("get_filters",code,message); } }
    // Older scope-aware hosts keep their original eager, optional fetch.
    if let Ok(len) = fetched {
        let html = unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(BODY_BUF).cast::<u8>(), len) };
        if let Ok(d) = html_parse(html) {
            let doc = OwnedDescriptor(d);
            if let Ok(batch) = DescriptorBatch::select(doc.0, b"a.filter-tag") {
                if batch.count > 0 && batch.count <= 512 {
                    let mut entries = 0;
                    let mut options = [0; 32768];
                    let mut used = 0;
                    for i in 0..batch.count {
                        let Some(node) = batch.at(i) else { continue };
                        let mut h = [0; 256]; let mut t = [0; 512];
                        let Some(href) = attr_into(node, b"href", &mut h) else { continue };
                        // The page also marks status/sort links filter-tag; only
                        // the provider's tags route is a genre option.
                        let Some(id) = href.strip_prefix(b"/category/tags/") else { continue };
                        let Some(name) = text_into(node, &mut t) else { continue };
                        if name.is_empty() || id.len() > 10 || number(id).is_none() { continue; }
                        if !(entries == 0 || write_bytes(&mut options, &mut used, b",")) || !write_bytes(&mut options, &mut used, br##"{"value":""##) || !write_bytes(&mut options, &mut used, id) || !write_bytes(&mut options, &mut used, br##"","label":""##) || !append_json_escaped(&mut options, &mut used, name) || !write_bytes(&mut options, &mut used, br#""}"#) { return write_error("get_filters", "internal_error", "genre overflow"); }
                        entries += 1;
                    }
                    if entries > 0 && (!write_bytes(out, &mut n, r#"{"id":"genre","name":"题材","type":"select","scope":"browse","options":["#.as_bytes()) || !write_bytes(out, &mut n, &options[..used]) || !write_bytes(out, &mut n, br#"],"default":"0"},"#)) { return write_error("get_filters", "internal_error", "overflow"); }
                    genre_written = entries > 0;
                }
            }
        }
    }
    if fetching && !genre_written { return write_error("get_filters", "parse_error", "genre options unavailable"); }
    if !write_bytes(out, &mut n, r#"{"id":"status","name":"状态","type":"select","scope":"browse","options":[{"value":"0","label":"全部"},{"value":"1","label":"连载"},{"value":"2","label":"完结"}],"default":"0"},{"id":"sort","name":"排序","type":"select","scope":"browse","options":[{"value":"id","label":"最新"},{"value":"hits","label":"热门"},{"value":"addtime","label":"更新"}]}]}"#.as_bytes()) { return write_error("get_filters", "internal_error", "overflow"); }
    write_success_payload("get_filters", n)
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
