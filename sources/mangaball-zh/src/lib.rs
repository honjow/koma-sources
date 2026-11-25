#![no_std]
extern crate alloc;
use alloc::{collections::{BTreeMap, BTreeSet}, string::String, vec::Vec};
// Full Unicode lowercase (including contextual sigma) follows the adopted
// reference. Reuse Rust alloc rather than approximate per-character folding.
#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;
// Public contract: Keiyoushi MangaBall.kt and Dto.kt, Apache-2.0, revision
// 1319949ca9bc456908196e3ba2d104e94182c5d6. No authentication/mirror/JS processing.
use koma_source_sdk::chapters::{Chapter, ChapterNumber, ChapterPage, ChapterPlacement, ChapterResult, DecimalChapterNumber, GroupedChapters};
use koma_source_sdk::{
    host,
    json_utils::*,
    source::{SourceCapabilities, SourceInfo},
};
const BASE: &[u8] = b"https://mangaball.com";
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.mangaball.zh.koma",
    name: "MangaBall",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "MangaBall 的中文章节书库，同一作品可能有不同翻译组发布的章节。收录范围以章节语言为准，书名不一定是中文。",
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
    filters: true,
    settings: true,
    image_request: true,
    credentials: false,
};
// The canonical chapter API returns every language in one complete array.
// Public One Piece evidence is 6,322,808 raw bytes / 9,847 entries; keep
// bounded 8 MiB input/envelope budgets while the App result remains 1 MiB.
koma_source_sdk::koma_source_buffers! { payload:1024*1024,http_out:8*1024*1024,body:8*1024*1024,http_req:8192,scratch:8192, }
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
        // Only a complete, validated provider array can establish no chapters.
        // Keep metadata available and let the App distinguish this from removal.
        "empty catalogue" | "no Chinese chapters" => "no_chapters",
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
        "payload overflow" | "HTTP envelope too large" => "internal_error",
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
        // ABI -3 is a local output-capacity failure, not site transport.
        if e == -3 { "HTTP envelope too large" } else { "network failure" }
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
// These filters belong to upstream search; discovery keeps its adopted sort and adult default.
fn search_filters_supported(r: &[u8]) -> bool { host_feature(r,b"source_filter_scope_header_v1") && host_feature(r,b"source_filter_tristate_v1") }
fn filter_string<'a>(f: &'a [u8], key: &[u8]) -> Option<&'a [u8]> { json_object_member(f,key).and_then(|v| v.strip_prefix(b"\"")?.strip_suffix(b"\"")) }
fn append_search_filters(f: &[u8],path: &mut [u8],n: &mut usize) -> Result<(), &'static str> {
    for (key,param,allowed) in [(b"type".as_slice(),b"type".as_slice(), &[b"all".as_slice(),b"manga",b"manhwa",b"manhua",b"comics"][..]),(b"demographic".as_slice(),b"publicationDemographic".as_slice(), &[b"all".as_slice(),b"shounen",b"shoujo",b"seinen",b"josei"][..]),(b"status".as_slice(),b"status".as_slice(), &[b"all".as_slice(),b"ongoing",b"completed",b"hiatus",b"cancelled"][..])] {
        if let Some(v)=filter_string(f,key) { if !allowed.contains(&v) {return Err("invalid request");} if v!=b"all" {bytes(path,n,b"&")?;bytes(path,n,param)?;bytes(path,n,b"=")?;bytes(path,n,v)?;} }
    }
    let mut included=[0u8;4096]; let mut ni=0; let mut excluded=[0u8;4096]; let mut ne=0;
    for group in [b"tags_content".as_slice(),b"tags_format",b"tags_genre",b"tags_theme"] { if let Some(a)=json_object_member(f,group) { let mut spans=[(0,0);128];let count=array_spans(a,&mut spans)?; for (start,end) in &spans[..count] { let value=a[*start..*end].strip_prefix(b"\"").and_then(|v|v.strip_suffix(b"\"")).ok_or("invalid request")?; let (neg,id)=match value.strip_prefix(b"!"){Some(v)=>(true,v),None=>(false,value)}; if id.len()!=24 || !id.iter().all(u8::is_ascii_hexdigit) {return Err("invalid request");} let (dst,k)=if neg {(&mut excluded,&mut ne)} else {(&mut included,&mut ni)}; if *k>0 {bytes(dst,k,b",")?;} bytes(dst,k,id)?; } } }
    for (key,v) in [(b"included_tags".as_slice(),&included[..ni]),(b"excluded_tags".as_slice(),&excluded[..ne])] { if !v.is_empty(){bytes(path,n,b"&")?;bytes(path,n,key)?;bytes(path,n,b"=")?;if !write_url_encoded(path,n,v){return Err("payload overflow");}} }
    Ok(())
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
        let filters=if search_filters_supported(r) {json_object_member(args(r),b"filters").unwrap_or(b"{}")} else {b"{}"};
        let mut qb = [0; 1024];
        let query = text(args(r), b"query", &mut qb)?;
        if query.is_empty() {
            return Err("empty query");
        }
        if let Some(sort)=filter_string(filters,b"sort") { if sort!=b"relevance" { let pos=sort.iter().position(|x|*x==b':').ok_or("invalid request")?; let (field,dir0)=sort.split_at(pos);let dir=&dir0[1..]; if ![b"lastupdate".as_slice(),b"views",b"rating",b"created_at",b"name"].contains(&field)|| ![b"asc".as_slice(),b"desc"].contains(&dir) {return Err("invalid request");} bytes(&mut path,&mut n,b"&sort_by=")?;bytes(&mut path,&mut n,field)?;bytes(&mut path,&mut n,b"&sort_order=")?;bytes(&mut path,&mut n,dir)?; } }
        let mode=filter_string(filters,b"tagMode").unwrap_or(b"AND"); if mode!=b"AND" && mode!=b"OR" {return Err("invalid request");}
        bytes(&mut path,&mut n,b"&tag_mode=")?; bytes(&mut path,&mut n,mode)?; bytes(&mut path,&mut n,b"&adult_mode=")?;
        bytes(&mut path,&mut n,if hide_nsfw(r) {b"no_18"} else {b"all"})?;
        bytes(&mut path,&mut n,b"&keyword=")?;
        if !write_url_encoded(&mut path, &mut n, query) {
            return Err("payload overflow");
        }
        append_search_filters(filters,&mut path,&mut n)?;
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
fn decimal(v: &[u8], allow_negative: bool) -> Result<(), &'static str> {
    if v == b"null" {
        return Ok(());
    }
    json_complete(v)?;
    let s = core::str::from_utf8(v).map_err(|_| "invalid number")?;
    let f = s.parse::<f64>().map_err(|_| "invalid number")?;
    if !f.is_finite() || (!allow_negative && f < 0.0) {
        return Err("invalid number");
    }
    Ok(())
}
// The canonical API and current Keiyoushi preserve the complete provider array.
// A number is a display label: split parts can share it, and it can be absent.
// Keep positions from that array, before any language/blacklist selection. Only
// actual language/site/group evidence separates editions, never equal numbers.
fn chapter_scope(row: &[u8]) -> Result<String, &'static str> {
    let lang=string(row,b"lang").ok_or("missing language")?;
    let site=string(row,b"site").filter(|v|!v.is_empty() && v.len()<=128
        && v.iter().all(|c|c.is_ascii_alphanumeric() || matches!(*c,b'.'|b'_'|b'-')))
        .unwrap_or(b"unassigned");
    Ok(alloc::format!("{}:site:{}",core::str::from_utf8(lang).map_err(|_|"invalid language")?,
        core::str::from_utf8(site).map_err(|_|"invalid site")?))
}
fn chapter_group_id(row: &[u8]) -> Option<&[u8]> {
    string(row,b"group_id").filter(|v|id_valid(v)).or_else(||{
        let group=json_object_member(row,b"group")?;
        string(group,b"id").or_else(||string(group,b"_id")).filter(|v|id_valid(v))
    })
}
fn chapter_group_name(row: &[u8]) -> Option<&[u8]> {
    string(json_object_member(row,b"group")?,b"name").filter(|v|!v.is_empty())
}
fn edition_alias(scope:&str,name:Option<&[u8]>) -> String {
    match name { Some(name)=>alloc::format!("{scope}:name:{}",core::str::from_utf8(name).unwrap_or("")),None=>scope.into() }
}
fn chapter_edition(row:&[u8],known:&BTreeMap<String,BTreeSet<Vec<u8>>>) -> Result<String,&'static str> {
    let scope=chapter_scope(row)?;
    let alias=edition_alias(&scope,chapter_group_name(row));
    // Missing group_id is not a reason to drop a chapter. Reuse an unambiguous
    // same-language/site/name identity; conflicting real group IDs stay distinct.
    let group=chapter_group_id(row).or_else(||known.get(&alias).filter(|ids|ids.len()==1)
        .and_then(|ids|ids.iter().next()).map(Vec::as_slice));
    let identity=match group {
        Some(id)=>alloc::format!("group:{}",core::str::from_utf8(id).map_err(|_|"invalid group")?),
        None=>match chapter_group_name(row) {
            Some(name)=>alloc::format!("name:{}",core::str::from_utf8(name).map_err(|_|"invalid group")?),
            None=>String::from("unassigned"),
        },
    };
    Ok(alloc::format!("{scope}:{identity}"))
}

