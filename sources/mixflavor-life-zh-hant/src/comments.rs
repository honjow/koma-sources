use super::{
    body_buf, fetch, number, parse_identity, payload_buf, q, raw_string, w, MID, ORIGIN, PL, POST,
    R,
};
use koma_source_sdk::{
    comments, host,
    json_utils::{extract_json_string, json_object_member, write_usize, JsonArrayIter},
};

const PAGE_SIZE: usize = 10;

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

pub(super) fn load(request: &[u8]) -> R<usize> {
    let query = comments::request(request)?;
    if query.chapter_id.is_some() || !parse_identity(query.manga_id, MID)?.is_empty() {
        return Err("invalid_request");
    }
    let post = unsafe { &POST[..PL] };
    let position = comments::position(query.cursor)?.unwrap_or(0);
    let mut url = [0; 256];
    let mut offset = 0;
    w(&mut url, &mut offset, ORIGIN)?;
    w(
        &mut url,
        &mut offset,
        b"/wp-json/wp/v2/comments?parent=0&post=",
    )?;
    w(&mut url, &mut offset, post)?;
    // Request one additional record to read the next-page signal from the body.
    w(&mut url, &mut offset, b"&per_page=")?;
    decimal(PAGE_SIZE + 1, &mut url, &mut offset)?;
    w(&mut url, &mut offset, b"&offset=")?;
    decimal(position, &mut url, &mut offset)?;
    let len = fetch(&url[..offset])?;
    let mut rows = JsonArrayIter::from_raw_array(&body_buf()[..len]).ok_or("parse_error")?;
    let out = payload_buf();
    let mut offset = 0;
    let mut written = 0;
    w(out, &mut offset, br#"{"items":["#)?;
    while written < PAGE_SIZE {
        let Some(row) = rows.next_object() else {
            break;
        };
        if field(row, b"post")? != post || field(row, b"parent")? != b"0" {
            return Err("identity_mismatch");
        }
        let key = number(field(row, b"id")?)?;
        let mut author = [0; 1024];
        let mut date = [0; 64];
        let mut content = [0; 32768];
        let mut display = [0; 32768];
        let author = text(row, b"author_name", &mut author)?;
        let date_len = text(row, b"date_gmt", &mut date)?.len();
        if date_len == 19 {
            date[date_len] = b'Z';
        }
        let date = &date[..date_len + usize::from(date_len == 19)];
        let content = text(field(row, b"content")?, b"rendered", &mut content)?;
        let doc = host::html_parse(content).map_err(|_| "html_error")?;
        let text_result = host::html_text(doc, &mut display).map_err(|_| "html_error");
        host::html_close(doc).map_err(|_| "html_error")?;
        let content = &display[..text_result?];
        if written > 0 {
            w(out, &mut offset, b",")?;
        }
        w(out, &mut offset, br#"{"id":""#)?;
        decimal(key, out, &mut offset)?;
        w(out, &mut offset, br#"", "author":"#)?;
        q(out, &mut offset, author)?;
        w(out, &mut offset, br#", "content":"#)?;
        q(out, &mut offset, content)?;
        w(out, &mut offset, br#", "createdAt":"#)?;
        q(out, &mut offset, date)?;
        w(out, &mut offset, b"}")?;
        written += 1;
    }
    w(out, &mut offset, b"]")?;
    if rows.next_object().is_some() {
        w(out, &mut offset, br#", "nextCursor":""#)?;
        decimal(
            position.checked_add(written).ok_or("invalid_request")?,
            out,
            &mut offset,
        )?;
        w(out, &mut offset, b"\"")?;
    }
    w(out, &mut offset, b"}")?;
    Ok(offset)
}
