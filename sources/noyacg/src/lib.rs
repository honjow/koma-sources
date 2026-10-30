#![no_std]

extern crate koma_source_sdk;

use koma_source_sdk::host::{self, http_request, log_info};
use koma_source_sdk::json_utils::{
    append_json_escaped, append_json_unescaped_then_escaped, contains_bytes, extract_json_number,
    extract_json_string, find_subslice, json_object_member, write_bytes, write_url_encoded, write_usize,
};
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
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

// Preserve decoded query bytes independently from URL/form scratch buffers.
static mut SEARCH_QUERY: [u8; 8192] = [0; 8192];

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.noyacg.koma",
    name: "NoyAcg",
    version: "0.1.9",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "NoyAcg JSON API source.",
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
    filters: true,
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
    if form && !write_bytes(dst,c,br#","Content-Type":"application/x-www-form-urlencoded""#) {return false}
    write_bytes(dst,c,b"}")
}
fn build_post_form_request(dst: &mut [u8], url: &[u8], body: &[u8], adult: &[u8]) -> Option<usize> {
    let mut c = 0usize;
    write_bytes(dst, &mut c, br#"{"version":1,"method":"POST","url":""#).then_some(())?;
    append_json_escaped(dst, &mut c, url).then_some(())?;
    build_headers(dst, &mut c, true, adult).then_some(())?;
    write_bytes(dst, &mut c, br#","bodyBase64":""#).then_some(())?;
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

fn extract_object_for_key<'a>(data: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    extract_value_for_key(data, key, b'{')
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

fn write_tags(payload: &mut [u8], c: &mut usize, obj: &[u8]) -> bool {
    let mut written = 0usize;
    for key in [b"tags" as &[u8], b"pname", b"otag", b"Otag", b"ptag", b"Ptag"] {
        if let Some(arr) = extract_array_for_key(obj, key) {
            if !write_tags_from_array(payload, c, arr, &mut written) {
                return false;
            }
        } else if let Some(s) = extract_json_string(obj, key) {
            if !s.is_empty() {
                if written > 0 && !write_bytes(payload, c, b",") {
                    return false;
                }
                if !write_json_string_value(payload, c, s) {
                    return false;
                }
                written += 1;
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

// Only hosts that understand operation scope may see and send the extra search filters.
fn filter_scope_supported(req: &[u8]) -> bool {
    json_object_member(req, b"hostFeatures").map(|v| { let v=trim_ascii(v); v.starts_with(b"[") && v.ends_with(b"]") && v[1..v.len()-1].split(|c| *c==b',').any(|x| trim_ascii(x)==b"\"source_filter_scope_header_v1\"") }).unwrap_or(false)
}
fn search_filter_values<'a>(req: &'a [u8], mode: &'a [u8]) -> Option<(&'a [u8], &'a [u8], &'a [u8])> {
    if !filter_scope_supported(req) { return Some((mode,b"",b"")); }
    let filters=json_object_member(req,b"args").and_then(|a|json_object_member(a,b"filters"));
    let read=|key: &[u8]| filters.and_then(|f|extract_json_string(f,key));
    let mode=read(b"mode").unwrap_or(mode); let sort=read(b"sort").unwrap_or(b"time"); let finished=read(b"finished").unwrap_or(b"all");
    if !matches!(mode,b"default"|b"tag"|b"author") || !matches!(sort,b"time"|b"views"|b"favorites"|b"rating") || !matches!(finished,b"all"|b"false"|b"true") { return None; }
    Some((mode,if sort==b"time" {b""} else {sort},if finished==b"all" {b""} else {finished}))
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
    let Some((mode,sort,finished)) = search_filter_values(req,mode) else { return write_error("search", "invalid_request", "invalid search filter"); };
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

fn fetch_detail(id: &[u8], req: &[u8]) -> Result<usize, FetchError> {
    let buf = scratch_a();
    let mut c = 0usize;
    if !(write_bytes(buf, &mut c, BASE_URL)
        && write_bytes(buf, &mut c, b"/api/b1/bookInfo/")
        && write_url_encoded(buf, &mut c, id))
    {
        return Err(FetchError::Network);
    }
    if adult(req)==b"both" { return fetch_get(scratch_a_slice(c), None); }
    let n=koma_source_sdk::build_get_request(http_req_buf(),scratch_a_slice(c),None,&[(b"allow-adult",adult(req))]).ok_or(FetchError::Network)?;
    let len=http_request(&http_req_buf()[..n],http_out()).map_err(|_|FetchError::Network)?;
    decode_json_body(&http_out()[..len])
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
    let len = match fetch_detail(id, req) {
        Ok(n) => n,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_manga", c, m);
        }
    };
    let json = body_slice(len);
    if contains_bytes(json, br#""status":"login""#) {
        return write_error("get_manga", "auth_required", "site returned login status");
    }
    let info = extract_object_for_key(json, b"info")
        .or_else(|| extract_object_for_key(json, b"book"))
        .unwrap_or(json);
    let title = title_from_manga(info, id);
    let author = author_from_manga(info);
    let desc = desc_from_manga(info);
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
        && write_bytes(
            payload,
            &mut c,
            br#"],"artists":[],"status":"unknown","contentRating":"nsfw","language":"zh","tags":["#,
        )
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

fn run_get_chapters(req: &[u8]) -> u32 {
    let manga_id = match extract_json_string(req, b"mangaId") {
        Some(v) => v,
        None => return write_error("get_chapters", "invalid_request", "missing mangaId"),
    };
    let id = match manga_id_value(manga_id) {
        Some(v) => v,
        None => return write_error("get_chapters", "invalid_request", "unexpected mangaId"),
    };
    let len = match fetch_detail(id, req) {
        Ok(n) => n,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_chapters", c, m);
        }
    };
    let json = body_slice(len);
    if contains_bytes(json, br#""status":"login""#) {
        return write_error("get_chapters", "auth_required", "site returned login status");
    }
    let chapters = match extract_object_for_key(json, b"chapters") {
        Some(v) => v,
        None => return write_error("get_chapters", "parse_error", "missing chapters"),
    };
    let data = extract_object_for_key(chapters, b"data").unwrap_or(chapters);
    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("get_chapters", "internal_error", "payload overflow");
    }
    let mut pos = 0usize;
    let mut written = 0usize;
    while let Some(obj) = next_raw_object(data, &mut pos) {
        let cid = match extract_json_number(obj, b"id").or_else(|| extract_json_string(obj, b"id")) {
            Some(v) => v,
            None => continue,
        };
        let title = extract_json_string(obj, b"name").unwrap_or(b"Chapter");
        let count = extract_json_number(obj, b"count").unwrap_or(b"");
        let created = extract_json_string(obj, b"created_at").unwrap_or(b"");
        if written > 0 && !write_bytes(payload, &mut c, b",") {
            break;
        }
        let ok = write_bytes(payload, &mut c, br#"{"id":"chapter:"#)
            && append_json_escaped(payload, &mut c, id)
            && write_bytes(payload, &mut c, b":")
            && append_json_escaped(payload, &mut c, cid)
            && write_bytes(payload, &mut c, br#"","mangaId":"manga:"#)
            && append_json_escaped(payload, &mut c, id)
            && write_bytes(payload, &mut c, br#"","title":""#)
            && append_json_unescaped_then_escaped(payload, &mut c, title)
            && write_bytes(payload, &mut c, br#"","chapterNumber":"#)
            && if count.is_empty() {
                write_bytes(payload, &mut c, b"null")
            } else {
                write_json_string_value(payload, &mut c, count)
            }
            && write_bytes(payload, &mut c, br#","volumeNumber":null,"language":"zh","publishedAt":"#)
            && if created.is_empty() {
                write_bytes(payload, &mut c, b"null")
            } else {
                write_json_string_value(payload, &mut c, created)
            }
            && write_bytes(payload, &mut c, br#","updatedAt":null,"pageCount":"#)
            && if count.is_empty() {
                write_bytes(payload, &mut c, b"null")
            } else {
                append_json_escaped(payload, &mut c, count)
            }
            && write_bytes(payload, &mut c, b"}");
        if !ok {
            return write_error("get_chapters", "internal_error", "payload overflow");
        }
        written += 1;
        if written >= 1000 {
            break;
        }
    }
    if !write_bytes(
        payload,
        &mut c,
        br#"],"page":{"nextCursor":null,"hasMore":false}}"#,
    ) {
        return write_error("get_chapters", "internal_error", "payload overflow");
    }
    write_success_payload("get_chapters", c)
}

fn fetch_chapter(chapter_id: &[u8], req: &[u8]) -> Result<usize, FetchError> {
    let buf = scratch_a();
    let mut c = 0usize;
    if !(write_bytes(buf, &mut c, BASE_URL)
        && write_bytes(buf, &mut c, b"/api/b1/chapter/")
        && write_url_encoded(buf, &mut c, chapter_id))
    {
        return Err(FetchError::Network);
    }
    if adult(req)==b"both" { return fetch_get(scratch_a_slice(c), None); }
    let n=koma_source_sdk::build_get_request(http_req_buf(),scratch_a_slice(c),None,&[(b"allow-adult",adult(req))]).ok_or(FetchError::Network)?;
    let len=http_request(&http_req_buf()[..n],http_out()).map_err(|_|FetchError::Network)?;
    decode_json_body(&http_out()[..len])
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

fn run_get_pages(req: &[u8]) -> u32 {
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
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_pages", c, m);
        }
    };
    let json = body_slice(len);
    if contains_bytes(json, br#""status":"login""#) {
        return write_error("get_pages", "auth_required", "site returned login status");
    }
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
            break;
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
    if !write_bytes(payload, &mut c, b"]}") {
        return write_error("get_pages", "internal_error", "payload overflow");
    }
    write_success_payload("get_pages", c)
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
    const SETTINGS: &str = r#"{"settings":[{"id":"POPULAR_MANGAS","name":"熱門漫畫顯示內容","kind":"select","default":"day","options":[{"value":"day","label":"日閱讀榜"},{"value":"week","label":"周閱讀榜"},{"value":"month","label":"月閱讀榜"}]},{"id":"ADULT","name":"漫畫內容類型","kind":"select","default":"both","options":[{"value":"false","label":"僅顯示全年齡內容"},{"value":"true","label":"僅顯示成人内容"},{"value":"both","label":"顯示所有内容"}]}]}"#;
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

fn run_get_filters(req: &[u8]) -> u32 {
    if filter_scope_supported(req) {
        const FILTERS: &str = r#"{"filters":[{"id":"mode","name":"類型","kind":"select","scope":"search","default":"default","options":[{"value":"default","label":"預設"},{"value":"tag","label":"標籤"},{"value":"author","label":"作者"}]},{"id":"sort","name":"排序方式","kind":"select","scope":"search","default":"time","options":[{"value":"time","label":"時間"},{"value":"views","label":"閲讀"},{"value":"favorites","label":"收藏"},{"value":"rating","label":"評分"}]},{"id":"finished","name":"完結狀態","kind":"select","scope":"search","default":"all","options":[{"value":"all","label":"全部"},{"value":"false","label":"連載中"},{"value":"true","label":"已完結"}]}]}"#;
        payload_buf()[..FILTERS.len()].copy_from_slice(FILTERS.as_bytes());
        return write_success_payload("get_filters",FILTERS.len());
    }
    // The old capability was false: keep old hosts from exposing an inactive browse control.
    const FILTERS: &str = "{\"filters\":[]}";
    let bytes = FILTERS.as_bytes();
    if bytes.len() > payload_buf().len() {
        return write_error("get_filters", "internal_error", "payload overflow");
    }
    payload_buf()[..bytes.len()].copy_from_slice(bytes);
    write_success_payload("get_filters", bytes.len())
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
        && if adult(req)==b"both" { write_bytes(payload,&mut c,br#""}"#) } else {
            write_bytes(payload,&mut c,br#"","headers":{"allow-adult":""#)
                && write_bytes(payload,&mut c,adult(req))
                && write_bytes(payload,&mut c,br#""}}"#)
        };
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

#[no_mangle]
pub extern "C" fn koma_source_get_chapters(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_chapters", "invalid_request", "empty request"),
    };
    log_info(b"noyacg get_chapters");
    run_get_chapters(req)
}

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
