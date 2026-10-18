#![no_std]
use koma_source_sdk::json_utils::*;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{build_get_request, build_post_request, decode_json_body_into, host};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.tipo.ipmini.zh-hant.koma",
    name: "IP手札mini! 官方直達專欄",
    version: "0.1.0",
    api_version: "0.2",
    language: "zh-Hant",
    author: "Koma",
    description:
        "僅收錄智慧財產局漫畫目錄中可直達專欄的IP手札mini!作品。舊泛目錄不支援，作品評級尚未確認。",
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

const LIST: &[u8] = b"https://www.tipo.gov.tw/tw/tipo1/295.html";
const DETAIL: &[u8] = b"https://cloud.tipo.gov.tw/S400/article/columnist/";
const IMAGE: &[u8] = b"https://cloud.tipo.gov.tw/S400/bapi/article/columnist/";
const MID: &[u8] = b"tipoipmini:zh-hant:article:";
const CID: &[u8] = b"tipoipminiread:zh-hant:article:";
const PID: &[u8] = b"tipoipminipage:zh-hant:article:";
const COVER: &[u8] = b"https://www.tipo.gov.tw/wSite/public/Attachment/0/f";
const MAX: usize = 24;
const MAX_IMAGES: usize = 16;
static mut IDS: [[u8; 12]; MAX] = [[0; 12]; MAX];
static mut IDL: [usize; MAX] = [0; MAX];
static mut TITLES: [[u8; 512]; MAX] = [[0; 512]; MAX];
static mut TL: [usize; MAX] = [0; MAX];
static mut COVERS: [[u8; 512]; MAX] = [[0; 512]; MAX];
static mut CL: [usize; MAX] = [0; MAX];
static mut USED: usize = 0;
static mut EXCLUDED: usize = 0;
static mut NEXT: [u8; 262144] = [0; 262144];
static mut CONTENT: [u8; 262144] = [0; 262144];
static mut DT: [u8; 512] = [0; 512];
static mut DTL: usize = 0;
static mut AUTHOR: [u8; 512] = [0; 512];
static mut AL: usize = 0;
static mut SUMMARY: [u8; 8192] = [0; 8192];
static mut SL: usize = 0;
static mut DATE: [u8; 8] = [0; 8];
static mut IMAGES: [[u8; 512]; MAX_IMAGES] = [[0; 512]; MAX_IMAGES];
static mut IL: [usize; MAX_IMAGES] = [0; MAX_IMAGES];
static mut IC: usize = 0;
fn positive(v: &[u8]) -> R<usize> {
    if v.len() > 12 || v.first() == Some(&b'0') {
        return Err("invalid_request");
    }
    let n = number(v).map_err(|_| "invalid_request")?;
    if n == 0 {
        Err("invalid_request")
    } else {
        Ok(n)
    }
}
fn sid<'a>(v: &'a [u8], prefix: &[u8]) -> R<&'a [u8]> {
    let id = v.strip_prefix(prefix).ok_or("invalid_request")?;
    positive(id)?;
    Ok(id)
}
fn pref(p: &mut [u8], c: &mut usize, prefix: &[u8], id: &[u8]) -> R<()> {
    w(p, c, prefix)?;
    w(p, c, id)
}
fn route(p: &mut [u8], c: &mut usize, id: &[u8]) -> R<()> {
    pref(p, c, DETAIL, id)
}
fn owner(req: &[u8], id: &[u8]) -> R<()> {
    if let Some(v) = arg_string(req, b"mangaId")? {
        if sid(v, MID)? != id {
            return Err("invalid_request");
        }
    }
    if let Some(v) = arg_string(req, b"mangaUrl")? {
        if sid(v, DETAIL)? != id {
            return Err("invalid_request");
        }
    }
    Ok(())
}
fn fetch(url: &[u8], post: bool) -> R<usize> {
    let len = if post {
        build_post_request(
            http_req_buf(),
            url,
            b"q_stitle=&q_xbody=&q_xpostDate_start=&q_xpostDate_end=&nowPage=2&pageSize=12",
            b"application/x-www-form-urlencoded",
            Some(LIST),
        )
    } else {
        build_get_request(http_req_buf(), url, Some(LIST), &[])
    }
    .ok_or("capacity_exceeded")?;
    let n = host::http_request(&http_req_buf()[..len], http_out()).map_err(|_| "network_error")?;
    if extract_json_number(&http_out()[..n], b"status") != Some(b"200") {
        return Err("http_error");
    }
    decode_json_body_into(&http_out()[..n], body_buf()).map_err(|_| "network_error")
}
fn indexed(dst: &mut [u8], at: usize, suffix: &[u8]) -> R<usize> {
    let mut c = 0;
    w(
        dst,
        &mut c,
        b"section.lp > .listAlbum > .Flex-set > .thumbnail:nth-child(",
    )?;
    n(dst, &mut c, at + 1)?;
    w(dst, &mut c, b") > a[href]")?;
    w(dst, &mut c, suffix)?;
    Ok(c)
}
fn title_bytes(v: &[u8]) -> R<&[u8]> {
    let v = trim_ascii(v);
    if v.is_empty() || v.len() > 512 || core::str::from_utf8(v).is_err() {
        return Err("parse_error");
    }
    Ok(v)
}
fn cover(v: &[u8]) -> R<()> {
    let x = v.strip_prefix(COVER).ok_or("invalid_request")?;
    let x = x.strip_suffix(b".jpg").ok_or("invalid_request")?;
    if x.is_empty() || x.len() > 20 || x[0] == b'0' || !x.iter().all(|b| b.is_ascii_digit()) {
        return Err("invalid_request");
    }
    Ok(())
}
fn body_uri(v: &[u8], id: &[u8]) -> R<()> {
    let x = v.strip_prefix(IMAGE).ok_or("invalid_request")?;
    let x = x
        .strip_prefix(id)
        .and_then(|s| s.strip_prefix(b"/image/"))
        .ok_or("invalid_request")?;
    positive(x)?;
    Ok(())
}
struct Page {
    total: usize,
    pages: usize,
    records: usize,
    supported: usize,
}
fn catalogue_page(which: usize) -> R<Page> {
    let len = fetch(LIST, which == 2)?;
    if len > 262144 {
        return Err("capacity_exceeded");
    }
    let doc = host::html_parse(&body_buf()[..len]).map_err(|_| "html_error")?;
    let r = (|| {
        let mut b = [0; 1024];
        let mut sel = [0; 256];
        let l = attr(
            doc,
            b"form#contentSearchForm[action][method]",
            b"action",
            &mut b,
        )?;
        if &b[..l] != LIST {
            return Err("identity_mismatch");
        }
        let l = attr(
            doc,
            b"form#contentSearchForm[action][method]",
            b"method",
            &mut b,
        )?;
        if &b[..l] != b"post" {
            return Err("parse_error");
        }
        if count(doc, b"form#contentSearchForm input[name]")? != 6 {
            return Err("parse_error");
        }
        for (name, value) in [
            (b"q_stitle".as_slice(), b"".as_slice()),
            (b"q_xbody", b""),
            (b"q_xpostDate_start", b""),
            (b"q_xpostDate_end", b""),
            (b"nowPage", if which == 1 { b"1" } else { b"2" }),
            (b"pageSize", b"12"),
        ] {
            let mut c = 0;
            w(&mut sel, &mut c, b"form#contentSearchForm input[name='")?;
            w(&mut sel, &mut c, name)?;
            w(&mut sel, &mut c, b"'][value]")?;
            let l = attr(doc, &sel[..c], b"value", &mut b)?;
            if &b[..l] != value {
                return Err("parse_error");
            }
        }
        let l = text(doc, b".pagination .total > span:nth-of-type(1)", &mut b)?;
        let total = positive(trim_ascii(&b[..l]))?;
        if total > MAX {
            return Err("capacity_exceeded");
        }
        let l = text(doc, b".pagination .total > span:nth-of-type(2)", &mut b)?;
        let raw = trim_ascii(&b[..l]);
        let slash = find_subslice(raw, b"/").ok_or("parse_error")?;
        let current = positive(&raw[..slash])?;
        let pages = positive(&raw[slash + 1..])?;
        if current != which || pages > 2 || pages != (total + 11) / 12 {
            return Err("parse_error");
        }
        if which < pages {
            let l = attr(
                doc,
                b".pagination li.next > a[data-fn='listChangePage'][data-args]",
                b"data-args",
                &mut b,
            )?;
            if trim_ascii(&b[..l]) != b"2, 12" {
                return Err("parse_error");
            }
        } else if count(doc, b".pagination li.next")? != 0 {
            return Err("parse_error");
        }
        let records = count(doc, b"section.lp > .listAlbum > .Flex-set > .thumbnail")?;
        if records != core::cmp::min(12, total - (which - 1) * 12) {
            return Err("parse_error");
        }
        let before = unsafe { USED };
        for record_index in 0..records {
            let sl = indexed(&mut sel, record_index, b"")?;
            let l = attr(doc, &sel[..sl], b"href", &mut b)?;
            let href = trim_ascii(&b[..l]);
            let id = if let Some(v) = href.strip_prefix(DETAIL) {
                positive(v)?;
                v
            } else {
                let v = href
                    .strip_prefix(b"https://cloud.tipo.gov.tw/S400/article/")
                    .ok_or("invalid_request")?;
                positive(v)?;
                unsafe {
                    EXCLUDED += 1;
                }
                continue;
            };
            let mut idbuf = [0; 12];
            idbuf[..id.len()].copy_from_slice(id);
            let idlen = id.len();
            let stored_index = unsafe { USED };
            if stored_index >= MAX {
                return Err("capacity_exceeded");
            }
            for i in 0..stored_index {
                if &idbuf[..idlen] == unsafe { &IDS[i][..IDL[i]] } {
                    return Err("duplicate_identity");
                }
            }
            let sl = indexed(&mut sel, record_index, b" > .caption > .text")?;
            let l = text(doc, &sel[..sl], &mut b)?;
            let title = title_bytes(&b[..l])?;
            unsafe {
                IDS[stored_index] = idbuf;
                IDL[stored_index] = idlen;
                TITLES[stored_index][..title.len()].copy_from_slice(title);
                TL[stored_index] = title.len();
            }
            let sl = indexed(&mut sel, record_index, b" > .img-container > img[src]")?;
            let l = attr(doc, &sel[..sl], b"src", &mut b)?;
            let raw = trim_ascii(&b[..l]);
            let mut full = [0; 512];
            let mut c = 0;
            if raw.starts_with(b"/wSite/") {
                w(&mut full, &mut c, b"https://www.tipo.gov.tw")?;
                w(&mut full, &mut c, raw)?;
            } else {
                w(&mut full, &mut c, raw)?;
            }
            cover(&full[..c])?;
            unsafe {
                COVERS[stored_index][..c].copy_from_slice(&full[..c]);
                CL[stored_index] = c;
                USED += 1;
            }
        }
        Ok(Page {
            total,
            pages,
            records,
            supported: unsafe { USED } - before,
        })
    })();
    let close = host::html_close(doc).map_err(|_| "html_error");
    close?;
    r
}
fn reset() {
    unsafe {
        USED = 0;
        EXCLUDED = 0;
        IC = 0;
    }
}
fn load_second(first: &Page) -> R<Page> {
    if first.pages != 2 {
        return Err("invalid_request");
    }
    let second = catalogue_page(2)?;
    if second.total != first.total || second.pages != first.pages {
        return Err("catalogue_changed");
    }
    Ok(second)
}
fn find(id: Option<&[u8]>) -> R<Option<usize>> {
    reset();
    let first = catalogue_page(1)?;
    if let Some(id) = id {
        for i in 0..unsafe { USED } {
            if id == unsafe { &IDS[i][..IDL[i]] } {
                return Ok(Some(i));
            }
        }
    }
    if first.pages == 2 {
        load_second(&first)?;
    }
    if let Some(id) = id {
        for i in 0..unsafe { USED } {
            if id == unsafe { &IDS[i][..IDL[i]] } {
                return Ok(Some(i));
            }
        }
        return Err("not_found");
    }
    Ok(None)
}
fn same_words(a: &[u8], b: &[u8]) -> bool {
    let (mut i, mut j) = (0, 0);
    loop {
        i = whitespace(a, i);
        j = whitespace(b, j);
        if i == a.len() || j == b.len() {
            return i == a.len() && j == b.len();
        }
        while i < a.len() && !a[i].is_ascii_whitespace() {
            if b.get(j) != Some(&a[i]) {
                return false;
            }
            i += 1;
            j += 1;
        }
        if j < b.len() && !b[j].is_ascii_whitespace() {
            return false;
        }
    }
}
fn detail(i: usize) -> R<()> {
    let mut url = [0; 256];
    let mut c = 0;
    route(&mut url, &mut c, unsafe { &IDS[i][..IDL[i]] })?;
    let len = fetch(&url[..c], false)?;
    if len > 262144 {
        return Err("capacity_exceeded");
    }
    let doc = host::html_parse(&body_buf()[..len]).map_err(|_| "html_error")?;
    let next = (|| {
        let n = text(
            doc,
            b"script#__NEXT_DATA__[type='application/json']",
            unsafe { &mut NEXT },
        )?;
        if n >= 262144 {
            return Err("capacity_exceeded");
        }
        Ok(n)
    })();
    let close = host::html_close(doc).map_err(|_| "html_error");
    close?;
    let n = next?;
    let data = member(member(unsafe { &NEXT[..n] }, b"props")?, b"pageProps")?;
    let article = member(data, b"article")?;
    if member(article, b"id")? != unsafe { &IDS[i][..IDL[i]] } {
        return Err("identity_mismatch");
    }
    if member(member(article, b"columnistArticleCategory")?, b"id")? != b"8" {
        return Err("identity_mismatch");
    }
    let l = raw_string(string(article, b"title")?, unsafe { &mut DT })?;
    title_bytes(unsafe { &DT[..l] })?;
    unsafe {
        DTL = l;
    }
    let l = raw_string(string(article, b"author")?, unsafe { &mut AUTHOR })?;
    title_bytes(unsafe { &AUTHOR[..l] })?;
    unsafe {
        AL = l;
    }
    unsafe {
        SL = 0;
    }
    if let Some(summary) = direct(article, b"summary")? {
        if summary != b"null" {
            if summary.len() < 2 || summary[0] != b'"' || summary[summary.len() - 1] != b'"' {
                return Err("parse_error");
            }
            let l = raw_string(&summary[1..summary.len() - 1], unsafe { &mut SUMMARY })?;
            if core::str::from_utf8(unsafe { &SUMMARY[..l] }).is_err() {
                return Err("parse_error");
            }
            unsafe {
                SL = l;
            }
        }
    }
    let date = string(article, b"startDate")?;
    if date.len() != 8 || !date.iter().all(|b| b.is_ascii_digit()) {
        return Err("parse_error");
    }
    unsafe {
        DATE.copy_from_slice(date);
    }
    let content = raw_string(string(data, b"content")?, unsafe { &mut CONTENT })?;
    if content == 0 || content >= 262144 {
        return Err("capacity_exceeded");
    }
    let doc = host::html_parse(unsafe { &CONTENT[..content] }).map_err(|_| "html_error")?;
    let r = (|| {
        let mut b = [0; 1024];
        let l = text(doc, b"body > .container > h1", &mut b)?;
        let h = trim_ascii(&b[..l]);
        if !h.starts_with("IP手札mini!".as_bytes()) || !same_words(h, unsafe { &DT[..DTL] }) {
            return Err("identity_mismatch");
        }
        // ID/category establish ownership; old directory titles legitimately include 待辦事項- absent from detail title.
        let count_images = count(doc, b"body > .container > p > img[src]")?;
        if count_images == 0
            || count_images > MAX_IMAGES
            || count(doc, b"body > .container img")? != count_images
        {
            return Err("parse_error");
        }
        for at in 0..count_images {
            // select_all indexes exactly the original container p/img sequence, independent of header/logo or English links.
            let mut handles = [0u8; 64];
            let n = unsafe {
                select_all_raw(
                    doc.raw(),
                    b"body > .container > p > img[src]".as_ptr(),
                    b"body > .container > p > img[src]".len() as u32,
                    handles.as_mut_ptr(),
                    handles.len() as u32,
                )
            };
            if n != count_images as i32 {
                return Err("html_error");
            }
            let mut selected = None;
            for k in 0..count_images {
                let o = k * 4;
                let raw = i32::from_le_bytes([
                    handles[o],
                    handles[o + 1],
                    handles[o + 2],
                    handles[o + 3],
                ]);
                let d = host::HtmlDescriptor::from_raw(raw);
                let rr = if k == at {
                    Some(host::html_attr(d, b"src", &mut b))
                } else {
                    None
                };
                let closed = host::html_close(d);
                if let Some(rr) = rr {
                    selected = Some(rr);
                }
                if closed.is_err() {
                    return Err("html_error");
                }
            }
            let l = selected.ok_or("html_error")?.map_err(|_| "html_error")?;
            let u = trim_ascii(&b[..l]);
            body_uri(u, unsafe { &IDS[i][..IDL[i]] })?;
            for j in 0..at {
                if u == unsafe { &IMAGES[j][..IL[j]] } {
                    return Err("duplicate_identity");
                }
            }
            unsafe {
                IMAGES[at][..u.len()].copy_from_slice(u);
                IL[at] = u.len();
            }
        }
        unsafe {
            IC = count_images;
        }
        Ok(())
    })();
    let close = host::html_close(doc).map_err(|_| "html_error");
    close?;
    r
}
fn book(p: &mut [u8], c: &mut usize, i: usize, full: bool) -> R<()> {
    w(p, c, b"{\"id\":\"")?;
    pref(p, c, MID, unsafe { &IDS[i][..IDL[i]] })?;
    w(p, c, b"\",\"title\":")?;
    q(
        p,
        c,
        if full {
            unsafe { &DT[..DTL] }
        } else {
            unsafe { &TITLES[i][..TL[i]] }
        },
    )?;
    w(p, c, b",\"url\":\"")?;
    route(p, c, unsafe { &IDS[i][..IDL[i]] })?;
    w(p, c, b"\",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(p, c, unsafe { &COVERS[i][..CL[i]] })?;
    w(p, c, b"},\"language\":\"zh-Hant\",\"authors\":[")?;
    if full {
        q(p, c, unsafe { &AUTHOR[..AL] })?;
    }
    w(
        p,
        c,
        b"],\"artists\":[],\"status\":\"unknown\",\"contentRating\":\"unknown\",\"description\":",
    )?;
    let mut desc = [0; 12288];
    let mut d = 0;
    w(
        &mut desc,
        &mut d,
        "僅收錄智慧財產局可直達專欄的IP手札mini!漫畫。舊泛目錄尚不支援。".as_bytes(),
    )?;
    if full {
        if unsafe { SL } > 0 {
            w(&mut desc, &mut d, "\n原文簡介：".as_bytes())?;
            w(&mut desc, &mut d, unsafe { &SUMMARY[..SL] })?;
        }
        w(&mut desc, &mut d, "\n發布日期：".as_bytes())?;
        w(&mut desc, &mut d, unsafe { &DATE })?;
        w(&mut desc, &mut d, "\n原署名：".as_bytes())?;
        w(&mut desc, &mut d, unsafe { &AUTHOR[..AL] })?;
    }
    q(p, c, &desc[..d])?;
    if full {
        w(p, c, b",\"links\":[{\"kind\":\"source\",\"url\":\"")?;
        route(p, c, unsafe { &IDS[i][..IDL[i]] })?;
        w(p, c, b"\"}]")?;
    }
    w(p, c, b"}")
}
fn listing(which: usize) -> R<usize> {
    reset();
    let first = catalogue_page(1)?;
    let start = if which == 1 { 0 } else { unsafe { USED } };
    let row = if which == 1 {
        first
    } else {
        load_second(&first)?
    };
    let end = unsafe { USED };
    let p = payload_buf();
    let mut c = 0;
    w(p, &mut c, b"{\"items\":[")?;
    for i in start..end {
        if i > start {
            w(p, &mut c, b",")?;
        }
        book(p, &mut c, i, false)?;
    }
    w(p, &mut c, b"],\"page\":{\"nextCursor\":")?;
    if which < row.pages {
        w(p, &mut c, b"\"2\",\"hasMore\":true},\"hasNextPage\":true}")?;
    } else {
        w(p, &mut c, b"null,\"hasMore\":false},\"hasNextPage\":false}")?;
    }
    Ok(c)
}
fn run_get_manga_list(req: &[u8]) -> u32 {
    if arg_string(req, b"listingId").ok().flatten() != Some(b"listing:direct") {
        return write_error("get_manga_list", "invalid_request", "");
    }
    match page(req) {
        Ok(n) if n <= 2 => result("get_manga_list", listing(n)),
        _ => write_error("get_manga_list", "invalid_request", ""),
    }
}
fn work(req: &[u8], key: &[u8], prefix: &[u8]) -> R<usize> {
    let id = sid(arg_string(req, key)?.ok_or("invalid_request")?, prefix)?;
    owner(req, id)?;
    find(Some(id))?.ok_or("not_found")
}
fn run_get_manga(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let i = work(req, b"mangaId", MID)?;
        detail(i)?;
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
        let i = work(req, b"mangaId", MID)?;
        detail(i)?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"chapters\":[{\"id\":\"")?;
        pref(p, &mut c, CID, unsafe { &IDS[i][..IDL[i]] })?;
        w(p, &mut c, b"\",\"mangaId\":\"")?;
        pref(p, &mut c, MID, unsafe { &IDS[i][..IDL[i]] })?;
        w(p, &mut c, b"\",\"title\":")?;
        q(p, &mut c, unsafe { &DT[..DTL] })?;
        w(p, &mut c, b",\"url\":\"")?;
        route(p, &mut c, unsafe { &IDS[i][..IDL[i]] })?;
        w(
            p,
            &mut c,
            b"\",\"chapterNumber\":null,\"volumeNumber\":null,\"readingGroup\":\"",
        )?;
        pref(p, &mut c, MID, unsafe { &IDS[i][..IDL[i]] })?;
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
        let i = work(req, b"chapterId", CID)?;
        detail(i)?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"pages\":[")?;
        for at in 0..unsafe { IC } {
            if at > 0 {
                w(p, &mut c, b",")?;
            }
            w(p, &mut c, b"{\"id\":\"")?;
            pref(p, &mut c, PID, unsafe { &IDS[i][..IDL[i]] })?;
            w(p, &mut c, b":")?;
            n(p, &mut c, at)?;
            w(p, &mut c, b"\",\"index\":")?;
            n(p, &mut c, at)?;
            w(p, &mut c, b",\"image\":{\"kind\":\"url\",\"url\":")?;
            q(p, &mut c, unsafe { &IMAGES[at][..IL[at]] })?;
            w(p, &mut c, b"}}")?;
        }
        w(p, &mut c, b"]}")?;
        Ok(c)
    }
    result("get_pages", inner(req))
}
fn run_get_image_request(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        if let Some(v) = arg_string(req, b"mangaId")? {
            sid(v, MID)?;
        }
        if let Some(v) = arg_string(req, b"mangaUrl")? {
            sid(v, DETAIL)?;
        }

        let u = arg_string(req, b"url")?.ok_or("invalid_request")?;
        let mut ordinal = None;
        let id = if let Some(v) = arg_string(req, b"pageId")? {
            if let Some(v) = v.strip_prefix(PID) {
                let at = find_subslice(v, b":").ok_or("invalid_request")?;
                positive(&v[..at])?;
                let ix = &v[at + 1..];
                let ix = if ix == b"0" { 0 } else { positive(ix)? };
                if ix >= MAX_IMAGES {
                    return Err("invalid_request");
                }
                body_uri(u, &v[..at])?;
                ordinal = Some(ix);
                Some(&v[..at])
            } else {
                cover(u)?;
                Some(sid(v, MID)?)
            }
        } else {
            cover(u)?;
            None
        };
        if let Some(id) = id {
            owner(req, id)?;
        }
        let i = if let Some(id) = id {
            find(Some(id))?.ok_or("not_found")?
        } else {
            find(None)?;
            let mut found = None;
            for i in 0..unsafe { USED } {
                if u == unsafe { &COVERS[i][..CL[i]] } {
                    if found.is_some() {
                        return Err("invalid_request");
                    }
                    found = Some(i);
                }
            }
            found.ok_or("invalid_request")?
        };
        owner(req, unsafe { &IDS[i][..IDL[i]] })?;
        if let Some(at) = ordinal {
            detail(i)?;
            if at >= unsafe { IC } || u != unsafe { &IMAGES[at][..IL[at]] } {
                return Err("invalid_request");
            }
        } else if u != unsafe { &COVERS[i][..CL[i]] } {
            return Err("invalid_request");
        }
        let mut referer = [0; 256];
        let mut r = 0;
        if ordinal.is_some() {
            route(&mut referer, &mut r, unsafe { &IDS[i][..IDL[i]] })?;
        } else {
            w(&mut referer, &mut r, LIST)?;
        }
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"imageRequest\":{\"url\":")?;
        q(p, &mut c, u)?;
        w(p,&mut c,b",\"headers\":{\"User-Agent\":\"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0\",\"Referer\":")?;
        q(p, &mut c, &referer[..r])?;
        w(p, &mut c, b"}}}")?;
        Ok(c)
    }
    result("get_image_request", inner(req))
}
fn run_get_listings(_req: &[u8]) -> u32 {
    fn inner() -> R<usize> {
        find(None)?;
        let mut title = [0; 256];
        let mut t = 0;
        w(&mut title, &mut t, "官方直達專欄・目前".as_bytes())?;
        n(&mut title, &mut t, unsafe { USED })?;
        w(&mut title, &mut t, "篇（不含舊泛目錄）".as_bytes())?;
        let p = payload_buf();
        let mut c = 0;
        w(
            p,
            &mut c,
            b"{\"listings\":[{\"id\":\"listing:direct\",\"name\":",
        )?;
        q(p, &mut c, &title[..t])?;
        w(p, &mut c, b"}]}")?;
        Ok(c)
    }
    result("get_listings", inner())
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
