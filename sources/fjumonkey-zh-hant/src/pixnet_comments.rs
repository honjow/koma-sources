use super::{body_buf, checked_request, fetch, id, payload_buf, q, raw_string, w, BLOG, MID, R};
use koma_source_sdk::{
    comments, host,
    json_utils::{extract_json_string, json_object_member, write_usize, JsonArrayIter},
};

fn field<'a>(value: &'a [u8], name: &[u8]) -> R<&'a [u8]> {
    json_object_member(value, name).ok_or("parse_error")
}

fn text<'a>(value: &[u8], name: &[u8], out: &'a mut [u8]) -> R<&'a [u8]> {
    let raw = extract_json_string(value, name).ok_or("parse_error")?;
    let len = raw_string(raw, out)?;
    Ok(&out[..len])
}

fn decimal(value: usize, out: &mut [u8], offset: &mut usize) -> R<()> {
    write_usize(out, offset, value)
        .then_some(())
        .ok_or("capacity_exceeded")
}

fn load(request: &[u8]) -> R<usize> {
    let query = comments::request(request)?;
    if query.chapter_id.is_some() {
        return Err("invalid_request");
    }
    let post = id(query.manga_id, MID)?;
    let page = comments::position(query.cursor)?.unwrap_or(1);
    if page == 0 {
        return Err("invalid_request");
    }
    let mut url = [0; 256];
    let mut offset = 0;
    w(&mut url, &mut offset, BLOG)?;
    w(&mut url, &mut offset, b"/get-comments?postId=")?;
    w(&mut url, &mut offset, post)?;
    w(&mut url, &mut offset, b"&page=")?;
    decimal(page, &mut url, &mut offset)?;
    let len = fetch(&url[..offset])?;
    let body = &body_buf()[..len];
    let meta = field(body, b"meta")?;
    if super::number(field(meta, b"page")?)? != page {
        return Err("parse_error");
    }
    let last_page = super::number(field(meta, b"pageCount")?)?;
    let mut rows = JsonArrayIter::from_raw_array(field(body, b"data")?).ok_or("parse_error")?;
    let out = payload_buf();
    let mut offset = 0;
    let mut written = false;
    w(out, &mut offset, br#"{"items":["#)?;
    while let Some(row) = rows.next_object() {
        // The web experience shows entries with other visibility levels only to their authors.
        if field(row, b"visibility")? != b"1" {
            continue;
        }
        let mut key = [0; 64];
        let mut author = [0; 1024];
        let mut date = [0; 64];
        let mut content = [0; 32768];
        let mut display = [0; 32768];
        let key = text(row, b"id", &mut key)?;
        let author = text(field(row, b"commenter")?, b"display_name", &mut author)?;
        let date = text(row, b"datetime", &mut date)?;
        let content = text(row, b"content", &mut content)?;
        let content = if content.contains(&b'<') || content.contains(&b'&') {
            let doc = host::html_parse(content).map_err(|_| "html_error")?;
            let result = host::html_text(doc, &mut display).map_err(|_| "html_error");
            host::html_close(doc).map_err(|_| "html_error")?;
            &display[..result?]
        } else {
            content
        };
        if written {
            w(out, &mut offset, b",")?;
        }
        w(out, &mut offset, br#"{"id":"#)?;
        q(out, &mut offset, key)?;
        w(out, &mut offset, br#", "author":"#)?;
        q(out, &mut offset, author)?;
        w(out, &mut offset, br#", "content":"#)?;
        q(out, &mut offset, content)?;
        w(out, &mut offset, br#", "createdAt":"#)?;
        q(out, &mut offset, date)?;
        w(out, &mut offset, b"}")?;
        written = true;
    }
    w(out, &mut offset, b"]")?;
    if page < last_page {
        w(out, &mut offset, br#", "nextCursor":""#)?;
        decimal(page + 1, out, &mut offset)?;
        w(out, &mut offset, b"\"")?;
    }
    w(out, &mut offset, b"}")?;
    Ok(offset)
}

#[no_mangle]
pub extern "C" fn koma_source_get_comments(ptr: u32, len: u32) -> u32 {
    let result = checked_request(ptr, len)
        .ok_or("invalid_request")
        .and_then(load);
    super::result(comments::OPERATION, result)
}
