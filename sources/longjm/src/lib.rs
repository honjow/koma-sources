#![no_std]

use koma_source_sdk::{
    host,
    json_utils::*,
    source::{SourceCapabilities, SourceInfo},
};

#[link(wasm_import_module = "koma_host")]
extern "C" {
    #[link_name = "html_select_all"]
    fn select_all(desc: i32, selector: *const u8, len: u32, out: *mut i32, cap: u32) -> i32;
}

const DEFAULT_BASE: &[u8] = b"https://longjm.com";
const MIRRORS: &[&[u8]] = &[
    b"https://longjm.com",
    b"https://long52.com",
    b"https://long92.org",
    b"https://long91.xyz",
    b"https://long91.org",
    b"https://long91.com",
    b"https://long7.org",
    b"https://long5.org",
    b"https://longjm.org",
    b"https://52longjm.com",
    b"https://long9.xyz",
    b"https://long6.xyz",
    b"https://long82.com",
    b"https://long81.com",
    b"https://long7.xyz",
];

const SETTINGS_JSON: &str = r#"{"settings":[{"id":"preferredDomain","name":"备用域名","kind":"select","default":"https://longjm.com","description":"站点提供的备用域名；默认使用 longjm.com，无法访问时可切换。","options":[{"value":"https://longjm.com","label":"longjm.com"},{"value":"https://long52.com","label":"long52.com"},{"value":"https://long92.org","label":"long92.org"},{"value":"https://long91.xyz","label":"long91.xyz"},{"value":"https://long91.org","label":"long91.org"},{"value":"https://long91.com","label":"long91.com"},{"value":"https://long7.org","label":"long7.org"},{"value":"https://long5.org","label":"long5.org"},{"value":"https://longjm.org","label":"longjm.org"},{"value":"https://52longjm.com","label":"52longjm.com"},{"value":"https://long9.xyz","label":"long9.xyz"},{"value":"https://long6.xyz","label":"long6.xyz"},{"value":"https://long82.com","label":"long82.com"},{"value":"https://long81.com","label":"long81.com"},{"value":"https://long7.xyz","label":"long7.xyz"}]},{"id":"popularRank","name":"热门使用的排行榜","kind":"select","default":"weekly","description":"只影响热门列表；总榜始终有独立栏目。","options":[{"value":"weekly","label":"周榜"},{"value":"daily","label":"日榜"},{"value":"monthly","label":"月榜"}]}]}"#;

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.longjm.koma",
    name: "龙禁漫",
    version: "0.1.2",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "龙禁漫公开 HTML 源；成人内容，单作品分级与用量未知。",
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

koma_source_sdk::koma_source_buffers! {
    payload: 2 * 1024 * 1024,
    http_out: 4 * 1024 * 1024,
    body: 4 * 1024 * 1024,
    http_req: 16 * 1024,
    scratch: 512 * 1024,
}

koma_source_sdk::koma_source_helpers!();

static mut LAST_HTTP_STATUS: u16 = 0;

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
        host::html_select(self.0, s).map(Self).map_err(|_| "required HTML node missing")
    }
    fn attr<'a>(&self, k: &[u8], out: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
        let n = host::html_attr(self.0, k, out).map_err(|_| "HTML attribute overflow")?;
        if n >= out.len() {
            return Err("HTML attribute may be truncated");
        };
        Ok(&out[..n])
    }
    fn optional_attr<'a>(&self, k: &[u8], out: &'a mut [u8]) -> Result<Option<&'a [u8]>, &'static str> {
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
        let mut v = Self { ids: [0; 2048], n: 0 };
        let n = unsafe { select_all(root.0.raw(), s.as_ptr(), s.len() as u32, v.ids.as_mut_ptr(), (v.ids.len() * 4) as u32) };
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

fn selected(nodes: &Nodes, i: usize) -> core::mem::ManuallyDrop<Node> {
    core::mem::ManuallyDrop::new(nodes.at(i))
}

fn bytes(b: &mut [u8], n: &mut usize, s: &[u8]) -> Result<(), &'static str> {
    if write_bytes(b, n, s) { Ok(()) } else { Err("payload overflow") }
}

fn q(b: &mut [u8], n: &mut usize, s: &[u8]) -> Result<(), &'static str> {
    bytes(b, n, b"\"")?;
    if !append_json_escaped(b, n, s) {
        return Err("payload overflow");
    };
    bytes(b, n, b"\"")
}

