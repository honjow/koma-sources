#![no_std]
// Independent anonymous simplified-Chinese MangaDex source; fixed primary1319949.
use koma_source_sdk::{
    host,
    json_utils::*,
    source::{SourceCapabilities, SourceInfo},
};
type E = &'static str;
const MAX: usize = 1048576;
const SOURCE_INFO:SourceInfo=SourceInfo{id:"org.mangadex.zh-hans.koma",name:"MangaDex",version:"0.1.1",api_version:"0.2",language:"zh-Hans",author:"Koma",description:"MangaDex public simplified-Chinese chapters. Usage unknown; followedCount listing is not a usage ranking.",content_rating:"nsfw"};
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
static mut URL: [u8; 32768] = [0; 32768];
static mut SEEN: [[u8; 36]; 4096] = [[0; 36]; 4096];
const API: &[u8] = b"https://api.mangadex.org";
const WEB: &[u8] = b"https://mangadex.org";
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

fn uuid(v: &[u8]) -> Result<&[u8], E> {
    if v.len() != 36
        || !v.iter().enumerate().all(|(i, c)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                *c == b'-'
            } else {
                c.is_ascii_digit() || matches!(c, b'a'..=b'f')
            }
        })
    {
        return Err("invalid UUID");
    }
    Ok(v)
}
fn key<'a>(r: &'a [u8], k: &[u8], prefix: &[u8]) -> Result<&'a [u8], E> {
    let s = text(args(r)?, k)?;
    uuid(s.strip_prefix(prefix).ok_or("invalid stable identity")?)
}
fn uq<'a>(raw: &[u8], dst: &'a mut [u8]) -> Result<&'a [u8], E> {
    unescape(quoted(raw)?, dst)
}
fn rawq(b: &mut [u8], n: &mut usize, v: &[u8]) -> Result<(), E> {
    quoted(v)?;
    bytes(b, n, v)
}
fn success(v: &[u8]) -> Result<(), E> {
    if text(v, b"result")? != b"ok" {
        return Err("API result failure");
    }
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
    }
    if !url.starts_with(b"https://api.mangadex.org/") {
        return Err("API URL disallowed");
    }
    let n = koma_source_sdk::build_get_request(
        http_req_buf(),
        url,
        Some(b"https://mangadex.org/"),
        &[
            (b"Origin", WEB),
            (b"User-Agent", b"Koma/0.1.0 (public Chinese source)"),
            (b"Accept", b"application/json"),
        ],
    )
    .ok_or("request capacity")?;
    let len = host::http_request(&http_req_buf()[..n], http_out())
        .map_err(|_| "HTTP transport failure")?;
    if len == 0 || len >= MAX {
        return Err("HTTP response capacity");
    }
    let envelope = &http_out()[..len];
    valid_json(envelope)?;
    if required(envelope, b"ok")? != b"true" {
        return Err("HTTP host failure");
    }
    let status = member(envelope, b"status")?
        .or(member(envelope, b"statusCode")?)
        .ok_or("HTTP status missing")?;
    let status = num(status)?;
    if !(200..300).contains(&status) {
        return Err(match status {
            401 => "HTTP401",
            402 => "HTTP402",
            403 => "HTTP403",
            404 => "HTTP404",
            429 => "HTTP429",
            _ => "HTTP status failure",
        });
    }
    let body = unescape(text(envelope, b"bodyText")?, body_buf())?;
    if body.len() >= MAX {
        return Err("body capacity");
    };
    valid_json(body)?;
    success(body)?;
    Ok(unsafe { core::slice::from_raw_parts(body.as_ptr(), body.len()) })
}
fn relation<'a>(v: &'a [u8], kind: &[u8]) -> Result<Option<&'a [u8]>, E> {
    let mut a = Array::new(required(v, b"relationships")?)?;
    let mut found = None;
    while let Some(x) = a.next()? {
        if text(x, b"type")? == kind {
            if found.is_some() {
                return Err("duplicate relationship type");
            }
            found = Some(x)
        }
    }
    Ok(found)
}
fn owned(v: &[u8], m: &[u8]) -> Result<(), E> {
    if text(
        relation(v, b"manga")?.ok_or("chapter manga missing")?,
        b"id",
    )? != m
    {
        return Err("chapter owner mismatch");
    }
    Ok(())
}
fn literal_map<'a>(obj: &'a [u8], lang: &[u8]) -> Result<Option<&'a [u8]>, E> {
    match member(obj, lang)? {
        Some(v) => {
            quoted(v)?;
            if v != b"\"\"" {
                Ok(Some(v))
            } else {
                Ok(None)
            }
        }
        None => Ok(None),
    }
}
fn title<'a>(a: &'a [u8]) -> Result<&'a [u8], E> {
    let primary = required(a, b"title")?;
    if let Some(s) = literal_map(primary, b"zh")? {
        return Ok(s);
    }
    let mut alts = Array::new(required(a, b"altTitles")?)?;
    while let Some(x) = alts.next()? {
        if let Some(s) = literal_map(x, b"zh")? {
            return Ok(s);
        }
    }
    if let Some(s) = literal_map(primary, b"en")? {
        return Ok(s);
    }
    let mut i = ws(primary, 1);
    while primary.get(i) != Some(&b'}') {
        let end = json_string_end(primary, i).ok_or("title map key")?;
        i = ws(primary, end);
        if primary.get(i) != Some(&b':') {
            return Err("title map colon");
        };
        i = ws(primary, i + 1);
        let end = json_value_end(primary, i, 0).ok_or("title map value")?;
        let s = &primary[i..end];
        quoted(s)?;
        if s != b"\"\"" {
            return Ok(s);
        }
        i = ws(primary, end);
        if primary.get(i) == Some(&b',') {
            i = ws(primary, i + 1)
        }
    }
    Err("title missing")
}
fn rating<'a>(a: &'a [u8]) -> Result<&'a [u8], E> {
    match member(a, b"contentRating")? {
        None | Some(b"null") => Ok(b"unknown"),
        Some(v) => {
            let v = quoted(v)?;
            if matches!(v, b"safe" | b"suggestive" | b"erotica" | b"pornographic") {
                Ok(v)
            } else {
                Err("invalid content rating")
            }
        }
    }
}
fn safe(a: &[u8]) -> Result<(), E> {
    if !matches!(rating(a)?, b"safe" | b"suggestive") {
        return Err("list rating outside public scope");
    }
    Ok(())
}
fn filename(s: &[u8]) -> Result<(), E> {
    if s.is_empty()
        || s.len() > 256
        || s == b"."
        || s == b".."
        || s.iter()
            .any(|c| !c.is_ascii_alphanumeric() && !matches!(c, b'.' | b'_' | b'-'))
    {
        return Err("image filename unsafe");
    }
    Ok(())
}
fn host_url(s: &[u8]) -> Result<(), E> {
    let s = s.strip_prefix(b"https://").ok_or("image requires HTTPS")?;
    let split = s.iter().position(|c| *c == b'/').unwrap_or(s.len());
    let h = &s[..split];
    if h.is_empty()
        || h.iter()
            .any(|c| !c.is_ascii_lowercase() && !c.is_ascii_digit() && !matches!(c, b'.' | b'-'))
    {
        return Err("image host invalid");
    };
    if h != b"uploads.mangadex.org"
        && !(h.ends_with(b".mangadex.network") && h.len() > b".mangadex.network".len())
    {
        return Err("image host disallowed");
    };
    if s[split..]
        .iter()
        .any(|c| *c <= 32 || matches!(c, b'?' | b'#' | b'\\'))
    {
        return Err("image path invalid");
    }
    Ok(())
}
// Published variants only; the Host nonempty value `original` means empty
// cover suffix. Defaults preserve original images, IDs, language/rating filters,
// complete owned catalogue order and the per-image descriptor-refresh contract.
fn cover_quality(r: &[u8]) -> Result<&'static [u8], E> {
    let settings = member(r, b"settings")?.unwrap_or(b"{}");
    let value = match member(settings, b"thumbnailQuality_zh")? { Some(v) => quoted(v)?, None => b"" };
    Ok(match value { b".512.jpg" => b".512.jpg", b".256.jpg" => b".256.jpg", _ => b"" })
}
fn data_saver(r: &[u8]) -> Result<bool, E> {
    let settings = member(r, b"settings")?.unwrap_or(b"{}");
    Ok(matches!(member(settings, b"dataSaverV5_zh")?, Some(b"true" | b"\"true\"")))
}
fn card(b: &mut [u8], n: &mut usize, v: &[u8], expected: Option<&[u8]>, quality: &[u8]) -> Result<(), E> {
    if text(v, b"type")? != b"manga" {
        return Err("manga type");
    };
    let id = uuid(text(v, b"id")?)?;
    if expected.map(|x| x != id).unwrap_or(false) {
        return Err("manga identity mismatch");
    };
    let a = required(v, b"attributes")?;
    let t = title(a)?;
    bytes(b, n, b"{\"id\":\"mdzh:")?;
    bytes(b, n, id)?;
    bytes(b, n, b"\",\"title\":")?;
    rawq(b, n, t)?;
    if let Some(c) = relation(v, b"cover_art")? {
        let ca = required(c, b"attributes")?;
        let raw = text(ca, b"fileName")?;
        filename(raw)?;
        bytes(
            b,
            n,
            b",\"cover\":{\"kind\":\"url\",\"url\":\"https://uploads.mangadex.org/covers/",
        )?;
        bytes(b, n, id)?;
        bytes(b, n, b"/")?;
        bytes(b, n, raw)?;
        bytes(b, n, quality)?;
        bytes(b, n, b"\"}")?
    }
    bytes(b, n, b",\"authors\":[")?;
    let mut ar = Array::new(required(v, b"relationships")?)?;
    let mut ac = 0;
    while let Some(x) = ar.next()? {
        if matches!(text(x, b"type")?, b"author" | b"artist") {
            if let Some(x) = member(x, b"attributes")? {
                if let Some(name) = member(x, b"name")? {
                    if ac > 0 {
                        bytes(b, n, b",")?
                    }
                    rawq(b, n, name)?;
                    ac += 1
                }
            }
        }
    }
    bytes(b, n, b"],\"status\":")?;
    let status = member(a, b"status")?.unwrap_or(b"\"unknown\"");
    if status == b"null" {
        q(b, n, b"unknown")?
    } else {
        rawq(b, n, status)?
    };
    bytes(b, n, b",\"contentRating\":")?;
    q(b, n, rating(a)?)?;
    if let Some(desc) = member(a, b"description")? {
        if let Some(s) = literal_map(desc, b"zh")?.or(literal_map(desc, b"en")?) {
            bytes(b, n, b",\"description\":")?;
            rawq(b, n, s)?
        }
    }
    bytes(b,n,b",\"language\":\"zh-Hans\",\"links\":[{\"kind\":\"source\",\"url\":\"https://mangadex.org/title/")?;
    bytes(b, n, id)?;
    bytes(b, n, b"\"}],\"tags\":[")?;
    let mut tags = Array::new(required(a, b"tags")?)?;
    let mut tc = 0;
    while let Some(x) = tags.next()? {
        let at = required(x, b"attributes")?;
        if let Some(names) = member(at, b"name")? {
            if let Some(t) = literal_map(names, b"zh")?.or(literal_map(names, b"en")?) {
                if tc > 0 {
                    bytes(b, n, b",")?
                }
                rawq(b, n, t)?;
                tc += 1
            }
        }
    }
    bytes(b, n, b"]}")
}
fn request_offset(r: &[u8], limit: usize) -> Result<usize, E> {
    let a = args(r)?;
    let page = member(a, b"page")?;
    let p = match page {
        None => 1,
        Some(v) if v.first() == Some(&b'{') => match member(v, b"cursor")? {
            None | Some(b"null") => 1,
            Some(v) => num(v)?,
        },
        Some(v) => num(v)?,
    };
    if p == 0 {
        return Err("zero page");
    };
    p.checked_sub(1)
        .and_then(|p| p.checked_mul(limit))
        .ok_or("page overflow")
}
fn listing(r: &[u8], search: bool) -> Result<usize, E> {
    let quality = cover_quality(r)?;
    if !search {
        if let Some(v) = member(args(r)?, b"listingId")? {
            if quoted(v)? != b"listing:followed" {
                return Err("unsupported listing");
            }
        }
    }
    let offset = request_offset(r, 20)?;
    let b = unsafe { &mut *core::ptr::addr_of_mut!(URL) };
    let mut n = 0;
    bytes(b, &mut n, API)?;
    bytes(b, &mut n, b"/manga?limit=20&offset=")?;
    wn(b, &mut n, offset)?;
    bytes(b,&mut n,b"&includes%5B%5D=cover_art&hasAvailableChapters=true&availableTranslatedLanguage%5B%5D=zh&contentRating%5B%5D=safe&contentRating%5B%5D=suggestive")?;
    if search {
        let mut querybuf = [0; 8192];
        let query = uq(required(args(r)?, b"query")?, &mut querybuf)?;
        bytes(b, &mut n, b"&title=")?;
        if !write_url_encoded(b, &mut n, query) {
            return Err("query URL capacity");
        };
        bytes(b, &mut n, b"&order%5Brelevance%5D=desc")?
    } else {
        bytes(b, &mut n, b"&order%5BfollowedCount%5D=desc")?
    }
    let v = fetch(unsafe { core::slice::from_raw_parts(b.as_ptr(), n) })?;
    if field_num(v, b"offset")? != offset || field_num(v, b"limit")? != 20 {
        return Err("listing pagination mismatch");
    };
    let total = field_num(v, b"total")?;
    let mut arr = Array::new(required(v, b"data")?)?;
    let output = payload_buf();
    let mut n = 0;
    bytes(output, &mut n, b"{\"items\":[")?;
    let mut count = 0;
    let mut ids = [[0; 36]; 20];
    while let Some(x) = arr.next()? {
        if count >= 20 {
            return Err("listing item capacity");
        };
        let id = uuid(text(x, b"id")?)?;
        if ids[..count].iter().any(|x| x == id) {
            return Err("same-page duplicate");
        };
        ids[count].copy_from_slice(id);
        safe(required(x, b"attributes")?)?;
        if count > 0 {
            bytes(output, &mut n, b",")?
        };
        card(output, &mut n, x, None, quality)?;
        count += 1
    }
    let end = offset.checked_add(count).ok_or("listing count overflow")?;
    if end > total || (count == 0 && offset < total) {
        return Err("listing no progress");
    };
    let more = end < total;
    if more && count != 20 {
        return Err("listing incomplete nonterminal page");
    };
    bytes(output, &mut n, b"],\"page\":{\"hasMore\":")?;
    bytes(output, &mut n, if more { b"true" } else { b"false" })?;
    bytes(output, &mut n, b",\"nextCursor\":")?;
    if more {
        q(output, &mut n, {
            let s = scratch_a();
            let mut j = 0;
            wn(s, &mut j, offset / 20 + 2)?;
            &s[..j]
        })?
    } else {
        bytes(output, &mut n, b"null")?
    };
    bytes(output, &mut n, b"}}")?;
    Ok(n)
}
fn detail(r: &[u8]) -> Result<usize, E> {
    let quality = cover_quality(r)?;
    let id = key(r, b"mangaId", b"mdzh:")?;
    let url = make_url(&[
        API,
        b"/manga/",
        id,
        b"?includes%5B%5D=cover_art&includes%5B%5D=author&includes%5B%5D=artist",
    ])?;
    let v = fetch(url)?;
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"manga\":")?;
    card(out, &mut n, required(v, b"data")?, Some(id), quality)?;
    bytes(out, &mut n, b"}")?;
    Ok(n)
}
fn nullable_number(b: &mut [u8], n: &mut usize, v: Option<&[u8]>) -> Result<(), E> {
    match v {
        None | Some(b"null") => bytes(b, n, b"null"),
        Some(v) => {
            let v = quoted(v)?;
            if v.is_empty() {
                return bytes(b, n, b"null");
            };
            if json_value_end(v, 0, 0) != Some(v.len())
                || !matches!(v.first(), Some(b'-' | b'0'..=b'9'))
            {
                return Err("chapter number invalid");
            };
            let value = core::str::from_utf8(v)
                .map_err(|_| "chapter number UTF8")?
                .parse::<f64>()
                .map_err(|_| "chapter number invalid")?;
            if !value.is_finite() {
                return Err("chapter number nonfinite");
            };
            bytes(b, n, v)
        }
    }
}
fn chapter_card(b: &mut [u8], n: &mut usize, v: &[u8], m: &[u8]) -> Result<bool, E> {
    if text(v, b"type")? != b"chapter" {
        return Err("chapter type");
    };
    let id = uuid(text(v, b"id")?)?;
    owned(v, m)?;
    let a = required(v, b"attributes")?;
    if text(a, b"translatedLanguage")? != b"zh" {
        return Err("chapter language mismatch");
    };
    let pages = field_num(a, b"pages")?;
    let unavailable = match member(a, b"isUnavailable")? {
        None | Some(b"false") => false,
        Some(b"true") => true,
        _ => return Err("availability type"),
    };
    let external = match member(a, b"externalUrl")? {
        None | Some(b"null") => false,
        Some(v) => {
            quoted(v)?;
            true
        }
    };
    if pages == 0 || unavailable || external {
        return Ok(false);
    }
    bytes(b, n, b"{\"id\":\"mdch:zh:")?;
    bytes(b, n, m)?;
    bytes(b, n, b":")?;
    bytes(b, n, id)?;
    bytes(b, n, b"\",\"mangaId\":\"mdzh:")?;
    bytes(b, n, m)?;
    bytes(b, n, b"\",\"title\":")?;
    match member(a, b"title")? {
        None | Some(b"null") | Some(b"\"\"") => q(b, n, b"Chapter")?,
        Some(t) => rawq(b, n, t)?,
    };
    bytes(b, n, b",\"chapterNumber\":")?;
    nullable_number(b, n, member(a, b"chapter")?)?;
    bytes(b, n, b",\"volumeNumber\":")?;
    nullable_number(b, n, member(a, b"volume")?)?;
    bytes(b, n, b",\"language\":\"zh-Hans\",\"pageCount\":")?;
    wn(b, n, pages)?;
    bytes(b, n, b",\"publishedAt\":")?;
    rawq(b, n, required(a, b"publishAt")?)?;
    bytes(b, n, b",\"updatedAt\":")?;
    rawq(b, n, required(a, b"updatedAt")?)?;
    let mut rel = Array::new(required(v, b"relationships")?)?;
    let mut first = true;
    while let Some(x) = rel.next()? {
        if text(x, b"type")? == b"scanlation_group" {
            if let Some(at) = member(x, b"attributes")? {
                if let Some(t) = member(at, b"name")? {
                    if first {
                        bytes(b, n, b",\"scanlator\":")?;
                        rawq(b, n, t)?;
                        first = false
                    }
                }
            }
        }
    }
    bytes(b, n, b"}")?;
    Ok(true)
}
fn feed(m: &[u8], target: Option<&[u8]>) -> Result<(usize, usize), E> {
    let mut offset = 0;
    let mut total0 = None;
    let mut rawcount = 0;
    let mut emitted = 0;
    let mut wanted = None;
    let output = payload_buf();
    let mut n = 0;
    bytes(output, &mut n, b"{\"items\":[")?;
    loop {
        let mut off = [0; 24];
        let mut j = 0;
        wn(&mut off, &mut j, offset)?;
        let url=make_url(&[API,b"/manga/",m,b"/feed?includes%5B%5D=scanlation_group&includes%5B%5D=user&limit=100&offset=",&off[..j],b"&translatedLanguage%5B%5D=zh&order%5Bvolume%5D=desc&order%5Bchapter%5D=desc&includeFuturePublishAt=0&includeEmptyPages=0&includeUnavailable=0&contentRating%5B%5D=safe&contentRating%5B%5D=suggestive&contentRating%5B%5D=erotica&contentRating%5B%5D=pornographic"])?;
        let v = fetch(url)?;
        if field_num(v, b"offset")? != offset || field_num(v, b"limit")? != 100 {
            return Err("feed pagination mismatch");
        };
        let total = field_num(v, b"total")?;
        if total > 4096 {
            return Err("feed capacity");
        };
        if total0.map(|x| x != total).unwrap_or(false) {
            return Err("feed total drift");
        };
        total0 = Some(total);
        let mut arr = Array::new(required(v, b"data")?)?;
        let mut pagecount = 0;
        while let Some(x) = arr.next()? {
            if pagecount >= 100 || rawcount >= 4096 {
                return Err("feed count capacity");
            };
            let id = uuid(text(x, b"id")?)?;
            let seen = unsafe { &mut *core::ptr::addr_of_mut!(SEEN) };
            if seen[..rawcount].iter().any(|s| s == id) {
                return Err("duplicate chapter UUID");
            };
            seen[rawcount].copy_from_slice(id);
            rawcount += 1;
            pagecount += 1;
            let before = n;
            if emitted > 0 {
                bytes(output, &mut n, b",")?
            };
            let keep = chapter_card(output, &mut n, x, m)?;
            if !keep {
                n = before
            } else {
                emitted += 1;
                if target == Some(id) {
                    wanted = Some(field_num(required(x, b"attributes")?, b"pages")?)
                }
            }
        }
        if offset.checked_add(pagecount).ok_or("feed overflow")? > total {
            return Err("feed total mismatch");
        };
        let end = offset.checked_add(100).ok_or("feed offset overflow")?;
        if end >= total {
            if rawcount != total {
                return Err("feed incomplete");
            };
            break;
        }
        if pagecount != 100 {
            return Err("feed missing page items");
        };
        offset = end;
    }
    bytes(
        output,
        &mut n,
        b"],\"page\":{\"hasMore\":false,\"nextCursor\":null}}",
    )?;
    match target {
        Some(_) => Ok((
            n,
            wanted.ok_or("chapter not publicly available in owned zh feed")?,
        )),
        None => Ok((n, 0)),
    }
}
fn chapter_key(r: &[u8]) -> Result<(&[u8], &[u8]), E> {
    let k = text(args(r)?, b"chapterId")?
        .strip_prefix(b"mdch:zh:")
        .ok_or("chapter namespace")?;
    if k.len() != 73 || k[36] != b':' {
        return Err("chapter identity");
    };
    Ok((uuid(&k[..36])?, uuid(&k[37..])?))
}
fn descriptor(ch: &[u8]) -> Result<&'static [u8], E> {
    fetch(make_url(&[API, b"/at-home/server/", ch])?)
}
fn validate_files(v: &[u8], saver: bool) -> Result<(), E> {
    let data = required(required(v, b"chapter")?, if saver { b"dataSaver" } else { b"data" })?;
    let mut a = Array::new(data)?;
    let mut seen: [Option<&[u8]>; 4096] = [None; 4096];
    let mut count = 0;
    while let Some(x) = a.next()? {
        let f = quoted(x)?;
        filename(f)?;
        if count == seen.len() {
            return Err("page capacity");
        }
        if seen[..count].iter().any(|prior| *prior == Some(f)) {
            return Err(if saver { "duplicate saver filename" } else { "duplicate original filename" });
        }
        seen[count] = Some(f);
        count += 1;
    }
    if count == 0 {
        return Err(if saver { "empty saver data" } else { "empty original data" });
    }
    Ok(())
}
fn image_emit(b: &mut [u8], n: &mut usize, v: &[u8], index: usize, saver: bool) -> Result<(), E> {
    let mut hb = [0; 512];
    let h = uq(required(v, b"baseUrl")?, &mut hb)?;
    host_url(h)?;
    if h.ends_with(b"/") {
        return Err("descriptor base trailing slash");
    };
    let chapter = required(v, b"chapter")?;
    let hash = text(chapter, b"hash")?;
    if hash.len() != 32 || !hash.iter().all(|c| c.is_ascii_hexdigit()) {
        return Err("descriptor hash invalid");
    };
    let mut a = Array::new(required(chapter, if saver { b"dataSaver" } else { b"data" })?)?;
    let mut count = 0;
    let mut selected = None;
    while let Some(x) = a.next()? {
        let f = quoted(x)?;
        filename(f)?;
        if count == index {
            selected = Some(f)
        };
        count += 1;
        if count > 4096 {
            return Err("page capacity");
        }
    }
    let f = selected.ok_or("page index out of range")?;
    bytes(b, n, h)?;
    bytes(b, n, if saver { b"/data-saver/" } else { b"/data/" })?;
    bytes(b, n, hash)?;
    bytes(b, n, b"/")?;
    bytes(b, n, f)
}
fn pages(r: &[u8]) -> Result<usize, E> {
    let saver = data_saver(r)?;
    let (m, ch) = chapter_key(r)?;
    let (_, count) = feed(m, Some(ch))?;
    let v = descriptor(ch)?;
    validate_files(v, false)?;
    if saver { validate_files(v, true)?; }
    let data = required(required(v, b"chapter")?, if saver { b"dataSaver" } else { b"data" })?;
    let mut arr = Array::new(data)?;
    let mut actual = 0;
    while arr.next()?.is_some() {
        actual += 1
    }
    if actual != count || actual == 0 || actual > 4096 {
        return Err("page count mismatch");
    };
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"chapterId\":")?;
    rawq(out, &mut n, required(args(r)?, b"chapterId")?)?;
    bytes(out, &mut n, b",\"pages\":[")?;
    for i in 0..actual {
        if i > 0 {
            bytes(out, &mut n, b",")?
        };
        bytes(out, &mut n, b"{\"id\":\"mdpage:zh:")?;
        bytes(out, &mut n, ch)?;
        bytes(out, &mut n, b":")?;
        wn(out, &mut n, i)?;
        bytes(out, &mut n, b"\",\"index\":")?;
        wn(out, &mut n, i)?;
        bytes(out, &mut n, b",\"image\":{\"kind\":\"url\",\"url\":\"")?;
        image_emit(out, &mut n, v, i, saver)?;
        bytes(out, &mut n, b"\"}}")?
    }
    bytes(out, &mut n, b"]}")?;
    Ok(n)
}
fn image_request(r: &[u8]) -> Result<usize, E> {
    let saver = data_saver(r)?;
    let a = args(r)?;
    let page_id = text(a, b"pageId")?;
    if let Some(m) = page_id.strip_prefix(b"mdzh:") {
        let m = uuid(m)?;
        let mut ub = [0; 2048];
        let url = uq(required(a, b"url")?, &mut ub)?;
        let path = url
            .strip_prefix(b"https://uploads.mangadex.org/covers/")
            .ok_or("cover host or path invalid")?;
        if path.len() < 38 || &path[..36] != m || path[36] != b'/' {
            return Err("cover owner mismatch");
        }
        filename(&path[37..])?;
        let out = payload_buf();
        let mut n = 0;
        bytes(out, &mut n, b"{\"url\":")?;
        q(out, &mut n, url)?;
        bytes(out, &mut n, b",\"headers\":{\"Referer\":\"https://mangadex.org/\",\"Origin\":\"https://mangadex.org\",\"User-Agent\":\"Koma/0.1.0 (public Chinese source)\"}}")?;
        return Ok(n);
    }
    let id = page_id
        .strip_prefix(b"mdpage:zh:")
        .ok_or("page namespace")?;
    if id.len() < 38 || id[36] != b':' {
        return Err("page identity");
    };
    let ch = uuid(&id[..36])?;
    let idx = number(&id[37..]).ok_or("page index invalid")?;
    if idx >= 4096 {
        return Err("page index capacity");
    };
    let mut oldurl = [0; 2048];
    host_url(uq(required(a, b"url")?, &mut oldurl)?)?;
    let v = descriptor(ch)?;
    validate_files(v, false)?;
    if saver { validate_files(v, true)?; }
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"url\":\"")?;
    image_emit(out, &mut n, v, idx, saver)?;
    bytes(out,&mut n,b"\",\"headers\":{\"Referer\":\"https://mangadex.org/\",\"Origin\":\"https://mangadex.org\",\"User-Agent\":\"Koma/0.1.0 (public Chinese source)\"}}")?;
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
    execute(
        "get_chapters",
        key(r, b"mangaId", b"mdzh:").and_then(|m| feed(m, None).map(|x| x.0)),
    )
}
fn run_get_pages(r: &[u8]) -> u32 {
    execute("get_pages", pages(r))
}
fn run_get_image_request(r: &[u8]) -> u32 {
    execute("get_image_request", image_request(r))
}
fn run_get_listings(_: &[u8]) -> u32 {
    let data="{\"listings\":[{\"id\":\"listing:followed\",\"name\":\"关注较多\",\"kind\":\"popular\",\"supportsPaging\":true}]}".as_bytes();
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
    let data = r#"{"settings":[{"id":"thumbnailQuality_zh","name":"封面品質","kind":"select","options":[{"value":"original","label":"原圖"},{"value":".512.jpg","label":"中等"},{"value":".256.jpg","label":"低"}],"default":"original"},{"id":"dataSaverV5_zh","name":"省流量","kind":"toggle","default":"false"}]}"#.as_bytes();
    payload_buf()[..data.len()].copy_from_slice(data);
    write_success_payload("get_settings", data.len())
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
 b"{\"sourceInfo\":{\"id\":\"", SOURCE_INFO.id.as_bytes(), b"\",\"name\":\"", SOURCE_INFO.name.as_bytes(), b"\",\"version\":\"", SOURCE_INFO.version.as_bytes(), b"\",\"apiVersion\":\"0.2\",\"language\":\"zh-Hans\",\"author\":\"Koma\",\"description\":\"",SOURCE_INFO.description.as_bytes(),
 "\",\"contentRating\":\"nsfw\",\"localizedNames\":{\"zh-Hans\":\"MangaDex（简体中文）\",\"zh-Hant\":\"MangaDex（簡體中文）\",\"en\":\"MangaDex (Simplified Chinese)\"}},\"capabilities\":{\"search\":true,\"mangaDetail\":true,\"chapters\":true,\"pages\":true,\"listings\":true,\"mangaList\":true,\"imageRequest\":true,\"home\":false,\"filters\":false,\"settings\":true,\"future\":{\"login\":false,\"auth\":false,\"process_page_image\":false,\"page_description\":false,\"base_url\":false,\"deeplink\":false,\"migration\":false}}}".as_bytes()
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
