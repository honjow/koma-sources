#![no_std]
// Public contract: Keiyoushi ComicFury.kt / ContentWarningInterceptor.kt, Apache-2.0, revision
// 1319949ca9bc456908196e3ba2d104e94182c5d6. No authentication/mirror/JS processing.
use koma_source_sdk::{
    host,
    json_utils::*,
    source::{SourceCapabilities, SourceInfo},
};
const BASE: &[u8] = b"https://comicfury.com";
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.comicfury.zh.koma",
    name: "Comic Fury",
    version: "0.1.7",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description:
        "Comic Fury Chinese public HTML; upstream MIXED; optional author notes; no warning confirmation. Usage unknown.",
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
    if write_bytes(&mut out, &mut n, b"comicfury_zh stage=")
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
        | "invalid filter"
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
    host::log_info(b"comicfury_zh stage=fetch_begin");
    let mut url = [0; 8192];
    let mut n = 0;
    bytes(&mut url, &mut n, base)?;
    bytes(&mut url, &mut n, path)?;
    let len = koma_source_sdk::build_get_request(
        http_req_buf(),
        &url[..n],
        Some(b"https://comicfury.com/"),
        &[(b"Origin", BASE)],
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
    if write_bytes(&mut out, &mut n, b"comicfury_zh stage=http_status value=")
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
    host::log_info(b"comicfury_zh stage=body_ready");
    if !contains_bytes(&body_buf()[..n], b"</html>") {
        return Err("incomplete HTML");
    }
    let root = Node::parse(&body_buf()[..n])?;
    if Nodes::get(&root, b"input[name=proceed][value='View Webcomic']")?.n > 0 {
        return Err("permission denied");
    }

    let title = root.select(b"title")?;
    let mut tb = [0u8; 2048];
    if contains_bytes(title.text(&mut tb)?, b"Content Warning")
        && Nodes::get(&root, b"input[name=token]")?.n > 0
    {
        return Err("permission denied");
    }
    host::log_info(b"comicfury_zh stage=html_ready");
    Ok(root)
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
fn path(u: &[u8]) -> Result<&[u8], &'static str> {
    let s = if u.starts_with(BASE) {
        u.get(BASE.len()..).ok_or("invalid path")?
    } else {
        u
    };
    valid_uri(s)?;
    if !s.starts_with(b"/")
        || s.starts_with(b"//")
        || contains_bytes(s, b"#")
        || contains_bytes(s, b"/../")
        || contains_bytes(s, b"/./")
    {
        return Err("invalid path");
    }
    Ok(s)
}
fn profile_slug(p: &[u8]) -> Result<&[u8], &'static str> {
    let s = path(p)?
        .strip_prefix(b"/comicprofile.php?url=")
        .ok_or("invalid mangaId")?;
    if s.is_empty()
        || s.len() > 120
        || !s
            .iter()
            .all(|c| c.is_ascii_alphanumeric() || *c == b'-' || *c == b'_')
    {
        return Err("invalid mangaId");
    }
    Ok(s)
}
fn manga(r: &[u8], out: &mut [u8]) -> Result<usize, &'static str> {
    let raw = string(args(r), b"mangaId").ok_or("missing mangaId")?;
    let id = unescape(raw, out)?;
    profile_slug(id)?;
    Ok(id.len())
}
fn canonical_image<'a>(u: &[u8], out: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    let mut n = 0;
    if !u.starts_with(b"https://") {
        if u.starts_with(b"//") || contains_bytes(u, b":") {
            return Err("invalid image URL");
        }
        bytes(out, &mut n, BASE)?;
        if !u.starts_with(b"/") {
            bytes(out, &mut n, b"/")?;
        }
    }
    bytes(out, &mut n, u)?;
    let v = &out[..n];
    valid_uri(v)?;
    if !(v.starts_with(b"https://comicfury.com/") || v.starts_with(b"https://img.comicfury.com/"))
        || contains_bytes(v, b"#")
    {
        return Err("invalid image URL");
    }
    Ok(v)
}
fn own_text<'a>(node: &Node, out: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    // Host exposes combined text only. Subtracting exact child suffixes retains own text
    // for the published description parent (description followed by tags).
    let mut full = [0u8; 16384];
    let text = node.text(&mut full)?;
    let children = Nodes::get(node, b"div.description-tags")?;
    if children.n != 1 {
        return Err("description ownership mismatch");
    }
    let mut end = text.len();
    for i in (0..children.n).rev() {
        let child = selected(&children, i);
        let mut buf = [0u8; 4096];
        let t = child.text(&mut buf)?;
        if !t.is_empty() {
            let s = trim_ascii(&text[..end]);
            if !s.ends_with(t) {
                return Err("description ownership mismatch");
            }
            end = s.len() - t.len();
        }
    }
    let own = trim_ascii(&text[..end]);
    if own.len() >= out.len() {
        return Err("description overflow");
    }
    out[..own.len()].copy_from_slice(own);
    Ok(&out[..own.len()])
}
fn select_filter<'a>(filters: &[u8], key: &[u8], max: u8, default: &'a [u8]) -> Result<&'a [u8], &'static str> {
    match json_object_member(filters, key) {
        None | Some(b"null") => Ok(default),
        Some(v) if v.len() == 3 && v[0] == b'"' && v[2] == b'"' && v[1] >= b'0' && v[1] <= b'0' + max => {
            match v[1] { b'0' => Ok(b"0"), b'1' => Ok(b"1"), b'2' => Ok(b"2"), b'3' => Ok(b"3"), b'4' => Ok(b"4"), _ => Err("invalid filter") }
        },
        _ => Err("invalid filter"),
    }
}

