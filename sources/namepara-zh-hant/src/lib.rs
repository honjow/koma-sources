#![no_std]
use koma_source_sdk::json_utils::*;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{build_get_request, decode_json_body_into, host};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.beeworks.namepara.zh-hant.koma",
    name: "菇菇官方四格漫畫",
    version: "0.1.0",
    api_version: "0.2",
    language: "zh-Hant",
    author: "Koma",
    description: "Official Traditional Chinese Nameko four-panel comics from namepara.com.",
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

const BASE: &[u8] = b"https://namepara.com/zh/info/comic/";
const MID: &[u8] = b"namepara:zh-hant:four-panel";
const ICON: &[u8] = b"https://namepara.com/assets/img/apple-touch-icon.png";
const ORIGIN: &[u8] = b"https://namepara.com";
static mut IDS: [[u8; 13]; 1000] = [[0; 13]; 1000];
static mut DATES: [[u8; 10]; 1000] = [[0; 10]; 1000];
fn part<'a>(h: &'a [u8], begin: &[u8], finish: &[u8]) -> R<&'a [u8]> {
    let at = find_subslice(h, begin).ok_or("parse_error")? + begin.len();
    let stop = find_subslice(&h[at..], finish).ok_or("parse_error")? + at;
    Ok(&h[at..stop])
}
fn valid_id(id: &[u8]) -> bool {
    id.len() == 13
        && id[6] == b'_'
        && id[..6].iter().all(u8::is_ascii_digit)
        && id[7..].iter().all(u8::is_ascii_digit)
}
fn date_ok(v: &[u8]) -> bool {
    v.len() == 10
        && v[4] == b'.'
        && v[7] == b'.'
        && v.iter()
            .enumerate()
            .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
}
fn image_url(u: &[u8]) -> bool {
    if u == ICON {
        return true;
    }
    let Some(t) = u.strip_prefix(b"https://namepara.com/zh/info/img/") else {
        return false;
    };
    t.strip_suffix(b".png")
        .or_else(|| t.strip_suffix(b".jpg"))
        .map(|x| {
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
    }
    decode_json_body_into(&http_out()[..n], body_buf()).map_err(|_| "network_error")
}
fn indicator(h: &[u8]) -> R<(usize, usize)> {
    let v = part(h, b"<p class=\"e-pagenation-indicator\">", b"</p>")?;
    let at = find_subslice(v, b" / ").ok_or("parse_error")?;
    Ok((number(&v[..at])?, number(&v[at + 3..])?))
}
fn add_catalogue(h: &[u8], count: &mut usize) -> R<()> {
    if !contains_bytes(h, b"<html lang=\"zh\">")
        || part(h, b"<meta property=\"og:url\" content=\"", b"\">")? != BASE
    {
        return Err("wrong_owner");
    }
    if !contains_bytes(h, "<title>菇菇四格漫畫｜".as_bytes()) {
        return Err("wrong_owner");
    }
    let mut pos = 0;
    let mut rawcount = 0;
    let mark = b"<a href=\"/zh/info/comic/";
    while let Some(at) = find_subslice(&h[pos..], mark) {
        let at = pos + at + mark.len();
        let end = find_subslice(&h[at..], b"\"").ok_or("parse_error")? + at;
        let close = find_subslice(&h[end..], b"</a>").ok_or("parse_error")? + end;
        let card = &h[end..close];
        pos = close + 4;
        if !contains_bytes(card, b"class=\"card-top__img\"") {
            continue;
        }
        let id = h[at..end].strip_suffix(b".html").ok_or("invalid_route")?;
        if !valid_id(id) {
            return Err("invalid_route");
        }
        if !contains_bytes(
            card,
            "<p class=\"card-top__info__tag\">菇菇四格漫畫</p>".as_bytes(),
        ) {
            return Err("unsupported_category");
        }
        let date = part(card, b"<p class=\"card-top__info__date\">", b"</p>")?;
        if !date_ok(date) {
            return Err("parse_error");
        }
        rawcount += 1;
        if let Some(old) = unsafe { IDS[..*count].iter().position(|x| x.as_slice() == id) } {
            if unsafe { DATES[old].as_slice() } != date {
                return Err("conflicting_identity");
            }
            continue;
        }
        if *count >= 1000 {
            return Err("capacity_exceeded");
        }
        unsafe {
            IDS[*count].copy_from_slice(id);
            DATES[*count].copy_from_slice(date);
        }
        *count += 1;
    }
    if rawcount == 0 {
        return Err("empty_catalogue");
    }
    Ok(())
}
fn catalogue() -> R<usize> {
    let size = fetch(BASE)?;
    let h = &body_buf()[..size];
    let (first, total) = indicator(h)?;
    if first != 1 || total == 0 || total > 32 {
        return Err("incomplete_catalogue");
    }
    // Each future request must have been explicitly declared by the actual first archive page.
    for page in 2..=total {
        let mut needle = [0; 128];
        let mut n0 = 0;
        w(&mut needle, &mut n0, b"href=\"/zh/info/comic/index_")?;
        n(&mut needle, &mut n0, page)?;
        w(&mut needle, &mut n0, b".html\"")?;
        if !contains_bytes(h, &needle[..n0]) {
            return Err("incomplete_catalogue");
        }
    }
    let mut count = 0;
    add_catalogue(h, &mut count)?;
    for page in 2..=total {
        let mut u = [0; 256];
        let mut c = 0;
        w(&mut u, &mut c, BASE)?;
        w(&mut u, &mut c, b"index_")?;
        n(&mut u, &mut c, page)?;
        w(&mut u, &mut c, b".html")?;
        let size = fetch(&u[..c])?;
        let h = &body_buf()[..size];
        if indicator(h)? != (page, total) {
            return Err("incomplete_catalogue");
        }
        add_catalogue(h, &mut count)?;
    }
    Ok(count)
}
fn book(p: &mut [u8], c: &mut usize) -> R<()> {
    w(p, c, b"{\"id\":")?;
    q(p, c, MID)?;
    w(p, c, b",\"title\":")?;
    q(p, c, "菇菇四格漫畫".as_bytes())?;
    w(p, c, b",\"url\":")?;
    q(p, c, BASE)?;
    w(p, c, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(p, c, ICON)?;
    w(p,c,b"},\"language\":\"zh-Hant\",\"authors\":[],\"status\":\"unknown\",\"contentRating\":\"unknown\",\"description\":")?;
    q(
        p,
        c,
        "菇菇官方網站的繁中四格漫畫，依網站實際篇目順序列出；章號未知。".as_bytes(),
    )?;
    w(p, c, b",\"links\":[{\"kind\":\"source\",\"url\":")?;
    q(p, c, BASE)?;
    w(p, c, b"}]}")
}
fn result(op: &str, r: R<usize>) -> u32 {
    match r {
        Ok(n) => write_success_payload(op, n),
        Err(e) => write_error(
            op,
            e,
            "Official public comic contract unavailable or invalid",
        ),
    }
}
fn empty(op: &str) -> u32 {
    let b = b"{\"items\":[],\"page\":{\"hasMore\":false}}";
    payload_buf()[..b.len()].copy_from_slice(b);
    write_success_payload(op, b.len())
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
    let n0 = match raw_string(raw, &mut b) {
        Ok(n) => n,
        Err(e) => return write_error("search", e, ""),
    };
    if n0 > 0
        && !contains_bytes(
            "菇菇 四格漫畫 方吉 namepara Nameko nameko".as_bytes(),
            &b[..n0],
        )
    {
        return empty("search");
    }
    match page(req) {
        Ok(1) => result("search", listing()),
        Ok(_) => empty("search"),
        Err(e) => write_error("search", e, ""),
    }
}
fn run_get_manga_list(req: &[u8]) -> u32 {
    if extract_json_string(req, b"listingId") != Some(b"listing:official") {
        return write_error("get_manga_list", "invalid_request", "");
    }
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
        }
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
        }
        let count = catalogue()?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"chapters\":[")?;
        for i in 0..count {
            if i > 0 {
                w(p, &mut c, b",")?;
            }
            w(p, &mut c, b"{\"id\":\"nameparach:zh-hant:")?;
            w(p, &mut c, unsafe { &IDS[i] })?;
            w(p, &mut c, b"\",\"mangaId\":")?;
            q(p, &mut c, MID)?;
            w(p, &mut c, b",\"title\":")?;
            q(p, &mut c, unsafe { &DATES[i] })?;
            w(
                p,
                &mut c,
                b",\"chapterNumber\":null,\"volumeNumber\":null,\"readingGroup\":",
            )?;
            q(p, &mut c, MID)?;
            w(
                p,
                &mut c,
                b",\"continuationEligible\":false,\"language\":\"zh-Hant\",\"pageCount\":null}",
            )?;
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
        let id = extract_json_string(req, b"chapterId")
            .ok_or("invalid_request")?
            .strip_prefix(b"nameparach:zh-hant:")
            .ok_or("invalid_request")?;
        if !valid_id(id) {
            return Err("invalid_request");
        }
        let count = catalogue()?;
        if !unsafe { IDS[..count].iter().any(|x| x.as_slice() == id) } {
            return Err("not_found");
        }
        let mut url = [0; 256];
        let mut len = 0;
        w(&mut url, &mut len, BASE)?;
        w(&mut url, &mut len, id)?;
        w(&mut url, &mut len, b".html")?;
        let size = fetch(&url[..len])?;
        let h = &body_buf()[..size];
        if !contains_bytes(h, b"<body id=\"body-article-comic\">")
            || part(h, b"<meta property=\"og:url\" content=\"", b"\">")? != &url[..len]
        {
            return Err("wrong_owner");
        }
        let body = part(
            h,
            b"<div id=\"js-article-content\" class=\"p-article-content comic\">",
            b"</div>",
        )?;
        let p = payload_buf();
        let mut c = 0;
        let mut pos = 0;
        let mut count = 0;
        w(p, &mut c, b"{\"pages\":[")?;
        while let Some(at) = find_subslice(&body[pos..], b"<img src=\"") {
            let at = pos + at + b"<img src=\"".len();
            let end = find_subslice(&body[at..], b"\"").ok_or("parse_error")? + at;
            pos = end + 1;
            let src = &body[at..end];
            let mut u = [0; 1024];
            let mut size = 0;
            if src.starts_with(b"/zh/info/img/") {
                w(&mut u, &mut size, ORIGIN)?;
            }
            w(&mut u, &mut size, src)?;
            if !image_url(&u[..size]) || &u[..size] == ICON {
                return Err("invalid_image_uri");
            }
            if count >= 64 {
                return Err("capacity_exceeded");
            }
            if count > 0 {
                w(p, &mut c, b",")?;
            }
            w(p, &mut c, b"{\"id\":\"nameparapage:zh-hant:")?;
            w(p, &mut c, id)?;
            w(p, &mut c, b":")?;
            n(p, &mut c, count)?;
            w(p, &mut c, b"\",\"index\":")?;
            n(p, &mut c, count)?;
            w(p, &mut c, b",\"image\":{\"kind\":\"url\",\"url\":")?;
            q(p, &mut c, &u[..size])?;
            w(p, &mut c, b"}}")?;
            count += 1;
        }
        if count == 0 {
            return Err("empty_pages");
        }
        w(p, &mut c, b"]}")?;
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
        let mut u = [0; 2048];
        let size = raw_string(raw, &mut u)?;
        if !image_url(&u[..size]) {
            return Err("invalid_request");
        }
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"imageRequest\":{\"url\":")?;
        q(p, &mut c, &u[..size])?;
        w(p, &mut c, b",\"headers\":{\"Referer\":")?;
        q(p, &mut c, BASE)?;
        w(p, &mut c, b"}}}")?;
        Ok(c)
    }
    result("get_image_request", inner(req))
}
fn run_get_listings(_req: &[u8]) -> u32 {
    let b = "{\"listings\":[{\"id\":\"listing:official\",\"name\":\"官方四格漫畫\"}]}".as_bytes();
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
