#![no_std]

extern crate koma_source_sdk;

use koma_source_sdk::host::{self, http_request, log_info};
use koma_source_sdk::json_utils::{
    append_json_escaped as append_bytes_escaped, contains_bytes, find_subslice,
    write_bytes, write_usize,
};
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{FetchError, build_get_request, fetch_error_code, parse_status_code};

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
    version: "0.1.7",
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
    home: false,
    filters: false,
    settings: false,
    credentials: false,
    image_request: true,
};


fn body_slice(len: usize) -> &'static [u8] {
    unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(BODY_BUF) as *const u8, len) }
}

fn fetch_json(url: &[u8]) -> Result<&'static [u8], FetchError> {
    let req_len = build_get_request(http_req_buf(), url, None, &[]).ok_or(FetchError::Network)?;
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
    let status = json_object_member(resp, b"status").or_else(|| json_object_member(resp, b"statusCode"))
        .map(parse_status_code).unwrap_or(0);
    match status {
        200..=299 => {},
        404 => return Err(FetchError::NotFound),
        429 => return Err(FetchError::RateLimit),
        400..=499 => return Err(FetchError::ClientError),
        500..=599 => return Err(FetchError::ServerError),
        _ => return Err(FetchError::Network),
    }

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

fn request_page(req: &[u8]) -> Option<usize> {
    let args = json_object_member(req, b"args").unwrap_or(req);
    let raw = match json_object_member(args, b"page") { Some(v) => v, None => return Some(1) };
    let value = if raw.first() == Some(&b'{') { json_object_member(raw, b"cursor").unwrap_or(b"null") } else { raw };
    if value == b"null" { return Some(1); }
    let bytes = if value.len() >= 2 && value.first() == Some(&b'"') && value.last() == Some(&b'"') { &value[1..value.len()-1] } else { value };
    if bytes.is_empty() { return None; }
    let mut page = 0usize;
    let mut digits=JsonBytes::new(bytes);
    while let Some(digit)=digits.next().ok()? { if !digit.is_ascii_digit() { return None; } page=page.checked_mul(10)?.checked_add((digit-b'0')as usize)?; }
    if page == 0 { None } else { Some(page) }
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

// Raw string contents are borrowed from direct members; decoding is streamed
// to the existing destination, without a new per-field buffer or length limit.
#[derive(Clone)]
struct JsonBytes<'a> { raw: &'a [u8], at: usize, pending: [u8; 4], used: usize, len: usize }
impl<'a> JsonBytes<'a> {
    fn new(raw: &'a [u8]) -> Self { Self { raw, at: 0, pending: [0;4], used: 0, len: 0 } }
    fn hex(&mut self) -> Result<u32, &'static str> {
        let mut n=0;
        for _ in 0..4 { let b=*self.raw.get(self.at).ok_or("incomplete JSON escape")?; self.at+=1;
            n=(n<<4)|match b { b'0'..=b'9'=>(b-b'0')as u32,b'a'..=b'f'=>(b-b'a'+10)as u32,b'A'..=b'F'=>(b-b'A'+10)as u32,_=>return Err("invalid JSON escape") }; }
        Ok(n)
    }
    fn next(&mut self) -> Result<Option<u8>, &'static str> {
        if self.used<self.len { let b=self.pending[self.used];self.used+=1;return Ok(Some(b)); }
        let Some(&b)=self.raw.get(self.at) else { return Ok(None); };self.at+=1;
        if b!=b'\\' { return Ok(Some(b)); }
        let e=*self.raw.get(self.at).ok_or("incomplete JSON escape")?;self.at+=1;
        let cp=match e { b'"'|b'\\'|b'/'=>return Ok(Some(e)),b'b'=>return Ok(Some(8)),b'f'=>return Ok(Some(12)),b'n'=>return Ok(Some(10)),b'r'=>return Ok(Some(13)),b't'=>return Ok(Some(9)),b'u'=>self.hex()?,_=>return Err("invalid JSON escape") };
        let cp=if (0xd800..=0xdbff).contains(&cp) {
            if self.raw.get(self.at..self.at+2)!=Some(b"\\u") { return Err("missing low surrogate"); }self.at+=2;
            let low=self.hex()?;if !(0xdc00..=0xdfff).contains(&low) { return Err("invalid low surrogate"); }
            0x10000+((cp-0xd800)<<10)+(low-0xdc00)
        } else { if (0xdc00..=0xdfff).contains(&cp) { return Err("unexpected low surrogate"); }cp };
        self.len=koma_source_sdk::json_utils::encode_utf8(cp,&mut self.pending);self.used=1;Ok(Some(self.pending[0]))
    }
}
fn append_json_escaped(out: &mut [u8], at: &mut usize, raw: &[u8]) -> bool {
    let mut bytes=JsonBytes::new(raw);
    loop { match bytes.next() { Ok(Some(b))=>if !append_bytes_escaped(out,at,&[b]) { return false; },Ok(None)=>return true,Err(_)=>return false } }
}
fn write_url_encoded(out: &mut [u8], at: &mut usize, raw: &[u8]) -> bool {
    let mut bytes=JsonBytes::new(raw);
    loop { match bytes.next() { Ok(Some(b))=>if !koma_source_sdk::json_utils::write_url_encoded(out,at,&[b]) { return false; },Ok(None)=>return true,Err(_)=>return false } }
}
fn contains_json_text(raw: &[u8], query: &[u8]) -> bool {
    let mut start=JsonBytes::new(raw);
    loop { let mut hay=start.clone();let mut needle=JsonBytes::new(query);
        loop { match needle.next() { Ok(None)=>return true,Ok(Some(n))=>if hay.next()!=Ok(Some(n)) { break; },Err(_)=>return false } }
        match start.next() { Ok(Some(_))=>{},_=>return false }
    }
}
fn json_text_eq(raw: &[u8], plain: &[u8]) -> bool {
    let mut bytes=JsonBytes::new(raw);
    for &expected in plain { if bytes.next()!=Ok(Some(expected)) { return false; } }
    bytes.next()==Ok(None)
}
fn json_string_end(data: &[u8], start: usize) -> Option<usize> {
    if data.get(start)!=Some(&b'"') { return None; }
    let mut at=start+1;
    while at<data.len() { match data[at] { b'\\'=>at+=2,b'"'=>return Some(at+1),_=>at+=1 } }
    None
}
// Walk direct members only. Keys use JSON string semantics, including escapes;
// nested namesakes and quoted key-like text never supply an outer field.
fn json_object_member<'a>(data: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let mut at=0;
    while data.get(at).map_or(false,|b|b.is_ascii_whitespace()) { at+=1; }
    if data.get(at)!=Some(&b'{') { return None; }at+=1;
    loop {
        while data.get(at).map_or(false,|b|b.is_ascii_whitespace()) { at+=1; }
        if data.get(at)==Some(&b'}') { return None; }
        let key_start=at+1;let key_end=json_string_end(data,at)?;at=key_end;
        while data.get(at).map_or(false,|b|b.is_ascii_whitespace()) { at+=1; }
        if data.get(at)!=Some(&b':') { return None; }at+=1;
        while data.get(at).map_or(false,|b|b.is_ascii_whitespace()) { at+=1; }
        let start=at;
        at=match *data.get(at)? {
            b'"'=>json_string_end(data,at)?,
            b'{'=>at+extract_balanced(data,at,b'{',b'}')?.len(),
            b'['=>at+extract_balanced(data,at,b'[',b']')?.len(),
            _=>{while at<data.len() && !matches!(data[at],b','|b'}') {at+=1;}at},
        };
        let mut end=at;while end>start && data[end-1].is_ascii_whitespace() {end-=1;}
        if json_text_eq(&data[key_start..key_end-1],key) { return data.get(start..end); }
        while data.get(at).map_or(false,|b|b.is_ascii_whitespace()) {at+=1;}
        match data.get(at) {Some(b',')=>at+=1,_=>return None}
    }
}
fn extract_json_string<'a>(data: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let value=json_object_member(data,key)?;
    if value.len()>=2 && value.first()==Some(&b'"') && value.last()==Some(&b'"') { Some(&value[1..value.len()-1]) } else { None }
}
fn request_string<'a>(request: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    extract_json_string(json_object_member(request,b"args").unwrap_or(request),key)
}
fn extract_json_number<'a>(data: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let value=json_object_member(data,key)?;
    if value.first().map_or(false,|b|b.is_ascii_digit()||*b==b'-') { Some(value) } else { None }
}
fn extract_object_for_key<'a>(data: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let value=json_object_member(data,key)?;
    if value.first()==Some(&b'{') && value.last()==Some(&b'}') { Some(value) } else { None }
}
fn extract_array_for_key<'a>(data: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let value=json_object_member(data,key)?;
    if value.first()==Some(&b'[') && value.last()==Some(&b']') { Some(&value[1..value.len()-1]) } else { None }
}

