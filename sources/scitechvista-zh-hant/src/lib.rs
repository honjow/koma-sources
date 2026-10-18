#![no_std]
use koma_source_sdk::json_utils::*;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{build_get_request, decode_json_body_into, host};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.scitechvista.zh-hant.koma",
    name: "科技大觀園・科普漫畫大賽",
    version: "0.1.0",
    api_version: "0.2",
    language: "zh-Hant",
    author: "Koma",
    description: "科技大觀園刊載的科普漫畫大賽作品。保留各篇創作者與完整原圖，作品評級尚未確認。",
    content_rating: "unknown",
};
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: false,
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
koma_source_sdk::koma_source_buffers! {payload:1024*1024-1024,http_out:1024*1024,body:1024*1024,http_req:4096,scratch:8192,}
koma_source_sdk::koma_source_helpers!();
type R<T> = Result<T, &'static str>;
fn whitespace(s: &[u8], mut i: usize) -> usize {
    while i < s.len() && s[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}
fn end_value(s: &[u8], start: usize) -> R<usize> {
    parse_value(s, start, 0)
}
fn parse_value(s: &[u8], start: usize, depth: usize) -> R<usize> {
    if depth > 64 {
        return Err("capacity_exceeded");
    }
    let start = whitespace(s, start);
    let first = *s.get(start).ok_or("parse_error")?;
    if first == b'"' {
        let mut i = start + 1;
        while i < s.len() {
            match s[i] {
                b'"' => return Ok(i + 1),
                b'\\' => {
                    i += 1;
                    let e = *s.get(i).ok_or("parse_error")?;
                    if e == b'u' {
                        if i + 5 > s.len() || !s[i + 1..i + 5].iter().all(|b| b.is_ascii_hexdigit())
                        {
                            return Err("parse_error");
                        }
                        i += 4;
                    } else if !matches!(e, b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') {
                        return Err("parse_error");
                    }
                }
                b if b < 32 => return Err("parse_error"),
                _ => {}
            }
            i += 1;
        }
        return Err("parse_error");
    }
    if matches!(first, b'{' | b'[') {
        let end = if first == b'{' { b'}' } else { b']' };
        let mut i = whitespace(s, start + 1);
        if s.get(i) == Some(&end) {
            return Ok(i + 1);
        }
        loop {
            if first == b'{' {
                if s.get(i) != Some(&b'"') {
                    return Err("parse_error");
                }
                i = whitespace(s, parse_value(s, i, depth + 1)?);
                if s.get(i) != Some(&b':') {
                    return Err("parse_error");
                }
                i = whitespace(s, i + 1);
            }
            i = whitespace(s, parse_value(s, i, depth + 1)?);
            match s.get(i) {
                Some(v) if *v == end => return Ok(i + 1),
                Some(b',') => {
                    i = whitespace(s, i + 1);
                    if s.get(i) == Some(&end) {
                        return Err("parse_error");
                    }
                }
                _ => return Err("parse_error"),
            }
        }
    }
    for v in [b"true".as_slice(), b"false".as_slice(), b"null".as_slice()] {
        if s[start..].starts_with(v) {
            return Ok(start + v.len());
        }
    }
    let mut i = start;
    if s.get(i) == Some(&b'-') {
        i += 1;
    }
    match s.get(i) {
        Some(b'0') => {
            i += 1;
            if s.get(i).is_some_and(|b| b.is_ascii_digit()) {
                return Err("parse_error");
            }
        }
        Some(b'1'..=b'9') => {
            i += 1;
            while s.get(i).is_some_and(|b| b.is_ascii_digit()) {
                i += 1;
            }
        }
        _ => return Err("parse_error"),
    }
    if s.get(i) == Some(&b'.') {
        i += 1;
        let at = i;
        while s.get(i).is_some_and(|b| b.is_ascii_digit()) {
            i += 1;
        }
        if at == i {
            return Err("parse_error");
        }
    }
    if matches!(s.get(i), Some(b'e') | Some(b'E')) {
        i += 1;
        if matches!(s.get(i), Some(b'+') | Some(b'-')) {
            i += 1;
        }
        let at = i;
        while s.get(i).is_some_and(|b| b.is_ascii_digit()) {
            i += 1;
        }
        if at == i {
            return Err("parse_error");
        }
    }
    Ok(i)
}
fn direct<'a>(s: &'a [u8], key: &[u8]) -> R<Option<&'a [u8]>> {
    let mut i = whitespace(s, 0);
    if s.get(i) != Some(&b'{') {
        return Err("parse_error");
    }
    i = whitespace(s, i + 1);
    let mut found = None;
    if s.get(i) == Some(&b'}') {
        if whitespace(s, i + 1) != s.len() {
            return Err("parse_error");
        }
        return Ok(None);
    }
    loop {
        if s.get(i) != Some(&b'"') {
            return Err("parse_error");
        }
        let e = end_value(s, i)?;
        let k = &s[i + 1..e - 1];
        if k.contains(&b'\\') {
            return Err("parse_error");
        }
        i = whitespace(s, e);
        if s.get(i) != Some(&b':') {
            return Err("parse_error");
        }
        i = whitespace(s, i + 1);
        let e = end_value(s, i)?;
        if k == key {
            if found.is_some() {
                return Err("duplicate_identity");
            }
            found = Some(&s[i..e]);
        }
        i = whitespace(s, e);
        match s.get(i) {
            Some(b',') => i = whitespace(s, i + 1),
            Some(b'}') => {
                if whitespace(s, i + 1) != s.len() {
                    return Err("parse_error");
                }
                return Ok(found);
            }
            _ => return Err("parse_error"),
        }
    }
}
fn arg_string<'a>(s: &'a [u8], key: &[u8]) -> R<Option<&'a [u8]>> {
    match direct(s, key).map_err(|_| "invalid_request")? {
        None => Ok(None),
        Some(v) if v.len() >= 2 && v[0] == b'"' && v[v.len() - 1] == b'"' => {
            Ok(Some(&v[1..v.len() - 1]))
        }
        _ => Err("invalid_request"),
    }
}

