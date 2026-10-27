#![no_std]
// Independent modified adaptation of NamiComi published JSON contract, Apache-2.0.
// Keiyoushi primary1319949ca9bc456908196e3ba2d104e94182c5d6. No auth/premium fallback.
use koma_source_sdk::{
    host,
    json_utils::*,
    source::{SourceCapabilities, SourceInfo},
};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.namicomi.zh-hant.koma",
    name: "NamiComi",
    version: "0.1.3",
    api_version: "0.2",
    language: "zh-Hant",
    author: "Koma",
    description:
        "NamiComi Traditional Chinese public API; anonymous readable pages; optional locked chapter metadata. Usage unknown.",
    content_rating: "safe",
};
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: true,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: true,
    manga_list: true,
    home: false,
    filters: false,
    settings: true,
    image_request: true,
    credentials: false,
};
const API: &[u8] = b"https://api.namicomi.com";
const WEB: &[u8] = b"https://namicomi.com";
const CDN: &[u8] = b"https://uploads.namicomi.com";
const MP: &[u8] = b"namicomi:zh-hant:";
const CP: &[u8] = b"namicomich:zh-hant:";
const PP: &[u8] = b"namicomipage:zh-hant:";
const INCLUDES:&[u8]=b"&includes%5B%5D=cover_art&includes%5B%5D=organization&includes%5B%5D=tag&includes%5B%5D=primary_tag&includes%5B%5D=secondary_tag";
const TYPES: &[u8] = b"&types%5B%5D=manhua&types%5B%5D=manwha&types%5B%5D=manga&types%5B%5D=comic";
// Leave response framing headroom: complete result must remain below native1MiB.
koma_source_sdk::koma_source_buffers! {payload:1048000,http_out:2*1024*1024,body:1024*1024,http_req:65536,scratch:8192,}
koma_source_sdk::koma_source_helpers!();
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
fn first_string(map: &[u8]) -> Result<Option<&[u8]>, E> {
    if map.first() != Some(&b'{') {
        return Err("expected language map");
    }
    let mut i = ws(map, 1);
    while map.get(i) != Some(&b'}') {
        let end = json_string_end(map, i).ok_or("invalid map key")?;
        i = ws(map, end);
        if map.get(i) != Some(&b':') {
            return Err("invalid map member");
        }
        i = ws(map, i + 1);
        let end = json_value_end(map, i, 0).ok_or("invalid map value")?;
        let v = &map[i..end];
        if v.first() == Some(&b'"') && !quoted(v)?.is_empty() {
            return Ok(Some(v));
        }
        i = ws(map, end);
        if map.get(i) == Some(&b',') {
            i = ws(map, i + 1);
        } else if map.get(i) != Some(&b'}') {
            return Err("invalid map separator");
        }
    }
    Ok(None)
}
fn localized(map: &[u8], first: bool) -> Result<Option<&[u8]>, E> {
    for k in [b"zh-hant".as_slice(), b"en".as_slice()] {
        if let Some(v) = member(map, k)? {
            if !quoted(v)?.is_empty() {
                return Ok(Some(v));
            }
        }
    }
    if first {
        first_string(map)
    } else {
        Ok(None)
    }
}
fn args(r: &[u8]) -> Result<&[u8], E> {
    valid_json(r)?;
    Ok(member(r, b"args")?.unwrap_or(r))
}
fn integer(v: &[u8]) -> Result<usize, E> {
    number(v).ok_or("invalid integer")
}
fn page(r: &[u8]) -> Result<usize, E> {
    let a = args(r)?;
    let v = match member(a, b"page")? {
        None => return Ok(1),
        Some(v) => v,
    };
    let v = if v.first() == Some(&b'{') {
        member(v, b"cursor")?.unwrap_or(b"null")
    } else {
        v
    };
    if v == b"null" {
        return Ok(1);
    }
    let v = if v.first() == Some(&b'"') {
        quoted(v)?
    } else {
        v
    };
    integer(v).map_err(|_| "invalid request").and_then(|n| {
        if n > 0 && n <= 100000 {
            Ok(n)
        } else {
            Err("invalid request")
        }
    })
}
fn id(s: &[u8]) -> Result<&[u8], E> {
    if s.is_empty() || s.len() > 64 || s.iter().any(|c| !c.is_ascii_alphanumeric()) {
        return Err("invalid identity");
    }
    Ok(s)
}
fn manga_id(r: &[u8]) -> Result<&[u8], E> {
    id(text(args(r)?, b"mangaId")?
        .strip_prefix(MP)
        .ok_or("invalid request")?)
}
fn chapter_id(r: &[u8]) -> Result<(&[u8], &[u8]), E> {
    let s = text(args(r)?, b"chapterId")?
        .strip_prefix(CP)
        .ok_or("invalid request")?;
    let mut parts = s.split(|c| *c == b':');
    let title = id(parts.next().ok_or("invalid request")?)?;
    let chapter = id(parts.next().ok_or("invalid request")?)?;
    if parts.next().is_some() {
        return Err("invalid request");
    }
    Ok((title, chapter))
}
fn qid(b: &mut [u8], n: &mut usize, p: &[u8], a: &[u8], suffix: Option<&[u8]>) -> Result<(), E> {
    bytes(b, n, b"\"")?;
    bytes(b, n, p)?;
    bytes(b, n, a)?;
    if let Some(s) = suffix {
        bytes(b, n, b":")?;
        bytes(b, n, s)?;
    }
    bytes(b, n, b"\"")
}
fn rawq(b: &mut [u8], n: &mut usize, v: &[u8]) -> Result<(), E> {
    quoted(v)?;
    bytes(b, n, v)
}
fn wn(b: &mut [u8], n: &mut usize, v: usize) -> Result<(), E> {
    if write_usize(b, n, v) {
        Ok(())
    } else {
        Err("payload overflow")
    }
}
fn error_code(e: &str) -> &'static str {
    match e {
        "authentication required" => "auth_required",
        "payment or account required" => "auth_required",
        "permission denied" => "permission_denied",
        "not found" => "not_found",
        "rate limited" => "rate_limit",
        "client failure" => "client_error",
        "server failure" => "server_error",
        "network failure" => "network_error",
        "cancelled" => "cancelled",
        "payload overflow" | "catalogue capacity exceeded" => "internal_error",
        "invalid request" | "invalid identity" | "empty query" | "unknown listing"
        | "invalid image URL" => "invalid_request",
        _ => "parse_error",
    }
}
fn execute(op: &str, res: Result<usize, E>) -> u32 {
    match res {
        Ok(n) if n + 256 < 1048576 => write_success_payload(op, n),
        Ok(_) => write_error(op, "internal_error", "payload overflow"),
        Err(e) => {
            host::log_info(error_code(e).as_bytes());
            write_error(op, error_code(e), e)
        }
    }
}
fn fetch(url: &[u8], post: Option<&[u8]>) -> Result<&'static [u8], E> {
    if host::check_cancel() {
        return Err("cancelled");
    }
    let b = http_req_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"version\":1,\"method\":")?;
    q(b, &mut n, if post.is_some() { b"POST" } else { b"GET" })?;
    bytes(b, &mut n, b",\"url\":")?;
    q(b, &mut n, url)?;
    bytes(b,&mut n,b",\"headers\":{\"User-Agent\":\"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0\",\"Referer\":\"https://namicomi.com/\",\"Origin\":\"https://namicomi.com\"")?;
    if post.is_some() {
        bytes(b, &mut n, b",\"Content-Type\":\"application/json\"")?;
    } else {
        bytes(b, &mut n, b",\"Cache-Control\":\"max-age=600\"")?;
    }
    bytes(
        b,
        &mut n,
        b"},\"timeoutMs\":15000,\"responseKind\":\"bodyText\"",
    )?;
    if let Some(body) = post {
        valid_json(body)?;
        bytes(b, &mut n, b",\"bodyBase64\":")?;
        q(b, &mut n, body)?;
    }
    bytes(b, &mut n, b"}")?;
    let len = host::http_request(&b[..n], http_out()).map_err(|_| "network failure")?;
    let response =
        unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(HTTP_OUT) as *const u8, len) };
    valid_json(response)?;
    if member(response, b"ok")? != Some(b"true".as_slice()) {
        return Err("network failure");
    }
    let status = integer(
        member(response, b"status")?
            .or(member(response, b"statusCode")?)
            .ok_or("missing HTTP status")?,
    )?;
    match status {
        200..=299 => {}
        401 => return Err("authentication required"),
        402 => return Err("payment or account required"),
        403 => return Err("permission denied"),
        404 => return Err("not found"),
        429 => return Err("rate limited"),
        400..=499 => return Err("client failure"),
        500..=599 => return Err("server failure"),
        _ => return Err("unexpected HTTP status"),
    }
    let raw = text(response, b"bodyText")?;
    let body = unescape(raw, body_buf())?;
    valid_json(body)?;
    if text(body, b"result")? != b"ok" {
        return Err("API result failed");
    }
    Ok(body)
}
fn api_url<'a>(path: &[u8], b: &'a mut [u8]) -> Result<&'a [u8], E> {
    let mut n = 0;
    bytes(b, &mut n, API)?;
    bytes(b, &mut n, path)?;
    Ok(&b[..n])
}
fn filename(s: &[u8]) -> Result<&[u8], E> {
    if s.is_empty()
        || s.len() > 256
        || s == b"."
        || s == b".."
        || s.iter()
            .any(|c| !c.is_ascii_alphanumeric() && !b"._-".contains(c))
    {
        return Err("invalid image URL");
    }
    Ok(s)
}
fn valid_image(url: &[u8]) -> Result<(), E> {
    let rest = url
        .strip_prefix(CDN)
        .and_then(|v| v.strip_prefix(b"/"))
        .ok_or("invalid image URL")?;
    let parts: [Option<&[u8]>; 5] = {
        let mut it = rest.split(|c| *c == b'/');
        [it.next(), it.next(), it.next(), it.next(), it.next()]
    };
    if rest.starts_with(b"covers/") {
        let mut it = rest.split(|c| *c == b'/');
        it.next();
        id(it.next().ok_or("invalid image URL")?)?;
        filename(it.next().ok_or("invalid image URL")?)?;
        if it.next().is_some() {
            return Err("invalid image URL");
        }
    } else {
        if parts[0] != Some(b"chapter") {
            return Err("invalid image URL");
        }
        id(parts[1].ok_or("invalid image URL")?)?;
        let hash = parts[2].ok_or("invalid image URL")?;
        if hash.len() != 32 || hash.iter().any(|c| !c.is_ascii_hexdigit()) {
            return Err("invalid image URL");
        }
        if parts[3] != Some(b"source") && parts[3] != Some(b"low") {
            return Err("invalid image URL");
        }
        filename(parts[4].ok_or("invalid image URL")?)?;
        if rest.split(|c| *c == b'/').count() != 5 {
            return Err("invalid image URL");
        }
    }
    Ok(())
}
// Explicit quality preferences follow the published NamiComi variants only.
// Host option IDs cannot be blank: `original` encodes upstream empty suffix.
// Missing/unknown settings preserve original covers and source pages; no auth,
// premium, language, catalogue order or identity boundary is changed.
fn cover_quality(r: &[u8]) -> Result<&'static [u8], E> {
    let settings = member(r, b"settings")?.unwrap_or(b"{}");
    let value = match member(settings, b"thumbnailQuality_zh-hant")? {
        Some(v) => quoted(v)?, None => b"",
    };
    Ok(match value { b".512.jpg" => b".512.jpg", b".256.jpg" => b".256.jpg", _ => b"" })
}
fn data_saver(r: &[u8]) -> Result<bool, E> {
    let settings = member(r, b"settings")?.unwrap_or(b"{}");
    Ok(matches!(member(settings, b"dataSaver_zh-hant")?, Some(b"true" | b"\"true\"")))
}
fn cover_url<'a>(entity: &[u8], quality: &[u8], b: &'a mut [u8]) -> Result<Option<&'a [u8]>, E> {
    let title = id(text(entity, b"id")?)?;
    let mut it = Array::new(required(entity, b"relationships")?)?;
    while let Some(rel) = it.next()? {
        if text(rel, b"type")? == b"cover_art" {
            let a = required(rel, b"attributes")?;
            let raw = match member(a, b"fileName")? {
                Some(b"null") | None => return Ok(None),
                Some(v) => v,
            };
            let mut f = [0; 256];
            let f = unescape(quoted(raw)?, &mut f)?;
            filename(f)?;
            let mut n = 0;
            bytes(b, &mut n, CDN)?;
            bytes(b, &mut n, b"/covers/")?;
            bytes(b, &mut n, title)?;
            bytes(b, &mut n, b"/")?;
            bytes(b, &mut n, f)?;
            bytes(b, &mut n, quality)?;
            valid_image(&b[..n])?;
            return Ok(Some(&b[..n]));
        }
    }
    Ok(None)
}
fn manga(entity: &[u8], quality: &[u8], b: &mut [u8], n: &mut usize) -> Result<(), E> {
    if text(entity, b"type")? != b"title" {
        return Err("invalid title entity");
    }
    let titleid = id(text(entity, b"id")?)?;
    let attr = required(entity, b"attributes")?;
    let title = localized(required(attr, b"title")?, true)?.ok_or("empty title")?;
    bytes(b, n, b"{\"id\":")?;
    qid(b, n, MP, titleid, None)?;
    bytes(b, n, b",\"title\":")?;
    rawq(b, n, title)?;
    bytes(b, n, b",\"cover\":")?;
    let mut u = [0; 1024];
    match cover_url(entity, quality, &mut u)? {
        Some(url) => {
            bytes(b, n, b"{\"kind\":\"url\",\"url\":")?;
            q(b, n, url)?;
            bytes(b, n, b"}")?
        }
        None => bytes(b, n, b"{\"kind\":\"none\"}")?,
    }
    bytes(b, n, b",\"authors\":[")?;
    let mut authors: [&[u8]; 32] = [b""; 32];
    let mut count = 0;
    let rels = required(entity, b"relationships")?;
    let mut it = Array::new(rels)?;
    while let Some(rel) = it.next()? {
        if text(rel, b"type")? == b"organization" {
            if let Some(a) = member(rel, b"attributes")? {
                if a == b"null" {
                    continue;
                }
                let name = raw_string(a, b"name")?;
                if !quoted(name)?.is_empty() && !authors[..count].contains(&name) {
                    if count == 32 {
                        return Err("too many authors");
                    }
                    authors[count] = name;
                    count += 1;
                }
            }
        }
    }
    for (i, a) in authors[..count].iter().enumerate() {
        if i > 0 {
            bytes(b, n, b",")?;
        }
        rawq(b, n, a)?;
    }
    bytes(b, n, b"],\"tags\":[")?;
    let mut emitted = 0;
    for group in [
        b"content-warnings".as_slice(),
        b"format".as_slice(),
        b"genre".as_slice(),
        b"theme".as_slice(),
    ] {
        let mut tags: [&[u8]; 128] = [b""; 128];
        let mut tc = 0;
        let mut it = Array::new(rels)?;
        while let Some(rel) = it.next()? {
            let rt = text(rel, b"type")?;
            if rt == b"tag" || rt == b"primary_tag" || rt == b"secondary_tag" {
                let a = required(rel, b"attributes")?;
                if text(a, b"group")? == group {
                    if let Some(name) = localized(required(a, b"name")?, false)? {
                        if !tags[..tc].contains(&name) {
                            if tc == tags.len() {
                                return Err("too many tags");
                            }
                            tags[tc] = name;
                            tc += 1;
                        }
                    }
                }
            }
        }
        tags[..tc].sort_unstable();
        for tag in &tags[..tc] {
            if emitted > 0 {
                bytes(b, n, b",")?;
            }
            rawq(b, n, tag)?;
            emitted += 1;
        }
    }
    bytes(b, n, b"],\"status\":")?;
    let status = member(attr, b"publicationStatus")?.unwrap_or(b"null");
    let status = match status {
        b"\"ongoing\"" | b"\"completed\"" | b"\"hiatus\"" | b"\"cancelled\"" => status,
        _ => b"\"unknown\"",
    };
    rawq(b, n, status)?;
    bytes(b, n, b",\"contentRating\":")?;
    let rating = member(attr, b"contentRating")?.unwrap_or(b"null");
    let rating = match rating {
        b"\"safe\"" | b"\"restricted\"" | b"\"mature\"" => rating,
        _ => b"\"unknown\"",
    };
    rawq(b, n, rating)?;
    bytes(b, n, b",\"summary\":")?;
    match member(attr, b"description")? {
        Some(d) if d != b"null" => match localized(d, true)? {
            Some(v) => rawq(b, n, v)?,
            None => bytes(b, n, b"null")?,
        },
        _ => bytes(b, n, b"null")?,
    }
    bytes(b, n, b",\"url\":\"")?;
    bytes(b, n, WEB)?;
    bytes(b, n, b"/zh-hant/title/")?;
    bytes(b, n, titleid)?;
    bytes(b, n, b"\"}")
}
fn meta(body: &[u8]) -> Result<(usize, usize, usize), E> {
    let m = required(body, b"meta")?;
    Ok((
        integer(required(m, b"limit")?)?,
        integer(required(m, b"offset")?)?,
        integer(required(m, b"total")?)?,
    ))
}
fn list(r: &[u8], search: bool) -> Result<usize, E> {
    let quality = cover_quality(r)?;
    let pn = page(r)?;
    let offset = 20usize.checked_mul(pn - 1).ok_or("invalid request")?;
    let mut path = [0; 8192];
    let mut n = 0;
    bytes(&mut path, &mut n, b"/title/search?")?;
    if search {
        let raw = text(args(r)?, b"query")?;
        let mut qbuf = [0; 4096];
        let query = unescape(raw, &mut qbuf)?;
        let query = core::str::from_utf8(query).map_err(|_| "invalid UTF8")?;
        let mut normalized = [0; 4096];
        let mut len = 0;
        for word in query.split_whitespace() {
            if len > 0 {
                bytes(&mut normalized, &mut len, b" ")?;
            }
            bytes(&mut normalized, &mut len, word.as_bytes())?;
        }
        if len == 0 {
            return Err("empty query");
        }
        bytes(&mut path,&mut n,b"order%5BpublishedAt%5D=desc&hasAvailableChapters=true&availableTranslatedLanguages%5B%5D=zh-hant&title=")?;
        if !write_url_encoded(&mut path, &mut n, &normalized[..len]) {
            return Err("payload overflow");
        }
    } else {
        let listing = member(args(r)?, b"listingId")?.unwrap_or(b"\"popular\"");
        let order = match quoted(listing)? {
            b"popular" => b"views".as_slice(),
            b"latest" => b"publishedAt".as_slice(),
            _ => return Err("unknown listing"),
        };
        bytes(&mut path, &mut n, b"order%5B")?;
        bytes(&mut path, &mut n, order)?;
        bytes(
            &mut path,
            &mut n,
            b"%5D=desc&availableTranslatedLanguages%5B%5D=zh-hant",
        )?;
    }
    bytes(&mut path, &mut n, b"&limit=20&offset=")?;
    wn(&mut path, &mut n, offset)?;
    bytes(&mut path, &mut n, INCLUDES)?;
    bytes(&mut path, &mut n, TYPES)?;
    let mut u = [0; 8192];
    let body = fetch(api_url(&path[..n], &mut u)?, None)?;
    if text(body, b"type")? != b"collection" {
        return Err("invalid collection");
    }
    let (limit, got, total) = meta(body)?;
    if limit != 20 || got != offset {
        return Err("invalid list pagination");
    }
    let mut it = Array::new(required(body, b"data")?)?;
    let b = payload_buf();
    let mut n = 0;
    let mut count = 0;
    bytes(b, &mut n, b"{\"items\":[")?;
    while let Some(entity) = it.next()? {
        if count >= 20 {
            return Err("too many list entries");
        }
        if count > 0 {
            bytes(b, &mut n, b",")?;
        }
        manga(entity, quality, b, &mut n)?;
        count += 1;
    }
    if count != total.saturating_sub(offset).min(limit) {
        return Err("incomplete list response");
    }
    let more = offset.checked_add(limit).ok_or("invalid list pagination")? < total;
    bytes(b, &mut n, b"],\"page\":{\"nextCursor\":")?;
    if more {
        bytes(b, &mut n, b"\"")?;
        wn(b, &mut n, pn + 1)?;
        bytes(b, &mut n, b"\"")?;
    } else {
        bytes(b, &mut n, b"null")?;
    }
    bytes(
        b,
        &mut n,
        if more {
            b",\"hasMore\":true}}"
        } else {
            b",\"hasMore\":false}}"
        },
    )?;
    Ok(n)
}
fn detail(r: &[u8]) -> Result<usize, E> {
    let quality = cover_quality(r)?;
    let mid = manga_id(r)?;
    let mut path = [0; 1024];
    let mut n = 0;
    bytes(&mut path, &mut n, b"/title/")?;
    bytes(&mut path, &mut n, mid)?;
    bytes(&mut path, &mut n, b"?")?;
    bytes(&mut path, &mut n, &INCLUDES[1..])?;
    let mut u = [0; 2048];
    let body = fetch(api_url(&path[..n], &mut u)?, None)?;
    let e = required(body, b"data")?;
    if text(e, b"id")? != mid {
        return Err("title identity mismatch");
    }
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"manga\":")?;
    manga(e, quality, b, &mut n)?;
    bytes(b, &mut n, b"}")?;
    Ok(n)
}
#[derive(Clone, Copy)]
struct Record {
    start: usize,
    len: usize,
    accessible: bool,
}
const EMPTY: Record = Record {
    start: 0,
    len: 0,
    accessible: false,
};
static mut CHAPTER_BYTES: [u8; 1048576] = [0; 1048576];
static mut RECORDS: [Record; 8192] = [EMPTY; 8192];
fn record(r: Record, chbytes: &[u8]) -> &[u8] {
    &chbytes[r.start..r.start + r.len]
}
fn chapter_list_url<'a>(title: &[u8], offset: usize, b: &'a mut [u8]) -> Result<&'a [u8], E> {
    let mut n = 0;
    bytes(b, &mut n, API)?;
    bytes(b, &mut n, b"/chapter?titleId=")?;
    bytes(b, &mut n, title)?;
    bytes(b, &mut n, b"&includes%5B%5D=organization&limit=200&offset=")?;
    wn(b, &mut n, offset)?;
    bytes(
        b,
        &mut n,
        b"&translatedLanguages%5B%5D=zh-hant&order%5Bvolume%5D=desc&order%5Bchapter%5D=desc",
    )?;
    Ok(&b[..n])
}
fn gate(ids: &[&[u8]]) -> Result<&'static [u8], E> {
    if ids.is_empty() || ids.len() > 200 {
        return Err("invalid gating chunk");
    }
    let mut b = [0; 32768];
    let mut n = 0;
    bytes(&mut b, &mut n, b"{\"entities\":[")?;
    for (i, cid) in ids.iter().enumerate() {
        id(cid)?;
        if i > 0 {
            bytes(&mut b, &mut n, b",")?;
        }
        bytes(&mut b, &mut n, b"{\"entityId\":")?;
        q(&mut b, &mut n, cid)?;
        bytes(&mut b, &mut n, b",\"entityType\":\"chapter\"}")?;
    }
    bytes(&mut b, &mut n, b"]}")?;
    let body = fetch(b"https://api.namicomi.com/gating/check", Some(&b[..n]))?;
    let d = required(body, b"data")?;
    if text(d, b"type")? != b"entity_access_map" {
        return Err("invalid access response");
    }
    let map = required(required(d, b"attributes")?, b"map")?;
    for cid in ids {
        match required(map, cid)? {
            b"true" | b"false" => {}
            _ => return Err("nonboolean chapter access"),
        }
    }
    Ok(map)
}
fn chapter_number(v: Option<&[u8]>) -> Result<Option<&[u8]>, E> {
    let v = match v {
        None | Some(b"null") => return Ok(None),
        Some(v) => quoted(v)?,
    };
    if v.is_empty() {
        return Ok(None);
    }
    let mut seen = false;
    let mut dot = false;
    for &c in v {
        if c == b'.' && !dot {
            dot = true;
        } else if c.is_ascii_digit() {
            seen = true
        } else {
            return Ok(None);
        }
    }
    if !seen
        || v.first() == Some(&b'.')
        || v.last() == Some(&b'.')
        || v.len() > 16
        || (v.len() > 1 && v[0] == b'0' && v[1] != b'.')
    {
        return Ok(None);
    }
    Ok(Some(v))
}
fn exclusions_supported(r:&[u8])->bool {
    let Some(raw)=json_object_member(r,b"hostFeatures") else{return false};
    raw.starts_with(b"[")&&raw.ends_with(b"]")&&raw[1..raw.len()-1].split(|b|*b==b',').any(|v|v.trim_ascii()==br#""source_chapter_exclusions_v1""#)
}
fn show_locked_chapters(r:&[u8])->bool {
    exclusions_supported(r)&&matches!(json_object_member(r,b"settings").and_then(|v|json_object_member(v,b"showLockedChapters_zh-hant")),Some(b"true"|b"\"true\""))
}
fn emit_chapter(
    e: &[u8],
    title: &[u8],
    index: usize,
    locked: bool,
    b: &mut [u8],
    n: &mut usize,
) -> Result<(), E> {
    let cid = id(text(e, b"id")?)?;
    let a = required(e, b"attributes")?;
    let mut name = [0; 8192];
    let mut nn = 0;
    if locked {bytes(&mut name,&mut nn,"🔒 ".as_bytes())?;}
    for (key, prefix) in [
        (b"volume".as_slice(), b"Vol.".as_slice()),
        (b"chapter".as_slice(), b"Ch.".as_slice()),
    ] {
        if let Some(v) = member(a, key)? {
            if v != b"null" {
                let mut decoded = [0; 4096];
                let v = unescape(quoted(v)?, &mut decoded)?;
                if !trim_ascii(v).is_empty() {
                    if nn > 0 && !(locked && key == b"volume") {
                        bytes(&mut name, &mut nn, b" ")?;
                    }
                    bytes(&mut name, &mut nn, prefix)?;
                    bytes(&mut name, &mut nn, v)?;
                }
            }
        }
    }
    if let Some(v) = member(a, b"name")? {
        if v != b"null" {
            let mut decoded = [0; 8192];
            let v = unescape(quoted(v)?, &mut decoded)?;
            if !trim_ascii(v).is_empty() {
                if nn > 0 {
                    bytes(&mut name, &mut nn, b" - ")?;
                }
                bytes(&mut name, &mut nn, v)?;
            }
        }
    }
    // A published chapter may have null name/volume/chapter. Accessible chapters
    // keep an empty title for Host's existing unnamed label; locked ones retain
    // the reference lock prefix. Never infer a chapter number from missing data.
    bytes(b, n, b"{\"id\":")?;
    qid(b, n, CP, title, Some(cid))?;
    bytes(b, n, b",\"mangaId\":")?;
    qid(b, n, MP, title, None)?;
    bytes(b, n, b",\"title\":")?;
    q(b, n, &name[..nn])?;
    bytes(b, n, b",\"index\":")?;
    wn(b, n, index)?;
    for (out, k) in [
        (b",\"chapterNumber\":".as_slice(), b"chapter".as_slice()),
        (b",\"volumeNumber\":".as_slice(), b"volume".as_slice()),
    ] {
        bytes(b, n, out)?;
        match chapter_number(member(a, k)?)? {
            Some(v) => bytes(b, n, v)?,
            None => bytes(b, n, b"null")?,
        }
    }
    bytes(b, n, b",\"language\":\"zh-Hant\",\"publishedAt\":")?;
    let date = raw_string(a, b"publishAt")?;
    rawq(b, n, date)?;
    bytes(b, n, b",\"updatedAt\":null,\"pageCount\":")?;
    match member(a, b"pages")? {
        Some(v) if v != b"null" => {
            integer(v)?;
            bytes(b, n, v)?
        }
        _ => bytes(b, n, b"null")?,
    };
    bytes(b, n, b",\"scanlator\":null}")
}
fn chapters(r: &[u8]) -> Result<usize, E> {
    let title = manga_id(r)?;
    let supports=exclusions_supported(r);
    let show_locked=show_locked_chapters(r);
    let wanted=if supports&&member(args(r)?,b"chapterId")?.is_some(){
        let (owner,cid)=chapter_id(r)?;
        if owner!=title{return Err("chapter title mismatch");}
        Some(cid)
    }else{None};
    if page(r)? != 1 {
        return Err("invalid request");
    }
    let records = unsafe { &mut *core::ptr::addr_of_mut!(RECORDS) };
    let chbytes = unsafe { &mut *core::ptr::addr_of_mut!(CHAPTER_BYTES) };
    let mut used = 0;
    let mut count = 0;
    let mut offset = 0;
    let mut total = None;
    loop {
        let mut url = [0; 2048];
        let body = fetch(chapter_list_url(title, offset, &mut url)?, None)?;
        if text(body, b"type")? != b"collection" {
            return Err("invalid collection");
        }
        let (limit, got, nt) = meta(body)?;
        if limit == 0 || limit > 200 || got != offset || nt > records.len() {
            return Err("invalid catalogue pagination");
        }
        if let Some(t) = total {
            if t != nt {
                return Err("catalogue total changed");
            }
        } else {
            total = Some(nt);
        }
        let mut it = Array::new(required(body, b"data")?)?;
        let mut onpage = 0;
        while let Some(e) = it.next()? {
            if onpage >= limit || count >= records.len() {
                return Err("catalogue capacity exceeded");
            }
            if text(e, b"type")? != b"chapter" {
                return Err("invalid chapter entity");
            }
            let cid = id(text(e, b"id")?)?;
            let a = required(e, b"attributes")?;
            if text(a, b"translatedLanguage")? != b"zh-hant" {
                return Err("chapter language mismatch");
            }
            let mut related = false;
            let mut rels = Array::new(required(e, b"relationships")?)?;
            while let Some(rel) = rels.next()? {
                if text(rel, b"type")? == b"title" {
                    if text(rel, b"id")? != title {
                        return Err("chapter title mismatch");
                    }
                    related = true;
                }
            }
            if !related {
                return Err("chapter title relationship missing");
            }
            for rr in &records[..count] {
                if text(record(*rr, chbytes), b"id")? == cid {
                    return Err("duplicate chapter identity");
                }
            }
            let start = used;
            bytes(chbytes, &mut used, e)?;
            records[count] = Record {
                start,
                len: e.len(),
                accessible: false,
            };
            count += 1;
            onpage += 1;
        }
        if onpage != nt.saturating_sub(offset).min(limit) {
            return Err("incomplete catalogue page");
        }
        if offset + limit >= nt {
            break;
        }
        offset = offset
            .checked_add(limit)
            .ok_or("invalid catalogue pagination")?;
    }
    if count != total.unwrap_or(0) {
        return Err("incomplete catalogue");
    }
    for start in (0..count).step_by(200) {
        let end = (start + 200).min(count);
        let mut ids: [&[u8]; 200] = [b""; 200];
        for i in start..end {
            ids[i - start] = text(record(records[i], chbytes), b"id")?;
        }
        let map = gate(&ids[..end - start])?;
        for i in start..end {
            records[i].accessible = required(map, ids[i - start])? == b"true";
        }
    }
    let b = payload_buf();
    let mut n = 0;
    let mut emitted = 0;
    bytes(b, &mut n, b"{\"chapters\":[")?;
    // Complete owner/language/pagination and gating validation precedes visibility.
    // Visibility never removes retained Library/Reader data or changes page access.
    for rr in &records[..count] {
        let raw=record(*rr,chbytes);
        if let Some(cid)=wanted {
            if text(raw,b"id")?!=cid{continue;}
        }else if !rr.accessible&&!show_locked{continue;}
        if emitted > 0 {bytes(b, &mut n, b",")?;}
        emit_chapter(raw,title,emitted,!rr.accessible,b,&mut n)?;
        emitted += 1;
    }
    if wanted.is_some()&&emitted!=1{return Err("chapter ownership mismatch");}
    bytes(b,&mut n,b"]")?;
    if supports&&wanted.is_none(){
        bytes(b,&mut n,b",\"excludedChapterIds\":[")?;let mut excluded=0;
        if !show_locked {
            for rr in &records[..count] {
                if rr.accessible{continue;}
                if excluded>0{bytes(b,&mut n,b",")?;}
                qid(b,&mut n,CP,title,Some(text(record(*rr,chbytes),b"id")?))?;excluded+=1;
            }
        }
        bytes(b,&mut n,b"]")?;
    }
    bytes(b,&mut n,b",\"page\":{\"nextCursor\":null,\"hasMore\":false}}")?;
    Ok(n)
}
fn pages(r: &[u8]) -> Result<usize, E> {
    let saver = data_saver(r)?;
    let (title, cid) = chapter_id(r)?;
    let map = gate(&[cid])?;
    if required(map, cid)? != b"true" {
        return Err("payment or account required");
    }
    let mut path = [0; 1024];
    let mut n = 0;
    bytes(&mut path, &mut n, b"/images/chapter/")?;
    bytes(&mut path, &mut n, cid)?;
    bytes(&mut path, &mut n, b"?newQualities=true")?;
    let mut u = [0; 2048];
    let body = fetch(api_url(&path[..n], &mut u)?, None)?;
    let d = required(body, b"data")?;
    let mut base = [0; 512];
    let base = unescape(text(d, b"baseUrl")?, &mut base)?;
    if base != CDN {
        return Err("unpublished image host");
    }
    let hash = text(d, b"hash")?;
    if hash.len() != 32 || hash.iter().any(|c| !c.is_ascii_hexdigit()) {
        return Err("invalid image hash");
    }
    let mut it = Array::new(required(d, if saver { b"low" } else { b"source" })?)?;
    let b = payload_buf();
    let mut n = 0;
    let mut count = 0;
    bytes(b, &mut n, b"{\"chapterId\":")?;
    qid(b, &mut n, CP, title, Some(cid))?;
    bytes(b, &mut n, b",\"pages\":[")?;
    while let Some(e) = it.next()? {
        if count >= 8192 {
            return Err("too many pages");
        }
        let mut f = [0; 256];
        let f = unescape(text(e, b"filename")?, &mut f)?;
        filename(f)?;
        let mut url = [0; 1024];
        let mut un = 0;
        for s in [
            base,
            b"/chapter/".as_slice(),
            cid,
            b"/".as_slice(),
            hash,
            if saver { b"/low/".as_slice() } else { b"/source/".as_slice() },
            f,
        ] {
            bytes(&mut url, &mut un, s)?;
        }
        valid_image(&url[..un])?;
        if count > 0 {
            bytes(b, &mut n, b",")?;
        }
        bytes(b, &mut n, b"{\"id\":\"")?;
        bytes(b, &mut n, PP)?;
        bytes(b, &mut n, title)?;
        bytes(b, &mut n, b":")?;
        bytes(b, &mut n, cid)?;
        bytes(b, &mut n, b":")?;
        wn(b, &mut n, count)?;
        bytes(b, &mut n, b"\",\"index\":")?;
        wn(b, &mut n, count)?;
        bytes(b, &mut n, b",\"image\":{\"kind\":\"url\",\"url\":")?;
        q(b, &mut n, &url[..un])?;
        bytes(b, &mut n, b"}}")?;
        count += 1;
    }
    if count == 0 {
        return Err("empty page array");
    }
    bytes(b, &mut n, b"]}")?;
    Ok(n)
}
fn image_request(r: &[u8]) -> Result<usize, E> {
    let mut u = [0; 1024];
    let url = unescape(text(args(r)?, b"url")?, &mut u)?;
    valid_image(url)?;
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"url\":")?;
    q(b, &mut n, url)?;
    bytes(b,&mut n,b",\"headers\":{\"User-Agent\":\"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0\",\"Referer\":\"https://namicomi.com/\",\"Origin\":\"https://namicomi.com\"}}")?;
    Ok(n)
}
fn run_search(r: &[u8]) -> u32 {
    execute("search", list(r, true))
}
fn run_get_manga_list(r: &[u8]) -> u32 {
    execute("get_manga_list", list(r, false))
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
    let data="{\"listings\":[{\"id\":\"popular\",\"name\":\"热门漫画\",\"supportsPaging\":true},{\"id\":\"latest\",\"name\":\"最新漫画\",\"supportsPaging\":true}]}".as_bytes();
    payload_buf()[..data.len()].copy_from_slice(data);
    write_success_payload("get_listings", data.len())
}
fn run_get_home(_: &[u8]) -> u32 {
    write_error("get_home", "not_implemented", "not supported")
}
fn run_get_filters(_: &[u8]) -> u32 {
    write_error("get_filters", "not_implemented", "not supported")
}
fn run_get_settings(r: &[u8]) -> u32 {
    let data = r#"{"settings":[{"id":"thumbnailQuality_zh-hant","name":"封面品質","kind":"select","options":[{"value":"original","label":"原圖"},{"value":".512.jpg","label":"中等"},{"value":".256.jpg","label":"低"}],"default":"original"},{"id":"dataSaver_zh-hant","name":"省流量","kind":"toggle","default":"false"}]}"#.as_bytes();
    if !exclusions_supported(r){payload_buf()[..data.len()].copy_from_slice(data);return write_success_payload("get_settings",data.len());}
    let out=payload_buf();let mut n=0;
    let result=bytes(out,&mut n,&data[..data.len()-2]).and_then(|_|bytes(out,&mut n,br#",{"id":"showLockedChapters_zh-hant","name":"Show locked/paywalled chapters","description":"Display chapters that require an account with a premium subscription","kind":"toggle","default":"false"}]}"#));
    match result{Ok(())=>write_success_payload("get_settings",n),Err(e)=>write_error("get_settings","internal_error",e)}
}
koma_source_sdk::koma_source_exports!("namicomi-zh-hant");