// Array content excludes the outer brackets. The cursor points immediately
// after the previous object, so only end-of-content is a successful terminal.
fn next_raw_object<'a>(data: &'a [u8], pos: &mut usize) -> Result<Option<&'a [u8]>, &'static str> {
    let after_object = *pos > 0;
    while *pos < data.len() && matches!(data[*pos], b' ' | b'\t' | b'\n' | b'\r') { *pos += 1; }
    if *pos == data.len() { return Ok(None); }
    if after_object {
        if data[*pos] != b',' { return Err("missing array separator"); }
        *pos += 1;
        while *pos < data.len() && matches!(data[*pos], b' ' | b'\t' | b'\n' | b'\r') { *pos += 1; }
        if *pos == data.len() { return Err("missing array element"); }
    }
    if data[*pos] != b'{' { return Err("invalid array element"); }
    let obj = extract_balanced(data, *pos, b'{', b'}').ok_or("incomplete array object")?;
    *pos += obj.len();
    Ok(Some(obj))
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
    let mut bytes=JsonBytes::new(id);
    loop { let start=bytes.at;match bytes.next().ok()? {
        Some(b'/')=>return if start>0 && bytes.at<id.len() { Some((&id[..start],&id[bytes.at..])) }else { None },
        Some(_)=>{},None=>return None,
    } }
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
            .map(|v| contains_json_text(v, query))
            .unwrap_or(false)
        || extract_json_string(obj, b"subtitle")
            .map(|v| contains_json_text(v, query))
            .unwrap_or(false)
        || extract_json_string(obj, b"introduction")
            .map(|v| contains_json_text(v, query))
            .unwrap_or(false)
        || extract_array_for_key(obj, b"keywords")
            .map(|v| contains_json_text(v, query))
            .unwrap_or(false)
}

