use super::{
    art_detail, bytes, fetch, field_id, namespace, payload_buf, q, raw_string, rawq, required,
    text, wi, wn, Array, E, WEB,
};
use koma_source_sdk::comments;

const PAGE_SIZE: usize = 10;

pub(super) fn load(request: &[u8]) -> Result<usize, E> {
    let query = comments::request(request)?;
    if query.chapter_id.is_none() {
        return Err("invalid identity");
    }
    let (series, manga, _) = namespace(request, b"mangaId", false)?;
    let (chapter_series, owner, item) = namespace(request, b"chapterId", true)?;
    if series != chapter_series || manga != owner {
        return Err("chapter ownership mismatch");
    }
    let art = item.unwrap_or(owner);
    let work = art_detail(art)?;
    if series && field_id(required(work, b"series")?, b"id")? != owner {
        return Err("chapter ownership mismatch");
    }
    let position = comments::position(query.cursor)?.unwrap_or(0);
    let mut url = [0; 256];
    let mut offset = 0;
    bytes(&mut url, &mut offset, WEB)?;
    bytes(
        &mut url,
        &mut offset,
        b"/ajax/illusts/comments/roots?illust_id=",
    )?;
    wi(&mut url, &mut offset, art)?;
    bytes(&mut url, &mut offset, b"&offset=")?;
    wn(&mut url, &mut offset, position)?;
    bytes(&mut url, &mut offset, b"&limit=")?;
    wn(&mut url, &mut offset, PAGE_SIZE)?;
    bytes(&mut url, &mut offset, b"&lang=zh")?;
    let body = fetch(&url[..offset])?;
    let mut entries = Array::new(required(body, b"comments")?)?;
    let output = payload_buf();
    let mut offset = 0;
    let mut count = 0;
    let mut written = false;
    bytes(output, &mut offset, br#"{"items":["#)?;
    while let Some(row) = entries.next()? {
        count += 1;
        let comment = text(row, b"comment")?;
        let stamp = required(row, b"stampId")? != b"null";
        if comment.is_empty() && !stamp {
            continue;
        }
        if written {
            bytes(output, &mut offset, b",")?;
        }
        bytes(output, &mut offset, br#"{"id":"#)?;
        rawq(output, &mut offset, raw_string(row, b"id")?)?;
        bytes(output, &mut offset, br#", "author":"#)?;
        rawq(output, &mut offset, raw_string(row, b"userName")?)?;
        bytes(output, &mut offset, br#", "content":"#)?;
        if comment.is_empty() {
            q(output, &mut offset, "（贴图）".as_bytes())?;
        } else {
            rawq(output, &mut offset, raw_string(row, b"comment")?)?;
        }
        bytes(output, &mut offset, br#", "createdAt":"#)?;
        rawq(output, &mut offset, raw_string(row, b"commentDate")?)?;
        bytes(output, &mut offset, b"}")?;
        written = true;
    }
    bytes(output, &mut offset, b"]")?;
    if count != 0 && required(body, b"hasNext")? == b"true" {
        bytes(output, &mut offset, br#", "nextCursor":""#)?;
        wn(
            output,
            &mut offset,
            position.checked_add(count).ok_or("invalid cursor")?,
        )?;
        bytes(output, &mut offset, b"\"")?;
    }
    bytes(output, &mut offset, b"}")?;
    Ok(offset)
}