fn member<'a>(v: &'a [u8], k: &[u8]) -> R<&'a [u8]> {
    direct(v, k)?.ok_or("parse_error")
}
fn string<'a>(v: &'a [u8], k: &[u8]) -> R<&'a [u8]> {
    let x = member(v, k)?;
    if x.len() < 2 || x[0] != b'"' || x[x.len() - 1] != b'"' {
        return Err("parse_error");
    };
    Ok(&x[1..x.len() - 1])
}
fn number(v: &[u8]) -> R<usize> {
    if v.is_empty() {
        return Err("parse_error");
    };
    let mut n = 0usize;
    for b in v {
        if !b.is_ascii_digit() {
            return Err("parse_error");
        };
        n = n
            .checked_mul(10)
            .and_then(|n| n.checked_add((b - b'0') as usize))
            .ok_or("parse_error")?;
    }
    Ok(n)
}
fn raw_string(v: &[u8], out: &mut [u8]) -> R<usize> {
    let mut i = 0;
    let mut c = 0;
    while i < v.len() {
        let b = v[i];
        i += 1;
        if b != b'\\' {
            if b < 32 || !write_bytes(out, &mut c, &[b]) {
                return Err("parse_error");
            };
            continue;
        }
        if i == v.len() {
            return Err("parse_error");
        };
        let e = v[i];
        i += 1;
        match e {
            b'"' | b'\\' | b'/' => {
                if !write_bytes(out, &mut c, &[e]) {
                    return Err("capacity_exceeded");
                }
            }
            b'n' | b'r' | b't' | b'b' | b'f' => {
                let b = match e {
                    b'n' => 10,
                    b'r' => 13,
                    b't' => 9,
                    b'b' => 8,
                    _ => 12,
                };
                if !write_bytes(out, &mut c, &[b]) {
                    return Err("capacity_exceeded");
                }
            }
            b'u' => {
                if i + 4 > v.len() || !v[i..i + 4].iter().all(|b| b.is_ascii_hexdigit()) {
                    return Err("parse_error");
                };
                let mut cp = hex_to_u16(&v[i..i + 4]) as u32;
                i += 4;
                if (0xd800..=0xdbff).contains(&cp) {
                    if i + 6 > v.len()
                        || &v[i..i + 2] != b"\\u"
                        || !v[i + 2..i + 6].iter().all(|b| b.is_ascii_hexdigit())
                    {
                        return Err("parse_error");
                    };
                    let low = hex_to_u16(&v[i + 2..i + 6]) as u32;
                    if !(0xdc00..=0xdfff).contains(&low) {
                        return Err("parse_error");
                    };
                    cp = 0x10000 + ((cp - 0xd800) << 10) + (low - 0xdc00);
                    i += 6;
                } else if (0xdc00..=0xdfff).contains(&cp) {
                    return Err("parse_error");
                };
                let mut b = [0; 4];
                let n = encode_utf8(cp, &mut b);
                if !write_bytes(out, &mut c, &b[..n]) {
                    return Err("capacity_exceeded");
                }
            }
            _ => return Err("parse_error"),
        }
    }
    Ok(c)
}
fn q(p: &mut [u8], c: &mut usize, v: &[u8]) -> R<()> {
    if write_bytes(p, c, b"\"") && append_json_escaped(p, c, v) && write_bytes(p, c, b"\"") {
        Ok(())
    } else {
        Err("capacity_exceeded")
    }
}
fn w(p: &mut [u8], c: &mut usize, v: &[u8]) -> R<()> {
    if write_bytes(p, c, v) {
        Ok(())
    } else {
        Err("capacity_exceeded")
    }
}
fn n(p: &mut [u8], c: &mut usize, v: usize) -> R<()> {
    if write_usize(p, c, v) {
        Ok(())
    } else {
        Err("capacity_exceeded")
    }
}

fn result(op: &str, r: R<usize>) -> u32 {
    match r {
        Ok(n) => write_success_payload(op, n),
        Err(e) => write_error(op, e, "Official public contract unavailable or invalid"),
    }
}
fn page(req: &[u8]) -> R<usize> {
    let args = req;
    let v = match direct(args, b"page").map_err(|_| "invalid_request")? {
        None => return Ok(1),
        Some(v) => v,
    };
    let v = if v.first() == Some(&b'{') {
        direct(v, b"cursor")
            .map_err(|_| "invalid_request")?
            .unwrap_or(b"null")
    } else {
        v
    };
    if v == b"null" {
        return Ok(1);
    }
    let v = if v.len() >= 2 && v[0] == b'"' && v[v.len() - 1] == b'"' {
        &v[1..v.len() - 1]
    } else {
        v
    };
    let n = positive(v)?;
    if n == 0 {
        return Err("invalid_request");
    }
    Ok(n)
}

