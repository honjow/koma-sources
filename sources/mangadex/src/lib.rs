#![no_std]

extern crate koma_source_sdk;

use koma_source_sdk::host::{self, http_request, log_info};
use koma_source_sdk::json_utils::{
    JsonArrayIter, JsonObjectStringIter, append_json_escaped, contains_bytes,
    extract_json_number, extract_json_string, find_subslice, write_bytes, write_url_encoded,
    write_usize, json_object_member,
};
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{FetchError, build_get_request, fetch_error_code, parse_status_code};

const API_BASE: &[u8] = b"https://api.mangadex.org";
const COVERS_BASE: &[u8] = b"https://uploads.mangadex.org/covers/";
const MANGADEX_USER_AGENT: &[u8] = b"Koma/0.1 (+https://github.com/honjow/Koma)";
koma_source_sdk::koma_source_buffers! {
    payload: 128 * 1024,
    http_out: 512 * 1024,
    body: 2 * 1024 * 1024,
    http_req: 32768,
    scratch: 65536,
}
koma_source_sdk::koma_source_helpers!();

// Search decoding must not alias the URL buffer or consume the bounded WASM stack.
static mut SEARCH_QUERY: [u8; 65536] = [0; 65536];
// MangaDex's 20-item browse responses can exceed 256 KiB once cover-art and
// relationship metadata are included. Keep the decoded JSON buffer below the
// host response capacity while preserving the page size expected by Koma.
const JSON_BUF_CAP: usize = 512 * 1024;

#[path = "../first_volume_cover_en.rs"]
mod first_volume_cover;
#[path = "../original_language.rs"]
mod original_language;
#[path = "../content_rating.rs"]
mod content_rating;
#[path = "../final_chapter_description.rs"]
mod final_chapter_description;
#[path = "../title_preference.rs"]
mod title_preference;
#[path = "../../mangadex/blacklist.rs"]
mod blacklist;
static mut COVER_JSON: [u8; JSON_BUF_CAP] = [0; JSON_BUF_CAP];

static mut JSON_BUF: [u8; JSON_BUF_CAP] = [0; JSON_BUF_CAP];

const ALT_TITLE_LANG: &[u8] = b"en";
const ALT_DESCRIPTION_KEY: &[u8] = b"altTitlesInDesc_en";

fn alt_description_enabled(req: &[u8]) -> bool {
    json_object_member(req,b"settings").and_then(|v|json_object_member(v,ALT_DESCRIPTION_KEY))
        .map(|v|v==b"true" || v==b"\"true\"").unwrap_or(false)
}
// Source settings only append a description block. Titles, identity, chapter
// order and the default description bytes are deliberately unchanged.
fn append_description_content(out: &mut [u8], n: &mut usize, attributes: &[u8], base: &[u8], alternatives: bool, final_chapter: bool) -> bool {
    let start = *n;
    if !append_alt_description_content(out, n, attributes, base, alternatives) { return false; }
    !final_chapter || final_chapter_description::append(out, n, attributes, *n > start)
}
fn append_alt_description_content(out: &mut [u8], n: &mut usize, attributes: &[u8], base: &[u8], enabled: bool) -> bool {
    if !enabled { return write_bytes(out,n,base); }
    let original = json_object_member(attributes,b"originalLanguage").unwrap_or(b"null");
    let romanized = match original { b"\"ja\""=>b"ja-ro".as_slice(), b"\"ko\""=>b"ko-ro", b"\"zh\"" | b"\"zh-hk\""=>b"zh-ro", _=>b"" };
    if !write_bytes(out,n,base) { return false; }
    let mut started=false;
    if let Some(alts)=json_object_member(attributes,b"altTitles") {
        let Some(mut alts)=JsonArrayIter::from_raw_array(alts) else { return false; };
        while let Some(item)=alts.next_object() {
            if json_object_member(item,ALT_TITLE_LANG).is_none() && json_object_member(item,romanized).is_none() { continue; }
            let Some(mut values)=JsonObjectStringIter::new(item) else { return false; };
            let Some(value)=values.next_string() else { continue; };
            if value.is_empty() || values.next_string().is_some() { continue; }
            if !started {
                if !base.is_empty() && !write_bytes(out,n,b"\\n\\n") { return false; }
                if !write_bytes(out,n,b"Alternative titles:\\n") { return false; }
            } else if !write_bytes(out,n,b"\\n") { return false; }
            if !write_bytes(out,n,"• ".as_bytes()) || !koma_source_sdk::plain_text::append_entity_decoded_json_content(out,n,value) { return false; }
            started=true;
        }
    }
    true
}
fn nonempty_title_string(value: &[u8]) -> Option<&[u8]> {
    if value.len() >= 2 && value.first() == Some(&b'"') && value.last() == Some(&b'"') {
        let title = &value[1..value.len() - 1];
        if title.iter().any(|b| !b.is_ascii_whitespace()) {
            return Some(title);
        }
    }
    None
}

fn manga_title(obj: &[u8]) -> &[u8] {
    let attributes = json_object_member(obj, b"attributes").unwrap_or(b"");
    let primary = json_object_member(attributes, b"title").unwrap_or(b"");
    let languages: [&[u8]; 3] = [b"en", b"ja", b"ja-ro"];
    for language in languages {
        if let Some(title) = json_object_member(primary, language).and_then(nonempty_title_string) {
            return title;
        }
    }
    // Preferred-language alternatives never replace an existing preferred primary title.
    if let Some(alternatives) = json_object_member(attributes, b"altTitles") {
        for language in languages {
            if let Some(mut iter) = JsonArrayIter::from_raw_array(alternatives) {
                while let Some(alternate) = iter.next_object() {
                    if let Some(title) = json_object_member(alternate, language).and_then(nonempty_title_string) {
                        return title;
                    }
                }
            }
        }
    }
    if let Some(mut iter) = JsonObjectStringIter::new(primary) {
        while let Some(title) = iter.next_string() {
            if title.iter().any(|b| !b.is_ascii_whitespace()) {
                return title;
            }
        }
    }
    b"Unknown"
}

// Only manga card/detail/Home titles consume this preference. Chapter titles
// are a separate provider field and must never enter the manga selector.
fn append_manga_title(out: &mut [u8], n: &mut usize, obj: &[u8], req: &[u8]) -> bool {
    if title_preference::prefer(req, b"preferExtensionLangTitle_en") {
        append_api_string(out, n, manga_title(obj))
    } else {
        title_preference::append_primary_first(out, n, api_attributes(obj), b"en")
    }
}

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "org.mangadex.koma",
    name: "MangaDex",
    version: "0.1.25",
    api_version: "0.2",
    language: "en",
    author: "Koma",
    description: "MangaDex API source (JSON, no HTML scraping).",
    content_rating: "unknown",
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
    credentials: false,
};


fn json_buf() -> &'static mut [u8] {
    unsafe { &mut *core::ptr::addr_of_mut!(JSON_BUF) }
}

// Follow the fixed MangaDex cover variants. `original` encodes the upstream
// empty suffix because Host option IDs cannot be blank. No setting keeps every
// original URL and existing language/rating/chapter behavior unchanged.
fn cover_quality(req: &[u8]) -> &'static [u8] {
    let value = json_object_member(req, b"settings")
        .and_then(|settings| json_object_member(settings, b"thumbnailQuality_en"));
    match value { Some(br#"".512.jpg""#) => b".512.jpg", Some(br#"".256.jpg""#) => b".256.jpg", _ => b"" }
}
fn append_cover_filename(out: &mut [u8], n: &mut usize, filename: &[u8], req: &[u8]) -> bool {
    append_api_string(out, n, filename) && write_bytes(out, n, cover_quality(req))
}

// --- HTTP helper ---

fn api_string<'a>(obj: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let value = json_object_member(obj, key)?;
    if value.len() >= 2 && value.first() == Some(&b'"') && value.last() == Some(&b'"') {
        Some(&value[1..value.len() - 1])
    } else { None }
}

fn api_number<'a>(obj: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let value = json_object_member(obj, key)?;
    if !value.is_empty() && value.iter().all(u8::is_ascii_digit) { Some(value) } else { None }
}

fn api_array<'a>(obj: &'a [u8], key: &[u8]) -> Option<JsonArrayIter<'a>> {
    let value = json_object_member(obj, key)?;
    if value.first() != Some(&b'[') || value.last() != Some(&b']') { return None; }
    JsonArrayIter::from_raw_array(value)
}

fn api_attributes(obj: &[u8]) -> &[u8] {
    json_object_member(obj, b"attributes").unwrap_or(b"{}")
}

// API string contents already carry JSON escaping, including surrogate pairs.
// Copy them unchanged inside an output JSON string; do not double-escape them.
fn append_api_string(dst: &mut [u8], cursor: &mut usize, raw: &[u8]) -> bool {
    write_bytes(dst, cursor, raw)
}

