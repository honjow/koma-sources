use super::{
    bytes, fetch, id_valid, input_ch, input_mid, member, q, string, text, uint, write_num,
};
use koma_source_sdk::{comments, json_utils::JsonArrayIter};

type Result<T> = core::result::Result<T, &'static str>;

pub(super) fn load(request: &[u8], out: &mut [u8]) -> Result<usize> {
    let query = comments::request(request)?;
    let mut work = [0; 64];
    let work = input_mid(request, &mut work)?;
    let mut chapter = [0; 128];
    let (kind, key): (&[u8], &[u8]) = if query.chapter_id.is_some() {
        let (owner, id) = input_ch(request, &mut chapter)?;
        if owner != work {
            return Err("chapter owner mismatch");
        }
        (b"chapter", id)
    } else {
        (b"title", work)
    };
    let page = comments::position(query.cursor)?.unwrap_or(1);
    if page == 0 {
        return Err("invalid page cursor");
    }
    let mut body = [0; 256];
    let mut offset = 0;
    bytes(&mut body, &mut offset, br#"{"object_id":"#)?;
    q(&mut body, &mut offset, key)?;
    bytes(&mut body, &mut offset, br#", "object_type":"#)?;
    q(&mut body, &mut offset, kind)?;
    bytes(&mut body, &mut offset, br#", "filter_type":"#)?;
    // The site also offers a combined series and chapter feed for titles.
    q(
        &mut body,
        &mut offset,
        if kind == b"title" {
            b"title_only"
        } else {
            b"all"
        },
    )?;
    bytes(&mut body, &mut offset, br#", "limit":10,"page":"#)?;
    write_num(&mut body, &mut offset, page)?;
    bytes(&mut body, &mut offset, b"}")?;
    // The provider uses POST for this read-only list operation.
    let data = fetch(b"/api/v1/comment/load-comments", Some(&body[..offset]))?;
    let mut rows =
        JsonArrayIter::from_raw_array(member(data, b"data")?).ok_or("invalid comments")?;
    let mut offset = 0;
    let mut count = 0;
    bytes(out, &mut offset, br#"{"items":["#)?;
    while let Some(row) = rows.next_object() {
        if string(row, b"object_id") != Some(key) || string(row, b"object_type") != Some(kind) {
            return Err("comment owner mismatch");
        }
        let id = string(row, b"id")
            .filter(|id| id_valid(id))
            .ok_or("invalid comment id")?;
        let mut author = [0; 1024];
        let mut content = [0; 32768];
        let mut date = [0; 128];
        let author = if string(row, b"user_name").is_some() {
            text(row, b"user_name", &mut author)?
        } else {
            b""
        };
        let content = text(row, b"content", &mut content)?;
        let date = text(row, b"created_at", &mut date)?;
        if count > 0 {
            bytes(out, &mut offset, b",")?;
        }
        bytes(out, &mut offset, br#"{"id":"#)?;
        q(out, &mut offset, id)?;
        bytes(out, &mut offset, br#", "author":"#)?;
        q(out, &mut offset, author)?;
        bytes(out, &mut offset, br#", "content":"#)?;
        q(out, &mut offset, content)?;
        bytes(out, &mut offset, br#", "createdAt":"#)?;
        q(out, &mut offset, date)?;
        bytes(out, &mut offset, b"}")?;
        count += 1;
    }
    let pagination = member(data, b"pagination")?;
    if uint(pagination, b"page")? != page {
        return Err("comment page mismatch");
    }
    bytes(out, &mut offset, b"]")?;
    if count > 0 && page < uint(pagination, b"total_pages")? {
        bytes(out, &mut offset, br#", "nextCursor":""#)?;
        write_num(out, &mut offset, page + 1)?;
        bytes(out, &mut offset, b"\"")?;
    }
    bytes(out, &mut offset, b"}")?;
    Ok(offset)
}
