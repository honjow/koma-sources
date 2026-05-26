#![no_std]

extern crate koma_source_sdk;

use koma_source_sdk::host::{http_request, log_info};
use koma_source_sdk::json_utils::{
    append_json_escaped, append_json_unescaped_then_escaped, contains_bytes, extract_json_string,
    find_subslice, write_bytes, write_url_encoded, write_usize, JsonArrayIter,
};
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{
    build_get_request, build_post_request, decode_json_body_into, fetch_error_code, FetchError,
};

const API_BASE: &[u8] = b"https://api.tongli.tw";
const BASE_URL: &[u8] = b"https://ebook.tongli.com.tw";
const FIREBASE_SIGNUP_URL: &[u8] = b"https://www.googleapis.com/identitytoolkit/v3/relyingparty/signupNewUser?key=AIzaSyAJ22CmxAWmZYfQI-OWdALy4vKS_uB-VJ4";
const FIREBASE_SIGNUP_FALLBACK_URL: &[u8] = b"https://www.googleapis.com/identitytoolkit/v3/relyingparty/signupNewUser?key=AIzaSyAJbYmo7KyhM_7CDXjjFXnp8bdRTNgbUIE";
const LATEST_SHELF_ID: &[u8] = b"6e7e5b75-1acd-4b7c-0097-08d6179fc10a";
const MULTIPART_CONTENT_TYPE: &[u8] = b"multipart/form-data; boundary=BOUNDARY";

koma_source_sdk::koma_source_buffers! {
    payload: 1024 * 1024,
    http_out: 2 * 1024 * 1024,
    body: 2 * 1024 * 1024,
    http_req: 64 * 1024,
    scratch: 16 * 1024,
}
koma_source_sdk::koma_source_helpers!();

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.tongli.koma",
    name: "東立",
    version: "0.1.0",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "Tongli JSON API source with Firebase anonymous auth.",
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
    filters: false,
    settings: false,
    image_request: true,
    credentials: false,
};

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
    let raw = koma_source_sdk::json_utils::extract_json_number(req, b"page")
        .or_else(|| extract_json_string(req, b"page"))
        .map(parse_usize)
        .unwrap_or(1);
    if raw == 0 {
        1
    } else {
        raw
    }
}

fn strip_prefix<'a>(bytes: &'a [u8], prefix: &[u8]) -> Option<&'a [u8]> {
    if bytes.len() <= prefix.len() || &bytes[..prefix.len()] != prefix {
        None
    } else {
        Some(&bytes[prefix.len()..])
    }
}

fn split_manga_id(id: &[u8]) -> Option<(&[u8], &[u8])> {
    let inner = strip_prefix(id, b"tl:")?;
    let comma = find_subslice(inner, b",")?;
    Some((&inner[..comma], &inner[comma + 1..]))
}

fn strip_chapter_id(id: &[u8]) -> Option<&[u8]> {
    strip_prefix(id, b"tlch:")
}

fn extract_json_string_loose<'a>(data: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let mut pattern = [0u8; 64];
    let needed = key.len() + 2;
    if needed > pattern.len() {
        return None;
    }
    pattern[0] = b'"';
    pattern[1..1 + key.len()].copy_from_slice(key);
    pattern[1 + key.len()] = b'"';
    let mut i = find_subslice(data, &pattern[..needed])? + needed;
    while i < data.len() && matches!(data[i], b' ' | b'\t' | b'\n' | b'\r') {
        i += 1;
    }
    if i >= data.len() || data[i] != b':' {
        return None;
    }
    i += 1;
    while i < data.len() && matches!(data[i], b' ' | b'\t' | b'\n' | b'\r') {
        i += 1;
    }
    if i >= data.len() || data[i] != b'"' {
        return None;
    }
    i += 1;
    let start = i;
    while i < data.len() {
        if data[i] == b'\\' {
            i += 2;
            continue;
        }
        if data[i] == b'"' {
            return Some(&data[start..i]);
        }
        i += 1;
    }
    None
}