fn args(r: &[u8]) -> &[u8] {
    json_object_member(r, b"args").unwrap_or(r)
}

fn number(s: &[u8]) -> Option<usize> {
    if s.is_empty() { return None; };
    let mut n = 0usize;
    for &c in s {
        if !c.is_ascii_digit() { return None; };
        n = n.checked_mul(10)?.checked_add((c - b'0') as usize)?;
    }
    Some(n)
}

fn positive_id(s: &[u8]) -> bool {
    !s.is_empty() && s[0] != b'0' && s.iter().all(|c| c.is_ascii_digit())
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && haystack.len() >= needle.len() && haystack.windows(needle.len()).any(|w| w == needle)
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() { return None; };
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn hex4(s: &[u8]) -> Option<u32> {
    if s.len() != 4 { return None; };
    let mut n = 0u32;
    for &c in s {
        n = n * 16 + match c {
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
            if c < 32 || c == b'"' { return Err("invalid JSON string"); };
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
                let mut cp = hex4(s.get(i..i + 4).ok_or("invalid unicode escape")?).ok_or("invalid unicode escape")?;
                i += 4;
                if (0xd800..=0xdbff).contains(&cp) {
                    if s.get(i..i + 2) != Some(b"\\u") { return Err("invalid unicode pair"); };
                    i += 2;
                    let low = hex4(s.get(i..i + 4).ok_or("invalid unicode pair")?).ok_or("invalid unicode pair")?;
                    i += 4;
                    if !(0xdc00..=0xdfff).contains(&low) { return Err("invalid unicode pair"); };
                    cp = 0x10000 + ((cp - 0xd800) << 10) + (low - 0xdc00);
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

fn decode_entities<'a>(s: &[u8], out: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    let mut i = 0;
    let mut n = 0;
    while i < s.len() {
        if s[i] == b'&' {
            if let Some(rel) = s[i..].iter().position(|b| *b == b';') {
                let end = i + rel;
                let name = &s[i + 1..end];
                let named: Option<&[u8]> = if name == b"amp" { Some(b"&") }
                    else if name == b"lt" { Some(b"<") }
                    else if name == b"gt" { Some(b">") }
                    else if name == b"quot" { Some(b"\"") }
                    else if name == b"apos" { Some(b"'") }
                    else if name == b"nbsp" { Some(b" ") }
                    else if name == b"hellip" { Some("\u{2026}".as_bytes()) }
                    else if name == b"hearts" { Some("\u{2665}".as_bytes()) }
                    else if name == b"mdash" { Some("\u{2014}".as_bytes()) }
                    else if name == b"ndash" { Some("\u{2013}".as_bytes()) }
                    else if name == b"ldquo" { Some("\u{201c}".as_bytes()) }
                    else if name == b"rdquo" { Some("\u{201d}".as_bytes()) }
                    else if name == b"lsquo" { Some("\u{2018}".as_bytes()) }
                    else if name == b"rsquo" { Some("\u{2019}".as_bytes()) }
                    else { None };
                if let Some(v) = named {
                    bytes(out, &mut n, v)?;
                    i = end + 1;
                    continue;
                }
                if let Some(rest) = name.strip_prefix(b"#") {
                    let cp = if let Some(hex) = rest.strip_prefix(b"x").or_else(|| rest.strip_prefix(b"X")) {
                        if hex.is_empty() { None } else { let mut v = 0u32; let mut ok = true; for &c in hex { match c { b'0'..=b'9' => v = v * 16 + (c - b'0') as u32, b'a'..=b'f' => v = v * 16 + (c - b'a' + 10) as u32, b'A'..=b'F' => v = v * 16 + (c - b'A' + 10) as u32, _ => { ok = false; break; } } } if ok { Some(v) } else { None } }
                    } else { number(rest).map(|v| v as u32) };
                    if let Some(cp) = cp.and_then(char::from_u32) {
                        let mut tmp = [0; 4];
                        bytes(out, &mut n, cp.encode_utf8(&mut tmp).as_bytes())?;
                        i = end + 1;
                        continue;
                    }
                }
            }
        }
        bytes(out, &mut n, &[s[i]])?;
        i += 1;
    }
    core::str::from_utf8(&out[..n]).map_err(|_| "invalid UTF8")?;
    Ok(&out[..n])
}

fn site_path(path: &[u8]) -> &[u8] {
    let mut p = path;
    if let Some(rest) = p.strip_prefix(b"https://") { p = slash_or_root(rest); }
    else if let Some(rest) = p.strip_prefix(b"http://") { p = slash_or_root(rest); }
    else if let Some(rest) = p.strip_prefix(b"//") { p = slash_or_root(rest); }
    p
}

fn slash_or_root(rest: &[u8]) -> &[u8] {
    match rest.iter().position(|b| *b == b'/') {
        Some(i) => &rest[i..],
        None => b"/",
    }
}

fn absolute<'a>(base: &[u8], path: &[u8], out: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    let mut n = 0;
    let ok = if path.starts_with(b"http://") || path.starts_with(b"https://") {
        write_bytes(out, &mut n, path)
    } else if path.starts_with(b"//") {
        write_bytes(out, &mut n, b"https:") && write_bytes(out, &mut n, path)
    } else {
        write_bytes(out, &mut n, base) && write_bytes(out, &mut n, path)
    };
    if !ok { return Err("url overflow"); };
    Ok(&out[..n])
}

fn mirror_origin(req: &[u8]) -> &'static [u8] {
    let selected = json_object_member(req, b"settings").and_then(|s| extract_json_string(s, b"preferredDomain"));
    match selected {
        Some(v) => MIRRORS.iter().copied().find(|m| *m == v).unwrap_or(DEFAULT_BASE),
        None => DEFAULT_BASE,
    }
}

fn popular_path(req: &[u8]) -> &'static [u8] {
    let selected = json_object_member(req, b"settings").and_then(|s| extract_json_string(s, b"popularRank"));
    match selected {
        Some(v) if v == b"daily" => b"/bookrank/daily",
        Some(v) if v == b"monthly" => b"/bookrank/monthly",
        _ => b"/bookrank/weekly",
    }
}

fn fetch(req: &[u8], path: &[u8]) -> Result<Node, &'static str> {
    unsafe { LAST_HTTP_STATUS = 0 };
    if host::check_cancel() { return Err("cancelled"); }
    let base = mirror_origin(req);
    let mut url = [0; 8192];
    let u = absolute(base, path, &mut url)?;
    let n = koma_source_sdk::build_get_request(http_req_buf(), u, Some(base), &[]).ok_or("request overflow")?;
    let size = host::http_request(&http_req_buf()[..n], http_out()).map_err(|_| "network request failed")?;
    let response = &http_out()[..size];
    let status = json_object_member(response, b"status").or_else(|| json_object_member(response, b"statusCode")).map(koma_source_sdk::parse_status_code).unwrap_or(0);
    unsafe { LAST_HTTP_STATUS = status };
    match status {
        200..=299 => {}
        404 => return Err("not found"),
        429 => return Err("rate limited"),
        401 => return Err("authentication required"),
        403 => return Err("public request rejected"),
        400..=499 => return Err("HTTP client error"),
        500..=599 => return Err("server error"),
        _ => return Err("invalid HTTP status"),
    }
    let n = koma_source_sdk::decode_json_body_into(response, body_buf()).map_err(|_| "invalid HTTP body")?;
    Node::parse(&body_buf()[..n])
}

