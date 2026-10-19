#![no_std]
use koma_source_sdk::{
    host,
    json_utils::*,
    source::{SourceCapabilities, SourceInfo},
};
const BASE: &[u8] = b"https://www.manhuagui.com";
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.manhuagui.koma",
    name: "Manhuagui",
    version: "0.1.2",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "漫画柜普通公开 HTML 适配；不支持成人隐藏目录或挑战。",
    content_rating: "nsfw",
};
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: true,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: true,
    manga_list: true,
    image_request: true,
    home: false,
    filters: false,
    settings: false,
    credentials: false,
};
koma_source_sdk::koma_source_buffers! { payload:1024*1024,http_out:5*1024*1024,body:2*1024*1024,http_req:16*1024,scratch:512*1024, }
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

fn absolute<'a>(u: &[u8], out: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    let mut n = 0;
    if u.starts_with(b"https://") {
        bytes(out, &mut n, u)?
    } else if u.starts_with(b"//") {
        bytes(out, &mut n, b"https:")?;
        bytes(out, &mut n, u)?
    } else if u.starts_with(b"/") {
        bytes(out, &mut n, BASE)?;
        bytes(out, &mut n, u)?
    } else {
        return Err("invalid public URL");
    };
    let s = &out[..n];
    let tail = s.strip_prefix(b"https://").ok_or("invalid public URL")?;
    let host = tail
        .split(|c| matches!(c, b'/' | b'?' | b'#'))
        .next()
        .ok_or("invalid public URL")?;
    if host.is_empty()
        || host
            .iter()
            .any(|c| !c.is_ascii_alphanumeric() && !matches!(c, b'.' | b'-' | b':'))
        || host.starts_with(b".")
        || host.ends_with(b".")
    {
        return Err("invalid public URL");
    };
    if s.iter()
        .any(|c| c.is_ascii_control() || matches!(c, b' ' | b'\\' | b'"'))
    {
        return Err("invalid public URL");
    };
    Ok(s)
}
// Only finite stages, known operation/code labels and the numeric HTTP status are logged.
// URLs, request arguments, titles, headers and response data never enter diagnostics.
fn diagnostic_status(status: u16) {
    let mut b = [0; 48];
    let mut n = 0;
    let _ = write_bytes(&mut b, &mut n, b"manhuagui fetch http_status=");
    let _ = write_usize(&mut b, &mut n, status as usize);
    host::log_info(&b[..n]);
}
fn diagnostic_error(op: &str, code: &str) {
    let known_op = match op {
        "search" => b"search".as_slice(),
        "get_manga_list" => b"get_manga_list",
        "get_manga" => b"get_manga",
        "get_chapters" => b"get_chapters",
        "get_pages" => b"get_pages",
        "get_image_request" => b"get_image_request",
        _ => b"unknown",
    };
    let known_code = match code {
        "cancelled" => b"cancelled".as_slice(),
        "network_error" => b"network_error",
        "timeout" => b"timeout",
        "not_found" => b"not_found",
        "rate_limited" => b"rate_limited",
        "permission_denied" => b"permission_denied",
        "authentication_required" => b"authentication_required",
        "not_implemented" => b"not_implemented",
        _ => b"source_error",
    };
    let mut b = [0; 128];
    let mut n = 0;
    let _ = write_bytes(&mut b, &mut n, b"manhuagui error op=");
    let _ = write_bytes(&mut b, &mut n, known_op);
    let _ = write_bytes(&mut b, &mut n, b" code=");
    let _ = write_bytes(&mut b, &mut n, known_code);
    host::log_info(&b[..n]);
}
static mut LAST_HTTP_STATUS: u16 = 0;
fn fetch_body(path: &[u8]) -> Result<usize, &'static str> {
    unsafe { LAST_HTTP_STATUS = 0 };
    host::log_info(b"manhuagui fetch stage=enter");
    if host::check_cancel() {
        return Err("cancelled");
    };
    let mut url = [0; 8192];
    let u = absolute(path, &mut url)?;
    let b = http_req_buf();
    let n = pc_request(b, u)?;
    host::log_info(b"manhuagui fetch stage=request_ready");
    let size = host::http_request(&b[..n], http_out()).map_err(|_| "network request failed")?;
    host::log_info(b"manhuagui fetch stage=http_return");
    if size >= http_out().len() {
        return Err("HTTP response may be truncated");
    }
    let response = &http_out()[..size];
    let status = json_object_member(response, b"status")
        .or_else(|| json_object_member(response, b"statusCode"))
        .map(koma_source_sdk::parse_status_code)
        .unwrap_or(0);
    if let Some(e) = transport::failed(response, status) {
        host::log_info(b"manhuagui fetch stage=transport_failed");
        return Err(e);
    }
    unsafe { LAST_HTTP_STATUS = status };
    diagnostic_status(status);
    if let Some(e) = transport::http_error(status) {
        return Err(e);
    }
    let n = koma_source_sdk::decode_json_body_into(response, body_buf())
        .map_err(|_| "invalid HTTP body")?;
    if n >= body_buf().len() {
        return Err("HTTP body may be truncated");
    }
    host::log_info(b"manhuagui fetch stage=body_ready");
    Ok(n)
}
fn error(op: &str, e: &str) -> u32 {
    let mut diagnostic = [0; 192];
    let mut n = 0;
    let _ = write_bytes(&mut diagnostic, &mut n, e.as_bytes());
    let status = unsafe { LAST_HTTP_STATUS };
    if status > 0 {
        let _ = write_bytes(&mut diagnostic, &mut n, b" (HTTP ");
        let _ = write_usize(&mut diagnostic, &mut n, status as usize);
        let _ = write_bytes(&mut diagnostic, &mut n, b")");
    };
    let message = core::str::from_utf8(&diagnostic[..n]).unwrap_or(e);
    let code = match e {
        "cancelled" => "cancelled",
        "request timed out" => "timeout",
        "network request failed"
        | "transport unavailable"
        | "transport rejected"
        | "invalid transport response" => "network_error",
        "not found" => "not_found",
        "rate limited" => "rate_limited",
        "public request rejected" | "hidden or adult content unsupported" => "permission_denied",
        "authentication required" => "authentication_required",
        "captcha unsupported" => "permission_denied",
        "public comic unavailable" => "not_found",
        _ => "source_error",
    };
    diagnostic_error(op, code);
    write_error(op, code, message)
}
fn execute(op: &str, result: Result<usize, &'static str>) -> u32 {
    match result {
        Ok(n) => write_success_payload(op, n),
        Err(e) => error(op, e),
    }
}

