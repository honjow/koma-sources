#![no_std]

extern crate koma_source_sdk;

use koma_source_sdk::host;
use koma_source_sdk::json_utils::{
    append_json_escaped, append_json_unescaped_then_escaped, extract_json_string, find_subslice,
    json_object_member, request_page_number, write_bytes, write_url_encoded, write_usize,
};
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{build_get_request, FetchError};

const BASE: &[u8] = b"https://www.webtoons.com";
const MOBILE: &[u8] = b"https://m.webtoons.com";
// Keep the language preference only; never pre-accept an age or consent gate.
const EN_COOKIE: &[u8] = b"locale=en";
const SEARCH_REFERER: &[u8] = b"https://www.webtoons.com/en/";
const MOBILE_REFERER: &[u8] = b"https://m.webtoons.com/";
const IMAGE_PAGE_CAP: usize = 512;
// The public all-types search page returns twelve cards per page (observed on
// page 1, page 2, and the final page); the server's section count is total rows.
const SEARCH_PAGE_SIZE: usize = 12;

koma_source_sdk::koma_source_buffers! {
    payload: 384 * 1024,
    http_out: 2 * 1024 * 1024,
    body: 2 * 1024 * 1024,
    http_req: 16 * 1024,
    scratch: 16 * 1024,
}
koma_source_sdk::koma_source_helpers!();

static mut SELECT_BUFFER: [u8; 4096] = [0; 4096];

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.webtoons.koma",
    name: "WEBTOON (English)",
    version: "0.1.4",
    api_version: "0.2",
    language: "en",
    author: "Koma source contributors",
    description: "Public English WEBTOON and CANVAS search and reader source.",
    content_rating: "unknown",
};

const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: true,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: false,
    manga_list: false,
    home: false,
    filters: false,
    settings: false,
    credentials: false,
    image_request: true,
};

#[link(wasm_import_module = "koma_host")]
extern "C" {
    #[link_name = "html_select_all"]
    fn host_html_select_all(
        descriptor: i32,
        selector_ptr: *const u8,
        selector_len: u32,
        out_ptr: *mut u8,
        out_cap: u32,
    ) -> i32;
}

fn html_select_all(descriptor: i32, selector: &[u8], output: &mut [u8]) -> i32 {
    unsafe {
        host_html_select_all(
            descriptor,
            selector.as_ptr(),
            selector.len() as u32,
            output.as_mut_ptr(),
            output.len() as u32,
        )
    }
}

fn close_html_descriptors(descriptors: &[u8], start: usize, end: usize) {
    for index in start..end {
        let offset = index * 4;
        let raw = i32::from_le_bytes([
            descriptors[offset],
            descriptors[offset + 1],
            descriptors[offset + 2],
            descriptors[offset + 3],
        ]);
        if raw > 0 {
            let _ = host::html_close(host::HtmlDescriptor::from_raw(raw));
        }
    }
}

fn request(url: &[u8], referer: &[u8]) -> Result<usize, FetchError> {
    let req_len = build_get_request(
        http_req_buf(),
        url,
        Some(referer),
        &[
            (b"Cookie", EN_COOKIE),
            (b"Accept-Language", b"en-US,en;q=0.9"),
        ],
    )
    .ok_or(FetchError::Network)?;
    let response_len = host::http_request(&http_req_buf()[..req_len], http_out())
        .map_err(|_| FetchError::Network)?;
    decode_json_body(&http_out()[..response_len])
}

fn append_slice<'a>(out: &'a mut [u8], cursor: &mut usize, bytes: &[u8]) -> bool {
    write_bytes(out, cursor, bytes)
}

fn attr(descriptor: host::HtmlDescriptor, key: &[u8], dst: &mut [u8]) -> Option<usize> {
    host::html_attr(descriptor, key, dst).ok()
}

fn text(descriptor: host::HtmlDescriptor, dst: &mut [u8]) -> Option<usize> {
    host::html_text(descriptor, dst).ok()
}

fn element_text(parent: host::HtmlDescriptor, selector: &[u8], dst: &mut [u8]) -> Option<usize> {
    let selected = host::html_select(parent, selector).ok()?;
    let value = text(selected, dst);
    let _ = host::html_close(selected);
    value
}

fn digits(bytes: &[u8]) -> bool {
    !bytes.is_empty() && bytes.iter().all(u8::is_ascii_digit)
}

fn host_matches_domain(host: &[u8], domain: &[u8]) -> bool {
    host.eq_ignore_ascii_case(domain)
        || (host.len() > domain.len()
            && host[host.len() - domain.len() - 1] == b'.'
            && host[host.len() - domain.len()..].eq_ignore_ascii_case(domain))
}

fn allowed_image_url(url: &[u8]) -> bool {
    let Some(rest) = url.strip_prefix(b"https://") else {
        return false;
    };
    let authority_end = rest
        .iter()
        .position(|byte| matches!(byte, b'/' | b'?' | b'#'))
        .unwrap_or(rest.len());
    let mut authority = &rest[..authority_end];
    if authority.is_empty() || authority.contains(&b'@') {
        return false;
    }
    // Only accept the normal HTTPS port; do not interpret credentials or
    // arbitrary port text as part of an allow-listed hostname.
    if authority.ends_with(b":443") {
        authority = &authority[..authority.len() - 4];
    } else if authority.contains(&b':') {
        return false;
    }
    host_matches_domain(authority, b"pstatic.net")
        || host_matches_domain(authority, b"webtoons.com")
}

// Parse the whole decimal value, without prefix parsing or saturation.
fn strict_usize(bytes: &[u8]) -> Option<usize> {
    let bytes = trim_ascii(bytes);
    if !digits(bytes) { return None; }
    let mut value = 0usize;
    for &byte in bytes {
        value = value.checked_mul(10)?.checked_add((byte - b'0') as usize)?;
    }
    Some(value)
}

