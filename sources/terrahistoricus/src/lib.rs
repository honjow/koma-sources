#![no_std]

extern crate koma_source_sdk;

use koma_source_sdk::host::{self, http_request, log_info};
use koma_source_sdk::json_utils::{
    append_json_escaped, contains_bytes, extract_json_number, extract_json_string, find_subslice,
    write_bytes, write_url_encoded, write_usize,
};
use koma_source_sdk::result::ResultBuffer;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};

const BASE_URL: &[u8] = b"https://comic.hypergryph.com";
const TOPIC_TERRA: &[u8] = b"terra-historicus";
const TOPIC_TALOS: &[u8] = b"talos-ii-historicus";
koma_source_sdk::koma_source_buffers! {
    payload: 512 * 1024,
    http_out: 1024 * 1024,
    body: 1024 * 1024,
    http_req: 2048,
    scratch: 4096,
}
koma_source_sdk::koma_source_helpers!();

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.terrahistoricus.koma",
    name: "TerraHistoricus",
    version: "0.1.0",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "泰拉记事社 — 明日方舟官方漫画平台",
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
    credentials: false,
    image_request: true,
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

fn fetch_json(url: &[u8]) -> Result<&'static [u8], FetchError> {
    let req_len = build_get_request(http_req_buf(), url).ok_or(FetchError::Network)?;
    let mut resp_len = 0usize;
    let mut failed = true;
    for attempt in 0..3u8 {
        match http_request(&http_req_buf()[..req_len], http_out()) {
            Ok(n) => {
                resp_len = n;
                failed = false;
                break;
            }
            Err(_) => {
                if attempt < 2 {
                    log_info(b"terrahistoricus: http transport error, retrying");
                }
            }
        }
    }
    if failed {
        return Err(FetchError::Network);
    }
    let resp = &http_out()[..resp_len];
    if !contains_bytes(resp, br#""ok":true"#) {
        let err = if let Some(code) = extract_json_number(resp, b"statusCode") {
            match parse_status_code(code) {
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

    let marker = b"\"bodyText\":\"";
    let mut i = find_subslice(resp, marker).ok_or(FetchError::Network)? + marker.len();
    let dst = body_buf();
    let mut out = 0usize;
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
                _ => next,
            };
            if out >= dst.len() {
                return Err(FetchError::Network);
            }
            dst[out] = unescaped;
            out += 1;
            i += 2;
            continue;
        }
        if b == b'"' {
            break;
        }
        if out >= dst.len() {
            return Err(FetchError::Network);
        }
        dst[out] = b;
        out += 1;
        i += 1;
    }
    Ok(body_slice(out))
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
    if let Some(s) = extract_json_string(req, b"page") {
        return parse_usize(s, 1);
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

fn extract_value_start(data: &[u8], key: &[u8]) -> Option<usize> {
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
    Some(pos)
}

fn extract_object_for_key<'a>(data: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let pos = extract_value_start(data, key)?;
    if pos >= data.len() || data[pos] != b'{' {
        return None;
    }
    extract_balanced(data, pos, b'{', b'}')
}

fn extract_array_for_key<'a>(data: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let pos = extract_value_start(data, key)?;
    if pos >= data.len() || data[pos] != b'[' {
        return None;
    }
    extract_balanced(data, pos, b'[', b']').map(|v| &v[1..v.len() - 1])
}

fn next_raw_object<'a>(data: &'a [u8], pos: &mut usize) -> Option<&'a [u8]> {
    while *pos < data.len() {
        match data[*pos] {
            b' ' | b'\t' | b'\n' | b'\r' | b',' => *pos += 1,
            b'{' => break,
            _ => return None,
        }
    }
    let obj = extract_balanced(data, *pos, b'{', b'}')?;
    *pos += obj.len();
    Some(obj)
}

fn first_array_string<'a>(obj: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let arr = extract_array_for_key(obj, key)?;
    let mut i = 0usize;
    while i < arr.len() && matches!(arr[i], b' ' | b'\t' | b'\n' | b'\r' | b',') {
        i += 1;
    }
    if i >= arr.len() || arr[i] != b'"' {
        return None;
    }
    let start = i + 1;
    i = start;
    while i < arr.len() {
        if arr[i] == b'\\' {
            i += 2;
            continue;
        }
        if arr[i] == b'"' {
            return Some(&arr[start..i]);
        }
        i += 1;
    }
    None
}

