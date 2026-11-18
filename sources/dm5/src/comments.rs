use koma_source_sdk::comments::{self, OPERATION};
use koma_source_sdk::json_utils::{extract_json_string, write_bytes, write_usize, JsonArrayIter};
use koma_source_sdk::source_protocol::{direct, raw_string};
use super::{extract_js_number, extract_js_single_quoted_string, extract_js_string, fetch_body, fetch_error_code,
    parse_usize, payload_buf, trim_ascii, write_comment_fields, write_error, write_success_payload, BASE_URL, BODY_BUF};

type Error = (&'static str, &'static str);
const OVERFLOW: Error = ("result_too_large", "comments response too large");
const INVALID: Error = ("parse_error", "invalid chapter comments");

pub(super) fn fetch_rows(cid: &[u8], topic: &[u8], page: usize, size: usize) -> Result<&'static [u8], Error> {
    let mut url = [0; 1024];
    let mut len = 0;
    if !(write_bytes(&mut url, &mut len, BASE_URL)
        && write_bytes(&mut url, &mut len, b"/m") && write_bytes(&mut url, &mut len, cid)
        && write_bytes(&mut url, &mut len, b"/pagerdata.ashx?pageindex=") && write_usize(&mut url, &mut len, page)
        && write_bytes(&mut url, &mut len, b"&pagesize=") && write_usize(&mut url, &mut len, size)
        && write_bytes(&mut url, &mut len, b"&tid=") && write_bytes(&mut url, &mut len, topic)
        && write_bytes(&mut url, &mut len, b"&cid=") && write_bytes(&mut url, &mut len, cid)
        && write_bytes(&mut url, &mut len, b"&t=9")) {
        return Err(("internal_error", "comment URL overflow"));
    }
    let len = fetch_body(&url[..len], Some(BASE_URL)).map_err(fetch_error_code)?;
    Ok(trim_ascii(unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(BODY_BUF).cast::<u8>(), len) }))
}

fn render(body: &[u8], page: usize, size: usize, pages: usize, out: &mut [u8]) -> Result<usize, Error> {
    let mut rows = JsonArrayIter::from_raw_array(body).ok_or(INVALID)?;
    let mut count = 0;
    let mut end = 1;
    let mut c = 0;
    if !write_bytes(out, &mut c, b"{\"items\":[") { return Err(OVERFLOW); }
    while let Some(row) = rows.next_object() {
        let start = row.as_ptr() as usize - body.as_ptr() as usize;
        let gap = trim_ascii(&body[end..start]);
        if (count == 0 && !gap.is_empty()) || (count > 0 && gap != b",") { return Err(INVALID); }
        end = start + row.len();
        let id = direct(row, b"Id").map_err(|_| INVALID)?
            .filter(|id| !id.is_empty() && id.iter().all(u8::is_ascii_digit)).ok_or(INVALID)?;
        let poster = raw_string(row, b"Poster").map_err(|_| INVALID)?.ok_or(INVALID)?;
        let content = raw_string(row, b"PostContent").map_err(|_| INVALID)?.ok_or(INVALID)?;
        if (count > 0 && !write_bytes(out, &mut c, b","))
            || !write_bytes(out, &mut c, br#"{"id":""#) || !write_bytes(out, &mut c, id)
            || !write_bytes(out, &mut c, br#"","author":""#) || !write_comment_fields(out, &mut c, &[poster])
            || !write_bytes(out, &mut c, br#"","content":""#) || !write_comment_fields(out, &mut c, &[content])
            || !write_bytes(out, &mut c, b"\"") {
            return Err(OVERFLOW);
        }
        if let Some(date) = extract_json_string(row, b"PostTime") {
            if !write_bytes(out, &mut c, br#","createdAt":""#) ||
                !write_comment_fields(out, &mut c, &[date]) || !write_bytes(out, &mut c, b"\"") {
                return Err(OVERFLOW);
            }
        }
        if !write_bytes(out, &mut c, b"}") { return Err(OVERFLOW); }
        count += 1;
    }
    if trim_ascii(&body[end..]) != b"]" { return Err(INVALID); }
    if !write_bytes(out, &mut c, b"]") { return Err(OVERFLOW); }
    if count >= size && page < pages &&
        !(write_bytes(out, &mut c, br#","nextCursor":""#) && write_usize(out, &mut c, page + 1)
          && write_bytes(out, &mut c, b"\"")) {
        return Err(OVERFLOW);
    }
    if !write_bytes(out, &mut c, b"}") { return Err(OVERFLOW); }
    Ok(c)
}

fn read(input: &[u8]) -> Result<usize, Error> {
    let args = comments::request(input).map_err(|code| (code, "invalid comments request"))?;
    let chapter = args.chapter_id.ok_or(("unsupported_operation", "chapter comments required"))?;
    let identity = chapter.strip_prefix(b"chapter:").ok_or(("invalid_request", "invalid chapter identity"))?;
    let mut parts = identity.split(|&b| b == b':');
    let work = parts.next().filter(|id| !id.is_empty());
    let cid = parts.next().filter(|id| !id.is_empty() && id.iter().all(u8::is_ascii_digit));
    let cid = cid.filter(|_| args.manga_id.strip_prefix(b"manga:") == work && work.is_some())
        .ok_or(("invalid_request", "chapter does not belong to work"))?;
    let page = comments::position(args.cursor).map_err(|code| (code, "invalid cursor"))?.unwrap_or(1);
    if page == 0 { return Err(("invalid_request", "invalid comment page")); }

    let mut url = [0; 256];
    let mut len = 0;
    if !(write_bytes(&mut url, &mut len, BASE_URL) && write_bytes(&mut url, &mut len, b"/m")
        && write_bytes(&mut url, &mut len, cid) && write_bytes(&mut url, &mut len, b"/")) {
        return Err(("invalid_request", "invalid chapter address"));
    }
    let size = fetch_body(&url[..len], Some(BASE_URL)).map_err(fetch_error_code)?;
    let html = unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(BODY_BUF).cast::<u8>(), size) };
    let topic = extract_js_single_quoted_string(html, b"DM5_TIEBATOPICID='")
        .or_else(|| extract_js_string(html, b"DM5_TIEBATOPICID=\""))
        .filter(|id| !id.is_empty() && id.len() <= 64 && id.iter().all(u8::is_ascii_digit))
        .ok_or(("parse_error", "missing chapter comment metadata"))?;
    let pages = extract_js_number(html, b"DM5_PAGEPCOUNT = ").or_else(|| extract_js_number(html, b"DM5_PAGEPCOUNT="))
        .and_then(parse_usize).ok_or(("parse_error", "missing chapter comment page count"))?;
    if topic == b"0" || page > pages {
        let payload = br#"{"items":[]}"#;
        payload_buf()[..payload.len()].copy_from_slice(payload);
        return Ok(payload.len());
    }
    let mut topic_copy = [0; 64];
    topic_copy[..topic.len()].copy_from_slice(topic);
    let size = 10; // The website's pager requests ten rows per page.
    let body = fetch_rows(cid, &topic_copy[..topic.len()], page, size)?;
    render(body, page, size, pages, payload_buf())
}

pub(super) fn run(input: &[u8]) -> u32 {
    match read(input) {
        Ok(len) => write_success_payload(OPERATION, len),
        Err((code, message)) => write_error(OPERATION, code, message),
    }
}
