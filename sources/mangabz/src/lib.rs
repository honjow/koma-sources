#![no_std]

extern crate koma_source_sdk;

use koma_source_sdk::host::{self, http_request, log_info};
use koma_source_sdk::json_utils::{
    append_json_escaped, contains_bytes, extract_json_number, extract_json_string, find_subslice,
    json_object_member, write_bytes, write_url_encoded, write_usize,
};
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{build_get_request, fetch_error_code, FetchError};

const BASE_URL: &[u8] = b"https://www.mangabz.com";
koma_source_sdk::koma_source_buffers! {
    payload: 1024 * 1024,
    http_out: 2 * 1024 * 1024,
    body: 2 * 1024 * 1024,
    http_req: 2048,
    scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.mangabz.koma",
    name: "Mangabz",
    version: "0.1.5",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "Mangabz HTML scraping source.",
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
    image_request: true,
    credentials: false,
};

fn fetch_body(url: &[u8], referer: Option<&[u8]>) -> Result<usize, FetchError> {
    let req_len = build_get_request(
        http_req_buf(),
        url,
        referer,
        &[(b"User-Agent", b"Koma/0.1")],
    )
    .ok_or(FetchError::Network)?;
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
                    log_info(b"mangabz: http transport error, retrying");
                }
            }
        }
    }
    if transport_failed {
        return Err(FetchError::Network);
    }
    let response = &http_out()[..resp_len];
    // Native transport `ok` acknowledges the exchange, not HTTP success.
    let status = json_object_member(response, b"status")
        .or_else(|| json_object_member(response, b"statusCode"))
        .and_then(parse_usize)
        .ok_or(FetchError::Network)?;
    match status {
        200..=299 => decode_json_body(response),
        404 => Err(FetchError::NotFound),
        429 => Err(FetchError::RateLimit),
        400..=499 => Err(FetchError::ClientError),
        500..=599 => Err(FetchError::ServerError),
        _ => Err(FetchError::Network),
    }
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
    if href.len() < 4 || href[0] != b'/' {
        return None;
    }
    let mut end = 1usize;
    while end < href.len() && href[end] != b'/' && href[end] != b'?' && href[end] != b'#' {
        end += 1;
    }
    if end <= 2 || &href[end - 2..end] != b"bz" {
        return None;
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
        && write_bytes(payload, c, br#""},"authors":[],"status":"unknown","contentRating":"safe","description":"","sourceTags":["#)
        && write_bytes(payload, c, b"\"")
        && append_json_escaped(payload, c, tag)
        && write_bytes(payload, c, br#""]}"#)
}

fn parse_cards(html: &[u8], payload: &mut [u8], c: &mut usize, tag: &[u8]) -> Result<usize, &'static str> {
    let mut pos = 0usize;
    let mut written = 0usize;
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
        let title = match attr_value(a_tag, b"title") {
            Some(v) => v,
            None => continue,
        };
        let mut cover = b"" as &[u8];
        let search_start = a_start.saturating_sub(700);
        let before = &html[search_start..a_start];
        if let Some(img_rel) = find_last_subslice(before, b"<img") {
            let img_start = search_start + img_rel;
            let img_end = find_subslice(&html[img_start..], b">")
                .map(|v| img_start + v + 1)
                .unwrap_or(a_start);
            cover = attr_value(&html[img_start..img_end], b"src").unwrap_or(b"");
        }
        if written > 0 && !write_bytes(payload, c, b",") {
            return Err("payload overflow");
        }
        if !write_card_item(payload, c, id, title, cover, tag) {
            return Err("payload overflow");
        }
        written += 1;
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

fn write_manga_page_from_url(operation: &str, url: &[u8], page: usize) -> u32 {
    let html_len = match fetch_body(url, None) {
        Ok(n) => n,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error(operation, c, m);
        }
    };
    let html = unsafe { core::slice::from_raw_parts(BODY_BUF.as_ptr(), html_len) };
    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return write_error(operation, "internal_error", "payload overflow");
    }
    let written = match parse_cards(html, payload, &mut c, b"mangabz") {
        Ok(count) => count,
        Err(message) => return write_error(operation, "parse_error", message),
    };
    let mut marker = [0u8; 64];
    let mut m = 0;
    write_bytes(&mut marker, &mut m, b"data-index=\"");
    write_usize(&mut marker, &mut m, page + 1);
    write_bytes(&mut marker, &mut m, b"\"");
    let has_more = written > 0 && contains_bytes(html, &marker[..m]);
    let ok = write_bytes(payload, &mut c, br#"],"page":{"nextCursor":"#)
        && if has_more {
            write_bytes(payload, &mut c, b"\"")
                && write_usize(payload, &mut c, page + 1)
                && write_bytes(payload, &mut c, b"\"")
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

fn run_search(req: &[u8]) -> u32 {
    let query = extract_json_string(req, b"query").unwrap_or(b"");
    if query.is_empty() {
        return run_manga_list(req, "search");
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
    let page = match request_page(req) {
        Some(v) => v,
        None => return write_error("search", "invalid_request", "invalid page cursor"),
    };
    let mut c = 0usize;
    let url_buf = scratch_a();
    if !(write_bytes(url_buf, &mut c, BASE_URL)
        && write_bytes(url_buf, &mut c, b"/search?title=")
        && write_url_encoded(url_buf, &mut c, query)
        && write_bytes(url_buf, &mut c, b"&page=")
        && write_usize(url_buf, &mut c, page))
    {
        return write_error("search", "internal_error", "url overflow");
    }
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), c) };
    write_manga_page_from_url("search", url, page)
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
    let title = extract_between_text(
        html,
        b"class=\"detail-info-title\"",
        b"</p>",
        &mut title_buf,
    )
    .unwrap_or(id);
    let cover = extract_attr_after(html, b"class=\"detail-info-cover\"", b"src").unwrap_or(b"");
    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"items":["#)
        && write_card_item(payload, &mut c, id, title, cover, b"mangabz")
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

// Detail-only cleanup: retain hidden synopsis continuation, omit fold controls.
fn clean_description<'a>(src: &[u8], out: &'a mut [u8]) -> Option<&'a [u8]> {
    let mut i = 0usize;
    let mut n = 0usize;
    let mut space = false;
    while i < src.len() {
        if src[i] == b'<' {
            let end = find_subslice(&src[i..], b">")? + i + 1;
            let tag = &src[i..end];
            let class = attr_value(tag, b"class").unwrap_or(b"");
            if tag.starts_with(b"<a ") && (class == b"fold_open" || class == b"fold_close") {
                i = find_subslice(&src[end..], b"</a>")? + end + 4;
            } else {
                i = end;
            }
            continue;
        }
        if src[i].is_ascii_whitespace() {
            space = n > 0;
            i += 1;
            continue;
        }
        if space {
            if n >= out.len() {
                return None;
            }
            out[n] = b' ';
            n += 1;
            space = false;
        }
        if n >= out.len() {
            return None;
        }
        out[n] = src[i];
        n += 1;
        i += 1;
    }
    Some(&out[..n])
}

fn detail_description<'a>(html: &[u8], out: &'a mut [u8]) -> Option<&'a [u8]> {
    let p = find_subslice(html, b"class=\"detail-info-content\"")?;
    let start = find_subslice(&html[p..], b">")? + p + 1;
    let end = find_subslice(&html[start..], b"</p>")? + start;
    clean_description(&html[start..end], out)
}

