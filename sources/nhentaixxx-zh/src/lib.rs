#![no_std]
use koma_source_sdk::host::HtmlDescriptor;
use koma_source_sdk::{host, json_utils::*, source::SourceInfo};
type E = &'static str;
type R<T> = Result<T, E>;
const MAX: usize = 1048576;
const WEB: &[u8] = b"https://nhentai.xxx";

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.nhentaixxx.zh.koma",
    name: "NHentai.xxx",
    version: "0.1.0",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "Chinese-language provider catalogue. Per-work rating and source usage unknown.",
    content_rating: "adult",
};
koma_source_sdk::koma_source_buffers! {payload:1048000,http_out:MAX,body:MAX,http_req:32768,scratch:8192,}
koma_source_sdk::koma_source_helpers!();
static mut REQUEST: [u8; MAX] = [0; MAX];
fn bytes(b: &mut [u8], n: &mut usize, s: &[u8]) -> Result<(), &'static str> {
    if write_bytes(b, n, s) {
        Ok(())
    } else {
        Err("payload overflow")
    }
}
fn q(b: &mut [u8], n: &mut usize, s: &[u8]) -> Result<(), &'static str> {
    core::str::from_utf8(s).map_err(|_| "invalid UTF8")?;
    bytes(b, n, b"\"")?;
    if !append_json_escaped(b, n, s) {
        return Err("payload overflow");
    };
    bytes(b, n, b"\"")
}
fn number(s: &[u8]) -> Option<usize> {
    if s.is_empty() {
        return None;
    };
    let mut n = 0usize;
    for &c in s {
        if !c.is_ascii_digit() {
            return None;
        };
        n = n.checked_mul(10)?.checked_add((c - b'0') as usize)?;
    }
    Some(n)
}
fn ws(s: &[u8], mut p: usize) -> usize {
    while p < s.len() && matches!(s[p], b' ' | b'\t' | b'\r' | b'\n') {
        p += 1
    }
    p
}
fn json_string_end(s: &[u8], p: usize) -> Option<usize> {
    if s.get(p) != Some(&b'"') {
        return None;
    };
    let mut i = p + 1;
    while i < s.len() {
        match s[i] {
            b'"' => return Some(i + 1),
            0..=31 => return None,
            b'\\' => {
                i += 1;
                match *s.get(i)? {
                    b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't' => {}
                    b'u' => {
                        let cp = hex4(s.get(i + 1..i + 5)?)?;
                        i += 4;
                        if (0xd800..=0xdbff).contains(&cp) {
                            if s.get(i + 1..i + 3) != Some(b"\\u") {
                                return None;
                            }
                            let low = hex4(s.get(i + 3..i + 7)?)?;
                            if !(0xdc00..=0xdfff).contains(&low) {
                                return None;
                            }
                            i += 6;
                        } else if (0xdc00..=0xdfff).contains(&cp) {
                            return None;
                        }
                    }
                    _ => return None,
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}
fn json_value_end(s: &[u8], p: usize, depth: usize) -> Option<usize> {
    if depth > 32 {
        return None;
    };
    let p = ws(s, p);
    match *s.get(p)? {
        b'"' => json_string_end(s, p),
        b'{' | b'[' => {
            let object = s[p] == b'{';
            let close = if object { b'}' } else { b']' };
            let mut i = ws(s, p + 1);
            if s.get(i) == Some(&close) {
                return Some(i + 1);
            };
            loop {
                if object {
                    i = json_string_end(s, i)?;
                    i = ws(s, i);
                    if s.get(i) != Some(&b':') {
                        return None;
                    };
                    i = ws(s, i + 1)
                }
                i = json_value_end(s, i, depth + 1)?;
                i = ws(s, i);
                if s.get(i) == Some(&close) {
                    return Some(i + 1);
                };
                if s.get(i) != Some(&b',') {
                    return None;
                };
                i = ws(s, i + 1);
            }
        }
        b't' => {
            if s.get(p..p + 4) == Some(b"true") {
                Some(p + 4)
            } else {
                None
            }
        }
        b'f' => {
            if s.get(p..p + 5) == Some(b"false") {
                Some(p + 5)
            } else {
                None
            }
        }
        b'n' => {
            if s.get(p..p + 4) == Some(b"null") {
                Some(p + 4)
            } else {
                None
            }
        }
        b'-' | b'0'..=b'9' => {
            let mut i = p;
            if s[i] == b'-' {
                i += 1
            };
            if s.get(i) == Some(&b'0') {
                i += 1
            } else {
                if !matches!(s.get(i), Some(b'1'..=b'9')) {
                    return None;
                };
                while s.get(i).map(|c| c.is_ascii_digit()).unwrap_or(false) {
                    i += 1
                }
            }
            if s.get(i) == Some(&b'.') {
                i += 1;
                let start = i;
                while s.get(i).map(|c| c.is_ascii_digit()).unwrap_or(false) {
                    i += 1
                }
                if i == start {
                    return None;
                }
            }
            if matches!(s.get(i), Some(b'e' | b'E')) {
                i += 1;
                if matches!(s.get(i), Some(b'+' | b'-')) {
                    i += 1
                };
                let start = i;
                while s.get(i).map(|c| c.is_ascii_digit()).unwrap_or(false) {
                    i += 1
                }
                if i == start {
                    return None;
                }
            }
            Some(i)
        }
        _ => None,
    }
}
fn hex4(s: &[u8]) -> Option<u32> {
    if s.len() != 4 {
        return None;
    };
    let mut n = 0;
    for &c in s {
        n = n * 16
            + match c {
                b'0'..=b'9' => (c - b'0') as u32,
                b'a'..=b'f' => (c - b'a' + 10) as u32,
                b'A'..=b'F' => (c - b'A' + 10) as u32,
                _ => return None,
            };
    }
    Some(n)
}
fn unescape<'a>(s: &[u8], out: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    let mut i = 0;
    let mut n = 0;
    while i < s.len() {
        let c = s[i];
        i += 1;
        if c != b'\\' {
            if c < 32 || c == b'"' {
                return Err("invalid JSON string");
            };
            bytes(out, &mut n, &[c])?;
            continue;
        }
        let c = *s.get(i).ok_or("invalid JSON escape")?;
        i += 1;
        let c = match c {
            b'"' => b'"',
            b'\\' => b'\\',
            b'/' => b'/',
            b'n' => b'\n',
            b'r' => b'\r',
            b't' => b'\t',
            b'b' => 8,
            b'f' => 12,
            b'u' => {
                let mut cp = hex4(s.get(i..i + 4).ok_or("invalid unicode escape")?)
                    .ok_or("invalid unicode escape")?;
                i += 4;
                if (0xd800..=0xdbff).contains(&cp) {
                    if s.get(i..i + 2) != Some(b"\\u") {
                        return Err("invalid unicode pair");
                    };
                    i += 2;
                    let low = hex4(s.get(i..i + 4).ok_or("invalid unicode pair")?)
                        .ok_or("invalid unicode pair")?;
                    i += 4;
                    if !(0xdc00..=0xdfff).contains(&low) {
                        return Err("invalid unicode pair");
                    };
                    cp = 0x10000 + ((cp - 0xd800) << 10) + (low - 0xdc00)
                } else if (0xdc00..=0xdfff).contains(&cp) {
                    return Err("invalid unicode pair");
                };
                let ch = char::from_u32(cp).ok_or("invalid unicode scalar")?;
                let mut tmp = [0; 4];
                bytes(out, &mut n, ch.encode_utf8(&mut tmp).as_bytes())?;
                continue;
            }
            _ => return Err("invalid JSON escape"),
        };
        bytes(out, &mut n, &[c])?;
    }
    core::str::from_utf8(&out[..n]).map_err(|_| "invalid UTF8")?;
    Ok(&out[..n])
}

fn valid_json(s: &[u8]) -> Result<(), E> {
    core::str::from_utf8(s).map_err(|_| "invalid UTF8")?;
    if json_value_end(s, 0, 0).map(|i| ws(s, i)) != Some(s.len()) {
        return Err("invalid complete JSON");
    }
    Ok(())
}
fn member<'a>(obj: &'a [u8], key: &[u8]) -> Result<Option<&'a [u8]>, E> {
    if obj.first() != Some(&b'{') {
        return Err("expected object");
    }
    let mut i = ws(obj, 1);
    let mut found = None;
    while obj.get(i) != Some(&b'}') {
        let end = json_string_end(obj, i).ok_or("invalid object key")?;
        let matches = &obj[i + 1..end - 1] == key;
        i = ws(obj, end);
        if obj.get(i) != Some(&b':') {
            return Err("invalid object member");
        }
        i = ws(obj, i + 1);
        let end = json_value_end(obj, i, 0).ok_or("invalid object value")?;
        if matches {
            if found.is_some() {
                return Err("duplicate object member");
            }
            found = Some(&obj[i..end]);
        }
        i = ws(obj, end);
        if obj.get(i) == Some(&b',') {
            i = ws(obj, i + 1)
        } else if obj.get(i) != Some(&b'}') {
            return Err("invalid object separator");
        }
    }
    Ok(found)
}
fn required<'a>(obj: &'a [u8], k: &[u8]) -> Result<&'a [u8], E> {
    member(obj, k)?.ok_or("required field missing")
}
fn quoted(v: &[u8]) -> Result<&[u8], E> {
    if v.first() != Some(&b'"') || json_string_end(v, 0) != Some(v.len()) {
        return Err("expected string");
    }
    Ok(&v[1..v.len() - 1])
}
fn text<'a>(obj: &'a [u8], k: &[u8]) -> Result<&'a [u8], E> {
    quoted(required(obj, k)?)
}
#[allow(dead_code)]
fn raw_string<'a>(obj: &'a [u8], k: &[u8]) -> Result<&'a [u8], E> {
    let v = required(obj, k)?;
    quoted(v)?;
    Ok(v)
}
fn args(r: &[u8]) -> Result<&[u8], E> {
    valid_json(r)?;
    if let Some(s) = member(r, b"sourceId")? {
        let value = quoted(s)?;
        let image_host_selected = value.is_empty()
            && member(r, b"operation")?.and_then(|v| quoted(v).ok())
                == Some(b"get_image_request".as_slice());
        if value != SOURCE_INFO.id.as_bytes() && !image_host_selected {
            return Err("request source identity");
        }
    }
    if let Some(v) = member(r, b"version")? {
        if num(v)? != 1 {
            return Err("request version");
        }
    }
    let a = member(r, b"args")?.unwrap_or(r);
    if a.first() != Some(&b'{') {
        return Err("args object");
    }
    Ok(a)
}
fn scalar(v: &[u8]) -> Result<&[u8], E> {
    if v.first() == Some(&b'"') {
        quoted(v)
    } else {
        Ok(v)
    }
}
fn num(v: &[u8]) -> Result<usize, E> {
    number(scalar(v)?).ok_or("invalid integer")
}

