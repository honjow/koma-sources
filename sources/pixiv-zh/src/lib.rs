#![no_std]
// Independent source-only adapter of the published Pixiv All ages contract.
// Keiyoushi primary1319949ca9bc456908196e3ba2d104e94182c5d6, Apache-2.0.
use koma_source_sdk::{
    host,
    json_utils::*,
    source::{SourceCapabilities, SourceInfo},
};
const SOURCE_INFO:SourceInfo=SourceInfo{id:"com.pixiv.zh.koma",name:"pixiv",version:"0.1.15",api_version:"0.2",language:"zh",author:"Koma",description:"支持浏览和搜索漫画作品与系列。",content_rating:"nsfw"};
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: true,
    manga_detail: true,
    chapters: true,
    pages: true,
    image_request: true,
    listings: true,
    manga_list: true,
    home: false,
    filters: true,
    settings: true,
    credentials: false,
};
const MAX: usize = 1024 * 1024;
const COUNT: usize = 4096;
koma_source_sdk::koma_source_buffers! {payload:1048000,http_out:2*MAX,body:MAX,http_req:32768,scratch:8192,}
koma_source_sdk::koma_source_helpers!();
static mut REQUEST: [u8; MAX] = [0; MAX];
static mut SEARCH: [u8; MAX] = [0; MAX];
static mut CURSOR: [u8; 256 * 1024] = [0; 256 * 1024];
static mut CH_BYTES: [u8; 1048000] = [0; 1048000];
static mut CARD: [u8; 65536] = [0; 65536];
static mut SERIES_DETAIL: [u8; 32768] = [0; 32768];
static mut WORKS: [u64; COUNT] = [0; COUNT];
static mut SEEN_SERIES: [u64; COUNT] = [0; COUNT];
static mut CH_IDS: [u64; COUNT] = [0; COUNT];
static mut CH_PAGES: [usize; COUNT] = [0; COUNT];
static mut ORDERS: [usize; COUNT] = [0; COUNT];
const WEB: &[u8] = b"https://www.pixiv.net";
const UA: &[u8] =
    b"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0";
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