fn error(op: &str, e: &str) -> u32 {
    let code = match e {
        "cancelled" => "cancelled",
        "not found" => "not_found",
        "rate limited" => "rate_limited",
        "public request rejected" => "permission_denied",
        "authentication required" => "authentication_required",
        _ => "source_error",
    };
    let mut msg = [0; 192];
    let mut n = 0;
    let _ = write_bytes(&mut msg, &mut n, e.as_bytes());
    let status = unsafe { LAST_HTTP_STATUS };
    if status > 0 {
        let _ = write_bytes(&mut msg, &mut n, b" (HTTP ");
        let _ = write_usize(&mut msg, &mut n, status as usize);
        let _ = write_bytes(&mut msg, &mut n, b")");
    }
    write_error(op, code, core::str::from_utf8(&msg[..n]).unwrap_or(e))
}

fn execute(op: &str, result: Result<usize, &'static str>) -> u32 {
    match result { Ok(n) => write_success_payload(op, n), Err(e) => error(op, e) }
}

fn write_static(op: &str, data: &[u8]) -> u32 {
    let p = payload_buf();
    if data.len() > p.len() { return write_error(op, "internal_error", "overflow"); }
    p[..data.len()].copy_from_slice(data);
    write_success_payload(op, data.len())
}

fn manga_id(req: &[u8]) -> Result<&[u8], &'static str> {
    let raw = extract_json_string(args(req), b"mangaId").ok_or("missing mangaId")?;
    let id = raw.strip_prefix(b"manga:").unwrap_or(raw);
    if positive_id(id) { Ok(id) } else { Err("invalid mangaId") }
}

