#![no_std]
use koma_source_sdk::json_utils::*;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{build_get_request, decode_json_body_into, host};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.huanta.zh.koma",
    name: "幻塔官方中文漫画",
    version: "0.1.0",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "幻塔官网公开漫画。每篇保留完整原图，创作者和作品评级尚未确认。",
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

const BASE: &[u8] = b"https://ht.wanmei.com/";
const LIST: &[u8] = b"https://ht.wanmei.com/videostation/cartoon/index.html";
const DIR: &[u8] = b"/videostation/cartoon/";
const MEDIA: &[u8] = b"https://ht.wanmei.com/resources/jpg/";
const MAX: usize = 128;
static mut PATHS: [[u8; 96]; MAX] = [[0; 96]; MAX];
static mut PL: [usize; MAX] = [0; MAX];
static mut TITLES: [[u8; 256]; MAX] = [[0; 256]; MAX];
static mut TL: [usize; MAX] = [0; MAX];
static mut COVERS: [[u8; 256]; MAX] = [[0; 256]; MAX];
static mut CL: [usize; MAX] = [0; MAX];
static mut USED: usize = 0;
static mut IMAGE: [u8; 256] = [0; 256];
static mut IL: usize = 0;
#[derive(Clone, Copy)]
struct Paging {
    current: usize,
    pages: usize,
    per: usize,
    total: usize,
    items: usize,
}
fn fetch(u: &[u8]) -> R<usize> {
    let n = build_get_request(http_req_buf(), u, Some(BASE), &[]).ok_or("capacity_exceeded")?;
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
fn path(v: &[u8]) -> R<()> {
    let v = v
        .strip_prefix(DIR)
        .and_then(|x| x.strip_suffix(b".shtml"))
        .ok_or("wrong_owner")?;
    if v.len() < 10
        || v[8] != b'/'
        || !v[..8].iter().all(|b| b.is_ascii_digit())
        || !v.starts_with(b"20")
    {
        return Err("wrong_owner");
    }
    positive(&v[9..])?;
    Ok(())
}
fn media(v: &[u8]) -> R<()> {
    let v = v
        .strip_prefix(MEDIA)
        .and_then(|x| x.strip_suffix(b".jpg"))
        .ok_or("wrong_owner")?;
    if v.len() < 8
        || v[6] != b'/'
        || !v[..6].iter().all(|b| b.is_ascii_digit())
        || !v[7..].iter().all(|b| b.is_ascii_digit())
        || v[7..].is_empty()
        || v[7..].len() > 32
    {
        return Err("wrong_owner");
    }
    Ok(())
}
fn route(p: &mut [u8], c: &mut usize, i: usize) -> R<()> {
    w(p, c, BASE)?;
    w(p, c, unsafe { &PATHS[i][1..PL[i]] })
}
fn identifier(p: &mut [u8], c: &mut usize, prefix: &[u8], i: usize) -> R<()> {
    w(p, c, prefix)?;
    let a = unsafe { &PATHS[i][DIR.len()..PL[i] - 6] };
    w(p, c, &a[..8])?;
    w(p, c, b":")?;
    w(p, c, &a[9..])
}
fn mid(p: &mut [u8], c: &mut usize, i: usize) -> R<()> {
    identifier(p, c, b"huanta:zh:comic:", i)
}
fn sid(v: &[u8], prefix: &[u8], out: &mut [u8]) -> R<usize> {
    let v = v.strip_prefix(prefix).ok_or("invalid_request")?;
    if v.len() < 10
        || v[8] != b':'
        || !v[..8].iter().all(|b| b.is_ascii_digit())
        || !v.starts_with(b"20")
    {
        return Err("invalid_request");
    }
    positive(&v[9..])?;
    let mut c = 0;
    w(out, &mut c, DIR)?;
    w(out, &mut c, &v[..8])?;
    w(out, &mut c, b"/")?;
    w(out, &mut c, &v[9..])?;
    w(out, &mut c, b".shtml")?;
    Ok(c)
}
fn owner(req: &[u8], expected: &[u8]) -> R<()> {
    if let Some(v) = arg_string(req, b"mangaUrl")? {
        let mut b = [0; 256];
        let n = raw_string(v, &mut b)?;
        let mut u = [0; 256];
        let mut c = 0;
        w(&mut u, &mut c, BASE)?;
        w(&mut u, &mut c, &expected[1..])?;
        if b[..n] != u[..c] {
            return Err("invalid_request");
        }
    }
    if let Some(v) = arg_string(req, b"mangaId")? {
        let mut b = [0; 96];
        let n = sid(v, b"huanta:zh:comic:", &mut b)?;
        if &b[..n] != expected {
            return Err("invalid_request");
        }
    }
    Ok(())
}
fn index_file(dst: &mut [u8], page: usize) -> R<usize> {
    let mut c = 0;
    w(dst, &mut c, b"index")?;
    if page > 1 {
        n(dst, &mut c, page - 1)?;
    }
    w(dst, &mut c, b".html")?;
    Ok(c)
}
fn index_url(dst: &mut [u8], page: usize) -> R<usize> {
    let mut file = [0; 32];
    let size = index_file(&mut file, page)?;
    let mut c = 0;
    w(dst, &mut c, b"https://ht.wanmei.com/videostation/cartoon/")?;
    w(dst, &mut c, &file[..size])?;
    Ok(c)
}
fn catalogue_page(which: usize, offset: usize) -> R<Paging> {
    let mut u = [0; 256];
    let l = index_url(&mut u, which)?;
    let size = fetch(&u[..l])?;
    let doc = host::html_parse(&body_buf()[..size]).map_err(|_| "html_error")?;
    let r = (|| {
        let mut b = [0; 2048];
        let l = text(doc, b"title", &mut b)?;
        if trim_ascii(&b[..l]) != "漫画 -《幻塔》官网-轻科幻开放世界游戏".as_bytes()
        {
            return Err("wrong_owner");
        }
        let l = attr(doc, b".list_nav > a.on[href]", b"href", &mut b)?;
        if &b[..l] != LIST {
            return Err("wrong_owner");
        }
        if count(doc, b".page td.nlist")? != 1 || count(doc, b".page td.nlist > strong")? != 4 {
            return Err("catalogue_changed");
        }
        let mut numbers = [0; 4];
        for (i, value) in numbers.iter_mut().enumerate() {
            let mut sel = [0; 128];
            let mut c = 0;
            w(&mut sel, &mut c, b".page td.nlist > strong:nth-of-type(")?;
            n(&mut sel, &mut c, i + 1)?;
            w(&mut sel, &mut c, b")")?;
            let l = text(doc, &sel[..c], &mut b)?;
            *value = positive(trim_ascii(&b[..l]))?;
        }
        let paging = Paging {
            current: numbers[0],
            pages: numbers[1],
            per: numbers[2],
            total: numbers[3],
            items: count(doc, b"ul.media-list.cartoon-list > li")?,
        };
        if paging.current != which
            || paging.per != 9
            || paging.total > MAX
            || paging.pages != 2
            || paging.pages != (paging.total + 8) / 9
            || which > paging.pages
            || paging.items != core::cmp::min(9, paging.total - (which - 1) * 9)
            || offset + paging.items > MAX
        {
            return Err("catalogue_changed");
        }
        if which < paging.pages {
            let mut file = [0; 32];
            let len = index_file(&mut file, which + 1)?;
            let mut sel = [0; 128];
            let mut c = 0;
            w(&mut sel, &mut c, b".page td.nlist > a[href='")?;
            w(&mut sel, &mut c, &file[..len])?;
            w(&mut sel, &mut c, b"']")?;
            if count(doc, &sel[..c])? == 0 {
                return Err("catalogue_changed");
            }
        }
        for j in 0..paging.items {
            let i = offset + j;
            let mut root = [0; 256];
            let mut c = 0;
            w(
                &mut root,
                &mut c,
                b"ul.media-list.cartoon-list > li:nth-child(",
            )?;
            n(&mut root, &mut c, j + 1)?;
            w(&mut root, &mut c, b")")?;
            let len = c;
            w(&mut root, &mut c, b" > a[href]")?;
            let l = attr(doc, &root[..c], b"href", &mut b)?;
            path(&b[..l])?;
            if l > 96 {
                return Err("capacity_exceeded");
            }
            for k in 0..i {
                if unsafe { &PATHS[k][..PL[k]] } == &b[..l] {
                    return Err("duplicate_identity");
                }
            }
            unsafe {
                PATHS[i][..l].copy_from_slice(&b[..l]);
                PL[i] = l;
            }
            c = len;
            w(&mut root, &mut c, b" > a > span")?;
            let l = text(doc, &root[..c], &mut b)?;
            let title = trim_ascii(&b[..l]);
            if title.is_empty() || title.len() > 256 || core::str::from_utf8(title).is_err() {
                return Err("parse_error");
            }
            unsafe {
                TITLES[i][..title.len()].copy_from_slice(title);
                TL[i] = title.len();
            }
            c = len;
            w(&mut root, &mut c, b" > a > img[src]")?;
            let l = attr(doc, &root[..c], b"src", &mut b)?;
            media(&b[..l])?;
            if l > 256 {
                return Err("capacity_exceeded");
            }
            unsafe {
                COVERS[i][..l].copy_from_slice(&b[..l]);
                CL[i] = l;
            }
        }
        Ok(paging)
    })();
    host::html_close(doc).map_err(|_| "html_error")?;
    r
}
fn catalogue_all() -> R<()> {
    unsafe {
        USED = 0;
    }
    let first = catalogue_page(1, 0)?;
    let mut used = first.items;
    for which in 2..=first.pages {
        let row = catalogue_page(which, used)?;
        if row.total != first.total || row.pages != first.pages {
            return Err("catalogue_changed");
        }
        used += row.items;
    }
    if used != first.total {
        return Err("catalogue_changed");
    }
    unsafe {
        USED = used;
    }
    Ok(())
}
fn find(v: &[u8]) -> R<usize> {
    for i in 0..unsafe { USED } {
        if v == unsafe { &PATHS[i][..PL[i]] } {
            return Ok(i);
        }
    }
    Err("not_found")
}
fn current_image(i: usize) -> R<()> {
    let mut u = [0; 256];
    let mut c = 0;
    route(&mut u, &mut c, i)?;
    let size = fetch(&u[..c])?;
    let doc = host::html_parse(&body_buf()[..size]).map_err(|_| "html_error")?;
    let r = (|| {
        let mut b = [0; 2048];
        let l = text(doc, b".box_main > h2.detail_t", &mut b)?;
        if trim_ascii(&b[..l]) != unsafe { &TITLES[i][..TL[i]] } {
            return Err("wrong_owner");
        }
        let l = text(doc, b"title", &mut b)?;
        let mut expected = [0; 512];
        let mut e = 0;
        w(&mut expected, &mut e, unsafe { &TITLES[i][..TL[i]] })?;
        w(
            &mut expected,
            &mut e,
            " -《幻塔》官网-轻科幻开放世界游戏".as_bytes(),
        )?;
        if trim_ascii(&b[..l]) != &expected[..e] {
            return Err("wrong_owner");
        }
        if count(doc, b".box_main > .detail_time > span")? != 2 {
            return Err("wrong_owner");
        }
        let l = text(doc, b".box_main > .detail_time > span:nth-child(2)", &mut b)?;
        if trim_ascii(&b[..l]) != "《幻塔》手游官网".as_bytes() {
            return Err("wrong_owner");
        }
        let l = text(doc, b".box_main > .detail_time > span:nth-child(1)", &mut b)?;
        let date = trim_ascii(&b[..l]);
        let pp = unsafe { &PATHS[i][DIR.len()..DIR.len() + 8] };
        if date.len() != 10
            || date[4] != b'-'
            || date[7] != b'-'
            || date[..4] != pp[..4]
            || date[5..7] != pp[4..6]
            || date[8..] != pp[6..]
        {
            return Err("wrong_owner");
        }
        if count(doc, b".box_main > .text_cont")? != 1
            || count(doc, b".box_main > .text_cont img")? != 1
            || count(doc, b".box_main > .text_cont > a")? != 1
        {
            return Err("incomplete_pages");
        }
        let l = attr(doc, b".box_main > .text_cont > a[href]", b"href", &mut b)?;
        media(&b[..l])?;
        if l > 256 {
            return Err("capacity_exceeded");
        }
        let mut original = [0; 256];
        original[..l].copy_from_slice(&b[..l]);
        let len = attr(
            doc,
            b".box_main > .text_cont > a > img[src]",
            b"src",
            &mut b,
        )?;
        if b[..len] != original[..l] {
            return Err("wrong_owner");
        }
        // alt is duplicated in the real HTML: neither alt occurrence establishes ownership.
        unsafe {
            IMAGE[..l].copy_from_slice(&original[..l]);
            IL = l;
        }
        Ok(())
    })();
    host::html_close(doc).map_err(|_| "html_error")?;
    r
}
fn book(p: &mut [u8], c: &mut usize, i: usize, detail: bool) -> R<()> {
    w(p, c, b"{\"id\":\"")?;
    mid(p, c, i)?;
    w(p, c, b"\",\"title\":")?;
    q(p, c, unsafe { &TITLES[i][..TL[i]] })?;
    w(p, c, b",\"url\":\"")?;
    route(p, c, i)?;
    w(p, c, b"\",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(p, c, unsafe { &COVERS[i][..CL[i]] })?;
    w(p,c,b"},\"language\":\"zh\",\"authors\":[],\"artists\":[],\"status\":\"unknown\",\"contentRating\":\"unknown\"")?;
    if detail {
        w(p, c, b",\"description\":")?;
        q(
            p,
            c,
            "幻塔官网公开漫画。每篇保留完整原图，创作者和作品评级尚未确认。".as_bytes(),
        )?;
        w(p, c, b",\"links\":[{\"kind\":\"source\",\"url\":\"")?;
        route(p, c, i)?;
        w(p, c, b"\"}]")?;
    }
    w(p, c, b"}")
}
fn listing(which: usize) -> R<usize> {
    let first = catalogue_page(1, 0)?;
    let mut row = first;
    let mut used = first.items;
    if which <= first.pages {
        for j in 2..=which {
            row = catalogue_page(j, used)?;
            if row.total != first.total || row.pages != first.pages {
                return Err("catalogue_changed");
            }
            used += row.items;
        }
    }
    let p = payload_buf();
    let mut c = 0;
    w(p, &mut c, b"{\"items\":[")?;
    if which <= first.pages {
        let start = used - row.items;
        for i in start..used {
            if i > start {
                w(p, &mut c, b",")?;
            }
            book(p, &mut c, i, false)?;
        }
    }
    w(p, &mut c, b"],\"page\":{\"nextCursor\":")?;
    if which < first.pages {
        w(p, &mut c, b"\"")?;
        n(p, &mut c, which + 1)?;
        w(p, &mut c, b"\",\"hasMore\":true},\"hasNextPage\":true}")?;
    } else {
        w(p, &mut c, b"null,\"hasMore\":false},\"hasNextPage\":false}")?;
    }
    Ok(c)
}
fn run_search(_req: &[u8]) -> u32 {
    write_error("search", "not_implemented", "此源不提供搜索")
}
fn run_get_manga_list(req: &[u8]) -> u32 {
    if arg_string(req, b"listingId").ok().flatten() != Some(b"listing:comics") {
        return write_error("get_manga_list", "invalid_request", "");
    }
    match page(req) {
        Ok(n) if n <= 2 => result("get_manga_list", listing(n)),
        _ => write_error("get_manga_list", "invalid_request", ""),
    }
}
fn request_work(req: &[u8], key: &[u8], prefix: &[u8]) -> R<usize> {
    let mut b = [0; 96];
    let size = sid(
        arg_string(req, key)?.ok_or("invalid_request")?,
        prefix,
        &mut b,
    )?;
    owner(req, &b[..size])?;
    catalogue_all()?;
    find(&b[..size])
}
fn run_get_manga(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let i = request_work(req, b"mangaId", b"huanta:zh:comic:")?;
        current_image(i)?;
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
        let i = request_work(req, b"mangaId", b"huanta:zh:comic:")?;
        current_image(i)?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"chapters\":[{\"id\":\"")?;
        identifier(p, &mut c, b"huantaread:zh:comic:", i)?;
        w(p, &mut c, b"\",\"mangaId\":\"")?;
        mid(p, &mut c, i)?;
        w(p, &mut c, b"\",\"title\":")?;
        q(p, &mut c, unsafe { &TITLES[i][..TL[i]] })?;
        w(p, &mut c, b",\"url\":\"")?;
        route(p, &mut c, i)?;
        w(
            p,
            &mut c,
            b"\",\"chapterNumber\":null,\"volumeNumber\":null,\"readingGroup\":\"",
        )?;
        mid(p, &mut c, i)?;
        w(
            p,
            &mut c,
            b"\",\"continuationEligible\":false,\"language\":\"zh\",\"pageCount\":1}]}",
        )?;
        Ok(c)
    }
    result("get_chapters", inner(req))
}
fn run_get_pages(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let i = request_work(req, b"chapterId", b"huantaread:zh:comic:")?;
        current_image(i)?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"pages\":[{\"id\":\"")?;
        identifier(p, &mut c, b"huantapage:zh:comic:", i)?;
        w(
            p,
            &mut c,
            b":0\",\"index\":0,\"image\":{\"kind\":\"url\",\"url\":",
        )?;
        q(p, &mut c, unsafe { &IMAGE[..IL] })?;
        w(p, &mut c, b"}}]}")?;
        Ok(c)
    }
    result("get_pages", inner(req))
}
fn run_get_image_request(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let mut url = [0; 512];
        let size = raw_string(arg_string(req, b"url")?.ok_or("invalid_request")?, &mut url)?;
        let url = &url[..size];
        media(url).map_err(|_| "invalid_request")?;
        let mut expected = [0; 96];
        let mut el = 0;
        let body = if let Some(v) = arg_string(req, b"pageId")? {
            if let Some(v) = v.strip_prefix(b"huantapage:zh:comic:") {
                let v = v.strip_suffix(b":0").ok_or("invalid_request")?;
                let mut id = [0; 128];
                let mut c = 0;
                w(&mut id, &mut c, b"huanta:zh:comic:")?;
                w(&mut id, &mut c, v)?;
                el = sid(&id[..c], b"huanta:zh:comic:", &mut expected)?;
                true
            } else {
                el = sid(v, b"huanta:zh:comic:", &mut expected)?;
                false
            }
        } else {
            false
        };
        if el > 0 {
            owner(req, &expected[..el])?;
        }
        catalogue_all()?;
        let i = if el > 0 {
            find(&expected[..el])?
        } else {
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
        owner(req, unsafe { &PATHS[i][..PL[i]] })?;
        if body {
            current_image(i)?;
            if url != unsafe { &IMAGE[..IL] } {
                return Err("invalid_request");
            }
        } else if url != unsafe { &COVERS[i][..CL[i]] } {
            return Err("invalid_request");
        }
        let mut referer = [0; 256];
        let mut len = 0;
        route(&mut referer, &mut len, i)?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"imageRequest\":{\"url\":")?;
        q(p, &mut c, url)?;
        w(p, &mut c, b",\"headers\":{\"Referer\":")?;
        q(p, &mut c, &referer[..len])?;
        w(p, &mut c, b"}}}")?;
        Ok(c)
    }
    result("get_image_request", inner(req))
}
fn run_get_listings(_req: &[u8]) -> u32 {
    let b = "{\"listings\":[{\"id\":\"listing:comics\",\"name\":\"官方漫画\"}]}".as_bytes();
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
