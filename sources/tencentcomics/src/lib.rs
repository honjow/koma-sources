//! Tencent Comics public HTML source; attribution and support boundaries in NOTICE/README.
#![no_std]
mod nonce;
use koma_source_sdk::host::{
    html_attr, html_close, html_parse, html_select, html_text, http_request, HtmlDescriptor,
};
use koma_source_sdk::json_utils::{
    append_json_escaped, extract_json_number, extract_json_string, json_object_member,
    request_page_number, write_bytes, write_url_encoded, write_usize, JsonArrayIter,
};
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{build_get_request, fetch_error_code, FetchError};
const BASE: &[u8] = b"https://ac.qq.com";
const MOBILE: &[u8] = b"https://m.ac.qq.com";
const UA:&[u8]=b"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/139.0.0.0 Safari/537.36";
const MOBILE_UA:&[u8]=b"Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Mobile Safari/537.36";
const SOURCE_INFO:SourceInfo=SourceInfo{id:"com.tencentcomics.koma",name:"腾讯动漫",version:"0.1.2",api_version:"0.2",language:"zh",author:"Koma",description:"Tencent Comics public catalogues and free chapters; paid content and JavaScript execution are unsupported.",content_rating:"safe"};
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
    credentials: false,
    image_request: true,
};
koma_source_sdk::koma_source_buffers! {payload:1024*1024,http_out:2*1024*1024,body:2*1024*1024,http_req:8192,scratch:16384,}
koma_source_sdk::koma_source_helpers!();
// Diagnostic text is source-owned: no request arguments, response bodies or image URLs.
// Native callers intentionally hide raw source failures from the visible UI.
fn source_error(operation: &str, code: &str, message: &str) -> u32 {
    let mut line = [0u8; 256];
    let mut len = 0;
    if write_bytes(&mut line, &mut len, b"tencent failure operation=")
        && write_bytes(&mut line, &mut len, operation.as_bytes())
        && write_bytes(&mut line, &mut len, b" code=")
        && write_bytes(&mut line, &mut len, code.as_bytes())
        && write_bytes(&mut line, &mut len, b" stage=")
        && write_bytes(&mut line, &mut len, message.as_bytes())
    {
        koma_source_sdk::host::log_info(&line[..len]);
    } else {
        koma_source_sdk::host::log_info(b"tencent failure diagnostic truncated");
    }
    write_error(operation, code, message)
}