fn manga_id_from_href(href: &[u8]) -> Option<&[u8]> {
    let path = site_path(href);
    let rest = path.strip_prefix(b"/manhua/")?;
    let id = rest.strip_suffix(b".html")?;
    if positive_id(id) { Some(id) } else { None }
}

fn chapter_parts(href: &[u8]) -> Option<(&[u8], &[u8])> {
    let path = site_path(href);
    let rest = path.strip_prefix(b"/manhua/")?;
    let mut parts = rest.split(|b| *b == b'/');
    let id = parts.next()?;
    let hash = parts.next()?.strip_suffix(b".html")?;
    if parts.next().is_some() || !positive_id(id) || hash.is_empty() || hash.contains(&b'/') || hash.contains(&b':') {
        return None;
    }
    Some((id, hash))
}

fn page_cursor(req: &[u8]) -> usize {
    let page = match json_object_member(req, b"page") { Some(v) => v, None => return 0 };
    let cursor = match json_object_member(page, b"cursor") { Some(v) => trim_ascii(v), None => return 0 };
    let cursor = cursor.strip_prefix(b"\"").and_then(|v| v.strip_suffix(b"\"")).unwrap_or(cursor);
    if cursor.is_empty() || cursor == b"null" { 0 } else { number(cursor).unwrap_or(0) }
}

fn cover_url<'a>(img: &Node, base: &[u8], out: &'a mut [u8]) -> Result<Option<&'a [u8]>, &'static str> {
    let mut a = [0; 8192];
    for key in [b"data-original".as_slice(), b"data-src", b"src"] {
        if let Some(v) = img.optional_attr(key, &mut a)? {
            if v.is_empty() || contains(v, b"/imgs/") { continue; }
            return Ok(Some(absolute(base, v, out)?));
        }
    }
    Ok(None)
}

fn content_image<'a>(img: &Node, base: &[u8], out: &'a mut [u8]) -> Result<Option<&'a [u8]>, &'static str> {
    let mut a = [0; 8192];
    for key in [b"data-src".as_slice(), b"data-original", b"src"] {
        if let Some(v) = img.optional_attr(key, &mut a)? {
            if v.is_empty() || contains(v, b"/imgs/") { continue; }
            let mut cand = [0; 8192];
            let u = absolute(base, v, &mut cand)?;
            if contains(u, b"img.nnpic.xyz") || contains(u, b"/upload") {
                let mut n = 0;
                if !write_bytes(out, &mut n, u) { return Err("url overflow"); }
                return Ok(Some(&out[..n]));
            }
            if contains(u, b"/upload") {
                let mut n = 0;
                if !write_bytes(out, &mut n, u) { return Err("url overflow"); }
                return Ok(Some(&out[..n]));
            }
        }
    }
    Ok(None)
}

