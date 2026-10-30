#![no_std]
use koma_source_sdk::{
    host,
    json_utils::*,
    source::{SourceCapabilities, SourceInfo},
};
const BASE: &[u8] = b"https://www.liumanhua.com";
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.sixmh.koma",
    name: "六漫画",
    version: "0.1.6",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "六漫画公开 HTML 目录与搜索适配。",
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
    let _ = write_bytes(&mut b, &mut n, b"sixmh fetch http_status=");
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
        "not_found" => b"not_found",
        "rate_limited" => b"rate_limited",
        "permission_denied" => b"permission_denied",
        "authentication_required" => b"authentication_required",
        "not_implemented" => b"not_implemented",
        _ => b"source_error",
    };
    let mut b = [0; 128];
    let mut n = 0;
    let _ = write_bytes(&mut b, &mut n, b"sixmh error op=");
    let _ = write_bytes(&mut b, &mut n, known_op);
    let _ = write_bytes(&mut b, &mut n, b" code=");
    let _ = write_bytes(&mut b, &mut n, known_code);
    host::log_info(&b[..n]);
}
static mut LAST_HTTP_STATUS: u16 = 0;
fn fetch_body(path: &[u8]) -> Result<usize, &'static str> {
    fetch_body_with_ua(path, PC_UA)
}
fn fetch_body_with_ua(path: &[u8], ua: &[u8]) -> Result<usize, &'static str> {
    unsafe { LAST_HTTP_STATUS = 0 };
    host::log_info(b"sixmh fetch stage=enter");
    if host::check_cancel() {
        return Err("cancelled");
    };
    let mut url = [0; 8192];
    let u = absolute(path, &mut url)?;
    let b = http_req_buf();
    let n = request(b, u, ua)?;
    host::log_info(b"sixmh fetch stage=request_ready");
    let size = host::http_request(&b[..n], http_out()).map_err(|_| "network request failed")?;
    host::log_info(b"sixmh fetch stage=http_return");
    if size >= http_out().len() {
        return Err("HTTP response may be truncated");
    }
    let response = &http_out()[..size];
    // A formal transport failure has no HTTP response status. Do not classify
    // malformed/unknown HTTP statuses or other host errors as this known case.
    if json_object_member(response, b"status").is_none()
        && json_object_member(response, b"statusCode").is_none()
        && json_object_member(response, b"ok") == Some(b"false".as_slice())
        && json_object_member(response, b"error")
            .and_then(|error| json_object_member(error, b"code"))
            == Some(br#""network_error""#.as_slice())
    {
        return Err("network request failed");
    }
    let status = json_object_member(response, b"status")
        .or_else(|| json_object_member(response, b"statusCode"))
        .map(koma_source_sdk::parse_status_code)
        .unwrap_or(0);
    unsafe { LAST_HTTP_STATUS = status };
    diagnostic_status(status);
    match status {
        200..=299 => {}
        404 => return Err("not found"),
        429 => return Err("rate limited"),
        401 => return Err("authentication required"),
        403 => return Err("public request rejected"),
        400..=499 => return Err("HTTP client error"),
        500..=599 => return Err("server error"),
        _ => return Err("invalid HTTP status"),
    };
    let n = koma_source_sdk::decode_json_body_into(response, body_buf())
        .map_err(|_| "invalid HTTP body")?;
    if n >= body_buf().len() {
        return Err("HTTP body may be truncated");
    }
    host::log_info(b"sixmh fetch stage=body_ready");
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
        "network request failed" => "network_error",
        "cancelled" => "cancelled",
        "not found" => "not_found",
        "rate limited" => "rate_limited",
        "public request rejected" => "permission_denied",
        "authentication required" => "authentication_required",
        "captcha unsupported" => "permission_denied",
        "public comic unavailable" => "not_found",
        _ => "source_error",
    };
    diagnostic_error(op, code);
    if op == "get_chapters" {
        // Only fixed source literals become tokens; never log the raw message.
        let reason = match e {
            "HTML selection failed" => b"selection_failed".as_slice(),
            "too many HTML nodes" => b"node_capacity",
            "chapter structure missing" => b"chapter_structure_missing",
            "required HTML node missing" => b"chapter_child_missing",
            "HTML attribute overflow" => b"href_read_failed",
            "HTML attribute may be truncated" => b"href_capacity",
            "invalid canonical path" => b"chapter_path_invalid",
            "public comic unavailable" => b"public_comic_unavailable",
            "HTML text overflow" => b"title_read_failed",
            "HTML text may be truncated" => b"title_capacity",
            "chapter title missing" => b"chapter_title_missing",
            "payload overflow" => b"payload_capacity",
            _ => b"other",
        };
        let mut b = [0; 96];
        let mut n = 0;
        let _ = write_bytes(&mut b, &mut n, b"sixmh chapters failure=");
        let _ = write_bytes(&mut b, &mut n, reason);
        host::log_info(&b[..n]);
    }
    write_error(op, code, message)
}
fn execute(op: &str, result: Result<usize, &'static str>) -> u32 {
    match result {
        Ok(n) => write_success_payload(op, n),
        Err(e) => error(op, e),
    }
}

