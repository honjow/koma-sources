#![no_std]

extern crate koma_source_sdk;

use koma_source_sdk::host::{self, http_request, log_info};
use koma_source_sdk::json_utils::{
    append_json_escaped, contains_bytes, extract_json_number, extract_json_string, find_subslice,
    write_bytes, write_url_encoded, write_usize, request_page_number, json_object_member,
    append_json_unescaped_then_escaped, JsonArrayIter,
};
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{FetchError, build_get_request, fetch_error_code};

const BASE_URL: &[u8] = b"https://www.dm5.cn";
koma_source_sdk::koma_source_buffers! {
    payload: 1024 * 1024,
    http_out: 2 * 1024 * 1024,
    body: 2 * 1024 * 1024,
    http_req: 2048,
    scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.dm5.koma",
    name: "DM5",
    version: "0.1.22",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "",
    content_rating: "nsfw",
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


fn fetch_body(url: &[u8], referer: Option<&[u8]>) -> Result<usize, FetchError> {
    let req_len =
        build_get_request(http_req_buf(), url, referer, &[]).ok_or(FetchError::Network)?;
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
                    log_info(b"DM5: http transport error, retrying");
                }
            }
        }
    }
    if transport_failed {
        return Err(FetchError::Network);
    }
    decode_json_body(&http_out()[..resp_len])
}

fn make_url(parts: &[&[u8]]) -> Option<usize> {
    let buf = scratch_a();
    let mut c = 0usize;
    for part in parts {
        write_bytes(buf, &mut c, part).then_some(())?;
    }
    Some(c)
}

fn manga_id_from_href(href: &[u8]) -> Option<&[u8]> {
    if href.len() < 10 || href[0] != b'/' || !href[1..].starts_with(b"manhua-") {
        return None;
    }
    let mut end = 1usize;
    while end < href.len() && href[end] != b'/' && href[end] != b'?' && href[end] != b'#' {
        end += 1;
    }
    Some(&href[1..end])
}

fn chapter_id_from_href(href: &[u8]) -> Option<&[u8]> {
    if href.len() < 4 || href[0] != b'/' || href[1] != b'm' {
        return None;
    }
    let mut end = 2usize;
    while end < href.len() && href[end] >= b'0' && href[end] <= b'9' {
        end += 1;
    }
    if end == 2 {
        None
    } else {
        Some(&href[2..end])
    }
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

fn strip_tags_to<'a>(src: &[u8], out: &'a mut [u8]) -> &'a [u8] {
    let mut c = 0usize;
    let mut in_tag = false;
    for &b in src {
        if b == b'<' {
            in_tag = true;
            continue;
        }
        if b == b'>' {
            in_tag = false;
            continue;
        }
        if !in_tag && c < out.len() {
            out[c] = b;
            c += 1;
        }
    }
    trim_ascii(&out[..c])
}

fn write_card_item(
    payload: &mut [u8],
    c: &mut usize,
    id: &[u8],
    title: &[u8],
    cover: &[u8],
    tag: &[u8],
) -> bool {
    write_bytes(payload, c, br#"{"id":"manga:"#)
        && append_json_escaped(payload, c, id)
        && write_bytes(payload, c, br#"","title":""#)
        && append_json_escaped(payload, c, title)
        && write_bytes(payload, c, br#"","cover":{"kind":"url","url":""#)
        && append_json_escaped(payload, c, cover)
        && write_bytes(payload, c, br#""},"authors":[],"status":"unknown","contentRating":"nsfw","description":"","sourceTags":["#)
        && write_bytes(payload, c, b"\"")
        && append_json_escaped(payload, c, tag)
        && write_bytes(payload, c, br#""]}"#)
}

// Read the cover only from this title's original card parent. A nearby image
// outside the li (for example the hidden search-empty image) is not a cover.
fn card_cover<'a>(html: &'a [u8], title_at: usize, id: &[u8]) -> Option<&'a [u8]> {
    let start = find_last_subslice(&html[..title_at], b"<li>")?;
    let end = start + find_subslice(&html[start..], b"</li>")?;
    if title_at >= end { return None; }
    let card = &html[start..end];
    if !contains_bytes(card, b"class=\"mh-item mh-card-wrap\"")
        || extract_attr_after(card, b"class=\"manga-info\"", b"uk")? != id
    {
        return None;
    }
    let marker = b"class=\"mh-cover\"";
    let at = find_subslice(card, marker)?;
    if contains_bytes(&card[at + marker.len()..], marker) { return None; }
    let style = extract_attr_after(card, marker, b"style")?;
    let prefix = b"background-image: url(";
    let from = find_subslice(style, prefix)? + prefix.len();
    let to = from + find_subslice(&style[from..], b")")?;
    let cover = trim_ascii(&style[from..to]);
    if cover.is_empty() { None } else { Some(cover) }
}

fn parse_cards(html: &[u8], payload: &mut [u8], c: &mut usize, tag: &[u8]) -> Result<usize, &'static str> {
    parse_cards_with_limit(html, payload, c, tag, usize::MAX)
}

fn parse_cards_with_limit(html: &[u8], payload: &mut [u8], c: &mut usize, tag: &[u8], limit: usize) -> Result<usize, &'static str> {
    let mut pos = 0usize;
    let mut written = 0usize;
    let mut last_id = b"" as &[u8];
    loop {
        let rel = match find_subslice(&html[pos..], b"<a ") {
            Some(v) => v,
            None => break,
        };
        let a_start = pos + rel;
        let a_end = match find_subslice(&html[a_start..], b">") {
            Some(v) => a_start + v + 1,
            None => return Err("unterminated card anchor"),
        };
        pos = a_end;
        let a_tag = &html[a_start..a_end];
        let href = match attr_value(a_tag, b"href") {
            Some(v) => v,
            None => continue,
        };
        let id = match manga_id_from_href(href) {
            Some(v) => v,
            None => continue,
        };
        if id == last_id {
            continue;
        }
        let title = match attr_value(a_tag, b"title") {
            Some(v) => v,
            None => continue,
        };
        let cover = card_cover(html, a_start, id).unwrap_or(b"");
        if written > 0 && !write_bytes(payload, c, b",") {
            return Err("payload overflow");
        }
        if !write_card_item(payload, c, id, title, cover, tag) {
            return Err("payload overflow");
        }
        last_id = id;
        written += 1;
        if written >= limit { break; }
    }
    Ok(written)
}

fn find_last_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    let mut result = None;
    let mut pos = 0usize;
    while pos + needle.len() <= haystack.len() {
        if &haystack[pos..pos + needle.len()] == needle {
            result = Some(pos);
        }
        pos += 1;
    }
    result
}

fn write_manga_page_from_url(operation: &str, url: &[u8]) -> u32 {
    write_manga_page_from_url_at(operation, url, None)
}

fn write_manga_page_from_url_at(operation: &str, url: &[u8], search_page: Option<usize>) -> u32 {
    let html_len = match fetch_body(url, None) {
        Ok(n) => n,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error(operation, c, m);
        }
    };
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };
    if search_page.is_some() {
        if contains_bytes(html, b"class=\"box404\"") {
            return write_error(operation, "not_found", "search page not found");
        }
        if !contains_bytes(html, b"id=\"txtKeywords\"") {
            return write_error(operation, "parse_error", "missing search response");
        }
    }
    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error(operation, "internal_error", "payload overflow");
    }
    let written = match parse_cards(html, payload, &mut c, b"dm5") {
        Ok(count) => count,
        Err(message) => return write_error(operation, "parse_error", message),
    };
    let mut next_link = [0u8; 48];
    let mut next_link_len = 0usize;
    let has_more = if let Some(page) = search_page {
        write_bytes(&mut next_link, &mut next_link_len, b"page=")
            && write_usize(&mut next_link, &mut next_link_len, page.saturating_add(1))
            && write_bytes(&mut next_link, &mut next_link_len, b"\"")
            && contains_bytes(html, &next_link[..next_link_len])
            && written > 0
    } else {
        contains_bytes(html, b">>") || contains_bytes(html, b">&gt;<")
    };
    let ok = write_bytes(payload, &mut c, br#"],"page":{"nextCursor":"#)
        && if has_more {
            if let Some(page) = search_page {
                write_bytes(payload, &mut c, b"\"")
                    && write_usize(payload, &mut c, page.saturating_add(1))
                    && write_bytes(payload, &mut c, b"\"")
            } else {
                write_bytes(payload, &mut c, b"\"next\"")
            }
        } else {
            write_bytes(payload, &mut c, b"null")
        }
        && write_bytes(payload, &mut c, br#","hasMore":"#)
        && write_bytes(
            payload,
            &mut c,
            if has_more && written > 0 {
                b"true"
            } else {
                b"false"
            },
        )
        && write_bytes(payload, &mut c, b"}}");
    if !ok {
        return write_error(operation, "internal_error", "payload overflow");
    }
    write_success_payload(operation, c)
}

// Decode into a distinct bounded buffer, not the WAMR stack or URL scratch.
static mut SEARCH_QUERY: [u8; 8192] = [0; 8192];
fn run_search(req: &[u8]) -> u32 {
    let page = request_page_number(req);
    let query = extract_json_string(req, b"query").unwrap_or(b"");
    let decoded = unsafe { &mut *core::ptr::addr_of_mut!(SEARCH_QUERY) };
    let query = match koma_source_sdk::plain_text::decode_json_content(query, decoded) {
        Ok(v) => v,
        Err(_) => return write_error("search", "invalid_request", "invalid query"),
    };
    if query.is_empty() {
        return run_get_listings(req);
    }
    if query.starts_with(b"id:") {
        let id = &query[3..];
        let url_len = match make_url(&[BASE_URL, b"/", id, b"/"]) {
            Some(n) => n,
            None => return write_error("search", "internal_error", "url overflow"),
        };
        let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), url_len) };
        return search_single_manga(url, id);
    }
    let mut c = 0usize;
    let url_buf = scratch_a();
    if !(write_bytes(url_buf, &mut c, BASE_URL)
        && write_bytes(url_buf, &mut c, b"/search?title=")
        && write_url_encoded(url_buf, &mut c, query)
        && write_bytes(url_buf, &mut c, b"&language=1&page=")
        && write_usize(url_buf, &mut c, page))
    {
        return write_error("search", "internal_error", "url overflow");
    }
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), c) };
    write_manga_page_from_url_at("search", url, Some(page))
}