fn write_detail_authors(html: &[u8], payload: &mut [u8], cursor: &mut usize) -> bool {
    let p = match find_subslice(html, b"class=\"detail-info-tip\"") {
        Some(v) => v,
        None => return true,
    };
    let end = match find_subslice(&html[p..], b"</p>") {
        Some(v) => p + v,
        None => return true,
    };
    let tips = &html[p..end];
    let span = match find_subslice(tips, b"<span") {
        Some(v) => v,
        None => return true,
    };
    let span_end = match find_subslice(&tips[span..], b"</span>") {
        Some(v) => span + v,
        None => return true,
    };
    let authors = &tips[span..span_end];
    let mut i = 0usize;
    let mut count = 0usize;
    while let Some(relative) = find_subslice(&authors[i..], b"<a ") {
        let start = i + relative;
        let text_start = match find_subslice(&authors[start..], b">") {
            Some(v) => start + v + 1,
            None => break,
        };
        let text_end = match find_subslice(&authors[text_start..], b"</a>") {
            Some(v) => text_start + v,
            None => break,
        };
        i = text_end + 4;
        let mut text_buf = [0u8; 512];
        let text = strip_tags_to(&authors[text_start..text_end], &mut text_buf);
        if text.is_empty() {
            continue;
        }
        if count > 0 && !write_bytes(payload, cursor, b",") {
            return false;
        }
        if !write_bytes(payload, cursor, b"\"")
            || !append_json_escaped(payload, cursor, text)
            || !write_bytes(payload, cursor, b"\"")
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
    let title = extract_between_text(
        html,
        b"class=\"detail-info-title\"",
        b"</p>",
        &mut title_buf,
    )
    .unwrap_or(id);
    let desc = detail_description(html, &mut desc_buf).unwrap_or(b"");
    let cover = extract_attr_after(html, b"class=\"detail-info-cover\"", b"src").unwrap_or(b"");
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
        && write_bytes(payload, &mut c, br#""},"authors":["#)
        && write_detail_authors(html, payload, &mut c)
        && write_bytes(payload, &mut c, br#"],"artists":[],"status":""#)
        && write_bytes(payload, &mut c, status)
        && write_bytes(payload, &mut c, br#"","contentRating":"safe","language":"zh","tags":[],"links":[{"kind":"source","url":""#)
        && append_json_escaped(payload, &mut c, url)
        && write_bytes(payload, &mut c, br#""}]}}"#);
    if !ok {
        return write_error("get_manga", "internal_error", "payload overflow");
    }
    write_success_payload("get_manga", c)
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
    let mut pos = 0usize;
    let mut written = 0usize;
    while let Some(rel) = find_subslice(&list[pos..], b"<a ") {
        let a_start = pos + rel;
        let a_end = match find_subslice(&list[a_start..], b"</a>") {
            Some(v) => a_start + v + 4,
            None => break,
        };
        let block = &list[a_start..a_end];
        pos = a_end;
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
        let mut title_buf = [0u8; 256];
        let title = strip_tags_to(
            &block[tag_end..block.len().saturating_sub(4)],
            &mut title_buf,
        );
        let page_count = extract_page_count(title);
        let sequence = chapter_sequence(title);
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
            && append_json_escaped(payload, &mut c, title)
            && write_bytes(payload, &mut c, br#"","chapterNumber":"#)
            && write_chapter_sequence_number(payload, &mut c, sequence, false)
            && write_bytes(payload, &mut c, br#", "volumeNumber":"#)
            && write_chapter_sequence_number(payload, &mut c, sequence, true)
            && write_bytes(payload, &mut c, br#", "readingGroup":"#)
            && match sequence {
                Some((_, true)) => write_bytes(payload, &mut c, b"\"volume\""),
                Some((_, false)) => write_bytes(payload, &mut c, b"\"chapter\""),
                None => write_bytes(payload, &mut c, b"null"),
            }
            && write_bytes(payload, &mut c, br#", "continuationEligible":"#)
            && write_bytes(payload, &mut c, if sequence.is_some() { b"true" } else { b"false" })
            && write_bytes(payload, &mut c, br#", "publishedAt":null,"updatedAt":null,"pageCount":"#)
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

// The site mixes volumes, regular episodes and extras in one parent list.
// Only an explicit leading chapter/volume label supplies a sequence; neither
// the resource cid nor a trailing page count is a chapter ordinal.
fn chapter_sequence(title: &[u8]) -> Option<(&[u8], bool)> {
    let title = trim_ascii(title);
    let bytes = title.strip_prefix("第".as_bytes()).unwrap_or(title);
    let mut end = 0;
    while end < bytes.len() && bytes[end].is_ascii_digit() { end += 1; }
    if end == 0 { return None; }
    if bytes.get(end) == Some(&b'.') {
        end += 1;
        let start = end;
        while end < bytes.len() && bytes[end].is_ascii_digit() { end += 1; }
        if end == start { return None; }
    }
    let suffix = &bytes[end..];
    if let Some(rest) = suffix.strip_prefix("卷".as_bytes()) {
        // "20卷 附錄" is an extra, not the complete numbered volume.
        let rest = trim_ascii(rest);
        let rest = rest.strip_prefix("（".as_bytes())
            .and_then(|v| v.strip_suffix("P）".as_bytes()))
            .or_else(|| rest.strip_prefix(b"(").and_then(|v| v.strip_suffix(b"P)")));
        if trim_ascii(suffix.strip_prefix("卷".as_bytes())?).is_empty()
            || rest.map(|v| !v.is_empty() && v.iter().all(u8::is_ascii_digit)).unwrap_or(false)
        { return Some((&bytes[..end], true)); }
        return None;
    }
    if suffix.starts_with("話".as_bytes()) || suffix.starts_with("话".as_bytes())
        || suffix.starts_with("章".as_bytes())
    { return Some((&bytes[..end], false)); }
    None
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
    let encoded_page_count = sep2.and_then(|v| parse_usize(&tail[v + 1..]));
    let page_count = match extract_json_number(req, b"pageCount")
        .and_then(parse_usize)
        .or(encoded_page_count)
    {
        Some(n) if n > 0 && n <= 5000 => n,
        _ => {
            return write_error(
                "get_pages",
                "invalid_request",
                "missing or invalid known page count",
            )
        }
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
        let got = match fetch_chapter_images(manga, cid, page, payload, &mut c, &mut written) {
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
        return write_error(
            "get_pages",
            "parse_error",
            "incomplete public chapter image list",
        );
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
        n = n.checked_mul(10)?.checked_add((b - b'0') as usize)?;
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
        && write_bytes(url_buf, &mut u, b"/chapterimage.ashx?cid=")
        && write_bytes(url_buf, &mut u, cid)
        && write_bytes(url_buf, &mut u, b"&page=")
        && write_usize(url_buf, &mut u, page))
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
    let arr_start = find_subslice(unpacked, b"pvalue=[\"").ok_or(FetchError::Network)? + 9;
    let mut pos = arr_start;
    let mut added = 0usize;
    while pos < unpacked.len() {
        let end = match find_subslice(&unpacked[pos..], b"\"") {
            Some(v) => pos + v,
            None => break,
        };
        let path = &unpacked[pos..end];
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
            && append_json_escaped(payload, c, pix)
            && append_json_escaped(payload, c, path)
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
        n = match n.checked_mul(base).and_then(|n| n.checked_add(v)) {
            Some(n) => n,
            None => return usize::MAX,
        };
    }
    n
}

fn extract_js_string<'a>(data: &'a [u8], marker: &[u8]) -> Option<&'a [u8]> {
    let start = find_subslice(data, marker)? + marker.len();
    let end = find_subslice(&data[start..], b"\"")? + start;
    Some(&data[start..end])
}

fn run_get_listings(_req: &[u8]) -> u32 {
    let data = r#"{"listings":[{"id":"popular","name":"熱門漫畫","supportsPaging":true},{"id":"latest","name":"最新更新","supportsPaging":true}]}"#.as_bytes();
    payload_buf()[..data.len()].copy_from_slice(data);
    write_success_payload("get_listings", data.len())
}

fn request_page(req: &[u8]) -> Option<usize> {
    let args = json_object_member(req, b"args").unwrap_or(req);
    let raw = match json_object_member(args, b"page") {
        Some(v) => v,
        None => return Some(1),
    };
    let bytes = if raw.first() == Some(&b'{') {
        let cursor = json_object_member(raw, b"cursor").unwrap_or(b"null");
        if cursor == b"null" {
            return Some(1);
        }
        if cursor.len() < 2 || cursor.first() != Some(&b'"') || cursor.last() != Some(&b'"') {
            return None;
        }
        &cursor[1..cursor.len() - 1]
    } else {
        raw
    };
    let page = parse_usize(bytes)?;
    if page == 0 || page > 100000 {
        None
    } else {
        Some(page)
    }
}

fn run_get_manga_list(req: &[u8]) -> u32 {
    run_manga_list(req, "get_manga_list")
}
fn run_manga_list(req: &[u8], op: &str) -> u32 {
    let listing = extract_json_string(req, b"listingId").unwrap_or(b"popular");
    if listing != b"popular" && listing != b"latest" {
        return write_error(op, "invalid_request", "unknown listing");
    }
    let page = match request_page(req) {
        Some(v) => v,
        None => return write_error(op, "invalid_request", "invalid page cursor"),
    };
    let url_buf = scratch_a();
    let mut c = 0usize;
    let ok = write_bytes(url_buf, &mut c, BASE_URL)
        && if listing == b"latest" {
            write_bytes(url_buf, &mut c, b"/manga-list-0-0-2-p")
        } else {
            write_bytes(url_buf, &mut c, b"/manga-list-p")
        }
        && write_usize(url_buf, &mut c, page)
        && write_bytes(url_buf, &mut c, b"/");
    if !ok {
        return write_error(op, "internal_error", "url overflow");
    }
    let url = unsafe { core::slice::from_raw_parts(SCRATCH_A.as_ptr(), c) };
    write_manga_page_from_url(op, url, page)
}

fn run_get_home(_req: &[u8]) -> u32 {
    const HOME_JSON: &str = r#"{"sections":[{"title":"热门漫画","listingId":"popular"},{"title":"最新更新","listingId":"latest"}]}"#;
    let payload = payload_buf();
    let bytes = HOME_JSON.as_bytes();
    if bytes.len() > payload.len() {
        return write_error("get_home", "internal_error", "payload overflow");
    }
    payload[..bytes.len()].copy_from_slice(bytes);
    write_success_payload("get_home", bytes.len())
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

fn run_get_settings(_req: &[u8]) -> u32 {
    const SETTINGS: &[u8] = br#"{"settings":[]}"#;
    let payload = payload_buf();
    if SETTINGS.len() > payload.len() {
        return write_error("get_settings", "internal_error", "payload overflow");
    }
    payload[..SETTINGS.len()].copy_from_slice(SETTINGS);
    write_success_payload("get_settings", SETTINGS.len())
}

fn run_get_image_request(req: &[u8]) -> u32 {
    let url = match extract_json_string(req, b"url") {
        Some(v) => v,
        None => return write_error("get_image_request", "invalid_request", "missing url"),
    };
    let payload = payload_buf();
    let mut c = 0usize;
    let ok = write_bytes(payload, &mut c, br#"{"url":""#)
        && append_json_escaped(payload, &mut c, url)
        && write_bytes(payload, &mut c, br#"","headers":{"Referer":"https://www.mangabz.com/","User-Agent":"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0"}}"#);
    if !ok {
        return write_error("get_image_request", "internal_error", "payload overflow");
    }
    write_success_payload("get_image_request", c)
}

#[no_mangle]
pub extern "C" fn koma_source_init(_manifest_ptr: u32, manifest_len: u32) -> i32 {
    log_info(b"mangabz source init");
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
    log_info(b"mangabz search");
    run_search(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga", "invalid_request", "empty request"),
    };
    log_info(b"mangabz get_manga");
    run_get_manga(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_chapters(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_chapters", "invalid_request", "empty request"),
    };
    log_info(b"mangabz get_chapters");
    run_get_chapters(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_pages(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_pages", "invalid_request", "empty request"),
    };
    log_info(b"mangabz get_pages");
    run_get_pages(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_listings(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_listings", "invalid_request", "empty request"),
    };
    log_info(b"mangabz get_listings");
    run_get_listings(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_manga_list(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_manga_list", "invalid_request", "empty request"),
    };
    log_info(b"mangabz get_manga_list");
    run_get_manga_list(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_home(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_home", "invalid_request", "empty request"),
    };
    log_info(b"mangabz get_home");
    run_get_home(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_filters(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_filters", "invalid_request", "empty request"),
    };
    log_info(b"mangabz get_filters");
    run_get_filters(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_settings(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_settings", "invalid_request", "empty request"),
    };
    log_info(b"mangabz get_settings");
    run_get_settings(req)
}

#[no_mangle]
pub extern "C" fn koma_source_get_image_request(req_ptr: u32, req_len: u32) -> u32 {
    let req = match read_request(req_ptr, req_len) {
        Some(r) => r,
        None => return write_error("get_image_request", "invalid_request", "empty request"),
    };
    log_info(b"mangabz get_image_request");
    run_get_image_request(req)
}

#[no_mangle]
pub extern "C" fn koma_source_free(result_ptr: u32) {
    response_buffer().free(result_ptr)
}
