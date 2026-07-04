#![no_std]

extern crate koma_source_sdk;

use koma_source_sdk::host::{self, http_request, log_info};
use koma_source_sdk::json_utils::{
    append_json_escaped, contains_bytes, extract_json_number, extract_json_string, find_subslice,
    write_bytes, write_url_encoded, write_usize,
};
use koma_source_sdk::result::ResultBuffer;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};

const BASE_URL: &[u8] = b"https://m.happymh.com";
koma_source_sdk::koma_source_buffers! {
    payload: 1024 * 1024,
    http_out: 2 * 1024 * 1024,
    body: 2 * 1024 * 1024,
    http_req: 4096,
    scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.happymh.koma",
    name: "Happy漫画",
    version: "0.1.0",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "Happy漫画 source for happymh.com.",
    content_rating: "safe",
};

const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: true,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: true,
    manga_list: true,
    home: true,
    filters: true,
    settings: true,
    image_request: true,
    credentials: true,
};


fn read_request<'a>(req_ptr: u32, req_len: u32) -> Option<&'a [u8]> {
    if req_ptr == 0 || req_len == 0 {
        return None;
    }
    Some(unsafe { core::slice::from_raw_parts(req_ptr as *const u8, req_len as usize) })
}

#[derive(Copy, Clone)]
enum FetchError {
    Network,
    NotFound,
    RateLimit,
    ClientError,
    ServerError,
}

fn write_headers(
    dst: &mut [u8],
    c: &mut usize,
    referer: Option<&[u8]>,
    ajax_id: Option<&[u8]>,
) -> bool {
    if !write_bytes(dst, c, br#","headers":{"Accept":"application/json, text/plain, */*","User-Agent":"Mozilla/5.0 (Linux; Android 14) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Mobile Safari/537.36""#) {
        return false;
    }
    // Inject cookies from host settings if available
    let mut cookie_buf = [0u8; 2048];
    if let Some(cookies) = host::get_setting(b"cookies", &mut cookie_buf) {
        if !cookies.is_empty() {
            let _ = write_bytes(dst, c, br#","Cookie":""#)
                && append_json_escaped(dst, c, cookies)
                && write_bytes(dst, c, b"\"");
        }
    }
    if let Some(r) = referer {
        if !(write_bytes(dst, c, br#","Referer":""#)
            && append_json_escaped(dst, c, r)
            && write_bytes(dst, c, b"\""))
        {
            return false;
        }
    }
    if let Some(id) = ajax_id {
        if !(write_bytes(
            dst,
            c,
            br#","X-Requested-With":"XMLHttpRequest","X-Requested-Id":""#,
        ) && append_json_escaped(dst, c, id)
            && write_bytes(dst, c, b"\""))
        {
            return false;
        }
    }
    write_bytes(dst, c, b"}")
}

fn build_post_form_request(
    dst: &mut [u8],
    url: &[u8],
    referer: &[u8],
    body: &[u8],
) -> Option<usize> {
    let mut c = 0usize;
    write_bytes(dst, &mut c, br#"{"version":1,"method":"POST","url":""#).then_some(())?;
    append_json_escaped(dst, &mut c, url).then_some(())?;
    write_bytes(dst, &mut c, br#"","headers":{"Accept":"application/json, text/plain, */*","User-Agent":"Mozilla/5.0 (Linux; Android 14) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Mobile Safari/537.36","Content-Type":"application/x-www-form-urlencoded","Referer":""#).then_some(())?;
    append_json_escaped(dst, &mut c, referer).then_some(())?;
    write_bytes(dst, &mut c, br#""},"bodyBase64":""#).then_some(())?;
    append_json_escaped(dst, &mut c, body).then_some(())?;
    write_bytes(
        dst,
        &mut c,
        br#"","timeoutMs":15000,"responseKind":"bodyText"}"#,
    )
    .then_some(())?;
    Some(c)
}

fn fetch_with_request(req_len: usize, label: &[u8]) -> Result<usize, FetchError> {
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
                    log_info(label);
                }
            }
        }
    }
    if transport_failed {
        return Err(FetchError::Network);
    }
    decode_json_body(&http_out()[..resp_len])
}

fn fetch_post_form(url: &[u8], referer: &[u8], body: &[u8]) -> Result<usize, FetchError> {
    let req_len =
        build_post_form_request(http_req_buf(), url, referer, body).ok_or(FetchError::Network)?;
    fetch_with_request(req_len, b"happymh: http transport error, retrying")
}

fn body_slice(len: usize) -> &'static [u8] {
    unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(BODY_BUF).cast::<u8>(), len) }
}

