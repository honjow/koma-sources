#![no_std]

extern crate koma_source_sdk;

use koma_source_sdk::host::{self, http_request, log_info};
use koma_source_sdk::json_utils::{
    append_json_escaped, append_json_unescaped_then_escaped, contains_bytes, extract_json_number,
    extract_json_string, find_subslice, write_bytes, write_url_encoded, write_usize,
};
use koma_source_sdk::result::ResultBuffer;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};

const SITE_BASE: &[u8] = b"https://www.mhua5.com";
const PAGE_SIZE: usize = 18;

koma_source_sdk::koma_source_buffers! {
    payload: 256 * 1024,
    http_out: 2 * 1024 * 1024,
    body: 2 * 1024 * 1024,
    http_req: 4096,
    scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.manhuawu.koma",
    name: "\u{6f2b}\u{753b}\u{5c4b} (Manhuawu)",
    version: "0.1.0",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "\u{6f2b}\u{753b}\u{5c4b} manga source (mhua5.com) based on MCCMS API",
    content_rating: "unknown",
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
    image_request: true,
    credentials: false,
};

#[cfg(not(test))]

    unsafe { &mut *core::ptr::addr_of_mut!(RESPONSE) }
}

fn read_request<'a>(req_ptr: u32, req_len: u32) -> Option<&'a [u8]> {
    if req_ptr == 0 || req_len == 0 {
        return None;
    }
    Some(unsafe { core::slice::from_raw_parts(req_ptr as *const u8, req_len as usize) })
}

// --- Raw JSON array iterator (for arrays that are not keyed in an object) ---

struct RawArrayIter<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> RawArrayIter<'a> {
    fn new(data: &'a [u8]) -> Self {
        // Find the '[' and start after it
        let pos = match find_subslice(data, b"[") {
            Some(p) => p + 1,
            None => data.len(),
        };
        Self { data, pos }
    }

    fn next_object(&mut self) -> Option<&'a [u8]> {
        while self.pos < self.data.len() {
            match self.data[self.pos] {
                b' ' | b'\t' | b'\n' | b'\r' | b',' => self.pos += 1,
                b']' | b'}' => return None,
                b'{' => break,
                _ => return None,
            }
        }
        if self.pos >= self.data.len() {
            return None;
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

// --- HTTP helpers ---

fn fetch_json(url_bytes: &[u8]) -> Result<usize, ()> {
    let req_len = build_get_request(http_req_buf(), url_bytes).ok_or(())?;
    let mut resp_len = 0usize;
    let mut transport_failed = true;
    for attempt in 0..3u8 {
        let req_slice = &http_req_buf()[..req_len];
        match http_request(req_slice, http_out()) {
            Ok(n) => {
                resp_len = n;
                transport_failed = false;
                break;
            }
            Err(_) => {
                if attempt < 2 {
                    log_info(b"manhuawu: http transport error, retrying");
                }
            }
        }
    }
    if transport_failed {
        return Err(());
    }

    let resp = &http_out()[..resp_len];
    if !contains_bytes(resp, br#""ok":true"#) {
        log_info(b"manhuawu: http response not ok");
        return Err(());
    }

    // Decode bodyText into body_buf
    let body_marker = b"\"bodyText\":\"";
    let body_start = find_subslice(resp, body_marker).ok_or(())? + body_marker.len();
    let out = body_buf();
    let mut out_cursor = 0usize;
    let mut i = body_start;
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
            if out_cursor >= out.len() {
                return Err(());
            }
            out[out_cursor] = unescaped;
            out_cursor += 1;
            i += 2;
            continue;
        }
        if b == b'"' {
            return Ok(out_cursor);
        }
        if out_cursor >= out.len() {
            return Err(());
        }
        out[out_cursor] = b;
        out_cursor += 1;
        i += 1;
    }
    Err(())
}

fn body_slice(len: usize) -> &'static [u8] {
    unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), len) }
}

// --- URL building helpers ---

fn build_list_url(page: usize, order: &[u8]) -> Option<usize> {
    let dst = scratch_a();
    let mut c = 0usize;
    write_bytes(dst, &mut c, SITE_BASE).then_some(())?;
    write_bytes(dst, &mut c, b"/api/data/comic?page=").then_some(())?;
    write_usize(dst, &mut c, page).then_some(())?;
    write_bytes(dst, &mut c, b"&size=18&order=").then_some(())?;
    write_bytes(dst, &mut c, order).then_some(())?;
    Some(c)
}

