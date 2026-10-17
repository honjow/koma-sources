#![no_std]
// Public contract: Keiyoushi DongmanManhua.kt / KeiSource.kt, Apache-2.0, revision
// 1319949ca9bc456908196e3ba2d104e94182c5d6. No authentication/mirror/JS processing.
use koma_source_sdk::{
    host,
    json_utils::*,
    source::{SourceCapabilities, SourceInfo},
};
const BASE: &[u8] = b"https://www.dongmanmanhua.cn";
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "cn.dongmanmanhua.koma",
    name: "咚漫 (DongmanManhua)",
    version: "0.1.4",
    api_version: "0.2",
    language: "zh-Hans",
    author: "Koma",
    description: "Dongman public source. Upstream SAFE; individual content and usage unknown.",
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
koma_source_sdk::koma_source_buffers! { payload:1024*1024,http_out:2*1024*1024,body:20*65536,http_req:8192,scratch:8192, }
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
    if write_bytes(&mut out, &mut n, b"dongman stage=")
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
                return write_error(op, "internal_error", "payload exceeds native limit");
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
fn fetch_body(base: &[u8], path: &[u8]) -> Result<usize, &'static str> {
    host::log_info(b"dongman stage=fetch_begin");
    let mut url = [0; 8192];
    let mut n = 0;
    bytes(&mut url, &mut n, base)?;
    bytes(&mut url, &mut n, path)?;
    let len = koma_source_sdk::build_get_request(
        http_req_buf(),
        &url[..n],
        Some(b"https://www.dongmanmanhua.cn/"),
        &[(b"Origin", b"https://www.dongmanmanhua.cn")],
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
    if write_bytes(&mut out, &mut n, b"dongman stage=http_status value=")
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
    if n == 0 || n >= body_buf().len() {
        return Err("empty HTTP body");
    }
    core::str::from_utf8(&body_buf()[..n]).map_err(|_| "invalid UTF8")?;
    unsafe {
        BODY_LEN = n;
    }
    host::log_info(b"dongman stage=body_ready");
    Ok(n)
}

