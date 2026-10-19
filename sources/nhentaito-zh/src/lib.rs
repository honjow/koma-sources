#![no_std]
use koma_source_sdk::host::HtmlDescriptor;
use koma_source_sdk::{host, json_utils::*, source::SourceInfo};
type E = &'static str;
type R<T> = Result<T, E>;
const MAX: usize = 1048576;
const WEB: &[u8] = b"https://nhentai.to";

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.nhentaito.zh.koma",
    name: "NHentai.to",
    version: "0.1.2",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "NHentai.to 中文漫画目录源。",
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
    if !url.starts_with(b"https://nhentai.to/") {
        return Err("metadata host");
    }
    let b = http_req_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"version\":1,\"method\":\"GET\",\"url\":")?;
    q(b, &mut n, url)?;
    bytes(b,&mut n,b",\"headers\":{\"Referer\":\"https://nhentai.to/\",\"Origin\":\"https://nhentai.to\",\"Cache-Control\":\"max-age=600\",\"User-Agent\":")?;
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
    if s.starts_with(b"/") && !s.starts_with(b"//") {
        make(&[WEB, s], b)
    } else if s.starts_with(b"https://nhentai.to/") {
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
        s.strip_prefix(b"https://nhentai.to/g/")
            .and_then(|s| s.strip_suffix(b"/"))
            .ok_or("work route")?,
    )
}
struct Asset<'a> {
    media: &'a [u8],
    file: &'a [u8],
    host: &'a [u8],
}
fn asset(s: &[u8]) -> R<Asset<'_>> {
    let (host, p) = if let Some(v) = s.strip_prefix(b"https://supercdn.site/galleries/") {
        (b"supercdn.site".as_slice(), v)
    } else if let Some(v) = s.strip_prefix(b"https://cdn.supercdn.site/galleries/") {
        (b"cdn.supercdn.site".as_slice(), v)
    } else {
        return Err("asset host");
    };
    let k = p.iter().position(|c| *c == b'/').ok_or("asset path")?;
    let media = ident(&p[..k])?;
    let file = &p[k + 1..];
    if file.is_empty()
        || file.len() > 40
        || !file.iter().all(|c| c.is_ascii_alphanumeric() || *c == b'.')
        || file == b"."
        || file == b".."
    {
        return Err("asset filename");
    };
    Ok(Asset { media, file, host })
}
fn extension(k: u8) -> R<&'static [u8]> {
    match k {
        b'j' => Ok(b"jpg"),
        b'p' => Ok(b"png"),
        b'w' => Ok(b"webp"),
        b'b' => Ok(b"bmp"),
        b'g' => Ok(b"gif"),
        _ => Err("unknown page extension"),
    }
}
fn detail_doc(id: &[u8]) -> R<Owned> {
    let mut u = [0; 512];
    let d = fetch(work_url(id, &mut u)?)?;
    let mut c = [0; 512];
    if sa(d.0, b"link[rel=canonical]", b"href", &mut c)? != work_url(id, &mut u)? {
        return Err("detail canonical owner");
    };
    let mut gb = [0; 64];
    let t = st(d.0, b"#bigcontainer h3#gallery_id", &mut gb)?;
    let compact = t.strip_prefix(b"#").ok_or("detail displayed owner")?;
    if trim_ascii(compact) != id {
        return Err("detail displayed owner");
    };
    let mut lb = [0; 64];
    if st(
        d.0,
        b"#info-block a[href='/language/chinese/'] .name",
        &mut lb,
    )? != b"chinese"
    {
        return Err("Chinese language declaration");
    };
    Ok(d)
}
fn count(d: HtmlDescriptor) -> R<usize> {
    let fields = All::new(d, b"#info-block .field-name")?;
    let mut found = None;
    for i in 0..fields.len {
        let mut b = [0; 8192];
        let s = txt(fields.at(i), &mut b)?;
        if let Some(p) = s.strip_prefix(b"Pages: ") {
            let n = number(p).ok_or("page count")?;
            if n == 0 || n > 1000 || found.is_some() {
                return Err("page capacity or ambiguity");
            };
            found = Some(n)
        }
    }
    found.ok_or("page count missing")
}
fn image_attr<'a>(d: HtmlDescriptor, selector: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    // Preserve the original image owner. The presence selector is a subset
    // of this single proven image, never permission to pick another image.
    let original = All::new(d, selector)?;
    if original.len != 1 {
        return Err("image owner ambiguity");
    }
    for k in [
        b"data-cfsrc".as_slice(),
        b"data-src",
        b"data-lazy-src",
        b"srcset",
        b"src",
    ] {
        let mut filter = [0u8; 256];
        let filtered = make(&[selector, b"[", k, b"]"], &mut filter)?;
        let matches = All::new(d, filtered)?;
        if matches.len == 0 {
            continue;
        }
        if matches.len != 1 {
            return Err("image attribute owner ambiguity");
        }
        let mut n = host::html_attr(matches.at(0), k, b).map_err(|_| "image attr")?;
        if n >= b.len() {
            return Err("image attr capacity");
        }
        if n == 0 {
            continue;
        }
        if k == b"srcset" {
            n = b[..n]
                .iter()
                .position(|c| *c == b',' || c.is_ascii_whitespace())
                .unwrap_or(n);
            if n == 0 {
                return Err("empty srcset descriptor");
            }
        }
        core::str::from_utf8(&b[..n]).map_err(|_| "image UTF8")?;
        return Ok(&b[..n]);
    }
    Err("image descriptor missing")
}
fn cover<'a>(d: HtmlDescriptor, b: &'a mut [u8]) -> R<&'a [u8]> {
    let s = image_attr(d, b"#cover > a > img", b)?;
    let a = asset(s)?;
    if !matches!(a.file, b"cover.jpg" | b"cover.webp" | b"cover.png") {
        return Err("cover template");
    };
    Ok(s)
}
fn emit_work(o: &mut [u8], n: &mut usize, id: &[u8], t: &[u8], c: &[u8]) -> R<()> {
    let mut u = [0; 512];
    let url = work_url(id, &mut u)?;
    bytes(o, n, b"{\"id\":\"nht:zh:")?;
    bytes(o, n, id)?;
    bytes(o, n, b"\",\"title\":")?;
    q(o, n, t)?;
    bytes(o, n, b",\"url\":")?;
    q(o, n, url)?;
    bytes(o, n, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(o, n, c)?;
    bytes(
        o,
        n,
        b"},\"contentRating\":\"unknown\",\"links\":[{\"kind\":\"source\",\"url\":",
    )?;
    q(o, n, url)?;
    bytes(o, n, b"}]}")
}
fn cursor_page(s: &[u8]) -> R<usize> {
    let p = s
        .strip_prefix(b"https://nhentai.to/search?q=%2A&sort=most-favorited&lang=chinese&page=")
        .ok_or("listing cursor owner")?;
    ident(p)?;
    let n = number(p).ok_or("listing page")?;
    if n < 2 || n > 10000 {
        Err("listing cursor range")
    } else {
        Ok(n)
    }
}
fn listing(r: &[u8]) -> R<usize> {
    let a = args(r)?;
    if text(a, b"listingId")? != b"listing:popular" {
        return Err("unsupported listing");
    };
    let mut ub = [0; 2048];
    let mut current = 1;
    let uri = if let Some(p) = member(a, b"page")? {
        match member(p, b"cursor")? {
            Some(v) if v != b"null" => {
                let s = uq(v, &mut ub)?;
                current = cursor_page(s)?;
                s
            }
            _ => b"https://nhentai.to/search?q=*&sort=most-favorited&lang=chinese&page=1",
        }
    } else {
        b"https://nhentai.to/search?q=*&sort=most-favorited&lang=chinese&page=1"
    };
    let d = fetch(uri)?;
    let cards = All::new(d.0, b".gallery a:not([rel~=sponsored])")?;
    let next = All::new(d.0, b"a.next")?;
    if next.len > 1 {
        return Err("next ambiguity");
    };
    let mut nb = [0; 2048];
    let nxt = if next.len == 1 {
        let v = attr(next.at(0), b"href", &mut nb)?;
        if cursor_page(v)? != current.checked_add(1).ok_or("page overflow")? {
            return Err("next discontinuity");
        };
        if cards.len == 0 {
            return Err("listing no progress");
        };
        Some(v)
    } else {
        None
    };
    let mut seen = [[0; 20]; 1000];
    let mut lens = [0; 1000];
    let o = payload_buf();
    let mut n = 0;
    bytes(o, &mut n, b"{\"items\":[")?;
    for i in 0..cards.len {
        let c = cards.at(i);
        let mut hb = [0; 1024];
        let mut full = [0; 1024];
        let id = work_href(absolute(attr(c, b"href", &mut hb)?, &mut full)?)?;
        if (0..i).any(|j| &seen[j][..lens[j]] == id) {
            return Err("duplicate work");
        };
        seen[i][..id.len()].copy_from_slice(id);
        lens[i] = id.len();
        let mut tb = [0; 4096];
        let t = st(c, b".caption", &mut tb)?;
        if t.is_empty() {
            return Err("empty card title");
        };
        let mut cb = [0; 2048];
        let url = image_attr(c, b"img", &mut cb)?;
        let ac = asset(url)?;
        if !matches!(ac.file, b"thumb.jpg" | b"thumb.webp" | b"thumb.png") {
            return Err("cover template");
        };
        if i > 0 {
            bytes(o, &mut n, b",")?
        };
        emit_work(o, &mut n, id, t, url)?
    }
    bytes(o, &mut n, b"],\"page\":{\"hasMore\":")?;
    bytes(o, &mut n, if nxt.is_some() { b"true" } else { b"false" })?;
    bytes(o, &mut n, b",\"nextCursor\":")?;
    if let Some(v) = nxt {
        q(o, &mut n, v)?
    } else {
        bytes(o, &mut n, b"null")?
    };
    bytes(o, &mut n, b"}}")?;
    Ok(n)
}
fn detail(r: &[u8]) -> R<usize> {
    let id = key(r, b"mangaId", b"nht:zh:")?;
    let d = detail_doc(id)?;
    let total = count(d.0)?;
    let mut tb = [0; 4096];
    let t = st(d.0, b"#info h1", &mut tb)?;
    if t.is_empty() {
        return Err("detail title");
    };
    let mut cb = [0; 2048];
    let c = cover(d.0, &mut cb)?;
    let authors = All::new(d.0, b"#info-block a[href^='/artist/'] .name")?;
    let tags = All::new(d.0, b"#info-block a[href^='/tag/'] .name")?;
    let o = payload_buf();
    let mut n = 0;
    bytes(o, &mut n, b"{\"manga\":")?;
    emit_work(o, &mut n, id, t, c)?;
    n -= 1;
    bytes(o, &mut n, b",\"status\":\"unknown\",\"authors\":[")?;
    for i in 0..authors.len {
        if i > 0 {
            bytes(o, &mut n, b",")?
        };
        let mut b = [0; 1024];
        let s = txt(authors.at(i), &mut b)?;
        if s.is_empty() {
            return Err("empty author");
        };
        q(o, &mut n, s)?
    }
    bytes(
        o,
        &mut n,
        b"],\"description\":\"Provider language: Chinese; Pages: ",
    )?;
    wn(o, &mut n, total)?;
    bytes(o, &mut n, b"\",\"tags\":[")?;
    for i in 0..tags.len {
        if i > 0 {
            bytes(o, &mut n, b",")?
        };
        let mut b = [0; 1024];
        let s = txt(tags.at(i), &mut b)?;
        if s.is_empty() {
            return Err("empty tag");
        };
        q(o, &mut n, s)?
    }
    bytes(o, &mut n, b"]}}")?;
    Ok(n)
}
fn chapters(r: &[u8]) -> R<usize> {
    let id = key(r, b"mangaId", b"nht:zh:")?;
    let d = detail_doc(id)?;
    count(d.0)?;
    let mut tb = [0; 4096];
    let t = st(d.0, b"#info h1", &mut tb)?;
    if t.is_empty() {
        return Err("chapter title");
    };
    let o = payload_buf();
    let mut n = 0;
    bytes(o, &mut n, b"{\"chapters\":[{\"id\":\"nhtgallery:zh:")?;
    bytes(o, &mut n, id)?;
    bytes(o, &mut n, b"\",\"mangaId\":\"nht:zh:")?;
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
#[derive(Clone, Copy)]
struct PageData {
    kind: u8,
    width: usize,
    height: usize,
}
fn reader_array(id: &[u8], expected_media: &[u8], total: usize) -> R<[PageData; 1000]> {
    let html = &body_buf()[..unsafe { BODY_LEN }];
    let marker = b"var reader = new N.reader({";
    let start = find_subslice(html, marker).ok_or("reader declaration missing")? + marker.len();
    if find_subslice(&html[start..], marker).is_some() {
        return Err("reader declaration ambiguity");
    };
    let tail = &html[start..];
    if !tail.starts_with(b"\n") && tail.is_empty() {
        return Err("reader boundary");
    };
    let media_url = b"media_url: 'https://supercdn.site/'";
    if find_subslice(tail, media_url).is_none() {
        return Err("reader media host");
    };
    let gs = find_subslice(tail, b"gallery:").ok_or("reader gallery missing")? + b"gallery:".len();
    let p = ws(tail, gs);
    let end = json_value_end(tail, p, 0).ok_or("reader gallery JSON")?;
    let g = &tail[p..end];
    valid_json(g)?;
    if ident(scalar(required(g, b"id")?)?)? != id
        || ident(text(g, b"media_id")?)? != expected_media
        || num(required(g, b"num_pages")?)? != total
    {
        return Err("reader gallery owner/count");
    };
    let rest = &tail[end..];
    let mut at = ws(rest, 0);
    if rest.get(at) != Some(&b',') {
        return Err("reader start page separator");
    }
    at = ws(rest, at + 1);
    let field = b"start_page:";
    if rest.get(at..at + field.len()) != Some(field.as_slice()) {
        return Err("reader start page field");
    }
    at = ws(rest, at + field.len());
    let begin = at;
    while rest.get(at).map(|c| c.is_ascii_digit()).unwrap_or(false) {
        at += 1;
    }
    if ident(&rest[begin..at]).and_then(|v| number(v).ok_or("reader start integer"))? != 1 {
        return Err("reader start page not one");
    }
    at = ws(rest, at);
    if rest.get(at..at + 3) != Some(b"});") {
        return Err("reader start page end delimiter");
    }
    let pages = required(required(g, b"images")?, b"pages")?;
    if pages.first() != Some(&b'[') {
        return Err("reader pages array");
    };
    let mut p = ws(pages, 1);
    let mut got = 0;
    let mut out = [PageData {
        kind: 0,
        width: 0,
        height: 0,
    }; 1000];
    while pages.get(p) != Some(&b']') {
        if got >= total {
            return Err("reader pages count");
        };
        let end = json_value_end(pages, p, 0).ok_or("reader page record")?;
        let v = &pages[p..end];
        let k = text(v, b"t")?;
        if k.len() != 1 {
            return Err("page extension");
        };
        extension(k[0])?;
        let w = num(required(v, b"w")?)?;
        let h = num(required(v, b"h")?)?;
        if w == 0 || h == 0 || w > 100000 || h > 100000 {
            return Err("page dimensions");
        };
        out[got] = PageData {
            kind: k[0],
            width: w,
            height: h,
        };
        got += 1;
        p = ws(pages, end);
        if pages.get(p) == Some(&b',') {
            p = ws(pages, p + 1)
        } else if pages.get(p) != Some(&b']') {
            return Err("page array separator");
        }
    }
    if got != total {
        return Err("reader pages incomplete");
    };
    Ok(out)
}
fn page_url<'a>(media: &[u8], i: usize, k: u8, b: &'a mut [u8]) -> R<&'a [u8]> {
    let mut n = 0;
    bytes(b, &mut n, b"https://supercdn.site/galleries/")?;
    bytes(b, &mut n, media)?;
    bytes(b, &mut n, b"/")?;
    wn(b, &mut n, i + 1)?;
    bytes(b, &mut n, b".")?;
    bytes(b, &mut n, extension(k)?)?;
    Ok(&b[..n])
}
fn pages(r: &[u8]) -> R<usize> {
    let id = key(r, b"chapterId", b"nhtgallery:zh:")?;
    let d = detail_doc(id)?;
    let total = count(d.0)?;
    let mut cb = [0; 2048];
    let cov = cover(d.0, &mut cb)?;
    let ac = asset(cov)?;
    let mut media = [0; 20];
    let ml = ac.media.len();
    media[..ml].copy_from_slice(ac.media);
    let ts = All::new(d.0, b"#thumbnail-container .thumb-container")?;
    if ts.len != total {
        return Err("detail full thumbnail count");
    };
    let mut first = [0; 1024];
    let mut widths = [0usize; 1000];
    let mut heights = [0usize; 1000];
    for i in 0..total {
        let mut hb = [0; 1024];
        let mut full = [0; 1024];
        let uri = absolute(
            sa(ts.at(i), b"a.gallerythumb", b"href", &mut hb)?,
            &mut full,
        )?;
        let mut exp = [0; 1024];
        let mut n = 0;
        bytes(&mut exp, &mut n, WEB)?;
        bytes(&mut exp, &mut n, b"/g/")?;
        bytes(&mut exp, &mut n, id)?;
        bytes(&mut exp, &mut n, b"/")?;
        wn(&mut exp, &mut n, i + 1)?;
        bytes(&mut exp, &mut n, b"/")?;
        if uri != &exp[..n] {
            return Err("thumbnail reader owner/sequence");
        };
        if i == 0 {
            first[..uri.len()].copy_from_slice(uri)
        };
        let mut ib = [0; 2048];
        let im = sa(ts.at(i), b"img[data-src]", b"data-src", &mut ib)?;
        let mut wb = [0; 32];
        let mut hb = [0; 32];
        widths[i] =
            number(sa(ts.at(i), b"img[data-src]", b"width", &mut wb)?).ok_or("thumbnail width")?;
        heights[i] = number(sa(ts.at(i), b"img[data-src]", b"height", &mut hb)?)
            .ok_or("thumbnail height")?;
        let a = asset(im)?;
        let mut f = [0; 32];
        let mut z = 0;
        wn(&mut f, &mut z, i + 1)?;
        bytes(&mut f, &mut z, b"t.jpg")?;
        if a.media != &media[..ml] || a.host != b"supercdn.site" || a.file != &f[..z] {
            return Err("thumbnail media/sequence template");
        }
    }
    drop(ts);
    drop(d);
    let len = first
        .iter()
        .position(|c| *c == 0)
        .ok_or("first reader capacity")?;
    let rd = fetch(&first[..len])?;
    let mut c = [0; 1024];
    if sa(rd.0, b"link[rel=canonical]", b"href", &mut c)? != &first[..len] {
        return Err("reader canonical owner");
    };
    let backs = All::new(rd.0, b"#page-container a.back_btn")?;
    if backs.len == 0 {
        return Err("reader back missing");
    };
    for i in 0..backs.len {
        let mut b = [0; 1024];
        let mut f = [0; 1024];
        let mut w = [0; 1024];
        if absolute(attr(backs.at(i), b"href", &mut b)?, &mut f)? != work_url(id, &mut w)? {
            return Err("reader back owner");
        }
    }
    let displays = All::new(rd.0, b"#page-container .page-number")?;
    if displays.len == 0 {
        return Err("reader display missing");
    };
    let mut db = [0; 64];
    let mut dn = 0;
    bytes(&mut db, &mut dn, b"1 of ")?;
    wn(&mut db, &mut dn, total)?;
    for i in 0..displays.len {
        let mut b = [0; 64];
        if txt(displays.at(i), &mut b)? != &db[..dn] {
            return Err("reader page display");
        }
    }
    let ps = reader_array(id, &media[..ml], total)?;
    if (0..total).any(|i| ps[i].width != widths[i] || ps[i].height != heights[i]) {
        return Err("reader thumbnail dimensions");
    };
    let active = All::new(rd.0, b"#image-container img")?;
    if active.len != 1 {
        return Err("original image ambiguity");
    };
    let mut ib = [0; 2048];
    let im = attr(active.at(0), b"src", &mut ib)?;
    let mut expected = [0; 1024];
    if im != page_url(&media[..ml], 0, ps[0].kind, &mut expected)? {
        return Err("first original owner");
    };
    let o = payload_buf();
    let mut n = 0;
    bytes(o, &mut n, b"{\"pages\":[")?;
    for i in 0..total {
        if host::check_cancel() {
            return Err("cancelled");
        };
        if i > 0 {
            bytes(o, &mut n, b",")?
        };
        bytes(o, &mut n, b"{\"id\":\"nhtpage:zh:")?;
        bytes(o, &mut n, id)?;
        bytes(o, &mut n, b":")?;
        bytes(o, &mut n, &media[..ml])?;
        bytes(o, &mut n, b":")?;
        wn(o, &mut n, i)?;
        bytes(o, &mut n, b":")?;
        bytes(o, &mut n, &[ps[i].kind])?;
        bytes(o, &mut n, b"\",\"index\":")?;
        wn(o, &mut n, i)?;
        bytes(o, &mut n, b",\"image\":{\"kind\":\"url\",\"url\":")?;
        let mut b = [0; 1024];
        q(o, &mut n, page_url(&media[..ml], i, ps[i].kind, &mut b)?)?;
        bytes(o, &mut n, b"}}")?
    }
    bytes(o, &mut n, b"]}")?;
    Ok(n)
}
fn image_request(r: &[u8]) -> R<usize> {
    let a = args(r)?;
    let mut ub = [0; 2048];
    let uri = uq(required(a, b"url")?, &mut ub)?;
    let ac = asset(uri)?;
    let mut pb = [0; 256];
    let pid = uq(required(a, b"pageId")?, &mut pb)?;
    let mut rb = [0; 1024];
    let route = uq(required(a, b"mangaUrl")?, &mut rb)?;
    let id;
    if let Some(p) = pid.strip_prefix(b"nhtpage:zh:") {
        let mut f = p.split(|c| *c == b':');
        id = ident(f.next().ok_or("page owner")?)?;
        let media = ident(f.next().ok_or("page media")?)?;
        let idx = f.next().ok_or("page index")?;
        let index = number(idx).ok_or("page index")?;
        if index >= 1000 || (idx.len() > 1 && idx[0] == b'0') {
            return Err("page index capacity");
        };
        let kind = f.next().ok_or("page format")?;
        if kind.len() != 1 || f.next().is_some() {
            return Err("page identity");
        };
        let mut expected = [0; 1024];
        if uri != page_url(media, index, kind[0], &mut expected)? {
            return Err("image media/sequence");
        }
    } else {
        id = ident(pid.strip_prefix(b"nht:zh:").ok_or("cover identity")?)?;
        if !matches!(
            ac.file,
            b"cover.jpg"
                | b"cover.webp"
                | b"cover.png"
                | b"thumb.jpg"
                | b"thumb.webp"
                | b"thumb.png"
        ) {
            return Err("cover file");
        }
    }
    let mut b = [0; 1024];
    if route != work_url(id, &mut b)? {
        return Err("independent work route");
    };
    let o = payload_buf();
    let mut n = 0;
    bytes(o, &mut n, b"{\"imageRequest\":{\"method\":\"GET\",\"url\":")?;
    q(o, &mut n, uri)?;
    bytes(o,&mut n,b",\"headers\":{\"Referer\":\"https://nhentai.to/\",\"Origin\":\"https://nhentai.to\",\"Cache-Control\":\"max-age=600\",\"User-Agent\":")?;
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
    response_buffer().write_success("source_info","{\"sourceInfo\":{\"id\":\"com.nhentaito.zh.koma\",\"name\":\"NHentai.to\",\"version\":\"0.1.2\",\"apiVersion\":\"0.2\",\"language\":\"zh\",\"author\":\"Koma\",\"description\":\"NHentai.to 中文漫画目录源。\",\"contentRating\":\"adult\",\"localizedNames\":{\"zh-Hans\":\"NHentai.to（中文）\",\"zh-Hant\":\"NHentai.to（中文）\",\"en\":\"NHentai.to (Chinese)\"}},\"capabilities\":{\"search\":false,\"mangaDetail\":true,\"chapters\":true,\"pages\":true,\"listings\":true,\"mangaList\":true,\"imageRequest\":true,\"home\":false,\"filters\":false,\"settings\":false,\"future\":{\"login\":false,\"auth\":false,\"process_page_image\":false,\"page_description\":false,\"base_url\":false,\"deeplink\":false,\"migration\":false}}}".as_bytes())
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