fn write_cards(req: &[u8], root: &Node, has_more: bool, next: Option<usize>) -> Result<usize, &'static str> {
    let nodes = Nodes::get(root, b"a.module-poster-item")?;
    if nodes.n == 0 { return Err("empty catalog"); }
    let base = mirror_origin(req);
    let b = payload_buf();
    let mut n = 0;
    let mut count = 0;
    bytes(b, &mut n, b"{\"items\":[")?;
    for i in 0..nodes.n {
        let card = selected(&nodes, i);
        let mut h = [0; 8192];
        let href = match card.optional_attr(b"href", &mut h)? { Some(v) => v, None => continue };
        let id = match manga_id_from_href(href) { Some(v) => v, None => continue };
        let mut tb = [0; 16384];
        let mut ab = [0; 16384];
        let title = match card.optional_attr(b"title", &mut tb)? {
            Some(v) if !v.is_empty() => v,
            _ => match card.select(b"img") { Ok(img) => img.attr(b"alt", &mut ab).unwrap_or(b""), Err(_) => b"" },
        };
        if title.is_empty() { return Err("manga title missing"); }
        let mut td = [0; 16384];
        let title = decode_entities(title, &mut td)?;
        let mut cbuf = [0; 8192];
        let cover = match card.select(b"img") { Ok(img) => cover_url(&img, base, &mut cbuf)?, Err(_) => None };
        let mut nb = [0; 4096];
        let note = match card.select(b"div.module-item-note") { Ok(v) => Some(v.text(&mut nb)?), Err(_) => None };
        if count > 0 { bytes(b, &mut n, b",")?; }
        bytes(b, &mut n, b"{\"id\":\"manga:")?;
        bytes(b, &mut n, id)?;
        bytes(b, &mut n, b"\",\"title\":")?;
        q(b, &mut n, title)?;
        if let Some(note) = note {
            if !note.is_empty() {
                let mut nd = [0; 4096];
                let note = decode_entities(note, &mut nd)?;
                bytes(b, &mut n, b",\"subtitle\":")?;
                q(b, &mut n, note)?;
            }
        }
        match cover {
            Some(u) => { bytes(b, &mut n, b",\"cover\":{\"kind\":\"url\",\"url\":")?; q(b, &mut n, u)?; bytes(b, &mut n, b"}")?; }
            None => { bytes(b, &mut n, b",\"cover\":{\"kind\":\"none\"}")?; }
        }
        bytes(b, &mut n, b",\"contentRating\":\"unknown\"}")?;
        count += 1;
    }
    bytes(b, &mut n, b"],\"page\":{\"nextCursor\":")?;
    match next {
        Some(v) => { bytes(b, &mut n, b"\"")?; write_usize(b, &mut n, v); bytes(b, &mut n, b"\"")?; }
        None => { bytes(b, &mut n, b"null")?; }
    }
    bytes(b, &mut n, if has_more { b",\"hasMore\":true}}" } else { b",\"hasMore\":false}}" })?;
    Ok(n)
}

fn write_list(req: &[u8], path: &[u8]) -> Result<usize, &'static str> {
    let root = fetch(req, path)?;
    write_cards(req, &root, false, None)
}

fn search_next_page(root: &Node, page: usize) -> Option<usize> {
    let nodes = Nodes::get(root, b"a").ok()?;
    let mut best: Option<usize> = None;
    for i in 0..nodes.n {
        let a = selected(&nodes, i);
        let mut h = [0; 4096];
        let href = match a.optional_attr(b"href", &mut h) { Ok(Some(v)) => v, _ => continue };
        let idx = match find_bytes(href, b"/page/") { Some(v) => v, None => continue };
        let rest = &href[idx + 6..];
        let end = rest.iter().position(|b| !b.is_ascii_digit()).unwrap_or(rest.len());
        if let Some(n) = number(&rest[..end]) {
            if n > page && best.map_or(true, |b| n < b) { best = Some(n); }
        }
    }
    best
}

fn write_search(req: &[u8]) -> Result<usize, &'static str> {
    let raw = extract_json_string(args(req), b"query").ok_or("missing query")?;
    let mut qbuf = [0; 4096];
    let query = unescape(raw, &mut qbuf)?;
    if trim_ascii(query).is_empty() { return Err("empty query"); }
    let page = page_cursor(req).max(1);
    let mut path = [0; 8192];
    let mut len = 0;
    bytes(&mut path, &mut len, b"/search/")?;
    if !write_url_encoded(&mut path, &mut len, query) { return Err("query overflow"); }
    if page > 1 {
        bytes(&mut path, &mut len, b"/page/")?;
        if !write_usize(&mut path, &mut len, page) { return Err("page overflow"); }
    }
    let root = fetch(req, &path[..len])?;
    let next = search_next_page(&root, page);
    write_cards(req, &root, next.is_some(), next)
}

fn strip_book_marks(s: &[u8]) -> &[u8] {
    let s = trim_ascii(s);
    let s = s.strip_prefix("\u{300a}".as_bytes()).unwrap_or(s);
    s.strip_suffix("\u{300b}".as_bytes()).unwrap_or(s)
}

