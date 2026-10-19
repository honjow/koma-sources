#![no_std]
use koma_source_sdk::host::HtmlDescriptor;
use koma_source_sdk::{host, json_utils::*, source::SourceInfo};
type E = &'static str;
type R<T> = Result<T, E>;
const MAX: usize = 1048576;
const WEB: &[u8] = b"https://3hentai.net";

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.hentai3.zh.koma",
    name: "3Hentai",
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
static mut PAGES: [[u8; 2048]; 1000] = [[0; 2048]; 1000];
static mut PAGE_LEN: [usize; 1000] = [0; 1000];
static mut JSON: [u8; MAX] = [0; MAX];
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
        let imageHostSelected = value.is_empty()
            && member(r, b"operation")?.and_then(|v| quoted(v).ok())
                == Some(b"get_image_request".as_slice());
        if value != SOURCE_INFO.id.as_bytes() && !imageHostSelected {
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
fn fetch(url: &[u8]) -> R<Owned> {
    if host::check_cancel() {
        return Err("cancelled");
    }
    if !url.starts_with(b"https://3hentai.net/") {
        return Err("metadata host");
    }
    let b = http_req_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"version\":1,\"method\":\"GET\",\"url\":")?;
    q(b, &mut n, url)?;
    bytes(b,&mut n,b",\"headers\":{\"Referer\":\"https://3hentai.net/\",\"Origin\":\"https://3hentai.net\",\"Cache-Control\":\"max-age=600\",\"User-Agent\":")?;
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
    };
    core::str::from_utf8(&body_buf()[..z]).map_err(|_| "HTML UTF8")?;
    Ok(Owned(
        host::html_parse(&body_buf()[..z]).map_err(|_| "HTML parse")?,
    ))
}
fn make<'a>(ps: &[&[u8]], b: &'a mut [u8]) -> R<&'a [u8]> {
    let mut n = 0;
    for p in ps {
        bytes(b, &mut n, p)?
    }
    Ok(&b[..n])
}
fn ident(s: &[u8]) -> R<&[u8]> {
    if s.is_empty()
        || s.len() > 20
        || number(s).filter(|n| *n > 0).is_none()
        || (s.len() > 1 && s[0] == b'0')
    {
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
    if s.starts_with(b"/") && !s.starts_with(b"//") {
        make(&[WEB, s], b)
    } else if s.starts_with(b"https://3hentai.net/") {
        make(&[s], b)
    } else {
        Err("route host")
    }
}

fn work_url<'a>(id: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    ident(id)?;
    make(&[WEB, b"/d/", id], b)
}
fn work_href(s: &[u8]) -> R<&[u8]> {
    let id = s
        .strip_prefix(b"https://3hentai.net/d/")
        .ok_or("work URL host")?;
    ident(id)
}
fn asset<'a>(s: &'a [u8]) -> R<(&'a [u8], &'a [u8])> {
    let rest = s.strip_prefix(b"https://s").ok_or("asset scheme or host")?;
    let z = rest.iter().position(|c| *c == b'.').ok_or("asset host")?;
    if !matches!(&rest[..z], b"2" | b"4" | b"9") {
        return Err("unobserved asset host");
    }
    let p = rest[z..]
        .strip_prefix(b".3hentai.net/d")
        .ok_or("asset domain")?;
    let k = p.iter().position(|c| *c == b'/').ok_or("asset path")?;
    let id = ident(&p[..k])?;
    let file = &p[k + 1..];
    if file.is_empty()
        || file.len() > 128
        || file.contains(&b'/')
        || !file
            .iter()
            .all(|c| c.is_ascii_alphanumeric() || matches!(*c, b'.' | b'_' | b'-'))
        || file == b"."
        || file == b".."
    {
        return Err("asset filename");
    }
    Ok((id, file))
}
fn own_asset(s: &[u8], id: &[u8]) -> R<()> {
    if asset(s)?.0 != id {
        return Err("asset work owner");
    }
    Ok(())
}
fn cn(d: HtmlDescriptor) -> R<()> {
    let langs = All::new(d, b"#main-info a[href*='/language/']")?;
    let mut found = false;
    for i in 0..langs.len {
        let mut b = [0u8; 1024];
        let s = attr(langs.at(i), b"href", &mut b)?;
        if s == b"https://3hentai.net/language/chinese" {
            found = true
        }
    }
    if !found {
        return Err("work Chinese language missing");
    }
    Ok(())
}
fn detail_doc(id: &[u8]) -> R<Owned> {
    let mut u = [0u8; 512];
    let doc = fetch(work_url(id, &mut u)?)?;
    let mut c = [0u8; 1024];
    let uri = sa(doc.0, b"link[rel=canonical]", b"href", &mut c)?;
    if uri != work_url(id, &mut u)? {
        return Err("detail work owner");
    }
    cn(doc.0)?;
    Ok(doc)
}
fn title<'a>(d: HtmlDescriptor, b: &'a mut [u8]) -> R<&'a [u8]> {
    let t = st(d, b"#main-info h1", b)?;
    if t.is_empty() {
        Err("empty title")
    } else {
        Ok(t)
    }
}
fn cover<'a>(d: HtmlDescriptor, b: &'a mut [u8]) -> R<&'a [u8]> {
    let imgs = All::new(d, b"#main-info img")?;
    if imgs.len == 0 {
        return Err("detail cover missing");
    };
    let mut n = host::html_attr(imgs.at(0), b"data-src", b).map_err(|_| "cover attribute")?;
    if n == 0 {
        n = host::html_attr(imgs.at(0), b"src", b).map_err(|_| "cover attribute")?
    };
    if n >= b.len() {
        return Err("cover capacity");
    };
    core::str::from_utf8(&b[..n]).map_err(|_| "cover UTF8")?;
    asset(&b[..n])?;
    Ok(&b[..n])
}
fn count(d: HtmlDescriptor) -> R<usize> {
    let fields = All::new(d, b"#main-info div.tag-container")?;
    let mut result = None;
    for i in 0..fields.len {
        let mut b = [0u8; 8192];
        let t = txt(fields.at(i), &mut b)?;
        if let Some(v) = t.strip_prefix(b"Pages: ") {
            let n = number(v).ok_or("page count number")?;
            if n == 0 || n > 1000 || result.is_some() {
                return Err("page count capacity or ambiguity");
            }
            result = Some(n)
        }
    }
    result.ok_or("page count missing")
}
fn emit_work(out: &mut [u8], n: &mut usize, id: &[u8], t: &[u8], c: &[u8]) -> R<()> {
    let mut w = [0u8; 512];
    bytes(out, n, b"{\"id\":\"h3:zh:")?;
    bytes(out, n, id)?;
    bytes(out, n, b"\",\"title\":")?;
    q(out, n, t)?;
    bytes(out, n, b",\"url\":")?;
    q(out, n, work_url(id, &mut w)?)?;
    bytes(out, n, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(out, n, c)?;
    bytes(out, n, b"},\"contentRating\":\"unknown\"}")
}
fn cursor_page(s: &[u8]) -> R<usize> {
    let v = s
        .strip_prefix(b"https://3hentai.net/language/chinese/")
        .ok_or("listing cursor owner")?;
    let v = v
        .strip_suffix(b"?sort=popular")
        .ok_or("listing cursor sort")?;
    let n = number(v).ok_or("listing cursor page")?;
    if n < 2 || n > 10000 || v[0] == b'0' {
        return Err("listing cursor range");
    }
    Ok(n)
}
fn listing(r: &[u8]) -> R<usize> {
    let a = args(r)?;
    if text(a, b"listingId")? != b"listing:popular" {
        return Err("unsupported listing");
    };
    let mut u = [0u8; 2048];
    let mut current = 1;
    let uri = if let Some(p) = member(a, b"page")? {
        match member(p, b"cursor")? {
            Some(v) if v != b"null" => {
                let s = uq(v, &mut u)?;
                current = cursor_page(s)?;
                s
            }
            _ => b"https://3hentai.net/language/chinese/?sort=popular",
        }
    } else {
        b"https://3hentai.net/language/chinese/?sort=popular"
    };
    let d = fetch(uri)?;
    let cards = All::new(d.0, b"a[href*='/d/']")?;
    let next = All::new(d.0, b"a[rel=next]")?;
    if next.len > 1 {
        return Err("listing next ambiguity");
    };
    let mut nb = [0u8; 2048];
    let nxt = if next.len == 1 {
        let s = attr(next.at(0), b"href", &mut nb)?;
        if cursor_page(s)? != current.checked_add(1).ok_or("page overflow")? {
            return Err("listing next discontinuity");
        }
        if cards.len == 0 {
            return Err("listing no progress");
        }
        Some(s)
    } else {
        None
    };
    let mut seen = [[0u8; 20]; 1000];
    let mut lens = [0usize; 1000];
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"items\":[")?;
    for i in 0..cards.len {
        let d = cards.at(i);
        let mut h = [0u8; 1024];
        let id = work_href(attr(d, b"href", &mut h)?)?;
        if (0..i).any(|j| &seen[j][..lens[j]] == id) {
            return Err("duplicate work");
        };
        seen[i][..id.len()].copy_from_slice(id);
        lens[i] = id.len();
        let mut tb = [0u8; 4096];
        let t = st(d, b"div.title", &mut tb)?;
        if t.is_empty() {
            return Err("card title");
        };
        let mut cb = [0u8; 2048];
        let c = sa(d, b"img[data-src]", b"data-src", &mut cb)?;
        let (_, file) = asset(c)?;
        if file != b"thumb.jpg" {
            return Err("card cover template");
        };
        if i > 0 {
            bytes(out, &mut n, b",")?
        };
        emit_work(out, &mut n, id, t, c)?;
    }
    bytes(out, &mut n, b"],\"page\":{\"hasMore\":")?;
    bytes(out, &mut n, if nxt.is_some() { b"true" } else { b"false" })?;
    bytes(out, &mut n, b",\"nextCursor\":")?;
    if let Some(v) = nxt {
        q(out, &mut n, v)?
    } else {
        bytes(out, &mut n, b"null")?
    };
    bytes(out, &mut n, b"}}")?;
    Ok(n)
}
fn detail(r: &[u8]) -> R<usize> {
    let id = key(r, b"mangaId", b"h3:zh:")?;
    let d = detail_doc(id)?;
    let mut tb = [0u8; 4096];
    let t = title(d.0, &mut tb)?;
    let mut cb = [0u8; 2048];
    let c = cover(d.0, &mut cb)?;
    let total = count(d.0)?;
    let mut author = [0u8; 8192];
    let mut an = 0;
    let groups = All::new(d.0, b"#main-info a[href*='/groups/']")?;
    let artists = All::new(d.0, b"#main-info a[href*='/artists/']")?;
    let auth = if groups.len > 0 { &groups } else { &artists };
    for i in 0..auth.len {
        let mut b = [0u8; 1024];
        let v = txt(auth.at(i), &mut b)?;
        if v.is_empty() {
            return Err("author empty");
        };
        if i > 0 {
            bytes(&mut author, &mut an, b", ")?
        }
        bytes(&mut author, &mut an, v)?;
    }
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"manga\":")?;
    emit_work(out, &mut n, id, t, c)?;
    n -= 1;
    bytes(out, &mut n, b",\"status\":\"completed\",\"authors\":[")?;
    if an > 0 {
        q(out, &mut n, &author[..an])?
    };
    bytes(
        out,
        &mut n,
        b"],\"description\":\"Language: Chinese (provider declaration); Pages: ",
    )?;
    wn(out, &mut n, total)?;
    bytes(out, &mut n, b"\",\"tags\":[")?;
    let tags = All::new(d.0, b"#main-info a[href*='/tags/']")?;
    for i in 0..tags.len {
        let mut b = [0u8; 1024];
        let v = txt(tags.at(i), &mut b)?;
        if v.is_empty() {
            return Err("tag empty");
        };
        if i > 0 {
            bytes(out, &mut n, b",")?
        }
        q(out, &mut n, v)?
    }
    bytes(out, &mut n, b"]}}")?;
    Ok(n)
}
fn chapters(r: &[u8]) -> R<usize> {
    let id = key(r, b"mangaId", b"h3:zh:")?;
    let d = detail_doc(id)?;
    count(d.0)?;
    let o = payload_buf();
    let mut n = 0;
    bytes(o, &mut n, b"{\"chapters\":[{\"id\":\"h3ch:zh:")?;
    bytes(o, &mut n, id)?;
    bytes(o, &mut n, b"\",\"mangaId\":\"h3:zh:")?;
    bytes(o, &mut n, id)?;
    bytes(o,&mut n,b"\",\"title\":\"Chapter\",\"chapterNumber\":null,\"volumeNumber\":null,\"language\":\"zh\"}]}")?;
    Ok(n)
}
fn pages(r: &[u8]) -> R<usize> {
    let id = key(r, b"chapterId", b"h3ch:zh:")?;
    let d = detail_doc(id)?;
    let total = count(d.0)?;
    let imgs = All::new(d.0, b".single-thumb")?;
    if imgs.len != total {
        return Err("page count mismatch");
    };
    let urls = unsafe { &mut *core::ptr::addr_of_mut!(PAGES) };
    let lens = unsafe { &mut *core::ptr::addr_of_mut!(PAGE_LEN) };
    for i in 0..total {
        let mut b = [0u8; 2048];
        let u = sa(imgs.at(i), b"img[data-src]", b"data-src", &mut b)?;
        let mut href = [0u8; 1024];
        let h = sa(imgs.at(i), b"a", b"href", &mut href)?;
        let mut exp = [0u8; 1024];
        let mut cur = 0;
        bytes(&mut exp, &mut cur, WEB)?;
        bytes(&mut exp, &mut cur, b"/d/")?;
        bytes(&mut exp, &mut cur, id)?;
        bytes(&mut exp, &mut cur, b"/")?;
        wn(&mut exp, &mut cur, i + 1)?;
        if h != &exp[..cur] {
            return Err("page reader href owner/sequence");
        };
        let (owner, file) = asset(u)?;
        if owner != id {
            return Err("page work owner");
        };
        let v = file
            .strip_suffix(b"t.jpg")
            .ok_or("unobserved page format")?;
        let seq = number(v).ok_or("page sequence")?;
        if seq != i + 1 || v[0] == b'0' {
            return Err("page sequence mismatch");
        };
        if (0..i).any(|j| &urls[j][..lens[j]] == u) {
            return Err("duplicate page");
        };
        let mut n = 0;
        bytes(&mut urls[i], &mut n, &u[..u.len() - 5])?;
        bytes(&mut urls[i], &mut n, b".jpg")?;
        lens[i] = n;
    }
    let o = payload_buf();
    let mut n = 0;
    bytes(o, &mut n, b"{\"pages\":[")?;
    for i in 0..total {
        if i > 0 {
            bytes(o, &mut n, b",")?
        }
        bytes(o, &mut n, b"{\"id\":\"h3page:zh:")?;
        bytes(o, &mut n, id)?;
        bytes(o, &mut n, b":")?;
        wn(o, &mut n, i)?;
        bytes(o, &mut n, b"\",\"index\":")?;
        wn(o, &mut n, i)?;
        bytes(o, &mut n, b",\"image\":{\"kind\":\"url\",\"url\":")?;
        q(o, &mut n, &urls[i][..lens[i]])?;
        bytes(o, &mut n, b"}}")?;
    }
    bytes(o, &mut n, b"]}")?;
    Ok(n)
}
fn image_request(r: &[u8]) -> R<usize> {
    let a = args(r)?;
    let mut ub = [0u8; 2048];
    let uri = uq(required(a, b"url")?, &mut ub)?;
    let mut pb = [0u8; 256];
    let pid = uq(required(a, b"pageId")?, &mut pb)?;
    let mut rb = [0u8; 512];
    let route = uq(required(a, b"mangaUrl")?, &mut rb)?;
    let id;
    if let Some(p) = pid.strip_prefix(b"h3page:zh:") {
        let k = p.iter().position(|c| *c == b':').ok_or("page identity")?;
        id = ident(&p[..k])?;
        let index = number(&p[k + 1..]).ok_or("page index")?;
        if index >= 1000 {
            return Err("page index capacity");
        };
        let (owner, file) = asset(uri)?;
        if owner != id {
            return Err("image work owner");
        };
        let num = file.strip_suffix(b".jpg").ok_or("image page format")?;
        if number(num) != index.checked_add(1) || num.first() == Some(&b'0') {
            return Err("image page sequence");
        }
    } else {
        id = ident(pid.strip_prefix(b"h3:zh:").ok_or("cover identity")?)?;
        let (_, file) = asset(uri)?;
        if !matches!(file, b"cover.jpg" | b"thumb.jpg") {
            return Err("cover format");
        }
    }
    let mut w = [0u8; 512];
    let referrer = work_url(id, &mut w)?;
    if route != referrer {
        return Err("image independent work route");
    };
    let o = payload_buf();
    let mut n = 0;
    bytes(o, &mut n, b"{\"imageRequest\":{\"method\":\"GET\",\"url\":")?;
    q(o, &mut n, uri)?;
    bytes(o,&mut n,b",\"headers\":{\"Referer\":\"https://3hentai.net/\",\"Origin\":\"https://3hentai.net\",\"Cache-Control\":\"max-age=600\",\"User-Agent\":")?;
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
    response_buffer().write_success("source_info","{\"sourceInfo\":{\"id\":\"com.hentai3.zh.koma\",\"name\":\"3Hentai\",\"version\":\"0.1.0\",\"apiVersion\":\"0.2\",\"language\":\"zh\",\"author\":\"Koma\",\"description\":\"Chinese-language provider catalogue. Per-work rating and source usage unknown.\",\"contentRating\":\"adult\",\"localizedNames\":{\"zh-Hans\":\"3Hentai（中文）\",\"zh-Hant\":\"3Hentai（中文）\",\"en\":\"3Hentai (Chinese)\"}},\"capabilities\":{\"search\":false,\"mangaDetail\":true,\"chapters\":true,\"pages\":true,\"listings\":true,\"mangaList\":true,\"imageRequest\":true,\"home\":false,\"filters\":false,\"settings\":false,\"future\":{\"login\":false,\"auth\":false,\"process_page_image\":false,\"page_description\":false,\"base_url\":false,\"deeplink\":false,\"migration\":false}}}".as_bytes())
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
