#![no_std]
// Public contract: Keiyoushi Miaoqu.kt / MCCMSWeb.kt, Apache-2.0, revision
// 1319949ca9bc456908196e3ba2d104e94182c5d6. No authentication/mirror/JS processing.
use koma_source_sdk::{
    host,
    json_utils::*,
    source::{SourceCapabilities, SourceInfo},
};
const BASE: &[u8] = b"https://www.miaoqumh.org";
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.miaoqu.koma",
    name: "喵趣漫画",
    version: "0.1.1",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description:
        "Miaoqu public HTML source; shares catalogue with SixMH. Upstream SAFE; usage unknown.",
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
    settings: false,
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
    if write_bytes(&mut out, &mut n, b"miaoqu stage=")
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
fn fetch(base: &[u8], path: &[u8]) -> Result<Node, &'static str> {
    host::log_info(b"miaoqu stage=fetch_begin");
    let mut url = [0; 8192];
    let mut n = 0;
    bytes(&mut url, &mut n, base)?;
    bytes(&mut url, &mut n, path)?;
    let len = koma_source_sdk::build_get_request(http_req_buf(), &url[..n], None, &[]).ok_or_else(
        || {
            diagnostic(b"request_build", b"network_error");
            "network failure"
        },
    )?;
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
    if write_bytes(&mut out, &mut n, b"miaoqu stage=http_status value=")
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
    host::log_info(b"miaoqu stage=body_ready");
    let root = Node::parse(&body_buf()[..n])?;
    host::log_info(b"miaoqu stage=html_ready");
    Ok(root)
}
const MOBILE: &[u8] = b"https://m.miaoqumh.org";
fn canonical(s: &[u8]) -> Result<&[u8], &'static str> {
    let p = s
        .strip_prefix(BASE)
        .or_else(|| s.strip_prefix(MOBILE))
        .unwrap_or(s);
    if !p.starts_with(b"/")
        || p.starts_with(b"//")
        || p.len() > 1024
        || p.split(|c| *c == b'/').any(|c| c == b"." || c == b"..")
        || p.iter()
            .any(|c| !c.is_ascii_alphanumeric() && !b"/._-%".contains(c))
    {
        return Err("invalid public path");
    }
    let mut i = 0;
    while i < p.len() {
        if p[i] == b'%' {
            if i + 2 >= p.len() || !p[i + 1].is_ascii_hexdigit() || !p[i + 2].is_ascii_hexdigit() {
                return Err("invalid public path");
            }
            i += 3;
        } else {
            i += 1;
        }
    }
    Ok(p)
}
fn slug(p: &[u8]) -> Result<&[u8], &'static str> {
    let p = canonical(p)?
        .strip_prefix(b"/")
        .ok_or("invalid manga path")?;
    if p.is_empty()
        || p.len() > 256
        || p.iter()
            .any(|c| !c.is_ascii_alphanumeric() && !b"_-".contains(c))
    {
        return Err("invalid manga path");
    }
    Ok(p)
}
fn manga_identity(r: &[u8]) -> Result<(&[u8], &[u8]), &'static str> {
    let id = string(args(r), b"mangaId").ok_or("invalid request")?;
    let s = id.strip_prefix(b"miaoqu:").ok_or("invalid request")?;
    if s.is_empty()
        || s.len() > 256
        || s.iter()
            .any(|c| !c.is_ascii_alphanumeric() && !b"_-".contains(c))
    {
        return Err("invalid request");
    }
    Ok((id, s))
}
fn detail_path<'a>(s: &[u8], out: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    let mut n = 0;
    bytes(out, &mut n, b"/")?;
    bytes(out, &mut n, s)?;
    Ok(&out[..n])
}
fn chapter_path(s: &[u8]) -> Result<(&[u8], usize), &'static str> {
    let raw = canonical(s)?;
    let p = raw
        .strip_prefix(b"/index.php")
        .unwrap_or(raw)
        .strip_prefix(b"/")
        .ok_or("invalid chapter path")?;
    let mut parts = p.split(|c| *c == b'/');
    let book = parts
        .next()
        .and_then(number)
        .filter(|n| *n > 0)
        .ok_or("invalid chapter path")?;
    let last = parts
        .next()
        .and_then(|v| v.strip_suffix(b".html"))
        .and_then(number)
        .filter(|n| *n > 0)
        .ok_or("invalid chapter path")?;
    if book == 0 || parts.next().is_some() {
        return Err("invalid chapter path");
    }
    Ok((raw, last))
}
fn chapter_id(r: &[u8]) -> Result<(&[u8], usize), &'static str> {
    chapter_path(
        string(args(r), b"chapterId")
            .and_then(|s| s.strip_prefix(b"miaoquch:"))
            .ok_or("invalid request")?,
    )
}
fn qid(b: &mut [u8], n: &mut usize, prefix: &[u8], s: &[u8]) -> Result<(), &'static str> {
    bytes(b, n, b"\"")?;
    if !append_json_escaped(b, n, prefix) || !append_json_escaped(b, n, s) {
        return Err("payload overflow");
    }
    bytes(b, n, b"\"")
}
fn valid_image(url: &[u8]) -> Result<(), &'static str> {
    core::str::from_utf8(url).map_err(|_| "invalid image URL")?;
    let rest = url.strip_prefix(b"https://").ok_or("invalid image URL")?;
    if url.len() >= 8192
        || url
            .iter()
            .any(|v| *v <= 32 || *v == 127 || b"\\#\"<>".contains(v))
    {
        return Err("invalid image URL");
    }
    let host = rest
        .split(|c| *c == b'/' || *c == b'?')
        .next()
        .ok_or("invalid image URL")?;
    if !host.contains(&b'.')
        || host.iter().all(|c| c.is_ascii_digit() || *c == b'.')
        || host.split(|c| *c == b'.').any(|v| {
            v.is_empty()
                || v.len() > 63
                || !v[0].is_ascii_alphanumeric()
                || !v[v.len() - 1].is_ascii_alphanumeric()
                || v.iter().any(|c| !c.is_ascii_alphanumeric() && *c != b'-')
        })
    {
        return Err("invalid image URL");
    }
    let mut i = 0;
    while i < url.len() {
        if url[i] == b'%' {
            if i + 2 >= url.len() {
                return Err("invalid image URL");
            }
            let a = hex_digit(url[i + 1]).ok_or("invalid image URL")?;
            let b = hex_digit(url[i + 2]).ok_or("invalid image URL")?;
            let c = a * 16 + b;
            if c < 32 || c == 127 {
                return Err("invalid image URL");
            }
            i += 3;
        } else {
            i += 1;
        }
    }
    Ok(())
}
fn hex_digit(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}
fn absolute<'a>(url: &[u8], base: &[u8], out: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    let mut n = 0;
    if url.starts_with(b"https://") {
        bytes(out, &mut n, url)?;
    } else if url.starts_with(b"//") {
        bytes(out, &mut n, b"https:")?;
        bytes(out, &mut n, url)?;
    } else if url.starts_with(b"/") {
        bytes(out, &mut n, base)?;
        bytes(out, &mut n, url)?;
    } else {
        return Err("invalid image URL");
    }
    valid_image(&out[..n])?;
    Ok(&out[..n])
}
fn list(r: &[u8], search: bool) -> Result<usize, &'static str> {
    let pn = page(r)?;
    if pn > 100000 {
        return Err("invalid page cursor");
    }
    let mut path = [0; 8192];
    let mut n = 0;
    if search {
        let raw = string(args(r), b"query").ok_or("missing query")?;
        let mut tmp = [0; 2048];
        let query = trim_ascii(unescape(raw, &mut tmp)?);
        if query.is_empty() {
            return Err("empty query");
        }
        bytes(&mut path, &mut n, b"/search/")?;
        if !write_url_encoded(&mut path, &mut n, query) {
            return Err("payload overflow");
        }
        bytes(&mut path, &mut n, b"/")?;
    } else {
        let id = string(args(r), b"listingId").unwrap_or(b"popular");
        bytes(
            &mut path,
            &mut n,
            match id {
                b"popular" => b"/category/order/hits/page/".as_slice(),
                b"latest" => b"/category/order/addtime/page/",
                _ => return Err("unknown listing"),
            },
        )?;
    }
    if !write_usize(&mut path, &mut n, pn) {
        return Err("payload overflow");
    }
    let root = fetch(BASE, &path[..n])?;
    if root.select(b"#code-div").is_ok() {
        return Err("permission denied");
    }
    let _wrap = root.select(b"#mangawrap")?;
    let cards = Nodes::get(&root, b"#mangawrap > *")?;
    let mut href = [0; 1024];
    let has_more = match root.select(b"#next") {
        Ok(next) => {
            let raw = next.attr(b"href", &mut href)?;
            let raw = canonical(raw)?;
            let suffix = raw
                .rsplit(|c| *c == b'/')
                .next()
                .and_then(number)
                .ok_or("invalid pagination number")?;
            let current_prefix = path[..n]
                .rsplit(|c| *c == b'/')
                .next()
                .map(|s| n - s.len())
                .ok_or("invalid pagination path")?;
            let next_prefix = raw
                .rsplit(|c| *c == b'/')
                .next()
                .map(|s| raw.len() - s.len())
                .ok_or("invalid pagination path")?;
            if raw[..next_prefix] != path[..current_prefix] || (suffix != pn && suffix != pn + 1) {
                return Err("pagination target mismatch");
            }
            suffix != pn
        }
        Err(_) => false,
    };
    if cards.n == 0 && has_more {
        return Err("empty page has next cursor");
    }
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"items\":[")?;
    for i in 0..cards.n {
        let card = selected(&cards, i);
        let link = card.select(b"a.manga-img")?;
        let mut hp = [0; 1024];
        let sp = slug(link.attr(b"href", &mut hp)?)?;
        let mut titlebuf = [0; 4096];
        let title = card.select(b".manga-name")?.text(&mut titlebuf)?;
        if title.is_empty() {
            return Err("empty manga title");
        }
        let mut style = [0; 8192];
        let style = link.attr(b"style", &mut style)?;
        let start = find_subslice(style, b"background: url(").ok_or("cover style missing")? + 16;
        let tail = &style[start..];
        let end = tail
            .iter()
            .position(|c| *c == b')')
            .ok_or("cover style invalid")?;
        let mut abs = [0; 8192];
        let cover = absolute(&tail[..end], BASE, &mut abs)?;
        if i > 0 {
            bytes(b, &mut n, b",")?;
        }
        bytes(b, &mut n, b"{\"id\":")?;
        qid(b, &mut n, b"miaoqu:", sp)?;
        bytes(b, &mut n, b",\"title\":")?;
        q(b, &mut n, title)?;
        bytes(
            b,
            &mut n,
            b",\"contentRating\":\"unknown\",\"cover\":{\"kind\":\"url\",\"url\":",
        )?;
        q(b, &mut n, cover)?;
        bytes(b, &mut n, b"}}")?;
    }
    bytes(b, &mut n, b"],\"page\":{\"nextCursor\":")?;
    if has_more {
        bytes(b, &mut n, b"\"")?;
        if !write_usize(b, &mut n, pn + 1) {
            return Err("payload overflow");
        }
        bytes(b, &mut n, b"\",\"hasMore\":true}}")?;
    } else {
        bytes(b, &mut n, b"null,\"hasMore\":false}}")?;
    }
    Ok(n)
}
fn detail(r: &[u8]) -> Result<usize, &'static str> {
    let (id, s) = manga_identity(r)?;
    let mut path = [0; 1024];
    let root = fetch(MOBILE, detail_path(s, &mut path)?)?;
    let owner = root.select(b".infobox")?;
    let mut text = [0; 32768];
    let title = owner.select(b".title")?.text(&mut text)?;
    if title.is_empty() {
        return Err("empty manga title");
    }
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"manga\":{\"id\":")?;
    q(b, &mut n, id)?;
    bytes(b, &mut n, b",\"title\":")?;
    q(b, &mut n, title)?;
    let mut raw = [0; 8192];
    let mut abs = [0; 8192];
    let cover = absolute(
        owner.select(b"img")?.attr(b"src", &mut raw)?,
        MOBILE,
        &mut abs,
    )?;
    bytes(b, &mut n, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(b, &mut n, cover)?;
    bytes(b, &mut n, b"},\"contentRating\":\"unknown\",\"authors\":[")?;
    let labels = Nodes::get(&owner, b".tage")?;
    let mut found = false;
    for i in 0..labels.n {
        let t = selected(&labels, i).text(&mut text)?;
        if let Some(a) = t.strip_prefix("作者：".as_bytes()) {
            let a = trim_ascii(a);
            if a.is_empty() {
                return Err("empty author");
            }
            if found {
                bytes(b, &mut n, b",")?;
            }
            q(b, &mut n, a)?;
            found = true;
        }
    }
    bytes(b, &mut n, b"],\"tags\":[")?;
    let mut count = 0;
    for i in 0..labels.n {
        let node = selected(&labels, i);
        let t = node.text(&mut text)?;
        if t.starts_with("类型：".as_bytes()) {
            let tags = Nodes::get(&node, b"a")?;
            for j in 0..tags.n {
                let t = selected(&tags, j).text(&mut text)?;
                if t.is_empty() {
                    return Err("empty tag");
                }
                if count > 0 {
                    bytes(b, &mut n, b",")?;
                }
                q(b, &mut n, t)?;
                count += 1;
            }
        }
    }
    bytes(b, &mut n, b"],\"status\":\"unknown\",\"summary\":")?;
    let mut summary = [0; 65536];
    let mut sn = 0;
    for i in 0..labels.n {
        let t = selected(&labels, i).text(&mut text)?;
        if t.starts_with("更新于".as_bytes()) {
            bytes(&mut summary, &mut sn, t)?;
            bytes(&mut summary, &mut sn, b"\n\n")?;
        }
    }
    let description = root.select(b".text")?.text(&mut text)?;
    bytes(&mut summary, &mut sn, description)?;
    q(b, &mut n, &summary[..sn])?;
    let mut page_url = [0; 2048];
    let mut page_len = 0;
    bytes(&mut page_url, &mut page_len, MOBILE)?;
    bytes(&mut page_url, &mut page_len, detail_path(s, &mut path)?)?;
    bytes(b, &mut n, b",\"links\":[{\"kind\":\"source\",\"url\":")?;
    q(b, &mut n, &page_url[..page_len])?;
    bytes(b, &mut n, b"}]}}")?;
    Ok(n)
}
fn explicit_chapter_number(title: &[u8]) -> Option<usize> {
    let title = trim_ascii(title);
    let s = title.strip_prefix("第".as_bytes()).unwrap_or(title);
    let len = s.iter().take_while(|c| c.is_ascii_digit()).count();
    if len == 0 || len > 9 {
        return None;
    }
    let tail = trim_ascii(&s[len..]);
    let unit = ["话", "話", "回", "章"]
        .iter()
        .any(|t| tail.starts_with(t.as_bytes()));
    let paired = core::str::from_utf8(tail)
        .ok()
        .map(|t| {
            let rest = t.strip_prefix('(').or_else(|| t.strip_prefix('（'));
            rest.and_then(|r| r.strip_suffix(')').or_else(|| r.strip_suffix('）')))
                .map(|r| r == "上" || r == "下")
                .unwrap_or(false)
        })
        .unwrap_or(false);
    if !tail.is_empty() && !unit && !paired {
        return None;
    }
    number(&s[..len])
}
fn chapters(r: &[u8]) -> Result<usize, &'static str> {
    let (id, s) = manga_identity(r)?;
    let mut path = [0; 1024];
    let root = fetch(MOBILE, detail_path(s, &mut path)?)?;
    let nodes = Nodes::get(&root, b"ul.list > li > :first-child")?;
    if nodes.n == 0 {
        return Err("chapter structure missing");
    }
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"chapters\":[")?;
    for (index, i) in (0..nodes.n).rev().enumerate() {
        let a = selected(&nodes, i);
        let mut href = [0; 1024];
        let (cp, _) = chapter_path(a.attr(b"href", &mut href)?)?;
        let mut text = [0; 4096];
        let title = a.text(&mut text)?;
        if title.is_empty() {
            return Err("empty chapter title");
        }
        if index > 0 {
            bytes(b, &mut n, b",")?;
        }
        bytes(b, &mut n, b"{\"id\":")?;
        qid(b, &mut n, b"miaoquch:", cp)?;
        bytes(b, &mut n, b",\"mangaId\":")?;
        q(b, &mut n, id)?;
        bytes(b, &mut n, b",\"title\":")?;
        q(b, &mut n, title)?;
        bytes(b, &mut n, b",\"index\":")?;
        if !write_usize(b, &mut n, index) {
            return Err("payload overflow");
        }
        bytes(b, &mut n, b",\"chapterNumber\":")?;
        match explicit_chapter_number(title) {
            Some(v) => {
                if !write_usize(b, &mut n, v) {
                    return Err("payload overflow");
                }
            }
            None => bytes(b, &mut n, b"null")?,
        };
        bytes(b,&mut n,b",\"volumeNumber\":null,\"language\":\"zh\",\"publishedAt\":null,\"updatedAt\":null,\"pageCount\":null,\"scanlator\":null}")?;
    }
    bytes(
        b,
        &mut n,
        b"],\"page\":{\"nextCursor\":null,\"hasMore\":false}}",
    )?;
    Ok(n)
}
static mut DECODE_MID: [u8; 262144] = [0; 262144];
static mut DECODE_JSON: [u8; 262144] = [0; 262144];
fn b64_value(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}
fn base64(s: &[u8], out: &mut [u8]) -> Result<usize, &'static str> {
    if s.is_empty() || s.len() % 4 != 0 {
        return Err("invalid Base64");
    }
    let mut n = 0;
    for (i, chunk) in s.chunks_exact(4).enumerate() {
        let a = b64_value(chunk[0]).ok_or("invalid Base64")?;
        let b = b64_value(chunk[1]).ok_or("invalid Base64")?;
        let pad2 = chunk[2] == b'=';
        let pad3 = chunk[3] == b'=';
        if (pad2 && !pad3) || ((pad2 || pad3) && i + 1 != s.len() / 4) {
            return Err("invalid Base64");
        }
        let c = if pad2 {
            0
        } else {
            b64_value(chunk[2]).ok_or("invalid Base64")?
        };
        let d = if pad3 {
            0
        } else {
            b64_value(chunk[3]).ok_or("invalid Base64")?
        };
        if (pad2 && b & 15 != 0) || (!pad2 && pad3 && c & 3 != 0) {
            return Err("noncanonical Base64");
        }
        bytes(out, &mut n, &[(a << 2) | (b >> 4)])?;
        if !pad2 {
            bytes(out, &mut n, &[(b << 4) | (c >> 2)])?;
        }
        if !pad3 {
            bytes(out, &mut n, &[(c << 6) | d])?;
        }
    }
    Ok(n)
}
fn pages(r: &[u8]) -> Result<usize, &'static str> {
    let (cp, cid) = chapter_id(r)?;
    let _root = fetch(BASE, cp)?;
    let body = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), BODY_LEN) };
    // The initialized HTTP body is UTF8 and Node::parse consumed the exact decoded length.
    let start = find_subslice(body, b"var DATA='").ok_or("page DATA missing")? + 10;
    let rest = &body[start..];
    let end = rest
        .iter()
        .position(|c| *c == b'\'')
        .ok_or("page DATA unterminated")?;
    let literal = &rest[..end];
    if find_subslice(&rest[end + 1..], b"var DATA='").is_some() {
        return Err("duplicate page DATA");
    }
    let keys = [
        b"8-bXd9iN",
        b"8-RXyjry",
        b"8-oYvwVy",
        b"8-4ZY57U",
        b"8-mbJpU7",
        b"8-6MM2Ei",
        b"8-54TiQr",
        b"8-Ph5xx9",
        b"8-bYgePR",
        b"8-Z9A3bW",
    ];
    let mid = unsafe { &mut DECODE_MID };
    let size = base64(literal, mid)?;
    let key = keys[cid % 10];
    for (i, v) in mid[..size].iter_mut().enumerate() {
        *v ^= key[i & 7];
    }
    let decoded = unsafe { &mut DECODE_JSON };
    let size = base64(&mid[..size], decoded)?;
    let data = &decoded[..size];
    core::str::from_utf8(data).map_err(|_| "invalid UTF8")?;
    if data.first() != Some(&b'[')
        || json_value_end(data, 0, 0).map(|n| ws(data, n)) != Some(data.len())
    {
        return Err("invalid page JSON");
    }
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"chapterId\":")?;
    q(
        b,
        &mut n,
        string(args(r), b"chapterId").ok_or("invalid request")?,
    )?;
    bytes(b, &mut n, b",\"pages\":[")?;
    let mut at = ws(data, 1);
    let mut count = 0;
    while data.get(at) != Some(&b']') {
        if count >= 2048 {
            return Err("too many page entries");
        }
        let end = json_value_end(data, at, 0).ok_or("invalid page JSON")?;
        let entry = &data[at..end];
        if entry.first() != Some(&b'{') {
            return Err("invalid page entry");
        }
        let raw = string(entry, b"url").ok_or("missing page URL")?;
        let mut u = [0; 8192];
        let url = unescape(raw, &mut u)?;
        valid_image(url)?;
        if count > 0 {
            bytes(b, &mut n, b",")?;
        }
        bytes(b, &mut n, b"{\"id\":\"")?;
        bytes(b, &mut n, b"miaoqupage:")?;
        if !append_json_escaped(b, &mut n, cp) {
            return Err("payload overflow");
        }
        bytes(b, &mut n, b":")?;
        if !write_usize(b, &mut n, count) {
            return Err("payload overflow");
        }
        bytes(b, &mut n, b"\",\"index\":")?;
        if !write_usize(b, &mut n, count) {
            return Err("payload overflow");
        }
        bytes(b, &mut n, b",\"image\":{\"kind\":\"url\",\"url\":")?;
        q(b, &mut n, url)?;
        bytes(b, &mut n, b"}}")?;
        count += 1;
        at = ws(data, end);
        if data.get(at) == Some(&b',') {
            at = ws(data, at + 1);
        } else if data.get(at) != Some(&b']') {
            return Err("invalid page JSON");
        }
    }
    if count == 0 {
        return Err("empty page array");
    }
    bytes(b, &mut n, b"]}")?;
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
    let d="{\"listings\":[{\"id\":\"popular\",\"name\":\"热门漫画\",\"supportsPaging\":true},{\"id\":\"latest\",\"name\":\"最新漫画\",\"supportsPaging\":true}]}".as_bytes();
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
fn image_request(r: &[u8]) -> Result<usize, &'static str> {
    let raw = string(args(r), b"url").ok_or("missing image URL")?;
    let mut u = [0; 8192];
    let url = unescape(raw, &mut u)?;
    valid_image(url)?;
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"url\":")?;
    q(b, &mut n, url)?;
    bytes(b,&mut n,b",\"headers\":{\"User-Agent\":\"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0\"}}")?;
    Ok(n)
}
fn run_get_image_request(r: &[u8]) -> u32 {
    execute("get_image_request", image_request(r))
}
koma_source_sdk::koma_source_exports!("miaoqu");