fn episode_string<'a>(episode: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let raw = json_object_member(episode, key)?;
    if raw.len() < 2 || raw.first() != Some(&b'"') || raw.last() != Some(&b'"') {
        return None;
    }
    Some(&raw[1..raw.len() - 1])
}

// Decode a JSON string's contents without allocating a title-sized buffer.
// This is metadata interpretation only; the existing title output and required
// record checks remain unchanged.
#[derive(Clone, Copy)]
struct TitleChars<'a> { raw: &'a str, pos: usize, failed: bool }
impl<'a> TitleChars<'a> {
    fn new(raw: &'a [u8]) -> Option<Self> {
        Some(Self { raw: core::str::from_utf8(raw).ok()?, pos: 0, failed: false })
    }
    fn hex4(&mut self) -> Option<u32> {
        let bytes = self.raw.as_bytes().get(self.pos..self.pos + 4)?;
        let mut n = 0;
        for &b in bytes {
            n = n * 16 + match b {
                b'0'..=b'9' => (b - b'0') as u32,
                b'a'..=b'f' => (b - b'a' + 10) as u32,
                b'A'..=b'F' => (b - b'A' + 10) as u32,
                _ => return None,
            };
        }
        self.pos += 4;
        Some(n)
    }
    fn next(&mut self) -> Option<char> {
        if self.failed || self.pos == self.raw.len() { return None; }
        let result = (|| {
            let c = self.raw[self.pos..].chars().next()?;
            self.pos += c.len_utf8();
            if c != '\\' { return (c >= ' ' && c != '"').then_some(c); }
            let escape = *self.raw.as_bytes().get(self.pos)?;
            self.pos += 1;
            Some(match escape {
                b'"' => '"', b'\\' => '\\', b'/' => '/',
                b'b' => '\u{8}', b'f' => '\u{c}', b'n' => '\n', b'r' => '\r', b't' => '\t',
                b'u' => {
                    let mut cp = self.hex4()?;
                    if (0xd800..=0xdbff).contains(&cp) {
                        if self.raw.as_bytes().get(self.pos..self.pos + 2)? != b"\\u" { return None; }
                        self.pos += 2;
                        let low = self.hex4()?;
                        if !(0xdc00..=0xdfff).contains(&low) { return None; }
                        cp = 0x10000 + ((cp - 0xd800) << 10) + low - 0xdc00;
                    }
                    char::from_u32(cp)?
                }
                _ => return None,
            })
        })();
        if result.is_none() { self.failed = true; }
        result
    }
    fn peek(&self) -> Option<char> { let mut copy = *self; copy.next() }
    fn spaces(&mut self) {
        while self.peek().map(|c| c.is_ascii_whitespace()).unwrap_or(false) { self.next(); }
    }
}

fn title_number_span(title: &[u8]) -> Option<&[u8]> {
    let mut scan = TitleChars::new(title)?;
    let mut valid = scan;
    while valid.next().is_some() {}
    if valid.failed { return None; }
    let mut word_before = false;
    while let Some(c) = scan.peek() {
        if !word_before {
            // Explicit English episode/chapter markers from the primary
            // consumer, not arbitrary digits or the internal episode number.
            for marker in ["episode", "chapter", "ep", "ch"] {
                let mut cur = scan;
                if !marker.chars().all(|m| cur.next().map(|x| x.eq_ignore_ascii_case(&m)).unwrap_or(false)) { continue; }
                cur.spaces();
                if cur.peek() == Some('.') { cur.next(); }
                cur.spaces();
                let start = cur.pos;
                if !cur.peek().map(|x| x.is_ascii_digit()).unwrap_or(false) { continue; }
                while cur.peek().map(|x| x.is_ascii_digit()).unwrap_or(false) { cur.next(); }
                if cur.peek() == Some('.') {
                    cur.next();
                    if !cur.peek().map(|x| x.is_ascii_digit()).unwrap_or(false) { continue; }
                    while cur.peek().map(|x| x.is_ascii_digit()).unwrap_or(false) { cur.next(); }
                }
                if cur.peek().map(|x| x.is_alphanumeric() || x == '_' || x == '.').unwrap_or(false) { continue; }
                return title.get(start..cur.pos);
            }
        }
        scan.next();
        word_before = c.is_alphanumeric() || c == '_';
    }
    None
}

fn append_title_chapter_number(out: &mut [u8], cursor: &mut usize, title: &[u8]) -> bool {
    match title_number_span(title) {
        Some(number) => write_bytes(out, cursor, b"\"")
            && append_json_unescaped_then_escaped(out, cursor, number)
            && write_bytes(out, cursor, b"\""),
        None => write_bytes(out, cursor, b"null"),
    }
}