fn write_detail(req: &[u8]) -> Result<usize, &'static str> {
    let id = manga_id(req)?;
    let base = mirror_origin(req);
    let mut path = [0; 8192];
    let mut len = 0;
    bytes(&mut path, &mut len, b"/manhua/")?;
    bytes(&mut path, &mut len, id)?;
    bytes(&mut path, &mut len, b".html")?;
    let root = fetch(req, &path[..len])?;
    let h1 = root.select(b"h1")?;
    let mut tb = [0; 32768];
    let title = decode_entities(strip_book_marks(h1.text(&mut tb)?), scratch_a())?;
    if title.is_empty() { return Err("manga title missing"); }
    let mut author = [0; 8192];
    let mut author_len = 0;
    let mut status = b"unknown".as_slice();
    // The author cell can contain <a>name</a><span>/</span>. Read the anchors,
    // not the concatenated cell text, so the separator and indentation do not
    // become part of the author string.
    if let Ok(authors) = Nodes::get(&root, b"div.module-info-item-content a") {
        let mut count = 0;
        for i in 0..authors.n {
            let a = selected(&authors, i);
            let mut ab = [0; 4096];
            let raw = a.text(&mut ab)?;
            let mut db = [0; 4096];
            let value = decode_entities(raw, &mut db)?;
            if value.is_empty() { continue; }
            if count > 0 && !write_bytes(&mut author, &mut author_len, b", ") { break; }
            if !write_bytes(&mut author, &mut author_len, value) { break; }
            count += 1;
        }
    }
    if let Ok(items) = Nodes::get(&root, b"div.module-info-item-content") {
        if author_len == 0 && items.n > 0 {
            let first = selected(&items, 0);
            let mut ab = [0; 8192];
            let raw = trim_ascii(first.text(&mut ab)?);
            let end = raw.iter().position(|b| *b == b'\n' || *b == b'/').unwrap_or(raw.len());
            let decoded = decode_entities(&raw[..end], &mut author)?;
            author_len = decoded.len();
        }
        if items.n > 1 {
            let second = selected(&items, 1);
            let mut sb = [0; 4096];
            let text = second.text(&mut sb)?;
            if contains(text, "连载".as_bytes()) || contains(text, "連載".as_bytes()) { status = b"ongoing"; }
            else if contains(text, "完结".as_bytes()) || contains(text, "完結".as_bytes()) { status = b"completed"; }
        }
    }
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"manga\":{\"id\":\"manga:")?;
    bytes(b, &mut n, id)?;
    bytes(b, &mut n, b"\",\"title\":")?;
    q(b, &mut n, title)?;
    bytes(b, &mut n, b",\"description\":")?;
    if let Ok(desc) = root.select(b"div.module-info-introduction-content") {
        let mut db = [0; 32768];
        let raw = desc.text(&mut db)?;
        let mut decoded_buf = [0; 32768];
        let decoded = decode_entities(raw, &mut decoded_buf)?;
        q(b, &mut n, decoded)?;
    } else {
        q(b, &mut n, b"")?;
    }
    bytes(b, &mut n, b",\"cover\":")?;
    let mut cb = [0; 8192];
    match root.select(b"div.module-info-poster img") {
        Ok(img) => match cover_url(&img, base, &mut cb)? {
            Some(u) => { bytes(b, &mut n, b"{\"kind\":\"url\",\"url\":")?; q(b, &mut n, u)?; bytes(b, &mut n, b"}")?; }
            None => { bytes(b, &mut n, b"null")?; }
        },
        Err(_) => { bytes(b, &mut n, b"null")?; }
    }
    bytes(b, &mut n, b",\"authors\":[")?;
    if author_len > 0 { q(b, &mut n, &author[..author_len])?; }
    bytes(b, &mut n, b"],\"artists\":[],\"status\":")?;
    q(b, &mut n, status)?;
    bytes(b, &mut n, b",\"contentRating\":\"unknown\",\"language\":\"zh\",\"tags\":[")?;
    if let Ok(tags) = Nodes::get(&root, b"div.module-info-tag-link a") {
        let mut first = true;
        for i in 0..tags.n {
            let tag = selected(&tags, i);
            let mut xb = [0; 4096];
            let raw = tag.text(&mut xb)?;
            if raw.is_empty() { continue; }
            let mut db = [0; 4096];
            let decoded = decode_entities(raw, &mut db)?;
            if !first { bytes(b, &mut n, b",")?; }
            q(b, &mut n, decoded)?;
            first = false;
        }
    }
    bytes(b, &mut n, b"],\"links\":[{\"kind\":\"source\",\"url\":")?;
    let mut wu = [0; 8192];
    let mut wn = 0;
    bytes(&mut wu, &mut wn, base)?;
    bytes(&mut wu, &mut wn, b"/manhua/")?;
    bytes(&mut wu, &mut wn, id)?;
    bytes(&mut wu, &mut wn, b".html")?;
    q(b, &mut n, &wu[..wn])?;
    bytes(b, &mut n, b"}]}}")?;
    Ok(n)
}

