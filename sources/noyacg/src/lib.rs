#![no_std]

extern crate koma_source_sdk;

use koma_source_sdk::host::{self, http_request, log_info};
use koma_source_sdk::json_utils::{
    append_json_escaped, append_json_unescaped_then_escaped, contains_bytes, extract_json_number,
    extract_json_string, find_subslice, json_object_member, write_bytes, write_url_encoded, write_usize,
};
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::chapters::{Chapter, ChapterNumber, ChapterPage, ChapterPlacement, ChapterResponse, ChapterResult, GroupedChapters};
use koma_source_sdk::source_protocol as noy_json;
use koma_source_sdk::plain_text::decode_json_content;
use koma_source_sdk::{FetchError, fetch_error_code};

const BASE_URL: &[u8] = b"https://noymanga.com";
const IMG_BASE: &[u8] = b"https://img.noymanga.com/";
koma_source_sdk::koma_source_buffers! {
    payload: 1024 * 1024,
    http_out: 2 * 1024 * 1024,
    body: 2 * 1024 * 1024,
    http_req: 4096,
    scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

// The request query must not alias URL/form scratch buffers or consume the bounded WASM stack.
static mut SEARCH_QUERY: [u8; 8192] = [0; 8192];

// Opt-in local candidate only; default builds keep the published filter surface.
#[cfg(feature = "noyacg_filter_candidate")]
#[path = "../filter_candidate.rs"]
mod filter_candidate;

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.noyacg.koma",
    name: "NoyAcg",
    version: "0.1.15",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "NoyAcg 是漫画与同人志社区，书目分全年龄、成人及综合范围。受保护内容需要在原站登录，是否可读依网站与账号状态而定。",
    content_rating: "nsfw",
};

const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: true,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: true,
    manga_list: true,
    home: false,
    filters: cfg!(feature = "noyacg_filter_candidate"),
    settings: true,
    image_request: true,
    credentials: false,
};

fn body_slice(len: usize) -> &'static [u8] {
    unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(BODY_BUF).cast::<u8>(), len) }
}
fn scratch_a_slice(len: usize) -> &'static [u8] {
    unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(SCRATCH_A).cast::<u8>(), len) }
}
fn scratch_b_slice(len: usize) -> &'static [u8] {
    unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(SCRATCH_B).cast::<u8>(), len) }
}