// Only objects are supported episode records. Distinguish a normal array end
// from an unknown item, missing separator, trailing comma or unfinished object.
struct EpisodeArray<'a> { data: &'a [u8], pos: usize, first: bool, ended: bool }
impl<'a> EpisodeArray<'a> {
    fn new(data: &'a [u8]) -> Option<Self> {
        let data = trim_ascii(data);
        if data.first() != Some(&b'[') { return None; }
        Some(Self { data, pos: 1, first: true, ended: false })
    }
    fn skip_space(&mut self) {
        while self.pos < self.data.len() && self.data[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }
    fn next(&mut self) -> Result<Option<&'a [u8]>, ()> {
        if self.ended { return Ok(None); }
        self.skip_space();
        if self.data.get(self.pos) == Some(&b']') {
            self.pos += 1;
            self.skip_space();
            if self.pos != self.data.len() { return Err(()); }
            self.ended = true;
            return Ok(None);
        }
        if !self.first {
            if self.data.get(self.pos) != Some(&b',') { return Err(()); }
            self.pos += 1;
            self.skip_space();
        }
        if self.data.get(self.pos) != Some(&b'{') { return Err(()); }
        self.first = false;
        let start = self.pos;
        let mut depth = 0usize;
        let mut quoted = false;
        while self.pos < self.data.len() {
            let byte = self.data[self.pos];
            if quoted {
                if byte == b'\\' {
                    self.pos += 1;
                    if self.pos >= self.data.len() { return Err(()); }
                } else if byte == b'"' { quoted = false; }
            } else {
                match byte {
                    b'"' => quoted = true,
                    b'{' | b'[' => depth += 1,
                    b'}' | b']' => {
                        if depth == 0 { return Err(()); }
                        depth -= 1;
                        if depth == 0 {
                            if byte != b'}' { return Err(()); }
                            self.pos += 1;
                            return Ok(Some(&self.data[start..self.pos]));
                        }
                    },
                    _ => {},
                }
            }
            self.pos += 1;
        }
        Err(())
    }
}

fn parse_usize(bytes: &[u8]) -> usize {
    let mut value = 0usize;
    for &byte in bytes {
        if !byte.is_ascii_digit() {
            break;
        }
        value = value
            .saturating_mul(10)
            .saturating_add((byte - b'0') as usize);
    }
    value
}

fn number_after<'a>(url: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let key_pos = find_subslice(url, key)? + key.len();
    let start = key_pos;
    let mut end = start;
    while end < url.len() && url[end].is_ascii_digit() {
        end += 1;
    }
    (end > start).then_some(&url[start..end])
}

fn normalize_url(url: &[u8], output: &mut [u8]) -> Option<usize> {
    let mut c = 0;
    if url.starts_with(b"https://") || url.starts_with(b"http://") {
        append_slice(output, &mut c, url).then_some(())?;
    } else if url.starts_with(b"//") {
        append_slice(output, &mut c, b"https:").then_some(())?;
        append_slice(output, &mut c, url).then_some(())?;
    } else if url.starts_with(b"/") {
        append_slice(output, &mut c, BASE).then_some(())?;
        append_slice(output, &mut c, url).then_some(())?;
    } else {
        append_slice(output, &mut c, BASE).then_some(())?;
        append_slice(output, &mut c, b"/").then_some(())?;
        append_slice(output, &mut c, url).then_some(())?;
    }
    Some(c)
}

// Extract the path below /en/ from a source page URL and preserve it verbatim.
fn route_from_url(url: &[u8]) -> Option<&[u8]> {
    let marker = find_subslice(url, b"/en/")? + 4;
    let mut end = marker;
    while end < url.len() && url[end] != b'?' && url[end] != b'#' {
        end += 1;
    }
    let path = &url[marker..end];
    if path.is_empty() {
        None
    } else {
        Some(path)
    }
}

fn title_number_from_url(url: &[u8]) -> Option<&[u8]> {
    let value = number_after(url, b"title_no=")?;
    digits(value).then_some(value)
}

fn manga_id_parts(id: &[u8]) -> Option<(&[u8], &[u8])> {
    let rest = id.strip_prefix(b"manga:webtoons:")?;
    let split = rest.iter().position(|b| *b == b':')?;
    let title_no = &rest[..split];
    let route = &rest[split + 1..];
    if !digits(title_no) || route.is_empty() || !route.ends_with(b"/list") {
        return None;
    }
    Some((title_no, route))
}

fn chapter_id_parts(id: &[u8]) -> Option<(&[u8], &[u8], &[u8])> {
    let rest = id.strip_prefix(b"chapter:webtoons:")?;
    let mut split = rest.splitn(3, |b| *b == b':');
    let title_no = split.next()?;
    let episode_no = split.next()?;
    let route = split.next()?;
    if !digits(title_no) || !digits(episode_no) || route.is_empty() || !route.ends_with(b"/list") {
        return None;
    }
    Some((title_no, episode_no, route))
}

fn viewer_url(
    title_no: &[u8],
    episode_no: &[u8],
    route: &[u8],
    output: &mut [u8],
) -> Option<usize> {
    let base_path = route.strip_suffix(b"/list")?;
    let mut c = 0;
    append_slice(output, &mut c, BASE).then_some(())?;
    append_slice(output, &mut c, b"/en/").then_some(())?;
    append_slice(output, &mut c, base_path).then_some(())?;
    append_slice(output, &mut c, b"/episode/viewer?title_no=").then_some(())?;
    append_slice(output, &mut c, title_no).then_some(())?;
    append_slice(output, &mut c, b"&episode_no=").then_some(())?;
    append_slice(output, &mut c, episode_no).then_some(())?;
    Some(c)
}

fn search_url(query: &[u8], page: usize, output: &mut [u8]) -> Option<usize> {
    let mut c = 0;
    append_slice(output, &mut c, BASE).then_some(())?;
    append_slice(output, &mut c, b"/en/search?keyword=").then_some(())?;
    write_url_encoded(output, &mut c, query).then_some(())?;
    append_slice(output, &mut c, b"&page=").then_some(())?;
    write_usize(output, &mut c, page).then_some(())?;
    Some(c)
}

