use super::{bytes, chapter_id, fetch, integer, manga_id, rawq, required, text, wn, API, E};
use koma_source_sdk::{comments, json_utils::JsonArrayIter};

type Result<T> = core::result::Result<T, E>;

fn author<'a>(comment: &'a [u8], chapter: &[u8]) -> Result<&'a [u8]> {
    let relationships = required(comment, b"relationships")?;
    let mut relationships =
        JsonArrayIter::from_raw_array(relationships).ok_or("invalid comment relationships")?;
    let mut match_chapter = false;
    let mut user = None;
    while let Some(relation) = relationships.next_object() {
        match text(relation, b"type")? {
            b"chapter" => match_chapter = text(relation, b"id")? == chapter,
            b"user" => user = Some(required(relation, b"attributes")?),
            _ => {}
        }
    }
    if !match_chapter {
        return Err("invalid chapter comment owner");
    }
    let Some(user) = user else {
        return Ok(br#""""#);
    };
    let name = required(user, b"displayName")?;
    if name == br#""""# || name == b"null" {
        required(user, b"username")
    } else {
        Ok(name)
    }
}

pub(super) fn load(request: &[u8], out: &mut [u8]) -> Result<usize> {
    let query = comments::request(request)?;
    let (work, chapter) = chapter_id(request)?;
    if work != manga_id(request)? {
        return Err("invalid request");
    }
    let start = comments::position(query.cursor)?.unwrap_or(0);
    let mut url = [0; 512];
    let mut offset = 0;
    bytes(&mut url, &mut offset, API)?;
    bytes(&mut url, &mut offset, b"/comment/chapter/")?;
    bytes(&mut url, &mut offset, chapter)?;
    bytes(&mut url, &mut offset, b"?parentId=NULL&limit=10&order%5BcreatedAt%5D=desc&includes%5B%5D=user&showDeleted=false&offset=")?;
    wn(&mut url, &mut offset, start)?;
    let data = fetch(&url[..offset], None)?;
    let meta = required(data, b"meta")?;
    if integer(required(meta, b"offset")?)? != start {
        return Err("incorrect comment offset");
    }
    let mut rows =
        JsonArrayIter::from_raw_array(required(data, b"data")?).ok_or("invalid comments")?;
    let mut count = 0;
    let mut offset = 0;
    bytes(out, &mut offset, br#"{"items":["#)?;
    while let Some(row) = rows.next_object() {
        let fields = required(row, b"attributes")?;
        if required(fields, b"spoiler")? == b"true" || required(fields, b"deletedAt")? != b"null" {
            continue;
        }
        let message = required(fields, b"message")?;
        if message == b"null" || message == br#""""# {
            continue;
        }
        if count != 0 {
            bytes(out, &mut offset, b",")?;
        }
        bytes(out, &mut offset, br#"{"id":"#)?;
        rawq(out, &mut offset, required(row, b"id")?)?;
        bytes(out, &mut offset, br#", "author":"#)?;
        rawq(out, &mut offset, author(row, chapter)?)?;
        bytes(out, &mut offset, br#", "content":"#)?;
        rawq(out, &mut offset, message)?;
        bytes(out, &mut offset, br#", "createdAt":"#)?;
        rawq(out, &mut offset, required(fields, b"createdAt")?)?;
        bytes(out, &mut offset, b"}")?;
        count += 1;
    }
    bytes(out, &mut offset, b"]")?;
    let next = start
        .checked_add(integer(required(meta, b"limit")?)?)
        .ok_or("invalid comment offset")?;
    if count > 0 && next > start && next < integer(required(meta, b"total")?)? {
        bytes(out, &mut offset, br#", "nextCursor":""#)?;
        wn(out, &mut offset, next)?;
        bytes(out, &mut offset, b"\"")?;
    }
    bytes(out, &mut offset, b"}")?;
    Ok(offset)
}
