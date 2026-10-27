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
const WEB: &[u8] = b"https://asmhentai.com";
const SOURCE_INFO:SourceInfo=SourceInfo{id:"com.asmhentai.zh.koma",name:"AsmHentai",version:"0.1.2",api_version:"0.2",language:"zh",author:"Koma",description:"AsmHentai provider Chinese catalogue; per-work rating and usage unknown. Anonymous published metadata only.",content_rating:"nsfw"};
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
    settings: true,
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
    b"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0";
fn fetch(url: &[u8], form: Option<&[u8]>) -> R<Owned> {
    if host::check_cancel() {
        return Err("cancelled");
    };
    if !url.starts_with(b"https://asmhentai.com/") {
        return Err("metadata host");
    };
    let mut n = 0;
    let dst = http_req_buf();
    bytes(dst, &mut n, b"{\"version\":1,\"url\":")?;
    q(dst, &mut n, url)?;
    bytes(
        dst,
        &mut n,
        if form.is_some() {
            b",\"method\":\"POST\""
        } else {
            b",\"method\":\"GET\""
        },
    )?;
    bytes(dst,&mut n,b",\"headers\":{\"Referer\":\"https://asmhentai.com/\",\"Origin\":\"https://asmhentai.com\",\"User-Agent\":")?;
    q(dst, &mut n, UA)?;
    if form.is_some() {
        bytes(dst,&mut n,b",\"X-Requested-With\":\"XMLHttpRequest\",\"Content-Type\":\"application/x-www-form-urlencoded\"")?
    };
    bytes(dst, &mut n, b"}")?;
    if let Some(f) = form {
        bytes(dst, &mut n, b",\"bodyBase64\":")?;
        q(dst, &mut n, f)?
    };
    bytes(
        dst,
        &mut n,
        b",\"timeoutMs\":15000,\"responseKind\":\"bodyText\"}",
    )?;
    let z = host::http_request(&dst[..n], http_out()).map_err(|_| "transport error")?;
    if z >= MAX {
        return Err("HTTP envelope capacity");
    };
    let env = &http_out()[..z];
    valid_json(env)?;
    let status = num(member(env, b"status")?
        .or(member(env, b"statusCode")?)
        .ok_or("HTTP status missing")?)?;
    if !(200..300).contains(&status) {
        return Err("HTTP status error");
    };
    let n =
        koma_source_sdk::decode_json_body_into(env, body_buf()).map_err(|_| "HTTP body decode")?;
    if n >= MAX {
        return Err("HTML body capacity");
    };
    core::str::from_utf8(&body_buf()[..n]).map_err(|_| "HTML UTF8")?;
    Ok(Owned(
        host::html_parse(&body_buf()[..n]).map_err(|_| "HTML parse")?,
    ))
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
    let p = u
        .strip_prefix(b"https://images.asmhentai.com/")
        .ok_or("image host")?;
    if p.is_empty()
        || p.len() > 900
        || p.iter()
            .any(|c| !c.is_ascii_alphanumeric() && !matches!(c, b'/' | b'.' | b'_' | b'-'))
        || p.windows(2).any(|w| w == b"..")
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
    let v = v.strip_prefix(WEB).unwrap_or(v);
    ident(
        v.strip_prefix(b"/g/")
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
fn make<'a>(parts: &[&[u8]], b: &'a mut [u8]) -> R<&'a [u8]> {
    let mut n = 0;
    for x in parts {
        bytes(b, &mut n, x)?
    }
    Ok(&b[..n])
}
fn detail_doc(id: &[u8]) -> R<Owned> {
    let mut u = [0; 128];
    let expected = make(&[WEB, b"/g/", id, b"/"], &mut u)?;
    let d = fetch(expected, None)?;
    let mut b = [0; 256];
    if sa(d.0, b"link[rel=canonical]", b"href", &mut b)? != expected {
        return Err("detail canonical ownership");
    };
    Ok(d)
}
fn child(d: HtmlDescriptor, s: &[u8]) -> R<Owned> {
    Ok(Owned(
        host::html_select(d, s).map_err(|_| "missing element")?,
    ))
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
            return Err("image attribute capacity");
        };
        if n > 0 {
            core::str::from_utf8(&b[..n]).map_err(|_| "image UTF8")?;
            return Ok(if k == b"srcset" {
                &b[..n].split(|x| *x == b' ').next().ok_or("srcset")?
            } else {
                &b[..n]
            });
        }
    }
    Err("image descriptor missing")
}
fn chinese(d: HtmlDescriptor) -> R<bool> {
    let flags = All::new(d, b"a[href^=\"/language/\"]")?;
    if flags.len != 1 {
        return Err("ambiguous language flag");
    };
    let _flag = child(flags.at(0), b".flag")?;
    let mut b = [0; 128];
    Ok(attr(flags.at(0), b"href", &mut b)? == b"/language/chinese/")
}
fn safe_text<'a>(d: HtmlDescriptor, s: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    let t = st(d, s, b)?;
    core::str::from_utf8(t).map_err(|_| "text UTF8")?;
    if t.is_empty() {
        return Err("empty text");
    };
    Ok(t)
}
fn emit_card(out: &mut [u8], n: &mut usize, d: HtmlDescriptor, req: &[u8]) -> R<usize> {
    let mut h = [0; 256];
    let id = work_href(sa(d, b".image a", b"href", &mut h)?)?;
    let wid = number(id).ok_or("work number")?;
    let mut title = [0; 4096];
    let title = safe_text(d, b".caption", &mut title)?;
    let mut shortened = [0; 4096];
    let title = koma_source_sdk::gallery_titles::display(req, title, &mut shortened)?;
    let image = child(d, b".image img")?;
    let mut raw = [0; 1024];
    let mut cover = [0; 1024];
    let url = absolute(imgattr(image.0, &mut raw)?, &mut cover)?;
    image_url(url)?;
    bytes(out, n, b"{\"id\":\"asm:zh:")?;
    bytes(out, n, id)?;
    bytes(out, n, b"\",\"title\":")?;
    q(out, n, title)?;
    bytes(out, n, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(out, n, url)?;
    bytes(out, n, b"},\"contentRating\":\"unknown\"}")?;
    Ok(wid)
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
    let p = number(&s[..colon]).ok_or("cursor number")?;
    let mut enc = [0; 8192];
    let mut z = 0;
    if !write_url_encoded(&mut enc, &mut z, query) {
        return Err("query cursor capacity");
    };
    if &s[colon + 1..] != &enc[..z] || p < 2 || p > 10000 {
        return Err("cursor binding");
    };
    Ok(p)
}
fn list(r: &[u8], search: bool) -> R<usize> {
    let mut qb = [0; 2048];
    let query = if search {
        uq(required(args(r)?, b"query")?, &mut qb)?
    } else {
        b""
    };
    if search && trim_ascii(query).is_empty() {
        return Err("empty query");
    };
    if !search && text(args(r)?, b"listingId")? != b"listing:popular" {
        return Err("listing identity");
    };
    let p = page(r, query)?;
    let mut url = [0; 8192];
    let mut z = 0;
    if search {
        bytes(&mut url, &mut z, b"https://asmhentai.com/search/?q=")?;
        if !write_url_encoded(&mut url, &mut z, query) {
            return Err("query URL capacity");
        };
        bytes(&mut url, &mut z, b"&sort=popular&page=")?
    } else {
        bytes(
            &mut url,
            &mut z,
            b"https://asmhentai.com/language/chinese/popular/?page=",
        )?
    };
    wn(&mut url, &mut z, p)?;
    let doc = fetch(&url[..z], None)?;
    let cards = All::new(doc.0, b".preview_item")?;
    if cards.len == 0 {
        return Err("empty or gated listing");
    };
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"items\":[")?;
    let mut count = 0;
    let mut seen = [0usize; 1000];
    for i in 0..cards.len {
        let d = cards.at(i);
        let mut hb = [0; 256];
        let id = number(work_href(sa(d, b".image a", b"href", &mut hb)?)?).ok_or("work number")?;
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
        emit_card(out, &mut n, d, r)?;
        count += 1;
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
        let mut abs = [0; 8192];
        let actual = absolute(attr(next.at(0), b"href", &mut h)?, &mut abs)?;
        let np = p.checked_add(1).ok_or("page overflow")?;
        if np > 10000 {
            return Err("page capacity");
        };
        let mut expected = [0; 8192];
        let mut e = 0;
        bytes(&mut expected, &mut e, &url[..z])?;
        let pos = expected[..e]
            .iter()
            .rposition(|x| *x == b'=')
            .ok_or("page url")?;
        e = pos + 1;
        wn(&mut expected, &mut e, np)?;
        if actual != &expected[..e] {
            return Err("next URL mismatch");
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
fn detail_language(book: HtmlDescriptor) -> R<()> {
    let x = All::new(book, b".tag_list a[href^=\"/language/\"]")?;
    let mut cn = false;
    for i in 0..x.len {
        let mut b = [0; 256];
        if attr(x.at(i), b"href", &mut b)? == b"/language/chinese/" {
            cn = true
        }
    }
    if !cn {
        return Err("detail not Chinese");
    };
    Ok(())
}
fn detail(r: &[u8], chapters: bool) -> R<usize> {
    let id = key(r, b"mangaId", b"asm:zh:")?;
    let doc = detail_doc(id)?;
    let book = child(doc.0, b".book_page")?;
    detail_language(book.0)?;
    let mut t = [0; 4096];
    let title = safe_text(book.0, b"h1", &mut t)?;
    let out = payload_buf();
    let mut n = 0;
    if chapters {
        bytes(out, &mut n, b"{\"chapters\":[{\"id\":\"asmch:zh:")?;
        bytes(out, &mut n, id)?;
        bytes(out, &mut n, b"\",\"mangaId\":\"asm:zh:")?;
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
    let full_title = title;
    let mut shortened = [0; 4096];
    let title = koma_source_sdk::gallery_titles::display(r, title, &mut shortened)?;
    let im = child(book.0, b".cover img")?;
    let mut raw = [0; 1024];
    let mut ab = [0; 1024];
    let thumb = absolute(imgattr(im.0, &mut raw)?, &mut ab)?;
    image_url(thumb)?;
    bytes(out, &mut n, b"{\"manga\":{\"id\":\"asm:zh:")?;
    bytes(out, &mut n, id)?;
    bytes(out, &mut n, b"\",\"title\":")?;
    q(out, &mut n, title)?;
    bytes(out, &mut n, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(out, &mut n, thumb)?;
    bytes(
        out,
        &mut n,
        b"},\"status\":\"completed\",\"contentRating\":\"unknown\",\"tags\":[",
    )?;
    let tags = All::new(book.0, b".tag_list a[href^=\"/tag/\"] .tag")?;
    for i in 0..tags.len {
        if i > 0 {
            bytes(out, &mut n, b",")?
        };
        let mut b = [0; 1024];
        q(out, &mut n, tag_name(txt(tags.at(i), &mut b)?))?
    }
    bytes(out, &mut n, b"],\"authors\":[")?;
    let artists = All::new(book.0, b".tag_list a[href^=\"/artist/\"] .tag")?;
    for i in 0..artists.len {
        if i > 0 {
            bytes(out, &mut n, b",")?
        };
        let mut b = [0; 1024];
        q(out, &mut n, tag_name(txt(artists.at(i), &mut b)?))?
    }
    bytes(out, &mut n, b"]")?;
    if koma_source_sdk::gallery_titles::enabled(r) {
        bytes(out, &mut n, b",\"description\":\"**Full title**: ")?;
        if !append_json_escaped(out, &mut n, full_title) { return Err("payload overflow"); }
        bytes(out, &mut n, b"\"")?;
    }
    bytes(
        out,
        &mut n,
        b",\"links\":[{\"kind\":\"source\",\"url\":\"https://asmhentai.com/g/",
    )?;
    bytes(out, &mut n, id)?;
    bytes(out, &mut n, b"/\"}]}}")?;
    Ok(n)
}
fn tag_name(v: &[u8]) -> &[u8] {
    if let Some(p) = find_subslice(v, b" (") {
        trim_ascii(&v[..p])
    } else {
        v
    }
}
fn input<'a>(d: HtmlDescriptor, key: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    let mut s = [0; 128];
    let sel = make(&[b"input[id=", key, b"]"], &mut s)?;
    sa(d, sel, b"value", b)
}
fn collect_thumbs(
    d: HtmlDescriptor,
    selector: &[u8],
    id: &[u8],
    start: usize,
    total: usize,
) -> R<usize> {
    let thumbs = All::new(d, selector)?;
    let mut count = start;
    for i in 0..thumbs.len {
        if count >= total || count >= 1024 {
            return Err("page count overflow");
        };
        let anchor = thumbs.at(i);
        let mut href = [0; 256];
        let h = attr(anchor, b"href", &mut href)?;
        let mut expected = [0; 256];
        let mut z = 0;
        bytes(&mut expected, &mut z, b"/gallery/")?;
        bytes(&mut expected, &mut z, id)?;
        bytes(&mut expected, &mut z, b"/")?;
        wn(&mut expected, &mut z, count + 1)?;
        bytes(&mut expected, &mut z, b"/")?;
        if h != &expected[..z] {
            return Err("page ownership or order");
        };
        let im = child(anchor, b"img")?;
        let mut raw = [0; 1024];
        let mut ab = [0; 1024];
        let u = absolute(imgattr(im.0, &mut raw)?, &mut ab)?;
        image_url(u)?;
        let tail = u.rsplit(|x| *x == b'/').next().ok_or("image filename")?;
        let dot = tail
            .iter()
            .rposition(|x| *x == b'.')
            .ok_or("image extension")?;
        let seq = tail[..dot]
            .strip_suffix(b"t")
            .ok_or("actual thumbnail suffix")?;
        if number(seq) != Some(count + 1) {
            return Err("thumbnail sequence");
        };
        let ext = &tail[dot + 1..];
        if ![b"jpg".as_slice(), b"png", b"webp", b"gif", b"bmp"].contains(&ext) {
            return Err("image extension");
        };
        let base = &u[..u.len() - tail.len()];
        let parts: AllImagePath = AllImagePath::parse(base, id)?;
        let _ = parts;
        let mut original = [0; 1024];
        let v = make(&[base, seq, b".", ext], &mut original)?;
        unsafe {
            for j in 0..count {
                if &PAGES[j][..PAGE_LEN[j]] == v {
                    return Err("duplicate image URL");
                }
            }
            PAGES[count][..v.len()].copy_from_slice(v);
            PAGE_LEN[count] = v.len()
        };
        count += 1;
    }
    Ok(count)
}
struct AllImagePath;
impl AllImagePath {
    fn parse(p: &[u8], id: &[u8]) -> R<Self> {
        let path = p
            .strip_prefix(b"https://images.asmhentai.com/")
            .ok_or("image host")?;
        let mut it = path.split(|x| *x == b'/');
        let dir = it.next().ok_or("image directory")?;
        let work = it.next().ok_or("image owner")?;
        if dir.is_empty()
            || !dir.iter().all(|x| x.is_ascii_digit())
            || work != id
            || it.next() != Some(b"")
            || it.next().is_some()
        {
            return Err("image ownership");
        };
        Ok(Self)
    }
}
fn pages(r: &[u8]) -> R<usize> {
    let id = key(r, b"chapterId", b"asmch:zh:")?;
    let doc = detail_doc(id)?;
    let book = child(doc.0, b".book_page")?;
    detail_language(book.0)?;
    let mut ib = [0; 64];
    if input(doc.0, b"load_id", &mut ib)? != id {
        return Err("loader ownership");
    };
    let mut tb = [0; 32];
    let total = number(input(doc.0, b"t_pages", &mut tb)?).ok_or("page total")?;
    if total == 0 || total > 1000 {
        return Err("page capacity");
    };
    let first = collect_thumbs(doc.0, b".preview_thumb a", id, 0, total)?;
    if first == 0 {
        return Err("initial thumbnails missing");
    };
    let mut count = first;
    if count < total {
        let mut dir = [0; 32];
        let dir = input(doc.0, b"load_dir", &mut dir)?;
        if dir.is_empty() || !dir.iter().all(|c| c.is_ascii_digit()) {
            return Err("loader directory");
        };
        let mut token = [0; 256];
        let tok = sa(doc.0, b"meta[name=csrf-token]", b"content", &mut token)?;
        let mut form = [0; 2048];
        let mut n = 0;
        bytes(&mut form, &mut n, b"id=")?;
        bytes(&mut form, &mut n, id)?;
        bytes(&mut form, &mut n, b"&dir=")?;
        bytes(&mut form, &mut n, dir)?;
        bytes(&mut form, &mut n, b"&visible_pages=")?;
        wn(&mut form, &mut n, count)?;
        bytes(&mut form, &mut n, b"&t_pages=")?;
        wn(&mut form, &mut n, total)?;
        bytes(&mut form, &mut n, b"&type=2")?;
        if !tok.is_empty() {
            bytes(&mut form, &mut n, b"&_token=")?;
            if !write_url_encoded(&mut form, &mut n, tok) {
                return Err("token capacity");
            }
        };
        let more = fetch(
            b"https://asmhentai.com/inc/thumbs_loader.php",
            Some(&form[..n]),
        )?;
        count = collect_thumbs(more.0, b"a", id, count, total)?;
    };
    if count != total {
        return Err("incomplete page descriptors");
    };
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"pages\":[")?;
    for i in 0..count {
        if i > 0 {
            bytes(out, &mut n, b",")?
        };
        bytes(out, &mut n, b"{\"id\":\"asmpage:zh:")?;
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
    let id = if let Some(v) = pid.strip_prefix(b"asm:zh:") {
        ident(v)?
    } else {
        let v = pid.strip_prefix(b"asmpage:zh:").ok_or("image namespace")?;
        let colon = v.iter().position(|x| *x == b':').ok_or("page identity")?;
        let id = ident(&v[..colon])?;
        let index = number(&v[colon + 1..]).ok_or("page index")?;
        if index >= 1000 {
            return Err("page index capacity");
        };
        let filename = url.rsplit(|x| *x == b'/').next().ok_or("filename")?;
        let stem = filename.split(|x| *x == b'.').next().ok_or("filename")?;
        if number(stem) != Some(index + 1) {
            return Err("image index");
        };
        id
    };
    let path = url.strip_prefix(b"https://images.asmhentai.com/").unwrap();
    let mut seg = path.split(|x| *x == b'/');
    let dir = seg.next().ok_or("directory")?;
    if dir.is_empty()
        || !dir.iter().all(|x| x.is_ascii_digit())
        || seg.next() != Some(id)
        || seg.next().is_none()
        || seg.next().is_some()
    {
        return Err("image ownership");
    };
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"url\":")?;
    q(out, &mut n, url)?;
    bytes(out,&mut n,b",\"headers\":{\"Referer\":\"https://asmhentai.com/\",\"Origin\":\"https://asmhentai.com\",\"User-Agent\":")?;
    q(out, &mut n, UA)?;
    bytes(out, &mut n, b"}}")?;
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
fn run_get_settings(req: &[u8]) -> u32 {
    let prefix = br#"{"settings":[{"id":"pref_short_title","name":"Display Short Titles","kind":"toggle","default":"false","description":""#;
    let state = if koma_source_sdk::gallery_titles::enabled(req) { b"Showing short Titles".as_slice() } else { b"Showing Long Titles".as_slice() };
    let out = payload_buf();
    let mut n = 0;
    if !(write_bytes(out, &mut n, prefix) && write_bytes(out, &mut n, state) && write_bytes(out, &mut n, br#""}]}"#)) {
        return write_error("get_settings", "internal_error", "payload overflow");
    }
    write_success_payload("get_settings", n)
}
fn run_get_filters(_: &[u8]) -> u32 {
    write_error("get_filters", "not_supported", "not supported")
}
// Source-local ABI gate. Check before the SDK helper forms a slice.
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
 "\",\"contentRating\":\"nsfw\",\"localizedNames\":{\"zh-Hans\":\"AsmHentai（中文）\",\"zh-Hant\":\"AsmHentai（中文）\",\"en\":\"AsmHentai (Chinese)\"}},\"capabilities\":{\"search\":true,\"mangaDetail\":true,\"chapters\":true,\"pages\":true,\"listings\":true,\"mangaList\":true,\"imageRequest\":true,\"home\":false,\"filters\":false,\"settings\":true,\"future\":{\"login\":false,\"auth\":false,\"process_page_image\":false,\"page_description\":false,\"base_url\":false,\"deeplink\":false,\"migration\":false}}}".as_bytes()
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
