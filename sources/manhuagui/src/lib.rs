#![no_std]

extern crate koma_source_sdk;

use koma_source_sdk::host::{
    html_attr, html_close, html_parse, html_text, http_request, log_info, HtmlDescriptor,
};
use koma_source_sdk::json_utils::{
    append_json_escaped, contains_bytes, extract_json_number, extract_json_string, find_subslice,
    write_bytes, write_url_encoded, write_usize,
};
use koma_source_sdk::result::ResultBuffer;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};

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

const BASE_URL: &[u8] = b"https://www.manhuagui.com";
const MOBILE_URL: &[u8] = b"https://m.manhuagui.com";
const IMAGE_CDN: &[u8] = b"https://i.hamreus.com";
koma_source_sdk::koma_source_buffers! {
    payload: 1024 * 1024,
    http_out: 2 * 1024 * 1024,
    body: 2 * 1024 * 1024,
    http_req: 2048,
    scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();
const SELECT_CAP: usize = 16000;

static mut SELECT_ALL_BUF: [u8; SELECT_CAP] = [0; SELECT_CAP];

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.manhuagui.koma",
    name: "Manhuagui",
    version: "0.1.0",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "漫画柜 HTML scraping source.",
    content_rating: "nsfw",
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
    credentials: true,
    image_request: false,
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

fn fetch_body(url: &[u8], referer: Option<&[u8]>) -> Result<usize, FetchError> {
    let req_len = build_get_request(http_req_buf(), url, referer).ok_or(FetchError::Network)?;
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
                    log_info(b"manhuagui: http transport error, retrying");
                }
            }
        }
    }
    if failed {
        return Err(FetchError::Network);
    }
    let resp = &http_out()[..resp_len];
    if !contains_bytes(resp, br#""ok":true"#) {
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
    decode_body_text(resp)
}

fn hex_value(b: u8) -> Option<u32> {
    match b {
        b'0'..=b'9' => Some((b - b'0') as u32),
        b'a'..=b'f' => Some((b - b'a' + 10) as u32),
        b'A'..=b'F' => Some((b - b'A' + 10) as u32),
        _ => None,
    }
}

fn write_utf8(dst: &mut [u8], c: &mut usize, code: u32) -> bool {
    if code < 0x80 {
        if *c >= dst.len() {
            return false;
        }
        dst[*c] = code as u8;
        *c += 1;
    } else if code < 0x800 {
        if *c + 2 > dst.len() {
            return false;
        }
        dst[*c] = 0xC0 | (code >> 6) as u8;
        dst[*c + 1] = 0x80 | (code & 0x3F) as u8;
        *c += 2;
    } else {
        if *c + 3 > dst.len() {
            return false;
        }
        dst[*c] = 0xE0 | (code >> 12) as u8;
        dst[*c + 1] = 0x80 | ((code >> 6) & 0x3F) as u8;
        dst[*c + 2] = 0x80 | (code & 0x3F) as u8;
        *c += 3;
    }
    true
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
                b'"' => {
                    if out >= dst.len() {
                        return Err(FetchError::Network);
                    }
                    dst[out] = b'"';
                    out += 1;
                    i += 2;
                }
                b'\\' => {
                    if out >= dst.len() {
                        return Err(FetchError::Network);
                    }
                    dst[out] = b'\\';
                    out += 1;
                    i += 2;
                }
                b'/' => {
                    if out >= dst.len() {
                        return Err(FetchError::Network);
                    }
                    dst[out] = b'/';
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
                        code =
                            (code << 4) | hex_value(resp[i + 2 + k]).ok_or(FetchError::Network)?;
                        k += 1;
                    }
                    if !write_utf8(dst, &mut out, code) {
                        return Err(FetchError::Network);
                    }
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

fn parse_usize(bytes: &[u8]) -> Option<usize> {
    let mut n = 0usize;
    let mut any = false;
    for &b in bytes {
        if b < b'0' || b > b'9' {
            return None;
        }
        n = n * 10 + (b - b'0') as usize;
        any = true;
    }
    if any {
        Some(n)
    } else {
        None
    }
}

fn page_from_request(req: &[u8]) -> usize {
    extract_json_number(req, b"page")
        .and_then(parse_usize)
        .unwrap_or(1)
        .max(1)
}

fn manga_slug_from_href<'a>(href: &'a [u8], out: &'a mut [u8]) -> Option<&'a [u8]> {
    let marker = b"/comic/";
    let start = if let Some(p) = find_subslice(href, marker) {
        p + 1
    } else if href.starts_with(b"comic/") {
        0
    } else {
        return None;
    };
    let mut i = start;
    let mut len = 0usize;
    while i < href.len() {
        let b = href[i];
        if b == b'?' || b == b'#' {
            break;
        }
        if b == b'/' {
            if i > start + 5 {
                if len >= out.len() {
                    return None;
                }
                out[len] = b;
                len += 1;
                break;
            }
        }
        if len >= out.len() {
            return None;
        }
        out[len] = b;
        len += 1;
        i += 1;
    }
    if len > 0 && out[len - 1] == b'/' {
        len -= 1;
    }
    if len == 0 {
        None
    } else {
        Some(&out[..len])
    }
}

fn normalize_manga_id<'a>(manga_id: &'a [u8]) -> Option<&'a [u8]> {
    if manga_id.starts_with(b"manga:") {
        Some(&manga_id[6..])
    } else if manga_id.starts_with(b"comic/") {
        Some(manga_id)
    } else if manga_id.iter().all(|b| *b >= b'0' && *b <= b'9') {
        Some(manga_id)
    } else {
        None
    }
}