// Fixed reference values only; absent or malformed preferences retain both.
fn adult(req:&[u8])->&'static [u8] {
    match json_object_member(req,b"settings").and_then(|s|extract_json_string(s,b"ADULT")) {
        Some(b"false")=>b"false", Some(b"true")=>b"true", _=>b"both",
    }
}
fn build_headers(dst: &mut [u8], c: &mut usize, form: bool, adult: &[u8]) -> bool {
    if !(write_bytes(dst,c,br#"","headers":{"Accept":"application/json, text/plain, */*","User-Agent":"Mozilla/5.0 (Linux; Android 14) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Mobile Safari/537.36","referer":"https://noymanga.com/","allow-adult":""#)
        && write_bytes(dst,c,adult) && write_bytes(dst,c,b"\"")) {return false}
    if form && !write_bytes(dst,c,br#", "Content-Type":"application/x-www-form-urlencoded""#) {return false}
    write_bytes(dst,c,b"}")
}
fn build_post_form_request(dst: &mut [u8], url: &[u8], body: &[u8], adult: &[u8]) -> Option<usize> {
    let mut c = 0usize;
    write_bytes(dst, &mut c, br#"{"version":1,"method":"POST","url":""#).then_some(())?;
    append_json_escaped(dst, &mut c, url).then_some(())?;
    build_headers(dst, &mut c, true, adult).then_some(())?;
    write_bytes(dst, &mut c, br#", "bodyBase64":""#).then_some(())?;
    append_json_escaped(dst, &mut c, body).then_some(())?;
    write_bytes(dst,&mut c,br#"","timeoutMs":15000,"responseKind":"bodyText"}"#).then_some(())?;
    Some(c)
}

fn fetch_with_request(req_len: usize) -> Result<usize, FetchError> {
    let mut resp_len = 0usize;
    let mut transport_failed = true;
    for attempt in 0..3u8 {
        match http_request(&http_req_buf()[..req_len], http_out()) {
            Ok(n) => {
                resp_len = n;
                transport_failed = false;
                break;
            }
            Err(_) => {
                if attempt < 2 {
                    log_info(b"noyacg: http transport error, retrying");
                }
            }
        }
    }
    if transport_failed {
        return Err(FetchError::Network);
    }
    decode_json_body(&http_out()[..resp_len])
}

fn fetch_post_form(url: &[u8], body: &[u8], req: &[u8]) -> Result<usize, FetchError> {
    let req_len = build_post_form_request(http_req_buf(), url, body, adult(req)).ok_or(FetchError::Network)?;
    fetch_with_request(req_len)
}

fn parse_usize(bytes: &[u8], default: usize) -> usize {
    if bytes.is_empty() {
        return default;
    }
    let mut n = 0usize;
    for &b in bytes {
        if b < b'0' || b > b'9' {
            return default;
        }
        n = n * 10 + (b - b'0') as usize;
    }
    if n == 0 {
        default
    } else {
        n
    }
}

fn request_page(req: &[u8]) -> usize {
    if let Some(n) = extract_json_number(req, b"page") {
        return parse_usize(n, 1);
    }
    if let Some(c) = extract_json_string(req, b"cursor") {
        return parse_usize(c, 1);
    }
    1
}

fn extract_balanced(data: &[u8], start: usize, open: u8, close: u8) -> Option<&[u8]> {
    let mut pos = start;
    let mut depth = 0i32;
    let mut in_string = false;
    while pos < data.len() {
        let b = data[pos];
        if in_string {
            if b == b'\\' {
                pos += 1;
            } else if b == b'"' {
                in_string = false;
            }
        } else if b == b'"' {
            in_string = true;
        } else if b == open {
            depth += 1;
        } else if b == close {
            depth -= 1;
            if depth == 0 {
                return Some(&data[start..pos + 1]);
            }
        }
        pos += 1;
    }
    None
}

fn extract_value_for_key<'a>(data: &'a [u8], key: &[u8], want: u8) -> Option<&'a [u8]> {
    let mut pattern = [0u8; 64];
    let needed = key.len() + 3;
    if needed > pattern.len() {
        return None;
    }
    pattern[0] = b'"';
    pattern[1..1 + key.len()].copy_from_slice(key);
    pattern[1 + key.len()] = b'"';
    pattern[2 + key.len()] = b':';
    let mut pos = find_subslice(data, &pattern[..needed])? + needed;
    while pos < data.len() && matches!(data[pos], b' ' | b'\t' | b'\n' | b'\r') {
        pos += 1;
    }
    if pos >= data.len() || data[pos] != want {
        return None;
    }
    if want == b'{' {
        extract_balanced(data, pos, b'{', b'}')
    } else {
        extract_balanced(data, pos, b'[', b']').map(|v| &v[1..v.len() - 1])
    }
}

fn extract_array_for_key<'a>(data: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    extract_value_for_key(data, key, b'[')
}

fn next_raw_object<'a>(data: &'a [u8], pos: &mut usize) -> Option<&'a [u8]> {
    while *pos < data.len() {
        match data[*pos] {
            b' ' | b'\t' | b'\n' | b'\r' | b',' => *pos += 1,
            b']' => return None,
            b'{' => break,
            _ => {
                *pos += 1;
            }
        }
    }
    if *pos >= data.len() {
        return None;
    }
    let obj = extract_balanced(data, *pos, b'{', b'}')?;
    *pos += obj.len();
    Some(obj)
}

fn id_from_manga(obj: &[u8]) -> Option<&[u8]> {
    extract_json_number(obj, b"Bid")
        .or_else(|| extract_json_number(obj, b"id"))
        .or_else(|| extract_json_string(obj, b"Bid"))
        .or_else(|| extract_json_string(obj, b"id"))
}

fn title_from_manga<'a>(obj: &'a [u8], id: &'a [u8]) -> &'a [u8] {
    extract_json_string(obj, b"Bookname")
        .or_else(|| extract_json_string(obj, b"name"))
        .unwrap_or(id)
}

fn desc_from_manga(obj: &[u8]) -> &[u8] {
    extract_json_string(obj, b"Description")
        .or_else(|| extract_json_string(obj, b"description"))
        .unwrap_or(b"")
}

fn author_from_manga(obj: &[u8]) -> &[u8] {
    extract_json_string(obj, b"Author")
        .or_else(|| extract_json_string(obj, b"author"))
        .unwrap_or(b"")
}

fn write_cover(payload: &mut [u8], c: &mut usize, id: &[u8]) -> bool {
    write_bytes(payload, c, br#"{"kind":"url","url":""#)
        && write_bytes(payload, c, IMG_BASE)
        && append_json_escaped(payload, c, id)
        && write_bytes(payload, c, br#"/m1.webp"}"#)
}

fn write_json_string_value(payload: &mut [u8], c: &mut usize, bytes: &[u8]) -> bool {
    write_bytes(payload, c, b"\"")
        && append_json_unescaped_then_escaped(payload, c, bytes)
        && write_bytes(payload, c, b"\"")
}

fn write_tags_from_array(payload: &mut [u8], c: &mut usize, arr: &[u8], written: &mut usize) -> bool {
    let mut pos = 0usize;
    while pos < arr.len() {
        while pos < arr.len() && matches!(arr[pos], b' ' | b'\t' | b'\n' | b'\r' | b',') {
            pos += 1;
        }
        if pos >= arr.len() || arr[pos] != b'"' {
            break;
        }
        pos += 1;
        let start = pos;
        while pos < arr.len() {
            if arr[pos] == b'\\' {
                pos += 2;
                continue;
            }
            if arr[pos] == b'"' {
                break;
            }
            pos += 1;
        }
        if pos <= arr.len() {
            if *written > 0 && !write_bytes(payload, c, b",") {
                return false;
            }
            if !write_json_string_value(payload, c, &arr[start..pos]) {
                return false;
            }
            *written += 1;
        }
        pos += 1;
        if *written >= 32 {
            break;
        }
    }
    true
}

// The legacy DTO joins several names in one space-delimited string; v4
// arrays already contain individual names. Preserve spaces inside array items.
fn write_tags_from_string(payload: &mut [u8], c: &mut usize, value: &[u8], written: &mut usize) -> bool {
    let mut start = 0usize;
    let mut pos = 0usize;
    while pos <= value.len() {
        let escaped_space = value.get(pos..pos.saturating_add(6)) == Some(b"\\u0020");
        let separator = pos == value.len() || value.get(pos) == Some(&b' ') || escaped_space;
        if separator {
            if start < pos {
                if *written > 0 && !write_bytes(payload, c, b",") { return false; }
                if !write_json_string_value(payload, c, &value[start..pos]) { return false; }
                *written += 1;
            }
            if pos == value.len() { break; }
            pos += if escaped_space { 6 } else { 1 };
            start = pos;
        } else if value[pos] == b'\\' && pos + 1 < value.len() {
            // A literal backslash must not turn the following u0020 into a separator.
            pos += 2;
        } else {
            pos += 1;
        }
    }
    true
}

fn write_tags(payload: &mut [u8], c: &mut usize, obj: &[u8]) -> bool {
    let mut written = 0usize;
    for key in [b"tags" as &[u8], b"pname", b"otag", b"Otag", b"ptag", b"Ptag"] {
        if let Some(arr) = extract_array_for_key(obj, key) {
            if !write_tags_from_array(payload, c, arr, &mut written) {
                return false;
            }
        } else if let Some(s) = extract_json_string(obj, key) {
            if !write_tags_from_string(payload, c, s, &mut written) {
                return false;
            }
        }
    }
    true
}

fn write_manga_card(payload: &mut [u8], c: &mut usize, obj: &[u8], source_tag: &[u8]) -> bool {
    let id = match id_from_manga(obj) {
        Some(v) if !v.is_empty() => v,
        _ => return true,
    };
    let title = title_from_manga(obj, id);
    let author = author_from_manga(obj);
    write_bytes(payload, c, br#"{"id":"manga:"#)
        && append_json_escaped(payload, c, id)
        && write_bytes(payload, c, br#"","title":""#)
        && append_json_unescaped_then_escaped(payload, c, title)
        && write_bytes(payload, c, br#"","cover":"#)
        && write_cover(payload, c, id)
        && write_bytes(payload, c, br#","authors":["#)
        && if author.is_empty() {
            true
        } else {
            write_json_string_value(payload, c, author)
        }
        && write_bytes(payload, c, br#"],"status":"unknown","contentRating":"nsfw","description":""#)
        && append_json_unescaped_then_escaped(payload, c, desc_from_manga(obj))
        && write_bytes(payload, c, br#"","sourceTags":["#)
        && write_json_string_value(payload, c, source_tag)
        && write_bytes(payload, c, br#"]}"#)
}

fn write_manga_list_response(operation: &str, json: &[u8], array_key: &[u8], page: usize, source_tag: &[u8]) -> u32 {
    if contains_bytes(json, br#""status":"login""#) {
        return write_error(operation, "auth_required", "site returned login status");
    }
    let items = match extract_array_for_key(json, array_key)
        .or_else(|| extract_array_for_key(json, b"data"))
        .or_else(|| extract_array_for_key(json, b"info"))
        .or_else(|| extract_array_for_key(json, b"list"))
    {
        Some(v) => v,
        None => return write_error(operation, "parse_error", "missing manga list"),
    };
    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error(operation, "internal_error", "payload overflow");
    }
    let mut pos = 0usize;
    let mut written = 0usize;
    while let Some(obj) = next_raw_object(items, &mut pos) {
        if written >= 100 {
            break;
        }
        if written > 0 && !write_bytes(payload, &mut c, b",") {
            break;
        }
        let before = c;
        if !write_manga_card(payload, &mut c, obj, source_tag) {
            return write_error(operation, "internal_error", "payload overflow");
        }
        if c == before {
            if written > 0 {
                c -= 1;
            }
            continue;
        }
        written += 1;
    }
    let total = extract_json_number(json, b"count")
        .or_else(|| extract_json_number(json, b"len"))
        .map(|v| parse_usize(v, 0))
        .unwrap_or(0);
    let has_more = if total > 0 {
        page * written < total
    } else {
        written > 0
    };
    let ok = write_bytes(payload, &mut c, br#"],"page":{"nextCursor":"#)
        && if has_more {
            write_bytes(payload, &mut c, b"\"")
                && write_usize(payload, &mut c, page + 1)
                && write_bytes(payload, &mut c, b"\"")
        } else {
            write_bytes(payload, &mut c, b"null")
        }
        && write_bytes(payload, &mut c, br#","hasMore":"#)
        && write_bytes(payload, &mut c, if has_more { b"true" } else { b"false" })
        && write_bytes(payload, &mut c, b"}}");
    if !ok {
        return write_error(operation, "internal_error", "payload overflow");
    }
    write_success_payload(operation, c)
}

fn build_url(path: &[u8]) -> Option<usize> {
    let buf = scratch_a();
    let mut c = 0usize;
    write_bytes(buf, &mut c, BASE_URL).then_some(())?;
    write_bytes(buf, &mut c, path).then_some(())?;
    Some(c)
}

fn run_search(req: &[u8]) -> u32 {
    // JSON escapes belong to the host request, not the search term sent to the provider.
    let args = json_object_member(req, b"args").unwrap_or(req);
    let raw = match json_object_member(args, b"query") {
        None => b"".as_slice(),
        Some(v) if v.len() >= 2 && v[0] == b'"' && v[v.len() - 1] == b'"' => &v[1..v.len() - 1],
        Some(_) => return write_error("search", "invalid_request", "invalid query"),
    };
    let out = unsafe { &mut *core::ptr::addr_of_mut!(SEARCH_QUERY) };
    let query = match koma_source_sdk::plain_text::decode_json_content(raw, out) {
        Ok(v) if core::str::from_utf8(v).is_ok() => v,
        _ => return write_error("search", "invalid_request", "invalid query"),
    };
    let page = request_page(req);
    let mode = extract_json_string(req, b"mode").unwrap_or(b"default");
    #[cfg(feature = "noyacg_filter_candidate")]
    let (mode, sort, finished) = match filter_candidate::selected(req, mode) { Some(v) => v, None => return write_error("search", "invalid_request", "invalid search filter") };
    #[cfg(not(feature = "noyacg_filter_candidate"))]
    let (sort, finished): (&[u8], &[u8]) = (b"", b"");
    let body = scratch_b();
    let mut bc = 0usize;
    if !(write_bytes(body, &mut bc, b"value=")
        && write_url_encoded(body, &mut bc, query)
        && write_bytes(body, &mut bc, b"&page=")
        && write_usize(body, &mut bc, page)
        && write_bytes(body, &mut bc, b"&type=book&mode=")
        && write_url_encoded(body, &mut bc, mode)
        && write_bytes(body, &mut bc, b"&sort=")
        && write_url_encoded(body, &mut bc, sort)
        && write_bytes(body, &mut bc, b"&finished=")
        && write_url_encoded(body, &mut bc, finished))
    {
        return write_error("search", "internal_error", "body overflow");
    }
    let url_len = match build_url(b"/api/v4/search/fetch") {
        Some(n) => n,
        None => return write_error("search", "internal_error", "url overflow"),
    };
    let len = match fetch_post_form(scratch_a_slice(url_len), scratch_b_slice(bc), req) {
        Ok(n) => n,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("search", c, m);
        }
    };
    write_manga_list_response("search", body_slice(len), b"data", page, b"search")
}

fn fetch_leaderboard(operation: &str, page: usize, typ: &[u8], req: &[u8]) -> u32 {
    let body = scratch_b();
    let mut bc = 0usize;
    if !(write_bytes(body, &mut bc, b"type=")
        && write_url_encoded(body, &mut bc, typ)
        && write_bytes(body, &mut bc, b"&page=")
        && write_usize(body, &mut bc, page))
    {
        return write_error(operation, "internal_error", "body overflow");
    }
    let url_len = match build_url(b"/api/readLeaderboard") {
        Some(n) => n,
        None => return write_error(operation, "internal_error", "url overflow"),
    };
    let len = match fetch_post_form(scratch_a_slice(url_len), scratch_b_slice(bc), req) {
        Ok(n) => n,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error(operation, c, m);
        }
    };
    write_manga_list_response(operation, body_slice(len), b"info", page, b"popular")
}

fn fetch_latest(operation: &str, page: usize, req: &[u8]) -> u32 {
    let body = scratch_b();
    let mut bc = 0usize;
    if !(write_bytes(body, &mut bc, b"page=")
        && write_usize(body, &mut bc, page)
        && write_bytes(body, &mut bc, b"&sort=new"))
    {
        return write_error(operation, "internal_error", "body overflow");
    }
    let url_len = match build_url(b"/api/b1/booklist") {
        Some(n) => n,
        None => return write_error(operation, "internal_error", "url overflow"),
    };
    let len = match fetch_post_form(scratch_a_slice(url_len), scratch_b_slice(bc), req) {
        Ok(n) => n,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error(operation, c, m);
        }
    };
    write_manga_list_response(operation, body_slice(len), b"info", page, b"latest")
}

fn manga_id_value<'a>(id: &'a [u8]) -> Option<&'a [u8]> {
    let prefix = b"manga:";
    if id.len() > prefix.len() && &id[..prefix.len()] == prefix {
        Some(&id[prefix.len()..])
    } else {
        None
    }
}

fn chapter_id_value<'a>(id: &'a [u8]) -> Option<(&'a [u8], &'a [u8])> {
    let prefix = b"chapter:";
    if id.len() <= prefix.len() || &id[..prefix.len()] != prefix {
        return None;
    }
    let rest = &id[prefix.len()..];
    let sep = find_subslice(rest, b":")?;
    Some((&rest[..sep], &rest[sep + 1..]))
}

// Same-origin v4 is the current website contract. Keep legacy routes only for
// an explicit missing resource; auth/denial/transport/DTO errors never select it.
// External manga:/chapter: identities stay unchanged. See this source README.
type NoyError = (&'static str, &'static str);
type NoyResult<T> = Result<T, NoyError>;
#[derive(Clone, Copy, PartialEq)]
enum NoyApi { V4, Legacy }
fn noy_parse(message: &'static str) -> NoyError { ("parse_error", message) }
fn noy_member<'a>(json: &'a [u8], key: &[u8]) -> NoyResult<Option<&'a [u8]>> {
    noy_json::direct(json, key).map_err(|_| noy_parse("invalid or repeated DTO member"))
}
fn noy_string<'a>(json: &'a [u8], key: &[u8]) -> NoyResult<Option<&'a [u8]>> {
    noy_json::raw_string(json, key).map_err(|_| noy_parse("invalid DTO string"))
}
fn noy_identity(raw: &[u8]) -> NoyResult<&[u8]> {
    let raw = if raw.first() == Some(&b'"') && raw.last() == Some(&b'"') && raw.len() >= 2 {
        &raw[1..raw.len()-1]
    } else { raw };
    if raw.is_empty() || raw.len() > 20 || !raw.iter().all(u8::is_ascii_digit) {
        return Err(noy_parse("invalid numeric business identity"));
    }
    Ok(raw)
}
fn noy_canonical(id: &[u8]) -> &[u8] {
    let i = id.iter().position(|b| *b != b'0').unwrap_or(id.len()-1);
    &id[i..]
}
fn noy_id<'a>(obj: &'a [u8], key: &[u8]) -> NoyResult<&'a [u8]> {
    noy_identity(noy_member(obj, key)?.ok_or(noy_parse("business identity missing"))?)
}
fn noy_uint(raw: &[u8]) -> NoyResult<usize> {
    if raw.is_empty() || !raw.iter().all(u8::is_ascii_digit) { return Err(noy_parse("invalid nonnegative DTO integer")); }
    let mut n = 0usize;
    for &b in raw { n = n.checked_mul(10).and_then(|n| n.checked_add((b-b'0') as usize)).ok_or(noy_parse("DTO integer overflow"))?; }
    Ok(n)
}
fn noy_alias<'a>(obj: &'a [u8], upper: &[u8], lower: &[u8]) -> NoyResult<Option<&'a [u8]>> {
    Ok(noy_member(obj, upper)?.or(noy_member(obj, lower)?))
}
fn noy_check_body(json: &[u8]) -> NoyResult<()> {
    noy_json::complete(json).map_err(|_| noy_parse("incomplete JSON response"))?;
    if noy_string(json, b"status")? == Some(b"login") {
        return Err(("auth_required", "site returned login status"));
    }
    Ok(())
}
fn noy_book_info<'a>(json: &'a [u8], requested: &[u8], api: NoyApi) -> NoyResult<&'a [u8]> {
    noy_check_body(json)?;
    let info = if let Some(book) = noy_member(json, b"book")? {
        noy_member(book, b"info")?.or(if api == NoyApi::Legacy { Some(book) } else { None })
    } else if api == NoyApi::Legacy { noy_member(json, b"info")? } else { None }
        .ok_or(noy_parse("book.info missing"))?;
    let raw = noy_alias(info, b"Bid", b"id")?.ok_or(noy_parse("book identity missing"))?;
    if noy_canonical(noy_identity(raw)?) != noy_canonical(noy_identity(requested)?) {
        return Err(("source_error", "returned book identity differs; no ID remapping"));
    }
    if let (Some(a), Some(b)) = (noy_member(info, b"Bid")?, noy_member(info, b"id")?) {
        if noy_canonical(noy_identity(a)?) != noy_canonical(noy_identity(b)?) { return Err(noy_parse("conflicting book identity aliases")); }
    }
    Ok(info)
}
fn noy_get(path: &[u8], id: &[u8], tail: &[u8], req: &[u8]) -> NoyResult<usize> {
    let buf = scratch_a(); let mut c = 0;
    if !(write_bytes(buf,&mut c,BASE_URL) && write_bytes(buf,&mut c,path)
        && write_url_encoded(buf,&mut c,id) && write_bytes(buf,&mut c,tail)) { return Err(("internal_error","URL overflow")); }
    let dst = http_req_buf(); let mut n = 0;
    let ok = write_bytes(dst,&mut n,br#"{"version":1,"method":"GET","url":""#)
        && append_json_escaped(dst,&mut n,scratch_a_slice(c))
        && write_bytes(dst,&mut n,br#"","headers":{"User-Agent":"NoyAcg/3.0","Accept":"application/json, text/plain, */*","Referer":"https://noymanga.com/","allow-adult":""#)
        && write_bytes(dst,&mut n,adult(req))
        && write_bytes(dst,&mut n,br#""},"timeoutMs":15000,"responseKind":"bodyText"}"#);
    if !ok { return Err(("internal_error","request overflow")); }
    let len = http_request(&dst[..n],http_out()).map_err(|_| ("network_error","connection or timeout failure"))?;
    let envelope = &http_out()[..len];
    if noy_json::direct(envelope,b"status").ok().flatten().or(noy_json::direct(envelope,b"statusCode").ok().flatten()) == Some(b"401") {
        return Err(("auth_required","site returned HTTP 401"));
    }
    let n = decode_json_body(envelope).map_err(fetch_error_code)?;
    noy_check_body(body_slice(n))?;
    Ok(n)
}
fn fetch_detail(id: &[u8], req: &[u8]) -> NoyResult<(usize,NoyApi)> {
    noy_identity(id).map_err(|_|("invalid_request","invalid numeric manga identity"))?;
    match noy_get(b"/api/v4/book/",id,b"?comment=false",req) {
        Ok(n) => Ok((n,NoyApi::V4)),
        Err(("not_found",_)) => noy_get(b"/api/b1/bookInfo/",id,b"",req).map(|n|(n,NoyApi::Legacy)),
        Err(error) => Err(error),
    }
}

fn run_get_manga(req: &[u8]) -> u32 {
    let manga_id = match extract_json_string(req, b"mangaId") {
        Some(v) => v,
        None => return write_error("get_manga", "invalid_request", "missing mangaId"),
    };
    let id = match manga_id_value(manga_id) {
        Some(v) => v,
        None => return write_error("get_manga", "invalid_request", "unexpected mangaId"),
    };
    let (len,api) = match fetch_detail(id, req) {
        Ok(value) => value,
        Err((code,message)) => return write_error("get_manga",code,message),
    };
    let json = body_slice(len);
    let info = match noy_book_info(json,id,api) {
        Ok(info) => info,
        Err((code,message)) => return write_error("get_manga",code,message),
    };
    let (title,author,desc) = if api==NoyApi::V4 {
        let text = |upper:&[u8],lower:&[u8]| -> NoyResult<Option<&[u8]>> {
            Ok(noy_string(info,upper)?.or(noy_string(info,lower)?))
        };
        match (text(b"Bookname",b"name"),text(b"Author",b"author"),text(b"Description",b"description")) {
            (Ok(Some(title)),Ok(author),Ok(desc)) => (title,author.unwrap_or(b""),desc.unwrap_or(b"")),
            _ => return write_error("get_manga","parse_error","invalid book text DTO"),
        }
    } else {(title_from_manga(info,id),author_from_manga(info),desc_from_manga(info))};
    let status = if api==NoyApi::V4 {
        match (noy_alias(info,b"Mode",b"mode"),noy_alias(info,b"Status",b"status")) {
            (Ok(Some(b"0")),_) | (_,Ok(Some(b"1"))) => b"completed".as_slice(),
            (Ok(Some(_)),Ok(Some(_))) => b"ongoing".as_slice(),
            _ => b"unknown".as_slice(),
        }
    } else {b"unknown".as_slice()};
    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"manga":{"id":"manga:"#)
        && append_json_escaped(payload, &mut c, id)
        && write_bytes(payload, &mut c, br#"","title":""#)
        && append_json_unescaped_then_escaped(payload, &mut c, title)
        && write_bytes(payload, &mut c, br#"","alternateTitles":[],"description":""#)
        && append_json_unescaped_then_escaped(payload, &mut c, desc)
        && write_bytes(payload, &mut c, br#"","cover":"#)
        && write_cover(payload, &mut c, id)
        && write_bytes(payload, &mut c, br#","authors":["#)
        && if author.is_empty() {
            true
        } else {
            write_json_string_value(payload, &mut c, author)
        }
        && write_bytes(payload,&mut c,br#"],"artists":[],"status":""#)
        && write_bytes(payload,&mut c,status)
        && write_bytes(payload,&mut c,br#"","contentRating":"nsfw","language":"zh","tags":["#)
        && write_tags(payload, &mut c, info)
        && write_bytes(payload, &mut c, br#"],"links":[{"kind":"source","url":""#)
        && write_bytes(payload, &mut c, BASE_URL)
        && write_bytes(payload, &mut c, b"/manga/")
        && append_json_escaped(payload, &mut c, id)
        && write_bytes(payload, &mut c, br#""}]}}"#);
    if !ok {
        return write_error("get_manga", "internal_error", "payload overflow");
    }
    write_success_payload("get_manga", c)
}

// Traverse chapter records, not their category/data wrapper. Preserve provider
// encounter order; a page count is never a chapter number or reading ordinal.
fn next_catalog_chapter<'a>(data: &'a [u8], pos: &mut usize) -> Result<Option<&'a [u8]>, &'static str> {
    while *pos < data.len() {
        match data[*pos] {
            b'"' => {
                *pos += 1;
                while *pos < data.len() && data[*pos] != b'"' {
                    if data[*pos] == b'\\' { *pos += 1; }
                    *pos += 1;
                }
                if *pos >= data.len() { return Err("unterminated chapter string"); }
                *pos += 1;
            }
            b'{' => {
                let start = *pos;
                let obj = extract_balanced(data, start, b'{', b'}').ok_or("incomplete chapter object")?;
                if json_object_member(obj, b"id").is_some() {
                    if json_object_member(obj, b"name").is_none() { return Err("chapter title missing"); }
                    *pos += obj.len();
                    return Ok(Some(obj));
                }
                if json_object_member(obj, b"name").is_some() && json_object_member(obj, b"count").is_some() {
                    return Err("chapter id missing");
                }
                // Enter a wrapper without treating nested id/name/count as its own.
                *pos += 1;
            }
            _ => *pos += 1,
        }
    }
    Ok(None)
}

// No allocation/dependency/lockfile change. 8192 records exceed what the
// 1MiB typed response can encode; exceeding either bound is an explicit error.
const NOY_MAX_ROWS: usize = 8192;
#[derive(Clone, Copy)]
struct NoyRow<'a> { raw: &'a [u8], category: &'a [u8], hash: u64, position: Option<u64> }
const NOY_EMPTY: NoyRow<'static> = NoyRow { raw: b"",category:b"",hash:0,position:None };
static mut NOY_ROWS: [NoyRow<'static>;NOY_MAX_ROWS] = [NOY_EMPTY;NOY_MAX_ROWS];
fn noy_rows() -> &'static mut [NoyRow<'static>] { unsafe { &mut *core::ptr::addr_of_mut!(NOY_ROWS) } }
struct NoyArray<'a> { raw: &'a [u8], pos: usize }
impl<'a> NoyArray<'a> {
    fn new(raw: &'a [u8]) -> NoyResult<Self> {
        let raw=raw.trim_ascii();
        if raw.first()!=Some(&b'[') || raw.last()!=Some(&b']') { return Err(noy_parse("DTO object array expected")); }
        Ok(Self{raw,pos:1})
    }
    fn next(&mut self) -> NoyResult<Option<&'a [u8]>> {
        while self.pos<self.raw.len() && (self.raw[self.pos].is_ascii_whitespace() || self.raw[self.pos]==b',') { self.pos+=1; }
        if self.raw.get(self.pos)==Some(&b']') { return Ok(None); }
        if self.raw.get(self.pos)!=Some(&b'{') { return Err(noy_parse("non-object in DTO array")); }
        let obj=extract_balanced(self.raw,self.pos,b'{',b'}').ok_or(noy_parse("incomplete DTO object"))?;
        self.pos+=obj.len(); Ok(Some(obj))
    }
}
fn noy_hash(id:&[u8])->u64 { id.iter().fold(0xcbf29ce484222325u64,|h,b|(h^(*b as u64)).wrapping_mul(0x100000001b3)) }
fn noy_count(obj:&[u8])->NoyResult<usize> { noy_uint(noy_member(obj,b"count")?.ok_or(noy_parse("chapter page count missing"))?) }
fn noy_add_row<'a>(rows:&mut[NoyRow<'a>],used:&mut usize,obj:&'a[u8],category:&'a[u8],position:Option<u64>)->NoyResult<()> {
    let id=noy_id(obj,b"id")?;
    if noy_canonical(id)==b"0" { return Err(noy_parse("ordinary chapter uses reserved single-chapter ID")); }
    noy_string(obj,b"name")?.ok_or(noy_parse("chapter title missing"))?;
    noy_count(obj)?;
    let hash=noy_hash(noy_canonical(id));
    for previous in &rows[..*used] {
        if previous.hash==hash && noy_canonical(noy_id(previous.raw,b"id")?)==noy_canonical(id) {
            // Both upstream and website address this chapter by bid/cid only.
            // A repeat is the same business identity even if its metadata changed;
            // retain the first occurrence in provider category order.
            return Ok(());
        }
    }
    if *used>=rows.len() { return Err(("source_error","complete catalog exceeds response capacity")); }
    rows[*used]=NoyRow{raw:obj,category,hash,position}; *used+=1; Ok(())
}
fn noy_single_len(info:&[u8])->NoyResult<Option<usize>> {
    match noy_alias(info,b"Len",b"len")? {
        Some(b"null")|None => Ok(None),
        Some(raw) => Ok(Some(noy_uint(raw)?).filter(|v|*v>0)),
    }
}
fn noy_collect<'a>(json:&'a[u8],bid:&[u8],api:NoyApi,rows:&mut[NoyRow<'a>])->NoyResult<(usize,Option<usize>)> {
    let info=noy_book_info(json,bid,api)?;
    let Some(chapters)=noy_member(json,b"chapters")? else {
        return if api==NoyApi::V4 {Ok((0,noy_single_len(info)?))} else {Err(noy_parse("legacy chapters missing"))};
    };
    let categories=noy_member(chapters,b"categories")?;
    let data=noy_member(chapters,b"data")?;
    let mut used=0;
    let Some(categories)=categories else {
        if api==NoyApi::V4 && data.is_some() { return Err(noy_parse("v4 categories missing")); }
        if api==NoyApi::Legacy {
            let raw=data.unwrap_or(chapters); let mut pos=0;
            while let Some(obj)=next_catalog_chapter(raw,&mut pos).map_err(noy_parse)? { noy_add_row(rows,&mut used,obj,b"",None)?; }
        }
        return Ok((used,if used==0 && api==NoyApi::V4 {noy_single_len(info)?} else {None}));
    };
    let mut cats=NoyArray::new(categories)?; let mut category_count=0;
    while let Some(category)=cats.next()? {
        category_count+=1;
        let cid=noy_id(category,b"id")?;
        noy_string(category,b"name")?.ok_or(noy_parse("category name missing"))?;
        let mut duplicates=0; let mut all=NoyArray::new(categories)?;
        while let Some(other)=all.next()? { if noy_canonical(noy_id(other,b"id")?)==noy_canonical(cid) { duplicates+=1; } }
        if duplicates!=1 { return Err(noy_parse("duplicate provider category identity")); }
        let data=data.ok_or(noy_parse("category data missing"))?;
        let array=noy_member(data,cid)?.ok_or(noy_parse("declared category data missing"))?;
        let mut chapters=NoyArray::new(array)?;
        let mut position=0u64;
        while let Some(obj)=chapters.next()? {
            // Both adopted/current upstream reverse this complete data array for
            // Mihon's newest-first source list. Positive reading restores this
            // original array order. sort is not a ChapterDto member or gate.
            // Count every original slot before identity deduplication so that
            // keeping a repeated chapter's first occurrence never shifts peers.
            // The retained b1 endpoint is a separate, unverified direction
            // contract; a matching JSON shape alone does not prove v4 order.
            noy_add_row(rows,&mut used,obj,category,(api==NoyApi::V4).then_some(position))?;
            position+=1;
        }
    }
    // Every data key must correspond to a declared category: never omit a group.
    if let Some(data)=data {
        if data.first()!=Some(&b'{') { return Err(noy_parse("category data object expected")); }
        let mut pos=1;
        while pos<data.len() {
            while pos<data.len() && (data[pos].is_ascii_whitespace() || data[pos]==b',') {pos+=1;}
            if data.get(pos)==Some(&b'}') {break;}
            if data.get(pos)!=Some(&b'"') { return Err(noy_parse("invalid category data key")); }
            pos+=1; let start=pos;
            while pos<data.len() && data[pos]!=b'"' {if data[pos]==b'\\' {pos+=1;} pos+=1;}
            let mut keybuf=[0;64]; let key=decode_json_content(&data[start..pos],&mut keybuf).map_err(noy_parse)?;
            let mut found=false; let mut all=NoyArray::new(categories)?;
            while let Some(category)=all.next()? { if noy_id(category,b"id")?==key {found=true;} }
            if !found {return Err(noy_parse("undeclared provider category data"));}
            let value=noy_member(data,key)?.ok_or(noy_parse("category value missing"))?;
            pos=value.as_ptr() as usize-data.as_ptr() as usize+value.len();
        }
    }
    Ok((used,if category_count==0 && api==NoyApi::V4 {noy_single_len(info)?} else {None}))
}
fn noy_sdk(error:koma_source_sdk::source::SourceError)->NoyError { (error.code(),error.message()) }
fn noy_qualified<'a>(dst:&'a mut[u8],prefix:&[u8],bid:&[u8],cid:Option<&[u8]>)->NoyResult<&'a[u8]> {
    let mut n=0;
    if !(write_bytes(dst,&mut n,prefix)&&write_bytes(dst,&mut n,bid)) {return Err(("internal_error","identity overflow"));}
    if let Some(cid)=cid {if !(write_bytes(dst,&mut n,b":")&&write_bytes(dst,&mut n,cid)) {return Err(("internal_error","identity overflow"));}}
    Ok(&dst[..n])
}
fn render_chapter_catalog<'a,'j>(payload:&'a mut[u8],request:&[u8],bid:&[u8],json:&'j[u8],api:NoyApi,rows:&mut[NoyRow<'j>])->ChapterResult<'a,NoyError> {
    let (used,single)=noy_collect(json,bid,api,rows)?;
    let mut mid=[0;64]; let mid=noy_qualified(&mut mid,b"manga:",bid,None)?;
    if let Some(count)=single {
        let mut cid=[0;96];let cid=noy_qualified(&mut cid,b"chapter:",bid,Some(b"0"))?;
        let mut title=[0;96];let mut n=0;
        if !(write_bytes(&mut title,&mut n,"單章節（".as_bytes())&&write_usize(&mut title,&mut n,count)&&write_bytes(&mut title,&mut n,"P）".as_bytes())) {return Err(("internal_error","single chapter title overflow"));}
        let mut chapter=Chapter::new(cid,mid,&title[..n]);chapter.chapter_number=Some(ChapterNumber::Integer(0));chapter.page_count=Some(count);chapter.language=Some(b"zh");
        let info=noy_book_info(json,bid,api)?;
        if let Some(time)=noy_alias(info,b"Time",b"time")? {noy_uint(time)?;chapter.published_at=Some(time);}
        return ChapterResponse::single(payload,chapter).map_err(noy_sdk);
    }
    let mut catalog=GroupedChapters::new(payload,request).map_err(noy_sdk)?;
    for row in &rows[..used] {
        let mut cid=[0;96];let cid=noy_qualified(&mut cid,b"chapter:",bid,Some(noy_id(row.raw,b"id")?))?;
        let mut titlebuf=[0;8192];let title=decode_json_content(noy_string(row.raw,b"name")?.ok_or(noy_parse("chapter title missing"))?,&mut titlebuf).map_err(noy_parse)?;
        let mut chapter=Chapter::new(cid,mid,title);chapter.page_count=Some(noy_count(row.raw)?);chapter.language=Some(b"zh");
        let mut namebuf=[0;8192];let mut groupbuf=[0;96];
        let group=if !row.category.is_empty() {
            chapter.scanlator=Some(decode_json_content(noy_string(row.category,b"name")?.ok_or(noy_parse("category name missing"))?,&mut namebuf).map_err(noy_parse)?);
            let mut n=0;
            if !(write_bytes(&mut groupbuf,&mut n,b"noy-category:")&&write_bytes(&mut groupbuf,&mut n,noy_id(row.category,b"id")?)) {return Err(("internal_error","category key overflow"));}
            &groupbuf[..n]
        } else {b"".as_slice()};
        let mut datebuf=[0;128];
        if let Some(raw)=noy_member(row.raw,b"created_at")? {
            if raw.first()==Some(&b'"') {
                chapter.published_at=Some(decode_json_content(noy_string(row.raw,b"created_at")?.ok_or(noy_parse("invalid creation date"))?,&mut datebuf).map_err(noy_parse)?);
            } else if raw!=b"null" {noy_uint(raw)?;chapter.published_at=Some(raw);}
        }
        // Every complete classified v4 array supplies every chapter's ordinal.
        // Optional sort, display numbers and repeated titles never gate it.
        // The retained legacy endpoint lacks this adopted v4 direction contract.
        let placement=match row.position {
            Some(position)=>ChapterPlacement::InGroup{group,position},
            None=>ChapterPlacement::ManualOnly,
        };
        catalog.push(chapter,placement).map_err(noy_sdk)?;
    }
    catalog.finish(ChapterPage::Complete).map_err(noy_sdk)
}
fn run_get_chapters(req:&[u8])->ChapterResult<'static,NoyError> {
    let manga=extract_json_string(req,b"mangaId").ok_or(("invalid_request","missing mangaId"))?;
    let id=manga_id_value(manga).ok_or(("invalid_request","unexpected mangaId"))?;
    let (len,api)=fetch_detail(id,req)?;
    render_chapter_catalog(payload_buf(),req,id,body_slice(len),api,noy_rows())
}
fn write_chapter_error(error:NoyError)->u32 {write_error("get_chapters",error.0,error.1)}

fn fetch_chapter(chapter_id:&[u8],req:&[u8])->NoyResult<usize> {
    noy_get(b"/api/b1/chapter/",chapter_id,b"",req)
}

fn looks_like_image_url(url: &[u8]) -> bool {
    (contains_bytes(url, b".webp")
        || contains_bytes(url, b".jpg")
        || contains_bytes(url, b".jpeg")
        || contains_bytes(url, b".png"))
        && (contains_bytes(url, b"noymanga") || contains_bytes(url, b"http"))
}

fn next_json_string<'a>(data: &'a [u8], pos: &mut usize) -> Option<&'a [u8]> {
    while *pos < data.len() && data[*pos] != b'"' {
        *pos += 1;
    }
    if *pos >= data.len() {
        return None;
    }
    *pos += 1;
    let start = *pos;
    while *pos < data.len() {
        if data[*pos] == b'\\' {
            *pos += 2;
            continue;
        }
        if data[*pos] == b'"' {
            let out = &data[start..*pos];
            *pos += 1;
            return Some(out);
        }
        *pos += 1;
    }
    None
}

fn run_legacy_pages(req: &[u8]) -> u32 {
    let chapter_id = match extract_json_string(req, b"chapterId") {
        Some(v) => v,
        None => return write_error("get_pages", "invalid_request", "missing chapterId"),
    };
    let (_manga, cid) = match chapter_id_value(chapter_id) {
        Some(v) => v,
        None => return write_error("get_pages", "invalid_request", "unexpected chapterId"),
    };
    let len = match fetch_chapter(cid, req) {
        Ok(n) => n,
        Err((code,message)) => return write_error("get_pages",code,message),
    };
    let json = body_slice(len);
    if let Err((code,message))=noy_check_body(json) {return write_error("get_pages",code,message);}
    let payload = payload_buf();
    let mut c = 0usize;
    if !(write_bytes(payload, &mut c, br#"{"chapterId":"#)
        && write_json_string_value(payload, &mut c, chapter_id)
        && write_bytes(payload, &mut c, br#","pages":["#))
    {
        return write_error("get_pages", "internal_error", "payload overflow");
    }
    let mut pos = 0usize;
    let mut written = 0usize;
    while let Some(url) = next_json_string(json, &mut pos) {
        if !looks_like_image_url(url) {
            continue;
        }
        if written > 0 && !write_bytes(payload, &mut c, b",") {
            return write_error("get_pages","internal_error","payload overflow");
        }
        let ok = write_bytes(payload, &mut c, br#"{"id":"page:"#)
            && append_json_escaped(payload, &mut c, cid)
            && write_bytes(payload, &mut c, b":")
            && write_usize(payload, &mut c, written)
            && write_bytes(payload, &mut c, br#"","index":"#)
            && write_usize(payload, &mut c, written)
            && write_bytes(payload, &mut c, br#","image":{"kind":"url","url":""#)
            && append_json_unescaped_then_escaped(payload, &mut c, url)
            && write_bytes(payload, &mut c, br#""}}"#);
        if !ok {
            return write_error("get_pages", "internal_error", "payload overflow");
        }
        written += 1;
    }
    if written==0 {return write_error("get_pages","parse_error","legacy response has no image pages");}
    if !write_bytes(payload, &mut c, b"]}") {
        return write_error("get_pages", "internal_error", "payload overflow");
    }
    write_success_payload("get_pages", c)
}

fn noy_render_pages(payload:&mut[u8],chapter_id:&[u8],bid:&[u8],cid:&[u8],count:usize,single:bool)->NoyResult<usize> {
    let mut c=0;
    if !(write_bytes(payload,&mut c,br#"{"chapterId":"#)&&write_json_string_value(payload,&mut c,chapter_id)&&write_bytes(payload,&mut c,br#","pages":["#)) {return Err(("internal_error","payload overflow"));}
    for index in 0..count {
        let ok=(index==0 || write_bytes(payload,&mut c,b","))
            && write_bytes(payload,&mut c,br#"{"id":"page:"#)&&append_json_escaped(payload,&mut c,cid)
            && write_bytes(payload,&mut c,b":")&&write_usize(payload,&mut c,index)
            && write_bytes(payload,&mut c,br#"","index":"#)&&write_usize(payload,&mut c,index)
            && write_bytes(payload,&mut c,br#","image":{"kind":"url","url":""#)&&write_bytes(payload,&mut c,IMG_BASE)
            && append_json_escaped(payload,&mut c,bid)&&write_bytes(payload,&mut c,b"/")
            && (single || (append_json_escaped(payload,&mut c,cid)&&write_bytes(payload,&mut c,b"/")))
            && write_usize(payload,&mut c,index+1)&&write_bytes(payload,&mut c,br#".webp"}}"#);
        if !ok {return Err(("internal_error","payload overflow"));}
    }
    if !write_bytes(payload,&mut c,b"]}") {return Err(("internal_error","payload overflow"));}
    Ok(c)
}
fn run_get_pages(req:&[u8])->u32 {
    let chapter_id=match extract_json_string(req,b"chapterId") {Some(v)=>v,None=>return write_error("get_pages","invalid_request","missing chapterId")};
    let (bid,cid)=match chapter_id_value(chapter_id) {Some(v)=>v,None=>return write_error("get_pages","invalid_request","unexpected chapterId")};
    if noy_identity(bid).is_err() || noy_identity(cid).is_err() {return write_error("get_pages","invalid_request","invalid chapter identity");}
    if let Some(manga)=extract_json_string(req,b"mangaId") {
        if manga_id_value(manga)!=Some(bid) {return write_error("get_pages","invalid_request","chapter and manga identities differ");}
    }
    // Pages need the v4 catalog or the exact old chapter route, not an extra
    // legacy book lookup. At most one explicit fallback follows a missing ID.
    let len=match noy_get(b"/api/v4/book/",bid,b"?comment=false",req) {
        Ok(value)=>value,
        Err(("not_found",_))=>return run_legacy_pages(req),
        Err((code,message))=>return write_error("get_pages",code,message),
    };
    let rows=noy_rows();
    let (used,single)=match noy_collect(body_slice(len),bid,NoyApi::V4,rows) {Ok(v)=>v,Err((code,message))=>return write_error("get_pages",code,message)};
    let (count,is_single)=if cid==b"0" {
        match single {Some(count)=>(count,true),None=>return write_error("get_pages","not_found","not a verified single-chapter work")}
    } else {
        let matched=rows[..used].iter().find(|row|noy_id(row.raw,b"id").is_ok_and(|id|noy_canonical(id)==noy_canonical(cid)));
        match matched {
            Some(row)=>match noy_count(row.raw) {Ok(count)=>(count,false),Err((code,message))=>return write_error("get_pages",code,message)},
            // Exact old identity absent: retain the old endpoint, never match a title or ordinal.
            None=>return run_legacy_pages(req),
        }
    };
    match noy_render_pages(payload_buf(),chapter_id,bid,cid,count,is_single) {
        Ok(n)=>write_success_payload("get_pages",n),Err((code,message))=>write_error("get_pages",code,message),
    }
}

fn run_get_listings(_req: &[u8]) -> u32 {
    const LISTINGS: &str = r#"{"listings":[{"id":"popular","name":"热门","kind":"popular"},{"id":"latest","name":"最新","kind":"latest"}]}"#;
    payload_buf()[..LISTINGS.len()].copy_from_slice(LISTINGS.as_bytes());
    write_success_payload("get_listings", LISTINGS.len())
}

// Fixed NoyAcg preference: only the popular navigation consumes this value.
// Explicit day/week/month listings retain their original override and request contract.
fn popular_type(req: &[u8]) -> &'static [u8] {
    match json_object_member(req, b"settings")
        .and_then(|s| json_object_member(s, b"POPULAR_MANGAS"))
    {
        Some(br#""week""#) => b"week",
        Some(br#""month""#) => b"month",
        _ => b"day",
    }
}

fn run_get_settings(_req: &[u8]) -> u32 {
    const SETTINGS: &str = r#"{"settings":[{"id":"POPULAR_MANGAS","name":"热门漫画显示内容","kind":"select","default":"day","options":[{"value":"day","label":"日阅读榜"},{"value":"week","label":"周阅读榜"},{"value":"month","label":"月阅读榜"}]},{"id":"ADULT","name":"漫画内容类型","kind":"select","default":"both","options":[{"value":"false","label":"仅显示全年龄内容"},{"value":"true","label":"仅显示成人内容"},{"value":"both","label":"显示所有内容"}]}]}"#;
    payload_buf()[..SETTINGS.len()].copy_from_slice(SETTINGS.as_bytes());
    write_success_payload("get_settings", SETTINGS.len())
}

fn run_get_manga_list(req: &[u8]) -> u32 {
    let page = request_page(req);
    let listing = extract_json_string(req, b"listingId").unwrap_or(b"latest");
    if listing == b"popular" || listing == b"day" || listing == b"week" || listing == b"month" {
        let typ = if listing == b"popular" { popular_type(req) } else { listing };
        fetch_leaderboard("get_manga_list", page, typ, req)
    } else {
        fetch_latest("get_manga_list", page, req)
    }
}

fn run_get_filters(_req: &[u8]) -> u32 {
    #[cfg(feature = "noyacg_filter_candidate")]
    { return filter_candidate::write_schema(); }
    #[cfg(not(feature = "noyacg_filter_candidate"))]
    {
    const FILTERS: &str = "{\"filters\":[{\"id\":\"mode\",\"name\":\"搜索模式\",\"kind\":\"select\",\"options\":[{\"value\":\"default\",\"label\":\"默认\"},{\"value\":\"tag\",\"label\":\"标签\"},{\"value\":\"author\",\"label\":\"作者\"}],\"default\":\"default\"}]}";
    let bytes = FILTERS.as_bytes();
    if bytes.len() > payload_buf().len() {
        return write_error("get_filters", "internal_error", "payload overflow");
    }
    payload_buf()[..bytes.len()].copy_from_slice(bytes);
    write_success_payload("get_filters", bytes.len())
    }
}

fn run_get_image_request(req: &[u8]) -> u32 {
    let url = extract_json_string(req, b"url")
        .or_else(|| extract_json_string(req, b"imageUrl"))
        .unwrap_or(b"");
    if url.is_empty() {
        return write_error("get_image_request", "invalid_request", "missing url");
    }
    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"url":""#)
        && append_json_unescaped_then_escaped(payload, &mut c, url)
        && write_bytes(payload,&mut c,br#"","headers":{"User-Agent":"NoyAcg/3.0","referer":"https://noymanga.com/","allow-adult":""#)
        && write_bytes(payload,&mut c,adult(req))
        && write_bytes(payload,&mut c,br#""}}"#);
    if !ok {
        return write_error("get_image_request", "internal_error", "payload overflow");
    }
    write_success_payload("get_image_request", c)
}

#[no_mangle]
pub extern "C" fn koma_source_init(_manifest_ptr: u32, manifest_len: u32) -> i32 {
    log_info(b"noyacg source init");
    if host::check_cancel() {
        return -2;
    }
    if manifest_len > 0 {
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
pub extern "C" fn koma_source_search(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("search", "invalid_request", "empty request"),
    };
    log_info(b"noyacg search");
    run_search(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga", "invalid_request", "empty request"),
    };
    log_info(b"noyacg get_manga");
    run_get_manga(req)
}

koma_source_sdk::koma_source_chapter_export!("noyacg",run_get_chapters,write_chapter_error);

#[no_mangle]
pub extern "C" fn koma_source_get_pages(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_pages", "invalid_request", "empty request"),
    };
    log_info(b"noyacg get_pages");
    run_get_pages(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_listings(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_listings", "invalid_request", "empty request"),
    };
    log_info(b"noyacg get_listings");
    run_get_listings(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga_list(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga_list", "invalid_request", "empty request"),
    };
    log_info(b"noyacg get_manga_list");
    run_get_manga_list(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_settings(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_settings", "invalid_request", "empty request"),
    };
    run_get_settings(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_filters(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_filters", "invalid_request", "empty request"),
    };
    log_info(b"noyacg get_filters");
    run_get_filters(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_image_request(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_image_request", "invalid_request", "empty request"),
    };
    log_info(b"noyacg get_image_request");
    run_get_image_request(req)
}

#[no_mangle]
pub extern "C" fn koma_source_free(result_ptr: u32) {
    response_buffer().free(result_ptr)
}