const PC_UA: &[u8] =
    b"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:150.0) Gecko/20100101 Firefox/150.0";
const MOBILE_UA: &[u8] = b"Mozilla/5.0 (Linux; Android 10; Mobile) AppleWebKit/537.36 Chrome/120.0.0.0 Mobile Safari/537.36";
fn request(b: &mut [u8], u: &[u8], ua: &[u8]) -> Result<usize, &'static str> {
    let mut n = 0;
    bytes(b, &mut n, b"{\"version\":1,\"method\":\"GET\",\"url\":")?;
    q(b, &mut n, u)?;
    bytes(b, &mut n, b",\"headers\":{\"User-Agent\":")?;
    q(b, &mut n, ua)?;
    bytes(
        b,
        &mut n,
        b"},\"timeoutMs\":15000,\"responseKind\":\"bodyText\"}",
    )?;
    Ok(n)
}
fn path(s: &[u8]) -> Result<&[u8], &'static str> {
    let p = if let Some(v) = s.strip_prefix(BASE) {
        v
    } else {
        s
    };
    if !p.starts_with(b"/")
        || p.starts_with(b"//")
        || p.len() > 4096
        || p.iter()
            .any(|c| c.is_ascii_control() || matches!(c, b' ' | b'\\' | b'"' | b'\'' | b'?' | b'#'))
    {
        return Err("invalid canonical path");
    }
    if p.split(|c| *c == b'/').any(|x| x == b"." || x == b"..") {
        return Err("invalid canonical path");
    }
    let p = p.strip_prefix(b"/index.php").unwrap_or(p);
    if p.is_empty() || p == b"/err/comic" {
        return Err("public comic unavailable");
    }
    Ok(p)
}
fn identifier<'a>(r: &'a [u8], key: &[u8]) -> Result<&'a [u8], &'static str> {
    path(
        string(args(r), key)
            .or_else(|| string(args(r), b"id"))
            .ok_or("missing identifier")?,
    )
}
fn fetch_doc(p: &[u8]) -> Result<Node, &'static str> {
    let n = fetch_body(p)?;
    let root = Node::parse(&body_buf()[..n])?;
    if root.select(b"#code-div").is_ok() {
        return Err("captcha unsupported");
    }
    if root.select(b"form[action*='/err/comic']").is_ok() {
        return Err("public comic unavailable");
    }
    host::log_info(b"sixmh fetch stage=html_ready");
    Ok(root)
}
fn image_url<'a>(node: &Node, out: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    let mut attr = [0; 8192];
    absolute(node.attr(b"src", &mut attr)?, out)
}
fn listing_prefix(r: &[u8]) -> Result<&'static [u8], &'static str> {
    match string(args(r), b"listingId").unwrap_or(b"popular") {
        b"popular" => Ok(b"/category/order/hits/page/"),
        b"latest" => Ok(b"/category/order/addtime/page/"),
        _ => Err("unknown listing"),
    }
}
fn list(r: &[u8], search: bool) -> Result<usize, &'static str> {
    let p = page(r)?;
    if p > 1_000_000 {
        return Err("invalid page cursor");
    }
    let mut route = [0; 8192];
    let mut z = 0;
    let mut decoded = [0; 2048];
    let term = if search {
        let raw = string(args(r), b"query").ok_or("missing query")?;
        let term = trim_ascii(unescape(raw, &mut decoded)?);
        if term.is_empty() {
            return Err("empty query");
        }
        bytes(&mut route, &mut z, b"/search/")?;
        if !write_url_encoded(&mut route, &mut z, term) {
            return Err("query overflow");
        }
        bytes(&mut route, &mut z, b"/")?;
        term
    } else {
        bytes(&mut route, &mut z, listing_prefix(r)?)?;
        b"".as_slice()
    };
    if !write_usize(&mut route, &mut z, p) {
        return Err("page overflow");
    }
    let root = if search {
        let count = fetch_body_with_ua(&route[..z], MOBILE_UA)?;
        let root = Node::parse(&body_buf()[..count])?;
        if root.select(b"#code-div").is_ok() {
            return Err("captcha unsupported");
        }
        root
    } else {
        fetch_doc(&route[..z])?
    };
    let container = root.select(b"div.cy_list_mh")?;
    let cards = Nodes::get(&root, b"div.cy_list_mh ul")?;
    if cards.n == 0 {
        if !search {
            return Err("public listing empty or unknown");
        }
        let mut text = [0; 8192];
        if !container
            .text(&mut text)?
            .windows("没有找到相关漫画".len())
            .any(|w| w == "没有找到相关漫画".as_bytes())
        {
            return Err("unknown empty search response");
        }
    }
    let nav = root.select(b"#Pagination, .NewPages")?;
    let links = Nodes::get(&nav, b"a")?;
    if links.n < 2 {
        return Err("pagination controls missing");
    }
    let mut h = [0; 8192];
    let mut last = [0; 4096];
    let mut next = [0; 4096];
    let v = selected(&links, links.n - 1).attr(b"href", &mut h)?;
    if v.len() > last.len() {
        return Err("pagination overflow");
    }
    let nl = v.len();
    last[..nl].copy_from_slice(v);
    let v = selected(&links, links.n - 2).attr(b"href", &mut h)?;
    if v.len() > next.len() {
        return Err("pagination overflow");
    }
    let nn = v.len();
    next[..nn].copy_from_slice(v);
    // Search's next link may equal the last link when there are exactly two pages.
    // Compare against the currently selected page instead of the last-page link.
    let at_end = if search {
        let mut current = [0; 4096];
        let href = nav.select(b"a.on")?.attr(b"href", &mut current)?;
        href == &next[..nn]
    } else {
        last[..nl] == next[..nn]
    };
    let target = if at_end {
        None
    } else {
        let n = if search {
            let v = next[..nn]
                .strip_prefix(b"/search/")
                .ok_or("unexpected search pagination")?;
            let v = v
                .strip_prefix(term)
                .and_then(|v| v.strip_prefix(b"/"))
                .ok_or("unexpected search query")?;
            number(v).ok_or("invalid pagination number")?
        } else {
            let v = path(&next[..nn])?;
            number(
                v.strip_prefix(listing_prefix(r)?)
                    .ok_or("unexpected pagination path")?,
            )
            .ok_or("invalid pagination number")?
        };
        if n != p.checked_add(1).ok_or("page overflow")? {
            return Err("unexpected next page number");
        }
        Some(n)
    };
    if cards.n == 0 && target.is_some() {
        return Err("empty search has next page");
    }
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"items\":[")?;
    for i in 0..cards.n {
        let card = selected(&cards, i);
        let a = card.select(b"li.title > a")?;
        let mut href = [0; 8192];
        let id = path(a.attr(b"href", &mut href)?)?;
        let mut text = [0; 16384];
        let title = a.text(&mut text)?;
        if title.is_empty() {
            return Err("manga title missing");
        }
        let mut cover = [0; 8192];
        let cover = image_url(&card.select(b"img")?, &mut cover)?;
        if i > 0 {
            bytes(b, &mut n, b",")?
        }
        bytes(b, &mut n, b"{\"id\":")?;
        q(b, &mut n, id)?;
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
        }
        bytes(b, &mut n, b"\",\"hasMore\":true}}")?
    } else {
        bytes(b, &mut n, b"null,\"hasMore\":false}}")?
    }
    Ok(n)
}
fn detail(r: &[u8]) -> Result<usize, &'static str> {
    let id = identifier(r, b"mangaId")?;
    let root = fetch_doc(id)?;
    let card = root.select(b"div.cy_info")?;
    let mut text = [0; 32768];
    let title = card.select(b"div.cy_title")?.text(&mut text)?;
    if title.is_empty() {
        return Err("manga title missing");
    }
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"manga\":{\"id\":")?;
    q(b, &mut n, id)?;
    bytes(b, &mut n, b",\"title\":")?;
    q(b, &mut n, title)?;
    bytes(b, &mut n, b",\"contentRating\":\"unknown\",\"cover\":")?;
    if let Ok(img) = card.select(b"div.cy_info_cover > a > img.pic") {
        let mut u = [0; 8192];
        let u = image_url(&img, &mut u)?;
        bytes(b, &mut n, b"{\"kind\":\"url\",\"url\":")?;
        q(b, &mut n, u)?;
        bytes(b, &mut n, b"}")?
    } else {
        bytes(b, &mut n, b"null")?
    }
    let infos = Nodes::get(&card, b"div.cy_xinxi")?;
    if infos.n < 2 {
        return Err("manga information missing");
    }
    bytes(b, &mut n, b",\"authors\":[")?;
    if let Ok(a) = selected(&infos, 0).select(b"span:first-child > a") {
        let v = a.text(&mut text)?;
        if !v.is_empty() {
            q(b, &mut n, v)?
        }
    }
    bytes(b, &mut n, b"],\"tags\":[")?;
    let tags = Nodes::get(&selected(&infos, 1), b"span:first-child > a")?;
    for i in 0..tags.n {
        if i > 0 {
            bytes(b, &mut n, b",")?
        }
        q(b, &mut n, selected(&tags, i).text(&mut text)?)?
    }
    bytes(b, &mut n, b"],\"status\":")?;
    let status = if let Ok(s) = selected(&infos, 0).select(b"span:nth-child(2)") {
        let v = s.text(&mut text)?;
        if find_subslice(v, "连载".as_bytes()).is_some() {
            b"ongoing".as_slice()
        } else if find_subslice(v, "完结".as_bytes()).is_some() {
            b"completed".as_slice()
        } else {
            b"unknown".as_slice()
        }
    } else {
        b"unknown"
    };
    q(b, &mut n, status)?;
    bytes(b, &mut n, b",\"summary\":")?;
    if let Ok(s) = card.select(b"div.cy_desc #comic-description") {
        q(b, &mut n, s.text(&mut text)?)?
    } else {
        q(b, &mut n, b"")?
    }
    let mut page_url = [0; 8192];
    bytes(b, &mut n, b",\"links\":[{\"kind\":\"source\",\"url\":")?;
    q(b, &mut n, absolute(id, &mut page_url)?)?;
    bytes(b, &mut n, b"}]}}")?;
    Ok(n)
}
fn chapters(r: &[u8]) -> Result<usize, &'static str> {
    let id = identifier(r, b"mangaId")?;
    let root = fetch_doc(id)?;
    let nodes = Nodes::get(&root, b"ul#mh-chapter-list-ol-0 li.chapter__item")?;
    if nodes.n == 0 {
        if let Ok(card) = root.select(b"div.cy_info") {
            if let Ok(infos) = Nodes::get(&card, b"div.cy_xinxi") {
                if infos.n > 0 {
                    if let Ok(status) = selected(&infos, 0).select(b"span:nth-child(2)") {
                        let mut text = [0; 32768];
                        if let Ok(value) = status.text(&mut text) {
                            if find_subslice(value, "已下架".as_bytes()).is_some() {
                                return Err("public comic unavailable");
                            }
                        }
                    }
                }
            }
        }
        return Err("chapter structure missing");
    }
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"chapters\":[")?;
    for i in 0..nodes.n {
        let a = selected(&nodes, i).select(b"li.chapter__item > :first-child")?;
        let mut h = [0; 8192];
        let cid = path(a.attr(b"href", &mut h)?)?;
        let mut text = [0; 16384];
        let title = a.text(&mut text)?;
        if title.is_empty() {
            return Err("chapter title missing");
        }
        if i > 0 {
            bytes(b, &mut n, b",")?
        }
        bytes(b, &mut n, b"{\"id\":")?;
        q(b, &mut n, cid)?;
        bytes(b, &mut n, b",\"mangaId\":")?;
        q(b, &mut n, id)?;
        bytes(b, &mut n, b",\"title\":")?;
        q(b, &mut n, title)?;
        bytes(b, &mut n, b",\"index\":")?;
        if !write_usize(b, &mut n, i) {
            return Err("payload overflow");
        }
        bytes(
            b,
            &mut n,
            b",\"chapterNumber\":null,\"updatedAt\":null,\"scanlator\":null}",
        )?;
    }
    bytes(b, &mut n, b"]}")?;
    Ok(n)
}
fn literal(html: &[u8]) -> Result<&[u8], &'static str> {
    let key = b"params";
    let mut offset = 0;
    let mut found = None;
    while let Some(i) = find_subslice(&html[offset..], key) {
        let i = i + offset;
        offset = i + key.len();
        if i > 0 && (html[i - 1].is_ascii_alphanumeric() || html[i - 1] == b'_') {
            continue;
        }
        let k = ws(html, offset);
        if html.get(k) != Some(&b'=') {
            continue;
        }
        let k = ws(html, k + 1);
        if html.get(k) != Some(&b'\'') {
            return Err("reader params must be quoted literal");
        }
        let start = k + 1;
        let len = html[start..]
            .iter()
            .position(|c| *c == b'\'')
            .ok_or("unterminated reader params")?;
        if found.is_some() {
            return Err("ambiguous reader params");
        }
        let v = &html[start..start + len];
        if v.len() > 1024 * 1024 {
            return Err("reader params overflow");
        }
        found = Some(v);
    }
    found.ok_or("reader params missing")
}