fn build_search_url(query: &[u8], page: usize) -> Option<usize> {
    let dst = scratch_a();
    let mut c = 0usize;
    write_bytes(dst, &mut c, SITE_BASE).then_some(())?;
    write_bytes(dst, &mut c, b"/api/data/comic?page=").then_some(())?;
    write_usize(dst, &mut c, page).then_some(())?;
    write_bytes(dst, &mut c, b"&size=18&key=").then_some(())?;
    write_url_encoded(dst, &mut c, query).then_some(())?;
    Some(c)
}

fn build_chapter_url(mid: &[u8]) -> Option<usize> {
    let dst = scratch_a();
    let mut c = 0usize;
    write_bytes(dst, &mut c, SITE_BASE).then_some(())?;
    write_bytes(dst, &mut c, b"/api/comic/chapter?mid=").then_some(())?;
    write_bytes(dst, &mut c, mid).then_some(())?;
    Some(c)
}

fn build_reader_url(chapter_url: &[u8]) -> Option<usize> {
    let dst = scratch_a();
    let mut c = 0usize;
    write_bytes(dst, &mut c, SITE_BASE).then_some(())?;
    write_bytes(dst, &mut c, chapter_url).then_some(())?;
    Some(c)
}

// --- mangaId parsing ---
// Format: mh:SLUG:PICID  (e.g. mh:some-slug-name:12345)

fn parse_manga_id(manga_id: &[u8]) -> Option<(&[u8], &[u8])> {
    let prefix = b"mh:";
    if manga_id.len() <= prefix.len() || &manga_id[..prefix.len()] != prefix {
        return None;
    }
    let rest = &manga_id[prefix.len()..];
    // Find last colon to split SLUG:PICID
    let mut last_colon = 0usize;
    let mut i = 0usize;
    while i < rest.len() {
        if rest[i] == b':' {
            last_colon = i;
        }
        i += 1;
    }
    if last_colon == 0 || last_colon >= rest.len() - 1 {
        return None;
    }
    Some((&rest[..last_colon], &rest[last_colon + 1..]))
}

// --- Parse items from MCCMS API ---

fn extract_pic_id(pic_field: &[u8]) -> Option<&[u8]> {
    let hash_pos = find_subslice(pic_field, b"#")?;
    let id_start = hash_pos + 1;
    if id_start >= pic_field.len() {
        return None;
    }
    let mut end = id_start;
    while end < pic_field.len() && pic_field[end] >= b'0' && pic_field[end] <= b'9' {
        end += 1;
    }
    if end == id_start {
        return None;
    }
    Some(&pic_field[id_start..end])
}

/// Extract slug from url like "/comic/SLUG.html"
fn extract_slug(url_field: &[u8]) -> Option<&[u8]> {
    let prefix = b"/comic/";
    let start = find_subslice(url_field, prefix)? + prefix.len();
    let mut end = start;
    while end < url_field.len()
        && url_field[end] != b'.'
        && url_field[end] != b'?'
        && url_field[end] != b'#'
    {
        end += 1;
    }
    if end == start {
        return None;
    }
    Some(&url_field[start..end])
}

