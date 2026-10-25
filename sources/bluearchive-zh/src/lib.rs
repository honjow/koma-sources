#![no_std]
use koma_source_sdk::json_utils::*;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{build_get_request, decode_json_body_into, host};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.yostar.bluearchive.zh.koma",
    name: "蔚蓝档案官方漫画",
    version: "0.1.1",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "Public Chinese Blue Archive four-panel comics from the official Yostar website.",
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

const BASE: &[u8] = b"https://www.bluearchive-cn.com/official";
const LIST: &[u8] = b"https://www.bluearchive-cn.com/api/comic/list?pageIndex=1&pageNum=999&type=";
const DETAIL: &[u8] = b"https://www.bluearchive-cn.com/api/comic/detail?chapters=";
const MID: &[u8] = b"bluearchive:zh:four-panel";
const ICON: &[u8] = b"https://webcnstatic.yostar.net/ba_cn_web/prod/web/favicon.png";
static mut IDS: [usize; 1000] = [0; 1000];
fn image_url(u: &[u8]) -> bool {
    if u == ICON {
        return true;
    }
    let Some(t) = u.strip_prefix(b"https://webcnstatic.yostar.net/ba_cn_web/prod/upload/comic/")
    else {
        return false;
    };
    let name = t
        .strip_suffix(b".jpeg")
        .or_else(|| t.strip_suffix(b".jpg"))
        .or_else(|| t.strip_suffix(b".png"));
    name.map(|x| {
        !x.is_empty()
            && x.len() <= 128
            && x.iter()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
    })
    .unwrap_or(false)
}
fn fetch(u: &[u8]) -> R<usize> {
    let n = build_get_request(http_req_buf(), u, Some(BASE), &[]).ok_or("capacity_exceeded")?;
    let n = host::http_request(&http_req_buf()[..n], http_out()).map_err(|_| "network_error")?;
    if extract_json_number(&http_out()[..n], b"status") != Some(b"200") {
        return Err("http_error");
    };
    let n = decode_json_body_into(&http_out()[..n], body_buf()).map_err(|_| "network_error")?;
    if member(&body_buf()[..n], b"code")? != b"0" {
        return Err("source_error");
    };
    Ok(n)
}
fn data(v: &[u8]) -> R<&[u8]> {
    member(v, b"data")
}
fn decoded_url<'a>(row: &[u8], key: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    let n = raw_string(string(row, key)?, b)?;
    let u = &b[..n];
    if !image_url(u) {
        return Err("parse_error");
    };
    Ok(u)
}
fn catalogue() -> R<(usize, usize)> {
    let n = fetch(LIST)?;
    let d = data(&body_buf()[..n])?;
    let total = number(member(d, b"count")?)?;
    if total == 0 || total > 1000 {
        return Err("capacity_exceeded");
    };
    let mut it = JsonArrayIter::from_raw_array(member(d, b"rows")?).ok_or("parse_error")?;
    let mut count = 0;
    while let Some(row) = it.next_object() {
        let id = number(member(row, b"chapters")?)?;
        if id == 0 || count >= 1000 || unsafe { IDS[..count].contains(&id) } {
            return Err("parse_error");
        };
        if number(member(row, b"type")?)? != 1 {
            return Err("unsupported_type");
        };
        if string(row, b"title")?.is_empty() {
            return Err("parse_error");
        };
        let mut b = [0; 2048];
        decoded_url(row, b"thumbnail", &mut b)?;
        unsafe { IDS[count] = id };
        count += 1
    }
    if count != total {
        return Err("incomplete_catalogue");
    };
    Ok((n, count))
}
fn detail(id: usize) -> R<usize> {
    let (_, count) = catalogue()?;
    if !unsafe { IDS[..count].contains(&id) } {
        return Err("not_found");
    };
    let mut u = [0; 1024];
    let mut len = 0;
    w(&mut u, &mut len, DETAIL)?;
    n(&mut u, &mut len, id)?;
    w(&mut u, &mut len, b"&type=1")?;
    let len = fetch(&u[..len])?;
    let d = data(&body_buf()[..len])?;
    let comic = member(d, b"comic")?;
    if number(member(comic, b"chapters")?)? != id
        || number(member(comic, b"id")?)? != id
        || number(member(comic, b"type")?)? != 1
    {
        return Err("wrong_owner");
    };
    if string(comic, b"title")?.is_empty() {
        return Err("parse_error");
    };
    let mut b = [0; 2048];
    decoded_url(comic, b"content", &mut b)?;
    Ok(len)
}
fn book(p: &mut [u8], c: &mut usize) -> R<()> {
    w(p, c, b"{\"id\":")?;
    q(p, c, MID)?;
    w(p, c, b",\"title\":")?;
    q(p, c, "蔚蓝档案四格漫画".as_bytes())?;
    w(p, c, b",\"url\":")?;
    q(p, c, BASE)?;
    w(p, c, b",\"description\":")?;
    q(p, c, "蔚蓝档案国服官网公开四格漫画。".as_bytes())?;
    w(p, c, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(p, c, ICON)?;
    w(p,c,b"},\"language\":\"zh\",\"authors\":[],\"status\":\"unknown\",\"contentRating\":\"unknown\",\"links\":[{\"kind\":\"source\",\"url\":")?;
    q(p, c, BASE)?;
    w(p, c, b"}]}")
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
fn listing() -> R<usize> {
    catalogue()?;
    let p = payload_buf();
    let mut c = 0;
    w(p, &mut c, b"{\"items\":[")?;
    book(p, &mut c)?;
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
            "蔚蓝档案四格漫画 Blue Archive blue archive".as_bytes(),
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
        Ok(1) => result("get_manga_list", listing()),
        Ok(_) => empty("get_manga_list"),
        Err(e) => write_error("get_manga_list", e, ""),
    }
}
fn run_get_manga(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        if extract_json_string(req, b"mangaId") != Some(MID) {
            return Err("not_found");
        };
        catalogue()?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"manga\":")?;
        book(p, &mut c)?;
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
        let (len, _) = catalogue()?;
        let d = data(&body_buf()[..len])?;
        let mut it = JsonArrayIter::from_raw_array(member(d, b"rows")?).ok_or("parse_error")?;
        let p = payload_buf();
        let mut c = 0;
        let mut count = 0;
        w(p, &mut c, b"{\"chapters\":[")?;
        while let Some(row) = it.next_object() {
            let id = number(member(row, b"chapters")?)?;
            if count > 0 {
                w(p, &mut c, b",")?
            };
            w(p, &mut c, b"{\"id\":\"bach:zh:four-panel:")?;
            n(p, &mut c, id)?;
            w(p, &mut c, b"\",\"mangaId\":")?;
            q(p, &mut c, MID)?;
            w(p, &mut c, b",\"title\":")?;
            q(p, &mut c, string(row, b"title")?)?;
            w(p, &mut c, b",\"chapterNumber\":")?;
            n(p, &mut c, id)?;
            w(p, &mut c, b",\"readingGroup\":")?;
            q(p, &mut c, MID)?;
            w(p, &mut c, b",\"continuationEligible\":true")?;
            w(
                p,
                &mut c,
                b",\"volumeNumber\":null,\"language\":\"zh\",\"pageCount\":1}",
            )?;
            count += 1
        }
        w(p, &mut c, b"]}")?;
        Ok(c)
    }
    result("get_chapters", inner(req))
}
fn run_get_pages(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        if let Some(mid) = extract_json_string(req, b"mangaId") {
            if mid != MID {
                return Err("not_found");
            }
        }
        let raw = extract_json_string(req, b"chapterId").ok_or("invalid_request")?;
        let id = number(
            raw.strip_prefix(b"bach:zh:four-panel:")
                .ok_or("invalid_request")?,
        )?;
        if id == 0 {
            return Err("invalid_request");
        };
        let len = detail(id)?;
        let comic = member(data(&body_buf()[..len])?, b"comic")?;
        let mut b = [0; 2048];
        let u = decoded_url(comic, b"content", &mut b)?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"pages\":[{\"id\":\"bapage:zh:four-panel:")?;
        n(p, &mut c, id)?;
        w(
            p,
            &mut c,
            b":0\",\"index\":0,\"image\":{\"kind\":\"url\",\"url\":",
        )?;
        q(p, &mut c, u)?;
        w(p, &mut c, b"}}]}")?;
        Ok(c)
    }
    result("get_pages", inner(req))
}
fn run_get_image_request(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        if let Some(raw) = extract_json_string(req, b"mangaUrl") {
            let mut route = [0; 2048];
            let size = raw_string(raw, &mut route)?;
            if &route[..size] != BASE {
                return Err("invalid_request");
            }
        }
        let raw = extract_json_string(req, b"url").ok_or("invalid_request")?;
        let mut b = [0; 2048];
        let n = raw_string(raw, &mut b)?;
        let u = &b[..n];
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