fn fetch_json_with_headers(
    url: &[u8],
    headers: &[(&[u8], &[u8])],
) -> core::result::Result<usize, FetchError> {
    let req_len = build_get_request(http_req_buf(), url, Some(BASE_URL), headers)
        .ok_or(FetchError::Network)?;
    let resp_len =
        http_request(&http_req_buf()[..req_len], http_out()).map_err(|_| FetchError::Network)?;
    decode_json_body_into(&http_out()[..resp_len], body_buf())
}

fn post_json(
    url: &[u8],
    request_body: &[u8],
    content_type: &[u8],
) -> core::result::Result<usize, FetchError> {
    let req_len = build_post_request(
        http_req_buf(),
        url,
        request_body,
        content_type,
        Some(BASE_URL),
    )
    .ok_or(FetchError::Network)?;
    let resp_len =
        http_request(&http_req_buf()[..req_len], http_out()).map_err(|_| FetchError::Network)?;
    decode_json_body_into(&http_out()[..resp_len], body_buf())
}

fn fetch_json(url: &[u8]) -> core::result::Result<usize, FetchError> {
    fetch_json_with_headers(url, &[])
}

fn fetch_anon_token() -> core::result::Result<&'static [u8], FetchError> {
    let mut len = post_json(
        FIREBASE_SIGNUP_URL,
        br#"{"returnSecureToken":true}"#,
        b"application/json;charset=UTF-8",
    )?;
    if extract_json_string_loose(&body_buf()[..len], b"idToken").is_none() {
        log_info(b"tongli: primary firebase key rejected, trying reference key");
        len = post_json(
            FIREBASE_SIGNUP_FALLBACK_URL,
            br#"{"returnSecureToken":true}"#,
            b"application/json;charset=UTF-8",
        )?;
    }
    let body = &body_buf()[..len];
    let token = extract_json_string_loose(body, b"idToken").ok_or(FetchError::ClientError)?;
    if token.len() > scratch_b().len() {
        return Err(FetchError::Network);
    }
    scratch_b()[..token.len()].copy_from_slice(token);
    Ok(unsafe {
        core::slice::from_raw_parts(core::ptr::addr_of!(SCRATCH_B) as *const u8, token.len())
    })
}

fn fetch_authorized_json(url: &[u8]) -> core::result::Result<usize, FetchError> {
    let token = fetch_anon_token()?;
    let mut auth = [0u8; 4096];
    let mut c = 0usize;
    if !(write_bytes(&mut auth, &mut c, b"Bearer ") && write_bytes(&mut auth, &mut c, token)) {
        return Err(FetchError::Network);
    }
    fetch_json_with_headers(url, &[(b"Authorization", &auth[..c])])
}

fn build_latest_url(page: usize) -> Option<usize> {
    let dst = scratch_a();
    let mut c = 0usize;
    (write_bytes(dst, &mut c, API_BASE)
        && write_bytes(dst, &mut c, b"/SellShelf/")
        && write_bytes(dst, &mut c, LATEST_SHELF_ID)
        && write_bytes(dst, &mut c, b"/")
        && write_usize(dst, &mut c, page)
        && write_bytes(dst, &mut c, b"?pageSize=20"))
    .then_some(c)
}

fn build_detail_url(book_group_id: &[u8], is_serial: &[u8]) -> Option<usize> {
    let dst = scratch_a();
    let mut c = 0usize;
    (write_bytes(dst, &mut c, API_BASE)
        && write_bytes(dst, &mut c, b"/Book?bookGroupID=")
        && write_url_encoded(dst, &mut c, book_group_id)
        && write_bytes(dst, &mut c, b"&isSerial=")
        && write_url_encoded(dst, &mut c, is_serial))
    .then_some(c)
}

fn build_chapters_url(book_group_id: &[u8], is_serial: &[u8]) -> Option<usize> {
    let dst = scratch_a();
    let mut c = 0usize;
    (write_bytes(dst, &mut c, API_BASE)
        && write_bytes(dst, &mut c, b"/Book/BookVol/")
        && write_url_encoded(dst, &mut c, book_group_id)
        && write_bytes(dst, &mut c, b"?bookID=null&isSerial=")
        && write_url_encoded(dst, &mut c, is_serial))
    .then_some(c)
}

fn build_pages_url(book_id: &[u8]) -> Option<usize> {
    let dst = scratch_a();
    let mut c = 0usize;
    (write_bytes(dst, &mut c, API_BASE)
        && write_bytes(dst, &mut c, b"/Comic/sas/")
        && write_url_encoded(dst, &mut c, book_id))
    .then_some(c)
}

