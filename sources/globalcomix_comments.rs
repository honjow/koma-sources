use super::{bytes, chapter_parts, fetch, is, manga_number, member, q, text, uint, write_num, API};
use koma_source_sdk::{comments, json_utils::JsonArrayIter};

type Result<T> = core::result::Result<T, &'static str>;

pub(super) fn load(request: &[u8], out: &mut [u8]) -> Result<usize> {
    let query = comments::request(request)?;
    let work = manga_number(query.manga_id)?;
    let (comic, release, _) = chapter_parts(query.chapter_id.ok_or("invalid request")?)?;
    if work != comic || query.cursor.is_some() {
        return Err("invalid request");
    }
    let mut route = [0; 128];
    let mut offset = 0;
    bytes(&mut route, &mut offset, b"/releases/")?;
    write_num(&mut route, &mut offset, release)?;
    bytes(&mut route, &mut offset, b"/comments")?;
    let data = fetch(API, &route[..offset])?;
    // The web client requests this list once. Replies are nested in each result.
    let mut rows = JsonArrayIter::from_raw_array(data).ok_or("invalid comment list")?;
    let mut offset = 0;
    let mut count = 0;
    bytes(out, &mut offset, br#"{"items":["#)?;
    while let Some(row) = rows.next_object() {
        if !is(row, b"comment_type", br#""comics-releases""#) || uint(row, b"object_id")? != release
        {
            return Err("release owner mismatch");
        }
        let id = uint(row, b"id")?;
        let deleted = uint(row, b"is_deleted")? != 0;
        if count > 0 {
            bytes(out, &mut offset, b",")?;
        }
        bytes(out, &mut offset, br#"{"id":""#)?;
        write_num(out, &mut offset, id)?;
        if deleted {
            bytes(
                out,
                &mut offset,
                br#"","author":"","content":"","deleted":true}"#,
            )?;
        } else {
            let mut author = [0; 1024];
            let mut body = [0; 32768];
            let mut date = [0; 128];
            let author = text(member(row, b"author")?, b"name", &mut author)?;
            let body = text(row, b"body", &mut body)?;
            let date = text(row, b"post_date", &mut date)?;
            bytes(out, &mut offset, br#"", "author":"#)?;
            q(out, &mut offset, author)?;
            bytes(out, &mut offset, br#", "content":"#)?;
            q(out, &mut offset, body)?;
            bytes(out, &mut offset, br#", "createdAt":"#)?;
            q(out, &mut offset, date)?;
            bytes(out, &mut offset, b"}")?;
        }
        count += 1;
    }
    bytes(out, &mut offset, b"]}")?;
    Ok(offset)
}
