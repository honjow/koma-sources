#![no_std]
use koma_source_sdk::json_utils::*;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{build_get_request, decode_json_body_into, host};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.bilibili.fgo.zh.koma",
    name: "FGO国服官方漫画",
    version: "0.1.2",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "Public Chinese Fate/Grand Order manga from its official bilibili game website.",
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
    filters: false,
    settings: false,
    image_request: true,
    credentials: false,
};
koma_source_sdk::koma_source_buffers! {payload:1024*1024-1024,http_out:1024*1024,body:1024*1024,http_req:4096,scratch:8192,}
koma_source_sdk::koma_source_helpers!();
type R<T> = Result<T, &'static str>;
fn member<'a>(v: &'a [u8], k: &[u8]) -> R<&'a [u8]> {
    json_object_member(v, k).ok_or("parse_error")
}
fn string<'a>(v: &'a [u8], k: &[u8]) -> R<&'a [u8]> {
    let x = member(v, k)?;
    if x.len() < 2 || x[0] != b'"' || x[x.len() - 1] != b'"' {
        return Err("parse_error");
    };
    Ok(&x[1..x.len() - 1])
}
fn number(v: &[u8]) -> R<usize> {
    if v.is_empty() {
        return Err("parse_error");
    };
    let mut n = 0usize;
    for b in v {
        if !b.is_ascii_digit() {
            return Err("parse_error");
        };
        n = n
            .checked_mul(10)
            .and_then(|n| n.checked_add((b - b'0') as usize))
            .ok_or("parse_error")?;
    }
    Ok(n)
}
fn raw_string(v: &[u8], out: &mut [u8]) -> R<usize> {
    let mut i = 0;
    let mut c = 0;
    while i < v.len() {
        let b = v[i];
        i += 1;
        if b != b'\\' {
            if b < 32 || !write_bytes(out, &mut c, &[b]) {
                return Err("parse_error");
            };
            continue;
        }
        if i == v.len() {
            return Err("parse_error");
        };
        let e = v[i];
        i += 1;
        match e {
            b'"' | b'\\' | b'/' => {
                if !write_bytes(out, &mut c, &[e]) {
                    return Err("capacity_exceeded");
                }
            }
            b'n' | b'r' | b't' | b'b' | b'f' => {
                let b = match e {
                    b'n' => 10,
                    b'r' => 13,
                    b't' => 9,
                    b'b' => 8,
                    _ => 12,
                };
                if !write_bytes(out, &mut c, &[b]) {
                    return Err("capacity_exceeded");
                }
            }
            b'u' => {
                if i + 4 > v.len() || !v[i..i + 4].iter().all(|b| b.is_ascii_hexdigit()) {
                    return Err("parse_error");
                };
                let mut cp = hex_to_u16(&v[i..i + 4]) as u32;
                i += 4;
                if (0xd800..=0xdbff).contains(&cp) {
                    if i + 6 > v.len()
                        || &v[i..i + 2] != b"\\u"
                        || !v[i + 2..i + 6].iter().all(|b| b.is_ascii_hexdigit())
                    {
                        return Err("parse_error");
                    };
                    let low = hex_to_u16(&v[i + 2..i + 6]) as u32;
                    if !(0xdc00..=0xdfff).contains(&low) {
                        return Err("parse_error");
                    };
                    cp = 0x10000 + ((cp - 0xd800) << 10) + (low - 0xdc00);
                    i += 6;
                } else if (0xdc00..=0xdfff).contains(&cp) {
                    return Err("parse_error");
                };
                let mut b = [0; 4];
                let n = encode_utf8(cp, &mut b);
                if !write_bytes(out, &mut c, &b[..n]) {
                    return Err("capacity_exceeded");
                }
            }
            _ => return Err("parse_error"),
        }
    }
    Ok(c)
}
fn q(p: &mut [u8], c: &mut usize, v: &[u8]) -> R<()> {
    if write_bytes(p, c, b"\"")
        && append_json_unescaped_then_escaped(p, c, v)
        && write_bytes(p, c, b"\"")
    {
        Ok(())
    } else {
        Err("capacity_exceeded")
    }
}
fn w(p: &mut [u8], c: &mut usize, v: &[u8]) -> R<()> {
    if write_bytes(p, c, v) {
        Ok(())
    } else {
        Err("capacity_exceeded")
    }
}
fn n(p: &mut [u8], c: &mut usize, v: usize) -> R<()> {
    if write_usize(p, c, v) {
        Ok(())
    } else {
        Err("capacity_exceeded")
    }
}