fn build_search_body(query: &[u8]) -> Option<usize> {
    let dst = scratch_a();
    let mut c = 0usize;
    (write_bytes(dst, &mut c, b"--BOUNDARY\r\n")
        && write_bytes(
            dst,
            &mut c,
            b"Content-Disposition: form-data; name=\"SearchStr\"\r\n\r\n",
        )
        && write_bytes(dst, &mut c, query)
        && write_bytes(dst, &mut c, b"\r\n--BOUNDARY--\r\n"))
    .then_some(c)
}

fn status_from_serial(is_serial: &[u8]) -> &'static [u8] {
    if is_serial == b"true" || is_serial == b"True" {
        b"ongoing"
    } else {
        b"unknown"
    }
}

fn write_manga_card(payload: &mut [u8], c: &mut usize, obj: &[u8]) -> bool {
    let title = extract_json_string(obj, b"BookTitle")
        .or_else(|| extract_json_string(obj, b"Title"))
        .unwrap_or(b"Unknown");
    let cover = extract_json_string(obj, b"BookCoverURL")
        .or_else(|| extract_json_string(obj, b"CoverURL"))
        .unwrap_or(b"");
    let book_group_id = match extract_json_string(obj, b"BookGroupID") {
        Some(v) => v,
        None => return true,
    };
    let is_serial = if contains_bytes(obj, br#""IsSerial":true"#) {
        b"true" as &[u8]
    } else {
        b"false" as &[u8]
    };

    write_bytes(payload, c, br#"{"id":"tl:"#)
        && append_json_escaped(payload, c, book_group_id)
        && write_bytes(payload, c, b",")
        && write_bytes(payload, c, is_serial)
        && write_bytes(payload, c, br#"","title":""#)
        && append_json_unescaped_then_escaped(payload, c, title)
        && write_bytes(payload, c, br#"","cover":{"kind":"url","url":""#)
        && append_json_unescaped_then_escaped(payload, c, cover)
        && write_bytes(payload, c, br#""},"authors":[],"status":""#)
        && write_bytes(payload, c, status_from_serial(is_serial))
        && write_bytes(
            payload,
            c,
            br#"","contentRating":"safe","sourceTags":["tongli"]}"#,
        )
}

struct RootArrayIter<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> RootArrayIter<'a> {
    fn new(data: &'a [u8]) -> Option<Self> {
        let mut pos = 0usize;
        while pos < data.len() && matches!(data[pos], b' ' | b'\t' | b'\n' | b'\r') {
            pos += 1;
        }
        if pos < data.len() && data[pos] == b'[' {
            Some(Self { data, pos: pos + 1 })
        } else {
            None
        }
    }

    fn next_object(&mut self) -> Option<&'a [u8]> {
        while self.pos < self.data.len() {
            match self.data[self.pos] {
                b' ' | b'\t' | b'\n' | b'\r' | b',' => self.pos += 1,
                b']' => return None,
                b'{' => break,
                _ => return None,
            }
        }
        let start = self.pos;
        let mut depth = 0i32;
        let mut in_string = false;
        while self.pos < self.data.len() {
            let b = self.data[self.pos];
            if in_string {
                if b == b'\\' {
                    self.pos += 1;
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
                            self.pos += 1;
                            return Some(&self.data[start..self.pos]);
                        }
                    }
                    _ => {}
                }
            }
            self.pos += 1;
        }
        None
    }
}

fn write_manga_list_from_iter<I>(operation: &str, mut next: I, has_more: bool) -> u32
where
    I: FnMut() -> Option<&'static [u8]>,
{
    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error(operation, "internal_error", "overflow");
    }
    let mut written = 0usize;
    while let Some(obj) = next() {
        if written >= 20 {
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
    let more = if has_more {
        b"true" as &[u8]
    } else {
        b"false" as &[u8]
    };
    if !(write_bytes(
        payload,
        &mut c,
        br#"],"page":{"nextCursor":null,"hasMore":"#,
    ) && write_bytes(payload, &mut c, more)
        && write_bytes(payload, &mut c, br#"}}"#))
    {
        return write_error(operation, "internal_error", "overflow");
    }
    write_success_payload(operation, c)
}

fn run_popular_list(operation: &str) -> u32 {
    let len = match fetch_json(b"https://api.tongli.tw/SellRanking/1") {
        Ok(v) => v,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error(operation, c, m);
        }
    };
    let json =
        unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(BODY_BUF) as *const u8, len) };
    let mut iter = match JsonArrayIter::new(json, b"Week") {
        Some(v) => v,
        None => return write_error(operation, "parse_error", "missing Week"),
    };
    write_manga_list_from_iter(operation, || iter.next_object(), false)
}

