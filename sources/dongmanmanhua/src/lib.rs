#![no_std]

extern crate koma_source_sdk;

use koma_source_sdk::host::{
    self, html_attr, html_close, html_parse, html_select, html_text, http_request, log_info,
    HtmlDescriptor,
};
use koma_source_sdk::json_utils::{
    append_json_escaped, contains_bytes, extract_json_string, find_subslice, write_bytes,
    write_url_encoded, write_usize,
};
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{build_get_request, decode_json_body_into, fetch_error_code, FetchError};

// html_select_all host import
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

static mut SELECT_ALL_BUF: [u8; 16000] = [0; 16000]; // 4000 * 4 bytes

// Static buffers for get_manga detail parsing (needed for lifetime reasons)
static mut DETAIL_TITLE_BUF: [u8; 512] = [0; 512];
static mut DETAIL_AUTHOR_BUF: [u8; 256] = [0; 256];
static mut DETAIL_DESC_BUF: [u8; 2048] = [0; 2048];
static mut DETAIL_STATUS_BUF: [u8; 128] = [0; 128];
static mut DETAIL_COVER_BUF: [u8; 1024] = [0; 1024];
// Static buffer for chapter name parsing
static mut CHAPTER_NAME_BUF: [u8; 512] = [0; 512];

const SITE_BASE: &[u8] = b"https://www.dongmanmanhua.cn";
koma_source_sdk::koma_source_buffers! {
    payload: 1024 * 1024,
    http_out: 2 * 1024 * 1024,
    body: 2 * 1024 * 1024,
    http_req: 2048,
    scratch: 4096,
}
koma_source_sdk::koma_source_helpers!();

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "cn.dongmanmanhua.koma",
    name: "咚漫 (DongmanManhua)",
    version: "0.1.0",
    api_version: "0.2",
    language: "zh-Hans",
    author: "Koma",
    description: "DongmanManhua (dongmanmanhua.cn) HTML scraping source.",
    content_rating: "safe",
};

const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: false,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: true,
    manga_list: true,
    home: false,
    filters: false,
    settings: false,
    image_request: true,
    credentials: false,
};

fn detail_title_buf() -> &'static mut [u8] {
    unsafe { &mut *core::ptr::addr_of_mut!(DETAIL_TITLE_BUF) }
}
fn detail_author_buf() -> &'static mut [u8] {
    unsafe { &mut *core::ptr::addr_of_mut!(DETAIL_AUTHOR_BUF) }
}
fn detail_desc_buf() -> &'static mut [u8] {
    unsafe { &mut *core::ptr::addr_of_mut!(DETAIL_DESC_BUF) }
}
fn detail_status_buf() -> &'static mut [u8] {
    unsafe { &mut *core::ptr::addr_of_mut!(DETAIL_STATUS_BUF) }
}
fn detail_cover_buf() -> &'static mut [u8] {
    unsafe { &mut *core::ptr::addr_of_mut!(DETAIL_COVER_BUF) }
}
fn chapter_name_buf() -> &'static mut [u8] {
    unsafe { &mut *core::ptr::addr_of_mut!(CHAPTER_NAME_BUF) }
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

fn fetch_html(url: &[u8]) -> Result<usize, FetchError> {
    let req_len =
        build_get_request(http_req_buf(), url, Some(SITE_BASE), &[]).ok_or(FetchError::Network)?;
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
                    log_info(b"dongmanmanhua: http transport error, retrying");
                }
            }
        }
    }
    if transport_failed {
        return Err(FetchError::Network);
    }
    decode_json_body_into(&http_out()[..resp_len], body_buf())
}

/// Extract the path from a full URL or protocol-relative URL, stripping the domain prefix.
/// Handles: "https://www.dongmanmanhua.cn/PATH" and "//www.dongmanmanhua.cn/PATH"
fn path_from_url(url: &[u8]) -> Option<&[u8]> {
    if url.starts_with(b"//") {
        // protocol-relative: //domain/path
        let after_dslash = 2usize;
        let path_start = find_subslice(&url[after_dslash..], b"/")? + after_dslash;
        Some(&url[path_start..])
    } else if let Some(pos) = find_subslice(url, b"://") {
        // full URL: https://domain/path
        let proto_end = pos + 3;
        let path_start = find_subslice(&url[proto_end..], b"/")? + proto_end;
        Some(&url[path_start..])
    } else {
        // already a relative path
        Some(url)
    }
}