fn write_manga_item(payload: &mut [u8], c: &mut usize, obj: &[u8], tag: &[u8]) -> bool {
    let cid = match extract_json_string(obj, b"cid") {
        Some(v) => v,
        None => return false,
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
        None => return false,
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
    loop {
        let obj = match next_raw_object(data, &mut pos) {
            Ok(Some(value)) => value,
            Ok(None) => break,
            Err(message) => return Err(write_error(operation, "parse_error", message)),
        };
        if host::check_cancel() {
            return Err(write_error(operation, "cancelled", "operation cancelled"));
        }
        if extract_json_string(obj,b"cid").map_or(true, |id|id.is_empty()) {
            return Err(write_error(operation,"parse_error","missing comic identity"));
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
    loop {
        let obj = match next_raw_object(data, &mut pos) {
            Ok(Some(value)) => value,
            Ok(None) => break,
            Err(message) => return Err(write_error(operation, "parse_error", message)),
        };
        if extract_json_string(obj,b"comicCid").map_or(true, |id|id.is_empty()) {
            return Err(write_error(operation,"parse_error","missing comic identity"));
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
    let query = request_string(req, b"query").unwrap_or(b"");
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
    let cid = match request_string(req, b"mangaId") {
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

// shortTitle is nullable text upstream. A complete decimal label may order
// regular episodes; arbitrary captions, page counts and extras may not.
fn numeric_episode_label(label: &[u8]) -> bool {
    let mut bytes=JsonBytes::new(label);let mut digits=0;let mut decimal=false;let mut fraction=0;
    loop { match bytes.next() {
        Ok(Some(b'0'..=b'9'))=>if decimal { fraction+=1; }else { digits+=1; },
        Ok(Some(b'.')) if !decimal && digits>0=>decimal=true,
        Ok(None)=>return digits>0 && (!decimal || fraction>0),
        _=>return false,
    } }
}

fn run_get_chapters(req: &[u8]) -> u32 {
    let cid = match request_string(req, b"mangaId") {
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
    loop {
        let ep = match next_raw_object(episodes, &mut pos) {
            Ok(Some(value)) => value,
            Ok(None) => break,
            Err(message) => return write_error("get_chapters", "parse_error", message),
        };
        let ep_cid = match extract_json_string(ep, b"cid") {
            Some(v) if !v.is_empty() => v,
            _ => return write_error("get_chapters","parse_error","missing episode identity"),
        };
        let title = extract_json_string(ep, b"title").unwrap_or(b"");
        let short = extract_json_string(ep, b"shortTitle").unwrap_or(b"");
        let raw_kind = extract_json_number(ep, b"type");
        let kind = raw_kind.map(|v| parse_usize(v, 1)).unwrap_or(1);
        // Only a numeric label of an explicitly regular episode can order it.
        // Extras' shortTitle, resource cid and array position are not ordinals.
        let numbered_regular = raw_kind == Some(b"1") && numeric_episode_label(short);
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
            && write_bytes(payload, &mut c, br#"","chapterNumber":"#)
            && if numbered_regular {
                write_bytes(payload, &mut c, b"\"")
                    && append_json_escaped(payload, &mut c, short)
                    && write_bytes(payload, &mut c, b"\"")
            } else { write_bytes(payload, &mut c, b"null") }
            && write_bytes(payload, &mut c, br#", "readingGroup":"#)
            && write_bytes(payload, &mut c, if numbered_regular { b"\"chapter\"" } else { b"null" })
            && write_bytes(payload, &mut c, br#", "continuationEligible":"#)
            && write_bytes(payload, &mut c, if numbered_regular { b"true" } else { b"false" })
            && write_bytes(payload, &mut c, br#", "volumeNumber":null,"publishedAt":null,"updatedAt":null,"pageCount":null}"#);
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
    let chapter_id = match request_string(req, b"chapterId") {
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
    loop {
        let _page = match next_raw_object(page_infos, &mut pos) {
            Ok(Some(value)) => value,
            Ok(None) => break,
            Err(message) => return write_error("get_pages", "parse_error", message),
        };
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
    let url = request_string(req, b"url")
        .or_else(|| request_string(req, b"imageUrl"))
        .unwrap_or(b"");
    if url.is_empty() {
        return write_error("get_image_request", "invalid_request", "missing url");
    }
    let resolves_api=contains_json_text(url,b"comic.hypergryph.com/api/comic/") && contains_json_text(url,b"/page?pageNum=");
    // Only an API-resolving URI needs a contiguous request input. The existing
    // HTTP request budget is smaller than scratch; direct image plans stream
    // their JSON text and acquire no new per-URI length limit.
    let url=if resolves_api {
        let mut decoded=JsonBytes::new(url);let mut length=0;
        loop { match decoded.next() {
            Ok(Some(b))=>if !write_bytes(scratch_b(),&mut length,&[b]) { return write_error("get_image_request","internal_error","url overflow"); },
            Ok(None)=>break,Err(message)=>return write_error("get_image_request","invalid_request",message),
        } }
        unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(SCRATCH_B).cast::<u8>(),length) }
    } else { url };
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
        && append_json_escaped(payload,&mut c,resolved)
        && write_bytes(payload, &mut c, br#"","headers":{"Referer":"https://comic.hypergryph.com/","User-Agent":"Mozilla/5.0 (Linux; Android 14) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Mobile Safari/537.36"}}"#);
    if !ok {
        return write_error("get_image_request", "internal_error", "payload overflow");
    }
    write_success_payload("get_image_request", c)
}

fn run_get_listings(_req: &[u8]) -> u32 {
    let data = r#"{"listings":[{"id":"popular","name":"全部漫画","supportsPaging":true},{"id":"latest","name":"最新更新","supportsPaging":true}]}"#.as_bytes();
    payload_buf()[..data.len()].copy_from_slice(data);
    write_success_payload("get_listings", data.len())
}

fn run_get_manga_list(req: &[u8]) -> u32 {
    let listing = request_string(req, b"listingId").unwrap_or(b"popular");
    let page = match request_page(req) { Some(n) => n, None => return write_error("get_manga_list", "invalid_request", "invalid page cursor") };
    let explicit_talos = json_text_eq(listing,b"talos") || json_text_eq(listing,b"latest-talos");
    if !json_text_eq(listing,b"popular") && !json_text_eq(listing,b"latest") && !explicit_talos { return write_error("get_manga_list", "invalid_request", "unknown listing"); }
    if page > 2 || (explicit_talos && page > 1) {
        let data = br#"{"items":[],"page":{"nextCursor":null,"hasMore":false}}"#;
        payload_buf()[..data.len()].copy_from_slice(data);
        return write_success_payload("get_manga_list", data.len());
    }
    let topic = if json_text_eq(listing,b"talos") || page == 2 {
        TOPIC_TALOS
    } else {
        TOPIC_TERRA
    };
    let latest = json_text_eq(listing,b"latest") || json_text_eq(listing,b"latest-talos");
    let topic = if json_text_eq(listing,b"latest-talos") {
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
    let has_more = !explicit_talos && page == 1;
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
