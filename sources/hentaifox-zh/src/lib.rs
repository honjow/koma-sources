#![no_std]
// Independent published anonymous Chinese metadata source. No auth/retry/dummy.
use koma_source_sdk::host::HtmlDescriptor;
use koma_source_sdk::{
    host,
    json_utils::*,
    source::{SourceCapabilities, SourceInfo},
};
type E = &'static str;
type R<T> = Result<T, E>;
const MAX: usize = 1048576;
const WEB: &[u8] = b"https://hentaifox.com";
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.hentaifox.zh.koma",
    name: "HentaiFox",
    version: "0.1.1",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "HentaiFox provider Chinese catalogue. Per-work rating and usage unknown.",
    content_rating: "nsfw",
};
#[allow(dead_code)]
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: true,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: true,
    manga_list: true,
    image_request: true,
    home: false,
    filters: false,
    settings: false,
    credentials: false,
};
koma_source_sdk::koma_source_buffers! {payload:1048000,http_out:MAX,body:MAX,http_req:32768,scratch:8192,}
koma_source_sdk::koma_source_helpers!();
static mut REQUEST: [u8; MAX] = [0; MAX];
static mut PAGES: [[u8; 1024]; 1024] = [[0; 1024]; 1024];
static mut PAGE_LEN: [usize; 1024] = [0; 1024];
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
    Ok(member(r, b"args")?.unwrap_or(r))
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
    let n = host::html_text(d, b).map_err(|_| "parse_error")?;
    if n >= b.len() {
        return Err("capacity_exceeded");
    }
    Ok(trim_ascii(&b[..n]))
}
fn sa<'a>(d: HtmlDescriptor, s: &[u8], k: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    let x = Owned(host::html_select(d, s).map_err(|_| "missing_field")?);
    attr(x.0, k, b)
}
fn st<'a>(d: HtmlDescriptor, s: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    let x = Owned(host::html_select(d, s).map_err(|_| "missing_field")?);
    txt(x.0, b)
}
const UA: &[u8] =
    b"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/121.0";