fn write_abs_url(dst: &mut [u8], c: &mut usize, url: &[u8], default_base: &[u8]) -> bool {
    if url.starts_with(b"//") {
        write_bytes(dst, c, b"https:") && append_json_escaped(dst, c, url)
    } else if url.starts_with(b"http://") || url.starts_with(b"https://") {
        append_json_escaped(dst, c, url)
    } else if url.starts_with(b"/") {
        write_bytes(dst, c, default_base) && append_json_escaped(dst, c, url)
    } else {
        write_bytes(dst, c, default_base)
            && write_bytes(dst, c, b"/")
            && append_json_escaped(dst, c, url)
    }
}

fn write_card(payload: &mut [u8], c: &mut usize, id: &[u8], title: &[u8], cover: &[u8]) -> bool {
    write_bytes(payload, c, br#"{"id":""#)
        && append_json_escaped(payload, c, id)
        && write_bytes(payload, c, br#"","title":""#)
        && append_json_escaped(payload, c, trim_ascii(title))
        && write_bytes(payload, c, br#"","cover":{"kind":"url","url":""#)
        && write_abs_url(payload, c, cover, BASE_URL)
        && write_bytes(payload, c, br#""},"authors":[],"status":"unknown","contentRating":"nsfw","description":"","sourceTags":["manhuagui"]}"#)
}

fn run_search(req: &[u8]) -> u32 {
    let query = extract_json_string(req, b"query").unwrap_or(b"");
    if query.is_empty() {
        return run_get_manga_list(req);
    }
    let page = page_from_request(req);
    let mut u = 0usize;
    let url_buf = scratch_a();
    if !(write_bytes(url_buf, &mut u, BASE_URL)
        && write_bytes(url_buf, &mut u, b"/s/")
        && write_url_encoded(url_buf, &mut u, query)
        && write_bytes(url_buf, &mut u, b"_p")
        && write_usize(url_buf, &mut u, page)
        && write_bytes(url_buf, &mut u, b".html"))
    {
        return write_error("search", "internal_error", "url overflow");
    }
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), u) };
    parse_manga_cards_from_url("search", url, b"div.book-result > ul > li", true)
}

fn run_get_manga_list(req: &[u8]) -> u32 {
    let page = page_from_request(req);
    let mut u = 0usize;
    let url_buf = scratch_a();
    if !(write_bytes(url_buf, &mut u, BASE_URL)
        && write_bytes(url_buf, &mut u, b"/list/view_p")
        && write_usize(url_buf, &mut u, page)
        && write_bytes(url_buf, &mut u, b".html"))
    {
        return write_error("get_manga_list", "internal_error", "url overflow");
    }
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), u) };
    parse_manga_cards_from_url("get_manga_list", url, b"ul#contList > li", false)
}