fn chapters(r: &[u8]) -> ChapterResult<'static, &'static str> {
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
    // Keep larger catalogue bookkeeping off the native 64 KiB call stack.
    let mut spans = alloc::vec![(0, 0); 16384];
    let count = array_spans(arr, &mut spans)?;
    if count == 0 {
        return Err("empty catalogue");
    }
    let observe=observations_supported(r);
    let blacklist=scanlator_blacklist(r)?;
    let mut excluded: Vec<String>=Vec::new();
    let mut known: Vec<String>=Vec::new();
    let mut chinese=0;
    let mut catalog = GroupedChapters::new(payload_buf(), r).map_err(|e| e.message())?;
    let mut mid = [0; 64];
    let mut mid_len = 0;
    bytes(&mut mid, &mut mid_len, b"mb:")?;
    bytes(&mut mid, &mut mid_len, id)?;
    let mut written = 0;
    let mut editions: BTreeMap<String,BTreeSet<Vec<u8>>> = BTreeMap::new();
    for (a,z) in &spans[..count] {
        let row=&arr[*a..*z];
        if !string(row,b"lang").is_some_and(zh) {continue;}
        if let Some(group)=chapter_group_id(row) {
            let scope=chapter_scope(row)?;
            editions.entry(scope.clone()).or_default().insert(group.to_vec());
            if let Some(name)=chapter_group_name(row) {
                editions.entry(edition_alias(&scope,Some(name))).or_default().insert(group.to_vec());
            }
        }
    }
    let mut seen_ids = BTreeSet::new();
    for (index, (a, z)) in spans[..count].iter().enumerate() {
        let o = &arr[*a..*z];
        let cid = string(o, b"id").ok_or("missing chapter ID")?;
        if !id_valid(cid) || string(o, b"title_id") != Some(id) {
            return Err("chapter owner mismatch");
        }
        // Validate all languages before selection; a foreign/duplicate entity
        // cannot be hidden by language or blacklist filtering. Preserve API order.
        if !seen_ids.insert(cid) {
            return Err("duplicate chapter ID");
        }
        let lang = string(o, b"lang").ok_or("missing language")?;
        let num = json_object_member(o, b"number").unwrap_or(b"null");
        // The reference Float accepts real signed chapter numbers (e.g. Chapter -1).
        // Validate every language without inventing a replacement number.
        decimal(num, true)?;
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
        decimal(volume, false)?;
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
        let mut chapter_id = [0; 128];
        let mut len = 0;
        bytes(&mut chapter_id, &mut len, b"mbc:")?;
        bytes(&mut chapter_id, &mut len, id)?;
        bytes(&mut chapter_id, &mut len, b":")?;
        bytes(&mut chapter_id, &mut len, cid)?;
        let mut chapter = Chapter::new(&chapter_id[..len], &mid[..mid_len], title);
        chapter.chapter_number = DecimalChapterNumber::new(num).map(ChapterNumber::Decimal);
        chapter.volume_number = DecimalChapterNumber::new(volume).map(ChapterNumber::Decimal);
        chapter.language = Some(b"zh");
        let group=chapter_edition(o,&editions)?;
        let placement = if wanted.is_none() {
            ChapterPlacement::InGroup {
                group: group.as_bytes(),
                // Global raw index includes all languages and all hidden entries.
                // The endpoint is a complete newest-first array, not a host page.
                position: (count - 1 - index) as u64,
            }
        } else {
            ChapterPlacement::ManualOnly
        };
        catalog.push(chapter, placement).map_err(|e| e.message())?;
        written += 1;
    }
    if chinese == 0 {
        return Err("no Chinese chapters");
    }
    if wanted.is_some() && written==0 {return Err("not found");}
    let response = if observe {
        catalog.finish_with_scanlators(
            ChapterPage::Complete,
            b"known_scanlators_pref",
            known.iter().map(|name| name.as_bytes()),
            blacklist_supported(r).then(|| excluded.iter().map(|id| id.as_bytes())),
        )
    } else {
        catalog.finish(ChapterPage::Complete)
    }.map_err(|e| e.message())?;
    diagnostic(b"operation_ok", b"get_chapters");
    Ok(response)
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
fn run_get_chapters(r: &[u8]) -> ChapterResult<'static, &'static str> {
    chapters(r)
}
fn write_chapter_error(message: &str) -> u32 {
    let error = code(message);
    diagnostic(b"operation_error", error.as_bytes());
    write_error("get_chapters", error, message)
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
fn run_get_filters(r: &[u8]) -> u32 {
    if !search_filters_supported(r) { let v=br#"{"filters":[]}"#; payload_buf()[..v.len()].copy_from_slice(v); return write_success_payload("get_filters",v.len()); }
    let v=r###"{"filters":[{"id":"sort","name":"排序方式","type":"select","scope":"search","options":[{"value":"relevance","label":"相关度"},{"value":"lastupdate:asc","label":"最近更新（升序）"},{"value":"lastupdate:desc","label":"最近更新（降序）"},{"value":"views:asc","label":"最多浏览（升序）"},{"value":"views:desc","label":"最多浏览（降序）"},{"value":"rating:asc","label":"最高评分（升序）"},{"value":"rating:desc","label":"最高评分（降序）"},{"value":"created_at:asc","label":"最近添加（升序）"},{"value":"created_at:desc","label":"最近添加（降序）"},{"value":"name:asc","label":"标题（升序）"},{"value":"name:desc","label":"标题（降序）"}],"default":"relevance"},{"id":"type","name":"作品类型","type":"select","scope":"search","options":[{"value":"all","label":"全部"},{"value":"manga","label":"日漫"},{"value":"manhwa","label":"韩漫"},{"value":"manhua","label":"国漫"},{"value":"comics","label":"欧美漫画"}],"default":"all"},{"id":"demographic","name":"适合受众","type":"select","scope":"search","options":[{"value":"all","label":"全部"},{"value":"shounen","label":"少年"},{"value":"shoujo","label":"少女"},{"value":"seinen","label":"青年"},{"value":"josei","label":"女性"}],"default":"all"},{"id":"status","name":"连载状态","type":"select","scope":"search","options":[{"value":"all","label":"全部"},{"value":"ongoing","label":"连载中"},{"value":"completed","label":"已完结"},{"value":"hiatus","label":"休刊中"},{"value":"cancelled","label":"已腰斩"}],"default":"all"},{"id":"tagMode","name":"标签匹配模式","type":"select","scope":"search","options":[{"value":"AND","label":"全部满足（AND）"},{"value":"OR","label":"任一满足（OR）"}],"default":"AND"},{"id":"tags_content","name":"内容警告","type":"tristate","scope":"search","options":[{"value":"685148d115e8b86aae68e4f3","label":"血腥暴力"},{"value":"685146c5f3ed681c80f257e7","label":"性暴力"}]},{"id":"tags_format","name":"作品形式","type":"tristate","scope":"search","options":[{"value":"685148d115e8b86aae68e4ec","label":"四格"},{"value":"685148cf15e8b86aae68e4de","label":"改编"},{"value":"685148e915e8b86aae68e558","label":"合集"},{"value":"685148fe15e8b86aae68e5a7","label":"获奖作品"},{"value":"6851490e15e8b86aae68e5da","label":"同人志"},{"value":"6851498215e8b86aae68e704","label":"粉丝上色"},{"value":"685148d615e8b86aae68e502","label":"全彩"},{"value":"685148d915e8b86aae68e517","label":"条漫"},{"value":"6851493515e8b86aae68e64a","label":"官方全彩"},{"value":"685148eb15e8b86aae68e56c","label":"短篇"},{"value":"6851492e15e8b86aae68e633","label":"独立出版"},{"value":"685148d715e8b86aae68e50d","label":"网络漫画"}]},{"id":"tags_genre","name":"题材","type":"tristate","scope":"search","options":[{"value":"685146c5f3ed681c80f257e3","label":"动作"},{"value":"689371f0a943baf927094f03","label":"成人"},{"value":"685146c5f3ed681c80f257e6","label":"冒险"},{"value":"685148ef15e8b86aae68e573","label":"耽美"},{"value":"68ecab8507ec62d87e62780f","label":"漫画"},{"value":"685146c5f3ed681c80f257e5","label":"喜剧"},{"value":"685148da15e8b86aae68e51f","label":"犯罪"},{"value":"685148cf15e8b86aae68e4dd","label":"剧情"},{"value":"6892a73ba943baf927094e37","label":"擦边"},{"value":"685146c5f3ed681c80f257ea","label":"奇幻"},{"value":"685148da15e8b86aae68e524","label":"百合"},{"value":"685148db15e8b86aae68e527","label":"历史"},{"value":"685148da15e8b86aae68e520","label":"恐怖"},{"value":"685146c5f3ed681c80f257e9","label":"异世界"},{"value":"694cc2d9f8014f5e0a63ac73","label":"女性向"},{"value":"6851490d15e8b86aae68e5d4","label":"魔法少女"},{"value":"68ecab1e07ec62d87e627806","label":"日漫"},{"value":"68ecab4807ec62d87e62780b","label":"国漫"},{"value":"68ecab3b07ec62d87e627809","label":"韩漫"},{"value":"68932d11a943baf927094e7b","label":"成熟向"},{"value":"6851490c15e8b86aae68e5d2","label":"机甲"},{"value":"6851494e15e8b86aae68e66e","label":"医疗"},{"value":"685148d215e8b86aae68e4f4","label":"悬疑"},{"value":"685148e215e8b86aae68e544","label":"哲理"},{"value":"685148d715e8b86aae68e507","label":"心理"},{"value":"694cc2d9f8014f5e0a63ac75","label":"复仇"},{"value":"685148cf15e8b86aae68e4db","label":"恋爱"},{"value":"685148cf15e8b86aae68e4da","label":"科幻"},{"value":"694cc2d9f8014f5e0a63ac74","label":"少女向"},{"value":"689f0ab1f2e66744c6091524","label":"少年爱"},{"value":"685148d015e8b86aae68e4e3","label":"日常"},{"value":"689371f2a943baf927094f04","label":"情色"},{"value":"685148f515e8b86aae68e588","label":"体育"},{"value":"6851492915e8b86aae68e61c","label":"超级英雄"},{"value":"685148d915e8b86aae68e51e","label":"惊悚"},{"value":"685148db15e8b86aae68e529","label":"悲剧"},{"value":"68932c3ea943baf927094e77","label":"用户创作"},{"value":"6851490715e8b86aae68e5c3","label":"武侠"},{"value":"68932f68a943baf927094eaa","label":"Yaoi"},{"value":"6896a885a943baf927094f66","label":"Yuri"}]},{"id":"tags_theme","name":"主题","type":"tristate","scope":"search","options":[{"value":"6a0026ba63a8d384c0a4be13","label":"3D"},{"value":"6851490d15e8b86aae68e5d5","label":"外星人"},{"value":"685148e715e8b86aae68e54b","label":"动物"},{"value":"68bf09ff8fdeab0b6a9bc2b7","label":"漫画"},{"value":"685148d215e8b86aae68e4f8","label":"美食"},{"value":"685148df15e8b86aae68e534","label":"伪娘"},{"value":"685148d915e8b86aae68e519","label":"不良少年"},{"value":"685146c5f3ed681c80f257e4","label":"恶魔"},{"value":"685148d715e8b86aae68e505","label":"性转"},{"value":"685148d615e8b86aae68e501","label":"幽灵"},{"value":"685148d015e8b86aae68e4e8","label":"辣妹"},{"value":"685146c5f3ed681c80f257e8","label":"后宫"},{"value":"68bfceaf4dbc442a26519889","label":"Hentai"},{"value":"685148f215e8b86aae68e584","label":"乱伦"},{"value":"685148d715e8b86aae68e506","label":"萝莉"},{"value":"685148d915e8b86aae68e518","label":"黑帮"},{"value":"685148d715e8b86aae68e509","label":"魔法"},{"value":"68f5f5ce5f29d3c1863dec3a","label":"18+韩漫"},{"value":"6851490615e8b86aae68e5c2","label":"武术"},{"value":"685148e215e8b86aae68e541","label":"军事"},{"value":"685148db15e8b86aae68e52c","label":"魔物娘"},{"value":"685146c5f3ed681c80f257e2","label":"怪物"},{"value":"685148d015e8b86aae68e4e4","label":"音乐"},{"value":"685148d715e8b86aae68e508","label":"忍者"},{"value":"685148d315e8b86aae68e4fd","label":"职场"},{"value":"6851498815e8b86aae68e714","label":"警察"},{"value":"685148e215e8b86aae68e540","label":"末世"},{"value":"685146c5f3ed681c80f257e1","label":"转生"},{"value":"685148df15e8b86aae68e533","label":"逆后宫"},{"value":"6851490415e8b86aae68e5b9","label":"武士"},{"value":"685148d015e8b86aae68e4e7","label":"校园"},{"value":"6a0025c263a8d384c0a4be07","label":"青年向"},{"value":"685148d115e8b86aae68e4ed","label":"正太"},{"value":"685148db15e8b86aae68e528","label":"超自然"},{"value":"685148cf15e8b86aae68e4dc","label":"生存"},{"value":"6851490c15e8b86aae68e5d1","label":"穿越"},{"value":"6851493515e8b86aae68e645","label":"传统游戏"},{"value":"685148f915e8b86aae68e597","label":"吸血鬼"},{"value":"685148e115e8b86aae68e53c","label":"电子游戏"},{"value":"6851492115e8b86aae68e602","label":"恶役千金"},{"value":"68514a1115e8b86aae68e83e","label":"虚拟现实"},{"value":"6851490c15e8b86aae68e5d3","label":"丧尸"}]}]}"###.as_bytes(); payload_buf()[..v.len()].copy_from_slice(v); write_success_payload("get_filters",v.len())
}
fn settings_schema(r: &[u8]) -> Result<usize, &'static str> {
    json_complete(r)?;let out=payload_buf();let mut n=0;
    bytes(out,&mut n,r#"{"settings":[{"id":"nsfw_pref","name":"隐藏成人内容","kind":"toggle","description":"隐藏标记为 18+ 的作品","default":"true"}"#.as_bytes())?;
    if blacklist_supported(r) {
        let raw=json_object_member(args(r),b"settingObservations").and_then(|v|json_object_member(v,KNOWN_KEY)).unwrap_or(b"[]");
        let mut known=names_array(raw)?;
        // Kotlin String ordering is UTF-16; retain raw displayed names and only
        // lowercase for its stable ordering, never casefold/deduplicate aliases.
        known.sort_by(|a,b|a.to_lowercase().encode_utf16().cmp(b.to_lowercase().encode_utf16()));
        bytes(out,&mut n,r#",{"id":"scanlator_blacklist_pref","name":"汉化组黑名单","kind":"multiselect","description":"隐藏所选汉化组的章节；浏览作品时列表自动填充。","default":[],"options":["#.as_bytes())?;
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
koma_source_sdk::koma_source_exports!("mangaball_zh", typed_chapters, write_chapter_error);

mod comments;
koma_source_sdk::koma_source_comments_export!(run_get_comments);

fn run_get_comments(request: &[u8]) -> u32 {
    execute(koma_source_sdk::comments::OPERATION, comments::load(request, payload_buf()))
}