fn search_single_manga(url: &[u8], id: &[u8]) -> u32 {
    let html_len = match fetch_body(url, None) {
        Ok(n) => n,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("search", c, m);
        }
    };
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };
    let mut title_buf = [0u8; 256];
    let title =
        extract_between_text(html, b"<p class=\"title\"", b"</p>", &mut title_buf).unwrap_or(id);
    let cover = extract_first_img_src_after(html, b"class=\"cover\"")
        .or_else(|| extract_attr_after(html, b"class=\"cover\"", b"src"))
        .unwrap_or(b"");
    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"items":["#)
        && write_card_item(payload, &mut c, id, title, cover, b"dm5")
        && write_bytes(
            payload,
            &mut c,
            br#"],"page":{"nextCursor":null,"hasMore":false}}"#,
        );
    if !ok {
        return write_error("search", "internal_error", "payload overflow");
    }
    write_success_payload("search", c)
}

fn extract_attr_after<'a>(html: &'a [u8], marker: &[u8], attr: &[u8]) -> Option<&'a [u8]> {
    let p = find_subslice(html, marker)?;
    let tag_start = find_back(html, p, b'<').unwrap_or(p);
    let tag_end = find_subslice(&html[p..], b">")? + p + 1;
    attr_value(&html[tag_start..tag_end], attr)
}

fn extract_first_img_src_after<'a>(html: &'a [u8], marker: &[u8]) -> Option<&'a [u8]> {
    let p = find_subslice(html, marker)?;
    let img_rel = find_subslice(&html[p..], b"<img")?;
    let img_start = p + img_rel;
    let img_end = find_subslice(&html[img_start..], b">")? + img_start + 1;
    attr_value(&html[img_start..img_end], b"src")
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

fn extract_between_text<'a>(
    html: &[u8],
    marker: &[u8],
    end_marker: &[u8],
    out: &'a mut [u8],
) -> Option<&'a [u8]> {
    let marker_pos = find_subslice(html, marker)?;
    let gt = find_subslice(&html[marker_pos..], b">")? + marker_pos + 1;
    let end = find_subslice(&html[gt..], end_marker)? + gt;
    Some(strip_tags_to(&html[gt..end], out))
}

