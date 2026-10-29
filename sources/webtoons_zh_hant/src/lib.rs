#![no_std]
// Keiyoushi Webtoons.kt, Apache-2.0, revision 10f06eb833af5990bc5470c84c5e93edc159ad4e.
// Independent zh-Hant public source. No age/consent pre-acceptance, accounts or retries.
use koma_source_sdk::{
    host,
    json_utils::*,
    source::{SourceCapabilities, SourceInfo},
};
const BASE: &[u8] = b"https://www.webtoons.com";
const MOBILE: &[u8] = b"https://m.webtoons.com";
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.webtoons.zh-hant.koma",
    name: "LINE WEBTOON (Traditional Chinese)",
    version: "0.1.10",
    api_version: "0.2",
    language: "zh-Hant",
    author: "Koma source contributors",
    description:
        "",
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
    filters: true,
    settings: true,
    credentials: false,
    image_request: true,
};
koma_source_sdk::koma_source_buffers! {payload:1024*1024,http_out:4*1024*1024,body:2*1024*1024,http_req:16384,scratch:16384,}
koma_source_sdk::koma_source_helpers!();
// Existing host ABI, already exported by the native and development runtimes.
#[link(wasm_import_module = "koma_host")]
extern "C" {
    #[link_name = "html_select_all"]
    fn select_all(desc: i32, selector: *const u8, len: u32, out: *mut i32, cap: u32) -> i32;
}
struct Node(host::HtmlDescriptor);
impl Drop for Node {
    fn drop(&mut self) {
        let _ = host::html_close(self.0);
    }
}
impl Node {
    fn parse(b: &[u8]) -> Result<Self, &'static str> {
        host::html_parse(b).map(Self).map_err(|_| "invalid HTML")
    }
    fn select(&self, s: &[u8]) -> Result<Self, &'static str> {
        host::html_select(self.0, s)
            .map(Self)
            .map_err(|_| "required HTML node missing")
    }
    fn attr<'a>(&self, k: &[u8], out: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
        let n = host::html_attr(self.0, k, out).map_err(|_| "HTML attribute overflow")?;
        if n >= out.len() {
            return Err("HTML attribute may be truncated");
        };
        Ok(&out[..n])
    }
    fn optional_attr<'a>(
        &self,
        k: &[u8],
        out: &'a mut [u8],
    ) -> Result<Option<&'a [u8]>, &'static str> {
        match host::html_attr(self.0, k, out) {
            Err(-1) => Ok(None),
            Err(_) => Err("HTML attribute overflow"),
            Ok(n) if n >= out.len() => Err("HTML attribute may be truncated"),
            Ok(0) => Ok(None),
            Ok(n) => Ok(Some(&out[..n])),
        }
    }
    fn text<'a>(&self, out: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
        let n = host::html_text(self.0, out).map_err(|_| "HTML text overflow")?;
        if n >= out.len() {
            return Err("HTML text may be truncated");
        };
        Ok(trim_ascii(&out[..n]))
    }
}
struct Nodes {
    ids: [i32; 2048],
    n: usize,
}
impl Nodes {
    fn get(root: &Node, s: &[u8]) -> Result<Self, &'static str> {
        let mut v = Self {
            ids: [0; 2048],
            n: 0,
        };
        let n = unsafe {
            select_all(
                root.0.raw(),
                s.as_ptr(),
                s.len() as u32,
                v.ids.as_mut_ptr(),
                (v.ids.len() * 4) as u32,
            )
        };
        if n < 0 {
            return Err("HTML selection failed");
        };
        v.n = (n as usize).min(v.ids.len());
        if n as usize > v.ids.len() {
            return Err("too many HTML nodes");
        };
        Ok(v)
    }
    fn at(&self, i: usize) -> Node {
        Node(host::HtmlDescriptor::from_raw(self.ids[i]))
    }
}
impl Drop for Nodes {
    fn drop(&mut self) {
        for &id in &self.ids[..self.n] {
            let _ = host::html_close(host::HtmlDescriptor::from_raw(id));
        }
    }
}
// A borrowed selected node must not be closed twice: at() transfers no ownership.
fn selected(nodes: &Nodes, i: usize) -> core::mem::ManuallyDrop<Node> {
    core::mem::ManuallyDrop::new(nodes.at(i))
}
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
        None => return Ok(1),
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
    if write_bytes(&mut out, &mut n, b"webtoons_zh_hant stage=")
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
        "cancelled" => "cancelled",
        "timeout" => "timeout",
        "client failure" => "client_error",
        "server failure" => "server_error",
        "invalid request"
        | "invalid page cursor"
        | "invalid search cursor"
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
fn input(r: &[u8]) -> Result<&[u8], &'static str> {
    core::str::from_utf8(r).map_err(|_| "invalid UTF8")?;
    if r.len() > 65536
        || r.first() != Some(&b'{')
        || json_value_end(r, 0, 0).map(|p| ws(r, p)) != Some(r.len())
    {
        return Err("invalid request");
    }
    let a = args(r);
    if a.first() != Some(&b'{') {
        return Err("invalid request");
    };
    Ok(a)
}
fn http(url: &[u8], mobile: bool) -> Result<&'static [u8], &'static str> {
    diagnostic(b"fetch_begin", b"request");
    let referer = if mobile {
        b"https://m.webtoons.com/".as_slice()
    } else {
        b"https://www.webtoons.com/".as_slice()
    };
    let extra: &[(&[u8], &[u8])] = if mobile { &[] } else { &[(b"Origin", BASE)] };
    let n = koma_source_sdk::build_get_request(http_req_buf(), url, Some(referer), extra)
        .ok_or("invalid request")?;
    let len = host::http_request(&http_req_buf()[..n], http_out()).map_err(|e| {
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
    if len >= http_out().len() {
        return Err("HTTP envelope overflow");
    }
    let r = unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(HTTP_OUT).cast::<u8>(), len) };
    if json_value_end(r, 0, 0).map(|p| ws(r, p)) != Some(r.len()) {
        return Err("invalid HTTP envelope");
    }
    let status = json_object_member(r, b"status")
        .and_then(valid_status)
        .or_else(|| json_object_member(r, b"statusCode").and_then(valid_status));
    let mut log = [0; 80];
    let mut l = 0;
    bytes(
        &mut log,
        &mut l,
        b"webtoons_zh_hant stage=http_status value=",
    )?;
    if !write_usize(&mut log, &mut l, status.unwrap_or(0)) {
        return Err("payload overflow");
    };
    host::log_info(&log[..l]);
    match status {
        Some(200..=299) => {}
        Some(401 | 403) => return Err("permission denied"),
        Some(404) => return Err("not found"),
        Some(429) => return Err("rate limited"),
        Some(400..=499) => return Err("client failure"),
        Some(500..=599) => return Err("server failure"),
        Some(_) => return Err("unexpected HTTP status"),
        None => {
            if json_object_member(r, b"ok") == Some(b"false".as_slice()) {
                return Err(
                    match string(json_object_member(r, b"error").unwrap_or(r), b"code") {
                        Some(b"timeout") => "timeout",
                        Some(
                            b"network_error"
                            | b"transport_unavailable"
                            | b"transport_invalid_response",
                        ) => "network failure",
                        _ => "invalid HTTP envelope",
                    },
                );
            };
            return Err("invalid HTTP envelope");
        }
    }
    let n = koma_source_sdk::decode_json_body_into(r, body_buf())
        .map_err(|_| "HTTP body invalid or too large")?;
    if n == 0 || n >= body_buf().len() {
        return Err("HTTP body empty or overflow");
    }
    let body =
        unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(BODY_BUF).cast::<u8>(), n) };
    core::str::from_utf8(body).map_err(|_| "invalid UTF8")?;
    diagnostic(b"body_ready", b"ok");
    Ok(body)
}
fn doc(url: &[u8]) -> Result<Node, &'static str> {
    Node::parse(http(url, false)?)
}
fn nib(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'A'..=b'F' => Some(c - b'A' + 10),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    }
}
fn safe_url_chars(s: &[u8]) -> Result<(), &'static str> {
    if s.is_empty()
        || s.len() > 8192
        || !s.is_ascii()
        || s.iter()
            .any(|c| *c <= 32 || *c == 127 || b"\"<>\\#".contains(c))
    {
        return Err("invalid public URL");
    }
    let mut i = 0;
    while i < s.len() {
        if s[i] == b'%' {
            let c = nib(*s.get(i + 1).ok_or("invalid public URL")?)
                .and_then(|a| nib(*s.get(i + 2)?).map(|b| a * 16 + b))
                .ok_or("invalid public URL")?;
            if c < 32 || c == 127 {
                return Err("invalid public URL");
            };
            i += 3
        } else {
            i += 1
        }
    }
    Ok(())
}
fn relative(s: &[u8]) -> Result<&[u8], &'static str> {
    let p = s.strip_prefix(BASE).unwrap_or(s);
    safe_url_chars(p)?;
    // Canvas's real translated episode API returns an /en/canvas/ viewer
    // with an explicit reading language. Accept only that exact zh-Hant form;
    // do not rewrite the returned chapter identity or admit arbitrary English pages.
    let route = p.split(|b| *b == b'?').next().unwrap_or(b"");
    let mut language_count = 0;
    let mut traditional = false;
    if let Some(query) = p.split(|b| *b == b'?').nth(1) {
        for field in query.split(|b| *b == b'&') {
            if let Some(value) = field.strip_prefix(b"readingLanguageCode=") {
                language_count += 1;
                traditional = value == b"zh-hant";
            }
        }
    }
    let translated_canvas = route.starts_with(b"/en/canvas/") && route.ends_with(b"/viewer")
        && language_count == 1 && traditional;
    if (!p.starts_with(b"/zh-hant/") && !translated_canvas)
        || p.starts_with(b"//")
        || p.split(|c| *c == b'/').any(|v| v == b".." || v == b".")
    {
        return Err("invalid language route");
    };
    Ok(p)
}
fn param<'a>(p: &'a [u8], key: &[u8]) -> Result<&'a [u8], &'static str> {
    let start = p
        .iter()
        .position(|c| *c == b'?')
        .ok_or("missing identity query")?;
    let mut result = None;
    for v in p[start + 1..].split(|c| *c == b'&') {
        if let Some(k) = v.strip_prefix(key).and_then(|x| x.strip_prefix(b"=")) {
            if result.is_some() || k.len() > 20 || number(k).filter(|n| *n > 0).is_none() {
                return Err("invalid numeric identity");
            };
            result = Some(k)
        }
    }
    result.ok_or("missing identity query")
}
fn manga_route(p: &[u8]) -> Result<(&[u8], &[u8], &[u8]), &'static str> {
    let p = relative(p)?;
    let id = param(p, b"title_no")?;
    let path = p
        .split(|c| *c == b'?')
        .next()
        .ok_or("invalid manga route")?;
    if !path.ends_with(b"/list") || path.split(|c| *c == b'/').count() != 5 {
        return Err("invalid manga route");
    };
    let kind = if path.starts_with(b"/zh-hant/canvas/") {
        b"canvas".as_slice()
    } else {
        b"webtoon".as_slice()
    };
    Ok((kind, id, path))
}
fn manga_parts(id: &[u8]) -> Result<(&[u8], &[u8], &[u8]), &'static str> {
    let rest = id
        .strip_prefix(b"manga:webtoons:zh-Hant:")
        .ok_or("invalid request")?;
    let mut p = rest.splitn(3, |c| *c == b':');
    let kind = p.next().ok_or("invalid request")?;
    let title = p.next().ok_or("invalid request")?;
    let path = p.next().ok_or("invalid request")?;
    if (kind != b"canvas" && kind != b"webtoon") || number(title).filter(|n| *n > 0).is_none() {
        return Err("invalid request");
    };
    relative(path)?;
    if !path.ends_with(b"/list")
        || path.contains(&b'?')
        || (path.starts_with(b"/zh-hant/canvas/") != (kind == b"canvas"))
    {
        return Err("invalid request");
    };
    Ok((kind, title, path))
}
fn make_url<'a>(path: &[u8], out: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    let mut n = 0;
    bytes(out, &mut n, BASE)?;
    bytes(out, &mut n, path)?;
    Ok(&out[..n])
}
fn manga_url<'a>(id: &[u8], out: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    let (_, title, path) = manga_parts(id)?;
    let mut n = 0;
    bytes(out, &mut n, BASE)?;
    bytes(out, &mut n, path)?;
    bytes(out, &mut n, b"?title_no=")?;
    bytes(out, &mut n, title)?;
    Ok(&out[..n])
}
fn mid(
    b: &mut [u8],
    n: &mut usize,
    kind: &[u8],
    id: &[u8],
    path: &[u8],
) -> Result<(), &'static str> {
    bytes(b, n, b"\"manga:webtoons:zh-Hant:")?;
    bytes(b, n, kind)?;
    bytes(b, n, b":")?;
    bytes(b, n, id)?;
    bytes(b, n, b":")?;
    if !append_json_escaped(b, n, path) {
        return Err("payload overflow");
    };
    bytes(b, n, b"\"")
}
fn chapter_route(p: &[u8]) -> Result<(&[u8], &[u8], &[u8]), &'static str> {
    let p = relative(p)?;
    if !p
        .split(|c| *c == b'?')
        .next()
        .unwrap_or(b"")
        .ends_with(b"/viewer")
    {
        return Err("invalid chapter route");
    };
    Ok((param(p, b"title_no")?, param(p, b"episode_no")?, p))
}
fn chapters_id(r: &[u8]) -> Result<&[u8], &'static str> {
    let a = input(r)?;
    let id = string(a, b"chapterId").ok_or("invalid request")?;
    let p = id
        .strip_prefix(b"chapter:webtoons:zh-Hant:")
        .ok_or("invalid request")?;
    chapter_route(p)?;
    Ok(p)
}
fn image_url(s: &[u8]) -> Result<(), &'static str> {
    // Primary list covers contain genuine UTF8 filenames and literal path spaces.
    // Retain the logical IRI exactly; reject control/escape/authority injection.
    core::str::from_utf8(s).map_err(|_| "invalid image URL")?;
    if s.is_empty()
        || s.len() > 8192
        || s.iter()
            .any(|c| *c < 32 || *c == 127 || b"\"<>\\#".contains(c))
    {
        return Err("invalid image URL");
    }
    let mut i = 0;
    while i < s.len() {
        if s[i] == b'%' {
            let c = nib(*s.get(i + 1).ok_or("invalid image URL")?)
                .and_then(|a| nib(*s.get(i + 2)?).map(|b| a * 16 + b))
                .ok_or("invalid image URL")?;
            if c < 32 || c == 127 {
                return Err("invalid image URL");
            }
            i += 3;
        } else {
            i += 1;
        }
    }
    let rest = s.strip_prefix(b"https://").ok_or("invalid image URL")?;
    let host = rest
        .split(|c| *c == b'/')
        .next()
        .ok_or("invalid image URL")?;
    if host != b"webtoon-phinf.pstatic.net" && host != b"swebtoon-phinf.pstatic.net" {
        return Err("invalid image URL");
    };
    if rest.len() == host.len() {
        return Err("invalid image URL");
    };
    Ok(())
}
// The All page is a preview of both categories. Keep its order; subsequent
// category pages retain the same MID so BrowseViewModel can deduplicate them.
#[derive(Clone, Copy)]
enum SearchPage { All, Originals(usize, bool), Canvas(usize) }