fn split_chapter_id(id: &[u8]) -> Option<(&[u8], &[u8])> {
    let pos = find_subslice(id, b"/")?;
    if pos == 0 || pos + 1 >= id.len() {
        return None;
    }
    Some((&id[..pos], &id[pos + 1..]))
}

fn build_topic_url(topic: &[u8]) -> Option<usize> {
    let buf = scratch_a();
    let mut c = 0usize;
    write_bytes(buf, &mut c, BASE_URL).then_some(())?;
    write_bytes(buf, &mut c, b"/api/comic?topicKey=").then_some(())?;
    write_url_encoded(buf, &mut c, topic).then_some(())?;
    Some(c)
}

fn build_recent_url(topic: &[u8]) -> Option<usize> {
    let buf = scratch_a();
    let mut c = 0usize;
    write_bytes(buf, &mut c, BASE_URL).then_some(())?;
    write_bytes(buf, &mut c, b"/api/recentUpdate?topicKey=").then_some(())?;
    write_url_encoded(buf, &mut c, topic).then_some(())?;
    Some(c)
}

fn build_detail_url(cid: &[u8]) -> Option<usize> {
    let buf = scratch_a();
    let mut c = 0usize;
    write_bytes(buf, &mut c, BASE_URL).then_some(())?;
    write_bytes(buf, &mut c, b"/api/comic/").then_some(())?;
    write_url_encoded(buf, &mut c, cid).then_some(())?;
    Some(c)
}

fn build_episode_url(cid: &[u8], episode: &[u8]) -> Option<usize> {
    let buf = scratch_a();
    let mut c = 0usize;
    write_bytes(buf, &mut c, BASE_URL).then_some(())?;
    write_bytes(buf, &mut c, b"/api/comic/").then_some(())?;
    write_url_encoded(buf, &mut c, cid).then_some(())?;
    write_bytes(buf, &mut c, b"/episode/").then_some(())?;
    write_url_encoded(buf, &mut c, episode).then_some(())?;
    Some(c)
}

fn scratch_slice(len: usize) -> &'static [u8] {
    unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(SCRATCH_A).cast::<u8>(), len) }
}

fn item_matches_query(obj: &[u8], query: &[u8]) -> bool {
    query.is_empty()
        || extract_json_string(obj, b"title")
            .map(|v| contains_bytes(v, query))
            .unwrap_or(false)
        || extract_json_string(obj, b"subtitle")
            .map(|v| contains_bytes(v, query))
            .unwrap_or(false)
        || extract_json_string(obj, b"introduction")
            .map(|v| contains_bytes(v, query))
            .unwrap_or(false)
        || extract_array_for_key(obj, b"keywords")
            .map(|v| contains_bytes(v, query))
            .unwrap_or(false)
}