type E = &'static str;
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
fn id(v: &[u8]) -> Result<u64, E> {
    let s = scalar(v)?;
    if s.is_empty() || s.len() > 20 || s[0] == b'0' {
        return Err("invalid identity");
    };
    let mut n = 0u64;
    for &c in s {
        if !c.is_ascii_digit() {
            return Err("invalid identity");
        };
        n = n
            .checked_mul(10)
            .and_then(|v| v.checked_add((c - b'0') as u64))
            .ok_or("identity overflow")?;
    }
    Ok(n)
}
fn field_id(v: &[u8], k: &[u8]) -> Result<u64, E> {
    id(required(v, k)?)
}
fn wn(b: &mut [u8], n: &mut usize, x: usize) -> Result<(), E> {
    if write_usize(b, n, x) {
        Ok(())
    } else {
        Err("payload overflow")
    }
}
fn wi(b: &mut [u8], n: &mut usize, x: u64) -> Result<(), E> {
    let mut tmp = [0; 20];
    let mut i = 20;
    let mut v = x;
    loop {
        i -= 1;
        tmp[i] = b'0' + (v % 10) as u8;
        v /= 10;
        if v == 0 {
            break;
        }
    }
    bytes(b, n, &tmp[i..])
}
fn rawq(b: &mut [u8], n: &mut usize, s: &[u8]) -> Result<(), E> {
    quoted(s)?;
    bytes(b, n, s)
}
fn manga_key(b: &mut [u8], n: &mut usize, series: bool, x: u64) -> Result<(), E> {
    bytes(
        b,
        n,
        if series {
            b"\"pixiv:series:"
        } else {
            b"\"pixiv:artwork:"
        },
    )?;
    wi(b, n, x)?;
    bytes(b, n, b"\"")
}
fn namespace(r: &[u8], key: &[u8], chapter: bool) -> Result<(bool, u64, Option<u64>), E> {
    let s = text(args(r)?, key)?;
    let p = if chapter {
        b"pixivch:".as_slice()
    } else {
        b"pixiv:".as_slice()
    };
    let s = s.strip_prefix(p).ok_or("invalid identity")?;
    let mut a = s.split(|b| *b == b':');
    let kind = a.next().ok_or("invalid identity")?;
    let series = match kind {
        b"series" => true,
        b"artwork" => false,
        _ => return Err("unsupported identity"),
    };
    let first = id(a.next().ok_or("invalid identity")?)?;
    let second = if chapter && series {
        Some(id(a.next().ok_or("invalid identity")?)?)
    } else {
        None
    };
    if a.next().is_some() {
        return Err("invalid identity");
    };
    Ok((series, first, second))
}
fn make_url(path: &[u8], x: u64, suffix: &[u8]) -> Result<&'static [u8], E> {
    let b = scratch_a();
    let mut n = 0;
    bytes(b, &mut n, WEB)?;
    bytes(b, &mut n, path)?;
    wi(b, &mut n, x)?;
    bytes(b, &mut n, suffix)?;
    Ok(unsafe { core::slice::from_raw_parts(b.as_ptr(), n) })
}
fn fetch(url: &[u8]) -> Result<&'static [u8], E> {
    if host::check_cancel() {
        return Err("cancelled");
    };
    let b = http_req_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"version\":1,\"method\":\"GET\",\"url\":")?;
    q(b, &mut n, url)?;
    bytes(b, &mut n, b",\"headers\":{\"User-Agent\":")?;
    q(b, &mut n, UA)?;
    bytes(
        b,
        &mut n,
        b",\"Accept\":\"application/json\"},\"timeoutMs\":25000,\"responseKind\":\"bodyText\"}",
    )?;
    let request = unsafe { core::slice::from_raw_parts(b.as_ptr(), n) };
    let len = host::http_request(request, http_out()).map_err(|_| "network failure")?;
    if len == 0 || len > http_out().len() {
        return Err("response capacity exceeded");
    };
    let envelope = unsafe { core::slice::from_raw_parts(http_out().as_ptr(), len) };
    valid_json(envelope)?;
    let ok = required(envelope, b"ok")?;
    if ok != b"true" {
        return Err("network failure");
    };
    let status = num(member(envelope, b"status")?
        .or(member(envelope, b"statusCode")?)
        .ok_or("missing HTTP status")?)?;
    match status {
        401 | 402 | 403 => return Err("public access rejected"),
        429 => return Err("rate limit"),
        200..=299 => (),
        _ => return Err("HTTP error"),
    };
    let raw = text(envelope, b"bodyText")?;
    let decoded = unescape(raw, body_buf())?;
    if decoded.len() >= MAX {
        return Err("response capacity exceeded");
    };
    valid_json(decoded)?;
    if required(decoded, b"error")? != b"false" {
        return Err("upstream API error");
    };
    let data = required(decoded, b"body")?;
    Ok(unsafe { core::slice::from_raw_parts(data.as_ptr(), data.len()) })
}
fn guard(v: &[u8]) -> Result<(), E> {
    if field_num(v, b"x_restrict")? != 0
        || field_num(v, b"restrict")? != 0
        || field_num(v, b"type")? != 1
    {
        return Err("outside All ages manga scope");
    };
    for k in [b"is_private".as_slice(), b"is_mypixiv".as_slice()] {
        if required(v, k)? != b"false" {
            return Err("private artwork");
        }
    }
    if let Some(v) = member(v, b"visible")? {
        if v != b"true" {
            return Err("invisible artwork");
        }
    }
    if let Some(v) = member(v, b"is_login_only")? {
        if v != b"false" {
            return Err("login required");
        }
    }
    Ok(())
}
fn image_url(s: &[u8]) -> Result<(), E> {
    if !s.starts_with(b"https://i.pximg.net/")
        || s.len() > 2048
        || s.iter()
            .any(|b| *b < 33 || *b > 126 || matches!(b, b'\\' | b'"' | b'#'))
        || s.windows(3).any(|x| x == b"/..")
        || s.windows(3).any(|x| x.eq_ignore_ascii_case(b"%2e"))
    {
        return Err("invalid image URL");
    };
    Ok(())
}
fn qimage(b: &mut [u8], n: &mut usize, raw: &[u8]) -> Result<(), E> {
    let mut tmp = [0; 2048];
    let u = unescape(quoted(raw)?, &mut tmp)?;
    image_url(u)?;
    q(b, n, u)
}
fn author<'a>(v: &'a [u8]) -> Result<&'a [u8], E> {
    let a = required(v, b"author_details")?;
    if field_id(a, b"user_id")? != field_id(v, b"user_id")? {
        return Err("author identity mismatch");
    };
    raw_string(a, b"user_name")
}
fn artwork_card(b: &mut [u8], n: &mut usize, v: &[u8], x: u64) -> Result<(), E> {
    guard(v)?;
    if field_id(v, b"id")? != x {
        return Err("artwork identity mismatch");
    };
    bytes(b, n, b"{\"id\":")?;
    manga_key(b, n, false, x)?;
    bytes(b, n, b",\"title\":")?;
    rawq(b, n, raw_string(v, b"title")?)?;
    bytes(b, n, b",\"authors\":[")?;
    rawq(b, n, author(v)?)?;
    bytes(
        b,
        n,
        b"],\"status\":\"unknown\",\"contentRating\":\"safe\",\"cover\":{\"kind\":\"url\",\"url\":",
    )?;
    qimage(b, n, required(v, b"url")?)?;
    bytes(b, n, b"}")?;
    if let Some(comment) = member(v, b"comment")? {
        if comment != b"null" {
            bytes(b, n, b",\"description\":")?;
            rawq(b, n, comment)?;
        }
    }
    bytes(b, n, b",\"tags\":")?;
    let tags = required(v, b"tags")?;
    let mut a = Array::new(tags)?;
    while let Some(v) = a.next()? {
        quoted(v)?;
    }
    bytes(b, n, tags)?;
    bytes(
        b,
        n,
        b",\"links\":[{\"kind\":\"source\",\"url\":\"https://www.pixiv.net/artworks/",
    )?;
    wi(b, n, x)?;
    bytes(b, n, b"\"}]}")
}
fn emit_chapter(b: &mut [u8], n: &mut usize, s: Option<u64>, v: &[u8], reading_order: bool) -> Result<(), E> {
    let x = field_id(v, b"id")?;
    bytes(b, n, b"{\"id\":\"pixivch:")?;
    if let Some(s) = s {
        bytes(b, n, b"series:")?;
        wi(b, n, s)?;
        bytes(b, n, b":")?
    } else {
        bytes(b, n, b"artwork:")?
    }
    wi(b, n, x)?;
    bytes(b, n, b"\",\"mangaId\":")?;
    manga_key(b, n, s.is_some(), s.unwrap_or(x))?;
    bytes(b, n, b",\"title\":")?;
    rawq(b, n, raw_string(v, b"title")?)?;
    bytes(b, n, b",\"chapterNumber\":null,\"volumeNumber\":null,\"language\":null")?;
    if reading_order && s.is_some() {
        // The API's validated content_order belongs to the series, independent
        // of listing/display order and artwork title.
        let order = field_num(required(v, b"series")?, b"content_order")?;
        bytes(b, n, br#", "readingGroup":"pixiv-series", "continuationEligible":true, "readingOrder":"#)?;
        wn(b, n, order)?;
    }
    bytes(b, n, b"}")
}
fn art_detail(x: u64) -> Result<&'static [u8], E> {
    let d = fetch(make_url(
        b"/touch/ajax/illust/details?illust_id=",
        x,
        b"&lang=zh",
    )?)?;
    let v = required(d, b"illust_details")?;
    guard(v)?;
    if field_id(v, b"id")? != x {
        return Err("artwork identity mismatch");
    };
    Ok(v)
}
// The complete public catalogue is validated before emitting a successful series card/chapter list.
fn collect(s: u64, reading_order: bool) -> Result<(usize, usize, usize), E> {
    let d = fetch(make_url(b"/touch/ajax/illust/series/", s, b"?lang=zh")?)?;
    let d = required(d, b"series")?;
    if field_id(d, b"id")? != s {
        return Err("series identity mismatch");
    };
    let owner = field_id(d, b"userId")?;
    let total = field_num(d, b"total")?;
    // The top-level display count describes this public catalogue; total/workCount
    // may include unavailable orders. Completeness needs this count and an empty
    // content page, never max(content_order) or an inferred hidden-work identity.
    let visible_total = field_num(d, b"display_series_content_count")?;
    if total == 0
        || total > COUNT
        || visible_total == 0
        || visible_total > total
        || field_num(d, b"workCount")? != total
        || d.len() > 32768
    {
        return Err("series count capacity mismatch");
    };
    unsafe { SERIES_DETAIL[..d.len()].copy_from_slice(d) };
    let dlen = d.len();
    let mut count = 0;
    let mut cn = 0;
    let mut cardlen = 0;
    loop {
        if host::check_cancel() {
            return Err("cancelled");
        };
        let b = scratch_b();
        let mut n = 0;
        bytes(b, &mut n, b"?lang=zh&last_order=")?;
        // This endpoint pages by cumulative entry offset, not content_order;
        // real reading orders can have gaps and must never become a cursor.
        wn(b, &mut n, count)?;
        let suf = unsafe { core::slice::from_raw_parts(b.as_ptr(), n) };
        let data = fetch(make_url(b"/touch/ajax/illust/series_content/", s, suf)?)?;
        let mut a = Array::new(required(data, b"series_contents")?)?;
        let before = count;
        while let Some(v) = a.next()? {
            if count >= visible_total || count >= COUNT {
                return Err("series count exceeded");
            };
            guard(v)?;
            if required(v, b"visible")? != b"true" {
                return Err("invisible artwork");
            };
            let x = field_id(v, b"id")?;
            let vs = required(v, b"series")?;
            if field_id(vs, b"id")? != s
                || field_id(vs, b"user_id")? != owner
                || field_id(vs, b"userId")? != owner
                || field_id(v, b"user_id")? != owner
            {
                return Err("series ownership mismatch");
            };
            // Real public responses also embed the full total here. Validate
            // either observed count scope without using it to end pagination.
            let embedded_visible = field_num(vs, b"display_series_content_count")?;
            if field_num(vs, b"total")? != total
                || field_num(vs, b"workCount")? != total
                || (embedded_visible != total && embedded_visible != visible_total)
            {
                return Err("series count drift");
            };
            let order = field_num(vs, b"content_order")?;
            if order == 0 || order > total {
                return Err("series order outside catalogue");
            };
            unsafe {
                if CH_IDS[..count].contains(&x) || ORDERS[..count].contains(&order) {
                    return Err("duplicate series member");
                };
                CH_IDS[count] = x;
                ORDERS[count] = order;
                CH_PAGES[count] = field_num(v, b"page_count")?;
                if CH_PAGES[count] == 0 {
                    return Err("empty artwork");
                }
            };
            if count == 0 {
                let cd = unsafe { &SERIES_DETAIL[..dlen] };
                let cb = unsafe { &mut CARD };
                let mut z = 0;
                bytes(cb, &mut z, b"{\"id\":")?;
                manga_key(cb, &mut z, true, s)?;
                bytes(cb, &mut z, b",\"title\":")?;
                rawq(cb, &mut z, raw_string(cd, b"title")?)?;
                bytes(cb, &mut z, b",\"description\":")?;
                rawq(cb, &mut z, raw_string(cd, b"caption")?)?;
                bytes(cb, &mut z, b",\"authors\":[")?;
                rawq(cb, &mut z, author(v)?)?;
                bytes(cb,&mut z,b"],\"status\":\"unknown\",\"contentRating\":\"safe\",\"cover\":{\"kind\":\"url\",\"url\":")?;
                let cover = member(cd, b"coverImage")?.unwrap_or(b"null");
                qimage(
                    cb,
                    &mut z,
                    if cover == b"null" || cover == b"false" {
                        required(v, b"url")?
                    } else {
                        cover
                    },
                )?;
                bytes(cb,&mut z,b"},\"tags\":[],\"links\":[{\"kind\":\"source\",\"url\":\"https://www.pixiv.net/user/")?;
                wi(cb, &mut z, owner)?;
                bytes(cb, &mut z, b"/series/")?;
                wi(cb, &mut z, s)?;
                bytes(cb, &mut z, b"\"}]}")?;
                cardlen = z
            };
            unsafe {
                if count > 0 {
                    bytes(&mut CH_BYTES, &mut cn, b",")?
                };
                emit_chapter(&mut CH_BYTES, &mut cn, Some(s), v, reading_order)?;
            }
            count += 1;
        }
        if before == count {
            if count != visible_total {
                return Err("incomplete series catalogue");
            };
            break;
        }
        if count > visible_total {
            return Err("series count exceeded");
        }
    }
    Ok((count, cn, cardlen))
}
fn cursor_arrays(v: &[u8], k: &[u8], series: bool) -> Result<usize, E> {
    let mut a = Array::new(required(v, k)?)?;
    let mut count = 0;
    while let Some(v) = a.next()? {
        if count >= COUNT {
            return Err("cursor capacity exceeded");
        };
        let x = id(v)?;
        unsafe {
            let b = if series { &mut SEEN_SERIES } else { &mut WORKS };
            if b[..count].contains(&x) {
                return Err("duplicate cursor identity");
            };
            b[count] = x
        };
        count += 1;
    }
    Ok(count)
}
// A search series card describes declared search metadata, not a complete catalogue.
fn search_series_owner(v: &[u8], series: &[u8]) -> Result<u64, E> {
    let owner = field_id(v, b"user_id")?;
    for key in [b"userId".as_slice(), b"user_id".as_slice()] {
        if let Some(declared) = member(series, key)? {
            if declared != b"null" && id(declared)? != owner {
                return Err("search series ownership mismatch");
            }
        }
    }
    Ok(owner)
}
fn search_series_card(b: &mut [u8], n: &mut usize, v: &[u8], series: &[u8], sid: u64) -> Result<(), E> {
    let owner = search_series_owner(v, series)?;
    bytes(b, n, b"{\"id\":")?;
    manga_key(b, n, true, sid)?;
    bytes(b, n, b",\"title\":")?;
    rawq(b, n, raw_string(series, b"title")?)?;
    bytes(b, n, b",\"authors\":[")?;
    rawq(b, n, author(v)?)?;
    bytes(b, n, b"],\"status\":\"unknown\",\"contentRating\":\"unknown\",\"cover\":{\"kind\":\"url\",\"url\":")?;
    let cover = member(series, b"coverImage")?;
    let mut decoded = [0u8; 2048];
    let mut cover_len = 0;
    if let Some(raw) = cover {
        if let Ok(raw) = quoted(raw) {
            if let Ok(url) = unescape(raw, &mut decoded) {
                if image_url(url).is_ok() { cover_len = url.len(); }
            }
        }
    }
    if cover_len > 0 {
        q(b, n, &decoded[..cover_len])?;
    } else {
        qimage(b, n, required(v, b"url")?)?;
    }
    bytes(b, n, b"}")?;
    if let Some(caption) = member(series, b"caption")? {
        if caption != b"null" {
            bytes(b, n, b",\"description\":")?;
            rawq(b, n, caption)?;
        }
    }
    bytes(b, n, b",\"tags\":[],\"links\":[{\"kind\":\"source\",\"url\":\"https://www.pixiv.net/user/")?;
    wi(b, n, owner)?;
    bytes(b, n, b"/series/")?;
    wi(b, n, sid)?;
    bytes(b, n, b"\"}]}")
}
// Date filters are available only to hosts that preserve search scope.
fn filter_host(r: &[u8]) -> bool {
    member(r,b"hostFeatures").ok().flatten().and_then(|v| Array::new(v).ok()).map_or(false,|mut a| {
        while let Ok(Some(v))=a.next() { if v==br#""source_filter_scope_header_v1""# {return true;} } false
    })
}
fn extended_search_supported(r: &[u8]) -> bool {
    filter_host(r) && member(r,b"hostFeatures").ok().flatten().and_then(|v|Array::new(v).ok()).map_or(false,|mut a|{while let Ok(Some(v))=a.next(){if v==br#""source_filter_search_without_query_v1""# {return true}}false})
}
fn tag_match(v: &[u8], filter: &[u8], full: bool) -> Result<bool,E> {
    let wanted=core::str::from_utf8(filter).map_err(|_|"invalid tag filter")?;
    if wanted.trim().is_empty(){return Ok(true)}
    let mut flags=[false;128];let mut count=0;
    for _ in wanted.split(' ') {if count>=flags.len(){return Err("too many tag terms")} count+=1;}
    let tags=match member(v,b"tags")? {None|Some(b"null")=>return Ok(false),Some(t)=>t};
    let mut rows=Array::new(tags)?;
    while let Some(tag)=rows.next()? {let mut b=[0u8;8192];let value=unescape(quoted(tag)?,&mut b)?;let text=core::str::from_utf8(value).map_err(|_|"invalid tag")?;for (i,term) in wanted.split(' ').enumerate(){if if full {text==term} else {text.contains(term)} {flags[i]=true}}}
    Ok(if full {flags[..count].iter().all(|v|*v)} else {flags[..count].iter().any(|v|*v)})
}
fn filter_text<'a>(f: &[u8], key: &[u8], out: &'a mut [u8]) -> Result<&'a [u8], E> {
    match member(f,key)? { None | Some(b"null") => Ok(b""), Some(v) => unescape(quoted(v)?,out) }
}
// Reference takes 36 samples, estimates a bounded window, then filters it.
// Cursor carries the remainder of the last fetched public page so refreshes
// cannot silently change the boundary between two result pages.
fn adaptive_search(r:&[u8],query:&[u8],tags:&[u8],mode:&[u8],order:&[u8],before:&[u8],after:&[u8])->Result<usize,E>{
    let a=args(r)?;let mut page=1usize;let mut offset=0usize;let mut wc=0usize;let mut sc=0usize;let mut pending_len=0usize;
    if let Some(pg)=member(a,b"page")? { if pg.first()!=Some(&b'{'){return Err("invalid page contract")}
        if let Some(c)=member(pg,b"cursor")? { if c!=b"null" {
            let cv=unsafe{unescape(quoted(c)?,&mut CURSOR)?};valid_json(cv)?;
            if member(cv,b"adaptive")?!=Some(b"true".as_slice()){return Err("invalid adaptive cursor")}
            let mut qbuf=[0u8;8192];if unescape(text(cv,b"query")?,&mut qbuf)?!=query{return Err("cursor query mismatch")}
            let mut t=[0u8;4096];let mut m=[0u8;32];let mut o=[0u8;32];let mut be=[0u8;1024];let mut af=[0u8;1024];
            if filter_text(cv,b"tags",&mut t)?!=tags || filter_text(cv,b"tagsMode",&mut m)?!=mode || filter_text(cv,b"order",&mut o)?!=order || filter_text(cv,b"dateBefore",&mut be)?!=before || filter_text(cv,b"dateAfter",&mut af)?!=after {return Err("cursor filter mismatch")}
            page=field_num(cv,b"p")?;offset=field_num(cv,b"offset")?;wc=cursor_arrays(cv,b"works",false)?;sc=cursor_arrays(cv,b"series",true)?;
            if page>1024 || offset>4096 || sc>wc || (page==0 && offset!=0){return Err("invalid adaptive cursor")}
            if let Some(p)=member(cv,b"pending")? {let data=unsafe{unescape(quoted(p)?,&mut SEARCH)?};valid_json(data)?;Array::new(required(data,b"illusts")?)?;pending_len=data.len()}
            if (offset!=0)!=(pending_len!=0){return Err("invalid adaptive snapshot")}
        }}
    }
    let out=payload_buf();let mut n=0;let mut emitted=0usize;let mut sampled=0usize;let mut hits=0usize;let mut goal=36usize;let mut retained=0usize;let mut fetched=0usize;
    bytes(out,&mut n,b"{\"items\":[")?;
    while page!=0 && sampled<goal {
        fetched+=1;if fetched>128{return Err("adaptive request capacity exceeded")}
        let len=if pending_len!=0 {let n=pending_len;pending_len=0;n} else {
            let mut u=[0u8;16384];let mut z=0;bytes(&mut u,&mut z,b"https://www.pixiv.net/touch/ajax/search/illusts?lang=zh&word=")?;
            if !write_url_encoded(&mut u,&mut z,query){return Err("query URL overflow")}
            bytes(&mut u,&mut z,b"&s_mode=s_tc&type=manga&mode=all&p=")?;wn(&mut u,&mut z,page)?;
            if order==b"date"{bytes(&mut u,&mut z,b"&order=date")?}
            for (k,v) in [(b"&ecd=".as_slice(),before),(b"&scd=".as_slice(),after)] {if !v.is_empty(){bytes(&mut u,&mut z,k)?;if !write_url_encoded(&mut u,&mut z,v){return Err("filter URL overflow")}}}
            let body=fetch(&u[..z])?;if body.len()>=MAX{return Err("search response capacity exceeded")};let n=body.len();unsafe{SEARCH[..n].copy_from_slice(body)};n
        };
        let data=unsafe{&SEARCH[..len]};let last=field_num(data,b"lastPage")?;if last>1024{return Err("search page capacity exceeded")}
        let mut rows=Array::new(required(data,b"illusts")?)?;
        if last==0 { if rows.next()?.is_some(){return Err("search page outside server window")};page=0;offset=0;break }
        if page>last{return Err("search page outside server window")}let mut index=0usize;let mut stopped=false;
        while let Some(v)=rows.next()? {
            if index<offset {index+=1;continue} if sampled>=goal {stopped=true;break} index+=1;
            if member(v,b"is_ad_container")?==Some(b"1".as_slice()){continue}
            match guard(v){Ok(())=>(),Err("outside All ages manga scope"|"private artwork"|"invisible artwork"|"login required")=>continue,Err(e)=>return Err(e)}
            sampled+=1;let matched=tag_match(v,tags,mode==b"full")?;if sampled<=36 && matched{hits+=1}
            if sampled==36 {goal=if hits==0{1000}else{(50usize*36/hits).clamp(36,1000)}}
            let x=field_id(v,b"id")?;if unsafe{WORKS[..wc].contains(&x)}{continue}if wc>=COUNT{return Err("adaptive cursor capacity exceeded")};unsafe{WORKS[wc]=x};wc+=1;
            if !matched{continue} let series=member(v,b"series")?.unwrap_or(b"null");
            let sid=if series!=b"null" {let id=field_id(series,b"id")?;search_series_owner(v,series)?;if unsafe{SEEN_SERIES[..sc].contains(&id)}{continue}if sc>=COUNT{return Err("adaptive cursor capacity exceeded")};unsafe{SEEN_SERIES[sc]=id};sc+=1;Some(id)} else {None};
            if emitted!=0{bytes(out,&mut n,b",")?}if let Some(sid)=sid{search_series_card(out,&mut n,v,series,sid)?}else{artwork_card(out,&mut n,v,x)?}emitted+=1;
        }
        if offset>index{return Err("adaptive cursor offset")}
        if stopped {offset=index;retained=len;break}
        if index==0 || page>=last {page=0;offset=0;break}
        page=page.checked_add(1).ok_or("search page overflow")?;offset=0;
    }
    bytes(out,&mut n,b"],\"page\":{\"hasMore\":")?;bytes(out,&mut n,if emitted>0{b"true"}else{b"false"})?;bytes(out,&mut n,b",\"nextCursor\":")?;
    if emitted==0{bytes(out,&mut n,b"null")?}else{
        let c=unsafe{&mut CURSOR};let mut cn=0;bytes(c,&mut cn,b"{\"adaptive\":true,\"p\":")?;wn(c,&mut cn,page)?;bytes(c,&mut cn,b",\"offset\":")?;wn(c,&mut cn,offset)?;
        for (k,v) in [(b",\"query\":".as_slice(),query),(b",\"tags\":",tags),(b",\"tagsMode\":",mode),(b",\"order\":",order),(b",\"dateBefore\":",before),(b",\"dateAfter\":",after)]{bytes(c,&mut cn,k)?;q(c,&mut cn,v)?}
        bytes(c,&mut cn,b",\"works\":[")?;for i in 0..wc{if i!=0{bytes(c,&mut cn,b",")?}unsafe{wi(c,&mut cn,WORKS[i])?}}
        bytes(c,&mut cn,b"],\"series\":[")?;for i in 0..sc{if i!=0{bytes(c,&mut cn,b",")?}unsafe{wi(c,&mut cn,SEEN_SERIES[i])?}}bytes(c,&mut cn,b"]")?;
        if retained!=0{bytes(c,&mut cn,b",\"pending\":")?;q(c,&mut cn,unsafe{&SEARCH[..retained]})?}bytes(c,&mut cn,b"}")?;q(out,&mut n,&c[..cn])?
    }
    bytes(out,&mut n,b"}}")?;Ok(n)
}
fn search(r: &[u8]) -> Result<usize, E> {
    let a = args(r)?;
    let mut qb = [0; 8192];
    let query = unescape(text(a, b"query")?, &mut qb)?;
    let extended=extended_search_supported(r);
    if query.is_empty() && !extended { return Err("empty query"); }
    let filters = if filter_host(r) { member(a,b"filters")?.unwrap_or(b"{}") } else { b"{}" };
    let mut before_buf=[0u8;1024]; let before=filter_text(filters,b"dateBefore",&mut before_buf)?;
    let mut after_buf=[0u8;1024]; let after=filter_text(filters,b"dateAfter",&mut after_buf)?;
    let mut tags_buf=[0u8;4096];let tags=if extended {filter_text(filters,b"tags",&mut tags_buf)?}else{b""};
    let mut mode_buf=[0u8;32];let mode=if extended {filter_text(filters,b"tagsMode",&mut mode_buf)?}else{b""};
    let mut order_buf=[0u8;32];let order=if extended {filter_text(filters,b"order",&mut order_buf)?}else{b""};
    if !matches!(mode,b""|b"partial"|b"full") || !matches!(order,b""|b"default"|b"date") {return Err("invalid search filter")}
    let blank=extended && core::str::from_utf8(query).map_err(|_|"invalid query")?.trim().is_empty();
    let tags_blank=core::str::from_utf8(tags).map_err(|_|"invalid tags")?.trim().is_empty();
    let word=if blank {if tags_blank {"漫画".as_bytes()} else {tags}} else {query};
    if extended && !blank && !tags_blank { return adaptive_search(r,query,tags,mode,order,before,after); }
    let mut page = 1;
    let mut raw = 0;
    let mut sc = 0;
    let pg = member(a, b"page")?;
    if let Some(pg) = pg {
        if pg.first() != Some(&b'{') {
            return Err("invalid page contract");
        };
        if let Some(c) = member(pg, b"cursor")? {
            if c != b"null" {
                let cv = unsafe { unescape(quoted(c)?, &mut CURSOR)? };
                valid_json(cv)?;
                let mut prev = [0; 8192];
                if unescape(text(cv, b"query")?, &mut prev)? != query {
                    return Err("cursor query mismatch");
                };
                let mut prev_before=[0u8;1024]; let mut prev_after=[0u8;1024];
                if filter_text(cv,b"dateBefore",&mut prev_before)? != before || filter_text(cv,b"dateAfter",&mut prev_after)? != after { return Err("cursor filter mismatch"); }
                let mut t=[0u8;4096];let mut m=[0u8;32];let mut o=[0u8;32];
                if filter_text(cv,b"tags",&mut t)?!=tags || filter_text(cv,b"tagsMode",&mut m)?!=mode || filter_text(cv,b"order",&mut o)?!=order {return Err("cursor filter mismatch")}
                page = field_num(cv, b"p")?;
                raw = field_num(cv, b"raw")?;
                let wc = cursor_arrays(cv, b"works", false)?;
                sc = cursor_arrays(cv, b"series", true)?;
                if page > 1024 {
                    return Err("search page capacity exceeded");
                };
                if page < 2
                    || raw != wc
                    || raw == 0
                    || sc > raw
                {
                    return Err("invalid cursor");
                };
            }
        }
    };
    let b = scratch_a();
    let mut z = 0;
    bytes(
        b,
        &mut z,
        b"https://www.pixiv.net/touch/ajax/search/illusts?lang=zh&word=",
    )?;
    if !write_url_encoded(b, &mut z, word) { return Err("query URL overflow"); }
    bytes(b,&mut z,if blank {if mode==b"full" {b"&s_mode=s_tag_full"} else {b"&s_mode=s_tag"}} else {b"&s_mode=s_tc"})?;
    bytes(b, &mut z, b"&type=manga&mode=all&p=")?;
    wn(b, &mut z, page)?;
    if order==b"date" { bytes(b,&mut z,b"&order=date")?; }
    for (key,value) in [(b"&ecd=".as_slice(),before),(b"&scd=".as_slice(),after)] {
        if !value.is_empty() { bytes(b,&mut z,key)?; if !write_url_encoded(b,&mut z,value) { return Err("filter URL overflow"); } }
    }
    let u = unsafe { core::slice::from_raw_parts(b.as_ptr(), z) };
    let data = fetch(u)?;
    if data.len() >= MAX {
        return Err("search response capacity exceeded");
    };
    unsafe { SEARCH[..data.len()].copy_from_slice(data) };
    let data = unsafe { &SEARCH[..data.len()] };
    let _total = field_num(data, b"total")?;
    let last = field_num(data, b"lastPage")?;
    if last > 1024 {
        return Err("search page capacity exceeded");
    };
    let mut list = Array::new(required(data, b"illusts")?)?;
    let out = payload_buf();
    let mut n = 0;
    let mut emitted = 0;
    let raw_before = raw;
    let mut page_count = 0;
    bytes(out, &mut n, b"{\"items\":[")?;
    while let Some(v) = list.next()? {
        if page > last {
            return Err("search page outside server window");
        };
        guard(v)?;
        let x = field_id(v, b"id")?;
        // Search no longer calls collect; reuse its bounded temporary IDs for this page.
        if page_count >= COUNT { return Err("search page capacity exceeded"); }
        unsafe {
            if CH_IDS[..page_count].contains(&x) { return Err("duplicate search artwork within page"); }
            CH_IDS[page_count] = x;
        }
        page_count += 1;
        if unsafe { WORKS[..raw_before].contains(&x) } {
            // Validate the complete matched card before de-duplicating a legal overlap.
            let series = member(v, b"series")?.unwrap_or(b"null");
            let mut checked_len = 0;
            if series != b"null" {
                let sid = field_id(series, b"id")?;
                let cap = payload_buf().len();
                search_series_card(&mut http_out()[..cap], &mut checked_len, v, series, sid)?;
            } else {
                let cap = payload_buf().len();
                artwork_card(&mut http_out()[..cap], &mut checked_len, v, x)?;
            }
            continue;
        }
        if raw >= COUNT {
            return Err("search cursor capacity exceeded");
        };
        unsafe {
            if WORKS[..raw].contains(&x) {
                return Err("duplicate search artwork");
            };
            WORKS[raw] = x
        };
        raw += 1;
        if extended && !blank && !tags_blank && !tag_match(v,tags,mode==b"full")? { continue; }
        let vs = member(v, b"series")?.unwrap_or(b"null");
        let is_series = vs != b"null";
        let mut series_id = 0;
        if is_series {
            series_id = field_id(vs, b"id")?;
            search_series_owner(v, vs)?;
            if unsafe { SEEN_SERIES[..sc].contains(&series_id) } {
                continue;
            };
            if sc >= COUNT {
                return Err("series cursor capacity exceeded");
            };
            unsafe { SEEN_SERIES[sc] = series_id };
            sc += 1;
        }
        if emitted > 0 {
            bytes(out, &mut n, b",")?
        };
        if is_series {
            search_series_card(out, &mut n, v, vs, series_id)?;
        } else {
            artwork_card(out, &mut n, v, x)?
        };
        emitted += 1;
    }
    if page_count > 0 && raw == raw_before {
        return Err("search pagination no progress");
    };
    let more = raw > raw_before && page < last;
    bytes(out, &mut n, b"],\"page\":{\"hasMore\":")?;
    bytes(out, &mut n, if more { b"true" } else { b"false" })?;
    bytes(out, &mut n, b",\"nextCursor\":")?;
    if more {
        let c = unsafe { &mut CURSOR };
        let mut cn = 0;
        bytes(c, &mut cn, b"{\"p\":")?;
        wn(c, &mut cn, page.checked_add(1).ok_or("search page overflow")?)?;
        bytes(c, &mut cn, b",\"raw\":")?;
        wn(c, &mut cn, raw)?;
        bytes(c, &mut cn, b",\"query\":")?;
        q(c, &mut cn, query)?;
        if !before.is_empty() || !after.is_empty() { bytes(c,&mut cn,b",\"dateBefore\":")?; q(c,&mut cn,before)?; bytes(c,&mut cn,b",\"dateAfter\":")?; q(c,&mut cn,after)?; }
        if !tags.is_empty() || !mode.is_empty() || !order.is_empty() { bytes(c,&mut cn,b",\"tags\":")?;q(c,&mut cn,tags)?;bytes(c,&mut cn,b",\"tagsMode\":")?;q(c,&mut cn,mode)?;bytes(c,&mut cn,b",\"order\":")?;q(c,&mut cn,order)?; }
        bytes(c, &mut cn, b",\"works\":[")?;
        for i in 0..raw {
            if i > 0 {
                bytes(c, &mut cn, b",")?
            };
            unsafe { wi(c, &mut cn, WORKS[i])? }
        }
        bytes(c, &mut cn, b"],\"series\":[")?;
        for i in 0..sc {
            if i > 0 {
                bytes(c, &mut cn, b",")?
            };
            unsafe { wi(c, &mut cn, SEEN_SERIES[i])? }
        }
        bytes(c, &mut cn, b"]}")?;
        q(out, &mut n, &c[..cn])?;
    } else {
        bytes(out, &mut n, b"null")?
    };
    bytes(out, &mut n, b"}}")?;
    Ok(n)
}
fn detail(r: &[u8]) -> Result<usize, E> {
    let (series, x, _) = namespace(r, b"mangaId", false)?;
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"manga\":")?;
    if series {
        let (_, _, len) = collect(x, false)?;
        unsafe { bytes(b, &mut n, &CARD[..len])? }
    } else {
        artwork_card(b, &mut n, art_detail(x)?, x)?
    }
    bytes(b, &mut n, b"}")?;
    Ok(n)
}
fn chapters(r: &[u8]) -> Result<usize, E> {
    let (series, x, _) = namespace(r, b"mangaId", false)?;
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"items\":[")?;
    if series {
        let reading_order = member(r, b"hostFeatures")?.and_then(|raw| Array::new(raw).ok())
            .map_or(false, |mut values| {
                while let Ok(Some(value)) = values.next() {
                    if value == br#""chapter_reading_order_v1""# { return true; }
                }
                false
            });
        let (_, len, _) = collect(x, reading_order)?;
        unsafe { bytes(b, &mut n, &CH_BYTES[..len])? }
    } else {
        emit_chapter(b, &mut n, None, art_detail(x)?, false)?
    }
    bytes(b, &mut n, b"]}")?;
    Ok(n)
}
// Match Keiyoushi's quality order and upward fallback using only URLs from
// this same page response. Default original and original identity guards stay
// unchanged; a preference never opens login/private/adult or another host.
const IMAGE_QUALITIES: [&[u8]; 4] = [b"thumb_mini", b"small", b"regular", b"original"];
fn requested_image_quality(r: &[u8]) -> Result<usize, E> {
    let settings = member(r, b"settings")?.unwrap_or(b"{}");
    let quality = match member(settings, b"pref_image_quality")? {
        Some(value) => quoted(value)?, None => b"original",
    };
    Ok(IMAGE_QUALITIES.iter().position(|name| *name == quality).unwrap_or(3))
}
fn pages(r: &[u8]) -> Result<usize, E> {
    let (series, x, ch) = namespace(r, b"chapterId", true)?;
    let quality = requested_image_quality(r)?;
    let artwork = ch.unwrap_or(x);
    let expected = if series {
        let (count, _, _) = collect(x, false)?;
        let ix = unsafe { CH_IDS[..count].iter().position(|id| *id == artwork) }
            .ok_or("chapter ownership mismatch")?;
        unsafe { CH_PAGES[ix] }
    } else {
        field_num(art_detail(artwork)?, b"page_count")?
    };
    if expected == 0 || expected > COUNT {
        return Err("page capacity exceeded");
    };
    let data = fetch(make_url(b"/ajax/illust/", artwork, b"/pages?lang=zh")?)?;
    let mut a = Array::new(data)?;
    let b = payload_buf();
    let mut n = 0;
    let mut count = 0;
    let mut original_bytes = 0;
    bytes(b, &mut n, b"{\"pages\":[")?;
    while let Some(v) = a.next()? {
        if count >= expected {
            return Err("page count exceeded");
        };
        if count > 0 {
            bytes(b, &mut n, b",")?
        };
        bytes(b, &mut n, b"{\"id\":\"pixivpage:")?;
        if series {
            wi(b, &mut n, x)?;
            bytes(b, &mut n, b":")?
        };
        wi(b, &mut n, artwork)?;
        bytes(b, &mut n, b":")?;
        wn(b, &mut n, count)?;
        bytes(b, &mut n, b"\",\"index\":")?;
        wn(b, &mut n, count)?;
        bytes(b, &mut n, b",\"image\":{\"kind\":\"url\",\"url\":")?;
        let urls = required(v, b"urls")?;
        let mut original_buffer = [0; 2048];
        let original = unescape(
            text(urls, b"original")?,
            &mut original_buffer,
        )?;
        image_url(original)?;
        if !original.starts_with(b"https://i.pximg.net/img-original/") {
            return Err("not an original image URL");
        }
        unsafe {
            for previous in 0..count {
                if &CH_BYTES[ORDERS[previous]..ORDERS[previous] + CH_PAGES[previous]] == original {
                    return Err("duplicate original page");
                }
            }
            ORDERS[count] = original_bytes;
            CH_PAGES[count] = original.len();
            bytes(&mut CH_BYTES, &mut original_bytes, original)?;
        }
        let mut selected = None;
        for key in &IMAGE_QUALITIES[quality..] {
            if let Some(candidate) = member(urls, key)? {
                if candidate != b"null" { selected = Some(candidate); break; }
            }
        }
        let mut selected_buffer = [0; 2048];
        let selected = match selected {
            Some(raw) => unescape(quoted(raw)?, &mut selected_buffer)?,
            None => original,
        };
        image_url(selected)?;
        q(b, &mut n, selected)?;
        bytes(b, &mut n, b"}}")?;
        count += 1;
    }
    if count != expected {
        return Err("incomplete original pages");
    };
    bytes(b, &mut n, b"]}")?;
    Ok(n)
}
fn image_request(r: &[u8]) -> Result<usize, E> {
    let mut tmp = [0; 2048];
    let u = unescape(text(args(r)?, b"url")?, &mut tmp)?;
    image_url(u)?;
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"url\":")?;
    q(b, &mut n, u)?;
    bytes(
        b,
        &mut n,
        b",\"headers\":{\"Referer\":\"https://www.pixiv.net/\",\"User-Agent\":",
    )?;
    q(b, &mut n, UA)?;
    bytes(b, &mut n, b"}}")?;
    Ok(n)
}
fn execute(op: &str, result: Result<usize, E>) -> u32 {
    match result {
        Ok(n) if n + 256 < MAX => write_success_payload(op, n),
        Ok(_) => write_error(op, "internal_error", "payload overflow"),
        Err(e) => write_error(
            op,
            if e == "cancelled" {
                "cancelled"
            } else if e == "network failure" {
                "network_error"
            } else if e == "public access rejected"
                || e == "outside All ages manga scope"
                || e == "private artwork"
                || e == "login required"
            {
                "permission_error"
            } else {
                "source_error"
            },
            e,
        ),
    }
}
fn run_search(r: &[u8]) -> u32 {
    execute("search", search(r))
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
// Reference feeds buffer fifty unique works/series. Keep continuation state in
// the opaque cursor, not a process-global paging buffer that another view can
// overwrite. The established anonymous All ages guard remains authoritative.
fn feed(r: &[u8]) -> Result<usize, E> {
    let a = args(r)?;
    let mode = text(a, b"listingId")?;
    if mode != b"popular" && mode != b"latest" { return Err("unknown listing"); }
    let mut page = 1usize; let mut offset = 0usize; let mut wc = 0usize; let mut sc = 0usize;
    let mut pending_len = 0usize;
    if let Some(pg) = member(a, b"page")? {
        if pg.first() != Some(&b'{') { return Err("invalid page contract"); }
        if let Some(c) = member(pg, b"cursor")? {
            if c != b"null" {
                let cv = unsafe { unescape(quoted(c)?, &mut CURSOR)? };
                valid_json(cv)?;
                if text(cv, b"mode")? != mode { return Err("cursor mode mismatch"); }
                page = field_num(cv, b"p")?; offset = field_num(cv, b"offset")?;
                wc = cursor_arrays(cv, b"works", false)?; sc = cursor_arrays(cv, b"series", true)?;
                if page > 1024 || offset > 4096 { return Err("invalid feed cursor"); }
                if page == 0 && offset != 0 { return Err("invalid feed cursor"); }
                if let Some(pending) = member(cv, b"pending")? {
                    let data = unsafe { unescape(quoted(pending)?, &mut SEARCH)? };
                    valid_json(data)?;
                    Array::new(required(data, if mode == b"popular" { b"illust_details" } else { b"illusts" })?)?;
                    pending_len = data.len();
                }
                if (offset != 0) != (pending_len != 0) { return Err("invalid feed cursor snapshot"); }
            }
        }
    }
    let out = payload_buf(); let mut n = 0; let mut emitted = 0usize; let mut fetched = 0usize; let mut retained_len = 0usize;
    bytes(out, &mut n, b"{\"items\":[")?;
    while page != 0 && emitted < 50 {
        fetched += 1; if fetched > 128 { return Err("feed request capacity exceeded"); }
        let feed_len;
        if pending_len != 0 {
            feed_len = pending_len; pending_len = 0;
        } else {
        let mut url = [0u8; 8192]; let mut un = 0;
        if mode == b"popular" { bytes(&mut url, &mut un, b"https://www.pixiv.net/touch/ajax/ranking/illust?mode=daily&type=manga&lang=zh&page=")?; }
        else { bytes(&mut url, &mut un, b"https://www.pixiv.net/touch/ajax/latest?type=manga&lang=zh&p=")?; }
        wn(&mut url, &mut un, page)?;
        let body = fetch(&url[..un])?;
        let mut expected = 0usize;
        if mode == b"popular" {
            let mut ranks = Array::new(required(body, b"ranking")?)?;
            while let Some(row) = ranks.next()? {
                if expected >= COUNT { return Err("ranking capacity exceeded"); }
                let x = field_id(row, b"illustId")?;
                unsafe { if CH_IDS[..expected].contains(&x) { return Err("duplicate ranking identity"); } CH_IDS[expected] = x; }
                expected += 1;
            }
            if expected == 0 { page = 0; offset = 0; break; }
            let mut details = [0u8; 32768]; let mut dn = 0;
            bytes(&mut details, &mut dn, b"https://www.pixiv.net/touch/ajax/illust/details/many?lang=zh&")?;
            for i in 0..expected {
                if i != 0 { bytes(&mut details, &mut dn, b"&")?; }
                bytes(&mut details, &mut dn, b"illust_ids%5B%5D=")?; unsafe { wi(&mut details, &mut dn, CH_IDS[i])?; }
            }
            let data = fetch(&details[..dn])?;
            if data.len() >= MAX { return Err("feed response capacity exceeded"); }
            feed_len = data.len();
            unsafe { SEARCH[..data.len()].copy_from_slice(data); }
            let data = unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(SEARCH).cast::<u8>(), data.len()) };
            let mut rows = Array::new(required(data, b"illust_details")?)?;
            let mut count = 0usize;
            while let Some(v) = rows.next()? {
                if count >= expected || field_id(v, b"id")? != unsafe { CH_IDS[count] } { return Err("ranking detail identity mismatch"); }
                count += 1;
            }
            if count != expected { return Err("ranking details incomplete"); }
        } else {
            if body.len() >= MAX { return Err("feed response capacity exceeded"); }
            feed_len = body.len();
            unsafe { SEARCH[..body.len()].copy_from_slice(body); }
        }
        }
        let data = unsafe { &SEARCH[..feed_len] };
        let mut rows = Array::new(required(data, if mode == b"popular" { b"illust_details" } else { b"illusts" })?)?;
        let mut index = 0usize; let mut stopped = false;
        while let Some(v) = rows.next()? {
            if index < offset { index += 1; continue; }
            if emitted == 50 { stopped = true; break; }
            index += 1;
            if member(v, b"is_ad_container")? == Some(b"1".as_slice()) { continue; }
            match guard(v) {
                Ok(()) => (),
                Err("outside All ages manga scope" | "private artwork" | "invisible artwork" | "login required") => continue,
                Err(e) => return Err(e),
            }
            let x = field_id(v, b"id")?;
            let series = member(v, b"series")?.unwrap_or(b"null");
            let sid = if series != b"null" {
                let id = field_id(series, b"id")?; search_series_owner(v, series)?;
                if unsafe { SEEN_SERIES[..sc].contains(&id) } { continue; }
                if sc >= COUNT { return Err("feed cursor capacity exceeded"); }
                unsafe { SEEN_SERIES[sc] = id }; sc += 1; Some(id)
            } else {
                if unsafe { WORKS[..wc].contains(&x) } { continue; }
                if wc >= COUNT { return Err("feed cursor capacity exceeded"); }
                unsafe { WORKS[wc] = x }; wc += 1; None
            };
            if emitted != 0 { bytes(out, &mut n, b",")?; }
            if let Some(sid) = sid { search_series_card(out, &mut n, v, series, sid)?; }
            else { artwork_card(out, &mut n, v, x)?; }
            emitted += 1;
        }
        if offset > index { return Err("feed cursor offset"); }
        if stopped { offset = index; retained_len = feed_len; break; }
        if index == 0 { page = 0; offset = 0; break; }
        page = page.checked_add(1).ok_or("feed page overflow")?; if page > 1024 { return Err("feed page capacity exceeded"); }
        offset = 0;
    }
    // Reference reports a further page whenever this call returned items; a
    // completed stream therefore emits one terminal empty continuation.
    bytes(out, &mut n, b"],\"page\":{\"hasMore\":")?;
    bytes(out, &mut n, if emitted > 0 { b"true" } else { b"false" })?;
    bytes(out, &mut n, b",\"nextCursor\":")?;
    if emitted == 0 { bytes(out, &mut n, b"null")?; }
    else {
        let c = unsafe { &mut CURSOR }; let mut cn = 0;
        bytes(c, &mut cn, b"{\"mode\":")?; q(c, &mut cn, mode)?; bytes(c, &mut cn, b",\"p\":")?; wn(c, &mut cn, page)?;
        bytes(c, &mut cn, b",\"offset\":")?; wn(c, &mut cn, offset)?; bytes(c, &mut cn, b",\"works\":[")?;
        for i in 0..wc { if i != 0 { bytes(c, &mut cn, b",")?; } unsafe { wi(c, &mut cn, WORKS[i])?; } }
        bytes(c, &mut cn, b"],\"series\":[")?;
        for i in 0..sc { if i != 0 { bytes(c, &mut cn, b",")?; } unsafe { wi(c, &mut cn, SEEN_SERIES[i])?; } }
        bytes(c, &mut cn, b"]")?;
        if retained_len != 0 {
            bytes(c, &mut cn, b",\"pending\":")?;
            q(c, &mut cn, unsafe { &SEARCH[..retained_len] })?;
        }
        bytes(c, &mut cn, b"}")?; q(out, &mut n, &c[..cn])?;
    }
    bytes(out, &mut n, b"}}")?; Ok(n)
}