// Search category is operation-scoped; the legacy All cursor chain remains unchanged.
fn search_filter_supported(r: &[u8]) -> bool {
    json_object_member(r,b"hostFeatures").map(|v| { let v=trim_ascii(v);v.starts_with(b"[") && v.ends_with(b"]") && v[1..v.len()-1].split(|c| *c==b',').any(|v|trim_ascii(v)==b"\"source_filter_scope_header_v1\"") }).unwrap_or(false)
}
fn search_category(r: &[u8]) -> Result<&[u8], &'static str> {
    if !search_filter_supported(r) { return Ok(b"all"); }
    let a=input(r)?; let filter=json_object_member(a,b"filters").unwrap_or(b"{}");
    match string(filter,b"searchType").unwrap_or(b"all") { b"all"=>Ok(b"all"),b"originals"=>Ok(b"originals"),b"canvas"=>Ok(b"canvas"),_=>Err("invalid request") }
}
fn search_page(r: &[u8]) -> Result<SearchPage, &'static str> {
    let a = input(r)?;
    let raw = match json_object_member(a, b"page") { None | Some(b"null") => return Ok(SearchPage::All), Some(v) => v };
    let v = if raw.first() == Some(&b'{') { json_object_member(raw, b"cursor").unwrap_or(b"null") } else { raw };
    if v == b"null" || v == b"1" || v == b"\"1\"" { return Ok(SearchPage::All); }
    if v.len() < 3 || v[0] != b'"' || v[v.len()-1] != b'"' { return Err("invalid search cursor"); }
    let v = &v[1..v.len()-1];
    let (kind, rest) = if let Some(v) = v.strip_prefix(b"o:") { (b'o', v) } else if let Some(v) = v.strip_prefix(b"c:") { (b'c', v) } else { return Err("invalid search cursor"); };
    let (digits, canvas) = if kind == b'o' {
        let at = rest.iter().position(|b| *b == b':').ok_or("invalid search cursor")?;
        let (n, c) = (&rest[..at], &rest[at+1..]);
        (n, match c { b"0" => false, b"1" => true, _ => return Err("invalid search cursor") })
    } else { (rest, false) };
    if digits.len() > 1 && digits[0] == b'0' { return Err("invalid search cursor"); }
    let n = number(digits).filter(|n| *n > 0 && *n <= 10000).ok_or("invalid search cursor")?;
    Ok(if kind == b'o' { SearchPage::Originals(n, canvas) } else { SearchPage::Canvas(n) })
}

