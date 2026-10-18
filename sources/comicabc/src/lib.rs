//! ComicABC public source; support limits and Apache attribution in README/NOTICE.
#![no_std]
mod records;
mod s2t;
use koma_source_sdk::host::{
    html_attr, html_close, html_parse, html_select, html_text, http_request, HtmlDescriptor,
};
use koma_source_sdk::json_utils::{
    append_json_escaped, extract_json_number, extract_json_string, request_page_number,
    write_bytes, write_url_encoded, write_usize,
};
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{build_get_request, fetch_error_code, FetchError};
const BASE: &[u8] = b"https://www.8comic.com";
const READER: &[u8] = b"https://articles.onemoreplace.tw";
const UA:&[u8]=b"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/139.0.0.0 Safari/537.36";
const MOBILE_UA: &[u8] = UA;
const SOURCE_INFO:SourceInfo=SourceInfo{id:"com.comicabc.koma",name:"無限動漫",version:"0.1.10",api_version:"0.2",language:"zh",author:"Koma",description:"ComicABC public Chinese catalogues and bounded literal-record pages; no accounts or JavaScript execution.",content_rating:"safe"};
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
fn source_error(op: &str, code: &str, message: &str) -> u32 {
    let mut line = [0; 256];
    let mut n = 0;
    if write_bytes(&mut line, &mut n, b"comicabc failure ")
        && write_bytes(&mut line, &mut n, op.as_bytes())
        && write_bytes(&mut line, &mut n, b" ")
        && write_bytes(&mut line, &mut n, code.as_bytes())
        && write_bytes(&mut line, &mut n, b" ")
        && write_bytes(&mut line, &mut n, message.as_bytes())
    {
        koma_source_sdk::host::log_info(&line[..n]);
    }
    write_error(op, code, message)
}
fn failure(op: &str, e: FetchError) -> u32 {
    let (c, m) = fetch_error_code(e);
    source_error(op, c, m)
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
// Chapter handles use the consumed HTML buffer prefix. Text reads own only
// its remaining suffix; the DOM itself is owned independently by Lexbor.
struct ChapterSelection {
    len: usize,
    bytes: usize,
}
impl ChapterSelection {
    fn new(d: HtmlDescriptor) -> Result<Self, &'static str> {
        let selector = b"#chapters a, .comic_chapters a";
        let total = unsafe {
            select_all(d.raw(), selector.as_ptr(), selector.len() as u32,
                core::ptr::addr_of_mut!(BODY_BUF) as *mut u8, 0)
        };
        if total < 0 { return Err("chapter selection failed"); }
        let bytes = (total as usize).checked_mul(4)
            .filter(|bytes| *bytes <= BODY_CAP)
            .ok_or("chapter handles exceed available buffer")?;
        if total == 0 { return Ok(Self { len: 0, bytes: 0 }); }
        let actual = unsafe {
            select_all(d.raw(), selector.as_ptr(), selector.len() as u32,
                core::ptr::addr_of_mut!(BODY_BUF) as *mut u8, bytes as u32)
        };
        // Native negative paths allocate no child descriptors. Never inspect
        // unwritten buffer bytes or release more than the actual second pass.
        if actual < 0 { return Err("chapter selection failed"); }
        let out = Self { len: (actual as usize).min(bytes / 4), bytes };
        if actual != total { return Err("chapter selection changed"); }
        Ok(out)
    }
    fn at(&self, i: usize) -> Option<HtmlDescriptor> {
        if i >= self.len { return None; }
        let raw = unsafe {
            core::ptr::read_unaligned((core::ptr::addr_of!(BODY_BUF) as *const u8).add(i * 4).cast::<i32>())
        };
        if raw <= 0 { return None; }
        Some(unsafe { core::mem::transmute(raw) })
    }
    fn title_buffer(&self) -> &'static mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(
                (core::ptr::addr_of_mut!(BODY_BUF) as *mut u8).add(self.bytes),
                BODY_CAP - self.bytes,
            )
        }
    }
}
impl Drop for ChapterSelection {
    fn drop(&mut self) {
        for i in 0..self.len {
            if let Some(node) = self.at(i) { let _ = html_close(node); }
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
fn manga_id(r: &[u8]) -> Option<&[u8]> {
    numeric_id(extract_json_string(r, b"mangaId")?, b"comicabc:manga:")
}
fn allowed_image(u: &[u8]) -> bool {
    if u.iter().any(|&c| c <= 32 || c == 127 || c == b'\\') {
        return false;
    }
    if u.starts_with(b"https://www.8comic.com/pics/") {
        return true;
    }
    let p = b"https://img";
    if !u.starts_with(p) {
        return false;
    }
    let b = &u[p.len()..];
    b.first().map_or(false, u8::is_ascii_digit)
        && b.get(1..).map_or(false, |x| x.starts_with(b".8comic.com/"))
}
fn cover<'a>(raw: &[u8], out: &'a mut [u8]) -> Option<&'a [u8]> {
    let n = if raw.starts_with(b"/pics/") {
        url(out, &[BASE, raw])?
    } else {
        url(out, &[raw])?
    };
    if allowed_image(&out[..n]) {
        Some(&out[..n])
    } else {
        None
    }
}
fn path_manga<'a>(href: &'a [u8]) -> Option<&'a [u8]> {
    let p = href
        .strip_prefix(BASE)
        .unwrap_or(href)
        .strip_prefix(b"/html/")?
        .strip_suffix(b".html")?;
    if decimal(p) {
        Some(p)
    } else {
        None
    }
}
fn has(d: HtmlDescriptor, s: &[u8]) -> bool {
    html_select(d, s)
        .map(|x| {
            let _ = html_close(x);
            true
        })
        .unwrap_or(false)
}
fn list_fetch(query: &[u8], page: usize, latest: bool) -> Result<Doc, FetchError> {
    let mut u = [0; 8192];
    let mut n = 0;
    let ok = if !query.is_empty() {
        write_bytes(&mut u, &mut n, BASE)
            && write_bytes(&mut u, &mut n, b"/member/search.aspx?key=")
            && write_url_encoded(&mut u, &mut n, query)
            && write_bytes(&mut u, &mut n, b"&page=")
            && write_usize(&mut u, &mut n, page)
    } else {
        write_bytes(&mut u, &mut n, BASE)
            && write_bytes(
                &mut u,
                &mut n,
                if latest { b"/comic/u-" } else { b"/comic/h-" },
            )
            && write_usize(&mut u, &mut n, page)
            && write_bytes(&mut u, &mut n, b".html")
    };
    if !ok {
        return Err(FetchError::ClientError);
    }
    fetch_html(&u[..n], false)
}
fn traditional<'a>(query: &[u8], out: &'a mut [u8]) -> Option<&'a [u8]> {
    let query = core::str::from_utf8(query).ok()?;
    let mut n = 0;
    for ch in query.chars() {
        let mut it = s2t::PAIRS.chars();
        let mut result = ch;
        while let Some(from) = it.next() {
            let to = it.next()?;
            if from == ch {
                result = to;
                break;
            }
        }
        let mut b = [0; 4];
        if !write_bytes(out, &mut n, result.encode_utf8(&mut b).as_bytes()) {
            return None;
        }
    }
    Some(&out[..n])
}
fn list(r: &[u8], op: &str) -> u32 {
    let page = request_page_number(r);
    if page > 1000 {
        return source_error(op, "invalid_request", "page limit");
    }
    let mut q = [0; 4096];
    let query = if op == "search" {
        match decode_string(extract_json_string(r, b"query").unwrap_or(b""), &mut q) {
            Some(x) => x,
            None => return source_error(op, "invalid_request", "invalid query"),
        }
    } else {
        b""
    };
    let listing = extract_json_string(r, b"listingId").unwrap_or(b"popular");
    if op != "search" && listing != b"popular" && listing != b"latest" {
        return source_error(op, "invalid_request", "unknown listing");
    }
    let latest = op != "search" && listing == b"latest";
    let mut d = match list_fetch(query, page, latest) {
        Ok(x) => x,
        Err(e) => return failure(op, e),
    };
    let selector = if latest {
        b".container .row .cat2_list a" as &[u8]
    } else {
        b".container .row a.comicpic_col6"
    };
    // Match upstream behavior: one first-page simplified-to-traditional retry only on an empty result.
    if !query.is_empty() && page == 1 && !has(d.0, selector) {
        let mut converted = [0; 4096];
        if let Some(t) = traditional(query, &mut converted) {
            if t != query {
                drop(d);
                d = match list_fetch(t, page, false) {
                    Ok(x) => x,
                    Err(e) => return failure(op, e),
                };
            }
        }
    }
    let nodes = match Selection::new(d.0, selector) {
        Ok(x) => x,
        Err(e) => return source_error(op, "parse_error", e),
    };
    if nodes.len == 0 && (query.is_empty() || !has(d.0, b".container .row")) {
        return source_error(op, "parse_error", "missing public catalogue");
    }
    let out = payload_buf();
    let mut n = 0;
    if !write_bytes(out, &mut n, br#"{"items":["#) {
        return source_error(op, "internal_error", "overflow");
    }
    for i in 0..nodes.len {
        let node = nodes.at(i);
        let mut h = [0; 512];
        let mut t = [0; 4096];
        let mut c = [0; 4096];
        let mut normalized = [0; 4096];
        let hn = match html_attr(node, b"href", &mut h) {
            Ok(x) => x,
            Err(_) => return source_error(op, "parse_error", "missing catalogue ID"),
        };
        let id = match path_manga(&h[..hn]) {
            Some(x) => x,
            None => return source_error(op, "parse_error", "invalid catalogue ID"),
        };
        let title = match child_text(node, b"li.nowraphide", &mut t) {
            Some(x) if !x.is_empty() => x,
            _ => return source_error(op, "parse_error", "missing title"),
        };
        let image = match child_attr(node, b"img", b"src", &mut c)
            .and_then(|x| cover(x, &mut normalized))
        {
            Some(x) => x,
            None => return source_error(op, "parse_error", "unsupported cover"),
        };
        if i > 0 && !write_bytes(out, &mut n, b",") {
            return source_error(op, "internal_error", "overflow");
        }
        if !(write_bytes(out, &mut n, br#"{"id":"comicabc:manga:"#)
            && write_bytes(out, &mut n, id)
            && write_bytes(out, &mut n, br#"","title":"#)
            && quoted(out, &mut n, title)
            && write_bytes(out, &mut n, br#", "cover":{"kind":"url","url":"#)
            && quoted(out, &mut n, image)
            && write_bytes(
                out,
                &mut n,
                br#"},"authors":[],"status":"unknown","contentRating":"safe"}"#,
            ))
        {
            return source_error(op, "internal_error", "overflow");
        }
    }
    let more = has(d.0, b"div.pager a span.mdi-skip-next");
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
fn detail(id: &[u8]) -> Result<Doc, FetchError> {
    let mut u = [0; 512];
    let n = url(&mut u, &[BASE, b"/html/", id, b".html"]).ok_or(FetchError::ClientError)?;
    fetch_html(&u[..n], false)
}
fn run_get_manga(r: &[u8]) -> u32 {
    let op = "get_manga";
    let id = match manga_id(r) {
        Some(x) => x,
        None => return source_error(op, "invalid_request", "invalid manga ID"),
    };
    let d = match detail(id) {
        Ok(x) => x,
        Err(e) => return failure(op, e),
    };
    let mut title = [0; 4096];
    let mut c = [0; 4096];
    let mut normalized = [0; 4096];
    let mut description = [0; 16384];
    let mut author = [0; 4096];
    let mut status = [0; 128];
    let title = match child_text(d.0, b".item_content_box .h2", &mut title) {
        Some(x) if !x.is_empty() => x,
        _ => return source_error(op, "not_found", "public details absent"),
    };
    let image = match child_attr(d.0, b".item-cover img", b"src", &mut c)
        .and_then(|x| cover(x, &mut normalized))
    {
        Some(x) => x,
        None => return source_error(op, "parse_error", "unsupported cover"),
    };
    let description = child_text(
        d.0,
        b".item_content_box .item_info_detail",
        &mut description,
    )
    .unwrap_or(b"");
    let author =
        child_text(d.0, b".item_content_box .item-info-author", &mut author).unwrap_or(b"");
    let author = author.strip_prefix("作者: ".as_bytes()).unwrap_or(author);
    let status =
        child_text(d.0, b".item_content_box .item-info-status", &mut status).unwrap_or(b"");
    let status = if status == "連載中".as_bytes() {
        b"ongoing" as &[u8]
    } else if status == "已完結".as_bytes() {
        b"completed" as &[u8]
    } else {
        b"unknown" as &[u8]
    };
    let out = payload_buf();
    let mut n = 0;
    if !(write_bytes(out,&mut n,br#"{"manga":{"id":"comicabc:manga:"#)&&write_bytes(out,&mut n,id)&&write_bytes(out,&mut n,br#"","title":"#)&&quoted(out,&mut n,title)&&write_bytes(out,&mut n,br#", "cover":{"kind":"url","url":"#)&&quoted(out,&mut n,image)&&write_bytes(out,&mut n,br#"},"description":"#)&&quoted(out,&mut n,description)&&write_bytes(out,&mut n,br#", "authors":["#)&&quoted(out,&mut n,author)&&write_bytes(out,&mut n,br#"],"status":"#)&&quoted(out,&mut n,status)&&write_bytes(out,&mut n,br#", "language":"zh","contentRating":"safe","links":[{"kind":"source","url":"https://www.8comic.com/html/"#)&&write_bytes(out,&mut n,id)&&write_bytes(out,&mut n,b".html\"}]}}")){return source_error(op,"internal_error","overflow");}
    write_success_payload(op, n)
}
fn chapter_value(v: &[u8]) -> Option<(&[u8], Option<u8>, usize)> {
    let part = if v.last()?.is_ascii_lowercase() {
        Some(*v.last()?)
    } else {
        None
    };
    let digits = if part.is_some() { &v[..v.len() - 1] } else { v };
    if !decimal(digits) {
        return None;
    }
    let n = digits.iter().try_fold(0usize, |n, c| {
        n.checked_mul(10)?.checked_add((c - b'0') as usize)
    })?;
    if n > 8051 {
        return None;
    }
    Some((digits, part, n))
}
fn chapter_parts(v: &[u8]) -> Option<(&[u8], &[u8])> {
    let v = v.strip_prefix(b"comicabc:chapter:")?;
    let x = v.iter().position(|c| *c == b':')?;
    let m = &v[..x];
    let c = &v[x + 1..];
    if decimal(m) && chapter_value(c).is_some() {
        Some((m, c))
    } else {
        None
    }
}
// Attr/Text return copied bytes, not total length. Retry saturated reads
// locally in the existing buffer; only an unsaturated read is complete.
fn complete_chapter_field<'a>(
    d: HtmlDescriptor,
    attr: Option<&[u8]>,
    out: &'a mut [u8],
    initial: usize,
) -> Result<&'a [u8], &'static str> {
    let mut cap = initial.min(out.len());
    loop {
        let n = match attr {
            Some(name) => html_attr(d, name, &mut out[..cap]),
            None => html_text(d, &mut out[..cap]),
        }.map_err(|_| "chapter field read failed")?;
        if n < cap {
            return Ok(&out[..n]);
        }
        if cap == out.len() {
            return Err("chapter field cannot be read completely within available buffer");
        }
        cap = cap.checked_mul(2).unwrap_or(out.len()).min(out.len());
    }
}
fn node_chapter<'a>(value: &'a [u8], m: &[u8]) -> Option<&'a [u8]> {
    let data = value.strip_prefix(b"cview('")?;
    let quote = data.iter().position(|c| *c == b'\'')?;
    let data = data[..quote].strip_suffix(b".html")?;
    let dash = data.iter().position(|c| *c == b'-')?;
    if &data[..dash] != m {
        return None;
    }
    let c = &data[dash + 1..];
    chapter_value(c)?;
    Some(c)
}
// Only an explicit decimal in the original visible chapter title supplies a fraction.
// The integer must match the independently validated identity; no suffix-to-number mapping.
fn literal_fraction_number<'a>(title: &'a [u8], integer: &[u8]) -> Option<&'a [u8]> {
    let value = title.strip_prefix("第".as_bytes()).unwrap_or(title);
    let value = value.strip_suffix("話".as_bytes())?;
    let dot = value.iter().position(|byte| *byte == b'.')?;
    let fraction = &value[dot + 1..];
    if &value[..dot] == integer && !fraction.is_empty()
        && fraction.iter().all(|byte| byte.is_ascii_digit()) {
        Some(value)
    } else {
        None
    }
}
static mut CHAPTER_SEEN: [u8; 27176] = [0; 27176];
fn run_get_chapters(r: &[u8]) -> u32 {
    let op = "get_chapters";
    let m = match manga_id(r) {
        Some(x) => x,
        None => return source_error(op, "invalid_request", "invalid manga ID"),
    };
    let d = match detail(m) {
        Ok(x) => x,
        Err(e) => return failure(op, e),
    };
    let nodes = match ChapterSelection::new(d.0) {
        Ok(x) if x.len > 0 => x,
        Ok(_) => return source_error(op, "not_found", "public chapters absent"),
        Err(e) => return source_error(op, "parse_error", e),
    };
    unsafe {
        (*core::ptr::addr_of_mut!(CHAPTER_SEEN)).fill(0);
    }
    let out = payload_buf();
    let mut n = 0;
    let mut count = 0;
    if !write_bytes(out, &mut n, br#"{"items":["#) {
        return source_error(op, "internal_error", "overflow");
    }
    for i in 0..nodes.len {
        let node = match nodes.at(i) {
            Some(node) => node,
            None => return source_error(op, "parse_error", "missing chapter descriptor"),
        };
        // The HTTP envelope has been consumed and Lexbor owns its DOM.
        // Keep onclick identity in HTTP_OUT while title uses BODY_BUF.
        let onclick = match complete_chapter_field(node, Some(b"onclick"), http_out(), 512) {
            Ok(value) => value,
            Err(message) => return source_error(op, "parse_error", message),
        };
        let c = match node_chapter(onclick, m) {
            Some(x) => x,
            None => return source_error(op, "parse_error", "unsupported chapter identity"),
        };
        let (digits, part, cid) = match chapter_value(c) {
            Some(x) => x,
            None => return source_error(op, "parse_error", "invalid chapter identity"),
        };
        let key = cid * 27 + part.map_or(0, |p| (p - b'a' + 1) as usize);
        let bit = 1u8 << (key % 8);
        unsafe {
            let seen = core::ptr::addr_of_mut!(CHAPTER_SEEN);
            if (*seen)[key / 8] & bit != 0 {
                continue;
            }
            (*seen)[key / 8] |= bit;
        }
        let title = match complete_chapter_field(node, None, nodes.title_buffer(), 4096) {
            Ok(value) => value,
            Err(message) => return source_error(op, "parse_error", message),
        };
        let text = trim_ascii(title);
        let badge = b"document.getElementById(";
        let text = if let Some(pos) = text.windows(badge.len()).position(|x| x == badge) {
            trim_ascii(&text[..pos])
        } else {
            text
        };
        if text.is_empty() {
            return source_error(op, "parse_error", "missing chapter title");
        }
        if count > 0 && !write_bytes(out, &mut n, b",") {
            return source_error(op, "internal_error", "overflow");
        }
        if !(write_bytes(out, &mut n, br#"{"id":"comicabc:chapter:"#)
            && write_bytes(out, &mut n, m)
            && write_bytes(out, &mut n, b":")
            && write_bytes(out, &mut n, c)
            && write_bytes(out, &mut n, br#"","mangaId":"comicabc:manga:"#)
            && write_bytes(out, &mut n, m)
            && write_bytes(out, &mut n, br#"","title":"#)
            && quoted(out, &mut n, text)
            && write_bytes(out, &mut n, br#", "sequenceIndex":"#)
            && write_usize(out, &mut n, count)
            && write_bytes(out, &mut n, br#", "chapterNumber":"#)
            && quoted(out, &mut n, literal_fraction_number(text, digits).unwrap_or(digits))
            && write_bytes(out, &mut n, br#", "locked":false}"#))
        {
            return source_error(op, "internal_error", "overflow");
        }
        count += 1;
    }
    if count == 0 || !tail(out, &mut n, false, 1) {
        return source_error(op, "parse_error", "empty or oversized chapters");
    }
    let result = write_success_payload(op, n);
    if result == 0 {
        return source_error(op, "internal_error", "response overflow");
    }
    // Native ReadEnvelope limits the full JSON envelope, not just data.
    let envelope_len = unsafe {
        core::ptr::read_unaligned((result as *const u8).add(8).cast::<u32>())
    };
    if envelope_len as usize > 1024 * 1024 {
        return source_error(op, "source_error", "chapter response exceeds Native envelope capacity");
    }
    result
}
fn run_get_pages(r: &[u8]) -> u32 {
    let op = "get_pages";
    let (m, c) = match extract_json_string(r, b"chapterId").and_then(chapter_parts) {
        Some(x) => x,
        None => return source_error(op, "invalid_request", "invalid chapter ID"),
    };
    let (_, part, chapter) = match chapter_value(c) {
        Some(x) => x,
        None => return source_error(op, "invalid_request", "invalid chapter ID"),
    };
    let comic = match m.iter().try_fold(0usize, |n, c| {
        n.checked_mul(10)?.checked_add((c - b'0') as usize)
    }) {
        Some(x) => x,
        None => return source_error(op, "invalid_request", "invalid manga ID"),
    };
    let mut u = [0; 512];
    let len = match url(&mut u, &[READER, b"/online/new-", m, b".html?ch=", c]) {
        Some(x) => x,
        None => return source_error(op, "invalid_request", "oversized identity"),
    };
    let mut identity = [0; 96];
    let mut ilen = 0;
    if write_bytes(&mut identity, &mut ilen, b"comicabc pages manga=")
        && write_bytes(&mut identity, &mut ilen, m)
        && write_bytes(&mut identity, &mut ilen, b" chapter=")
        && write_bytes(&mut identity, &mut ilen, c)
    {
        koma_source_sdk::host::log_info(&identity[..ilen]);
    }
    let len = match fetch_body(&u[..len], false) {
        Ok(x) => x,
        Err(e) => return failure(op, e),
    };
    let rec = match records::decode(&body_buf()[..len], comic, chapter, part) {
        Some(x) => x,
        None => {
            return source_error(
                op,
                "parse_error",
                "unsupported public literal record schema",
            )
        }
    };
    let out = payload_buf();
    let mut n = 0;
    if !write_bytes(out, &mut n, br#"{"pages":["#) {
        return source_error(op, "internal_error", "overflow");
    }
    for index in 0..rec.page_count {
        let page = index + 1;
        let mm = ((page - 1) / 10) % 10 + ((page - 1) % 10) * 3;
        let mut image = [0; 512];
        let mut ni = 0;
        let ok = write_bytes(&mut image, &mut ni, b"https://img")
            && write_usize(&mut image, &mut ni, rec.server / 10)
            && write_bytes(&mut image, &mut ni, b".8comic.com/")
            && write_usize(&mut image, &mut ni, rec.server % 10)
            && write_bytes(&mut image, &mut ni, b"/")
            && write_bytes(&mut image, &mut ni, m)
            && write_bytes(&mut image, &mut ni, b"/")
            && write_usize(&mut image, &mut ni, chapter)
            && (rec.part == b'0' || write_bytes(&mut image, &mut ni, &[rec.part]))
            && write_bytes(&mut image, &mut ni, b"/")
            && write_bytes(
                &mut image,
                &mut ni,
                if page < 10 {
                    b"00"
                } else if page < 100 {
                    b"0"
                } else {
                    b""
                },
            )
            && write_usize(&mut image, &mut ni, page)
            && write_bytes(&mut image, &mut ni, b"_")
            && write_bytes(&mut image, &mut ni, &rec.codes[mm..mm + 3])
            && write_bytes(&mut image, &mut ni, b".jpg");
        if !ok || !allowed_image(&image[..ni]) {
            return source_error(op, "parse_error", "invalid image schema");
        }
        if index > 0 && !write_bytes(out, &mut n, b",") {
            return source_error(op, "internal_error", "overflow");
        }
        if !(write_bytes(out, &mut n, br#"{"id":"comicabc:page:"#)
            && write_bytes(out, &mut n, m)
            && write_bytes(out, &mut n, b":")
            && write_bytes(out, &mut n, c)
            && write_bytes(out, &mut n, b":")
            && write_usize(out, &mut n, page)
            && write_bytes(out, &mut n, br#"","index":"#)
            && write_usize(out, &mut n, index)
            && write_bytes(out, &mut n, br#", "image":{"kind":"url","url":"#)
            && quoted(out, &mut n, &image[..ni])
            && write_bytes(out, &mut n, b"}}"))
        {
            return source_error(op, "internal_error", "overflow");
        }
    }
    if !write_bytes(out, &mut n, b"]}") {
        return source_error(op, "internal_error", "overflow");
    }
    let result = write_success_payload(op, n);
    if result == 0 {
        return source_error(op, "internal_error", "response overflow");
    }
    let envelope_len = unsafe {
        core::ptr::read_unaligned((result as *const u8).add(8).cast::<u32>())
    };
    if envelope_len as usize > 1024 * 1024 {
        return source_error(op, "source_error", "page response exceeds Native envelope capacity");
    }
    result
}
fn run_get_image_request(r: &[u8]) -> u32 {
    let op = "get_image_request";
    let mut image = [0; 8192];
    let u = match extract_json_string(r, b"url").and_then(|x| decode_string(x, &mut image)) {
        Some(x) if allowed_image(x) => x,
        _ => return source_error(op, "invalid_request", "unsupported image URL"),
    };
    let out = payload_buf();
    let mut n = 0;
    let referer = if u.starts_with(b"https://www.8comic.com/") {
        b"https://www.8comic.com/" as &[u8]
    } else {
        b"https://articles.onemoreplace.tw/"
    };
    if !(write_bytes(out, &mut n, br#"{"url":"#)
        && quoted(out, &mut n, u)
        && write_bytes(out, &mut n, br#", "headers":{"Referer":"#)
        && quoted(out, &mut n, referer)
        && write_bytes(out, &mut n, br#", "User-Agent":"#)
        && quoted(out, &mut n, UA)
        && write_bytes(out, &mut n, b"}}"))
    {
        return source_error(op, "internal_error", "overflow");
    }
    write_success_payload(op, n)
}
fn run_get_listings(_: &[u8]) -> u32 {
    let b=r#"{"listings":[{"id":"popular","name":"熱門","kind":"popular"},{"id":"latest","name":"最新","kind":"latest"}]}"#.as_bytes();
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
koma_source_sdk::koma_source_exports!("comicabc");