fn chapter_number(title: &[u8]) -> Option<&[u8]> {
    let start = find_bytes(title, "\u{7b2c}".as_bytes())? + 3;
    let mut end = start;
    while end < title.len() && title[end].is_ascii_digit() { end += 1; }
    if end == start { return None; }
    let tail = &title[end..];
    if tail.starts_with("\u{8bdd}".as_bytes()) || tail.starts_with("\u{8a71}".as_bytes()) { Some(&title[start..end]) } else { None }
}

fn strip_chapter_prefix<'a>(title: &'a [u8], manga: &[u8]) -> &'a [u8] {
    if let Some(rest) = title.strip_prefix(manga) {
        if let Some(rest) = rest.strip_prefix(b"-") { return rest; }
    }
    title
}

fn write_chapters(req: &[u8]) -> Result<usize, &'static str> {
    let id = manga_id(req)?;
    let mut path = [0; 8192];
    let mut len = 0;
    bytes(&mut path, &mut len, b"/manhua/")?;
    bytes(&mut path, &mut len, id)?;
    bytes(&mut path, &mut len, b".html")?;
    let root = fetch(req, &path[..len])?;
    let mut manga_title_buf = [0; 32768];
    let manga_title = match root.select(b"h1") {
        Ok(h1) => decode_entities(strip_book_marks(h1.text(&mut manga_title_buf)?), scratch_a())?,
        Err(_) => b"",
    };
    let nodes = Nodes::get(&root, b"a.module-play-list-link")?;
    if nodes.n == 0 { return Err("empty chapters"); }
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"items\":[")?;
    let mut count = 0;
    for i in 0..nodes.n {
        let a = selected(&nodes, i);
        let mut hb = [0; 8192];
        let href = match a.optional_attr(b"href", &mut hb)? { Some(v) => v, None => continue };
        let (cid, hash) = match chapter_parts(href) { Some(v) => v, None => continue };
        if cid != id { continue; }
        let mut tb = [0; 16384];
        let mut xb = [0; 16384];
        let raw_title = match a.optional_attr(b"title", &mut tb)? {
            Some(v) if !v.is_empty() => v,
            _ => a.text(&mut xb)?,
        };
        let mut db = [0; 16384];
        let decoded = decode_entities(raw_title, &mut db)?;
        let title = strip_chapter_prefix(decoded, manga_title);
        if title.is_empty() { continue; }
        let number = chapter_number(title);
        if count > 0 { bytes(b, &mut n, b",")?; }
        bytes(b, &mut n, b"{\"id\":\"chapter:")?;
        bytes(b, &mut n, id)?;
        bytes(b, &mut n, b":")?;
        bytes(b, &mut n, hash)?;
        bytes(b, &mut n, b"\",\"mangaId\":\"manga:")?;
        bytes(b, &mut n, id)?;
        bytes(b, &mut n, b"\",\"title\":")?;
        q(b, &mut n, title)?;
        bytes(b, &mut n, b",\"chapterNumber\":")?;
        match number { Some(v) => q(b, &mut n, v)?, None => bytes(b, &mut n, b"null")? }
        bytes(b, &mut n, b",\"volumeNumber\":null,\"language\":\"zh\",\"publishedAt\":null,\"updatedAt\":null,\"pageCount\":null}")?;
        count += 1;
    }
    if count == 0 { return Err("empty chapters"); }
    bytes(b, &mut n, b"],\"page\":{\"nextCursor\":null,\"hasMore\":false}}")?;
    Ok(n)
}

fn write_pages(req: &[u8]) -> Result<usize, &'static str> {
    let raw = extract_json_string(args(req), b"chapterId").ok_or("missing chapterId")?;
    let rest = raw.strip_prefix(b"chapter:").ok_or("invalid chapterId")?;
    let mut parts = rest.split(|b| *b == b':');
    let id = parts.next().ok_or("invalid chapterId")?;
    let hash = parts.next().ok_or("invalid chapterId")?;
    if parts.next().is_some() || !positive_id(id) || hash.is_empty() || hash.contains(&b'/') { return Err("invalid chapterId"); }
    let base = mirror_origin(req);
    let mut path = [0; 8192];
    let mut len = 0;
    bytes(&mut path, &mut len, b"/manhua/")?;
    bytes(&mut path, &mut len, id)?;
    bytes(&mut path, &mut len, b"/")?;
    bytes(&mut path, &mut len, hash)?;
    bytes(&mut path, &mut len, b".html")?;
    let root = fetch(req, &path[..len])?;
    let nodes = Nodes::get(&root, b"img")?;
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"pages\":[")?;
    let mut index = 0;
    for i in 0..nodes.n {
        let img = selected(&nodes, i);
        let mut ub = [0; 8192];
        let url = match content_image(&img, base, &mut ub)? { Some(v) => v, None => continue };
        if index > 0 { bytes(b, &mut n, b",")?; }
        bytes(b, &mut n, b"{\"id\":\"page:")?;
        bytes(b, &mut n, hash)?;
        bytes(b, &mut n, b":")?;
        write_usize(b, &mut n, index);
        bytes(b, &mut n, b"\",\"index\":")?;
        write_usize(b, &mut n, index);
        bytes(b, &mut n, b",\"image\":{\"kind\":\"url\",\"url\":")?;
        q(b, &mut n, url)?;
        bytes(b, &mut n, b"}}")?;
        index += 1;
    }
    if index == 0 { return Err("empty pages"); }
    bytes(b, &mut n, b"]}")?;
    Ok(n)
}