const UA:&[u8]=b"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36";
const LANGUAGE: &[u8] = b"zh-CN,zh;q=0.9,en-US;q=0.8,en;q=0.7";
fn headers(b: &mut [u8], n: &mut usize) -> Result<(), &'static str> {
    bytes(b, n, b"{\"User-Agent\":")?;
    q(b, n, UA)?;
    bytes(b, n, b",\"Accept-Language\":")?;
    q(b, n, LANGUAGE)?;
    bytes(
        b,
        n,
        b",\"Referer\":\"https://www.manhuagui.com/\",\"Origin\":\"https://www.manhuagui.com\"}",
    )
}
fn pc_request(b: &mut [u8], u: &[u8]) -> Result<usize, &'static str> {
    let mut n = 0;
    bytes(b, &mut n, b"{\"version\":1,\"method\":\"GET\",\"url\":")?;
    q(b, &mut n, u)?;
    bytes(b, &mut n, b",\"headers\":")?;
    headers(b, &mut n)?;
    bytes(
        b,
        &mut n,
        b",\"timeoutMs\":15000,\"responseKind\":\"bodyText\"}",
    )?;
    Ok(n)
}
fn positive(s: &[u8]) -> bool {
    !s.is_empty() && s[0] != b'0' && number(s).is_some()
}
fn book_id<'a>(s: &'a [u8]) -> Result<&'a [u8], &'static str> {
    let s = s.strip_prefix(b"manga:").unwrap_or(s);
    let s = s.strip_prefix(b"comic/").unwrap_or(s);
    let s = s.strip_suffix(b"/").unwrap_or(s);
    if !positive(s) {
        return Err("invalid manga ID");
    };
    Ok(s)
}
fn manga_id(r: &[u8]) -> Result<&[u8], &'static str> {
    book_id(
        string(args(r), b"mangaId")
            .or_else(|| string(args(r), b"id"))
            .ok_or("missing manga ID")?,
    )
}
fn canonical_path(s: &[u8]) -> Result<&[u8], &'static str> {
    let p = s.strip_prefix(BASE).unwrap_or(s);
    if !p.starts_with(b"/")
        || p.starts_with(b"//")
        || p.len() > 4096
        || p.iter()
            .any(|c| c.is_ascii_control() || matches!(c, b' ' | b'\\' | b'"' | b'\'' | b'?' | b'#'))
        || p.split(|c| *c == b'/').any(|s| s == b"." || s == b"..")
    {
        return Err("invalid canonical path");
    };
    Ok(p)
}
fn href_id(h: &[u8]) -> Result<&[u8], &'static str> {
    book_id(
        canonical_path(h)?
            .strip_prefix(b"/comic/")
            .ok_or("invalid manga href")?,
    )
}
fn chapter_path(h: &[u8]) -> Result<&[u8], &'static str> {
    let p = canonical_path(h)?;
    let rest = p.strip_prefix(b"/comic/").ok_or("invalid chapter path")?;
    let slash = rest
        .iter()
        .position(|c| *c == b'/')
        .ok_or("invalid chapter path")?;
    if !positive(&rest[..slash])
        || !positive(
            rest[slash + 1..]
                .strip_suffix(b".html")
                .ok_or("invalid chapter suffix")?,
        )
    {
        return Err("invalid chapter path");
    };
    Ok(p)
}
fn qbook(b: &mut [u8], n: &mut usize, id: &[u8]) -> Result<(), &'static str> {
    bytes(b, n, b"\"comic/")?;
    bytes(b, n, id)?;
    bytes(b, n, b"\"")
}
fn fetch_doc(p: &[u8]) -> Result<Node, &'static str> {
    let n = fetch_body(p)?;
    let root = Node::parse(&body_buf()[..n])?;
    // Explicitly refuse visibility/access gates; never decode hidden catalogue or set cookies.
    if root.select(b"#__VIEWSTATE, #erroraudit_show").is_ok() {
        return Err("hidden or adult content unsupported");
    }
    host::log_info(b"manhuagui fetch stage=html_ready");
    Ok(root)
}
fn manga_doc(id: &[u8]) -> Result<Node, &'static str> {
    let mut p = [0; 256];
    let mut n = 0;
    bytes(&mut p, &mut n, b"/comic/")?;
    bytes(&mut p, &mut n, id)?;
    bytes(&mut p, &mut n, b"/")?;
    fetch_doc(&p[..n])
}
fn image_url<'a>(node: &Node, out: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    let mut a = [0; 8192];
    if let Some(v) = node.optional_attr(b"src", &mut a)? {
        absolute(v, out)
    } else {
        absolute(node.attr(b"data-src", &mut a)?, out)
    }
}
fn list(r: &[u8], search: bool) -> Result<usize, &'static str> {
    let p = page(r)?;
    if p > 1_000_000 {
        return Err("invalid page cursor");
    };
    let mut route = [0; 8192];
    let mut len = 0;
    if search {
        let raw = string(args(r), b"query").ok_or("missing query")?;
        let mut decoded = [0; 4096];
        let v = unescape(raw, &mut decoded)?;
        if trim_ascii(v).is_empty() {
            return Err("empty text search unsupported");
        };
        bytes(&mut route, &mut len, b"/s/")?;
        if !write_url_encoded(&mut route, &mut len, v) {
            return Err("query overflow");
        };
        bytes(&mut route, &mut len, b"_p")?
    } else {
        bytes(
            &mut route,
            &mut len,
            match string(args(r), b"listingId").unwrap_or(b"popular") {
                b"popular" => b"/list/view_p",
                b"latest" => b"/list/update_p",
                _ => return Err("unknown listing"),
            },
        )?
    }
    if !write_usize(&mut route, &mut len, p) {
        return Err("page overflow");
    };
    bytes(&mut route, &mut len, b".html")?;
    let root = fetch_doc(&route[..len])?;
    let selector = if search {
        b"div.book-result > ul > li".as_slice()
    } else {
        b"ul#contList > li"
    };
    let _owner = root.select(if search {
        b"div.book-result".as_slice()
    } else {
        b"ul#contList"
    })?;
    let cards = Nodes::get(&root, selector)?;
    let next = root.select(b"span.current + a").ok();
    if cards.n == 0 && next.is_some() {
        return Err("empty page has next cursor");
    }
    let mut target = None;
    if let Some(a) = next {
        let mut h = [0; 8192];
        let h = canonical_path(a.attr(b"href", &mut h)?)?;
        let marker = h
            .windows(2)
            .rposition(|w| w == b"_p")
            .ok_or("invalid next page path")?;
        let nr = h[marker + 2..]
            .strip_suffix(b".html")
            .and_then(number)
            .ok_or("invalid next page cursor")?;
        if &h[..marker + 2]
            != route[..len]
                .split_at(
                    route[..len]
                        .windows(2)
                        .rposition(|w| w == b"_p")
                        .ok_or("invalid list path")?
                        + 2,
                )
                .0
            || nr != p.checked_add(1).ok_or("page overflow")?
        {
            return Err("unexpected next page URL");
        };
        target = Some(nr)
    }
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"items\":[")?;
    for i in 0..cards.n {
        let card = selected(&cards, i);
        let a = card.select(if search {
            b"div.book-detail dl > dt > a".as_slice()
        } else {
            b"a.bcover"
        })?;
        let mut h = [0; 8192];
        let id = href_id(a.attr(b"href", &mut h)?)?;
        let mut text = [0; 16384];
        let title = a.attr(b"title", &mut text)?;
        if trim_ascii(title).is_empty() {
            return Err("manga title missing");
        }
        let img = card.select(if search {
            b"div.book-cover > a.bcover > img".as_slice()
        } else {
            b"a.bcover img"
        })?;
        let mut cover = [0; 8192];
        let cover = image_url(&img, &mut cover)?;
        if i > 0 {
            bytes(b, &mut n, b",")?
        }
        bytes(b, &mut n, b"{\"id\":")?;
        qbook(b, &mut n, id)?;
        bytes(b, &mut n, b",\"title\":")?;
        q(b, &mut n, title)?;
        bytes(b, &mut n, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
        q(b, &mut n, cover)?;
        bytes(b, &mut n, b"},\"contentRating\":\"unknown\"}")?;
    }
    bytes(b, &mut n, b"],\"page\":{\"nextCursor\":")?;
    if let Some(t) = target {
        bytes(b, &mut n, b"\"")?;
        if !write_usize(b, &mut n, t) {
            return Err("payload overflow");
        };
        bytes(b, &mut n, b"\",\"hasMore\":true}}")?
    } else {
        bytes(b, &mut n, b"null,\"hasMore\":false}}")?
    };
    Ok(n)
}
fn labels(
    root: &Node,
    b: &mut [u8],
    n: &mut usize,
    label1: &[u8],
    label2: &[u8],
) -> Result<(), &'static str> {
    let spans = Nodes::get(root, b"span")?;
    let mut text = [0; 16384];
    let mut found = false;
    let mut count = 0;
    for i in 0..spans.n {
        let span = selected(&spans, i);
        let t = span.text(&mut text)?;
        if find_subslice(t, label1).is_none() && find_subslice(t, label2).is_none() {
            continue;
        }
        if found {
            return Err("ambiguous manga metadata label");
        };
        found = true;
        let links = Nodes::get(&span, b"a")?;
        for j in 0..links.n {
            let t = selected(&links, j).text(&mut text)?;
            if t.is_empty() {
                continue;
            };
            if count > 0 {
                bytes(b, n, b",")?
            };
            q(b, n, t)?;
            count += 1;
        }
    }
    Ok(())
}
fn detail(r: &[u8]) -> Result<usize, &'static str> {
    let id = manga_id(r)?;
    let root = manga_doc(id)?;
    let mut text = [0; 32768];
    let title = root
        .select(b"div.book-title > h1:nth-child(1)")?
        .text(&mut text)?;
    if title.is_empty() {
        return Err("manga title missing");
    }
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"manga\":{\"id\":")?;
    qbook(b, &mut n, id)?;
    bytes(b, &mut n, b",\"title\":")?;
    q(b, &mut n, title)?;
    bytes(b, &mut n, b",\"contentRating\":\"unknown\",\"cover\":")?;
    if let Ok(img) = root.select(b"p.hcover > img") {
        let mut u = [0; 8192];
        let u = image_url(&img, &mut u)?;
        bytes(b, &mut n, b"{\"kind\":\"url\",\"url\":")?;
        q(b, &mut n, u)?;
        bytes(b, &mut n, b"}")?
    } else {
        bytes(b, &mut n, b"null")?
    }
    bytes(b, &mut n, b",\"authors\":[")?;
    labels(
        &root,
        b,
        &mut n,
        "漫画作者".as_bytes(),
        "漫畫作者".as_bytes(),
    )?;
    bytes(b, &mut n, b"],\"tags\":[")?;
    labels(
        &root,
        b,
        &mut n,
        "漫画剧情".as_bytes(),
        "漫畫劇情".as_bytes(),
    )?;
    bytes(b, &mut n, b"],\"summary\":")?;
    if let Ok(s) = root.select(b"div#intro-all") {
        q(b, &mut n, s.text(&mut text)?)?
    } else {
        q(b, &mut n, b"")?
    }
    bytes(b, &mut n, b",\"status\":")?;
    let status =
        if let Ok(s) = root.select(b"div.book-detail > ul.detail-list > li.status > span > span") {
            match s.text(&mut text)? {
                v if v == "连载中".as_bytes() || v == "連載中".as_bytes() => {
                    b"ongoing".as_slice()
                }
                v if v == "已完结".as_bytes() || v == "已完結".as_bytes() => b"completed",
                _ => b"unknown",
            }
        } else {
            b"unknown"
        };
    q(b, &mut n, status)?;
    bytes(b, &mut n, b"}}")?;
    Ok(n)
}
fn chapters(r: &[u8]) -> Result<usize, &'static str> {
    let id = manga_id(r)?;
    let root = manga_doc(id)?;
    let sections = Nodes::get(&root, b"[id^=chapter-list-]")?;
    if sections.n == 0 {
        return Err("chapter structure missing");
    }
    let b = payload_buf();
    let mut n = 0;
    let mut count = 0;
    bytes(b, &mut n, b"{\"chapters\":[")?;
    for si in 0..sections.n {
        let uls = Nodes::get(&selected(&sections, si), b"ul")?;
        if uls.n == 0 {
            return Err("chapter group structure missing");
        }
        for ui in (0..uls.n).rev() {
            let links = Nodes::get(&selected(&uls, ui), b"li > a.status0")?;
            for j in 0..links.n {
                if count >= 8192 {
                    return Err("chapter count overflow");
                };
                let a = selected(&links, j);
                let mut h = [0; 8192];
                let cp = chapter_path(a.attr(b"href", &mut h)?)?;
                let rest = &cp[b"/comic/".len()..];
                if !rest.starts_with(id) || rest.get(id.len()) != Some(&b'/') {
                    return Err("chapter manga mismatch");
                }
                let mut title = [0; 16384];
                let t = if a
                    .optional_attr(b"title", &mut title)?
                    .is_some_and(|t| !t.is_empty())
                {
                    a.attr(b"title", &mut title)?
                } else {
                    a.select(b"span")?.text(&mut title)?
                };
                if t.is_empty() {
                    return Err("chapter title missing");
                }
                if count > 0 {
                    bytes(b, &mut n, b",")?
                }
                bytes(b, &mut n, b"{\"id\":\"")?;
                bytes(b, &mut n, BASE)?;
                bytes(b, &mut n, cp)?;
                bytes(b, &mut n, b"\",\"mangaId\":")?;
                qbook(b, &mut n, id)?;
                bytes(b, &mut n, b",\"title\":")?;
                q(b, &mut n, t)?;
                bytes(b, &mut n, b",\"index\":")?;
                if !write_usize(b, &mut n, count) {
                    return Err("payload overflow");
                };
                bytes(
                    b,
                    &mut n,
                    b",\"chapterNumber\":null,\"updatedAt\":null,\"scanlator\":null}",
                )?;
                count += 1;
            }
        }
    }
    if count == 0 {
        return Err("public chapter list empty");
    };
    bytes(b, &mut n, b"]}")?;
    Ok(n)
}
mod lz;
mod packed;
mod transport;
fn unique<'a>(obj: &'a [u8], key: &[u8]) -> Result<&'a [u8], &'static str> {
    let mut p = ws(obj, 0);
    if obj.get(p) != Some(&b'{') {
        return Err("metadata must be object");
    };
    p = ws(obj, p + 1);
    let mut found = None;
    while obj.get(p) != Some(&b'}') {
        let end = json_string_end(obj, p).ok_or("invalid metadata key")?;
        let mut decoded = [0; 256];
        let k = unescape(&obj[p + 1..end - 1], &mut decoded)?;
        p = ws(obj, end);
        if obj.get(p) != Some(&b':') {
            return Err("invalid metadata separator");
        };
        p = ws(obj, p + 1);
        let end = json_value_end(obj, p, 0).ok_or("invalid metadata value")?;
        if k == key {
            if found.is_some() {
                return Err("ambiguous metadata key");
            };
            found = Some(&obj[p..end])
        };
        p = ws(obj, end);
        if obj.get(p) == Some(&b',') {
            p = ws(obj, p + 1)
        } else if obj.get(p) != Some(&b'}') {
            return Err("invalid metadata object");
        }
    }
    if ws(obj, p + 1) != obj.len() {
        return Err("metadata trailing data");
    };
    found.ok_or("required metadata field missing")
}
fn strvalue<'a>(v: &[u8], out: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    if v.first() != Some(&b'"') || json_string_end(v, 0) != Some(v.len()) {
        return Err("metadata string expected");
    };
    unescape(&v[1..v.len() - 1], out)
}
fn pages(r: &[u8]) -> Result<usize, &'static str> {
    let raw = string(args(r), b"chapterId").ok_or("missing chapter ID")?;
    let cp = chapter_path(raw)?;
    let n = fetch_body(cp)?;
    let root = Node::parse(&body_buf()[..n])?;
    if root.select(b"#__VIEWSTATE, #erroraudit_show").is_ok() {
        return Err("hidden or adult content unsupported");
    };
    drop(root);
    let n = packed::unpack(&body_buf()[..n], scratch_b())?;
    let data = &scratch_b()[..n];
    let start = data
        .iter()
        .position(|v| *v == b'{')
        .ok_or("Comic JSON missing")?;
    let end = json_value_end(data, start, 0).ok_or("invalid complete Comic JSON")?;
    if data[end..].contains(&b'{') {
        return Err("ambiguous Comic JSON");
    };
    let obj = &data[start..end];
    let files = trim_ascii(unique(obj, b"files")?);
    if files.first() != Some(&b'[') || files.last() != Some(&b']') {
        return Err("files must be array");
    };
    let mut pathbuf = [0; 8192];
    let path = strvalue(unique(obj, b"path")?, &mut pathbuf)?;
    if !path.starts_with(b"/")
        || path.contains(&b'?')
        || path.contains(&b'#')
        || path.split(|c| *c == b'/').any(|s| s == b"." || s == b"..")
    {
        return Err("invalid image path");
    }
    let sl = unique(obj, b"sl")?;
    let e = unique(sl, b"e")?;
    if !e.is_empty() && e.iter().any(|c| !c.is_ascii_digit()) {
        return Err("invalid image e parameter");
    };
    let mut m = [0; 8192];
    let m = strvalue(unique(sl, b"m")?, &mut m)?;
    let b = payload_buf();
    let mut n = 0;
    let mut pos = ws(files, 1);
    let mut index = 0;
    bytes(b, &mut n, b"{\"pages\":[")?;
    while files.get(pos) != Some(&b']') {
        if index >= 2048 {
            return Err("page count overflow");
        };
        let end = json_string_end(files, pos).ok_or("file must be string")?;
        let mut decoded = [0; 8192];
        let file = unescape(&files[pos + 1..end - 1], &mut decoded)?;
        if file.is_empty()
            || file.starts_with(b"/")
            || file.contains(&b'?')
            || file.contains(&b'#')
            || file.split(|c| *c == b'/').any(|s| s == b"." || s == b"..")
        {
            return Err("invalid image file");
        }
        let mut url = [0; 16384];
        let mut len = 0;
        bytes(&mut url, &mut len, b"https://i.hamreus.com")?;
        bytes(&mut url, &mut len, path)?;
        bytes(&mut url, &mut len, file)?;
        bytes(&mut url, &mut len, b"?e=")?;
        bytes(&mut url, &mut len, e)?;
        bytes(&mut url, &mut len, b"&m=")?;
        if !write_url_encoded(&mut url, &mut len, m) {
            return Err("image parameter overflow");
        };
        let mut checked = [0; 16384];
        let image = absolute(&url[..len], &mut checked)?;
        if index > 0 {
            bytes(b, &mut n, b",")?
        }
        bytes(b, &mut n, b"{\"id\":\"page:")?;
        bytes(b, &mut n, BASE)?;
        bytes(b, &mut n, cp)?;
        bytes(b, &mut n, b":")?;
        if !write_usize(b, &mut n, index) {
            return Err("payload overflow");
        };
        bytes(b, &mut n, b"\",\"index\":")?;
        if !write_usize(b, &mut n, index) {
            return Err("payload overflow");
        };
        bytes(b, &mut n, b",\"image\":{\"kind\":\"url\",\"url\":")?;
        q(b, &mut n, image)?;
        bytes(b, &mut n, b"}}")?;
        index += 1;
        pos = ws(files, end);
        if files.get(pos) == Some(&b',') {
            pos = ws(files, pos + 1);
            if files.get(pos) == Some(&b']') {
                return Err("trailing file comma");
            }
        } else if files.get(pos) != Some(&b']') {
            return Err("invalid file separator");
        }
    }
    if index == 0 {
        return Err("public image list empty");
    };
    if ws(files, pos + 1) != files.len() {
        return Err("file array trailing data");
    };
    bytes(b, &mut n, b"]}")?;
    host::log_info(b"manhuagui pages stage=payload_ready");
    Ok(n)
}
fn image_request(r: &[u8]) -> Result<usize, &'static str> {
    let raw = string(args(r), b"url").ok_or("missing image URL")?;
    let mut decoded = [0; 16384];
    let u = unescape(raw, &mut decoded)?;
    let mut checked = [0; 16384];
    let u = absolute(u, &mut checked)?;
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"url\":")?;
    q(b, &mut n, u)?;
    bytes(b, &mut n, b",\"headers\":")?;
    headers(b, &mut n)?;
    bytes(b, &mut n, b"}")?;
    Ok(n)
}
fn run_search(r: &[u8]) -> u32 {
    unsafe { LAST_HTTP_STATUS = 0 };
    execute("search", list(r, true))
}
fn run_get_manga_list(r: &[u8]) -> u32 {
    unsafe { LAST_HTTP_STATUS = 0 };
    execute("get_manga_list", list(r, false))
}
fn run_get_manga(r: &[u8]) -> u32 {
    unsafe { LAST_HTTP_STATUS = 0 };
    execute("get_manga", detail(r))
}
fn run_get_chapters(r: &[u8]) -> u32 {
    unsafe { LAST_HTTP_STATUS = 0 };
    execute("get_chapters", chapters(r))
}
fn run_get_pages(r: &[u8]) -> u32 {
    unsafe { LAST_HTTP_STATUS = 0 };
    execute("get_pages", pages(r))
}
fn run_get_image_request(r: &[u8]) -> u32 {
    unsafe { LAST_HTTP_STATUS = 0 };
    execute("get_image_request", image_request(r))
}
fn run_get_listings(_: &[u8]) -> u32 {
    let data=r#"{"listings":[{"id":"popular","name":"热门","supportsPaging":true},{"id":"latest","name":"最近更新","supportsPaging":true}]}"#.as_bytes();
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
koma_source_sdk::koma_source_exports!("manhuagui");