fn check_search_link(href: &[u8], section: &[u8], query: &[u8], page: Option<usize>) -> Result<(), &'static str> {
    let href = href.strip_prefix(BASE).unwrap_or(href);
    let tail = href.strip_prefix(section).and_then(|s| s.strip_prefix(b"?keyword=")).ok_or("invalid search link")?;
    let end = tail.iter().position(|b| *b == b'&').unwrap_or(tail.len());
    let suffix = &tail[end..];
    match page {
        None if !suffix.is_empty() => return Err("invalid search link"),
        Some(n) => { let p = suffix.strip_prefix(b"&page=").and_then(number).ok_or("invalid search link")?; if p != n { return Err("invalid search link"); } },
        _ => (),
    }
    let mut decoded = [0; 4096]; let mut i = 0; let mut len = 0;
    while i < end {
        let c = match tail[i] {
            b'%' => { if i+2 >= end { return Err("invalid search link"); } let v = nib(tail[i+1]).zip(nib(tail[i+2])).map(|(a,b)| a*16+b).ok_or("invalid search link")?; i += 3; v },
            b'+' => { i += 1; b' ' },
            c => { i += 1; c },
        };
        if len >= decoded.len() { return Err("invalid search link"); }
        decoded[len] = c; len += 1;
    }
    if &decoded[..len] != query { return Err("invalid search link"); }
    Ok(())
}