fn run_latest_list(operation: &str, page: usize) -> u32 {
    let url_len = match build_latest_url(page) {
        Some(v) => v,
        None => return write_error(operation, "internal_error", "url overflow"),
    };
    let url = unsafe {
        core::slice::from_raw_parts(core::ptr::addr_of!(SCRATCH_A) as *const u8, url_len)
    };
    let len = match fetch_json(url) {
        Ok(v) => v,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error(operation, c, m);
        }
    };
    let json =
        unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(BODY_BUF) as *const u8, len) };
    let total = koma_source_sdk::json_utils::extract_json_number(json, b"TotalPage")
        .map(parse_usize)
        .unwrap_or(page);
    let current = koma_source_sdk::json_utils::extract_json_number(json, b"Page")
        .map(parse_usize)
        .unwrap_or(page);
    let mut iter = match JsonArrayIter::new(json, b"Books") {
        Some(v) => v,
        None => return write_error(operation, "parse_error", "missing Books"),
    };
    write_manga_list_from_iter(operation, || iter.next_object(), total > current)
}

fn run_search(req: &[u8]) -> u32 {
    let query = extract_json_string(req, b"query").unwrap_or(b"");
    if query.is_empty() {
        return run_popular_list("search");
    }
    let body_len = match build_search_body(query) {
        Some(v) => v,
        None => return write_error("search", "internal_error", "body overflow"),
    };
    let request_body = unsafe {
        core::slice::from_raw_parts(core::ptr::addr_of!(SCRATCH_A) as *const u8, body_len)
    };
    let len = match post_json(
        b"https://api.tongli.tw/Search",
        request_body,
        MULTIPART_CONTENT_TYPE,
    ) {
        Ok(v) => v,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("search", c, m);
        }
    };
    let json =
        unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(BODY_BUF) as *const u8, len) };
    let mut iter = match RootArrayIter::new(json) {
        Some(v) => v,
        None => return write_error("search", "parse_error", "missing search array"),
    };
    write_manga_list_from_iter("search", || iter.next_object(), false)
}

fn write_authors(payload: &mut [u8], c: &mut usize, obj: &[u8]) -> bool {
    let mut iter = match JsonArrayIter::new(obj, b"Authors") {
        Some(v) => v,
        None => return true,
    };
    let mut count = 0usize;
    while let Some(author) = iter.next_object() {
        let name = extract_json_string(author, b"Name").unwrap_or(b"");
        if name.is_empty() {
            continue;
        }
        let title = extract_json_string(author, b"Title").unwrap_or(b"");
        if count > 0 && !write_bytes(payload, c, b",") {
            return false;
        }
        if !(write_bytes(payload, c, b"\"")) {
            return false;
        }
        if !title.is_empty() {
            if !(append_json_unescaped_then_escaped(payload, c, title)
                && write_bytes(payload, c, b"\xEF\xBC\x9A"))
            {
                return false;
            }
        }
        if !(append_json_unescaped_then_escaped(payload, c, name) && write_bytes(payload, c, b"\""))
        {
            return false;
        }
        count += 1;
    }
    true
}