fn is_verification_page(body: &[u8]) -> bool {
    contains_bytes(body, b"<!doctype html")
        || contains_bytes(body, b"<html")
        || contains_bytes(body, b"cf-mitigated")
        || contains_bytes(body, b"\xe4\xba\xba\xe6\x9c\xba\xe9\xaa\x8c\xe8\xaf\x81")
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

fn extract_object_for_key<'a>(data: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
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
    if pos >= data.len() || data[pos] != b'{' {
        return None;
    }
    extract_balanced(data, pos, b'{', b'}')
}

fn extract_array_for_key<'a>(data: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
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
    if pos >= data.len() || data[pos] != b'[' {
        return None;
    }
    extract_balanced(data, pos, b'[', b']').map(|v| &v[1..v.len() - 1])
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
        } else {
            if b == b'"' {
                in_string = true;
            } else if b == open {
                depth += 1;
            } else if b == close {
                depth -= 1;
                if depth == 0 {
                    return Some(&data[start..pos + 1]);
                }
            }
        }
        pos += 1;
    }
    None
}

fn next_raw_object<'a>(data: &'a [u8], pos: &mut usize) -> Option<&'a [u8]> {
    while *pos < data.len() {
        match data[*pos] {
            b' ' | b'\t' | b'\n' | b'\r' | b',' => *pos += 1,
            b']' => return None,
            b'{' => break,
            _ => return None,
        }
    }
    if *pos >= data.len() {
        return None;
    }
    let obj = extract_balanced(data, *pos, b'{', b'}')?;
    *pos += obj.len();
    Some(obj)
}

fn is_json_true(data: &[u8], key: &[u8]) -> bool {
    let mut pattern = [0u8; 64];
    let needed = key.len() + 8;
    if needed > pattern.len() {
        return false;
    }
    pattern[0] = b'"';
    pattern[1..1 + key.len()].copy_from_slice(key);
    pattern[1 + key.len()] = b'"';
    pattern[2 + key.len()] = b':';
    pattern[3 + key.len()] = b't';
    pattern[4 + key.len()] = b'r';
    pattern[5 + key.len()] = b'u';
    pattern[6 + key.len()] = b'e';
    contains_bytes(data, &pattern[..needed - 1])
}

fn attr_value<'a>(tag: &'a [u8], attr: &[u8]) -> Option<&'a [u8]> {
    let pos = find_subslice(tag, attr)?;
    let mut i = pos + attr.len();
    while i < tag.len() && matches!(tag[i], b' ' | b'\t' | b'\n' | b'\r') {
        i += 1;
    }
    if i >= tag.len() || tag[i] != b'=' {
        return None;
    }
    i += 1;
    while i < tag.len() && matches!(tag[i], b' ' | b'\t' | b'\n' | b'\r') {
        i += 1;
    }
    if i >= tag.len() {
        return None;
    }
    let quote = tag[i];
    if quote != b'"' && quote != b'\'' {
        return None;
    }
    i += 1;
    let start = i;
    while i < tag.len() && tag[i] != quote {
        i += 1;
    }
    if i >= tag.len() {
        None
    } else {
        Some(&tag[start..i])
    }
}

fn strip_tags_to<'a>(src: &[u8], out: &'a mut [u8]) -> &'a [u8] {
    let mut c = 0usize;
    let mut in_tag = false;
    let mut entity = false;
    for &b in src {
        if b == b'<' {
            in_tag = true;
            continue;
        }
        if b == b'>' {
            in_tag = false;
            continue;
        }
        if in_tag {
            continue;
        }
        if b == b'&' {
            entity = true;
            if c < out.len() {
                out[c] = b' ';
                c += 1;
            }
            continue;
        }
        if entity {
            if b == b';' {
                entity = false;
            }
            continue;
        }
        if c < out.len() {
            out[c] = b;
            c += 1;
        }
    }
    trim_ascii(&out[..c])
}

fn find_tag_by_class<'a>(html: &'a [u8], tag: &[u8], class: &[u8]) -> Option<&'a [u8]> {
    let mut pos = 0usize;
    let mut open = [0u8; 16];
    if tag.len() + 1 > open.len() {
        return None;
    }
    open[0] = b'<';
    open[1..1 + tag.len()].copy_from_slice(tag);
    let open = &open[..1 + tag.len()];
    while let Some(rel) = find_subslice(&html[pos..], open) {
        let start = pos + rel;
        let end = find_subslice(&html[start..], b">")? + start + 1;
        let tag_bytes = &html[start..end];
        if contains_bytes(tag_bytes, class) {
            return Some(tag_bytes);
        }
        pos = end;
    }
    None
}

