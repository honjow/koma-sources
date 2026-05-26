#![no_std]

extern crate koma_source_sdk;

use koma_source_sdk::host::{
    html_attr, html_close, html_parse, html_select, html_text, http_request, log_info,
    HtmlDescriptor,
};
use koma_source_sdk::json_utils::{
    append_json_escaped, append_json_unescaped_then_escaped, contains_bytes, extract_json_number,
    extract_json_string, find_subslice, write_bytes, write_usize,
};
use koma_source_sdk::result::{empty_filters, empty_home, empty_listings};
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{build_get_request, fetch_error_code, parse_status_code, FetchError};

#[link(wasm_import_module = "koma_host")]
extern "C" {
    #[link_name = "html_select_all"]
    fn koma_host_html_select_all(
        descriptor: i32,
        selector_ptr: *const u8,
        selector_len: u32,
        out_ptr: *mut u8,
        out_cap: u32,
    ) -> i32;
}

fn html_select_all(descriptor: i32, selector: &[u8], out: &mut [u8]) -> i32 {
    unsafe {
        koma_host_html_select_all(
            descriptor,
            selector.as_ptr(),
            selector.len() as u32,
            out.as_mut_ptr(),
            out.len() as u32,
        )
    }
}

static mut SELECT_ALL_BUF: [u8; 16000] = [0; 16000];

const SITE_BASE: &[u8] = b"https://comic.bh3.com";
const MANGA_PREFIX: &[u8] = b"bh3:";
const CHAPTER_PREFIX: &[u8] = b"bh3ch:";

koma_source_sdk::koma_source_buffers! {
    payload: 1024 * 1024,
    http_out: 2 * 1024 * 1024,
    body: 2 * 1024 * 1024,
    http_req: 2048,
    scratch: 4096,
}
koma_source_sdk::koma_source_helpers!();

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.bh3.koma",
    name: "崩坏3 IP站",
    version: "0.1.0",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "BH3 comic.bh3.com HTML and JSON source.",
    content_rating: "safe",
};

const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: false,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: false,
    manga_list: true,
    home: false,
    filters: false,
    settings: false,
    credentials: false,
    image_request: false,
};

fn body_slice(len: usize) -> &'static [u8] {
    unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(BODY_BUF) as *const u8, len) }
}