static mut BODY_LEN: usize = 0;
fn fetch(url: &[u8]) -> R<Owned> {
    if host::check_cancel() {
        return Err("cancelled");
    };
    if !url.starts_with(b"https://hentaifox.com/") {
        return Err("metadata host");
    };
    let d = http_req_buf();
    let mut n = 0;
    bytes(d, &mut n, b"{\"version\":1,\"url\":")?;
    q(d, &mut n, url)?;
    bytes(d,&mut n,b",\"method\":\"GET\",\"headers\":{\"Referer\":\"https://hentaifox.com/\",\"Origin\":\"https://hentaifox.com\",\"Cache-Control\":\"max-age=600\",\"User-Agent\":")?;
    q(d, &mut n, UA)?;
    bytes(
        d,
        &mut n,
        b"},\"timeoutMs\":15000,\"responseKind\":\"bodyText\"}",
    )?;
    let z = host::http_request(&d[..n], http_out()).map_err(|_| "transport error")?;
    if z >= MAX {
        return Err("HTTP envelope capacity");
    };
    let e = &http_out()[..z];
    valid_json(e)?;
    let st = num(member(e, b"status")?
        .or(member(e, b"statusCode")?)
        .ok_or("HTTP status missing")?)?;
    if !(200..300).contains(&st) {
        return Err("HTTP status error");
    };
    let n =
        koma_source_sdk::decode_json_body_into(e, body_buf()).map_err(|_| "HTTP body decode")?;
    if n >= MAX {
        return Err("HTML body capacity");
    };
    core::str::from_utf8(&body_buf()[..n]).map_err(|_| "HTML UTF8")?;
    unsafe { BODY_LEN = n };
    Ok(Owned(
        host::html_parse(&body_buf()[..n]).map_err(|_| "HTML parse")?,
    ))
}
fn make<'a>(parts: &[&[u8]], b: &'a mut [u8]) -> R<&'a [u8]> {
    let mut n = 0;
    for x in parts {
        bytes(b, &mut n, x)?
    }
    Ok(&b[..n])
}
fn absolute<'a>(u: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    let mut n = 0;
    if u.starts_with(b"//") {
        bytes(b, &mut n, b"https:")?
    } else if u.starts_with(b"/") {
        bytes(b, &mut n, WEB)?
    } else if !u.starts_with(b"https://") {
        return Err("absolute URL");
    };
    bytes(b, &mut n, u)?;
    Ok(&b[..n])
}
fn image_url(u: &[u8]) -> R<()> {
    let t = u.strip_prefix(b"https://").ok_or("image scheme")?;
    let p = t.iter().position(|x| *x == b'/').ok_or("image path")?;
    let h = &t[..p];
    let sub = h.strip_suffix(b".hentaifox.com").ok_or("image host")?;
    if sub != b"i"
        && !(sub.len() > 1
            && matches!(sub[0], b'i' | b'm')
            && sub[1..].iter().all(|x| x.is_ascii_digit())
            && sub.len() <= 4)
    {
        return Err("image host");
    };
    let path = &t[p + 1..];
    if path.is_empty()
        || path.len() > 900
        || path
            .iter()
            .any(|c| !c.is_ascii_alphanumeric() && !matches!(c, b'/' | b'.' | b'_' | b'-'))
        || path.windows(2).any(|x| x == b"..")
    {
        return Err("image path");
    };
    Ok(())
}
fn ident(v: &[u8]) -> R<&[u8]> {
    if v.is_empty() || v.len() > 15 || v[0] == b'0' || number(v).is_none() {
        Err("work identity")
    } else {
        Ok(v)
    }
}
fn work_href(v: &[u8]) -> R<&[u8]> {
    ident(
        v.strip_prefix(WEB)
            .unwrap_or(v)
            .strip_prefix(b"/gallery/")
            .and_then(|x| x.strip_suffix(b"/"))
            .ok_or("work href")?,
    )
}
fn key<'a>(r: &'a [u8], field: &[u8], pre: &[u8]) -> R<&'a [u8]> {
    ident(
        text(args(r)?, field)?
            .strip_prefix(pre)
            .ok_or("namespace")?,
    )
}
fn child(d: HtmlDescriptor, s: &[u8]) -> R<Owned> {
    Ok(Owned(
        host::html_select(d, s).map_err(|_| "missing element")?,
    ))
}
// Match published Jsoup text(): collapse formatting whitespace, not title characters.
fn safe_text<'a>(d: HtmlDescriptor, s: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    let x = child(d, s)?;
    let len = host::html_text(x.0, b).map_err(|_| "text parse")?;
    if len >= b.len() {
        return Err("text capacity");
    };
    core::str::from_utf8(&b[..len]).map_err(|_| "text UTF8")?;
    let mut n = 0;
    let mut space = false;
    for i in 0..len {
        let c = b[i];
        if c.is_ascii_whitespace() {
            if n > 0 {
                space = true
            }
        } else {
            if space {
                b[n] = b' ';
                n += 1;
                space = false
            };
            b[n] = c;
            n += 1
        }
    }
    if n == 0 {
        return Err("empty text");
    };
    Ok(&b[..n])
}
fn imgattr<'a>(d: HtmlDescriptor, b: &'a mut [u8]) -> R<&'a [u8]> {
    for k in [
        b"data-cfsrc".as_slice(),
        b"data-src",
        b"data-lazy-src",
        b"srcset",
        b"src",
    ] {
        let n = host::html_attr(d, k, b).map_err(|_| "image attribute")?;
        if n >= b.len() {
            return Err("image capacity");
        };
        if n > 0 {
            core::str::from_utf8(&b[..n]).map_err(|_| "image UTF8")?;
            return Ok(if k == b"srcset" {
                b[..n].split(|x| *x == b' ').next().ok_or("srcset")?
            } else {
                &b[..n]
            });
        }
    }
    Err("missing image")
}
fn chinese(d: HtmlDescriptor) -> R<bool> {
    let mut b = [0; 256];
    let v = attr(d, b"data-languages", &mut b)?;
    if v.is_empty() {
        return Err("language declaration missing");
    };
    let mut found = false;
    for t in v
        .split(|x| x.is_ascii_whitespace())
        .filter(|x| !x.is_empty())
    {
        if number(t).is_none() {
            return Err("language code");
        };
        found |= t == b"6"
    }
    Ok(found)
}
fn emit_card(out: &mut [u8], n: &mut usize, d: HtmlDescriptor) -> R<usize> {
    let mut h = [0; 256];
    let id = work_href(sa(d, b".inner_thumb a", b"href", &mut h)?)?;
    let wid = number(id).ok_or("work number")?;
    let mut t = [0; 4096];
    let title = safe_text(d, b".caption a", &mut t)?;
    let im = child(d, b".inner_thumb img")?;
    let mut raw = [0; 1024];
    let mut b = [0; 1024];
    let u = absolute(imgattr(im.0, &mut raw)?, &mut b)?;
    image_url(u)?;
    bytes(out, n, b"{\"id\":\"hf:zh:")?;
    bytes(out, n, id)?;
    bytes(out, n, b"\",\"title\":")?;
    q(out, n, title)?;
    bytes(out, n, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(out, n, u)?;
    bytes(out, n, b"},\"contentRating\":\"unknown\"}")?;
    Ok(wid)
}
// Published query normalization: non-ASCII/punctuation runs preceding ASCII terms become '+'.
fn normalized<'a>(s: &[u8], out: &'a mut [u8]) -> R<&'a [u8]> {
    let s = trim_ascii(s);
    if s.is_empty() {
        return Err("empty query");
    };
    let mut i = 0;
    let mut n = 0;
    while i < s.len() {
        let c = s[i];
        if c.is_ascii_alphanumeric() || c == b'"' {
            bytes(out, &mut n, &[c])?;
            i += 1
        } else {
            let start = i;
            while i < s.len() && !s[i].is_ascii_alphanumeric() && s[i] != b'"' {
                i += 1
            }
            if i < s.len() {
                bytes(out, &mut n, b"+")?
            } else {
                bytes(out, &mut n, &s[start..i])?
            }
        }
    }
    bytes(out, &mut n, b"+chinese")?;
    Ok(&out[..n])
}
fn query_encoded(out: &mut [u8], n: &mut usize, s: &[u8]) -> R<()> {
    for segment in s.split(|x| *x == b'+').enumerate() {
        if segment.0 > 0 {
            bytes(out, n, b"+")?
        };
        if !write_url_encoded(out, n, segment.1) {
            return Err("query URL capacity");
        }
    }
    Ok(())
}
fn page(r: &[u8], query: &[u8]) -> R<usize> {
    let a = args(r)?;
    let p = member(a, b"page")?.unwrap_or(b"{}");
    if p.first() != Some(&b'{') {
        return Err("page envelope");
    };
    let v = member(p, b"cursor")?.unwrap_or(b"null");
    if v == b"null" {
        return Ok(1);
    };
    let mut b = [0; 8192];
    let s = uq(v, &mut b)?;
    let colon = s.iter().position(|x| *x == b':').ok_or("cursor")?;
    let p = number(&s[..colon]).ok_or("cursor page")?;
    let mut enc = [0; 8192];
    let mut n = 0;
    if !write_url_encoded(&mut enc, &mut n, query) {
        return Err("cursor capacity");
    };
    if p < 2 || p > 10000 || &s[colon + 1..] != &enc[..n] {
        return Err("cursor binding");
    };
    Ok(p)
}
fn list_url<'a>(query: &[u8], p: usize, search: bool, out: &'a mut [u8]) -> R<&'a [u8]> {
    let mut n = 0;
    if search {
        bytes(out, &mut n, b"https://hentaifox.com/search/?q=")?;
        query_encoded(out, &mut n, query)?;
        bytes(out, &mut n, b"&sort=popular&page=")?;
        wn(out, &mut n, p)?
    } else {
        bytes(
            out,
            &mut n,
            b"https://hentaifox.com/language/chinese/popular/pag/",
        )?;
        wn(out, &mut n, p)?;
        bytes(out, &mut n, b"/")?
    };
    Ok(&out[..n])
}
fn list(r: &[u8], search: bool) -> R<usize> {
    let mut qb = [0; 2048];
    let query = if search {
        uq(required(args(r)?, b"query")?, &mut qb)?
    } else {
        b""
    };
    if !search && text(args(r)?, b"listingId")? != b"listing:popular" {
        return Err("listing identity");
    };
    let p = page(r, query)?;
    let mut nb = [0; 4096];
    let norm = if search {
        normalized(query, &mut nb)?
    } else {
        b""
    };
    let mut ub = [0; 8192];
    let doc = fetch(list_url(norm, p, search, &mut ub)?)?;
    let cards = All::new(doc.0, b".lc_galleries .thumb")?;
    if cards.len == 0 {
        let mut h = [0; 4096];
        let heading = safe_text(doc.0, b"h1.tag_info", &mut h)?;
        if !search || !contains_bytes(heading, b"- 0 results") {
            return Err("empty or gated listing");
        }
    };
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"items\":[")?;
    let mut count = 0;
    let mut seen = [0usize; 1000];
    for i in 0..cards.len {
        let d = cards.at(i);
        let mut h = [0; 256];
        let id =
            number(work_href(sa(d, b".inner_thumb a", b"href", &mut h)?)?).ok_or("work number")?;
        if seen[..i].contains(&id) {
            return Err("same page duplicate");
        };
        seen[i] = id;
        if !chinese(d)? {
            continue;
        };
        if count > 0 {
            bytes(out, &mut n, b",")?
        };
        emit_card(out, &mut n, d)?;
        count += 1
    }
    let next = All::new(
        doc.0,
        b".pagination li.active + li:not(.disabled) a.page-link",
    )?;
    if next.len > 1 {
        return Err("ambiguous next");
    };
    bytes(out, &mut n, b"],\"page\":{")?;
    if next.len == 1 {
        let mut h = [0; 8192];
        let mut a = [0; 8192];
        let actual = absolute(attr(next.at(0), b"href", &mut h)?, &mut a)?;
        let np = p.checked_add(1).ok_or("page overflow")?;
        if np > 10000 {
            return Err("page capacity");
        };
        let mut expected = [0; 8192];
        if actual != list_url(norm, np, search, &mut expected)? {
            return Err("next ownership");
        };
        let mut c = [0; 8192];
        let mut k = 0;
        wn(&mut c, &mut k, np)?;
        bytes(&mut c, &mut k, b":")?;
        if !write_url_encoded(&mut c, &mut k, query) {
            return Err("cursor capacity");
        };
        bytes(out, &mut n, b"\"nextCursor\":")?;
        q(out, &mut n, &c[..k])?;
        bytes(out, &mut n, b",\"hasMore\":true}}")?
    } else {
        bytes(out, &mut n, b"\"nextCursor\":null,\"hasMore\":false}}")?
    };
    Ok(n)
}
// Repeated hidden IDs are legitimate in retained HTML only when all declarations agree.
fn input<'a>(d: HtmlDescriptor, k: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    let mut sel = [0; 128];
    let a = All::new(d, make(&[b"input[id=", k, b"]"], &mut sel)?)?;
    if a.len == 0 {
        return Err("missing input");
    };
    let n = attr(a.at(0), b"value", b)?.len();
    for i in 1..a.len {
        let mut t = [0; 256];
        if attr(a.at(i), b"value", &mut t)? != &b[..n] {
            return Err("contradictory input");
        }
    }
    Ok(&b[..n])
}
fn detail_doc(id: &[u8]) -> R<Owned> {
    let mut u = [0; 128];
    let d = fetch(make(&[WEB, b"/gallery/", id, b"/"], &mut u)?)?;
    let mut i = [0; 64];
    if input(d.0, b"gallery_id", &mut i)? != id {
        return Err("detail ownership");
    };
    let langs = All::new(d.0, b"ul.languages a")?;
    let mut cn = false;
    for j in 0..langs.len {
        let mut b = [0; 256];
        cn |= attr(langs.at(j), b"href", &mut b)? == b"/language/chinese/"
    }
    if !cn {
        return Err("detail not Chinese");
    };
    Ok(d)
}
fn cover<'a>(d: HtmlDescriptor, b: &'a mut [u8]) -> R<&'a [u8]> {
    let im = child(d, b".gallery_top .cover img")?;
    let mut raw = [0; 1024];
    let u = absolute(imgattr(im.0, &mut raw)?, b)?;
    image_url(u)?;
    Ok(u)
}
fn anchor_name<'a>(d: HtmlDescriptor, b: &'a mut [u8]) -> R<&'a [u8]> {
    // Name is encoded in the provider's exact tag href, excluding count badges.
    let h = attr(d, b"href", b)?;
    let h = h.strip_suffix(b"/").ok_or("tag href")?;
    let name = h.rsplit(|x| *x == b'/').next().ok_or("tag name")?;
    if name.is_empty() {
        return Err("tag name");
    };
    Ok(name)
}
fn detail(r: &[u8], chapters: bool) -> R<usize> {
    let id = key(r, b"mangaId", b"hf:zh:")?;
    let d = detail_doc(id)?;
    let mut tb = [0; 4096];
    let title = safe_text(d.0, b".gallery_top h1", &mut tb)?;
    let out = payload_buf();
    let mut n = 0;
    if chapters {
        bytes(out, &mut n, b"{\"chapters\":[{\"id\":\"hfch:zh:")?;
        bytes(out, &mut n, id)?;
        bytes(out, &mut n, b"\",\"mangaId\":\"hf:zh:")?;
        bytes(out, &mut n, id)?;
        bytes(out, &mut n, b"\",\"title\":")?;
        q(out, &mut n, title)?;
        bytes(
            out,
            &mut n,
            b",\"chapterNumber\":null,\"volumeNumber\":null,\"language\":\"zh\"}]}",
        )?;
        return Ok(n);
    };
    let mut cb = [0; 1024];
    let c = cover(d.0, &mut cb)?;
    bytes(out, &mut n, b"{\"manga\":{\"id\":\"hf:zh:")?;
    bytes(out, &mut n, id)?;
    bytes(out, &mut n, b"\",\"title\":")?;
    q(out, &mut n, title)?;
    bytes(out, &mut n, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(out, &mut n, c)?;
    bytes(
        out,
        &mut n,
        b"},\"status\":\"completed\",\"contentRating\":\"unknown\",\"tags\":[",
    )?;
    for (idx, sel) in [b"ul.tags a".as_slice(), b"ul.artists a"]
        .iter()
        .enumerate()
    {
        if idx == 1 {
            bytes(out, &mut n, b"],\"authors\":[")?
        };
        let a = All::new(d.0, sel)?;
        for i in 0..a.len {
            if i > 0 {
                bytes(out, &mut n, b",")?
            };
            let mut b = [0; 1024];
            q(out, &mut n, anchor_name(a.at(i), &mut b)?)?
        }
    }
    bytes(
        out,
        &mut n,
        b"],\"links\":[{\"kind\":\"source\",\"url\":\"https://hentaifox.com/gallery/",
    )?;
    bytes(out, &mut n, id)?;
    bytes(out, &mut n, b"/\"}]}}")?;
    Ok(n)
}
fn page_base<'a>(d: HtmlDescriptor, id: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    let mut dir = [0; 64];
    let dir = input(d, b"load_dir", &mut dir)?;
    if dir.is_empty() || !dir.iter().all(|c| c.is_ascii_digit()) {
        return Err("load directory");
    };
    let mut load = [0; 64];
    let load = ident(input(d, b"load_id", &mut load)?)?;
    let mut g = [0; 64];
    if input(d, b"gallery_id", &mut g)? != id {
        return Err("gallery owner");
    };
    let mut cv = [0; 1024];
    let c = cover(d, &mut cv)?;
    let after = c.strip_prefix(b"https://").ok_or("cover scheme")?;
    let pos = after.iter().position(|c| *c == b'/').ok_or("cover host")?;
    let host = &after[..pos];
    let mut declared = [0; 64];
    let mut sel = [0; 128];
    let server = All::new(d, make(&[b"input[id=load_server]"], &mut sel)?)?;
    let mut n = 0;
    bytes(b, &mut n, b"https://")?;
    if server.len > 0 {
        let s = input(d, b"load_server", &mut declared)?;
        if s.is_empty() {
            bytes(b, &mut n, host)?
        } else {
            if s.len() > 3 || !s.iter().all(|c| c.is_ascii_digit()) {
                return Err("server number");
            };
            bytes(b, &mut n, b"m")?;
            bytes(b, &mut n, s)?;
            bytes(b, &mut n, b".hentaifox.com")?
        }
    } else {
        bytes(b, &mut n, host)?
    };
    bytes(b, &mut n, b"/")?;
    bytes(b, &mut n, dir)?;
    bytes(b, &mut n, b"/")?;
    bytes(b, &mut n, load)?;
    bytes(b, &mut n, b"/")?;
    let mut expected = [0; 1024];
    let expected = make(
        &[b"https://", host, b"/", dir, b"/", load, b"/cover.jpg"],
        &mut expected,
    )?;
    if expected != c {
        return Err("cover load ownership");
    };
    image_url(&b[..n])?;
    Ok(&b[..n])
}
fn collect(d: HtmlDescriptor, id: &[u8]) -> R<usize> {
    let mut tb = [0; 64];
    let total = number(input(d, b"load_pages", &mut tb)?).ok_or("page total")?;
    if total == 0 || total > 1024 {
        return Err("page capacity");
    };
    let mut pb = [0; 1024];
    let base = page_base(d, id, &mut pb)?;
    let raw = unsafe { &body_buf()[..BODY_LEN] };
    let marker = b"$.parseJSON('";
    let start = find_subslice(raw, marker).ok_or("static page JSON missing")? + marker.len();
    if find_subslice(&raw[start..], marker).is_some() {
        return Err("ambiguous page JSON");
    };
    let end = find_subslice(&raw[start..], b"');").ok_or("static JSON terminator")? + start;
    let map = trim_ascii(&raw[start..end]);
    valid_json(map)?;
    if map.first() != Some(&b'{') {
        return Err("page JSON object");
    };
    let mut at = ws(map, 1);
    let mut count = 0;
    while map.get(at) != Some(&b'}') {
        let keyend = json_string_end(map, at).ok_or("page key")?;
        let seq = number(&map[at + 1..keyend - 1]).ok_or("page key integer")?;
        if seq != count + 1 || count >= total {
            return Err("page key order or count");
        };
        at = ws(map, keyend);
        if map.get(at) != Some(&b':') {
            return Err("page separator");
        };
        at = ws(map, at + 1);
        let vend = json_value_end(map, at, 0).ok_or("page value")?;
        let mut v = [0; 128];
        let value = uq(&map[at..vend], &mut v)?;
        let mut parts = value.split(|x| *x == b',');
        let ext = match parts.next().ok_or("extension")? {
            b"j" => b"jpg".as_slice(),
            b"p" => b"png",
            b"b" => b"bmp",
            b"g" => b"gif",
            b"w" => b"webp",
            _ => return Err("extension code"),
        };
        let width = number(parts.next().ok_or("width")?).ok_or("width integer")?;
        let height = number(parts.next().ok_or("height")?).ok_or("height integer")?;
        if width == 0 || height == 0 || width > 100000 || height > 100000 || parts.next().is_some()
        {
            return Err("page dimensions");
        };
        let mut url = [0; 1024];
        let mut n = 0;
        bytes(&mut url, &mut n, base)?;
        wn(&mut url, &mut n, seq)?;
        bytes(&mut url, &mut n, b".")?;
        bytes(&mut url, &mut n, ext)?;
        image_url(&url[..n])?;
        unsafe {
            PAGES[count][..n].copy_from_slice(&url[..n]);
            PAGE_LEN[count] = n
        };
        count += 1;
        at = ws(map, vend);
        if map.get(at) == Some(&b',') {
            at = ws(map, at + 1)
        } else if map.get(at) != Some(&b'}') {
            return Err("page separator");
        }
    }
    if count != total {
        return Err("incomplete page descriptors");
    };
    Ok(count)
}
fn pages(r: &[u8]) -> R<usize> {
    let id = key(r, b"chapterId", b"hfch:zh:")?;
    let d = detail_doc(id)?;
    let count = collect(d.0, id)?;
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"pages\":[")?;
    for i in 0..count {
        if i > 0 {
            bytes(out, &mut n, b",")?
        };
        bytes(out, &mut n, b"{\"id\":\"hfpage:zh:")?;
        bytes(out, &mut n, id)?;
        bytes(out, &mut n, b":")?;
        wn(out, &mut n, i)?;
        bytes(out, &mut n, b"\",\"index\":")?;
        wn(out, &mut n, i)?;
        bytes(out, &mut n, b",\"image\":{\"kind\":\"url\",\"url\":")?;
        unsafe { q(out, &mut n, &PAGES[i][..PAGE_LEN[i]])? };
        bytes(out, &mut n, b"}}")?
    }
    bytes(out, &mut n, b"]}")?;
    Ok(n)
}
fn image_request(r: &[u8]) -> R<usize> {
    let a = args(r)?;
    let mut ub = [0; 2048];
    let url = uq(required(a, b"url")?, &mut ub)?;
    image_url(url)?;
    let pid = text(a, b"pageId")?;
    if let Some(id) = pid.strip_prefix(b"hf:zh:") {
        let id = ident(id)?;
        let d = detail_doc(id)?;
        let mut b = [0; 1024];
        let base = page_base(d.0, id, &mut b)?;
        let mut cb = [0; 1024];
        let actual = cover(d.0, &mut cb)?;
        let mut th = [0; 1024];
        let thumb = make(&[base, b"thumb.jpg"], &mut th)?;
        if url != actual && url != thumb {
            return Err("cover ownership");
        }
    } else {
        let v = pid.strip_prefix(b"hfpage:zh:").ok_or("image namespace")?;
        let p = v.iter().position(|c| *c == b':').ok_or("page identity")?;
        let id = ident(&v[..p])?;
        let idx = number(&v[p + 1..]).ok_or("page index")?;
        let d = detail_doc(id)?;
        let count = collect(d.0, id)?;
        if idx >= count || unsafe { url != &PAGES[idx][..PAGE_LEN[idx]] } {
            return Err("page image ownership");
        }
    };
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"imageRequest\":{\"url\":")?;
    q(out, &mut n, url)?;
    bytes(out,&mut n,b",\"headers\":{\"Referer\":\"https://hentaifox.com/\",\"Origin\":\"https://hentaifox.com\",\"User-Agent\":")?;
    q(out, &mut n, UA)?;
    bytes(out, &mut n, b"}}}")?;
    Ok(n)
}
fn execute(op: &str, r: R<usize>) -> u32 {
    match r {
        Ok(n) => write_success_payload(op, n),
        Err(e) => write_error(op, "source_contract_error", e),
    }
}
fn run_search(r: &[u8]) -> u32 {
    execute("search", list(r, true))
}
fn run_get_manga_list(r: &[u8]) -> u32 {
    execute("get_manga_list", list(r, false))
}
fn run_get_manga(r: &[u8]) -> u32 {
    execute("get_manga", detail(r, false))
}
fn run_get_chapters(r: &[u8]) -> u32 {
    execute("get_chapters", detail(r, true))
}
fn run_get_pages(r: &[u8]) -> u32 {
    execute("get_pages", pages(r))
}
fn run_get_image_request(r: &[u8]) -> u32 {
    execute("get_image_request", image_request(r))
}
fn run_get_listings(_: &[u8]) -> u32 {
    let s="{\"listings\":[{\"id\":\"listing:popular\",\"name\":\"中文人气\",\"kind\":\"popular\",\"supportsPaging\":true}]}".as_bytes();
    payload_buf()[..s.len()].copy_from_slice(s);
    write_success_payload("get_listings", s.len())
}
fn run_get_home(_: &[u8]) -> u32 {
    write_error("get_home", "not_supported", "not supported")
}
fn run_get_settings(_: &[u8]) -> u32 {
    write_error("get_settings", "not_supported", "not supported")
}
fn run_get_filters(_: &[u8]) -> u32 {
    write_error("get_filters", "not_supported", "not supported")
}
// Source-local ABI gate; SDK remains byte-identical. Check before helper forms a slice.
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
    response_buffer().write_success_parts("source_info", &[
 b"{\"sourceInfo\":{\"id\":\"", SOURCE_INFO.id.as_bytes(), b"\",\"name\":\"", SOURCE_INFO.name.as_bytes(), b"\",\"version\":\"", SOURCE_INFO.version.as_bytes(), b"\",\"apiVersion\":\"0.2\",\"language\":\"zh\",\"author\":\"Koma\",\"description\":\"",SOURCE_INFO.description.as_bytes(),
 "\",\"contentRating\":\"nsfw\",\"localizedNames\":{\"zh-Hans\":\"HentaiFox（中文）\",\"zh-Hant\":\"HentaiFox（中文）\",\"en\":\"HentaiFox (Chinese)\"}},\"capabilities\":{\"search\":true,\"mangaDetail\":true,\"chapters\":true,\"pages\":true,\"listings\":true,\"mangaList\":true,\"imageRequest\":true,\"home\":false,\"filters\":false,\"settings\":false,\"future\":{\"login\":false,\"auth\":false,\"process_page_image\":false,\"page_description\":false,\"base_url\":false,\"deeplink\":false,\"migration\":false}}}".as_bytes()
 ])
}
macro_rules! export_op {
    ($f:ident,$op:literal,$run:ident) => {
        #[no_mangle]
        pub extern "C" fn $f(ptr: u32, len: u32) -> u32 {
            match checked_request(ptr, len) {
                Some(r) => $run(r),
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
