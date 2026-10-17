#![no_std]
// Keiyoushi BH3.kt, Apache-2.0, revision 10f06eb833af5990bc5470c84c5e93edc159ad4e.
// Public BH3 source upgrade. No accounts, scripts, redirects or retries.
use koma_source_sdk::{
    host,
    json_utils::*,
    source::{SourceCapabilities, SourceInfo},
};
const BASE: &[u8] = b"https://comic.bh3.com";
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.bh3.koma",
    name: "崩坏3 IP站",
    version: "0.1.2",
    api_version: "0.2",
    language: "zh",
    author: "Koma source contributors",
    description: "Public BH3 comics source. Upstream SAFE; individual works unclassified.",
    content_rating: "safe",
};
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: false,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: true,
    manga_list: true,
    home: false,
    filters: false,
    settings: false,
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
    if write_bytes(&mut out, &mut n, b"bh3 stage=")
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
fn http(url: &[u8], _mobile: bool) -> Result<&'static [u8], &'static str> {
    diagnostic(b"fetch_begin", b"request");
    let referer = b"https://comic.bh3.com/".as_slice();
    let extra: &[(&[u8], &[u8])] = &[(b"Origin", BASE)];
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
    bytes(&mut log, &mut l, b"bh3 stage=http_status value=")?;
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

fn numeric(s: &[u8]) -> Result<&[u8], &'static str> {
    if s.len() > 20 || number(s).filter(|n| *n > 0).is_none() {
        return Err("invalid request");
    }
    Ok(s)
}
fn manga_id(r: &[u8]) -> Result<&[u8], &'static str> {
    numeric(
        string(input(r)?, b"mangaId")
            .and_then(|v| v.strip_prefix(b"bh3:"))
            .ok_or("invalid request")?,
    )
}
fn chapter_id(r: &[u8]) -> Result<(&[u8], &[u8]), &'static str> {
    let s = string(input(r)?, b"chapterId")
        .and_then(|v| v.strip_prefix(b"bh3ch:"))
        .ok_or("invalid request")?;
    let i = s.iter().position(|c| *c == b':').ok_or("invalid request")?;
    Ok((numeric(&s[..i])?, numeric(&s[i + 1..])?))
}
fn route<'a>(
    id: &[u8],
    chapter: Option<&[u8]>,
    suffix: &[u8],
    out: &'a mut [u8],
) -> Result<&'a [u8], &'static str> {
    let mut n = 0;
    bytes(out, &mut n, BASE)?;
    bytes(out, &mut n, b"/book/")?;
    bytes(out, &mut n, id)?;
    if let Some(ch) = chapter {
        bytes(out, &mut n, b"/")?;
        bytes(out, &mut n, ch)?;
    }
    bytes(out, &mut n, suffix)?;
    Ok(&out[..n])
}
fn image_url(s: &[u8]) -> Result<(), &'static str> {
    core::str::from_utf8(s).map_err(|_| "invalid image URL")?;
    if s.len() > 8192
        || s.iter()
            .any(|c| *c <= 32 || *c == 127 || b"\"<>\\#".contains(c))
    {
        return Err("invalid image URL");
    }
    let h = s
        .strip_prefix(b"https://")
        .ok_or("invalid image URL")?
        .split(|c| *c == b'/')
        .next()
        .ok_or("invalid image URL")?;
    if h.is_empty()
        || !h.is_ascii()
        || h.iter()
            .any(|c| !c.is_ascii_alphanumeric() && *c != b'.' && *c != b'-')
    {
        return Err("invalid image URL");
    }
    if !(h == b"comic.bh3.com" || h == b"mihoyo.com" || h.ends_with(b".mihoyo.com")) {
        return Err("invalid image URL");
    }
    let mut i = 0;
    while i < s.len() {
        if s[i] == b'%' {
            let a = *s.get(i + 1).ok_or("invalid image URL")?;
            let b = *s.get(i + 2).ok_or("invalid image URL")?;
            let n = hex4(&[b'0', b'0', a, b]).ok_or("invalid image URL")?;
            if n <= 32 || n == 127 {
                return Err("invalid image URL");
            }
            i += 3;
        } else {
            i += 1;
        }
    }
    Ok(())
}
fn absolute<'a>(s: &[u8], out: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    let mut n = 0;
    if s.starts_with(b"/") && !s.starts_with(b"//") {
        bytes(out, &mut n, BASE)?;
    }
    bytes(out, &mut n, s)?;
    image_url(&out[..n])?;
    Ok(&out[..n])
}
fn list(r: &[u8]) -> Result<usize, &'static str> {
    let a = input(r)?;
    if page(r)? != 1 {
        return Err("invalid page cursor");
    }
    if let Some(id) = string(a, b"listingId") {
        if id != b"popular" {
            return Err("unknown listing");
        }
    }
    let root = doc(b"https://comic.bh3.com/book")?;
    let all = Nodes::get(&root, b"a[href*=book]")?;
    if all.n == 0 {
        return Err("public catalogue missing");
    }
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"items\":[")?;
    for i in 0..all.n {
        let node = selected(&all, i);
        let mut id = [0; 64];
        let id = node.select(b"div.container")?.attr(b"id", &mut id)?;
        numeric(id)?;
        let mut title = [0; 1024];
        let title = node.select(b"div.container-title")?.text(&mut title)?;
        if title.is_empty() {
            return Err("missing title");
        }
        let mut raw = [0; 8192];
        let src = node.select(b"img")?.attr(b"src", &mut raw)?;
        let mut abs = [0; 8192];
        let src = absolute(src, &mut abs)?;
        if i > 0 {
            bytes(b, &mut n, b",")?;
        }
        bytes(b, &mut n, b"{\"id\":\"bh3:")?;
        bytes(b, &mut n, id)?;
        bytes(b, &mut n, b"\",\"title\":")?;
        q(b, &mut n, title)?;
        bytes(b, &mut n, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
        q(b, &mut n, src)?;
        bytes(b,&mut n,b"},\"authors\":[],\"status\":\"unknown\",\"contentRating\":\"unknown\",\"language\":\"zh\"}")?;
    }
    bytes(
        b,
        &mut n,
        b"],\"page\":{\"nextCursor\":null,\"hasMore\":false}}",
    )?;
    Ok(n)
}
fn detail(r: &[u8]) -> Result<usize, &'static str> {
    let id = manga_id(r)?;
    let mut url = [0; 512];
    let url = route(id, None, b"", &mut url)?;
    let root = doc(url)?;
    let mut title = [0; 1024];
    let title = root.select(b"div.title")?.text(&mut title)?;
    if title.is_empty() {
        return Err("missing title");
    }
    let mut desc = [0; 16384];
    let desc = root.select(b"div.detail_info1")?.text(&mut desc)?;
    let mut raw = [0; 8192];
    let src = root.select(b"img.cover")?.attr(b"src", &mut raw)?;
    let mut abs = [0; 8192];
    let src = absolute(src, &mut abs)?;
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"manga\":{\"id\":\"bh3:")?;
    bytes(b, &mut n, id)?;
    bytes(b, &mut n, b"\",\"title\":")?;
    q(b, &mut n, title)?;
    bytes(b, &mut n, b",\"description\":")?;
    q(b, &mut n, desc)?;
    bytes(b, &mut n, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(b, &mut n, src)?;
    bytes(b,&mut n,b"},\"authors\":[],\"artists\":[],\"tags\":[],\"status\":\"unknown\",\"contentRating\":\"unknown\",\"language\":\"zh\",\"links\":[{\"kind\":\"source\",\"url\":")?;
    q(b, &mut n, url)?;
    bytes(b, &mut n, b"}]}}")?;
    Ok(n)
}
// Recognize an explicit Chinese episode label only before the title separator.
fn explicit_label(label: &[u8]) -> Option<usize> {
    let label = core::str::from_utf8(label).ok()?.trim();
    let label = label.strip_prefix('第')?.strip_suffix('话')
        .or_else(|| label.strip_prefix('第')?.strip_suffix('話'))?;
    if !label.is_empty() && label.bytes().all(|b| b.is_ascii_digit()) {
        return label.bytes().try_fold(0usize, |v, b| {
            v.checked_mul(10)?.checked_add((b - b'0') as usize)
        });
    }
    if label == "零" || label == "〇" { return Some(0); }
    let mut total = 0usize;
    let mut section = 0usize;
    let mut digit = None;
    let mut last_unit = usize::MAX;
    let mut last_section = usize::MAX;
    let mut zero = false;
    let mut digit_after_zero = false;
    for c in label.chars() {
        let value = match c {
            '一' => Some(1usize), '二' => Some(2), '三' => Some(3),
            '四' => Some(4), '五' => Some(5), '六' => Some(6),
            '七' => Some(7), '八' => Some(8), '九' => Some(9), _ => None,
        };
        if let Some(v) = value {
            if digit.is_some() { return None; }
            digit = Some(v); digit_after_zero = zero; zero = false; continue;
        }
        if c == '零' || c == '〇' {
            if zero || digit.is_some() || (section == 0 && total == 0) { return None; }
            zero = true; continue;
        }
        let unit = match c { '十' => 10, '百' => 100, '千' => 1000, _ => 0 };
        if unit > 0 {
            if zero || unit >= last_unit { return None; }
            if last_unit != usize::MAX && (last_unit / unit > 10) != digit_after_zero {
                return None;
            }
            let v = match digit.take() {
                Some(v) => v,
                None if unit == 10 && section == 0 => 1,
                _ => return None,
            };
            section = section.checked_add(v.checked_mul(unit)?)?;
            last_unit = unit;
            continue;
        }
        let scale = match c { '万' => 10000, '亿' => 100000000, _ => return None };
        if zero || scale >= last_section { return None; }
        section = section.checked_add(digit.take().unwrap_or(0))?;
        if section == 0 { return None; }
        total = total.checked_add(section.checked_mul(scale)?)?;
        section = 0; last_unit = usize::MAX; last_section = scale;
    }
    if zero { return None; }
    if digit.is_some() && ((last_unit != usize::MAX && last_unit > 10) ||
        (last_unit == usize::MAX && total > 0)) && !digit_after_zero { return None; }
    total.checked_add(section)?.checked_add(digit.unwrap_or(0))
        .filter(|v| *v > 0)
}
fn explicit_number(title: &[u8]) -> Option<usize> {
    let t = title.strip_prefix("第".as_bytes())?;
    let count = t.iter().take_while(|c| c.is_ascii_digit()).count();
    if count == 0 {
        let label = title.split(|b| b.is_ascii_whitespace()).next()?;
        return explicit_label(label);
    }
    let tail = &t[count..];
    if ["话", "話", "章", "回"]
        .iter()
        .any(|s| tail.starts_with(s.as_bytes()))
    {
        number(&t[..count])
    } else {
        None
    }
}
fn chapters(r: &[u8]) -> Result<usize, &'static str> {
    let id = manga_id(r)?;
    let mut url = [0; 512];
    let body = http(route(id, None, b"/get_chapter", &mut url)?, false)?;
    let start = ws(body, 0);
    if body.get(start) != Some(&b'[')
        || json_value_end(body, start, 0).map(|p| ws(body, p)) != Some(body.len())
    {
        return Err("invalid chapter JSON");
    }
    let b = payload_buf();
    let mut n = 0;
    let mut p = ws(body, start + 1);
    let mut count = 0;
    bytes(b, &mut n, b"{\"chapters\":[")?;
    while body.get(p) != Some(&b']') {
        if count >= 2048 {
            return Err("too many chapters");
        }
        let e = json_value_end(body, p, 0).ok_or("invalid chapter JSON")?;
        let obj = &body[p..e];
        if obj.first() != Some(&b'{') {
            return Err("invalid chapter object");
        }
        let book = json_object_member(obj, b"bookid").ok_or("missing book ID")?;
        let book = if book.first() == Some(&b'\"') {
            &book[1..book.len() - 1]
        } else {
            book
        };
        numeric(book)?;
        if book != id {
            return Err("chapter book mismatch");
        }
        let chapter = json_object_member(obj, b"chapterid").ok_or("missing chapter ID")?;
        let chapter = if chapter.first() == Some(&b'\"') {
            &chapter[1..chapter.len() - 1]
        } else {
            chapter
        };
        numeric(chapter)?;
        let mut title = [0; 2048];
        let title = unescape(
            string(obj, b"title").ok_or("missing chapter title")?,
            &mut title,
        )?;
        if title.is_empty() {
            return Err("missing chapter title");
        }
        if count > 0 {
            bytes(b, &mut n, b",")?;
        }
        bytes(b, &mut n, b"{\"id\":\"bh3ch:")?;
        bytes(b, &mut n, book)?;
        bytes(b, &mut n, b":")?;
        bytes(b, &mut n, chapter)?;
        bytes(b, &mut n, b"\",\"mangaId\":\"bh3:")?;
        bytes(b, &mut n, id)?;
        bytes(b, &mut n, b"\",\"title\":")?;
        q(b, &mut n, title)?;
        bytes(b, &mut n, b",\"index\":")?;
        if !write_usize(b, &mut n, count) {
            return Err("payload overflow");
        }
        bytes(b, &mut n, b",\"chapterNumber\":")?;
        if let Some(v) = explicit_number(title) {
            if !write_usize(b, &mut n, v) {
                return Err("payload overflow");
            }
            bytes(b, &mut n, b",\"readingGroup\":\"chapter\",\"continuationEligible\":true")?;
        } else {
            bytes(b, &mut n, b"null,\"readingGroup\":\"unknown\",\"continuationEligible\":false")?;
        }
        bytes(
            b,
            &mut n,
            b",\"volumeNumber\":null,\"language\":\"zh\",\"publishedAt\":",
        )?;
        if let Some(raw) = string(obj, b"timestamp") {
            let mut date = [0; 128];
            let date = unescape(raw, &mut date)?;
            q(b, &mut n, date)?;
        } else {
            bytes(b, &mut n, b"null")?;
        }
        bytes(b, &mut n, b",\"uploadDate\":null}")?;
        count += 1;
        p = ws(body, e);
        if body.get(p) == Some(&b',') {
            p = ws(body, p + 1);
        } else if body.get(p) != Some(&b']') {
            return Err("invalid chapter JSON");
        }
    }
    if count == 0 {
        return Err("public chapters missing");
    }
    bytes(
        b,
        &mut n,
        b"],\"page\":{\"nextCursor\":null,\"hasMore\":false}}",
    )?;
    Ok(n)
}
fn pages(r: &[u8]) -> Result<usize, &'static str> {
    let (book, chapter) = chapter_id(r)?;
    let mut url = [0; 512];
    let root = doc(route(book, Some(chapter), b"", &mut url)?)?;
    let all = Nodes::get(&root, b"img.lazy.comic_img")?;
    if all.n == 0 {
        return Err("public page images missing or gated");
    }
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"chapterId\":\"bh3ch:")?;
    bytes(b, &mut n, book)?;
    bytes(b, &mut n, b":")?;
    bytes(b, &mut n, chapter)?;
    bytes(b, &mut n, b"\",\"pages\":[")?;
    for i in 0..all.n {
        let mut raw = [0; 8192];
        let src = selected(&all, i).attr(b"data-original", &mut raw)?;
        let mut abs = [0; 8192];
        let src = absolute(src, &mut abs)?;
        if i > 0 {
            bytes(b, &mut n, b",")?;
        }
        bytes(b, &mut n, b"{\"id\":\"bh3page:")?;
        bytes(b, &mut n, book)?;
        bytes(b, &mut n, b":")?;
        bytes(b, &mut n, chapter)?;
        bytes(b, &mut n, b":")?;
        if !write_usize(b, &mut n, i) {
            return Err("payload overflow");
        }
        bytes(b, &mut n, b"\",\"index\":")?;
        if !write_usize(b, &mut n, i) {
            return Err("payload overflow");
        }
        bytes(b, &mut n, b",\"image\":{\"kind\":\"url\",\"url\":")?;
        q(b, &mut n, src)?;
        bytes(b, &mut n, b"}}")?;
    }
    bytes(b, &mut n, b"]}")?;
    Ok(n)
}
fn image_request(r: &[u8]) -> Result<usize, &'static str> {
    let mut buf = [0; 8192];
    let url = unescape(
        string(input(r)?, b"url").ok_or("missing image URL")?,
        &mut buf,
    )?;
    image_url(url)?;
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"url\":")?;
    q(b, &mut n, url)?;
    bytes(b,&mut n,b",\"headers\":{\"User-Agent\":\"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0\",\"Referer\":\"https://comic.bh3.com/\",\"Origin\":\"https://comic.bh3.com\"}}")?;
    Ok(n)
}
fn run_get_manga_list(r: &[u8]) -> u32 {
    execute("get_manga_list", list(r))
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
    let data =
        "{\"listings\":[{\"id\":\"popular\",\"name\":\"漫画目录\",\"supportsPaging\":false}]}"
            .as_bytes();
    payload_buf()[..data.len()].copy_from_slice(data);
    write_success_payload("get_listings", data.len())
}
fn run_search(_: &[u8]) -> u32 {
    write_error("search", "unsupported", "search unsupported")
}
fn run_get_home(_: &[u8]) -> u32 {
    write_error("get_home", "unsupported", "home unsupported")
}
fn run_get_filters(_: &[u8]) -> u32 {
    write_error("get_filters", "unsupported", "filters unsupported")
}
fn run_get_settings(_: &[u8]) -> u32 {
    write_error("get_settings", "unsupported", "settings unsupported")
}
koma_source_sdk::koma_source_exports!("bh3");
