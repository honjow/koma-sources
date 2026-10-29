#![no_std]
extern crate koma_source_sdk;
mod nuxt;
use koma_source_sdk::host;
use koma_source_sdk::json_utils::*;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{fetch_error_code, FetchError};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.kuaikan.koma",
    name: "快看漫画",
    version: "0.1.5",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "Kuaikan public catalogue and free chapters.",
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
koma_source_sdk::koma_source_buffers! { payload:256*1024,http_out:2*1024*1024,body:1536*1024,http_req:8192,scratch:8192, }
koma_source_sdk::koma_source_helpers!();
static mut NUXT: [u8; 1024 * 1024] = [0; 1024 * 1024];
const BASE: &[u8] = b"https://www.kuaikanmanhua.com";
fn literal_string(v: &[u8]) -> Option<&[u8]> {
    if v.len() >= 2 && v[0] == b'"' && v[v.len() - 1] == b'"' {
        Some(&v[1..v.len() - 1])
    } else {
        None
    }
}
fn number(v: &[u8]) -> Option<usize> {
    core::str::from_utf8(v).ok()?.parse().ok()
}
fn field<'a>(v: &'a [u8], k: &[u8]) -> Option<&'a [u8]> {
    json_object_member(v, k)
}
fn text<'a>(v: &'a [u8], k: &[u8]) -> Option<&'a [u8]> {
    field(v, k).and_then(literal_string)
}
fn id<'a>(req: &'a [u8], key: &[u8], prefix: &[u8]) -> Option<&'a [u8]> {
    let v = extract_json_string(req, key)?;
    let v = v.strip_prefix(prefix)?;
    if !v.is_empty() && v.iter().all(u8::is_ascii_digit) {
        Some(v)
    } else {
        None
    }
}
fn body(len: usize) -> &'static [u8] {
    unsafe { core::slice::from_raw_parts(body_buf().as_ptr(), len) }
}
fn network(op: &str, e: FetchError) -> u32 {
    let (c, m) = fetch_error_code(e);
    write_error(op, c, m)
}
enum WebError {
    Network(FetchError),
    Source(&'static str),
}
impl From<&'static str> for WebError {
    fn from(message: &'static str) -> Self { Self::Source(message) }
}
fn fetch_web(path: &[u8]) -> Result<&'static [u8], WebError> {
    if host::check_cancel() {
        return Err("cancelled".into());
    }
    let b = scratch_a();
    let mut n = 0;
    if !write_bytes(b, &mut n, BASE) || !write_bytes(b, &mut n, path) {
        return Err("url overflow".into());
    }
    let url = unsafe { core::slice::from_raw_parts(b.as_ptr(), n) };
    let len = fetch_get(url, Some(BASE)).map_err(WebError::Network)?;
    let dst = unsafe { &mut *core::ptr::addr_of_mut!(NUXT) };
    let n = nuxt::decode(body(len), dst).map_err(|_| "unsupported or invalid Nuxt data")?;
    Ok(unsafe { core::slice::from_raw_parts(dst.as_ptr(), n) })
}
fn array<'a>(j: &'a [u8], key: &[u8]) -> Option<JsonArrayIter<'a>> {
    JsonArrayIter::from_raw_array(field(j, key)?)
}
fn first_data(j: &[u8]) -> Option<&[u8]> {
    array(j, b"data")?.next_object()
}
fn web_topic(topic: &[u8]) -> Result<&'static [u8], WebError> {
    let b = scratch_b();
    let mut n = 0;
    write_bytes(b, &mut n, b"/web/topic/")
        .then_some(())
        .ok_or("url overflow")?;
    write_bytes(b, &mut n, topic)
        .then_some(())
        .ok_or("url overflow")?;
    let path = unsafe { core::slice::from_raw_parts(b.as_ptr(), n) };
    let j = fetch_web(path)?;
    Ok(first_data(j).ok_or("missing topic data")?)
}
fn web_comic(topic: &[u8], comic: &[u8]) -> Result<&'static [u8], WebError> {
    let b = scratch_b();
    let mut n = 0;
    write_bytes(b, &mut n, b"/webs/comic-next/")
        .then_some(())
        .ok_or("url overflow")?;
    write_bytes(b, &mut n, comic)
        .then_some(())
        .ok_or("url overflow")?;
    let path = unsafe { core::slice::from_raw_parts(b.as_ptr(), n) };
    let j = fetch_web(path)?;
    let d = first_data(j).ok_or("missing comic data")?;
    let ti = field(d, b"topicInfo").ok_or("missing comic topic identity")?;
    if field(ti, b"id").and_then(number) != number(topic) {
        return Err("comic topic mismatch".into());
    }
    let info = field(d, b"res")
        .and_then(|v| field(v, b"data"))
        .and_then(|v| field(v, b"comic_info"))
        .ok_or("missing comic info")?;
    if field(info, b"id").and_then(number) != number(comic) {
        return Err("comic identity mismatch".into());
    }
    if field(info, b"locked") != Some(b"false")
        || field(info, b"is_free") != Some(b"true")
        || field(info, b"need_vip") == Some(b"true")
        || field(info, b"is_pay_comic") == Some(b"true")
    {
        return Err("paid or locked chapter is not supported".into());
    }
    Ok(info)
}
fn fail_web(op: &str, error: WebError) -> u32 {
    let e = match error {
        WebError::Network(error) => return network(op, error),
        WebError::Source(message) => message,
    };
    write_error(
        op,
        if e == "paid or locked chapter is not supported" {
            "permission_denied"
        } else if e == "cancelled" {
            "cancelled"
        } else {
            "source_error"
        },
        e,
    )
}
fn quoted(b: &mut [u8], n: &mut usize, v: &[u8]) -> bool {
    write_bytes(b, n, b"\"") && write_bytes(b, n, v) && write_bytes(b, n, b"\"")
}
fn card(b: &mut [u8], n: &mut usize, v: &[u8]) -> bool {
    let mid = match field(v, b"id") {
        Some(x) if number(x).is_some() => x,
        _ => return false,
    };
    let title = match text(v, b"title") {
        Some(x) => x,
        None => return false,
    };
    write_bytes(b, n, br#"{"id":"kkt:"#)
        && write_bytes(b, n, mid)
        && write_bytes(b, n, br#"","title":"#)
        && quoted(b, n, title)
        && write_bytes(b, n, br#", "cover":{"kind":"url","url":"#)
        && quoted(b, n, text(v, b"vertical_image_url").unwrap_or(b""))
        && write_bytes(b, n, br#"},"contentRating":"safe"}"#)
}
fn page_end(op: &str, n: &mut usize, next: Option<usize>) -> u32 {
    let b = payload_buf();
    let ok = write_bytes(b, n, br#"],"page":{"nextCursor":"#)
        && if let Some(next) = next {
            write_bytes(b, n, b"\"") && write_usize(b, n, next) && write_bytes(b, n, b"\"")
        } else {
            write_bytes(b, n, b"null")
        }
        && write_bytes(
            b,
            n,
            if next.is_some() {
                br#", "hasMore":true}}"#
            } else {
                br#", "hasMore":false}}"#
            },
        );
    if !ok {
        return write_error(op, "internal_error", "payload overflow");
    }
    write_success_payload(op, *n)
}
fn cursor(req: &[u8]) -> Result<Option<usize>, &'static str> {
    let args = field(req, b"args").unwrap_or(req);
    let page = match field(args, b"page") {
        Some(p) => p,
        None => return Ok(None),
    };
    let c = match field(page, b"cursor") {
        Some(c) if c != b"null" => c,
        _ => return Ok(None),
    };
    let v = literal_string(c).ok_or("cursor must be a numeric string or null")?;
    number(v).map(Some).ok_or("invalid cursor")
}
// Decode search terms separately from request scratch and the bounded WASM stack.
static mut SEARCH_QUERY: [u8; 8192] = [0; 8192];
fn run_search(req: &[u8]) -> u32 {
    let query = extract_json_string(req, b"query").unwrap_or(b"");
    let decoded = unsafe { &mut *core::ptr::addr_of_mut!(SEARCH_QUERY) };
    let query = match koma_source_sdk::plain_text::decode_json_content(query, decoded) {
        Ok(v) => v,
        Err(_) => return write_error("search", "invalid_request", "invalid query"),
    };
    if query.is_empty() {
        return browse(req, "search");
    }
    let since = match cursor(req) {
        Ok(c) => c.unwrap_or(0),
        Err(e) => return write_error("search", "invalid_request", e),
    };
    let b = scratch_a();
    let mut n = 0;
    let ok = write_bytes(b, &mut n, b"https://api.kkmh.com/v1/search/topic?q=")
        && write_url_encoded(b, &mut n, query)
        && write_bytes(b, &mut n, b"&since=")
        && write_usize(b, &mut n, since)
        && write_bytes(b, &mut n, b"&size=10");
    if !ok {
        return write_error("search", "invalid_request", "query too long");
    }
    let url = unsafe { core::slice::from_raw_parts(b.as_ptr(), n) };
    let len = match fetch_get(url, Some(BASE)) {
        Ok(n) => n,
        Err(e) => return network("search", e),
    };
    let j = body(len);
    let d = match field(j, b"data") {
        Some(d) => d,
        None => return write_error("search", "parse_error", "missing search data"),
    };
    let mut it = match array(d, b"hit") {
        Some(x) => x,
        None => return write_error("search", "parse_error", "missing search results"),
    };
    let b = payload_buf();
    let mut n = 0;
    write_bytes(b, &mut n, br#"{"items":["#);
    let mut count = 0;
    while let Some(item) = it.next_object() {
        if count > 0 && !write_bytes(b, &mut n, b",") {
            return write_error("search", "internal_error", "payload overflow");
        }
        if !card(b, &mut n, item) {
            return write_error("search", "parse_error", "invalid result or overflow");
        }
        count += 1;
    }
    let next = field(d, b"since")
        .and_then(number)
        .filter(|v| *v > since && count > 0);
    page_end("search", &mut n, next)
}
fn browse(req: &[u8], op: &str) -> u32 {
    let page = match cursor(req) {
        Ok(c) => c.unwrap_or(1),
        Err(e) => return write_error(op, "invalid_request", e),
    };
    if page == 0 {
        return write_error(op, "invalid_request", "page must be positive");
    }
    let latest = extract_json_string(req, b"listingId") == Some(b"latest");
    let b = scratch_b();
    let mut n = 0;
    let ok = write_bytes(
        b,
        &mut n,
        if latest {
            b"/tag/0?region=1&pays=0&state=0&sort=3&page="
        } else {
            b"/tag/0?region=1&pays=0&state=0&sort=2&page="
        },
    ) && write_usize(b, &mut n, page);
    if !ok {
        return write_error(op, "invalid_request", "page too large");
    }
    let path = unsafe { core::slice::from_raw_parts(b.as_ptr(), n) };
    let j = match fetch_web(path) {
        Ok(j) => j,
        Err(e) => return fail_web(op, e),
    };
    let d = match first_data(j) {
        Some(d) => d,
        None => return write_error(op, "parse_error", "missing listing data"),
    };
    let mut it = match array(d, b"dataList") {
        Some(it) => it,
        None => return write_error(op, "parse_error", "missing listing items"),
    };
    let b = payload_buf();
    let mut n = 0;
    write_bytes(b, &mut n, br#"{"items":["#);
    let mut count = 0;
    while let Some(item) = it.next_object() {
        if count > 0 && !write_bytes(b, &mut n, b",") {
            return write_error(op, "internal_error", "payload overflow");
        }
        if !card(b, &mut n, item) {
            return write_error(op, "parse_error", "invalid listing or overflow");
        }
        count += 1;
    }
    let total = field(d, b"total").and_then(number);
    let per = field(d, b"count").and_then(number).filter(|n| *n > 0);
    let next = match (total, per) {
        (Some(total), Some(per)) if count > 0 && page.saturating_mul(per) < total => {
            Some(page.saturating_add(1))
        }
        (Some(_), Some(_)) => None,
        _ => return write_error(op, "parse_error", "invalid listing pagination"),
    };
    page_end(op, &mut n, next)
}
fn run_get_listings(_req: &[u8]) -> u32 {
    let data=r#"{"listings":[{"id":"popular","name":"热门","kind":"popular"},{"id":"latest","name":"最近更新","kind":"latest"}]}"#.as_bytes();
    payload_buf()[..data.len()].copy_from_slice(data);
    write_success_payload("get_listings", data.len())
}
fn run_get_manga_list(req: &[u8]) -> u32 {
    browse(req, "get_manga_list")
}
fn run_get_manga(req: &[u8]) -> u32 {
    let topic = match id(req, b"mangaId", b"kkt:") {
        Some(v) => v,
        None => return write_error("get_manga", "invalid_request", "bad mangaId"),
    };
    let d = match web_topic(topic) {
        Ok(v) => v,
        Err(e) => return fail_web("get_manga", e),
    };
    let info = match field(d, b"topicInfo") {
        Some(v) => v,
        None => return write_error("get_manga", "parse_error", "missing topic info"),
    };
    if field(info, b"id").and_then(number) != number(topic) {
        return write_error("get_manga", "parse_error", "topic mismatch");
    }
    let b = payload_buf();
    let mut n = 0;
    let status = match text(info, b"update_status") {
        Some(s) if s == "已完结".as_bytes() => b"completed".as_slice(),
        Some(s) if s == "连载中".as_bytes() => b"ongoing".as_slice(),
        _ => b"unknown".as_slice(),
    };
    let ok=write_bytes(b,&mut n,br#"{"manga":{"id":"kkt:"#)&&write_bytes(b,&mut n,topic)&&write_bytes(b,&mut n,br#"","title":"#)&&quoted(b,&mut n,text(info,b"title").unwrap_or(b""))&&write_bytes(b,&mut n,br#", "cover":{"kind":"url","url":"#)&&quoted(b,&mut n,text(info,b"vertical_image_url").unwrap_or(b""))&&write_bytes(b,&mut n,br#"},"authors":["#)&&quoted(b,&mut n,field(info,b"user").and_then(|v|text(v,b"nickname")).unwrap_or(b""))&&write_bytes(b,&mut n,br#"],"status":"#)&&quoted(b,&mut n,status)&&write_bytes(b,&mut n,br#", "description":"#)&&quoted(b,&mut n,text(info,b"description").unwrap_or(b""))&&write_bytes(b,&mut n,br#", "contentRating":"safe","links":[{"kind":"source","url":"https://www.kuaikanmanhua.com/web/topic/"#)&&write_bytes(b,&mut n,topic)&&write_bytes(b,&mut n,br#""}]}}"#);
    if !ok {
        return write_error("get_manga", "internal_error", "payload overflow");
    }
    write_success_payload("get_manga", n)
}
// A title supplies chapter semantics only when it starts with an explicit chapter label.
// Keep dates, announcements and other numbers unknown; never infer from sequenceIndex.
fn explicit_chapter(title: &[u8]) -> Option<usize> {
    let title = trim_ascii(title);
    let rest = title.strip_prefix("第".as_bytes())?;
    let digits = rest.iter().take_while(|b| b.is_ascii_digit()).count();
    if digits == 0 {
        return None;
    }
    let suffix = &rest[digits..];
    let tail = suffix.strip_prefix("话".as_bytes())
        .or_else(|| suffix.strip_prefix("話".as_bytes()))?;
    if !tail.is_empty() && !tail[0].is_ascii_whitespace()
        && ![":", "：", "、", ".", "．", "。", "!", "！", "?", "？"]
            .iter().any(|s| tail.starts_with(s.as_bytes())) {
        return None;
    }
    rest[..digits].iter().try_fold(0usize, |n, d| {
        n.checked_mul(10)?.checked_add((d - b'0') as usize)
    })
}
fn run_get_chapters(req: &[u8]) -> u32 {
    let topic = match id(req, b"mangaId", b"kkt:") {
        Some(v) => v,
        None => return write_error("get_chapters", "invalid_request", "bad mangaId"),
    };
    let d = match web_topic(topic) {
        Ok(v) => v,
        Err(e) => return fail_web("get_chapters", e),
    };
    if field(d, b"topicInfo")
        .and_then(|v| field(v, b"id"))
        .and_then(number)
        != number(topic)
    {
        return write_error("get_chapters", "parse_error", "topic mismatch");
    }
    let key = if field(d, b"comics").is_some() {
        b"comics".as_slice()
    } else {
        b"comicList".as_slice()
    };
    let mut it = match array(d, key) {
        Some(it) => it,
        None => return write_error("get_chapters", "parse_error", "missing public catalogue"),
    };
    let b = payload_buf();
    let mut n = 0;
    write_bytes(b, &mut n, br#"{"items":["#);
    let mut count = 0;
    while let Some(ch) = it.next_object() {
        if host::check_cancel() {
            return write_error("get_chapters", "cancelled", "cancelled");
        }
        let cid = match field(ch, b"id") {
            Some(id) if number(id).is_some() => id,
            _ => return write_error("get_chapters", "parse_error", "invalid chapter identity"),
        };
        if count > 0 && !write_bytes(b, &mut n, b",") {
            return write_error("get_chapters", "internal_error", "payload overflow");
        }
        let ok = write_bytes(b, &mut n, br#"{"id":"kkc:"#)
            && write_bytes(b, &mut n, topic)
            && write_bytes(b, &mut n, b":")
            && write_bytes(b, &mut n, cid)
            && write_bytes(b, &mut n, br#"","mangaId":"kkt:"#)
            && write_bytes(b, &mut n, topic)
            && write_bytes(b, &mut n, br#"","title":"#)
            && quoted(b, &mut n, text(ch, b"title").unwrap_or(b""))
            && write_bytes(b, &mut n, br#", "chapterNumber":"#)
            && if let Some(number) = explicit_chapter(text(ch, b"title").unwrap_or(b"")) {
                write_usize(b, &mut n, number)
                    && write_bytes(b, &mut n, br#", "readingGroup":"chapter","continuationEligible":true"#)
            } else {
                write_bytes(b, &mut n, br#"null,"readingGroup":"unknown","continuationEligible":false"#)
            }
            && write_bytes(b, &mut n, br#", "sequenceIndex":"#)
            && write_usize(b, &mut n, count)
            && write_bytes(b, &mut n, br#", "language":"zh","locked":"#)
            && write_bytes(
                b,
                &mut n,
                if field(ch, b"locked") == Some(b"true") {
                    b"true"
                } else {
                    b"false"
                },
            )
            && write_bytes(b, &mut n, b"}");
        if !ok {
            return write_error("get_chapters", "internal_error", "payload overflow");
        }
        count += 1;
    }
    let expected = field(d, b"topicInfo")
        .and_then(|v| field(v, b"comics_count"))
        .and_then(number);
    if expected.is_some() && expected != Some(count) {
        return write_error("get_chapters", "parse_error", "incomplete public catalogue");
    }
    page_end("get_chapters", &mut n, None)
}
fn split_id<'a>(v: &'a [u8], prefix: &[u8], page: bool) -> Option<(&'a [u8], &'a [u8], usize)> {
    let mut parts = v.strip_prefix(prefix)?.split(|b| *b == b':');
    let t = parts.next()?;
    let c = parts.next()?;
    if number(t).is_none() || number(c).is_none() {
        return None;
    }
    let i = if page { number(parts.next()?)? } else { 0 };
    if parts.next().is_some() {
        return None;
    }
    Some((t, c, i))
}
fn image_at(info: &[u8], index: usize) -> Option<&[u8]> {
    let mut it = array(info, b"comic_images")?;
    for i in 0..=index {
        let v = it.next_object()?;
        if i == index {
            return text(v, b"url1280")
                .filter(|v| !v.is_empty())
                .or_else(|| text(v, b"url"));
        }
    }
    None
}
fn run_get_pages(req: &[u8]) -> u32 {
    let (topic, comic, _) =
        match extract_json_string(req, b"chapterId").and_then(|v| split_id(v, b"kkc:", false)) {
            Some(v) => v,
            None => return write_error("get_pages", "invalid_request", "bad chapterId"),
        };
    let info = match web_comic(topic, comic) {
        Ok(v) => v,
        Err(e) => return fail_web("get_pages", e),
    };
    let mut it = match array(info, b"comic_images") {
        Some(it) => it,
        None => return write_error("get_pages", "parse_error", "missing free images"),
    };
    let b = payload_buf();
    let mut n = 0;
    write_bytes(b, &mut n, br#"{"pages":["#);
    let mut count = 0;
    while let Some(v) = it.next_object() {
        let url = match text(v, b"url1280")
            .filter(|v| !v.is_empty())
            .or_else(|| text(v, b"url"))
        {
            Some(v) => v,
            None => return write_error("get_pages", "parse_error", "missing free image url"),
        };
        if count > 0 && !write_bytes(b, &mut n, b",") {
            return write_error("get_pages", "internal_error", "payload overflow");
        }
        let ok = write_bytes(b, &mut n, br#"{"id":"kkp:"#)
            && write_bytes(b, &mut n, topic)
            && write_bytes(b, &mut n, b":")
            && write_bytes(b, &mut n, comic)
            && write_bytes(b, &mut n, b":")
            && write_usize(b, &mut n, count)
            && write_bytes(b, &mut n, br#"","index":"#)
            && write_usize(b, &mut n, count)
            && write_bytes(b, &mut n, br#", "image":{"kind":"url","url":"#)
            && quoted(b, &mut n, url)
            && write_bytes(b, &mut n, b"}}");
        if !ok {
            return write_error("get_pages", "internal_error", "payload overflow");
        }
        count += 1;
    }
    if count == 0 {
        return write_error("get_pages", "permission_denied", "no public free images");
    }
    if !write_bytes(b, &mut n, b"]}") {
        return write_error("get_pages", "internal_error", "payload overflow");
    }
    write_success_payload("get_pages", n)
}
fn run_get_image_request(req: &[u8]) -> u32 {
    let supplied = match extract_json_string(req, b"url") {
        Some(v) if !v.is_empty() => v,
        _ => return write_error("get_image_request", "invalid_request", "missing url"),
    };
    let page = extract_json_string(req, b"pageId").unwrap_or(b"");
    let url = if page.starts_with(b"kkp:") {
        let (topic, comic, index) = match split_id(page, b"kkp:", true) {
            Some(v) => v,
            None => return write_error("get_image_request", "invalid_request", "bad pageId"),
        };
        let info = match web_comic(topic, comic) {
            Ok(v) => v,
            Err(e) => return fail_web("get_image_request", e),
        };
        match image_at(info, index) {
            Some(v) => v,
            None => return write_error("get_image_request", "not_found", "page not found"),
        }
    } else {
        supplied
    };
    let b = payload_buf();
    let mut n = 0;
    let ok = write_bytes(b, &mut n, br#"{"url":"#)
        && quoted(b, &mut n, url)
        && write_bytes(
            b,
            &mut n,
            br#", "headers":{"Referer":"https://www.kuaikanmanhua.com/"}}"#,
        );
    if !ok {
        return write_error("get_image_request", "internal_error", "payload overflow");
    }
    write_success_payload("get_image_request", n)
}
fn run_get_home(_r: &[u8]) -> u32 {
    write_error("get_home", "not_implemented", "not supported")
}
fn run_get_filters(_r: &[u8]) -> u32 {
    write_error("get_filters", "not_implemented", "not supported")
}
fn run_get_settings(_r: &[u8]) -> u32 {
    write_error("get_settings", "not_implemented", "not supported")
}
koma_source_sdk::koma_source_exports!("kuaikan");