const BASE: &[u8] = b"https://game.bilibili.com/fgo/manga01";
const READER: &[u8] = b"https://game.bilibili.com/fgo/manga01_comic?episode=";
const READER_TEMPLATE: &[u8] = b"https://game.bilibili.com/fgo/manga01_comic";
const MID: &[u8] = b"fgo:zh:manga01";
const TITLE: &str = "从漫画了解！Fate/Grand Order";
static mut CSS: [u8; 128 * 1024] = [0; 128 * 1024];
static mut IDS: [usize; 1000] = [0; 1000];
fn image_url(u: &[u8]) -> bool {
    let Some(tail) = u.strip_prefix(b"https://i0.hdslb.com/bfs/game/") else {
        return false;
    };
    let key = tail
        .strip_suffix(b".jpg")
        .or_else(|| tail.strip_suffix(b".png"));
    key.map(|key| key.len() == 40 && key.iter().all(u8::is_ascii_hexdigit))
        .unwrap_or(false)
}
fn url(s: &[u8], out: &mut [u8]) -> R<usize> {
    let mut b = [0; 2048];
    let len = raw_string(s, &mut b)?;
    let mut c = 0;
    if b[..len].starts_with(b"//") {
        w(out, &mut c, b"https:")?
    };
    w(out, &mut c, &b[..len])?;
    Ok(c)
}
fn fetch(u: &[u8]) -> R<usize> {
    let n = build_get_request(http_req_buf(), u, Some(BASE), &[]).ok_or("capacity_exceeded")?;
    let n = host::http_request(&http_req_buf()[..n], http_out()).map_err(|_| "network_error")?;
    if extract_json_number(&http_out()[..n], b"status") != Some(b"200") {
        return Err("http_error");
    };
    decode_json_body_into(&http_out()[..n], body_buf()).map_err(|_| "network_error")
}
fn nuxt(size: usize) -> R<&'static [u8]> {
    let h = &body_buf()[..size];
    let at = find_subslice(h, b"window.__NUXT__=").ok_or("parse_error")? + 16;
    let end = find_subslice(&h[at..], b";</script>").ok_or("parse_error")? + at;
    let v = &h[at..end];
    if v.first() != Some(&b'{') || v.last() != Some(&b'}') {
        return Err("parse_error");
    };
    let mut it = ObjectArray::new(member(v, b"data")?)?;
    let d = it.next()?.ok_or("parse_error")?;
    if it.next()?.is_some() {
        return Err("parse_error");
    };
    let meta = member(d, b"metaInfo")?;
    let attrs = member(meta, b"htmlAttrs")?;
    if string(attrs, b"site-id")? != b"fgo" || member(attrs, b"game-id")? != b"49" {
        return Err("wrong_owner");
    };
    Ok(d)
}
// Object-array iteration distinguishes the legitimate end from invalid elements.
struct ObjectArray<'a> { data: &'a [u8], pos: usize, after_object: bool, done: bool }
impl<'a> ObjectArray<'a> {
    fn new(data: &'a [u8]) -> R<Self> {
        let data = trim_ascii(data);
        if data.first() != Some(&b'[') { return Err("parse_error"); }
        Ok(Self { data, pos: 1, after_object: false, done: false })
    }
    fn space(&mut self) {
        while matches!(self.data.get(self.pos), Some(b' ' | b'\t' | b'\r' | b'\n')) { self.pos += 1; }
    }
    fn next(&mut self) -> R<Option<&'a [u8]>> {
        if self.done { return Ok(None); }
        self.space();
        if self.after_object {
            if self.data.get(self.pos) == Some(&b',') {
                self.pos += 1;
                self.space();
                if self.data.get(self.pos) == Some(&b']') { return Err("parse_error"); }
            } else if self.data.get(self.pos) != Some(&b']') { return Err("parse_error"); }
        }
        if self.data.get(self.pos) == Some(&b']') {
            self.pos += 1;
            self.space();
            if self.pos != self.data.len() { return Err("parse_error"); }
            self.done = true;
            return Ok(None);
        }
        if self.data.get(self.pos) != Some(&b'{') { return Err("parse_error"); }
        let stop = end(self.data, self.pos)?;
        if self.data.get(stop - 1) != Some(&b'}') { return Err("parse_error"); }
        let value = &self.data[self.pos..stop];
        self.pos = stop;
        self.after_object = true;
        Ok(Some(value))
    }
}
fn catalogue() -> R<usize> {
    let size = fetch(BASE)?;
    let d = nuxt(size)?;
    let mut it = ObjectArray::new(member(d, b"comList")?)?;
    let mut count = 0;
    while let Some(x) = it.next()? {
        if string(x, b"category")? != b"BiliButton" {
            continue;
        };
        let opt = member(x, b"option")?;
        let Some(raw) = json_object_member(opt, b"href") else {
            continue;
        };
        if raw.len() < 2 || raw[0] != b'"' {
            continue;
        };
        let mut ub = [0; 2048];
        let len = url(&raw[1..raw.len() - 1], &mut ub)?;
        let u = &ub[..len];
        let Some(ep) = u.strip_prefix(READER) else {
            continue;
        };
        let ep = number(ep)?;
        if ep == 0 || count >= 1000 || unsafe { IDS[..count].contains(&ep) } {
            return Err("parse_error");
        };
        unsafe { IDS[count] = ep };
        count += 1;
    }
    if count == 0 {
        return Err("empty_catalogue");
    };
    Ok(count)
}
fn cover(d: &[u8], out: &mut [u8]) -> R<usize> {
    let meta = member(d, b"metaInfo")?;
    let mut it = ObjectArray::new(member(meta, b"meta")?)?;
    while let Some(x) = it.next()? {
        if json_object_member(x, b"property") == Some(b"\"og:image\"") {
            let n = url(string(x, b"content")?, out)?;
            if !image_url(&out[..n]) {
                return Err("parse_error");
            };
            return Ok(n);
        }
    }
    Err("parse_error")
}
fn book(p: &mut [u8], c: &mut usize, cov: &[u8], detail: bool) -> R<()> {
    w(p, c, b"{\"id\":")?;
    q(p, c, MID)?;
    w(p, c, b",\"title\":")?;
    q(p, c, TITLE.as_bytes())?;
    w(p, c, b",\"url\":")?;
    q(p, c, BASE)?;
    w(p, c, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(p, c, cov)?;
    w(p,c,b"},\"language\":\"zh\",\"authors\":[],\"status\":\"unknown\",\"contentRating\":\"unknown\"")?;
    if detail {
        w(p, c, b",\"description\":")?;
        q(p, c, "在漫画中了解更多关于《FGO》的知识吧！".as_bytes())?;
        w(p, c, b",\"links\":[{\"kind\":\"source\",\"url\":")?;
        q(p, c, BASE)?;
        w(p, c, b"}]")?
    };
    w(p, c, b"}")
}
fn empty(op: &str) -> u32 {
    let b = b"{\"items\":[],\"page\":{\"hasMore\":false}}";
    payload_buf()[..b.len()].copy_from_slice(b);
    write_success_payload(op, b.len())
}
fn result(op: &str, r: R<usize>) -> u32 {
    match r {
        Ok(n) => write_success_payload(op, n),
        Err(e) => write_error(op, e, "Official public contract unavailable or invalid"),
    }
}
fn page(req: &[u8]) -> R<usize> {
    let args = json_object_member(req, b"args").unwrap_or(req);
    let v = match json_object_member(args, b"page") {
        None => return Ok(1),
        Some(v) => v,
    };
    let v = if v.first() == Some(&b'{') {
        json_object_member(v, b"cursor").unwrap_or(b"null")
    } else {
        v
    };
    if v == b"null" {
        return Ok(1);
    }
    let v = if v.len() >= 2 && v[0] == b'"' && v[v.len() - 1] == b'"' {
        &v[1..v.len() - 1]
    } else {
        v
    };
    let n = number(v).map_err(|_| "invalid_request")?;
    if n == 0 {
        return Err("invalid_request");
    }
    Ok(n)
}
fn listing() -> R<usize> {
    let size = fetch(BASE)?;
    let d = nuxt(size)?;
    let mut ub = [0; 2048];
    let len = cover(d, &mut ub)?;
    let p = payload_buf();
    let mut c = 0;
    w(p, &mut c, b"{\"items\":[")?;
    book(p, &mut c, &ub[..len], false)?;
    w(p, &mut c, b"],\"page\":{\"hasMore\":false}}")?;
    Ok(c)
}
fn run_search(req: &[u8]) -> u32 {
    let raw = extract_json_string(req, b"query").unwrap_or(b"");
    let mut b = [0; 1024];
    let len = match raw_string(raw, &mut b) {
        Ok(n) => n,
        Err(e) => return write_error("search", e, ""),
    };
    if !b[..len].is_empty()
        && !contains_bytes(
            "FGO fgo Fate/Grand Order 从漫画了解 命运冠位指定".as_bytes(),
            &b[..len],
        )
    {
        return empty("search");
    };
    match page(req) {
        Ok(1) => result("search", listing()),
        Ok(_) => empty("search"),
        Err(e) => write_error("search", e, ""),
    }
}
fn run_get_manga_list(req: &[u8]) -> u32 {
    if extract_json_string(req, b"listingId") != Some(b"listing:official") {
        return write_error("get_manga_list", "invalid_request", "");
    };
    match page(req) {
        Ok(1) => {}
        Ok(_) => return empty("get_manga_list"),
        Err(e) => return write_error("get_manga_list", e, ""),
    };
    result("get_manga_list", listing())
}
fn run_get_manga(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        if extract_json_string(req, b"mangaId") != Some(MID) {
            return Err("not_found");
        };
        let size = fetch(BASE)?;
        let d = nuxt(size)?;
        let mut ub = [0; 2048];
        let len = cover(d, &mut ub)?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"manga\":")?;
        book(p, &mut c, &ub[..len], true)?;
        w(p, &mut c, b"}")?;
        Ok(c)
    }
    result("get_manga", inner(req))
}
fn run_get_chapters(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        if extract_json_string(req, b"mangaId") != Some(MID) {
            return Err("not_found");
        };
        let count = catalogue()?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"chapters\":[")?;
        for i in 0..count {
            let ep = unsafe { IDS[i] };
            if i > 0 {
                w(p, &mut c, b",")?
            };
            w(p, &mut c, b"{\"id\":\"fgoch:zh:manga01:")?;
            n(p, &mut c, ep)?;
            w(p, &mut c, b"\",\"mangaId\":")?;
            q(p, &mut c, MID)?;
            w(p, &mut c, b",\"title\":\"")?;
            n(p, &mut c, ep)?;
            w(p, &mut c, b"\",\"chapterNumber\":")?;
            n(p, &mut c, ep)?;
            w(p, &mut c, b",\"readingGroup\":")?;
            q(p, &mut c, MID)?;
            w(p, &mut c, b",\"continuationEligible\":true,\"language\":\"zh\",\"pageCount\":1}")?
        }
        w(p, &mut c, b"]}")?;
        Ok(c)
    }
    result("get_chapters", inner(req))
}
// Data-only balanced value scanner; it never evaluates website script.
fn end(v: &[u8], start: usize) -> R<usize> {
    let mut i = start;
    let mut depth = 0;
    let mut quoted = false;
    while i < v.len() {
        let b = v[i];
        if quoted {
            if b == b'\\' {
                i += 1
            } else if b == b'"' {
                quoted = false
            }
        } else {
            match b {
                b'"' => quoted = true,
                b'[' | b'{' => depth += 1,
                b']' | b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(i + 1);
                    }
                }
                _ => {}
            }
        };
        i += 1;
    }
    Err("parse_error")
}
fn run_get_pages(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let id = extract_json_string(req, b"chapterId").ok_or("invalid_request")?;
        let ep = number(
            id.strip_prefix(b"fgoch:zh:manga01:")
                .ok_or("invalid_request")?,
        )?;
        let count = catalogue()?;
        if !unsafe { IDS[..count].contains(&ep) } {
            return Err("not_found");
        };
        let size = fetch(READER_TEMPLATE)?;
        let d = nuxt(size)?;
        let mut it = ObjectArray::new(member(d, b"comList")?)?;
        let mut banner = None;
        let mut selector = false;
        while let Some(x) = it.next()? {
            let cat = string(x, b"category")?;
            if cat == b"BiliBanner" {
                if banner.is_some() {
                    return Err("parse_error");
                };
                banner = Some(x)
            } else if cat == b"BiliUser" {
                let opt = member(x, b"option")?;
                if string(opt, b"script_id")? == b"vue_e800a3b6620266002a7832f50cbae7c4075109e3" {
                    selector = true
                }
            }
        }
        if !selector {
            return Err("unsupported_selector");
        };
        let banner = banner.ok_or("parse_error")?;
        if string(banner, b"id")? != b"autoid-lsHKbInB" {
            return Err("unsupported_selector");
        }
        let rows = member(banner, b"subComponentsList")?;
        if rows.first() != Some(&b'[') {
            return Err("parse_error");
        };
        let mut i = 1;
        let mut rowcount = 0;
        let mut selected = [0u8; 128];
        let mut selected_len = 0;
        loop {
            while i < rows.len() && (rows[i].is_ascii_whitespace() || rows[i] == b',') {
                i += 1
            }
            if rows.get(i) == Some(&b']') {
                break;
            };
            if rows.get(i) != Some(&b'[') {
                return Err("parse_error");
            };
            let stop = end(rows, i)?;
            let mut nested = ObjectArray::new(&rows[i..stop])?;
            let x = nested.next()?.ok_or("parse_error")?;
            if nested.next()?.is_some() || string(x, b"category")? != b"BiliImage" {
                return Err("parse_error");
            };
            let id = string(x, b"id")?;
            if rowcount + 1 == ep {
                if id.len() > selected.len() {
                    return Err("capacity_exceeded");
                };
                selected[..id.len()].copy_from_slice(id);
                selected_len = id.len()
            };
            rowcount += 1;
            i = stop
        }
        if rowcount != count || selected_len == 0 {
            return Err("incomplete_pages");
        };
        let meta = member(d, b"metaInfo")?;
        let mut it = ObjectArray::new(member(meta, b"style")?)?;
        let css = unsafe { &mut *core::ptr::addr_of_mut!(CSS) };
        let mut c = 0;
        while let Some(x) = it.next()? {
            let raw = string(x, b"cssText")?;
            let len = raw_string(raw, &mut css[c..])?;
            c += len;
        }
        let mut pattern = [0; 132];
        let mut n0 = 0;
        w(&mut pattern, &mut n0, b"#")?;
        w(&mut pattern, &mut n0, &selected[..selected_len])?;
        w(&mut pattern, &mut n0, b"{")?;
        let at = find_subslice(&css[..c], &pattern[..n0]).ok_or("parse_error")? + n0;
        let stop = css[at..c]
            .iter()
            .position(|b| *b == b'}')
            .ok_or("parse_error")?
            + at;
        let rule = &css[at..stop];
        let marker = b"background-image:url(";
        let at = find_subslice(rule, marker).ok_or("parse_error")? + marker.len();
        let stop = rule[at..]
            .iter()
            .position(|b| *b == b')')
            .ok_or("parse_error")?
            + at;
        let mut ub = [0; 2048];
        let len = url(&rule[at..stop], &mut ub)?;
        if !image_url(&ub[..len]) {
            return Err("parse_error");
        };
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"pages\":[{\"id\":\"fgopage:zh:manga01:")?;
        n(p, &mut c, ep)?;
        w(
            p,
            &mut c,
            b":0\",\"index\":0,\"image\":{\"kind\":\"url\",\"url\":",
        )?;
        q(p, &mut c, &ub[..len])?;
        w(p, &mut c, b"}}]}")?;
        Ok(c)
    }
    result("get_pages", inner(req))
}
fn run_get_image_request(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let u = extract_json_string(req, b"url").ok_or("invalid_request")?;
        if !image_url(u) {
            return Err("invalid_request");
        };
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"imageRequest\":{\"url\":")?;
        q(p, &mut c, u)?;
        w(p, &mut c, b",\"headers\":{\"Referer\":")?;
        q(p, &mut c, BASE)?;
        w(p, &mut c, b"}}}")?;
        Ok(c)
    }
    result("get_image_request", inner(req))
}
fn run_get_listings(_req: &[u8]) -> u32 {
    let b = "{\"listings\":[{\"id\":\"listing:official\",\"name\":\"官方漫画\"}]}".as_bytes();
    payload_buf()[..b.len()].copy_from_slice(b);
    write_success_payload("get_listings", b.len())
}
fn run_get_home(_req: &[u8]) -> u32 {
    write_error("get_home", "not_implemented", "")
}
fn run_get_filters(_req: &[u8]) -> u32 {
    write_error("get_filters", "not_implemented", "")
}
fn run_get_settings(_req: &[u8]) -> u32 {
    write_error("get_settings", "not_implemented", "")
}
const REQUEST_CAP: usize = 1024 * 1024;
static mut REQUEST: [u8; REQUEST_CAP] = [0; REQUEST_CAP];
fn checked_request(ptr: u32, len: u32) -> Option<&'static [u8]> {
    let cap = core::arch::wasm32::memory_size::<0>().checked_mul(65536)?;
    let start = ptr as usize;
    let end = start.checked_add(len as usize)?;
    if start == 0 || len == 0 || len as usize >= REQUEST_CAP || end > cap {
        return None;
    }
    read_request(ptr, len)
}
#[no_mangle]
pub extern "C" fn koma_source_alloc(len: u32) -> u32 {
    if len == 0 || len as usize >= REQUEST_CAP {
        0
    } else {
        core::ptr::addr_of_mut!(REQUEST) as *mut u8 as u32
    }
}
#[no_mangle]
pub extern "C" fn koma_source_init(ptr: u32, len: u32) -> i32 {
    if checked_request(ptr, len).is_some() {
        0
    } else {
        -1
    }
}
#[no_mangle]
pub extern "C" fn koma_source_info() -> u32 {
    response_buffer().write_source_metadata(&SOURCE_INFO, &SOURCE_CAPS)
}
#[no_mangle]
pub extern "C" fn koma_source_free(ptr: u32) {
    response_buffer().free(ptr)
}
macro_rules! export_op {
    ($f:ident,$op:literal,$run:ident) => {
        #[no_mangle]
        pub extern "C" fn $f(ptr: u32, len: u32) -> u32 {
            let req = match checked_request(ptr, len) {
                Some(r) => r,
                None => return write_error($op, "invalid_request", "invalid ABI range"),
            };
            if host::check_cancel() {
                return write_error($op, "cancelled", "");
            }
            let args = match json_object_member(req, b"args") {
                Some(a) if a.first() == Some(&b'{') => a,
                _ => return write_error($op, "invalid_request", "missing args"),
            };
            if string(req, b"operation").ok() != Some($op.as_bytes()) {
                return write_error($op, "invalid_request", "operation mismatch");
            }
            $run(args)
        }
    };
}
export_op!(koma_source_search, "search", run_search);
export_op!(koma_source_get_manga, "get_manga", run_get_manga);
export_op!(koma_source_get_chapters, "get_chapters", run_get_chapters);
export_op!(koma_source_get_pages, "get_pages", run_get_pages);
export_op!(koma_source_get_listings, "get_listings", run_get_listings);
export_op!(
    koma_source_get_manga_list,
    "get_manga_list",
    run_get_manga_list
);
export_op!(koma_source_get_home, "get_home", run_get_home);
export_op!(koma_source_get_filters, "get_filters", run_get_filters);
export_op!(koma_source_get_settings, "get_settings", run_get_settings);
export_op!(
    koma_source_get_image_request,
    "get_image_request",
    run_get_image_request
);