fn write_search_item(out: &mut [u8], cursor: &mut usize, item: host::HtmlDescriptor) -> bool {
    let mut href = [0u8; 2048];
    let Some(href_len) = attr(item, b"href", &mut href) else {
        return false;
    };
    let mut absolute_href = [0u8; 2048];
    let Some(absolute_href_len) = normalize_url(&href[..href_len], &mut absolute_href) else {
        return false;
    };
    let href = &absolute_href[..absolute_href_len];
    let Some(title_no) = title_number_from_url(href) else {
        return false;
    };
    let Some(route) = route_from_url(href) else {
        return false;
    };

    let mut title = [0u8; 1024];
    let Some(title_len) =
        element_text(item, b".title", &mut title).or_else(|| text(item, &mut title))
    else {
        return false;
    };
    let mut author = [0u8; 512];
    let author_len = element_text(item, b".author", &mut author).unwrap_or(0);

    let mut cover = [0u8; 2048];
    let cover_len = if let Ok(img) = host::html_select(item, b"img") {
        let mut n = attr(img, b"src", &mut cover).or_else(|| attr(img, b"data-src", &mut cover));
        let _ = host::html_close(img);
        if n.unwrap_or(0) == 0 {
            if let Ok(img) = host::html_select(item, b"img") {
                n = attr(img, b"data-src", &mut cover);
                let _ = host::html_close(img);
            }
        }
        n.unwrap_or(0)
    } else {
        0
    };
    let mut absolute_cover = [0u8; 2048];
    let absolute_cover_len = if cover_len > 0 {
        normalize_url(&cover[..cover_len], &mut absolute_cover).unwrap_or(0)
    } else {
        0
    };

    append_slice(out, cursor, br#"{"id":"manga:webtoons:"#)
        && append_json_escaped(out, cursor, title_no)
        && append_slice(out, cursor, b":")
        && append_json_escaped(out, cursor, route)
        && append_slice(out, cursor, br#"","title":""#)
        && append_json_escaped(out, cursor, &title[..title_len])
        && append_slice(out, cursor, br#"","cover":{"kind":"url","url":""#)
        && append_json_escaped(out, cursor, &absolute_cover[..absolute_cover_len])
        && append_slice(out, cursor, b"\"},\"authors\":[\"")
        && append_json_escaped(out, cursor, &author[..author_len])
        && append_slice(
            out,
            cursor,
            b"\"],\"status\":\"unknown\",\"contentRating\":\"unknown\",\"sourceTags\":[]}",
        )
}

fn run_search(req: &[u8]) -> u32 {
    let query = extract_json_string(req, b"query").unwrap_or(b"");
    let page = request_page_number(req);
    let mut url = [0u8; 8192];
    let Some(url_len) = search_url(query, page, &mut url) else {
        return write_error("search", "invalid_request", "search URL is too long");
    };
    let body_len = match request(&url[..url_len], SEARCH_REFERER) {
        Ok(n) => n,
        Err(e) => {
            return write_error(
                "search",
                koma_source_sdk::fetch_error_code(e).0,
                match e {
                    FetchError::NotFound => "search page not found",
                    FetchError::RateLimit => "provider rate limited search",
                    FetchError::ClientError => "provider rejected search request",
                    FetchError::ServerError => "provider search service failed",
                    _ => "unable to fetch WEBTOON search page",
                },
            )
        }
    };
    let doc = match host::html_parse(&body_buf()[..body_len]) {
        Ok(doc) => doc,
        Err(_) => {
            return write_error(
                "search",
                "parse_error",
                "WEBTOON search HTML could not be parsed",
            )
        }
    };
    let out = payload_buf();
    let mut c = 0;
    let mut count = 0usize;
    if !write_bytes(out, &mut c, br#"{"items":["#) {
        let _ = host::html_close(doc);
        return write_error("search", "internal_error", "response buffer overflow");
    }
    let selected_count = html_select_all(doc.raw(), b".webtoon_list li a", unsafe {
        &mut *core::ptr::addr_of_mut!(SELECT_BUFFER)
    });
    if selected_count < 0 {
        let _ = host::html_close(doc);
        return write_error(
            "search",
            "parse_error",
            "WEBTOON search results could not be selected",
        );
    }
    let descriptors = unsafe { &*core::ptr::addr_of!(SELECT_BUFFER) };
    if selected_count as usize > descriptors.len() / 4 {
        let _ = host::html_close(doc);
        return write_error(
            "search",
            "result_limit",
            "WEBTOON search returned more card descriptors than this source can safely process",
        );
    }
    let max = selected_count as usize;
    let mut serialization_failed = false;
    let mut failed_at = 0usize;
    let mut out_of_room = false;
    for i in 0..max {
        let off = i * 4;
        let raw = i32::from_le_bytes([
            descriptors[off],
            descriptors[off + 1],
            descriptors[off + 2],
            descriptors[off + 3],
        ]);
        if raw <= 0 {
            serialization_failed = true;
            failed_at = i + 1;
            break;
        }
        let item = host::HtmlDescriptor::from_raw(raw);
        let item_start = c;
        if count > 0 && !write_bytes(out, &mut c, b",") {
            let _ = host::html_close(item);
            serialization_failed = true;
            failed_at = i + 1;
            out_of_room = true;
            break;
        }
        if write_search_item(out, &mut c, item) {
            count += 1;
        } else {
            c = item_start;
            serialization_failed = true;
            failed_at = i + 1;
            let _ = host::html_close(item);
            break;
        }
        let _ = host::html_close(item);
    }
    if serialization_failed {
        for i in failed_at..max {
            let off = i * 4;
            let raw = i32::from_le_bytes([
                descriptors[off],
                descriptors[off + 1],
                descriptors[off + 2],
                descriptors[off + 3],
            ]);
            if raw > 0 {
                let _ = host::html_close(host::HtmlDescriptor::from_raw(raw));
            }
        }
        let _ = host::html_close(doc);
        return write_error(
            "search",
            if out_of_room {
                "result_limit"
            } else {
                "parse_error"
            },
            if out_of_room {
                "WEBTOON search response exceeds the source output buffer"
            } else {
                "a WEBTOON search result could not be parsed safely"
            },
        );
    }
    // The all-types search page has no pagination anchors; it provides a
    // section total and 12 visible cards while accepting `page=N` directly.
    let total_items = element_text(doc, b".series_count .number", scratch_a())
        .and_then(|n| strict_usize(&scratch_a()[..n]));
    let total_items = match total_items {
        Some(total) => total,
        None if count == 0 => 0,
        None => {
            let _ = host::html_close(doc);
            return write_error("search", "parse_error", "WEBTOON search total could not be read; refusing to claim a terminal page");
        }
    };
    let has_next = count > 0 && total_items > page.saturating_mul(SEARCH_PAGE_SIZE);
    let _ = host::html_close(doc);
    if !write_bytes(out, &mut c, br#"],"page":{"nextCursor":"#) {
        return write_error("search", "internal_error", "response buffer overflow");
    }
    if has_next {
        if !write_bytes(out, &mut c, b"\"")
            || !write_usize(out, &mut c, page.saturating_add(1))
            || !write_bytes(out, &mut c, b"\"")
        {
            return write_error("search", "internal_error", "response buffer overflow");
        }
    } else if !write_bytes(out, &mut c, b"null") {
        return write_error("search", "internal_error", "response buffer overflow");
    }
    if !write_bytes(
        out,
        &mut c,
        if has_next {
            br#","hasMore":true}}"#
        } else {
            br#","hasMore":false}}"#
        },
    ) {
        return write_error("search", "internal_error", "response buffer overflow");
    }
    write_success_payload("search", c)
}

fn run_get_manga(req: &[u8]) -> u32 {
    let id = extract_json_string(req, b"mangaId").unwrap_or(b"");
    let Some((title_no, route)) = manga_id_parts(id) else {
        return write_error(
            "get_manga",
            "invalid_id",
            "expected WEBTOON manga ID with title number and route",
        );
    };
    let mut url = [0u8; 4096];
    let mut uc = 0;
    if !append_slice(&mut url, &mut uc, BASE)
        || !append_slice(&mut url, &mut uc, b"/en/")
        || !append_slice(&mut url, &mut uc, route)
        || !append_slice(&mut url, &mut uc, b"?title_no=")
        || !append_slice(&mut url, &mut uc, title_no)
    {
        return write_error("get_manga", "invalid_id", "WEBTOON route is too long");
    }
    let body_len = match request(&url[..uc], SEARCH_REFERER) {
        Ok(n) => n,
        Err(error) => {
            let (code, message) = koma_source_sdk::fetch_error_code(error);
            return write_error("get_manga", code, message);
        }
    };
    let doc = match host::html_parse(&body_buf()[..body_len]) {
        Ok(doc) => doc,
        Err(_) => {
            return write_error(
                "get_manga",
                "parse_error",
                "WEBTOON detail HTML could not be parsed",
            )
        }
    };
    let mut title = [0u8; 1024];
    let title_len = element_text(doc, b"h1.subj, h3.subj", &mut title).unwrap_or(0);
    if title_len == 0 {
        let _ = host::html_close(doc);
        return write_error(
            "get_manga",
            "not_found",
            "public WEBTOON title was not found",
        );
    }
    let title_text = trim_ascii(&title[..title_len]);
    let mut author = [0u8; 512];
    let author_len = element_text(
        doc,
        b".detail_header .info .author:nth-of-type(1)",
        &mut author,
    )
    .or_else(|| element_text(doc, b".detail_header .info .author_area", &mut author))
    .unwrap_or(0);
    let author_text = trim_ascii(&author[..author_len]);
    let mut description = [0u8; 4096];
    let desc_len = element_text(doc, b"#_asideDetail p.summary", &mut description).unwrap_or(0);
    let description_text = trim_ascii(&description[..desc_len]);
    let mut cover = [0u8; 2048];
    let mut cover_len =
        if let Ok(meta) = host::html_select(doc, b"head meta[property=\"og:image\"]") {
            let n = attr(meta, b"content", &mut cover).unwrap_or(0);
            let _ = host::html_close(meta);
            n
        } else {
            0
        };
    if cover_len == 0 {
        if let Ok(img) = host::html_select(doc, b".detail_header .thmb img") {
            cover_len = attr(img, b"src", &mut cover).unwrap_or(0);
            let _ = host::html_close(img);
        }
    }
    let mut status = [0u8; 128];
    let status_len = element_text(doc, b"#_asideDetail p.day_info", &mut status).unwrap_or(0);
    let status_text = &status[..status_len];
    let status_value = if find_subslice(status_text, b"COMPLETED").is_some()
        || find_subslice(status_text, b"END").is_some()
    {
        b"completed".as_slice()
    } else if find_subslice(status_text, b"UP").is_some()
        || find_subslice(status_text, b"EVERY").is_some()
    {
        b"ongoing".as_slice()
    } else {
        b"unknown".as_slice()
    };
    let _ = host::html_close(doc);
    let mut absolute_cover = [0u8; 2048];
    let absolute_cover_len = if cover_len > 0 {
        normalize_url(&cover[..cover_len], &mut absolute_cover).unwrap_or(0)
    } else {
        0
    };

    let out = payload_buf();
    let mut c = 0;
    let prefix_ok = write_bytes(out, &mut c, br#"{"manga":{"id":""#)
        && append_json_escaped(out, &mut c, id)
        && write_bytes(out, &mut c, br#"","title":""#)
        && append_json_escaped(out, &mut c, title_text);
    let cover_ok = if absolute_cover_len > 0 {
        write_bytes(out, &mut c, br#"","cover":{"kind":"url","url":""#)
            && append_json_escaped(out, &mut c, &absolute_cover[..absolute_cover_len])
            && write_bytes(out, &mut c, br#""},"authors":[""#)
    } else {
        write_bytes(out, &mut c, br#"","cover":{"kind":"none"},"authors":[""#)
    };
    let ok = prefix_ok
        && cover_ok
        && append_json_escaped(out, &mut c, author_text)
        && write_bytes(out, &mut c, br#""],"status":""#)
        && append_json_escaped(out, &mut c, status_value)
        && write_bytes(
            out,
            &mut c,
            br#"","contentRating":"unknown","description":""#,
        )
        && append_json_escaped(out, &mut c, description_text)
        && write_bytes(
            out,
            &mut c,
            br#"","tags":[],"links":[{"kind":"source","url":""#,
        )
        && append_json_escaped(out, &mut c, &url[..uc])
        && write_bytes(out, &mut c, br#""}]}}"#);
    if !ok {
        return write_error("get_manga", "internal_error", "response buffer overflow");
    }
    write_success_payload("get_manga", c)
}

fn run_get_chapters(req: &[u8]) -> u32 {
    let id = extract_json_string(req, b"mangaId").unwrap_or(b"");
    let Some((title_no, route)) = manga_id_parts(id) else {
        return write_error(
            "get_chapters",
            "invalid_id",
            "expected WEBTOON manga ID with title number and route",
        );
    };
    let kind = if route.starts_with(b"canvas/") {
        b"canvas".as_slice()
    } else {
        b"webtoon".as_slice()
    };
    let requested_cursor = json_object_member(req, b"args")
        .and_then(|args| json_object_member(args, b"page"))
        .and_then(|page| json_object_member(page, b"cursor"));
    let cursor_bytes = match requested_cursor {
        None | Some(b"null") => b"0".as_slice(),
        Some(value) => {
            let raw = if value.len() >= 2 && value[0] == b'"' && value[value.len() - 1] == b'"' {
                &value[1..value.len() - 1]
            } else {
                value
            };
            if !digits(raw) {
                return write_error(
                    "get_chapters",
                    "invalid_cursor",
                    "expected a numeric WEBTOON episode cursor",
                );
            }
            raw
        }
    };
    let cursor_value = parse_usize(cursor_bytes);
    let mut url = [0u8; 2048];
    let mut uc = 0;
    let mut ok = append_slice(&mut url, &mut uc, MOBILE)
        && append_slice(&mut url, &mut uc, b"/api/v1/")
        && append_slice(&mut url, &mut uc, kind)
        && append_slice(&mut url, &mut uc, b"/")
        && append_slice(&mut url, &mut uc, title_no)
        && append_slice(&mut url, &mut uc, b"/episodes?pageSize=100");
    if ok && kind == b"canvas" {
        ok = append_slice(&mut url, &mut uc, b"&readingLanguageCode=en");
    }
    if ok && cursor_value > 0 {
        ok = append_slice(&mut url, &mut uc, b"&cursor=")
            && append_slice(&mut url, &mut uc, cursor_bytes);
    }
    if !ok {
        return write_error("get_chapters", "invalid_id", "WEBTOON title ID is too long");
    }
    let body_len = match request(&url[..uc], MOBILE_REFERER) {
        Ok(n) => n,
        Err(error) => {
            let (code, message) = koma_source_sdk::fetch_error_code(error);
            return write_error("get_chapters", code, message);
        }
    };
    let body = &body_buf()[..body_len];
    let result = match json_object_member(body, b"result") {
        Some(value) => value,
        None => {
            return write_error(
                "get_chapters",
                "parse_error",
                "WEBTOON episode response has no result object",
            )
        }
    };
    let next_cursor_raw = match json_object_member(result, b"nextCursor") {
        Some(value) => value,
        None => {
            return write_error(
                "get_chapters",
                "parse_error",
                "WEBTOON episode response has no pagination cursor",
            )
        }
    };
    let next_cursor_bytes = if next_cursor_raw == b"null" {
        b"0".as_slice()
    } else {
        let raw = if next_cursor_raw.len() >= 2
            && next_cursor_raw[0] == b'"'
            && next_cursor_raw[next_cursor_raw.len() - 1] == b'"'
        {
            &next_cursor_raw[1..next_cursor_raw.len() - 1]
        } else {
            next_cursor_raw
        };
        if !digits(raw) {
            return write_error(
                "get_chapters",
                "parse_error",
                "WEBTOON episode cursor is malformed",
            );
        }
        raw
    };
    let next_cursor = parse_usize(next_cursor_bytes);
    let mut episodes = match json_object_member(result, b"episodeList").and_then(EpisodeArray::new) {
        Some(it) => it,
        None => {
            return write_error(
                "get_chapters",
                "parse_error",
                "WEBTOON episode response has no episodeList",
            )
        }
    };
    let out = payload_buf();
    let mut c = 0;
    if !write_bytes(out, &mut c, br#"{"items":["#) {
        return write_error("get_chapters", "internal_error", "response buffer overflow");
    }
    let mut count = 0usize;
    loop {
        let episode = match episodes.next() {
            Ok(Some(episode)) => episode,
            Ok(None) => break,
            Err(()) => return write_error("get_chapters", "parse_error", "WEBTOON episodeList contains an unsupported or incomplete record; no partial list was returned"),
        };
        if count >= 100 {
            return write_error(
                "get_chapters",
                "pagination_error",
                "WEBTOON returned more than pageSize=100 episodes; refusing to emit a partial page",
            );
        }
        let Some(viewer_link) = episode_string(episode, b"viewerLink") else {
            return write_error(
                "get_chapters",
                "parse_error",
                "episode record lacks viewerLink; refusing to return a partial chapter list",
            );
        };
        let Some(episode_no) = number_after(viewer_link, b"episode_no=") else {
            return write_error(
                "get_chapters",
                "parse_error",
                "episode viewerLink lacks a stable episode number",
            );
        };
        let Some(title) = episode_string(episode, b"episodeTitle") else {
            return write_error(
                "get_chapters",
                "parse_error",
                "episode record lacks episodeTitle; refusing to return a partial chapter list",
            );
        };
        if count > 0 && !write_bytes(out, &mut c, b",") {
            return write_error("get_chapters", "internal_error", "response buffer overflow");
        }
        let item_ok = write_bytes(out, &mut c, br#"{"id":"chapter:webtoons:"#)
            && append_json_escaped(out, &mut c, title_no)
            && write_bytes(out, &mut c, b":")
            && append_json_escaped(out, &mut c, episode_no)
            && write_bytes(out, &mut c, b":")
            && append_json_escaped(out, &mut c, route)
            && write_bytes(out, &mut c, br#"","mangaId":""#)
            && append_json_escaped(out, &mut c, id)
            && write_bytes(out, &mut c, br#"","title":""#)
            && append_json_unescaped_then_escaped(out, &mut c, title)
            && write_bytes(out, &mut c, br#"","chapterNumber":"#)
            && append_title_chapter_number(out, &mut c, title)
            && write_bytes(out, &mut c, br#","language":"en"}"#);
        if !item_ok {
            return write_error(
                "get_chapters",
                "internal_error",
                "chapter list exceeds the source response buffer; no partial list was returned",
            );
        }
        count += 1;
    }
    if count == 0 && next_cursor != 0 {
        return write_error(
            "get_chapters",
            "pagination_error",
            "WEBTOON returned an empty page with a nonterminal cursor",
        );
    }
    if next_cursor != 0 && next_cursor <= cursor_value {
        return write_error(
            "get_chapters",
            "pagination_error",
            "WEBTOON returned a repeated or backward episode cursor",
        );
    }
    if !write_bytes(out, &mut c, br#"],"page":{"nextCursor":"#) {
        return write_error("get_chapters", "internal_error", "response buffer overflow");
    }
    if next_cursor == 0 {
        if !write_bytes(out, &mut c, b"null") {
            return write_error("get_chapters", "internal_error", "response buffer overflow");
        }
    } else if !write_bytes(out, &mut c, b"\"")
        || !append_json_escaped(out, &mut c, next_cursor_bytes)
        || !write_bytes(out, &mut c, b"\"")
    {
        return write_error("get_chapters", "internal_error", "response buffer overflow");
    }
    if !write_bytes(
        out,
        &mut c,
        if next_cursor == 0 {
            br#", "hasMore":false}}"#
        } else {
            br#", "hasMore":true}}"#
        },
    ) {
        return write_error("get_chapters", "internal_error", "response buffer overflow");
    }
    write_success_payload("get_chapters", c)
}

fn run_get_pages(req: &[u8]) -> u32 {
    let id = extract_json_string(req, b"chapterId").unwrap_or(b"");
    let Some((title_no, episode_no, route)) = chapter_id_parts(id) else {
        return write_error(
            "get_pages",
            "invalid_id",
            "expected WEBTOON chapter ID with stable title and episode numbers",
        );
    };
    let mut viewer = [0u8; 4096];
    let Some(viewer_len) = viewer_url(title_no, episode_no, route, &mut viewer) else {
        return write_error(
            "get_pages",
            "invalid_id",
            "unsupported WEBTOON viewer route",
        );
    };
    let body_len = match request(&viewer[..viewer_len], SEARCH_REFERER) {
        Ok(n) => n,
        Err(FetchError::NotFound) => {
            return write_error(
                "get_pages",
                "not_found",
                "public WEBTOON episode is unavailable",
            )
        }
        Err(error) => {
            let (code, message) = koma_source_sdk::fetch_error_code(error);
            return write_error("get_pages", code, message);
        }
    };
    let doc = match host::html_parse(&body_buf()[..body_len]) {
        Ok(doc) => doc,
        Err(_) => {
            return write_error(
                "get_pages",
                "parse_error",
                "WEBTOON episode HTML could not be parsed",
            )
        }
    };
    let out = payload_buf();
    let mut c = 0;
    if !write_bytes(out, &mut c, br#"{"pages":["#) {
        let _ = host::html_close(doc);
        return write_error("get_pages", "internal_error", "response buffer overflow");
    }
    let count = html_select_all(doc.raw(), b"div#_imageList > img", unsafe {
        &mut *core::ptr::addr_of_mut!(SELECT_BUFFER)
    });
    let descriptors = unsafe { &*core::ptr::addr_of!(SELECT_BUFFER) };
    if count < 0 {
        let _ = host::html_close(doc);
        return write_error(
            "get_pages",
            "parse_error",
            "WEBTOON episode images could not be selected",
        );
    }
    let descriptor_capacity = descriptors.len() / 4;
    if count as usize > IMAGE_PAGE_CAP || count as usize > descriptor_capacity {
        close_html_descriptors(descriptors, 0, (count as usize).min(descriptor_capacity));
        let _ = host::html_close(doc);
        return write_error("get_pages", "page_limit", "WEBTOON episode image count exceeds this source's safe output limit; no partial page list was returned");
    }
    let max = if count > 0 { count as usize } else { 0 };
    let mut written = 0usize;
    for i in 0..max {
        let off = i * 4;
        let raw = i32::from_le_bytes([
            descriptors[off],
            descriptors[off + 1],
            descriptors[off + 2],
            descriptors[off + 3],
        ]);
        if raw <= 0 {
            close_html_descriptors(descriptors, i + 1, max);
            let _ = host::html_close(doc);
            return write_error(
                "get_pages",
                "parse_error",
                "WEBTOON image descriptor is invalid; refusing a partial page list",
            );
        }
        let img = host::HtmlDescriptor::from_raw(raw);
        let mut image = [0u8; 4096];
        let image_len = attr(img, b"data-url", &mut image)
            .filter(|n| *n > 0)
            .or_else(|| attr(img, b"src", &mut image))
            .unwrap_or(0);
        let _ = host::html_close(img);
        if image_len == 0 {
            close_html_descriptors(descriptors, i + 1, max);
            let _ = host::html_close(doc);
            return write_error(
                "get_pages",
                "parse_error",
                "WEBTOON image record has no usable URL; refusing a partial page list",
            );
        }
        let mut absolute = [0u8; 4096];
        let Some(absolute_len) = normalize_url(&image[..image_len], &mut absolute) else {
            close_html_descriptors(descriptors, i + 1, max);
            let _ = host::html_close(doc);
            return write_error(
                "get_pages",
                "parse_error",
                "WEBTOON image URL could not be normalized; refusing a partial page list",
            );
        };
        if written > 0 && !write_bytes(out, &mut c, b",") {
            close_html_descriptors(descriptors, i + 1, max);
            let _ = host::html_close(doc);
            return write_error("get_pages", "internal_error", "response buffer overflow");
        }
        let page_id_ok = write_bytes(out, &mut c, br#"{"id":"page:webtoons:"#)
            && append_json_escaped(out, &mut c, title_no)
            && write_bytes(out, &mut c, b":")
            && append_json_escaped(out, &mut c, episode_no)
            && write_bytes(out, &mut c, b":")
            && write_usize(out, &mut c, written)
            && write_bytes(out, &mut c, b":")
            && append_json_escaped(out, &mut c, route)
            && write_bytes(out, &mut c, br#"","index":"#)
            && write_usize(out, &mut c, written)
            && write_bytes(out, &mut c, b",\"image\":{\"kind\":\"url\",\"url\":\"")
            && append_json_escaped(out, &mut c, &absolute[..absolute_len])
            && write_bytes(out, &mut c, br#""}}"#);
        if !page_id_ok {
            close_html_descriptors(descriptors, i + 1, max);
            let _ = host::html_close(doc);
            return write_error("get_pages", "internal_error", "response buffer overflow");
        }
        written += 1;
    }
    let _ = host::html_close(doc);
    if written == 0 {
        return write_error(
            "get_pages",
            "unsupported_format",
            "no ordinary still images found; MotionToon and locked episodes are not supported",
        );
    }
    if !write_bytes(out, &mut c, b"]}") {
        return write_error("get_pages", "internal_error", "response buffer overflow");
    }
    write_success_payload("get_pages", c)
}

fn run_get_image_request(req: &[u8]) -> u32 {
    let image_id = extract_json_string(req, b"sourceImageId")
        .or_else(|| extract_json_string(req, b"pageId"))
        .unwrap_or(b"");
    let mut referer = [0u8; 4096];
    let referer_len = if let Some(rest) = image_id.strip_prefix(b"page:webtoons:") {
        let mut parts = rest.splitn(4, |b| *b == b':');
        let title_no = parts.next().unwrap_or(b"");
        let episode_no = parts.next().unwrap_or(b"");
        let _index = parts.next().unwrap_or(b"");
        let route = parts.next().unwrap_or(b"");
        if !digits(title_no) || !digits(episode_no) || !route.ends_with(b"/list") {
            return write_error(
                "get_image_request",
                "invalid_page_id",
                "WEBTOON page identity is malformed",
            );
        }
        let Some(len) = viewer_url(title_no, episode_no, route, &mut referer) else {
            return write_error(
                "get_image_request",
                "invalid_page_id",
                "unsupported WEBTOON viewer route",
            );
        };
        len
    } else if let Some((title_no, route)) = manga_id_parts(image_id) {
        let mut c = 0;
        if !append_slice(&mut referer, &mut c, BASE)
            || !append_slice(&mut referer, &mut c, b"/en/")
            || !append_slice(&mut referer, &mut c, route)
            || !append_slice(&mut referer, &mut c, b"?title_no=")
            || !append_slice(&mut referer, &mut c, title_no)
        {
            return write_error(
                "get_image_request",
                "invalid_page_id",
                "WEBTOON title route is too long",
            );
        }
        c
    } else {
        return write_error(
            "get_image_request",
            "invalid_page_id",
            "expected a WEBTOON manga or page identity",
        );
    };
    let url = extract_json_string(req, b"url").unwrap_or(b"");
    if !allowed_image_url(url) {
        return write_error(
            "get_image_request",
            "invalid_image_url",
            "image host is outside the WEBTOON public image domains",
        );
    }
    let out = payload_buf();
    let mut c = 0;
    if !write_bytes(out, &mut c, br#"{"imageRequest":{"url":""#)
        || !append_json_unescaped_then_escaped(out, &mut c, url)
        || !write_bytes(out, &mut c, br#"","headers":{"Referer":""#)
        || !append_json_escaped(out, &mut c, &referer[..referer_len])
        || !write_bytes(out, &mut c, br#""}}}"#)
    {
        return write_error(
            "get_image_request",
            "internal_error",
            "response buffer overflow",
        );
    }
    write_success_payload("get_image_request", c)
}

fn run_get_listings(_req: &[u8]) -> u32 {
    write_error(
        "get_listings",
        "not_implemented",
        "browse listings are not supported",
    )
}
fn run_get_manga_list(_req: &[u8]) -> u32 {
    write_error(
        "get_manga_list",
        "not_implemented",
        "browse listings are not supported",
    )
}
fn run_get_home(_req: &[u8]) -> u32 {
    write_error("get_home", "not_implemented", "home feed is not supported")
}
fn run_get_filters(_req: &[u8]) -> u32 {
    write_error(
        "get_filters",
        "not_implemented",
        "search filters are not supported",
    )
}
fn run_get_settings(_req: &[u8]) -> u32 {
    write_error(
        "get_settings",
        "not_implemented",
        "account and source settings are not supported",
    )
}

koma_source_sdk::koma_source_exports!("webtoons");