fn list(r: &[u8], search: bool) -> Result<usize, &'static str> {
    let pg = page(r)?;
    if pg > 10000 {
        return Err("invalid page cursor");
    }
    let a = args(r);
    let filters = match json_object_member(a, b"filters") {
        None | Some(b"null") => b"{}".as_slice(),
        Some(v) if v.first() == Some(&b'{') => v,
        _ => return Err("invalid filter"),
    };
    let mut p = [0; 8192];
    let mut pn = 0;
    bytes(&mut p, &mut pn, b"/search.php?query=")?;
    if search {
        let raw = string(a, b"query").ok_or("missing query")?;
        let mut qb = [0; 2048];
        let query = unescape(raw, &mut qb)?;
        if query.is_empty() {
            return Err("empty query");
        }
        if !write_url_encoded(&mut p, &mut pn, query) {
            return Err("payload overflow");
        }
    }
    bytes(&mut p, &mut pn, b"&page=")?;
    if !write_usize(&mut p, &mut pn, pg) {
        return Err("payload overflow");
    }
    bytes(&mut p, &mut pn, b"&language=zh&tags=")?;
    if let Some(v) = json_object_member(filters, b"tags") {
        if v != b"null" {
            if v.len() < 2 || v[0] != b'"' || v[v.len()-1] != b'"' { return Err("invalid filter"); }
            let mut decoded = [0; 2048];
            let tags = unescape(&v[1..v.len()-1], &mut decoded)?;
            // Reference removes a single space immediately after commas.
            let mut normalized = [0; 2048]; let mut len = 0; let mut previous_comma = false;
            for &ch in tags {
                if previous_comma && ch == b' ' { previous_comma = false; continue; }
                bytes(&mut normalized, &mut len, &[ch])?;
                previous_comma = ch == b',';
            }
            if !write_url_encoded(&mut p, &mut pn, &normalized[..len]) { return Err("payload overflow"); }
        }
    }
    bytes(&mut p, &mut pn, b"&sort=")?;
    let default_sort = if search {
        b"0".as_slice()
    } else {
        match string(a, b"listingId").unwrap_or(b"popular") {
            b"popular" => b"1",
            b"latest" => b"2",
            _ => return Err("unknown listing"),
        }
    };
    bytes(&mut p, &mut pn, select_filter(filters, b"sort", 2, default_sort)?)?;
    bytes(&mut p, &mut pn, b"&lastupdate=")?;
    bytes(&mut p, &mut pn, select_filter(filters, b"lastupdate", 4, b"0")?)?;
    bytes(&mut p, &mut pn, b"&completed=")?;
    bytes(&mut p, &mut pn, match json_object_member(filters, b"completed") { None | Some(b"null") | Some(b"false") => b"1", Some(b"true") => b"0", _ => return Err("invalid filter") })?;
    for key in [b"fv".as_slice(), b"fn", b"fl", b"fs"] {
        bytes(&mut p, &mut pn, b"&")?; bytes(&mut p, &mut pn, key)?; bytes(&mut p, &mut pn, b"=")?;
        bytes(&mut p, &mut pn, select_filter(filters, key, 2, b"0")?)?;
    }
    let doc = fetch(BASE, &p[..pn])?;
    let cards = Nodes::get(&doc, b"div.webcomic-result")?;
    if cards.n == 0 {
        if search {
            let empty = Nodes::get(&doc, b"form.search-wrap div.search-results > div.search-error")?;
            if empty.n == 1 {
                let mut text = [0u8; 512];
                let message = selected(&empty, 0).text(&mut text)?;
                if trim_ascii(message) == b"There were no results for your search!" {
                    let b = payload_buf();
                    let mut n = 0;
                    bytes(b, &mut n, b"{\"items\":[],\"page\":{\"nextCursor\":null,\"hasMore\":false}}")?;
                    return Ok(n);
                }
            }
        }
        return Err("empty listing");
    }
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"items\":[")?;
    for i in 0..cards.n {
        let card = selected(&cards, i);
        let link = card.select(b"div.webcomic-result-avatar a")?;
        let img = link.select(b"img")?;
        let title = card.select(b"div.webcomic-result-title")?;
        let mut hb = [0; 512];
        let href = link.attr(b"href", &mut hb)?;
        let id = path(href)?;
        profile_slug(id)?;
        let mut tb = [0; 2048];
        let name = title.attr(b"title", &mut tb)?;
        if name.is_empty() {
            return Err("empty title");
        }
        let mut ib = [0; 4096];
        let src = img.attr(b"src", &mut ib)?;
        let mut ub = [0; 8192];
        let image = canonical_image(src, &mut ub)?;
        if i > 0 {
            bytes(b, &mut n, b",")?;
        }
        bytes(b, &mut n, b"{\"id\":")?;
        q(b, &mut n, id)?;
        bytes(b, &mut n, b",\"title\":")?;
        q(b, &mut n, name)?;
        bytes(b, &mut n, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
        q(b, &mut n, image)?;
        bytes(
            b,
            &mut n,
            b"},\"authors\":[],\"status\":\"unknown\",\"contentRating\":\"unknown\"}",
        )?;
    }
    let next = Nodes::get(&doc, b"div.search-next-page")?;
    if next.n > 1 {
        return Err("ambiguous pagination");
    }
    bytes(b, &mut n, b"],\"page\":{\"nextCursor\":")?;
    if next.n == 0 {
        bytes(b, &mut n, b"null")?;
    } else {
        let mut value = [0; 24];
        let mut count = 0;
        if !write_usize(&mut value, &mut count, pg + 1) {
            return Err("payload overflow");
        }
        q(b, &mut n, &value[..count])?;
    }
    bytes(
        b,
        &mut n,
        if next.n == 0 {
            b",\"hasMore\":false}}"
        } else {
            b",\"hasMore\":true}}"
        },
    )?;
    Ok(n)
}
fn detail(r: &[u8]) -> Result<usize, &'static str> {
    let mut idbuf = [0; 512];
    let len = manga(r, &mut idbuf)?;
    let id = &idbuf[..len];
    let doc = fetch(BASE, id)?;
    let title = doc.select(b"div.username-and-title div.authorname")?;
    let mut tb = [0; 2048];
    let name = title.text(&mut tb)?;
    if name.is_empty() {
        return Err("empty title");
    }
    let avatar = doc.select(b"div.profile-avatar img")?;
    let mut ib = [0; 4096];
    let src = avatar.attr(b"src", &mut ib)?;
    let mut ub = [0; 8192];
    let cover = canonical_image(src, &mut ub)?;
    let descriptions = Nodes::get(&doc, b"div.pccontent")?;
    let mut matches = 0;
    let mut chosen = 0;
    for i in 0..descriptions.n {
        if Nodes::get(&selected(&descriptions, i), b"div.description-tags")?.n == 1 {
            matches += 1;
            chosen = i;
        }
    }
    if matches != 1 {
        return Err("missing description parent");
    }
    let description = selected(&descriptions, chosen);
    let mut db = [0; 16384];
    let desc = own_text(&description, &mut db)?;
    let authors = Nodes::get(&doc, b"a.authorname")?;
    let genres = Nodes::get(&doc, b"div.description-tags > a")?;
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"manga\":{\"id\":")?;
    q(b, &mut n, id)?;
    bytes(b, &mut n, b",\"title\":")?;
    q(b, &mut n, name)?;
    bytes(b, &mut n, b",\"description\":")?;
    q(b, &mut n, desc)?;
    bytes(b, &mut n, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(b, &mut n, cover)?;
    bytes(b, &mut n, b"},\"authors\":[")?;
    for i in 0..authors.n {
        if i > 0 {
            bytes(b, &mut n, b",")?;
        }
        let mut t = [0; 2048];
        q(b, &mut n, selected(&authors, i).text(&mut t)?)?;
    }
    bytes(b, &mut n, b"],\"tags\":[")?;
    for i in 0..genres.n {
        if i > 0 {
            bytes(b, &mut n, b",")?;
        }
        let mut t = [0; 1024];
        q(b, &mut n, selected(&genres, i).text(&mut t)?)?;
    }
    bytes(b,&mut n,b"],\"status\":\"unknown\",\"language\":\"zh\",\"contentRating\":\"unknown\",\"links\":[{\"kind\":\"source\",\"url\":")?;
    let mut url = [0; 1024];
    let mut un = 0;
    bytes(&mut url, &mut un, BASE)?;
    bytes(&mut url, &mut un, id)?;
    q(b, &mut n, &url[..un])?;
    bytes(b, &mut n, b"}]}}")?;
    Ok(n)
}
const MAX_CHAPTERS: usize = 2048;
static mut CHAPTER_BYTES: [u8; 1024 * 1024] = [0; 1024 * 1024];
static mut CHAPTER_OFFSETS: [(usize, usize); MAX_CHAPTERS] = [(0, 0); MAX_CHAPTERS];
static mut CHAPTER_LEN: usize = 0;
static mut CHAPTER_COUNT: usize = 0;
fn explicit_number(title: &[u8]) -> Option<usize> {
    let s = core::str::from_utf8(title).ok()?.trim();
    let s = s.strip_prefix('第')?.trim_start();
    let end = s.find(|c: char| !c.is_ascii_digit())?;
    let n = number(s[..end].as_bytes())?;
    let rest = s[end..].trim_start();
    if rest.starts_with('話')
        || rest.starts_with('话')
        || rest.starts_with('回')
        || rest.starts_with('章')
    {
        Some(n)
    } else {
        None
    }
}
fn comic_path(p: &[u8], slug: &[u8]) -> Result<usize, &'static str> {
    let p = path(p)?;
    let mut prefix = [0; 256];
    let mut n = 0;
    bytes(&mut prefix, &mut n, b"/read/")?;
    bytes(&mut prefix, &mut n, slug)?;
    bytes(&mut prefix, &mut n, b"/comics/")?;
    let tail = p.strip_prefix(&prefix[..n]).ok_or("invalid chapterId")?;
    number(tail).filter(|x| *x > 0).ok_or("invalid chapterId")
}
fn collect(
    doc: Node,
    slug: &[u8],
    mid: &[u8],
    header: &[u8],
    initial_path: &[u8],
) -> Result<(), &'static str> {
    let mut current = doc;
    let mut visited = [[0u8; 512]; 128];
    let mut lengths = [0usize; 128];
    let mut visited_n = 1;
    if initial_path.len() >= 512 {
        return Err("invalid catalogue path");
    }
    visited[0][..initial_path.len()].copy_from_slice(initial_path);
    lengths[0] = initial_path.len();
    loop {
        let comics = Nodes::get(&current, b"a")?;
        let before_count = unsafe { CHAPTER_COUNT };
        for i in 0..comics.n {
            let node = selected(&comics, i);
            let items = Nodes::get(&node, b"div.archive-comic")?;
            if items.n == 0 {
                continue;
            }
            if items.n != 1 {
                return Err("ambiguous comic link");
            }
            let mut hb = [0; 512];
            let href = node.attr(b"href", &mut hb)?;
            let cp = path(href)?;
            comic_path(cp, slug)?;
            let title = node.select(b".archive-comic-title")?;
            let mut tb = [0; 2048];
            let t = title.text(&mut tb)?;
            if t.is_empty() {
                return Err("empty chapter title");
            }
            let mut name = [0; 4096];
            let mut nn = 0;
            if !header.is_empty() {
                bytes(&mut name, &mut nn, header)?;
                bytes(&mut name, &mut nn, b" - ")?;
            }
            bytes(&mut name, &mut nn, t)?;
            let mut item = [0; 8192];
            let mut n = 0;
            bytes(&mut item, &mut n, b"{\"id\":")?;
            q(&mut item, &mut n, cp)?;
            bytes(&mut item, &mut n, b",\"mangaId\":")?;
            q(&mut item, &mut n, mid)?;
            bytes(&mut item, &mut n, b",\"title\":")?;
            q(&mut item, &mut n, &name[..nn])?;
            bytes(&mut item, &mut n, b",\"chapterNumber\":")?;
            if let Some(num) = explicit_number(&name[..nn]) {
                if !write_usize(&mut item, &mut n, num) {
                    return Err("payload overflow");
                }
            } else {
                bytes(&mut item, &mut n, b"null")?;
            }
            bytes(&mut item,&mut n,b",\"volumeNumber\":null,\"language\":\"zh\",\"publishedAt\":null,\"updatedAt\":null,\"pageCount\":null}")?;
            unsafe {
                if CHAPTER_COUNT >= MAX_CHAPTERS {
                    return Err("too many chapters");
                }
                let start = CHAPTER_LEN;
                bytes(&mut CHAPTER_BYTES, &mut CHAPTER_LEN, &item[..n])?;
                CHAPTER_OFFSETS[CHAPTER_COUNT] = (start, n);
                CHAPTER_COUNT += 1;
            }
        }
        if unsafe { CHAPTER_COUNT } == before_count {
            return Err("empty catalogue group");
        }
        let next = Nodes::get(&current, b"span.vfpagecurrent + a.vfpage")?;
        if next.n == 0 {
            break;
        }
        if next.n != 1 {
            return Err("ambiguous pagination");
        }
        let mut nb = [0; 512];
        let dest = selected(&next, 0).attr(b"href", &mut nb)?;
        let dest = path(dest)?;
        let mut prefix = [0; 256];
        let mut pn = 0;
        bytes(&mut prefix, &mut pn, b"/read/")?;
        bytes(&mut prefix, &mut pn, slug)?;
        bytes(&mut prefix, &mut pn, b"/archive")?;
        if !dest.starts_with(&prefix[..pn]) {
            return Err("invalid catalogue next");
        }
        if dest.len() >= 512 || visited_n >= 128 {
            return Err("catalogue pagination limit");
        }
        for i in 0..visited_n {
            if &visited[i][..lengths[i]] == dest {
                return Err("catalogue cycle");
            }
        }
        visited[visited_n][..dest.len()].copy_from_slice(dest);
        lengths[visited_n] = dest.len();
        visited_n += 1;
        current = fetch(BASE, dest)?;
    }
    Ok(())
}
fn chapters(r: &[u8]) -> Result<usize, &'static str> {
    let mut idbuf = [0; 512];
    let len = manga(r, &mut idbuf)?;
    let mid = &idbuf[..len];
    let slug = profile_slug(mid)?;
    let mut p = [0; 512];
    let mut pn = 0;
    bytes(&mut p, &mut pn, b"/read/")?;
    bytes(&mut p, &mut pn, slug)?;
    bytes(&mut p, &mut pn, b"/archive")?;
    let doc = fetch(BASE, &p[..pn])?;
    unsafe {
        CHAPTER_LEN = 0;
        CHAPTER_COUNT = 0;
    }
    let group_nodes = Nodes::get(&doc, b"div.archive-chapter")?;
    let groups = Nodes::get(&doc, b"a")?;
    if group_nodes.n == 0 {
        collect(doc, slug, mid, b"", &p[..pn])?;
    } else {
        if group_nodes.n > 128 {
            return Err("too many catalogue groups");
        }
        let mut group_paths = [[0u8; 512]; 128];
        let mut group_lengths = [0usize; 128];
        let mut group_count = 0;
        for i in 0..groups.n {
            let node = selected(&groups, i);
            let items = Nodes::get(&node, b"div.archive-chapter")?;
            if items.n == 0 {
                continue;
            }
            if items.n != 1 {
                return Err("ambiguous catalogue group");
            }
            let mut hb = [0; 512];
            let dest = path(node.attr(b"href", &mut hb)?)?;
            if !dest.starts_with(&p[..pn]) || dest.len() <= pn {
                return Err("invalid catalogue group");
            }
            if group_count >= 128 || dest.len() >= 512 {
                return Err("too many catalogue groups");
            }
            for j in 0..group_count {
                if &group_paths[j][..group_lengths[j]] == dest {
                    return Err("duplicate catalogue group");
                }
            }
            group_paths[group_count][..dest.len()].copy_from_slice(dest);
            group_lengths[group_count] = dest.len();
            group_count += 1;
            let title = node.select(b"div.archive-chapter")?;
            let mut tb = [0; 2048];
            let header = title.text(&mut tb)?;
            collect(fetch(BASE, dest)?, slug, mid, header, dest)?;
        }
        if group_count != group_nodes.n {
            return Err("catalogue group ownership mismatch");
        }
    }
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"chapters\":[")?;
    unsafe {
        if CHAPTER_COUNT == 0 {
            return Err("empty catalogue");
        }
        for i in (0..CHAPTER_COUNT).rev() {
            if i != CHAPTER_COUNT - 1 {
                bytes(b, &mut n, b",")?;
            }
            let (start, len) = CHAPTER_OFFSETS[i];
            bytes(b, &mut n, &CHAPTER_BYTES[start..start + len - 1])?;
            bytes(b, &mut n, b",\"index\":")?;
            if !write_usize(b, &mut n, CHAPTER_COUNT - 1 - i) {
                return Err("payload overflow");
            }
            bytes(b, &mut n, b"}")?;
        }
    }
    bytes(
        b,
        &mut n,
        b"],\"page\":{\"nextCursor\":null,\"hasMore\":false}}",
    )?;
    Ok(n)
}
fn supports_notes(r: &[u8]) -> bool {
    let Some(v) = json_object_member(r, b"hostFeatures") else {
        return false;
    };
    let v = trim_ascii(v);
    v.starts_with(b"[")
        && v.ends_with(b"]")
        && v[1..v.len() - 1]
            .split(|c| *c == b',')
            .any(|v| trim_ascii(v) == b"\"text_page_image_v1\"")
}
fn notes_enabled(r: &[u8]) -> bool {
    supports_notes(r)
        && json_object_member(r, b"settings")
            .and_then(|v| json_object_member(v, b"showAuthorsNotes"))
            .map(|v| v == b"true" || v == b"\"true\"")
            .unwrap_or(false)
}
// The existing HTML host returns text nodes without block/BR separators. Preserve
// Android Html.fromHtml's visible paragraph boundaries before the same HTML5 parser
// decodes entities. This second DOM is only used by explicit author notes; original
// image selection, owner checks and default pages keep their original DOM.
static mut NOTES_HTML: [u8; 2 * 1024 * 1024] = [0; 2 * 1024 * 1024];
fn notes_document() -> Result<Node, &'static str> {
    let raw = unsafe { core::slice::from_raw_parts(body_buf().as_ptr(), BODY_LEN) };
    let out = unsafe { &mut *core::ptr::addr_of_mut!(NOTES_HTML) };
    let mut n = 0;
    let mut i = 0;
    while i < raw.len() {
        if raw[i] != b'<' {
            // HTML source indentation is whitespace, unlike explicit BR/blocks.
            if raw[i].is_ascii_whitespace() {
                if n > 0 && out[n - 1] != b' ' {
                    bytes(out, &mut n, b" ")?;
                }
                while i < raw.len() && raw[i].is_ascii_whitespace() {
                    i += 1;
                }
            } else {
                bytes(out, &mut n, &raw[i..i + 1])?;
                i += 1;
            }
            continue;
        }
        let start = i;
        i += 1;
        let mut quote = 0;
        while i < raw.len() {
            let c = raw[i];
            i += 1;
            if quote != 0 {
                if c == quote {
                    quote = 0;
                }
            } else if c == b'\'' || c == b'"' {
                quote = c;
            } else if c == b'>' {
                break;
            }
        }
        if raw.get(i.wrapping_sub(1)) != Some(&b'>') {
            return Err("invalid note HTML");
        }
        let tag = &raw[start..i];
        let closing = tag.get(1) == Some(&b'/');
        let offset = if closing { 2 } else { 1 };
        let end = tag[offset..]
            .iter()
            .position(|c| !c.is_ascii_alphanumeric())
            .unwrap_or(0)
            + offset;
        let name = &tag[offset..end];
        let block = [
            b"div".as_slice(),
            b"p",
            b"blockquote",
            b"li",
            b"ul",
            b"ol",
            b"h1",
            b"h2",
            b"h3",
            b"h4",
            b"h5",
            b"h6",
        ]
        .iter()
        .any(|v| name.eq_ignore_ascii_case(v));
        bytes(out, &mut n, tag)?;
        if !closing && name.eq_ignore_ascii_case(b"br") {
            bytes(out, &mut n, b"&#10;")?;
        } else if block {
            bytes(out, &mut n, b"&#13;")?;
        }
    }
    Node::parse(&out[..n])
}
fn append_notes(
    r: &[u8],
    cp: &[u8],
    comicid: usize,
    first: usize,
    b: &mut [u8],
    n: &mut usize,
) -> Result<(), &'static str> {
    if !notes_enabled(r) {
        return Ok(());
    }
    let doc = notes_document()?;
    let owners = Nodes::get(&doc, b"div.is--comic-page")?;
    let owner = selected(&owners, 0);
    let mut id = [0; 64];
    if number(owner.attr(b"data-comicid", &mut id)?) != Some(comicid) {
        return Err("comic owner mismatch");
    }
    let notes = Nodes::get(&owner, b"div.is--author-notes div.is--comment-box")?;
    for i in 0..notes.n {
        let note = selected(&notes, i);
        let mut author = [0; 4096];
        let authors = Nodes::get(&note, b"a.is--comment-author")?;
        let author = if authors.n == 0 {
            b"".as_slice()
        } else {
            selected(&authors, 0).text(&mut author)?
        };
        let contents = Nodes::get(&note, b"div.is--comment-content")?;
        let mut text = [0; 32768];
        let text = if contents.n == 0 {
            b"".as_slice()
        } else {
            selected(&contents, 0).text(&mut text)?
        };
        let mut plain = [0; 32768];
        let mut pn = 0;
        for &c in text {
            if c == b'\r' {
                // A block supplies a minimum paragraph gap; explicit BRs never
                // get truncated, including three or more consecutive BRs.
                while pn > 0 && plain[pn - 1] == b' ' {
                    pn -= 1;
                }
                let trailing = plain[..pn]
                    .iter()
                    .rev()
                    .take_while(|v| **v == b'\n')
                    .count();
                for _ in trailing..2 {
                    bytes(&mut plain, &mut pn, b"\n")?;
                }
            } else if c == b'\n' {
                while pn > 0 && plain[pn - 1] == b' ' {
                    pn -= 1;
                }
                bytes(&mut plain, &mut pn, b"\n")?;
            } else if c.is_ascii_whitespace() {
                if pn > 0 && plain[pn - 1] != b' ' && plain[pn - 1] != b'\n' {
                    bytes(&mut plain, &mut pn, b" ")?;
                }
            } else {
                bytes(&mut plain, &mut pn, &[c])?;
            }
        }
        let text = trim_ascii(&plain[..pn]);
        bytes(b, n, b",{\"id\":\"")?;
        if !append_json_escaped(b, n, cp) {
            return Err("payload overflow");
        }
        bytes(b, n, b":author-note:")?;
        if !write_usize(b, n, i) {
            return Err("payload overflow");
        }
        bytes(b, n, b"\",\"index\":")?;
        if !write_usize(b, n, first + i) {
            return Err("payload overflow");
        }
        bytes(b, n, b",\"image\":{\"kind\":\"text\",\"text\":{\"template\":\"webtoon-author-notes\",\"heading\":")?;
        let mut heading = [0; 4200];
        let mut hn = 0;
        if authors.n > 0 {
            bytes(&mut heading, &mut hn, b"Author's Notes from ")?;
            bytes(&mut heading, &mut hn, author)?;
        }
        q(b, n, &heading[..hn])?;
        bytes(b, n, b",\"paragraphs\":[")?;
        q(b, n, text)?;
        bytes(b, n, b"]}}}")?;
    }
    Ok(())
}
fn pages(r: &[u8]) -> Result<usize, &'static str> {
    let raw = string(args(r), b"chapterId").ok_or("missing chapterId")?;
    let mut cb = [0; 512];
    let cp = path(unescape(raw, &mut cb)?)?;
    let tail = cp.strip_prefix(b"/read/").ok_or("invalid chapterId")?;
    let sep = find_subslice(tail, b"/comics/").ok_or("invalid chapterId")?;
    let slug = &tail[..sep];
    if slug.is_empty()
        || !slug
            .iter()
            .all(|x| x.is_ascii_alphanumeric() || *x == b'-' || *x == b'_')
    {
        return Err("invalid chapterId");
    }
    let comicid = comic_path(cp, slug)?;
    let doc = fetch(BASE, cp)?;
    let owners = Nodes::get(&doc, b"div.is--comic-page")?;
    if owners.n == 0 {
        return Err("missing comic owner");
    }
    let mut found = 0;
    let mut index = 0;
    for i in 0..owners.n {
        let mut ob = [0; 64];
        let id = number(selected(&owners, i).attr(b"data-comicid", &mut ob)?)
            .ok_or("invalid comic owner")?;
        if id == comicid {
            found += 1;
            index = i;
        }
    }
    if found != 1 || index != 0 {
        return Err("comic owner mismatch");
    }
    let owner = selected(&owners, index);
    let images = Nodes::get(&owner, b"div.is--image-segment div img")?;
    if images.n == 0 {
        return Err("empty comic images");
    }
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"chapterId\":")?;
    q(b, &mut n, cp)?;
    bytes(b, &mut n, b",\"pages\":[")?;
    for i in 0..images.n {
        let image = selected(&images, i);
        let mut ib = [0; 8192];
        let src = image.attr(b"src", &mut ib)?;
        let mut ub = [0; 8192];
        let url = canonical_image(src, &mut ub)?;
        if !url.starts_with(b"https://img.comicfury.com/") {
            return Err("invalid page image host");
        }
        if i > 0 {
            bytes(b, &mut n, b",")?;
        }
        let mut pid = [0; 640];
        let mut count = 0;
        bytes(&mut pid, &mut count, cp)?;
        bytes(&mut pid, &mut count, b":page:")?;
        if !write_usize(&mut pid, &mut count, i) {
            return Err("payload overflow");
        }
        bytes(b, &mut n, b"{\"id\":")?;
        q(b, &mut n, &pid[..count])?;
        bytes(b, &mut n, b",\"index\":")?;
        if !write_usize(b, &mut n, i) {
            return Err("payload overflow");
        }
        bytes(b, &mut n, b",\"image\":{\"kind\":\"url\",\"url\":")?;
        q(b, &mut n, url)?;
        bytes(b, &mut n, b"}}")?;
    }
    append_notes(r, cp, comicid, images.n, b, &mut n)?;
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
    let d="{\"listings\":[{\"id\":\"popular\",\"name\":\"热门\",\"kind\":\"popular\",\"supportsPaging\":true},{\"id\":\"latest\",\"name\":\"最新\",\"kind\":\"latest\",\"supportsPaging\":true}]}".as_bytes();
    payload_buf()[..d.len()].copy_from_slice(d);
    write_success_payload("get_listings", d.len())
}
fn run_get_home(_: &[u8]) -> u32 {
    write_error("get_home", "not_implemented", "not supported")
}
fn run_get_filters(_: &[u8]) -> u32 {
    // Omit a universal sort default: search/popular/latest have distinct
    // established defaults. Keep the already-adopted content flags at zero.
    let d = r#"{"filters":[
      {"id":"tags","name":"Tags","type":"text"},
      {"id":"sort","name":"Sort By","type":"select","options":[{"value":"0","label":"Relevance"},{"value":"1","label":"Popularity"},{"value":"2","label":"Last Update"}]},
      {"id":"lastupdate","name":"Last Updated","type":"select","default":"0","options":[{"value":"0","label":"All Time"},{"value":"1","label":"This Week"},{"value":"2","label":"This Month"},{"value":"3","label":"This Year"},{"value":"4","label":"Completed Only"}]},
      {"id":"completed","name":"Comic Completed","type":"check","default":false},
      {"id":"fv","name":"Violence","type":"select","default":"0","options":[{"value":"0","label":"None / Minimal"},{"value":"1","label":"Violent Content"},{"value":"2","label":"Gore / Graphic"}]},
      {"id":"fn","name":"Frontal Nudity","type":"select","default":"0","options":[{"value":"0","label":"None"},{"value":"1","label":"Occasional"},{"value":"2","label":"Frequent"}]},
      {"id":"fl","name":"Strong Language","type":"select","default":"0","options":[{"value":"0","label":"None"},{"value":"1","label":"Occasional"},{"value":"2","label":"Frequent"}]},
      {"id":"fs","name":"Sexual Content","type":"select","default":"0","options":[{"value":"0","label":"No Sexual Content"},{"value":"1","label":"Sexual Situations"},{"value":"2","label":"Strong Sexual Themes"}]}
    ]}"#.as_bytes();
    payload_buf()[..d.len()].copy_from_slice(d);
    write_success_payload("get_filters", d.len())
}
fn run_get_settings(r: &[u8]) -> u32 {
    let d = if supports_notes(r) {
        "{\"settings\":[{\"id\":\"showAuthorsNotes\",\"name\":\"显示作者附言\",\"kind\":\"toggle\",\"description\":\"在章节末尾显示作者附言（如有）\",\"default\":\"false\"}]}".as_bytes()
    } else {
        b"{\"settings\":[]}".as_slice()
    };
    payload_buf()[..d.len()].copy_from_slice(d);
    write_success_payload("get_settings", d.len())
}
fn run_get_image_request(r: &[u8]) -> u32 {
    execute("get_image_request", image_request(r))
}
fn image_request(r: &[u8]) -> Result<usize, &'static str> {
    let raw = string(args(r), b"url").ok_or("missing image URL")?;
    let mut ub = [0; 8192];
    let u = unescape(raw, &mut ub)?;
    let mut vb = [0; 8192];
    let url = canonical_image(u, &mut vb)?;
    if u != url {
        return Err("invalid image URL");
    }
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"url\":")?;
    q(b, &mut n, url)?;
    bytes(b,&mut n,b",\"headers\":{\"User-Agent\":\"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0\",\"Referer\":\"https://comicfury.com/\",\"Origin\":\"https://comicfury.com\"}}")?;
    Ok(n)
}
koma_source_sdk::koma_source_exports!("comicfury_zh");
