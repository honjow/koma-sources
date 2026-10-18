#![no_std]
// Public contract: Keiyoushi GlobalComix.kt and DTOs, Apache-2.0, revision
// 1319949ca9bc456908196e3ba2d104e94182c5d6. No authentication/mirror/JS processing.
use koma_source_sdk::{
    host,
    json_utils::*,
    source::{SourceCapabilities, SourceInfo},
};
const BASE: &[u8] = b"https://globalcomix.com";
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.globalcomix.zh-hans.koma",
    name: "GlobalComix",
    version: "0.1.1",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description:
        "GlobalComix Chinese public JSON; upstream MIXED; only explicit free releases/pages. Usage unknown.",
    content_rating: "nsfw",
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
    settings: false,
    image_request: true,
    credentials: false,
};
koma_source_sdk::koma_source_buffers! { payload:1024*1024,http_out:2*1024*1024,body:1024*1024,http_req:8192,scratch:8192, }
koma_source_sdk::koma_source_helpers!();
const API: &[u8] = b"https://api.globalcomix.com/v1";
const CLIENT: &[u8] = b"gck_d0f170d5729446dcb3b55e6b3ebc7bf6";
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
    while p < s.len() && s[p].is_ascii_whitespace() {
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
                        hex4(s.get(i + 1..i + 5)?)?;
                        i += 4;
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
fn args(r: &[u8]) -> &[u8] {
    json_object_member(r, b"args").unwrap_or(r)
}
fn page(r: &[u8]) -> Result<usize, &'static str> {
    let a = args(r);
    let raw = match json_object_member(a, b"page") {
        None => json_object_member(a, b"cursor").unwrap_or(b"null"),
        Some(v) => v,
    };
    let v = if raw.first() == Some(&b'{') {
        json_object_member(raw, b"cursor").unwrap_or(b"null")
    } else {
        raw
    };
    if v == b"null" {
        return Ok(1);
    };
    let v = if v.first() == Some(&b'"') && v.last() == Some(&b'"') {
        &v[1..v.len() - 1]
    } else {
        v
    };
    number(v).filter(|n| *n > 0).ok_or("invalid page cursor")
}
fn string<'a>(obj: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let v = json_object_member(obj, key)?;
    if v.len() < 2 || v[0] != b'"' || *v.last()? != b'"' {
        None
    } else {
        Some(&v[1..v.len() - 1])
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

fn diagnostic(stage: &[u8], code: &[u8]) {
    let mut out = [0; 160];
    let mut n = 0;
    if write_bytes(&mut out, &mut n, b"globalcomix_zh_hans stage=")
        && write_bytes(&mut out, &mut n, stage)
        && write_bytes(&mut out, &mut n, b" code=")
        && write_bytes(&mut out, &mut n, code)
    {
        host::log_info(&out[..n]);
    }
}
fn code(e: &str) -> &'static str {
    match e {
        "permission denied" => "permission_denied",
        "not found" => "not_found",
        "rate limited" => "rate_limit",
        "network failure" => "network_error",
        "timeout" => "timeout",
        "client failure" => "client_error",
        "server failure" => "server_error",
        "invalid request"
        | "invalid page cursor"
        | "missing query"
        | "empty query"
        | "unknown listing"
        | "missing image URL"
        | "invalid image URL" => "invalid_request",
        "payload overflow" => "internal_error",
        _ => "parse_error",
    }
}
fn execute(op: &str, r: Result<usize, &'static str>) -> u32 {
    match r {
        Ok(n) => {
            diagnostic(b"operation_ok", op.as_bytes());
            write_success_payload(op, n)
        }
        Err(e) => {
            let c = code(e);
            diagnostic(b"operation_error", c.as_bytes());
            write_error(op, c, e)
        }
    }
}
fn valid_status(s: &[u8]) -> Option<usize> {
    number(s).filter(|n| (100..=599).contains(n))
}
static mut BODY_LEN: usize = 0;
fn fetch(base: &[u8], path: &[u8]) -> Result<&'static [u8], &'static str> {
    host::log_info(b"globalcomix_zh_hans stage=fetch_begin");
    let mut url = [0; 8192];
    let mut n = 0;
    bytes(&mut url, &mut n, base)?;
    bytes(&mut url, &mut n, path)?;
    let len = koma_source_sdk::build_get_request(
        http_req_buf(),
        &url[..n],
        Some(b"https://globalcomix.com/"),
        &[
            (b"Origin", BASE),
            (b"x-gc-client", CLIENT),
            (b"x-gc-identmode", b"cookie"),
        ],
    )
    .ok_or_else(|| {
        diagnostic(b"request_build", b"network_error");
        "network failure"
    })?;
    let len = host::http_request(&http_req_buf()[..len], http_out()).map_err(|e| {
        diagnostic(
            b"http_return",
            match e {
                -1 => b"-1",
                -2 => b"-2",
                -3 => b"-3",
                _ => b"other",
            },
        );
        "network failure"
    })?;
    let response = unsafe { core::slice::from_raw_parts(HTTP_OUT.as_ptr(), len) };
    if json_value_end(response, 0, 0).map(|e| ws(response, e)) != Some(response.len()) {
        return Err("invalid HTTP envelope");
    }
    let status = json_object_member(response, b"status")
        .and_then(valid_status)
        .or_else(|| json_object_member(response, b"statusCode").and_then(valid_status));
    let mut out = [0; 80];
    let mut n = 0;
    if write_bytes(
        &mut out,
        &mut n,
        b"globalcomix_zh_hans stage=http_status value=",
    ) && write_usize(&mut out, &mut n, status.unwrap_or(0))
    {
        host::log_info(&out[..n]);
    }
    match status {
        Some(200..=299) => {}
        Some(401 | 403) => return Err("permission denied"),
        Some(404) => return Err("not found"),
        Some(429) => return Err("rate limited"),
        Some(400..=499) => return Err("client failure"),
        Some(500..=599) => return Err("server failure"),
        Some(_) => return Err("unexpected HTTP status"),
        None => {
            if json_object_member(response, b"ok") == Some(b"false".as_slice()) {
                let c = match string(
                    json_object_member(response, b"error").unwrap_or(response),
                    b"code",
                ) {
                    Some(b"timeout") => "timeout",
                    Some(b"network_error")
                    | Some(b"transport_unavailable")
                    | Some(b"transport_invalid_response") => "network failure",
                    _ => "invalid HTTP envelope",
                };
                diagnostic(b"transport", code(c).as_bytes());
                return Err(c);
            }
            return Err("invalid HTTP envelope");
        }
    }
    let n = koma_source_sdk::decode_json_body_into(response, body_buf())
        .map_err(|_| "HTTP body invalid or too large")?;
    if n == 0 {
        return Err("empty HTTP body");
    }
    core::str::from_utf8(&body_buf()[..n]).map_err(|_| "invalid UTF8")?;
    unsafe {
        BODY_LEN = n;
    }
    host::log_info(b"globalcomix_zh_hans stage=body_ready");
    json_complete(&body_buf()[..n])?;
    let result = unsafe { core::slice::from_raw_parts(body_buf().as_ptr(), n) };
    if json_object_member(result, b"error").ok_or("missing API error")? != b"null" {
        return Err("API failure");
    }
    let payload = json_object_member(result, b"payload").ok_or("missing API payload")?;
    json_object_member(payload, b"results").ok_or("missing API results")
}
fn valid_uri(u: &[u8]) -> Result<(), &'static str> {
    core::str::from_utf8(u).map_err(|_| "invalid UTF8")?;
    if u.is_empty() || u.len() >= 8192 || u.iter().any(|c| *c <= 32 || *c == 127 || *c == b'\\') {
        return Err("invalid path");
    }
    let mut i = 0;
    while i < u.len() {
        if u[i] == b'%' {
            let h = u.get(i + 1..i + 3).ok_or("invalid path")?;
            if !h.iter().all(|c| c.is_ascii_hexdigit()) {
                return Err("invalid path");
            }
            let digit = |c: u8| -> u8 {
                match c {
                    b'0'..=b'9' => c - b'0',
                    b'a'..=b'f' => c - b'a' + 10,
                    _ => c - b'A' + 10,
                }
            };
            let decoded = digit(h[0]) * 16 + digit(h[1]);
            if decoded < 32 || decoded == 127 || decoded == b'\\' {
                return Err("invalid path");
            }
            i += 2;
        }
        i += 1;
    }
    Ok(())
}
fn json_complete(s: &[u8]) -> Result<(), &'static str> {
    let start = ws(s, 0);
    if json_value_end(s, start, 0).map(|e| ws(s, e)) != Some(s.len()) {
        return Err("incomplete JSON");
    }
    Ok(())
}
fn array_spans<'a>(s: &'a [u8], out: &mut [(usize, usize)]) -> Result<usize, &'static str> {
    json_complete(s)?;
    if s.first() != Some(&b'[') {
        return Err("expected array");
    };
    let mut i = ws(s, 1);
    let mut n = 0;
    while s.get(i) != Some(&b']') {
        if n == out.len() {
            return Err("too many items");
        };
        let e = json_value_end(s, i, 0).ok_or("invalid JSON item")?;
        out[n] = (i, e);
        n += 1;
        i = ws(s, e);
        if s.get(i) == Some(&b',') {
            i = ws(s, i + 1)
        } else if s.get(i) != Some(&b']') {
            return Err("invalid JSON array");
        }
    }
    Ok(n)
}
fn member<'a>(o: &'a [u8], k: &[u8]) -> Result<&'a [u8], &'static str> {
    json_object_member(o, k).ok_or("missing required field")
}
fn text<'a>(o: &[u8], k: &[u8], b: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    unescape(string(o, k).ok_or("missing required string")?, b)
}
fn uint(o: &[u8], k: &[u8]) -> Result<usize, &'static str> {
    number(member(o, k)?).ok_or("invalid integer")
}
fn is(o: &[u8], k: &[u8], v: &[u8]) -> bool {
    member(o, k) == Ok(v)
}
fn write_num(b: &mut [u8], n: &mut usize, v: usize) -> Result<(), &'static str> {
    if write_usize(b, n, v) {
        Ok(())
    } else {
        Err("payload overflow")
    }
}
fn slug_valid(s: &[u8]) -> bool {
    !s.is_empty()
        && s.len() <= 256
        && s.iter()
            .all(|c| c.is_ascii_alphanumeric() || *c == b'-' || *c == b'_')
}
fn mid_parts(v: &[u8]) -> Result<(usize, &[u8]), &'static str> {
    let s = v.strip_prefix(b"gc:").ok_or("invalid mangaId")?;
    let i = s.iter().position(|c| *c == b':').ok_or("invalid mangaId")?;
    let id = number(&s[..i])
        .filter(|n| *n > 0)
        .ok_or("invalid mangaId")?;
    let slug = &s[i + 1..];
    if !slug_valid(slug) {
        return Err("invalid mangaId");
    }
    Ok((id, slug))
}
fn make_mid<'a>(o: &[u8], b: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    let id = uint(o, b"id")?;
    if id == 0 {
        return Err("invalid manga identity");
    }
    let mut sb = [0u8; 512];
    let slug = text(o, b"slug", &mut sb)?;
    if !slug_valid(slug) {
        return Err("invalid manga identity");
    }
    let mut n = 0;
    bytes(b, &mut n, b"gc:")?;
    write_num(b, &mut n, id)?;
    bytes(b, &mut n, b":")?;
    bytes(b, &mut n, slug)?;
    Ok(&b[..n])
}
fn input_mid(r: &[u8], b: &mut [u8]) -> Result<usize, &'static str> {
    let id = text(args(r), b"mangaId", b)?;
    mid_parts(id)?;
    Ok(id.len())
}
fn uuid_valid(v: &[u8]) -> bool {
    v.len() == 36
        && v.iter().enumerate().all(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) {
                *c == b'-'
            } else {
                c.is_ascii_hexdigit()
            }
        })
}
fn chapter_parts(v: &[u8]) -> Result<(usize, usize, &[u8]), &'static str> {
    let s = v.strip_prefix(b"gcr:").ok_or("invalid chapterId")?;
    let i = s
        .iter()
        .position(|c| *c == b':')
        .ok_or("invalid chapterId")?;
    let c = number(&s[..i])
        .filter(|x| *x > 0)
        .ok_or("invalid chapterId")?;
    let s = &s[i + 1..];
    let i = s
        .iter()
        .position(|c| *c == b':')
        .ok_or("invalid chapterId")?;
    let r = number(&s[..i])
        .filter(|x| *x > 0)
        .ok_or("invalid chapterId")?;
    let key = &s[i + 1..];
    if !uuid_valid(key) {
        return Err("invalid chapterId");
    }
    Ok((c, r, key))
}
fn image_valid(u: &[u8]) -> Result<(), &'static str> {
    valid_uri(u)?;
    if contains_bytes(u, b"#") {
        return Err("invalid image URL");
    }
    if ![
        b"https://globalcomix.com/".as_slice(),
        b"https://globalcomix-comics-assets-files-desktop.nyc3.cdn.digitaloceanspaces.com/",
    ]
    .iter()
    .any(|host| u.starts_with(host))
    {
        return Err("invalid image URL");
    }
    Ok(())
}
fn optional_text<'a>(o: &[u8], k: &[u8], b: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    match json_object_member(o, k) {
        None | Some(b"null") => Ok(b""),
        _ => text(o, k, b),
    }
}
fn status(v: &[u8]) -> &'static [u8] {
    match v {
        b"Ongoing" | b"Preview" => b"ongoing",
        b"Finished" => b"completed",
        b"On hold" => b"hiatus",
        b"Cancelled" => b"cancelled",
        _ => b"unknown",
    }
}
fn emit_manga(
    o: &[u8],
    language: Option<&[u8]>,
    out: &mut [u8],
    n: &mut usize,
) -> Result<(), &'static str> {
    if !is(o, b"entity_type", b"\"Comic\"") {
        return Err("invalid manga entity");
    }
    let mut idb = [0; 512];
    let mid = make_mid(o, &mut idb)?;
    let mut nb = [0; 4096];
    let name = text(language.unwrap_or(o), b"name", &mut nb)?;
    if name.is_empty() {
        return Err("empty title");
    }
    let mut cb = [0; 8192];
    let cover = text(o, b"image_url", &mut cb)?;
    image_valid(cover)?;
    bytes(out, n, b"{\"id\":")?;
    q(out, n, mid)?;
    bytes(out, n, b",\"title\":")?;
    q(out, n, name)?;
    bytes(out, n, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(out, n, cover)?;
    bytes(
        out,
        n,
        b"},\"language\":\"zh\",\"contentRating\":\"unknown\",\"authors\":[",
    )?;
    let artist = member(o, b"artist")?;
    let mut ab = [0; 4096];
    let roman = optional_text(artist, b"roman_name", &mut ab)?;
    if !roman.is_empty() {
        q(out, n, roman)?;
    } else {
        let mut b = [0; 4096];
        q(out, n, text(artist, b"name", &mut b)?)?;
    }
    bytes(out, n, b"],\"tags\":[")?;
    let mut tb = [0; 2048];
    let category = optional_text(o, b"category_name", &mut tb)?;
    if !category.is_empty() {
        q(out, n, category)?;
    }
    bytes(out, n, b"],\"status\":")?;
    let mut sb = [0; 128];
    q(out, n, status(optional_text(o, b"status_name", &mut sb)?))?;
    if let Some(lang) = language {
        let mut db = [0; 16384];
        bytes(out, n, b",\"description\":")?;
        q(out, n, optional_text(lang, b"description", &mut db)?)?;
    }
    bytes(out, n, b"}")
}
fn pagination<'a>(body: &'a [u8]) -> Result<&'a [u8], &'static str> {
    let p = member(body, b"payload")?;
    member(p, b"pagination")
}
fn list(r: &[u8], search: bool) -> Result<usize, &'static str> {
    let p = page(r)?;
    if p > 10000 {
        return Err("invalid page cursor");
    }
    let mut path = [0; 8192];
    let mut n = 0;
    bytes(&mut path, &mut n, b"/comics?lang_id%5B%5D=cn&p=")?;
    write_num(&mut path, &mut n, p)?;
    if search {
        bytes(&mut path, &mut n, b"&sort=relevance&q=")?;
        let mut qb = [0; 2048];
        let qv = text(args(r), b"query", &mut qb)?;
        if qv.is_empty() {
            return Err("empty query");
        }
        if !write_url_encoded(&mut path, &mut n, qv) {
            return Err("payload overflow");
        }
    } else {
        match string(args(r), b"listingId").unwrap_or(b"popular") {
            b"popular" => {}
            b"latest" => bytes(&mut path, &mut n, b"&sort=recent")?,
            _ => return Err("unknown listing"),
        }
    }
    let a = fetch(API, &path[..n])?;
    let mut spans = [(0, 0); 2048];
    let count = array_spans(a, &mut spans)?;
    let raw = unsafe { core::slice::from_raw_parts(body_buf().as_ptr(), BODY_LEN) };
    let pg = pagination(raw)?;
    if uint(pg, b"page")? != p {
        return Err("pagination mismatch");
    }
    let total = uint(pg, b"total_pages")?;
    let per = uint(pg, b"per_page")?;
    if total == 0 || count == 0 || count > per || p > total {
        return Err("empty or invalid listing");
    }
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"items\":[")?;
    for (i, (s, e)) in spans[..count].iter().enumerate() {
        if i > 0 {
            bytes(out, &mut n, b",")?;
        }
        emit_manga(&a[*s..*e], None, out, &mut n)?;
    }
    bytes(out, &mut n, b"],\"page\":{\"nextCursor\":")?;
    if p < total {
        let mut b = [0; 32];
        let mut bn = 0;
        write_num(&mut b, &mut bn, p + 1)?;
        q(out, &mut n, &b[..bn])?;
        bytes(out, &mut n, b",\"hasMore\":true}}")?;
    } else {
        bytes(out, &mut n, b"null,\"hasMore\":false}}")?;
    }
    Ok(n)
}
fn detail(r: &[u8]) -> Result<usize, &'static str> {
    let mut ib = [0; 512];
    let idn = input_mid(r, &mut ib)?;
    let (id, slug) = mid_parts(&ib[..idn])?;
    let mut path = [0; 1024];
    let mut pn = 0;
    bytes(&mut path, &mut pn, b"/read/")?;
    bytes(&mut path, &mut pn, slug)?;
    let o = fetch(API, &path[..pn])?;
    if uint(o, b"id")? != id {
        return Err("manga owner mismatch");
    }
    let mut sb = [0; 512];
    if text(o, b"slug", &mut sb)? != slug {
        return Err("manga owner mismatch");
    }
    let langs = member(o, b"comic_langs")?;
    let mut spans = [(0, 0); 128];
    let count = array_spans(langs, &mut spans)?;
    let mut found = None;
    for (s, e) in &spans[..count] {
        let l = &langs[*s..*e];
        if is(l, b"lang_id", b"\"cn\"") {
            if found.is_some() || uint(l, b"comic_id")? != id {
                return Err("language owner mismatch");
            }
            found = Some(l);
        }
    }
    let selected = found.ok_or("Chinese language missing")?;
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"manga\":")?;
    emit_manga(o, Some(selected), out, &mut n)?;
    bytes(out, &mut n, b"}")?;
    Ok(n)
}
fn free_release(o: &[u8], comic: usize) -> Result<(), &'static str> {
    if !is(o, b"entity_type", b"\"Release\"")
        || uint(o, b"comic_id")? != comic
        || !is(o, b"lang_id", b"\"cn\"")
    {
        return Err("release owner mismatch");
    }
    if member(o, b"premium_only")? != b"0" || member(o, b"is_free")? != b"true" {
        return Err("permission denied");
    }
    Ok(())
}
fn chapters(r: &[u8]) -> Result<usize, &'static str> {
    let mut ib = [0; 512];
    let len = input_mid(r, &mut ib)?;
    let mid = &ib[..len];
    let (comic, _) = mid_parts(mid)?;
    let mut path = [0; 1024];
    let mut n = 0;
    bytes(&mut path, &mut n, b"/comics/")?;
    write_num(&mut path, &mut n, comic)?;
    bytes(&mut path, &mut n, b"/releases?lang_id=cn&all=true")?;
    let a = fetch(API, &path[..n])?;
    let mut spans = [(0, 0); 2048];
    let count = array_spans(a, &mut spans)?;
    let raw = unsafe { core::slice::from_raw_parts(body_buf().as_ptr(), BODY_LEN) };
    let pg = pagination(raw)?;
    if uint(pg, b"page")? != 1
        || uint(pg, b"total_pages")? != 1
        || uint(pg, b"total_results")? != count
        || count == 0
    {
        return Err("incomplete catalogue");
    }
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"chapters\":[")?;
    let mut emitted = 0;
    for (s, e) in &spans[..count] {
        let o = &a[*s..*e];
        if !is(o, b"entity_type", b"\"Release\"")
            || uint(o, b"comic_id")? != comic
            || !is(o, b"lang_id", b"\"cn\"")
        {
            return Err("release owner mismatch");
        }
        let premium = member(o, b"premium_only")?;
        let free = member(o, b"is_free")?;
        if premium == b"1" || free == b"false" {
            continue;
        }
        free_release(o, comic)?;
        let rid = uint(o, b"id")?;
        if rid == 0 {
            return Err("invalid release identity");
        }
        let mut kb = [0; 64];
        let key = text(o, b"key", &mut kb)?;
        if !uuid_valid(key) {
            return Err("invalid release identity");
        }
        let mut cid = [0; 256];
        let mut cn = 0;
        bytes(&mut cid, &mut cn, b"gcr:")?;
        write_num(&mut cid, &mut cn, comic)?;
        bytes(&mut cid, &mut cn, b":")?;
        write_num(&mut cid, &mut cn, rid)?;
        bytes(&mut cid, &mut cn, b":")?;
        bytes(&mut cid, &mut cn, key)?;
        let mut tb = [0; 4096];
        let title = text(o, b"title", &mut tb)?;
        if title.is_empty() {
            return Err("empty chapter title");
        }
        if emitted > 0 {
            bytes(out, &mut n, b",")?;
        }
        bytes(out, &mut n, b"{\"id\":")?;
        q(out, &mut n, &cid[..cn])?;
        bytes(out, &mut n, b",\"mangaId\":")?;
        q(out, &mut n, mid)?;
        bytes(out, &mut n, b",\"title\":")?;
        q(out, &mut n, title)?;
        bytes(out, &mut n, b",\"chapterNumber\":")?;
        let mut num = [0; 128];
        let value = text(o, b"chapter", &mut num)?;
        if let Some(v) = number(value) {
            write_num(out, &mut n, v)?;
        } else {
            bytes(out, &mut n, b"null")?;
        }
        bytes(
            out,
            &mut n,
            b",\"volumeNumber\":null,\"language\":\"zh\",\"pageCount\":",
        )?;
        write_num(out, &mut n, uint(o, b"page_count")?)?;
        bytes(out, &mut n, b",\"index\":")?;
        write_num(out, &mut n, emitted)?;
        bytes(out, &mut n, b"}")?;
        emitted += 1;
    }
    if emitted == 0 {
        return Err("permission denied");
    }
    bytes(
        out,
        &mut n,
        b"],\"page\":{\"nextCursor\":null,\"hasMore\":false}}",
    )?;
    Ok(n)
}
fn pages(r: &[u8]) -> Result<usize, &'static str> {
    let mut cb = [0; 256];
    let cp = text(args(r), b"chapterId", &mut cb)?;
    let (comic, rid, key) = chapter_parts(cp)?;
    let mut path = [0; 256];
    let mut pn = 0;
    bytes(&mut path, &mut pn, b"/readV2/")?;
    bytes(&mut path, &mut pn, key)?;
    let o = fetch(API, &path[..pn])?;
    free_release(o, comic)?;
    let mut kb = [0; 64];
    if uint(o, b"id")? != rid || text(o, b"key", &mut kb)? != key {
        return Err("release owner mismatch");
    }
    let a = member(o, b"page_objects")?;
    let mut spans = [(0, 0); 2048];
    let count = array_spans(a, &mut spans)?;
    if count == 0 || count != uint(o, b"page_count")? {
        return Err("incomplete pages");
    }
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"chapterId\":")?;
    q(out, &mut n, cp)?;
    bytes(out, &mut n, b",\"pages\":[")?;
    let mut seen = [0usize; 2048];
    for (i, (s, e)) in spans[..count].iter().enumerate() {
        let item = &a[*s..*e];
        if !is(item, b"entity_type", b"\"ReleasePage\"")
            || uint(item, b"comic_id")? != comic
            || uint(item, b"release_id")? != rid
            || !is(item, b"lang_id", b"\"cn\"")
        {
            return Err("page owner mismatch");
        }
        if member(item, b"is_page_paid")? != b"false" {
            return Err("permission denied");
        }
        let pageid = uint(item, b"id")?;
        if pageid == 0 || seen[..i].contains(&pageid) {
            return Err("invalid page identity");
        }
        seen[i] = pageid;
        let mut ub = [0; 8192];
        let u = text(item, b"desktop_image_url", &mut ub)?;
        image_valid(u)?;
        let mut pid = [0; 320];
        let mut j = 0;
        bytes(&mut pid, &mut j, b"gcp:")?;
        write_num(&mut pid, &mut j, comic)?;
        bytes(&mut pid, &mut j, b":")?;
        write_num(&mut pid, &mut j, rid)?;
        bytes(&mut pid, &mut j, b":")?;
        write_num(&mut pid, &mut j, pageid)?;
        if i > 0 {
            bytes(out, &mut n, b",")?;
        }
        bytes(out, &mut n, b"{\"id\":")?;
        q(out, &mut n, &pid[..j])?;
        bytes(out, &mut n, b",\"index\":")?;
        write_num(out, &mut n, i)?;
        bytes(out, &mut n, b",\"image\":{\"kind\":\"url\",\"url\":")?;
        q(out, &mut n, u)?;
        bytes(out, &mut n, b"}}")?;
    }
    bytes(out, &mut n, b"]}")?;
    Ok(n)
}
fn image_request(r: &[u8]) -> Result<usize, &'static str> {
    let mut ub = [0; 8192];
    let u = text(args(r), b"url", &mut ub)?;
    image_valid(u)?;
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"url\":")?;
    q(out, &mut n, u)?;
    bytes(out,&mut n,b",\"headers\":{\"User-Agent\":\"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0\",\"Referer\":\"https://globalcomix.com/\",\"Origin\":\"https://globalcomix.com\",\"x-gc-client\":")?;
    q(out, &mut n, CLIENT)?;
    bytes(out, &mut n, b",\"x-gc-identmode\":\"cookie\"}}")?;
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
    let d="{\"listings\":[{\"id\":\"popular\",\"name\":\"热门\",\"kind\":\"popular\",\"supportsPaging\":true},{\"id\":\"latest\",\"name\":\"最新\",\"kind\":\"latest\",\"supportsPaging\":true}]}".as_bytes();
    payload_buf()[..d.len()].copy_from_slice(d);
    write_success_payload("get_listings", d.len())
}
fn run_get_home(_: &[u8]) -> u32 {
    write_error("get_home", "not_implemented", "not supported")
}
fn run_get_filters(_: &[u8]) -> u32 {
    write_error("get_filters", "not_implemented", "not supported")
}
fn run_get_settings(_: &[u8]) -> u32 {
    write_error("get_settings", "not_implemented", "not supported")
}
koma_source_sdk::koma_source_exports!("globalcomix_zh_hans");
