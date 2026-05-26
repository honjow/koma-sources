#![no_std]

extern crate koma_source_sdk;

use koma_source_sdk::host::{http_request, log_info};
use koma_source_sdk::json_utils::{
    append_json_escaped, append_json_unescaped_then_escaped, contains_bytes, extract_json_number,
    extract_json_string, find_subslice, write_bytes, write_usize, JsonArrayIter,
};
use koma_source_sdk::result::ResultBuffer;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};

const API_URL: &[u8] = b"https://komiic.com/api/query";
const IMAGE_BASE: &[u8] = b"https://komiic.com/api/image/";
koma_source_sdk::koma_source_buffers! {
    payload: 1024 * 1024,
    http_out: 2 * 1024 * 1024,
    body: 2 * 1024 * 1024,
    http_req: 64 * 1024,
    scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();
const JSON_BUF_CAP: usize = 2 * 1024 * 1024;
const GQL_BODY_CAP: usize = 32 * 1024;
const PAGE_SIZE: usize = 30;

static mut JSON_BUF: [u8; JSON_BUF_CAP] = [0; JSON_BUF_CAP];
static mut GQL_BODY_BUF: [u8; GQL_BODY_CAP] = [0; GQL_BODY_CAP];

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.komiic.koma",
    name: "Komiic",
    version: "0.1.0",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "Komiic GraphQL API source.",
    content_rating: "nsfw",
};

const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: true,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: true,
    manga_list: true,
    home: true,
    filters: false,
    settings: false,
    image_request: true,
    credentials: false,
};

const QUERY_POPULAR: &[u8] = b"query hotComics($pagination: Pagination!) { comics: hotComics(pagination: $pagination) { id title description status imageUrl authors { id name } categories { id name } } }";
const QUERY_SEARCH: &[u8] = b"query searchComicAndAuthorQuery($keyword: String!) { searchComicsAndAuthors(keyword: $keyword) { comics { id title description status imageUrl authors { id name } categories { id name } } } }";
const QUERY_DETAIL: &[u8] = b"query chapterByComicId($comicId: ID!) { comicById(comicId: $comicId) { id title description status imageUrl authors { id name } categories { id name } } chaptersByComicId(comicId: $comicId) { id serial type size dateCreated } }";
const QUERY_PAGES: &[u8] = b"query imagesByChapterId($chapterId: ID!) { imagesByChapterId(chapterId: $chapterId) { kid } }";

#[cfg(not(test))]

    unsafe { &mut *core::ptr::addr_of_mut!(RESPONSE) }
}

fn json_slice(len: usize) -> &'static [u8] {
    unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(JSON_BUF) as *const u8, len) }
}

fn read_request<'a>(req_ptr: u32, req_len: u32) -> Option<&'a [u8]> {
    if req_ptr == 0 || req_len == 0 {
        return None;
    }
    Some(unsafe { core::slice::from_raw_parts(req_ptr as *const u8, req_len as usize) })
}

fn parse_usize(bytes: &[u8]) -> usize {
    let mut n = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if b < b'0' || b > b'9' {
            break;
        }
        n = n.saturating_mul(10).saturating_add((b - b'0') as usize);
        i += 1;
    }
    n
}

fn request_page(req: &[u8]) -> usize {
    let n = extract_json_number(req, b"page").map(parse_usize).unwrap_or(1);
    if n == 0 { 1 } else { n }
}

fn strip_prefix<'a>(value: &'a [u8], prefix: &[u8]) -> Option<&'a [u8]> {
    if value.len() <= prefix.len() || &value[..prefix.len()] != prefix {
        None
    } else {
        Some(&value[prefix.len()..])
    }
}

fn decode_body_text(resp: &[u8]) -> Option<usize> {
    let marker = b"\"bodyText\":\"";
    let mut i = find_subslice(resp, marker)? + marker.len();
    let out = json_buf();
    let mut c = 0usize;
    while i < resp.len() {
        let b = resp[i];
        if b == b'\\' && i + 1 < resp.len() {
            let next = resp[i + 1];
            let unescaped = match next {
                b'"' => b'"',
                b'\\' => b'\\',
                b'/' => b'/',
                b'n' => b'\n',
                b'r' => b'\r',
                b't' => b'\t',
                b'b' => 0x08,
                b'f' => 0x0c,
                _ => next,
            };
            if c >= out.len() {
                return None;
            }
            out[c] = unescaped;
            c += 1;
            i += 2;
            continue;
        }
        if b == b'"' {
            return Some(c);
        }
        if c >= out.len() {
            return None;
        }
        out[c] = b;
        c += 1;
        i += 1;
    }
    None
}