fn run_get_manga(req: &[u8]) -> u32 {
    let manga_id = match extract_json_string(req, b"mangaId") {
        Some(v) => v,
        None => return write_error("get_manga", "invalid_request", "missing mangaId"),
    };
    if manga_id.len() <= 6 || &manga_id[..6] != b"manga:" {
        return write_error("get_manga", "invalid_request", "unexpected mangaId");
    }
    let id = &manga_id[6..];
    let url_len = match make_url(&[BASE_URL, b"/", id, b"/"]) {
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
    let mut desc_buf = [0u8; 2048];
    let title = extract_js_string(html, b"DM5_COMIC_MNAME=\"").unwrap_or_else(|| {
        extract_between_text(html, b"<p class=\"title\"", b"</p>", &mut title_buf).unwrap_or(id)
    });
    let desc =
        extract_between_text(html, b"<p class=\"content\"", b"</p>", &mut desc_buf).unwrap_or(b"");
    let cover = extract_first_img_src_after(html, b"class=\"cover\"")
        .or_else(|| extract_attr_after(html, b"class=\"cover\"", b"src"))
        .unwrap_or(b"");
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
    let ok = write_bytes(payload, &mut c, br#"{"manga":{"id":"manga:"#)
        && append_json_escaped(payload, &mut c, id)
        && write_bytes(payload, &mut c, br#"","title":""#)
        && append_json_escaped(payload, &mut c, title)
        && write_bytes(payload, &mut c, br#"","alternateTitles":[],"description":""#)
        && append_json_escaped(payload, &mut c, desc)
        && write_bytes(payload, &mut c, br#"","cover":{"kind":"url","url":""#)
        && append_json_escaped(payload, &mut c, cover)
        && write_bytes(payload, &mut c, br#""},"authors":[],"artists":[],"status":""#)
        && write_bytes(payload, &mut c, status)
        && write_bytes(payload, &mut c, br#"","contentRating":"nsfw","language":"zh","tags":[],"links":[{"kind":"source","url":""#)
        && append_json_escaped(payload, &mut c, url)
        && write_bytes(payload, &mut c, br#""}]}}"#);
    if !ok {
        return write_error("get_manga", "internal_error", "payload overflow");
    }
    write_success_payload("get_manga", c)
}

fn chapter_anchor_number(list: &[u8], start: usize) -> usize {
    let block = &list[start..];
    find_subslice(block, b">").and_then(|end| attr_value(&block[..end + 1], b"href"))
        .and_then(chapter_id_from_href)
        .map(|id| id.iter().fold(0usize, |n, b| n.saturating_mul(10).saturating_add((b - b'0') as usize)))
        .unwrap_or(0)
}

fn run_get_chapters(req: &[u8]) -> u32 {
    let manga_id = match extract_json_string(req, b"mangaId") {
        Some(v) => v,
        None => return write_error("get_chapters", "invalid_request", "missing mangaId"),
    };
    if manga_id.len() <= 6 || &manga_id[..6] != b"manga:" {
        return write_error("get_chapters", "invalid_request", "unexpected mangaId");
    }
    let id = &manga_id[6..];
    let url_len = match make_url(&[BASE_URL, b"/", id, b"/"]) {
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
    let list_start = match find_subslice(html, b"id=\"chapterlistload\"") {
        Some(v) => v,
        None => return write_error("get_chapters", "parse_error", "chapter list not found"),
    };
    let list_end = find_subslice(&html[list_start..], b"</div>")
        .map(|v| list_start + v)
        .unwrap_or(html.len());
    let list = &html[list_start..list_end];
    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error("get_chapters", "internal_error", "payload overflow");
    }
    // Default keeps the delivered site order. Explicit sortChapter follows
    // Keiyoushi's descending numeric chapter URL order, without changing IDs,
    // page counts or title-derived sequence metadata. Equal IDs retain site order.
    let sort = json_object_member(req, b"settings")
        .and_then(|s| json_object_member(s, b"sortChapter"))
        .map(|v| v == b"true" || v == b"\"true\"").unwrap_or(false);
    let mut anchors = [0usize; 8192];
    let mut count = 0usize;
    let mut pos = 0usize;
    while let Some(rel) = find_subslice(&list[pos..], b"<a ") {
        if count == anchors.len() {
            return write_error("get_chapters", "parse_error", "too many chapter anchors");
        }
        anchors[count] = pos + rel;
        count += 1;
        pos = match find_subslice(&list[anchors[count - 1]..], b"</a>") {
            Some(end) => anchors[count - 1] + end + 4,
            None => return write_error("get_chapters", "parse_error", "unterminated chapter anchor"),
        };
    }
    if sort {
        anchors[..count].sort_unstable_by(|a, b| {
            chapter_anchor_number(list, *b).cmp(&chapter_anchor_number(list, *a))
                .then_with(|| a.cmp(b))
        });
    }
    let mut written = 0usize;
    for &a_start in &anchors[..count] {
        let a_end = match find_subslice(&list[a_start..], b"</a>") {
            Some(v) => a_start + v + 4,
            None => return write_error("get_chapters", "parse_error", "unterminated chapter anchor"),
        };
        let block = &list[a_start..a_end];
        let tag_end = match find_subslice(block, b">") {
            Some(v) => v + 1,
            None => continue,
        };
        let href = match attr_value(&block[..tag_end], b"href") {
            Some(v) => v,
            None => continue,
        };
        let cid = match chapter_id_from_href(href) {
            Some(v) => v,
            None => continue,
        };
        // Retain the established identity/metadata algorithm separately from display.
        let title_html=&block[tag_end..block.len().saturating_sub(4)];
        let display_title=DisplayChapterTitle::new(title_html);
        let mut title_buf = [0u8; 256];
        let identity_title = strip_tags_to(
            title_html,
            &mut title_buf,
        );
        let page_count = extract_page_count(identity_title);
        let sequence = chapter_sequence(identity_title);
        if written > 0 && !write_bytes(payload, &mut c, b",") {
            return write_error("get_chapters", "internal_error", "payload overflow");
        }
        let ok = write_bytes(payload, &mut c, br#"{"id":"chapter:"#)
            && append_json_escaped(payload, &mut c, id)
            && write_bytes(payload, &mut c, b":")
            && append_json_escaped(payload, &mut c, cid)
            && write_bytes(payload, &mut c, b":")
            && write_usize(payload, &mut c, page_count)
            && write_bytes(payload, &mut c, br#"","mangaId":"manga:"#)
            && append_json_escaped(payload, &mut c, id)
            && write_bytes(payload, &mut c, br#"","title":""#)
            && write_display_chapter_title(payload, &mut c, display_title)
            && write_bytes(payload, &mut c, br#"","chapterNumber":"#)
            && write_chapter_sequence_number(payload, &mut c, sequence, false)
            && write_bytes(payload, &mut c, br#", "volumeNumber":"#)
            && write_chapter_sequence_number(payload, &mut c, sequence, true)
            && write_bytes(payload, &mut c, br#", "readingGroup":""#)
            && write_bytes(payload, &mut c, if sequence.map(|(_, volume)| volume).unwrap_or(false) { b"volume" } else { b"chapter" })
            && write_bytes(payload, &mut c, br#"","continuationEligible":"#)
            && write_bytes(payload, &mut c, if sequence.is_some() { b"true" } else { b"false" })
            && write_bytes(payload, &mut c, br#","publishedAt":null,"updatedAt":null,"pageCount":"#)
            && if page_count > 0 { write_usize(payload, &mut c, page_count) } else { write_bytes(payload, &mut c, b"null") }
            && write_bytes(payload, &mut c, b"}");
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

// Preserve strip_tags_to's byte semantics and its four-byte trim set, without
// materializing a truncated title. The first pass locates the complete trim range.
#[derive(Clone)]
struct DisplayChapterTitle<'a> { raw: &'a [u8], pos: usize, end: usize, in_tag: bool }
impl<'a> DisplayChapterTitle<'a> {
    fn new(raw: &'a [u8]) -> Self {
        let mut first = raw.len();
        let mut end = 0;
        let mut in_tag = false;
        for (i, &byte) in raw.iter().enumerate() {
            if byte == b'<' { in_tag = true; continue; }
            if byte == b'>' { in_tag = false; continue; }
            if !in_tag && !matches!(byte, b' ' | b'\t' | b'\n' | b'\r') {
                first = first.min(i);
                end = i + 1;
            }
        }
        Self { raw, pos: first, end, in_tag: false }
    }
}
impl Iterator for DisplayChapterTitle<'_> {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        while self.pos < self.end {
            let byte = self.raw[self.pos];
            self.pos += 1;
            if byte == b'<' { self.in_tag = true; continue; }
            if byte == b'>' { self.in_tag = false; continue; }
            if !self.in_tag { return Some(byte); }
        }
        None
    }
}
fn write_display_chapter_title(output: &mut [u8], cursor: &mut usize, title: DisplayChapterTitle<'_>) -> bool {
    for byte in title {
        if !append_json_escaped(output, cursor, &[byte]) { return false; }
    }
    true
}
fn extract_page_count(title: &[u8]) -> usize {
    let p_marker = "P".as_bytes();
    let p = match find_subslice(title, p_marker) {
        Some(v) => v,
        None => return 0,
    };
    let mut start = p;
    while start > 0 && title[start - 1] >= b'0' && title[start - 1] <= b'9' {
        start -= 1;
    }
    let mut n = 0usize;
    let mut i = start;
    while i < p {
        n = n * 10 + (title[i] - b'0') as usize;
        i += 1;
    }
    n
}

// Only the source's chapter/volume label grammar supplies an order. Page counts,
// announcements and unknown extras remain in the manual catalog without adjacency.
fn chapter_sequence(title: &[u8]) -> Option<(&[u8], bool)> {
    let title = trim_ascii(title);
    let bytes = title.strip_prefix("第".as_bytes()).unwrap_or(title);
    let mut end = 0;
    while end < bytes.len() && bytes[end].is_ascii_digit() { end += 1; }
    if end == 0 { return None; }
    if end < bytes.len() && bytes[end] == b'.' {
        end += 1;
        let decimal_start = end;
        while end < bytes.len() && bytes[end].is_ascii_digit() { end += 1; }
        if end == decimal_start { return None; }
    }
    let suffix = &bytes[end..];
    let volume = if suffix.starts_with("卷".as_bytes()) { true }
        else if suffix.starts_with("话".as_bytes()) || suffix.starts_with("話".as_bytes()) { false }
        else { return None; };
    Some((&bytes[..end], volume))
}

fn write_chapter_sequence_number(
    output: &mut [u8], cursor: &mut usize, sequence: Option<(&[u8], bool)>, volume: bool,
) -> bool {
    match sequence {
        Some((number, is_volume)) if is_volume == volume => {
            write_bytes(output, cursor, b"\"")
                && append_json_escaped(output, cursor, number)
                && write_bytes(output, cursor, b"\"")
        }
        _ => write_bytes(output, cursor, b"null"),
    }
}

fn hex4(s: &[u8]) -> Option<u32> {
    if s.len() != 4 {
        return None;
    };
    let mut n = 0;
    for &c in s {
        n = n * 16
            + match c {
                b'0'..=b'9' => (c - b'0') as u32,
                b'a'..=b'f' => (c - b'a' + 10) as u32,
                b'A'..=b'F' => (c - b'A' + 10) as u32,
                _ => return None,
            };
    }
    Some(n)
}
fn comment_unescape<'a>(s: &[u8], out: &'a mut [u8]) -> Result<&'a [u8], &'static str> {
    let mut i = 0;
    let mut n = 0;
    while i < s.len() {
        let c = s[i];
        i += 1;
        if c != b'\\' {
            if c < 32 || c == b'"' {
                return Err("invalid JSON string");
            };
            if !write_bytes(out, &mut n, &[c]) { return Err("comment overflow"); }
            continue;
        }
        let c = *s.get(i).ok_or("invalid JSON escape")?;
        i += 1;
        let c = match c {
            b'"' => b'"',
            b'\\' => b'\\',
            b'/' => b'/',
            b'n' => b'\n',
            b'r' => b'\r',
            b't' => b'\t',
            b'b' => 8,
            b'f' => 12,
            b'u' => {
                let mut cp = hex4(s.get(i..i + 4).ok_or("invalid unicode escape")?)
                    .ok_or("invalid unicode escape")?;
                i += 4;
                if (0xd800..=0xdbff).contains(&cp) {
                    if s.get(i..i + 2) != Some(b"\\u") {
                        return Err("invalid unicode pair");
                    };
                    i += 2;
                    let low = hex4(s.get(i..i + 4).ok_or("invalid unicode pair")?)
                        .ok_or("invalid unicode pair")?;
                    i += 4;
                    if !(0xdc00..=0xdfff).contains(&low) {
                        return Err("invalid unicode pair");
                    };
                    cp = 0x10000 + ((cp - 0xd800) << 10) + (low - 0xdc00)
                } else if (0xdc00..=0xdfff).contains(&cp) {
                    return Err("invalid unicode pair");
                };
                let ch = char::from_u32(cp).ok_or("invalid unicode scalar")?;
                let mut tmp = [0; 4];
                if !write_bytes(out, &mut n, ch.encode_utf8(&mut tmp).as_bytes()) { return Err("comment overflow"); }
                continue;
            }
            _ => return Err("invalid JSON escape"),
        };
        if !write_bytes(out, &mut n, &[c]) { return Err("comment overflow"); }
    }
    core::str::from_utf8(&out[..n]).map_err(|_| "invalid UTF8")?;
    Ok(&out[..n])
}

// Reuse the host's HTML5 entity table without interpreting literal markup.
// Escaping only literal angle brackets preserves Parser.unescapeEntities(false)
// semantics; CR is numeric-escaped to avoid HTML input newline normalization.
static mut COMMENT_TEXT: [u8; 65536] = [0; 65536];
static mut COMMENT_HTML: [u8; 4 * 65536 + 16] = [0; 4 * 65536 + 16];
fn write_plain_comment(output: &mut [u8], cursor: &mut usize, poster: &[u8], content: &[u8]) -> bool {
    let text = unsafe { &mut *core::ptr::addr_of_mut!(COMMENT_TEXT) };
    let html = unsafe { &mut *core::ptr::addr_of_mut!(COMMENT_HTML) };
    let mut n = 0;
    if !write_bytes(html, &mut n, b"<div>") { return false; }
    for (index, raw) in [poster, content].iter().enumerate() {
        if index > 0 && !write_bytes(html, &mut n, b": ") { return false; }
        let decoded = match comment_unescape(raw, text) { Ok(v) => v, Err(_) => return false };
        for &b in decoded {
            let escaped = match b { b'<' => b"&lt;".as_slice(), b'>' => b"&gt;", b'\r' => b"&#13;", _ => core::slice::from_ref(&b) };
            if !write_bytes(html, &mut n, escaped) { return false; }
        }
    }
    if !write_bytes(html, &mut n, b"</div>") { return false; }
    let descriptor = match host::html_parse(&html[..n]) { Ok(v) => v, Err(_) => return false };
    let len = host::html_text(descriptor, text);
    let closed = host::html_close(descriptor).is_ok();
    match len {
        Ok(len) if closed && len < text.len() => append_json_escaped(output, cursor, &text[..len]),
        _ => false,
    }
}

fn supports_text_pages(req: &[u8]) -> bool {
    let Some(features) = json_object_member(req, b"hostFeatures") else { return false; };
    let features = trim_ascii(features);
    features.starts_with(b"[") && features.ends_with(b"]") &&
        features[1..features.len() - 1].split(|b| *b == b',')
            .any(|value| trim_ascii(value) == b"\"text_page_image_v1\"")
}
fn enabled_setting(req: &[u8], key: &[u8]) -> bool {
    json_object_member(req, b"settings").and_then(|s| json_object_member(s, key))
        .map(|v| v == b"true" || v == b"\"true\"").unwrap_or(false)
}

fn run_get_pages(req: &[u8]) -> u32 {
    let chapter_id = match extract_json_string(req, b"chapterId") {
        Some(v) => v,
        None => return write_error("get_pages", "invalid_request", "missing chapterId"),
    };
    if chapter_id.len() <= 8 || &chapter_id[..8] != b"chapter:" {
        return write_error("get_pages", "invalid_request", "unexpected chapterId");
    }
    let rest = &chapter_id[8..];
    let sep = match find_subslice(rest, b":") {
        Some(v) => v,
        None => return write_error("get_pages", "invalid_request", "chapterId missing cid"),
    };
    let manga = &rest[..sep];
    let tail = &rest[sep + 1..];
    let sep2 = find_subslice(tail, b":");
    let cid = if let Some(v) = sep2 { &tail[..v] } else { tail };
    let chapter_url_len = match make_url(&[BASE_URL, b"/m", cid, b"/"]) {
        Some(n) => n,
        None => return write_error("get_pages", "internal_error", "url overflow"),
    };
    let chapter_url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), chapter_url_len) };
    let html_len = match fetch_body(chapter_url, Some(BASE_URL)) {
        Ok(n) => n,
        Err(e) => {
            let (code, msg) = fetch_error_code(e);
            return write_error("get_pages", code, msg);
        }
    };
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };
    let comments = supports_text_pages(req) && enabled_setting(req, b"chapterComments");
    // Copy the real viewer fields before image requests reuse BODY_BUF.
    let mut topic_store = [0u8; 64];
    let mut topic_len = 0usize;
    let mut comment_pages = 0usize;
    if comments {
        let topic = extract_js_single_quoted_string(html, b"DM5_TIEBATOPICID='")
            .or_else(|| extract_js_string(html, b"DM5_TIEBATOPICID=\""));
        let total = extract_js_number(html, b"DM5_PAGEPCOUNT = ")
            .or_else(|| extract_js_number(html, b"DM5_PAGEPCOUNT="))
            .and_then(parse_usize);
        match (topic, total) {
            (Some(t), Some(total)) if !t.is_empty() && t.len() <= topic_store.len()
                && t.iter().all(|b| b.is_ascii_digit()) && total <= 2048 => {
                    topic_store[..t.len()].copy_from_slice(t); topic_len = t.len(); comment_pages = total;
                }
            _ => return write_error("get_pages", "parse_error", "missing chapter comment metadata"),
        }
    }
    let mid_raw = extract_js_number(html, b"DM5_MID=").unwrap_or(b"");
    let dt_raw = extract_js_string(html, b"DM5_VIEWSIGN_DT=\"").unwrap_or(b"");
    let sign_raw = extract_js_string(html, b"DM5_VIEWSIGN=\"").unwrap_or(b"");
    if mid_raw.is_empty() || dt_raw.is_empty() || sign_raw.is_empty() {
        return write_error("get_pages", "parse_error", "missing DM5 chapter signature");
    }
    let mut mid_store = [0u8; 32];
    let mut dt_store = [0u8; 64];
    let mut sign_store = [0u8; 64];
    if mid_raw.len() > mid_store.len()
        || dt_raw.len() > dt_store.len()
        || sign_raw.len() > sign_store.len()
    {
        return write_error("get_pages", "parse_error", "chapter signature too long");
    }
    mid_store[..mid_raw.len()].copy_from_slice(mid_raw);
    dt_store[..dt_raw.len()].copy_from_slice(dt_raw);
    sign_store[..sign_raw.len()].copy_from_slice(sign_raw);
    let mid = &mid_store[..mid_raw.len()];
    let dt = &dt_store[..dt_raw.len()];
    let sign = &sign_store[..sign_raw.len()];
    let encoded_page_count = sep2.and_then(|v| parse_usize(&tail[v + 1..]));
    let page_count = match extract_js_number(html, b"DM5_IMAGE_COUNT=")
        .and_then(parse_usize)
        .or_else(|| extract_json_number(req, b"pageCount").and_then(parse_usize))
        .or(encoded_page_count)
    {
        Some(n) if n > 0 => n,
        _ => return write_error("get_pages", "parse_error", "missing known chapter page count"),
    };
    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"chapterId":"chapter:"#)
        && append_json_escaped(payload, &mut c, manga)
        && write_bytes(payload, &mut c, b":")
        && append_json_escaped(payload, &mut c, cid)
        && write_bytes(payload, &mut c, b":")
        && write_usize(payload, &mut c, page_count)
        && write_bytes(payload, &mut c, br#"","pages":["#);
    if !ok {
        return write_error("get_pages", "internal_error", "payload overflow");
    }
    let mut page = 1usize;
    let mut written = 0usize;
    while page <= page_count {
        let got = match fetch_chapter_images(
            manga,
            cid,
            mid,
            dt,
            sign,
            page,
            payload,
            &mut c,
            &mut written,
        ) {
            Ok(n) => n,
            Err(e) => {
                let (code, msg) = fetch_error_code(e);
                return write_error("get_pages", code, msg);
            }
        };
        if got == 0 {
            break;
        }
        page += got;
        if written >= page_count {
            break;
        }
    }
    if written != page_count {
        return write_error("get_pages", "parse_error", "incomplete public chapter image list");
    }
    if comments {
        for page in 1..=comment_pages {
            let mut url = [0u8; 1024]; let mut u = 0;
            let ok = write_bytes(&mut url, &mut u, BASE_URL)
                && write_bytes(&mut url, &mut u, b"/m") && write_bytes(&mut url, &mut u, cid)
                && write_bytes(&mut url, &mut u, b"/pagerdata.ashx?pageindex=") && write_usize(&mut url, &mut u, page)
                && write_bytes(&mut url, &mut u, b"&pagesize=") && write_usize(&mut url, &mut u, comment_pages)
                && write_bytes(&mut url, &mut u, b"&tid=") && write_bytes(&mut url, &mut u, &topic_store[..topic_len])
                && write_bytes(&mut url, &mut u, b"&cid=") && write_bytes(&mut url, &mut u, cid)
                && write_bytes(&mut url, &mut u, b"&t=9");
            if !ok { return write_error("get_pages", "internal_error", "comment URL overflow"); }
            let len = match fetch_body(&url[..u], Some(BASE_URL)) {
                Ok(n) => n,
                Err(e) => { let (code, msg) = fetch_error_code(e); return write_error("get_pages", code, msg); }
            };
            let body = trim_ascii(unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), len) });
            if !body.ends_with(b"]") { return write_error("get_pages", "parse_error", "invalid chapter comments"); }
            let mut rows = match JsonArrayIter::from_raw_array(body) {
                Some(v) => v, None => return write_error("get_pages", "parse_error", "invalid chapter comments"),
            };
            let ok = write_bytes(payload, &mut c, b",{\"id\":\"page:dm5:")
                && append_json_escaped(payload, &mut c, cid)
                && write_bytes(payload, &mut c, b":comments:") && write_usize(payload, &mut c, page)
                && write_bytes(payload, &mut c, b"\",\"index\":") && write_usize(payload, &mut c, page_count + page - 1)
                && write_bytes(payload, &mut c, b",\"image\":{\"kind\":\"text\",\"text\":{\"template\":\"dm5-chapter-comments\",\"paragraphs\":[");
            if !ok { return write_error("get_pages", "internal_error", "payload overflow"); }
            let mut count = 0;
            let mut consumed = 1usize;
            while let Some(row) = rows.next_object() {
                let start = row.as_ptr() as usize - body.as_ptr() as usize;
                let gap = trim_ascii(&body[consumed..start]);
                if (count == 0 && !gap.is_empty()) || (count > 0 && gap != b",") {
                    return write_error("get_pages", "parse_error", "invalid chapter comments");
                }
                consumed = start + row.len();
                let poster = match extract_json_string(row, b"Poster") { Some(v) => v, None => return write_error("get_pages", "parse_error", "comment poster missing") };
                let content = match extract_json_string(row, b"PostContent") { Some(v) => v, None => return write_error("get_pages", "parse_error", "comment text missing") };
                if (count > 0 && !write_bytes(payload, &mut c, b","))
                    || !write_bytes(payload, &mut c, b"\"")
                    || !write_plain_comment(payload, &mut c, poster, content)
                    || !write_bytes(payload, &mut c, b"\"") {
                    return write_error("get_pages", "parse_error", "comment text could not be decoded");
                }
                count += 1;
            }
            if trim_ascii(&body[consumed..]) != b"]" {
                return write_error("get_pages", "parse_error", "invalid chapter comments");
            }
            if count == 0 && !write_bytes(payload, &mut c, "\"没有吐槽\"".as_bytes()) {
                return write_error("get_pages", "internal_error", "payload overflow");
            }
            if !write_bytes(payload, &mut c, b"]}}}") { return write_error("get_pages", "internal_error", "payload overflow"); }
        }
    }
    if !write_bytes(payload, &mut c, b"]}") {
        return write_error("get_pages", "internal_error", "payload overflow");
    }
    write_success_payload("get_pages", c)
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

fn fetch_chapter_images(
    manga: &[u8],
    cid: &[u8],
    mid: &[u8],
    dt: &[u8],
    sign: &[u8],
    page: usize,
    payload: &mut [u8],
    c: &mut usize,
    written: &mut usize,
) -> Result<usize, FetchError> {
    let url_buf = scratch_a();
    let mut u = 0usize;
    if !(write_bytes(url_buf, &mut u, BASE_URL)
        && write_bytes(url_buf, &mut u, b"/m")
        && write_bytes(url_buf, &mut u, cid)
        && write_bytes(url_buf, &mut u, b"/chapterfun.ashx?cid=")
        && write_bytes(url_buf, &mut u, cid)
        && write_bytes(url_buf, &mut u, b"&page=")
        && write_usize(url_buf, &mut u, page)
        && write_bytes(url_buf, &mut u, b"&key=&language=1&gtk=6&_cid=")
        && write_bytes(url_buf, &mut u, cid)
        && write_bytes(url_buf, &mut u, b"&_mid=")
        && write_bytes(url_buf, &mut u, mid)
        && write_bytes(url_buf, &mut u, b"&_dt=")
        && write_url_encoded(url_buf, &mut u, dt)
        && write_bytes(url_buf, &mut u, b"&_sign=")
        && write_bytes(url_buf, &mut u, sign))
    {
        return Err(FetchError::Network);
    }
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), u) };
    let referer_buf = scratch_b();
    let mut r = 0usize;
    if !(write_bytes(referer_buf, &mut r, BASE_URL)
        && write_bytes(referer_buf, &mut r, b"/m")
        && write_bytes(referer_buf, &mut r, cid)
        && write_bytes(referer_buf, &mut r, b"/"))
    {
        return Err(FetchError::Network);
    }
    let referer = unsafe { core::slice::from_raw_parts(SCRATCH_B.as_ptr(), r) };
    let js_len = fetch_body(url, Some(referer))?;
    let js = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), js_len) };
    unpack_and_write_images(js, manga, cid, payload, c, written)
}

fn unpack_and_write_images(
    packed: &[u8],
    manga: &[u8],
    cid: &[u8],
    payload: &mut [u8],
    c: &mut usize,
    written: &mut usize,
) -> Result<usize, FetchError> {
    let payload_start = find_subslice(packed, b"}('").ok_or(FetchError::Network)? + 3;
    let payload_end =
        find_subslice(&packed[payload_start..], b"',").ok_or(FetchError::Network)? + payload_start;
    let after = payload_end + 2;
    let mut comma = after;
    while comma < packed.len() && packed[comma] != b',' {
        comma += 1;
    }
    let base = parse_usize(&packed[after..comma]).ok_or(FetchError::Network)?;
    let dict_marker = b"'";
    let dict_start =
        find_subslice(&packed[comma..], dict_marker).ok_or(FetchError::Network)? + comma + 1;
    let dict_end = find_subslice(&packed[dict_start..], b"'.split('|')")
        .ok_or(FetchError::Network)?
        + dict_start;
    let dict = &packed[dict_start..dict_end];
    let mut starts = [0usize; 128];
    let mut lens = [0usize; 128];
    let mut count = 0usize;
    let mut s = 0usize;
    let mut i = 0usize;
    while i <= dict.len() && count < starts.len() {
        if i == dict.len() || dict[i] == b'|' {
            starts[count] = s;
            lens[count] = i - s;
            count += 1;
            s = i + 1;
        }
        i += 1;
    }
    let out = scratch_b();
    let mut o = 0usize;
    let mut p = payload_start;
    while p < payload_end {
        let b = packed[p];
        if is_word(b) {
            let start = p;
            while p < payload_end && is_word(packed[p]) {
                p += 1;
            }
            let idx = parse_base(&packed[start..p], base);
            if idx < count && lens[idx] > 0 {
                if o + lens[idx] > out.len() {
                    return Err(FetchError::Network);
                }
                out[o..o + lens[idx]].copy_from_slice(&dict[starts[idx]..starts[idx] + lens[idx]]);
                o += lens[idx];
            } else {
                let len = p - start;
                if o + len > out.len() {
                    return Err(FetchError::Network);
                }
                out[o..o + len].copy_from_slice(&packed[start..p]);
                o += len;
            }
        } else if b == b'\\' && p + 1 < payload_end {
            if o >= out.len() {
                return Err(FetchError::Network);
            }
            out[o] = packed[p + 1];
            o += 1;
            p += 2;
        } else {
            if o >= out.len() {
                return Err(FetchError::Network);
            }
            out[o] = b;
            o += 1;
            p += 1;
        }
    }
    let unpacked = &out[..o];
    let pix = extract_js_string(unpacked, b"pix=\"").ok_or(FetchError::Network)?;
    let key = extract_js_single_quoted_string(unpacked, b"key='").ok_or(FetchError::Network)?;
    let arr_start = find_subslice(unpacked, b"pvalue=[\"").ok_or(FetchError::Network)? + 9;
    let mut pos = arr_start;
    let mut added = 0usize;
    while pos < unpacked.len() {
        let end = match find_subslice(&unpacked[pos..], b"\"") {
            Some(v) => pos + v,
            None => break,
        };
        let path = &unpacked[pos..end];
        let suffix = if path.starts_with(b"http://") || path.starts_with(b"https://") {
            None
        } else {
            Some(original_image_suffix(unpacked, cid, key)?)
        };
        if *written > 0 && !write_bytes(payload, c, b",") {
            return Err(FetchError::Network);
        }
        let ok = write_bytes(payload, c, br#"{"id":"page:"#)
            && append_json_escaped(payload, c, manga)
            && write_bytes(payload, c, b":")
            && append_json_escaped(payload, c, cid)
            && write_bytes(payload, c, b":")
            && write_usize(payload, c, *written)
            && write_bytes(payload, c, br#"","index":"#)
            && write_usize(payload, c, *written)
            && write_bytes(payload, c, br#","image":{"kind":"url","url":""#)
            && if path.starts_with(b"http://") || path.starts_with(b"https://") {
                append_json_escaped(payload, c, path)
            } else {
                append_json_escaped(payload, c, pix)
                    && append_json_escaped(payload, c, path)
                    && append_json_escaped(payload, c, suffix.ok_or(FetchError::Network)?)
            }
            && write_bytes(payload, c, br#""}}"#);
        if !ok {
            return Err(FetchError::Network);
        }
        *written += 1;
        added += 1;
        pos = end + 1;
        if pos + 2 <= unpacked.len() && &unpacked[pos..pos + 2] == b",\"" {
            pos += 2;
        } else {
            break;
        }
    }
    Ok(added)
}


// Read the site's explicit concatenation literal; never regenerate signed fields.
fn original_image_suffix<'a>(
    unpacked: &'a [u8],
    cid: &[u8],
    key: &[u8],
) -> Result<&'a [u8], FetchError> {
    let marker = b"pvalue[i]=pix+pvalue[i]+'";
    let start = find_subslice(unpacked, marker).ok_or(FetchError::Network)? + marker.len();
    if find_subslice(&unpacked[start..], marker).is_some() {
        return Err(FetchError::Network);
    }
    let end = find_subslice(&unpacked[start..], b"'").ok_or(FetchError::Network)? + start;
    let suffix = &unpacked[start..end];
    if suffix.len() > 1024
        || suffix.iter().any(|b| !(0x21..=0x7e).contains(b) || matches!(b, b'\\' | b'"' | b'#' | b'<' | b'>'))
        || !matches!(unpacked.get(end + 1), Some(b';' | b'}'))
    {
        return Err(FetchError::Network);
    }
    let rest = suffix.strip_prefix(b"?cid=")
        .and_then(|v| v.strip_prefix(cid))
        .and_then(|v| v.strip_prefix(b"&key="))
        .and_then(|v| v.strip_prefix(key))
        .ok_or(FetchError::Network)?;
    if !rest.is_empty() && !rest.starts_with(b"&") {
        return Err(FetchError::Network);
    }
    Ok(suffix)
}

fn is_word(b: u8) -> bool {
    (b >= b'0' && b <= b'9') || (b >= b'a' && b <= b'z') || (b >= b'A' && b <= b'Z') || b == b'_'
}

fn parse_base(bytes: &[u8], base: usize) -> usize {
    let mut n = 0usize;
    for &b in bytes {
        let v = match b {
            b'0'..=b'9' => (b - b'0') as usize,
            b'a'..=b'z' => (b - b'a' + 10) as usize,
            b'A'..=b'Z' => (b - b'A' + 36) as usize,
            _ => return usize::MAX,
        };
        if v >= base {
            return usize::MAX;
        }
        n = n * base + v;
    }
    n
}

fn extract_js_string<'a>(data: &'a [u8], marker: &[u8]) -> Option<&'a [u8]> {
    let start = find_subslice(data, marker)? + marker.len();
    let end = find_subslice(&data[start..], b"\"")? + start;
    Some(&data[start..end])
}

fn extract_js_single_quoted_string<'a>(data: &'a [u8], marker: &[u8]) -> Option<&'a [u8]> {
    let start = find_subslice(data, marker)? + marker.len();
    let end = find_subslice(&data[start..], b"'")? + start;
    Some(&data[start..end])
}

