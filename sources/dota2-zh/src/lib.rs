#![no_std]
use koma_source_sdk::json_utils::*;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{build_get_request, decode_json_body_into, host};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.perfectworld.dota2.zh.koma",
    name: "DOTA2官方漫画",
    version: "0.1.1",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "Public Chinese Are We Heroes Yet? comic from the official DOTA2 China website.",
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

const BASE: &[u8] = b"https://www.dota2.com.cn/comics/";
const VIEW: &[u8] = b"https://www.dota2.com.cn/comics/comics.htm?=0";
const SCRIPT: &[u8] = b"https://www.dota2.com.cn/js/cartoon.js";
const MID: &[u8] = b"dota2:zh:are-we-heroes-yet";
const CID: &[u8] = b"dota2read:zh:are-we-heroes-yet";
const TITLE: &str = "我们是英雄了吗  Are We Heroes Yet? - 秘密店铺之传说";
const COVER: &[u8] = b"https://www.dota2.com.cn/images/cartoon/img1.jpg";
// Original URI bytes from published cartoonJson.cartoonJson1, in its exact order.
// This is a fixed official-contract allowlist, not cross-operation guest state.
const APPROVED_PATHS: [&[u8]; 48] = [
    b"/images/cartoon/part_1/page_01_cn.jpg",
    b"/images/cartoon/part_1/page_02_cn.jpg",
    b"/images/cartoon/part_1/page_03_cn.jpg",
    b"/images/cartoon/part_1/page_04_cn.jpg",
    b"/images/cartoon/part_1/page_05_cn.jpg",
    b"/images/cartoon/part_1/page_06_cn.jpg",
    b"/images/cartoon/part_1/page_07_cn.jpg",
    b"/images/cartoon/part_1/page_08_cn.jpg",
    b"/images/cartoon/part_1/page_09_cn.jpg",
    b"/images/cartoon/part_1/page_10_cn.jpg",
    b"/images/cartoon/part_1/page_11_cn.jpg",
    b"/images/cartoon/part_1/page_12_cn.jpg",
    b"/images/cartoon/part_1/page_13_cn.jpg",
    b"/images/cartoon/part_1/page_14_cn.jpg",
    b"/images/cartoon/part_1/page_15_cn.jpg",
    b"/images/cartoon/part_1/page_16_cn.jpg",
    b"/images/cartoon/part_1/page_17_cn.jpg",
    b"/images/cartoon/part_1/page_18_cn.jpg",
    b"/images/cartoon/part_1/page_19_cn.jpg",
    b"/images/cartoon/part_1/page_20_cn.jpg",
    b"/images/cartoon/part_1/page_21_cn.jpg",
    b"/images/cartoon/part_1/page_22_cn.jpg",
    b"/images/cartoon/part_1/page_23_cn.jpg",
    b"/images/cartoon/part_1/page_24_cn.jpg",
    b"/images/cartoon/part_1/page_25_cn.jpg",
    b"/images/cartoon/part_1/page_26_cn.jpg",
    b"/images/cartoon/part_1/page_27_cn.jpg",
    b"/images/cartoon/part_1/page_28_cn.jpg",
    b"/images/cartoon/part_1/page_29_cn.jpg",
    b"/images/cartoon/part_1/page_30_cn.jpg",
    b"/images/cartoon/part_1/page_31_cn.jpg",
    b"/images/cartoon/part_1/page_32_cn.jpg",
    b"/images/cartoon/part_1/page_33_cn.jpg",
    b"/images/cartoon/part_1/page_34_cn.jpg",
    b"/images/cartoon/part_1/page_35_cn.jpg",
    b"/images/cartoon/part_1/page_36_cn.jpg",
    b"/images/cartoon/part_1/page_37_cn.jpg",
    b"/images/cartoon/part_1/page_38_cn.jpg",
    b"/images/cartoon/part_1/page_39_cn.jpg",
    b"/images/cartoon/part_1/page_40_cn.jpg",
    b"/images/cartoon/part_1/page_41_cn.jpg",
    b"/images/cartoon/part_1/page_42_cn.jpg",
    b"/images/cartoon/part_1/page_43_cn.jpg",
    b"/images/cartoon/part_1/page_44_cn.jpg",
    b"/images/cartoon/part_1/page_45_cn.jpg",
    b"/images/cartoon/part_1/page_46_cn.jpg",
    b"/images/cartoon/part_1/page_47_cn.jpg",
    b"/images/cartoon/part_1/page_48_cn.jpg",
];
static mut PATHS: [[u8; 128]; 256] = [[0; 128]; 256];
static mut LENS: [usize; 256] = [0; 256];
fn fetch(u: &[u8]) -> R<usize> {
    let n = build_get_request(http_req_buf(), u, Some(BASE), &[]).ok_or("capacity_exceeded")?;
    let n = host::http_request(&http_req_buf()[..n], http_out()).map_err(|_| "network_error")?;
    if extract_json_number(&http_out()[..n], b"status") != Some(b"200") {
        return Err("http_error");
    }
    decode_json_body_into(&http_out()[..n], body_buf()).map_err(|_| "network_error")
}
fn catalogue() -> R<()> {
    let size = fetch(BASE)?;
    let h = &body_buf()[..size];
    if !contains_bytes(h, TITLE.as_bytes()) || !contains_bytes(h, b"<div id=\"main\">") {
        return Err("wrong_owner");
    }
    // Published chapter labels identify six entry points into one global viewer.
    // Their offsets are not interpreted as chapter end boundaries.
    let mut start = 0;
    for (label, link) in [
        ("官方漫画 第1章", "comics.htm?=0"),
        ("官方漫画 第2章", "comics.htm?=8"),
        ("官方漫画 第3章", "comics.htm?=16"),
        ("官方漫画 第4章", "comics.htm?=24"),
        ("官方漫画 第5章", "comics.htm?=32"),
        ("官方漫画 第6章", "comics.htm?=38"),
    ] {
        let at =
            find_subslice(&h[start..], label.as_bytes()).ok_or("incomplete_catalogue")? + start;
        let end = find_subslice(&h[at..], b"</li>").ok_or("parse_error")? + at;
        let mut pat = [0; 64];
        let mut c = 0;
        w(&mut pat, &mut c, b"href=\"")?;
        w(&mut pat, &mut c, link.as_bytes())?;
        w(&mut pat, &mut c, b"\"")?;
        if !contains_bytes(&h[at..end], &pat[..c]) {
            return Err("unsupported_selector");
        }
        start = end + 5;
    }
    if !contains_bytes(h, b"src=\"/images/cartoon/img1.jpg\"") {
        return Err("incomplete_catalogue");
    }
    Ok(())
}
fn page_path(u: &[u8]) -> bool {
    let Some(t) = u.strip_prefix(b"/images/cartoon/part_1/page_") else {
        return false;
    };
    let Some(t) = t.strip_suffix(b"_cn.jpg") else {
        return false;
    };
    if t.len() != 2 || !t.iter().all(u8::is_ascii_digit) {
        return false;
    }
    matches!(number(t), Ok(1..=48))
}
fn pages_contract() -> R<usize> {
    catalogue()?;
    let size = fetch(VIEW)?;
    let h = &body_buf()[..size];
    if !contains_bytes(h, TITLE.as_bytes())
        || !contains_bytes(h, b"id=\"cartoonImg\"")
        || !contains_bytes(h, b"src=\"/js/cartoon.js\"")
    {
        return Err("wrong_owner");
    }
    let size = fetch(SCRIPT)?;
    let js = &body_buf()[..size];
    // Check the actual published controller; no JS evaluation or unused arrays.
    for token in [
        b"$(\"#cartoonImg\").attr(\"src\",cartoonJson.cartoonJson1[n]);".as_slice(),
        b"cartoonImg(i);",
        b"cartoonJson.cartoonJson1.length",
        b"href.indexOf(\"=\")",
    ] {
        if !contains_bytes(js, token) {
            return Err("unsupported_selector");
        }
    }
    let marker = b"'cartoonJson1':[";
    let at = find_subslice(js, marker).ok_or("parse_error")? + marker.len();
    let end = find_subslice(&js[at..], b"]").ok_or("parse_error")? + at;
    let a = &js[at..end];
    let mut pos = 0;
    let mut count = 0;
    while pos < a.len() {
        while pos < a.len() && a[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if a.get(pos) != Some(&b'\'') {
            return Err("parse_error");
        }
        pos += 1;
        let end = a[pos..]
            .iter()
            .position(|b| *b == b'\'')
            .ok_or("parse_error")?
            + pos;
        let path = &a[pos..end];
        if count >= 256 || !page_path(path) || path.len() > 128 {
            return Err("incomplete_pages");
        }
        for j in 0..count {
            if unsafe { &PATHS[j][..LENS[j]] } == path {
                return Err("incomplete_pages");
            }
        }
        unsafe {
            PATHS[count][..path.len()].copy_from_slice(path);
            LENS[count] = path.len();
        }
        count += 1;
        pos = end + 1;
        while pos < a.len() && a[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos < a.len() {
            if a[pos] != b',' {
                return Err("parse_error");
            }
            pos += 1;
        }
    }
    if count != APPROVED_PATHS.len() {
        return Err("incomplete_pages");
    }
    // Fail closed on any upstream replacement or reorder; never emit stale pages.
    for (i, approved) in APPROVED_PATHS.iter().enumerate() {
        if unsafe { &PATHS[i][..LENS[i]] } != *approved {
            return Err("official_contract_changed");
        }
    }
    Ok(count)
}
fn book(p: &mut [u8], c: &mut usize, detail: bool) -> R<()> {
    w(p, c, b"{\"id\":")?;
    q(p, c, MID)?;
    w(p, c, b",\"title\":")?;
    q(p, c, TITLE.as_bytes())?;
    w(p, c, b",\"url\":")?;
    q(p, c, BASE)?;
    w(p, c, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(p, c, COVER)?;
    w(p,c,b"},\"language\":\"zh\",\"authors\":[],\"status\":\"unknown\",\"contentRating\":\"unknown\"")?;
    if detail {
        w(p, c, b",\"description\":")?;
        q(p,c,"DOTA2 国服官网公开的《秘密店铺之传说—我们是英雄了吗？》中文漫画。官网六个章链接定位同一全书阅读器，本源保留完整公开页数组的原顺序。".as_bytes())?;
        w(p, c, b",\"links\":[{\"kind\":\"source\",\"url\":")?;
        q(p, c, BASE)?;
        w(p, c, b"}]")?;
    }
    w(p, c, b"}")
}
fn listing() -> R<usize> {
    catalogue()?;
    let p = payload_buf();
    let mut c = 0;
    w(p, &mut c, b"{\"items\":[")?;
    book(p, &mut c, false)?;
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
    if len > 0
        && !contains_bytes(
            "DOTA2 dota2 刀塔 秘密店铺之传说 我们是英雄了吗 Are We Heroes Yet?".as_bytes(),
            &b[..len],
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
        book(p, &mut c, true)?;
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
        let count = pages_contract()?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"chapters\":[{\"id\":")?;
        q(p, &mut c, CID)?;
        w(p, &mut c, b",\"mangaId\":")?;
        q(p, &mut c, MID)?;
        w(p, &mut c, b",\"title\":")?;
        q(p, &mut c, TITLE.as_bytes())?;
        w(p, &mut c, b",\"url\":")?;
        q(p, &mut c, VIEW)?;
        w(
            p,
            &mut c,
            b",\"chapterNumber\":null,\"volumeNumber\":null,\"readingGroup\":",
        )?;
        q(p, &mut c, MID)?;
        w(
            p,
            &mut c,
            b",\"continuationEligible\":false,\"language\":\"zh\",\"pageCount\":",
        )?;
        n(p, &mut c, count)?;
        w(p, &mut c, b"}]}")?;
        Ok(c)
    }
    result("get_chapters", inner(req))
}
fn run_get_pages(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        if extract_json_string(req, b"chapterId") != Some(CID) {
            return Err("not_found");
        }
        if let Some(mid) = extract_json_string(req, b"mangaId") {
            if mid != MID {
                return Err("not_found");
            }
        }
        let count = pages_contract()?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"pages\":[")?;
        for i in 0..count {
            if i > 0 {
                w(p, &mut c, b",")?;
            }
            w(p, &mut c, b"{\"id\":\"dota2page:zh:are-we-heroes-yet:")?;
            n(p, &mut c, i)?;
            w(p, &mut c, b"\",\"index\":")?;
            n(p, &mut c, i)?;
            w(
                p,
                &mut c,
                b",\"image\":{\"kind\":\"url\",\"url\":\"https://www.dota2.com.cn",
            )?;
            w(p, &mut c, unsafe { &PATHS[i][..LENS[i]] })?;
            w(p, &mut c, b"\"}}")?;
        }
        w(p, &mut c, b"]}")?;
        Ok(c)
    }
    result("get_pages", inner(req))
}
fn run_get_image_request(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        if let Some(raw) = extract_json_string(req, b"mangaUrl") {
            let mut b = [0; 2048];
            let len = raw_string(raw, &mut b)?;
            if &b[..len] != BASE {
                return Err("invalid_request");
            }
        }
        let raw = extract_json_string(req, b"url").ok_or("invalid_request")?;
        let mut b = [0; 2048];
        let len = raw_string(raw, &mut b)?;
        let u = &b[..len];
        let path = u
            .strip_prefix(b"https://www.dota2.com.cn")
            .ok_or("invalid_request")?;
        let pid = extract_json_string(req, b"pageId");
        if u == COVER {
            // Koma uses the remote manga ID as the cover image identity.
            // It must never be accepted as a body page identity.
            if let Some(id) = pid {
                if id != MID {
                    return Err("invalid_request");
                }
            }
        } else {
            if !page_path(path) {
                return Err("invalid_request");
            }
            let index = APPROVED_PATHS
                .iter()
                .position(|approved| *approved == path)
                .ok_or("invalid_request")?;
            if let Some(id) = pid {
                let raw = id
                    .strip_prefix(b"dota2page:zh:are-we-heroes-yet:")
                    .ok_or("invalid_request")?;
                if raw.is_empty() || (raw.len() > 1 && raw[0] == b'0') || number(raw)? != index {
                    return Err("invalid_request");
                }
            }
        }
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"imageRequest\":{\"url\":")?;
        q(p, &mut c, u)?;
        w(p, &mut c, b",\"headers\":{\"Referer\":")?;
        q(p, &mut c, VIEW)?;
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