#[link(wasm_import_module = "koma_host")]
extern "C" {
    #[link_name = "html_select_all"]
    fn select_all(d: i32, s: *const u8, n: u32, o: *mut u8, c: u32) -> i32;
}
struct Doc(HtmlDescriptor);
impl Drop for Doc {
    fn drop(&mut self) {
        let _ = html_close(self.0);
    }
}
// One bounded handle buffer is owned by one live Selection at a time.
// Avoid a 32 KiB local/return array under the production interpreter stack.
static mut HANDLES: [i32; 8192] = [-1; 8192];
static mut HANDLES_BORROWED: bool = false;
struct Selection {
    len: usize,
}
impl Selection {
    fn new(d: HtmlDescriptor, s: &[u8]) -> Result<Self, &'static str> {
        unsafe {
            if HANDLES_BORROWED {
                return Err("nested HTML handle selection");
            }
            HANDLES_BORROWED = true;
        }
        let mut out = Self { len: 0 };
        let n = unsafe {
            select_all(
                d.raw(),
                s.as_ptr(),
                s.len() as u32,
                core::ptr::addr_of_mut!(HANDLES) as *mut u8,
                (8192 * 4) as u32,
            )
        };
        if n < 0 {
            return Err("html selection failed or exceeded capacity");
        }
        out.len = (n as usize).min(8192);
        if n as usize >= 8192 {
            return Err("html selection exceeded capacity");
        }
        Ok(out)
    }
    fn at(&self, i: usize) -> HtmlDescriptor {
        unsafe { core::mem::transmute(HANDLES[i]) }
    }
}
impl Drop for Selection {
    fn drop(&mut self) {
        for i in 0..self.len {
            unsafe {
                if HANDLES[i] >= 0 {
                    let _ = html_close(self.at(i));
                }
                HANDLES[i] = -1;
            }
        }
        unsafe {
            HANDLES_BORROWED = false;
        }
    }
}
fn child_text<'a>(d: HtmlDescriptor, s: &[u8], o: &'a mut [u8]) -> Option<&'a [u8]> {
    let e = Doc(html_select(d, s).ok()?);
    let n = html_text(e.0, o).ok()?;
    Some(trim_ascii(&o[..n]))
}
fn child_attr<'a>(d: HtmlDescriptor, s: &[u8], a: &[u8], o: &'a mut [u8]) -> Option<&'a [u8]> {
    let e = Doc(html_select(d, s).ok()?);
    let n = html_attr(e.0, a, o).ok()?;
    Some(&o[..n])
}
// Decode JSON string contents before URL encoding (including UTF-16 pairs).
fn decode_string<'a>(b: &[u8], out: &'a mut [u8]) -> Option<&'a [u8]> {
    let mut i = 0;
    let mut n = 0;
    while i < b.len() {
        if b[i] != b'\\' {
            if !write_bytes(out, &mut n, &b[i..i + 1]) {
                return None;
            }
            i += 1;
            continue;
        }
        i += 1;
        let c = *b.get(i)?;
        i += 1;
        match c {
            b'"' | b'\\' | b'/' => {
                if !write_bytes(out, &mut n, &[c]) {
                    return None;
                }
            }
            b'n' | b'r' | b't' | b'b' | b'f' => {
                let c = match c {
                    b'n' => 10,
                    b'r' => 13,
                    b't' => 9,
                    b'b' => 8,
                    _ => 12,
                };
                if !write_bytes(out, &mut n, &[c]) {
                    return None;
                }
            }
            b'u' => {
                let mut cp = hex4(b.get(i..i + 4)?)?;
                i += 4;
                if (0xd800..=0xdbff).contains(&cp) {
                    if b.get(i..i + 2)? != b"\\u" {
                        return None;
                    }
                    i += 2;
                    let lo = hex4(b.get(i..i + 4)?)?;
                    i += 4;
                    if !(0xdc00..=0xdfff).contains(&lo) {
                        return None;
                    }
                    cp = 0x10000 + ((cp - 0xd800) << 10) + (lo - 0xdc00)
                } else if (0xdc00..=0xdfff).contains(&cp) {
                    return None;
                }
                let mut tmp = [0u8; 4];
                let len = koma_source_sdk::json_utils::encode_utf8(cp, &mut tmp);
                if !write_bytes(out, &mut n, &tmp[..len]) {
                    return None;
                }
            }
            _ => return None,
        }
    }
    Some(&out[..n])
}
fn hex4(b: &[u8]) -> Option<u32> {
    let mut n = 0;
    for &c in b {
        let v = match c {
            b'0'..=b'9' => c - b'0',
            b'a'..=b'f' => c - b'a' + 10,
            b'A'..=b'F' => c - b'A' + 10,
            _ => return None,
        };
        n = (n << 4) | v as u32
    }
    Some(n)
}
fn decimal(b: &[u8]) -> bool {
    !b.is_empty() && b.len() <= 12 && b.iter().all(u8::is_ascii_digit)
}
fn numeric_id<'a>(b: &'a [u8], prefix: &[u8]) -> Option<&'a [u8]> {
    let id = b.strip_prefix(prefix)?;
    if decimal(id) {
        Some(id)
    } else {
        None
    }
}
fn fetch_html(url: &[u8], mobile: bool) -> Result<Doc, FetchError> {
    let len = fetch_body(url, mobile)?;
    html_parse(&body_buf()[..len])
        .map(Doc)
        .map_err(|_| FetchError::ClientError)
}
fn fetch_body(url: &[u8], mobile: bool) -> Result<usize, FetchError> {
    let n = build_get_request(
        http_req_buf(),
        url,
        Some(BASE),
        &[(b"User-Agent", if mobile { MOBILE_UA } else { UA })],
    )
    .ok_or(FetchError::Network)?;
    let n = http_request(&http_req_buf()[..n], http_out()).map_err(|_| FetchError::Network)?;
    let r = &http_out()[..n];
    let c = extract_json_number(r, b"statusCode")
        .or_else(|| extract_json_number(r, b"status"))
        .ok_or(FetchError::Network)?;
    let c = c
        .iter()
        .try_fold(0u32, |n, b| {
            if b.is_ascii_digit() {
                n.checked_mul(10)?.checked_add((b - b'0') as u32)
            } else {
                None
            }
        })
        .ok_or(FetchError::Network)?;
    match c {
        200..=299 => {}
        404 => return Err(FetchError::NotFound),
        429 => return Err(FetchError::RateLimit),
        400..=499 => return Err(FetchError::ClientError),
        500..=599 => return Err(FetchError::ServerError),
        _ => return Err(FetchError::Network),
    }
    decode_json_body(r)
}
fn path_id<'a>(b: &'a [u8], p: &[u8]) -> Option<&'a [u8]> {
    let b = b.strip_prefix(BASE).unwrap_or(b);
    let b = b.strip_prefix(MOBILE).unwrap_or(b);
    numeric_id(b, p)
}
fn chapter_parts(b: &[u8]) -> Option<(&[u8], &[u8])> {
    let b = b.strip_prefix(b"tencent:chapter:")?;
    let p = b.iter().position(|&b| b == b':')?;
    let (m, c) = (&b[..p], &b[p + 1..]);
    if decimal(m) && decimal(c) {
        Some((m, c))
    } else {
        None
    }
}
fn chapter_path(b: &[u8]) -> Option<(&[u8], &[u8])> {
    let b = b
        .strip_prefix(BASE)
        .unwrap_or(b)
        .strip_prefix(b"/ComicView/index/id/")?;
    let p = b.windows(5).position(|x| x == b"/cid/")?;
    let (m, c) = (&b[..p], &b[p + 5..]);
    if decimal(m) && decimal(c) {
        Some((m, c))
    } else {
        None
    }
}
fn allowed_image(u: &[u8]) -> bool {
    u.starts_with(b"https://manhua.acimg.cn/")
        && !u.iter().any(|&b| b <= 32 || b == 127 || b == b'\\')
}
fn url(out: &mut [u8], parts: &[&[u8]]) -> Option<usize> {
    let mut n = 0;
    for &b in parts {
        if !write_bytes(out, &mut n, b) {
            return None;
        }
    }
    Some(n)
}
fn quoted(out: &mut [u8], n: &mut usize, b: &[u8]) -> bool {
    write_bytes(out, n, b"\"") && append_json_escaped(out, n, b) && write_bytes(out, n, b"\"")
}
fn tail(out: &mut [u8], n: &mut usize, more: bool, page: usize) -> bool {
    if !write_bytes(out, n, br#"],"page":{"nextCursor":"#) {
        return false;
    }
    if more {
        write_bytes(out, n, b"\"")
            && write_usize(out, n, page + 1)
            && write_bytes(out, n, br#"","hasMore":true}}"#)
    } else {
        write_bytes(out, n, b"null,\"hasMore\":false}}")
    }
}
fn failure(op: &str, e: FetchError) -> u32 {
    let (c, m) = fetch_error_code(e);
    source_error(op, c, m)
}
fn list(req: &[u8], op: &str) -> u32 {
    let page = request_page_number(req);
    if page > 1000 {
        return source_error(op, "invalid_request", "page limit exceeded");
    }
    let mobile = op == "search";
    let mut query = [0u8; 4096];
    let q = match decode_string(
        extract_json_string(req, b"query").unwrap_or(b""),
        &mut query,
    ) {
        Some(q) => q,
        None => return source_error(op, "invalid_request", "invalid query"),
    };
    let listing = extract_json_string(req, b"listingId").unwrap_or(b"popular");
    if !mobile && listing != b"popular" && listing != b"latest" {
        return source_error(op, "invalid_request", "unknown listing");
    }
    let mut u = [0u8; 8192];
    let mut n = 0;
    let ok = if mobile && !q.is_empty() {
        write_bytes(&mut u, &mut n, MOBILE)
            && write_bytes(&mut u, &mut n, b"/search/result?word=")
            && write_url_encoded(&mut u, &mut n, q)
            && write_bytes(&mut u, &mut n, b"&page=")
            && write_usize(&mut u, &mut n, page)
    } else {
        write_bytes(&mut u, &mut n, BASE)
            && write_bytes(&mut u, &mut n, b"/Comic/all/search/")
            && write_bytes(
                &mut u,
                &mut n,
                if listing == b"latest" {
                    b"time"
                } else {
                    b"hot"
                },
            )
            && write_bytes(&mut u, &mut n, b"/page/")
            && write_usize(&mut u, &mut n, page)
    };
    if !ok {
        return source_error(op, "invalid_request", "query too long");
    }
    let mobile = mobile && !q.is_empty();
    let d = match fetch_html(&u[..n], mobile) {
        Ok(d) => d,
        Err(e) => return failure(op, e),
    };
    if mobile && !has(d.0, b"ul#lst_searchResult") {
        return source_error(op, "parse_error", "missing public search result container");
    }
    let cards = match Selection::new(
        d.0,
        if mobile {
            b"ul > li.comic-item > a"
        } else {
            b"ul.ret-search-list.clearfix > li"
        },
    ) {
        Ok(x) => x,
        Err(m) => return source_error(op, "parse_error", m),
    };
    let out = payload_buf();
    let mut n = 0;
    let mut count = 0;
    if !write_bytes(out, &mut n, br#"{"items":["#) {
        return source_error(op, "internal_error", "overflow");
    }
    for i in 0..cards.len {
        let mut href = [0u8; 512];
        let mut title = [0u8; 4096];
        let mut cover = [0u8; 4096];
        let node = cards.at(i);
        let h = if mobile {
            html_attr(node, b"href", &mut href).ok().map(|n| &href[..n])
        } else {
            child_attr(node, b"div > a", b"href", &mut href)
        };
        let id = match h.and_then(|h| {
            path_id(
                h,
                if mobile {
                    b"/comic/index/id/"
                } else {
                    b"/Comic/comicInfo/id/"
                },
            )
        }) {
            Some(id) => id,
            None => return source_error(op, "parse_error", "invalid catalogue identity"),
        };
        let t = if mobile {
            child_text(node, b"div > strong", &mut title)
        } else {
            child_attr(node, b"div > a", b"title", &mut title)
        };
        let t = match t {
            Some(t) if !t.is_empty() => t,
            _ => return source_error(op, "parse_error", "missing title"),
        };
        let c = child_attr(
            node,
            if mobile {
                b"div > img"
            } else {
                b"div > a > img"
            },
            if mobile { b"src" } else { b"data-original" },
            &mut cover,
        );
        let c = match c {
            Some(c) if allowed_image(c) => c,
            _ => return source_error(op, "parse_error", "unsupported cover"),
        };
        if count > 0 && !write_bytes(out, &mut n, b",") {
            return source_error(op, "internal_error", "overflow");
        }
        if !(write_bytes(out, &mut n, br#"{"id":"tencent:manga:"#)
            && write_bytes(out, &mut n, id)
            && write_bytes(out, &mut n, br#"","title":"#)
            && quoted(out, &mut n, t)
            && write_bytes(out, &mut n, br#", "cover":{"kind":"url","url":"#)
            && quoted(out, &mut n, c)
            && write_bytes(
                out,
                &mut n,
                br#"},"authors":[],"status":"unknown","contentRating":"safe"}"#,
            ))
        {
            return source_error(op, "internal_error", "overflow");
        }
        count += 1;
    }
    drop(cards);
    // Current desktop site supplies an actual next link; mobile has a ten-item contract.
    let mut pager = [0u8; 512];
    let more = if mobile {
        count == 10
    } else {
        child_attr(d.0, b"nav.mod-pager a.mod-pager-next", b"href", &mut pager).is_some()
    };
    if !mobile && count == 0 {
        return source_error(op, "parse_error", "missing public catalogue");
    }
    if !tail(out, &mut n, more, page) {
        return source_error(op, "internal_error", "overflow");
    }
    write_success_payload(op, n)
}
fn run_search(r: &[u8]) -> u32 {
    list(r, "search")
}
fn run_get_manga_list(r: &[u8]) -> u32 {
    list(r, "get_manga_list")
}
fn manga_id(r: &[u8]) -> Option<&[u8]> {
    extract_json_string(r, b"mangaId").and_then(|b| numeric_id(b, b"tencent:manga:"))
}
fn detail(id: &[u8]) -> Result<Doc, FetchError> {
    let mut u = [0u8; 512];
    let n = url(&mut u, &[BASE, b"/Comic/comicInfo/id/", id]).ok_or(FetchError::Network)?;
    fetch_html(&u[..n], false)
}
fn run_get_manga(r: &[u8]) -> u32 {
    let op = "get_manga";
    let id = match manga_id(r) {
        Some(x) => x,
        None => return source_error(op, "invalid_request", "invalid manga identity"),
    };
    let d = match detail(id) {
        Ok(d) => d,
        Err(e) => return failure(op, e),
    };
    let mut title = [0u8; 4096];
    let mut cover = [0u8; 4096];
    let mut desc = [0u8; 16384];
    let mut author = [0u8; 4096];
    let mut status = [0u8; 128];
    let t = match child_text(d.0, b".works-intro-title > strong", &mut title) {
        Some(t) if !t.is_empty() => t,
        _ => return source_error(op, "not_found", "public comic details unavailable"),
    };
    let c = match child_attr(
        d.0,
        b"div.works-cover.ui-left > a > img",
        b"src",
        &mut cover,
    ) {
        Some(c) if allowed_image(c) => c,
        _ => return source_error(op, "parse_error", "unsupported cover"),
    };
    let de = child_text(d.0, b"p.works-intro-short", &mut desc).unwrap_or(b"");
    let a = child_text(d.0, b"p.works-intro-digi > span.first > em", &mut author).unwrap_or(b"");
    let s = child_text(d.0, b"label.works-intro-status", &mut status).unwrap_or(b"");
    let st = if s == "已完结".as_bytes() || s == "已完結".as_bytes() {
        b"completed" as &[u8]
    } else if s == "连载中".as_bytes() || s == "連載中".as_bytes() {
        b"ongoing"
    } else {
        b"unknown"
    };
    let out = payload_buf();
    let mut n = 0;
    if !(write_bytes(out,&mut n,br#"{"manga":{"id":"tencent:manga:"#)&&write_bytes(out,&mut n,id)&&write_bytes(out,&mut n,br#"","title":"#)&&quoted(out,&mut n,t)&&write_bytes(out,&mut n,br#", "cover":{"kind":"url","url":"#)&&quoted(out,&mut n,c)&&write_bytes(out,&mut n,br#"},"description":"#)&&quoted(out,&mut n,de)&&write_bytes(out,&mut n,br#", "authors":["#)&&quoted(out,&mut n,a)&&write_bytes(out,&mut n,br#"],"status":"#)&&quoted(out,&mut n,st)&&write_bytes(out,&mut n,br#", "contentRating":"safe","language":"zh","links":[{"kind":"source","url":"https://ac.qq.com/Comic/comicInfo/id/"#)&&write_bytes(out,&mut n,id)&&write_bytes(out,&mut n,b"\"}]}}")){return source_error(op,"internal_error","overflow")};
    write_success_payload(op, n)
}
fn has(d: HtmlDescriptor, s: &[u8]) -> bool {
    html_select(d, s)
        .map(|h| {
            let _ = html_close(h);
            true
        })
        .unwrap_or(false)
}
fn chapter_is_free(d: HtmlDescriptor) -> bool {
    has(d, b".ui-icon-free") && !has(d, b".ui-icon-pay")
}
// The official chapter directory uses both "1.title" and "259 title".
// Require a delimiter; a title beginning with digits alone is not a contract.
fn explicit_chapter_number(title: &[u8]) -> Option<&[u8]> {
    let end = title.iter().position(|b| !b.is_ascii_digit()).unwrap_or(title.len());
    if end == 0 { return None; }
    match title.get(end) {
        Some(b'.') => Some(&title[..end]),
        Some(separator) if separator.is_ascii_whitespace() &&
            !trim_ascii(&title[end..]).is_empty() => Some(&title[..end]),
        _ => None,
    }
}
fn run_get_chapters(r: &[u8]) -> u32 {
    let op = "get_chapters";
    let id = match manga_id(r) {
        Some(x) => x,
        None => return source_error(op, "invalid_request", "invalid manga identity"),
    };
    let d = match detail(id) {
        Ok(d) => d,
        Err(e) => return failure(op, e),
    };
    let chapters = match Selection::new(d.0, b".chapter-page-all .works-chapter-item") {
        Ok(x) if x.len > 0 => x,
        Ok(_) => return source_error(op, "not_found", "public chapters unavailable"),
        Err(m) => return source_error(op, "parse_error", m),
    };
    let out = payload_buf();
    let mut n = 0;
    write_bytes(out, &mut n, br#"{"items":["#);
    for i in 0..chapters.len {
        let node = chapters.at(i);
        let mut h = [0u8; 512];
        let mut t = [0u8; 4096];
        let (m, c) = match child_attr(node, b"a", b"href", &mut h).and_then(chapter_path) {
            Some((m, c)) if m == id => (m, c),
            _ => return source_error(op, "parse_error", "mismatched chapter identity"),
        };
        let title = match child_text(node, b"a", &mut t) {
            Some(t) if !t.is_empty() => t,
            _ => return source_error(op, "parse_error", "missing chapter title"),
        };
        let number = explicit_chapter_number(title);
        let free = chapter_is_free(node);
        if i > 0 && !write_bytes(out, &mut n, b",") {
            return source_error(op, "internal_error", "overflow");
        }
        if !(write_bytes(out, &mut n, br#"{"id":"tencent:chapter:"#)
            && write_bytes(out, &mut n, m)
            && write_bytes(out, &mut n, b":")
            && write_bytes(out, &mut n, c)
            && write_bytes(out, &mut n, br#"","mangaId":"tencent:manga:"#)
            && write_bytes(out, &mut n, m)
            && write_bytes(out, &mut n, br#"","title":"#)
            && quoted(out, &mut n, title)
            && write_bytes(out, &mut n, br#", "sequenceIndex":"#)
            && write_usize(out, &mut n, i)
            && write_bytes(out, &mut n, br#", "chapterNumber":"#)
            && if let Some(number) = number {
                quoted(out, &mut n, number) && write_bytes(out, &mut n,
                    br#", "readingGroup":"chapter","continuationEligible":true"#)
            } else {
                write_bytes(out, &mut n,
                    br#"null, "readingGroup":"unknown","continuationEligible":false"#)
            }
            && write_bytes(out, &mut n, br#", "language":"zh","locked":"#)
            && write_bytes(out, &mut n, if free { b"false" } else { b"true" })
            && write_bytes(out, &mut n, b"}"))
        {
            return source_error(op, "internal_error", "overflow");
        }
    }
    if !tail(out, &mut n, false, 1) {
        return source_error(op, "internal_error", "overflow");
    }
    write_success_payload(op, n)
}
fn free_chapter(m: &[u8], c: &[u8]) -> Result<(), (&'static str, &'static str)> {
    let d = detail(m).map_err(fetch_error_code)?;
    let chapters = Selection::new(d.0, b".chapter-page-all .works-chapter-item")
        .map_err(|e| ("parse_error", e))?;
    for i in 0..chapters.len {
        let mut h = [0u8; 512];
        let node = chapters.at(i);
        if let Some((a, b)) = child_attr(node, b"a", b"href", &mut h).and_then(chapter_path) {
            if a == m && b == c {
                return if chapter_is_free(node) {
                    Ok(())
                } else {
                    Err((
                        "permission_denied",
                        "paid or unmarked chapter is not supported",
                    ))
                };
            }
        }
    }
    Err(("not_found", "chapter is absent from public catalogue"))
}
static mut ENCODED: [u8; 128 * 1024] = [0; 128 * 1024];
static mut DECODED: [u8; 128 * 1024] = [0; 128 * 1024];
fn field<'a>(b: &'a [u8], k: &[u8]) -> Option<&'a [u8]> {
    json_object_member(b, k)
}
fn scalar<'a>(b: &'a [u8], k: &[u8]) -> Option<&'a [u8]> {
    let v = field(b, k)?;
    if v.first() == Some(&b'"') && v.last() == Some(&b'"') {
        Some(&v[1..v.len() - 1])
    } else {
        Some(v)
    }
}
fn run_get_pages(r: &[u8]) -> u32 {
    let op = "get_pages";
    let (m, c) = match extract_json_string(r, b"chapterId").and_then(chapter_parts) {
        Some(x) => x,
        None => return source_error(op, "invalid_request", "invalid chapter identity"),
    };
    // Only validated public numeric identities are logged, never URLs or response content.
    let mut identity = [0u8; 96];
    let mut identity_len = 0;
    if write_bytes(&mut identity, &mut identity_len, b"tencent pages manga=")
        && write_bytes(&mut identity, &mut identity_len, m)
        && write_bytes(&mut identity, &mut identity_len, b" chapter=")
        && write_bytes(&mut identity, &mut identity_len, c)
    {
        koma_source_sdk::host::log_info(&identity[..identity_len]);
    }
    if let Err((code, msg)) = free_chapter(m, c) {
        return source_error(op, code, msg);
    }
    let mut u = [0u8; 512];
    let n = match url(&mut u, &[BASE, b"/ComicView/index/id/", m, b"/cid/", c]) {
        Some(n) => n,
        None => return source_error(op, "invalid_request", "invalid chapter identity"),
    };
    let n = match fetch_body(&u[..n], false) {
        Ok(n) => n,
        Err(e) => return failure(op, e),
    };
    let n = unsafe {
        nonce::chapter_data(
            &body_buf()[..n],
            &mut *core::ptr::addr_of_mut!(ENCODED),
            &mut *core::ptr::addr_of_mut!(DECODED),
        )
    };
    let n = match n {
        Some(n) => n,
        None => {
            return source_error(
                op,
                "parse_error",
                "unsupported public chapter encoding; JavaScript is not executed",
            )
        }
    };
    let data = unsafe { &(&*core::ptr::addr_of!(DECODED))[..n] };
    let comic = match field(data, b"comic") {
        Some(x) => x,
        None => return source_error(op, "parse_error", "missing comic"),
    };
    let chapter = match field(data, b"chapter") {
        Some(x) => x,
        None => return source_error(op, "parse_error", "missing chapter"),
    };
    if scalar(comic, b"id") != Some(m) || scalar(chapter, b"cid") != Some(c) {
        return source_error(op, "parse_error", "chapter response identity mismatch");
    }
    if field(chapter, b"canRead") != Some(b"true") {
        return source_error(op, "permission_denied", "chapter is not publicly readable");
    }
    let mut pics = match field(data, b"picture").and_then(JsonArrayIter::from_raw_array) {
        Some(x) => x,
        None => return source_error(op, "parse_error", "missing public pages"),
    };
    let out = payload_buf();
    let mut n = 0;
    write_bytes(out, &mut n, br#"{"pages":["#);
    let mut count = 0;
    let mut ids = [[0u8; 12]; 256];
    let mut lens = [0usize; 256];
    while let Some(pic) = pics.next_object() {
        if count >= 256 {
            return source_error(op, "parse_error", "page limit exceeded");
        }
        let id = match scalar(pic, b"pid") {
            Some(x) if decimal(x) => x,
            _ => return source_error(op, "parse_error", "missing stable page identity"),
        };
        if (0..count).any(|i| &ids[i][..lens[i]] == id) {
            return source_error(op, "parse_error", "duplicate page identity");
        }
        ids[count][..id.len()].copy_from_slice(id);
        lens[count] = id.len();
        let mut image = [0u8; 8192];
        let image =
            match extract_json_string(pic, b"url").and_then(|x| decode_string(x, &mut image)) {
                Some(x) if allowed_image(x) => x,
                _ => return source_error(op, "parse_error", "unsupported image URL"),
            };
        if count > 0 && !write_bytes(out, &mut n, b",") {
            return source_error(op, "internal_error", "overflow");
        }
        if !(write_bytes(out, &mut n, br#"{"id":"tencent:page:"#)
            && write_bytes(out, &mut n, m)
            && write_bytes(out, &mut n, b":")
            && write_bytes(out, &mut n, c)
            && write_bytes(out, &mut n, b":")
            && write_bytes(out, &mut n, id)
            && write_bytes(out, &mut n, br#"","index":"#)
            && write_usize(out, &mut n, count)
            && write_bytes(out, &mut n, br#", "image":{"kind":"url","url":"#)
            && quoted(out, &mut n, image)
            && write_bytes(out, &mut n, b"}}"))
        {
            return source_error(op, "internal_error", "overflow");
        };
        count += 1;
    }
    if count == 0 {
        return source_error(op, "parse_error", "empty public pages");
    }
    if !write_bytes(out, &mut n, b"]}") {
        return source_error(op, "internal_error", "overflow");
    }
    write_success_payload(op, n)
}
fn run_get_image_request(r: &[u8]) -> u32 {
    let mut u = [0u8; 8192];
    let u = match extract_json_string(r, b"url").and_then(|x| decode_string(x, &mut u)) {
        Some(u) if allowed_image(u) => u,
        _ => {
            return source_error(
                "get_image_request",
                "invalid_request",
                "unsupported image URL",
            )
        }
    };
    let out = payload_buf();
    let mut n = 0;
    if !(write_bytes(out, &mut n, br#"{"url":"#)
        && quoted(out, &mut n, u)
        && write_bytes(
            out,
            &mut n,
            br#", "headers":{"Referer":"https://ac.qq.com/","User-Agent":"#,
        )
        && quoted(out, &mut n, UA)
        && write_bytes(out, &mut n, b"}}"))
    {
        return source_error("get_image_request", "internal_error", "overflow");
    }
    write_success_payload("get_image_request", n)
}
fn run_get_listings(_: &[u8]) -> u32 {
    let b = r#"{"listings":[{"id":"popular","name":"热门","kind":"popular"},{"id":"latest","name":"最新","kind":"latest"}]}"#
        .as_bytes();
    payload_buf()[..b.len()].copy_from_slice(b);
    write_success_payload("get_listings", b.len())
}
fn run_get_home(_: &[u8]) -> u32 {
    source_error("get_home", "not_implemented", "")
}
fn run_get_filters(_: &[u8]) -> u32 {
    source_error("get_filters", "not_implemented", "")
}
fn run_get_settings(_: &[u8]) -> u32 {
    source_error("get_settings", "not_implemented", "")
}
koma_source_sdk::koma_source_exports!("tencentcomics");