fn extract_query_value<'a>(url: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let mut marker = [0u8; 64];
    if key.len() + 1 > marker.len() {
        return None;
    }
    marker[..key.len()].copy_from_slice(key);
    marker[key.len()] = b'=';
    let start = find_subslice(url, &marker[..key.len() + 1])? + key.len() + 1;
    let mut end = start;
    while end < url.len() && url[end] != b'&' && url[end] != b'#' {
        end += 1;
    }
    if end > start {
        Some(&url[start..end])
    } else {
        None
    }
}

fn extract_js_number<'a>(data: &'a [u8], marker: &[u8]) -> Option<&'a [u8]> {
    let start = find_subslice(data, marker)? + marker.len();
    let mut end = start;
    while end < data.len() && data[end] >= b'0' && data[end] <= b'9' {
        end += 1;
    }
    if end > start {
        Some(&data[start..end])
    } else {
        None
    }
}

#[derive(Clone, Copy)]
enum Listing { Popular, Latest }

impl Listing {
    fn id(self) -> &'static [u8] {
        match self { Self::Popular => b"popular", Self::Latest => b"latest" }
    }
    fn prefix(self) -> &'static [u8] {
        match self { Self::Popular => b"/manhua-list-p", Self::Latest => b"/manhua-list-s2-p" }
    }
    fn first(self) -> &'static [u8] {
        match self { Self::Popular => b"/manhua-list-p1/", Self::Latest => b"/manhua-list-s2-p1/" }
    }
    fn page<'a>(self, uri: &'a [u8]) -> Option<&'a [u8]> {
        let path = uri.strip_prefix(BASE_URL).unwrap_or(uri);
        if matches!(self, Self::Popular) && path == b"/manhua-list/" { return Some(b"1"); }
        if matches!(self, Self::Latest) && path == b"/manhua-list-s2/" { return Some(b"1"); }
        let digits = path.strip_prefix(self.prefix())?.strip_suffix(b"/")?;
        decimal(digits)
    }
}

