use super::{array, field, number, payload_buf, text, web_topic, WebError};
use koma_source_sdk::{
    comments,
    json_utils::{append_json_unescaped_then_escaped, write_bytes},
};

type Result<T> = core::result::Result<T, WebError>;

fn bytes(out: &mut [u8], offset: &mut usize, value: &[u8]) -> Result<()> {
    write_bytes(out, offset, value)
        .then_some(())
        .ok_or("payload overflow".into())
}

fn quote(out: &mut [u8], offset: &mut usize, value: &[u8]) -> Result<()> {
    bytes(out, offset, b"\"")?;
    if !append_json_unescaped_then_escaped(out, offset, value) {
        return Err("invalid comment text".into());
    }
    bytes(out, offset, b"\"")
}

pub(super) fn load(request: &[u8]) -> Result<usize> {
    let query = comments::request(request)?;
    let topic = query
        .manga_id
        .strip_prefix(b"kkt:")
        .ok_or("invalid topic")?;
    let topic_number = number(topic).filter(|&id| id > 0).ok_or("invalid topic")?;
    if query.chapter_id.is_some() || query.cursor.is_some() {
        return Err("invalid request".into());
    }
    let data = web_topic(topic)?;
    if field(data, b"topicInfo")
        .and_then(|row| field(row, b"id"))
        .and_then(number)
        != Some(topic_number)
    {
        return Err("topic mismatch".into());
    }
    // The website publishes its complete featured list in the topic response.
    let mut rows = array(data, b"commentsData").ok_or("missing topic comments")?;
    let out = payload_buf();
    let mut offset = 0;
    let mut written = false;
    bytes(out, &mut offset, br#"{"items":["#)?;
    while let Some(row) = rows.next_object() {
        let id = text(row, b"id").ok_or("missing comment id")?;
        let user = field(row, b"user").ok_or("missing comment author")?;
        let author = text(user, b"nickname").unwrap_or(b"");
        let content = text(row, b"content").ok_or("missing comment text")?;
        if content.is_empty() {
            continue;
        }
        if written {
            bytes(out, &mut offset, b",")?;
        }
        bytes(out, &mut offset, br#"{"id":"#)?;
        quote(out, &mut offset, id)?;
        bytes(out, &mut offset, br#", "author":"#)?;
        quote(out, &mut offset, author)?;
        bytes(out, &mut offset, br#", "content":"#)?;
        quote(out, &mut offset, content)?;
        if let Some(date) = text(row, b"created_at_info") {
            bytes(out, &mut offset, br#", "createdAt":"#)?;
            quote(out, &mut offset, date)?;
        }
        bytes(out, &mut offset, b"}")?;
        written = true;
    }
    bytes(out, &mut offset, b"]}")?;
    Ok(offset)
}