fn run_get_listings(_req: &[u8]) -> u32 {
    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"listings":[{"id":"popular","title":"Popular","supportsLatest":false,"filters":[]}],"defaultListingId":"popular"}"#);
    if !ok {
        return write_error("get_listings", "internal_error", "payload overflow");
    }
    write_success_payload("get_listings", c)
}

fn parse_manga_cards_from_url(
    operation: &str,
    url: &[u8],
    li_selector: &[u8],
    is_search: bool,
) -> u32 {
    let html_len = match fetch_body(url, None) {
        Ok(n) => n,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error(operation, c, m);
        }
    };
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };
    let document = match html_parse(html) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error(operation, "parse_error", "html_parse failed"),
    };
    let selects = select_buf();
    let count = html_select_all(document.0.raw(), li_selector, selects);
    let max_items = if count > 0 {
        (count as usize).min(100)
    } else {
        0
    };
    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error(operation, "internal_error", "payload overflow");
    }
    let mut written = 0usize;
    for i in 0..max_items {
        let off = i * 4;
        if off + 4 > selects.len() {
            break;
        }
        let raw = i32::from_le_bytes([
            selects[off],
            selects[off + 1],
            selects[off + 2],
            selects[off + 3],
        ]);
        if raw <= 0 {
            continue;
        }
        let li: HtmlDescriptor = unsafe { core::mem::transmute(raw) };
        let mut href_buf = [0u8; 256];
        let mut title_buf = [0u8; 256];
        let mut cover_buf = [0u8; 512];
        let mut id_buf = [0u8; 64];
        let link_selector = if is_search {
            b"dl > dt > a" as &[u8]
        } else {
            b"a.bcover" as &[u8]
        };
        let link = koma_source_sdk::host::html_select(li, link_selector).ok();
        let image = koma_source_sdk::host::html_select(li, b"img").ok();
        let mut href = None;
        let mut title = None;
        if let Some(a) = link {
            href = attr_into(a, b"href", &mut href_buf);
            title = if is_search {
                text_into(a, &mut title_buf)
            } else {
                match attr_into(a, b"title", &mut title_buf) {
                    Some(v) => Some(v),
                    None => text_into(a, &mut title_buf),
                }
            };
            let _ = html_close(a);
        }
        let cover = if let Some(img) = image {
            let src = match attr_into(img, b"src", &mut cover_buf) {
                Some(v) => Some(v),
                None => attr_into(img, b"data-src", &mut cover_buf),
            };
            let _ = html_close(img);
            src
        } else {
            None
        };
        if let (Some(h), Some(t)) = (href, title) {
            if let Some(id) = manga_slug_from_href(h, &mut id_buf) {
                if written > 0 && !write_bytes(payload, &mut c, b",") {
                    let _ = html_close(li);
                    break;
                }
                if !write_card(payload, &mut c, id, t, cover.unwrap_or(b"")) {
                    let _ = html_close(li);
                    break;
                }
                written += 1;
            }
        }
        let _ = html_close(li);
    }
    let has_more = written > 0
        && (contains_bytes(html, b"_p")
            || contains_bytes(html, "下一页".as_bytes())
            || contains_bytes(html, b"next"));
    let ok = write_bytes(payload, &mut c, br#"],"page":{"nextCursor":"#)
        && if has_more {
            write_bytes(payload, &mut c, b"\"next\"")
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

fn build_mobile_manga_url(id: &[u8], out: &mut [u8]) -> Option<usize> {
    let mut u = 0usize;
    write_bytes(out, &mut u, MOBILE_URL).then_some(())?;
    if id.starts_with(b"comic/") {
        write_bytes(out, &mut u, b"/").then_some(())?;
        write_bytes(out, &mut u, id).then_some(())?;
    } else {
        write_bytes(out, &mut u, b"/comic/").then_some(())?;
        write_bytes(out, &mut u, id).then_some(())?;
    }
    if !out[..u].ends_with(b"/") {
        write_bytes(out, &mut u, b"/").then_some(())?;
    }
    Some(u)
}

fn extract_attr_after<'a>(html: &'a [u8], marker: &[u8], attr: &[u8]) -> Option<&'a [u8]> {
    let p = find_subslice(html, marker)?;
    let start = find_back(html, p, b'<').unwrap_or(p);
    let end = find_subslice(&html[p..], b">")? + p + 1;
    attr_value(&html[start..end], attr)
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

fn find_back(bytes: &[u8], mut pos: usize, needle: u8) -> Option<usize> {
    while pos > 0 {
        if bytes[pos] == needle {
            return Some(pos);
        }
        pos -= 1;
    }
    None
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

fn extract_between_text<'a>(
    html: &[u8],
    marker: &[u8],
    end_marker: &[u8],
    out: &'a mut [u8],
) -> Option<&'a [u8]> {
    let p = find_subslice(html, marker)?;
    let gt = find_subslice(&html[p..], b">")? + p + 1;
    let end = find_subslice(&html[gt..], end_marker)
        .map(|v| gt + v)
        .unwrap_or(html.len());
    Some(strip_tags_to(&html[gt..end], out))
}