#[derive(Copy, Clone)]
enum FetchError {
    Network,
    NotFound,
    RateLimit,
    ClientError,
    ServerError,
    Graphql,
}

fn post_graphql(body: &[u8]) -> Result<&'static [u8], FetchError> {
    let req_len = build_post_request(http_req_buf(), body).ok_or(FetchError::Network)?;
    let mut resp_len = 0usize;
    let mut failed = true;
    let mut attempt = 0u8;
    while attempt < 3 {
        let req = &http_req_buf()[..req_len];
        match http_request(req, http_out()) {
            Ok(n) => {
                resp_len = n;
                failed = false;
                break;
            }
            Err(_) => {
                if attempt < 2 {
                    log_info(b"komiic: http transport error, retrying");
                }
            }
        }
        attempt += 1;
    }
    if failed {
        return Err(FetchError::Network);
    }

    let resp = &http_out()[..resp_len];
    if !contains_bytes(resp, br#""ok":true"#) {
        let err = if let Some(code_bytes) = extract_json_number(resp, b"status") {
            match parse_status_code(code_bytes) {
                404 => FetchError::NotFound,
                429 => FetchError::RateLimit,
                400..=499 => FetchError::ClientError,
                500..=599 => FetchError::ServerError,
                _ => FetchError::Network,
            }
        } else {
            FetchError::Network
        };
        return Err(err);
    }

    let len = decode_body_text(resp).ok_or(FetchError::Network)?;
    let body = json_slice(len);
    if contains_bytes(body, br#""errors":["#) || contains_bytes(body, br#""errors":[{"#) {
        return Err(FetchError::Graphql);
    }
    Ok(body)
}

