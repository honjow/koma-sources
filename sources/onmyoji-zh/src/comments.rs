use super::{body_buf, fetch, id, member, n, number, parse_ch, q, string, unprefix, w, CP, MP, R};
use koma_source_sdk::{comments, json_utils::JsonArrayIter};

const PAGE_SIZE: usize = 10;

fn append_rows(
    out: &mut [u8],
    offset: &mut usize,
    array: &[u8],
    owner: &[u8],
    count: &mut usize,
) -> R<usize> {
    let mut rows = JsonArrayIter::from_raw_array(array).ok_or("parse_error")?;
    let mut read = 0;
    while let Some(row) = rows.next_object() {
        if string(row, b"article_id")? != owner {
            return Err("owner_mismatch");
        }
        let key = id(string(row, b"id")?).map_err(|_| "parse_error")?;
        if *count > 0 {
            w(out, offset, b",")?;
        }
        w(out, offset, br#"{"id":"#)?;
        q(out, offset, key)?;
        w(out, offset, br#", "author":"#)?;
        q(out, offset, string(member(row, b"author")?, b"nickname")?)?;
        w(out, offset, br#", "content":"#)?;
        q(out, offset, string(row, b"content")?)?;
        w(out, offset, br#", "createdAt":"#)?;
        q(out, offset, string(row, b"time")?)?;
        w(out, offset, b"}")?;
        *count += 1;
        read += 1;
    }
    Ok(read)
}

pub(super) fn load(request: &[u8], out: &mut [u8]) -> R<usize> {
    let query = comments::request(request)?;
    let manga = unprefix(query.manga_id, MP)?;
    let mut url = [0; 512];
    let mut offset = 0;
    w(
        &mut url,
        &mut offset,
        b"https://huajia.tongren.163.com/comment/apps/web/?article_id=",
    )?;
    w(&mut url, &mut offset, manga)?;
    let owner = if let Some(raw) = query.chapter_id {
        let (work, chapter, _, page) = parse_ch(raw, CP)?;
        if work != manga || page.is_some() {
            return Err("invalid_request");
        }
        w(&mut url, &mut offset, b"&chapter_id=")?;
        w(&mut url, &mut offset, chapter)?;
        chapter
    } else {
        w(&mut url, &mut offset, b"&game=web")?;
        manga
    };
    let start = comments::position(query.cursor)?.unwrap_or(0);
    w(&mut url, &mut offset, b"&reply_count=0&span=")?;
    n(&mut url, &mut offset, PAGE_SIZE)?;
    w(&mut url, &mut offset, b"&start=")?;
    n(&mut url, &mut offset, start)?;
    let len = fetch(&url[..offset])?;
    let data = member(&body_buf()[..len], b"data")?;
    let total = number(member(data, b"count")?)?;
    let mut offset = 0;
    let mut count = 0;
    w(out, &mut offset, br#"{"items":["#)?;
    if start == 0 {
        append_rows(
            out,
            &mut offset,
            member(data, b"top_comments")?,
            owner,
            &mut count,
        )?;
    }
    let size = append_rows(
        out,
        &mut offset,
        member(data, b"comments")?,
        owner,
        &mut count,
    )?;
    w(out, &mut offset, b"]")?;
    let next = start.checked_add(size).ok_or("parse_error")?;
    if size > 0 && next < total {
        w(out, &mut offset, br#", "nextCursor":""#)?;
        n(out, &mut offset, next)?;
        w(out, &mut offset, b"\"")?;
    }
    w(out, &mut offset, b"}")?;
    Ok(offset)
}