fn search_more(root: &Node, selector: &[u8], section: &[u8], query: &[u8]) -> Result<bool, &'static str> {
    let nodes = Nodes::get(root, selector)?;
    if nodes.n > 1 { return Err("ambiguous search section"); }
    if nodes.n == 0 { return Ok(false); }
    let mut h = [0; 8192];
    check_search_link(selected(&nodes, 0).attr(b"href", &mut h)?, section, query, None)?;
    Ok(true)
}

fn search_next(root: &Node, section: &[u8], query: &[u8], pn: usize) -> Result<bool, &'static str> {
    let tabs = Nodes::get(root, b"a.snb_tab[aria-current=page]")?;
    if tabs.n != 1 { return Err("search category identity missing"); }
    let mut tab_href = [0; 8192];
    check_search_link(selected(&tabs, 0).attr(b"href", &mut tab_href)?, section, query, None)?;
    let pagers = Nodes::get(root, b".list_pagination")?;
    if pagers.n == 0 { return if pn == 1 { Ok(false) } else { Err("search page missing pagination") }; }
    if pagers.n != 1 { return Err("ambiguous search pagination"); }
    let current = Nodes::get(root, b".list_pagination a.pagination[aria-current=true]")?;
    if current.n != 1 { return Err("search current page missing"); }
    let mut text = [0; 32];
    if number(selected(&current, 0).text(&mut text)?) != Some(pn) { return Err("search current page mismatch"); }
    let links = Nodes::get(root, b".list_pagination a.pagination[aria-current=true] + a.pagination, .list_pagination button.next[data-url-to]")?;
    if links.n > 1 { return Err("ambiguous search next"); }
    if links.n == 0 { return Ok(false); }
    let node = selected(&links, 0);
    let mut h = [0; 8192];
    let href = match node.optional_attr(b"href", &mut h)? { Some(v) => v, None => node.attr(b"data-url-to", &mut h)? };
    check_search_link(href, section, query, Some(pn.checked_add(1).ok_or("search page overflow")?))?;
    Ok(true)
}