fn fetch_url(url: &[u8]) -> Result<usize, FetchError> {
    let req_len =
        build_get_request(http_req_buf(), url, Some(SITE_BASE), &[]).ok_or(FetchError::Network)?;
    let mut resp_len = 0usize;
    let mut failed = true;
    let mut attempt = 0u8;
    while attempt < 3 {
        match http_request(&http_req_buf()[..req_len], http_out()) {
            Ok(n) => {
                resp_len = n;
                failed = false;
                break;
            }
            Err(_) => {
                if attempt < 2 {
                    log_info(b"bh3: http transport error, retrying");
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
        return Err(
            if let Some(code_bytes) = extract_json_number(resp, b"statusCode") {
                match parse_status_code(code_bytes) {
                    404 => FetchError::NotFound,
                    429 => FetchError::RateLimit,
                    400..=499 => FetchError::ClientError,
                    500..=599 => FetchError::ServerError,
                    _ => FetchError::Network,
                }
            } else {
                FetchError::Network
            },
        );
    }
    decode_body_text(resp)
}

fn decode_body_text(resp: &[u8]) -> Result<usize, FetchError> {
    let marker = b"\"bodyText\":\"";
    let mut i = find_subslice(resp, marker).ok_or(FetchError::Network)? + marker.len();
    let dst = body_buf();
    let mut out = 0usize;
    while i < resp.len() {
        let b = resp[i];
        if b == b'\\' && i + 1 < resp.len() {
            let next = resp[i + 1];
            match next {
                b'"' | b'\\' | b'/' => {
                    if out >= dst.len() {
                        return Err(FetchError::Network);
                    }
                    dst[out] = if next == b'/' { b'/' } else { next };
                    out += 1;
                    i += 2;
                }
                b'n' | b'r' | b't' => {
                    if out >= dst.len() {
                        return Err(FetchError::Network);
                    }
                    dst[out] = if next == b'n' {
                        b'\n'
                    } else if next == b'r' {
                        b'\r'
                    } else {
                        b'\t'
                    };
                    out += 1;
                    i += 2;
                }
                b'u' => {
                    if i + 5 >= resp.len() {
                        return Err(FetchError::Network);
                    }
                    let mut code = 0u32;
                    let mut k = 0usize;
                    while k < 4 {
                        let h = resp[i + 2 + k];
                        let v = match h {
                            b'0'..=b'9' => (h - b'0') as u32,
                            b'a'..=b'f' => (h - b'a' + 10) as u32,
                            b'A'..=b'F' => (h - b'A' + 10) as u32,
                            _ => return Err(FetchError::Network),
                        };
                        code = (code << 4) | v;
                        k += 1;
                    }
                    let mut encoded = [0u8; 4];
                    let len = koma_source_sdk::json_utils::encode_utf8(code, &mut encoded);
                    if out + len > dst.len() {
                        return Err(FetchError::Network);
                    }
                    dst[out..out + len].copy_from_slice(&encoded[..len]);
                    out += len;
                    i += 6;
                }
                _ => {
                    if out >= dst.len() {
                        return Err(FetchError::Network);
                    }
                    dst[out] = next;
                    out += 1;
                    i += 2;
                }
            }
            continue;
        }
        if b == b'"' {
            return Ok(out);
        }
        if out >= dst.len() {
            return Err(FetchError::Network);
        }
        dst[out] = b;
        out += 1;
        i += 1;
    }
    Err(FetchError::Network)
}

struct OwnedDescriptor(HtmlDescriptor);

impl Drop for OwnedDescriptor {
    fn drop(&mut self) {
        let _ = html_close(self.0);
    }
}

fn attr_into<'a>(desc: HtmlDescriptor, name: &[u8], out: &'a mut [u8]) -> Option<&'a [u8]> {
    let len = html_attr(desc, name, out).ok()?;
    Some(&out[..len])
}

fn text_into<'a>(desc: HtmlDescriptor, out: &'a mut [u8]) -> Option<&'a [u8]> {
    let len = html_text(desc, out).ok()?;
    Some(&out[..len])
}

fn write_abs_url(dst: &mut [u8], cursor: &mut usize, url: &[u8]) -> bool {
    if starts_with(url, b"http://") || starts_with(url, b"https://") || starts_with(url, b"//") {
        append_json_escaped(dst, cursor, url)
    } else if starts_with(url, b"/") {
        write_bytes(dst, cursor, SITE_BASE) && append_json_escaped(dst, cursor, url)
    } else {
        append_json_escaped(dst, cursor, url)
    }
}

fn starts_with(bytes: &[u8], prefix: &[u8]) -> bool {
    bytes.len() >= prefix.len() && &bytes[..prefix.len()] == prefix
}

fn valid_decimal(bytes: &[u8]) -> bool {
    !bytes.is_empty() && bytes.iter().all(|b| *b >= b'0' && *b <= b'9')
}

fn extract_manga_numeric<'a>(req: &'a [u8], op: &str) -> Result<&'a [u8], u32> {
    let manga_id = match extract_json_string(req, b"mangaId") {
        Some(v) => v,
        None => return Err(write_error(op, "invalid_request", "missing mangaId")),
    };
    if manga_id.len() <= MANGA_PREFIX.len() || &manga_id[..MANGA_PREFIX.len()] != MANGA_PREFIX {
        return Err(write_error(op, "invalid_request", "unexpected mangaId"));
    }
    let id = &manga_id[MANGA_PREFIX.len()..];
    if !valid_decimal(id) {
        return Err(write_error(op, "invalid_request", "invalid mangaId"));
    }
    Ok(id)
}