fn wn(b: &mut [u8], n: &mut usize, x: usize) -> Result<(), E> {
    if write_usize(b, n, x) {
        Ok(())
    } else {
        Err("payload overflow")
    }
}

fn uq<'a>(v: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    unescape(quoted(v)?, b)
}
#[link(wasm_import_module = "koma_host")]
extern "C" {
    #[link_name = "html_select_all"]
    fn all_raw(d: i32, s: *const u8, n: u32, o: *mut u8, c: u32) -> i32;
}
struct Owned(HtmlDescriptor);
impl Drop for Owned {
    fn drop(&mut self) {
        let _ = host::html_close(self.0);
    }
}
struct All {
    ids: [i32; 1000],
    len: usize,
}
impl All {
    fn new(d: HtmlDescriptor, s: &[u8]) -> R<Self> {
        let mut a = Self {
            ids: [0; 1000],
            len: 0,
        };
        let count = unsafe {
            all_raw(
                d.raw(),
                s.as_ptr(),
                s.len() as u32,
                a.ids.as_mut_ptr() as *mut u8,
                4000,
            )
        };
        if count < 0 {
            return Err("parse_error");
        }
        a.len = (count as usize).min(1000);
        if count as usize > 1000 {
            return Err("capacity_exceeded");
        }
        if a.ids[..a.len].iter().any(|x| *x <= 0) {
            return Err("parse_error");
        }
        Ok(a)
    }
    fn at(&self, i: usize) -> HtmlDescriptor {
        HtmlDescriptor::from_raw(self.ids[i])
    }
}
impl Drop for All {
    fn drop(&mut self) {
        for id in &self.ids[..self.len] {
            if *id > 0 {
                let _ = host::html_close(HtmlDescriptor::from_raw(*id));
            }
        }
    }
}
fn attr<'a>(d: HtmlDescriptor, k: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    let n = host::html_attr(d, k, b).map_err(|_| "parse_error")?;
    if n >= b.len() {
        return Err("capacity_exceeded");
    }
    core::str::from_utf8(&b[..n]).map_err(|_| "attribute UTF8")?;
    Ok(&b[..n])
}
fn txt<'a>(d: HtmlDescriptor, b: &'a mut [u8]) -> R<&'a [u8]> {
    let mut raw = [0u8; 8192];
    let z = host::html_text(d, &mut raw).map_err(|_| "text capacity or parse")?;
    if z >= raw.len() {
        return Err("text capacity");
    };
    core::str::from_utf8(&raw[..z]).map_err(|_| "text UTF8")?;
    let mut n = 0;
    let mut pending = false;
    for c in &raw[..z] {
        if c.is_ascii_whitespace() {
            if n > 0 {
                pending = true
            }
        } else {
            if pending {
                bytes(b, &mut n, b" ")?;
                pending = false
            }
            bytes(b, &mut n, &[*c])?
        }
    }
    Ok(&b[..n])
}
fn sa<'a>(d: HtmlDescriptor, s: &[u8], k: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    let x = Owned(host::html_select(d, s).map_err(|_| "missing_field")?);
    attr(x.0, k, b)
}
fn st<'a>(d: HtmlDescriptor, s: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    let x = Owned(host::html_select(d, s).map_err(|_| "missing_field")?);
    txt(x.0, b)
}
const UA:&[u8]=b"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/104.0.0.0 Safari/537.36";
// Canonical metadata only. The reader's declared data-src owns the asset host.
static mut BODY_LEN: usize = 0;
fn fetch(url: &[u8]) -> R<Owned> {
    if host::check_cancel() {
        return Err("cancelled");
    }
    if !url.starts_with(b"https://nhentai.xxx/") {
        return Err("metadata host");
    }
    let b = http_req_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"version\":1,\"method\":\"GET\",\"url\":")?;
    q(b, &mut n, url)?;
    bytes(b,&mut n,b",\"headers\":{\"Referer\":\"https://nhentai.xxx/\",\"Origin\":\"https://nhentai.xxx\",\"Cache-Control\":\"max-age=600\",\"User-Agent\":")?;
    q(b, &mut n, UA)?;
    bytes(
        b,
        &mut n,
        b"},\"timeoutMs\":15000,\"responseKind\":\"bodyText\"}",
    )?;
    let z = host::http_request(&b[..n], http_out()).map_err(|_| "transport error")?;
    if z >= MAX {
        return Err("HTTP capacity");
    }
    let e = &http_out()[..z];
    valid_json(e)?;
    let status = num(member(e, b"status")?
        .or(member(e, b"statusCode")?)
        .ok_or("HTTP status")?)?;
    if !(200..300).contains(&status) {
        return Err("HTTP status error");
    }
    let z = koma_source_sdk::decode_json_body_into(e, body_buf()).map_err(|_| "HTTP decode")?;
    if z >= MAX {
        return Err("body capacity");
    }
    core::str::from_utf8(&body_buf()[..z]).map_err(|_| "HTML UTF8")?;
    unsafe {
        BODY_LEN = z;
    }
    Ok(Owned(
        host::html_parse(&body_buf()[..z]).map_err(|_| "HTML parse")?,
    ))
}
fn make<'a>(ps: &[&[u8]], b: &'a mut [u8]) -> R<&'a [u8]> {
    let mut n = 0;
    for p in ps {
        bytes(b, &mut n, p)?;
    }
    Ok(&b[..n])
}
fn ident(s: &[u8]) -> R<&[u8]> {
    if s.is_empty() || s.len() > 20 || s[0] == b'0' || number(s).filter(|n| *n > 0).is_none() {
        Err("invalid identity")
    } else {
        Ok(s)
    }
}
fn key<'a>(r: &'a [u8], f: &[u8], prefix: &[u8]) -> R<&'a [u8]> {
    ident(
        text(args(r)?, f)?
            .strip_prefix(prefix)
            .ok_or("identity namespace")?,
    )
}
fn absolute<'a>(s: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    if s.starts_with(b"//nhentai.xxx/") {
        make(&[b"https:", s], b)
    } else if s.starts_with(b"/") && !s.starts_with(b"//") {
        make(&[WEB, s], b)
    } else if s.starts_with(b"https://nhentai.xxx/") {
        make(&[s], b)
    } else {
        Err("route host")
    }
}
fn work_url<'a>(id: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    ident(id)?;
    make(&[WEB, b"/g/", id, b"/"], b)
}
fn work_href(s: &[u8]) -> R<&[u8]> {
    ident(
        s.strip_prefix(b"https://nhentai.xxx/g/")
            .and_then(|s| s.strip_suffix(b"/"))
            .ok_or("work URL route")?,
    )
}
fn token(s: &[u8]) -> R<&[u8]> {
    if s.is_empty() || s.len() > 40 || !s.iter().all(|c| c.is_ascii_alphanumeric()) {
        Err("asset identity")
    } else {
        Ok(s)
    }
}
fn directory(s: &[u8]) -> R<&[u8]> {
    if s.len() != 3 || !s.iter().all(|c| c.is_ascii_digit()) {
        Err("asset directory")
    } else {
        Ok(s)
    }
}
fn server(s: &[u8]) -> R<&[u8]> {
    if matches!(s, b"2" | b"4" | b"5") {
        Ok(s)
    } else {
        Err("unobserved asset host")
    }
}
struct Asset<'a> {
    server: &'a [u8],
    dir: &'a [u8],
    id: &'a [u8],
    file: &'a [u8],
}
fn asset(s: &[u8]) -> R<Asset<'_>> {
    let p = s.strip_prefix(b"https://i").ok_or("asset scheme")?;
    let sep = p.iter().position(|c| *c == b'.').ok_or("asset host")?;
    let sv = server(&p[..sep])?;
    let p = p[sep..]
        .strip_prefix(b".nhentaimg.com/")
        .ok_or("asset domain")?;
    let sep = p.iter().position(|c| *c == b'/').ok_or("asset directory")?;
    let dir = directory(&p[..sep])?;
    let p = &p[sep + 1..];
    let sep = p.iter().position(|c| *c == b'/').ok_or("asset identity")?;
    let id = token(&p[..sep])?;
    let file = &p[sep + 1..];
    if file.is_empty()
        || file.len() > 40
        || !file.iter().all(|c| c.is_ascii_alphanumeric() || *c == b'.')
        || file == b"."
        || file == b".."
    {
        return Err("asset filename");
    }
    Ok(Asset {
        server: sv,
        dir,
        id,
        file,
    })
}
fn input<'a>(d: HtmlDescriptor, s: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    let all = All::new(d, s)?;
    if all.len != 1 {
        return Err("input missing or duplicate");
    }
    attr(all.at(0), b"value", b)
}
fn count(d: HtmlDescriptor, selector: &[u8]) -> R<usize> {
    let mut b = [0; 32];
    let s = input(d, selector, &mut b)?;
    let n = number(s).ok_or("page count")?;
    if n == 0 || n > 1000 || s[0] == b'0' {
        Err("page capacity")
    } else {
        Ok(n)
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
struct PageData {
    kind: u8,
    width: usize,
    height: usize,
}
const EMPTY: PageData = PageData {
    kind: 0,
    width: 0,
    height: 0,
};
struct Gallery {
    pages: [PageData; 1000],
    count: usize,
    sv: [u8; 1],
    dir: [u8; 3],
    asset: [u8; 40],
    alen: usize,
}
fn extension(kind: u8) -> R<&'static [u8]> {
    match kind {
        b'j' => Ok(b"jpg"),
        b'w' => Ok(b"webp"),
        _ => Err("unobserved page format"),
    }
}
fn fl_records(total: usize) -> R<[PageData; 1000]> {
    // Consume an ordinary JSON literal, never execute the surrounding script.
    let html = &body_buf()[..unsafe { BODY_LEN }];
    let marker = b"$.parseJSON('";
    let start = find_subslice(html, marker).ok_or("full page map missing")? + marker.len();
    if find_subslice(&html[start..], marker).is_some() {
        return Err("ambiguous page map");
    }
    let end = find_subslice(&html[start..], b"');").ok_or("page map boundary")? + start;
    let json = &html[start..end];
    valid_json(json)?;
    let fl = required(json, b"fl")?;
    if fl.first() != Some(&b'{') {
        return Err("page map object");
    }
    let mut out = [EMPTY; 1000];
    let mut seen = [false; 1000];
    let mut got = 0;
    let mut p = ws(fl, 1);
    while fl.get(p) != Some(&b'}') {
        let e = json_string_end(fl, p).ok_or("page map key")?;
        let k = ident(&fl[p + 1..e - 1])?;
        let index = number(k).ok_or("page sequence")?;
        if index > total {
            return Err("page count mismatch");
        }
        if seen[index - 1] {
            return Err("duplicate page");
        }
        seen[index - 1] = true;
        p = ws(fl, e);
        if fl.get(p) != Some(&b':') {
            return Err("page map member");
        }
        p = ws(fl, p + 1);
        let e = json_value_end(fl, p, 0).ok_or("page map value")?;
        let value = quoted(&fl[p..e])?;
        let mut fields = value.split(|c| *c == b',');
        let kind = fields.next().ok_or("page type")?;
        if kind.len() != 1 {
            return Err("page type");
        }
        extension(kind[0])?;
        let width = number(fields.next().ok_or("page width")?).ok_or("page width")?;
        let height = number(fields.next().ok_or("page height")?).ok_or("page height")?;
        if fields.next().is_some() || width == 0 || height == 0 || width > 100000 || height > 100000
        {
            return Err("page dimensions");
        }
        out[index - 1] = PageData {
            kind: kind[0],
            width,
            height,
        };
        got += 1;
        p = ws(fl, e);
        if fl.get(p) == Some(&b',') {
            p = ws(fl, p + 1);
        } else if fl.get(p) != Some(&b'}') {
            return Err("page map separator");
        }
    }
    if got != total || seen[..total].iter().any(|b| !*b) {
        return Err("incomplete page map");
    }
    Ok(out)
}
fn detail_doc(id: &[u8]) -> R<Owned> {
    let mut b = [0; 512];
    let d = fetch(work_url(id, &mut b)?)?;
    let mut a = [0; 64];
    let mut c = [0; 64];
    if input(d.0, b"input#gallery_id", &mut a)? != id
        || st(d.0, b".info .gallery_id", &mut c)? != id
    {
        return Err("detail work owner");
    }
    let mut l = [0; 128];
    if st(d.0, b".info a[href='/language/chinese/'] .tag_name", &mut l)? != b"chinese" {
        return Err("Chinese language declaration");
    }
    Ok(d)
}
fn gallery(d: HtmlDescriptor, reader: bool) -> R<Gallery> {
    let total = count(
        d,
        if reader {
            b"input#pages"
        } else {
            b"input#load_pages"
        },
    )?;
    let mut sb = [0; 8];
    let sv = server(input(
        d,
        if reader {
            b"input#server_id"
        } else {
            b"input#load_server"
        },
        &mut sb,
    )?)?;
    let mut db = [0; 16];
    let dir = directory(input(
        d,
        if reader {
            b"input#image_dir"
        } else {
            b"input#load_dir"
        },
        &mut db,
    )?)?;
    let mut ib = [0; 64];
    let aid = token(input(
        d,
        if reader {
            b"input#gallery_id"
        } else {
            b"input#load_id"
        },
        &mut ib,
    )?)?;
    let mut g = Gallery {
        pages: fl_records(total)?,
        count: total,
        sv: [sv[0]],
        dir: [0; 3],
        asset: [0; 40],
        alen: aid.len(),
    };
    g.dir.copy_from_slice(dir);
    g.asset[..g.alen].copy_from_slice(aid);
    Ok(g)
}
fn cover<'a>(d: HtmlDescriptor, b: &'a mut [u8]) -> R<&'a [u8]> {
    let c = sa(d, b".cover img[data-src]", b"data-src", b)?;
    let a = asset(c)?;
    if a.file != b"cover.jpg" {
        return Err("cover template");
    }
    Ok(c)
}
fn emit_work(o: &mut [u8], n: &mut usize, id: &[u8], t: &[u8], c: &[u8]) -> R<()> {
    let mut b = [0; 512];
    let uri = work_url(id, &mut b)?;
    bytes(o, n, b"{\"id\":\"nhx:zh:")?;
    bytes(o, n, id)?;
    bytes(o, n, b"\",\"title\":")?;
    q(o, n, t)?;
    bytes(o, n, b",\"url\":")?;
    q(o, n, uri)?;
    bytes(o, n, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(o, n, c)?;
    bytes(
        o,
        n,
        b"},\"contentRating\":\"unknown\",\"links\":[{\"kind\":\"source\",\"url\":",
    )?;
    q(o, n, uri)?;
    bytes(o, n, b"}]}")
}
fn cursor_page(s: &[u8]) -> R<usize> {
    let p = s
        .strip_prefix(b"https://nhentai.xxx/language/chinese/popular/?page=")
        .ok_or("listing cursor owner")?;
    ident(p)?;
    let n = number(p).ok_or("listing cursor")?;
    if n > 10000 {
        Err("listing page capacity")
    } else {
        Ok(n)
    }
}
fn listing(r: &[u8]) -> R<usize> {
    let a = args(r)?;
    if text(a, b"listingId")? != b"listing:popular" {
        return Err("unsupported listing");
    }
    let mut ub = [0; 2048];
    let mut current = 1;
    let uri = if let Some(p) = member(a, b"page")? {
        match member(p, b"cursor")? {
            Some(v) if v != b"null" => {
                let s = uq(v, &mut ub)?;
                current = cursor_page(s)?;
                s
            }
            _ => b"https://nhentai.xxx/language/chinese/popular/?page=1",
        }
    } else {
        b"https://nhentai.xxx/language/chinese/popular/?page=1"
    };
    let d = fetch(uri)?;
    let cards = All::new(d.0, b".galleries_box .gallery_item, .fav_item")?;
    let next = All::new(
        d.0,
        b".pagination li.active + li:not(.disabled) a, a[rel=next]",
    )?;
    if next.len > 1 {
        return Err("listing next ambiguity");
    }
    let mut nh = [0; 2048];
    let mut nb = [0; 2048];
    let nxt = if next.len == 1 {
        let v = absolute(attr(next.at(0), b"href", &mut nh)?, &mut nb)?;
        if cursor_page(v)? != current.checked_add(1).ok_or("page overflow")? {
            return Err("listing next discontinuity");
        }
        Some(v)
    } else {
        None
    };
    let mut seen = [[0; 20]; 1000];
    let mut lens = [0; 1000];
    let mut written = 0;
    let o = payload_buf();
    let mut n = 0;
    bytes(o, &mut n, b"{\"items\":[")?;
    for i in 0..cards.len {
        let d = cards.at(i);
        let mut lb = [0; 64];
        if attr(d, b"data-languages", &mut lb)? != b"3" {
            continue;
        }
        let mut h = [0; 1024];
        let mut full = [0; 1024];
        let uri = absolute(sa(d, b"a[href]", b"href", &mut h)?, &mut full)?;
        let id = work_href(uri)?;
        if (0..written).any(|j| &seen[j][..lens[j]] == id) {
            return Err("duplicate Chinese work");
        }
        seen[written][..id.len()].copy_from_slice(id);
        lens[written] = id.len();
        let mut tb = [0; 4096];
        let t = st(d, b".caption", &mut tb)?;
        if t.is_empty() {
            return Err("card title");
        }
        let mut cb = [0; 2048];
        let c = sa(d, b"img[data-src]", b"data-src", &mut cb)?;
        let ac = asset(c)?;
        if ac.file != b"thumb.jpg" {
            return Err("card cover template");
        }
        if written > 0 {
            bytes(o, &mut n, b",")?;
        }
        emit_work(o, &mut n, id, t, c)?;
        written += 1;
    }
    if cards.len == 0 && nxt.is_some() {
        return Err("listing no progress");
    }
    bytes(o, &mut n, b"],\"page\":{\"hasMore\":")?;
    bytes(o, &mut n, if nxt.is_some() { b"true" } else { b"false" })?;
    bytes(o, &mut n, b",\"nextCursor\":")?;
    if let Some(v) = nxt {
        q(o, &mut n, v)?;
    } else {
        bytes(o, &mut n, b"null")?;
    }
    bytes(o, &mut n, b"}}")?;
    Ok(n)
}
fn detail(r: &[u8]) -> R<usize> {
    let id = key(r, b"mangaId", b"nhx:zh:")?;
    let d = detail_doc(id)?;
    let g = gallery(d.0, false)?;
    let mut tb = [0; 4096];
    let t = st(d.0, b".info h1", &mut tb)?;
    if t.is_empty() {
        return Err("detail title");
    }
    let mut cb = [0; 2048];
    let c = cover(d.0, &mut cb)?;
    let ac = asset(c)?;
    if ac.server != g.sv || ac.dir != g.dir || ac.id != &g.asset[..g.alen] {
        return Err("detail cover owner");
    }
    let authors = All::new(d.0, b".info a[href^='/artist/'] .tag_name")?;
    let tags = All::new(d.0, b".info a[href^='/tag/'] .tag_name")?;
    let o = payload_buf();
    let mut n = 0;
    bytes(o, &mut n, b"{\"manga\":")?;
    emit_work(o, &mut n, id, t, c)?;
    n -= 1;
    bytes(o, &mut n, b",\"status\":\"unknown\",\"authors\":[")?;
    for i in 0..authors.len {
        let mut b = [0; 1024];
        let v = txt(authors.at(i), &mut b)?;
        if v.is_empty() {
            return Err("author empty");
        }
        if i > 0 {
            bytes(o, &mut n, b",")?;
        }
        q(o, &mut n, v)?;
    }
    bytes(
        o,
        &mut n,
        b"],\"description\":\"Language: Chinese (provider declaration); Pages: ",
    )?;
    wn(o, &mut n, g.count)?;
    bytes(o, &mut n, b"\",\"tags\":[")?;
    for i in 0..tags.len {
        let mut b = [0; 1024];
        let v = txt(tags.at(i), &mut b)?;
        if v.is_empty() {
            return Err("tag empty");
        }
        if i > 0 {
            bytes(o, &mut n, b",")?;
        }
        q(o, &mut n, v)?;
    }
    bytes(o, &mut n, b"]}}")?;
    Ok(n)
}
fn chapters(r: &[u8]) -> R<usize> {
    let id = key(r, b"mangaId", b"nhx:zh:")?;
    let d = detail_doc(id)?;
    gallery(d.0, false)?;
    let mut tb = [0; 4096];
    let t = st(d.0, b".info h1", &mut tb)?;
    if t.is_empty() {
        return Err("chapter title");
    }
    let o = payload_buf();
    let mut n = 0;
    bytes(o, &mut n, b"{\"chapters\":[{\"id\":\"nhxgallery:zh:")?;
    bytes(o, &mut n, id)?;
    bytes(o, &mut n, b"\",\"mangaId\":\"nhx:zh:")?;
    bytes(o, &mut n, id)?;
    bytes(o, &mut n, b"\",\"title\":")?;
    q(o, &mut n, t)?;
    bytes(
        o,
        &mut n,
        b",\"chapterNumber\":null,\"volumeNumber\":null,\"language\":\"zh\"}]}",
    )?;
    Ok(n)
}
fn page_url<'a>(g: &Gallery, i: usize, b: &'a mut [u8]) -> R<&'a [u8]> {
    if i >= g.count {
        return Err("page range");
    }
    let mut n = 0;
    bytes(b, &mut n, b"https://i")?;
    bytes(b, &mut n, &g.sv)?;
    bytes(b, &mut n, b".nhentaimg.com/")?;
    bytes(b, &mut n, &g.dir)?;
    bytes(b, &mut n, b"/")?;
    bytes(b, &mut n, &g.asset[..g.alen])?;
    bytes(b, &mut n, b"/")?;
    wn(b, &mut n, i + 1)?;
    bytes(b, &mut n, b".")?;
    bytes(b, &mut n, extension(g.pages[i].kind)?)?;
    Ok(&b[..n])
}
fn pages(r: &[u8]) -> R<usize> {
    let id = key(r, b"chapterId", b"nhxgallery:zh:")?;
    let d = detail_doc(id)?;
    let g = gallery(d.0, false)?;
    let mut h = [0; 1024];
    let href = sa(d.0, b".cover a[href]", b"href", &mut h)?;
    let mut ru = [0; 1024];
    let uri = absolute(href, &mut ru)?;
    let mut expected = [0; 1024];
    if uri != make(&[WEB, b"/g/", id, b"/1/"], &mut expected)? {
        return Err("first reader route");
    }
    drop(d);
    let d = fetch(uri)?;
    let rg = gallery(d.0, true)?;
    if g.count != rg.count
        || g.sv != rg.sv
        || g.dir != rg.dir
        || g.alen != rg.alen
        || g.asset != rg.asset
        || g.pages[..g.count] != rg.pages[..rg.count]
    {
        return Err("reader gallery contract mismatch");
    }
    let backs = All::new(d.0, b"a.back_btn[href]")?;
    if backs.len == 0 {
        return Err("reader work owner missing");
    }
    for i in 0..backs.len {
        let mut b = [0; 1024];
        let mut full = [0; 1024];
        let mut exp = [0; 1024];
        if absolute(attr(backs.at(i), b"href", &mut b)?, &mut full)? != work_url(id, &mut exp)? {
            return Err("reader work owner");
        }
    }
    let mut ib = [0; 2048];
    let img = sa(d.0, b"img#fimg", b"data-src", &mut ib)?;
    asset(img)?;
    let mut pb = [0; 1024];
    if img != page_url(&g, 0, &mut pb)? {
        return Err("first original image contract");
    }
    let mut cp = [0; 64];
    if sa(d.0, b"img#fimg", b"c_page", &mut cp)? != b"image_1" {
        return Err("reader first page");
    }
    let o = payload_buf();
    let mut n = 0;
    bytes(o, &mut n, b"{\"pages\":[")?;
    for i in 0..g.count {
        if host::check_cancel() {
            return Err("cancelled");
        }
        if i > 0 {
            bytes(o, &mut n, b",")?;
        }
        bytes(o, &mut n, b"{\"id\":\"nhxpage:zh:")?;
        bytes(o, &mut n, id)?;
        bytes(o, &mut n, b":")?;
        bytes(o, &mut n, &g.sv)?;
        bytes(o, &mut n, b":")?;
        bytes(o, &mut n, &g.dir)?;
        bytes(o, &mut n, b":")?;
        bytes(o, &mut n, &g.asset[..g.alen])?;
        bytes(o, &mut n, b":")?;
        wn(o, &mut n, i)?;
        bytes(o, &mut n, b":")?;
        bytes(o, &mut n, &[g.pages[i].kind])?;
        bytes(o, &mut n, b"\",\"index\":")?;
        wn(o, &mut n, i)?;
        bytes(o, &mut n, b",\"image\":{\"kind\":\"url\",\"url\":")?;
        let mut b = [0; 1024];
        q(o, &mut n, page_url(&g, i, &mut b)?)?;
        bytes(o, &mut n, b"}}")?;
    }
    bytes(o, &mut n, b"]}")?;
    Ok(n)
}
fn image_request(r: &[u8]) -> R<usize> {
    let a = args(r)?;
    let mut ub = [0; 2048];
    let uri = uq(required(a, b"url")?, &mut ub)?;
    let mut pb = [0; 256];
    let pid = uq(required(a, b"pageId")?, &mut pb)?;
    let mut rb = [0; 1024];
    let route = uq(required(a, b"mangaUrl")?, &mut rb)?;
    let ac = asset(uri)?;
    let id;
    if let Some(p) = pid.strip_prefix(b"nhxpage:zh:") {
        let mut f = p.split(|c| *c == b':');
        id = ident(f.next().ok_or("page owner")?)?;
        let sv = server(f.next().ok_or("page server")?)?;
        let dir = directory(f.next().ok_or("page directory")?)?;
        let aid = token(f.next().ok_or("page asset")?)?;
        let idx = f.next().ok_or("page index")?;
        let index = number(idx).ok_or("page index")?;
        if index >= 1000 || (idx.len() > 1 && idx[0] == b'0') {
            return Err("page index capacity");
        }
        let kind = f.next().ok_or("page format")?;
        if kind.len() != 1 || f.next().is_some() {
            return Err("page identity");
        }
        let mut file = [0; 32];
        let mut n = 0;
        wn(&mut file, &mut n, index + 1)?;
        bytes(&mut file, &mut n, b".")?;
        bytes(&mut file, &mut n, extension(kind[0])?)?;
        if ac.server != sv || ac.dir != dir || ac.id != aid || ac.file != &file[..n] {
            return Err("image asset owner or sequence");
        }
    } else {
        id = ident(pid.strip_prefix(b"nhx:zh:").ok_or("cover identity")?)?;
        if !matches!(ac.file, b"cover.jpg" | b"thumb.jpg") {
            return Err("cover format");
        }
    }
    let mut b = [0; 1024];
    if route != work_url(id, &mut b)? {
        return Err("image independent work route");
    }
    let o = payload_buf();
    let mut n = 0;
    bytes(o, &mut n, b"{\"imageRequest\":{\"method\":\"GET\",\"url\":")?;
    q(o, &mut n, uri)?;
    bytes(o,&mut n,b",\"headers\":{\"Referer\":\"https://nhentai.xxx/\",\"Origin\":\"https://nhentai.xxx\",\"Cache-Control\":\"max-age=600\",\"User-Agent\":")?;
    q(o, &mut n, UA)?;
    bytes(o, &mut n, b"}}}")?;
    Ok(n)
}
fn execute(op: &str, r: R<usize>) -> u32 {
    match r {
        Ok(n) => write_success_payload(op, n),
        Err(e) => write_error(op, "source_contract_error", e),
    }
}
fn run_search(_: &[u8]) -> u32 {
    write_error(
        "search",
        "not_implemented",
        "search actual contract unverified",
    )
}
fn run_get_manga_list(r: &[u8]) -> u32 {
    execute("get_manga_list", listing(r))
}
fn run_get_manga(r: &[u8]) -> u32 {
    execute("get_manga", detail(r))
}
fn run_get_chapters(r: &[u8]) -> u32 {
    execute("get_chapters", chapters(r))
}
fn run_get_pages(r: &[u8]) -> u32 {
    execute("get_pages", pages(r))
}
fn run_get_image_request(r: &[u8]) -> u32 {
    execute("get_image_request", image_request(r))
}
fn run_get_listings(_: &[u8]) -> u32 {
    response_buffer().write_success("get_listings",b"{\"listings\":[{\"id\":\"listing:popular\",\"name\":\"Popular (Chinese)\",\"supportsPaging\":true}]}")
}
fn run_get_home(_: &[u8]) -> u32 {
    write_error("get_home", "not_implemented", "unsupported")
}
fn run_get_filters(_: &[u8]) -> u32 {
    write_error("get_filters", "not_implemented", "unsupported")
}
fn run_get_settings(_: &[u8]) -> u32 {
    write_error("get_settings", "not_implemented", "unsupported")
}
fn checked_request(ptr: u32, len: u32) -> Option<&'static [u8]> {
    let cap = core::arch::wasm32::memory_size::<0>().checked_mul(65536)?;
    let start = ptr as usize;
    let end = start.checked_add(len as usize)?;
    if start == 0 || len == 0 || len as usize >= MAX || end > cap {
        return None;
    };
    read_request(ptr, len)
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
    response_buffer().write_success("source_info","{\"sourceInfo\":{\"id\":\"com.nhentaixxx.zh.koma\",\"name\":\"NHentai.xxx\",\"version\":\"0.1.0\",\"apiVersion\":\"0.2\",\"language\":\"zh\",\"author\":\"Koma\",\"description\":\"Chinese-language provider catalogue. Per-work rating and source usage unknown.\",\"contentRating\":\"adult\",\"localizedNames\":{\"zh-Hans\":\"NHentai.xxx（中文）\",\"zh-Hant\":\"NHentai.xxx（中文）\",\"en\":\"NHentai.xxx (Chinese)\"}},\"capabilities\":{\"search\":false,\"mangaDetail\":true,\"chapters\":true,\"pages\":true,\"listings\":true,\"mangaList\":true,\"imageRequest\":true,\"home\":false,\"filters\":false,\"settings\":false,\"future\":{\"login\":false,\"auth\":false,\"process_page_image\":false,\"page_description\":false,\"base_url\":false,\"deeplink\":false,\"migration\":false}}}".as_bytes())
}
macro_rules! export_op {
    ($f:ident,$op:literal,$run:ident) => {
        #[no_mangle]
        pub extern "C" fn $f(ptr: u32, len: u32) -> u32 {
            match checked_request(ptr, len) {
                Some(r) => match args(r) {
                    Ok(_) => match member(r, b"operation") {
                        Ok(Some(v)) if quoted(v).ok() != Some($op.as_bytes()) => {
                            write_error($op, "invalid_request", "operation mismatch")
                        }
                        Err(_) => write_error($op, "invalid_request", "malformed request"),
                        _ => $run(r),
                    },
                    Err(e) => write_error($op, "invalid_request", e),
                },
                None => write_error($op, "invalid_request", "invalid ABI request range"),
            }
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
#[no_mangle]
pub extern "C" fn koma_source_alloc(len: u32) -> u32 {
    if len == 0 || len as usize >= MAX {
        0
    } else {
        core::ptr::addr_of_mut!(REQUEST) as *mut u8 as u32
    }
}
#[no_mangle]
pub extern "C" fn koma_source_free(ptr: u32) {
    response_buffer().free(ptr)
}
