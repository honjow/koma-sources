use super::{
    bytes, comic_path, fetch, number, path, profile_slug, q, selected, unescape, write_usize,
    Nodes, BASE,
};
use koma_source_sdk::comments;

type Result<T> = core::result::Result<T, &'static str>;

pub(super) fn load(request: &[u8], out: &mut [u8]) -> Result<usize> {
    let query = comments::request(request)?;
    if query.cursor.is_some() {
        return Err("invalid request");
    }
    let mut manga = [0; 512];
    let slug = profile_slug(unescape(query.manga_id, &mut manga)?)?;
    let mut chapter = [0; 512];
    let chapter = path(unescape(
        query.chapter_id.ok_or("invalid request")?,
        &mut chapter,
    )?)?;
    let id = comic_path(chapter, slug)?;
    let mut route = [0; 512];
    let mut offset = 0;
    bytes(&mut route, &mut offset, b"/read/")?;
    bytes(&mut route, &mut offset, slug)?;
    bytes(&mut route, &mut offset, b"/comments/")?;
    if !write_usize(&mut route, &mut offset, id) {
        return Err("payload overflow");
    }
    let document = fetch(BASE, &route[..offset])?;
    let forms = Nodes::get(&document, b"form#reply-prototype")?;
    let mut owner = [0; 32];
    if forms.n != 1 || number(selected(&forms, 0).attr(b"data-comicid", &mut owner)?) != Some(id) {
        return Err("comic owner mismatch");
    }
    let rows = Nodes::get(&document, b".user-comments .form-box[id^=comment-]")?;
    let mut offset = 0;
    bytes(out, &mut offset, br#"{"items":["#)?;
    for index in 0..rows.n {
        let row = selected(&rows, index);
        let mut key = [0; 64];
        let id = row
            .attr(b"id", &mut key)?
            .strip_prefix(b"comment-")
            .ok_or("invalid comment")?;
        number(id).filter(|i| *i > 0).ok_or("invalid comment")?;
        let authors = Nodes::get(&row, b".form-box-author")?;
        let bodies = Nodes::get(&row, b".form-box-content")?;
        let dates = Nodes::get(&row, b".form-box-time")?;
        if authors.n != 1 || bodies.n != 1 {
            return Err("invalid comment");
        }
        let mut author = [0; 1024];
        let mut content = [0; 16384];
        let mut date = [0; 128];
        if index > 0 {
            bytes(out, &mut offset, b",")?;
        }
        bytes(out, &mut offset, br#"{"id":"#)?;
        q(out, &mut offset, id)?;
        bytes(out, &mut offset, br#", "author":"#)?;
        q(out, &mut offset, selected(&authors, 0).text(&mut author)?)?;
        bytes(out, &mut offset, br#", "content":"#)?;
        q(out, &mut offset, selected(&bodies, 0).text(&mut content)?)?;
        if dates.n == 1 {
            bytes(out, &mut offset, br#", "createdAt":"#)?;
            q(out, &mut offset, selected(&dates, 0).text(&mut date)?)?;
        }
        bytes(out, &mut offset, b"}")?;
    }
    bytes(out, &mut offset, b"]}")?;
    Ok(offset)
}
