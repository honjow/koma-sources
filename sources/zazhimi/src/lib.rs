#![no_std]

extern crate koma_source_sdk;

use koma_source_sdk::host::{self, http_request, log_info};
use koma_source_sdk::json_utils::{
    append_json_escaped, append_json_unescaped_then_escaped, contains_bytes, extract_json_number,
    extract_json_string, write_bytes, write_url_encoded, write_usize, JsonArrayIter,
};
use koma_source_sdk::result::ResultBuffer;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{build_get_request, decode_json_body_into, fetch_error_code, FetchError};

const API_BASE: &[u8] = b"https://android2026.zazhimi.net/api";
const SITE_BASE: &[u8] = b"https://www.zazhimi.net";
koma_source_sdk::koma_source_buffers! {
    payload: 1024 * 1024,
    http_out: 2 * 1024 * 1024,
    body: 2 * 1024 * 1024,
    http_req: 4096,
    scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();
const URL_CAP: usize = 2048;
const DEFAULT_LIMIT: usize = 20;

static mut URL_BUF: [u8; URL_CAP] = [0; URL_CAP];

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.zazhimi.koma",
    name: "杂志迷",
    version: "0.1.0",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "杂志迷 JSON API magazine source.",
    content_rating: "safe",
};

const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: true,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: false,
    manga_list: true,
    home: false,
    filters: false,
    settings: false,
    image_request: false,
    credentials: false,
};

fn body_slice(len: usize) -> &'static [u8] {
    unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(BODY_BUF) as *const u8, len) }
}

fn read_request<'a>(req_ptr: u32, req_len: u32) -> Option<&'a [u8]> {
    if req_ptr == 0 || req_len == 0 {
        return None;
    }
    Some(unsafe { core::slice::from_raw_parts(req_ptr as *const u8, req_len as usize) })
}

fn parse_usize(bytes: &[u8]) -> usize {
    let mut n = 0usize;
    for &b in bytes {
        if b < b'0' || b > b'9' {
            break;
        }
        n = n.saturating_mul(10).saturating_add((b - b'0') as usize);
    }
    n
}
fn request_page(req: &[u8]) -> usize {
    let page = extract_json_number(req, b"page")
        .map(parse_usize)
        .unwrap_or(1);
    if page == 0 {
        1
    } else {
        page
    }
}
fn request_limit(req: &[u8]) -> usize {
    let limit = extract_json_number(req, b"limit")
        .map(parse_usize)
        .unwrap_or(DEFAULT_LIMIT);
    if limit == 0 {
        DEFAULT_LIMIT
    } else if limit > 100 {
        100
    } else {
        limit
    }
}

fn fetch_json(url: &[u8]) -> Result<usize, FetchError> {
    let headers = [(b"User-Agent" as &[u8], b"ZaZhiMi_6.0.0" as &[u8])];
    let req_len = build_get_request(http_req_buf(), url, Some(SITE_BASE), &headers)
        .ok_or(FetchError::Network)?;
    let resp_len =
        http_request(&http_req_buf()[..req_len], http_out()).map_err(|_| FetchError::Network)?;
    decode_json_body_into(&http_out()[..resp_len], body_buf())
}

fn build_index_url(page: usize, limit: usize) -> Option<usize> {
    let mut c = 0usize;
    write_bytes(url_buf(), &mut c, API_BASE).then_some(())?;
    write_bytes(url_buf(), &mut c, b"/index.php?p=").then_some(())?;
    write_usize(url_buf(), &mut c, page).then_some(())?;
    write_bytes(url_buf(), &mut c, b"&s=").then_some(())?;
    write_usize(url_buf(), &mut c, limit).then_some(())?;
    Some(c)
}
fn build_search_url(query: &[u8], page: usize, limit: usize) -> Option<usize> {
    let mut c = 0usize;
    write_bytes(url_buf(), &mut c, API_BASE).then_some(())?;
    write_bytes(url_buf(), &mut c, b"/search.php?k=").then_some(())?;
    write_url_encoded(url_buf(), &mut c, query).then_some(())?;
    write_bytes(url_buf(), &mut c, b"&p=").then_some(())?;
    write_usize(url_buf(), &mut c, page).then_some(())?;
    write_bytes(url_buf(), &mut c, b"&s=").then_some(())?;
    write_usize(url_buf(), &mut c, limit).then_some(())?;
    Some(c)
}
fn build_beauty_list_url(page: usize, limit: usize) -> Option<usize> {
    let mut c = 0usize;
    write_bytes(url_buf(), &mut c, API_BASE).then_some(())?;
    write_bytes(url_buf(), &mut c, b"/lists.php?c=8&m=&p=").then_some(())?;
    write_usize(url_buf(), &mut c, page).then_some(())?;
    write_bytes(url_buf(), &mut c, b"&s=").then_some(())?;
    write_usize(url_buf(), &mut c, limit).then_some(())?;
    Some(c)
}
fn build_show_url(id: &[u8]) -> Option<usize> {
    let mut c = 0usize;
    write_bytes(url_buf(), &mut c, API_BASE).then_some(())?;
    write_bytes(url_buf(), &mut c, b"/show.php?a=").then_some(())?;
    write_url_encoded(url_buf(), &mut c, id).then_some(())?;
    Some(c)
}