fn run_get_manga(req: &[u8]) -> u32 {
    let manga_id = match extract_json_string(req, b"mangaId") {
        Some(v) => v,
        None => return write_error("get_manga", "invalid_request", "missing mangaId"),
    };
    let (book_group_id, is_serial) = match split_manga_id(manga_id) {
        Some(v) => v,
        None => return write_error("get_manga", "invalid_request", "bad mangaId"),
    };
    let url_len = match build_detail_url(book_group_id, is_serial) {
        Some(v) => v,
        None => return write_error("get_manga", "internal_error", "url overflow"),
    };
    let url = unsafe {
        core::slice::from_raw_parts(core::ptr::addr_of!(SCRATCH_A) as *const u8, url_len)
    };
    let len = match fetch_json(url) {
        Ok(v) => v,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_manga", c, m);
        }
    };
    let obj =
        unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(BODY_BUF) as *const u8, len) };
    let title = extract_json_string(obj, b"Title").unwrap_or(b"Unknown");
    let cover = extract_json_string(obj, b"CoverURL").unwrap_or(b"");
    let desc = extract_json_string(obj, b"Introduction").unwrap_or(b"");
    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"manga":{"id":""#)
        && append_json_escaped(payload, &mut c, manga_id)
        && write_bytes(payload, &mut c, br#"","title":""#)
        && append_json_unescaped_then_escaped(payload, &mut c, title)
        && write_bytes(payload, &mut c, br#"","alternateTitles":[],"description":""#)
        && append_json_unescaped_then_escaped(payload, &mut c, desc)
        && write_bytes(payload, &mut c, br#"","cover":{"kind":"url","url":""#)
        && append_json_unescaped_then_escaped(payload, &mut c, cover)
        && write_bytes(payload, &mut c, br#""},"authors":["#)
        && write_authors(payload, &mut c, obj)
        && write_bytes(payload, &mut c, br#"],"artists":[],"status":""#)
        && write_bytes(payload, &mut c, status_from_serial(is_serial))
        && write_bytes(payload, &mut c, br#"","contentRating":"safe","language":"zh","tags":[],"links":[{"kind":"source","url":""#)
        && write_bytes(payload, &mut c, BASE_URL)
        && write_bytes(payload, &mut c, b"/book?id=")
        && append_json_escaped(payload, &mut c, book_group_id)
        && write_bytes(payload, &mut c, b"&isGroup=true&isSerials=")
        && append_json_escaped(payload, &mut c, is_serial)
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
    let (book_group_id, is_serial) = match split_manga_id(manga_id) {
        Some(v) => v,
        None => return write_error("get_chapters", "invalid_request", "bad mangaId"),
    };
    let url_len = match build_chapters_url(book_group_id, is_serial) {
        Some(v) => v,
        None => return write_error("get_chapters", "internal_error", "url overflow"),
    };
    let url = unsafe {
        core::slice::from_raw_parts(core::ptr::addr_of!(SCRATCH_A) as *const u8, url_len)
    };
    let len = match fetch_authorized_json(url) {
        Ok(v) => v,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_chapters", c, m);
        }
    };
    let json =
        unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(BODY_BUF) as *const u8, len) };
    let mut root = match RootArrayIter::new(json) {
        Some(v) => v,
        None => return write_error("get_chapters", "parse_error", "missing chapters array"),
    };
    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("get_chapters", "internal_error", "overflow");
    }
    let mut written = 0usize;
    while let Some(obj) = root.next_object() {
        if contains_bytes(obj, br#""IsUpcoming":true"#) {
            continue;
        }
        let book_id = match extract_json_string(obj, b"BookID") {
            Some(v) => v,
            None => continue,
        };
        let vol = extract_json_string(obj, b"Vol").unwrap_or(b"");
        let readable = contains_bytes(obj, br#""IsFree":true"#)
            || contains_bytes(obj, br#""IsPurchased":true"#);
        if written > 0 && !write_bytes(payload, &mut c, b",") {
            return write_error("get_chapters", "internal_error", "overflow");
        }
        let ok = write_bytes(payload, &mut c, br#"{"id":"tlch:"#)
            && append_json_escaped(payload, &mut c, book_id)
            && write_bytes(payload, &mut c, br#"","mangaId":""#)
            && append_json_escaped(payload, &mut c, manga_id)
            && write_bytes(payload, &mut c, br#"","title":""#)
            && if readable {
                true
            } else {
                write_bytes(payload, &mut c, b"\xF0\x9F\x94\x92 ")
            }
            && append_json_unescaped_then_escaped(payload, &mut c, vol)
            && write_bytes(
                payload,
                &mut c,
                br#"","chapterNumber":null,"volumeNumber":null,"language":"zh","scanlator":""#,
            )
            && write_bytes(payload, &mut c, b"\xE6\x9D\xB1\xE7\xAB\x8B")
            && write_bytes(
                payload,
                &mut c,
                br#"","publishedAt":null,"updatedAt":null,"pageCount":null}"#,
            );
        if !ok {
            return write_error("get_chapters", "internal_error", "overflow");
        }
        written += 1;
    }
    if !write_bytes(
        payload,
        &mut c,
        br#"],"page":{"nextCursor":null,"hasMore":false}}"#,
    ) {
        return write_error("get_chapters", "internal_error", "overflow");
    }
    write_success_payload("get_chapters", c)
}

fn run_get_pages(req: &[u8]) -> u32 {
    let chapter_id = match extract_json_string(req, b"chapterId") {
        Some(v) => v,
        None => return write_error("get_pages", "invalid_request", "missing chapterId"),
    };
    let book_id = match strip_chapter_id(chapter_id) {
        Some(v) => v,
        None => return write_error("get_pages", "invalid_request", "bad chapterId"),
    };
    let url_len = match build_pages_url(book_id) {
        Some(v) => v,
        None => return write_error("get_pages", "internal_error", "url overflow"),
    };
    let url = unsafe {
        core::slice::from_raw_parts(core::ptr::addr_of!(SCRATCH_A) as *const u8, url_len)
    };
    let len = match fetch_authorized_json(url) {
        Ok(v) => v,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_pages", c, m);
        }
    };
    let json =
        unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(BODY_BUF) as *const u8, len) };
    let mut iter = match JsonArrayIter::new(json, b"Pages") {
        Some(v) => v,
        None => return write_error("get_pages", "parse_error", "missing Pages"),
    };
    let payload = payload_buf();
    let mut c = 0usize;
    if !(write_bytes(payload, &mut c, br#"{"chapterId":""#)
        && append_json_escaped(payload, &mut c, chapter_id)
        && write_bytes(payload, &mut c, br#"","pages":["#))
    {
        return write_error("get_pages", "internal_error", "overflow");
    }
    let mut index = 0usize;
    while let Some(obj) = iter.next_object() {
        let img = match extract_json_string(obj, b"ImageURL") {
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
            && append_json_unescaped_then_escaped(payload, &mut c, img)
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

fn run_get_listings(_req: &[u8]) -> u32 {
    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(
        payload,
        &mut c,
        br#"{"listings":[{"id":"popular","name":"Popular"},{"id":"latest","name":"Latest"}]}"#,
    );
    if !ok {
        return write_error("get_listings", "internal_error", "overflow");
    }
    write_success_payload("get_listings", c)
}

fn run_get_manga_list(req: &[u8]) -> u32 {
    let listing = extract_json_string(req, b"listingId").unwrap_or(b"latest");
    if listing == b"popular" {
        run_popular_list("get_manga_list")
    } else {
        run_latest_list("get_manga_list", request_page(req))
    }
}

fn run_get_home(_req: &[u8]) -> u32 {
    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"sections":[]}"#) {
        return write_error("get_home", "internal_error", "overflow");
    }
    write_success_payload("get_home", c)
}

fn run_get_filters(_req: &[u8]) -> u32 {
    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"filters":[]}"#) {
        return write_error("get_filters", "internal_error", "overflow");
    }
    write_success_payload("get_filters", c)
}

fn run_get_settings(_req: &[u8]) -> u32 {
    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{}"#) {
        return write_error("get_settings", "internal_error", "overflow");
    }
    write_success_payload("get_settings", c)
}

fn run_get_image_request(req: &[u8]) -> u32 {
    let url = match extract_json_string(req, b"url") {
        Some(v) => v,
        None => return write_error("get_image_request", "invalid_request", "missing url"),
    };
    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"url":""#)
        && append_json_unescaped_then_escaped(payload, &mut c, url)
        && write_bytes(
            payload,
            &mut c,
            br#"","headers":{"Referer":"https://ebook.tongli.com.tw/"}}"#,
        );
    if !ok {
        return write_error("get_image_request", "internal_error", "overflow");
    }
    write_success_payload("get_image_request", c)
}

koma_source_sdk::koma_source_exports!("tongli");