// Whitelisted route/id alphabets have no JSON-escaped output characters.
// Use the SDK's Unicode/escape decoder before validating those alphabets.
fn decode_route_string<'a>(raw: &[u8], dst: &'a mut [u8]) -> Option<&'a [u8]> {
    let mut len = 0;
    append_json_unescaped_then_escaped(dst, &mut len, raw).then_some(())?;
    Some(&dst[..len])
}

fn decimal(value: &[u8]) -> Option<&[u8]> {
    if value.is_empty() || !value.iter().all(|b| b.is_ascii_digit()) { return None; }
    let start = value.iter().position(|b| *b != b'0')?;
    Some(&value[start..])
}

fn decimal_after(a: &[u8], b: &[u8]) -> bool {
    a.len() > b.len() || (a.len() == b.len() && a > b)
}

fn decimal_next(current: &[u8], candidate: &[u8]) -> bool {
    let mut left = current.len();
    let mut right = candidate.len();
    let mut carry = 1u8;
    while left > 0 {
        if right == 0 { return false; }
        left -= 1; right -= 1;
        let digit = current[left] - b'0' + carry;
        if candidate[right] != b'0' + digit % 10 { return false; }
        carry = digit / 10;
    }
    if carry == 1 { right == 1 && candidate[0] == b'1' } else { right == 0 }
}