fn validate(r: &[u8]) -> Result<(), &'static str> {
    if r.len() > 65536 || json_value_end(r, 0, 0).map(|x| ws(r, x)) != Some(r.len()) {
        return Err("invalid request");
    }
    if let Some(a) = json_object_member(r, b"args") {
        if a.first() != Some(&b'{') {
            return Err("invalid request");
        }
    }
    Ok(())
}
fn valid_percent(s: &[u8]) -> bool {
    let mut i = 0;
    while i < s.len() {
        if s[i] == b'%' {
            if i + 2 >= s.len() || !s[i + 1].is_ascii_hexdigit() || !s[i + 2].is_ascii_hexdigit() {
                return false;
            }
            i += 3;
        } else {
            i += 1;
        }
    }
    true
}
fn path_hash(s: &[u8]) -> u64 {
    let mut h = 14695981039346656037u64;
    for c in s {
        h = (h ^ (*c as u64)).wrapping_mul(1099511628211);
    }
    h
}
fn valid_path(s: &[u8]) -> bool {
    !s.is_empty()
        && valid_percent(s)
        && s[0] == b'/'
        && !s.starts_with(b"//")
        && s.len() < 4096
        && s.iter()
            .all(|c| *c > 32 && *c < 127 && *c != b'\\' && *c != b'#' && *c != b'"')
        && !s.windows(2).any(|w| w == b"..")
        && !s.windows(3).any(|w| w.eq_ignore_ascii_case(b"%2e"))
}
fn canonical_path(s: &[u8]) -> Result<&[u8], &'static str> {
    let p = if s.starts_with(BASE) {
        &s[BASE.len()..]
    } else if s.starts_with(b"//www.dongmanmanhua.cn/") {
        &s[b"//www.dongmanmanhua.cn".len()..]
    } else {
        s
    };
    if valid_path(p) {
        Ok(p)
    } else {
        Err("invalid request")
    }
}
fn manga(r: &[u8]) -> Result<(&[u8], &[u8]), &'static str> {
    let id = string(args(r), b"mangaId").ok_or("invalid request")?;
    let p = id.strip_prefix(b"manga:").ok_or("invalid request")?;
    if !valid_path(p) {
        return Err("invalid request");
    }
    Ok((id, p))
}
fn fetch(path: &[u8]) -> Result<Node, &'static str> {
    let n = fetch_body(BASE, path)?;
    let b = &body_buf()[..n];
    if !contains_bytes(b, b"</html>") {
        return Err("incomplete HTML");
    }
    Node::parse(b)
}
fn valid_image(s: &[u8]) -> bool {
    s.starts_with(b"https://cdn.dongmanmanhua.cn/")
        && valid_percent(s)
        && s.len() < 4096
        && s.iter()
            .all(|c| *c > 32 && *c < 127 && *c != b'"' && *c != b'\\')
        && !s.windows(2).any(|w| w == b"..")
}
// Card-only template normalization: the same site's detail_body explicitly
// publishes this 306 form. Preserve unknown/signed/quality/larger-image queries.
fn card_cover_url(url: &[u8], out: &mut [u8]) -> Result<usize, &'static str> {
    let mut n = 0;
    if url.starts_with(b"https://cdn.dongmanmanhua.cn/") {
        if let Some(at) = url.iter().position(|c| *c == b'?') {
            let query = &url[at..];
            if !url[..at].contains(&b'#')
                && (query == b"?x-oss-process=image/resize,w_126,limit_0/sharpen,100"
                    || query == b"?x-oss-process=image/resize,w_138,limit_0")
            {
                bytes(out, &mut n, &url[..at])?;
                bytes(out, &mut n, b"?x-oss-process=image/resize,w_306,limit_0")?;
                return Ok(n);
            }
        }
    }
    bytes(out, &mut n, url)?;
    Ok(n)
}
fn paging(o: &mut [u8], n: &mut usize, has: bool, p: usize) -> Result<(), &'static str> {
    bytes(o, n, b"],\"page\":{\"nextCursor\":")?;
    if has {
        q(o, n, format_number(p + 1, &mut [0; 32])?)?
    } else {
        bytes(o, n, b"null")?
    }
    bytes(
        o,
        n,
        if has {
            b",\"hasMore\":true}}"
        } else {
            b",\"hasMore\":false}}"
        },
    )
}
fn format_number<'a>(v: usize, o: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    let mut n = 0;
    if !write_usize(o, &mut n, v) {
        return Err("payload overflow");
    }
    Ok(&o[..n])
}
fn next(root: &Node, out: &mut [u8]) -> Result<Option<usize>, &'static str> {
    let ns = Nodes::get(root, b"div.paginate a[onclick] + a")?;
    if ns.n > 1 {
        return Err("ambiguous pagination");
    }
    if ns.n == 0 {
        return Ok(None);
    }
    let a = selected(&ns, 0);
    let mut b = [0; 4096];
    let href = a.attr(b"href", &mut b)?;
    let p = canonical_path(href)?;
    if p.len() > out.len() {
        return Err("pagination overflow");
    }
    out[..p.len()].copy_from_slice(p);
    Ok(Some(p.len()))
}
fn list(r: &[u8], search: bool) -> Result<usize, &'static str> {
    validate(r)?;
    let pg = page(r)?;
    let mut uri = [0; 8192];
    let mut un = 0;
    if search {
        let query = string(args(r), b"query").ok_or("missing query")?;
        let mut qb = [0; 2048];
        let query = unescape(query, &mut qb)?;
        let query = trim_ascii(query);
        if query.is_empty() {
            return Err("empty query");
        }
        bytes(&mut uri, &mut un, b"/search?keyword=")?;
        if !write_url_encoded(&mut uri, &mut un, query) {
            return Err("payload overflow");
        };
        if pg > 1 {
            bytes(&mut uri, &mut un, b"&page=")?;
            bytes(&mut uri, &mut un, format_number(pg, &mut [0; 32])?)?
        }
    } else {
        let id = string(args(r), b"listingId").unwrap_or(b"dailySchedule");
        if id != b"dailySchedule" {
            return Err("unknown listing");
        }
        if pg != 1 {
            return Err("invalid page cursor");
        }
        bytes(&mut uri, &mut un, b"/dailySchedule")?;
    }
    let root = fetch(&uri[..un])?;
    if search {
        root.select(b"#content > div.card_wrap.search")?;
    }
    let nodes = Nodes::get(
        &root,
        if search {
            b"#content > div.card_wrap.search ul:not(#filterLayer) li a"
        } else {
            b"div#dailyList .daily_section li a, div.daily_lst.comp li a"
        },
    )?;
    let o = payload_buf();
    let mut n = 0;
    bytes(o, &mut n, b"{\"items\":[")?;
    for i in 0..nodes.n {
        let a = selected(&nodes, i);
        let mut hp = [0; 4096];
        let p = a.attr(b"href", &mut hp)?;
        let p = canonical_path(p)?;
        let mut tb = [0; 1024];
        let t = a.select(b"p.subj")?;
        let title = t.text(&mut tb)?;
        if title.is_empty() {
            return Err("empty title");
        };
        let im = a.select(b"img")?;
        let mut ib = [0; 4096];
        let image = im.attr(b"src", &mut ib)?;
        if !valid_image(image) {
            return Err("invalid image URL");
        }
        if i > 0 {
            bytes(o, &mut n, b",")?
        }
        bytes(o, &mut n, b"{\"id\":")?;
        let mut id = [0; 4096];
        let mut k = 0;
        bytes(&mut id, &mut k, b"manga:")?;
        bytes(&mut id, &mut k, p)?;
        q(o, &mut n, &id[..k])?;
        bytes(o, &mut n, b",\"title\":")?;
        q(o, &mut n, title)?;
        bytes(o, &mut n, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
        let mut cover = [0; 4096];
        let cover_len = card_cover_url(image, &mut cover)?;
        q(o, &mut n, &cover[..cover_len])?;
        bytes(
            o,
            &mut n,
            b"},\"contentRating\":\"unknown\",\"language\":\"zh-Hans\"}",
        )?;
    }
    let has = search
        && (root.select(b"div.more_area").is_ok()
            || root.select(b"div.paginate a[onclick] + a").is_ok());
    paging(o, &mut n, has, pg)?;
    Ok(n)
}
fn own_author<'a>(node: &Node, out: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    let len = node.text(out)?.len();
    let children = Nodes::get(node, b"a")?;
    let mut end = len;
    for i in (0..children.n).rev() {
        let ch = selected(&children, i);
        let mut cb = [0; 512];
        let t = ch.text(&mut cb)?;
        let full = trim_ascii(&out[..end]);
        if !full.ends_with(t) {
            return Err("author children ambiguous");
        }
        end = full.len() - t.len();
    }
    Ok(trim_ascii(&out[..end]))
}
fn detail(r: &[u8]) -> Result<usize, &'static str> {
    validate(r)?;
    let (id, path) = manga(r)?;
    let root = fetch(path)?;
    let info = root.select(b".detail_header .info")?;
    let mut tb = [0; 1024];
    let t = root.select(b"h1.subj,h3.subj")?;
    let title = t.text(&mut tb)?;
    if title.is_empty() {
        return Err("empty title");
    };
    let authors = Nodes::get(&info, b".author")?;
    let mut ab = [0; 1024];
    let author = if authors.n > 0 {
        own_author(&selected(&authors, 0), &mut ab)?
    } else {
        let a = info.select(b".author_area")?;
        own_author(&a, &mut ab)?
    };
    let mut ar = [0; 1024];
    let artist = if authors.n > 1 {
        own_author(&selected(&authors, 1), &mut ar)?
    } else {
        author
    };
    let aside = root.select(b"#_asideDetail")?;
    let mut db = [0; 8192];
    let d = aside.select(b"p.summary")?;
    let desc = d.text(&mut db)?;
    let mut sb = [0; 1024];
    let st = aside.select(b"p.day_info")?;
    let status = st.text(&mut sb)?;
    let status = if contains_bytes(status, "更新".as_bytes()) {
        b"ongoing".as_slice()
    } else if contains_bytes(status, "完结".as_bytes()) {
        b"completed".as_slice()
    } else {
        b"unknown".as_slice()
    };
    let mut cb = [0; 4096];
    let body = root.select(b"#content > div.cont_box > div.detail_body")?;
    let style = body.attr(b"style", &mut cb)?;
    let start = find_subslice(style, b"url(").ok_or("missing cover")? + 4;
    let end = find_subslice(&style[start..], b")").ok_or("invalid cover")? + start;
    let mut cover = &style[start..end];
    if cover.len() > 1
        && ((cover[0] == b'\'' && cover[cover.len() - 1] == b'\'')
            || (cover[0] == b'"' && cover[cover.len() - 1] == b'"'))
    {
        cover = &cover[1..cover.len() - 1]
    };
    if !valid_image(cover) {
        return Err("invalid image URL");
    }
    let o = payload_buf();
    let mut n = 0;
    bytes(o, &mut n, b"{\"manga\":{\"id\":")?;
    q(o, &mut n, id)?;
    bytes(o, &mut n, b",\"title\":")?;
    q(o, &mut n, title)?;
    bytes(o, &mut n, b",\"description\":")?;
    q(o, &mut n, desc)?;
    bytes(o, &mut n, b",\"authors\":[")?;
    q(o, &mut n, author)?;
    bytes(o, &mut n, b"],\"artists\":[")?;
    q(o, &mut n, artist)?;
    bytes(o, &mut n, b"],\"status\":")?;
    q(o, &mut n, status)?;
    bytes(o, &mut n, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(o, &mut n, cover)?;
    bytes(
        o,
        &mut n,
        b"},\"contentRating\":\"unknown\",\"language\":\"zh-Hans\"}}",
    )?;
    Ok(n)
}
fn explicit(t: &[u8]) -> Option<usize> {
    let t = t.strip_prefix("第".as_bytes())?;
    let end = t
        .iter()
        .position(|c| !c.is_ascii_digit())
        .unwrap_or(t.len());
    if end == 0 || !t[end..].starts_with("话".as_bytes()) {
        return None;
    }
    number(&t[..end])
}
fn chapters(r: &[u8]) -> Result<usize, &'static str> {
    validate(r)?;
    let (mid, path) = manga(r)?;
    let mut current = [0; 4096];
    current[..path.len()].copy_from_slice(path);
    let mut cur = path.len();
    let o = payload_buf();
    let mut n = 0;
    bytes(o, &mut n, b"{\"chapters\":[")?;
    let mut total = 0;
    let mut pages = 0;
    let mut seen = [0u64; 128];
    loop {
        let h = path_hash(&current[..cur]);
        if seen[..pages].contains(&h) {
            return Err("pagination cycle");
        }
        if pages >= seen.len() {
            return Err("catalogue page limit");
        }
        seen[pages] = h;
        pages += 1;
        if pages > 128 {
            return Err("catalogue page limit");
        };
        let root = fetch(&current[..cur])?;
        let entries = Nodes::get(&root, b"ul#_listUl li")?;
        if entries.n == 0 {
            return Err("empty catalogue");
        };
        for i in 0..entries.n {
            let li = selected(&entries, i);
            let a = li.select(b"a")?;
            let mut hb = [0; 4096];
            let href = a.attr(b"href", &mut hb)?;
            let p = canonical_path(href)?;
            let titleNode = li.select(b"span.subj span")?;
            let mut tb = [0; 1024];
            let title = titleNode.text(&mut tb)?;
            if title.is_empty() {
                return Err("empty title");
            };
            let mut cid = [0; 8192];
            let mut c = 0;
            bytes(&mut cid, &mut c, b"chapter:")?;
            bytes(&mut cid, &mut c, mid)?;
            bytes(&mut cid, &mut c, b":")?;
            bytes(&mut cid, &mut c, &p[1..])?;
            if total > 0 {
                bytes(o, &mut n, b",")?
            };
            bytes(o, &mut n, b"{\"id\":")?;
            q(o, &mut n, &cid[..c])?;
            bytes(o, &mut n, b",\"mangaId\":")?;
            q(o, &mut n, mid)?;
            bytes(o, &mut n, b",\"title\":")?;
            q(o, &mut n, title)?;
            bytes(o, &mut n, b",\"index\":")?;
            bytes(o, &mut n, format_number(total, &mut [0; 32])?)?;
            bytes(o, &mut n, b",\"chapterNumber\":")?;
            if let Some(v) = explicit(title) {
                bytes(o, &mut n, format_number(v, &mut [0; 32])?)?
            } else {
                bytes(o, &mut n, b"null")?
            };
            bytes(
                o,
                &mut n,
                b",\"publishedAt\":null,\"language\":\"zh-Hans\"}",
            )?;
            total += 1;
        }
        let mut nb = [0; 4096];
        match next(&root, &mut nb)? {
            None => break,
            Some(k) => {
                if nb[..k] == current[..cur] {
                    return Err("pagination cycle");
                };
                current[..k].copy_from_slice(&nb[..k]);
                cur = k;
            }
        }
    }
    paging(o, &mut n, false, 1)?;
    Ok(n)
}
fn pages(r: &[u8]) -> Result<usize, &'static str> {
    validate(r)?;
    let id = string(args(r), b"chapterId").ok_or("invalid request")?;
    let rest = id
        .strip_prefix(b"chapter:manga:")
        .ok_or("invalid request")?;
    let split = rest
        .iter()
        .position(|c| *c == b':')
        .ok_or("invalid request")?;
    if !valid_path(&rest[..split]) {
        return Err("invalid request");
    };
    let tail = &rest[split + 1..];
    let mut path = [0; 4096];
    let mut k = 0;
    bytes(&mut path, &mut k, b"/")?;
    bytes(&mut path, &mut k, tail)?;
    if !valid_path(&path[..k]) {
        return Err("invalid request");
    };
    let root = fetch(&path[..k])?;
    let ns = Nodes::get(&root, b"div#_imageList > img")?;
    if ns.n == 0 {
        return Err("empty image list");
    };
    let o = payload_buf();
    let mut n = 0;
    bytes(o, &mut n, b"{\"chapterId\":")?;
    q(o, &mut n, id)?;
    bytes(o, &mut n, b",\"pages\":[")?;
    for i in 0..ns.n {
        let im = selected(&ns, i);
        let mut ib = [0; 4096];
        let url = im.attr(b"data-url", &mut ib)?;
        if !valid_image(url) {
            return Err("invalid image URL");
        };
        if i > 0 {
            bytes(o, &mut n, b",")?
        }
        bytes(o, &mut n, b"{\"id\":\"page:")?;
        bytes(o, &mut n, format_number(i, &mut [0; 32])?)?;
        bytes(o, &mut n, b"\",\"index\":")?;
        bytes(o, &mut n, format_number(i, &mut [0; 32])?)?;
        bytes(o, &mut n, b",\"image\":{\"kind\":\"url\",\"url\":")?;
        q(o, &mut n, url)?;
        bytes(o, &mut n, b"}}")?;
    }
    bytes(o, &mut n, b"]}")?;
    Ok(n)
}
fn image_request(r: &[u8]) -> Result<usize, &'static str> {
    validate(r)?;
    let raw = string(args(r), b"url").ok_or("missing image URL")?;
    let mut ub = [0; 4096];
    let url = unescape(raw, &mut ub)?;
    if !valid_image(url) {
        return Err("invalid image URL");
    };
    let o = payload_buf();
    let mut n = 0;
    bytes(o, &mut n, b"{\"request\":{\"method\":\"GET\",\"url\":")?;
    q(o, &mut n, url)?;
    bytes(o, &mut n, b",\"headers\":{\"User-Agent\":")?;
    q(
        o,
        &mut n,
        b"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0",
    )?;
    bytes(o,&mut n,b",\"Referer\":\"https://www.dongmanmanhua.cn/\",\"Origin\":\"https://www.dongmanmanhua.cn\"}}}")?;
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
fn run_get_image_request(r: &[u8]) -> u32 {
    execute("get_image_request", image_request(r))
}
fn run_get_listings(_: &[u8]) -> u32 {
    let p = r#"{"listings":[{"id":"dailySchedule","name":"每日连载"}]}"#.as_bytes();
    payload_buf()[..p.len()].copy_from_slice(p);
    execute("get_listings", Ok(p.len()))
}
fn run_get_home(_: &[u8]) -> u32 {
    write_error("get_home", "unimplemented", "not supported")
}
fn run_get_filters(_: &[u8]) -> u32 {
    write_error("get_filters", "unimplemented", "not supported")
}
fn run_get_settings(_: &[u8]) -> u32 {
    write_error("get_settings", "unimplemented", "not supported")
}
koma_source_sdk::koma_source_exports!("dongmanmanhua");