// ─── search ────────────────────────────────────────────────────────────────

fn run_search(_req: &[u8]) -> u32 {
    // Search is JS-rendered; curl returns 0 bytes. Return empty result.
    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(
        payload,
        &mut c,
        br#"{"items":[],"page":{"nextCursor":null,"hasMore":false}}"#,
    ) {
        return write_error("search", "internal_error", "payload overflow");
    }
    write_success_payload("search", c)
}

// ─── get_manga ─────────────────────────────────────────────────────────────

fn run_get_manga(req: &[u8]) -> u32 {
    let manga_id = match extract_json_string(req, b"mangaId") {
        Some(v) => v,
        None => return write_error("get_manga", "invalid_request", "missing mangaId"),
    };
    let prefix = b"manga:";
    if manga_id.len() <= prefix.len() || &manga_id[..prefix.len()] != prefix {
        return write_error("get_manga", "invalid_request", "unexpected mangaId");
    }
    let path = &manga_id[prefix.len()..];

    let url_buf = scratch_a();
    let mut url_cursor = 0usize;
    if !(write_bytes(url_buf, &mut url_cursor, SITE_BASE)
        && write_bytes(url_buf, &mut url_cursor, path))
    {
        return write_error("get_manga", "internal_error", "url overflow");
    }
    let url_bytes = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_cursor) };

    let html_len = match fetch_html(url_bytes) {
        Ok(n) => n,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_manga", c, m);
        }
    };
    let html_bytes = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };

    let document = match html_parse(html_bytes) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error("get_manga", "parse_error", "html_parse failed"),
    };

    // Title: h1.subj or h3.subj
    let title_text = {
        let buf = detail_title_buf();
        if let Ok(d) = html_select(document.0, b"h1.subj") {
            let owned = OwnedDescriptor(d);
            text_into(owned.0, buf).map(|s| trim_ascii(s))
        } else if let Ok(d) = html_select(document.0, b"h3.subj") {
            let owned = OwnedDescriptor(d);
            text_into(owned.0, buf).map(|s| trim_ascii(s))
        } else {
            None
        }
    };

    // Author: .author_area or .detail_header .info .author
    let author_text = {
        let buf = detail_author_buf();
        if let Ok(d) = html_select(document.0, b".author_area") {
            let owned = OwnedDescriptor(d);
            text_into(owned.0, buf).map(|s| trim_ascii(s))
        } else if let Ok(d) = html_select(document.0, b".author") {
            let owned = OwnedDescriptor(d);
            text_into(owned.0, buf).map(|s| trim_ascii(s))
        } else {
            None
        }
    };

    // Description: #_asideDetail p.summary or p.summary
    let desc_text = {
        let buf = detail_desc_buf();
        if let Ok(d) = html_select(document.0, b"#_asideDetail p.summary") {
            let owned = OwnedDescriptor(d);
            text_into(owned.0, buf).map(|s| trim_ascii(s))
        } else if let Ok(d) = html_select(document.0, b"p.summary") {
            let owned = OwnedDescriptor(d);
            text_into(owned.0, buf).map(|s| trim_ascii(s))
        } else {
            None
        }
    };

    // Status: #_asideDetail p.day_info or p.day_info
    let status_text = {
        let buf = detail_status_buf();
        if let Ok(d) = html_select(document.0, b"#_asideDetail p.day_info") {
            let owned = OwnedDescriptor(d);
            text_into(owned.0, buf).map(|s| trim_ascii(s))
        } else if let Ok(d) = html_select(document.0, b"p.day_info") {
            let owned = OwnedDescriptor(d);
            text_into(owned.0, buf).map(|s| trim_ascii(s))
        } else {
            None
        }
    };

    let status = match status_text {
        Some(s) if contains_bytes(s, "完结".as_bytes()) => "completed",
        Some(s) if contains_bytes(s, "更新".as_bytes()) => "ongoing",
        _ => "unknown",
    };

    // Cover: div.detail_header span.thmb img
    let cover_url = {
        let buf = detail_cover_buf();
        if let Ok(header) = html_select(document.0, b"div.detail_header") {
            let header_owned = OwnedDescriptor(header);
            if let Ok(d) = html_select(header_owned.0, b"span.thmb img") {
                let owned = OwnedDescriptor(d);
                if let Some(u) = attr_into(owned.0, b"src", buf) {
                    Some(u)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    };

    let payload = payload_buf();
    let mut c = 0usize;

    let ok = write_bytes(payload, &mut c, br#"{"manga":{"id":"manga:"#)
        && append_json_escaped(payload, &mut c, path)
        && write_bytes(payload, &mut c, br#"","title":""#)
        && append_json_escaped(payload, &mut c, title_text.unwrap_or(b""))
        && write_bytes(
            payload,
            &mut c,
            br#"","alternateTitles":[],"description":""#,
        )
        && append_json_escaped(payload, &mut c, desc_text.unwrap_or(b""))
        && write_bytes(payload, &mut c, br#"","cover":{"kind":"url","url":""#)
        && append_json_escaped(payload, &mut c, cover_url.unwrap_or(b""))
        && write_bytes(payload, &mut c, br#""},"authors":["#);

    if !ok {
        return write_error("get_manga", "internal_error", "payload overflow");
    }

    if let Some(author) = author_text {
        if !append_json_escaped(payload, &mut c, author) {
            return write_error("get_manga", "internal_error", "payload overflow");
        }
    }

    let ok = write_bytes(payload, &mut c, br#""],"artists":[],"status":""#)
        && write_bytes(payload, &mut c, status.as_bytes())
        && write_bytes(
            payload,
            &mut c,
            br#"","contentRating":"safe","language":"zh-Hans","tags":["dongmanmanhua"],"links":[]}}"#,
        );

    if !ok {
        return write_error("get_manga", "internal_error", "payload overflow");
    }

    write_success_payload("get_manga", c)
}

// ─── get_chapters ──────────────────────────────────────────────────────────

fn run_get_chapters(req: &[u8]) -> u32 {
    let manga_id = match extract_json_string(req, b"mangaId") {
        Some(v) => v,
        None => return write_error("get_chapters", "invalid_request", "missing mangaId"),
    };
    let prefix = b"manga:";
    if manga_id.len() <= prefix.len() || &manga_id[..prefix.len()] != prefix {
        return write_error("get_chapters", "invalid_request", "unexpected mangaId");
    }
    let path = &manga_id[prefix.len()..];

    let url_buf = scratch_a();
    let mut url_cursor = 0usize;
    if !(write_bytes(url_buf, &mut url_cursor, SITE_BASE)
        && write_bytes(url_buf, &mut url_cursor, path))
    {
        return write_error("get_chapters", "internal_error", "url overflow");
    }
    let url_bytes = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_cursor) };

    let html_len = match fetch_html(url_bytes) {
        Ok(n) => n,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_chapters", c, m);
        }
    };
    let html_bytes = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };

    let document = match html_parse(html_bytes) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error("get_chapters", "parse_error", "html_parse failed"),
    };

    let select_buf = unsafe { &mut *core::ptr::addr_of_mut!(SELECT_ALL_BUF) };
    let count = html_select_all(document.0.raw(), b"ul#_listUl li", select_buf);

    let payload = payload_buf();
    let mut c = 0usize;

    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("get_chapters", "internal_error", "payload overflow");
    }

    let max_items = if count > 0 { count as usize } else { 0 };
    let max_items = if max_items > 2000 { 2000 } else { max_items };
    let mut written = 0usize;

    for i in 0..max_items {
        let offset = i * 4;
        if offset + 4 > select_buf.len() {
            break;
        }
        let desc_raw = i32::from_le_bytes([
            select_buf[offset],
            select_buf[offset + 1],
            select_buf[offset + 2],
            select_buf[offset + 3],
        ]);
        if desc_raw < 0 {
            continue;
        }

        let li: HtmlDescriptor = unsafe { core::mem::transmute(desc_raw) };

        let a_el = html_select(li, b"a");
        let (href_opt, name_opt) = if let Ok(a) = a_el {
            let a_owned = OwnedDescriptor(a);
            let href_scratch = scratch_a();
            let href = attr_into(a_owned.0, b"href", href_scratch);

            let name = {
                let name_buf = chapter_name_buf();
                if let Ok(sd) = html_select(a_owned.0, b"span.subj span") {
                    let s_owned = OwnedDescriptor(sd);
                    text_into(s_owned.0, name_buf).map(|s| trim_ascii(s))
                } else if let Ok(sd) = html_select(a_owned.0, b"span.subj") {
                    let s_owned = OwnedDescriptor(sd);
                    text_into(s_owned.0, name_buf).map(|s| trim_ascii(s))
                } else {
                    None
                }
            };
            (href, name)
        } else {
            (None, None)
        };

        if let (Some(href), Some(name)) = (href_opt, name_opt) {
            let chapter_path = path_from_url(href).unwrap_or(href);
            let chapter_id_start = find_subslice(chapter_path, b"/")
                .map(|i| i + 1)
                .unwrap_or(0);
            let chapter_slug = &chapter_path[chapter_id_start..];

            let ch_num = extract_number_from_end(chapter_slug).unwrap_or(b"1");

            if written > 0 {
                if !write_bytes(payload, &mut c, b",") {
                    break;
                }
            }
            let ok = write_bytes(payload, &mut c, br#"{"id":"chapter:"#)
                && append_json_escaped(payload, &mut c, manga_id)
                && write_bytes(payload, &mut c, b":")
                && append_json_escaped(payload, &mut c, chapter_slug)
                && write_bytes(payload, &mut c, br#"","mangaId":""#)
                && append_json_escaped(payload, &mut c, manga_id)
                && write_bytes(payload, &mut c, br#"","title":""#)
                && append_json_escaped(payload, &mut c, name)
                && write_bytes(payload, &mut c, br#"","chapterNumber":""#)
                && append_json_escaped(payload, &mut c, ch_num)
                && write_bytes(
                    payload,
                    &mut c,
                    br#"","volumeNumber":null,"language":"zh-Hans","publishedAt":null,"updatedAt":null,"pageCount":0}"#,
                );
            if !ok {
                break;
            }
            written += 1;
        }
        let _ = html_close(li);
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

fn extract_number_from_end(s: &[u8]) -> Option<&[u8]> {
    let mut end = s.len();
    while end > 0 && !(s[end - 1] >= b'0' && s[end - 1] <= b'9') {
        end -= 1;
    }
    if end == 0 {
        return None;
    }
    let mut start = end;
    while start > 0 && s[start - 1] >= b'0' && s[start - 1] <= b'9' {
        start -= 1;
    }
    if start == end {
        return None;
    }
    Some(&s[start..end])
}

// ─── get_pages ─────────────────────────────────────────────────────────────

fn run_get_pages(req: &[u8]) -> u32 {
    let chapter_id = match extract_json_string(req, b"chapterId") {
        Some(v) => v,
        None => return write_error("get_pages", "invalid_request", "missing chapterId"),
    };
    let prefix = b"chapter:";
    if chapter_id.len() <= prefix.len() || &chapter_id[..prefix.len()] != prefix {
        return write_error("get_pages", "invalid_request", "unexpected chapterId");
    }
    let rest = &chapter_id[prefix.len()..];

    let manga_prefix = b"manga:";
    if rest.len() <= manga_prefix.len() || &rest[..manga_prefix.len()] != manga_prefix {
        return write_error(
            "get_pages",
            "invalid_request",
            "unexpected chapterId format",
        );
    }
    let after_manga = &rest[manga_prefix.len()..];
    let colon_pos = match find_subslice(after_manga, b":") {
        Some(p) => p,
        None => return write_error("get_pages", "invalid_request", "malformed chapterId"),
    };
    let chapter_slug = &after_manga[colon_pos + 1..];

    let url_buf = scratch_a();
    let mut url_cursor = 0usize;
    if !(write_bytes(url_buf, &mut url_cursor, SITE_BASE)
        && write_bytes(url_buf, &mut url_cursor, b"/")
        && write_bytes(url_buf, &mut url_cursor, chapter_slug))
    {
        return write_error("get_pages", "internal_error", "url overflow");
    }
    let url_bytes = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_cursor) };

    let html_len = match fetch_html(url_bytes) {
        Ok(n) => n,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_pages", c, m);
        }
    };
    let html_bytes = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };

    let document = match html_parse(html_bytes) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error("get_pages", "parse_error", "html_parse failed"),
    };

    let select_buf = unsafe { &mut *core::ptr::addr_of_mut!(SELECT_ALL_BUF) };
    let count = html_select_all(document.0.raw(), b"div#_imageList img", select_buf);

    let payload = payload_buf();
    let mut c = 0usize;

    if !write_bytes(payload, &mut c, br#"{"chapterId":""#)
        || !append_json_escaped(payload, &mut c, chapter_id)
        || !write_bytes(payload, &mut c, br#"","pages":["#)
    {
        return write_error("get_pages", "internal_error", "payload overflow");
    }

    let max_items = if count > 0 { count as usize } else { 0 };
    let max_items = if max_items > 1000 { 1000 } else { max_items };

    for i in 0..max_items {
        let offset = i * 4;
        if offset + 4 > select_buf.len() {
            break;
        }
        let desc = i32::from_le_bytes([
            select_buf[offset],
            select_buf[offset + 1],
            select_buf[offset + 2],
            select_buf[offset + 3],
        ]);
        if desc < 0 {
            continue;
        }

        let hd: HtmlDescriptor = unsafe { core::mem::transmute(desc) };
        let mut img_url_buf = [0u8; 2048];
        // Try data-url first, then src
        let img_url = if let Some(u) = attr_into(hd, b"data-url", &mut img_url_buf) {
            Some(u)
        } else {
            attr_into(hd, b"src", &mut img_url_buf)
        };

        if let Some(url) = img_url {
            if i > 0 {
                if !write_bytes(payload, &mut c, b",") {
                    break;
                }
            }
            let ok = write_bytes(payload, &mut c, br#"{"id":"page:"#)
                && write_usize(payload, &mut c, i)
                && write_bytes(payload, &mut c, br#"","index":"#)
                && write_usize(payload, &mut c, i)
                && write_bytes(payload, &mut c, br#","image":{"kind":"url","url":""#)
                && append_json_escaped(payload, &mut c, url)
                && write_bytes(payload, &mut c, br#""}}"#);
            if !ok {
                break;
            }
        }
        let _ = html_close(hd);
    }

    if !write_bytes(payload, &mut c, br#"]}"#) {
        return write_error("get_pages", "internal_error", "payload overflow");
    }

    write_success_payload("get_pages", c)
}

// ─── get_listings ──────────────────────────────────────────────────────────

fn run_get_listings(_req: &[u8]) -> u32 {
    let payload = payload_buf();
    let mut c = 0usize;
    let name = "每日连载".as_bytes();
    let desc = "咚漫每日连载漫画".as_bytes();
    let ok = write_bytes(
        payload,
        &mut c,
        br#"{"listings":[{"id":"dailySchedule","name":""#,
    ) && append_json_escaped(payload, &mut c, name)
        && write_bytes(payload, &mut c, br#"","description":""#)
        && append_json_escaped(payload, &mut c, desc)
        && write_bytes(payload, &mut c, br#""}]}"#);
    if !ok {
        return write_error("get_listings", "internal_error", "payload overflow");
    }
    write_success_payload("get_listings", c)
}

// ─── get_manga_list ────────────────────────────────────────────────────────

fn run_get_manga_list(req: &[u8]) -> u32 {
    let listing_id = extract_json_string(req, b"listingId");

    if listing_id.is_none() || listing_id != Some(b"dailySchedule") {
        let payload = payload_buf();
        let mut c = 0usize;
        if !write_bytes(
            payload,
            &mut c,
            br#"{"items":[],"page":{"nextCursor":null,"hasMore":false}}"#,
        ) {
            return write_error("get_manga_list", "internal_error", "payload overflow");
        }
        return write_success_payload("get_manga_list", c);
    }

    let url_buf = scratch_a();
    let mut url_cursor = 0usize;
    if !write_bytes(url_buf, &mut url_cursor, SITE_BASE)
        || !write_bytes(url_buf, &mut url_cursor, b"/dailySchedule")
    {
        return write_error("get_manga_list", "internal_error", "url overflow");
    }
    let url_bytes = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_cursor) };

    let html_len = match fetch_html(url_bytes) {
        Ok(n) => n,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_manga_list", c, m);
        }
    };
    let html_bytes = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };

    let document = match html_parse(html_bytes) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error("get_manga_list", "parse_error", "html_parse failed"),
    };

    let select_buf = unsafe { &mut *core::ptr::addr_of_mut!(SELECT_ALL_BUF) };
    let count = html_select_all(
        document.0.raw(),
        b"a.daily_card_item",
        select_buf,
    );

    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("get_manga_list", "internal_error", "payload overflow");
    }

    let mut written = 0usize;
    let max_items = if count > 0 { count as usize } else { 0 };
    let max_items = if max_items > 500 { 500 } else { max_items };

    for i in 0..max_items {
        let offset = i * 4;
        if offset + 4 > select_buf.len() {
            break;
        }
        let desc = i32::from_le_bytes([
            select_buf[offset],
            select_buf[offset + 1],
            select_buf[offset + 2],
            select_buf[offset + 3],
        ]);
        if desc < 0 {
            continue;
        }

        let hd: HtmlDescriptor = unsafe { core::mem::transmute(desc) };
        let href_scratch = scratch_a();
        let title_scratch = scratch_b();
        let href = attr_into(hd, b"href", href_scratch);
        let title_el = html_select(hd, b"p.subj");
        let title_text = if let Ok(td) = title_el {
            let owned = OwnedDescriptor(td);
            text_into(owned.0, title_scratch).map(|s| trim_ascii(s))
        } else {
            None
        };

        let mut cover_buf = [0u8; 1024];
        let cover_url = if let Ok(img) = html_select(hd, b"img") {
            let owned = OwnedDescriptor(img);
            attr_into(owned.0, b"src", &mut cover_buf)
        } else {
            None
        };

        if let (Some(href), Some(title)) = (href, title_text) {
            let manga_path = path_from_url(href).unwrap_or(href);
            if written > 0 {
                if !write_bytes(payload, &mut c, b",") {
                    break;
                }
            }
            let ok = write_bytes(payload, &mut c, br#"{"id":"manga:"#)
                && append_json_escaped(payload, &mut c, manga_path)
                && write_bytes(payload, &mut c, br#"","title":""#)
                && append_json_escaped(payload, &mut c, title)
                && write_bytes(payload, &mut c, br#"","cover":{"kind":"url","url":""#)
                && append_json_escaped(payload, &mut c, cover_url.unwrap_or(b""))
                && write_bytes(
                    payload,
                    &mut c,
                    br#""},"authors":[],"status":"unknown","contentRating":"safe","description":"","sourceTags":["dongmanmanhua"]}"#,
                );
            if !ok {
                break;
            }
            written += 1;
        }
        let _ = html_close(hd);
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

// ─── WASM exports ──────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn koma_source_init(_manifest_ptr: u32, manifest_len: u32) -> i32 {
    log_info(b"dongmanmanhua source init");
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
    log_info(b"dongmanmanhua search");
    run_search(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga", "invalid_request", "empty request"),
    };
    log_info(b"dongmanmanhua get_manga");
    run_get_manga(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_chapters(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_chapters", "invalid_request", "empty request"),
    };
    log_info(b"dongmanmanhua get_chapters");
    run_get_chapters(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_pages(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_pages", "invalid_request", "empty request"),
    };
    log_info(b"dongmanmanhua get_pages");
    run_get_pages(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_listings(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_listings", "invalid_request", "empty request"),
    };
    log_info(b"dongmanmanhua get_listings");
    run_get_listings(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga_list(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga_list", "invalid_request", "empty request"),
    };
    log_info(b"dongmanmanhua get_manga_list");
    run_get_manga_list(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_home(_req_ptr: u32, _req_len: u32) -> u32 {
    write_error("get_home", "unimplemented", "not implemented")
}

#[no_mangle]
pub extern "C" fn koma_source_get_filters(_req_ptr: u32, _req_len: u32) -> u32 {
    write_error("get_filters", "unimplemented", "not implemented")
}

#[no_mangle]
pub extern "C" fn koma_source_get_settings(_req_ptr: u32, _req_len: u32) -> u32 {
    write_error("get_settings", "unimplemented", "not implemented")
}

#[no_mangle]
pub extern "C" fn koma_source_get_image_request(_req_ptr: u32, _req_len: u32) -> u32 {
    write_error("get_image_request", "unimplemented", "not implemented")
}

#[no_mangle]
pub extern "C" fn koma_source_free(result_ptr: u32) {
    response_buffer().free(result_ptr)
}
