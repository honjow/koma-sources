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
const WEB: &[u8] = b"https://www.cartoon18.com";
const SOURCE_INFO:SourceInfo=SourceInfo{id:"com.cartoon18.zh.koma",name:"Cartoon18",version:"0.1.0",api_version:"0.2",language:"zh",author:"Koma",description:"Cartoon18 published Chinese interface and anonymous manga metadata. Per-work rating, artwork availability and usage unknown.",content_rating:"nsfw"};
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
static mut SEEN: [[u8; 256]; 1000] = [[0; 256]; 1000];
const UA: &[u8] =
    b"Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:121.0) Gecko/20100101 Firefox/121.0";
fn fetch(url: &[u8]) -> R<Owned> {
    if host::check_cancel() {
        return Err("cancelled");
    }
    if !url.starts_with(b"https://www.cartoon18.com/") {
        return Err("metadata host");
    }
    let d = http_req_buf();
    let mut n = 0;
    bytes(d, &mut n, b"{\"version\":1,\"method\":\"GET\",\"url\":")?;
    q(d, &mut n, url)?;
    bytes(d, &mut n, b",\"headers\":{\"User-Agent\":")?;
    q(d, &mut n, UA)?;
    bytes(d,&mut n,b",\"Referer\":\"https://www.cartoon18.com/\",\"Origin\":\"https://www.cartoon18.com\",\"Accept\":\"text/html\"},\"responseKind\":\"bodyText\",\"timeoutMs\":15000}")?;
    let z = host::http_request(&d[..n], http_out()).map_err(|_| "transport error")?;
    if z >= MAX {
        return Err("HTTP envelope capacity");
    }
    let env = &http_out()[..z];
    valid_json(env)?;
    let status = num(member(env, b"status")?
        .or(member(env, b"statusCode")?)
        .ok_or("HTTP status missing")?)?;
    if !(200..300).contains(&status) {
        return Err("HTTP status error");
    }
    let n =
        koma_source_sdk::decode_json_body_into(env, body_buf()).map_err(|_| "HTTP body decode")?;
    if n >= MAX {
        return Err("HTML body capacity");
    }
    core::str::from_utf8(&body_buf()[..n]).map_err(|_| "HTML UTF8")?;
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
fn slug(v: &[u8]) -> R<&[u8]> {
    if v.is_empty() || v.len() > 128 || v.iter().any(|c| !c.is_ascii_alphanumeric()) {
        Err("work identity")
    } else {
        Ok(v)
    }
}
fn cid(v: &[u8]) -> R<&[u8]> {
    if v.is_empty() || v.len() > 15 || v[0] == b'0' || number(v).is_none() {
        Err("chapter identity")
    } else {
        Ok(v)
    }
}
fn work_href(v: &[u8]) -> R<&[u8]> {
    slug(
        v.strip_prefix(WEB)
            .unwrap_or(v)
            .strip_prefix(b"/zh-hans/v/")
            .ok_or("work href")?,
    )
}
fn chapter_href(v: &[u8]) -> R<&[u8]> {
    cid(v
        .strip_prefix(WEB)
        .unwrap_or(v)
        .strip_prefix(b"/zh-hans/story/")
        .and_then(|x| x.strip_suffix(b"/full"))
        .ok_or("chapter href")?)
}
fn work(r: &[u8]) -> R<&[u8]> {
    slug(
        text(args(r)?, b"mangaId")?
            .strip_prefix(b"c18:zh:")
            .ok_or("work namespace")?,
    )
}
fn chapter(r: &[u8]) -> R<(&[u8], &[u8])> {
    let v = text(args(r)?, b"chapterId")?
        .strip_prefix(b"c18ch:zh:")
        .ok_or("chapter namespace")?;
    let z = v.iter().position(|c| *c == b':').ok_or("chapter parts")?;
    Ok((slug(&v[..z])?, cid(&v[z + 1..])?))
}
fn nonempty(v: &[u8]) -> R<&[u8]> {
    core::str::from_utf8(v).map_err(|_| "text UTF8")?;
    let v = trim_ascii(v);
    if v.is_empty() {
        Err("empty text")
    } else {
        Ok(v)
    }
}
fn image_url(v: &[u8], cover: bool) -> R<()> {
    let x = v
        .strip_prefix(if cover {
            b"https://img.cartoon18.com/images/node/".as_slice()
        } else {
            b"https://img.cartoon18.com/images/image/".as_slice()
        })
        .ok_or("image host/path")?;
    let pos = x
        .iter()
        .position(|c| *c == b'?')
        .ok_or("image query missing")?;
    let path = &x[..pos];
    let query = &x[pos + 1..];
    if query.is_empty()
        || query.len() > 32
        || query.iter().any(|c| !c.is_ascii_digit())
        || !path.ends_with(b".avif")
    {
        return Err("image filename/query");
    };
    let base = &path[..path.len() - 5];
    let z = base.iter().position(|c| *c == b'/').ok_or("image path")?;
    if number(&base[..z]).is_none() || number(&base[z + 1..]).is_none() {
        return Err("image path");
    };
    Ok(())
}
fn canonical(d: HtmlDescriptor, path: &[u8]) -> R<()> {
    let xs = All::new(d, b"link[rel=canonical]")?;
    if xs.len == 0 {
        return Err("canonical missing");
    };
    for i in 0..xs.len {
        let mut b = [0; 2048];
        let v = attr(xs.at(i), b"href", &mut b)?;
        if v.strip_prefix(WEB).unwrap_or(v) != path {
            return Err("canonical owner");
        }
    }
    Ok(())
}
fn detail_doc(w: &[u8]) -> R<Owned> {
    let mut b = [0; 512];
    let path = make(&[b"/zh-hans/v/", w], &mut b)?;
    let mut u = [0; 1024];
    let d = fetch(make(&[WEB, path], &mut u)?)?;
    canonical(d.0, path)?;
    Ok(d)
}
fn cover<'a>(d: HtmlDescriptor, b: &'a mut [u8]) -> R<&'a [u8]> {
    let xs = All::new(d, b"div.content h1.title + div + div.row a img")?;
    if xs.len != 1 {
        return Err("detail cover template");
    };
    let u = attr(xs.at(0), b"src", b)?;
    image_url(u, true)?;
    Ok(u)
}
// Current published h1 ownText consists of a text prefix plus one trailing Vue button.
// Subtract only that exact trailing child's DOM text; never return Vue placeholder text.
fn title<'a>(d: HtmlDescriptor, b: &'a mut [u8]) -> R<&'a [u8]> {
    let hs = All::new(d, b"div.content h1.title")?;
    if hs.len != 1 {
        return Err("title template");
    };
    let children = All::new(hs.at(0), b"h1.title > *")?;
    let full = txt(hs.at(0), b)?;
    if children.len == 0 {
        return nonempty(full);
    };
    if children.len != 1 {
        return Err("title child template");
    };
    let buttons = All::new(hs.at(0), b"h1.title > button#app")?;
    if buttons.len != 1 {
        return Err("title child template");
    };
    let mut vb = [0; 512];
    let vue = nonempty(txt(buttons.at(0), &mut vb)?)?;
    nonempty(full.strip_suffix(vue).ok_or("title own text template")?)
}
fn metadata_text<'a>(d: HtmlDescriptor, s: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    let x = All::new(d, s)?;
    if x.len == 0 {
        return Ok(b"");
    };
    if x.len != 2 {
        return Err("metadata template");
    };
    txt(x.at(1), b)
}
fn catalogue(d: HtmlDescriptor) -> R<All> {
    let a = All::new(d, b"div.content h1.title + div a")?;
    if a.len == 0 {
        return Err("empty/gated catalogue");
    };
    for i in 0..a.len {
        let mut h = [0; 512];
        let id = chapter_href(attr(a.at(i), b"href", &mut h)?)?;
        let mut t = [0; 4096];
        nonempty(txt(a.at(i), &mut t)?)?;
        unsafe {
            for j in 0..i {
                let n = number(&SEEN[j][..15].split(|x| *x == 0).next().unwrap_or(b""));
                if n == number(id) {
                    return Err("duplicate chapter");
                }
            }
            SEEN[i] = [0; 256];
            SEEN[i][..id.len()].copy_from_slice(id)
        }
    }
    Ok(a)
}
fn chapter_number(t: &[u8]) -> Option<&[u8]> {
    let t = trim_ascii(t);
    let t = t.strip_prefix("第".as_bytes()).unwrap_or(t);
    let n = t.iter().take_while(|c| c.is_ascii_digit()).count();
    if n == 0 {
        return None;
    };
    let rest = trim_ascii(&t[n..]);
    if !(rest == "话".as_bytes() || rest == "話".as_bytes()) {
        return None;
    };
    let digits = &t[..n];
    let lead = digits
        .iter()
        .position(|c| *c != b'0')
        .unwrap_or(digits.len() - 1);
    Some(&digits[lead..])
}
fn detail(r: &[u8], chapters: bool) -> R<usize> {
    let w = work(r)?;
    let d = detail_doc(w)?;
    let out = payload_buf();
    let mut n = 0;
    if chapters {
        let a = catalogue(d.0)?;
        bytes(out, &mut n, b"{\"chapters\":[")?;
        for i in 0..a.len {
            let index = a.len - 1 - i;
            let mut h = [0; 512];
            let c = chapter_href(attr(a.at(index), b"href", &mut h)?)?;
            let mut t = [0; 4096];
            let name = nonempty(txt(a.at(index), &mut t)?)?;
            if i > 0 {
                bytes(out, &mut n, b",")?
            };
            bytes(out, &mut n, b"{\"id\":\"c18ch:zh:")?;
            bytes(out, &mut n, w)?;
            bytes(out, &mut n, b":")?;
            bytes(out, &mut n, c)?;
            bytes(out, &mut n, b"\",\"mangaId\":\"c18:zh:")?;
            bytes(out, &mut n, w)?;
            bytes(out, &mut n, b"\",\"title\":")?;
            q(out, &mut n, name)?;
            bytes(out, &mut n, b",\"chapterNumber\":")?;
            if let Some(x) = chapter_number(name) {
                bytes(out, &mut n, x)?
            } else {
                bytes(out, &mut n, b"null")?
            };
            bytes(out, &mut n, b",\"volumeNumber\":null,\"language\":\"zh\"}")?
        }
        bytes(out, &mut n, b"]}")?
    } else {
        let mut t = [0; 4096];
        let name = title(d.0, &mut t)?;
        let mut cb = [0; 1024];
        let cv = cover(d.0, &mut cb)?;
        let mut ab = [0; 4096];
        let au = metadata_text(
            d.0,
            b"div.content h1.title ~ div.row div.my-2 i.fa-user ~ span",
            &mut ab,
        )?;
        let mut db = [0; 32768];
        let desc = metadata_text(
            d.0,
            b"div.content h1.title ~ div.row div.my-2 i.fa-list ~ span",
            &mut db,
        )?;
        bytes(out, &mut n, b"{\"manga\":{\"id\":\"c18:zh:")?;
        bytes(out, &mut n, w)?;
        bytes(out, &mut n, b"\",\"title\":")?;
        q(out, &mut n, name)?;
        bytes(out, &mut n, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
        q(out, &mut n, cv)?;
        bytes(out, &mut n, b"},\"authors\":[")?;
        if !au.is_empty() {
            q(out, &mut n, au)?
        };
        bytes(out, &mut n, b"],\"description\":")?;
        q(out, &mut n, desc)?;
        bytes(out, &mut n, b",\"contentRating\":\"unknown\",\"tags\":[")?;
        let tags = All::new(
            d.0,
            b"div.content h1.title ~ div.row div.my-2 i.fa-tag ~ span a",
        )?;
        for i in 0..tags.len {
            let mut b = [0; 1024];
            let tag = nonempty(txt(tags.at(i), &mut b)?)?;
            if i > 0 {
                bytes(out, &mut n, b",")?
            };
            q(out, &mut n, tag)?
        }
        bytes(out, &mut n, b"],\"links\":[{\"kind\":\"source\",\"url\":\"")?;
        bytes(out, &mut n, WEB)?;
        bytes(out, &mut n, b"/zh-hans/v/")?;
        bytes(out, &mut n, w)?;
        bytes(out, &mut n, b"\"}]}}")?
    };
    Ok(n)
}
fn chapter_doc(w: &[u8], c: &[u8]) -> R<Owned> {
    let d = detail_doc(w)?;
    let a = catalogue(d.0)?;
    let mut found = false;
    for i in 0..a.len {
        let mut h = [0; 512];
        if chapter_href(attr(a.at(i), b"href", &mut h)?)? == c {
            found = true
        }
    }
    if !found {
        return Err("chapter membership");
    };
    drop(a);
    drop(d);
    let mut pb = [0; 512];
    let path = make(&[b"/zh-hans/story/", c, b"/full"], &mut pb)?;
    let mut ub = [0; 1024];
    let doc = fetch(make(&[WEB, path], &mut ub)?)?;
    canonical(doc.0, path)?;
    let mut hb = [0; 512];
    let wh = make(&[b"/zh-hans/v/", w], &mut hb)?;
    let mut sel = [0; 600];
    let selector = make(&[b"a[href=\"", wh, b"\"]"], &mut sel)?;
    let backlinks = All::new(doc.0, selector)?;
    if backlinks.len != 1 {
        return Err("chapter work backlink");
    };
    Ok(doc)
}
fn page_collect(d: HtmlDescriptor) -> R<usize> {
    let roots = All::new(d, b"div#app")?;
    if roots.len != 1 {
        return Err("page template");
    };
    let ims = All::new(d, b"div#app img")?;
    let containers = All::new(d, b"div#app div[data-seq]")?;
    if ims.len == 0 || ims.len != containers.len {
        return Err("page template count");
    };
    for i in 0..ims.len {
        let mut b = [0; 32];
        let seq = number(attr(containers.at(i), b"data-seq", &mut b)?).ok_or("page sequence")?;
        if seq != i + 1 {
            return Err("page sequence");
        };
        let childimgs = All::new(containers.at(i), b"img")?;
        if childimgs.len != 1 {
            return Err("page container");
        };
        let mut u = [0; 1024];
        let u1 = attr(ims.at(i), b"src", &mut u)?;
        let mut fb = [0; 1024];
        let v = if u1.is_empty() {
            attr(ims.at(i), b"data-src", &mut fb)?
        } else {
            u1
        };
        image_url(v, false)?;
        let mut ib = [0; 1024];
        if attr(childimgs.at(0), b"src", &mut ib)? != v {
            return Err("page container order");
        };
        unsafe {
            for j in 0..i {
                if &PAGES[j][..PAGE_LEN[j]] == v {
                    return Err("duplicate image");
                }
            }
            PAGES[i][..v.len()].copy_from_slice(v);
            PAGE_LEN[i] = v.len()
        }
    }
    Ok(ims.len)
}
fn pages(r: &[u8]) -> R<usize> {
    let (w, c) = chapter(r)?;
    let d = chapter_doc(w, c)?;
    let count = page_collect(d.0)?;
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"pages\":[")?;
    for i in 0..count {
        if i > 0 {
            bytes(out, &mut n, b",")?
        };
        bytes(out, &mut n, b"{\"id\":\"c18pg:zh:")?;
        bytes(out, &mut n, w)?;
        bytes(out, &mut n, b":")?;
        bytes(out, &mut n, c)?;
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
    let mut pb = [0; 1024];
    let p = uq(required(a, b"pageId")?, &mut pb)?;
    let mut ub = [0; 1024];
    let url = uq(required(a, b"url")?, &mut ub)?;
    if let Some(w) = p.strip_prefix(b"c18:zh:") {
        let d = detail_doc(slug(w)?)?;
        let mut b = [0; 1024];
        if cover(d.0, &mut b)? != url {
            return Err("cover owner");
        }
    } else {
        let p = p.strip_prefix(b"c18pg:zh:").ok_or("page namespace")?;
        let z = p.iter().position(|x| *x == b':').ok_or("page parts")?;
        let w = slug(&p[..z])?;
        let p = &p[z + 1..];
        let z = p.iter().position(|x| *x == b':').ok_or("page parts")?;
        let c = cid(&p[..z])?;
        let i = number(&p[z + 1..]).ok_or("page index")?;
        let d = chapter_doc(w, c)?;
        let count = page_collect(d.0)?;
        if i >= count {
            return Err("page range");
        };
        unsafe {
            if &PAGES[i][..PAGE_LEN[i]] != url {
                return Err("page URL owner");
            }
        }
    };
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"url\":")?;
    q(out, &mut n, url)?;
    bytes(out, &mut n, b",\"headers\":{\"User-Agent\":")?;
    q(out, &mut n, UA)?;
    bytes(
        out,
        &mut n,
        b",\"Referer\":\"https://www.cartoon18.com/\",\"Origin\":\"https://www.cartoon18.com\"}}",
    )?;
    Ok(n)
}
fn encoded<'a>(v: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    let mut n = 0;
    if !write_url_encoded(b, &mut n, v) {
        return Err("query capacity");
    };
    Ok(&b[..n])
}
fn next_path(v: &[u8], query: &[u8], search: bool) -> R<usize> {
    let p = v.strip_prefix(WEB).unwrap_or(v);
    if p.len() > 2048 || p.contains(&b'#') {
        return Err("cursor path");
    };
    let z = p.iter().position(|c| *c == b'?').ok_or("cursor query")?;
    let route = &p[..z];
    let kv = &p[z + 1..];
    let mut encoded_query = [0; 2048];
    let eq = encoded(query, &mut encoded_query)?;
    let mut qb = [0; 2100];
    if search {
        if route != make(&[b"/zh-hans/q/", eq], &mut qb)? {
            return Err("cursor query owner");
        }
    } else if route != b"/zh-hans" {
        return Err("cursor listing owner");
    };
    let (mut pg, mut sort, mut per) = (None, false, false);
    for param in kv.split(|c| *c == b'&') {
        let z = param
            .iter()
            .position(|c| *c == b'=')
            .ok_or("cursor parameter")?;
        let (k, v) = (&param[..z], &param[z + 1..]);
        match k {
            b"page" => {
                if pg.is_some() {
                    return Err("duplicate cursor page");
                };
                pg = number(v)
            }
            b"sort" => {
                if sort
                    || v != if search {
                        b"score".as_slice()
                    } else {
                        b"hits".as_slice()
                    }
                {
                    return Err("cursor sort");
                };
                sort = true
            }
            b"per-page" => {
                if per || v != b"24" {
                    return Err("cursor limit");
                };
                per = true
            }
            _ => return Err("unknown cursor parameter"),
        }
    }
    let pg = pg.ok_or("cursor page")?;
    if !sort || pg < 2 || pg > 1024 {
        return Err("cursor page/sort");
    };
    Ok(pg)
}
fn list(r: &[u8], search: bool) -> R<usize> {
    let a = args(r)?;
    let mut qb = [0; 512];
    let query = if search {
        nonempty(trim_ascii(uq(required(a, b"query")?, &mut qb)?))?
    } else {
        if let Some(v) = member(a, b"listingId")? {
            if quoted(v)? != b"listing:popular" {
                return Err("listing identity");
            }
        };
        b""
    };
    let mut path = [0; 2048];
    let mut queryenc = [0; 2048];
    let mut page = 1;
    let mut pathn = 0;
    let mut cbuf = [0; 4096];
    if let Some(p) = member(a, b"page")? {
        if p.first() != Some(&b'{') {
            return Err("page envelope");
        };
        if let Some(v) = member(p, b"cursor")? {
            if v != b"null" {
                let cur = uq(v, &mut cbuf)?;
                valid_json(cur)?;
                let mut saved = [0; 512];
                if uq(required(cur, b"q")?, &mut saved)? != query {
                    return Err("cursor query owner");
                };
                if text(cur, b"mode")?
                    != if search {
                        b"search".as_slice()
                    } else {
                        b"popular".as_slice()
                    }
                {
                    return Err("cursor mode");
                };
                let mut href = [0; 2048];
                let v = uq(required(cur, b"href")?, &mut href)?;
                page = next_path(v, query, search)?;
                let v = v.strip_prefix(WEB).unwrap_or(v);
                bytes(&mut path, &mut pathn, v)?
            }
        }
    };
    if pathn == 0 {
        bytes(&mut path, &mut pathn, b"/zh-hans?")?;
        if search {
            bytes(&mut path, &mut pathn, b"q=")?;
            bytes(&mut path, &mut pathn, encoded(query, &mut queryenc)?)?;
            bytes(&mut path, &mut pathn, b"&page=1&sort=score")?
        } else {
            bytes(&mut path, &mut pathn, b"sort=hits&page=1")?
        }
    };
    let mut ub = [0; 2200];
    let doc = fetch(make(&[WEB, &path[..pathn]], &mut ub)?)?;
    let cards = All::new(doc.0, b"#videos div.card")?;
    let nexts = All::new(doc.0, b"nav .pagination .next")?;
    if nexts.len > 1 {
        return Err("pagination ambiguity");
    };
    let mut next = [0; 2048];
    let mut nextn = 0;
    if nexts.len == 1 {
        let mut cl = [0; 256];
        let classes = attr(nexts.at(0), b"class", &mut cl)?;
        let disabled = classes
            .split(|c| c.is_ascii_whitespace())
            .any(|x| x == b"disabled");
        if !disabled {
            let links = All::new(nexts.at(0), b"a[href]")?;
            if links.len != 1 {
                return Err("next href template");
            };
            let mut hb = [0; 2048];
            let h = attr(links.at(0), b"href", &mut hb)?;
            if next_path(h, query, search)? != page.checked_add(1).ok_or("next page overflow")? {
                return Err("next page progress");
            };
            bytes(&mut next, &mut nextn, h.strip_prefix(WEB).unwrap_or(h))?
        }
    };
    if cards.len == 0 && nextn > 0 {
        return Err("empty nonterminal page");
    };
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"items\":[")?;
    for i in 0..cards.len {
        let links = All::new(cards.at(i), b".lines-2 a")?;
        if links.len != 1 {
            return Err("card title template");
        };
        let mut h = [0; 512];
        let w = work_href(attr(links.at(0), b"href", &mut h)?)?;
        unsafe {
            for j in 0..i {
                let prev = SEEN[j].split(|c| *c == 0).next().unwrap_or(b"");
                if prev == w {
                    return Err("duplicate work");
                }
            }
            SEEN[i] = [0; 256];
            SEEN[i][..w.len()].copy_from_slice(w)
        };
        let mut t = [0; 8192];
        let name = nonempty(txt(links.at(0), &mut t)?)?;
        let images = All::new(cards.at(i), b".embed-responsive img")?;
        if images.len != 1 {
            return Err("card cover template");
        };
        let mut ib = [0; 1024];
        let cv = attr(images.at(0), b"data-src", &mut ib)?;
        image_url(cv, true)?;
        if i > 0 {
            bytes(out, &mut n, b",")?
        };
        bytes(out, &mut n, b"{\"id\":\"c18:zh:")?;
        bytes(out, &mut n, w)?;
        bytes(out, &mut n, b"\",\"title\":")?;
        q(out, &mut n, name)?;
        bytes(out, &mut n, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
        q(out, &mut n, cv)?;
        bytes(
            out,
            &mut n,
            b"},\"contentRating\":\"unknown\",\"sourceTags\":[",
        )?;
        let tags = All::new(cards.at(i), b".card-body div a.badge")?;
        let mut tc = 0;
        for j in 0..tags.len {
            let mut b = [0; 1024];
            let tag = txt(tags.at(j), &mut b)?;
            if !tag.is_empty() {
                if tc > 0 {
                    bytes(out, &mut n, b",")?
                };
                q(out, &mut n, tag)?;
                tc += 1
            }
        }
        bytes(out, &mut n, b"]}")?
    }
    bytes(out, &mut n, b"],\"page\":{\"hasMore\":")?;
    bytes(out, &mut n, if nextn > 0 { b"true" } else { b"false" })?;
    bytes(out, &mut n, b",\"nextCursor\":")?;
    if nextn == 0 {
        bytes(out, &mut n, b"null")?
    } else {
        let mut cur = [0; 4096];
        let mut cn = 0;
        bytes(&mut cur, &mut cn, b"{\"q\":")?;
        q(&mut cur, &mut cn, query)?;
        bytes(&mut cur, &mut cn, b",\"mode\":")?;
        q(
            &mut cur,
            &mut cn,
            if search {
                b"search".as_slice()
            } else {
                b"popular".as_slice()
            },
        )?;
        bytes(&mut cur, &mut cn, b",\"href\":")?;
        q(&mut cur, &mut cn, &next[..nextn])?;
        bytes(&mut cur, &mut cn, b"}")?;
        q(out, &mut n, &cur[..cn])?
    };
    bytes(out, &mut n, b"}}")?;
    Ok(n)
}

