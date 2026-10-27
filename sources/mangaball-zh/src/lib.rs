#![no_std]
extern crate alloc;
use alloc::{string::String, vec::Vec};
// Full Unicode lowercase (including contextual sigma) follows the adopted
// reference. Reuse Rust alloc rather than approximate per-character folding.
#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;
// Public contract: Keiyoushi MangaBall.kt and Dto.kt, Apache-2.0, revision
// 1319949ca9bc456908196e3ba2d104e94182c5d6. No authentication/mirror/JS processing.
use koma_source_sdk::{
    host,
    json_utils::*,
    source::{SourceCapabilities, SourceInfo},
};
const BASE: &[u8] = b"https://mangaball.com";
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.mangaball.zh.koma",
    name: "Manga Ball",
    version: "0.1.2",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description:
        "提供 Manga Ball 的中文章节。",
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
    settings: true,
    image_request: true,
    credentials: false,
};
koma_source_sdk::koma_source_buffers! { payload:1024*1024,http_out:2*1024*1024,body:1024*1024,http_req:8192,scratch:8192, }
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
const NSFW_KEY: &[u8] = b"nsfw_pref";
const BLACKLIST_KEY: &[u8] = b"scanlator_blacklist_pref";
const KNOWN_KEY: &[u8] = b"known_scanlators_pref";
fn host_feature(r: &[u8], feature: &[u8]) -> bool {
    json_object_member(r,b"hostFeatures").map(|v| {
        let v=trim_ascii(v);
        v.starts_with(b"[") && v.ends_with(b"]") && v[1..v.len()-1].split(|c| *c==b',')
            .map(trim_ascii).any(|v|v.len()==feature.len()+2 && v.first()==Some(&b'"') && v.last()==Some(&b'"') && &v[1..v.len()-1]==feature)
    }).unwrap_or(false)
}
fn observations_supported(r: &[u8]) -> bool {host_feature(r,b"source_setting_observations_v1")}
fn blacklist_supported(r: &[u8]) -> bool {
    observations_supported(r) && host_feature(r,b"source_chapter_exclusions_v1")
}
fn setting<'a>(r: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    json_object_member(r,b"settings").and_then(|v|json_object_member(v,key))
}
fn hide_nsfw(r: &[u8]) -> bool {
    // Explicit human adoption preserves Koma's existing no_18 default. Only
    // false opts into the reference all request; unknown adult flags still fail.
    !matches!(setting(r,NSFW_KEY),Some(b"false" | b"\"false\""))
}
fn public_name(value: &[u8]) -> Result<String, &'static str> {
    let value=core::str::from_utf8(value).map_err(|_|"invalid scanlator UTF8")?;
    if value.encode_utf16().count()>256 || value.chars().any(|c|c<' ' || c=='\u{7f}') {
        return Err("invalid scanlator name");
    }
    Ok(String::from(value))
}
fn names_array(raw: &[u8]) -> Result<Vec<String>, &'static str> {
    let mut spans=[(0,0);512];let count=array_spans(raw,&mut spans)?;let mut names=Vec::new();
    for (a,z) in &spans[..count] {
        let v=&raw[*a..*z];if v.first()!=Some(&b'"') || v.last()!=Some(&b'"') {return Err("invalid scanlator setting");}
        let mut buf=[0;1024];let name=public_name(unescape(&v[1..v.len()-1],&mut buf)?)?;
        if !name.trim().is_empty() && !names.contains(&name) {names.push(name);}
    }
    Ok(names)
}
fn scanlator_blacklist(r: &[u8]) -> Result<Vec<String>, &'static str> {
    if !blacklist_supported(r) {return Ok(Vec::new());}
    let names=names_array(setting(r,BLACKLIST_KEY).unwrap_or(b"[]"))?;
    Ok(names.iter().map(|v|v.trim().to_lowercase()).collect())
}
fn chapter_scanlator(o: &[u8]) -> Result<Option<String>, &'static str> {
    let Some(group)=json_object_member(o,b"group") else {return Ok(None);};
    if group==b"null" {return Ok(None);}
    let mut buf=[0;1024];let name=public_name(text(group,b"name",&mut buf)?)?;
    Ok(Some(name))
}
fn write_names(out: &mut [u8], n: &mut usize, names: &[String]) -> Result<(), &'static str> {
    bytes(out,n,b"[")?;
    for (i,name) in names.iter().enumerate() {if i>0 {bytes(out,n,b",")?;}q(out,n,name.as_bytes())?;}
    bytes(out,n,b"]")
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
    if write_bytes(&mut out, &mut n, b"mangaball_zh stage=")
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
fn fetch(path: &[u8], post: Option<&[u8]>) -> Result<&'static [u8], &'static str> {
    host::log_info(b"mangaball_zh stage=fetch_begin");
    let mut url = [0; 8192];
    let mut n = 0;
    bytes(&mut url, &mut n, BASE)?;
    bytes(&mut url, &mut n, path)?;
    let mut rn = 0;
    let request = http_req_buf();
    bytes(request, &mut rn, b"{\"version\":1,\"method\":")?;
    q(
        request,
        &mut rn,
        if post.is_some() { b"POST" } else { b"GET" },
    )?;
    bytes(request, &mut rn, b",\"url\":")?;
    q(request, &mut rn, &url[..n])?;
    bytes(request,&mut rn,b",\"headers\":{\"User-Agent\":\"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0\",\"Referer\":\"https://mangaball.com/\",\"Origin\":\"https://mangaball.com\"")?;
    if post.is_some() {
        bytes(request, &mut rn, b",\"Content-Type\":\"application/json\"")?;
    }
    bytes(request, &mut rn, b"}")?;
    if let Some(body) = post {
        bytes(request, &mut rn, b",\"bodyBase64\":")?;
        q(request, &mut rn, body)?;
    }
    bytes(
        request,
        &mut rn,
        b",\"timeoutMs\":15000,\"responseKind\":\"bodyText\"}",
    )?;
    let len = rn;
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
    if write_bytes(&mut out, &mut n, b"mangaball_zh stage=http_status value=")
        && write_usize(&mut out, &mut n, status.unwrap_or(0))
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
    host::log_info(b"mangaball_zh stage=body_ready");
    json_complete(&body_buf()[..n])?;
    let result = unsafe { core::slice::from_raw_parts(body_buf().as_ptr(), n) };
    if string(result, b"status") != Some(b"success") {
        return Err("API failure");
    }
    if let Some(code) = json_object_member(result, b"code") {
        if code != b"200" {
            return Err("API failure");
        }
    }
    member(result, b"data")?;
    Ok(result)
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
fn write_num(b: &mut [u8], n: &mut usize, v: usize) -> Result<(), &'static str> {
    if write_usize(b, n, v) {
        Ok(())
    } else {
        Err("payload overflow")
    }
}
fn id_valid(v: &[u8]) -> bool {
    v.len() == 24
        && v.iter()
            .all(|c| c.is_ascii_digit() || matches!(c, b'a'..=b'f'))
}
fn slug_valid(v: &[u8]) -> bool {
    !v.is_empty() && v.len() < 256 && v.iter().all(|c| c.is_ascii_alphanumeric() || *c == b'-')
}
fn input_mid<'a>(r: &[u8], out: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    let v = text(args(r), b"mangaId", out)?;
    let id = v.strip_prefix(b"mb:").ok_or("invalid request")?;
    if !id_valid(id) {
        return Err("invalid request");
    }
    Ok(id)
}
fn input_ch<'a>(r: &[u8], out: &'a mut [u8]) -> Result<(&'a [u8], &'a [u8]), &'static str> {
    let v = text(args(r), b"chapterId", out)?;
    let v = v.strip_prefix(b"mbc:").ok_or("invalid request")?;
    let i = v.iter().position(|c| *c == b':').ok_or("invalid request")?;
    let (m, c) = (&v[..i], &v[i + 1..]);
    if !id_valid(m) || !id_valid(c) {
        return Err("invalid request");
    }
    Ok((m, c))
}
fn zh(v: &[u8]) -> bool {
    matches!(
        v,
        b"zh" | b"zh-cn" | b"zh-hk" | b"zh-mo" | b"zh-sg" | b"zh-tw" | b"cn"
    )
}
fn strict_bool(o: &[u8], k: &[u8]) -> Result<Option<bool>, &'static str> {
    match json_object_member(o, k) {
        None | Some(b"null") => Ok(None),
        Some(b"true") => Ok(Some(true)),
        Some(b"false") => Ok(Some(false)),
        _ => Err("invalid boolean"),
    }
}
fn denied(o: &[u8]) -> Result<(), &'static str> {
    for k in [
        b"paid".as_slice(),
        b"is_paid",
        b"premium",
        b"is_premium",
        b"locked",
    ] {
        if strict_bool(o, k)? == Some(true) {
            return Err("permission denied");
        }
    }
    Ok(())
}
fn bounded_url(u: &[u8]) -> Result<(), &'static str> {
    valid_uri(u)?;
    if u.iter().any(|c| matches!(c, b'?' | b'#' | b'@')) || contains_bytes(u, b"..") {
        return Err("invalid image URL");
    }
    for w in u.windows(3) {
        if w[0] == b'%'
            && matches!(
                (w[1].to_ascii_lowercase(), w[2].to_ascii_lowercase()),
                (b'2', b'f')
                    | (b'2', b'e')
                    | (b'5', b'c')
                    | (b'3', b'f')
                    | (b'2', b'3')
                    | (b'4', b'0')
            )
        {
            return Err("invalid image URL");
        }
    }
    Ok(())
}
fn cover<'a>(o: &[u8], id: &[u8], out: &'a mut [u8]) -> Result<Option<&'a [u8]>, &'static str> {
    let image = match json_object_member(o, b"image") {
        None | Some(b"null") => return Ok(None),
        Some(v) => v,
    };
    let cv = match json_object_member(image, b"cover") {
        None | Some(b"null") => return Ok(None),
        Some(v) => v,
    };
    let mut tmp = [0u8; 2048];
    let path = match string(cv, b"path") {
        None => return Ok(None),
        Some(v) => unescape(v, &mut tmp)?,
    };
    let mut normalized = [0u8; 2048];
    if path.is_empty() || path.len() >= normalized.len() {
        return Err("invalid cover path");
    }
    for (i, c) in path.iter().enumerate() {
        normalized[i] = if *c == b'\\' { b'/' } else { *c };
    }
    let path = &normalized[..path.len()];
    let path = path
        .strip_prefix(b"https://bulbasaur.poke-black-and-white.net/covers/")
        .unwrap_or(path);
    let suffix = path
        .strip_prefix(id)
        .and_then(|v| v.strip_prefix(b"/"))
        .ok_or("cover owner mismatch")?;
    if suffix.is_empty() || suffix.contains(&b'/') {
        return Err("invalid cover path");
    }
    let mut n = 0;
    bytes(
        out,
        &mut n,
        b"https://bulbasaur.poke-black-and-white.net/covers/",
    )?;
    bytes(out, &mut n, path)?;
    bounded_url(&out[..n])?;
    Ok(Some(&out[..n]))
}
fn image_url(u: &[u8]) -> Result<(), &'static str> {
    bounded_url(u)?;
    if let Some(path) = u.strip_prefix(b"https://bulbasaur.poke-black-and-white.net/covers/") {
        let i = path
            .iter()
            .position(|c| *c == b'/')
            .ok_or("invalid image URL")?;
        if !id_valid(&path[..i]) || path[i + 1..].is_empty() || path[i + 1..].contains(&b'/') {
            return Err("invalid image URL");
        }
        return Ok(());
    }
    let path = u
        .strip_prefix(b"https://chikorita.red-and-blue.net/storage/")
        .ok_or("invalid image URL")?;
    let mut parts = [b"".as_slice(); 6];
    let mut count = 0;
    for v in path.split(|c| *c == b'/') {
        if count == parts.len() || v.is_empty() {
            return Err("invalid image URL");
        }
        parts[count] = v;
        count += 1;
    }
    if count != 6 || !id_valid(parts[0]) || !zh(parts[4]) {
        return Err("invalid image URL");
    }
    let f = parts[5];
    if f.len() < 30
        || !id_valid(&f[..24])
        || f[24] != b'-'
        || !f[25..]
            .iter()
            .all(|c| c.is_ascii_alphanumeric() || *c == b'.' || *c == b'-')
    {
        return Err("invalid image URL");
    }
    Ok(())
}
fn string_array(o: &[u8], key: &[u8], b: &mut [u8], n: &mut usize) -> Result<(), &'static str> {
    bytes(b, n, b"[")?;
    if let Some(arr) = json_object_member(o, key) {
        let mut spans = [(0, 0); 128];
        let count = array_spans(arr, &mut spans)?;
        for (i, (a, z)) in spans[..count].iter().enumerate() {
            if i > 0 {
                bytes(b, n, b",")?;
            }
            let mut s = [0; 8192];
            let raw = &arr[*a..*z];
            let value = if raw.first() == Some(&b'{') {
                text(raw, b"name", &mut s)?
            } else {
                if raw.len() < 2 || raw.first() != Some(&b'"') || raw.last() != Some(&b'"') {
                    return Err("invalid string array");
                }
                unescape(&raw[1..raw.len() - 1], &mut s)?
            };
            q(b, n, value)?;
        }
    }
    bytes(b, n, b"]")
}
fn emit_manga(o: &[u8], b: &mut [u8], n: &mut usize, detail: bool) -> Result<(), &'static str> {
    let mut ib = [0; 32];
    let id = text(o, b"id", &mut ib)?;
    if !id_valid(id) {
        return Err("invalid manga ID");
    }
    let mut sb = [0; 256];
    if !slug_valid(text(o, b"slug", &mut sb)?) {
        return Err("invalid slug");
    }
    let mut tb = [0; 2048];
    let title = text(o, b"name", &mut tb)?;
    if title.is_empty() {
        return Err("empty title");
    }
    denied(o)?;
    bytes(b, n, b"{\"id\":\"mb:")?;
    bytes(b, n, id)?;
    bytes(b, n, b"\",\"title\":")?;
    q(b, n, title)?;
    bytes(b, n, b",\"contentRating\":\"unknown\",\"cover\":")?;
    let mut cb = [0; 4096];
    match cover(o, id, &mut cb)? {
        Some(url) => {
            bytes(b, n, b"{\"kind\":\"url\",\"url\":")?;
            q(b, n, url)?;
            bytes(b, n, b"}")?
        }
        None => bytes(b, n, b"null")?,
    };
    if detail {
        bytes(b, n, b",\"authors\":")?;
        string_array(o, b"author", b, n)?;
        bytes(b, n, b",\"tags\":")?;
        string_array(o, b"tags", b, n)?;
        bytes(b, n, b",\"description\":")?;
        let mut desc = [0; 32768];
        let mut dn = 0;
        if let Some(a) = json_object_member(o, b"description") {
            let mut spans = [(0, 0); 128];
            let count = array_spans(a, &mut spans)?;
            for (i, (a0, z)) in spans[..count].iter().enumerate() {
                let raw = &a[*a0..*z];
                if raw.len() < 2 || raw[0] != b'"' || raw[raw.len() - 1] != b'"' {
                    return Err("invalid description");
                }
                let mut tmp = [0; 8192];
                let v = unescape(&raw[1..raw.len() - 1], &mut tmp)?;
                if i > 0 {
                    bytes(&mut desc, &mut dn, b"\n\n")?;
                }
                bytes(&mut desc, &mut dn, v)?;
            }
        }
        q(b, n, &desc[..dn])?;
        bytes(b, n, b",\"status\":")?;
        q(
            b,
            n,
            match string(o, b"status") {
                Some(b"ongoing") => b"ongoing",
                Some(b"completed") => b"completed",
                Some(b"hiatus") => b"hiatus",
                Some(b"cancelled") => b"cancelled",
                _ => b"unknown",
            },
        )?;
        bytes(
            b,
            n,
            b",\"links\":[{\"kind\":\"source\",\"url\":\"https://mangaball.com/title-detail/",
        )?;
        bytes(b, n, id)?;
        bytes(b, n, b"\"}]")?;
    }
    bytes(b, n, b"}")
}
fn list(r: &[u8], search: bool) -> Result<usize, &'static str> {
    json_complete(r)?;
    let p = page(r)?;
    let mut path = [0; 4096];
    let mut n = 0;
    bytes(&mut path, &mut n, b"/api/v1/title/search-advanced?page=")?;
    write_num(&mut path, &mut n, p)?;
    bytes(&mut path, &mut n, b"&limit=24")?;
    if search {
        let mut qb = [0; 1024];
        let query = text(args(r), b"query", &mut qb)?;
        if query.is_empty() {
            return Err("empty query");
        }
        bytes(
            &mut path,
            &mut n,
            b"&tag_mode=AND&adult_mode=",
        )?;
        bytes(&mut path,&mut n,if hide_nsfw(r) {b"no_18"} else {b"all"})?;
        bytes(&mut path,&mut n,b"&keyword=")?;
        if !write_url_encoded(&mut path, &mut n, query) {
            return Err("payload overflow");
        }
    } else {
        let mut lb = [0; 64];
        let listing = text(args(r), b"listingId", &mut lb)?;
        bytes(
            &mut path,
            &mut n,
            match listing {
                b"popular" => b"&sort_by=views&sort_order=desc",
                b"latest" => b"&sort_by=lastupdate&sort_order=desc",
                _ => return Err("unknown listing"),
            },
        )?;
        bytes(&mut path, &mut n, b"&tag_mode=AND&adult_mode=")?;
        bytes(&mut path,&mut n,if hide_nsfw(r) {b"no_18"} else {b"all"})?;
    }
    let root = fetch(&path[..n], None)?;
    let data = member(root, b"data")?;
    let mut spans = [(0, 0); 24];
    let count = array_spans(data, &mut spans)?;
    let pagination = member(root, b"pagination")?;
    if uint(pagination, b"page")? != p || uint(pagination, b"limit")? != 24 {
        return Err("pagination mismatch");
    }
    let total = uint(pagination, b"total_pages")?;
    if total == 0 || p > total {
        return Err("pagination mismatch");
    }
    if count == 0 && p < total {
        return Err("empty nonterminal listing");
    }
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"items\":[")?;
    for (i, (a, z)) in spans[..count].iter().enumerate() {
        let o = &data[*a..*z];
        let id = string(o, b"id").ok_or("missing manga ID")?;
        for (pa, pz) in &spans[..i] {
            if string(&data[*pa..*pz], b"id") == Some(id) {
                return Err("duplicate manga ID");
            }
        }
        let adult=strict_bool(o,b"is18plus")?.ok_or("unexpected adult listing")?;
        if hide_nsfw(r) && adult {
            return Err("unexpected adult listing");
        }
        if i > 0 {
            bytes(b, &mut n, b",")?;
        }
        emit_manga(o, b, &mut n, false)?;
    }
    bytes(b, &mut n, b"],\"page\":{\"hasMore\":")?;
    bytes(b, &mut n, if p < total { b"true" } else { b"false" })?;
    bytes(b, &mut n, b",\"nextCursor\":")?;
    if p < total {
        bytes(b, &mut n, b"\"")?;
        write_num(b, &mut n, p.checked_add(1).ok_or("invalid page cursor")?)?;
        bytes(b, &mut n, b"\"")?;
    } else {
        bytes(b, &mut n, b"null")?;
    }
    bytes(b, &mut n, b"}}")?;
    Ok(n)
}
fn detail(r: &[u8]) -> Result<usize, &'static str> {
    json_complete(r)?;
    let mut mb = [0; 64];
    let id = input_mid(r, &mut mb)?;
    let mut path = [0; 256];
    let mut n = 0;
    bytes(&mut path, &mut n, b"/api/v1/title/detail/")?;
    bytes(&mut path, &mut n, id)?;
    let root = fetch(&path[..n], None)?;
    let o = member(root, b"data")?;
    if string(o, b"id") != Some(id) {
        return Err("detail owner mismatch");
    }
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"manga\":")?;
    emit_manga(o, b, &mut n, true)?;
    bytes(b, &mut n, b"}")?;
    Ok(n)
}
fn decimal(v: &[u8]) -> Result<(), &'static str> {
    if v == b"null" {
        return Ok(());
    }
    json_complete(v)?;
    let s = core::str::from_utf8(v).map_err(|_| "invalid number")?;
    let f = s.parse::<f64>().map_err(|_| "invalid number")?;
    if !f.is_finite() || f < 0.0 {
        return Err("invalid number");
    }
    Ok(())
}
fn chapters(r: &[u8]) -> Result<usize, &'static str> {
    json_complete(r)?;
    let mut mb = [0; 64];
    let id = input_mid(r, &mut mb)?;
    let mut wanted_buf=[0;128];
    let wanted=if json_object_member(args(r),b"chapterId").is_some() {
        let (owner,cid)=input_ch(r,&mut wanted_buf)?;
        if owner!=id {return Err("chapter owner mismatch");}
        Some(cid)
    } else {None};
    let mut body = [0; 128];
    let mut n = 0;
    bytes(&mut body, &mut n, b"{\"title_id\":")?;
    q(&mut body, &mut n, id)?;
    bytes(&mut body, &mut n, b"}")?;
    let root = fetch(
        b"/api/v1/chapter/chapter-listing-by-title-id",
        Some(&body[..n]),
    )?;
    let arr = member(root, b"data")?;
    let mut spans = [(0, 0); 2048];
    let count = array_spans(arr, &mut spans)?;
    if count == 0 {
        return Err("empty catalogue");
    }
    let observe=observations_supported(r);
    let blacklist=scanlator_blacklist(r)?;
    let mut excluded: Vec<String>=Vec::new();
    let mut known: Vec<String>=Vec::new();
    let mut chinese=0;
    let b = payload_buf();
    let mut n = 0;
    let mut written = 0;
    bytes(b, &mut n, b"{\"items\":[")?;
    for (i, (a, z)) in spans[..count].iter().enumerate() {
        let o = &arr[*a..*z];
        let cid = string(o, b"id").ok_or("missing chapter ID")?;
        if !id_valid(cid) || string(o, b"title_id") != Some(id) {
            return Err("chapter owner mismatch");
        }
        for (pa, pz) in &spans[..i] {
            if string(&arr[*pa..*pz], b"id") == Some(cid) {
                return Err("duplicate chapter ID");
            }
        }
        let lang = string(o, b"lang").ok_or("missing language")?;
        let num = member(o, b"number")?;
        decimal(num)?;
        if !zh(lang) {
            continue;
        }
        denied(o)?;
        if string(o, b"status") != Some(b"published") {
            return Err("permission denied");
        }
        chinese+=1;
        let mut blacklisted=false;
        if observe {
            if let Some(name)=chapter_scanlator(o)? {
                if !name.trim().is_empty() && !known.contains(&name) {
                    if known.len()==512 {return Err("too many scanlators");}
                    known.push(name.clone());
                }
                blacklisted=blacklist.contains(&name.trim().to_lowercase());
            }
        }
        let mut tb = [0; 4096];
        let title = match string(o, b"name") {
            Some(v) => unescape(v, &mut tb)?,
            None => b"",
        };
        if title.is_empty() {
            return Err("empty chapter title");
        }
        let volume = json_object_member(o, b"volume").unwrap_or(b"null");
        decimal(volume)?;
        // A selected group never hides a malformed/forbidden source entity.
        if let Some(wanted)=wanted {
            // Resume restores only its exact valid chapter. It is not a full
            // directory and cannot erase the saved progress via a list filter.
            if cid!=wanted {continue;}
        } else if blacklisted {
            let mut full=String::from("mbc:");
            full.push_str(core::str::from_utf8(id).map_err(|_|"invalid request")?);full.push(':');
            full.push_str(core::str::from_utf8(cid).map_err(|_|"invalid request")?);
            excluded.push(full);continue;
        }
        if written > 0 {
            bytes(b, &mut n, b",")?;
        }
        bytes(b, &mut n, b"{\"id\":\"mbc:")?;
        bytes(b, &mut n, id)?;
        bytes(b, &mut n, b":")?;
        bytes(b, &mut n, cid)?;
        bytes(b, &mut n, b"\",\"mangaId\":\"mb:")?;
        bytes(b, &mut n, id)?;
        bytes(b, &mut n, b"\",\"title\":")?;
        q(b, &mut n, title)?;
        bytes(b, &mut n, b",\"chapterNumber\":")?;
        bytes(b, &mut n, num)?;
        bytes(b, &mut n, b",\"volumeNumber\":")?;
        bytes(b, &mut n, volume)?;
        bytes(b, &mut n, b",\"language\":\"zh\"}")?;
        written += 1;
    }
    if chinese == 0 {
        return Err("no Chinese chapters");
    }
    if wanted.is_some() && written==0 {return Err("not found");}
    bytes(
        b,
        &mut n,
        b"],\"page\":{\"nextCursor\":null,\"hasMore\":false}",
    )?;
    if observe {
        bytes(b,&mut n,b",\"settingObservations\":{\"known_scanlators_pref\":")?;
        write_names(b,&mut n,&known)?;bytes(b,&mut n,b"}")?;
    }
    if blacklist_supported(r) {
        bytes(b,&mut n,b",\"excludedChapterIds\":")?;write_names(b,&mut n,&excluded)?;
    }
    bytes(b,&mut n,b"}")?;
    Ok(n)
}
fn pages(r: &[u8]) -> Result<usize, &'static str> {
    json_complete(r)?;
    let mut cb = [0; 128];
    let (mid, cid) = input_ch(r, &mut cb)?;
    let mut path = [0; 256];
    let mut n = 0;
    bytes(&mut path, &mut n, b"/api/v1/chapter-detail?chapter_id=")?;
    bytes(&mut path, &mut n, cid)?;
    let root = fetch(&path[..n], None)?;
    let data = member(root, b"data")?;
    let ch = member(data, b"chapter")?;
    if string(ch, b"id") != Some(cid)
        || string(ch, b"title_id") != Some(mid)
        || !zh(string(ch, b"lang").ok_or("missing language")?)
    {
        return Err("page owner mismatch");
    }
    if string(member(data, b"title")?, b"id") != Some(mid) {
        return Err("page title owner mismatch");
    }
    denied(ch)?;
    let arr = member(ch, b"pages")?;
    let mut spans = [(0, 0); 2048];
    let count = array_spans(arr, &mut spans)?;
    if count == 0 {
        return Err("empty pages");
    }
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"pages\":[")?;
    for (i, (a, z)) in spans[..count].iter().enumerate() {
        let raw = &arr[*a..*z];
        if raw.len() < 2 || raw[0] != b'"' || raw[raw.len() - 1] != b'"' {
            return Err("invalid page URL");
        }
        for (pa, pz) in &spans[..i] {
            if &arr[*pa..*pz] == raw {
                return Err("duplicate page URL");
            }
        }
        let mut ub = [0; 4096];
        let u = unescape(&raw[1..raw.len() - 1], &mut ub)?;
        image_url(u)?;
        for (pa, pz) in &spans[..i] {
            let raw_other = &arr[*pa..*pz];
            let mut ob = [0; 4096];
            let other = unescape(&raw_other[1..raw_other.len() - 1], &mut ob)?;
            if other == u {
                return Err("duplicate page URL");
            }
        }
        let path = u
            .strip_prefix(b"https://chikorita.red-and-blue.net/storage/")
            .ok_or("invalid image URL")?;
        let parts: (&[u8], &[u8]) = (
            path.split(|c| *c == b'/').next().ok_or("invalid path")?,
            path.rsplit(|c| *c == b'/').next().ok_or("invalid path")?,
        );
        if parts.0 != mid
            || parts.1.get(..24) != Some(cid)
            || path.split(|c| *c == b'/').nth(4) != string(ch, b"lang")
        {
            return Err("image owner mismatch");
        }
        if i > 0 {
            bytes(b, &mut n, b",")?;
        }
        bytes(b, &mut n, b"{\"id\":\"mbp:")?;
        bytes(b, &mut n, mid)?;
        bytes(b, &mut n, b":")?;
        bytes(b, &mut n, cid)?;
        bytes(b, &mut n, b":")?;
        write_num(b, &mut n, i)?;
        bytes(b, &mut n, b"\",\"index\":")?;
        write_num(b, &mut n, i)?;
        bytes(b, &mut n, b",\"image\":{\"kind\":\"url\",\"url\":")?;
        q(b, &mut n, u)?;
        bytes(b, &mut n, b"}}")?;
    }
    bytes(b, &mut n, b"]}")?;
    Ok(n)
}
fn image_request(r: &[u8]) -> Result<usize, &'static str> {
    json_complete(r)?;
    let mut ub = [0; 4096];
    let u = text(args(r), b"url", &mut ub)?;
    image_url(u)?;
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"imageRequest\":{\"url\":")?;
    q(b, &mut n, u)?;
    bytes(b,&mut n,b",\"method\":\"GET\",\"headers\":{\"User-Agent\":\"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0\",\"Referer\":\"https://mangaball.com/\",\"Origin\":\"https://mangaball.com\"}}}")?;
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
    let b = payload_buf();
    let v="{\"listings\":[{\"id\":\"popular\",\"name\":\"热门\",\"kind\":\"popular\",\"supportsPaging\":true},{\"id\":\"latest\",\"name\":\"最新\",\"kind\":\"latest\",\"supportsPaging\":true}]}".as_bytes();
    if v.len() > b.len() {
        return write_error("get_listings", "internal_error", "payload overflow");
    }
    b[..v.len()].copy_from_slice(v);
    write_success_payload("get_listings", v.len())
}
fn run_get_home(_: &[u8]) -> u32 {
    write_error("get_home", "not_implemented", "not supported")
}
fn run_get_filters(_: &[u8]) -> u32 {
    write_error("get_filters", "not_implemented", "not supported")
}
fn settings_schema(r: &[u8]) -> Result<usize, &'static str> {
    json_complete(r)?;let out=payload_buf();let mut n=0;
    bytes(out,&mut n,br#"{"settings":[{"id":"nsfw_pref","name":"Hide NSFW content","kind":"toggle","description":"Hide titles marked as 18+","default":"true"}"#)?;
    if blacklist_supported(r) {
        let raw=json_object_member(args(r),b"settingObservations").and_then(|v|json_object_member(v,KNOWN_KEY)).unwrap_or(b"[]");
        let mut known=names_array(raw)?;
        // Kotlin String ordering is UTF-16; retain raw displayed names and only
        // lowercase for its stable ordering, never casefold/deduplicate aliases.
        known.sort_by(|a,b|a.to_lowercase().encode_utf16().cmp(b.to_lowercase().encode_utf16()));
        bytes(out,&mut n,br#",{"id":"scanlator_blacklist_pref","name":"Scanlator blacklist","kind":"multiselect","description":"Hide chapters from the selected scanlators. The list fills up automatically as you browse titles.","default":[],"options":["#)?;
        for (i,name) in known.iter().enumerate() {
            if i>0 {bytes(out,&mut n,b",")?;}
            bytes(out,&mut n,b"{\"value\":")?;q(out,&mut n,name.as_bytes())?;
            bytes(out,&mut n,b",\"label\":")?;q(out,&mut n,name.as_bytes())?;bytes(out,&mut n,b"}")?;
        }
        bytes(out,&mut n,b"]}")?;
    }
    bytes(out,&mut n,b"]}")?;Ok(n)
}
fn run_get_settings(r: &[u8]) -> u32 {execute("get_settings",settings_schema(r))}
koma_source_sdk::koma_source_exports!("mangaball_zh");
