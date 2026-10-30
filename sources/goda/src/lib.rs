#![no_std]
// Public contract: Keiyoushi GoDaManhua.kt / GoDa.kt / Dto.kt, Apache-2.0, revision
// 1319949ca9bc456908196e3ba2d104e94182c5d6. No authentication/mirror/JS processing.
use koma_source_sdk::{
    host,
    json_utils::*,
    source::{SourceCapabilities, SourceInfo},
};
const BASE: &[u8] = b"https://baozimh.org";
const MIRRORS: [&[u8]; 6] = [BASE, b"https://godamh.com", b"https://m.baozimh.one", b"https://bzmh.org", b"https://g-mh.org", b"https://m.g-mh.org"];
// This API mirror choice does not change the separately declared godamh.com homepage.
// Unknown values preserve the original API origin and never expand permitted hosts.
fn selected_base(r: &[u8]) -> &'static [u8] {
    let chosen = json_object_member(r, b"settings").and_then(|v| extract_json_string(v, b"preferredMirror"));
    MIRRORS.iter().copied().find(|v| Some(*v) == chosen).unwrap_or(BASE)
}
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "org.goda.koma",
    name: "GoDa漫画",
    version: "0.1.5",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "GoDa public source. Upstream SAFE; individual content and usage unknown.",
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
koma_source_sdk::koma_source_buffers! { payload:1024*1024,http_out:2*1024*1024,body:1024*1024,http_req:8192,scratch:8192, }
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
    if write_bytes(&mut out, &mut n, b"goda stage=")
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
            if n > 1024 * 1024 - 512 {
                diagnostic(b"operation_error", b"internal_error");
                return write_error(op,"internal_error","payload exceeds native limit");
            }
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
fn fetch_body(base: &[u8], path: &[u8], site: &[u8]) -> Result<usize, &'static str> {
    host::log_info(b"goda stage=fetch_begin");
    let mut url = [0; 8192];
    let mut n = 0;
    bytes(&mut url, &mut n, base)?;
    bytes(&mut url, &mut n, path)?;
    let mut referer = [0; 256];
    let mut rn = 0;
    bytes(&mut referer, &mut rn, site)?;
    bytes(&mut referer, &mut rn, b"/")?;
    let len = koma_source_sdk::build_get_request(
        http_req_buf(),
        &url[..n],
        Some(&referer[..rn]),
        &[(b"Origin", site)],
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
    if write_bytes(&mut out, &mut n, b"goda stage=http_status value=")
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
    host::log_info(b"goda stage=body_ready");
    Ok(n)
}
const API: &[u8] = b"https://api-get-v3.mgsearcher.com";
const IMAGE: &[u8] = b"https://c-nd2-1.6wm.top";
fn fetch(base: &[u8], path: &[u8]) -> Result<Node, &'static str> {
    let n = fetch_body(base, path, base)?;
    Node::parse(&body_buf()[..n])
}
fn valid_path(s: &[u8]) -> bool {
    if s.is_empty()
        || s.len() >= 2048
        || s.starts_with(b"//")
        || s.split(|c| *c == b'/').any(|c| c == b"." || c == b"..")
    {
        return false;
    };
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        if c == b'%' {
            if i + 2 >= s.len() {
                return false;
            };
            let Some(v) = hex4(&[b'0', b'0', s[i + 1], s[i + 2]]) else {
                return false;
            };
            if v < 32 || v == 127 || v == 92 || v == 47 {
                return false;
            };
            i += 3
        } else {
            if !c.is_ascii_alphanumeric() && !b"/._-".contains(&c) {
                return false;
            };
            i += 1
        }
    }
    true
}
fn slug(s: &[u8]) -> Result<&[u8], &'static str> {
    let s = MIRRORS.iter().find_map(|base| s.strip_prefix(*base)).unwrap_or(s);
    let s = s.strip_prefix(b"/manga/").unwrap_or(s);
    let s = s.strip_suffix(b"/").unwrap_or(s);
    if !valid_path(s) || s.contains(&b'/') {
        return Err("invalid request");
    }
    Ok(s)
}
fn manga(r: &[u8]) -> Result<(&[u8], &[u8]), &'static str> {
    let id = string(args(r), b"mangaId").ok_or("invalid request")?;
    let s = id.strip_prefix(b"goda:").ok_or("invalid request")?;
    Ok((id, slug(s)?))
}
fn path<'a>(s: &[u8], o: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    let mut n = 0;
    bytes(o, &mut n, b"/manga/")?;
    bytes(o, &mut n, s)?;
    Ok(&o[..n])
}
fn json_complete(s: &[u8]) -> Result<(), &'static str> {
    if json_value_end(s, 0, 0).map(|i| ws(s, i)) != Some(s.len()) {
        Err("invalid JSON")
    } else {
        Ok(())
    }
}
fn member<'a>(s: &'a [u8], k: &[u8]) -> Result<&'a [u8], &'static str> {
    json_object_member(s, k).ok_or("missing JSON member")
}
fn decoded<'a>(s: &[u8], k: &[u8], o: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    unescape(string(s, k).ok_or("missing JSON string")?, o)
}
fn api_data(s: &[u8]) -> Result<&[u8], &'static str> {
    json_complete(s)?;
    if member(s, b"code")? != b"200" || member(s, b"status")? != b"true" {
        return Err("API rejected");
    };
    member(s, b"data")
}
fn append_paging(o: &mut [u8], n: &mut usize, has: bool, p: usize) -> Result<(), &'static str> {
    bytes(o, n, b"],\"page\":{\"nextCursor\":")?;
    if has {
        bytes(o, n, b"\"")?;
        if !write_usize(o, n, p.checked_add(1).ok_or("invalid page cursor")?) {
            return Err("payload overflow");
        };
        bytes(o, n, b"\",\"hasMore\":true}}")?
    } else {
        bytes(o, n, b"null,\"hasMore\":false}}")?
    }
    Ok(())
}
fn valid_image(s: &[u8]) -> Result<(), &'static str> {
    let relative = s.strip_prefix(b"https://c-nc-1.6wm.top").or_else(||s.strip_prefix(b"https://c-nd2-1.6wm.top")).ok_or("invalid image URL")?;
    if !relative.starts_with(b"/") || !valid_path(relative) {return Err("invalid image URL")};

    if !(s.starts_with(b"https://c-nc-1.6wm.top/") || s.starts_with(b"https://c-nd2-1.6wm.top/"))
        || s.iter()
            .any(|c| *c <= 32 || *c == 127 || *c == b'\\' || *c == b'"')
        || s.len() >= 4096
    {
        return Err("invalid image URL");
    };
    Ok(())
}
fn percent<'a>(s: &[u8], o: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    let mut i = 0;
    let mut n = 0;
    while i < s.len() {
        let c = if s[i] == b'%' {
            let h = hex4(&[
                b'0',
                b'0',
                *s.get(i + 1).ok_or("invalid escape")?,
                *s.get(i + 2).ok_or("invalid escape")?,
            ])
            .ok_or("invalid escape")?;
            i += 3;
            h as u8
        } else {
            let c = s[i];
            i += 1;
            c
        };
        bytes(o, &mut n, &[c])?
    }
    core::str::from_utf8(&o[..n]).map_err(|_| "invalid UTF8")?;
    Ok(&o[..n])
}
fn cover_url<'a>(s: &[u8], o: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    if let Some(i) = find_subslice(s, b"url=") {
        let q = &s[i + 4..];
        let end = q.iter().position(|c| *c == b'&').unwrap_or(q.len());
        let v = percent(&q[..end], o)?;
        valid_image(v)?;
        Ok(v)
    } else {
        valid_image(s)?;
        o[..s.len()].copy_from_slice(s);
        Ok(&o[..s.len()])
    }
}
fn list(r: &[u8], search: bool) -> Result<usize, &'static str> {
    let pg = page(r)?;
    let mut route = [0; 8192];
    let mut n = 0;
    if search {
        let mut qb = [0; 4096];
        let query = decoded(args(r), b"query", &mut qb)?;
        if trim_ascii(query).is_empty() {
            return Err("empty query");
        };
        bytes(&mut route, &mut n, b"/s/")?;
        if !write_url_encoded(&mut route, &mut n, query) {
            return Err("invalid request");
        };
        bytes(&mut route, &mut n, b"?page=")?
    } else {
        match string(args(r), b"listingId").unwrap_or(b"popular") {
            b"popular" => bytes(&mut route, &mut n, b"/hots/page/")?,
            b"latest" => bytes(&mut route, &mut n, b"/newss/page/")?,
            _ => return Err("unknown listing"),
        }
    }
    if !write_usize(&mut route, &mut n, pg) {
        return Err("invalid page cursor");
    };
    let root = fetch(selected_base(r), &route[..n])?;
    let nodes = Nodes::get(&root, b".container > .cardlist .pb-2 a")?;
    if nodes.n == 0 {
        return Err("empty catalogue HTML");
    };
    let has = root
        .select("a[aria-label='下一頁'] button".as_bytes())
        .is_ok();
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"items\":[")?;
    for i in 0..nodes.n {
        let a = selected(&nodes, i);
        let mut hb = [0; 2048];
        let s = slug(a.attr(b"href", &mut hb)?)?;
        let h = a.select(b"h3")?;
        let mut tb = [0; 4096];
        let title = h.text(&mut tb)?;
        if title.is_empty() {
            return Err("missing title");
        };
        let im = a.select(b"img")?;
        let mut ib = [0; 4096];
        let mut url = [0; 4096];
        let cover = cover_url(im.attr(b"src", &mut ib)?, &mut url)?;
        if i > 0 {
            bytes(out, &mut n, b",")?
        };
        bytes(out, &mut n, b"{\"id\":\"goda:")?;
        if !append_json_escaped(out, &mut n, s) {
            return Err("payload overflow");
        };
        bytes(out, &mut n, b"\",\"title\":")?;
        q(out, &mut n, title)?;
        bytes(out, &mut n, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
        q(out, &mut n, cover)?;
        bytes(out, &mut n, b"},\"contentRating\":\"safe\"}")?
    }
    append_paging(out, &mut n, has, pg)?;
    Ok(n)
}
fn detail(r: &[u8]) -> Result<usize, &'static str> {
    let (id, s) = manga(r)?;
    let mut pb = [0; 2048];
    let root = fetch(selected_base(r), path(s, &mut pb)?)?;
    let main = root.select(b"main")?;
    let h = main.select(b"h1")?;
    let status_nodes = Nodes::get(&h, b"span")?;
    if status_nodes.n != 1 {
        return Err("ambiguous title status");
    }
    let st = h.select(b"span")?;
    let mut sb = [0; 128];
    let status_text = st.text(&mut sb)?;
    let mut tb = [0; 4096];
    let full = h.text(&mut tb)?;
    let title = trim_ascii(
        full.strip_suffix(status_text)
            .ok_or("title status mismatch")?,
    );
    if title.is_empty() {
        return Err("missing title");
    };
    let status = match status_text {
        v if v == "連載中".as_bytes() || v == b"Ongoing" => b"ongoing".as_slice(),
        v if v == "完結".as_bytes() => b"completed",
        v if v == "停止更新".as_bytes() => b"cancelled",
        v if v == "休刊".as_bytes() => b"hiatus",
        _ => b"unknown",
    };
    let info = main.select(b"div.block.text-left.mx-auto")?;
    let author = Nodes::get(&info, b"div:nth-child(2) > a")?;
    let genre = Nodes::get(&info, b"div:nth-child(3) > a, div:nth-child(4) > a")?;
    let desc = info.select(b"p:nth-child(5)")?;
    let mut db = [0; 32768];
    let description = desc.text(&mut db)?;
    let mid = root.select(b"#mangachapters")?;
    let mut mb = [0; 64];
    let mid = mid.attr(b"data-mid", &mut mb)?;
    number(mid).ok_or("invalid manga id")?;
    let im = main.select(b"img.object-cover")?;
    let mut ib = [0; 4096];
    let cover = im.attr(b"src", &mut ib)?;
    valid_image(cover)?;
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"manga\":{\"id\":")?;
    q(out, &mut n, id)?;
    bytes(out, &mut n, b",\"title\":")?;
    q(out, &mut n, title)?;
    bytes(out, &mut n, b",\"status\":")?;
    q(out, &mut n, status)?;
    bytes(
        out,
        &mut n,
        b",\"contentRating\":\"safe\",\"cover\":{\"kind\":\"url\",\"url\":",
    )?;
    q(out, &mut n, cover)?;
    bytes(out, &mut n, b"},\"authors\":[")?;
    for i in 0..author.n {
        let node = selected(&author, i);
        let mut b = [0; 2048];
        let t = node.text(&mut b)?;
        if i > 0 {
            bytes(out, &mut n, b",")?
        }
        q(out, &mut n, t.strip_suffix(b" ,").unwrap_or(t))?
    }
    bytes(out, &mut n, b"],\"tags\":[")?;
    for i in 0..genre.n {
        let node = selected(&genre, i);
        let mut b = [0; 2048];
        let t = node.text(&mut b)?;
        if i > 0 {
            bytes(out, &mut n, b",")?
        };
        let t = t.strip_prefix(b"#").unwrap_or(t);
        q(out, &mut n, t.strip_suffix(b" ,").unwrap_or(t))?
    }
    bytes(out, &mut n, b"],\"description\":\"")?;
    if !append_json_escaped(out, &mut n, description) {
        return Err("payload overflow");
    };
    bytes(out, &mut n, b"\\n\\nID: ")?;
    bytes(out, &mut n, mid)?;
    bytes(
        out,
        &mut n,
        b"\",\"links\":[{\"kind\":\"source\",\"url\":\"",
    )?;
    bytes(out, &mut n, selected_base(r))?;
    bytes(out, &mut n, b"/manga/")?;
    bytes(out, &mut n, s)?;
    bytes(out, &mut n, b"\"}]}}")?;
    Ok(n)
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
fn explicit_number(t: &[u8]) -> Option<usize> {
    let t = t.strip_prefix("第".as_bytes()).unwrap_or(t);
    let d = t.iter().take_while(|c| c.is_ascii_digit()).count();
    if d == 0 {
        return None;
    };
    let tail = &t[d..];
    if tail.is_empty()
        || tail.first() == Some(&b' ')
        || ["话", "回", "章"]
            .iter()
            .any(|u| tail.starts_with(u.as_bytes()))
    {
        number(&t[..d])
    } else {
        None
    }
}
fn chapters(r: &[u8]) -> Result<usize, &'static str> {
    let (id, s) = manga(r)?;
    let mut pb = [0; 2048];
    let root = fetch(selected_base(r), path(s, &mut pb)?)?;
    let node = root.select(b"#mangachapters")?;
    let mut mb = [0; 64];
    let mid = node.attr(b"data-mid", &mut mb)?;
    number(mid).ok_or("invalid manga id")?;
    drop(node);
    drop(root);
    let mut url = [0; 256];
    let mut u = 0;
    bytes(&mut url, &mut u, b"/api/manga/get?mid=")?;
    bytes(&mut url, &mut u, mid)?;
    bytes(&mut url, &mut u, b"&mode=all")?;
    let len = fetch_body(API, &url[..u], selected_base(r))?;
    let data = api_data(&body_buf()[..len])?;
    let mut db = [0; 64];
    if decoded(data, b"id", &mut db)? != mid {
        return Err("manga id mismatch");
    };
    let mut slugb = [0; 2048];
    if decoded(data, b"slug", &mut slugb)? != s {
        return Err("manga slug mismatch");
    };
    let a = member(data, b"chapters")?;
    let mut spans = [(0, 0); 8192];
    let count = array_spans(a, &mut spans)?;
    if count == 0 {
        return Err("empty chapters");
    };
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"chapters\":[")?;
    for (ordinal, &(start, end)) in spans[..count].iter().rev().enumerate() {
        let c = &a[start..end];
        let at = member(c, b"attributes")?;
        let mut cb = [0; 64];
        let cid = decoded(c, b"id", &mut cb)?;
        number(cid).ok_or("invalid chapter id")?;
        let mut sb = [0; 2048];
        let cs = decoded(at, b"slug", &mut sb)?;
        if !valid_path(cs) || cs.contains(&b'/') {
            return Err("invalid chapter slug");
        };
        let mut tb = [0; 4096];
        let title = decoded(at, b"title", &mut tb)?;
        if title.is_empty() {
            return Err("missing title");
        };
        if ordinal > 0 {
            bytes(out, &mut n, b",")?
        };
        bytes(out, &mut n, b"{\"id\":\"godach:")?;
        bytes(out, &mut n, s)?;
        bytes(out, &mut n, b"/")?;
        bytes(out, &mut n, cs)?;
        bytes(out, &mut n, b"#")?;
        bytes(out, &mut n, mid)?;
        bytes(out, &mut n, b"/")?;
        bytes(out, &mut n, cid)?;
        bytes(out, &mut n, b"\",\"mangaId\":")?;
        q(out, &mut n, id)?;
        bytes(out, &mut n, b",\"title\":")?;
        q(out, &mut n, title)?;
        bytes(out, &mut n, b",\"chapterNumber\":")?;
        if let Some(v) = explicit_number(title) {
            if !write_usize(out, &mut n, v) {
                return Err("payload overflow");
            }
        } else {
            bytes(out, &mut n, b"null")?
        };
        bytes(out, &mut n, b",\"index\":")?;
        if !write_usize(out, &mut n, ordinal) {
            return Err("payload overflow");
        };
        bytes(out, &mut n, b",\"language\":\"zh\"}")?
    }
    bytes(
        out,
        &mut n,
        b"],\"page\":{\"nextCursor\":null,\"hasMore\":false}}",
    )?;
    Ok(n)
}
static mut DECODED: [u8; 1024 * 1024] = [0; 1024 * 1024];
static mut REORDER: [u8; 1024 * 1024] = [0; 1024 * 1024];
fn decode_images(s: &[u8]) -> Result<&'static [u8], &'static str> {
    let body = s
        .strip_prefix(b"J7r")
        .and_then(|s| s.strip_suffix(b"nQ"))
        .ok_or("invalid image markers")?;
    let plen = body
        .len()
        .checked_sub(5)
        .filter(|n| *n > 0)
        .ok_or("invalid image markers")?;
    let a = plen / 3;
    let b = (plen - a) / 2;
    let c = plen - a - b;
    if body.get(b..b + 2) != Some(b"kD") || body.get(b + 2 + c..b + 2 + c + 3) != Some(b"W4s") {
        return Err("invalid image markers");
    };
    let out = unsafe { &mut *core::ptr::addr_of_mut!(REORDER) };
    let mut n = 0;
    bytes(out, &mut n, &body[b + 2 + c + 3..])?;
    bytes(out, &mut n, &body[..b])?;
    bytes(out, &mut n, &body[b + 2..b + 2 + c])?;
    for (i, chunk) in out[..n].chunks_mut(7).enumerate() {
        if i % 2 == 1 {
            chunk.reverse()
        }
    }
    let custom = b"_-9876543210abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    for x in &mut out[..n] {
        *x = custom
            .iter()
            .position(|c| c == x)
            .ok_or("invalid image alphabet")? as u8
    }
    if n % 4 == 1 {
        return Err("invalid Base64 length");
    };
    let decoded = unsafe { &mut *core::ptr::addr_of_mut!(DECODED) };
    let mut p = 0;
    let mut i = 0;
    while i < n {
        let len = (n - i).min(4);
        let v = &out[i..i + len];
        if len < 2 {
            return Err("invalid Base64 length");
        };
        bytes(decoded, &mut p, &[(v[0] << 2) | (v[1] >> 4)])?;
        if len > 2 {
            bytes(decoded, &mut p, &[(v[1] << 4) | (v[2] >> 2)])?
        } else if v[1] & 15 != 0 {
            return Err("invalid Base64 padding");
        };
        if len > 3 {
            bytes(decoded, &mut p, &[(v[2] << 6) | v[3]])?
        } else if len == 3 && v[2] & 3 != 0 {
            return Err("invalid Base64 padding");
        };
        i += len
    }
    core::str::from_utf8(&decoded[..p]).map_err(|_| "invalid UTF8")?;
    json_complete(&decoded[..p])?;
    Ok(&decoded[..p])
}
fn pages(r: &[u8]) -> Result<usize, &'static str> {
    let id = string(args(r), b"chapterId").ok_or("invalid request")?;
    let tail = id.strip_prefix(b"godach:").ok_or("invalid request")?;
    let sep = tail
        .iter()
        .position(|c| *c == b'#')
        .ok_or("invalid request")?;
    let key = &tail[..sep];
    let keyslash = key
        .iter()
        .position(|c| *c == b'/')
        .ok_or("invalid request")?;
    let manga_slug = slug(&key[..keyslash])?;
    let chapter_slug = &key[keyslash + 1..];
    if !valid_path(chapter_slug) || chapter_slug.contains(&b'/') {
        return Err("invalid request");
    };
    let identity = &tail[sep + 1..];
    let slash = identity
        .iter()
        .position(|c| *c == b'/')
        .ok_or("invalid request")?;
    let (mid, cid) = (&identity[..slash], &identity[slash + 1..]);
    number(mid).ok_or("invalid request")?;
    number(cid).ok_or("invalid request")?;
    let mut url = [0; 256];
    let mut u = 0;
    bytes(&mut url, &mut u, b"/api/v2/chapter/getinfo?m=")?;
    bytes(&mut url, &mut u, mid)?;
    bytes(&mut url, &mut u, b"&c=")?;
    bytes(&mut url, &mut u, cid)?;
    let len = fetch_body(API, &url[..u], selected_base(r))?;
    let data = api_data(&body_buf()[..len])?;
    let info = member(data, b"info")?;
    let mut mb = [0; 64];
    if decoded(info, b"mid", &mut mb)? != mid {
        return Err("manga id mismatch");
    };
    let mut cb = [0; 64];
    if decoded(info, b"id", &mut cb)? != cid {
        return Err("chapter id mismatch");
    };
    let mut csb = [0; 2048];
    if decoded(info, b"slug", &mut csb)? != manga_slug {
        return Err("manga slug mismatch");
    };
    let imgs = member(info, b"images")?;
    let mut encoded = [0; 32768];
    let s = decoded(imgs, b"images", &mut encoded)?;
    let a = decode_images(s)?;
    let mut spans = [(0, 0); 2048];
    let count = array_spans(a, &mut spans)?;
    if count == 0 {
        return Err("empty pages");
    };
    let out = payload_buf();
    let mut n = 0;
    bytes(out, &mut n, b"{\"pages\":[")?;
    for (index, &(start, end)) in spans[..count].iter().enumerate() {
        let v = &a[start..end];
        let order = number(member(v, b"order")?).ok_or("invalid page order")?;
        let _ = order; // API order is data, never a host index or sort key.
        let mut ub = [0; 4096];
        let original = decoded(v, b"url", &mut ub)?;
        // The identity-checked chapter API owns this root-relative asset path.
        // Do not infer its storage prefix or folder name from the manga slug.
        if !original.starts_with(b"/") || !valid_path(original) {
            return Err("invalid image URL");
        };
        let mut abs = [0; 4096];
        let mut c = 0;
        bytes(&mut abs, &mut c, IMAGE)?;
        bytes(&mut abs, &mut c, original)?;
        if index > 0 {
            bytes(out, &mut n, b",")?
        };
        bytes(out, &mut n, b"{\"id\":\"godapage:")?;
        bytes(out, &mut n, tail)?;
        bytes(out, &mut n, b":")?;
        if !write_usize(out, &mut n, index) {
            return Err("payload overflow");
        };
        bytes(out, &mut n, b"\",\"image\":{\"kind\":\"url\",\"url\":")?;
        q(out, &mut n, &abs[..c])?;
        bytes(out, &mut n, b"},\"index\":")?;
        if !write_usize(out, &mut n, index) {
            return Err("payload overflow");
        };
        bytes(out, &mut n, b"}")?
    }
    bytes(out, &mut n, b"]}")?;
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
fn run_get_listings(_: &[u8]) -> u32 {
    let p=b"{\"listings\":[{\"id\":\"popular\",\"name\":\"Popular\"},{\"id\":\"latest\",\"name\":\"Latest\"}]}";
    payload_buf()[..p.len()].copy_from_slice(p);
    write_success_payload("get_listings", p.len())
}
fn run_get_home(_: &[u8]) -> u32 {
    write_error("get_home", "not_implemented", "not supported")
}
fn run_get_filters(_: &[u8]) -> u32 {
    write_error("get_filters", "not_implemented", "not supported")
}
fn run_get_settings(_: &[u8]) -> u32 {
    let p = r#"{"settings":[{"id":"preferredMirror","name":"首选镜像","kind":"select","default":"https://baozimh.org","options":[{"value":"https://baozimh.org","label":"baozimh.org"},{"value":"https://godamh.com","label":"godamh.com"},{"value":"https://m.baozimh.one","label":"m.baozimh.one"},{"value":"https://bzmh.org","label":"bzmh.org"},{"value":"https://g-mh.org","label":"g-mh.org"},{"value":"https://m.g-mh.org","label":"m.g-mh.org"}]}]}"#.as_bytes();
    payload_buf()[..p.len()].copy_from_slice(p);
    write_success_payload("get_settings", p.len())
}
fn image_request(r: &[u8]) -> Result<usize, &'static str> {
    let mut b = [0; 4096];
    let url = decoded(args(r), b"url", &mut b)?;
    valid_image(url)?;
    let o = payload_buf();
    let mut n = 0;
    bytes(o, &mut n, b"{\"url\":")?;
    q(o, &mut n, url)?;
    bytes(o,&mut n,b",\"method\":\"GET\",\"headers\":{\"User-Agent\":\"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0\",\"Referer\":\"")?;
    bytes(o,&mut n,selected_base(r))?;
    bytes(o,&mut n,b"/\",\"Origin\":\"")?;
    bytes(o,&mut n,selected_base(r))?;
    bytes(o,&mut n,b"\"}}")?;
    Ok(n)
}
fn run_get_image_request(r: &[u8]) -> u32 {
    execute("get_image_request", image_request(r))
}
koma_source_sdk::koma_source_exports!("goda_source");