fn fetch_json(url_bytes: &[u8]) -> Result<&'static [u8], FetchError> {
    fetch_json_into(url_bytes, json_buf())
}
fn first_covers(req: &[u8], json: &[u8]) -> first_volume_cover::Covers {
    first_volume_cover::load(req, b"tryUsingFirstVolumeCover_en", json, |url| {
        fetch_json_into(url, unsafe { &mut *core::ptr::addr_of_mut!(COVER_JSON) }).ok()
    })
}
fn fetch_json_into(url_bytes: &[u8], out: &'static mut [u8]) -> Result<&'static [u8], FetchError> {
    let req_len = build_get_request(
        http_req_buf(),
        url_bytes,
        None,
        &[(b"User-Agent", MANGADEX_USER_AGENT)],
    ).ok_or(FetchError::Network)?;
    // Retry transport-level failures up to 3 times.
    let mut resp_len = 0usize;
    let mut transport_failed = true;
    for attempt in 0..3u8 {
        let req_slice = &http_req_buf()[..req_len];
        match http_request(req_slice, http_out()) {
            Ok(n) => { resp_len = n; transport_failed = false; break; }
            Err(_) => {
                if attempt < 2 { log_info(b"mangadex: http transport error, retrying"); }
            }
        }
    }
    if transport_failed { return Err(FetchError::Network); }
    let resp = &http_out()[..resp_len];
    if let Some(code_bytes) = extract_json_number(resp, b"status") {
        match parse_status_code(code_bytes) {
            200..=299 => {}
            404 => return Err(FetchError::NotFound),
            429 => return Err(FetchError::RateLimit),
            400..=499 => return Err(FetchError::ClientError),
            500..=599 => return Err(FetchError::ServerError),
            _ => return Err(FetchError::Network),
        }
    }
    if !contains_bytes(resp, br#""ok":true"#) {
        log_info(b"mangadex: http response not ok");
        let err = if let Some(code_bytes) = extract_json_number(resp, b"statusCode") {
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
    let body_marker = b"\"bodyText\":\"";
    let body_start = find_subslice(resp, body_marker).ok_or(FetchError::Network)? + body_marker.len();
    let mut out_cursor = 0usize;
    let mut i = body_start;
    let mut body_closed = false;
    while i < resp.len() {
        let b = resp[i];
        if b == b'\\' && i + 1 < resp.len() {
            let next = resp[i + 1];
            let unescaped = match next {
                b'"' => b'"', b'\\' => b'\\', b'/' => b'/',
                b'n' => b'\n', b'r' => b'\r', b't' => b'\t',
                _ => next,
            };
            if out_cursor >= out.len() { return Err(FetchError::Network); }
            out[out_cursor] = unescaped;
            out_cursor += 1;
            i += 2;
            continue;
        }
        if b == b'"' { body_closed = true; break; }
        if out_cursor >= out.len() { return Err(FetchError::Network); }
        out[out_cursor] = b;
        out_cursor += 1;
        i += 1;
    }
    if !body_closed { return Err(FetchError::Network); }
    Ok(unsafe { core::slice::from_raw_parts(out.as_ptr(), out_cursor) })
}

// --- Operations ---

// Explicit single-class filters win; All delegates to the saved default set.
fn content_rating_query(req: &[u8], use_filter: bool) -> &'static [u8] {
    let args = json_object_member(req, b"args").unwrap_or(req);
    let filter = if use_filter {
        json_object_member(args, b"filters").unwrap_or(args)
    } else { b"{}" };
    let explicit = json_object_member(filter, b"contentRating").and_then(nonempty_title_string);
    match explicit {
        Some(b"safe") => return b"&contentRating%5B%5D=safe",
        Some(b"suggestive") => return b"&contentRating%5B%5D=suggestive",
        Some(b"erotica") => return b"&contentRating%5B%5D=erotica",
        _ => {},
    }
    let default = json_object_member(req, b"settings")
        .and_then(|settings| json_object_member(settings, b"defaultContentRating"))
        .and_then(nonempty_title_string);
    match default {
        Some(b"safe") => b"&contentRating%5B%5D=safe",
        Some(b"erotica") => b"&contentRating%5B%5D=safe&contentRating%5B%5D=suggestive&contentRating%5B%5D=erotica",
        _ => b"&contentRating%5B%5D=safe&contentRating%5B%5D=suggestive",
    }
}

// Public settings migration is request-scoped. Old Hosts retain the published
// three-value selector; only the context-capable Host consumes the new array.
fn rating_context_supported(req:&[u8])->bool {
    let Some(raw)=json_object_member(req,b"hostFeatures") else{return false};
    raw.starts_with(b"[")&&raw.ends_with(b"]")&&raw[1..raw.len()-1].split(|b|*b==b',').any(|v|v.trim_ascii()==br#""source_settings_context_v1""#)
}
fn append_content_rating_query(out:&mut [u8],n:&mut usize,req:&[u8],use_filter:bool)->bool {
    let args=json_object_member(req,b"args").unwrap_or(req);
    let filters=if use_filter{json_object_member(args,b"filters").unwrap_or(args)}else{b"{}"};
    let explicit=json_object_member(filters,b"contentRating").and_then(nonempty_title_string);
    let modern=json_object_member(req,b"settings").and_then(|v|json_object_member(v,b"contentRating_en"));
    if !rating_context_supported(req)||modern.is_none()||matches!(explicit,Some(b"safe"|b"suggestive"|b"erotica")) {
        write_bytes(out,n,content_rating_query(req,use_filter))
    } else {
        content_rating::append_query(out,n,req,b"contentRating_en")
    }
}
fn schema_rating_default(values:&[u8])->&'static [u8] {
    if json_object_member(values,b"contentRating_en").is_some(){return br#"["safe","suggestive"]"#;}
    match json_object_member(values,b"defaultContentRating").and_then(nonempty_title_string) {
        Some(b"safe")=>br#"["safe"]"#,
        Some(b"erotica")=>br#"["safe","suggestive","erotica"]"#,
        _=>br#"["safe","suggestive"]"#,
    }
}



fn run_search(req: &[u8]) -> u32 {
    let page = koma_source_sdk::json_utils::request_page_number(req);
    let offset = match (page - 1).checked_mul(20) {
        Some(offset) => offset,
        None => return write_error("search", "invalid_request", "invalid page"),
    };
    let args = json_object_member(req, b"args").unwrap_or(req);
    let raw = match json_object_member(args, b"query") {
        None => return write_error("search", "invalid_request", "missing query"),
        Some(v) if v.len() >= 2 && v[0] == b'"' && v[v.len() - 1] == b'"' => &v[1..v.len() - 1],
        Some(_) => return write_error("search", "invalid_request", "invalid query"),
    };
    // Decode JSON request escapes before URL encoding the actual search term.
    let output = unsafe { &mut *core::ptr::addr_of_mut!(SEARCH_QUERY) };
    let query = match koma_source_sdk::plain_text::decode_json_content(raw, output) {
        Ok(v) if core::str::from_utf8(v).is_ok() => v,
        _ => return write_error("search", "invalid_request", "invalid query"),
    };

    // Build URL: /manga?title=...&limit=20&includes[]=cover_art
    let url_buf = scratch_a();
    let mut url_cursor = 0usize;
    let ok = write_bytes(url_buf, &mut url_cursor, API_BASE)
        && write_bytes(url_buf, &mut url_cursor, b"/manga?title=")
        && write_url_encoded(url_buf, &mut url_cursor, query)
        && write_bytes(url_buf, &mut url_cursor, b"&limit=20&offset=")
        && write_usize(url_buf, &mut url_cursor, offset)
        && write_bytes(url_buf, &mut url_cursor, b"&includes%5B%5D=cover_art")
        && append_content_rating_query(url_buf, &mut url_cursor, req, true)
        && write_bytes(url_buf, &mut url_cursor, original_language::query(req, b"originalLanguage_en", true));
    if !ok { return write_error("search", "internal_error", "url overflow"); }
    // The same visible filters apply to search as to the catalogue. Missing or
    // default values retain the original search URL and API relevance order.
    let filters = json_object_member(args, b"filters").unwrap_or(b"{}");
    for (key, prefix) in [(b"status".as_slice(),b"&status%5B%5D=".as_slice()),(b"demographic".as_slice(),b"&publicationDemographic%5B%5D=".as_slice())] {
        if let Some(v) = extract_json_string(filters,key) {
            let valid = if key == b"status" { matches!(v,b"ongoing"|b"completed"|b"hiatus"|b"cancelled") } else { matches!(v,b"shounen"|b"shoujo"|b"seinen"|b"josei") };
            if valid && !(write_bytes(url_buf,&mut url_cursor,prefix) && write_bytes(url_buf,&mut url_cursor,v)) {return write_error("search","internal_error","url overflow");}
        }
    }
    if let Some(v)=extract_json_string(filters,b"order") {
        if matches!(v,b"followedCount"|b"latestUploadedChapter"|b"createdAt"|b"title") && !(write_bytes(url_buf,&mut url_cursor,b"&order%5B") && write_bytes(url_buf,&mut url_cursor,v) && write_bytes(url_buf,&mut url_cursor,b"%5D=desc")) {return write_error("search","internal_error","url overflow");}
    }
    let url_bytes = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_cursor) };

    let api_json = match fetch_json(url_bytes) {
        Ok(v) => v,
        Err(e) => { let (c, m) = fetch_error_code(e); return write_error("search", c, m); }
    };

    let first_covers = first_covers(req, api_json);
    // Parse the "data" array
    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("search", "internal_error", "overflow");
    }

    let mut iter = match api_array(api_json, b"data") {
        Some(it) => it,
        None => return write_error("search", "parse_error", "no data array"),
    };

    let mut written = 0usize;
    while let Some(obj) = iter.next_object() {
        if written >= 20 { break; }
        // Extract: id, title (en or first), status
        let id = match api_string(obj, b"id") {
            Some(v) => v,
            None => continue,
        };
        // Title: look for "en":"..." inside "title":{...}
        let status = api_string(api_attributes(obj), b"status").unwrap_or(b"unknown");

        // Cover: find fileName in relationships where type=cover_art
        let cover_fn = first_covers.file(id).or_else(|| extract_cover_filename(obj));

        if written > 0 {
            if !write_bytes(payload, &mut c, b",") { return write_error("search", "internal_error", "overflow"); }
        }
        let ok = write_bytes(payload, &mut c, br#"{"id":"mdx:"#)
            && append_api_string(payload, &mut c, id)
            && write_bytes(payload, &mut c, br#"","title":""#)
            && append_manga_title(payload, &mut c, obj, req)
            && write_bytes(payload, &mut c, br#"","cover":{"kind":"url","url":""#)
            && write_bytes(payload, &mut c, COVERS_BASE)
            && append_api_string(payload, &mut c, id)
            && write_bytes(payload, &mut c, b"/");
        if !ok { return write_error("search", "internal_error", "overflow"); }
        if let Some(cfn) = cover_fn {
            if !append_cover_filename(payload, &mut c, cfn, req) { return write_error("search", "internal_error", "overflow"); }
        } else {
            if !write_bytes(payload, &mut c, b"none.jpg") { return write_error("search", "internal_error", "overflow"); }
        }
        let ok2 = write_bytes(payload, &mut c, br#""},"authors":[],"status":""#)
            && append_api_string(payload, &mut c, status)
            && write_bytes(payload, &mut c, br#"","contentRating":"unknown","sourceTags":["mangadex"]}"#);
        if !ok2 { return write_error("search", "internal_error", "overflow"); }
        written += 1;
    }

    let total = match api_number(api_json, b"total")
        .and_then(|bytes| core::str::from_utf8(bytes).ok())
        .and_then(|text| text.parse::<usize>().ok()) {
        Some(total) => total,
        None => return write_error("search", "parse_error", "invalid pagination total"),
    };
    let has_more = written > 0 && offset.saturating_add(written) < total;
    if !(write_bytes(payload, &mut c, br#"],"page":{"nextCursor":"#)
        && if has_more {
            write_bytes(payload, &mut c, b"\"")
                && write_usize(payload, &mut c, page.saturating_add(1))
                && write_bytes(payload, &mut c, b"\"")
        } else { write_bytes(payload, &mut c, b"null") }
        && write_bytes(payload, &mut c, br#","hasMore":"#)
        && write_bytes(payload, &mut c, if has_more { b"true" } else { b"false" })
        && write_bytes(payload, &mut c, b"}}")) {
        return write_error("search", "internal_error", "overflow");
    }
    write_success_payload("search", c)
}

fn relationship_string<'a>(obj: &'a [u8], kind: &[u8], key: &[u8]) -> Option<&'a [u8]> {
    let mut relations = api_array(obj, b"relationships")?;
    while let Some(relation) = relations.next_object() {
        if api_string(relation, b"type") == Some(kind) {
            return api_string(api_attributes(relation), key);
        }
    }
    None
}

fn extract_cover_filename(obj: &[u8]) -> Option<&[u8]> {
    relationship_string(obj, b"cover_art", b"fileName")
}

fn run_get_manga(req: &[u8]) -> u32 {
    let manga_id = match extract_json_string(req, b"mangaId") {
        Some(v) => v,
        None => return write_error("get_manga", "invalid_request", "missing mangaId"),
    };
    let prefix = b"mdx:";
    if manga_id.len() <= prefix.len() || &manga_id[..prefix.len()] != prefix {
        return write_error("get_manga", "invalid_request", "bad mangaId format");
    }
    let uuid = &manga_id[prefix.len()..];

    // /manga/{id}?includes[]=cover_art&includes[]=author&includes[]=artist
    let url_buf = scratch_a();
    let mut url_cursor = 0usize;
    let ok = write_bytes(url_buf, &mut url_cursor, API_BASE)
        && write_bytes(url_buf, &mut url_cursor, b"/manga/")
        && write_bytes(url_buf, &mut url_cursor, uuid)
        && write_bytes(url_buf, &mut url_cursor, b"?includes%5B%5D=cover_art&includes%5B%5D=author&includes%5B%5D=artist");
    if !ok { return write_error("get_manga", "internal_error", "url overflow"); }
    let url_bytes = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_cursor) };

    let api_json = match fetch_json(url_bytes) {
        Ok(v) => v,
        Err(e) => { let (c, m) = fetch_error_code(e); return write_error("get_manga", c, m); }
    };

    let first_covers = first_covers(req, api_json);
    let obj = match json_object_member(api_json, b"data") {
        Some(obj) if obj.first() == Some(&b'{') => obj,
        _ => return write_error("get_manga", "parse_error", "no data"),
    };
    let attributes = api_attributes(obj);
    let status = api_string(attributes, b"status").unwrap_or(b"unknown");
    let content_rating = api_string(attributes, b"contentRating").unwrap_or(b"unknown");
    let desc = json_object_member(attributes, b"description")
        .and_then(|value| api_string(value, b"en")).unwrap_or(b"");
    let cover_fn = first_covers.file(uuid).or_else(|| extract_cover_filename(obj));

    // Extract author names from relationships
    let author_name = extract_author_name(obj);
    let artist_name = extract_artist_name(obj);

    // Extract tags
    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"manga":{"id":"mdx:"#)
        && append_json_escaped(payload, &mut c, uuid)
        && write_bytes(payload, &mut c, br#"","title":""#)
        && append_manga_title(payload, &mut c, obj, req)
        && write_bytes(payload, &mut c, br#"","alternateTitles":[],"description":""#)
        && append_description_content(payload, &mut c, attributes, desc, alt_description_enabled(req), final_chapter_description::enabled(req, b"finalChapterInDesc_en"))
        && write_bytes(payload, &mut c, br#"","cover":{"kind":"url","url":""#)
        && write_bytes(payload, &mut c, COVERS_BASE)
        && append_json_escaped(payload, &mut c, uuid)
        && write_bytes(payload, &mut c, b"/");
    if !ok { return write_error("get_manga", "internal_error", "overflow"); }
    if let Some(cfn) = cover_fn {
        if !append_cover_filename(payload, &mut c, cfn, req) {
            return write_error("get_manga", "internal_error", "overflow");
        }
    } else {
        if !write_bytes(payload, &mut c, b"none.jpg") {
            return write_error("get_manga", "internal_error", "overflow");
        }
    }
    let ok2 = write_bytes(payload, &mut c, br#""},"authors":["#);
    if !ok2 { return write_error("get_manga", "internal_error", "overflow"); }
    if let Some(author) = author_name {
        let ok_string = write_bytes(payload, &mut c, b"\"")
            && append_api_string(payload, &mut c, author)
            && write_bytes(payload, &mut c, b"\"");
        if !ok_string { return write_error("get_manga", "internal_error", "overflow"); }
    }
    let ok3 = write_bytes(payload, &mut c, br#"],"artists":["#);
    if !ok3 { return write_error("get_manga", "internal_error", "overflow"); }
    if let Some(artist) = artist_name {
        let ok_string = write_bytes(payload, &mut c, b"\"")
            && append_api_string(payload, &mut c, artist)
            && write_bytes(payload, &mut c, b"\"");
        if !ok_string { return write_error("get_manga", "internal_error", "overflow"); }
    }
    let ok4 = write_bytes(payload, &mut c, br#"],"status":""#)
        && append_api_string(payload, &mut c, status)
        && write_bytes(payload, &mut c, br#"","contentRating":""#)
        && append_api_string(payload, &mut c, content_rating)
        && write_bytes(payload, &mut c, br#"","language":"en","tags":["#);
    if !ok4 { return write_error("get_manga", "internal_error", "overflow"); }
    // Extract tag names from tags array
    if let Some(mut tag_iter) = api_array(attributes, b"tags") {
        let mut tag_count = 0usize;
        while let Some(tag_obj) = tag_iter.next_object() {
            if let Some(name) = json_object_member(api_attributes(tag_obj), b"name").and_then(|name| api_string(name, b"en")) {
                if tag_count > 0 {
                    if !write_bytes(payload, &mut c, b",") { return write_error("get_manga", "internal_error", "overflow"); }
                }
                let ok_string = write_bytes(payload, &mut c, b"\"")
                    && append_api_string(payload, &mut c, name)
                    && write_bytes(payload, &mut c, b"\"");
        if !ok_string { return write_error("get_manga", "internal_error", "overflow"); }
                tag_count += 1;
            }
        }
    }
    if !write_bytes(payload, &mut c, br#"],"links":[{"kind":"source","url":"https://mangadex.org/title/"#) {
        return write_error("get_manga", "internal_error", "overflow");
    }
    if !append_json_escaped(payload, &mut c, uuid) {
        return write_error("get_manga", "internal_error", "overflow");
    }
    if !write_bytes(payload, &mut c, br#""}]}}"#) {
        return write_error("get_manga", "internal_error", "overflow");
    }

    write_success_payload("get_manga", c)
}

fn extract_author_name(obj: &[u8]) -> Option<&[u8]> {
    relationship_string(obj, b"author", b"name")
}

fn extract_artist_name(obj: &[u8]) -> Option<&[u8]> {
    relationship_string(obj, b"artist", b"name")
}

// The cursor carries only this traversal's facts, never chapter identity or URLs.
fn exclusion_host(req: &[u8]) -> bool {
    json_object_member(req,b"hostFeatures").map(|v|v.starts_with(b"[") && v.ends_with(b"]") && v[1..v.len()-1].split(|c|*c==b',').any(|s|s.trim_ascii()==br#""source_chapter_exclusions_v1""#)).unwrap_or(false)
}

struct ChapterTraversal {
    page: usize,
    known: bool,
    hosted: bool,
    external: bool,
    other: bool,
}

fn chapter_traversal(req: &[u8]) -> Option<ChapterTraversal> {
    let cursor = json_object_member(req, b"args")
        .and_then(|args| json_object_member(args, b"page"))
        .and_then(|page| json_object_member(page, b"cursor"));
    if let Some(raw) = cursor {
        if raw != b"null" {
            let text = core::str::from_utf8(nonempty_title_string(raw)?).ok()?;
            if let Some(state) = text.strip_prefix("mdxc1:") {
                let mut fields = state.split(':');
                let page_text = fields.next()?;
                if page_text.is_empty() || !page_text.bytes().all(|byte| byte.is_ascii_digit()) { return None; }
                let page = page_text.parse::<usize>().ok()?;
                let mut flag = || match fields.next()? { "0" => Some(false), "1" => Some(true), _ => None };
                let known = flag()?;
                let hosted = flag()?;
                let external = flag()?;
                let other = flag()?;
                if page == 0 || fields.next().is_some() { return None; }
                return Some(ChapterTraversal { page, known, hosted, external, other });
            }
            if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) { return None; }
            let page = text.parse::<usize>().ok()?;
            if page == 0 { return None; }
            return Some(ChapterTraversal { page, known: page == 1, hosted: false, external: false, other: false });
        }
    }
    let page = koma_source_sdk::json_utils::request_page_number(req);
    Some(ChapterTraversal { page, known: page == 1, hosted: false, external: false, other: false })
}

// The host merges exclusions across pages only after the full traversal succeeds.
// Scan the matching raw page once; the last filtered page finishes any raw tail.
// Offsets are never compared between filtered and unfiltered sets because their
// pagination shifts when chapters are excluded.
fn append_chapter_exclusions(out: &mut [u8], pos: &mut usize, req: &[u8], manga: &[u8],
    language: &[u8], start: usize, last_filtered_page: bool, hidden: &[[u8;36]]) -> Result<usize, (&'static str, &'static str)> {
    let invalid = ("parse_error", "invalid unfiltered chapter feed");
    if !write_bytes(out, pos, br#","excludedChapterIds":["#) { return Err(("internal_error", "overflow")); }
    let mut offset = start;
    let mut first = true; let mut excluded = 0usize;
    for id in hidden {
        if !first && !write_bytes(out,pos,b",") {return Err(("internal_error","overflow"));}
        if !(write_bytes(out,pos,br#""ch:"#) && write_bytes(out,pos,id) && write_bytes(out,pos,b"\"")) {return Err(("internal_error","overflow"));}
        first=false; excluded+=1;
    }
    if !blacklist::active(req,b"blockedGroups_en",b"blockedUploader_en") {
        if !write_bytes(out,pos,b"]") {return Err(("internal_error","overflow"));}
        return Ok(excluded);
    }
    loop {
        if host::check_cancel() { return Err(("cancelled", "request cancelled")); }
        let mut url = [0u8; 1024]; let mut n = 0;
        if !(write_bytes(&mut url,&mut n,API_BASE) && write_bytes(&mut url,&mut n,b"/manga/")
            && write_bytes(&mut url,&mut n,manga) && write_bytes(&mut url,&mut n,b"/feed?limit=100&offset=")
            && write_usize(&mut url,&mut n,offset) && write_bytes(&mut url,&mut n,b"&translatedLanguage%5B%5D=")
            && write_url_encoded(&mut url,&mut n,language) && write_bytes(&mut url,&mut n,b"&order%5Bchapter%5D=desc")) {
            return Err(("internal_error", "url overflow"));
        }
        if exclusion_host(req) && !write_bytes(&mut url, &mut n, b"&includeUnavailable=1") { return Err(("internal_error", "url overflow")); }
        let data = fetch_json(&url[..n]).map_err(fetch_error_code)?;
        let number = |key: &[u8]| api_number(data,key).and_then(|v|core::str::from_utf8(v).ok()).and_then(|v|v.parse::<usize>().ok());
        if api_string(data,b"result") != Some(b"ok") || number(b"offset") != Some(offset) || number(b"limit") != Some(100) { return Err(invalid); }
        let total = number(b"total").ok_or(invalid)?;
        let mut chapters = api_array(data,b"data").ok_or(invalid)?; let mut count = 0usize;
        while let Some(ch) = chapters.next_object() {
            count += 1; if count > 100 { return Err(invalid); }
            let id = api_string(ch,b"id").ok_or(invalid)?;
            if id.len()!=36 || !id.iter().enumerate().all(|(i,c)| if matches!(i,8|13|18|23) {*c==b'-'} else {c.is_ascii_hexdigit()}) { return Err(invalid); }
            let attr = api_attributes(ch);
            let chapter_lang = api_string(attr,b"translatedLanguage").ok_or(invalid)?;
            let mut relations = api_array(ch,b"relationships").ok_or(invalid)?;
            let mut owned = false; let mut matched = false;
            while let Some(rel) = relations.next_object() {
                let kind = api_string(rel,b"type").ok_or(invalid)?;
                if kind == b"manga" { owned |= api_string(rel,b"id") == Some(manga); }
                if kind == b"scanlation_group" || kind == b"user" {
                    let rid = api_string(rel,b"id").ok_or(invalid)?;
                    let key = if kind == b"scanlation_group" { b"blockedGroups_en".as_slice() } else { b"blockedUploader_en".as_slice() };
                    matched |= blacklist::contains(req,key,rid);
                }
            }
            if !owned || chapter_lang != language { return Err(invalid); }
            let hosted = api_string(attr,b"externalUrl").is_none() && api_number(attr,b"pages") != Some(b"0");
            if matched && hosted {
                if !first && !write_bytes(out,pos,b",") { return Err(("internal_error", "overflow")); }
                if !(write_bytes(out,pos,br#""ch:"#) && append_api_string(out,pos,id) && write_bytes(out,pos,b"\"")) { return Err(("internal_error", "overflow")); }
                first = false; excluded += 1;
            }
        }
        if offset.checked_add(count).ok_or(invalid)? > total || (count == 0 && offset < total) { return Err(invalid); }
        let next = offset.checked_add(100).ok_or(invalid)?;
        if !last_filtered_page || next >= total { break; }
        offset = next;
    }
    if !write_bytes(out,pos,b"]") { return Err(("internal_error", "overflow")); }
    Ok(excluded)
}

fn run_get_chapters(req: &[u8]) -> u32 {
    let manga_id = match extract_json_string(req, b"mangaId") {
        Some(v) => v,
        None => return write_error("get_chapters", "invalid_request", "missing mangaId"),
    };
    let prefix = b"mdx:";
    if manga_id.len() <= prefix.len() || &manga_id[..prefix.len()] != prefix {
        return write_error("get_chapters", "invalid_request", "bad mangaId");
    }
    let uuid = &manga_id[prefix.len()..];

    let exact_chapter = match json_object_member(req, b"args")
        .and_then(|args| json_object_member(args, b"chapterId")) {
        Some(value) => {
            let id = match nonempty_title_string(value) {
                Some(id) if id.starts_with(b"ch:") => &id[3..],
                _ => return write_error("get_chapters", "invalid_request", "bad chapterId"),
            };
            if id.len() != 36 || !id.iter().enumerate().all(|(index, byte)| {
                if matches!(index, 8 | 13 | 18 | 23) { *byte == b'-' } else { byte.is_ascii_hexdigit() }
            }) {
                return write_error("get_chapters", "invalid_request", "bad chapterId");
            }
            Some(id)
        }
        None => None,
    };
    let language = json_object_member(req, b"settings")
        .and_then(|settings| extract_json_string(settings, b"language"))
        .filter(|value| !value.is_empty())
        .unwrap_or(b"en");
    let mut traversal = match chapter_traversal(req) {
        Some(value) => value,
        None => return write_error("get_chapters", "invalid_request", "invalid chapter cursor"),
    };
    let page = traversal.page;
    let offset = match (page - 1).checked_mul(100) {
        Some(value) => value,
        None => return write_error("get_chapters", "invalid_request", "invalid page"),
    };
    let url_buf = scratch_a();
    let mut url_cursor = 0usize;
    let ok = if let Some(chapter_uuid) = exact_chapter {
        write_bytes(url_buf, &mut url_cursor, API_BASE)
            && write_bytes(url_buf, &mut url_cursor, b"/chapter/")
            && write_bytes(url_buf, &mut url_cursor, chapter_uuid)
    } else { write_bytes(url_buf, &mut url_cursor, API_BASE)
        && write_bytes(url_buf, &mut url_cursor, b"/manga/")
        && write_bytes(url_buf, &mut url_cursor, uuid)
        && write_bytes(url_buf, &mut url_cursor, b"/feed?limit=100&offset=")
        && write_usize(url_buf, &mut url_cursor, offset)
        && write_bytes(url_buf, &mut url_cursor, b"&translatedLanguage%5B%5D=")
        && write_url_encoded(url_buf, &mut url_cursor, language)
        && write_bytes(url_buf, &mut url_cursor, b"&order%5Bchapter%5D=desc") };
    // Keep default visible chapters: request unavailable metadata so explicit exclusions
    // prevent retained chapters reappearing; additional feed pages may be needed.
    // Unlike the reference, external and zero-page chapters remain excluded; no permission is granted.
    let include_unavailable = exclusion_host(req) && matches!(json_object_member(req, b"settings").and_then(|v| json_object_member(v, b"includeUnavailable_en")), Some(b"true") | Some(br#""true""#));
    if exact_chapter.is_none() && exclusion_host(req) && !write_bytes(url_buf, &mut url_cursor, b"&includeUnavailable=1&includes%5B%5D=scanlation_group") { return write_error("get_chapters", "internal_error", "url overflow"); }
    if exact_chapter.is_none() && !blacklist::append(url_buf, &mut url_cursor, req, b"blockedGroups_en", b"blockedUploader_en") { return write_error("get_chapters", "invalid_request", "invalid blacklist or URL capacity"); }
    if !ok { return write_error("get_chapters", "internal_error", "url overflow"); }
    let url_bytes = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_cursor) };

    let api_json = match fetch_json(url_bytes) {
        Ok(v) => v,
        Err(e) => { let (c, m) = fetch_error_code(e); return write_error("get_chapters", c, m); }
    };

    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("get_chapters", "internal_error", "overflow");
    }

    let pagination_number = |key: &[u8]| json_object_member(api_json, key)
        .and_then(|bytes| core::str::from_utf8(bytes).ok())
        .and_then(|text| text.parse::<usize>().ok());
    let has_more = if exact_chapter.is_some() { false } else { match (pagination_number(b"offset"), pagination_number(b"limit"), pagination_number(b"total")) {
        (Some(response_offset), Some(100), Some(total)) if response_offset == offset =>
            response_offset.saturating_add(100) < total,
        _ => return write_error("get_chapters", "parse_error", "invalid feed pagination"),
    }};

    let mut exact_item = if let Some(chapter_uuid) = exact_chapter {
        let item = match json_object_member(api_json, b"data") {
            Some(item) => item,
            None => return write_error("get_chapters", "parse_error", "no chapter data"),
        };
        let same_id = json_object_member(item, b"id").and_then(nonempty_title_string) == Some(chapter_uuid);
        let mut same_manga = false;
        if let Some(mut relations) = api_array(item, b"relationships") {
            while let Some(relation) = relations.next_object() {
                if json_object_member(relation, b"type").and_then(nonempty_title_string) == Some(b"manga") &&
                    json_object_member(relation, b"id").and_then(nonempty_title_string) == Some(uuid) {
                    same_manga = true;
                }
            }
        }
        if !same_id || !same_manga {
            return write_error("get_chapters", "not_found", "chapter does not match manga");
        }
        Some(item)
    } else { None };
    let mut iter = if exact_chapter.is_none() {
        match api_array(api_json, b"data") {
            Some(it) => Some(it),
            None => return write_error("get_chapters", "parse_error", "no data"),
        }
    } else { None };

    let mut written = 0usize;
    let mut hidden = [[0u8;36];100]; let mut hidden_count=0usize;
    while let Some(obj) = exact_item.take().or_else(|| iter.as_mut().and_then(|it| it.next_object())) {
        if written >= 100 { break; }
        let ch_id = match api_string(obj, b"id") {
            Some(v) => v,
            None => { traversal.other = true; continue; },
        };
        // An unnumbered extra is unknown, not chapter zero. Keep real "0" intact.
        let chapter_num = api_string(api_attributes(obj), b"chapter")
            .filter(|value| !value.iter().all(u8::is_ascii_whitespace));
        let title = api_string(api_attributes(obj), b"title").unwrap_or(b"");
        let lang = api_string(api_attributes(obj), b"translatedLanguage").unwrap_or(b"en");

        let pages = api_number(api_attributes(obj), b"pages");

        // Skip chapters that MangaDex doesn't host (externalUrl, e.g. MangaPlus)
        let has_external = api_string(api_attributes(obj), b"externalUrl").is_some();
        let pages_zero = match pages {
            Some(p) => p == b"0",
            None => false,
        };
        let unavailable = json_object_member(api_attributes(obj), b"isUnavailable") == Some(b"true");
        // Do not present an unavailable row as hosted without a known positive page count.
        if has_external || pages_zero || (exclusion_host(req) && unavailable && pages.is_none()) {
            if has_external && api_string(api_attributes(obj), b"externalUrl").map_or(false, |value| !value.is_empty()) {
                traversal.external = true;
            } else { traversal.other = true; }
            continue;
        }

        if exact_chapter.is_none() && exclusion_host(req) && unavailable && !include_unavailable {
            if ch_id.len()!=36 || !ch_id.iter().enumerate().all(|(i,c)| if matches!(i,8|13|18|23) {*c==b'-'} else {c.is_ascii_hexdigit()}) {return write_error("get_chapters","parse_error","invalid chapter ID");}
            let mut owned=false;
            if let Some(mut rels)=api_array(obj,b"relationships") {while let Some(rel)=rels.next_object() {if api_string(rel,b"type")==Some(b"manga") && api_string(rel,b"id")==Some(uuid) {owned=true;}}}
            if !owned || lang != language || pages.is_none() {return write_error("get_chapters","parse_error","invalid chapter owner or pages");}
            if hidden_count>=100 {return write_error("get_chapters","parse_error","too many chapters");}
            hidden[hidden_count].copy_from_slice(ch_id); hidden_count+=1; traversal.other=true; continue;
        }
        if written > 0 {
            if !write_bytes(payload, &mut c, b",") { return write_error("get_chapters", "internal_error", "overflow"); }
        }
        let ok = write_bytes(payload, &mut c, br#"{"id":"ch:"#)
            && append_api_string(payload, &mut c, ch_id)
            && write_bytes(payload, &mut c, br#"","mangaId":"mdx:"#)
            && append_json_escaped(payload, &mut c, uuid)
            && write_bytes(payload, &mut c, br#"","title":""#)
            && append_api_string(payload, &mut c, title)
            && write_bytes(payload, &mut c, br#"","chapterNumber":"#)
            && if let Some(number) = chapter_num {
                write_bytes(payload, &mut c, b"\"")
                    && append_api_string(payload, &mut c, number)
                    && write_bytes(payload, &mut c, b"\"")
            } else {
                write_bytes(payload, &mut c, b"null")
            }
            && write_bytes(payload, &mut c, br#","volumeNumber":null,"language":""#)
            && append_api_string(payload, &mut c, lang)            && write_bytes(payload, &mut c, br#"","publishedAt":null,"updatedAt":null,"pageCount":"#);
        if !ok { return write_error("get_chapters", "internal_error", "overflow"); }
        if let Some(p) = pages {
            if !write_bytes(payload, &mut c, p) { return write_error("get_chapters", "internal_error", "overflow"); }
        } else {
            if !write_bytes(payload, &mut c, b"null") { return write_error("get_chapters", "internal_error", "overflow"); }
        }
        if include_unavailable && unavailable {
            let mut group = None;
            if let Some(mut relations) = api_array(obj, b"relationships") {
                while let Some(rel) = relations.next_object() {
                    if api_string(rel, b"type") == Some(b"scanlation_group") {
                        group = api_string(api_attributes(rel), b"name");
                        if group.is_some() { break; }
                    }
                }
            }
            if !write_bytes(payload, &mut c, br#","scanlator":"UNAVAILABLE"#) { return write_error("get_chapters", "internal_error", "overflow"); }
            if let Some(name) = group {
                if !write_bytes(payload, &mut c, b" - ") || !append_api_string(payload, &mut c, name) { return write_error("get_chapters", "internal_error", "overflow"); }
            }
            if !write_bytes(payload, &mut c, b"\"") { return write_error("get_chapters", "internal_error", "overflow"); }
        }
        if !write_bytes(payload, &mut c, b"}") { return write_error("get_chapters", "internal_error", "overflow"); }
        written += 1;
        traversal.hosted = true;
    }

    if !write_bytes(payload,&mut c,b"]") { return write_error("get_chapters", "internal_error", "overflow"); }
    if exact_chapter.is_none() && (blacklist::active(req,b"blockedGroups_en",b"blockedUploader_en") || hidden_count>0) {
        match append_chapter_exclusions(payload,&mut c,req,uuid,language,offset,!has_more,&hidden[..hidden_count]) {
            // Remember a real excluded hosted chapter across the existing cursor so
            // an external-only filtered remainder cannot hide that valid exclusion.
            Ok(count) if count > 0 => traversal.other = true,
            Ok(_) => {},
            Err((code,message)) => return write_error("get_chapters",code,message),
        }
    }
    if !has_more && traversal.known && traversal.external && !traversal.hosted && !traversal.other {
        return write_error("get_chapters", "unsupported", "external chapters are not hosted by this source");
    }
    let next_page = match page.checked_add(1) {
        Some(value) => value,
        None if has_more => return write_error("get_chapters", "invalid_request", "invalid page"),
        None => page,
    };
    if !(write_bytes(payload, &mut c, br#","page":{"nextCursor":"#)
        && if has_more {
            write_bytes(payload, &mut c, b"\"mdxc1:")
                && write_usize(payload, &mut c, next_page)
                && write_bytes(payload, &mut c, if traversal.known { b":1" } else { b":0" })
                && write_bytes(payload, &mut c, if traversal.hosted { b":1" } else { b":0" })
                && write_bytes(payload, &mut c, if traversal.external { b":1" } else { b":0" })
                && write_bytes(payload, &mut c, if traversal.other { b":1" } else { b":0" })
                && write_bytes(payload, &mut c, b"\"")
        } else { write_bytes(payload, &mut c, b"null") }
        && write_bytes(payload, &mut c, br#","hasMore":"#)
        && write_bytes(payload, &mut c, if has_more { b"true" } else { b"false" })
        && write_bytes(payload, &mut c, b"}")) {
        return write_error("get_chapters", "internal_error", "overflow");
    }
    if !write_bytes(payload,&mut c,b"}") { return write_error("get_chapters", "internal_error", "overflow"); }
    write_success_payload("get_chapters", c)
}

fn data_saver_enabled(req: &[u8]) -> bool {
    let value = json_object_member(req, b"settings")
        .and_then(|settings| json_object_member(settings, b"dataSaver"));
    matches!(value, Some(b"true") | Some(br#""true""#))
}

// Fixed MangaDex preference selects an at-home server via its API parameter.
// False/absent keeps the original request; never rewrite a returned image URL.
fn standard_https_enabled(req: &[u8]) -> bool {
    matches!(json_object_member(req, b"settings").and_then(|v| json_object_member(v, b"usePort443_en")), Some(b"true") | Some(br#""true""#))
}
fn run_get_pages(req: &[u8]) -> u32 {
    let data_saver = data_saver_enabled(req);
    let chapter_id = match extract_json_string(req, b"chapterId") {
        Some(v) => v,
        None => return write_error("get_pages", "invalid_request", "missing chapterId"),
    };
    let prefix = b"ch:";
    if chapter_id.len() <= prefix.len() || &chapter_id[..prefix.len()] != prefix {
        return write_error("get_pages", "invalid_request", "bad chapterId");
    }
    let ch_uuid = &chapter_id[prefix.len()..];

    // /at-home/server/{chapterId}
    let url_buf = scratch_a();
    let mut url_cursor = 0usize;
    let ok = write_bytes(url_buf, &mut url_cursor, API_BASE)
        && write_bytes(url_buf, &mut url_cursor, b"/at-home/server/")
        && write_bytes(url_buf, &mut url_cursor, ch_uuid)
        && (!standard_https_enabled(req) || write_bytes(url_buf, &mut url_cursor, b"?forcePort443=true"));
    if !ok { return write_error("get_pages", "internal_error", "url overflow"); }
    let url_bytes = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_cursor) };

    let api_json = match fetch_json(url_bytes) {
        Ok(v) => v,
        Err(e) => { let (c, m) = fetch_error_code(e); return write_error("get_pages", c, m); }
    };

    // Extract baseUrl and hash
    let base_url = match api_string(api_json, b"baseUrl") {
        Some(v) => v,
        None => return write_error("get_pages", "parse_error", "no baseUrl"),
    };
    let hash = match json_object_member(api_json, b"chapter").and_then(|obj| api_string(obj, b"hash")) {
        Some(v) => v,
        None => return write_error("get_pages", "parse_error", "no hash"),
    };

    // Parse "data":["file1.png","file2.png",...]
    let payload = payload_buf();
    let mut c = 0usize;
    let ok2 = write_bytes(payload, &mut c, br#"{"chapterId":"ch:"#)
        && append_json_escaped(payload, &mut c, ch_uuid)
        && write_bytes(payload, &mut c, br#"","pages":["#);
    if !ok2 { return write_error("get_pages", "internal_error", "overflow"); }

    // Select the filenames supplied for this quality; filenames are not aliases.
    let chapter = match json_object_member(api_json, b"chapter") {
        Some(value) => value,
        None => return write_error("get_pages", "parse_error", "no chapter object"),
    };
    let array_key: &[u8] = if data_saver { b"dataSaver" } else { b"data" };
    let filenames = match json_object_member(chapter, array_key) {
        Some(array) if array.starts_with(b"[") && array.ends_with(b"]") => array,
        _ => return write_error("get_pages", "parse_error", "no selected image array"),
    };
    let mut i = 1usize;
    let mut page_idx = 0usize;
    while i < filenames.len() {
        while i < filenames.len() && (filenames[i].is_ascii_whitespace() || filenames[i] == b',') {
            i += 1;
        }
        if i >= filenames.len() || filenames[i] == b']' { break; }
        if filenames[i] != b'"' { return write_error("get_pages", "parse_error", "invalid image filename"); }
        i += 1;
        let start = i;
        while i < filenames.len() && filenames[i] != b'"' {
            if filenames[i] == b'\\' { i += 1; }
            i += 1;
        }
        if i >= filenames.len() { return write_error("get_pages", "parse_error", "unterminated image filename"); }
        let filename = &filenames[start..i];
        i += 1;
        if page_idx > 0 {
            if !write_bytes(payload, &mut c, b",") { return write_error("get_pages", "internal_error", "overflow"); }
        }
        // Build full URL: {baseUrl}/data/{hash}/{filename}
        let ok3 = write_bytes(payload, &mut c, br#"{"id":"page:"#)
            && write_usize(payload, &mut c, page_idx)
            && write_bytes(payload, &mut c, br#"","index":"#)
            && write_usize(payload, &mut c, page_idx)
            && write_bytes(payload, &mut c, br#","image":{"kind":"url","url":""#)
            && append_api_string(payload, &mut c, base_url)
            && write_bytes(payload, &mut c, if data_saver { b"/data-saver/" } else { b"/data/" })
            && append_api_string(payload, &mut c, hash)
            && write_bytes(payload, &mut c, b"/")
            && append_api_string(payload, &mut c, filename)
            && write_bytes(payload, &mut c, br#""}}"#);
        if !ok3 { return write_error("get_pages", "internal_error", "overflow"); }
        page_idx += 1;
    }

    if !write_bytes(payload, &mut c, b"]}") {
        return write_error("get_pages", "internal_error", "overflow");
    }
    write_success_payload("get_pages", c)
}

fn run_get_listings(_req: &[u8]) -> u32 {
    // A listing is a browse entry point, not a manga result. Keep these stable
    // identifiers so every host can render a semantic browse menu without a
    // network request, then let get_manga_list resolve the corresponding order.
    const LISTINGS_JSON: &[u8] = br#"{"listings":[{"id":"listing:popular","name":"Popular","kind":"popular"},{"id":"listing:latest","name":"Latest Updates","kind":"latest"},{"id":"listing:top_rated","name":"Top Rated","kind":"top_rated"}]}"#;
    let payload = payload_buf();
    if LISTINGS_JSON.len() > payload.len() {
        return write_error("get_listings", "internal_error", "overflow");
    }
    payload[..LISTINGS_JSON.len()].copy_from_slice(LISTINGS_JSON);
    write_success_payload("get_listings", LISTINGS_JSON.len())
}

// --- get_filters: return browseable filter definitions ---

fn run_get_filters(_req: &[u8]) -> u32 {
    // MangaDex filters: status, contentRating, publicationDemographic, order
    // We keep a static definition; tags would require an API call to /manga/tag
    // but the most useful browsing filters are these fixed enums.
    const FILTERS_JSON: &[u8] = br#"{"filters":[{"id":"status","name":"Status","kind":"select","options":[{"value":"all","label":"All"},{"value":"ongoing","label":"Ongoing"},{"value":"completed","label":"Completed"},{"value":"hiatus","label":"Hiatus"},{"value":"cancelled","label":"Cancelled"}],"default":"all"},{"id":"contentRating","name":"Content Rating","kind":"select","options":[{"value":"all","label":"All"},{"value":"safe","label":"Safe"},{"value":"suggestive","label":"Suggestive"},{"value":"erotica","label":"Erotica"}],"default":"all"},{"id":"demographic","name":"Demographic","kind":"select","options":[{"value":"all","label":"All"},{"value":"shounen","label":"Shounen"},{"value":"shoujo","label":"Shoujo"},{"value":"seinen","label":"Seinen"},{"value":"josei","label":"Josei"}],"default":"all"},{"id":"order","name":"Order By","kind":"select","options":[{"value":"relevance","label":"Relevance"},{"value":"followedCount","label":"Popularity"},{"value":"latestUploadedChapter","label":"Latest Upload"},{"value":"createdAt","label":"Newest"},{"value":"title","label":"Title"}],"default":"relevance"}]}"#;

    let payload = payload_buf();
    let flen = FILTERS_JSON.len();
    if flen > payload.len() {
        return write_error("get_filters", "internal_error", "payload overflow");
    }
    payload[..flen].copy_from_slice(FILTERS_JSON);
    write_success_payload("get_filters", flen)
}

// The chapter endpoint is the only MangaDex latest endpoint that accepts
// group/uploader exclusions. Retain chapter-order IDs across the second fetch.
struct FilteredLatest { data: &'static [u8], ids: [[u8;36];100], count: usize, more: bool }
fn filtered_latest(req: &[u8], page: usize, use_filter: bool) -> Result<FilteredLatest, (&'static str,&'static str)> {
    let err=("parse_error","invalid filtered latest response");
    let offset=page.checked_sub(1).and_then(|n|n.checked_mul(100)).ok_or(err)?;
    let b=scratch_a(); let mut n=0;
    let lang=json_object_member(req,b"settings").and_then(|s|extract_json_string(s,b"language")).filter(|v|!v.is_empty()).unwrap_or(b"en");
    if !(write_bytes(b,&mut n,API_BASE) && write_bytes(b,&mut n,b"/chapter?limit=100&offset=") && write_usize(b,&mut n,offset)
        && write_bytes(b,&mut n,b"&translatedLanguage%5B%5D=") && write_url_encoded(b,&mut n,lang)
        && write_bytes(b,&mut n,b"&order%5BpublishAt%5D=desc&includeFutureUpdates=0&includeFuturePublishAt=0&includeEmptyPages=0")
        && append_content_rating_query(b,&mut n,req,use_filter)
        && write_bytes(b,&mut n,original_language::query(req,b"originalLanguage_en",false))
        && blacklist::append(b,&mut n,req,b"blockedGroups_en",b"blockedUploader_en")) { return Err(("invalid_request","invalid blacklist or URL capacity")); }
    let v=fetch_json(unsafe{core::slice::from_raw_parts(SCRATCH_A.as_ptr(),n)}).map_err(fetch_error_code)?;
    let total=api_number(v,b"total").and_then(|x|core::str::from_utf8(x).ok()).and_then(|x|x.parse::<usize>().ok()).ok_or(err)?;
    let got_offset=api_number(v,b"offset").and_then(|x|core::str::from_utf8(x).ok()).and_then(|x|x.parse::<usize>().ok()).ok_or(err)?;
    if got_offset!=offset {return Err(err)}
    let mut ids=[[0u8;36];100]; let mut count=0; let mut chapters=0;
    let mut iter=api_array(v,b"data").ok_or(err)?;
    while let Some(ch)=iter.next_object() {
        chapters+=1; if chapters>100 {return Err(err)}
        let mut rels=api_array(ch,b"relationships").ok_or(err)?;
        let mut found=None;
        while let Some(rel)=rels.next_object() { if api_string(rel,b"type")==Some(b"manga") { if found.is_some(){return Err(err)} found=api_string(rel,b"id"); } }
        let id=found.ok_or(err)?;
        if id.len()!=36 || !id.iter().enumerate().all(|(i,c)|if matches!(i,8|13|18|23){*c==b'-'}else{c.is_ascii_hexdigit()}) {return Err(err)}
        if !ids[..count].iter().any(|v|v.as_slice()==id) { ids[count].copy_from_slice(id); count+=1; }
    }
    if offset>total || offset.saturating_add(chapters)>total || (chapters==0 && offset<total) {return Err(err)}
    let more=offset.saturating_add(chapters)<total;
    if count==0 { return Ok(FilteredLatest{data:br#"{"data":[]}"#,ids,count,more}); }
    let b=scratch_a();let mut n=0;
    if !(write_bytes(b,&mut n,API_BASE) && write_bytes(b,&mut n,b"/manga?includes%5B%5D=cover_art&limit=") && write_usize(b,&mut n,count) && append_content_rating_query(b,&mut n,req,use_filter)) {return Err(err)}
    // Preserve the list filters that were already accepted by this source.
    let status=extract_json_string(req,b"status").unwrap_or(b"all");
    let demographic=extract_json_string(req,b"demographic").unwrap_or(b"all");
    if status!=b"all" && !(write_bytes(b,&mut n,b"&status%5B%5D=") && write_url_encoded(b,&mut n,status)) {return Err(err)}
    if demographic!=b"all" && !(write_bytes(b,&mut n,b"&publicationDemographic%5B%5D=") && write_url_encoded(b,&mut n,demographic)) {return Err(err)}
    let order=extract_json_string(req,b"order").unwrap_or(b"relevance");
    if order!=b"relevance" && !(write_bytes(b,&mut n,b"&order%5B") && write_url_encoded(b,&mut n,order) && write_bytes(b,&mut n,b"%5D=desc")) {return Err(err)}
    for id in &ids[..count] {if !(write_bytes(b,&mut n,b"&ids%5B%5D=") && write_bytes(b,&mut n,id)) {return Err(err)}}
    let data=fetch_json(unsafe{core::slice::from_raw_parts(SCRATCH_A.as_ptr(),n)}).map_err(fetch_error_code)?;
    let mut response_ids=[[0u8;36];100]; let mut actual=0;
    let mut returned=api_array(data,b"data").ok_or(err)?;
    while let Some(obj)=returned.next_object() {
        let id=api_string(obj,b"id").ok_or(err)?;
        if actual>=count || id.len()!=36 || !ids[..count].iter().any(|v| v.as_slice()==id) || response_ids[..actual].iter().any(|v| v.as_slice()==id) { return Err(err); }
        response_ids[actual].copy_from_slice(id); actual+=1;
    }
    // Explicit ordering remains a manga query and its returned order controls
    // display; otherwise the chapter endpoint determines the latest order.
    if order!=b"relevance" {ids=response_ids; count=actual;}
    Ok(FilteredLatest{data,ids,count,more})
}
fn latest_object<'a>(v: &'a [u8], id: &[u8]) -> Option<&'a [u8]> {
    let mut it=api_array(v,b"data")?; while let Some(obj)=it.next_object(){if api_string(obj,b"id")==Some(id){return Some(obj)}} None
}
fn emit_latest_items(out:&mut [u8], n:&mut usize, req:&[u8], latest:&FilteredLatest, home:bool) -> bool {
    // Cover metadata belongs only to rows this operation will actually emit.
    let mut selected=[[0u8;36];100]; let mut count=0;
    for id in &latest.ids[..latest.count] {
        if latest_object(latest.data,id).is_none() {continue}
        if home && count==10 {break}
        selected[count].copy_from_slice(id); count+=1;
    }
    let covers=first_volume_cover::load_selected(req,b"tryUsingFirstVolumeCover_en",latest.data,Some(&selected[..count]),|url| {
        fetch_json_into(url, unsafe { &mut *core::ptr::addr_of_mut!(COVER_JSON) }).ok()
    }); let mut written=0;
    for id in &latest.ids[..latest.count] {
        let Some(obj)=latest_object(latest.data,id) else {continue};
        if home && written==10 {break}
        if written>0 && !write_bytes(out,n,b",") {return false}
        if !(write_bytes(out,n,br#"{"id":"mdx:"#) && append_api_string(out,n,id)
            && write_bytes(out,n,br#"","title":""#) && append_manga_title(out,n,obj,req)
            && write_bytes(out,n,br#"","cover":{"kind":"url","url":""#)
            && write_bytes(out,n,COVERS_BASE) && append_api_string(out,n,id) && write_bytes(out,n,b"/")) {return false}
        if let Some(file)=covers.file(id).or_else(||extract_cover_filename(obj)) {if !append_cover_filename(out,n,file,req){return false}}
        else if !write_bytes(out,n,b"none.jpg") {return false}
        if home { if !write_bytes(out,n,br#""}}"#){return false} }
        else {let st=api_string(api_attributes(obj),b"status").unwrap_or(b"unknown");
            if !(write_bytes(out,n,br#""},"authors":[],"status":""#) && append_api_string(out,n,st)
                && write_bytes(out,n,br#"","contentRating":"unknown","sourceTags":["mangadex"]}"#)) {return false}}
        written+=1;
    } true
}
fn run_filtered_latest(req:&[u8], page:usize)->u32 {
    let latest=match filtered_latest(req,page,true){Ok(v)=>v,Err((c,m))=>return write_error("get_manga_list",c,m)};
    let b=payload_buf();let mut n=0;
    if !(write_bytes(b,&mut n,br#"{"items":["#) && emit_latest_items(b,&mut n,req,&latest,false)
        && write_bytes(b,&mut n,br#"],"page":{"nextCursor":"#)
        && if latest.more {write_bytes(b,&mut n,b"\"") && write_usize(b,&mut n,page+1) && write_bytes(b,&mut n,b"\"")} else {write_bytes(b,&mut n,b"null")}
        && write_bytes(b,&mut n,if latest.more {br#", "hasMore":true}}"#} else {br#", "hasMore":false}}"#})) {return write_error("get_manga_list","internal_error","overflow")}
    write_success_payload("get_manga_list",n)
}

// Explicit list ordering must remain global. Scan original manga pages in that
// order and test chapter availability; do not sort a recent-chapter subset.
static mut ORDERED_MANGA_JSON: [u8; JSON_BUF_CAP] = [0; JSON_BUF_CAP];
fn batch_available(req:&[u8], ids:&[[u8;36]], good:&mut [bool]) -> Result<(),(&'static str,&'static str)> {
    let err=("parse_error","invalid filtered chapter response");
    let batch= json_object_member(req,b"hostFeatures").map(|v| v.starts_with(b"[") && v.ends_with(b"]") && v[1..v.len()-1].split(|c|*c==b',').any(|s|s.trim_ascii()==br#""source_http_batch_v1""#)).unwrap_or(false);
    let lang=json_object_member(req,b"settings").and_then(|s|extract_json_string(s,b"language")).filter(|v|!v.is_empty()).unwrap_or(b"en");
    for start in (0..ids.len()).step_by(if batch {5}else{1}) {
        if host::check_cancel(){return Err(("cancelled","request cancelled"))}
        let end=core::cmp::min(start+if batch {5}else{1},ids.len());
        let mut requests=[0u8;196608]; let mut n=0;
        if batch && !write_bytes(&mut requests,&mut n,br#"{"version":1,"requests":["#){return Err(err)}
        for (index,id) in ids[start..end].iter().enumerate() {
            if batch && index>0 && !write_bytes(&mut requests,&mut n,b","){return Err(err)}
            let mut url=[0u8;32768]; let mut l=0;
            if !(write_bytes(&mut url,&mut l,API_BASE) && write_bytes(&mut url,&mut l,b"/chapter?limit=1&manga=") && write_bytes(&mut url,&mut l,id)
                && write_bytes(&mut url,&mut l,b"&translatedLanguage%5B%5D=") && write_url_encoded(&mut url,&mut l,lang)
                && write_bytes(&mut url,&mut l,b"&includeFutureUpdates=0&includeFuturePublishAt=0&includeEmptyPages=0")
                && append_content_rating_query(&mut url,&mut l,req,true)
                && write_bytes(&mut url,&mut l,original_language::query(req,b"originalLanguage_en",false))
                && blacklist::append(&mut url,&mut l,req,b"blockedGroups_en",b"blockedUploader_en")) {return Err(("invalid_request","invalid blacklist or URL capacity"))}
            let len=build_get_request(&mut requests[n..],&url[..l],None,&[(b"User-Agent",MANGADEX_USER_AGENT)]).ok_or(err)?; n+=len;
        }
        if batch && !write_bytes(&mut requests,&mut n,b"]}"){return Err(err)}
        let len=http_request(&requests[..n],http_out()).map_err(|_|("network_error","chapter probe failed"))?;
        let response=unsafe {core::slice::from_raw_parts(HTTP_OUT.as_ptr(),len)};
        let mut parts=if batch {Some(api_array(response,b"responses").ok_or(err)?)}else{None};
        for i in start..end {
            let item=if let Some(ref mut a)=parts {a.next_object().ok_or(err)?}else{response};
            let dest=body_buf();
            let size=koma_source_sdk::decode_json_body_into(item,dest).map_err(fetch_error_code)?;
            let v=&dest[..size];
            let total=api_number(v,b"total").and_then(|v|core::str::from_utf8(v).ok()).and_then(|v|v.parse::<usize>().ok()).ok_or(err)?;
            if api_string(v,b"result")!=Some(b"ok"){return Err(err)}
            let mut chapters=api_array(v,b"data").ok_or(err)?;
            let chapter=chapters.next_object();
            if chapters.next_object().is_some() || (total>0)!=chapter.is_some(){return Err(err)}
            if let Some(chapter)=chapter {
                let mut relations=api_array(chapter,b"relationships").ok_or(err)?; let mut owned=false;
                while let Some(rel)=relations.next_object(){if api_string(rel,b"type")==Some(b"manga") && api_string(rel,b"id")==Some(ids[i].as_slice()){owned=true}}
                if !owned{return Err(err)}
            }
            good[i]=total>0;
        }
        if parts.as_mut().and_then(|a|a.next_object()).is_some(){return Err(err)}
    }
    Ok(())
}
fn run_ordered_latest(req:&[u8], mut page:usize, order:&[u8])->u32 {
    let err=("parse_error","invalid ordered manga response");
    loop {
        if host::check_cancel(){return write_error("get_manga_list","cancelled","request cancelled")}
        let Some(offset)=page.checked_sub(1).and_then(|n|n.checked_mul(20)) else{return write_error("get_manga_list",err.0,err.1)};
        let b=scratch_a();let mut n=0;
        let status=extract_json_string(req,b"status").unwrap_or(b"all");let demographic=extract_json_string(req,b"demographic").unwrap_or(b"all");
        let mut ok=write_bytes(b,&mut n,API_BASE)&&write_bytes(b,&mut n,b"/manga?limit=20&includes%5B%5D=cover_art&offset=")&&write_usize(b,&mut n,offset);
        if status!=b"all" {ok=ok&&write_bytes(b,&mut n,b"&status%5B%5D=")&&write_url_encoded(b,&mut n,status)}
        ok=ok&&append_content_rating_query(b,&mut n,req,true)&&write_bytes(b,&mut n,original_language::query(req,b"originalLanguage_en",false));
        if demographic!=b"all" {ok=ok&&write_bytes(b,&mut n,b"&publicationDemographic%5B%5D=")&&write_url_encoded(b,&mut n,demographic)}
        ok=ok&&write_bytes(b,&mut n,b"&order%5B")&&write_url_encoded(b,&mut n,order)&&write_bytes(b,&mut n,b"%5D=desc");
        if !ok{return write_error("get_manga_list","internal_error","url overflow")}
        let data=match fetch_json(unsafe{core::slice::from_raw_parts(SCRATCH_A.as_ptr(),n)}){Ok(v)=>v,Err(e)=>{let(c,m)=fetch_error_code(e);return write_error("get_manga_list",c,m)}};
        let total=match api_number(data,b"total").and_then(|v|core::str::from_utf8(v).ok()).and_then(|v|v.parse::<usize>().ok()){Some(v)=>v,None=>return write_error("get_manga_list",err.0,err.1)};
        let dest=unsafe{&mut *core::ptr::addr_of_mut!(ORDERED_MANGA_JSON)};if data.len()>dest.len(){return write_error("get_manga_list",err.0,err.1)}dest[..data.len()].copy_from_slice(data);let saved=unsafe{core::slice::from_raw_parts(dest.as_ptr(),data.len())};
        let mut ids=[[0u8;36];20];let mut count=0;let mut it=match api_array(saved,b"data"){Some(v)=>v,None=>return write_error("get_manga_list",err.0,err.1)};
        while let Some(obj)=it.next_object(){let id=match api_string(obj,b"id"){Some(v) if v.len()==36=>v,_=>return write_error("get_manga_list",err.0,err.1)};if count==20||ids[..count].iter().any(|v|v.as_slice()==id){return write_error("get_manga_list",err.0,err.1)}ids[count].copy_from_slice(id);count+=1;}
        if offset.saturating_add(count)>total || (count==0 && offset<total){return write_error("get_manga_list",err.0,err.1)}
        let more=offset.saturating_add(count)<total;
        if more && count!=20 {return write_error("get_manga_list",err.0,err.1)}
        let mut good=[false;20];
        if let Err((c,m))=batch_available(req,&ids[..count],&mut good[..count]){return write_error("get_manga_list",c,m)}
        let mut selected=[[0u8;36];100];let mut used=0;for i in 0..count {if good[i]{selected[used]=ids[i];used+=1}}
        if used==0 && more {page=match page.checked_add(1){Some(v)=>v,None=>return write_error("get_manga_list",err.0,err.1)};continue}
        let latest=FilteredLatest{data:saved,ids:selected,count:used,more};let b=payload_buf();let mut n=0;
        if !(write_bytes(b,&mut n,br#"{"items":["#)&&emit_latest_items(b,&mut n,req,&latest,false)&&write_bytes(b,&mut n,br#"],"page":{"nextCursor":"#)
        && if more {write_bytes(b,&mut n,b"\"")&&write_usize(b,&mut n,page+1)&&write_bytes(b,&mut n,b"\"")}else{write_bytes(b,&mut n,b"null")}
        && write_bytes(b,&mut n,if more {br#", "hasMore":true}}"#}else{br#", "hasMore":false}}"#})){return write_error("get_manga_list","internal_error","overflow")}
        return write_success_payload("get_manga_list",n)
    }
}

// --- get_manga_list: browse with filters ---

fn run_get_manga_list(req: &[u8]) -> u32 {
    // Extract filter parameters from request
    let status = extract_json_string(req, b"status").unwrap_or(b"all");
    let demographic = extract_json_string(req, b"demographic").unwrap_or(b"all");
    let listing_id = extract_json_string(req, b"listingId").unwrap_or(b"listing:popular");
    let requested_order = extract_json_string(req, b"order").unwrap_or(b"relevance");
    if listing_id == b"listing:latest" && blacklist::active(req,b"blockedGroups_en",b"blockedUploader_en") { let page=koma_source_sdk::json_utils::request_page_number(req); return if requested_order == b"relevance" {run_filtered_latest(req,page)} else {run_ordered_latest(req,page,requested_order)}; }
    let order: &[u8] = if requested_order != b"relevance" {
        requested_order
    } else if listing_id == b"listing:latest" {
        b"latestUploadedChapter"
    } else if listing_id == b"listing:top_rated" {
        b"rating"
    } else {
        b"followedCount"
    };
    let page_num = koma_source_sdk::json_utils::request_page_number(req);
    let offset_val = match (page_num - 1).checked_mul(20) {
        Some(offset) => offset,
        None => return write_error("get_manga_list", "invalid_request", "invalid page"),
    };

    // Build URL: /manga?limit=20&offset=N&includes[]=cover_art + optional filters
    let url_buf = scratch_a();
    let mut url_cursor = 0usize;
    let ok = write_bytes(url_buf, &mut url_cursor, API_BASE)
        && write_bytes(url_buf, &mut url_cursor, b"/manga?limit=20&includes%5B%5D=cover_art&offset=")
        && write_usize(url_buf, &mut url_cursor, offset_val);
    if !ok { return write_error("get_manga_list", "internal_error", "url overflow"); }

    // Append status filter
    if status != b"all" {
        let ok2 = write_bytes(url_buf, &mut url_cursor, b"&status%5B%5D=")
            && write_bytes(url_buf, &mut url_cursor, status);
        if !ok2 { return write_error("get_manga_list", "internal_error", "url overflow"); }
    }
    if !(append_content_rating_query(url_buf, &mut url_cursor, req, true)
        && write_bytes(url_buf, &mut url_cursor, original_language::query(req, b"originalLanguage_en", false))) {
        return write_error("get_manga_list", "internal_error", "url overflow");
    }
    // Append demographic filter
    if demographic != b"all" {
        let ok2 = write_bytes(url_buf, &mut url_cursor, b"&publicationDemographic%5B%5D=")
            && write_bytes(url_buf, &mut url_cursor, demographic);
        if !ok2 { return write_error("get_manga_list", "internal_error", "url overflow"); }
    }
    // The selected listing supplies a default order; an explicit filter wins.
    let ok2 = write_bytes(url_buf, &mut url_cursor, b"&order%5B")
        && write_bytes(url_buf, &mut url_cursor, order)
        && write_bytes(url_buf, &mut url_cursor, b"%5D=desc");
    if !ok2 { return write_error("get_manga_list", "internal_error", "url overflow"); }

    let url_bytes = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_cursor) };

    let api_json = match fetch_json(url_bytes) {
        Ok(v) => v,
        Err(e) => { let (c, m) = fetch_error_code(e); return write_error("get_manga_list", c, m); }
    };

    let first_covers = first_covers(req, api_json);
    // Parse total from response for hasMore
    let total = match api_number(api_json, b"total")
        .and_then(|bytes| core::str::from_utf8(bytes).ok())
        .and_then(|text| text.parse::<usize>().ok()) {
        Some(total) => total,
        None => return write_error("get_manga_list", "parse_error", "invalid pagination total"),
    };
    let has_more = offset_val.saturating_add(20) < total;

    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("get_manga_list", "internal_error", "overflow");
    }

    let mut iter = match api_array(api_json, b"data") {
        Some(it) => it,
        None => return write_error("get_manga_list", "parse_error", "no data"),
    };

    let mut written = 0usize;
    while let Some(obj) = iter.next_object() {
        if written >= 20 { break; }
        let id = match api_string(obj, b"id") {
            Some(v) => v,
            None => continue,
        };
        let st = api_string(api_attributes(obj), b"status").unwrap_or(b"unknown");
        let cover_fn = first_covers.file(id).or_else(|| extract_cover_filename(obj));

        if written > 0 {
            if !write_bytes(payload, &mut c, b",") { return write_error("get_manga_list", "internal_error", "overflow"); }
        }
        let ok = write_bytes(payload, &mut c, br#"{"id":"mdx:"#)
            && append_api_string(payload, &mut c, id)
            && write_bytes(payload, &mut c, br#"","title":""#)
            && append_manga_title(payload, &mut c, obj, req)
            && write_bytes(payload, &mut c, br#"","cover":{"kind":"url","url":""#)
            && write_bytes(payload, &mut c, COVERS_BASE)
            && append_api_string(payload, &mut c, id)
            && write_bytes(payload, &mut c, b"/");
        if !ok { return write_error("get_manga_list", "internal_error", "overflow"); }
        if let Some(cfn) = cover_fn {
            if !append_cover_filename(payload, &mut c, cfn, req) { return write_error("get_manga_list", "internal_error", "overflow"); }
        } else {
            if !write_bytes(payload, &mut c, b"none.jpg") { return write_error("get_manga_list", "internal_error", "overflow"); }
        }
        let ok2 = write_bytes(payload, &mut c, br#""},"authors":[],"status":""#)
            && append_api_string(payload, &mut c, st)
            && write_bytes(payload, &mut c, br#"","contentRating":"unknown","sourceTags":["mangadex"]}"#);
        if !ok2 { return write_error("get_manga_list", "internal_error", "overflow"); }
        written += 1;
    }

    // A terminal page must not advertise another cursor.
    let ok_close = write_bytes(payload, &mut c, br#"],"page":{"nextCursor":"#)
        && if has_more {
            write_bytes(payload, &mut c, b"\"")
                && write_usize(payload, &mut c, page_num + 1)
                && write_bytes(payload, &mut c, b"\"")
        } else { write_bytes(payload, &mut c, b"null") }
        && write_bytes(payload, &mut c, br#","hasMore":"#);
    if !ok_close { return write_error("get_manga_list", "internal_error", "overflow"); }
    if has_more {
        if !write_bytes(payload, &mut c, b"true}}") {
            return write_error("get_manga_list", "internal_error", "overflow");
        }
    } else {
        if !write_bytes(payload, &mut c, b"false}}") {
            return write_error("get_manga_list", "internal_error", "overflow");
        }
    }

    write_success_payload("get_manga_list", c)
}

// --- modify_image_request: pass-through (MangaDex CDN needs no special headers) ---

fn run_image_request(req: &[u8]) -> u32 {
    // Just return the URL and empty headers as-is
    let url = match extract_json_string(req, b"url") {
        Some(u) => u,
        None => return write_error("image_request", "invalid_request", "missing url"),
    };

    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"url":""#)
        && append_json_escaped(payload, &mut c, url)
        && write_bytes(payload, &mut c, br#"","headers":{}}"#);
    if !ok { return write_error("image_request", "internal_error", "overflow"); }
    write_success_payload("image_request", c)
}

// --- get_home: multi-section home page ---

fn run_get_home(req: &[u8]) -> u32 {
    let url_popular = b"https://api.mangadex.org/manga?limit=10&includes%5B%5D=cover_art&order%5BfollowedCount%5D=desc&hasAvailableChapters=true";
    let url_latest = b"https://api.mangadex.org/manga?limit=10&includes%5B%5D=cover_art&order%5BlatestUploadedChapter%5D=desc&hasAvailableChapters=true";
    let url_top_rated = b"https://api.mangadex.org/manga?limit=10&includes%5B%5D=cover_art&order%5Brating%5D=desc&hasAvailableChapters=true";

    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"sections":["#) {
        return write_error("get_home", "internal_error", "overflow");
    }

    // Fetch and emit each section
    let sections: [(&[u8], &[u8], &[u8]); 3] = [
        (b"listing:popular", b"Popular New Titles", url_popular),
        (b"listing:latest", b"Recently Updated", url_latest),
        (b"listing:top_rated", b"Top Rated", url_top_rated),
    ];

    let mut section_idx = 0usize;
    for (section_id, section_title, url) in sections.iter() {
        if section_idx > 0 {
            if !write_bytes(payload, &mut c, b",") { return write_error("get_home", "internal_error", "overflow"); }
        }
        let ok = write_bytes(payload, &mut c, br#"{"id":""#)
            && append_json_escaped(payload, &mut c, section_id)
            && write_bytes(payload, &mut c, br#"","kind":"mangaList","title":""#)
            && append_json_escaped(payload, &mut c, section_title)
            && write_bytes(payload, &mut c, br#"","items":["#);
        if !ok { return write_error("get_home", "internal_error", "overflow"); }

        if *section_id == b"listing:latest" && blacklist::active(req,b"blockedGroups_en",b"blockedUploader_en") {
            let latest=match filtered_latest(req,1,false){Ok(v)=>v,Err((code,message))=>return write_error("get_home",code,message)};
            if !emit_latest_items(payload,&mut c,req,&latest,true) || !write_bytes(payload,&mut c,b"]}") { return write_error("get_home","internal_error","overflow"); }
            section_idx+=1; continue;
        }
        // The homepage has no per-list filter, so use the saved default set.
        let url_buf = scratch_a();
        let mut url_len = 0usize;
        if !(write_bytes(url_buf, &mut url_len, url)
            && append_content_rating_query(url_buf, &mut url_len, req, false)
            && write_bytes(url_buf, &mut url_len, original_language::query(req, b"originalLanguage_en", false))) {
            return write_error("get_home", "internal_error", "url overflow");
        }
        let section_url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_len) };
        let api_json = match fetch_json(section_url) {
            Ok(value) => value,
            Err(error) => {
                let (code, message) = fetch_error_code(error);
                return write_error("get_home", code, message);
            }
        };
        let first_covers = first_covers(req, api_json);
        let mut iter = match api_array(api_json, b"data") {
            Some(iter) => iter,
            None => return write_error("get_home", "parse_error", "no data array"),
        };
        {
                let mut item_count = 0usize;
                while let Some(obj) = iter.next_object() {
                    if item_count >= 10 { break; }
                    let id = match api_string(obj, b"id") {
                        Some(v) => v,
                        None => continue,
                    };
                    let cover_fn = first_covers.file(id).or_else(|| extract_cover_filename(obj));

                    if item_count > 0 {
                        if !write_bytes(payload, &mut c, b",") { return write_error("get_home", "internal_error", "overflow"); }
                    }
                    let ok2 = write_bytes(payload, &mut c, br#"{"id":"mdx:"#)
                        && append_api_string(payload, &mut c, id)
                        && write_bytes(payload, &mut c, br#"","title":""#)
                        && append_manga_title(payload, &mut c, obj, req)
                        && write_bytes(payload, &mut c, br#"","cover":{"kind":"url","url":""#)
                        && write_bytes(payload, &mut c, COVERS_BASE)
                        && append_api_string(payload, &mut c, id)
                        && write_bytes(payload, &mut c, b"/");
                    if !ok2 { return write_error("get_home", "internal_error", "overflow"); }
                    if let Some(cfn) = cover_fn {
                        if !append_cover_filename(payload, &mut c, cfn, req) { return write_error("get_home", "internal_error", "overflow"); }
                    } else {
                        if !write_bytes(payload, &mut c, b"none.jpg") { return write_error("get_home", "internal_error", "overflow"); }
                    }
                    if !write_bytes(payload, &mut c, br#""}}"#) { return write_error("get_home", "internal_error", "overflow"); }
                    item_count += 1;
                }
        }

        if !write_bytes(payload, &mut c, b"]}") { return write_error("get_home", "internal_error", "overflow"); }
        section_idx += 1;
    }

    if !write_bytes(payload, &mut c, b"]}") {
        return write_error("get_home", "internal_error", "overflow");
    }
    write_success_payload("get_home", c)
}

// --- get_settings: source-level user preferences ---

fn run_get_settings(req: &[u8]) -> u32 {
    // MangaDex settings: preferred language, content rating, data saver mode
    const SETTINGS_JSON: &[u8] = br#"{"settings":[{"id":"thumbnailQuality_en","name":"Cover Quality","kind":"select","options":[{"value":"original","label":"Original"},{"value":".512.jpg","label":"Medium"},{"value":".256.jpg","label":"Low"}],"default":"original"},{"id":"language","name":"Preferred Language","kind":"select","options":[{"value":"en","label":"English"},{"value":"ja","label":"Japanese"},{"value":"zh","label":"Chinese (Simplified)"},{"value":"zh-hk","label":"Chinese (Traditional)"},{"value":"ko","label":"Korean"},{"value":"fr","label":"French"},{"value":"es","label":"Spanish"},{"value":"de","label":"German"},{"value":"pt-br","label":"Portuguese (Brazil)"}],"default":"en"},{"id":"dataSaver","name":"Data Saver","kind":"toggle","description":"Use compressed images to save bandwidth","default":"false"},{"id":"defaultContentRating","name":"Default Content Rating","kind":"select","options":[{"value":"safe","label":"Safe Only"},{"value":"suggestive","label":"Safe + Suggestive"},{"value":"erotica","label":"All (including Erotica)"}],"default":"suggestive"},{"id":"altTitlesInDesc_en","name":"Alternative titles in description","kind":"toggle","description":"Include a manga's alternative titles at the end of its description","default":"false"},{"id":"usePort443_en","name":"Use HTTPS port 443 only","kind":"toggle","description":"Enable to only request image servers that use port 443. This allows users with stricter firewall restrictions to access MangaDex images","default":"false"},{"id":"tryUsingFirstVolumeCover_en","name":"Attempt to use the first volume cover as cover","kind":"toggle","description":"Existing library entries may need a manual refresh to show changed covers","default":"false"},{"id":"originalLanguage_en","name":"Filter original languages","kind":"multiselect","description":"Only show content that was originally published in the selected languages in both latest and browse","default":[],"options":[{"value":"ja","label":"Japanese"},{"value":"zh","label":"Chinese"},{"value":"ko","label":"Korean"}]},{"id":"finalChapterInDesc_en","name":"Final chapter in description","kind":"toggle","description":"Include a manga's final chapter number at the end of its description","default":"false"},{"id":"preferExtensionLangTitle_en","name":"Use alternative titles","kind":"toggle","description":"If there is an alternative title available which matches the extension language, it will be used","default":"true"}]}"#;

    let context=if rating_context_supported(req){json_object_member(req,b"args").and_then(|v|json_object_member(v,b"settingValues")).filter(|v|v.first()==Some(&b'{'))}else{None};
    if let Some(values)=context {
        let payload=payload_buf();let mut n=0;
        let Some(mut settings)=json_object_member(SETTINGS_JSON,b"settings").and_then(JsonArrayIter::from_raw_array)else{return write_error("get_settings","internal_error","invalid settings schema")};
        if !write_bytes(payload,&mut n,br#"{"settings":["#){return write_error("get_settings","internal_error","payload overflow");}
        let mut first=true;
        while let Some(field)=settings.next_object(){
            if !first&&!write_bytes(payload,&mut n,b","){return write_error("get_settings","internal_error","payload overflow");}first=false;
            let ok=if api_string(field,b"id")==Some(b"defaultContentRating"){
                write_bytes(payload,&mut n,br#"{"id":"contentRating_en","name":"Default content rating","kind":"multiselect","description":"Show content with the selected ratings by default","default":"#)
                &&write_bytes(payload,&mut n,schema_rating_default(values))
                &&write_bytes(payload,&mut n,br#", "options":[{"value":"safe","label":"Safe"},{"value":"suggestive","label":"Suggestive"},{"value":"erotica","label":"Erotica"},{"value":"pornographic","label":"Pornographic"}]}"#)
            }else{write_bytes(payload,&mut n,field)};
            if !ok{return write_error("get_settings","internal_error","payload overflow");}
        }
        if exclusion_host(req) && !write_bytes(payload,&mut n,br#",{"id":"includeUnavailable_en","name":"Show unavailable chapters","kind":"toggle","default":"false"}"#) {return write_error("get_settings","internal_error","payload overflow");}
        if blacklist::supported(req) && !write_bytes(payload,&mut n,br#",{"id":"blockedGroups_en","name":"Block groups by UUID","kind":"text","description":"Chapters from blocked groups will not show up in Latest or Manga feed. Enter as a Comma-separated list of group UUIDs","default":"","validation":{"kind":"uuid_list","message":"The text contains invalid UUIDs"}},{"id":"blockedUploader_en","name":"Block uploader by UUID","kind":"text","description":"Chapters from blocked uploaders will not show up in Latest or Manga feed. Enter as a Comma-separated list of uploader UUIDs","default":"","validation":{"kind":"uuid_list","message":"The text contains invalid UUIDs"}}"#) { return write_error("get_settings","internal_error","payload overflow"); }
        if !write_bytes(payload,&mut n,b"]}"){return write_error("get_settings","internal_error","payload overflow");}
        return write_success_payload("get_settings",n);
    }
    let payload = payload_buf();
    if exclusion_host(req) && !blacklist::supported(req) {
        let mut n=0;
        if !(write_bytes(payload,&mut n,&SETTINGS_JSON[..SETTINGS_JSON.len()-2]) && write_bytes(payload,&mut n,br#",{"id":"includeUnavailable_en","name":"Show unavailable chapters","kind":"toggle","default":"false"}]}"#)) {return write_error("get_settings","internal_error","payload overflow");}
        return write_success_payload("get_settings",n);
    }
    if blacklist::supported(req) {
        let mut n=0;
        if !(write_bytes(payload,&mut n,&SETTINGS_JSON[..SETTINGS_JSON.len()-2]) && write_bytes(payload,&mut n,br#",{"id":"includeUnavailable_en","name":"Show unavailable chapters","kind":"toggle","default":"false"}"#) && write_bytes(payload,&mut n,br#",{"id":"blockedGroups_en","name":"Block groups by UUID","kind":"text","description":"Chapters from blocked groups will not show up in Latest or Manga feed. Enter as a Comma-separated list of group UUIDs","default":"","validation":{"kind":"uuid_list","message":"The text contains invalid UUIDs"}},{"id":"blockedUploader_en","name":"Block uploader by UUID","kind":"text","description":"Chapters from blocked uploaders will not show up in Latest or Manga feed. Enter as a Comma-separated list of uploader UUIDs","default":"","validation":{"kind":"uuid_list","message":"The text contains invalid UUIDs"}}"#) && write_bytes(payload,&mut n,b"]}")) { return write_error("get_settings","internal_error","payload overflow"); }
        return write_success_payload("get_settings",n);
    }
    let flen = SETTINGS_JSON.len();
    if flen > payload.len() {
        return write_error("get_settings", "internal_error", "payload overflow");
    }
    payload[..flen].copy_from_slice(SETTINGS_JSON);
    write_success_payload("get_settings", flen)
}

// --- Exports ---

#[no_mangle]
pub extern "C" fn koma_source_init(_manifest_ptr: u32, manifest_len: u32) -> i32 {
    log_info(b"mangadex source init");
    if host::check_cancel() { return -2; }
    if manifest_len > 0 { 0 } else { -1 }
}

#[no_mangle]
pub extern "C" fn koma_source_info() -> u32 {
    response_buffer().write_source_metadata(&SOURCE_INFO, &SOURCE_CAPS)
}

#[no_mangle]
pub extern "C" fn koma_source_search(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("search", "invalid_request", "empty"),
    };
    log_info(b"mangadex search");
    run_search(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga", "invalid_request", "empty"),
    };
    log_info(b"mangadex get_manga");
    run_get_manga(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_chapters(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_chapters", "invalid_request", "empty"),
    };
    log_info(b"mangadex get_chapters");
    run_get_chapters(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_pages(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_pages", "invalid_request", "empty"),
    };
    log_info(b"mangadex get_pages");
    run_get_pages(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_listings(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_listings", "invalid_request", "empty"),
    };
    log_info(b"mangadex get_listings");
    run_get_listings(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_filters(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_filters", "invalid_request", "empty"),
    };
    log_info(b"mangadex get_filters");
    run_get_filters(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga_list(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga_list", "invalid_request", "empty"),
    };
    log_info(b"mangadex get_manga_list");
    run_get_manga_list(req)
}

#[no_mangle]
pub extern "C" fn koma_source_modify_image_request(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("image_request", "invalid_request", "empty"),
    };
    log_info(b"mangadex modify_image_request");
    run_image_request(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_home(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_home", "invalid_request", "empty"),
    };
    log_info(b"mangadex get_home");
    run_get_home(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_settings(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_settings", "invalid_request", "empty"),
    };
    log_info(b"mangadex get_settings");
    run_get_settings(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_image_request(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_image_request", "invalid_request", "empty"),
    };
    log_info(b"mangadex get_image_request");
    // MangaDex images are direct URLs, no modification needed
    let url = match extract_json_string(req, b"url") {
        Some(u) => u,
        None => return write_error("get_image_request", "invalid_request", "missing url"),
    };
    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"url":""#)
        && append_json_escaped(payload, &mut c, url)
        && write_bytes(payload, &mut c, br#"","headers":{}}"#);
    if !ok {
        return write_error("get_image_request", "internal_error", "payload overflow");
    }
    write_success_payload("get_image_request", c)
}

#[no_mangle]
pub extern "C" fn koma_source_free(result_ptr: u32) {
    response_buffer().free(result_ptr)
}