#[link(wasm_import_module = "koma_host")]
extern "C" {
    #[link_name = "html_select_all"]
    fn select_all_raw(desc: i32, sel: *const u8, len: u32, out: *mut u8, cap: u32) -> i32;
}
fn count(doc: host::HtmlDescriptor, sel: &[u8]) -> R<usize> {
    let mut buf = [0; 128];
    let n = unsafe {
        select_all_raw(
            doc.raw(),
            sel.as_ptr(),
            sel.len() as u32,
            buf.as_mut_ptr(),
            buf.len() as u32,
        )
    };
    if n < 0 {
        return Err("html_error");
    }
    for i in 0..core::cmp::min(n as usize, 32) {
        let at = i * 4;
        let d = i32::from_le_bytes([buf[at], buf[at + 1], buf[at + 2], buf[at + 3]]);
        host::html_close(host::HtmlDescriptor::from_raw(d)).map_err(|_| "html_error")?;
    }
    Ok(n as usize)
}
fn text(doc: host::HtmlDescriptor, sel: &[u8], dst: &mut [u8]) -> R<usize> {
    if count(doc, sel)? != 1 {
        return Err("parse_error");
    }
    let node = host::html_select(doc, sel).map_err(|_| "html_error")?;
    let r = host::html_text(node, dst).map_err(|_| "html_error");
    let closed = host::html_close(node).map_err(|_| "html_error");
    closed?;
    r
}
fn attr(doc: host::HtmlDescriptor, sel: &[u8], key: &[u8], dst: &mut [u8]) -> R<usize> {
    // Caller selector requires presence of the same key; Attr never probes absent attributes.
    if count(doc, sel)? != 1 {
        return Err("parse_error");
    }
    let node = host::html_select(doc, sel).map_err(|_| "html_error")?;
    let r = host::html_attr(node, key, dst).map_err(|_| "html_error");
    let closed = host::html_close(node).map_err(|_| "html_error");
    closed?;
    r
}

