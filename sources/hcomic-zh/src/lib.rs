#![no_std]
// Independent H-Comic metadata source; fixed primary1319949. No picture fetch in retained validation.
use koma_source_sdk::{
    host,
    json_utils::*,
    source::{SourceCapabilities, SourceInfo},
};
type E = &'static str;
const MAX: usize = 1048576;
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.hcomic.zh.koma",
    name: "H-Comic",
    version: "0.1.2",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "H-Comic public Chinese metadata. NSFW source; per-book rating and usage unknown.",
    content_rating: "nsfw",
};
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
static mut URL: [u8; 32768] = [0; 32768];
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
struct Array<'a> {
    s: &'a [u8],
    p: usize,
}
impl<'a> Array<'a> {
    fn new(s: &'a [u8]) -> Result<Self, E> {
        if s.first() != Some(&b'[') || s.last() != Some(&b']') {
            return Err("expected array");
        }
        Ok(Self { s, p: ws(s, 1) })
    }
    fn next(&mut self) -> Result<Option<&'a [u8]>, E> {
        if self.s.get(self.p) == Some(&b']') {
            return Ok(None);
        }
        let end = json_value_end(self.s, self.p, 0).ok_or("invalid array entry")?;
        let v = &self.s[self.p..end];
        self.p = ws(self.s, end);
        if self.s.get(self.p) == Some(&b',') {
            self.p = ws(self.s, self.p + 1)
        } else if self.s.get(self.p) != Some(&b']') {
            return Err("invalid array separator");
        }
        Ok(Some(v))
    }
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
fn field_num(v: &[u8], k: &[u8]) -> Result<usize, E> {
    num(required(v, k)?)
}
fn wn(b: &mut [u8], n: &mut usize, x: usize) -> Result<(), E> {
    if write_usize(b, n, x) {
        Ok(())
    } else {
        Err("payload overflow")
    }
}

const WEB: &[u8] = b"https://h-comic.com";
const IMAGE: &[u8] = b"https://h-comic.link/api/";
const UA: &[u8] =
    b"Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:121.0) Gecko/20100101 Firefox/121.0";