struct DirectoryDocument(host::HtmlDescriptor);
impl Drop for DirectoryDocument {
    fn drop(&mut self) { let _ = host::html_close(self.0); }
}

#[link(wasm_import_module = "koma_host")]
extern "C" {
    #[link_name = "html_select_all"]
    fn directory_select_all(parent: i32, selector: *const u8, selector_len: u32, out: *mut u8, out_cap: u32) -> i32;
}

fn select_directory_nodes(parent: host::HtmlDescriptor, dst: &mut [u8]) -> i32 {
    let selector = b".page-pagination a[data-index][href]";
    unsafe { directory_select_all(parent.raw(), selector.as_ptr(), selector.len() as u32, dst.as_mut_ptr(), dst.len() as u32) }
}

struct DirectoryNodes<'a> { bytes: &'a [u8], count: usize }
impl DirectoryNodes<'_> {
    fn at(&self, index: usize) -> Option<host::HtmlDescriptor> {
        if index >= self.count { return None; }
        let at = index * 4;
        let raw = i32::from_le_bytes([self.bytes[at], self.bytes[at+1], self.bytes[at+2], self.bytes[at+3]]);
        if raw <= 0 { return None; }
        Some(unsafe { core::mem::transmute(raw) })
    }
}
impl Drop for DirectoryNodes<'_> {
    fn drop(&mut self) {
        for index in 0..self.count { if let Some(h) = self.at(index) { let _ = host::html_close(h); } }
    }
}