fn extract_label_link_text<'a>(html: &[u8], label: &[u8], out: &'a mut [u8]) -> Option<&'a [u8]> {
    let p = find_subslice(html, label)?;
    let end = find_subslice(&html[p..], b"</span>")
        .map(|v| p + v)
        .unwrap_or((p + 500).min(html.len()));
    let slice = &html[p..end];
    let a = find_subslice(slice, b"<a")?;
    let gt = find_subslice(&slice[a..], b">")? + a + 1;
    let close = find_subslice(&slice[gt..], b"</a>")
        .map(|v| gt + v)
        .unwrap_or(slice.len());
    Some(strip_tags_to(&slice[gt..close], out))
}

fn run_get_manga(req: &[u8]) -> u32 {
    let manga_id = match extract_json_string(req, b"mangaId") {
        Some(v) => v,
        None => return write_error("get_manga", "invalid_request", "missing mangaId"),
    };
    let id = match normalize_manga_id(manga_id) {
        Some(v) => v,
        None => return write_error("get_manga", "invalid_request", "unexpected mangaId"),
    };
    let url_len = match build_mobile_manga_url(id, scratch_a()) {
        Some(n) => n,
        None => return write_error("get_manga", "internal_error", "url overflow"),
    };
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_len) };
    let html_len = match fetch_body(url, None) {
        Ok(n) => n,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_manga", c, m);
        }
    };
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };
    let mut title_buf = [0u8; 256];
    let mut desc_buf = [0u8; 4096];
    let mut author_buf = [0u8; 256];
    let mut genre_buf = [0u8; 256];
    let title =
        extract_between_text(html, b"class=\"book-title\"", b"</h1>", &mut title_buf).unwrap_or(id);
    let desc =
        extract_between_text(html, b"id=\"intro-all\"", b"</div>", &mut desc_buf).unwrap_or(b"");
    let author = extract_label_link_text(html, "漫画作者".as_bytes(), &mut author_buf);
    let genre = extract_label_link_text(html, "漫画剧情".as_bytes(), &mut genre_buf);
    let cover = extract_attr_after(html, b"class=\"hcover\"", b"src").unwrap_or(b"");
    let status = if contains_bytes(html, "已完结".as_bytes())
        || contains_bytes(html, "已完結".as_bytes())
    {
        b"completed" as &[u8]
    } else if contains_bytes(html, "连载中".as_bytes()) || contains_bytes(html, "連載中".as_bytes())
    {
        b"ongoing" as &[u8]
    } else {
        b"unknown" as &[u8]
    };
    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"manga":{"id":""#)
        && append_json_escaped(payload, &mut c, id)
        && write_bytes(payload, &mut c, br#"","title":""#)
        && append_json_escaped(payload, &mut c, title)
        && write_bytes(
            payload,
            &mut c,
            br#"","alternateTitles":[],"description":""#,
        )
        && append_json_escaped(payload, &mut c, desc)
        && write_bytes(payload, &mut c, br#"","cover":{"kind":"url","url":""#)
        && write_abs_url(payload, &mut c, cover, BASE_URL)
        && write_bytes(payload, &mut c, br#""},"authors":["#);
    if !ok {
        return write_error("get_manga", "internal_error", "payload overflow");
    }
    if let Some(a) = author {
        if !a.is_empty()
            && !(write_bytes(payload, &mut c, b"\"")
                && append_json_escaped(payload, &mut c, a)
                && write_bytes(payload, &mut c, b"\""))
        {
            return write_error("get_manga", "internal_error", "payload overflow");
        }
    }
    let ok = write_bytes(payload, &mut c, br#"],"artists":[],"status":""#)
        && write_bytes(payload, &mut c, status)
        && write_bytes(
            payload,
            &mut c,
            br#"","contentRating":"nsfw","language":"zh","tags":["#,
        );
    if !ok {
        return write_error("get_manga", "internal_error", "payload overflow");
    }
    if let Some(g) = genre {
        if !g.is_empty()
            && !(write_bytes(payload, &mut c, b"\"")
                && append_json_escaped(payload, &mut c, g)
                && write_bytes(payload, &mut c, b"\""))
        {
            return write_error("get_manga", "internal_error", "payload overflow");
        }
    }
    let ok = write_bytes(payload, &mut c, br#"],"links":[{"kind":"source","url":""#)
        && append_json_escaped(payload, &mut c, url)
        && write_bytes(payload, &mut c, br#""}]}}"#);
    if !ok {
        return write_error("get_manga", "internal_error", "payload overflow");
    }
    write_success_payload("get_manga", c)
}

fn chapter_url_from_href<'a>(href: &'a [u8], out: &'a mut [u8]) -> Option<&'a [u8]> {
    let mut c = 0usize;
    if href.starts_with(b"http://") || href.starts_with(b"https://") {
        write_bytes(out, &mut c, href).then_some(())?;
    } else if href.starts_with(b"/") {
        write_bytes(out, &mut c, BASE_URL).then_some(())?;
        write_bytes(out, &mut c, href).then_some(())?;
    } else {
        write_bytes(out, &mut c, BASE_URL).then_some(())?;
        write_bytes(out, &mut c, b"/").then_some(())?;
        write_bytes(out, &mut c, href).then_some(())?;
    }
    Some(&out[..c])
}

fn first_number(bytes: &[u8]) -> Option<&[u8]> {
    let mut i = 0usize;
    while i < bytes.len() && !(bytes[i] >= b'0' && bytes[i] <= b'9') {
        i += 1;
    }
    if i >= bytes.len() {
        return None;
    }
    let start = i;
    while i < bytes.len() && ((bytes[i] >= b'0' && bytes[i] <= b'9') || bytes[i] == b'.') {
        i += 1;
    }
    Some(&bytes[start..i])
}

fn run_get_chapters(req: &[u8]) -> u32 {
    let manga_id = match extract_json_string(req, b"mangaId") {
        Some(v) => v,
        None => return write_error("get_chapters", "invalid_request", "missing mangaId"),
    };
    let id = match normalize_manga_id(manga_id) {
        Some(v) => v,
        None => return write_error("get_chapters", "invalid_request", "unexpected mangaId"),
    };
    let url_len = match build_mobile_manga_url(id, scratch_a()) {
        Some(n) => n,
        None => return write_error("get_chapters", "internal_error", "url overflow"),
    };
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_len) };
    let html_len = match fetch_body(url, None) {
        Ok(n) => n,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_chapters", c, m);
        }
    };
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };
    let document = match html_parse(html) {
        Ok(d) => OwnedDescriptor(d),
        Err(_) => return write_error("get_chapters", "parse_error", "html_parse failed"),
    };
    let selects = select_buf();
    let count = html_select_all(
        document.0.raw(),
        b"[id^=chapter-list-] li > a.status0",
        selects,
    );
    let max_items = if count > 0 {
        (count as usize).min(1000)
    } else {
        0
    };
    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("get_chapters", "internal_error", "payload overflow");
    }
    let mut written = 0usize;
    for i in 0..max_items {
        let off = i * 4;
        if off + 4 > selects.len() {
            break;
        }
        let raw = i32::from_le_bytes([
            selects[off],
            selects[off + 1],
            selects[off + 2],
            selects[off + 3],
        ]);
        if raw <= 0 {
            continue;
        }
        let a: HtmlDescriptor = unsafe { core::mem::transmute(raw) };
        let mut href_buf = [0u8; 256];
        let mut title_buf = [0u8; 256];
        let mut abs_buf = [0u8; 512];
        let href = attr_into(a, b"href", &mut href_buf);
        let title = match attr_into(a, b"title", &mut title_buf) {
            Some(v) => Some(v),
            None => text_into(a, &mut title_buf),
        };
        if let (Some(h), Some(t)) = (href, title) {
            if let Some(ch_url) = chapter_url_from_href(h, &mut abs_buf) {
                if written > 0 && !write_bytes(payload, &mut c, b",") {
                    let _ = html_close(a);
                    break;
                }
                let ok = write_bytes(payload, &mut c, br#"{"id":""#)
                    && append_json_escaped(payload, &mut c, ch_url)
                    && write_bytes(payload, &mut c, br#"","mangaId":""#)
                    && append_json_escaped(payload, &mut c, id)
                    && write_bytes(payload, &mut c, br#"","title":""#)
                    && append_json_escaped(payload, &mut c, trim_ascii(t))
                    && write_bytes(payload, &mut c, br#"","chapterNumber":""#)
                    && append_json_escaped(payload, &mut c, first_number(t).unwrap_or(b""))
                    && write_bytes(payload, &mut c, br#"","volumeNumber":null,"language":"zh","publishedAt":null,"updatedAt":null,"pageCount":null}"#);
                if !ok {
                    let _ = html_close(a);
                    break;
                }
                written += 1;
            }
        }
        let _ = html_close(a);
    }
    if contains_bytes(html, b"__VIEWSTATE") && !contains_bytes(html, b"status0") {
        log_info(b"manhuagui: encrypted/R18 chapter data present; credentials may be required");
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

fn unescape_js_to<'a>(src: &[u8], out: &'a mut [u8]) -> &'a [u8] {
    let mut i = 0usize;
    let mut c = 0usize;
    while i < src.len() && c < out.len() {
        if src[i] == b'\\' && i + 1 < src.len() {
            match src[i + 1] {
                b'"' | b'\'' | b'\\' | b'/' => {
                    out[c] = src[i + 1];
                    c += 1;
                    i += 2;
                }
                b'n' | b'r' | b't' => {
                    out[c] = if src[i + 1] == b'n' {
                        b'\n'
                    } else if src[i + 1] == b'r' {
                        b'\r'
                    } else {
                        b'\t'
                    };
                    c += 1;
                    i += 2;
                }
                b'x' if i + 3 < src.len() => {
                    if let (Some(a), Some(b)) = (hex_value(src[i + 2]), hex_value(src[i + 3])) {
                        out[c] = ((a << 4) | b) as u8;
                        c += 1;
                        i += 4;
                    } else {
                        out[c] = src[i];
                        c += 1;
                        i += 1;
                    }
                }
                b'u' if i + 5 < src.len() => {
                    let mut code = 0u32;
                    let mut ok = true;
                    let mut k = 0usize;
                    while k < 4 {
                        if let Some(v) = hex_value(src[i + 2 + k]) {
                            code = (code << 4) | v;
                        } else {
                            ok = false;
                            break;
                        }
                        k += 1;
                    }
                    if ok && write_utf8(out, &mut c, code) {
                        i += 6;
                    } else {
                        out[c] = src[i];
                        c += 1;
                        i += 1;
                    }
                }
                _ => {
                    out[c] = src[i + 1];
                    c += 1;
                    i += 2;
                }
            }
        } else {
            out[c] = src[i];
            c += 1;
            i += 1;
        }
    }
    &out[..c]
}

fn find_json_value_end(bytes: &[u8], start: usize) -> usize {
    let mut i = start;
    let mut in_str = false;
    let mut esc = false;
    let mut depth = 0i32;
    while i < bytes.len() {
        let b = bytes[i];
        if in_str {
            if esc {
                esc = false;
            } else if b == b'\\' {
                esc = true;
            } else if b == b'"' {
                in_str = false;
            }
        } else if b == b'"' {
            in_str = true;
        } else if b == b'[' || b == b'{' {
            depth += 1;
        } else if b == b']' || b == b'}' {
            depth -= 1;
            if depth <= 0 {
                return i + 1;
            }
        } else if b == b';' && depth <= 0 {
            return i;
        }
        i += 1;
    }
    bytes.len()
}

fn find_img_json(html: &[u8]) -> Option<&[u8]> {
    let marker = b"\"files\"";
    let files = find_subslice(html, marker)?;
    let mut start = files;
    while start > 0 && html[start] != b'{' {
        start -= 1;
    }
    if html[start] != b'{' {
        return None;
    }
    let end = find_json_value_end(html, start);
    Some(&html[start..end])
}

fn extract_json_array<'a>(json: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let p = find_subslice(json, key)?;
    let mut i = p + key.len();
    while i < json.len() && json[i] != b'[' {
        i += 1;
    }
    if i >= json.len() {
        return None;
    }
    let start = i;
    let end = find_json_value_end(json, start);
    Some(&json[start..end])
}

fn extract_sl_value<'a>(json: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let sl = find_subslice(json, b"\"sl\"")?;
    let part = &json[sl..];
    if key == b"e" {
        extract_json_number(part, b"e")
    } else {
        extract_json_string(part, key)
    }
}

fn run_get_pages(req: &[u8]) -> u32 {
    let chapter_id = match extract_json_string(req, b"chapterId") {
        Some(v) => v,
        None => return write_error("get_pages", "invalid_request", "missing chapterId"),
    };
    let mut u = 0usize;
    let url_buf = scratch_a();
    if chapter_id.starts_with(b"http://") || chapter_id.starts_with(b"https://") {
        if !write_bytes(url_buf, &mut u, chapter_id) {
            return write_error("get_pages", "internal_error", "url overflow");
        }
    } else if chapter_id.starts_with(b"/") {
        if !(write_bytes(url_buf, &mut u, BASE_URL) && write_bytes(url_buf, &mut u, chapter_id)) {
            return write_error("get_pages", "internal_error", "url overflow");
        }
    } else {
        if !(write_bytes(url_buf, &mut u, BASE_URL)
            && write_bytes(url_buf, &mut u, b"/")
            && write_bytes(url_buf, &mut u, chapter_id))
        {
            return write_error("get_pages", "internal_error", "url overflow");
        }
    }
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), u) };
    let html_len = match fetch_body(url, Some(BASE_URL)) {
        Ok(n) => n,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_pages", c, m);
        }
    };
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };
    let decoded_ptr;
    let decoded_len;
    {
        let decoded = unescape_js_to(html, scratch_b());
        decoded_len = decoded.len();
        decoded_ptr = decoded.as_ptr();
    }
    let decoded = unsafe { core::slice::from_raw_parts(decoded_ptr, decoded_len) };
    let img_json = find_img_json(decoded).or_else(|| find_img_json(html));
    let json = match img_json {
        Some(v) => v,
        None => return write_error("get_pages", "parse_error", "image data not found"),
    };
    let files = match extract_json_array(json, b"\"files\"") {
        Some(v) => v,
        None => return write_error("get_pages", "parse_error", "files not found"),
    };
    let path = extract_json_string(json, b"path").unwrap_or(b"");
    let e = extract_sl_value(json, b"e").unwrap_or(b"");
    let m = extract_sl_value(json, b"m").unwrap_or(b"");
    let payload = payload_buf();
    let mut c = 0usize;
    if !(write_bytes(payload, &mut c, br#"{"chapterId":""#)
        && append_json_escaped(payload, &mut c, chapter_id)
        && write_bytes(payload, &mut c, br#"","pages":["#))
    {
        return write_error("get_pages", "internal_error", "payload overflow");
    }
    let mut i = 0usize;
    let mut index = 0usize;
    while i < files.len() {
        if files[i] != b'"' {
            i += 1;
            continue;
        }
        i += 1;
        let start = i;
        let mut esc = false;
        while i < files.len() {
            if esc {
                esc = false;
            } else if files[i] == b'\\' {
                esc = true;
            } else if files[i] == b'"' {
                break;
            }
            i += 1;
        }
        if i >= files.len() {
            break;
        }
        let file = &files[start..i];
        if index > 0 && !write_bytes(payload, &mut c, b",") {
            break;
        }
        let ok = write_bytes(payload, &mut c, br#"{"id":"page:"#)
            && append_json_escaped(payload, &mut c, chapter_id)
            && write_bytes(payload, &mut c, b":")
            && write_usize(payload, &mut c, index)
            && write_bytes(payload, &mut c, br#"","index":"#)
            && write_usize(payload, &mut c, index)
            && write_bytes(payload, &mut c, br#","image":{"kind":"url","url":""#)
            && write_bytes(payload, &mut c, IMAGE_CDN)
            && append_json_escaped(payload, &mut c, path)
            && append_json_escaped(payload, &mut c, file);
        if !ok {
            break;
        }
        if !e.is_empty() || !m.is_empty() {
            if !(write_bytes(payload, &mut c, b"?e=")
                && append_json_escaped(payload, &mut c, e)
                && write_bytes(payload, &mut c, b"&m=")
                && append_json_escaped(payload, &mut c, m))
            {
                break;
            }
        }
        if !write_bytes(payload, &mut c, br#""}}"#) {
            break;
        }
        index += 1;
        i += 1;
        if index >= 500 {
            break;
        }
    }
    if index == 0 {
        return write_error("get_pages", "parse_error", "no pages extracted");
    }
    if !write_bytes(payload, &mut c, b"]}") {
        return write_error("get_pages", "internal_error", "payload overflow");
    }
    write_success_payload("get_pages", c)
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
    log_info(b"manhuagui search");
    run_search(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga", "invalid_request", "empty request"),
    };
    log_info(b"manhuagui get_manga");
    run_get_manga(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_chapters(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_chapters", "invalid_request", "empty request"),
    };
    log_info(b"manhuagui get_chapters");
    run_get_chapters(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_pages(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_pages", "invalid_request", "empty request"),
    };
    log_info(b"manhuagui get_pages");
    run_get_pages(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_listings(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_listings", "invalid_request", "empty request"),
    };
    log_info(b"manhuagui get_listings");
    run_get_listings(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga_list(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga_list", "invalid_request", "empty request"),
    };
    log_info(b"manhuagui get_manga_list");
    run_get_manga_list(req)
}

#[no_mangle]
pub extern "C" fn koma_source_free(result_ptr: u32) {
    response_buffer().free(result_ptr)
}