static mut INDEX: [(usize, usize); 16384] = [(0, 0); 16384];
static mut INDEX_COUNT: usize = 0;
static mut DATA_PTR: *const u8 = core::ptr::null();
static mut DATA_LEN: usize = 0;
static mut IDS: [[u8; 32]; 1024] = [[0; 32]; 1024];
fn idx(n: usize) -> Result<&'static [u8], E> {
    unsafe {
        if n >= INDEX_COUNT {
            return Err("indexed reference out of bounds");
        };
        let (a, b) = INDEX[n];
        let data = core::slice::from_raw_parts(DATA_PTR, DATA_LEN);
        Ok(&data[a..b])
    }
}
fn refv(obj: &[u8], key: &[u8]) -> Result<&'static [u8], E> {
    idx(num(required(obj, key)?)?)
}
fn decoded<'a>(raw: &[u8], dst: &'a mut [u8]) -> Result<&'a [u8], E> {
    unescape(quoted(raw)?, dst)
}
fn atom(s: &[u8]) -> Result<(), E> {
    if s.is_empty()
        || s.len() > 32
        || !s
            .iter()
            .all(|c| c.is_ascii_alphanumeric() || *c == b'_' || *c == b'-')
    {
        Err("invalid ownership atom")
    } else {
        Ok(())
    }
}
fn decimal(s: &[u8]) -> Result<(), E> {
    atom(s)?;
    if number(s).filter(|n| *n > 0).is_none() {
        return Err("invalid numeric identity");
    };
    Ok(())
}
fn prepare(root: &[u8]) -> Result<(), E> {
    let mut nodes = Array::new(required(root, b"nodes")?)?;
    nodes.next()?.ok_or("missing first node")?;
    let node = nodes.next()?.ok_or("missing data node")?;
    let data = required(node, b"data")?;
    let mut a = Array::new(data)?;
    let mut n = 0;
    unsafe {
        while let Some(v) = a.next()? {
            if n >= 16384 {
                return Err("indexed data capacity");
            };
            let off = v.as_ptr() as usize - data.as_ptr() as usize;
            INDEX[n] = (off, off + v.len());
            n += 1
        }
        DATA_PTR = data.as_ptr();
        DATA_LEN = data.len();
        INDEX_COUNT = n;
    }
    if n < 2 {
        return Err("missing indexed structs");
    };
    Ok(())
}
fn make_url(parts: &[&[u8]]) -> Result<&'static [u8], E> {
    unsafe {
        let b = &mut *core::ptr::addr_of_mut!(URL);
        let mut n = 0;
        for p in parts {
            bytes(b, &mut n, p)?
        }
        Ok(core::slice::from_raw_parts(b.as_ptr(), n))
    }
}
fn fetch(url: &[u8]) -> Result<&'static [u8], E> {
    if host::check_cancel() {
        return Err("cancelled");
    };
    if !url.starts_with(b"https://h-comic.com/") {
        return Err("metadata host invalid");
    };
    let n = koma_source_sdk::build_get_request(
        http_req_buf(),
        url,
        Some(b"https://h-comic.com/"),
        &[
            (b"Origin", WEB),
            (b"User-Agent", UA),
            (b"Accept", b"application/json"),
            (b"Cache-Control", b"max-age=600"),
        ],
    )
    .ok_or("request capacity")?;
    let len = host::http_request(&http_req_buf()[..n], http_out())
        .map_err(|_| "HTTP transport failure")?;
    if len == 0 || len >= MAX {
        return Err("HTTP response capacity");
    };
    let env = &http_out()[..len];
    valid_json(env)?;
    if required(env, b"ok")? != b"true" {
        return Err("HTTP host failure");
    };
    let status = num(member(env, b"status")?
        .or(member(env, b"statusCode")?)
        .ok_or("HTTP status missing")?)?;
    if !(200..300).contains(&status) {
        return Err(match status {
            401 => "HTTP401",
            402 => "HTTP402",
            403 => "HTTP403",
            404 => "HTTP404",
            429 => "HTTP429",
            _ => "HTTP status failure",
        });
    };
    let body = unescape(text(env, b"bodyText")?, body_buf())?;
    if body.len() >= MAX {
        return Err("body capacity");
    };
    valid_json(body)?;
    let body = unsafe { core::slice::from_raw_parts(body.as_ptr(), body.len()) };
    prepare(body)?;
    Ok(body)
}
struct Work {
    id: [u8; 32],
    il: usize,
    media: [u8; 32],
    ml: usize,
    source: [u8; 32],
    sl: usize,
    title: [u8; 4096],
    tl: usize,
    count: usize,
    timestamp: usize,
    tags: usize,
}
fn work(v: &[u8]) -> Result<Work, E> {
    let mut w = Work {
        id: [0; 32],
        il: 0,
        media: [0; 32],
        ml: 0,
        source: [0; 32],
        sl: 0,
        title: [0; 4096],
        tl: 0,
        count: 0,
        timestamp: 0,
        tags: 0,
    };
    w.il = decoded(refv(v, b"id")?, &mut w.id)?.len();
    decimal(&w.id[..w.il])?;
    w.ml = decoded(refv(v, b"media_id")?, &mut w.media)?.len();
    decimal(&w.media[..w.ml])?;
    w.sl = decoded(refv(v, b"comic_source")?, &mut w.source)?.len();
    atom(&w.source[..w.sl])?;
    let title = refv(v, b"title")?;
    w.tl = decoded(refv(title, b"display")?, &mut w.title)?.len();
    if w.tl == 0 || w.title[..w.tl].iter().any(|c| *c < 32) {
        return Err("invalid title");
    };
    w.count = num(refv(v, b"num_pages")?)?;
    if w.count == 0 || w.count > 4096 {
        return Err("page count capacity");
    };
    w.timestamp = num(refv(v, b"upload_date")?)?;
    (w.timestamp as u64)
        .checked_mul(1000)
        .ok_or("timestamp overflow")?;
    w.tags = num(required(v, b"tags")?)?;
    let mut tags = Array::new(idx(w.tags)?)?;
    while let Some(t) = tags.next()? {
        let tag = idx(num(t)?)?;
        num(refv(tag, b"id")?)?;
        let mut tmp = [0; 2048];
        decoded(refv(tag, b"type")?, &mut tmp)?;
        decoded(refv(tag, b"name")?, &mut tmp)?;
        if member(tag, b"name_zh")?.is_some() {
            decoded(refv(tag, b"name_zh")?, &mut tmp)?;
        }
    }
    Ok(w)
}
fn hexemit(b: &mut [u8], n: &mut usize, s: &[u8]) -> Result<(), E> {
    for &c in s {
        bytes(
            b,
            n,
            &[
                b"0123456789abcdef"[(c >> 4) as usize],
                b"0123456789abcdef"[(c & 15) as usize],
            ],
        )?
    }
    Ok(())
}
fn identity(b: &mut [u8], n: &mut usize, prefix: &[u8], w: &Work) -> Result<(), E> {
    bytes(b, n, prefix)?;
    bytes(b, n, &w.id[..w.il])?;
    if prefix != b"hc:zh:" {
        bytes(b, n, b":")?;
        bytes(b, n, &w.source[..w.sl])?;
        bytes(b, n, b":")?;
        bytes(b, n, &w.media[..w.ml])?;
        bytes(b, n, b":")?;
        wn(b, n, w.count)?;
    }
    Ok(())
}
fn route(b: &mut [u8], n: &mut usize, w: &Work) -> Result<(), E> {
    bytes(b, n, WEB)?;
    bytes(b, n, b"/comics/")?;
    form_encode(b, n, &w.title[..w.tl])?;
    bytes(b, n, b"/1")
}
fn form_encode(b: &mut [u8], n: &mut usize, s: &[u8]) -> Result<(), E> {
    for &c in s {
        if c == b' ' {
            bytes(b, n, b"+")?
        } else if c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.' | b'*') {
            bytes(b, n, &[c])?
        } else {
            bytes(b, n, b"%")?;
            let h = b"0123456789ABCDEF";
            bytes(b, n, &[h[(c >> 4) as usize], h[(c & 15) as usize]])?
        }
    }
    Ok(())
}
fn imageurl(b: &mut [u8], n: &mut usize, w: &Work, page: Option<usize>) -> Result<(), E> {
    bytes(b, n, IMAGE)?;
    bytes(b, n, &w.source[..w.sl])?;
    bytes(b, n, b"/")?;
    bytes(b, n, &w.media[..w.ml])?;
    if let Some(p) = page {
        if p >= w.count {
            return Err("page index owner mismatch");
        };
        bytes(b, n, b"/pages/")?;
        wn(b, n, p.checked_add(1).ok_or("page overflow")?)?
    }
    Ok(())
}
fn tagtext(tag: &[u8], key: &[u8], dst: &mut [u8]) -> Result<usize, E> {
    Ok(decoded(refv(tag, key)?, dst)?.len())
}
fn emit_work(b: &mut [u8], n: &mut usize, w: &Work) -> Result<(), E> {
    bytes(b, n, b"{\"id\":\"")?;
    identity(b, n, b"hc:zh:", w)?;
    bytes(b, n, b"\",\"title\":")?;
    q(b, n, &w.title[..w.tl])?;
    bytes(b, n, b",\"cover\":{\"kind\":\"url\",\"url\":\"")?;
    imageurl(b, n, w, None)?;
    bytes(
        b,
        n,
        b"\"},\"status\":\"completed\",\"contentRating\":\"unknown\",\"authors\":[",
    )?;
    let mut a = Array::new(idx(w.tags)?)?;
    let mut written = 0;
    let mut tmp = [0; 2048];
    let mut kind = [0; 64];
    while let Some(t) = a.next()? {
        let tag = idx(num(t)?)?;
        let kl = tagtext(tag, b"type", &mut kind)?;
        if &kind[..kl] == b"artist" {
            let name = if member(tag, b"name_zh")?.is_some() {
                b"name_zh".as_slice()
            } else {
                b"name"
            };
            let l = tagtext(tag, name, &mut tmp)?;
            if written > 0 {
                bytes(b, n, b",")?
            };
            q(b, n, &tmp[..l])?;
            written += 1;
        }
    }
    bytes(b, n, b"],\"tags\":[")?;
    let mut a = Array::new(idx(w.tags)?)?;
    let mut written = 0;
    while let Some(t) = a.next()? {
        let tag = idx(num(t)?)?;
        let kl = tagtext(tag, b"type", &mut kind)?;
        if &kind[..kl] == b"tag" {
            let name = if member(tag, b"name_zh")?.is_some() {
                b"name_zh".as_slice()
            } else {
                b"name"
            };
            let l = tagtext(tag, name, &mut tmp)?;
            if written > 0 {
                bytes(b, n, b",")?
            };
            q(b, n, &tmp[..l])?;
            written += 1;
        }
    }
    bytes(b, n, b"],\"description\":\"")?;
    let mut a = Array::new(idx(w.tags)?)?;
    while let Some(t) = a.next()? {
        let tag = idx(num(t)?)?;
        let kl = tagtext(tag, b"type", &mut kind)?;
        if matches!(&kind[..kl], b"parody" | b"character" | b"group") {
            let name = if member(tag, b"name_zh")?.is_some() {
                b"name_zh".as_slice()
            } else {
                b"name"
            };
            let l = tagtext(tag, name, &mut tmp)?;
            if !append_json_escaped(b, n, &kind[..kl])
                || !append_json_escaped(b, n, b": ")
                || !append_json_escaped(b, n, &tmp[..l])
                || !append_json_escaped(b, n, b"\n")
            {
                return Err("payload overflow");
            };
        }
    }
    bytes(b, n, "页数：".as_bytes())?;
    wn(b, n, w.count)?;
    bytes(b, n, b"\",\"url\":\"")?;
    route(b, n, w)?;
    bytes(b, n, b"\",\"links\":[{\"kind\":\"source\",\"url\":\"")?;
    route(b, n, w)?;
    bytes(b, n, b"\"}]}")
}
fn parse_identity(s: &[u8], prefix: &[u8]) -> Result<Work, E> {
    let mut split = s
        .strip_prefix(prefix)
        .ok_or("work namespace")?
        .split(|c| *c == b':');
    let id = split.next().ok_or("work identity")?;
    decimal(id)?;
    let mut w = Work {
        id: [0; 32],
        il: id.len(),
        media: [0; 32],
        ml: 0,
        source: [0; 32],
        sl: 0,
        title: [0; 4096],
        tl: 0,
        count: 0,
        timestamp: 0,
        tags: 0,
    };
    w.id[..w.il].copy_from_slice(id);
    if prefix != b"hc:zh:" {
        let source = split.next().ok_or("source identity")?;
        let media = split.next().ok_or("media identity")?;
        let count = split.next().ok_or("count identity")?;
        atom(source)?;
        decimal(media)?;
        decimal(count)?;
        w.count = number(count).ok_or("count identity")?;
        if w.count > 4096 {
            return Err("page count capacity");
        }
        w.sl = source.len();
        w.ml = media.len();
        w.source[..w.sl].copy_from_slice(source);
        w.media[..w.ml].copy_from_slice(media);
    }
    if split.next().is_some() {
        return Err("identity shape");
    }
    Ok(w)
}
// Independently supplied route is data, never part of work identity. Only the
// exact canonical host and published one-segment form-encoded route are legal.
fn validate_route(u: &[u8]) -> Result<(), E> {
    let encoded = u
        .strip_prefix(b"https://h-comic.com/comics/")
        .and_then(|s| s.strip_suffix(b"/1"))
        .ok_or("manga route owner")?;
    if encoded.is_empty() {
        return Err("manga route empty");
    }
    let mut title = [0; 4096];
    let mut n = 0;
    let mut i = 0;
    while i < encoded.len() {
        let c = encoded[i];
        let v = if c == b'+' {
            i += 1;
            b' '
        } else if c == b'%' {
            let pair = encoded.get(i + 1..i + 3).ok_or("manga route escape")?;
            let h = [b'0', b'0', pair[0], pair[1]];
            i += 3;
            hex4(&h).ok_or("manga route escape")? as u8
        } else if c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.' | b'*') {
            i += 1;
            c
        } else {
            return Err("manga route syntax");
        };
        if n >= title.len() || v < 32 || v == 127 {
            return Err("manga route capacity/control");
        }
        title[n] = v;
        n += 1;
    }
    if &title[..n] == b"." || &title[..n] == b".." {
        return Err("manga route dot segment");
    }
    core::str::from_utf8(&title[..n]).map_err(|_| "manga route UTF8")?;
    let mut exact = [0; 32768];
    let mut en = 0;
    bytes(&mut exact, &mut en, b"https://h-comic.com/comics/")?;
    form_encode(&mut exact, &mut en, &title[..n])?;
    bytes(&mut exact, &mut en, b"/1")?;
    if &exact[..en] != u {
        return Err("manga route canonical encoding");
    }
    Ok(())
}
fn fresh(w: &Work, r: &[u8]) -> Result<Work, E> {
    let mut u = [0; 32768];
    let a = args(r)?;
    let raw = member(a, b"mangaUrl")?.ok_or("manga route missing")?;
    let route = decoded(raw, &mut u)?;
    validate_route(route)?;
    let url = make_url(&[route, b"/__data.json"])?;
    fetch(url)?;
    let found = work(idx(1)?)?;
    if found.id[..found.il] != w.id[..w.il] {
        return Err("detail work owner mismatch");
    }
    if w.sl > 0
        && (found.source[..found.sl] != w.source[..w.sl]
            || found.media[..found.ml] != w.media[..w.ml]
            || found.count != w.count)
    {
        return Err("detail media/count owner mismatch");
    }
    // If get_pages supplies a work identity, it must agree with the chapter.
    if let Some(raw) = member(a, b"mangaId")? {
        let mut id = [0; 128];
        let supplied = parse_identity(decoded(raw, &mut id)?, b"hc:zh:")?;
        if supplied.id[..supplied.il] != found.id[..found.il] {
            return Err("request work owner mismatch");
        }
    }
    Ok(found)
}
fn owned_request(r: &[u8], key: &[u8], prefix: &[u8]) -> Result<Work, E> {
    let mut tmp = [0; 16384];
    let id = decoded(required(args(r)?, key)?, &mut tmp)?;
    fresh(&parse_identity(id, prefix)?, r)
}
fn page_num(r: &[u8], search: bool, query: &[u8]) -> Result<usize, E> {
    let a = args(r)?;
    let page = member(a, b"page")?;
    let cursor = match page {
        Some(p) if p.first() == Some(&b'{') => member(p, b"cursor")?,
        _ => None,
    };
    let raw = match cursor {
        None | Some(b"null") => return Ok(1),
        Some(v) => v,
    };
    let mut tmp = [0; 16384];
    let c = decoded(raw, &mut tmp)?;
    let mut it = c.split(|c| *c == b':');
    if it.next()
        != Some(if search {
            b"search".as_slice()
        } else {
            b"latest"
        })
    {
        return Err("cursor operation");
    };
    let p = number(it.next().ok_or("cursor page")?)
        .filter(|p| *p >= 2 && *p <= 1000000)
        .ok_or("cursor page capacity")?;
    let hex = it.next().ok_or("cursor query")?;
    if it.next().is_some() || hex.len() != query.len() * 2 {
        return Err("cursor query mismatch");
    };
    for (i, p) in hex.chunks_exact(2).enumerate() {
        let mut h = [b'0', b'0', 0, 0];
        h[2..].copy_from_slice(p);
        if hex4(&h) != Some(query[i] as u32) {
            return Err("cursor query mismatch");
        };
    }
    Ok(p)
}
fn listing(r: &[u8], search: bool) -> Result<usize, E> {
    let a = args(r)?;
    let mut querybuf = [0; 4096];
    let query = if search {
        decoded(required(a, b"query")?, &mut querybuf)?
    } else {
        if text(a, b"listingId")? != b"listing:latest" {
            return Err("listing not supported");
        };
        b""
    };
    if search && (query.is_empty() || query.iter().any(|c| *c < 32)) {
        return Err("query invalid");
    };
    let page = page_num(r, search, query)?;
    let mut u = [0; 32768];
    let mut un = 0;
    bytes(&mut u, &mut un, WEB)?;
    if search {
        bytes(&mut u, &mut un, b"/__data.json?tag=&q=")?;
        form_encode(&mut u, &mut un, query)?;
        bytes(&mut u, &mut un, b"&page=")?
    } else {
        bytes(&mut u, &mut un, b"/__data.json?page=")?
    };
    wn(&mut u, &mut un, page)?;
    fetch(&u[..un])?;
    let root = idx(0)?;
    let comics = refv(root, b"comics")?;
    let total = match member(root, b"pages")? {
        Some(p) => num(refv(idx(num(p)?)?, b"pages")?)?,
        None => 1,
    };
    if total == 0 || total > 1000000 || page > total {
        return Err("pagination contradiction");
    };
    let mut a = Array::new(comics)?;
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"items\":[")?;
    let mut count = 0;
    while let Some(v) = a.next()? {
        if count >= 1024 {
            return Err("list capacity");
        };
        let w = work(idx(num(v)?)?)?;
        unsafe {
            for old in &IDS[..count] {
                if old[..w.il] == w.id[..w.il] && old[w.il..].iter().all(|c| *c == 0) {
                    return Err("duplicate work");
                };
            }
            IDS[count] = w.id;
        };
        if count > 0 {
            bytes(out, &mut n, b",")?
        };
        emit_work(out, &mut n, &w)?;
        count += 1;
    }
    if count == 0 && page < total {
        return Err("pagination no progress");
    };
    bytes(out, &mut n, b"],\"page\":{\"hasMore\":")?;
    bytes(out, &mut n, if page < total { b"true" } else { b"false" })?;
    bytes(out, &mut n, b",\"nextCursor\":")?;
    if page < total {
        bytes(out, &mut n, b"\"")?;
        bytes(out, &mut n, if search { b"search" } else { b"latest" })?;
        bytes(out, &mut n, b":")?;
        wn(out, &mut n, page.checked_add(1).ok_or("page overflow")?)?;
        bytes(out, &mut n, b":")?;
        hexemit(out, &mut n, query)?;
        bytes(out, &mut n, b"\"")?
    } else {
        bytes(out, &mut n, b"null")?
    };
    bytes(out, &mut n, b"}}")?;
    Ok(n)
}
fn detail(r: &[u8]) -> Result<usize, E> {
    let w = owned_request(r, b"mangaId", b"hc:zh:")?;
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"manga\":")?;
    emit_work(out, &mut n, &w)?;
    bytes(out, &mut n, b"}")?;
    Ok(n)
}
fn chapters(r: &[u8]) -> Result<usize, E> {
    let w = owned_request(r, b"mangaId", b"hc:zh:")?;
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"items\":[{\"id\":\"")?;
    identity(out, &mut n, b"hcch:zh:", &w)?;
    bytes(out, &mut n, b"\",\"mangaId\":\"")?;
    identity(out, &mut n, b"hc:zh:", &w)?;
    bytes(out, &mut n, b"\",\"title\":")?;
    q(out, &mut n, &w.title[..w.tl])?;
    bytes(
        out,
        &mut n,
        b",\"chapterNumber\":null,\"volumeNumber\":null,\"language\":\"zh\",\"dateUploaded\":",
    )?;
    w64(
        out,
        &mut n,
        (w.timestamp as u64)
            .checked_mul(1000)
            .ok_or("timestamp overflow")?,
    )?;
    bytes(
        out,
        &mut n,
        b"}],\"page\":{\"nextCursor\":null,\"hasMore\":false}}",
    )?;
    Ok(n)
}
fn pages(r: &[u8]) -> Result<usize, E> {
    let w = owned_request(r, b"chapterId", b"hcch:zh:")?;
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"pages\":[")?;
    for i in 0..w.count {
        if i > 0 {
            bytes(out, &mut n, b",")?
        };
        bytes(out, &mut n, b"{\"id\":\"")?;
        identity(out, &mut n, b"hcpg:zh:", &w)?;
        bytes(out, &mut n, b":")?;
        wn(out, &mut n, i)?;
        bytes(out, &mut n, b"\",\"index\":")?;
        wn(out, &mut n, i)?;
        bytes(out, &mut n, b",\"image\":{\"kind\":\"url\",\"url\":\"")?;
        imageurl(out, &mut n, &w, Some(i))?;
        bytes(out, &mut n, b"\"}}")?;
    }
    bytes(out, &mut n, b"]}")?;
    Ok(n)
}
fn image_request(r: &[u8]) -> Result<usize, E> {
    let a = args(r)?;
    let mut idbuf = [0; 16384];
    let id = decoded(required(a, b"pageId")?, &mut idbuf)?;
    let (w, p) = if id.starts_with(b"hc:zh:") {
        (fresh(&parse_identity(id, b"hc:zh:")?, r)?, None)
    } else {
        let pos = id.iter().rposition(|c| *c == b':').ok_or("page identity")?;
        let i = number(&id[pos + 1..]).ok_or("page index invalid")?;
        (
            fresh(&parse_identity(&id[..pos], b"hcpg:zh:")?, r)?,
            Some(i),
        )
    };
    let mut url = [0; 2048];
    let supplied = decoded(required(a, b"url")?, &mut url)?;
    let mut exact = [0; 2048];
    let mut en = 0;
    imageurl(&mut exact, &mut en, &w, p)?;
    if supplied != &exact[..en] {
        return Err("image URL owner mismatch");
    };
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"url\":")?;
    q(out, &mut n, supplied)?;
    bytes(out,&mut n,b",\"headers\":{\"Referer\":\"https://h-comic.com/\",\"Origin\":\"https://h-comic.com\",\"User-Agent\":")?;
    q(out, &mut n, UA)?;
    bytes(out, &mut n, b"}}")?;
    Ok(n)
}
fn execute(op: &str, result: Result<usize, E>) -> u32 {
    match result {
        Ok(n) => write_success_payload(op, n),
        Err(e) => write_error(op, "source_contract_error", e),
    }
}
fn run_search(r: &[u8]) -> u32 {
    execute("search", listing(r, true))
}
fn run_get_manga_list(r: &[u8]) -> u32 {
    execute("get_manga_list", listing(r, false))
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
    let data="{\"listings\":[{\"id\":\"listing:latest\",\"name\":\"最新\",\"kind\":\"latest\",\"supportsPaging\":true}]}".as_bytes();
    payload_buf()[..data.len()].copy_from_slice(data);
    write_success_payload("get_listings", data.len())
}
fn run_get_home(_: &[u8]) -> u32 {
    write_error("get_home", "not_supported", "not supported")
}
fn run_get_filters(_: &[u8]) -> u32 {
    write_error("get_filters", "not_supported", "not supported")
}
fn run_get_settings(_: &[u8]) -> u32 {
    write_error("get_settings", "not_supported", "not supported")
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
    let data=r#"{"sourceInfo":{"id":"com.hcomic.zh.koma","name":"H-Comic","version":"0.1.2","apiVersion":"0.2","language":"zh","author":"Koma","description":"H-Comic public Chinese metadata. NSFW source; per-book rating and usage unknown.","contentRating":"nsfw","localizedNames":{"zh-Hans":"H-Comic","zh-Hant":"H-Comic","en":"H-Comic"}},"capabilities":{"search":true,"mangaDetail":true,"chapters":true,"pages":true,"listings":true,"mangaList":true,"imageRequest":true,"home":false,"filters":false,"settings":false,"future":{"login":false,"auth":false,"process_page_image":false,"page_description":false,"base_url":false,"deeplink":false,"migration":false}}}"#.as_bytes();
    response_buffer().write_success_parts("source_info", &[data])
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

fn w64(b: &mut [u8], n: &mut usize, mut x: u64) -> Result<(), E> {
    let mut tmp = [0; 20];
    let mut i = 20;
    loop {
        i -= 1;
        tmp[i] = b'0' + (x % 10) as u8;
        x /= 10;
        if x == 0 {
            break;
        }
    }
    bytes(b, n, &tmp[i..])
}
