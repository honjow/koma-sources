#![no_std]
use koma_source_sdk::json_utils::*;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{build_get_request, decode_json_body_into, host};
const SOURCE_INFO:SourceInfo=SourceInfo{
 id:"com.mixflavor.life.zh-hant.koma",name:"綜合口味・生活小漫畫",version:"0.1.0",api_version:"0.2",language:"zh-Hant",author:"Koma",
 description:"綜合口味作者網站的生活小漫畫，依原分類分頁瀏覽，不收錄同時標為開箱心得或食的文章。每篇獨立閱讀，作品評級尚未確認。",content_rating:"unknown",
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
koma_source_sdk::koma_source_buffers! {payload:1024*1024-1024,http_out:2*1024*1024,body:1024*1024,http_req:4096,scratch:65536,}
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
#[link(wasm_import_module = "koma_host")]
extern "C" {
    #[link_name = "html_select_all"]
    fn select_all_raw(desc: i32, sel: *const u8, len: u32, out: *mut u8, cap: u32) -> i32;
}
fn close_raw(raw: &[u8], count: i32) {
    for i in 0..core::cmp::min(core::cmp::max(count, 0) as usize, raw.len() / 4) {
        let a = i * 4;
        let node = host::HtmlDescriptor::from_raw(i32::from_le_bytes([
            raw[a],
            raw[a + 1],
            raw[a + 2],
            raw[a + 3],
        ]));
        let _ = host::html_close(node);
    }
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
const ORIGIN: &[u8] = b"https://blog.mixflavor.com";
const CATEGORY: &[u8] =
    b"https://blog.mixflavor.com/search/label/%e7%94%9f%e6%b4%bb%e5%b0%8f%e6%bc%ab%e7%95%ab";
const REVIEW: &[u8] =
    b"https://blog.mixflavor.com/search/label/%e9%96%8b%e7%ae%b1%e5%bf%83%e5%be%97";
const FOOD: &[u8] = b"https://blog.mixflavor.com/search/label/%e9%a3%9f";
const AUTHOR: &[u8] = b"https://blog.mixflavor.com/author/mixflavor";
const AUTHOR_ID: &[u8] = b"https://blog.mixflavor.com/author/mixflavor#author";
const MEDIA: &[u8] = b"https://i0.wp.com/img.mixflavor.com/";
const UPLOAD: &[u8] = b"https://i0.wp.com/blog.mixflavor.com/wp-content/uploads/";
const MID: &[u8] = b"mixflavor:zh-hant:post:";
const CID: &[u8] = b"mixflavorread:zh-hant:post:";
const PID: &[u8] = b"mixflavorpage:zh-hant:post:";
const CARDS:&[u8]=b"main#eskimo-main-container > div.container > div.eskimo-masonry-grid > div.eskimo-two-columns > div.post";
const BODY:&[u8]=b"main#eskimo-main-container > div.container > div.eskimo-page-content > div[itemprop='articleBody']";
const MAX_IMAGES: usize = 64;
static mut POST: [u8; 24] = [0; 24];
static mut PL: usize = 0;
static mut PATH: [u8; 256] = [0; 256];
static mut PATH_LEN: usize = 0;
static mut TITLE: [u8; 1024] = [0; 1024];
static mut TL: usize = 0;
static mut COVER: [u8; 1024] = [0; 1024];
static mut CL: usize = 0;
static mut IMAGES: [[u8; 1024]; MAX_IMAGES] = [[0; 1024]; MAX_IMAGES];
static mut IL: [usize; MAX_IMAGES] = [0; MAX_IMAGES];
static mut IC: usize = 0;
static mut NOTE: [u8; 16384] = [0; 16384];
static mut NOTE_LEN: usize = 0;
static mut DATE: [u8; 64] = [0; 64];
static mut DL: usize = 0;
static mut NEXT: [u8; 512] = [0; 512];
static mut NL: usize = 0;
static mut TAGS: [[u8; 256]; 16] = [[0; 256]; 16];
static mut TAG_LEN: [usize; 16] = [0; 16];
static mut TC: usize = 0;
fn digits(v: &[u8]) -> bool {
    !v.is_empty() && v.len() <= 24 && v[0] != b'0' && v.iter().all(|b| b.is_ascii_digit())
}
fn positive(v: &[u8]) -> R<usize> {
    if !digits(v) {
        return Err("invalid_request");
    }
    let n = number(v).map_err(|_| "invalid_request")?;
    if n == 0 || n > 16384 {
        return Err("invalid_request");
    }
    Ok(n)
}
fn hex_digit(v: u8) -> Option<u8> {
    match v {
        b'0'..=b'9' => Some(v - b'0'),
        b'a'..=b'f' => Some(v - b'a' + 10),
        b'A'..=b'F' => Some(v - b'A' + 10),
        _ => None,
    }
}
fn valid_path(path: &[u8]) -> R<()> {
    if path.len() < 14
        || path.len() >= 256
        || path[0] != b'/'
        || path.get(5) != Some(&b'/')
        || path.get(8) != Some(&b'/')
        || !path[1..5].iter().all(|b| b.is_ascii_digit())
        || !path[6..8].iter().all(|b| b.is_ascii_digit())
    {
        return Err("invalid_request");
    }
    let name = path[9..].strip_suffix(b".html").ok_or("invalid_request")?;
    if name.is_empty() || name.windows(2).any(|x| x == b"..") {
        return Err("invalid_request");
    }
    let mut i = 0;
    while i < name.len() {
        let b = name[i];
        if b == b'%' {
            let a =
                hex_digit(*name.get(i + 1).ok_or("invalid_request")?).ok_or("invalid_request")?;
            let c =
                hex_digit(*name.get(i + 2).ok_or("invalid_request")?).ok_or("invalid_request")?;
            let decoded = a * 16 + c;
            if !(decoded >= 128 || decoded.is_ascii_alphanumeric() || b"_-".contains(&decoded)) {
                return Err("invalid_request");
            }
            i += 3;
        } else if b.is_ascii_alphanumeric() || b"_-".contains(&b) {
            i += 1;
        } else {
            return Err("invalid_request");
        }
    }
    Ok(())
}
fn set_post(post: &[u8]) -> R<()> {
    if !digits(post) {
        return Err("invalid_request");
    }
    unsafe {
        POST[..post.len()].copy_from_slice(post);
        PL = post.len();
    }
    Ok(())
}
fn set_uri(uri: &[u8]) -> R<()> {
    let path = uri.strip_prefix(ORIGIN).ok_or("invalid_request")?;
    valid_path(path)?;
    unsafe {
        PATH[..path.len()].copy_from_slice(path);
        PATH_LEN = path.len();
    }
    Ok(())
}
fn uri(out: &mut [u8]) -> R<usize> {
    let mut c = 0;
    w(out, &mut c, ORIGIN)?;
    w(out, &mut c, unsafe { &PATH[..PATH_LEN] })?;
    Ok(c)
}
fn ident(p: &mut [u8], c: &mut usize, prefix: &[u8]) -> R<()> {
    w(p, c, prefix)?;
    w(p, c, unsafe { &POST[..PL] })?;
    w(p, c, b":")?;
    for b in unsafe { &PATH[..PATH_LEN] } {
        w(
            p,
            c,
            &[
                b"0123456789abcdef"[(b >> 4) as usize],
                b"0123456789abcdef"[(b & 15) as usize],
            ],
        )?;
    }
    Ok(())
}
fn parse_identity<'a>(v: &'a [u8], prefix: &[u8]) -> R<&'a [u8]> {
    let rest = v.strip_prefix(prefix).ok_or("invalid_request")?;
    let at = rest
        .iter()
        .position(|b| *b == b':')
        .ok_or("invalid_request")?;
    set_post(&rest[..at])?;
    let h = &rest[at + 1..];
    let end = h.iter().position(|b| *b == b':').unwrap_or(h.len());
    let hex = &h[..end];
    if hex.is_empty() || hex.len() % 2 != 0 || hex.len() / 2 >= 256 {
        return Err("invalid_request");
    }
    let mut path = [0; 256];
    for i in 0..hex.len() / 2 {
        let a = hex_digit(hex[i * 2]).ok_or("invalid_request")?;
        let b = hex_digit(hex[i * 2 + 1]).ok_or("invalid_request")?;
        if hex[i * 2].is_ascii_uppercase() || hex[i * 2 + 1].is_ascii_uppercase() {
            return Err("invalid_request");
        }
        path[i] = a * 16 + b;
    }
    valid_path(&path[..hex.len() / 2])?;
    unsafe {
        PATH[..hex.len() / 2].copy_from_slice(&path[..hex.len() / 2]);
        PATH_LEN = hex.len() / 2;
    }
    Ok(&h[end..])
}
fn owner(req: &[u8]) -> R<()> {
    let mut u = [0; 512];
    let n = uri(&mut u)?;
    if let Some(v) = arg_string(req, b"mangaUrl")? {
        if v != &u[..n] {
            return Err("invalid_request");
        }
    }
    if let Some(v) = arg_string(req, b"mangaId")? {
        let mut b = [0; 1024];
        let mut c = 0;
        ident(&mut b, &mut c, MID)?;
        if v != &b[..c] {
            return Err("invalid_request");
        }
    }
    Ok(())
}
fn request_work(req: &[u8], key: &[u8], prefix: &[u8]) -> R<()> {
    if !parse_identity(arg_string(req, key)?.ok_or("invalid_request")?, prefix)?.is_empty() {
        return Err("invalid_request");
    }
    owner(req)
}
fn page_num(url: &[u8]) -> R<usize> {
    let suffix = url
        .strip_prefix(CATEGORY)
        .and_then(|x| x.strip_prefix(b"/page/"))
        .ok_or("invalid_request")?;
    positive(suffix)
}
fn list_route(req: &[u8], out: &mut [u8]) -> R<(usize, usize)> {
    let p = direct(req, b"page").map_err(|_| "invalid_request")?;
    let cursor = match p {
        None => None,
        Some(p) if p.first() == Some(&b'{') => {
            direct(p, b"cursor").map_err(|_| "invalid_request")?
        }
        _ => return Err("invalid_request"),
    };
    match cursor {
        None | Some(b"null") => {
            out[..CATEGORY.len()].copy_from_slice(CATEGORY);
            Ok((CATEGORY.len(), 1))
        }
        Some(v) => {
            if v.len() < 2 || v[0] != b'"' || v[v.len() - 1] != b'"' {
                return Err("invalid_request");
            }
            let u = &v[1..v.len() - 1];
            let page = page_num(u)?;
            if page < 2 || u.len() >= out.len() {
                return Err("invalid_request");
            }
            out[..u.len()].copy_from_slice(u);
            Ok((u.len(), page))
        }
    }
}
fn fetch(url: &[u8]) -> R<usize> {
    let n = build_get_request(http_req_buf(), url, Some(url), &[]).ok_or("capacity_exceeded")?;
    let n = host::http_request(&http_req_buf()[..n], http_out()).map_err(|_| "network_error")?;
    if extract_json_number(&http_out()[..n], b"status") != Some(b"200") {
        return Err("http_error");
    }
    let n = decode_json_body_into(&http_out()[..n], body_buf()).map_err(|_| "network_error")?;
    if n == 0 || n >= 1024 * 1024 {
        return Err("capacity_exceeded");
    }
    Ok(n)
}
fn eq_attr(doc: host::HtmlDescriptor, s: &[u8], k: &[u8], v: &[u8]) -> R<()> {
    let mut b = [0; 1024];
    let n = attr(doc, s, k, &mut b)?;
    if n >= 1024 || &b[..n] != v {
        return Err("identity_mismatch");
    }
    Ok(())
}
fn json_eq(obj: &[u8], key: &[u8], value: &[u8]) -> R<()> {
    let mut b = [0; 2048];
    let n = raw_string(string(obj, key)?, &mut b)?;
    if n >= b.len() || &b[..n] != value {
        return Err("identity_mismatch");
    }
    Ok(())
}
fn with_suffix(uri: &[u8], suffix: &[u8], out: &mut [u8]) -> R<usize> {
    let mut c = 0;
    w(out, &mut c, uri)?;
    w(out, &mut c, suffix)?;
    Ok(c)
}
fn post_token(v: &[u8], prefix: &[u8], out: &mut [u8]) -> R<usize> {
    let mut found = 0;
    let mut len = 0;
    for token in v
        .split(|b| b.is_ascii_whitespace())
        .filter(|x| !x.is_empty())
    {
        if let Some(x) = token.strip_prefix(prefix) {
            if !digits(x) || x.len() > out.len() {
                return Err("identity_mismatch");
            }
            out[..x.len()].copy_from_slice(x);
            len = x.len();
            found += 1;
        }
    }
    if found != 1 {
        return Err("identity_mismatch");
    }
    Ok(len)
}
fn scope(doc: host::HtmlDescriptor, selector: &[u8], all: &[u8]) -> R<bool> {
    let mut raw = [0; 16 * 4];
    let total = unsafe {
        select_all_raw(
            doc.raw(),
            selector.as_ptr(),
            selector.len() as u32,
            raw.as_mut_ptr(),
            raw.len() as u32,
        )
    };
    if total < 1 || total > 16 {
        close_raw(&raw, total);
        return Err("parse_error");
    }
    if count(doc, all) != Ok(total as usize) {
        close_raw(&raw, total);
        return Err("parse_error");
    }
    let mut original_uris = [[0; 512]; 16];
    let mut lens = [0; 16];
    let mut life = 0;
    let mut excluded = false;
    let mut error = None;
    for i in 0..total as usize {
        let a = i * 4;
        let node = host::HtmlDescriptor::from_raw(i32::from_le_bytes([
            raw[a],
            raw[a + 1],
            raw[a + 2],
            raw[a + 3],
        ]));
        let r = (|| {
            let n =
                host::html_attr(node, b"href", &mut original_uris[i]).map_err(|_| "html_error")?;
            if n == 0 || n >= 512 {
                return Err("capacity_exceeded");
            }
            let u = &original_uris[i][..n];
            if !u.starts_with(b"https://blog.mixflavor.com/search/label/")
                || u.contains(&b'?')
                || u.contains(&b'#')
            {
                return Err("identity_mismatch");
            }
            for j in 0..i {
                if lens[j] == n && &original_uris[j][..n] == u {
                    return Err("duplicate_identity");
                }
            }
            lens[i] = n;
            if u == CATEGORY {
                life += 1;
            }
            if u == REVIEW || u == FOOD {
                excluded = true;
            }
            let l = host::html_text(node, unsafe { &mut TAGS[i] }).map_err(|_| "html_error")?;
            if l == 0 || l >= 256 {
                return Err("parse_error");
            }
            unsafe {
                TAG_LEN[i] = l;
            }
            Ok(())
        })();
        let close = host::html_close(node).map_err(|_| "html_error");
        if let Err(e) = r.and(close) {
            if error.is_none() {
                error = Some(e);
            }
        }
    }
    if let Some(e) = error {
        return Err(e);
    }
    if life != 1 {
        return Err("out_of_scope");
    }
    unsafe {
        TC = total as usize;
    }
    Ok(!excluded)
}
fn media_path<'a>(u: &'a [u8], allow_upload: bool) -> R<&'a [u8]> {
    let rest = u
        .strip_prefix(MEDIA)
        .or_else(|| {
            if allow_upload {
                u.strip_prefix(UPLOAD)
            } else {
                None
            }
        })
        .ok_or("invalid_request")?;
    let at = rest
        .iter()
        .position(|b| *b == b'?')
        .ok_or("invalid_request")?;
    let path = &rest[..at];
    let query = &rest[at + 1..];
    if path.len() < 20
        || path.get(4) != Some(&b'/')
        || path.get(7) != Some(&b'/')
        || !path[..4].iter().all(|b| b.is_ascii_digit())
        || !path[5..7].iter().all(|b| b.is_ascii_digit())
    {
        return Err("invalid_request");
    }
    let file = &path[8..];
    let stem = file
        .strip_suffix(b".png")
        .or_else(|| file.strip_suffix(b".jpg"))
        .or_else(|| file.strip_suffix(b".jpeg"))
        .ok_or("invalid_request")?;
    let sep = stem
        .iter()
        .position(|b| *b == b'-')
        .ok_or("invalid_request")?;
    if sep != 10
        || !stem[..sep].iter().all(|b| b.is_ascii_digit())
        || stem.len() - sep - 1 < 16
        || stem.len() - sep - 1 > 64
        || !stem[sep + 1..].iter().all(|b| b.is_ascii_hexdigit())
    {
        return Err("invalid_request");
    }
    let value = query.strip_suffix(b"&ssl=1").ok_or("invalid_request")?;
    if let Some(v) = value.strip_prefix(b"w=") {
        positive(v)?;
    } else {
        let v = value
            .strip_prefix(b"fit=")
            .or_else(|| value.strip_prefix(b"resize="))
            .ok_or("invalid_request")?;
        let at = find_subslice(v, b"%2C")
            .or_else(|| find_subslice(v, b"%2c"))
            .ok_or("invalid_request")?;
        positive(&v[..at])?;
        positive(&v[at + 3..])?;
    }
    Ok(path)
}
fn selected_srcset(src: &[u8], srcset: &[u8], out: &mut [u8]) -> R<usize> {
    let original = media_path(src, true).map_err(|_| "parse_error")?;
    let mut largest = 0;
    let mut out_len = 0;
    let mut widths = [0usize; 16];
    let mut total = 0;
    for candidate in srcset.split(|b| *b == b',') {
        let candidate = trim_ascii(candidate);
        let at = candidate
            .iter()
            .position(|b| b.is_ascii_whitespace())
            .ok_or("parse_error")?;
        let u = &candidate[..at];
        let d = trim_ascii(&candidate[at..]);
        let width =
            positive(d.strip_suffix(b"w").ok_or("parse_error")?).map_err(|_| "parse_error")?;
        if total >= 16
            || widths[..total].contains(&width)
            || media_path(u, false).map_err(|_| "parse_error")? != original
        {
            return Err("parse_error");
        }
        widths[total] = width;
        total += 1;
        if width > largest {
            if u.len() >= out.len() {
                return Err("capacity_exceeded");
            }
            out[..u.len()].copy_from_slice(u);
            out_len = u.len();
            largest = width;
        }
    }
    if total == 0 {
        return Err("parse_error");
    }
    Ok(out_len)
}
fn body_images(doc: host::HtmlDescriptor) -> R<()> {
    if count(doc, BODY)? != 1 {
        return Err("parse_error");
    }
    let node = host::html_select(doc, BODY).map_err(|_| "html_error")?;
    let r = (|| {
        if count(node, b"iframe,video,script,picture")? != 0 {
            return Err("parse_error");
        }
        let total = count(node, b"img")?;
        let expected = count(node, b"p > img[src][srcset]")?;
        if total == 0 || total > MAX_IMAGES || total != expected {
            return Err("parse_error");
        }
        let mut raw = [0; MAX_IMAGES * 4];
        let s = b"p > img[src][srcset]";
        let n = unsafe {
            select_all_raw(
                node.raw(),
                s.as_ptr(),
                s.len() as u32,
                raw.as_mut_ptr(),
                raw.len() as u32,
            )
        };
        if n != total as i32 {
            close_raw(&raw, n);
            return Err("html_error");
        }
        let mut error = None;
        for i in 0..total {
            let a = i * 4;
            let d = host::HtmlDescriptor::from_raw(i32::from_le_bytes([
                raw[a],
                raw[a + 1],
                raw[a + 2],
                raw[a + 3],
            ]));
            let r = (|| {
                let mut src = [0; 1024];
                let n = host::html_attr(d, b"src", &mut src).map_err(|_| "html_error")?;
                if n == 0 || n >= 1024 {
                    return Err("capacity_exceeded");
                }
                let mut set = [0; 8192];
                let k = host::html_attr(d, b"srcset", &mut set).map_err(|_| "html_error")?;
                if k == 0 || k >= 8192 {
                    return Err("capacity_exceeded");
                }
                let l = selected_srcset(&src[..n], &set[..k], unsafe { &mut IMAGES[i] })?;
                for j in 0..i {
                    if unsafe { &IMAGES[j][..IL[j]] } == unsafe { &IMAGES[i][..l] } {
                        return Err("parse_error");
                    }
                }
                unsafe {
                    IL[i] = l;
                }
                Ok(())
            })();
            let close = host::html_close(d).map_err(|_| "html_error");
            if let Err(e) = r.and(close) {
                if error.is_none() {
                    error = Some(e);
                }
            }
        }
        if let Some(e) = error {
            return Err(e);
        }
        unsafe {
            IC = total;
            CL = IL[0];
            COVER[..CL].copy_from_slice(&IMAGES[0][..CL]);
        }
        let n = host::html_text(node, unsafe { &mut NOTE }).map_err(|_| "html_error")?;
        if n >= 16384 || core::str::from_utf8(unsafe { &NOTE[..n] }).is_err() {
            return Err("capacity_exceeded");
        }
        unsafe {
            NOTE_LEN = n;
        }
        Ok(())
    })();
    let close = host::html_close(node).map_err(|_| "html_error");
    r.and(close)
}
fn json_identity(doc: host::HtmlDescriptor, url: &[u8], detail: bool) -> R<()> {
    let n = text(doc, b"head script[type='application/ld+json']", scratch_a())?;
    if n >= 65536 {
        return Err("capacity_exceeded");
    }
    let j = trim_ascii(&scratch_a()[..n]);
    let graph = member(j, b"@graph")?;
    if graph.first() != Some(&b'[') {
        return Err("parse_error");
    }
    let mut at = whitespace(graph, 1);
    let mut article = 0;
    let mut webpage = 0;
    let mut website = 0;
    let mut person = 0;
    let mut entries = 0;
    while graph.get(at) != Some(&b']') {
        entries += 1;
        if entries > 32 {
            return Err("capacity_exceeded");
        }
        let e = end_value(graph, at)?;
        let obj = &graph[at..e];
        let ty = string(obj, b"@type")?;
        let mut expected = [0; 768];
        if detail && ty == b"Article" {
            article += 1;
            let n = with_suffix(url, b"#article", &mut expected)?;
            json_eq(obj, b"@id", &expected[..n])?;
            json_eq(obj, b"headline", unsafe { &TITLE[..TL] })?;
            json_eq(member(obj, b"author")?, b"@id", AUTHOR_ID)?;
            let n = with_suffix(url, b"#webpage", &mut expected)?;
            json_eq(member(obj, b"mainEntityOfPage")?, b"@id", &expected[..n])?;
            json_eq(obj, b"inLanguage", b"zh-TW")?;
            let n = raw_string(string(obj, b"datePublished")?, unsafe { &mut DATE })?;
            if n < 20 || n >= 64 {
                return Err("parse_error");
            }
            unsafe {
                DL = n;
            }
        }
        if (detail && ty == b"WebPage") || (!detail && ty == b"CollectionPage") {
            webpage += 1;
            json_eq(obj, b"url", url)?;
            let n = with_suffix(
                url,
                if detail {
                    b"#webpage"
                } else {
                    b"#collectionpage"
                },
                &mut expected,
            )?;
            json_eq(obj, b"@id", &expected[..n])?;
            json_eq(obj, b"inLanguage", b"zh-TW")?;
        }
        if ty == b"WebSite" {
            website += 1;
            json_eq(obj, b"url", b"https://blog.mixflavor.com/")?;
            json_eq(obj, b"name", "綜合口味".as_bytes())?;
        }
        if ty == b"Person" {
            let identity = string(obj, b"@id")?;
            let mut dec = [0; 512];
            let len = raw_string(identity, &mut dec)?;
            if (detail && &dec[..len] == AUTHOR_ID)
                || (!detail && &dec[..len] == b"https://blog.mixflavor.com/#person")
            {
                person += 1;
                json_eq(obj, b"name", b"mixflavor")?;
                if detail {
                    json_eq(obj, b"url", AUTHOR)?;
                }
            }
        }
        at = whitespace(graph, e);
        if graph.get(at) == Some(&b',') {
            at = whitespace(graph, at + 1);
        } else if graph.get(at) != Some(&b']') {
            return Err("parse_error");
        }
    }
    if article != if detail { 1 } else { 0 } || webpage != 1 || website != 1 || person != 1 {
        return Err("identity_mismatch");
    }
    Ok(())
}
fn article() -> R<()> {
    let mut u = [0; 512];
    let len = uri(&mut u)?;
    let n = fetch(&u[..len])?;
    let doc = host::html_parse(&body_buf()[..n]).map_err(|_| "html_error")?;
    let r = (|| {
        eq_attr(doc, b"head link[rel='canonical'][href]", b"href", &u[..len])?;
        eq_attr(
            doc,
            b"head meta[property='og:url'][content]",
            b"content",
            &u[..len],
        )?;
        let mut b = [0; 2048];
        let n = attr(doc, b"body[class]", b"class", &mut b)?;
        if n >= 2048 {
            return Err("capacity_exceeded");
        }
        let mut post = [0; 24];
        let k = post_token(&b[..n], b"postid-", &mut post)?;
        if &post[..k] != unsafe { &POST[..PL] } {
            return Err("identity_mismatch");
        }
        let s = b"main#eskimo-main-container > div.container > div.eskimo-page-title";
        if count(doc, s)? != 1 {
            return Err("parse_error");
        }
        let title = host::html_select(doc, s).map_err(|_| "html_error")?;
        let r = (|| {
            if !scope(
                title,
                b"div.eskimo-cat-meta > a[rel='category tag'][href]",
                b"div.eskimo-cat-meta > a",
            )? {
                return Err("out_of_scope");
            }
            eq_attr(title, b"a.author-meta[href]", b"href", AUTHOR)?;
            let n = text(title, b"a.author-meta", &mut b)?;
            if trim_ascii(&b[..n]) != b"mixflavor" {
                return Err("identity_mismatch");
            }
            let n = text(title, b"h1", unsafe { &mut TITLE })?;
            let mut temp = [0; 1024];
            let t = trim_ascii(unsafe { &TITLE[..n] });
            if n >= 1024 || t.is_empty() || core::str::from_utf8(t).is_err() {
                return Err("parse_error");
            }
            let k = t.len();
            temp[..k].copy_from_slice(t);
            unsafe {
                TITLE[..k].copy_from_slice(&temp[..k]);
                TL = k;
            }
            Ok(())
        })();
        let close = host::html_close(title).map_err(|_| "html_error");
        r.and(close)?;
        json_identity(doc, &u[..len], true)?;
        body_images(doc)
    })();
    let close = host::html_close(doc).map_err(|_| "html_error");
    r.and(close)
}
fn write_book(p: &mut [u8], c: &mut usize, full: bool) -> R<()> {
    w(p, c, b"{\"id\":\"")?;
    ident(p, c, MID)?;
    w(p, c, b"\",\"title\":")?;
    q(p, c, unsafe { &TITLE[..TL] })?;
    let mut u = [0; 512];
    let n = uri(&mut u)?;
    w(p, c, b",\"url\":")?;
    q(p, c, &u[..n])?;
    w(p, c, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(p, c, unsafe { &COVER[..CL] })?;
    w(p, c, b"},\"language\":\"zh-Hant\",\"authors\":[")?;
    q(p, c, "綜合口味".as_bytes())?;
    w(
        p,
        c,
        b"],\"artists\":[],\"status\":\"unknown\",\"contentRating\":\"unknown\",\"tags\":[",
    )?;
    for i in 0..unsafe { TC } {
        if i > 0 {
            w(p, c, b",")?;
        }
        q(p, c, unsafe { &TAGS[i][..TAG_LEN[i]] })?;
    }
    w(p, c, b"]")?;
    w(p, c, b",\"description\":\"")?;
    for v in [
        "收錄生活小漫畫，不包含同時標為開箱心得或食的整篇文章。作品評級尚未確認。發表：".as_bytes(),
        unsafe { &DATE[..DL] },
    ] {
        if !append_json_escaped(p, c, v) {
            return Err("capacity_exceeded");
        }
    }
    if full {
        if !append_json_escaped(p, c, "\n原文附記：\n".as_bytes())
            || !append_json_escaped(p, c, unsafe { &NOTE[..NOTE_LEN] })
        {
            return Err("capacity_exceeded");
        }
    }
    w(p, c, b"\"")?;
    if full {
        w(p, c, b",\"links\":[{\"kind\":\"source\",\"url\":")?;
        q(p, c, &u[..n])?;
        w(p, c, b"}]")?;
    }
    w(p, c, b"}")
}
fn pagination(doc: host::HtmlDescriptor, pn: usize) -> R<()> {
    unsafe {
        NL = 0;
    }
    let mut b = [0; 512];
    let k = text(
        doc,
        b"main#eskimo-main-container ul.pagination span.current",
        &mut b,
    )?;
    if positive(trim_ascii(&b[..k]))? != pn {
        return Err("identity_mismatch");
    }
    let s = b"main#eskimo-main-container ul.pagination > li > a[href][class]";
    let mut raw = [0; 64];
    let n = unsafe {
        select_all_raw(
            doc.raw(),
            s.as_ptr(),
            s.len() as u32,
            raw.as_mut_ptr(),
            raw.len() as u32,
        )
    };
    if n < 0 || n > 16 {
        close_raw(&raw, n);
        return Err("parse_error");
    }
    if count(
        doc,
        b"main#eskimo-main-container ul.pagination > li > a[href]",
    ) != Ok(n as usize)
        || count(doc, b"main#eskimo-main-container ul.pagination a.next")?
            != count(
                doc,
                b"main#eskimo-main-container ul.pagination a.next[href]",
            )?
        || count(doc, b"main#eskimo-main-container ul.pagination a.prev")?
            != count(
                doc,
                b"main#eskimo-main-container ul.pagination a.prev[href]",
            )?
    {
        close_raw(&raw, n);
        return Err("parse_error");
    }
    let mut prev = 0;
    let mut next = 0;
    let mut error = None;
    for i in 0..n as usize {
        let a = i * 4;
        let node = host::HtmlDescriptor::from_raw(i32::from_le_bytes([
            raw[a],
            raw[a + 1],
            raw[a + 2],
            raw[a + 3],
        ]));
        let r = (|| {
            let k = host::html_attr(node, b"href", &mut b).map_err(|_| "html_error")?;
            if k == 0 || k >= 512 {
                return Err("capacity_exceeded");
            }
            let page = if &b[..k] == CATEGORY {
                1
            } else {
                page_num(&b[..k]).map_err(|_| "parse_error")?
            };
            let mut classes = [0; 256];
            let l = host::html_attr(node, b"class", &mut classes).map_err(|_| "html_error")?;
            let forward = classes[..l]
                .split(|b| b.is_ascii_whitespace())
                .any(|x| x == b"next");
            let backward = classes[..l]
                .split(|b| b.is_ascii_whitespace())
                .any(|x| x == b"prev");
            if forward {
                if page != pn + 1 {
                    return Err("identity_mismatch");
                }
                next += 1;
                unsafe {
                    NEXT[..k].copy_from_slice(&b[..k]);
                    NL = k;
                }
            }
            if backward {
                if pn <= 1 || page != pn - 1 {
                    return Err("identity_mismatch");
                }
                prev += 1;
            }
            Ok(())
        })();
        let close = host::html_close(node).map_err(|_| "html_error");
        if let Err(e) = r.and(close) {
            if error.is_none() {
                error = Some(e);
            }
        }
    }
    if let Some(e) = error {
        return Err(e);
    }
    if next > 1 || prev != if pn == 1 { 0 } else { 1 } {
        return Err("parse_error");
    }
    Ok(())
}
fn run_get_listings(_req: &[u8]) -> u32 {
    let b =
        "{\"listings\":[{\"id\":\"listing:life\",\"name\":\"生活小漫畫（不含開箱心得及食）\"}]}"
            .as_bytes();
    payload_buf()[..b.len()].copy_from_slice(b);
    write_success_payload("get_listings", b.len())
}
fn run_get_manga_list(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        if arg_string(req, b"listingId")? != Some(b"listing:life") {
            return Err("invalid_request");
        }
        let mut u = [0; 512];
        let (ul, pn) = list_route(req, &mut u)?;
        let n = fetch(&u[..ul])?;
        let doc = host::html_parse(&body_buf()[..n]).map_err(|_| "html_error")?;
        let r = (|| {
            eq_attr(doc, b"head link[rel='canonical'][href]", b"href", &u[..ul])?;
            // Original category pages declare canonical + CollectionPage, with no og:url.
            json_identity(doc, &u[..ul], false)?;
            let mut title = [0; 256];
            let n = text(
                doc,
                b"main#eskimo-main-container > div.container > div.eskimo-page-title > h1",
                &mut title,
            )?;
            if trim_ascii(&title[..n]) != "生活小漫畫".as_bytes() {
                return Err("identity_mismatch");
            }
            pagination(doc, pn)?;
            let total = count(doc, CARDS)?;
            if total == 0 || total > 24 {
                return Err("parse_error");
            }
            let mut raw = [0; 96];
            let got = unsafe {
                select_all_raw(
                    doc.raw(),
                    CARDS.as_ptr(),
                    CARDS.len() as u32,
                    raw.as_mut_ptr(),
                    raw.len() as u32,
                )
            };
            if got != total as i32 {
                close_raw(&raw, got);
                return Err("html_error");
            }
            let p = payload_buf();
            let mut c = 0;
            w(p, &mut c, b"{\"items\":[")?;
            let mut kept = 0;
            let mut seen = [[0; 24]; 24];
            let mut sl = [0; 24];
            let mut error = None;
            for i in 0..total {
                let a = i * 4;
                let node = host::HtmlDescriptor::from_raw(i32::from_le_bytes([
                    raw[a],
                    raw[a + 1],
                    raw[a + 2],
                    raw[a + 3],
                ]));
                let r = (|| {
                    let mut b = [0; 2048];
                    let l = host::html_attr(node, b"class", &mut b).map_err(|_| "html_error")?;
                    let mut id = [0; 24];
                    let len = post_token(&b[..l], b"post-", &mut id)?;
                    for j in 0..i {
                        if sl[j] == len && seen[j][..len] == id[..len] {
                            return Err("duplicate_identity");
                        }
                    }
                    seen[i][..len].copy_from_slice(&id[..len]);
                    sl[i] = len;
                    set_post(&id[..len])?;
                    if !scope(
                        node,
                        b"div.card-category > span > a[rel='category tag'][href]",
                        b"div.card-category > span > a",
                    )? {
                        return Ok(());
                    }
                    let l = attr(node, b"h3.card-title > a[href]", b"href", &mut b)?;
                    set_uri(&b[..l]).map_err(|_| "identity_mismatch")?;
                    eq_attr(node, b"div.card > a[href]", b"href", &b[..l])?;
                    let n = text(node, b"h3.card-title > a", unsafe { &mut TITLE })?;
                    if n == 0 || n >= 1024 || core::str::from_utf8(unsafe { &TITLE[..n] }).is_err()
                    {
                        return Err("parse_error");
                    }
                    unsafe {
                        TL = n;
                    }
                    eq_attr(node, b"a.author-meta[href]", b"href", AUTHOR)?;
                    let l = text(node, b"a.author-meta", &mut b)?;
                    if trim_ascii(&b[..l]) != b"mixflavor" {
                        return Err("identity_mismatch");
                    }
                    let l = attr(node, b"div.card > a > img[src]", b"src", unsafe {
                        &mut COVER
                    })?;
                    media_path(unsafe { &COVER[..l] }, false).map_err(|_| "parse_error")?;
                    unsafe {
                        CL = l;
                    }
                    let l = text(node, b"div.eskimo-date-meta > a", unsafe { &mut DATE })?;
                    let d = trim_ascii(unsafe { &DATE[..l] });
                    if l >= 64 || d.len() != 10 || d[4] != b'/' || d[7] != b'/' {
                        return Err("parse_error");
                    }
                    let mut temp = [0; 64];
                    temp[..d.len()].copy_from_slice(d);
                    unsafe {
                        DL = d.len();
                        DATE[..DL].copy_from_slice(&temp[..DL]);
                    }
                    if kept > 0 {
                        w(p, &mut c, b",")?;
                    }
                    write_book(p, &mut c, false)?;
                    kept += 1;
                    Ok(())
                })();
                let close = host::html_close(node).map_err(|_| "html_error");
                if let Err(e) = r.and(close) {
                    if error.is_none() {
                        error = Some(e);
                    }
                }
            }
            if let Some(e) = error {
                return Err(e);
            }
            w(p, &mut c, b"],\"hasNextPage\":")?;
            w(
                p,
                &mut c,
                if unsafe { NL } > 0 { b"true" } else { b"false" },
            )?;
            w(p, &mut c, b",\"page\":{\"nextCursor\":")?;
            if unsafe { NL } > 0 {
                q(p, &mut c, unsafe { &NEXT[..NL] })?;
            } else {
                w(p, &mut c, b"null")?;
            }
            w(p, &mut c, b",\"hasMore\":")?;
            w(
                p,
                &mut c,
                if unsafe { NL } > 0 { b"true" } else { b"false" },
            )?;
            w(p, &mut c, b"}}")?;
            Ok(c)
        })();
        let close = host::html_close(doc).map_err(|_| "html_error");
        match (r, close) {
            (Ok(n), Ok(())) => Ok(n),
            (Err(e), _) | (_, Err(e)) => Err(e),
        }
    }
    result("get_manga_list", inner(req))
}
fn run_get_manga(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        request_work(req, b"mangaId", MID)?;
        article()?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"manga\":")?;
        write_book(p, &mut c, true)?;
        w(p, &mut c, b"}")?;
        Ok(c)
    }
    result("get_manga", inner(req))
}
fn run_get_chapters(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        request_work(req, b"mangaId", MID)?;
        if let Some(page) = direct(req, b"page").map_err(|_| "invalid_request")? {
            if page.first() != Some(&b'{')
                || !matches!(
                    direct(page, b"cursor").map_err(|_| "invalid_request")?,
                    None | Some(b"null")
                )
            {
                return Err("invalid_request");
            }
        }
        article()?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"chapters\":[{\"id\":\"")?;
        ident(p, &mut c, CID)?;
        w(p, &mut c, b"\",\"mangaId\":\"")?;
        ident(p, &mut c, MID)?;
        w(p, &mut c, b"\",\"title\":")?;
        q(p, &mut c, unsafe { &TITLE[..TL] })?;
        let mut u = [0; 512];
        let l = uri(&mut u)?;
        w(p, &mut c, b",\"url\":")?;
        q(p, &mut c, &u[..l])?;
        w(
            p,
            &mut c,
            b",\"chapterNumber\":null,\"volumeNumber\":null,\"readingGroup\":\"",
        )?;
        ident(p, &mut c, MID)?;
        w(
            p,
            &mut c,
            b"\",\"continuationEligible\":false,\"language\":\"zh-Hant\",\"pageCount\":",
        )?;
        n(p, &mut c, unsafe { IC })?;
        w(
            p,
            &mut c,
            b"}],\"page\":{\"nextCursor\":null,\"hasMore\":false}}",
        )?;
        Ok(c)
    }
    result("get_chapters", inner(req))
}
fn run_get_pages(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        request_work(req, b"chapterId", CID)?;
        article()?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"pages\":[")?;
        for i in 0..unsafe { IC } {
            if i > 0 {
                w(p, &mut c, b",")?;
            }
            w(p, &mut c, b"{\"id\":\"")?;
            ident(p, &mut c, PID)?;
            w(p, &mut c, b":")?;
            n(p, &mut c, i)?;
            w(p, &mut c, b"\",\"index\":")?;
            n(p, &mut c, i)?;
            w(p, &mut c, b",\"image\":{\"kind\":\"url\",\"url\":")?;
            q(p, &mut c, unsafe { &IMAGES[i][..IL[i]] })?;
            w(p, &mut c, b"}}")?;
        }
        w(p, &mut c, b"]}")?;
        Ok(c)
    }
    result("get_pages", inner(req))
}
fn run_get_image_request(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let v = arg_string(req, b"pageId")?.ok_or("invalid_request")?;
        let u = arg_string(req, b"url")?.ok_or("invalid_request")?;
        media_path(u, false)?;
        if v.starts_with(MID) {
            if !parse_identity(v, MID)?.is_empty() {
                return Err("invalid_request");
            }
            owner(req)?; /* Trust the prior catalogue/detail pairing: zero HTTP, no independent post->cover proof. */
        } else {
            let rest = parse_identity(v, PID)?;
            let ordinal = rest.strip_prefix(b":").ok_or("invalid_request")?;
            let at = if ordinal == b"0" {
                0
            } else {
                positive(ordinal)?
            };
            if at >= MAX_IMAGES {
                return Err("invalid_request");
            }
            owner(req)?;
            article()?;
            if at >= unsafe { IC } || u != unsafe { &IMAGES[at][..IL[at]] } {
                return Err("invalid_request");
            }
        }
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"imageRequest\":{\"url\":")?;
        q(p, &mut c, u)?;
        w(p,&mut c,b",\"headers\":{\"User-Agent\":\"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0\",\"Referer\":")?;
        let mut u = [0; 512];
        let l = uri(&mut u)?;
        q(p, &mut c, &u[..l])?;
        w(p, &mut c, b"}}}")?;
        Ok(c)
    }
    result("get_image_request", inner(req))
}
fn run_search(_req: &[u8]) -> u32 {
    write_error("search", "not_implemented", "此來源不提供搜尋")
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