fn first_word_len(bytes: &[u8]) -> usize {
    let mut i = 0usize;
    while i < bytes.len() && !matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r') {
        i += 1;
    }
    i
}

fn write_cover(payload: &mut [u8], c: &mut usize, cover: &[u8]) -> bool {
    if cover.is_empty() {
        write_bytes(payload, c, br#"{"kind":"none"}"#)
    } else {
        write_bytes(payload, c, br#"{"kind":"url","url":""#)
            && append_json_unescaped_then_escaped(payload, c, cover)
            && write_bytes(payload, c, br#""}"#)
    }
}

fn write_manga_card(payload: &mut [u8], c: &mut usize, obj: &[u8], tag: &[u8]) -> bool {
    let id = match extract_json_string(obj, b"magId") {
        Some(v) => v,
        None => return false,
    };
    let title = extract_json_string(obj, b"magName").unwrap_or(id);
    let cover = extract_json_string(obj, b"magCover")
        .or_else(|| extract_json_string(obj, b"thumbPic"))
        .unwrap_or(b"");
    let author_len = first_word_len(title);
    let author = if author_len > 0 {
        &title[..author_len]
    } else {
        title
    };

    write_bytes(payload, c, br#"{"id":""#)
        && append_json_unescaped_then_escaped(payload, c, id)
        && write_bytes(payload, c, br#"","title":""#)
        && append_json_unescaped_then_escaped(payload, c, title)
        && write_bytes(payload, c, br#"","cover":"#)
        && write_cover(payload, c, cover)
        && write_bytes(payload, c, br#","authors":[""#)
        && append_json_unescaped_then_escaped(payload, c, author)
        && write_bytes(
            payload,
            c,
            br#""],"status":"completed","contentRating":"safe","sourceTags":[""#,
        )
        && append_json_escaped(payload, c, tag)
        && write_bytes(payload, c, br#""]}"#)
}

fn write_list_from_array(
    operation: &str,
    json: &[u8],
    key: &[u8],
    limit: usize,
    tag: &[u8],
) -> u32 {
    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error(operation, "internal_error", "payload overflow");
    }
    let mut iter = match JsonArrayIter::new(json, key) {
        Some(v) => v,
        None => return write_error(operation, "parse_error", "missing magazine list"),
    };
    let mut written = 0usize;
    while let Some(obj) = iter.next_object() {
        if written >= limit {
            break;
        }
        if written > 0 && !write_bytes(payload, &mut c, b",") {
            return write_error(operation, "internal_error", "payload overflow");
        }
        if !write_manga_card(payload, &mut c, obj, tag) {
            return write_error(operation, "parse_error", "missing magazine id");
        }
        written += 1;
    }
    let has_more = if written >= limit {
        b"true" as &[u8]
    } else {
        b"false" as &[u8]
    };
    if !(write_bytes(
        payload,
        &mut c,
        br#"],"page":{"nextCursor":null,"hasMore":"#,
    ) && write_bytes(payload, &mut c, has_more)
        && write_bytes(payload, &mut c, br#"}}"#))
    {
        return write_error(operation, "internal_error", "payload overflow");
    }
    write_success_payload(operation, c)
}

fn run_get_manga_list(req: &[u8]) -> u32 {
    let page = request_page(req);
    let limit = request_limit(req);
    let url_len = match build_index_url(page, limit) {
        Some(v) => v,
        None => return write_error("get_manga_list", "internal_error", "url overflow"),
    };
    let url =
        unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(URL_BUF) as *const u8, url_len) };
    let body_len = match fetch_json(url) {
        Ok(v) => v,
        Err(e) => {
            let (code, message) = fetch_error_code(e);
            return write_error("get_manga_list", code, message);
        }
    };
    write_list_from_array(
        "get_manga_list",
        body_slice(body_len),
        b"new",
        limit,
        b"zazhimi",
    )
}

fn run_search(req: &[u8]) -> u32 {
    let query = extract_json_string(req, b"query").unwrap_or(b"");
    let page = request_page(req);
    let limit = request_limit(req);
    if query.is_empty() {
        return run_get_manga_list(req);
    }
    let url_len = match build_search_url(query, page, limit) {
        Some(v) => v,
        None => return write_error("search", "internal_error", "url overflow"),
    };
    let url =
        unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(URL_BUF) as *const u8, url_len) };
    let mut body_len = match fetch_json(url) {
        Ok(v) => v,
        Err(e) => {
            let (code, message) = fetch_error_code(e);
            return write_error("search", code, message);
        }
    };
    if contains_bytes(body_slice(body_len), br#""magazine":[]"#) {
        let fallback_len = match build_beauty_list_url(page, limit) {
            Some(v) => v,
            None => return write_error("search", "internal_error", "url overflow"),
        };
        let fallback_url = unsafe {
            core::slice::from_raw_parts(core::ptr::addr_of!(URL_BUF) as *const u8, fallback_len)
        };
        body_len = match fetch_json(fallback_url) {
            Ok(v) => v,
            Err(e) => {
                let (code, message) = fetch_error_code(e);
                return write_error("search", code, message);
            }
        };
    }
    write_list_from_array(
        "search",
        body_slice(body_len),
        b"magazine",
        limit,
        b"zazhimi",
    )
}

fn first_show_item(json: &[u8]) -> Option<&[u8]> {
    let mut iter = JsonArrayIter::new(json, b"content")?;
    iter.next_object()
}

fn fetch_show_for(operation: &str, id: &[u8]) -> Result<usize, u32> {
    let url_len = build_show_url(id)
        .ok_or_else(|| write_error(operation, "internal_error", "url overflow"))?;
    let url =
        unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(URL_BUF) as *const u8, url_len) };
    fetch_json(url).map_err(|e| {
        let (code, message) = fetch_error_code(e);
        write_error(operation, code, message)
    })
}

fn run_get_manga(req: &[u8]) -> u32 {
    let manga_id = match extract_json_string(req, b"mangaId") {
        Some(v) => v,
        None => return write_error("get_manga", "invalid_request", "missing mangaId"),
    };
    let body_len = match fetch_show_for("get_manga", manga_id) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let item = match first_show_item(body_slice(body_len)) {
        Some(v) => v,
        None => return write_error("get_manga", "parse_error", "empty content"),
    };
    let title = extract_json_string(item, b"magName").unwrap_or(manga_id);
    let cover = extract_json_string(item, b"magPic").unwrap_or(b"");
    let author_len = first_word_len(title);
    let author = if author_len > 0 {
        &title[..author_len]
    } else {
        title
    };
    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"manga":{"id":""#)
        && append_json_unescaped_then_escaped(payload, &mut c, manga_id)
        && write_bytes(payload, &mut c, br#"","title":""#)
        && append_json_unescaped_then_escaped(payload, &mut c, title)
        && write_bytes(payload, &mut c, br#"","alternateTitles":[],"description":"","cover":"#)
        && write_cover(payload, &mut c, cover)
        && write_bytes(payload, &mut c, br#","authors":[""#)
        && append_json_unescaped_then_escaped(payload, &mut c, author)
        && write_bytes(payload, &mut c, br#""],"artists":[],"status":"completed","contentRating":"safe","language":"zh","tags":[],"links":[]}}"#);
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
    let body_len = match fetch_show_for("get_chapters", manga_id) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let item = match first_show_item(body_slice(body_len)) {
        Some(v) => v,
        None => {
            return response_buffer().write_success(
                "get_chapters",
                br#"{"items":[],"page":{"nextCursor":null,"hasMore":false}}"#,
            );
        }
    };
    let title = extract_json_string(item, b"magName").unwrap_or(manga_id);
    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"items":[{"id":""#)
        && append_json_unescaped_then_escaped(payload, &mut c, manga_id)
        && write_bytes(payload, &mut c, br#"","mangaId":""#)
        && append_json_unescaped_then_escaped(payload, &mut c, manga_id)
        && write_bytes(payload, &mut c, br#"","title":""#)
        && append_json_unescaped_then_escaped(payload, &mut c, title)
        && write_bytes(payload, &mut c, br#"","chapterNumber":"1","volumeNumber":null,"language":"zh","publishedAt":null,"updatedAt":null,"pageCount":null}],"page":{"nextCursor":null,"hasMore":false}}"#);
    if !ok {
        return write_error("get_chapters", "internal_error", "payload overflow");
    }
    write_success_payload("get_chapters", c)
}

fn run_get_pages(req: &[u8]) -> u32 {
    let chapter_id = match extract_json_string(req, b"chapterId") {
        Some(v) => v,
        None => return write_error("get_pages", "invalid_request", "missing chapterId"),
    };
    let body_len = match fetch_show_for("get_pages", chapter_id) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let payload = payload_buf();
    let mut c = 0usize;
    if !(write_bytes(payload, &mut c, br#"{"chapterId":""#)
        && append_json_unescaped_then_escaped(payload, &mut c, chapter_id)
        && write_bytes(payload, &mut c, br#"","pages":["#))
    {
        return write_error("get_pages", "internal_error", "payload overflow");
    }
    let mut iter = match JsonArrayIter::new(body_slice(body_len), b"content") {
        Some(v) => v,
        None => return write_error("get_pages", "parse_error", "missing content"),
    };
    let mut written = 0usize;
    while let Some(obj) = iter.next_object() {
        let img = match extract_json_string(obj, b"magPic") {
            Some(v) => v,
            None => continue,
        };
        if written > 0 && !write_bytes(payload, &mut c, b",") {
            return write_error("get_pages", "internal_error", "payload overflow");
        }
        let ok = write_bytes(payload, &mut c, br#"{"id":"page:"#)
            && append_json_unescaped_then_escaped(payload, &mut c, chapter_id)
            && write_bytes(payload, &mut c, b":")
            && write_usize(payload, &mut c, written)
            && write_bytes(payload, &mut c, br#"","index":"#)
            && write_usize(payload, &mut c, written)
            && write_bytes(payload, &mut c, br#","image":{"kind":"url","url":""#)
            && append_json_unescaped_then_escaped(payload, &mut c, img)
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

#[no_mangle]
pub extern "C" fn koma_source_init(_manifest_ptr: u32, manifest_len: u32) -> i32 {
    log_info(b"zazhimi source init");
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
    log_info(b"zazhimi search");
    run_search(req)
}
#[no_mangle]
pub extern "C" fn koma_source_get_manga_list(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga_list", "invalid_request", "empty request"),
    };
    log_info(b"zazhimi get_manga_list");
    run_get_manga_list(req)
}
#[no_mangle]
pub extern "C" fn koma_source_get_manga(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga", "invalid_request", "empty request"),
    };
    log_info(b"zazhimi get_manga");
    run_get_manga(req)
}
#[no_mangle]
pub extern "C" fn koma_source_get_chapters(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_chapters", "invalid_request", "empty request"),
    };
    log_info(b"zazhimi get_chapters");
    run_get_chapters(req)
}
#[no_mangle]
pub extern "C" fn koma_source_get_pages(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_pages", "invalid_request", "empty request"),
    };
    log_info(b"zazhimi get_pages");
    run_get_pages(req)
}
#[no_mangle]
pub extern "C" fn koma_source_get_listings(_req_ptr: u32, _req_len: u32) -> u32 {
    response_buffer().write_success("get_listings", br#"{"listings":[]}"#)
}
#[no_mangle]
pub extern "C" fn koma_source_get_home(_req_ptr: u32, _req_len: u32) -> u32 {
    response_buffer().write_success("get_home", br#"{"sections":[]}"#)
}
#[no_mangle]
pub extern "C" fn koma_source_get_filters(_req_ptr: u32, _req_len: u32) -> u32 {
    response_buffer().write_success("get_filters", br#"{"filters":[]}"#)
}
#[no_mangle]
pub extern "C" fn koma_source_get_settings(_req_ptr: u32, _req_len: u32) -> u32 {
    response_buffer().write_success("get_settings", br#"{"settings":[]}"#)
}
#[no_mangle]
pub extern "C" fn koma_source_get_image_request(_req_ptr: u32, _req_len: u32) -> u32 {
    response_buffer().write_error("get_image_request", "unimplemented", "not implemented")
}
#[no_mangle]
pub extern "C" fn koma_source_free(result_ptr: u32) {
    response_buffer().free(result_ptr)
}
