#![no_std]
use koma_source_sdk::json_utils::*;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{build_get_request, decode_json_body_into, host};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.mihoyo.genshin.zh.koma",
    name: "原神官方漫画",
    version: "0.1.2",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "Public Chinese Genshin Impact manga from the official miHoYo website.",
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
const BASE: &[u8] = b"https://ys.mihoyo.com/main/manga";
const API: &[u8] =
    b"https://act-api-takumi-static.mihoyo.com/content_v2_user/app/16471662a82d418a/";
const LIST: &[u8] = b"https://act-api-takumi-static.mihoyo.com/content_v2_user/app/16471662a82d418a/getContentList?iAppId=43&iChanId=725&iPageSize=1000&iPage=1&sLangKey=zh-cn";
const MID: &[u8] = b"genshin:zh:official-manga";
static mut EXT: [u8; 128 * 1024] = [0; 128 * 1024];
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
fn image_url(url: &[u8]) -> bool {
    let host = b"https://uploadstatic.mihoyo.com/contentweb/";
    url.starts_with(host)
        && url.len() > host.len()
        && !url
            .iter()
            .any(|b| *b < 33 || matches!(b, b'\\' | b'"' | b'#'))
}
fn fetch(url: &[u8]) -> R<usize> {
    let n = build_get_request(http_req_buf(), url, Some(BASE), &[]).ok_or("capacity_exceeded")?;
    let n = host::http_request(&http_req_buf()[..n], http_out()).map_err(|_| "network_error")?;
    let r = &http_out()[..n];
    if extract_json_number(r, b"status") != Some(b"200") {
        return Err("http_error");
    };
    let n = decode_json_body_into(r, body_buf()).map_err(|_| "network_error")?;
    if member(&body_buf()[..n], b"retcode")? != b"0" {
        return Err("source_error");
    };
    Ok(n)
}
fn data(v: &[u8]) -> R<&[u8]> {
    member(v, b"data")
}
fn extension(v: &[u8]) -> R<&'static [u8]> {
    let x = string(v, b"sExt")?;
    let ext = unsafe { &mut *core::ptr::addr_of_mut!(EXT) };
    let n = raw_string(x, ext)?;
    Ok(&ext[..n])
}
fn pages_array(ext: &[u8]) -> R<&[u8]> {
    member(ext, b"725_0")
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
fn page_count(v: &[u8]) -> R<usize> {
    let ext = extension(v)?;
    let mut a = ObjectArray::new(pages_array(ext)?)?;
    let mut count = 0;
    while let Some(x) = a.next()? {
        let url = string(x, b"url")?;
        if !image_url(url) {
            return Err("parse_error");
        };
        count += 1;
    }
    if count == 0 {
        return Err("parse_error");
    };
    Ok(count)
}
fn list() -> R<usize> {
    let size = fetch(LIST)?;
    let d = data(&body_buf()[..size])?;
    let total = number(member(d, b"iTotal")?)?;
    let mut a = JsonArrayIter::from_raw_array(member(d, b"list")?).ok_or("parse_error")?;
    let mut ids = [0usize; 1000];
    let mut c = 0;
    while let Some(x) = a.next_object() {
        if c == ids.len() {
            return Err("capacity_exceeded");
        };
        let id = number(member(x, b"iInfoId")?)?;
        if ids[..c].contains(&id) || id == 0 {
            return Err("parse_error");
        };
        ids[c] = id;
        c += 1;
    }
    if c != total || c == 0 {
        return Err("incomplete_catalogue");
    };
    Ok(size)
}
fn cover(d: &[u8]) -> R<&'static [u8]> {
    let mut a = JsonArrayIter::from_raw_array(member(d, b"list")?).ok_or("parse_error")?;
    let mut last = None;
    while let Some(x) = a.next_object() {
        last = Some(x);
    }
    let ext = extension(last.ok_or("parse_error")?)?;
    let mut a = JsonArrayIter::from_raw_array(member(ext, b"725_1")?).ok_or("parse_error")?;
    let url = string(a.next_object().ok_or("parse_error")?, b"url")?;
    if !image_url(url) {
        return Err("parse_error");
    };
    Ok(url)
}
fn book(p: &mut [u8], c: &mut usize, cover: &[u8], detail: bool) -> R<()> {
    w(p, c, b"{\"id\":")?;
    q(p, c, MID)?;
    w(p, c, b",\"title\":")?;
    q(p, c, "原神".as_bytes())?;
    w(p, c, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(p, c, cover)?;
    w(p,c,b"},\"authors\":[],\"status\":\"unknown\",\"contentRating\":\"unknown\",\"language\":\"zh\"")?;
    if detail {
        w(p, c, b",\"description\":")?;
        q(p,c,"这里是七种元素神灵信仰交汇的幻想世界提瓦特。席卷大陆的灾难终于停息，疮痍逐渐被抚平——本应到来的安宁却并没有如期光临风之城「蒙德」。飞扬跋扈的组织「愚人众」借着保护的名义欺压诸国，而来自历史之暗的不祥也在等待着复仇的机会……".as_bytes())?;
        w(p, c, b",\"links\":[{\"kind\":\"source\",\"url\":")?;
        q(p, c, BASE)?;
        w(p, c, b"}]")?;
    }
    w(p, c, b"}")
}
fn empty(op: &str) -> u32 {
    let p = payload_buf();
    let b = b"{\"items\":[],\"page\":{\"hasMore\":false,\"nextCursor\":null}}";
    p[..b.len()].copy_from_slice(b);
    write_success_payload(op, b.len())
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
fn listing(req: &[u8], op: &str) -> R<usize> {
    if page(req)? > 1 {
        return Ok(0);
    };
    let size = list()?;
    let cov = cover(data(&body_buf()[..size])?)?;
    let p = payload_buf();
    let mut c = 0;
    w(p, &mut c, b"{\"items\":[")?;
    book(p, &mut c, cov, false)?;
    w(
        p,
        &mut c,
        b"],\"page\":{\"hasMore\":false,\"nextCursor\":null}}",
    )?;
    let _ = op;
    Ok(c)
}
fn result(op: &str, r: R<usize>) -> u32 {
    match r {
        Ok(0) => empty(op),
        Ok(n) => write_success_payload(op, n),
        Err(e) => write_error(op, e, "Official public contract unavailable or invalid"),
    }
}
fn run_search(req: &[u8]) -> u32 {
    let query = extract_json_string(req, b"query").unwrap_or(b"");
    let mut decoded = [0; 1024];
    let query = match raw_string(query, &mut decoded) {
        Ok(n) => &decoded[..n],
        Err(e) => return write_error("search", e, ""),
    };
    if !query.is_empty()
        && !contains_bytes(
            "原神 Genshin Impact genshin impact 原神官方漫画".as_bytes(),
            query,
        )
    {
        return empty("search");
    };
    let r = listing(req, "search");
    result("search", r)
}
fn run_get_manga_list(req: &[u8]) -> u32 {
    if extract_json_string(req, b"listingId") != Some(b"listing:official") {
        return write_error("get_manga_list", "invalid_request", "");
    };
    let r = listing(req, "get_manga_list");
    result("get_manga_list", r)
}
fn run_get_manga(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        if extract_json_string(req, b"mangaId") != Some(MID) {
            return Err("not_found");
        };
        let size = list()?;
        let cov = cover(data(&body_buf()[..size])?)?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"manga\":")?;
        book(p, &mut c, cov, true)?;
        w(p, &mut c, b"}")?;
        Ok(c)
    }
    let r = inner(req);
    result("get_manga", r)
}
// Only an explicit Chinese chapter-number prefix supplies the chapter number.
fn chapter_number(title: &[u8]) -> Option<usize> {
    let mut decoded = [0u8; 256];
    let len = raw_string(title, &mut decoded).ok()?;
    let title = core::str::from_utf8(&decoded[..len])
        .ok()?
        .strip_prefix("第")?;
    let numeral = title.split_once('话').or_else(|| title.split_once('話'))?.0;
    fn digit(c: char) -> Option<usize> {
        "一二三四五六七八九"
            .chars()
            .position(|x| x == c)
            .map(|n| n + 1)
    }
    let mut chars = numeral.chars();
    let a = chars.next()?;
    let b = chars.next();
    let c = chars.next();
    if chars.next().is_some() {
        return None;
    }
    match (a, b, c) {
        ('十', None, None) => Some(10),
        ('十', Some(ones), None) => Some(10 + digit(ones)?),
        (ones, None, None) => digit(ones),
        (tens, Some('十'), None) => Some(digit(tens)? * 10),
        (tens, Some('十'), Some(ones)) => Some(digit(tens)? * 10 + digit(ones)?),
        _ => None,
    }
}
fn run_get_chapters(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        if extract_json_string(req, b"mangaId") != Some(MID) {
            return Err("not_found");
        };
        let size = list()?;
        let d = data(&body_buf()[..size])?;
        let mut a = JsonArrayIter::from_raw_array(member(d, b"list")?).ok_or("parse_error")?;
        let p = payload_buf();
        let mut c = 0;
        let mut chapters: [&[u8]; 1000] = [&[]; 1000];
        let mut count = 0;
        while let Some(x) = a.next_object() {
            if count == chapters.len() {
                return Err("capacity_exceeded");
            }
            chapters[count] = x;
            count += 1;
        }
        w(p, &mut c, b"{\"items\":[")?;
        for (index, x) in chapters[..count].iter().rev().enumerate() {
            if index > 0 {
                w(p, &mut c, b",")?
            };
            let id = member(x, b"iInfoId")?;
            let pc = page_count(x)?;
            w(p, &mut c, b"{\"id\":\"genshinch:zh:")?;
            w(p, &mut c, id)?;
            w(p, &mut c, b"\",\"mangaId\":")?;
            q(p, &mut c, MID)?;
            w(p, &mut c, b",\"title\":")?;
            let title = string(x, b"sTitle")?;
            q(p, &mut c, title)?;
            w(p, &mut c, b",\"chapterNumber\":")?;
            match chapter_number(title) {
                Some(number) => n(p, &mut c, number)?,
                None => w(p, &mut c, b"null")?,
            }
            w(p, &mut c, b",\"readingGroup\":")?;
            q(p, &mut c, MID)?;
            w(p, &mut c, b",\"continuationEligible\":true")?;
            w(p, &mut c, b",\"language\":\"zh\",\"pageCount\":")?;
            n(p, &mut c, pc)?;
            w(p, &mut c, b"}")?;
        }
        w(
            p,
            &mut c,
            b"],\"page\":{\"hasMore\":false,\"nextCursor\":null}}",
        )?;
        Ok(c)
    }
    let r = inner(req);
    result("get_chapters", r)
}
fn run_get_pages(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let ch = extract_json_string(req, b"chapterId").ok_or("invalid_request")?;
        let id = ch.strip_prefix(b"genshinch:zh:").ok_or("invalid_request")?;
        let idn = number(id)?;
        if idn == 0 || id.len() > 10 {
            return Err("invalid_request");
        };
        let mut url = [0; 1024];
        let mut u = 0;
        w(&mut url, &mut u, API)?;
        w(&mut url, &mut u, b"getContent?iAppId=43&iInfoId=")?;
        w(&mut url, &mut u, id)?;
        w(&mut url, &mut u, b"&sLangKey=zh-cn&iAround=0")?;
        let size = fetch(&url[..u])?;
        let d = data(&body_buf()[..size])?;
        if number(member(d, b"iInfoId")?)? != idn {
            return Err("parse_error");
        };
        let chan = member(d, b"sChanId")?;
        if !contains_bytes(chan, b"\"725\"") {
            return Err("not_found");
        };
        let ext = extension(d)?;
        let mut a = ObjectArray::new(pages_array(ext)?)?;
        let p = payload_buf();
        let mut c = 0;
        let mut count = 0;
        w(p, &mut c, b"{\"pages\":[")?;
        while let Some(x) = a.next()? {
            let url = string(x, b"url")?;
            if !image_url(url) {
                return Err("parse_error");
            };
            if count > 0 {
                w(p, &mut c, b",")?
            };
            w(p, &mut c, b"{\"id\":\"genshinpage:zh:")?;
            w(p, &mut c, id)?;
            w(p, &mut c, b":")?;
            n(p, &mut c, count)?;
            w(p, &mut c, b"\",\"index\":")?;
            n(p, &mut c, count)?;
            w(p, &mut c, b",\"image\":{\"kind\":\"url\",\"url\":")?;
            q(p, &mut c, url)?;
            w(p, &mut c, b"}}")?;
            count += 1;
        }
        if count == 0 {
            return Err("parse_error");
        };
        w(p, &mut c, b"]}")?;
        Ok(c)
    }
    let r = inner(req);
    result("get_pages", r)
}
fn run_get_image_request(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let url = extract_json_string(req, b"url").ok_or("invalid_request")?;
        if !image_url(url) {
            return Err("invalid_request");
        };
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"url\":")?;
        q(p, &mut c, url)?;
        w(p, &mut c, b",\"headers\":{}}")?;
        Ok(c)
    }
    let r = inner(req);
    result("get_image_request", r)
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
