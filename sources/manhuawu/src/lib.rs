#![no_std]
// Public contract: Keiyoushi Manhuawu.kt / MCCMSWeb.kt, Apache-2.0, revision
// 10f06eb833af5990bc5470c84c5e93edc159ad4e. No authentication/mirror/JS processing.
use koma_source_sdk::{
    host,
    json_utils::*,
    source::{SourceCapabilities, SourceInfo},
};
const BASE: &[u8] = b"https://www.mhua5.com";
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.manhuawu.koma",
    name: "漫画屋 (Manhuawu)",
    version: "0.1.4",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description:
        "Manhuawu public MCCMSWeb HTML adapter. Upstream SAFE; individual works unclassified.",
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
koma_source_sdk::koma_source_buffers! { payload:512*1024,http_out:2*1024*1024,body:1024*1024,http_req:8192,scratch:8192, }
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
    if write_bytes(&mut out, &mut n, b"manhuawu stage=")
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
#[derive(Copy, Clone)]
enum OperationError {
    Message(&'static str),
    Http(u16),
}
impl From<&'static str> for OperationError {
    fn from(message: &'static str) -> Self { Self::Message(message) }
}
fn http_message(status: u16) -> &'static str {
    match status {
        401 | 403 => "permission denied",
        404 => "not found",
        429 => "rate limited",
        400..=499 => "client failure",
        500..=599 => "server failure",
        _ => "unexpected HTTP status",
    }
}
fn write_operation_error(op: &str, error: OperationError) -> u32 {
    let status = match error {
        OperationError::Message(message) => return write_error(op, code(message), message),
        OperationError::Http(status) => status,
    };
    let message = http_message(status);
    let mut envelope = [0u8; 512];
    let mut n = 0;
    let ok = write_bytes(&mut envelope, &mut n, br#"{"type":"response","version":1,"ok":false,"operation":""#)
        && append_json_escaped(&mut envelope, &mut n, op.as_bytes())
        && write_bytes(&mut envelope, &mut n, br#"","error":{"code":""#)
        && append_json_escaped(&mut envelope, &mut n, code(message).as_bytes())
        && write_bytes(&mut envelope, &mut n, br#"","message":""#)
        && append_json_escaped(&mut envelope, &mut n, message.as_bytes())
        && write_bytes(&mut envelope, &mut n, br#"","status":"#)
        && write_usize(&mut envelope, &mut n, status as usize)
        && write_bytes(&mut envelope, &mut n, br#"},"hostHints":{"network":false},"warnings":[]}"#);
    if !ok { return write_error(op, "internal_error", "error response overflow"); }
    response_buffer().write(&envelope[..n], false)
}
fn execute<E: Into<OperationError>>(op: &str, r: Result<usize, E>) -> u32 {
    match r {
        Ok(n) => {
            diagnostic(b"operation_ok", op.as_bytes());
            write_success_payload(op, n)
        }
        Err(e) => {
            let error = e.into();
            let message = match error {
                OperationError::Message(message) => message,
                OperationError::Http(status) => http_message(status),
            };
            diagnostic(b"operation_error", code(message).as_bytes());
            write_operation_error(op, error)
        }
    }
}
fn valid_status(s: &[u8]) -> Option<usize> {
    number(s).filter(|n| (100..=599).contains(n))
}
fn fetch(path: &[u8]) -> Result<Node, OperationError> {
    host::log_info(b"manhuawu stage=fetch_begin");
    let mut url = [0; 8192];
    let mut n = 0;
    bytes(&mut url, &mut n, BASE)?;
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
    let status = json_object_member(response, b"status")
        .and_then(valid_status)
        .or_else(|| json_object_member(response, b"statusCode").and_then(valid_status));
    let mut out = [0; 80];
    let mut n = 0;
    if write_bytes(&mut out, &mut n, b"manhuawu stage=http_status value=")
        && write_usize(&mut out, &mut n, status.unwrap_or(0))
    {
        host::log_info(&out[..n]);
    }
    match status {
        Some(200..=299) => {}
        Some(status) => return Err(OperationError::Http(status as u16)),
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
                return Err(c.into());
            }
            return Err("invalid HTTP envelope".into());
        }
    }
    let n = koma_source_sdk::decode_json_body_into(response, body_buf())
        .map_err(|_| "HTTP body invalid or too large")?;
    if n == 0 {
        return Err("empty HTTP body".into());
    }
    core::str::from_utf8(&body_buf()[..n]).map_err(|_| "invalid UTF8")?;
    host::log_info(b"manhuawu stage=body_ready");
    let root = Node::parse(&body_buf()[..n])?;
    host::log_info(b"manhuawu stage=html_ready");
    Ok(root)
}
fn canonical<'a>(s: &'a [u8]) -> Result<&'a [u8], &'static str> {
    let p = s.strip_prefix(BASE).unwrap_or(s);
    if !p.starts_with(b"/")
        || p.starts_with(b"//")
        || p.len() > 1024
        || p.iter()
            .any(|c| !c.is_ascii_alphanumeric() && !b"/._-%".contains(c) && *c < 128)
        || p.split(|c| *c == b'/').any(|v| v == b".." || v == b".")
    {
        return Err("invalid public path");
    }
    core::str::from_utf8(p).map_err(|_| "invalid UTF8")?;
    Ok(p)
}
fn url_decode<'a>(s: &[u8], out: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    let mut i = 0;
    let mut n = 0;
    while i < s.len() {
        let c = if s[i] == b'%' {
            if i + 2 >= s.len() {
                return Err("invalid public path");
            }
            let nib = |c: u8| match c {
                b'0'..=b'9' => Some(c - b'0'),
                b'a'..=b'f' => Some(c - b'a' + 10),
                b'A'..=b'F' => Some(c - b'A' + 10),
                _ => None,
            };
            let c = nib(s[i + 1])
                .and_then(|a| nib(s[i + 2]).map(|b| a * 16 + b))
                .ok_or("invalid public path")?;
            i += 3;
            c
        } else {
            let c = s[i];
            i += 1;
            c
        };
        bytes(out, &mut n, &[c])?;
    }
    core::str::from_utf8(&out[..n]).map_err(|_| "invalid UTF8")?;
    Ok(&out[..n])
}
fn clean_path(p: &[u8]) -> Result<&[u8], &'static str> {
    Ok(canonical(p)?
        .strip_prefix(b"/index.php")
        .unwrap_or(canonical(p)?))
}
fn manga_slug(path: &[u8]) -> Result<&[u8], &'static str> {
    let path = clean_path(path)?;
    let slug = path.strip_prefix(b"/comic/").ok_or("invalid manga path")?;
    let slug = slug.strip_suffix(b".html").unwrap_or(slug);
    if slug.is_empty()
        || slug.len() > 256
        || slug
            .iter()
            .any(|c| !c.is_ascii_alphanumeric() && !b"_-".contains(c))
    {
        return Err("invalid manga path");
    }
    Ok(slug)
}
fn manga_identity(r: &[u8]) -> Result<(&[u8], &[u8]), &'static str> {
    let id = string(args(r), b"mangaId").ok_or("invalid request")?;
    let raw = id.strip_prefix(b"mh:").ok_or("invalid request")?;
    let split = raw
        .iter()
        .rposition(|c| *c == b':')
        .ok_or("invalid request")?;
    let slug = &raw[..split];
    let pic = &raw[split + 1..];
    if slug.is_empty()
        || slug.len() > 256
        || slug
            .iter()
            .any(|c| !c.is_ascii_alphanumeric() && !b"_-".contains(c))
        || pic.is_empty()
        || pic.len() > 20
        || !pic.iter().all(u8::is_ascii_digit)
    {
        return Err("invalid request");
    }
    Ok((id, slug))
}
fn detail_path<'a>(slug: &[u8], out: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    let mut n = 0;
    bytes(out, &mut n, b"/index.php/comic/")?;
    bytes(out, &mut n, slug)?;
    Ok(&out[..n])
}
fn chapter_path(p: &[u8]) -> Result<&[u8], &'static str> {
    let raw = canonical(p)?;
    let path = clean_path(raw)?;
    let n = path
        .strip_prefix(b"/chapter/")
        .ok_or("invalid chapter path")?;
    if n.is_empty() || n.len() > 20 || !n.iter().all(u8::is_ascii_digit) {
        return Err("invalid chapter path");
    }
    Ok(raw)
}
fn chapter_id(r: &[u8]) -> Result<&[u8], &'static str> {
    chapter_path(
        string(args(r), b"chapterId")
            .and_then(|v| v.strip_prefix(b"ch:"))
            .ok_or("invalid request")?,
    )
}
fn qid(b: &mut [u8], n: &mut usize, prefix: &[u8], path: &[u8]) -> Result<(), &'static str> {
    bytes(b, n, b"\"")?;
    if !append_json_escaped(b, n, prefix) || !append_json_escaped(b, n, path) {
        return Err("payload overflow");
    }
    bytes(b, n, b"\"")
}
fn absolute<'a>(url: &[u8], out: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    core::str::from_utf8(url).map_err(|_| "invalid UTF8")?;
    if url.is_empty() || url.iter().any(|v| *v <= 32 || *v == 127) {
        return Err("invalid image URL");
    }
    let mut n = 0;
    if url.starts_with(b"https://") || url.starts_with(b"http://") {
        bytes(out, &mut n, url)?;
    } else if url.starts_with(b"//") {
        bytes(out, &mut n, b"https:")?;
        bytes(out, &mut n, url)?;
    } else if url.starts_with(b"/") {
        bytes(out, &mut n, BASE)?;
        bytes(out, &mut n, url)?;
    } else {
        return Err("unknown image URL");
    }
    Ok(&out[..n])
}
fn page_number(path: &[u8], prefix: &[u8]) -> Result<usize, &'static str> {
    let path = clean_path(path)?;
    if path == prefix {
        return Ok(1);
    }
    let suffix = path
        .strip_prefix(prefix)
        .and_then(|v| v.strip_prefix(b"/page/"))
        .ok_or("unexpected pagination path")?;
    number(suffix)
        .filter(|n| *n > 0)
        .ok_or("invalid pagination number")
}
fn pagination_control<'a>(pager: &Node, selector: &[u8], out: &'a mut [u8]) -> Result<Option<&'a [u8]>, &'static str> {
    let nodes = Nodes::get(pager, selector)?;
    match nodes.n {
        0 => Ok(None),
        1 => selected(&nodes, 0).attr(b"href", out).map(Some),
        _ => Err("ambiguous pagination control"),
    }
}
fn search_page_number(path: &[u8], prefix: &[u8]) -> Result<usize, &'static str> {
    let mut decoded = [0; 8192];
    url_decode(clean_path(path)?, &mut decoded)?
        .strip_prefix(prefix).and_then(number).filter(|n| *n > 0)
        .ok_or("invalid search pagination")
}
fn list(r: &[u8], search: bool) -> Result<usize, OperationError> {
    let pn = page(r)?;
    let mut path = [0; 8192];
    let mut n = 0;
    let mut prefix = [0; 8192];
    let mut plen = 0;
    if search {
        let raw = string(args(r), b"query").ok_or("missing query")?;
        let mut decoded = [0; 2048];
        let term = trim_ascii(unescape(raw, &mut decoded)?);
        if term.is_empty() {
            return Err("empty query".into());
        }
        bytes(&mut path, &mut n, b"/search/")?;
        if !write_url_encoded(&mut path, &mut n, term) {
            return Err("payload overflow".into());
        }
        bytes(&mut path, &mut n, b"/")?;
        if !write_usize(&mut path, &mut n, pn) {
            return Err("payload overflow".into());
        }
    } else {
        let listing = string(args(r), b"listingId").unwrap_or(b"popular");
        bytes(
            &mut prefix,
            &mut plen,
            match listing {
                b"popular" => b"/category/order/hits".as_slice(),
                b"latest" => b"/category/order/addtime",
                _ => return Err("unknown listing".into()),
            },
        )?;
        bytes(&mut path, &mut n, &prefix[..plen])?;
        bytes(&mut path, &mut n, b"/page/")?;
        if !write_usize(&mut path, &mut n, pn) {
            return Err("payload overflow".into());
        }
    }
    let root = fetch(&path[..n])?;
    if root.select(b"#code-div").is_ok() {
        return Err("permission denied".into());
    }
    let nodes = Nodes::get(&root, b".common-comic-item")?;
    if nodes.n == 0 {
        return Err("listing structure missing".into());
    }
    let mut next_buf = [0; 1024];
    let mut current_buf = [0; 1024];
    let mut terminal_buf = [0; 1024];
    let pager = root.select(b"#Pagination, .NewPages")?;
    let current = pagination_control(&pager, b"a.on", &mut current_buf)?
        .ok_or("pagination current missing")?;
    let terminal = pagination_control(&pager, b"a.end", &mut terminal_buf)?
        .ok_or("pagination terminal missing")?;
    let next = pagination_control(&pager, b"a.next", &mut next_buf)?;
    let (current_page, terminal_page, next_page) = if !search {
        (page_number(current, &prefix[..plen])?, page_number(terminal, &prefix[..plen])?,
         next.map(|p| page_number(p, &prefix[..plen])).transpose()?)
    } else {
        // Search controls must stay on the exact requested query route.
        let digits = path[..n].rsplit(|c| *c == b'/').next().ok_or("invalid request")?.len();
        let mut decoded_prefix = [0; 8192];
        let prefix = url_decode(&path[..n - digits], &mut decoded_prefix)?;
        (search_page_number(current, prefix)?, search_page_number(terminal, prefix)?,
         next.map(|p| search_page_number(p, prefix)).transpose()?)
    };
    if current_page != pn || terminal_page < pn {
        return Err("pagination current mismatch".into());
    }
    // The next page can also be the last page. Its relation to the current
    // page, rather than equality with the terminal link, determines continuation.
    let has_more = match next_page {
        Some(target) if pn < terminal_page
            && target == pn.checked_add(1).ok_or("invalid pagination number")?
            && target <= terminal_page => true,
        Some(target) if pn == terminal_page && target == pn => false,
        None if pn == terminal_page => false,
        _ => return Err("pagination target mismatch".into()),
    };
    if nodes.n == 0 && has_more {
        return Err("empty page has next cursor".into());
    }
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"items\":[")?;
    for i in 0..nodes.n {
        let card = selected(&nodes, i);
        let mut text = [0; 4096];
        let title = card
            .select(b".comic__title > :first-child")?
            .text(&mut text)?;
        if title.is_empty() {
            return Err("empty manga title".into());
        }
        let mut href = [0; 1024];
        let slug = manga_slug(
            card.select(b".comic__title > :first-child")?
                .attr(b"href", &mut href)?,
        )?;
        let mut src = [0; 8192];
        let mut abs = [0; 8192];
        let cover = absolute(
            card.select(b"img")?.attr(b"data-original", &mut src)?,
            &mut abs,
        )?;
        if i > 0 {
            bytes(b, &mut n, b",")?;
        }
        bytes(b, &mut n, b"{\"id\":")?;
        bytes(b, &mut n, b"\"mh:")?;
        if !append_json_escaped(b, &mut n, slug) {
            return Err("payload overflow".into());
        }
        bytes(b, &mut n, b":0\"")?;
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
            return Err("payload overflow".into());
        }
        bytes(b, &mut n, b"\",\"hasMore\":true}}")?;
    } else {
        bytes(b, &mut n, b"null,\"hasMore\":false}}")?;
    }
    Ok(n)
}
fn detail(r: &[u8]) -> Result<usize, OperationError> {
    let (id, slug) = manga_identity(r)?;
    let mut path = [0; 1024];
    let root = fetch(detail_path(slug, &mut path)?)?;
    let owner = root.select(b".de-info__box")?;
    let mut text = [0; 32768];
    let title = owner.select(b".comic-title")?.text(&mut text)?;
    if title.is_empty() {
        return Err("empty manga title".into());
    }
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"manga\":{\"id\":")?;
    q(b, &mut n, id)?;
    bytes(b, &mut n, b",\"title\":")?;
    q(b, &mut n, title)?;
    let mut src = [0; 8192];
    let mut abs = [0; 8192];
    let cover = absolute(owner.select(b"img")?.attr(b"src", &mut src)?, &mut abs)?;
    bytes(b, &mut n, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(b, &mut n, cover)?;
    bytes(b, &mut n, b"},\"contentRating\":\"unknown\",\"authors\":[")?;
    let author = owner.select(b".name")?.text(&mut text)?;
    if author.is_empty() {
        return Err("empty author".into());
    }
    q(b, &mut n, author)?;
    bytes(b, &mut n, b"],\"tags\":[")?;
    let tags = Nodes::get(&owner, b".comic-status a")?;
    for i in 0..tags.n {
        if i > 0 {
            bytes(b, &mut n, b",")?;
        }
        let t = selected(&tags, i).text(&mut text)?;
        if t.is_empty() {
            return Err("empty tag".into());
        }
        q(b, &mut n, t)?;
    }
    bytes(b, &mut n, b"],\"status\":\"unknown\",\"summary\":")?;
    q(b, &mut n, owner.select(b".intro-total")?.text(&mut text)?)?;
    let mut page_url = [0; 2048];
    let mut page_len = 0;
    bytes(&mut page_url, &mut page_len, BASE)?;
    bytes(&mut page_url, &mut page_len, detail_path(slug, &mut path)?)?;
    bytes(b, &mut n, b",\"links\":[{\"kind\":\"source\",\"url\":")?;
    q(b, &mut n, &page_url[..page_len])?;
    bytes(b, &mut n, b"}]}}")?;
    Ok(n)
}
fn explicit_chapter_number(title: &[u8]) -> Option<&[u8]> {
    let rest = title.strip_prefix("第".as_bytes())?;
    let count = rest.iter().take_while(|c| c.is_ascii_digit()).count();
    if count == 0 || count > 9 || (count > 1 && rest[0] == b'0') {
        return None;
    }
    let tail = rest.get(count..)?;
    if ["话", "話", "回", "章"]
        .iter()
        .any(|label| tail.starts_with(label.as_bytes()))
    {
        Some(&rest[..count])
    } else {
        None
    }
}
fn chapters(r: &[u8]) -> Result<usize, OperationError> {
    let (id, slug) = manga_identity(r)?;
    let mut path = [0; 1024];
    let root = fetch(detail_path(slug, &mut path)?)?;
    let _owner = root.select(b".chapter__list-box")?;
    let nodes = Nodes::get(&root, b".chapter__list-box > li > :first-child")?;
    if nodes.n == 0 {
        return Err("chapter structure missing".into());
    }
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"chapters\":[")?;
    for (index, i) in (0..nodes.n).rev().enumerate() {
        let a = selected(&nodes, i);
        let mut href = [0; 1024];
        let cp = chapter_path(a.attr(b"href", &mut href)?)?;
        let mut text = [0; 4096];
        let title = a.text(&mut text)?;
        if title.is_empty() {
            return Err("empty chapter title".into());
        }
        if index > 0 {
            bytes(b, &mut n, b",")?;
        }
        bytes(b, &mut n, b"{\"id\":")?;
        qid(b, &mut n, b"ch:", cp)?;
        bytes(b, &mut n, b",\"mangaId\":")?;
        q(b, &mut n, id)?;
        bytes(b, &mut n, b",\"title\":")?;
        q(b, &mut n, title)?;
        bytes(b, &mut n, b",\"index\":")?;
        if !write_usize(b, &mut n, index) {
            return Err("payload overflow".into());
        }
        bytes(b, &mut n, b",\"chapterNumber\":")?;
        bytes(b, &mut n, explicit_chapter_number(title).unwrap_or(b"null"))?;
        bytes(b,&mut n,b",\"volumeNumber\":null,\"language\":\"zh\",\"publishedAt\":null,\"updatedAt\":null,\"pageCount\":null,\"scanlator\":null}")?;
    }
    bytes(
        b,
        &mut n,
        b"],\"page\":{\"nextCursor\":null,\"hasMore\":false}}",
    )?;
    Ok(n)
}
fn pages(r: &[u8]) -> Result<usize, OperationError> {
    let cp = chapter_id(r)?;
    let root = fetch(cp)?;
    let nodes = Nodes::get(&root, b"img[data-original]")?;
    if nodes.n == 0 {
        return Err("page structure missing".into());
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
    for i in 0..nodes.n {
        let img = selected(&nodes, i);
        let mut src = [0; 8192];
        let mut abs = [0; 8192];
        let url = absolute(img.attr(b"data-original", &mut src)?, &mut abs)?;
        if i > 0 {
            bytes(b, &mut n, b",")?;
        }
        bytes(b, &mut n, b"{\"id\":\"p:")?;
        if !write_usize(b, &mut n, i) {
            return Err("payload overflow".into());
        }
        bytes(b, &mut n, b"\",\"index\":")?;
        if !write_usize(b, &mut n, i) {
            return Err("payload overflow".into());
        }
        bytes(b, &mut n, b",\"image\":{\"kind\":\"url\",\"url\":")?;
        q(b, &mut n, url)?;
        bytes(b, &mut n, b"}}")?;
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
fn run_get_settings(_: &[u8]) -> u32 {
    write_error("get_settings", "not_implemented", "not supported")
}
fn image_request(r: &[u8]) -> Result<usize, &'static str> {
    let raw = string(args(r), b"url").ok_or("missing image URL")?;
    let mut decoded = [0; 8192];
    let url = unescape(raw, &mut decoded).map_err(|_| "invalid image URL")?;
    core::str::from_utf8(url).map_err(|_| "invalid image URL")?;
    let rest = url
        .strip_prefix(b"https://")
        .or_else(|| url.strip_prefix(b"http://"))
        .ok_or("invalid image URL")?;
    if url.len() >= 8192
        || url
            .iter()
            .any(|v| *v <= 32 || *v == 127 || b"\\#\"<>".contains(v))
    {
        return Err("invalid image URL");
    }
    let authority = rest
        .split(|v| *v == b'/' || *v == b'?')
        .next()
        .ok_or("invalid image URL")?;
    if authority.is_empty()
        || !authority.contains(&b'.')
        || authority.iter().all(|c| c.is_ascii_digit() || *c == b'.')
        || authority.split(|c| *c == b'.').any(|label| {
            label.is_empty()
                || label.len() > 63
                || !label[0].is_ascii_alphanumeric()
                || !label[label.len() - 1].is_ascii_alphanumeric()
                || label
                    .iter()
                    .any(|c| !c.is_ascii_alphanumeric() && *c != b'-')
        })
    {
        return Err("invalid image URL");
    }
    let mut i = 0;
    while i < url.len() {
        if url[i] == b'%' {
            let nib = |c: u8| match c {
                b'0'..=b'9' => Some(c - b'0'),
                b'a'..=b'f' => Some(c - b'a' + 10),
                b'A'..=b'F' => Some(c - b'A' + 10),
                _ => None,
            };
            if i + 2 >= url.len() {
                return Err("invalid image URL");
            }
            let byte = nib(url[i + 1])
                .and_then(|a| nib(url[i + 2]).map(|b| a * 16 + b))
                .ok_or("invalid image URL")?;
            if byte < 32 || byte == 127 {
                return Err("invalid image URL");
            }
            i += 3;
        } else {
            i += 1;
        }
    }
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
koma_source_sdk::koma_source_exports!("manhuawu");