fn write_manga_items(api_json: &[u8], operation: &str) -> u32 {
    // Validate that the response looks like JSON (starts with [ or contains [{)
    let array_start = match find_subslice(api_json, b"[{") {
        Some(pos) => pos,
        None => {
            // No valid JSON array found
            let payload = payload_buf();
            let mut c = 0usize;
            if !write_bytes(
                payload,
                &mut c,
                br#"{"items":[],"page":{"nextCursor":null,"hasMore":false}}"#,
            ) {
                return write_error(operation, "internal_error", "overflow");
            }
            return write_success_payload(operation, c);
        }
    };

    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error(operation, "internal_error", "overflow");
    }

    let array_data = &api_json[array_start..];
    let mut iter = RawArrayIter::new(array_data);
    let mut written = 0usize;

    while let Some(obj) = iter.next_object() {
        let name = match extract_json_string(obj, b"name") {
            Some(v) => v,
            None => continue,
        };
        let url_field = match extract_json_string(obj, b"url") {
            Some(v) => v,
            None => continue,
        };
        let pic_field = extract_json_string(obj, b"pic").unwrap_or(b"");
        let slug = match extract_slug(url_field) {
            Some(s) => s,
            None => continue,
        };
        let pic_id = extract_pic_id(pic_field).unwrap_or(b"0");

        if written > 0 {
            if !write_bytes(payload, &mut c, b",") {
                break;
            }
        }

        // mangaId = mh:SLUG:PICID
        let ok = write_bytes(payload, &mut c, br#"{"id":"mh:"#)
            && append_json_escaped(payload, &mut c, slug)
            && write_bytes(payload, &mut c, b":")
            && append_json_escaped(payload, &mut c, pic_id)
            && write_bytes(payload, &mut c, br#"","title":""#)
            && append_json_unescaped_then_escaped(payload, &mut c, name)
            && write_bytes(payload, &mut c, br#"","cover":{"kind":"url","url":""#)
            && append_json_unescaped_then_escaped(payload, &mut c, pic_field)
            && write_bytes(payload, &mut c, br#""},"authors":[],"status":"unknown","contentRating":"unknown","sourceTags":["manhuawu"]}"#);

        if !ok {
            break;
        }
        written += 1;
    }

    let has_more: &[u8] = if written >= PAGE_SIZE {
        b"true"
    } else {
        b"false"
    };
    if !(write_bytes(
        payload,
        &mut c,
        br#"],"page":{"nextCursor":null,"hasMore":"#,
    ) && write_bytes(payload, &mut c, has_more)
        && write_bytes(payload, &mut c, br#"}}"#))
    {
        return write_error(operation, "internal_error", "overflow");
    }
    write_success_payload(operation, c)
}

// --- Operations ---

fn run_search(req: &[u8]) -> u32 {
    let query = match extract_json_string(req, b"query") {
        Some(q) => q,
        None => return write_error("search", "invalid_request", "missing query"),
    };

    let page_bytes = extract_json_number(req, b"page");
    let page_num = parse_usize_default(page_bytes, 1);

    let url_len = match build_search_url(query, page_num) {
        Some(v) => v,
        None => return write_error("search", "internal_error", "url overflow"),
    };
    let url_bytes = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_len) };

    let body_len = match fetch_json(url_bytes) {
        Ok(v) => v,
        Err(_) => {
            log_info(b"manhuawu: search fetch failed");
            return write_error("search", "network_error", "fetch failed");
        }
    };
    let api_json = body_slice(body_len);

    write_manga_items(api_json, "search")
}

fn run_get_manga_list(req: &[u8]) -> u32 {
    let page_bytes = extract_json_number(req, b"page");
    let page_num = parse_usize_default(page_bytes, 1);

    let url_len = match build_list_url(page_num, b"hits") {
        Some(v) => v,
        None => return write_error("get_manga_list", "internal_error", "url overflow"),
    };
    let url_bytes = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_len) };

    let body_len = match fetch_json(url_bytes) {
        Ok(v) => v,
        Err(_) => return write_error("get_manga_list", "network_error", "fetch failed"),
    };
    let api_json = body_slice(body_len);

    write_manga_items(api_json, "get_manga_list")
}

fn run_get_manga(req: &[u8]) -> u32 {
    let manga_id = match extract_json_string(req, b"mangaId") {
        Some(v) => v,
        None => return write_error("get_manga", "invalid_request", "missing mangaId"),
    };

    let (slug, _pic_id) = match parse_manga_id(manga_id) {
        Some(v) => v,
        None => return write_error("get_manga", "invalid_request", "invalid mangaId format"),
    };

    // Search for this manga by slug
    let url_len = match build_search_url(slug, 1) {
        Some(v) => v,
        None => return write_error("get_manga", "internal_error", "url overflow"),
    };
    let url_bytes = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_len) };

    let body_len = match fetch_json(url_bytes) {
        Ok(v) => v,
        Err(_) => return write_error("get_manga", "network_error", "fetch failed"),
    };
    let api_json = body_slice(body_len);

    // Find the matching item by slug in the URL field
    let array_start = match find_subslice(api_json, b"[{") {
        Some(pos) => pos,
        None => {
            return write_error(
                "get_manga",
                "not_found",
                "manga not found in search results",
            )
        }
    };
    let array_data = &api_json[array_start..];
    let mut iter = RawArrayIter::new(array_data);

    while let Some(obj) = iter.next_object() {
        let url_field = match extract_json_string(obj, b"url") {
            Some(v) => v,
            None => continue,
        };
        let obj_slug = match extract_slug(url_field) {
            Some(s) => s,
            None => continue,
        };

        if obj_slug == slug {
            let name = extract_json_string(obj, b"name").unwrap_or(b"Unknown");
            let pic_field = extract_json_string(obj, b"pic").unwrap_or(b"");
            let description = extract_json_string(obj, b"description").unwrap_or(b"");
            let author = extract_json_string(obj, b"author").unwrap_or(b"");
            let status_str = extract_json_string(obj, b"serialise").unwrap_or(b"");

            let status = parse_status(status_str);

            let payload = payload_buf();
            let mut c = 0usize;

            let ok = write_bytes(payload, &mut c, br#"{"manga":{"id":""#)
                && write_bytes(payload, &mut c, br#"""#)
                && append_json_escaped(payload, &mut c, manga_id)
                && write_bytes(payload, &mut c, br#"","title":""#)
                && append_json_unescaped_then_escaped(payload, &mut c, name)
                && write_bytes(payload, &mut c, br#"","description":""#)
                && append_json_unescaped_then_escaped(payload, &mut c, description)
                && write_bytes(payload, &mut c, br#"","cover":{"kind":"url","url":""#)
                && append_json_unescaped_then_escaped(payload, &mut c, pic_field)
                && write_bytes(payload, &mut c, br#""},"authors":["#);

            if !ok {
                return write_error("get_manga", "internal_error", "overflow");
            }

            // Write author(s) - comma/slash separated
            if !author.is_empty() {
                let mut ai = 0usize;
                let mut first = true;
                while ai < author.len() {
                    while ai < author.len()
                        && (author[ai] == b',' || author[ai] == b' ' || author[ai] == b'/')
                    {
                        ai += 1;
                    }
                    if ai >= author.len() {
                        break;
                    }
                    let start = ai;
                    while ai < author.len() && author[ai] != b',' && author[ai] != b'/' {
                        ai += 1;
                    }
                    let segment = &author[start..ai];
                    if !segment.is_empty() {
                        if !first {
                            if !write_bytes(payload, &mut c, b",") {
                                return write_error("get_manga", "internal_error", "overflow");
                            }
                        }
                        if !(write_bytes(payload, &mut c, b"\"")
                            && append_json_unescaped_then_escaped(payload, &mut c, segment)
                            && write_bytes(payload, &mut c, b"\""))
                        {
                            return write_error("get_manga", "internal_error", "overflow");
                        }
                        first = false;
                    }
                }
            }

            let ok2 = write_bytes(payload, &mut c, br#"],"artists":[],"status":""#)
                && write_bytes(payload, &mut c, status)
                && write_bytes(
                    payload,
                    &mut c,
                    br#"","contentRating":"unknown","language":"zh","tags":["manhuawu"],"links":[]}}"#,
                );

            if !ok2 {
                return write_error("get_manga", "internal_error", "overflow");
            }
            return write_success_payload("get_manga", c);
        }
    }

    write_error("get_manga", "not_found", "manga not found")
}

fn run_get_chapters(req: &[u8]) -> u32 {
    let manga_id = match extract_json_string(req, b"mangaId") {
        Some(v) => v,
        None => return write_error("get_chapters", "invalid_request", "missing mangaId"),
    };

    let (_slug, pic_id) = match parse_manga_id(manga_id) {
        Some(v) => v,
        None => return write_error("get_chapters", "invalid_request", "invalid mangaId format"),
    };

    let url_len = match build_chapter_url(pic_id) {
        Some(v) => v,
        None => return write_error("get_chapters", "internal_error", "url overflow"),
    };
    let url_bytes = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_len) };

    let body_len = match fetch_json(url_bytes) {
        Ok(v) => v,
        Err(_) => return write_error("get_chapters", "network_error", "fetch failed"),
    };
    let api_json = body_slice(body_len);

    // API returns: {"code":1,"data":[{id, name, link, pnum, ...}]}
    // Find the "data" array
    let array_start = match find_subslice(api_json, b"\"data\":[{") {
        Some(pos) => pos + 7, // skip "data": to get to the [
        None => {
            let payload = payload_buf();
            let mut c = 0usize;
            if write_bytes(
                payload,
                &mut c,
                br#"{"items":[],"page":{"nextCursor":null,"hasMore":false}}"#,
            ) {
                return write_success_payload("get_chapters", c);
            }
            return write_error("get_chapters", "internal_error", "overflow");
        }
    };
    let array_data = &api_json[array_start..];

    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("get_chapters", "internal_error", "overflow");
    }

    let mut iter = RawArrayIter::new(array_data);
    let mut written = 0usize;
    let mut items: [(&[u8], &[u8], &[u8]); 512] = [(&[], &[], &[]); 512];

    while let Some(obj) = iter.next_object() {
        if written >= items.len() {
            break;
        }
        let _ch_id = extract_json_number(obj, b"id").unwrap_or(b"0");
        let ch_name = extract_json_string(obj, b"name").unwrap_or(b"");
        // Use "link" field (e.g. /index.php/chapter/ID)
        let ch_link = extract_json_string(obj, b"link").unwrap_or(b"");
        items[written] = (_ch_id, ch_name, ch_link);
        written += 1;
    }

    // Write in reverse order (chapters come newest first, we want oldest first for reading)
    let mut first = true;
    let mut idx = written;
    while idx > 0 {
        idx -= 1;
        let (_ch_id, ch_name, ch_link) = items[idx];

        if !first {
            if !write_bytes(payload, &mut c, b",") {
                return write_error("get_chapters", "internal_error", "overflow");
            }
        }
        first = false;

        let ok = write_bytes(payload, &mut c, br#"{"id":"ch:"#)
            && append_json_escaped(payload, &mut c, ch_link)
            && write_bytes(payload, &mut c, br#"","mangaId":""#)
            && append_json_escaped(payload, &mut c, manga_id)
            && write_bytes(payload, &mut c, br#"","title":""#)
            && append_json_unescaped_then_escaped(payload, &mut c, ch_name)
            && write_bytes(
                payload,
                &mut c,
                br#"","chapterNumber":null,"volumeNumber":null,"language":"zh","publishedAt":null,"updatedAt":null}"#,
            );

        if !ok {
            return write_error("get_chapters", "internal_error", "overflow");
        }
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

    let prefix = b"ch:";
    if chapter_id.len() <= prefix.len() || &chapter_id[..prefix.len()] != prefix {
        return write_error("get_pages", "invalid_request", "invalid chapterId format");
    }
    let ch_url = &chapter_id[prefix.len()..];

    let url_len = match build_reader_url(ch_url) {
        Some(v) => v,
        None => return write_error("get_pages", "internal_error", "url overflow"),
    };
    let url_bytes = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_len) };

    let html_len = match fetch_json(url_bytes) {
        Ok(v) => v,
        Err(_) => return write_error("get_pages", "network_error", "fetch failed"),
    };
    let html = body_slice(html_len);

    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"chapterId":""#)
        || !append_json_escaped(payload, &mut c, chapter_id)
        || !write_bytes(payload, &mut c, br#"","pages":["#)
    {
        return write_error("get_pages", "internal_error", "overflow");
    }

    let mut written = 0usize;
    let mut pos = 0usize;

    while pos < html.len() {
        if pos + 4 <= html.len() && &html[pos..pos + 4] == b"<img" {
            let tag_start = pos;
            let mut tag_end = pos + 4;
            while tag_end < html.len() && html[tag_end] != b'>' {
                tag_end += 1;
            }
            if tag_end < html.len() {
                tag_end += 1;
            }
            let tag = &html[tag_start..tag_end];

            let img_url = extract_attr_value(tag, b"data-original")
                .or_else(|| extract_attr_value(tag, b"data-src"))
                .or_else(|| extract_attr_value(tag, b"src"));

            if let Some(url) = img_url {
                if url.len() > 10
                    && (contains_bytes(url, b".jpg")
                        || contains_bytes(url, b".png")
                        || contains_bytes(url, b".jpeg")
                        || contains_bytes(url, b".webp")
                        || contains_bytes(url, b".gif")
                        || contains_bytes(url, b"mhua")
                        || contains_bytes(url, b"manga")
                        || contains_bytes(url, b"comic"))
                {
                    if written > 0 {
                        if !write_bytes(payload, &mut c, b",") {
                            break;
                        }
                    }
                    let ok = write_bytes(payload, &mut c, br#"{"id":"p:"#)
                        && write_usize(payload, &mut c, written)
                        && write_bytes(payload, &mut c, br#"","index":"#)
                        && write_usize(payload, &mut c, written)
                        && write_bytes(payload, &mut c, br#","image":{"kind":"url","url":""#)
                        && append_json_unescaped_then_escaped(payload, &mut c, url)
                        && write_bytes(payload, &mut c, br#""}}"#);

                    if !ok {
                        break;
                    }
                    written += 1;
                }
            }
            pos = tag_end;
        } else {
            pos += 1;
        }
    }

    if !write_bytes(payload, &mut c, br#"]}"#) {
        return write_error("get_pages", "internal_error", "overflow");
    }
    write_success_payload("get_pages", c)
}

fn run_get_listings(_req: &[u8]) -> u32 {
    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"listings":[]}"#) {
        return write_error("get_listings", "internal_error", "overflow");
    }
    write_success_payload("get_listings", c)
}

fn run_get_home(_req: &[u8]) -> u32 {
    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"sections":[]}"#) {
        return write_error("get_home", "internal_error", "overflow");
    }
    write_success_payload("get_home", c)
}

fn run_image_request(req: &[u8]) -> u32 {
    let url = match extract_json_string(req, b"url") {
        Some(u) => u,
        None => return write_error("get_image_request", "invalid_request", "missing url"),
    };

    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"url":""#)
        && append_json_escaped(payload, &mut c, url)
        && write_bytes(
            payload,
            &mut c,
            br#"","headers":{"Referer":"https://www.mhua5.com/"}}"#,
        );
    if !ok {
        return write_error("get_image_request", "internal_error", "overflow");
    }
    write_success_payload("get_image_request", c)
}

// --- Helper functions ---

fn parse_usize_default(bytes: Option<&[u8]>, default: usize) -> usize {
    match bytes {
        Some(b) => {
            let mut n = 0usize;
            for &byte in b {
                if byte >= b'0' && byte <= b'9' {
                    n = n * 10 + (byte - b'0') as usize;
                } else {
                    break;
                }
            }
            if n == 0 {
                default
            } else {
                n
            }
        }
        None => default,
    }
}

fn parse_status(status: &[u8]) -> &'static [u8] {
    if contains_bytes(
        status,
        &[0xe8, 0xbf, 0x9e, 0xe8, 0xbd, 0xbd, 0xe4, 0xb8, 0xad],
    ) {
        b"ongoing"
    } else if contains_bytes(
        status,
        &[0xe5, 0xb7, 0xb2, 0xe5, 0xae, 0x8c, 0xe7, 0xbb, 0x93],
    ) {
        b"completed"
    } else {
        b"unknown"
    }
}

fn extract_attr_value<'a>(tag: &'a [u8], attr: &[u8]) -> Option<&'a [u8]> {
    let mut pattern = [0u8; 64];
    if attr.len() + 2 > pattern.len() {
        return None;
    }
    pattern[..attr.len()].copy_from_slice(attr);
    pattern[attr.len()] = b'=';
    pattern[attr.len() + 1] = b'"';
    let pat_len = attr.len() + 2;

    let start = find_subslice(tag, &pattern[..pat_len])? + pat_len;
    let mut end = start;
    while end < tag.len() && tag[end] != b'"' {
        end += 1;
    }
    if end >= tag.len() {
        return None;
    }
    Some(&tag[start..end])
}

// --- Exports ---

#[no_mangle]
pub extern "C" fn koma_source_init(_manifest_ptr: u32, manifest_len: u32) -> i32 {
    log_info(b"manhuawu source init");
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
    log_info(b"manhuawu search");
    run_search(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga", "invalid_request", "empty request"),
    };
    log_info(b"manhuawu get_manga");
    run_get_manga(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_chapters(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_chapters", "invalid_request", "empty request"),
    };
    log_info(b"manhuawu get_chapters");
    run_get_chapters(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_pages(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_pages", "invalid_request", "empty request"),
    };
    log_info(b"manhuawu get_pages");
    run_get_pages(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_listings(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_listings", "invalid_request", "empty request"),
    };
    log_info(b"manhuawu get_listings");
    run_get_listings(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga_list(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga_list", "invalid_request", "empty request"),
    };
    log_info(b"manhuawu get_manga_list");
    run_get_manga_list(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_home(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_home", "invalid_request", "empty request"),
    };
    log_info(b"manhuawu get_home");
    run_get_home(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_image_request(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_image_request", "invalid_request", "empty request"),
    };
    log_info(b"manhuawu get_image_request");
    run_image_request(req)
}

#[no_mangle]
pub extern "C" fn koma_source_free(result_ptr: u32) {
    response_buffer().free(result_ptr)
}