fn list(r: &[u8], search: bool) -> Result<usize, &'static str> {
    let a = input(r)?;
    let pn = if search { 1 } else { page(r)? };
    let step = if search {
        let page=search_page(r)?; let category=search_category(r)?;
        match (category,page) {
            (b"all",p)=>p,
            (b"originals",SearchPage::All)=>SearchPage::Originals(1,false),
            (b"originals",SearchPage::Originals(n,false))=>SearchPage::Originals(n,false),
            (b"canvas",SearchPage::All)=>SearchPage::Canvas(1),
            (b"canvas",SearchPage::Canvas(n))=>SearchPage::Canvas(n),
            _=>return Err("invalid search cursor"),
        }
    } else { SearchPage::All };
    let mut url = [0; 8192];
    let mut u = 0;
    bytes(&mut url, &mut u, BASE)?;
    let mut query_buf = [0; 4096];
    let mut query_len = 0;
    if search {
        let raw = string(a, b"query").ok_or("missing query")?;
        let query = unescape(raw, &mut query_buf)?;
        if trim_ascii(query).is_empty() {
            return Err("empty query");
        };
        query_len = query.len();
        let section = match step { SearchPage::All => b"/zh-hant/search".as_slice(), SearchPage::Originals(_, _) => b"/zh-hant/search/originals", SearchPage::Canvas(_) => b"/zh-hant/search/canvas" };
        bytes(&mut url, &mut u, section)?;
        bytes(&mut url, &mut u, b"?keyword=")?;
        if !write_url_encoded(&mut url, &mut u, query) {
            return Err("payload overflow");
        };
        if let SearchPage::Originals(page, _) | SearchPage::Canvas(page) = step {
            if page > 1 { bytes(&mut url, &mut u, b"&page=")?; if !write_usize(&mut url, &mut u, page) { return Err("payload overflow"); } }
        }
    } else {
        let listing = string(a, b"listingId")
            .or_else(|| string(a, b"listing"))
            .unwrap_or(b"popular");
        if listing != b"popular" {
            return Err("unknown listing");
        };
        let ranking = match pn {
            1 => b"trending".as_slice(),
            2 => b"popular",
            3 => b"originals",
            4 => b"canvas",
            _ => return Err("invalid page cursor"),
        };
        bytes(&mut url, &mut u, b"/zh-hant/ranking/")?;
        bytes(&mut url, &mut u, ranking)?;
    }
    let root = doc(&url[..u])?;
    if search && host::check_cancel() { return Err("cancelled"); }
    let nodes = Nodes::get(&root, b".webtoon_list li a")?;
    if nodes.n == 0 {
        if !search {
            return Err("listing structure missing");
        };
        match step {
            SearchPage::All => { root.select(b".search_wrap, .search_result, .search_area")?; },
            _ => { root.select(b".webtoon_list_wrap")?; },
        }
    }
    let mut next = [0; 32]; let mut next_len = 0;
    if search {
        let query = &query_buf[..query_len];
        match step {
            SearchPage::All => {
                let originals = search_more(&root, b"a._more_area", b"/zh-hant/search/originals", query)?;
                let canvas = search_more(&root, b"a._more_area_canvas", b"/zh-hant/search/canvas", query)?;
                if originals { bytes(&mut next, &mut next_len, if canvas { b"o:1:1" } else { b"o:1:0" })?; }
                else if canvas { bytes(&mut next, &mut next_len, b"c:1")?; }
            },
            SearchPage::Originals(page, canvas) => {
                if search_next(&root, b"/zh-hant/search/originals", query, page)? {
                    bytes(&mut next, &mut next_len, b"o:")?; if !write_usize(&mut next, &mut next_len, page+1) { return Err("payload overflow"); } bytes(&mut next, &mut next_len, if canvas { b":1" } else { b":0" })?;
                } else if canvas { bytes(&mut next, &mut next_len, b"c:1")?; }
            },
            SearchPage::Canvas(page) => {
                if search_next(&root, b"/zh-hant/search/canvas", query, page)? {
                    bytes(&mut next, &mut next_len, b"c:")?; if !write_usize(&mut next, &mut next_len, page+1) { return Err("payload overflow"); }
                }
            },
        }
    }
    let has_more = if search { next_len > 0 } else { pn < 4 };
    if nodes.n == 0 && has_more && !search {
        return Err("empty page has next cursor");
    };
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"items\":[")?;
    for i in 0..nodes.n {
        if search && host::check_cancel() { return Err("cancelled"); }
        let card = selected(&nodes, i);
        let mut h = [0; 4096];
        let h = card.attr(b"href", &mut h)?;
        let (kind, id, path) = manga_route(h)?;
        if search {
            match step {
                SearchPage::Originals(_, _) if kind != b"webtoon" => return Err("search category mismatch"),
                SearchPage::Canvas(_) if kind != b"canvas" => return Err("search category mismatch"),
                _ => (),
            }
        }
        let mut t = [0; 4096];
        let t = card.select(b".title")?.text(&mut t)?;
        if t.is_empty() {
            return Err("empty manga title");
        };
        let mut s = [0; 8192];
        let s = card.select(b"img")?.attr(b"src", &mut s)?;
        image_url(s)?;
        if i > 0 {
            bytes(b, &mut n, b",")?
        };
        bytes(b, &mut n, b"{\"id\":")?;
        mid(b, &mut n, kind, id, path)?;
        bytes(b, &mut n, b",\"title\":")?;
        q(b, &mut n, t)?;
        bytes(
            b,
            &mut n,
            b",\"contentRating\":\"unknown\",\"cover\":{\"kind\":\"url\",\"url\":",
        )?;
        q(b, &mut n, s)?;
        bytes(b, &mut n, b"}}")?;
    }
    bytes(b, &mut n, b"],\"page\":{\"nextCursor\":")?;
    if has_more {
        bytes(b, &mut n, b"\"")?;
        if search { bytes(b, &mut n, &next[..next_len])?; }
        else if !write_usize(b, &mut n, pn + 1) { return Err("payload overflow"); }
        bytes(b, &mut n, b"\",\"hasMore\":true}}")?
    } else {
        bytes(b, &mut n, b"null,\"hasMore\":false}}")?
    };
    Ok(n)
}
fn normalize<'a>(s: &[u8], out: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    let mut n = 0;
    let mut pending = false;
    for &c in trim_ascii(s) {
        if c.is_ascii_whitespace() {
            pending = n > 0
        } else {
            if pending {
                bytes(out, &mut n, b" ")?;
                pending = false
            };
            bytes(out, &mut n, &[c])?
        }
    }
    Ok(&out[..n])
}
fn detail(r: &[u8]) -> Result<usize, &'static str> {
    let a = input(r)?;
    let id = string(a, b"mangaId").ok_or("invalid request")?;
    let mut url = [0; 8192];
    let root = doc(manga_url(id, &mut url)?)?;
    let mut t = [0; 4096];
    let title = root.select(b"h1.subj, h3.subj")?.text(&mut t)?;
    if title.is_empty() {
        return Err("empty manga title");
    };
    let info = root.select(b".detail_header .info")?;
    let mut auth = [0; 8192];
    let mut normalized = [0; 8192];
    let authors = match info
        .select(b".author:nth-of-type(1)")
        .or_else(|_| info.select(b".author_area"))
    {
        Ok(node) => {
            let text = node.text(&mut auth)?;
            let mut suffix = [0; 1024];
            let text = match node.select(b"button") {
                Ok(btn) => {
                    let suffix = btn.text(&mut suffix)?;
                    trim_ascii(
                        text.strip_suffix(suffix)
                            .ok_or("author button structure mismatch")?,
                    )
                }
                Err(_) => text,
            };
            normalize(text, &mut normalized)?
        }
        Err(_) => b"",
    };
    let mut desc = [0; 16384];
    let description = match root.select(b"#_asideDetail p.summary") {
        Ok(node) => node.text(&mut desc)?,
        Err(_) => b"",
    };
    let mut cover = [0; 8192];
    let cover = root
        .select(b"head meta[property=\"og:image\"]")?
        .attr(b"content", &mut cover)?;
    image_url(cover)?;
    let genres = Nodes::get(&info, b".genre")?;
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"manga\":{\"id\":")?;
    q(b, &mut n, id)?;
    bytes(b, &mut n, b",\"title\":")?;
    q(b, &mut n, title)?;
    bytes(b, &mut n, b",\"description\":")?;
    q(b, &mut n, description)?;
    bytes(b, &mut n, b",\"authors\":[")?;
    if !authors.is_empty() {
        q(b, &mut n, authors)?
    };
    bytes(b,&mut n,b"],\"artists\":[],\"language\":\"zh-Hant\",\"status\":\"unknown\",\"contentRating\":\"unknown\",\"tags\":[")?;
    for i in 0..genres.n {
        if i > 0 {
            bytes(b, &mut n, b",")?
        };
        let mut text = [0; 4096];
        q(b, &mut n, selected(&genres, i).text(&mut text)?)?
    }
    bytes(b, &mut n, b"],\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(b, &mut n, cover)?;
    bytes(b, &mut n, b"},\"links\":[{\"kind\":\"source\",\"url\":")?;
    q(b, &mut n, manga_url(id, &mut url)?)?;
    bytes(b, &mut n, b"}]}}")?;
    Ok(n)
}
fn integer64(s: &[u8]) -> Result<&[u8], &'static str> {
    if s.is_empty() || s.len() > 20 {
        return Err("invalid chapter date");
    };
    let mut n = 0u64;
    for &c in s {
        if !c.is_ascii_digit() {
            return Err("invalid chapter date");
        };
        n = n
            .checked_mul(10)
            .and_then(|v| v.checked_add((c - b'0') as u64))
            .ok_or("invalid chapter date")?
    }
    Ok(s)
}
fn explicit_number(title: &[u8]) -> Option<&[u8]> {
    let s = title.strip_prefix("第".as_bytes())?;
    let n = s.iter().take_while(|c| c.is_ascii_digit()).count();
    if n == 0 || n > 9 {
        return None;
    };
    let tail = &s[n..];
    if !(tail.starts_with("話".as_bytes())
        || tail.starts_with("话".as_bytes())
        || tail.starts_with("回".as_bytes())
        || tail.starts_with("章".as_bytes()))
    {
        return None;
    };
    Some(&s[..n])
}
fn chapter_list(r: &[u8]) -> Result<usize, &'static str> {
    let a = input(r)?;
    let id = string(a, b"mangaId").ok_or("invalid request")?;
    let (kind, title, _) = manga_parts(id)?;
    let mut url = [0; 8192];
    let mut u = 0;
    bytes(&mut url, &mut u, MOBILE)?;
    bytes(&mut url, &mut u, b"/api/v1/")?;
    bytes(&mut url, &mut u, kind)?;
    bytes(&mut url, &mut u, b"/")?;
    bytes(&mut url, &mut u, title)?;
    bytes(&mut url, &mut u, b"/episodes?pageSize=99999")?;
    if kind == b"canvas" {
        bytes(&mut url, &mut u, b"&readingLanguageCode=zh-hant")?
    };
    let raw = http(&url[..u], true)?;
    if json_value_end(raw, 0, 0).map(|p| ws(raw, p)) != Some(raw.len()) {
        return Err("invalid catalogue JSON");
    };
    if json_object_member(raw, b"success") == Some(b"false".as_slice()) {
        return Err("catalogue API rejected");
    };
    let result = json_object_member(raw, b"result").ok_or("catalogue result missing")?;
    let cursor = json_object_member(result, b"nextCursor").ok_or("catalogue terminal missing")?;
    if cursor != b"0" && cursor != b"null" {
        return Err("catalogue incomplete");
    };
    let array = json_object_member(result, b"episodeList").ok_or("catalogue missing")?;
    if array.first() != Some(&b'[') {
        return Err("catalogue invalid array");
    };
    let mut ranges = [(0usize, 0usize); 2048];
    let mut count = 0;
    let mut at = ws(array, 1);
    while array.get(at) != Some(&b']') {
        if count == 2048 {
            return Err("too many chapters");
        };
        let end = json_value_end(array, at, 0).ok_or("invalid chapter JSON")?;
        if array.get(at) != Some(&b'{') {
            return Err("invalid chapter object");
        };
        ranges[count] = (at, end);
        count += 1;
        at = ws(array, end);
        if array.get(at) == Some(&b']') {
            break;
        };
        if array.get(at) != Some(&b',') {
            return Err("invalid chapter array");
        };
        at = ws(array, at + 1)
    }
    if count == 0 {
        return Err("empty catalogue");
    };
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"chapters\":[")?;
    for i in 0..count {
        let (start, end) = ranges[count - i - 1];
        let item = &array[start..end];
        let mut name = [0; 4096];
        let name = unescape(
            string(item, b"episodeTitle").ok_or("missing chapter title")?,
            &mut name,
        )?;
        if name.is_empty() {
            return Err("empty chapter title");
        };
        let mut path = [0; 8192];
        let path = unescape(
            string(item, b"viewerLink").ok_or("missing viewerLink")?,
            &mut path,
        )?;
        let (t, e, p) = chapter_route(path)?;
        if t != title || json_object_member(item, b"episodeNo").and_then(number) != number(e) {
            return Err("chapter identity mismatch");
        };
        let date = integer64(
            json_object_member(item, b"exposureDateMillis").ok_or("invalid chapter date")?,
        )?;
        if i > 0 {
            bytes(b, &mut n, b",")?
        };
        bytes(b, &mut n, b"{\"id\":\"chapter:webtoons:zh-Hant:")?;
        if !append_json_escaped(b, &mut n, p) {
            return Err("payload overflow");
        };
        bytes(b, &mut n, b"\",\"mangaId\":")?;
        q(b, &mut n, id)?;
        bytes(b, &mut n, b",\"title\":")?;
        q(b, &mut n, name)?;
        bytes(b, &mut n, b",\"index\":")?;
        if !write_usize(b, &mut n, i) {
            return Err("payload overflow");
        };
        bytes(b, &mut n, b",\"chapterNumber\":")?;
        // Explicit opt-in follows the upstream pre-reversal index + 1.
        // Default retains the accepted title-derived/null metadata.
        if setting_enabled(r, b"useSequentialNumbering") {
            if !write_usize(b, &mut n, count - i) { return Err("payload overflow"); }
        } else { match explicit_number(name) {
            Some(v) => {
                if !write_usize(b, &mut n, number(v).ok_or("invalid chapter number")?) {
                    return Err("payload overflow");
                }
            }
            None => bytes(b, &mut n, b"null")?,
        } };
        bytes(b, &mut n, b",\"language\":\"zh-Hant\",\"publishedAt\":")?;
        bytes(b, &mut n, date)?;
        bytes(b, &mut n, b",\"uploadDate\":")?;
        bytes(b, &mut n, date)?;
        bytes(b, &mut n, b"}")?;
    }
    bytes(
        b,
        &mut n,
        b"],\"page\":{\"nextCursor\":null,\"hasMore\":false}}",
    )?;
    Ok(n)
}
// Settings are request-scoped. Keep the delivered URI unchanged by default;
// the opt-in follows Keiyoushi Webtoons: remove only a complete type=q90 query
// pair, preserving all other parameters, escapes and the fragment byte-for-byte.
fn maximum_quality_uri<'a>(uri: &'a [u8], enabled: bool, out: &'a mut [u8]) -> &'a [u8] {
    if !enabled { return uri; }
    let Some(query) = uri.iter().position(|b| *b == b'?') else { return uri; };
    let end = uri.iter().position(|b| *b == b'#').unwrap_or(uri.len());
    if query >= end { return uri; }
    let mut n = query;
    out[..query].copy_from_slice(&uri[..query]);
    let mut first = true;
    for pair in uri[query + 1..end].split(|b| *b == b'&') {
        if pair == b"type=q90" { continue; }
        out[n] = if first { b'?' } else { b'&' }; n += 1;
        out[n..n + pair.len()].copy_from_slice(pair); n += pair.len();
        first = false;
    }
    out[n..n + uri.len() - end].copy_from_slice(&uri[end..]);
    n += uri.len() - end;
    &out[..n]
}
fn supports_text_pages(r: &[u8]) -> bool {
    let Some(features) = json_object_member(r, b"hostFeatures") else { return false; };
    let features = trim_ascii(features);
    features.starts_with(b"[") && features.ends_with(b"]") &&
        features[1..features.len() - 1].split(|b| *b == b',')
            .any(|value| trim_ascii(value) == b"\"text_page_image_v1\"")
}
fn setting_enabled(r: &[u8], key: &[u8]) -> bool {
    json_object_member(r, b"settings")
        .and_then(|s| json_object_member(s, key))
        .map(|v| v == b"true" || v == b"\"true\"").unwrap_or(false)
}
fn page_list(r: &[u8]) -> Result<usize, &'static str> {
    let path = chapters_id(r)?;
    let (title, episode, _) = chapter_route(path)?;
    let mut url = [0; 8192];
    let root = doc(make_url(path, &mut url)?)?;
    let nodes = Nodes::get(&root, b"div#_imageList > img")?;
    let maximum_quality = setting_enabled(r, b"useMaxQuality");
    if nodes.n == 0 {
        return Err("ordinary viewer images missing or gated");
    };
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"chapterId\":")?;
    q(
        b,
        &mut n,
        string(input(r)?, b"chapterId").ok_or("invalid request")?,
    )?;
    bytes(b, &mut n, b",\"pages\":[")?;
    for i in 0..nodes.n {
        let mut src = [0; 8192];
        let src = selected(&nodes, i).attr(b"data-url", &mut src)?;
        image_url(src)?;
        let mut quality_uri = [0; 8192];
        let src = maximum_quality_uri(src, maximum_quality, &mut quality_uri);
        if i > 0 {
            bytes(b, &mut n, b",")?
        };
        bytes(b, &mut n, b"{\"id\":\"page:webtoons:zh-Hant:")?;
        bytes(b, &mut n, title)?;
        bytes(b, &mut n, b":")?;
        bytes(b, &mut n, episode)?;
        bytes(b, &mut n, b":")?;
        if !write_usize(b, &mut n, i) {
            return Err("payload overflow");
        };
        bytes(b, &mut n, b"\",\"index\":")?;
        if !write_usize(b, &mut n, i) {
            return Err("payload overflow");
        };
        bytes(b, &mut n, b",\"image\":{\"kind\":\"url\",\"url\":")?;
        q(b, &mut n, src)?;
        bytes(b, &mut n, b"}}")?
    }
    // Opt-in local text descriptor requires the host's explicit render capability.
    // Old Apps continue receiving only the established URL pages.
    if supports_text_pages(r) && setting_enabled(r, b"showAuthorsNotes") {
        let notes = Nodes::get(&root, b"div.creator_note p.author_text")?;
        if notes.n > 0 {
            let mut note = [0; 65536];
            let note = selected(&notes, 0).text(&mut note)?;
            if !note.is_empty() {
                let names = Nodes::get(&root, b"div.creator_note .author_name span")?;
                let mut author = [0; 4096];
                let author = if names.n > 0 { selected(&names, 0).text(&mut author)? } else { b"" };
                bytes(b, &mut n, b",{\"id\":\"page:webtoons:zh-Hant:")?;
                bytes(b, &mut n, title)?; bytes(b, &mut n, b":")?;
                bytes(b, &mut n, episode)?; bytes(b, &mut n, b":author-notes\",\"index\":")?;
                if !write_usize(b, &mut n, nodes.n) { return Err("payload overflow"); }
                bytes(b, &mut n, b",\"image\":{\"kind\":\"text\",\"text\":{\"template\":\"webtoon-author-notes\",\"heading\":\"")?;
                bytes(b, &mut n, b"Author's Notes from ")?;
                if !append_json_escaped(b, &mut n, author) { return Err("payload overflow"); }
                bytes(b, &mut n, b"\",\"paragraphs\":[")?; q(b, &mut n, note)?;
                bytes(b, &mut n, b"]}}}")?;
            }
        }
    }
    bytes(b, &mut n, b"]}")?;
    Ok(n)
}
fn image_request(r: &[u8]) -> Result<usize, &'static str> {
    let a = input(r)?;
    let raw = string(a, b"url").ok_or("missing image URL")?;
    let mut decoded = [0; 8192];
    let url = unescape(raw, &mut decoded)?;
    image_url(url)?;
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"url\":")?;
    // Serialize literal ASCII spaces only for transport; stored image URI stays unchanged.
    let mut transport = [0; 24576];
    let mut len = 0;
    for &ch in url {
        if ch == b' ' {
            bytes(&mut transport, &mut len, b"%20")?;
        } else {
            bytes(&mut transport, &mut len, &[ch])?;
        }
    }
    q(b, &mut n, &transport[..len])?;
    bytes(b,&mut n,b",\"headers\":{\"User-Agent\":\"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0\",\"Referer\":\"https://www.webtoons.com/\",\"Origin\":\"https://www.webtoons.com\"}}")?;
    Ok(n)
}
fn run_search(r: &[u8]) -> u32 {
    if host::check_cancel() { return write_error("search", "cancelled", ""); }
    execute("search", list(r, true))
}
fn run_get_manga_list(r: &[u8]) -> u32 {
    execute("get_manga_list", list(r, false))
}
fn run_get_manga(r: &[u8]) -> u32 {
    execute("get_manga", detail(r))
}
fn run_get_chapters(r: &[u8]) -> u32 {
    execute("get_chapters", chapter_list(r))
}
fn run_get_pages(r: &[u8]) -> u32 {
    execute("get_pages", page_list(r))
}
fn run_get_image_request(r: &[u8]) -> u32 {
    execute("get_image_request", image_request(r))
}
fn run_get_listings(_: &[u8]) -> u32 {
    let p = "{\"listings\":[{\"id\":\"popular\",\"name\":\"熱門\",\"supportsPaging\":true}]}"
        .as_bytes();
    payload_buf()[..p.len()].copy_from_slice(p);
    write_success_payload("get_listings", p.len())
}
fn run_get_home(_: &[u8]) -> u32 {
    write_error("get_home", "unsupported", "home unsupported")
}
fn run_get_filters(r: &[u8]) -> u32 {
    let data=if search_filter_supported(r) { br#"{"filters":[{"id":"searchType","name":"Search Type","type":"select","scope":"search","default":"all","options":[{"value":"all","label":"ALL"},{"value":"originals","label":"Originals"},{"value":"canvas","label":"Canvas"}]}]}"#.as_slice() } else { br#"{"filters":[]}"#.as_slice() };
    payload_buf()[..data.len()].copy_from_slice(data);write_success_payload("get_filters",data.len())
}
fn run_get_settings(r: &[u8]) -> u32 {
    let b = payload_buf(); let mut n = 0;
    let quality = "{\"settings\":[{\"id\":\"useMaxQuality\",\"name\":\"最高畫質圖片\",\"kind\":\"toggle\",\"description\":\"移除網站 q90 壓縮參數；對之後載入的章節生效\",\"default\":\"false\"}";
    if !write_bytes(b, &mut n, quality.as_bytes()) { return write_error("get_settings", "internal_error", "payload overflow"); }
    let numbering = ",{\"id\":\"useSequentialNumbering\",\"name\":\"使用連續章節編號\",\"kind\":\"toggle\",\"description\":\"使用網站目錄順序編號，保留章節標題與排列\",\"default\":\"false\"}";
    if !write_bytes(b, &mut n, numbering.as_bytes()) { return write_error("get_settings", "internal_error", "payload overflow"); }
    if supports_text_pages(r) {
        let note = ",{\"id\":\"showAuthorsNotes\",\"name\":\"作者備註\",\"kind\":\"toggle\",\"description\":\"在章末顯示網站提供的作者備註\",\"default\":\"false\"}";
        if !write_bytes(b, &mut n, note.as_bytes()) { return write_error("get_settings", "internal_error", "payload overflow"); }
    }
    if !write_bytes(b, &mut n, b"]}") { return write_error("get_settings", "internal_error", "payload overflow"); }
    write_success_payload("get_settings", n)
}
koma_source_sdk::koma_source_exports!("webtoons_zh_hant");