fn find_element_text_by_class<'a>(
    html: &[u8],
    tag: &[u8],
    class: &[u8],
    out: &'a mut [u8],
) -> Option<&'a [u8]> {
    let open_tag = find_tag_by_class(html, tag, class)?;
    let start_offset =
        (open_tag.as_ptr() as usize).wrapping_sub(html.as_ptr() as usize) + open_tag.len();
    let mut close = [0u8; 16];
    if tag.len() + 3 > close.len() {
        return None;
    }
    close[0] = b'<';
    close[1] = b'/';
    close[2..2 + tag.len()].copy_from_slice(tag);
    close[2 + tag.len()] = b'>';
    let close = &close[..3 + tag.len()];
    let end_rel = find_subslice(&html[start_offset..], close)?;
    Some(strip_tags_to(
        &html[start_offset..start_offset + end_rel],
        out,
    ))
}

fn find_img_src_by_class<'a>(html: &'a [u8], class: &[u8]) -> Option<&'a [u8]> {
    find_tag_by_class(html, b"mip-img", class)
        .and_then(|tag| attr_value(tag, b"src"))
        .or_else(|| find_tag_by_class(html, b"img", class).and_then(|tag| attr_value(tag, b"src")))
}

fn manga_url_to_code<'a>(id: &'a [u8]) -> Option<&'a [u8]> {
    let prefix = b"manga:";
    if id.len() > prefix.len() && &id[..prefix.len()] == prefix {
        Some(&id[prefix.len()..])
    } else {
        None
    }
}

fn chapter_id_parts<'a>(id: &'a [u8]) -> Option<(&'a [u8], &'a [u8], &'a [u8])> {
    let prefix = b"chapter:";
    if id.len() <= prefix.len() || &id[..prefix.len()] != prefix {
        return None;
    }
    let rest = &id[prefix.len()..];
    let first = find_subslice(rest, b":")?;
    let code = &rest[..first];
    let tail = &rest[first + 1..];
    let second = find_subslice(tail, b":")?;
    Some((code, &tail[..second], &tail[second + 1..]))
}