fn write_manga_item(payload: &mut [u8], c: &mut usize, obj: &[u8], tag: &[u8]) -> bool {
    let cid = match extract_json_string(obj, b"cid") {
        Some(v) => v,
        None => return true,
    };
    let title = extract_json_string(obj, b"title").unwrap_or(b"Unknown");
    let cover = extract_json_string(obj, b"cover").unwrap_or(b"");
    let author = first_array_string(obj, b"authors").unwrap_or(b"");
    write_bytes(payload, c, br#"{"id":""#)
        && append_json_escaped(payload, c, cid)
        && write_bytes(payload, c, br#"","title":""#)
        && append_json_escaped(payload, c, title)
        && write_bytes(payload, c, br#"","cover":{"kind":"url","url":""#)
        && append_json_escaped(payload, c, cover)
        && write_bytes(payload, c, br#""},"authors":["#)
        && if author.is_empty() {
            true
        } else {
            write_bytes(payload, c, b"\"")
                && append_json_escaped(payload, c, author)
                && write_bytes(payload, c, b"\"")
        }
        && write_bytes(
            payload,
            c,
            br#"],"status":"unknown","contentRating":"safe","sourceTags":["#,
        )
        && write_bytes(payload, c, b"\"")
        && append_json_escaped(payload, c, tag)
        && write_bytes(payload, c, br#""]}"#)
}

fn write_recent_item(payload: &mut [u8], c: &mut usize, obj: &[u8], tag: &[u8]) -> bool {
    let cid = match extract_json_string(obj, b"comicCid") {
        Some(v) => v,
        None => return true,
    };
    let title = extract_json_string(obj, b"title").unwrap_or(b"Unknown");
    let cover = extract_json_string(obj, b"coverUrl").unwrap_or(b"");
    write_bytes(payload, c, br#"{"id":""#)
        && append_json_escaped(payload, c, cid)
        && write_bytes(payload, c, br#"","title":""#)
        && append_json_escaped(payload, c, title)
        && write_bytes(payload, c, br#"","cover":{"kind":"url","url":""#)
        && append_json_escaped(payload, c, cover)
        && write_bytes(
            payload,
            c,
            br#""},"authors":[],"status":"unknown","contentRating":"safe","sourceTags":["#,
        )
        && write_bytes(payload, c, b"\"")
        && append_json_escaped(payload, c, tag)
        && write_bytes(payload, c, br#""]}"#)
}

fn append_topic_items(
    operation: &str,
    topic: &[u8],
    query: &[u8],
    payload: &mut [u8],
    c: &mut usize,
    written: &mut usize,
) -> Result<(), u32> {
    let url_len = build_topic_url(topic)
        .ok_or_else(|| write_error(operation, "internal_error", "url overflow"))?;
    let json = match fetch_json(scratch_slice(url_len)) {
        Ok(v) => v,
        Err(e) => {
            let (code, msg) = fetch_error_code(e);
            return Err(write_error(operation, code, msg));
        }
    };
    let data = extract_array_for_key(json, b"data")
        .ok_or_else(|| write_error(operation, "parse_error", "missing data"))?;
    let mut pos = 0usize;
    while let Some(obj) = next_raw_object(data, &mut pos) {
        if host::check_cancel() {
            return Err(write_error(operation, "cancelled", "operation cancelled"));
        }
        if *written >= 40 {
            break;
        }
        if !item_matches_query(obj, query) {
            continue;
        }
        if *written > 0 && !write_bytes(payload, c, b",") {
            return Err(write_error(operation, "internal_error", "payload overflow"));
        }
        if !write_manga_item(payload, c, obj, topic) {
            return Err(write_error(operation, "internal_error", "payload overflow"));
        }
        *written += 1;
    }
    Ok(())
}

fn append_recent_items(
    operation: &str,
    topic: &[u8],
    payload: &mut [u8],
    c: &mut usize,
    written: &mut usize,
) -> Result<(), u32> {
    let url_len = build_recent_url(topic)
        .ok_or_else(|| write_error(operation, "internal_error", "url overflow"))?;
    let json = match fetch_json(scratch_slice(url_len)) {
        Ok(v) => v,
        Err(e) => {
            let (code, msg) = fetch_error_code(e);
            return Err(write_error(operation, code, msg));
        }
    };
    let data = extract_array_for_key(json, b"data")
        .ok_or_else(|| write_error(operation, "parse_error", "missing data"))?;
    let mut pos = 0usize;
    while let Some(obj) = next_raw_object(data, &mut pos) {
        if *written >= 40 {
            break;
        }
        if *written > 0 && !write_bytes(payload, c, b",") {
            return Err(write_error(operation, "internal_error", "payload overflow"));
        }
        if !write_recent_item(payload, c, obj, topic) {
            return Err(write_error(operation, "internal_error", "payload overflow"));
        }
        *written += 1;
    }
    Ok(())
}

fn run_search(req: &[u8]) -> u32 {
    let query = extract_json_string(req, b"query").unwrap_or(b"");
    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("search", "internal_error", "payload overflow");
    }
    let mut written = 0usize;
    if let Err(e) = append_topic_items("search", TOPIC_TERRA, query, payload, &mut c, &mut written)
    {
        return e;
    }
    if let Err(e) = append_topic_items("search", TOPIC_TALOS, query, payload, &mut c, &mut written)
    {
        return e;
    }
    if !write_bytes(
        payload,
        &mut c,
        br#"],"page":{"nextCursor":null,"hasMore":false}}"#,
    ) {
        return write_error("search", "internal_error", "payload overflow");
    }
    write_success_payload("search", c)
}

fn run_get_manga(req: &[u8]) -> u32 {
    let cid = match extract_json_string(req, b"mangaId") {
        Some(v) => v,
        None => return write_error("get_manga", "invalid_request", "missing mangaId"),
    };
    let url_len = match build_detail_url(cid) {
        Some(n) => n,
        None => return write_error("get_manga", "internal_error", "url overflow"),
    };
    let json = match fetch_json(scratch_slice(url_len)) {
        Ok(v) => v,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_manga", c, m);
        }
    };
    let data = match extract_object_for_key(json, b"data") {
        Some(v) => v,
        None => return write_error("get_manga", "parse_error", "missing data"),
    };
    let title = extract_json_string(data, b"title").unwrap_or(cid);
    let desc = extract_json_string(data, b"introduction").unwrap_or(b"");
    let cover = extract_json_string(data, b"cover").unwrap_or(b"");
    let author = first_array_string(data, b"authors").unwrap_or(b"");
    let keyword = first_array_string(data, b"keywords").unwrap_or(b"");
    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"manga":{"id":""#)
        && append_json_escaped(payload, &mut c, cid)
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
        && write_bytes(payload, &mut c, br#""},"authors":["#)
        && if author.is_empty() {
            true
        } else {
            write_bytes(payload, &mut c, b"\"")
                && append_json_escaped(payload, &mut c, author)
                && write_bytes(payload, &mut c, b"\"")
        }
        && write_bytes(
            payload,
            &mut c,
            br#"],"artists":[],"status":"unknown","contentRating":"safe","language":"zh","tags":["#,
        )
        && if keyword.is_empty() {
            true
        } else {
            write_bytes(payload, &mut c, b"\"")
                && append_json_escaped(payload, &mut c, keyword)
                && write_bytes(payload, &mut c, b"\"")
        }
        && write_bytes(payload, &mut c, br#"],"links":[{"kind":"source","url":""#)
        && write_bytes(payload, &mut c, BASE_URL)
        && write_bytes(payload, &mut c, b"/comic/")
        && append_json_escaped(payload, &mut c, cid)
        && write_bytes(payload, &mut c, br#""}]}}"#);
    if !ok {
        return write_error("get_manga", "internal_error", "payload overflow");
    }
    write_success_payload("get_manga", c)
}

fn episode_type_name(kind: usize) -> &'static [u8] {
    match kind {
        2 => "番外".as_bytes(),
        3 => "贺图".as_bytes(),
        4 => "公告".as_bytes(),
        _ => "正篇".as_bytes(),
    }
}

fn run_get_chapters(req: &[u8]) -> u32 {
    let cid = match extract_json_string(req, b"mangaId") {
        Some(v) => v,
        None => return write_error("get_chapters", "invalid_request", "missing mangaId"),
    };
    let url_len = match build_detail_url(cid) {
        Some(n) => n,
        None => return write_error("get_chapters", "internal_error", "url overflow"),
    };
    let json = match fetch_json(scratch_slice(url_len)) {
        Ok(v) => v,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_chapters", c, m);
        }
    };
    let data = match extract_object_for_key(json, b"data") {
        Some(v) => v,
        None => return write_error("get_chapters", "parse_error", "missing data"),
    };
    let episodes = match extract_array_for_key(data, b"episodes") {
        Some(v) => v,
        None => return write_error("get_chapters", "parse_error", "missing episodes"),
    };
    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("get_chapters", "internal_error", "payload overflow");
    }
    let mut pos = 0usize;
    let mut written = 0usize;
    while let Some(ep) = next_raw_object(episodes, &mut pos) {
        let ep_cid = match extract_json_string(ep, b"cid") {
            Some(v) => v,
            None => continue,
        };
        let title = extract_json_string(ep, b"title").unwrap_or(b"");
        let short = extract_json_string(ep, b"shortTitle").unwrap_or(b"");
        let kind = extract_json_number(ep, b"type")
            .map(|v| parse_usize(v, 1))
            .unwrap_or(1);
        if written > 0 && !write_bytes(payload, &mut c, b",") {
            return write_error("get_chapters", "internal_error", "payload overflow");
        }
        let ok = write_bytes(payload, &mut c, br#"{"id":""#)
            && append_json_escaped(payload, &mut c, cid)
            && write_bytes(payload, &mut c, b"/")
            && append_json_escaped(payload, &mut c, ep_cid)
            && write_bytes(payload, &mut c, br#"","mangaId":""#)
            && append_json_escaped(payload, &mut c, cid)
            && write_bytes(payload, &mut c, br#"","title":""#)
            && if kind == 1 {
                if short.is_empty() {
                    append_json_escaped(payload, &mut c, title)
                } else {
                    append_json_escaped(payload, &mut c, short)
                        && if title.is_empty() { true } else { write_bytes(payload, &mut c, b" ") && append_json_escaped(payload, &mut c, title) }
                }
            } else {
                write_bytes(payload, &mut c, episode_type_name(kind))
                    && if title.is_empty() { true } else { write_bytes(payload, &mut c, b" ") && append_json_escaped(payload, &mut c, title) }
            }
            && write_bytes(payload, &mut c, br#"","chapterNumber":""#)
            && append_json_escaped(payload, &mut c, short)
            && write_bytes(payload, &mut c, br#"","volumeNumber":null,"language":"zh","publishedAt":null,"updatedAt":null,"pageCount":null}"#);
        if !ok {
            return write_error("get_chapters", "internal_error", "payload overflow");
        }
        written += 1;
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
    let (cid, ep_cid) = match split_chapter_id(chapter_id) {
        Some(v) => v,
        None => return write_error("get_pages", "invalid_request", "bad chapterId format"),
    };
    let url_len = match build_episode_url(cid, ep_cid) {
        Some(n) => n,
        None => return write_error("get_pages", "internal_error", "url overflow"),
    };
    let json = match fetch_json(scratch_slice(url_len)) {
        Ok(v) => v,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_pages", c, m);
        }
    };
    let data = match extract_object_for_key(json, b"data") {
        Some(v) => v,
        None => return write_error("get_pages", "parse_error", "missing data"),
    };
    let page_infos = match extract_array_for_key(data, b"pageInfos") {
        Some(v) => v,
        None => return write_error("get_pages", "parse_error", "missing pageInfos"),
    };
    let payload = payload_buf();
    let mut c = 0usize;
    if !(write_bytes(payload, &mut c, br#"{"chapterId":""#)
        && append_json_escaped(payload, &mut c, chapter_id)
        && write_bytes(payload, &mut c, br#"","pages":["#))
    {
        return write_error("get_pages", "internal_error", "payload overflow");
    }
    let mut pos = 0usize;
    let mut index = 0usize;
    while let Some(_page) = next_raw_object(page_infos, &mut pos) {
        if index > 0 && !write_bytes(payload, &mut c, b",") {
            return write_error("get_pages", "internal_error", "payload overflow");
        }
        let page_num = index + 1;
        let ok = write_bytes(payload, &mut c, br#"{"id":"page:"#)
            && append_json_escaped(payload, &mut c, cid)
            && write_bytes(payload, &mut c, b":")
            && append_json_escaped(payload, &mut c, ep_cid)
            && write_bytes(payload, &mut c, b":")
            && write_usize(payload, &mut c, page_num)
            && write_bytes(payload, &mut c, br#"","index":"#)
            && write_usize(payload, &mut c, index)
            && write_bytes(payload, &mut c, br#","image":{"kind":"url","url":""#)
            && write_bytes(payload, &mut c, BASE_URL)
            && write_bytes(payload, &mut c, b"/api/comic/")
            && append_json_escaped(payload, &mut c, cid)
            && write_bytes(payload, &mut c, b"/episode/")
            && append_json_escaped(payload, &mut c, ep_cid)
            && write_bytes(payload, &mut c, b"/page?pageNum=")
            && write_usize(payload, &mut c, page_num)
            && write_bytes(payload, &mut c, br#""}}"#);
        if !ok {
            return write_error("get_pages", "internal_error", "payload overflow");
        }
        index += 1;
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
    let resolved = if contains_bytes(url, b"comic.hypergryph.com/api/comic/")
        && contains_bytes(url, b"/page?pageNum=")
    {
        let json = match fetch_json(url) {
            Ok(v) => v,
            Err(e) => {
                let (c, m) = fetch_error_code(e);
                return write_error("get_image_request", c, m);
            }
        };
        let data = match extract_object_for_key(json, b"data") {
            Some(v) => v,
            None => return write_error("get_image_request", "parse_error", "missing data"),
        };
        match extract_json_string(data, b"url") {
            Some(v) => v,
            None => return write_error("get_image_request", "parse_error", "missing url"),
        }
    } else {
        url
    };
    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"url":""#)
        && append_json_escaped(payload, &mut c, resolved)
        && write_bytes(payload, &mut c, br#"","headers":{"Referer":"https://comic.hypergryph.com/","User-Agent":"Mozilla/5.0 (Linux; Android 14) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Mobile Safari/537.36"}}"#);
    if !ok {
        return write_error("get_image_request", "internal_error", "payload overflow");
    }
    write_success_payload("get_image_request", c)
}

fn run_get_listings(_req: &[u8]) -> u32 {
    run_get_manga_list(br#"{"listingId":"popular","page":1}"#)
}

fn run_get_manga_list(req: &[u8]) -> u32 {
    let listing = extract_json_string(req, b"listingId").unwrap_or(b"popular");
    let page = request_page(req);
    let topic = if listing == b"talos" || page == 2 {
        TOPIC_TALOS
    } else {
        TOPIC_TERRA
    };
    let latest = listing == b"latest" || listing == b"latest-talos";
    let topic = if listing == b"latest-talos" {
        TOPIC_TALOS
    } else {
        topic
    };
    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("get_manga_list", "internal_error", "payload overflow");
    }
    let mut written = 0usize;
    let result = if latest {
        append_recent_items("get_manga_list", topic, payload, &mut c, &mut written)
    } else {
        append_topic_items("get_manga_list", topic, b"", payload, &mut c, &mut written)
    };
    if let Err(e) = result {
        return e;
    }
    let has_more = !latest && page == 1;
    if !(write_bytes(payload, &mut c, br#"],"page":{"nextCursor":"#)
        && if has_more {
            write_bytes(payload, &mut c, br#""2""#)
        } else {
            write_bytes(payload, &mut c, b"null")
        }
        && write_bytes(payload, &mut c, br#","hasMore":"#)
        && write_bytes(payload, &mut c, if has_more { b"true" } else { b"false" })
        && write_bytes(payload, &mut c, b"}}"))
    {
        return write_error("get_manga_list", "internal_error", "payload overflow");
    }
    write_success_payload("get_manga_list", c)
}

fn run_get_home(_req: &[u8]) -> u32 {
    const HOME: &str = r#"{"sections":[{"title":"最新更新","listingId":"latest"},{"title":"泰拉记事社","listingId":"popular"},{"title":"塔洛斯二号记事社","listingId":"talos"}]}"#;
    let bytes = HOME.as_bytes();
    let payload = payload_buf();
    if bytes.len() > payload.len() {
        return write_error("get_home", "internal_error", "payload overflow");
    }
    payload[..bytes.len()].copy_from_slice(bytes);
    write_success_payload("get_home", bytes.len())
}

fn run_get_filters(_req: &[u8]) -> u32 {
    const FILTERS: &[u8] = br#"{"filters":[]}"#;
    let payload = payload_buf();
    payload[..FILTERS.len()].copy_from_slice(FILTERS);
    write_success_payload("get_filters", FILTERS.len())
}

fn run_get_settings(_req: &[u8]) -> u32 {
    const SETTINGS: &[u8] = br#"{"settings":[]}"#;
    let payload = payload_buf();
    payload[..SETTINGS.len()].copy_from_slice(SETTINGS);
    write_success_payload("get_settings", SETTINGS.len())
}

#[no_mangle]
pub extern "C" fn koma_source_init(_manifest_ptr: u32, manifest_len: u32) -> i32 {
    log_info(b"terrahistoricus source init");
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
    log_info(b"terrahistoricus search");
    run_search(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga", "invalid_request", "empty request"),
    };
    log_info(b"terrahistoricus get_manga");
    run_get_manga(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_chapters(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_chapters", "invalid_request", "empty request"),
    };
    log_info(b"terrahistoricus get_chapters");
    run_get_chapters(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_pages(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_pages", "invalid_request", "empty request"),
    };
    log_info(b"terrahistoricus get_pages");
    run_get_pages(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_listings(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_listings", "invalid_request", "empty request"),
    };
    log_info(b"terrahistoricus get_listings");
    run_get_listings(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga_list(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga_list", "invalid_request", "empty request"),
    };
    log_info(b"terrahistoricus get_manga_list");
    run_get_manga_list(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_home(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_home", "invalid_request", "empty request"),
    };
    log_info(b"terrahistoricus get_home");
    run_get_home(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_filters(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_filters", "invalid_request", "empty request"),
    };
    log_info(b"terrahistoricus get_filters");
    run_get_filters(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_settings(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_settings", "invalid_request", "empty request"),
    };
    log_info(b"terrahistoricus get_settings");
    run_get_settings(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_image_request(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_image_request", "invalid_request", "empty request"),
    };
    log_info(b"terrahistoricus get_image_request");
    run_get_image_request(req)
}

#[no_mangle]
pub extern "C" fn koma_source_free(result_ptr: u32) {
    response_buffer().free(result_ptr)
}