fn execute(op: &str, r: R<usize>) -> u32 {
    match r {
        Ok(n) => write_success_payload(op, n),
        Err(e) => write_error(op, "source_error", e),
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
    let p = payload_buf();
    let v =
        "{\"listings\":[{\"id\":\"listing:popular\",\"name\":\"热门\",\"supportsPaging\":true}]}"
            .as_bytes();
    p[..v.len()].copy_from_slice(v);
    write_success_payload("get_listings", v.len())
}
fn run_get_home(_: &[u8]) -> u32 {
    write_error("get_home", "not_implemented", "not supported")
}
fn run_get_settings(_: &[u8]) -> u32 {
    write_error("get_settings", "not_implemented", "not supported")
}
fn run_get_filters(_: &[u8]) -> u32 {
    write_error("get_filters", "not_implemented", "not supported")
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
    response_buffer().write_success_parts("source_info", &[
 b"{\"sourceInfo\":{\"id\":\"", SOURCE_INFO.id.as_bytes(), b"\",\"name\":\"", SOURCE_INFO.name.as_bytes(), b"\",\"version\":\"", SOURCE_INFO.version.as_bytes(), b"\",\"apiVersion\":\"0.2\",\"language\":\"zh\",\"author\":\"Koma\",\"description\":\"",SOURCE_INFO.description.as_bytes(),
 "\",\"contentRating\":\"nsfw\",\"localizedNames\":{\"zh-Hans\":\"色漫网\",\"zh-Hant\":\"色漫網\",\"en\":\"Cartoon18 (Chinese)\"}},\"capabilities\":{\"search\":true,\"mangaDetail\":true,\"chapters\":true,\"pages\":true,\"listings\":true,\"mangaList\":true,\"imageRequest\":true,\"home\":false,\"filters\":false,\"settings\":false,\"future\":{\"login\":false,\"auth\":false,\"process_page_image\":false,\"page_description\":false,\"base_url\":false,\"deeplink\":false,\"migration\":false}}}".as_bytes()
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
