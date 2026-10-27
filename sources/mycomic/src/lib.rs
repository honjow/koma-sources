#![no_std]
use koma_source_sdk::{
    host,
    json_utils::*,
    source::{SourceCapabilities, SourceInfo},
};
const BASE: &[u8] = b"https://mycomic.com";
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.mycomic.koma",
    name: "MYCOMIC - 我的漫畫",
    version: "0.1.5",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "",
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
    settings: true,
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
fn positive_id(s: &[u8]) -> bool {
    if s.is_empty() || s[0] == b'0' {
        return false;
    };
    let mut n = 0u64;
    for &c in s {
        if !c.is_ascii_digit() {
            return false;
        };
        n = match n
            .checked_mul(10)
            .and_then(|v| v.checked_add((c - b'0') as u64))
        {
            Some(v) => v,
            None => return false,
        };
    }
    n <= i64::MAX as u64
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
fn slug(s: &[u8]) -> bool {
    !s.is_empty()
        && s.len() <= 256
        && s.iter()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
}
fn manga_id(r: &[u8]) -> Result<&[u8], &'static str> {
    let s = string(args(r), b"mangaId").ok_or("missing mangaId")?;
    if slug(s) {
        Ok(s)
    } else {
        Err("invalid mangaId")
    }
}
fn chapter_id(r: &[u8]) -> Result<&[u8], &'static str> {
    let s = string(args(r), b"chapterId").ok_or("missing chapterId")?;
    if positive_id(s) {
        Ok(s)
    } else {
        Err("invalid chapterId")
    }
}
fn canonical_path(s: &[u8]) -> Option<&[u8]> {
    let s = if s.starts_with(BASE) {
        s.strip_prefix(BASE)?
    } else {
        s
    };
    if !s.starts_with(b"/") || s.starts_with(b"//") || s.contains(&b'\\') {
        return None;
    };
    Some(s)
}
fn slug_from_href(s: &[u8]) -> Option<&[u8]> {
    let p = canonical_path(s)?;
    let p = p
        .strip_prefix(b"/cn/comics/")
        .or_else(|| p.strip_prefix(b"/comics/"))?;
    let p = p.strip_suffix(b"/").unwrap_or(p);
    if slug(p) {
        Some(p)
    } else {
        None
    }
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
    let _ = write_bytes(&mut b, &mut n, b"mycomic fetch http_status=");
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
        "not_found" => b"not_found",
        "rate_limited" => b"rate_limited",
        "permission_denied" => b"permission_denied",
        "authentication_required" => b"authentication_required",
        _ => b"source_error",
    };
    let mut b = [0; 128];
    let mut n = 0;
    let _ = write_bytes(&mut b, &mut n, b"mycomic error op=");
    let _ = write_bytes(&mut b, &mut n, known_op);
    let _ = write_bytes(&mut b, &mut n, b" code=");
    let _ = write_bytes(&mut b, &mut n, known_code);
    host::log_info(&b[..n]);
}
static mut LAST_HTTP_STATUS: u16 = 0;
fn fetch(path: &[u8]) -> Result<Node, &'static str> {
    unsafe { LAST_HTTP_STATUS = 0 };
    host::log_info(b"mycomic fetch stage=enter");
    if host::check_cancel() {
        return Err("cancelled");
    };
    let mut url = [0; 8192];
    let u = absolute(path, &mut url)?;
    let b = http_req_buf();
    let n = koma_source_sdk::build_get_request(b, u, Some(BASE), &[]).ok_or("request overflow")?;
    host::log_info(b"mycomic fetch stage=request_ready");
    let size = host::http_request(&b[..n], http_out()).map_err(|_| "network request failed")?;
    host::log_info(b"mycomic fetch stage=http_return");
    let response = &http_out()[..size];
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
    host::log_info(b"mycomic fetch stage=body_ready");
    let node = Node::parse(&body_buf()[..n])?;
    host::log_info(b"mycomic fetch stage=html_ready");
    Ok(node)
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
        "not found" => "not_found",
        "rate limited" => "rate_limited",
        "public request rejected" => "permission_denied",
        "authentication required" => "authentication_required",
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
fn image_url<'a>(node: &Node, out: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    let mut attr = [0; 8192];
    if let Some(src) = node.optional_attr(b"data-src", &mut attr)? {
        return absolute(src, out);
    };
    absolute(node.attr(b"src", &mut attr)?, out)
}
// Keep Koma's current cn default; fixed reference defaults unprefixed.
// The ABI select requires nonempty values, so original represents its empty prefix.
fn comics_path(r: &[u8]) -> &'static [u8] {
    let lang = json_object_member(r, b"settings").and_then(|v| string(v, b"pref_key_lang"));
    match lang {
        Some(b"original") | Some(b"") | Some(b"zh-hant") => b"/comics",
        _ => b"/cn/comics",
    }
}
fn list(r: &[u8], query: bool) -> Result<usize, &'static str> {
    let p = page(r)?;
    let mut path = [0; 8192];
    let mut len = 0;
    bytes(&mut path, &mut len, comics_path(r))?;
    if query {
        let raw = string(args(r), b"query").ok_or("missing query")?;
        let mut decoded = [0; 4096];
        let term = unescape(raw, &mut decoded)?;
        if trim_ascii(term).is_empty() {
            return Err("empty query");
        };
        bytes(&mut path, &mut len, b"?q=")?;
        if !write_url_encoded(&mut path, &mut len, term) {
            return Err("query overflow");
        };
    } else {
        let listing = string(args(r), b"listingId").unwrap_or(b"popular");
        bytes(
            &mut path,
            &mut len,
            match listing {
                b"popular" => b"?sort=-views",
                b"latest" => b"?sort=-update",
                _ => return Err("unknown listing"),
            },
        )?;
    }
    if p > 1 {
        bytes(&mut path, &mut len, b"&page=")?;
        if !write_usize(&mut path, &mut len, p) {
            return Err("page overflow");
        };
    }
    let root = fetch(&path[..len])?;
    let _grid = root.select(b"div.grid")?;
    let nodes = Nodes::get(&root, b"div.grid > div.group")?;
    let next = root.select(b"nav[role=navigation] a[rel=next]").ok();
    if nodes.n == 0 && next.is_some() {
        return Err("empty page has next cursor");
    };
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"items\":[")?;
    for i in 0..nodes.n {
        let card = selected(&nodes, i);
        let a = card.select(b"a")?;
        let img = card.select(b"img")?;
        let mut h = [0; 8192];
        let href = a.attr(b"href", &mut h)?;
        let id = slug_from_href(href).ok_or("invalid public manga href")?;
        let mut text = [0; 16384];
        let title = img.attr(b"alt", &mut text)?;
        if trim_ascii(title).is_empty() {
            return Err("manga title missing");
        };
        let mut cover = [0; 8192];
        let cover = image_url(&img, &mut cover)?;
        if i > 0 {
            bytes(b, &mut n, b",")?
        };
        bytes(b, &mut n, b"{\"id\":")?;
        q(b, &mut n, id)?;
        bytes(b, &mut n, b",\"title\":")?;
        q(b, &mut n, title)?;
        bytes(b, &mut n, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
        q(b, &mut n, cover)?;
        bytes(b, &mut n, b"},\"contentRating\":\"unknown\"}")?;
    }
    bytes(b, &mut n, b"],\"page\":{\"nextCursor\":")?;
    if let Some(next) = next {
        let mut href = [0; 8192];
        let href = next.attr(b"href", &mut href)?;
        let path = canonical_path(href).ok_or("invalid next page URL")?;
        if !path.strip_prefix(comics_path(r)).is_some_and(|tail|tail.starts_with(b"?")) {
            return Err("unexpected next page path");
        };
        let mut target = None;
        for part in path.split(|c| *c == b'?' || *c == b'&') {
            if let Some(v) = part.strip_prefix(b"page=") {
                target = number(v);
            }
        }
        let target = target.ok_or("next page missing number")?;
        if target != p.checked_add(1).ok_or("page overflow")? {
            return Err("unexpected next page number");
        };
        bytes(b, &mut n, b"\"")?;
        if !write_usize(b, &mut n, target) {
            return Err("payload overflow");
        };
        bytes(b, &mut n, b"\",\"hasMore\":true}}")?;
    } else {
        bytes(b, &mut n, b"null,\"hasMore\":false}}")?;
    }
    Ok(n)
}
fn manga_doc(id: &[u8], r: &[u8]) -> Result<Node, &'static str> {
    let mut p = [0; 512];
    let mut n = 0;
    bytes(&mut p, &mut n, comics_path(r))?;
    bytes(&mut p, &mut n, b"/")?;
    bytes(&mut p, &mut n, id)?;
    fetch(&p[..n])
}
fn detail(r: &[u8]) -> Result<usize, &'static str> {
    let id = manga_id(r)?;
    let root = manga_doc(id, r)?;
    let card = root.select(b"div[data-flux-card]")?;
    let heading = card.select(b"div[data-flux-heading]")?;
    let mut text = [0; 32768];
    let title = heading.text(&mut text)?;
    if title.is_empty() {
        return Err("manga title missing");
    };
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"manga\":{\"id\":")?;
    q(b, &mut n, id)?;
    bytes(b, &mut n, b",\"title\":")?;
    q(b, &mut n, title)?;
    bytes(b, &mut n, b",\"contentRating\":\"unknown\",\"cover\":")?;
    if let Ok(img) = card.select(b"img.object-cover") {
        let mut u = [0; 8192];
        let url = image_url(&img, &mut u)?;
        bytes(b, &mut n, b"{\"kind\":\"url\",\"url\":")?;
        q(b, &mut n, url)?;
        bytes(b, &mut n, b"}")?;
    } else {
        bytes(b, &mut n, b"null")?;
    }
    bytes(b, &mut n, b",\"authors\":[")?;
    if let Ok(author) = card.select(b"div[data-flux-badge] + div > :first-child a") {
        let a = author.text(&mut text)?;
        if !a.is_empty() {
            q(b, &mut n, a)?;
        }
    }
    bytes(b, &mut n, b"],\"tags\":[")?;
    let tags = Nodes::get(
        &card,
        b"div[data-flux-badge] + div > div:nth-child(2) ~ div a",
    )?;
    for i in 0..tags.n {
        if i > 0 {
            bytes(b, &mut n, b",")?;
        }
        q(b, &mut n, selected(&tags, i).text(&mut text)?)?;
    }
    bytes(b, &mut n, b"],\"status\":")?;
    let status = if let Ok(badge) = card.select(b"div[data-flux-badge]") {
        match badge.text(&mut text)? {
            v if v == "连载中".as_bytes() || v == "連載中".as_bytes() => {
                b"ongoing".as_slice()
            }
            v if v == "已完结".as_bytes() || v == "已完結".as_bytes() => {
                b"completed".as_slice()
            }
            _ => b"unknown",
        }
    } else {
        b"unknown"
    };
    q(b, &mut n, status)?;
    bytes(b, &mut n, b",\"summary\":")?;
    if let Ok(summary) = card.select(b"div[data-flux-badge] + div + div div[x-show=show]") {
        q(b, &mut n, summary.text(&mut text)?)?;
    } else if let Ok(meta) = root.select(b"meta[name=description]") {
        q(b, &mut n, meta.attr(b"content", &mut text)?)?;
    } else {
        q(b, &mut n, b"")?;
    }
    bytes(b, &mut n, b"}}")?;
    Ok(n)
}
// Isolate only a chapters property with a strict array of JSON objects. Never evaluate x-data.
fn chapter_array(s: &[u8]) -> Result<&[u8], &'static str> {
    let k = find_subslice(s, b"chapters:").ok_or("chapter literal missing")?;
    let s = trim_ascii(&s[k + 9..]);
    if s.first() != Some(&b'[') {
        return Err("chapter literal is not JSON array");
    };
    let mut depth = 0usize;
    let mut quoted = false;
    let mut escape = false;
    for (i, &c) in s.iter().enumerate() {
        if quoted {
            if escape {
                escape = false
            } else if c == b'\\' {
                escape = true
            } else if c == b'"' {
                quoted = false
            }
        } else {
            match c {
                b'"' => quoted = true,
                b'[' | b'{' => depth = depth.checked_add(1).ok_or("chapter depth overflow")?,
                b']' | b'}' => {
                    depth = depth.checked_sub(1).ok_or("invalid chapter brackets")?;
                    if depth == 0 {
                        let arr = &s[..i + 1];
                        if json_value_end(arr, 0, 0) != Some(arr.len()) {
                            return Err("chapter literal is not strict JSON");
                        };
                        return Ok(arr);
                    }
                }
                _ => {}
            }
        }
    }
    Err("incomplete chapter literal")
}
fn chapters(r: &[u8]) -> Result<usize, &'static str> {
    let id = manga_id(r)?;
    let root = manga_doc(id, r)?;
    let groups = Nodes::get(&root, b"div[data-flux-card] + div div[x-data]")?;
    if groups.n == 0 {
        return Err("complete chapter owner missing");
    };
    let b = payload_buf();
    let mut n = 0;
    let mut count = 0;
    bytes(b, &mut n, b"{\"items\":[")?;
    for i in 0..groups.n {
        let group = selected(&groups, i);
        let raw = group.attr(b"x-data", scratch_a())?;
        let arr = chapter_array(raw)?;
        let mut rest = trim_ascii(&arr[1..arr.len() - 1]);
        let mut first = true;
        let mut note = [0; 16384];
        let note = match group.select(b"div:first-child > div:first-child") {
            Ok(node) => node.text(&mut note)?,
            Err(_) => b"",
        };
        while !rest.is_empty() {
            if !first {
                if rest.first() != Some(&b',') {
                    return Err("invalid chapter JSON separator");
                };
                rest = trim_ascii(&rest[1..]);
                if rest.is_empty() {
                    return Err("trailing chapter comma");
                };
            }
            if rest.first() != Some(&b'{') {
                return Err("non-object chapter entry");
            };
            let end = json_value_end(rest, 0, 0).ok_or("invalid chapter JSON object")?;
            let obj = &rest[..end];
            let cid = json_object_member(obj, b"id")
                .filter(|v| positive_id(v))
                .ok_or("invalid chapter ID")?;
            let title = string(obj, b"title").ok_or("missing chapter title")?;
            let mut text = [0; 16384];
            let title = unescape(title, &mut text)?;
            if title.is_empty() {
                return Err("empty chapter title");
            };
            if count > 0 {
                bytes(b, &mut n, b",")?;
            }
            bytes(b, &mut n, b"{\"id\":\"")?;
            bytes(b, &mut n, cid)?;
            bytes(b, &mut n, b"\",\"mangaId\":")?;
            q(b, &mut n, id)?;
            bytes(b, &mut n, b",\"title\":")?;
            q(b, &mut n, title)?;
            bytes(b, &mut n, b",\"index\":")?;
            if !write_usize(b, &mut n, count) {
                return Err("payload overflow");
            };
            bytes(
                b,
                &mut n,
                b",\"chapterNumber\":null,\"updatedAt\":null,\"scanlator\":",
            )?;
            if note.is_empty() {
                bytes(b, &mut n, b"null")?;
            } else {
                q(b, &mut n, note)?;
            }
            bytes(b, &mut n, b"}")?;
            count += 1;
            rest = trim_ascii(&rest[obj.len()..]);
            first = false;
        }
    }
    bytes(b, &mut n, b"]}")?;
    Ok(n)
}
fn pages(r: &[u8]) -> Result<usize, &'static str> {
    let cid = chapter_id(r)?;
    let mut path = [0; 256];
    let mut len = 0;
    bytes(&mut path, &mut len, b"/chapters/")?;
    bytes(&mut path, &mut len, cid)?;
    let root = fetch(&path[..len])?;
    let nodes = Nodes::get(&root, b"img[x-ref]")?;
    if nodes.n == 0 {
        return Err("no public chapter images");
    };
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"pages\":[")?;
    for i in 0..nodes.n {
        let img = selected(&nodes, i);
        let mut u = [0; 8192];
        let u = image_url(&img, &mut u)?;
        if i > 0 {
            bytes(b, &mut n, b",")?;
        }
        bytes(b, &mut n, b"{\"id\":\"page:")?;
        bytes(b, &mut n, cid)?;
        bytes(b, &mut n, b":")?;
        if !write_usize(b, &mut n, i + 1) {
            return Err("payload overflow");
        };
        bytes(b, &mut n, b"\",\"index\":")?;
        if !write_usize(b, &mut n, i) {
            return Err("payload overflow");
        };
        bytes(b, &mut n, b",\"image\":{\"kind\":\"url\",\"url\":")?;
        q(b, &mut n, u)?;
        bytes(b, &mut n, b"}}")?;
    }
    bytes(b, &mut n, b"]}")?;
    Ok(n)
}
fn run_search(r: &[u8]) -> u32 {
    unsafe {
        LAST_HTTP_STATUS = 0;
    }
    execute("search", list(r, true))
}
fn run_get_manga_list(r: &[u8]) -> u32 {
    unsafe {
        LAST_HTTP_STATUS = 0;
    }
    execute("get_manga_list", list(r, false))
}
fn run_get_manga(r: &[u8]) -> u32 {
    unsafe {
        LAST_HTTP_STATUS = 0;
    }
    execute("get_manga", detail(r))
}
fn run_get_chapters(r: &[u8]) -> u32 {
    unsafe {
        LAST_HTTP_STATUS = 0;
    }
    execute("get_chapters", chapters(r))
}
fn run_get_pages(r: &[u8]) -> u32 {
    unsafe {
        LAST_HTTP_STATUS = 0;
    }
    execute("get_pages", pages(r))
}
fn run_get_listings(_: &[u8]) -> u32 {
    let data=r#"{"listings":[{"id":"popular","name":"最高人气","supportsPaging":true},{"id":"latest","name":"最近更新","supportsPaging":true}]}"#.as_bytes();
    payload_buf()[..data.len()].copy_from_slice(data);
    write_success_payload("get_listings", data.len())
}
fn image_request(r: &[u8]) -> Result<usize, &'static str> {
    let raw = string(args(r), b"url").ok_or("missing image URL")?;
    let mut decoded = [0; 8192];
    let u = unescape(raw, &mut decoded)?;
    if !u.starts_with(b"https://") {
        return Err("invalid image URL");
    };
    let mut abs = [0; 8192];
    let u = absolute(u, &mut abs)?;
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"url\":")?;
    q(b, &mut n, u)?;
    bytes(b, &mut n, b",\"headers\":{}}")?;
    Ok(n)
}
fn run_get_image_request(r: &[u8]) -> u32 {
    unsafe {
        LAST_HTTP_STATUS = 0;
    }
    execute("get_image_request", image_request(r))
}
fn run_get_home(_: &[u8]) -> u32 {
    write_error("get_home", "not_implemented", "not supported")
}
fn run_get_filters(_: &[u8]) -> u32 {
    write_error("get_filters", "not_implemented", "not supported")
}
fn run_get_settings(_: &[u8]) -> u32 {
    let data = r#"{"settings":[{"id":"pref_key_lang","name":"設置首選語言","kind":"select","default":"cn","options":[{"value":"original","label":"繁體中文"},{"value":"cn","label":"简体中文"}]}]}"#.as_bytes();
    payload_buf()[..data.len()].copy_from_slice(data);
    write_success_payload("get_settings", data.len())
}
koma_source_sdk::koma_source_exports!("mycomic");
