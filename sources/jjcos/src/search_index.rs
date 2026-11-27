//! The Gridea site publishes one JSON index. Search one record at a time so
//! the guest only retains the bounded HTTP envelope and a page of matches.
use alloc::{collections::BTreeSet, string::String, vec::Vec};
use koma_source_sdk::{
    host,
    http::{CallError, Client, Request, RequireSuccess},
    json_utils::JsonArrayIter,
    plain_text::decode_json_content,
    source_protocol,
};
use serde_json::Value;

use crate::{album, media, port, site_url, slug, Gallery, Result};

const INDEX_REQUEST: &[u8] = br#"{"version":1,"method":"GET","url":"https://jjcos.com/api/","headers":{"Accept":"application/json"},"timeoutMs":15000,"responseKind":"bodyText"}"#;
const MAX_ENVELOPE: usize = 16 * 1024 * 1024;
const PAGE_SIZE: usize = 20;

pub(crate) fn search(query: &str, page: usize) -> Result<Value> {
    if host::check_cancel() {
        return Err("cancelled");
    }
    let request = Request::from_json(INDEX_REQUEST);

    // This is an inbound native HTTP buffer, allocated only for a keyword
    // search. Ordinary listings and the WASM result use their smaller budgets.
    let mut buffer = Vec::new();
    buffer
        .try_reserve_exact(MAX_ENVELOPE)
        .map_err(|_| "result_too_large")?;
    buffer.resize(MAX_ENVELOPE, 0);
    let client = Client::<CallError>::new(&[&RequireSuccess]);
    let response = client
        .send(request, &mut buffer)
        .map_err(|error| match error {
            CallError::Rejected(code) => code,
            CallError::Transport(-3) => "result_too_large",
            _ => "network_error",
        })?;
    let json = response.as_json();
    let body = source_protocol::raw_string(json, b"bodyText")
        .map_err(|_| "network_error")?
        .ok_or("network_error")?;
    let start = body.as_ptr() as usize - json.as_ptr() as usize;
    let end = start + body.len();
    let json = unescape_in_place(&mut buffer[start..end])?;
    let posts = source_protocol::direct(json, b"posts")
        .map_err(|_| "parse_error")?
        .ok_or("parse_error")?;
    let mut posts = JsonArrayIter::from_raw_array(posts).ok_or("parse_error")?;

    let mut found = 0;
    let skip = (page - 1) * PAGE_SIZE;
    let mut seen = BTreeSet::new();
    let mut items = Vec::new();
    while let Some(post) = posts.next_object() {
        if host::check_cancel() {
            return Err("cancelled");
        }
        let post: Value = serde_json::from_slice(post).map_err(|_| "parse_error")?;
        let title = post["title"].as_str().ok_or("parse_error")?.trim();
        let content = post["content"].as_str().unwrap_or_default();
        if !title.to_lowercase().contains(query) && !content.to_lowercase().contains(query) {
            continue;
        }
        let link = post["link"].as_str().ok_or("parse_error")?;
        let url = site_url(link)?;
        let key = slug(&url)?;
        if !seen.insert(String::from(key)) {
            continue;
        }
        if found >= skip {
            if items.len() == PAGE_SIZE {
                return Ok(port::items(items, Some(page + 1)));
            }
            items.push(
                Gallery {
                    key: key.into(),
                    url: album(key)?,
                    title: title.into(),
                    cover: post["feature"].as_str().and_then(media),
                    tags: Vec::new(),
                    published: None,
                }
                .manga(),
            );
        }
        found += 1;
    }
    Ok(port::items(items, None))
}

// Decode the HTTP envelope's string where it already resides. The strict HTTP
// reader validated escape syntax. Use the SDK for each escape, including a
// possible surrogate pair, instead of holding a second copy of the index.
fn unescape_in_place(bytes: &mut [u8]) -> Result<&[u8]> {
    let mut read = 0;
    let mut write = 0;
    let mut decoded = [0; 4];
    while read < bytes.len() {
        let next_escape = bytes[read..]
            .iter()
            .position(|b| *b == b'\\')
            .map_or(bytes.len(), |offset| read + offset);
        bytes.copy_within(read..next_escape, write);
        write += next_escape - read;
        if next_escape == bytes.len() {
            break;
        }
        read = next_escape;
        let is_unicode = bytes.get(read + 1) == Some(&b'u');
        let mut end = read + if is_unicode { 6 } else { 2 };
        if is_unicode
            && matches!(bytes.get(read + 2), Some(b'd' | b'D'))
            && matches!(
                bytes.get(read + 3),
                Some(b'8' | b'9' | b'a' | b'b' | b'A' | b'B')
            )
        {
            end += 6;
        }
        let escape = bytes.get(read..end).ok_or("network_error")?;
        let decoded = decode_json_content(escape, &mut decoded).map_err(|_| "network_error")?;
        bytes[write..write + decoded.len()].copy_from_slice(decoded);
        write += decoded.len();
        read = end;
    }
    Ok(&bytes[..write])
}