fn directory_next(html: &[u8], listing: Listing, current: &[u8], dst: &mut [u8]) -> Result<usize, &'static str> {
    let document = DirectoryDocument(host::html_parse(html).map_err(|_| "directory HTML parse failed")?);
    let count = select_directory_nodes(document.0, &mut scratch_b()[..0]);
    if count <= 0 { return Err("missing directory pagination"); }
    let capacity = (count as usize).checked_mul(4).filter(|n| *n <= BODY_CAP).ok_or("pagination capacity exceeded")?;
    // Card parsing is complete; Lexbor owns its DOM storage independently.
    let handles = &mut body_buf()[..capacity];
    let actual = select_directory_nodes(document.0, handles);
    if actual < 0 { return Err("directory pagination selection failed"); }
    let nodes = DirectoryNodes { bytes: handles, count: (actual as usize).min(capacity / 4) };
    if actual != count { return Err("directory pagination changed"); }
    let mut active_seen = false;
    let mut forward_seen = false;
    let mut next_len = 0usize;
    for index in 0..nodes.count {
        let node = nodes.at(index).ok_or("missing pagination node")?;
        let work = scratch_b();
        let active = host::html_attr(node, b"class", work).ok()
            .map(|n| work[..n].split(|b| b.is_ascii_whitespace()).any(|token| token == b"active")).unwrap_or(false);
        let index_len = host::html_attr(node, b"data-index", work).map_err(|_| "missing pagination index")?;
        let page = decimal(&work[..index_len]).ok_or("invalid pagination index")?;
        let href_out = &mut http_out()[..PAYLOAD_CAP];
        let href_len = host::html_attr(node, b"href", href_out).map_err(|_| "missing pagination href")?;
        if href_len == href_out.len() { return Err("pagination href capacity exceeded"); }
        let href = &href_out[..href_len];
        let route_page = listing.page(href).ok_or("pagination route is outside listing")?;
        if route_page != page { return Err("pagination index and route differ"); }
        if active {
            if page != current { return Err("directory current page differs"); }
            active_seen = true;
        }
        if decimal_after(page, current) { forward_seen = true; }
        if decimal_next(current, page) {
            if next_len > 0 && listing_path(&dst[..next_len]) != listing_path(href) {
                return Err("conflicting directory next links");
            }
            if next_len == 0 {
                let mut len = 0;
                if !write_bytes(dst, &mut len, href) { return Err("pagination cursor capacity exceeded"); }
                next_len = len;
            }
        }
    }
    if !active_seen { return Err("missing directory current page"); }
    if next_len == 0 && forward_seen { return Err("missing consecutive directory next link"); }
    Ok(next_len)
}

fn listing_path(uri: &[u8]) -> &[u8] { uri.strip_prefix(BASE_URL).unwrap_or(uri) }

fn run_get_listings(_req: &[u8]) -> u32 {
    let json = r#"{"listings":[{"id":"popular","name":"热门漫画","kind":"popular"},{"id":"latest","name":"最新更新","kind":"latest"}]}"#.as_bytes();
    payload_buf()[..json.len()].copy_from_slice(json);
    write_success_payload("get_listings", json.len())
}

