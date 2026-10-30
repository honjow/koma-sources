#![no_std]
// Public contract adapted from Keiyoushi Mh160.kt (Apache-2.0), revision
// 10f06eb833af5990bc5470c84c5e93edc159ad4e. No JavaScript execution or credentials.
mod http_failure;
use http_failure::HttpFailure;
use koma_source_sdk::{
    host,
    json_utils::*,
    source::{SourceCapabilities, SourceInfo},
};
const BASE: &[u8] = b"https://www.mh160mh.com";
const MOBILE: &[u8] = b"https://m.mh160mh.com";
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.mh160.koma",
    name: "漫画160",
    version: "0.1.7",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description:
        "Manhua160 public HTML adapter. Upstream rating SAFE; current contents unverified.",
    content_rating: "unknown",
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
    settings: false,
    image_request: true,
    credentials: false,
};
koma_source_sdk::koma_source_buffers! { payload:512*1024,http_out:2*1024*1024,body:1024*1024,http_req:4096,scratch:8192, }
koma_source_sdk::koma_source_helpers!();
static mut DECODED: [u8; 512 * 1024] = [0; 512 * 1024];
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
fn diagnostic(stage: &[u8], code: &[u8]) {
    let mut out = [0; 160];
    let mut n = 0;
    if write_bytes(&mut out, &mut n, b"mh160 stage=")
        && write_bytes(&mut out, &mut n, stage)
        && write_bytes(&mut out, &mut n, b" code=")
        && write_bytes(&mut out, &mut n, code)
    {
        host::log_info(&out[..n]);
    }
}
fn error_code(e: &str) -> &'static str {
    match e {
        "public request rejected" => "permission_denied",
        "not found" => "not_found",
        "rate limited" => "rate_limit",
        "network failure" => "network_error",
        "client failure" => "client_error",
        "server failure" => "server_error",
        "chapter removed" => "not_found",
        "invalid request"
        | "invalid page cursor"
        | "missing query"
        | "empty query"
        | "unknown listing" => "invalid_request",
        "payload overflow" => "internal_error",
        _ => "parse_error",
    }
}
#[derive(Copy, Clone)]
enum OperationError {
    Message(&'static str),
    Http(HttpFailure),
}
impl From<&'static str> for OperationError {
    fn from(message: &'static str) -> Self { Self::Message(message) }
}
fn write_operation_error(op: &str, failure: OperationError) -> u32 {
    let http = match failure {
        OperationError::Message(message) => return write_error(op, error_code(message), message),
        OperationError::Http(http) => http,
    };
    let mut envelope = [0u8; 512];
    let mut n = 0;
    let ok = write_bytes(&mut envelope, &mut n, br#"{"type":"response","version":1,"ok":false,"operation":""#)
        && append_json_escaped(&mut envelope, &mut n, op.as_bytes())
        && write_bytes(&mut envelope, &mut n, br#"","error":{"code":""#)
        && append_json_escaped(&mut envelope, &mut n, http.code().as_bytes())
        && write_bytes(&mut envelope, &mut n, br#"","message":""#)
        && append_json_escaped(&mut envelope, &mut n, http.message().as_bytes())
        && write_bytes(&mut envelope, &mut n, br#"","status":"#)
        && write_usize(&mut envelope, &mut n, http.status as usize)
        && write_bytes(&mut envelope, &mut n, br#"},"hostHints":{"network":false},"warnings":[]}"#);
    if !ok { return write_error(op, "internal_error", "error response overflow"); }
    response_buffer().write(&envelope[..n], false)
}
fn execute<E: Into<OperationError>>(op: &str, result: Result<usize, E>) -> u32 {
    match result {
        Ok(n) => {
            diagnostic(b"operation_ok", op.as_bytes());
            write_success_payload(op, n)
        }
        Err(e) => {
            let failure = e.into();
            let code = match failure {
                OperationError::Message(message) => error_code(message),
                OperationError::Http(http) => http.code(),
            };
            diagnostic(b"operation_error", code.as_bytes());
            write_operation_error(op, failure)
        }
    }
}
fn fetch(base: &[u8], path: &[u8]) -> Result<usize, OperationError> {
    host::log_info(b"mh160 stage=fetch_begin");
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
    let status = json_object_member(response, b"status")
        .or_else(|| json_object_member(response, b"statusCode"))
        .filter(|raw| raw.len() == 3 && raw.iter().all(u8::is_ascii_digit))
        .map(koma_source_sdk::parse_status_code)
        .unwrap_or(0);
    let mut log = [0; 64];
    let mut c = 0;
    if write_bytes(&mut log, &mut c, b"mh160 stage=http_status value=")
        && write_usize(&mut log, &mut c, status as usize)
    {
        host::log_info(&log[..c]);
    }
    match status {
        200..=299 => {}
        400..=599 => return Err(OperationError::Http(HttpFailure::from_host(status,
            json_object_member(response, b"accessSignal")))),
        _ => {
            let code = match extract_json_string(response, b"code") {
                Some(b"transport_unavailable") => b"transport_unavailable".as_slice(),
                Some(b"transport_invalid_response") => b"transport_invalid_response",
                Some(b"invalid_request") => b"invalid_request",
                Some(b"request_too_large") => b"request_too_large",
                Some(b"timeout") => b"timeout",
                Some(b"network_error") => b"network_error",
                _ => b"unknown",
            };
            diagnostic(b"transport", code);
            return Err("network failure".into());
        }
    }
    let len = koma_source_sdk::decode_json_body_into(response, body_buf())
        .map_err(|_| "HTTP body invalid or too large")?;
    host::log_info(b"mh160 stage=body_ready");
    Ok(len)
}
fn fetch_doc(base: &[u8], path: &[u8]) -> Result<Node, OperationError> {
    let n = fetch(base, path)?;
    Node::parse(&body_buf()[..n]).map_err(OperationError::from)
}
fn path<'a>(href: &'a [u8]) -> Result<&'a [u8], &'static str> {
    let p = if href.starts_with(BASE) {
        &href[BASE.len()..]
    } else if href.starts_with(MOBILE) {
        &href[MOBILE.len()..]
    } else {
        href
    };
    if !p.starts_with(b"/kanmanhua/")
        || p.starts_with(b"//")
        || p.len() > 1024
        || p.iter()
            .any(|c| !c.is_ascii_alphanumeric() && !b"/._-".contains(c))
        || p.split(|c| *c == b'/')
            .any(|part| part == b".." || part == b".")
    {
        return Err("invalid public path");
    }
    Ok(p)
}
fn slug(p: &[u8]) -> Result<&[u8], &'static str> {
    let p = path(p)?;
    let p = &p[b"/kanmanhua/".len()..];
    let end = p.iter().position(|c| *c == b'/').unwrap_or(p.len());
    let s = &p[..end];
    if s.is_empty() || !s.iter().all(|c| c.is_ascii_alphanumeric()) {
        return Err("invalid manga slug");
    }
    Ok(s)
}
fn manga_id(r: &[u8]) -> Result<&[u8], &'static str> {
    let id = string(args(r), b"mangaId")
        .ok_or("invalid request")?
        .strip_prefix(b"mh160:")
        .ok_or("invalid request")?;
    if id.is_empty() || !id.iter().all(|c| c.is_ascii_alphanumeric()) {
        return Err("invalid request");
    }
    Ok(id)
}
fn chapter_path(r: &[u8]) -> Result<&[u8], &'static str> {
    path(
        string(args(r), b"chapterId")
            .ok_or("invalid request")?
            .strip_prefix(b"mh160ch:")
            .ok_or("invalid request")?,
    )
}
fn qid(b: &mut [u8], n: &mut usize, prefix: &[u8], id: &[u8]) -> Result<(), &'static str> {
    bytes(b, n, b"\"")?;
    if !append_json_escaped(b, n, prefix) || !append_json_escaped(b, n, id) {
        return Err("payload overflow");
    }
    bytes(b, n, b"\"")
}
fn absolute<'a>(u: &[u8], out: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    let mut n = 0;
    if u.starts_with(b"https://") || u.starts_with(b"http://") {
        bytes(out, &mut n, u)?;
    } else if u.starts_with(b"//") {
        bytes(out, &mut n, b"https:")?;
        bytes(out, &mut n, u)?;
    } else if u.starts_with(b"/") {
        bytes(out, &mut n, BASE)?;
        bytes(out, &mut n, u)?;
    } else {
        return Err("unknown image URL");
    }
    if out[..n].iter().any(|c| *c <= 32 || *c == 127) {
        return Err("invalid image URL");
    }
    Ok(&out[..n])
}
// The provider owns this public work route; consumers must not infer it from
// the opaque mh160: identity. It is the same route used by manga_doc below.
fn manga_url<'a>(id: &[u8], out: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    let mut n = 0;
    bytes(out, &mut n, BASE)?;
    bytes(out, &mut n, b"/kanmanhua/")?;
    bytes(out, &mut n, id)?;
    bytes(out, &mut n, b"/")?;
    Ok(&out[..n])
}
fn manga_doc(id: &[u8]) -> Result<Node, OperationError> {
    let mut out = [0; 1024];
    let mut n = 0;
    bytes(&mut out, &mut n, b"/kanmanhua/")?;
    bytes(&mut out, &mut n, id)?;
    bytes(&mut out, &mut n, b"/")?;
    fetch_doc(BASE, &out[..n])
}
// The reference category is browse-only; an unset selection keeps each listing's original URL.
fn supports_browse_filters(r: &[u8]) -> bool {
    json_object_member(r,b"hostFeatures").map(trim_ascii).map_or(false,|v| v.starts_with(b"[") && v.ends_with(b"]") && v[1..v.len()-1].split(|b|*b==b',').any(|p|trim_ascii(p)==br#""source_filter_scope_header_v1""#))
}
fn list(r: &[u8], search: bool) -> Result<usize, OperationError> {
    let p = page(r)?;
    let mut out = [0; 8192];
    let mut n = 0;
    let category = if !search && supports_browse_filters(r) { json_object_member(args(r), b"filters").and_then(|f|string(f,b"category")).filter(|v| matches!(*v, b"lianzai" | b"wanjie" | b"zaixian_rhmh" | b"zaixian_dlmh" | b"zaixian_gtmh" | b"zaixian_ommh" | b"zaixian_qita" | b"shaonianqu" | b"shaonvqu" | b"qingnian" | b"shaoer" | b"rexue" | b"gedou" | b"kehuan" | b"jingji" | b"gaoxiao" | b"tuili" | b"kongbu" | b"danmei" | b"lianai" | b"shenghuo" | b"zhanzheng" | b"gushi" | b"maoxian" | b"mohuan" | b"xuanhuan" | b"xiaoyuan" | b"xuanyi" | b"mengxi" | b"chuanyue" | b"hougong" | b"dushi" | b"wuxia" | b"lishi" | b"tongren" | b"lizhi" | b"baihe" | b"zhiyu" | b"jijia" | b"chunai" | b"meishi" | b"xuexing" | b"jiangshi" | b"egao" | b"nuexin" | b"dongzuo" | b"jingxian" | b"weimei" | b"zhenhan" | b"fuchou" | b"zhentan" | b"naodong" | b"qihuan" | b"gongdou" | b"baoxiao" | b"yundong" | b"qingchun" | b"lingyi" | b"gufeng" | b"quanmou" | b"jiecao" | b"mingxing" | b"anhei" | b"shehui" | b"langman" | b"xianxia" | b"weiniang")) } else { None };
    if search {
        let raw = string(args(r), b"query").ok_or("missing query")?;
        let mut decoded = [0; 4096];
        let term = trim_ascii(unescape(raw, &mut decoded)?);
        if term.is_empty() {
            return Err("empty query".into());
        }
        bytes(&mut out, &mut n, b"/statics/searchelxt1e1.aspx?key=")?;
        if !write_url_encoded(&mut out, &mut n, term) {
            return Err("payload overflow".into());
        }
        bytes(&mut out, &mut n, b"&page=")?;
        if !write_usize(&mut out, &mut n, p) {
            return Err("payload overflow".into());
        }
    } else {
        let listing = string(args(r), b"listingId").unwrap_or(b"popular");
        if listing != b"popular" && listing != b"latest" { return Err("unknown listing".into()); }
        if let Some(slug) = category {
            bytes(&mut out, &mut n, b"/kanmanhua/")?;
            bytes(&mut out, &mut n, slug)?;
        } else {
            bytes(&mut out, &mut n, if listing == b"latest" { b"/kanmanhua/zaixian_recent.html" } else { b"/kanmanhua/zaixian_hit.html" })?;
        }
        if p != 1 {
            return Err("invalid page cursor".into());
        }
    }
    let root = fetch_doc(if search { BASE } else { MOBILE }, &out[..n])?;
    let nodes = if search {
        let _container = root.select(b"ul.mh-search-list")?;
        Nodes::get(&root, b"ul.mh-search-list > li")?
    } else if category.is_some() {
        let _container = root.select(b"#listbody")?;
        Nodes::get(&root, b"#listbody li")?
    } else {
        Nodes::get(&root, b".itemBox")?
    };
    if !search && nodes.n == 0 {
        return Err("list structure missing".into());
    }
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"items\":[")?;
    for i in 0..nodes.n {
        let item = selected(&nodes, i);
        let a = item.select(if search {
            b".mh-works-title h4 a".as_slice()
        } else if category.is_some() {
            b"a.ImgA"
        } else {
            b".itemTxt a.title"
        })?;
        let mut href = [0; 1024];
        let id = slug(a.attr(b"href", &mut href)?)?;
        let mut title = [0; 4096];
        let title = if category.is_some() { match item.select(b"a.txtA") { Ok(node) => match node.text(&mut title) { Ok(value) if !value.is_empty() => value, _ => id }, Err(_) => id } } else { a.text(&mut title)? };
        if title.is_empty() {
            return Err("empty manga title".into());
        }
        if i > 0 {
            bytes(b, &mut n, b",")?;
        }
        bytes(b, &mut n, b"{\"id\":")?;
        qid(b, &mut n, b"mh160:", id)?;
        bytes(b, &mut n, b",\"title\":")?;
        q(b, &mut n, title)?;
        bytes(b, &mut n, b",\"url\":")?;
        let mut url = [0; 1024];
        q(b, &mut n, manga_url(id, &mut url)?)?;
        bytes(b, &mut n, b",\"contentRating\":\"unknown\",\"cover\":")?;
        if let Ok(img) = item.select(if search {
            b".mh-nlook-w img".as_slice()
        } else if category.is_some() {
            b"a.ImgA img"
        } else {
            b".itemImg img"
        }) {
            let mut src = [0; 8192];
            let mut u = [0; 8192];
            let url = absolute(img.attr(b"src", &mut src)?, &mut u)?;
            bytes(b, &mut n, b"{\"kind\":\"url\",\"url\":")?;
            q(b, &mut n, url)?;
            bytes(b, &mut n, b"}")?;
        } else {
            bytes(b, &mut n, b"null")?;
        }
        bytes(b, &mut n, b"}")?;
    }
    bytes(
        b,
        &mut n,
        b"],\"page\":{\"nextCursor\":null,\"hasMore\":false}}",
    )?;
    Ok(n)
}
fn first(root: &Node, selectors: &[&[u8]]) -> Result<Node, &'static str> {
    for s in selectors {
        if let Ok(n) = root.select(s) {
            return Ok(n);
        }
    }
    Err("required HTML node missing")
}
fn detail(r: &[u8]) -> Result<usize, OperationError> {
    let id = manga_id(r)?;
    let root = manga_doc(id)?;
    let title_node = first(
        &root,
        &[b".mh-date-info-name h4 a", b".txtItme.h1", b"title"],
    )?;
    let mut text = [0; 32768];
    let raw = title_node.text(&mut text)?;
    let title = if let Some(i) = find_subslice(raw, "漫画,".as_bytes()) {
        &raw[..i]
    } else {
        raw
    };
    if title.is_empty() {
        return Err("empty manga title".into());
    }
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"manga\":{\"id\":")?;
    qid(b, &mut n, b"mh160:", id)?;
    bytes(b, &mut n, b",\"title\":")?;
    q(b, &mut n, title)?;
    bytes(b, &mut n, b",\"url\":")?;
    let mut url = [0; 1024];
    q(b, &mut n, manga_url(id, &mut url)?)?;
    bytes(b, &mut n, b",\"contentRating\":\"unknown\",\"cover\":")?;
    if let Ok(img) = first(&root, &[b".mh-date-bgpic img", b"#Cover img"]) {
        let mut src = [0; 8192];
        let mut u = [0; 8192];
        let u = absolute(img.attr(b"src", &mut src)?, &mut u)?;
        bytes(b, &mut n, b"{\"kind\":\"url\",\"url\":")?;
        q(b, &mut n, u)?;
        bytes(b, &mut n, b"}")?;
    } else {
        bytes(b, &mut n, b"null")?;
    }
    bytes(b, &mut n, b",\"authors\":[")?;
    if let Ok(author) = root.select(b".works-info-tc .one em") {
        let a = author.text(&mut text)?;
        if !a.is_empty() {
            q(b, &mut n, a)?;
        }
    } else {
        let nodes = Nodes::get(&root, b".sub_r .txtItme")?;
        for i in 0..nodes.n {
            let node = selected(&nodes, i);
            let a = node.text(&mut text)?;
            if let Some(i) = find_subslice(a, "作者：".as_bytes()) {
                let author = trim_ascii(&a[i + "作者：".len()..]);
                if !author.is_empty() {
                    q(b, &mut n, author)?;
                }
                break;
            }
        }
    }
    bytes(b, &mut n, b"],\"tags\":[],\"status\":")?;
    let nodes = Nodes::get(&root, b"p.works-info-tc span")?;
    let mut status = b"unknown".as_slice();
    for i in 0..nodes.n {
        let node = selected(&nodes, i);
        if find_subslice(node.text(&mut text)?, "状态".as_bytes()).is_some() {
            if let Ok(em) = node.select(b"em") {
                status = match em.text(&mut text)? {
                    v if v == "连载中".as_bytes() => b"ongoing",
                    v if v == "已完结".as_bytes() || v == "完结".as_bytes() => b"completed",
                    _ => b"unknown",
                };
            }
            break;
        }
    }
    q(b, &mut n, status)?;
    bytes(b, &mut n, b",\"summary\":")?;
    if let Ok(summary) = first(&root, &[b"#workint", b".detailContent p"]) {
        q(b, &mut n, summary.text(&mut text)?)?;
    } else {
        q(b, &mut n, b"")?;
    }
    bytes(b, &mut n, b"}}")?;
    Ok(n)
}
fn chapters(r: &[u8]) -> Result<usize, OperationError> {
    let id = manga_id(r)?;
    let root = manga_doc(id)?;
    let nodes = Nodes::get(
        &root,
        b"ul[id^=mh-chapter-list-ol] li a, ul[id^=chapterList_ul] li a",
    )?;
    if nodes.n == 0 {
        return Err("chapter structure missing".into());
    }
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"chapters\":[")?;
    for i in 0..nodes.n {
        let a = selected(&nodes, i);
        let mut href = [0; 1024];
        let p = path(a.attr(b"href", &mut href)?)?;
        if slug(p)? != id {
            return Err("chapter manga mismatch".into());
        }
        let mut text = [0; 4096];
        let name = if let Ok(title) = a.select(b"p") {
            title.text(&mut text)?
        } else {
            a.text(&mut text)?
        };
        if name.is_empty() {
            return Err("empty chapter title".into());
        }
        if i > 0 {
            bytes(b, &mut n, b",")?;
        }
        bytes(b, &mut n, b"{\"id\":")?;
        qid(b, &mut n, b"mh160ch:", p)?;
        bytes(b, &mut n, b",\"mangaId\":")?;
        qid(b, &mut n, b"mh160:", id)?;
        bytes(b, &mut n, b",\"title\":")?;
        q(b, &mut n, name)?;
        bytes(b, &mut n, b",\"index\":")?;
        if !write_usize(b, &mut n, i) {
            return Err("payload overflow".into());
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
fn literal<'a>(html: &'a [u8], key: &[u8]) -> Result<&'a [u8], &'static str> {
    let mut found = None;
    let mut offset = 0;
    while let Some(i) = find_subslice(&html[offset..], key) {
        let i = i + offset;
        offset = i + key.len();
        if i > 0 && (html[i - 1].is_ascii_alphanumeric() || html[i - 1] == b'_') {
            continue;
        }
        let p = ws(html, offset);
        if html.get(p) != Some(&b'=') {
            continue;
        }
        let p = ws(html, p + 1);
        if html.get(p) != Some(&b'"') {
            return Err("literal is not double quoted");
        }
        let start = p + 1;
        let len = html[start..]
            .iter()
            .position(|c| *c == b'"')
            .ok_or("unterminated literal")?;
        let v = &html[start..start + len];
        if v.contains(&b'\\') {
            return Err("escaped literal unsupported");
        }
        if found.is_some() {
            return Err("ambiguous literal");
        }
        found = Some(v);
    }
    found.ok_or("required reader literal missing")
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
const IMAGE_HOSTS: [&[u8]; 5] = [
    b"https://mhpic5er.tgmhfc.uk",
    b"https://mhpic789-5.tgmhfc.uk",
    b"https://mhpic7fr.tgmhfc.uk",
    b"https://mhpicwt.tgmhfc.uk",
    b"https://mhpicwx.tgmhfc.uk",
];
fn page_url<'a>(p: &[u8], pid: usize, out: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    let mut n = 0;
    if p.starts_with(b"http://") || p.starts_with(b"https://") {
        return absolute(p, out);
    }
    if p.starts_with(b"/") {
        bytes(
            out,
            &mut n,
            if pid > 542724 {
                IMAGE_HOSTS[pid % 5]
            } else {
                b"https://mhpic6.tgmhfc.uk"
            },
        )?;
        for (i, segment) in p.split(|c| *c == b'/').enumerate() {
            if i > 0 {
                bytes(out, &mut n, b"/")?;
            }
            if !write_url_encoded(out, &mut n, segment) {
                return Err("payload overflow");
            }
        }
    } else {
        bytes(out, &mut n, BASE)?;
        bytes(out, &mut n, b"/statics/pic/?p=")?;
        if !write_url_encoded(out, &mut n, p) {
            return Err("payload overflow");
        }
    }
    Ok(&out[..n])
}
fn pages(r: &[u8]) -> Result<usize, OperationError> {
    let path = chapter_path(r)?;
    let len = fetch(BASE, path)?;
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), len) };
    let encoded = literal(html, b"qTcms_S_m_murl_e")?;
    let pid = number(literal(html, b"qTcms_S_p_id")?)
        .filter(|v| *v <= i64::MAX as usize)
        .ok_or("invalid chapter number literal")?;
    let decoded = unsafe { &mut *core::ptr::addr_of_mut!(DECODED) };
    let len = base64_decode(encoded, decoded)?;
    let decoded = &decoded[..len];
    core::str::from_utf8(decoded).map_err(|_| "invalid image path UTF8")?;
    if find_subslice(decoded, b"+http://").is_some()
        || find_subslice(decoded, b"+https://").is_some()
    {
        return Err("chapter removed".into());
    }
    let b = payload_buf();
    let mut n = 0;
    let mut count = 0;
    let mut rest = decoded;
    bytes(b, &mut n, b"{\"pages\":[")?;
    loop {
        let (part, next) = if let Some(i) = find_subslice(rest, b"$qingtiandy$") {
            (&rest[..i], Some(&rest[i + b"$qingtiandy$".len()..]))
        } else {
            (rest, None)
        };
        if !trim_ascii(part).is_empty() {
            let mut url = [0; 8192];
            let u = page_url(part, pid, &mut url)?;
            if count > 0 {
                bytes(b, &mut n, b",")?;
            }
            bytes(b, &mut n, b"{\"id\":\"page:mh160:")?;
            if !append_json_escaped(b, &mut n, path) {
                return Err("payload overflow".into());
            }
            bytes(b, &mut n, b":")?;
            if !write_usize(b, &mut n, count + 1) {
                return Err("payload overflow".into());
            }
            bytes(b, &mut n, b"\",\"index\":")?;
            if !write_usize(b, &mut n, count) {
                return Err("payload overflow".into());
            }
            bytes(b, &mut n, b",\"image\":{\"kind\":\"url\",\"url\":")?;
            q(b, &mut n, u)?;
            bytes(b, &mut n, b"}}")?;
            count += 1;
            if count > 4096 {
                return Err("too many pages".into());
            }
        }
        match next {
            Some(v) => rest = v,
            None => break,
        }
    }
    if count == 0 {
        return Err("empty image list".into());
    }
    bytes(b, &mut n, b"]}")?;
    host::log_info(b"mh160 stage=pages_ready");
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
    let data="{\"listings\":[{\"id\":\"popular\",\"name\":\"最高人气\",\"supportsPaging\":false},{\"id\":\"latest\",\"name\":\"最近更新\",\"supportsPaging\":false}]}".as_bytes();
    payload_buf()[..data.len()].copy_from_slice(data);
    write_success_payload("get_listings", data.len())
}
fn run_get_home(_: &[u8]) -> u32 {
    write_error("get_home", "not_implemented", "not supported")
}
fn run_get_filters(r: &[u8]) -> u32 {
    let schema = if supports_browse_filters(r) { r###"{"filters":[{"id":"category","type":"select","scope":"browse","name":"分类","options":[{"value":"all","label":"全部分类"},{"value":"lianzai","label":"连载中"},{"value":"wanjie","label":"完结"},{"value":"zaixian_rhmh","label":"日韩"},{"value":"zaixian_dlmh","label":"内地"},{"value":"zaixian_gtmh","label":"港台"},{"value":"zaixian_ommh","label":"欧美"},{"value":"zaixian_qita","label":"其他"},{"value":"shaonianqu","label":"少年"},{"value":"shaonvqu","label":"少女"},{"value":"qingnian","label":"青年"},{"value":"shaoer","label":"少儿"},{"value":"rexue","label":"热血"},{"value":"gedou","label":"格斗"},{"value":"kehuan","label":"科幻"},{"value":"jingji","label":"竞技"},{"value":"gaoxiao","label":"搞笑"},{"value":"tuili","label":"推理"},{"value":"kongbu","label":"恐怖"},{"value":"danmei","label":"耽美"},{"value":"lianai","label":"恋爱"},{"value":"shenghuo","label":"生活"},{"value":"zhanzheng","label":"战争"},{"value":"gushi","label":"故事"},{"value":"maoxian","label":"冒险"},{"value":"mohuan","label":"魔幻"},{"value":"xuanhuan","label":"玄幻"},{"value":"xiaoyuan","label":"校园"},{"value":"xuanyi","label":"悬疑"},{"value":"mengxi","label":"萌系"},{"value":"chuanyue","label":"穿越"},{"value":"hougong","label":"后宫"},{"value":"dushi","label":"都市"},{"value":"wuxia","label":"武侠"},{"value":"lishi","label":"历史"},{"value":"tongren","label":"同人"},{"value":"lizhi","label":"励志"},{"value":"baihe","label":"百合"},{"value":"zhiyu","label":"治愈"},{"value":"jijia","label":"机甲"},{"value":"chunai","label":"纯爱"},{"value":"meishi","label":"美食"},{"value":"xuexing","label":"血腥"},{"value":"jiangshi","label":"僵尸"},{"value":"egao","label":"恶搞"},{"value":"nuexin","label":"虐心"},{"value":"dongzuo","label":"动作"},{"value":"jingxian","label":"惊险"},{"value":"weimei","label":"唯美"},{"value":"zhenhan","label":"震撼"},{"value":"fuchou","label":"复仇"},{"value":"zhentan","label":"侦探"},{"value":"naodong","label":"脑洞"},{"value":"qihuan","label":"奇幻"},{"value":"gongdou","label":"宫斗"},{"value":"baoxiao","label":"爆笑"},{"value":"yundong","label":"运动"},{"value":"qingchun","label":"青春"},{"value":"lingyi","label":"灵异"},{"value":"gufeng","label":"古风"},{"value":"quanmou","label":"权谋"},{"value":"jiecao","label":"节操"},{"value":"mingxing","label":"明星"},{"value":"anhei","label":"暗黑"},{"value":"shehui","label":"社会"},{"value":"langman","label":"浪漫"},{"value":"xianxia","label":"仙侠"},{"value":"weiniang","label":"伪娘"}]}]}"###.as_bytes() } else { b"{\"filters\":[]}" };
    payload_buf()[..schema.len()].copy_from_slice(schema);
    write_success_payload("get_filters", schema.len())
}
fn run_get_settings(_: &[u8]) -> u32 {
    write_error("get_settings", "not_implemented", "not supported")
}
fn run_get_image_request(req: &[u8]) -> u32 {
    let raw = match string(args(req), b"url") {
        Some(value) => value,
        None => return write_error("get_image_request", "invalid_request", "missing image URL"),
    };
    let mut decoded = [0u8; 8192];
    let uri = match unescape(raw, &mut decoded) {
        Ok(value) => value,
        Err(_) => return write_error("get_image_request", "invalid_request", "invalid image URL"),
    };
    let mut normalized = [0u8; 8192];
    let url = match absolute(uri, &mut normalized) {
        Ok(value) => value,
        Err(_) => return write_error("get_image_request", "invalid_request", "invalid image URL"),
    };
    let out = payload_buf();
    let mut n = 0;
    // KeiSource's inherited imageRequest uses its source headers for images.
    let result = bytes(out, &mut n, br#"{"url":"#)
        .and_then(|_| q(out, &mut n, url))
        .and_then(|_| bytes(out, &mut n, br#", "headers":{"Referer":"https://www.mh160mh.com/","Origin":"https://www.mh160mh.com"}}"#));
    if result.is_err() { return write_error("get_image_request", "internal_error", "payload overflow"); }
    write_success_payload("get_image_request", n)
}
koma_source_sdk::koma_source_exports!("mh160");