macro_rules! unsupported {
    ($f:ident,$op:literal) => {
        fn $f(_: &[u8]) -> u32 {
            write_error($op, "not_implemented", "not supported")
        }
    };
}
fn run_get_listings(_: &[u8]) -> u32 {
    let data = "{\"listings\":[{\"id\":\"popular\",\"name\":\"热门\",\"supportsPaging\":true},{\"id\":\"latest\",\"name\":\"最新\",\"supportsPaging\":true}]}".as_bytes();
    payload_buf()[..data.len()].copy_from_slice(data); write_success_payload("get_listings", data.len())
}
fn run_get_manga_list(r: &[u8]) -> u32 { execute("get_manga_list", feed(r)) }
unsupported!(run_get_home, "get_home");
fn run_get_filters(r: &[u8]) -> u32 {
    let schema = if extended_search_supported(r) { r#"{"useSearchForFilters":true,"filters":[{"id":"tags","name":"标签","type":"text","scope":"search"},{"id":"tagsMode","name":"标签匹配","type":"select","scope":"search","default":"partial","options":[{"value":"partial","label":"部分匹配"},{"value":"full","label":"完整匹配"}]},{"id":"order","name":"排序","type":"select","scope":"search","options":[{"value":"default","label":"默认"},{"value":"date","label":"发布日期"}]},{"id":"dateBefore","name":"发布早于","type":"text","scope":"search"},{"id":"dateAfter","name":"发布晚于","type":"text","scope":"search"}]}"#.as_bytes() } else if filter_host(r) { r#"{"filters":[{"id":"dateBefore","name":"发布早于","type":"text","scope":"search"},{"id":"dateAfter","name":"发布晚于","type":"text","scope":"search"}]}"#.as_bytes() } else { br#"{"filters":[]}"#.as_slice() };
    payload_buf()[..schema.len()].copy_from_slice(schema); write_success_payload("get_filters",schema.len())
}
fn run_get_settings(_: &[u8]) -> u32 {
    let p = "{\"settings\":[{\"id\":\"pref_image_quality\",\"name\":\"圖片品質\",\"kind\":\"select\",\"options\":[{\"value\":\"thumb_mini\",\"label\":\"最小縮圖\"},{\"value\":\"small\",\"label\":\"小圖\"},{\"value\":\"regular\",\"label\":\"標準\"},{\"value\":\"original\",\"label\":\"原圖\"}],\"default\":\"original\"}]}".as_bytes();
    payload_buf()[..p.len()].copy_from_slice(p);
    write_success_payload("get_settings", p.len())
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
    response_buffer().write_source_metadata(&SOURCE_INFO, &SOURCE_CAPS)
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