fn write_manga_card(payload: &mut [u8], c: &mut usize, obj: &[u8], tag: &[u8]) -> bool {
    let code = extract_json_string(obj, b"manga_code")
        .or_else(|| extract_json_string(obj, b"code"))
        .unwrap_or(b"");
    if code.is_empty() {
        return true;
    }
    let title = extract_json_string(obj, b"name").unwrap_or(code);
    let cover = extract_json_string(obj, b"cover").unwrap_or(b"");
    write_bytes(payload, c, br#"{"id":"manga:"#)
        && append_json_escaped(payload, c, code)
        && write_bytes(payload, c, br#"","title":""#)
        && append_json_escaped(payload, c, title)
        && write_bytes(payload, c, br#"","cover":{"kind":"url","url":""#)
        && append_json_escaped(payload, c, cover)
        && write_bytes(payload, c, br#""},"authors":[],"status":"unknown","contentRating":"safe","description":"","sourceTags":["#)
        && write_bytes(payload, c, b"\"")
        && append_json_escaped(payload, c, tag)
        && write_bytes(payload, c, br#""]}"#)
}

fn write_manga_list_response(
    operation: &str,
    json: &[u8],
    page_num: usize,
    tag: &[u8],
    force_no_more: bool,
) -> u32 {
    let data = extract_object_for_key(json, b"data").unwrap_or(json);
    let items = match extract_array_for_key(data, b"items") {
        Some(v) => v,
        None => {
            if contains_bytes(json, b"<!doctype html") || contains_bytes(json, b"<html") {
                return write_error(operation, "source_error", "site returned verification page");
            }
            return write_error(operation, "parse_error", "missing items");
        }
    };

    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error(operation, "internal_error", "payload overflow");
    }
    let mut pos = 0usize;
    let mut written = 0usize;
    while let Some(obj) = next_raw_object(items, &mut pos) {
        if written >= 500 {
            break;
        }
        if written > 0 && !write_bytes(payload, &mut c, b",") {
            break;
        }
        let before = c;
        if !write_manga_card(payload, &mut c, obj, tag) {
            break;
        }
        if c == before {
            if written > 0 {
                c -= 1;
            }
            continue;
        }
        written += 1;
    }
    let has_more = !force_no_more && !is_json_true(data, b"isEnd") && written > 0;
    let ok = write_bytes(payload, &mut c, br#"],"page":{"nextCursor":"#)
        && if has_more {
            write_bytes(payload, &mut c, b"\"")
                && write_usize(payload, &mut c, page_num + 1)
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

fn build_index_url(
    page: usize,
    order: &[u8],
    genre: &[u8],
    area: &[u8],
    audience: &[u8],
    status: &[u8],
) -> Option<usize> {
    let buf = scratch_a();
    let mut c = 0usize;
    write_bytes(buf, &mut c, BASE_URL).then_some(())?;
    write_bytes(buf, &mut c, b"/apis/c/index?pn=").then_some(())?;
    write_usize(buf, &mut c, page).then_some(())?;
    write_bytes(buf, &mut c, b"&series_status=").then_some(())?;
    append_json_escaped(buf, &mut c, status).then_some(())?;
    write_bytes(buf, &mut c, b"&order=").then_some(())?;
    append_json_escaped(buf, &mut c, order).then_some(())?;
    if !genre.is_empty() {
        write_bytes(buf, &mut c, b"&genre=").then_some(())?;
        append_json_escaped(buf, &mut c, genre).then_some(())?;
    }
    if !area.is_empty() {
        write_bytes(buf, &mut c, b"&area=").then_some(())?;
        append_json_escaped(buf, &mut c, area).then_some(())?;
    }
    if !audience.is_empty() {
        write_bytes(buf, &mut c, b"&audience=").then_some(())?;
        append_json_escaped(buf, &mut c, audience).then_some(())?;
    }
    Some(c)
}

fn fetch_index(
    operation: &str,
    page: usize,
    order: &[u8],
    genre: &[u8],
    area: &[u8],
    audience: &[u8],
    status: &[u8],
    tag: &[u8],
) -> u32 {
    let url_len = match build_index_url(page, order, genre, area, audience, status) {
        Some(n) => n,
        None => return write_error(operation, "internal_error", "url overflow"),
    };
    let url = unsafe {
        core::slice::from_raw_parts(core::ptr::addr_of!(SCRATCH_A).cast::<u8>(), url_len)
    };
    let html_len = match fetch_get(url, Some(b"https://m.happymh.com/latest"), None) {
        Ok(n) => n,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error(operation, c, m);
        }
    };
    write_manga_list_response(operation, body_slice(html_len), page, tag, false)
}

fn run_search(req: &[u8]) -> u32 {
    let query = extract_json_string(req, b"query").unwrap_or(b"");
    let page = request_page(req);
    if query.is_empty() {
        let genre = extract_json_string(req, b"genre").unwrap_or(b"");
        let area = extract_json_string(req, b"area").unwrap_or(b"");
        let audience = extract_json_string(req, b"audience").unwrap_or(b"");
        let status = extract_json_string(req, b"series_status").unwrap_or(b"-1");
        return fetch_index(
            "search",
            page,
            b"last_date",
            genre,
            area,
            audience,
            status,
            b"happymh",
        );
    }

    let body = scratch_b();
    let mut bc = 0usize;
    if !(write_bytes(body, &mut bc, b"searchkey=")
        && write_url_encoded(body, &mut bc, query)
        && write_bytes(body, &mut bc, b"&v=v2.13"))
    {
        return write_error("search", "internal_error", "body overflow");
    }
    let body_bytes =
        unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(SCRATCH_B).cast::<u8>(), bc) };
    let url_buf = scratch_a();
    let mut uc = 0usize;
    if !(write_bytes(
        url_buf,
        &mut uc,
        b"https://m.happymh.com/v2.0/apis/manga/tgssearch?searchkey=",
    ) && write_url_encoded(url_buf, &mut uc, query)
        && write_bytes(url_buf, &mut uc, b"&v=v2.13"))
    {
        return write_error("search", "internal_error", "url overflow");
    }
    let url =
        unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(SCRATCH_A).cast::<u8>(), uc) };
    let mut len = match fetch_get(url, Some(b"https://m.happymh.com/tgsearch"), None) {
        Ok(n) => n,
        Err(_) => 0,
    };
    if len == 0 || is_verification_page(body_slice(len)) {
        len = match fetch_post_form(
            b"https://m.happymh.com/v2.0/apis/manga/ssearch",
            b"https://m.happymh.com/sssearch",
            body_bytes,
        ) {
            Ok(n) => n,
            Err(e) => {
                let (c, m) = fetch_error_code(e);
                return write_error("search", c, m);
            }
        };
    }
    write_manga_list_response("search", body_slice(len), page, b"happymh", true)
}

fn run_get_listings(req: &[u8]) -> u32 {
    let page = request_page(req);
    fetch_index(
        "get_listings",
        page,
        b"views",
        b"",
        b"",
        b"",
        b"-1",
        b"popular",
    )
}

fn run_get_manga_list(req: &[u8]) -> u32 {
    let page = request_page(req);
    let listing = extract_json_string(req, b"listingId").unwrap_or(b"latest");
    let order = if listing == b"popular" {
        b"views" as &[u8]
    } else {
        b"last_date" as &[u8]
    };
    let genre = extract_json_string(req, b"genre").unwrap_or(b"");
    let area = extract_json_string(req, b"area").unwrap_or(b"");
    let audience = extract_json_string(req, b"audience").unwrap_or(b"");
    let status = extract_json_string(req, b"series_status").unwrap_or(b"-1");
    fetch_index(
        "get_manga_list",
        page,
        order,
        genre,
        area,
        audience,
        status,
        listing,
    )
}

fn run_get_manga(req: &[u8]) -> u32 {
    let manga_id = match extract_json_string(req, b"mangaId") {
        Some(v) => v,
        None => return write_error("get_manga", "invalid_request", "missing mangaId"),
    };
    let code = match manga_url_to_code(manga_id) {
        Some(v) => v,
        None => return write_error("get_manga", "invalid_request", "unexpected mangaId"),
    };
    let url_buf = scratch_a();
    let mut uc = 0usize;
    if !(write_bytes(url_buf, &mut uc, BASE_URL)
        && write_bytes(url_buf, &mut uc, b"/manga/")
        && append_json_escaped(url_buf, &mut uc, code))
    {
        return write_error("get_manga", "internal_error", "url overflow");
    }
    let url =
        unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(SCRATCH_A).cast::<u8>(), uc) };
    let html_len = match fetch_get(url, None, None) {
        Ok(n) => n,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_manga", c, m);
        }
    };
    let html = body_slice(html_len);
    let mut title_buf = [0u8; 256];
    let mut author_buf = [0u8; 256];
    let mut desc_buf = [0u8; 2048];
    let title =
        find_element_text_by_class(html, b"h2", b"mg-title", &mut title_buf).unwrap_or(code);
    let cover = find_img_src_by_class(html, b"mg-cover").unwrap_or(b"");
    let author =
        find_element_text_by_class(html, b"p", b"mg-sub-title", &mut author_buf).unwrap_or(b"");
    let desc = match find_element_text_by_class(
        html,
        b"mip-showmore",
        b"manga-introduction",
        &mut desc_buf,
    ) {
        Some(v) => v,
        None => find_element_text_by_class(html, b"mip-showmore", b"showmore", &mut desc_buf)
            .unwrap_or(b""),
    };

    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"manga":{"id":"manga:"#)
        && append_json_escaped(payload, &mut c, code)
        && write_bytes(payload, &mut c, br#"","title":""#)
        && append_json_escaped(payload, &mut c, title)
        && write_bytes(
            payload,
            &mut c,
            br#"","alternateTitles":[],"description":""#,
        )
        && append_json_escaped(payload, &mut c, desc)
        && write_bytes(payload, &mut c, br#"","cover":{"kind":"url","url":""#)
        && append_json_escaped(payload, &mut c, cover)
        && write_bytes(payload, &mut c, br#""},"authors":["#);
    if !ok {
        return write_error("get_manga", "internal_error", "payload overflow");
    }
    if !author.is_empty() {
        let ok_author = write_bytes(payload, &mut c, b"\"")
            && append_json_escaped(payload, &mut c, author)
            && write_bytes(payload, &mut c, b"\"");
        if !ok_author {
            return write_error("get_manga", "internal_error", "payload overflow");
        }
    }
    let ok2 = write_bytes(
        payload,
        &mut c,
        br#"],"artists":[],"status":"unknown","contentRating":"safe","language":"zh","tags":["#,
    ) && write_genre_tags(html, payload, &mut c)
        && write_bytes(payload, &mut c, br#"],"links":[{"kind":"source","url":""#)
        && append_json_escaped(payload, &mut c, url)
        && write_bytes(payload, &mut c, br#""}]}}"#);
    if !ok2 {
        return write_error("get_manga", "internal_error", "payload overflow");
    }
    write_success_payload("get_manga", c)
}

fn write_genre_tags(html: &[u8], payload: &mut [u8], c: &mut usize) -> bool {
    let mut pos = 0usize;
    let mut written = 0usize;
    while let Some(rel) = find_subslice(&html[pos..], b"<a") {
        let start = pos + rel;
        let end = match find_subslice(&html[start..], b">") {
            Some(v) => start + v + 1,
            None => break,
        };
        let tag = &html[start..end];
        if !contains_bytes(tag, b"genre=") && !contains_bytes(tag, b"/latest/") {
            pos = end;
            continue;
        }
        let close = match find_subslice(&html[end..], b"</a>") {
            Some(v) => end + v,
            None => break,
        };
        let text = strip_tags_to(&html[end..close], scratch_b());
        if !text.is_empty() {
            if written > 0 && !write_bytes(payload, c, b",") {
                return false;
            }
            if !(write_bytes(payload, c, b"\"")
                && append_json_escaped(payload, c, text)
                && write_bytes(payload, c, b"\""))
            {
                return false;
            }
            written += 1;
        }
        pos = close + 4;
        if written >= 24 {
            break;
        }
    }
    true
}

fn fetch_chapter_page(code: &[u8], page: usize) -> Result<usize, FetchError> {
    let id = b"1740000000000";
    let url_buf = scratch_a();
    let mut c = 0usize;
    if !(write_bytes(url_buf, &mut c, BASE_URL)
        && write_bytes(url_buf, &mut c, b"/v2.0/apis/manga/chapterByPage?code=")
        && write_url_encoded(url_buf, &mut c, code)
        && write_bytes(url_buf, &mut c, b"&lang=cn&order=asc&page=")
        && write_usize(url_buf, &mut c, page)
        && write_bytes(url_buf, &mut c, b"&_t=")
        && write_bytes(url_buf, &mut c, id))
    {
        return Err(FetchError::Network);
    }
    let url =
        unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(SCRATCH_A).cast::<u8>(), c) };
    fetch_get(url, Some(BASE_URL), Some(id))
}

fn run_get_chapters(req: &[u8]) -> u32 {
    let manga_id = match extract_json_string(req, b"mangaId") {
        Some(v) => v,
        None => return write_error("get_chapters", "invalid_request", "missing mangaId"),
    };
    let code = match manga_url_to_code(manga_id) {
        Some(v) => v,
        None => return write_error("get_chapters", "invalid_request", "unexpected mangaId"),
    };

    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("get_chapters", "internal_error", "payload overflow");
    }

    let mut written = 0usize;
    let mut page = 1usize;
    while page <= 100 {
        let len = match fetch_chapter_page(code, page) {
            Ok(n) => n,
            Err(e) => {
                if written > 0 {
                    break;
                }
                let (ec, m) = fetch_error_code(e);
                return write_error("get_chapters", ec, m);
            }
        };
        let json = body_slice(len);
        let data = match extract_object_for_key(json, b"data") {
            Some(v) => v,
            None => break,
        };
        let items = match extract_array_for_key(data, b"items") {
            Some(v) => v,
            None => break,
        };
        let mut pos = 0usize;
        let mut page_items = 0usize;
        while let Some(obj) = next_raw_object(items, &mut pos) {
            let cid = match extract_json_number(obj, b"id") {
                Some(v) => v,
                None => continue,
            };
            let title = extract_json_string(obj, b"chapterName").unwrap_or(b"Chapter");
            let order = extract_json_number(obj, b"order").unwrap_or(cid);
            if written > 0 && !write_bytes(payload, &mut c, b",") {
                break;
            }
            let ok = write_bytes(payload, &mut c, br#"{"id":"chapter:"#)
                && append_json_escaped(payload, &mut c, code)
                && write_bytes(payload, &mut c, b":")
                && write_usize(payload, &mut c, page)
                && write_bytes(payload, &mut c, b":")
                && append_json_escaped(payload, &mut c, cid)
                && write_bytes(payload, &mut c, br#"","mangaId":"manga:"#)
                && append_json_escaped(payload, &mut c, code)
                && write_bytes(payload, &mut c, br#"","title":""#)
                && append_json_escaped(payload, &mut c, title)
                && write_bytes(payload, &mut c, br#"","chapterNumber":""#)
                && append_json_escaped(payload, &mut c, order)
                && write_bytes(payload, &mut c, br#"","volumeNumber":null,"language":"zh","publishedAt":null,"updatedAt":null,"pageCount":null}"#);
            if !ok {
                return write_error("get_chapters", "internal_error", "payload overflow");
            }
            written += 1;
            page_items += 1;
        }
        let is_end_num = extract_json_number(data, b"isEnd")
            .map(|v| parse_usize(v, 0))
            .unwrap_or(0);
        if is_end_num == 1 || page_items == 0 {
            break;
        }
        page += 1;
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

fn run_get_pages(req: &[u8]) -> u32 {
    let chapter_id = match extract_json_string(req, b"chapterId") {
        Some(v) => v,
        None => return write_error("get_pages", "invalid_request", "missing chapterId"),
    };
    let (code, _page_hint, cid) = match chapter_id_parts(chapter_id) {
        Some(v) => v,
        None => return write_error("get_pages", "invalid_request", "unexpected chapterId"),
    };
    let id = b"1740000000000";
    let url_buf = scratch_a();
    let mut uc = 0usize;
    if !(write_bytes(url_buf, &mut uc, BASE_URL)
        && write_bytes(url_buf, &mut uc, b"/v2.0/apis/manga/reading?code=")
        && write_url_encoded(url_buf, &mut uc, code)
        && write_bytes(url_buf, &mut uc, b"&cid=")
        && write_url_encoded(url_buf, &mut uc, cid)
        && write_bytes(url_buf, &mut uc, b"&v=v4.203411&_t=")
        && write_bytes(url_buf, &mut uc, id))
    {
        return write_error("get_pages", "internal_error", "url overflow");
    }
    let url =
        unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(SCRATCH_A).cast::<u8>(), uc) };
    let ref_len = {
        let r = scratch_b();
        let mut rc = 0usize;
        if !(write_bytes(r, &mut rc, BASE_URL)
            && write_bytes(r, &mut rc, b"/mangaread/")
            && append_json_escaped(r, &mut rc, code)
            && write_bytes(r, &mut rc, b"/")
            && append_json_escaped(r, &mut rc, cid))
        {
            return write_error("get_pages", "internal_error", "referer overflow");
        }
        rc
    };
    let referer = unsafe {
        core::slice::from_raw_parts(core::ptr::addr_of!(SCRATCH_B).cast::<u8>(), ref_len)
    };
    let len = match fetch_get(url, Some(referer), Some(id)) {
        Ok(n) => n,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_pages", c, m);
        }
    };
    let json = body_slice(len);
    let data = match extract_object_for_key(json, b"data") {
        Some(v) => v,
        None => return write_error("get_pages", "parse_error", "missing data"),
    };
    let scans = match extract_array_for_key(data, b"scans") {
        Some(v) => v,
        None => return write_error("get_pages", "parse_error", "missing scans"),
    };
    let payload = payload_buf();
    let mut c = 0usize;
    if !(write_bytes(payload, &mut c, br#"{"chapterId":"#)
        && write_bytes(payload, &mut c, b"\"")
        && append_json_escaped(payload, &mut c, chapter_id)
        && write_bytes(payload, &mut c, br#"","pages":["#))
    {
        return write_error("get_pages", "internal_error", "payload overflow");
    }
    let mut pos = 0usize;
    let mut written = 0usize;
    while let Some(obj) = next_raw_object(scans, &mut pos) {
        let n = extract_json_number(obj, b"n")
            .map(|v| parse_usize(v, 0))
            .unwrap_or(0);
        if n == 1 {
            continue;
        }
        let mut img = extract_json_string(obj, b"url").unwrap_or(b"");
        if img.is_empty() {
            continue;
        }
        let width = extract_json_number(obj, b"width")
            .map(|v| parse_usize(v, 0))
            .unwrap_or(0);
        let height = extract_json_number(obj, b"height")
            .map(|v| parse_usize(v, 0))
            .unwrap_or(0);
        if width > 16383 || height > 16383 {
            if let Some(q) = find_subslice(img, b"?q=") {
                img = &img[..q];
            }
        }
        if written > 0 && !write_bytes(payload, &mut c, b",") {
            break;
        }
        let ok = write_bytes(payload, &mut c, br#"{"id":"page:"#)
            && append_json_escaped(payload, &mut c, code)
            && write_bytes(payload, &mut c, b":")
            && append_json_escaped(payload, &mut c, cid)
            && write_bytes(payload, &mut c, b":")
            && write_usize(payload, &mut c, written)
            && write_bytes(payload, &mut c, br#"","index":"#)
            && write_usize(payload, &mut c, written)
            && write_bytes(payload, &mut c, br#","image":{"kind":"url","url":""#)
            && append_json_escaped(payload, &mut c, img)
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
        && append_json_escaped(payload, &mut c, url)
        && write_bytes(
            payload,
            &mut c,
            br#"","headers":{"Referer":"https://m.happymh.com/"}}"#,
        );
    if !ok {
        return write_error("get_image_request", "internal_error", "payload overflow");
    }
    write_success_payload("get_image_request", c)
}

fn run_get_home(_req: &[u8]) -> u32 {
    let payload = payload_buf();
    let mut c = 0usize;
    let home_json = "{\"sections\":[{\"title\":\"热门漫画\",\"items\":[]},{\"title\":\"最新更新\",\"items\":[]}]}";
    if !(write_bytes(payload, &mut c, home_json.as_bytes())) {
        return write_error("get_home", "internal_error", "payload overflow");
    }
    write_success_payload("get_home", c)
}

fn run_get_filters(_req: &[u8]) -> u32 {
    const FILTERS: &str = "{\"filters\":[{\"id\":\"genre\",\"name\":\"分类\",\"kind\":\"select\",\"options\":[{\"value\":\"\",\"label\":\"全部\"},{\"value\":\"rexue\",\"label\":\"热血\"},{\"value\":\"gedou\",\"label\":\"格斗\"},{\"value\":\"wuxia\",\"label\":\"武侠\"},{\"value\":\"mohuan\",\"label\":\"魔幻\"},{\"value\":\"maoxian\",\"label\":\"冒险\"},{\"value\":\"aiqing\",\"label\":\"爱情\"},{\"value\":\"gaoxiao\",\"label\":\"搞笑\"},{\"value\":\"xiaoyuan\",\"label\":\"校园\"},{\"value\":\"kehuan\",\"label\":\"科幻\"},{\"value\":\"xuanyi\",\"label\":\"悬疑\"},{\"value\":\"lianai\",\"label\":\"恋爱\"},{\"value\":\"dushi\",\"label\":\"都市\"},{\"value\":\"qihuan\",\"label\":\"奇幻\"},{\"value\":\"xuanhuan\",\"label\":\"玄幻\"}],\"default\":\"\"},{\"id\":\"area\",\"name\":\"地区\",\"kind\":\"select\",\"options\":[{\"value\":\"\",\"label\":\"全部\"},{\"value\":\"china\",\"label\":\"内地\"},{\"value\":\"japan\",\"label\":\"日本\"},{\"value\":\"hongkong\",\"label\":\"港台\"},{\"value\":\"europe\",\"label\":\"欧美\"},{\"value\":\"korea\",\"label\":\"韩国\"},{\"value\":\"other\",\"label\":\"其他\"}],\"default\":\"\"},{\"id\":\"audience\",\"name\":\"受众\",\"kind\":\"select\",\"options\":[{\"value\":\"\",\"label\":\"全部\"},{\"value\":\"shaonian\",\"label\":\"少年\"},{\"value\":\"shaonv\",\"label\":\"少女\"},{\"value\":\"qingnian\",\"label\":\"青年\"},{\"value\":\"BL\",\"label\":\"BL\"},{\"value\":\"GL\",\"label\":\"GL\"}],\"default\":\"\"},{\"id\":\"series_status\",\"name\":\"状态\",\"kind\":\"select\",\"options\":[{\"value\":\"-1\",\"label\":\"全部\"},{\"value\":\"0\",\"label\":\"连载中\"},{\"value\":\"1\",\"label\":\"完结\"}],\"default\":\"-1\"}]}";
    let bytes = FILTERS.as_bytes();
    if bytes.len() > payload_buf().len() {
        return write_error("get_filters", "internal_error", "payload overflow");
    }
    payload_buf()[..bytes.len()].copy_from_slice(bytes);
    write_success_payload("get_filters", bytes.len())
}

fn run_get_settings(_req: &[u8]) -> u32 {
    const SETTINGS: &str = "{\"settings\":[{\"id\":\"customUserAgent\",\"name\":\"User Agent\",\"kind\":\"text\",\"default\":\"\"},{\"id\":\"cookies\",\"name\":\"Cookies (from WebView)\",\"kind\":\"text\",\"default\":\"\",\"hint\":\"Open happymh.com in browser, pass CF verification, then copy cookies here\"}]}";
    let bytes = SETTINGS.as_bytes();
    if bytes.len() > payload_buf().len() {
        return write_error("get_settings", "internal_error", "payload overflow");
    }
    payload_buf()[..bytes.len()].copy_from_slice(bytes);
    write_success_payload("get_settings", bytes.len())
}

#[no_mangle]
pub extern "C" fn koma_source_init(_manifest_ptr: u32, manifest_len: u32) -> i32 {
    log_info(b"happymh source init");
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
    log_info(b"happymh search");
    run_search(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga", "invalid_request", "empty request"),
    };
    log_info(b"happymh get_manga");
    run_get_manga(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_chapters(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_chapters", "invalid_request", "empty request"),
    };
    log_info(b"happymh get_chapters");
    run_get_chapters(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_pages(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_pages", "invalid_request", "empty request"),
    };
    log_info(b"happymh get_pages");
    run_get_pages(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_listings(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_listings", "invalid_request", "empty request"),
    };
    log_info(b"happymh get_listings");
    run_get_listings(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga_list(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga_list", "invalid_request", "empty request"),
    };
    log_info(b"happymh get_manga_list");
    run_get_manga_list(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_home(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_home", "invalid_request", "empty request"),
    };
    log_info(b"happymh get_home");
    run_get_home(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_filters(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_filters", "invalid_request", "empty request"),
    };
    log_info(b"happymh get_filters");
    run_get_filters(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_settings(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_settings", "invalid_request", "empty request"),
    };
    log_info(b"happymh get_settings");
    run_get_settings(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_image_request(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_image_request", "invalid_request", "empty request"),
    };
    log_info(b"happymh get_image_request");
    run_get_image_request(req)
}

#[no_mangle]
pub extern "C" fn koma_source_free(result_ptr: u32) {
    response_buffer().free(result_ptr)
}