fn b64value(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}
fn base64_decode(input: &[u8], out: &mut [u8]) -> Result<usize, &'static str> {
    if input.is_empty() || input.len() % 4 != 0 {
        return Err("invalid base64 length");
    }
    let mut n = 0;
    for (i, chunk) in input.chunks_exact(4).enumerate() {
        let a = b64value(chunk[0]).ok_or("invalid base64")?;
        let b = b64value(chunk[1]).ok_or("invalid base64")?;
        let final_group = i + 1 == input.len() / 4;
        let pad = if chunk[2] == b'=' {
            if chunk[3] != b'=' || !final_group || b & 15 != 0 {
                return Err("invalid base64 padding");
            }
            2
        } else if chunk[3] == b'=' {
            if !final_group {
                return Err("invalid base64 padding");
            }
            1
        } else {
            0
        };
        let c = if pad == 2 {
            0
        } else {
            b64value(chunk[2]).ok_or("invalid base64")?
        };
        let d = if pad > 0 {
            0
        } else {
            b64value(chunk[3]).ok_or("invalid base64")?
        };
        if pad == 1 && c & 3 != 0 {
            return Err("invalid base64 padding");
        }
        let count = 3 - pad;
        if n + count > out.len() {
            return Err("base64 overflow");
        }
        out[n] = (a << 2) | (b >> 4);
        if count > 1 {
            out[n + 1] = (b << 4) | (c >> 2);
        }
        if count > 2 {
            out[n + 2] = (c << 6) | d;
        }
        n += count;
    }
    Ok(n)
}
mod crypto;
fn unique_images(json: &[u8]) -> Result<&[u8], &'static str> {
    let mut pos = ws(json, ws(json, 0) + 1);
    let mut found = None;
    while json.get(pos) != Some(&b'}') {
        let key_end = json_string_end(json, pos).ok_or("invalid reader key")?;
        let mut key_buf = [0; 256];
        let key = unescape(&json[pos + 1..key_end - 1], &mut key_buf)?;
        pos = ws(json, key_end);
        if json.get(pos) != Some(&b':') {
            return Err("invalid reader separator");
        }
        pos = ws(json, pos + 1);
        let end = json_value_end(json, pos, 0).ok_or("invalid reader value")?;
        if key == b"images" {
            if found.is_some() {
                return Err("ambiguous reader images");
            }
            found = Some(&json[pos..end])
        }
        pos = ws(json, end);
        if json.get(pos) == Some(&b',') {
            pos = ws(json, pos + 1)
        } else if json.get(pos) != Some(&b'}') {
            return Err("invalid reader object");
        }
    }
    found.ok_or("reader images missing")
}