fn next_top_object<'a>(data: &'a [u8], pos: &mut usize) -> Option<&'a [u8]> {
    while *pos < data.len() {
        match data[*pos] {
            b'[' | b',' | b' ' | b'\t' | b'\n' | b'\r' => *pos += 1,
            b']' => return None,
            b'{' => break,
            _ => return None,
        }
    }
    let start = *pos;
    let mut depth = 0i32;
    let mut in_string = false;
    while *pos < data.len() {
        let b = data[*pos];
        if in_string {
            if b == b'\\' {
                *pos += 1;
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
                        *pos += 1;
                        return Some(&data[start..*pos]);
                    }
                }
                _ => {}
            }
        }
        *pos += 1;
    }
    None
}

fn split_chapter_id<'a>(chapter_id: &'a [u8]) -> Option<(&'a [u8], &'a [u8])> {
    if chapter_id.len() <= CHAPTER_PREFIX.len()
        || &chapter_id[..CHAPTER_PREFIX.len()] != CHAPTER_PREFIX
    {
        return None;
    }
    let rest = &chapter_id[CHAPTER_PREFIX.len()..];
    let idx = find_subslice(rest, b":")?;
    let book_id = &rest[..idx];
    let chapter = &rest[idx + 1..];
    if valid_decimal(book_id) && valid_decimal(chapter) {
        Some((book_id, chapter))
    } else {
        None
    }
}

fn json_number_or_string<'a>(obj: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    extract_json_number(obj, key).or_else(|| extract_json_string(obj, key))
}

fn run_search(_req: &[u8]) -> u32 {
    write_error("search", "not_supported", "search is not supported")
}

