use super::{body_buf, bytes, fetch_body, number, q, valid_path, BASE};
use koma_source_sdk::{
    comments,
    json_utils::{append_json_unescaped_then_escaped, write_usize, JsonArrayIter},
    source_protocol::{direct, raw_string},
};

type Result<T> = core::result::Result<T, &'static str>;
const PAGE_SIZE: usize = 15;

fn param<'a>(path: &'a [u8], key: &[u8]) -> Result<&'a [u8]> {
    let query = path
        .splitn(2, |c| *c == b'?')
        .nth(1)
        .ok_or("invalid request")?;
    let value = query
        .split(|c| *c == b'&')
        .find_map(|field| field.strip_prefix(key));
    value
        .filter(|value| number(value).is_some_and(|n| n > 0))
        .ok_or("invalid request")
}

fn value<'a>(row: &'a [u8], name: &[u8]) -> Result<&'a [u8]> {
    raw_string(row, name)?.ok_or("invalid comment response")
}

fn quote(out: &mut [u8], offset: &mut usize, val: &[u8]) -> Result<()> {
    bytes(out, offset, b"\"")?;
    if !append_json_unescaped_then_escaped(out, offset, val) {
        return Err("payload overflow");
    }
    bytes(out, offset, b"\"")
}

pub(super) fn load(request: &[u8], out: &mut [u8]) -> Result<usize> {
    let query = comments::request(request)?;
    let manga = query
        .manga_id
        .strip_prefix(b"manga:")
        .ok_or("invalid request")?;
    let chapter = query
        .chapter_id
        .and_then(|id| id.strip_prefix(b"chapter:"))
        .and_then(|id| id.strip_prefix(query.manga_id))
        .and_then(|id| id.strip_prefix(b":"))
        .ok_or("invalid request")?;
    // Search yields /episodeList?titleNo=; the canonical list URL uses title_no=.
    let title = param(manga, b"title_no=").or_else(|_| param(manga, b"titleNo="))?;
    let episode = param(chapter, b"episode_no=")?;
    if !valid_path(manga) || title != param(chapter, b"title_no=")? {
        return Err("invalid request");
    }
    let page = comments::position(query.cursor)?.unwrap_or(1);
    if page == 0 {
        return Err("invalid request");
    }
    let mut route = [0; 256];
    let mut offset = 0;
    bytes(&mut route, &mut offset, b"/v2/comment?titleNo=")?;
    bytes(&mut route, &mut offset, title)?;
    bytes(&mut route, &mut offset, b"&episodeNo=")?;
    bytes(&mut route, &mut offset, episode)?;
    bytes(&mut route, &mut offset, b"&sortBy=new&limit=15&pageNo=")?;
    if !write_usize(&mut route, &mut offset, page) {
        return Err("invalid request");
    }
    let len = fetch_body(BASE, &route[..offset])?;
    let data = &body_buf()[..len];
    if direct(data, b"code")? != Some(b"200") {
        return Err("invalid comment response");
    }
    let data = direct(data, b"data")?.ok_or("invalid comment response")?;
    let mut offset = 0;
    bytes(out, &mut offset, br#"{"items":["#)?;
    if direct(data, b"hide")? == Some(b"true") {
        bytes(out, &mut offset, b"]}")?;
        return Ok(offset);
    }
    let mut rows =
        JsonArrayIter::from_raw_array(direct(data, b"commentList")?.ok_or("invalid comment list")?)
            .ok_or("invalid comment list")?;
    let mut count = 0;
    for row in core::iter::from_fn(|| rows.next_object()) {
        if value(row, b"titleNo")? != title
            || value(row, b"episodeNo")?
                .strip_prefix(title)
                .and_then(|rest| rest.strip_prefix(b"-"))
                != Some(episode)
        {
            return Err("comment owner mismatch");
        }
        if direct(row, b"visible")? != Some(b"true")
            || direct(row, b"secret")? == Some(b"true")
            || direct(row, b"forbid")? == Some(b"true")
            || direct(row, b"deleted")? == Some(b"true")
        {
            continue;
        }
        if count > 0 {
            bytes(out, &mut offset, b",")?;
        }
        bytes(out, &mut offset, br#"{"id":"#)?;
        quote(out, &mut offset, value(row, b"_id")?)?;
        bytes(out, &mut offset, br#", "author":"#)?;
        quote(out, &mut offset, value(row, b"userName")?)?;
        bytes(out, &mut offset, br#", "content":"#)?;
        quote(out, &mut offset, value(row, b"contents")?)?;
        let mut iso = [0; 20];
        if let Some(len) =
            direct(row, b"createTime")?.and_then(|raw| comments::utc_iso_millis(raw, &mut iso))
        {
            bytes(out, &mut offset, br#", "createdAt":"#)?;
            q(out, &mut offset, &iso[..len])?;
        }
        bytes(out, &mut offset, b"}")?;
        count += 1;
    }
    bytes(out, &mut offset, b"]")?;
    let total = direct(data, b"count")?
        .and_then(number)
        .ok_or("invalid comment count")?;
    if count > 0 && page.saturating_mul(PAGE_SIZE) < total {
        bytes(out, &mut offset, br#", "nextCursor":""#)?;
        if !write_usize(out, &mut offset, page + 1) {
            return Err("payload overflow");
        }
        bytes(out, &mut offset, b"\"")?;
    }
    bytes(out, &mut offset, b"}")?;
    Ok(offset)
}