fn build_search_body(query: &[u8]) -> Option<usize> {
    let dst = gql_body_buf();
    let mut c = 0usize;
    write_bytes(dst, &mut c, br#"{"query":""#).then_some(())?;
    append_json_escaped(dst, &mut c, QUERY_SEARCH).then_some(())?;
    write_bytes(dst, &mut c, br#"","variables":{"keyword":""#).then_some(())?;
    append_json_escaped(dst, &mut c, query).then_some(())?;
    write_bytes(dst, &mut c, br#""}}"#).then_some(())?;
    Some(c)
}

fn build_popular_body(page: usize) -> Option<usize> {
    let offset = page.saturating_sub(1).saturating_mul(PAGE_SIZE);
    let dst = gql_body_buf();
    let mut c = 0usize;
    write_bytes(dst, &mut c, br#"{"query":""#).then_some(())?;
    append_json_escaped(dst, &mut c, QUERY_POPULAR).then_some(())?;
    write_bytes(dst, &mut c, br#"","variables":{"pagination":{"offset":"#).then_some(())?;
    write_usize(dst, &mut c, offset).then_some(())?;
    write_bytes(dst, &mut c, br#","orderBy":"MONTH_VIEWS","status":"","asc":false,"limit":"#).then_some(())?;
    write_usize(dst, &mut c, PAGE_SIZE).then_some(())?;
    write_bytes(dst, &mut c, br#","sexyLevel":null},"categoryId":[]}}"#).then_some(())?;
    Some(c)
}

fn build_detail_body(id: &[u8]) -> Option<usize> {
    let dst = gql_body_buf();
    let mut c = 0usize;
    write_bytes(dst, &mut c, br#"{"query":""#).then_some(())?;
    append_json_escaped(dst, &mut c, QUERY_DETAIL).then_some(())?;
    write_bytes(dst, &mut c, br#"","variables":{"comicId":""#).then_some(())?;
    append_json_escaped(dst, &mut c, id).then_some(())?;
    write_bytes(dst, &mut c, br#""}}"#).then_some(())?;
    Some(c)
}

fn build_pages_body(id: &[u8]) -> Option<usize> {
    let dst = gql_body_buf();
    let mut c = 0usize;
    write_bytes(dst, &mut c, br#"{"query":""#).then_some(())?;
    append_json_escaped(dst, &mut c, QUERY_PAGES).then_some(())?;
    write_bytes(dst, &mut c, br#"","variables":{"chapterId":""#).then_some(())?;
    append_json_escaped(dst, &mut c, id).then_some(())?;
    write_bytes(dst, &mut c, br#""}}"#).then_some(())?;
    Some(c)
}

fn find_object_after_key<'a>(data: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let mut pattern = [0u8; 64];
    let needed = key.len() + 4;
    if needed > pattern.len() {
        return None;
    }
    pattern[0] = b'"';
    pattern[1..1 + key.len()].copy_from_slice(key);
    pattern[1 + key.len()] = b'"';
    pattern[2 + key.len()] = b':';
    pattern[3 + key.len()] = b'{';
    let start = find_subslice(data, &pattern[..needed])? + needed - 1;
    let mut i = start;
    let mut depth = 0i32;
    let mut in_string = false;
    while i < data.len() {
        let b = data[i];
        if in_string {
            if b == b'\\' {
                i += 1;
            } else if b == b'"' {
                in_string = false;
            }
        } else {
            match b {
                b'"' => in_string = true,
                b'{' | b'[' => depth += 1,
                b'}' | b']' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(&data[start..i + 1]);
                    }
                }
                _ => {}
            }
        }
        i += 1;
    }
    None
}

fn write_names_from_array(payload: &mut [u8], c: &mut usize, obj: &[u8], key: &[u8]) -> bool {
    if let Some(mut iter) = JsonArrayIter::new(obj, key) {
        let mut count = 0usize;
        while let Some(item) = iter.next_object() {
            let Some(name) = extract_json_string(item, b"name") else {
                continue;
            };
            if count > 0 && !write_bytes(payload, c, b",") {
                return false;
            }
            if !(write_bytes(payload, c, b"\"")
                && append_json_unescaped_then_escaped(payload, c, name)
                && write_bytes(payload, c, b"\""))
            {
                return false;
            }
            count += 1;
        }
    }
    true
}

fn write_manga_card(payload: &mut [u8], c: &mut usize, obj: &[u8]) -> bool {
    let id = match extract_json_string(obj, b"id") {
        Some(v) => v,
        None => return true,
    };
    let title = extract_json_string(obj, b"title").unwrap_or(b"Unknown");
    let cover = extract_json_string(obj, b"imageUrl").unwrap_or(b"");
    let status = normalize_status(extract_json_string(obj, b"status").unwrap_or(b""));

    write_bytes(payload, c, br#"{"id":"kic:"#)
        && append_json_escaped(payload, c, id)
        && write_bytes(payload, c, br#"","title":""#)
        && append_json_unescaped_then_escaped(payload, c, title)
        && write_bytes(payload, c, br#"","cover":{"kind":"url","url":""#)
        && append_json_unescaped_then_escaped(payload, c, cover)
        && write_bytes(payload, c, br#""},"authors":["#)
        && write_names_from_array(payload, c, obj, b"authors")
        && write_bytes(payload, c, br#"],"status":""#)
        && write_bytes(payload, c, status)
        && write_bytes(payload, c, br#"","contentRating":"nsfw","sourceTags":["komiic"]}"#)
}

fn normalize_status(status: &[u8]) -> &'static [u8] {
    if status == b"ONGOING" {
        b"ongoing"
    } else if status == b"END" {
        b"completed"
    } else {
        b"unknown"
    }
}

fn write_manga_list(operation: &str, api_json: &[u8]) -> u32 {
    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error(operation, "internal_error", "overflow");
    }
    let mut iter = match JsonArrayIter::new(api_json, b"comics") {
        Some(v) => v,
        None => return write_error(operation, "parse_error", "missing comics"),
    };
    let mut written = 0usize;
    while let Some(obj) = iter.next_object() {
        if written >= PAGE_SIZE {
            break;
        }
        if written > 0 && !write_bytes(payload, &mut c, b",") {
            return write_error(operation, "internal_error", "overflow");
        }
        if !write_manga_card(payload, &mut c, obj) {
            return write_error(operation, "internal_error", "overflow");
        }
        written += 1;
    }
    let has_more = if written == PAGE_SIZE { b"true" as &[u8] } else { b"false" as &[u8] };
    if !(write_bytes(payload, &mut c, br#"],"page":{"nextCursor":null,"hasMore":"#)
        && write_bytes(payload, &mut c, has_more)
        && write_bytes(payload, &mut c, br#"}}"#))
    {
        return write_error(operation, "internal_error", "overflow");
    }
    write_success_payload(operation, c)
}

fn run_search(req: &[u8]) -> u32 {
    let query = match extract_json_string(req, b"query") {
        Some(v) => v,
        None => return run_popular_list("search", req),
    };
    if query.is_empty() {
        return run_popular_list("search", req);
    }
    let body_len = match build_search_body(query) {
        Some(v) => v,
        None => return write_error("search", "internal_error", "body overflow"),
    };
    let body = &gql_body_buf()[..body_len];
    let api_json = match post_graphql(body) {
        Ok(v) => v,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("search", c, m);
        }
    };
    write_manga_list("search", api_json)
}

fn run_popular_list(operation: &str, req: &[u8]) -> u32 {
    let page = request_page(req);
    let body_len = match build_popular_body(page) {
        Some(v) => v,
        None => return write_error(operation, "internal_error", "body overflow"),
    };
    let body = &gql_body_buf()[..body_len];
    let api_json = match post_graphql(body) {
        Ok(v) => v,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error(operation, c, m);
        }
    };
    write_manga_list(operation, api_json)
}

fn run_get_manga_list(req: &[u8]) -> u32 {
    run_popular_list("get_manga_list", req)
}

fn run_get_listings(req: &[u8]) -> u32 {
    run_popular_list("get_listings", req)
}

fn run_get_home(_req: &[u8]) -> u32 {
    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"sections":[{"id":"popular","title":"Popular","items":[]}]}"#) {
        return write_error("get_home", "internal_error", "overflow");
    }
    write_success_payload("get_home", c)
}

fn run_get_manga(req: &[u8]) -> u32 {
    let manga_id = match extract_json_string(req, b"mangaId") {
        Some(v) => v,
        None => return write_error("get_manga", "invalid_request", "missing mangaId"),
    };
    let id = match strip_prefix(manga_id, b"kic:") {
        Some(v) => v,
        None => return write_error("get_manga", "invalid_request", "bad mangaId"),
    };
    let body_len = match build_detail_body(id) {
        Some(v) => v,
        None => return write_error("get_manga", "internal_error", "body overflow"),
    };
    let body = &gql_body_buf()[..body_len];
    let api_json = match post_graphql(body) {
        Ok(v) => v,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_manga", c, m);
        }
    };
    let obj = match find_object_after_key(api_json, b"comicById") {
        Some(v) => v,
        None => return write_error("get_manga", "parse_error", "missing comicById"),
    };
    let title = extract_json_string(obj, b"title").unwrap_or(b"Unknown");
    let desc = extract_json_string(obj, b"description").unwrap_or(b"");
    let cover = extract_json_string(obj, b"imageUrl").unwrap_or(b"");
    let status = normalize_status(extract_json_string(obj, b"status").unwrap_or(b""));
    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"manga":{"id":"kic:"#)
        && append_json_escaped(payload, &mut c, id)
        && write_bytes(payload, &mut c, br#"","title":""#)
        && append_json_unescaped_then_escaped(payload, &mut c, title)
        && write_bytes(payload, &mut c, br#"","alternateTitles":[],"description":""#)
        && append_json_unescaped_then_escaped(payload, &mut c, desc)
        && write_bytes(payload, &mut c, br#"","cover":{"kind":"url","url":""#)
        && append_json_unescaped_then_escaped(payload, &mut c, cover)
        && write_bytes(payload, &mut c, br#""},"authors":["#)
        && write_names_from_array(payload, &mut c, obj, b"authors")
        && write_bytes(payload, &mut c, br#"],"artists":[],"status":""#)
        && write_bytes(payload, &mut c, status)
        && write_bytes(payload, &mut c, br#"","contentRating":"nsfw","language":"zh","tags":["#)
        && write_names_from_array(payload, &mut c, obj, b"categories")
        && write_bytes(payload, &mut c, br#"],"links":[{"kind":"source","url":"https://komiic.com/comic/"#)
        && append_json_escaped(payload, &mut c, id)
        && write_bytes(payload, &mut c, br#""}]}}"#);
    if !ok {
        return write_error("get_manga", "internal_error", "overflow");
    }
    write_success_payload("get_manga", c)
}

fn run_get_chapters(req: &[u8]) -> u32 {
    let manga_id = match extract_json_string(req, b"mangaId") {
        Some(v) => v,
        None => return write_error("get_chapters", "invalid_request", "missing mangaId"),
    };
    let id = match strip_prefix(manga_id, b"kic:") {
        Some(v) => v,
        None => return write_error("get_chapters", "invalid_request", "bad mangaId"),
    };
    let body_len = match build_detail_body(id) {
        Some(v) => v,
        None => return write_error("get_chapters", "internal_error", "body overflow"),
    };
    let body = &gql_body_buf()[..body_len];
    let api_json = match post_graphql(body) {
        Ok(v) => v,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_chapters", c, m);
        }
    };
    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("get_chapters", "internal_error", "overflow");
    }
    let mut iter = match JsonArrayIter::new(api_json, b"chaptersByComicId") {
        Some(v) => v,
        None => return write_error("get_chapters", "parse_error", "missing chaptersByComicId"),
    };
    let mut written = 0usize;
    while let Some(obj) = iter.next_object() {
        let ch_id = match extract_json_string(obj, b"id") {
            Some(v) => v,
            None => continue,
        };
        let serial = extract_json_string(obj, b"serial").unwrap_or(b"");
        let typ = extract_json_string(obj, b"type").unwrap_or(b"chapter");
        let size = extract_json_number(obj, b"size").unwrap_or(b"0");
        let date = extract_json_string(obj, b"dateCreated").unwrap_or(b"");
        if written > 0 && !write_bytes(payload, &mut c, b",") {
            return write_error("get_chapters", "internal_error", "overflow");
        }
        let suffix = if typ == b"book" { b"\xE5\x8D\xB7" as &[u8] } else { b"\xE8\xA9\xB1" as &[u8] };
        let scanlator = if typ == b"book" { b"\xE5\x96\xAE\xE8\xA1\x8C\xE6\x9C\xAC" as &[u8] } else { b"\xE9\x80\xA3\xE8\xBC\x89" as &[u8] };
        let ok = write_bytes(payload, &mut c, br#"{"id":"kic-ch:"#)
            && append_json_escaped(payload, &mut c, ch_id)
            && write_bytes(payload, &mut c, br#"","mangaId":"kic:"#)
            && append_json_escaped(payload, &mut c, id)
            && write_bytes(payload, &mut c, br#"","title":""#)
            && append_json_unescaped_then_escaped(payload, &mut c, serial)
            && write_bytes(payload, &mut c, suffix)
            && write_bytes(payload, &mut c, b"\xEF\xBC\x88")
            && write_bytes(payload, &mut c, size)
            && write_bytes(payload, &mut c, b"P\xEF\xBC\x89")
            && write_bytes(payload, &mut c, br#"","chapterNumber":""#)
            && append_json_escaped(payload, &mut c, serial)
            && write_bytes(payload, &mut c, br#"","volumeNumber":null,"language":"zh","scanlator":""#)
            && write_bytes(payload, &mut c, scanlator)
            && write_bytes(payload, &mut c, br#"","publishedAt":""#)
            && append_json_escaped(payload, &mut c, date)
            && write_bytes(payload, &mut c, br#"","updatedAt":null,"pageCount":"#)
            && write_bytes(payload, &mut c, size)
            && write_bytes(payload, &mut c, b"}");
        if !ok {
            return write_error("get_chapters", "internal_error", "overflow");
        }
        written += 1;
    }
    if !write_bytes(payload, &mut c, br#"],"page":{"nextCursor":null,"hasMore":false}}"#) {
        return write_error("get_chapters", "internal_error", "overflow");
    }
    write_success_payload("get_chapters", c)
}

fn run_get_pages(req: &[u8]) -> u32 {
    let chapter_id = match extract_json_string(req, b"chapterId") {
        Some(v) => v,
        None => return write_error("get_pages", "invalid_request", "missing chapterId"),
    };
    let id = match strip_prefix(chapter_id, b"kic-ch:") {
        Some(v) => v,
        None => return write_error("get_pages", "invalid_request", "bad chapterId"),
    };
    let body_len = match build_pages_body(id) {
        Some(v) => v,
        None => return write_error("get_pages", "internal_error", "body overflow"),
    };
    let body = &gql_body_buf()[..body_len];
    let api_json = match post_graphql(body) {
        Ok(v) => v,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_pages", c, m);
        }
    };
    let payload = payload_buf();
    let mut c = 0usize;
    if !(write_bytes(payload, &mut c, br#"{"chapterId":"kic-ch:"#)
        && append_json_escaped(payload, &mut c, id)
        && write_bytes(payload, &mut c, br#"","pages":["#))
    {
        return write_error("get_pages", "internal_error", "overflow");
    }
    let mut iter = match JsonArrayIter::new(api_json, b"imagesByChapterId") {
        Some(v) => v,
        None => return write_error("get_pages", "parse_error", "missing imagesByChapterId"),
    };
    let mut index = 0usize;
    while let Some(obj) = iter.next_object() {
        let kid = match extract_json_string(obj, b"kid") {
            Some(v) => v,
            None => continue,
        };
        if index > 0 && !write_bytes(payload, &mut c, b",") {
            return write_error("get_pages", "internal_error", "overflow");
        }
        let ok = write_bytes(payload, &mut c, br#"{"id":"page:"#)
            && write_usize(payload, &mut c, index)
            && write_bytes(payload, &mut c, br#"","index":"#)
            && write_usize(payload, &mut c, index)
            && write_bytes(payload, &mut c, br#","image":{"kind":"url","url":""#)
            && write_bytes(payload, &mut c, IMAGE_BASE)
            && append_json_escaped(payload, &mut c, kid)
            && write_bytes(payload, &mut c, br#""}}"#);
        if !ok {
            return write_error("get_pages", "internal_error", "overflow");
        }
        index += 1;
    }
    if !write_bytes(payload, &mut c, b"]}") {
        return write_error("get_pages", "internal_error", "overflow");
    }
    write_success_payload("get_pages", c)
}

fn run_get_image_request(req: &[u8]) -> u32 {
    let url = match extract_json_string(req, b"url") {
        Some(v) => v,
        None => return write_error("get_image_request", "invalid_request", "missing url"),
    };
    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"url":""#)
        && append_json_escaped(payload, &mut c, url)
        && write_bytes(payload, &mut c, br#"","headers":{"accept":"image/avif,image/webp,image/apng,image/svg+xml,image/*,*/*;q=0.8","referer":"https://komiic.com/"}}"#);
    if !ok {
        return write_error("get_image_request", "internal_error", "overflow");
    }
    write_success_payload("get_image_request", c)
}

#[no_mangle]
pub extern "C" fn koma_source_init(_manifest_ptr: u32, manifest_len: u32) -> i32 {
    if manifest_len > 0 { 0 } else { -1 }
}

#[no_mangle]
pub extern "C" fn koma_source_info() -> u32 {
    response_buffer().write_source_metadata(&SOURCE_INFO, &SOURCE_CAPS)
}

#[no_mangle]
pub extern "C" fn koma_source_search(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(v) => v,
        None => return write_error("search", "invalid_request", "empty"),
    };
    log_info(b"komiic search");
    run_search(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(v) => v,
        None => return write_error("get_manga", "invalid_request", "empty"),
    };
    log_info(b"komiic get_manga");
    run_get_manga(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_chapters(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(v) => v,
        None => return write_error("get_chapters", "invalid_request", "empty"),
    };
    log_info(b"komiic get_chapters");
    run_get_chapters(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_pages(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(v) => v,
        None => return write_error("get_pages", "invalid_request", "empty"),
    };
    log_info(b"komiic get_pages");
    run_get_pages(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_listings(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(v) => v,
        None => return write_error("get_listings", "invalid_request", "empty"),
    };
    log_info(b"komiic get_listings");
    run_get_listings(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga_list(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(v) => v,
        None => return write_error("get_manga_list", "invalid_request", "empty"),
    };
    log_info(b"komiic get_manga_list");
    run_get_manga_list(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_home(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(v) => v,
        None => return write_error("get_home", "invalid_request", "empty"),
    };
    log_info(b"komiic get_home");
    run_get_home(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_image_request(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(v) => v,
        None => return write_error("get_image_request", "invalid_request", "empty"),
    };
    log_info(b"komiic get_image_request");
    run_get_image_request(req)
}

#[no_mangle]
pub extern "C" fn koma_source_free(result_ptr: u32) {
    response_buffer().free(result_ptr)
}