fn run_get_manga(req: &[u8]) -> u32 {
    let id = match extract_manga_numeric(req, "get_manga") {
        Ok(v) => v,
        Err(ptr) => return ptr,
    };
    let url = scratch_a();
    let mut uc = 0usize;
    if !(write_bytes(url, &mut uc, SITE_BASE)
        && write_bytes(url, &mut uc, b"/book/")
        && write_bytes(url, &mut uc, id))
    {
        return write_error("get_manga", "internal_error", "url overflow");
    }
    let url_bytes =
        unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(SCRATCH_A) as *const u8, uc) };

    let len = match fetch_url(url_bytes) {
        Ok(n) => n,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_manga", c, m);
        }
    };
    let html = body_slice(len);
    let doc = match html_parse(html) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error("get_manga", "parse_error", "html_parse failed"),
    };

    let mut title_buf = [0u8; 256];
    let mut desc_buf = [0u8; 2048];
    let mut cover_buf = [0u8; 1024];
    let title = if let Ok(d) = html_select(doc.0, b"div.title") {
        let owned = OwnedDescriptor(d);
        text_into(owned.0, &mut title_buf).map(trim_ascii)
    } else {
        None
    };
    let description = if let Ok(d) = html_select(doc.0, b"div.detail_info1") {
        let owned = OwnedDescriptor(d);
        text_into(owned.0, &mut desc_buf).map(trim_ascii)
    } else {
        None
    };
    let cover = if let Ok(d) = html_select(doc.0, b"img.cover") {
        let owned = OwnedDescriptor(d);
        attr_into(owned.0, b"src", &mut cover_buf)
    } else {
        None
    };

    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"manga":{"id":"bh3:"#)
        && append_json_escaped(payload, &mut c, id)
        && write_bytes(payload, &mut c, br#"","title":""#)
        && append_json_escaped(payload, &mut c, title.unwrap_or(id))
        && write_bytes(
            payload,
            &mut c,
            br#"","alternateTitles":[],"description":""#,
        )
        && append_json_escaped(payload, &mut c, description.unwrap_or(&[]))
        && write_bytes(payload, &mut c, br#"","cover":{"kind":"url","url":""#);
    if !ok {
        return write_error("get_manga", "internal_error", "payload overflow");
    }
    if let Some(src) = cover {
        if !write_abs_url(payload, &mut c, src) {
            return write_error("get_manga", "internal_error", "payload overflow");
        }
    }
    let ok = write_bytes(payload, &mut c, br#""},"authors":[],"artists":[],"status":"unknown","contentRating":"safe","language":"zh","tags":["bh3"],"links":[{"kind":"source","url":"https://comic.bh3.com/book/"#)
        && append_json_escaped(payload, &mut c, id)
        && write_bytes(payload, &mut c, br#""}]}}"#);
    if !ok {
        return write_error("get_manga", "internal_error", "payload overflow");
    }
    write_success_payload("get_manga", c)
}

fn run_get_chapters(req: &[u8]) -> u32 {
    let id = match extract_manga_numeric(req, "get_chapters") {
        Ok(v) => v,
        Err(ptr) => return ptr,
    };
    let url = scratch_a();
    let mut uc = 0usize;
    if !(write_bytes(url, &mut uc, SITE_BASE)
        && write_bytes(url, &mut uc, b"/book/")
        && write_bytes(url, &mut uc, id)
        && write_bytes(url, &mut uc, b"/get_chapter"))
    {
        return write_error("get_chapters", "internal_error", "url overflow");
    }
    let url_bytes =
        unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(SCRATCH_A) as *const u8, uc) };

    let len = match fetch_url(url_bytes) {
        Ok(n) => n,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_chapters", c, m);
        }
    };
    let json = body_slice(len);
    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("get_chapters", "internal_error", "payload overflow");
    }
    let mut pos = 0usize;
    let mut written = 0usize;
    while let Some(obj) = next_top_object(json, &mut pos) {
        let title = extract_json_string(obj, b"title").unwrap_or(b"Chapter");
        let book_id = json_number_or_string(obj, b"bookid").unwrap_or(id);
        let chapter_id = json_number_or_string(obj, b"chapterid").unwrap_or(b"0");
        let timestamp = extract_json_string(obj, b"timestamp");
        if written > 0 && !write_bytes(payload, &mut c, b",") {
            return write_error("get_chapters", "internal_error", "payload overflow");
        }
        let ok = write_bytes(payload, &mut c, br#"{"id":"bh3ch:"#)
            && append_json_escaped(payload, &mut c, book_id)
            && write_bytes(payload, &mut c, b":")
            && append_json_escaped(payload, &mut c, chapter_id)
            && write_bytes(payload, &mut c, br#"","mangaId":"bh3:"#)
            && append_json_escaped(payload, &mut c, book_id)
            && write_bytes(payload, &mut c, br#"","title":""#)
            && append_json_unescaped_then_escaped(payload, &mut c, title)
            && write_bytes(payload, &mut c, br#"","chapterNumber":""#)
            && append_json_escaped(payload, &mut c, chapter_id)
            && write_bytes(
                payload,
                &mut c,
                br#"","volumeNumber":null,"language":"zh","publishedAt":"#,
            );
        if !ok {
            return write_error("get_chapters", "internal_error", "payload overflow");
        }
        if let Some(ts) = timestamp {
            if ts.len() == 19 {
                let ok_ts = write_bytes(payload, &mut c, b"\"")
                    && append_json_escaped(payload, &mut c, &ts[..10])
                    && write_bytes(payload, &mut c, b"T")
                    && append_json_escaped(payload, &mut c, &ts[11..])
                    && write_bytes(payload, &mut c, b"+08:00\"");
                if !ok_ts {
                    return write_error("get_chapters", "internal_error", "payload overflow");
                }
            } else if !(write_bytes(payload, &mut c, b"\"")
                && append_json_escaped(payload, &mut c, ts)
                && write_bytes(payload, &mut c, b"\""))
            {
                return write_error("get_chapters", "internal_error", "payload overflow");
            }
        } else if !write_bytes(payload, &mut c, b"null") {
            return write_error("get_chapters", "internal_error", "payload overflow");
        }
        if !write_bytes(payload, &mut c, br#","updatedAt":null,"pageCount":null}"#) {
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
    let (book_id, chapter_num) = match split_chapter_id(chapter_id) {
        Some(v) => v,
        None => return write_error("get_pages", "invalid_request", "unexpected chapterId"),
    };
    let url = scratch_a();
    let mut uc = 0usize;
    if !(write_bytes(url, &mut uc, SITE_BASE)
        && write_bytes(url, &mut uc, b"/book/")
        && write_bytes(url, &mut uc, book_id)
        && write_bytes(url, &mut uc, b"/")
        && write_bytes(url, &mut uc, chapter_num))
    {
        return write_error("get_pages", "internal_error", "url overflow");
    }
    let url_bytes =
        unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(SCRATCH_A) as *const u8, uc) };

    let len = match fetch_url(url_bytes) {
        Ok(n) => n,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_pages", c, m);
        }
    };
    let html = body_slice(len);
    let doc = match html_parse(html) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error("get_pages", "parse_error", "html_parse failed"),
    };
    let select_buf = unsafe { &mut *core::ptr::addr_of_mut!(SELECT_ALL_BUF) };
    let count = html_select_all(doc.0.raw(), b"img.lazy.comic_img", select_buf);
    let max = if count > 0 {
        (count as usize).min(500)
    } else {
        0
    };

    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"chapterId":"bh3ch:"#)
        && append_json_escaped(payload, &mut c, book_id)
        && write_bytes(payload, &mut c, b":")
        && append_json_escaped(payload, &mut c, chapter_num)
        && write_bytes(payload, &mut c, br#"","pages":["#);
    if !ok {
        return write_error("get_pages", "internal_error", "payload overflow");
    }
    let mut written = 0usize;
    let mut i = 0usize;
    while i < max {
        let off = i * 4;
        if off + 4 > select_buf.len() {
            break;
        }
        let desc = i32::from_le_bytes([
            select_buf[off],
            select_buf[off + 1],
            select_buf[off + 2],
            select_buf[off + 3],
        ]);
        if desc > 0 {
            let hd: HtmlDescriptor = unsafe { core::mem::transmute(desc) };
            let mut src_buf = [0u8; 1024];
            if let Some(src) = attr_into(hd, b"data-original", &mut src_buf) {
                if written > 0 && !write_bytes(payload, &mut c, b",") {
                    return write_error("get_pages", "internal_error", "payload overflow");
                }
                let ok_page = write_bytes(payload, &mut c, br#"{"id":"bh3page:"#)
                    && append_json_escaped(payload, &mut c, book_id)
                    && write_bytes(payload, &mut c, b":")
                    && append_json_escaped(payload, &mut c, chapter_num)
                    && write_bytes(payload, &mut c, b":")
                    && write_usize(payload, &mut c, written)
                    && write_bytes(payload, &mut c, br#"","index":"#)
                    && write_usize(payload, &mut c, written)
                    && write_bytes(payload, &mut c, br#","image":{"kind":"url","url":""#)
                    && write_abs_url(payload, &mut c, src)
                    && write_bytes(payload, &mut c, br#""}}"#);
                if !ok_page {
                    return write_error("get_pages", "internal_error", "payload overflow");
                }
                written += 1;
            }
            let _ = html_close(hd);
        }
        i += 1;
    }
    if !write_bytes(payload, &mut c, b"]}") {
        return write_error("get_pages", "internal_error", "payload overflow");
    }
    write_success_payload("get_pages", c)
}

fn run_get_manga_list(_req: &[u8]) -> u32 {
    let len = match fetch_url(b"https://comic.bh3.com/book") {
        Ok(n) => n,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_manga_list", c, m);
        }
    };
    let html = body_slice(len);
    let doc = match html_parse(html) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error("get_manga_list", "parse_error", "html_parse failed"),
    };
    let select_buf = unsafe { &mut *core::ptr::addr_of_mut!(SELECT_ALL_BUF) };
    let count = html_select_all(doc.0.raw(), b"a[href*=book]", select_buf);
    let max = if count > 0 {
        (count as usize).min(300)
    } else {
        0
    };

    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("get_manga_list", "internal_error", "payload overflow");
    }
    let mut written = 0usize;
    let mut i = 0usize;
    while i < max {
        let off = i * 4;
        if off + 4 > select_buf.len() {
            break;
        }
        let desc = i32::from_le_bytes([
            select_buf[off],
            select_buf[off + 1],
            select_buf[off + 2],
            select_buf[off + 3],
        ]);
        if desc > 0 {
            let hd: HtmlDescriptor = unsafe { core::mem::transmute(desc) };
            let mut id_buf = [0u8; 64];
            let mut title_buf = [0u8; 256];
            let mut img_buf = [0u8; 1024];
            let id = if let Ok(d) = html_select(hd, b"div.container") {
                let owned = OwnedDescriptor(d);
                attr_into(owned.0, b"id", &mut id_buf)
            } else {
                None
            };
            let title = if let Ok(d) = html_select(hd, b"div.container-title") {
                let owned = OwnedDescriptor(d);
                text_into(owned.0, &mut title_buf).map(trim_ascii)
            } else {
                None
            };
            let image = if let Ok(d) = html_select(hd, b"img") {
                let owned = OwnedDescriptor(d);
                attr_into(owned.0, b"src", &mut img_buf)
            } else {
                None
            };
            if let Some(book_id) = id {
                if valid_decimal(book_id) {
                    if written > 0 && !write_bytes(payload, &mut c, b",") {
                        return write_error("get_manga_list", "internal_error", "payload overflow");
                    }
                    let ok = write_bytes(payload, &mut c, br#"{"id":"bh3:"#)
                        && append_json_escaped(payload, &mut c, book_id)
                        && write_bytes(payload, &mut c, br#"","title":""#)
                        && append_json_escaped(payload, &mut c, title.unwrap_or(book_id))
                        && write_bytes(payload, &mut c, br#"","cover":{"kind":"url","url":""#);
                    if !ok {
                        return write_error("get_manga_list", "internal_error", "payload overflow");
                    }
                    if let Some(src) = image {
                        if !write_abs_url(payload, &mut c, src) {
                            return write_error(
                                "get_manga_list",
                                "internal_error",
                                "payload overflow",
                            );
                        }
                    }
                    let ok = write_bytes(payload, &mut c, br#""},"authors":[],"status":"unknown","contentRating":"safe","sourceTags":["bh3"]}"#);
                    if !ok {
                        return write_error("get_manga_list", "internal_error", "payload overflow");
                    }
                    written += 1;
                }
            }
            let _ = html_close(hd);
        }
        i += 1;
    }
    if !write_bytes(
        payload,
        &mut c,
        br#"],"page":{"nextCursor":null,"hasMore":false}}"#,
    ) {
        return write_error("get_manga_list", "internal_error", "payload overflow");
    }
    write_success_payload("get_manga_list", c)
}

fn run_get_home(_req: &[u8]) -> u32 {
    let payload = payload_buf();
    let data = empty_home();
    payload[..data.len()].copy_from_slice(data);
    write_success_payload("get_home", data.len())
}

fn run_get_filters(_req: &[u8]) -> u32 {
    let payload = payload_buf();
    let data = empty_filters();
    payload[..data.len()].copy_from_slice(data);
    write_success_payload("get_filters", data.len())
}

fn run_get_settings(_req: &[u8]) -> u32 {
    write_error(
        "get_settings",
        "not_supported",
        "settings are not supported",
    )
}

fn run_get_image_request(_req: &[u8]) -> u32 {
    write_error(
        "get_image_request",
        "not_supported",
        "image request is not supported",
    )
}

fn run_get_listings(_req: &[u8]) -> u32 {
    let payload = payload_buf();
    let data = empty_listings();
    payload[..data.len()].copy_from_slice(data);
    write_success_payload("get_listings", data.len())
}

koma_source_sdk::koma_source_exports!("bh3");