fn pages(r: &[u8]) -> Result<usize, &'static str> {
    let cid = identifier(r, b"chapterId")?;
    let n = fetch_body(cid)?;
    let raw = literal(&body_buf()[..n])?;
    let tmp = scratch_b();
    let count = base64_decode(raw, tmp)?;
    let decoded = crypto::decode_metadata(&mut tmp[..count])?;
    let json = core::str::from_utf8(decoded)
        .map_err(|_| "invalid reader UTF8")?
        .as_bytes();
    if json_value_end(json, ws(json, 0), 0)
        .filter(|end| ws(json, *end) == json.len())
        .is_none()
    {
        return Err("invalid complete reader JSON");
    }
    if trim_ascii(json).first() != Some(&b'{') {
        return Err("reader data must be object");
    }
    let images = unique_images(json)?;
    let images = trim_ascii(images);
    if images.first() != Some(&b'[') || images.last() != Some(&b']') {
        return Err("reader images must be array");
    }
    let b = payload_buf();
    let mut n = 0;
    let mut pos = ws(images, 1);
    let mut index = 0;
    bytes(b, &mut n, b"{\"pages\":[")?;
    while images.get(pos) != Some(&b']') {
        if index >= 2048 {
            return Err("too many chapter images");
        }
        let end = json_string_end(images, pos).ok_or("image URL must be string")?;
        let mut decoded_url = [0; 8192];
        let url = unescape(&images[pos + 1..end - 1], &mut decoded_url)?;
        let mut u = [0; 8192];
        let u = absolute(url, &mut u)?;
        if index > 0 {
            bytes(b, &mut n, b",")?
        }
        bytes(b, &mut n, b"{\"id\":\"")?;
        if !append_json_escaped(b, &mut n, cid) {
            return Err("payload overflow");
        }
        bytes(b, &mut n, b":page:")?;
        if !write_usize(b, &mut n, index + 1) {
            return Err("payload overflow");
        }
        bytes(b, &mut n, b"\",\"index\":")?;
        if !write_usize(b, &mut n, index) {
            return Err("payload overflow");
        }
        bytes(b, &mut n, b",\"image\":{\"kind\":\"url\",\"url\":")?;
        q(b, &mut n, u)?;
        bytes(b, &mut n, b"}}")?;
        index += 1;
        pos = ws(images, end);
        if images.get(pos) == Some(&b',') {
            pos = ws(images, pos + 1);
            if images.get(pos) == Some(&b']') {
                return Err("trailing image comma");
            }
        } else if images.get(pos) != Some(&b']') {
            return Err("invalid image array separator");
        }
    }
    if index == 0 {
        return Err("no public chapter images");
    }
    if ws(images, pos + 1) != images.len() {
        return Err("invalid complete image array");
    }
    bytes(b, &mut n, b"]}")?;
    host::log_info(b"sixmh pages stage=payload_ready");
    Ok(n)
}
fn image_request(r: &[u8]) -> Result<usize, &'static str> {
    let raw = string(args(r), b"url").ok_or("missing image URL")?;
    let mut decoded = [0; 8192];
    let u = unescape(raw, &mut decoded)?;
    if !u.starts_with(b"https://") {
        return Err("invalid image URL");
    }
    let mut abs = [0; 8192];
    let u = absolute(u, &mut abs)?;
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"url\":")?;
    q(b, &mut n, u)?;
    bytes(b, &mut n, b",\"headers\":{\"User-Agent\":")?;
    q(b, &mut n, PC_UA)?;
    bytes(b, &mut n, b"}}")?;
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
    let data=r#"{"listings":[{"id":"popular","name":"热门人气","supportsPaging":true},{"id":"latest","name":"更新时间","supportsPaging":true}]}"#.as_bytes();
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
koma_source_sdk::koma_source_exports!("sixmh");