const BASE: &[u8] = b"https://scitechvista.nat.gov.tw";
const LIST: &[u8] = b"https://scitechvista.nat.gov.tw/Student/Article/C010002/new";
const ROUTE: &[u8] = b"/Article/C010002/detail?ID=";
const PAGE_PATH: &[u8] = b"/Student/Article/C010002/new?&PageIndex=";
const COVER_PATH: &[u8] = b"/UploadPlugin?file=";
const BODY_ROOT: &[u8] = b"/FileDownload/Article/";
const MAX: usize = 30;
const MAX_IMAGES: usize = 64;
static mut IDS: [[u8; 36]; MAX] = [[0; 36]; MAX];
static mut TITLES: [[u8; 512]; MAX] = [[0; 512]; MAX];
static mut TL: [usize; MAX] = [0; MAX];
static mut AUTHORS: [[u8; 256]; MAX] = [[0; 256]; MAX];
static mut AL: [usize; MAX] = [0; MAX];
static mut DIVISIONS: [[u8; 512]; MAX] = [[0; 512]; MAX];
static mut DL: [usize; MAX] = [0; MAX];
static mut DATES: [[u8; 16]; MAX] = [[0; 16]; MAX];
static mut DATE_LEN: [usize; MAX] = [0; MAX];
static mut COVERS: [[u8; 768]; MAX] = [[0; 768]; MAX];
static mut CL: [usize; MAX] = [0; MAX];
static mut USED: usize = 0;
static mut IMAGES: [[u8; 512]; MAX_IMAGES] = [[0; 512]; MAX_IMAGES];
static mut IL: [usize; MAX_IMAGES] = [0; MAX_IMAGES];
static mut IC: usize = 0;
#[derive(Clone, Copy)]
struct Paging {
    current: usize,
    pages: usize,
    total: usize,
    items: usize,
}
fn fetch(u: &[u8]) -> R<usize> {
    let n = build_get_request(http_req_buf(), u, Some(LIST), &[]).ok_or("capacity_exceeded")?;
    let n = host::http_request(&http_req_buf()[..n], http_out()).map_err(|_| "network_error")?;
    if extract_json_number(&http_out()[..n], b"status") != Some(b"200") {
        return Err("http_error");
    }
    decode_json_body_into(&http_out()[..n], body_buf()).map_err(|_| "network_error")
}
fn positive(v: &[u8]) -> R<usize> {
    if v.is_empty() || v[0] == b'0' || v.len() > 10 {
        return Err("invalid_request");
    }
    number(v).map_err(|_| "invalid_request")
}
fn uuid(v: &[u8]) -> R<()> {
    if v.len() != 36 {
        return Err("invalid_request");
    }
    for (i, b) in v.iter().enumerate() {
        if matches!(i, 8 | 13 | 18 | 23) {
            if *b != b'-' {
                return Err("invalid_request");
            }
        } else if !b.is_ascii_digit() && !(b'a'..=b'f').contains(b) {
            return Err("invalid_request");
        }
    }
    Ok(())
}
fn sid<'a>(v: &'a [u8], prefix: &[u8]) -> R<&'a [u8]> {
    let v = v.strip_prefix(prefix).ok_or("invalid_request")?;
    uuid(v)?;
    Ok(v)
}
fn route(p: &mut [u8], c: &mut usize, id: &[u8]) -> R<()> {
    w(p, c, BASE)?;
    w(p, c, ROUTE)?;
    w(p, c, id)
}
fn mid(p: &mut [u8], c: &mut usize, id: &[u8]) -> R<()> {
    pref(p, c, b"scitechvista:zh-hant:comic:", id)
}
fn pref(p: &mut [u8], c: &mut usize, prefix: &[u8], id: &[u8]) -> R<()> {
    w(p, c, prefix)?;
    w(p, c, id)
}
fn owner(req: &[u8], id: &[u8]) -> R<()> {
    if let Some(v) = arg_string(req, b"mangaUrl")? {
        let mut b = [0; 512];
        let n = raw_string(v, &mut b)?;
        let mut u = [0; 256];
        let mut c = 0;
        route(&mut u, &mut c, id)?;
        if b[..n] != u[..c] {
            return Err("invalid_request");
        }
    }
    if let Some(v) = arg_string(req, b"mangaId")? {
        if sid(v, b"scitechvista:zh-hant:comic:")? != id {
            return Err("invalid_request");
        }
    }
    Ok(())
}
fn absolute(v: &[u8], dst: &mut [u8]) -> R<usize> {
    let mut c = 0;
    if v.starts_with(b"/") && !v.starts_with(b"//") {
        w(dst, &mut c, BASE)?;
        w(dst, &mut c, v)?;
    } else if v.starts_with(BASE) && v.get(BASE.len()) == Some(&b'/') {
        w(dst, &mut c, v)?;
    } else {
        return Err("wrong_owner");
    }
    Ok(c)
}
fn cover(v: &[u8]) -> R<()> {
    let v = v
        .strip_prefix(BASE)
        .and_then(|x| x.strip_prefix(COVER_PATH))
        .ok_or("wrong_owner")?;
    if v.is_empty() || v.len() > 640 {
        return Err("wrong_owner");
    }
    let mut i = 0;
    while i < v.len() {
        if v[i].is_ascii_alphanumeric() {
            i += 1;
        } else if v[i] == b'%'
            && i + 3 <= v.len()
            && matches!(&v[i + 1..i + 3], b"2F" | b"2B" | b"3D")
        {
            i += 3;
        } else {
            return Err("wrong_owner");
        }
    }
    Ok(())
}
fn body_uri(v: &[u8]) -> R<()> {
    let v = v
        .strip_prefix(BASE)
        .and_then(|x| x.strip_prefix(BODY_ROOT))
        .ok_or("wrong_owner")?;
    let stem = v
        .strip_suffix(b".jpg")
        .or_else(|| v.strip_suffix(b".JPG"))
        .or_else(|| v.strip_suffix(b".jpeg"))
        .or_else(|| v.strip_suffix(b".png"))
        .or_else(|| v.strip_suffix(b".PNG"))
        .or_else(|| v.strip_suffix(b".gif"))
        .ok_or("wrong_owner")?;
    if stem.is_empty() || stem.len() > 32 || !stem.iter().all(|x| x.is_ascii_digit()) {
        return Err("wrong_owner");
    }
    Ok(())
}
fn date(v: &[u8]) -> R<()> {
    if v.len() > 16 || v.len() < 6 {
        return Err("parse_error");
    }
    let mut slash = 0;
    for b in v {
        if *b == b'/' {
            slash += 1;
        } else if !b.is_ascii_digit() {
            return Err("parse_error");
        }
    }
    if slash != 2 {
        return Err("parse_error");
    }
    Ok(())
}
fn page_path(out: &mut [u8], page: usize) -> R<usize> {
    let mut c = 0;
    w(out, &mut c, PAGE_PATH)?;
    n(out, &mut c, page)?;
    Ok(c)
}
fn page_url(out: &mut [u8], page: usize) -> R<usize> {
    let mut c = 0;
    if page == 1 {
        w(out, &mut c, LIST)?;
    } else {
        w(out, &mut c, BASE)?;
        let mut b = [0; 128];
        let n = page_path(&mut b, page)?;
        w(out, &mut c, &b[..n])?;
    }
    Ok(c)
}
fn next_link(doc: host::HtmlDescriptor, page: usize) -> R<()> {
    let mut p = [0; 128];
    let len = page_path(&mut p, page)?;
    let mut s = [0; 256];
    let mut c = 0;
    w(
        &mut s,
        &mut c,
        b".Stv-MainContent ul.pagination a.page-link[href='",
    )?;
    w(&mut s, &mut c, &p[..len])?;
    w(&mut s, &mut c, b"']")?;
    if count(doc, &s[..c])? == 0 {
        return Err("catalogue_changed");
    }
    Ok(())
}
fn pager(doc: host::HtmlDescriptor) -> R<Paging> {
    let mut b = [0; 256];
    let len = text(doc, b".Stv-MainContent .kf-paginator-date", &mut b)?;
    let v = trim_ascii(&b[..len]);
    let v = v
        .strip_prefix("資料筆數【".as_bytes())
        .ok_or("catalogue_changed")?;
    let at = find_subslice(v, "】頁數【".as_bytes()).ok_or("catalogue_changed")?;
    let total = positive(&v[..at])?;
    let v = v[at + "】頁數【".len()..]
        .strip_suffix("】".as_bytes())
        .ok_or("catalogue_changed")?;
    let at = v
        .iter()
        .position(|x| *x == b'/')
        .ok_or("catalogue_changed")?;
    let current = positive(&v[..at])?;
    let pages = positive(&v[at + 1..])?;
    let items = count(
        doc,
        b".Stv-MainContent > .kf-diagramtext-list > .kf-diagramtext-col",
    )?;
    if total > MAX
        || pages > 6
        || pages != (total + 4) / 5
        || current > pages
        || items != core::cmp::min(5, total - (current - 1) * 5)
    {
        return Err("catalogue_changed");
    }
    Ok(Paging {
        current,
        pages,
        total,
        items,
    })
}
fn catalogue_page(which: usize, offset: usize, require_link: Option<usize>) -> R<Paging> {
    let mut u = [0; 256];
    let len = page_url(&mut u, which)?;
    let size = fetch(&u[..len])?;
    let doc = host::html_parse(&body_buf()[..size]).map_err(|_| "html_error")?;
    let r = (|| {
        let mut b = [0; 2048];
        let l = text(doc, b".contents_wrap > .container-mx > h2.main_tit", &mut b)?;
        if trim_ascii(&b[..l]) != "科普漫畫大賽".as_bytes() {
            return Err("wrong_owner");
        }
        let row = pager(doc)?;
        if row.current != which || offset + row.items > MAX {
            return Err("catalogue_changed");
        }
        if which < row.pages {
            next_link(doc, which + 1)?;
        }
        if let Some(next) = require_link {
            if next <= row.pages {
                next_link(doc, next)?;
            } else {
                return Err("invalid_request");
            }
        }
        for j in 0..row.items {
            let i = offset + j;
            let mut root = [0; 256];
            let mut c = 0;
            w(
                &mut root,
                &mut c,
                b".Stv-MainContent > .kf-diagramtext-list > .kf-diagramtext-col:nth-child(",
            )?;
            n(&mut root, &mut c, j + 1)?;
            w(&mut root, &mut c, b") > a.kf-item")?;
            let prefix = c;
            w(&mut root, &mut c, b"[href]")?;
            let len = attr(doc, &root[..c], b"href", &mut b)?;
            let id = b[..len].strip_prefix(ROUTE).ok_or("wrong_owner")?;
            uuid(id)?;
            for k in 0..i {
                if unsafe { &IDS[k] } == id {
                    return Err("duplicate_identity");
                }
            }
            unsafe {
                IDS[i].copy_from_slice(id);
            }
            c = prefix;
            w(&mut root, &mut c, b"[title]")?;
            let len = attr(doc, &root[..c], b"title", &mut b)?;
            let title = trim_ascii(&b[..len]);
            if title.is_empty() || title.len() > 512 || core::str::from_utf8(title).is_err() {
                return Err("parse_error");
            }
            unsafe {
                TITLES[i][..title.len()].copy_from_slice(title);
                TL[i] = title.len();
            }
            c = prefix;
            w(&mut root, &mut c, b" .kf-title")?;
            let len = text(doc, &root[..c], &mut b)?;
            if trim_ascii(&b[..len]) != unsafe { &TITLES[i][..TL[i]] } {
                return Err("wrong_owner");
            }
            c = prefix;
            w(&mut root, &mut c, b" .Author")?;
            let len = text(doc, &root[..c], &mut b)?;
            let v = trim_ascii(&b[..len]);
            if v.len() > 256 || core::str::from_utf8(v).is_err() {
                return Err("parse_error");
            }
            unsafe {
                AUTHORS[i][..v.len()].copy_from_slice(v);
                AL[i] = v.len();
            }
            c = prefix;
            w(&mut root, &mut c, b" .Division")?;
            let len = text(doc, &root[..c], &mut b)?;
            let v = trim_ascii(&b[..len]);
            if v.len() > 512 || core::str::from_utf8(v).is_err() {
                return Err("parse_error");
            }
            unsafe {
                DIVISIONS[i][..v.len()].copy_from_slice(v);
                DL[i] = v.len();
            }
            c = prefix;
            w(&mut root, &mut c, b" .kf-date > span")?;
            let len = text(doc, &root[..c], &mut b)?;
            let v = trim_ascii(&b[..len]);
            date(v)?;
            unsafe {
                DATES[i][..v.len()].copy_from_slice(v);
                DATE_LEN[i] = v.len();
            }
            c = prefix;
            w(&mut root, &mut c, b" img[src]")?;
            let len = attr(doc, &root[..c], b"src", &mut b)?;
            let mut full = [0; 768];
            let n = absolute(&b[..len], &mut full)?;
            cover(&full[..n])?;
            unsafe {
                COVERS[i][..n].copy_from_slice(&full[..n]);
                CL[i] = n;
            }
        }
        Ok(row)
    })();
    host::html_close(doc).map_err(|_| "html_error")?;
    r
}
fn find(expected: Option<&[u8]>) -> R<Option<usize>> {
    unsafe {
        USED = 0;
    }
    let first = catalogue_page(1, 0, None)?;
    let mut used = first.items;
    for page in 1..=first.pages {
        if page > 1 {
            let row = catalogue_page(page, used, None)?;
            if row.total != first.total || row.pages != first.pages {
                return Err("catalogue_changed");
            }
            used += row.items;
        }
        unsafe {
            USED = used;
        }
        if let Some(id) = expected {
            for i in 0..used {
                if unsafe { &IDS[i] } == id {
                    return Ok(Some(i));
                }
            }
        }
    }
    if expected.is_some() {
        Err("not_found")
    } else {
        Ok(None)
    }
}
fn current_images(i: usize) -> R<()> {
    unsafe {
        IC = 0;
    }
    let mut u = [0; 256];
    let mut c = 0;
    route(&mut u, &mut c, unsafe { &IDS[i] })?;
    let size = fetch(&u[..c])?;
    let doc = host::html_parse(&body_buf()[..size]).map_err(|_| "html_error")?;
    let r = (|| {
        let mut b = [0; 2048];
        for (sel, expected) in [
            (
                b"#block > .DetailContent > h2.kf-det-title".as_slice(),
                unsafe { &TITLES[i][..TL[i]] },
            ),
            (
                b"#block > .DetailContent .Article-AuthorRow > .Author",
                unsafe { &AUTHORS[i][..AL[i]] },
            ),
            (
                b"#block > .DetailContent .Article-AuthorRow > .Division",
                unsafe { &DIVISIONS[i][..DL[i]] },
            ),
            (b"#block > .DetailContent .kf-date", unsafe {
                &DATES[i][..DATE_LEN[i]]
            }),
        ] {
            let len = text(doc, sel, &mut b)?;
            if trim_ascii(&b[..len]) != expected {
                return Err("wrong_owner");
            }
        }
        let len = attr(
            doc,
            b"meta[property='og:title'][content]",
            b"content",
            &mut b,
        )?;
        if &b[..len] != unsafe { &TITLES[i][..TL[i]] } {
            return Err("wrong_owner");
        }
        // JSON-LD carries the canonical owner while body scope excludes every recommendation.
        let len = text(doc, b"head > script[type='application/ld+json']", &mut b)?;
        let ld = &b[..len];
        let parent = direct(ld, b"mainEntityOfPage")?.ok_or("wrong_owner")?;
        let raw = arg_string(parent, b"@id")?.ok_or("wrong_owner")?;
        let mut actual = [0; 512];
        let n = raw_string(raw, &mut actual)?;
        if actual[..n] != u[..c] {
            return Err("wrong_owner");
        }
        let sel = b"#block > .DetailContent > span.templateLongText.kf-det-content";
        if count(doc, sel)? != 1 {
            return Err("incomplete_pages");
        }
        let all = b"#block > .DetailContent > span.templateLongText.kf-det-content img";
        let src = b"#block > .DetailContent > span.templateLongText.kf-det-content img[src]";
        let total = count(doc, all)?;
        if total == 0 || total > MAX_IMAGES || count(doc, src)? != total {
            return Err("incomplete_pages");
        }
        let mut descriptors = [0; MAX_IMAGES * 4];
        let found = unsafe {
            select_all_raw(
                doc.raw(),
                src.as_ptr(),
                src.len() as u32,
                descriptors.as_mut_ptr(),
                descriptors.len() as u32,
            )
        };
        if found != total as i32 {
            return Err("html_error");
        }
        let mut result = Ok(());
        for j in 0..total {
            let at = j * 4;
            let node = host::HtmlDescriptor::from_raw(i32::from_le_bytes([
                descriptors[at],
                descriptors[at + 1],
                descriptors[at + 2],
                descriptors[at + 3],
            ]));
            let item = (|| {
                let len = host::html_attr(node, b"src", &mut b).map_err(|_| "html_error")?;
                let mut full = [0; 512];
                let n = absolute(&b[..len], &mut full)?;
                body_uri(&full[..n])?;
                for k in 0..j {
                    if unsafe { &IMAGES[k][..IL[k]] } == &full[..n] {
                        return Err("duplicate_identity");
                    }
                }
                unsafe {
                    IMAGES[j][..n].copy_from_slice(&full[..n]);
                    IL[j] = n;
                }
                Ok(())
            })();
            let closed = host::html_close(node).map_err(|_| "html_error");
            if result.is_ok() {
                result = item.and(closed);
            }
        }
        result?;
        unsafe {
            IC = total;
        }
        Ok(())
    })();
    host::html_close(doc).map_err(|_| "html_error")?;
    r
}
fn book(p: &mut [u8], c: &mut usize, i: usize, detail: bool) -> R<()> {
    w(p, c, b"{\"id\":\"")?;
    mid(p, c, unsafe { &IDS[i] })?;
    w(p, c, b"\",\"title\":")?;
    q(p, c, unsafe { &TITLES[i][..TL[i]] })?;
    w(p, c, b",\"url\":\"")?;
    route(p, c, unsafe { &IDS[i] })?;
    w(p, c, b"\",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(p, c, unsafe { &COVERS[i][..CL[i]] })?;
    w(p, c, b"},\"language\":\"zh-Hant\",\"authors\":[")?;
    if unsafe { AL[i] } > 0 {
        q(p, c, unsafe { &AUTHORS[i][..AL[i]] })?;
    }
    w(
        p,
        c,
        b"],\"artists\":[],\"status\":\"unknown\",\"contentRating\":\"unknown\",\"description\":",
    )?;
    let mut desc = [0; 2048];
    let mut d = 0;
    w(
        &mut desc,
        &mut d,
        "科技大觀園刊載的科普漫畫大賽作品。\n刊登日期：".as_bytes(),
    )?;
    w(&mut desc, &mut d, unsafe { &DATES[i][..DATE_LEN[i]] })?;
    if unsafe { DL[i] } > 0 {
        w(&mut desc, &mut d, "\n學校／組別：".as_bytes())?;
        w(&mut desc, &mut d, unsafe { &DIVISIONS[i][..DL[i]] })?;
    }
    q(p, c, &desc[..d])?;
    if detail {
        w(p, c, b",\"links\":[{\"kind\":\"source\",\"url\":\"")?;
        route(p, c, unsafe { &IDS[i] })?;
        w(p, c, b"\"}]")?;
    }
    w(p, c, b"}")
}
fn listing(which: usize) -> R<usize> {
    let first = catalogue_page(1, 0, if which > 1 { Some(which) } else { None })?;
    let row = if which == 1 {
        first
    } else {
        let row = catalogue_page(which, first.items, None)?;
        if row.pages != first.pages || row.total != first.total {
            return Err("catalogue_changed");
        }
        row
    };
    let start = if which == 1 { 0 } else { first.items };
    let p = payload_buf();
    let mut c = 0;
    w(p, &mut c, b"{\"items\":[")?;
    for i in start..start + row.items {
        if i > start {
            w(p, &mut c, b",")?;
        }
        book(p, &mut c, i, false)?;
    }
    w(p, &mut c, b"],\"page\":{\"nextCursor\":")?;
    if which < row.pages {
        w(p, &mut c, b"\"")?;
        n(p, &mut c, which + 1)?;
        w(p, &mut c, b"\",\"hasMore\":true},\"hasNextPage\":true}")?;
    } else {
        w(p, &mut c, b"null,\"hasMore\":false},\"hasNextPage\":false}")?;
    }
    Ok(c)
}
fn run_search(_req: &[u8]) -> u32 {
    write_error("search", "not_implemented", "此來源不提供搜尋")
}
fn run_get_manga_list(req: &[u8]) -> u32 {
    if arg_string(req, b"listingId").ok().flatten() != Some(b"listing:comics") {
        return write_error("get_manga_list", "invalid_request", "");
    }
    match page(req) {
        Ok(n) if n <= 6 => result("get_manga_list", listing(n)),
        _ => write_error("get_manga_list", "invalid_request", ""),
    }
}
fn request_work(req: &[u8], key: &[u8], prefix: &[u8]) -> R<usize> {
    let id = sid(arg_string(req, key)?.ok_or("invalid_request")?, prefix)?;
    owner(req, id)?;
    find(Some(id))?.ok_or("not_found")
}
fn run_get_manga(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let i = request_work(req, b"mangaId", b"scitechvista:zh-hant:comic:")?;
        current_images(i)?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"manga\":")?;
        book(p, &mut c, i, true)?;
        w(p, &mut c, b"}")?;
        Ok(c)
    }
    result("get_manga", inner(req))
}
fn run_get_chapters(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let i = request_work(req, b"mangaId", b"scitechvista:zh-hant:comic:")?;
        current_images(i)?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"chapters\":[{\"id\":\"")?;
        pref(p, &mut c, b"scitechvistaread:zh-hant:comic:", unsafe {
            &IDS[i]
        })?;
        w(p, &mut c, b"\",\"mangaId\":\"")?;
        mid(p, &mut c, unsafe { &IDS[i] })?;
        w(p, &mut c, b"\",\"title\":")?;
        q(p, &mut c, unsafe { &TITLES[i][..TL[i]] })?;
        w(p, &mut c, b",\"url\":\"")?;
        route(p, &mut c, unsafe { &IDS[i] })?;
        w(
            p,
            &mut c,
            b"\",\"chapterNumber\":null,\"volumeNumber\":null,\"readingGroup\":\"",
        )?;
        mid(p, &mut c, unsafe { &IDS[i] })?;
        w(
            p,
            &mut c,
            b"\",\"continuationEligible\":false,\"language\":\"zh-Hant\",\"pageCount\":",
        )?;
        n(p, &mut c, unsafe { IC })?;
        w(p, &mut c, b"}]}")?;
        Ok(c)
    }
    result("get_chapters", inner(req))
}
fn run_get_pages(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let i = request_work(req, b"chapterId", b"scitechvistaread:zh-hant:comic:")?;
        current_images(i)?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"pages\":[")?;
        for j in 0..unsafe { IC } {
            if j > 0 {
                w(p, &mut c, b",")?;
            }
            w(p, &mut c, b"{\"id\":\"")?;
            pref(p, &mut c, b"scitechvistapage:zh-hant:comic:", unsafe {
                &IDS[i]
            })?;
            w(p, &mut c, b":")?;
            n(p, &mut c, j)?;
            w(p, &mut c, b"\",\"index\":")?;
            n(p, &mut c, j)?;
            w(p, &mut c, b",\"image\":{\"kind\":\"url\",\"url\":")?;
            q(p, &mut c, unsafe { &IMAGES[j][..IL[j]] })?;
            w(p, &mut c, b"}}")?;
        }
        w(p, &mut c, b"]}")?;
        Ok(c)
    }
    result("get_pages", inner(req))
}
fn run_get_image_request(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let mut url = [0; 1024];
        let len = raw_string(arg_string(req, b"url")?.ok_or("invalid_request")?, &mut url)?;
        let url = &url[..len];
        let mut id = None;
        let mut ordinal = None;
        if let Some(v) = arg_string(req, b"pageId")? {
            if let Some(v) = v.strip_prefix(b"scitechvistapage:zh-hant:comic:") {
                if v.len() < 38 || v[36] != b':' {
                    return Err("invalid_request");
                }
                uuid(&v[..36])?;
                let x = &v[37..];
                let at = if x == b"0" { 0 } else { positive(x)? };
                if at >= MAX_IMAGES {
                    return Err("invalid_request");
                }
                id = Some(&v[..36]);
                ordinal = Some(at);
            } else {
                id = Some(sid(v, b"scitechvista:zh-hant:comic:")?);
            }
        }
        if let Some(v) = id {
            owner(req, v)?;
        }
        let i = if let Some(v) = id {
            find(Some(v))?.ok_or("not_found")?
        } else {
            cover(url).map_err(|_| "invalid_request")?;
            find(None)?;
            let mut selected = None;
            for i in 0..unsafe { USED } {
                if url == unsafe { &COVERS[i][..CL[i]] } {
                    if selected.is_some() {
                        return Err("invalid_request");
                    }
                    selected = Some(i);
                }
            }
            selected.ok_or("invalid_request")?
        };
        owner(req, unsafe { &IDS[i] })?;
        if let Some(at) = ordinal {
            body_uri(url).map_err(|_| "invalid_request")?;
            current_images(i)?;
            if at >= unsafe { IC } || url != unsafe { &IMAGES[at][..IL[at]] } {
                return Err("invalid_request");
            }
        } else if url != unsafe { &COVERS[i][..CL[i]] } {
            return Err("invalid_request");
        }
        let mut referer = [0; 256];
        let mut n = 0;
        route(&mut referer, &mut n, unsafe { &IDS[i] })?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"imageRequest\":{\"url\":")?;
        q(p, &mut c, url)?;
        w(p, &mut c, b",\"headers\":{\"Referer\":")?;
        q(p, &mut c, &referer[..n])?;
        w(p, &mut c, b"}}}")?;
        Ok(c)
    }
    result("get_image_request", inner(req))
}
fn run_get_listings(_req: &[u8]) -> u32 {
    let b = "{\"listings\":[{\"id\":\"listing:comics\",\"name\":\"科普漫畫大賽作品\"}]}".as_bytes();
    payload_buf()[..b.len()].copy_from_slice(b);
    write_success_payload("get_listings", b.len())
}
fn run_get_home(_req: &[u8]) -> u32 {
    write_error("get_home", "not_implemented", "")
}
fn run_get_filters(_req: &[u8]) -> u32 {
    write_error("get_filters", "not_implemented", "")
}
fn run_get_settings(_req: &[u8]) -> u32 {
    write_error("get_settings", "not_implemented", "")
}
const REQUEST_CAP: usize = 1024 * 1024;
static mut REQUEST: [u8; REQUEST_CAP] = [0; REQUEST_CAP];
fn checked_request(ptr: u32, len: u32) -> Option<&'static [u8]> {
    let cap = core::arch::wasm32::memory_size::<0>().checked_mul(65536)?;
    let start = ptr as usize;
    let end = start.checked_add(len as usize)?;
    if start == 0 || len == 0 || len as usize >= REQUEST_CAP || end > cap {
        return None;
    }
    read_request(ptr, len)
}
#[no_mangle]
pub extern "C" fn koma_source_alloc(len: u32) -> u32 {
    if len == 0 || len as usize >= REQUEST_CAP {
        0
    } else {
        core::ptr::addr_of_mut!(REQUEST) as *mut u8 as u32
    }
}
#[no_mangle]
pub extern "C" fn koma_source_init(ptr: u32, len: u32) -> i32 {
    if checked_request(ptr, len).is_some() {
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
pub extern "C" fn koma_source_free(ptr: u32) {
    response_buffer().free(ptr)
}
macro_rules! export_op {
    ($f:ident,$op:literal,$run:ident) => {
        #[no_mangle]
        pub extern "C" fn $f(ptr: u32, len: u32) -> u32 {
            let req = match checked_request(ptr, len) {
                Some(r) => r,
                None => return write_error($op, "invalid_request", "invalid ABI range"),
            };
            if host::check_cancel() {
                return write_error($op, "cancelled", "");
            }
            let args = match direct(req, b"args") {
                Ok(Some(a)) if a.first() == Some(&b'{') => a,
                _ => return write_error($op, "invalid_request", "missing args"),
            };
            if string(req, b"operation").ok() != Some($op.as_bytes()) {
                return write_error($op, "invalid_request", "operation mismatch");
            }
            $run(args)
        }
    };
}
export_op!(koma_source_search, "search", run_search);
export_op!(koma_source_get_manga, "get_manga", run_get_manga);
export_op!(koma_source_get_chapters, "get_chapters", run_get_chapters);
export_op!(koma_source_get_pages, "get_pages", run_get_pages);
export_op!(koma_source_get_listings, "get_listings", run_get_listings);
export_op!(
    koma_source_get_manga_list,
    "get_manga_list",
    run_get_manga_list
);
export_op!(koma_source_get_home, "get_home", run_get_home);
export_op!(koma_source_get_filters, "get_filters", run_get_filters);
export_op!(koma_source_get_settings, "get_settings", run_get_settings);
export_op!(
    koma_source_get_image_request,
    "get_image_request",
    run_get_image_request
);