fn run_get_manga_list(req: &[u8]) -> u32 {
    let args = json_object_member(req, b"args").unwrap_or(req);
    let raw_id = extract_json_string(args, b"listingId").unwrap_or(b"popular");
    let id = match decode_route_string(raw_id, scratch_b()) {
        Some(id) => id,
        None => return write_error("get_manga_list", "invalid_request", "listing ID capacity exceeded"),
    };
    let listing = match id {
        b"popular" | b"listing:popular" => Listing::Popular,
        b"latest" | b"listing:latest" => Listing::Latest,
        _ => return write_error("get_manga_list", "invalid_request", "unknown listing"),
    };
    let raw_page = json_object_member(args, b"page");
    let raw_cursor = raw_page.and_then(|page| json_object_member(page, b"cursor"));
    if let Some(value) = raw_cursor {
        if value != b"null" && !(value.starts_with(b"\"") && value.ends_with(b"\"")) {
            return write_error("get_manga_list", "invalid_request", "cursor must be a string or null");
        }
    }
    let cursor = raw_cursor.filter(|raw| *raw != b"null")
        .and_then(|raw| raw.strip_prefix(b"\"").and_then(|v| v.strip_suffix(b"\"")));
    let path = if let Some(raw) = cursor {
        let value = match decode_route_string(raw, scratch_b()) {
            Some(value) => value,
            None => return write_error("get_manga_list", "invalid_request", "cursor capacity exceeded"),
        };
        if decimal(value).is_some() {
            let url = scratch_a(); let mut n = 0;
            if !(write_bytes(url, &mut n, listing.prefix()) && write_bytes(url, &mut n, value) && write_bytes(url, &mut n, b"/")) {
                return write_error("get_manga_list", "internal_error", "URL capacity exceeded");
            }
            unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), n) }
        } else {
            if listing.page(value).is_none() { return write_error("get_manga_list", "invalid_request", "cursor is outside listing"); }
            let mut n = 0;
            if !write_bytes(scratch_a(), &mut n, listing_path(value)) { return write_error("get_manga_list", "internal_error", "URL capacity exceeded"); }
            unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), n) }
        }
    } else if let Some(page) = raw_page.filter(|p| p.first() != Some(&b'{')) {
        let value = match decimal(page) {
            Some(value) => value,
            None => return write_error("get_manga_list", "invalid_request", "invalid numeric page"),
        };
        let mut n = 0;
        if !(write_bytes(scratch_a(), &mut n, listing.prefix()) && write_bytes(scratch_a(), &mut n, value) && write_bytes(scratch_a(), &mut n, b"/")) {
            return write_error("get_manga_list", "internal_error", "URL capacity exceeded");
        }
        unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), n) }
    } else { listing.first() };
    let mut n = 0;
    if !(write_bytes(scratch_b(), &mut n, BASE_URL) && write_bytes(scratch_b(), &mut n, path)) {
        return write_error("get_manga_list", "internal_error", "URL capacity exceeded");
    }
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_B.as_ptr(), n) };
    let page_value = match listing.page(url) {
        Some(value) => value,
        None => return write_error("get_manga_list", "invalid_request", "invalid listing route"),
    };
    // Keep current-page digits outside the HTTP/body buffers and URL scratch.
    let mut current_size = 0;
    if !write_bytes(scratch_a(), &mut current_size, page_value) {
        return write_error("get_manga_list", "internal_error", "page capacity exceeded");
    }
    let current = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), current_size) };
    let html_len = match fetch_body(url, None) {
        Ok(n) => n,
        Err(error) => { let (code, message) = fetch_error_code(error); return write_error("get_manga_list", code, message); }
    };
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };
    let payload = payload_buf(); let mut c = 0;
    if !(write_bytes(payload, &mut c, br#"{"listingId":""#) && write_bytes(payload, &mut c, listing.id()) && write_bytes(payload, &mut c, br#"","items":["#)) {
        return write_error("get_manga_list", "internal_error", "payload overflow");
    }
    if let Err(message) = parse_cards(html, payload, &mut c, b"dm5") {
        return write_error("get_manga_list", "parse_error", message);
    }
    let cursor_len = match directory_next(html, listing, current, http_req_buf()) {
        Ok(len) => len,
        Err(message) => return write_error("get_manga_list", "parse_error", message),
    };
    let has_more = cursor_len > 0;
    let ok = write_bytes(payload, &mut c, br#"],"page":{"nextCursor":"#)
        && if has_more { write_bytes(payload, &mut c, b"\"") && append_json_escaped(payload, &mut c, &http_req_buf()[..cursor_len]) && write_bytes(payload, &mut c, b"\"") } else { write_bytes(payload, &mut c, b"null") }
        && write_bytes(payload, &mut c, br#", "hasMore":"#)
        && write_bytes(payload, &mut c, if has_more { b"true" } else { b"false" })
        && write_bytes(payload, &mut c, b"}}");
    if !ok { return write_error("get_manga_list", "internal_error", "payload overflow"); }
    write_success_payload("get_manga_list", c)
}

// Home is a preview of the source's real lists, not an empty navigation gate.
// Preserve both listing IDs and their complete/paginated routes; bounded cards
// here use the very same parser/identity as those routes. 2026-10-26 user report.
fn run_get_home(_req: &[u8]) -> u32 {
    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"sections":["#) {
        return write_error("get_home", "internal_error", "payload overflow");
    }
    for (index, listing, title) in [(0usize, Listing::Popular, "热门漫画"), (1usize, Listing::Latest, "最新更新")] {
        let mut url_len = 0usize;
        if !(write_bytes(scratch_b(), &mut url_len, BASE_URL) && write_bytes(scratch_b(), &mut url_len, listing.first())) {
            return write_error("get_home", "internal_error", "URL capacity exceeded");
        }
        let url = unsafe { core::slice::from_raw_parts(SCRATCH_B.as_ptr(), url_len) };
        let html_len = match fetch_body(url, None) {
            Ok(len) => len,
            Err(error) => { let (code, message) = fetch_error_code(error); return write_error("get_home", code, message); }
        };
        let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };
        if index > 0 && !write_bytes(payload, &mut c, b",") {
            return write_error("get_home", "internal_error", "payload overflow");
        }
        if !(write_bytes(payload, &mut c, br#"{"id":"home:"#) && write_bytes(payload, &mut c, listing.id())
            && write_bytes(payload, &mut c, br#"","title":""#) && append_json_escaped(payload, &mut c, title.as_bytes())
            && write_bytes(payload, &mut c, br#"","kind":"mangaList","listingId":""#)
            && write_bytes(payload, &mut c, listing.id()) && write_bytes(payload, &mut c, br#"","items":["#)) {
            return write_error("get_home", "internal_error", "payload overflow");
        }
        match parse_cards_with_limit(html, payload, &mut c, b"dm5", 12) {
            Ok(0) => return write_error("get_home", "parse_error", "home list contains no manga"),
            Ok(_) => {},
            Err(message) => return write_error("get_home", "parse_error", message),
        }
        if !write_bytes(payload, &mut c, b"]}") {
            return write_error("get_home", "internal_error", "payload overflow");
        }
    }
    if !write_bytes(payload, &mut c, b"]}") { return write_error("get_home", "internal_error", "payload overflow"); }
    write_success_payload("get_home", c)
}

fn run_get_filters(_req: &[u8]) -> u32 {
    const FILTERS: &[u8] = br#"{"filters":[]}"#;
    let payload = payload_buf();
    if FILTERS.len() > payload.len() {
        return write_error("get_filters", "internal_error", "payload overflow");
    }
    payload[..FILTERS.len()].copy_from_slice(FILTERS);
    write_success_payload("get_filters", FILTERS.len())
}

fn run_get_settings(req: &[u8]) -> u32 {
    let payload = payload_buf(); let mut c = 0;
    let sort = "{\"settings\":[{\"id\":\"sortChapter\",\"name\":\"依照上傳時間排序章節\",\"kind\":\"toggle\",\"description\":\"依網站章節編號由新到舊排列，預設保留網站原順序\",\"default\":\"false\"}";
    if !write_bytes(payload, &mut c, sort.as_bytes()) { return write_error("get_settings", "internal_error", "payload overflow"); }
    if supports_text_pages(req) {
        let comments = ",{\"id\":\"chapterComments\",\"name\":\"章末吐槽頁\",\"kind\":\"toggle\",\"description\":\"在章節原圖後顯示網站吐槽；對之後載入的章節生效\",\"default\":\"false\"}";
        if !write_bytes(payload, &mut c, comments.as_bytes()) { return write_error("get_settings", "internal_error", "payload overflow"); }
    }
    if !write_bytes(payload, &mut c, b"]}") { return write_error("get_settings", "internal_error", "payload overflow"); }
    write_success_payload("get_settings", c)
}

// Covers are emitted by catalogue/detail metadata and have no chapter cid.
// This role trusts that metadata pairing; it does not fetch/revalidate the pairing.
fn cover_manga_slug(id: &[u8]) -> Option<&[u8]> {
    let slug = id.strip_prefix(b"manga:")?;
    if !slug.starts_with(b"manhua-") || slug.len() <= 7 || slug.len() > 160
        || !slug.iter().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        return None;
    }
    Some(slug)
}

fn valid_cover_url(url: &[u8]) -> bool {
    let rest = match url.strip_prefix(b"https://") { Some(v) => v, None => return false };
    let slash = match rest.iter().position(|b| *b == b'/') { Some(v) => v, None => return false };
    let hostname = &rest[..slash];
    let label = match hostname.strip_suffix(b".cdndm5.com") { Some(v) => v, None => return false };
    if !label.starts_with(b"mhfm") || label.len() <= 4 || label.len() > 64
        || !label.iter().all(|b| b.is_ascii_alphanumeric() || *b == b'-')
    {
        return false;
    }
    let mut parts = rest[slash + 1..].split(|b| *b == b'/');
    for _ in 0..2 {
        match parts.next() {
            Some(v) if !v.is_empty() && v.iter().all(u8::is_ascii_digit) => {},
            _ => return false,
        }
    }
    let file = match parts.next().and_then(|v| {
        v.strip_suffix(b".jpg").or_else(|| v.strip_suffix(b".jpeg"))
    }) {
        Some(v) => v, None => return false,
    };
    parts.next().is_none() && !file.is_empty() && file.len() <= 128
        && file.iter().all(|b| b.is_ascii_digit() || matches!(b, b'_' | b'x'))
}

fn write_cover_image_request(req: &[u8], id: &[u8], url: &[u8]) -> u32 {
    let slug = match cover_manga_slug(id) {
        Some(v) => v,
        None => return write_error("get_image_request", "invalid_request", "invalid cover owner"),
    };
    if !valid_cover_url(url) {
        return write_error("get_image_request", "invalid_request", "unsupported cover url");
    }
    let referer = scratch_a();
    let mut n = 0;
    if !(write_bytes(referer, &mut n, BASE_URL) && write_bytes(referer, &mut n, b"/")
        && write_bytes(referer, &mut n, slug) && write_bytes(referer, &mut n, b"/"))
    {
        return write_error("get_image_request", "internal_error", "cover referer overflow");
    }
    if let Some(owner_url) = extract_json_string(req, b"mangaUrl") {
        if owner_url != &referer[..n] {
            return write_error("get_image_request", "invalid_request", "cover owner mismatch");
        }
    }
    let payload = payload_buf();
    let mut c = 0;
    let ok = write_bytes(payload, &mut c, br#"{"url":""#)
        && append_json_escaped(payload, &mut c, url)
        && write_bytes(payload, &mut c, br#"","headers":{"Referer":""#)
        && append_json_escaped(payload, &mut c, &referer[..n])
        && write_bytes(payload, &mut c, br#"","Accept-Language":"zh-TW","User-Agent":"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/139.0.0.0 Safari/537.36"}}"#);
    if !ok { return write_error("get_image_request", "internal_error", "payload overflow"); }
    write_success_payload("get_image_request", c)
}

fn run_get_image_request(req: &[u8]) -> u32 {
    let url = match extract_json_string(req, b"url") {
        Some(v) => v,
        None => return write_error("get_image_request", "invalid_request", "missing url"),
    };
    if let Some(id) = extract_json_string(req, b"pageId") {
        if id.starts_with(b"manga:") {
            return write_cover_image_request(req, id, url);
        }
    }
    let cid = match extract_query_value(url, b"cid") {
        Some(v) => v,
        None => return write_error("get_image_request", "invalid_request", "missing cid"),
    };
    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"url":""#)
        && append_json_escaped(payload, &mut c, url)
        && write_bytes(payload, &mut c, br#"","headers":{"Referer":"https://www.dm5.cn/m"#)
        && append_json_escaped(payload, &mut c, cid)
        && write_bytes(payload, &mut c, br#"/","Accept-Language":"zh-TW","User-Agent":"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/139.0.0.0 Safari/537.36"}}"#);
    if !ok {
        return write_error("get_image_request", "internal_error", "payload overflow");
    }
    write_success_payload("get_image_request", c)
}

#[no_mangle]
pub extern "C" fn koma_source_init(_manifest_ptr: u32, manifest_len: u32) -> i32 {
    log_info(b"DM5 source init");
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
    log_info(b"DM5 search");
    run_search(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga", "invalid_request", "empty request"),
    };
    log_info(b"DM5 get_manga");
    run_get_manga(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_chapters(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_chapters", "invalid_request", "empty request"),
    };
    log_info(b"DM5 get_chapters");
    run_get_chapters(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_pages(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_pages", "invalid_request", "empty request"),
    };
    log_info(b"DM5 get_pages");
    run_get_pages(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_listings(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_listings", "invalid_request", "empty request"),
    };
    log_info(b"DM5 get_listings");
    run_get_listings(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga_list(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga_list", "invalid_request", "empty request"),
    };
    log_info(b"DM5 get_manga_list");
    run_get_manga_list(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_home(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_home", "invalid_request", "empty request"),
    };
    log_info(b"DM5 get_home");
    run_get_home(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_filters(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_filters", "invalid_request", "empty request"),
    };
    log_info(b"DM5 get_filters");
    run_get_filters(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_settings(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_settings", "invalid_request", "empty request"),
    };
    log_info(b"DM5 get_settings");
    run_get_settings(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_image_request(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_image_request", "invalid_request", "empty request"),
    };
    log_info(b"DM5 get_image_request");
    run_get_image_request(req)
}

#[no_mangle]
pub extern "C" fn koma_source_free(result_ptr: u32) {
    response_buffer().free(result_ptr)
}