fn run_get_settings(_: &[u8]) -> u32 {
    write_static("get_settings", SETTINGS_JSON.as_bytes())
}

fn run_get_listings(_: &[u8]) -> u32 {
    write_static("get_listings", r#"{"listings":[{"id":"popular","name":"热门","supportsPaging":false},{"id":"latest","name":"最近更新","supportsPaging":false},{"id":"newbook","name":"新书发布","supportsPaging":false},{"id":"recommend","name":"推荐漫画","supportsPaging":false},{"id":"hotsearch","name":"热搜榜","supportsPaging":false},{"id":"rank_week","name":"周榜","supportsPaging":false},{"id":"rank_day","name":"日榜","supportsPaging":false},{"id":"rank_month","name":"月榜","supportsPaging":false},{"id":"rank_total","name":"总榜","supportsPaging":false}]}"#.as_bytes())
}

fn empty_listing() -> u32 {
    write_static("get_manga_list", br#"{"items":[],"page":{"nextCursor":null,"hasMore":false}}"#)
}

fn run_get_manga_list(req: &[u8]) -> u32 {
    if page_cursor(req) > 0 { return empty_listing(); }
    let listing = extract_json_string(args(req), b"listingId").unwrap_or(b"popular");
    let path: &[u8] = if listing == b"popular" { popular_path(req) }
        else if listing == b"latest" { b"/newbook" }
        else if listing == b"newbook" { b"/newbook/newbook" }
        else if listing == b"recommend" { b"/newbook/recommend" }
        else if listing == b"hotsearch" { b"/hotsearch" }
        else if listing == b"rank_week" { b"/bookrank/weekly" }
        else if listing == b"rank_day" { b"/bookrank/daily" }
        else if listing == b"rank_month" { b"/bookrank/monthly" }
        else if listing == b"rank_total" { b"/bookrank/all" }
        else { return write_error("get_manga_list", "invalid_request", "unknown listing"); };
    execute("get_manga_list", write_list(req, path))
}

fn image_request(req: &[u8]) -> Result<usize, &'static str> {
    let raw = extract_json_string(args(req), b"url").ok_or("missing image URL")?;
    let mut ub = [0; 8192];
    let url = unescape(raw, &mut ub)?;
    if !url.starts_with(b"https://") && !url.starts_with(b"http://") { return Err("invalid image URL"); }
    let base = mirror_origin(req);
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"url\":")?;
    q(b, &mut n, url)?;
    bytes(b, &mut n, b",\"headers\":{\"Referer\":\"")?;
    bytes(b, &mut n, base)?;
    bytes(b, &mut n, b"/\",\"User-Agent\":\"Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/116.0.0.0 Mobile Safari/537.36\"}}")?;
    Ok(n)
}

fn run_search(req: &[u8]) -> u32 { execute("search", write_search(req)) }
fn run_get_manga(req: &[u8]) -> u32 { execute("get_manga", write_detail(req)) }
fn run_get_chapters(req: &[u8]) -> u32 { execute("get_chapters", write_chapters(req)) }
fn run_get_pages(req: &[u8]) -> u32 { execute("get_pages", write_pages(req)) }
fn run_get_image_request(req: &[u8]) -> u32 { execute("get_image_request", image_request(req)) }
fn run_get_home(_: &[u8]) -> u32 { write_error("get_home", "not_implemented", "not supported") }
fn run_get_filters(_: &[u8]) -> u32 { write_static("get_filters", br#"{"filters":[]}"#) }

koma_source_sdk::koma_source_exports!("longjm");

