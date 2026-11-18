use koma_source_sdk::comments::{self, OPERATION};
use serde_json::{json, Value};

use super::{fetch_error_code, payload_buf, post_graphql, write_error, write_success_payload};

const PAGE_SIZE: usize = 30;
const QUERY: &str = "query getMessagesByComicId($comicId: ID!, $pagination: Pagination!) { getMessagesByComicId(comicId: $comicId, pagination: $pagination) { id comicId account { nickname } message deleted dateCreated } }";

fn render(body: &[u8], comic_id: &str, offset: usize) -> Option<Value> {
    let response: Value = serde_json::from_slice(body).ok()?;
    if response["errors"].as_array().is_some_and(|errors| !errors.is_empty()) {
        return None;
    }
    let rows = response["data"]["getMessagesByComicId"].as_array()?;
    let mut items = alloc::vec::Vec::new();
    for row in rows.iter().take(PAGE_SIZE) {
        if row["comicId"].as_str()? != comic_id { return None; }
        let id = row["id"].as_str()?;
        let deleted = row["deleted"].as_bool()?;
        let mut item = if deleted {
            json!({"id": id, "author": "", "content": "", "deleted": true})
        } else {
            json!({"id": id, "author": row["account"]["nickname"].as_str().unwrap_or(""),
                "content": row["message"].as_str()?})
        };
        if let Some(date) = row["dateCreated"].as_str() { item["createdAt"] = json!(date); }
        items.push(item);
    }
    let mut page = json!({"items": items});
    if rows.len() > PAGE_SIZE {
        page["nextCursor"] = json!(alloc::format!("{}", offset.checked_add(PAGE_SIZE)?));
    }
    Some(page)
}

pub(super) fn run(input: &[u8]) -> u32 {
    let request = match comments::request(input) {
        Ok(value) if value.chapter_id.is_none() => value,
        Ok(_) => return write_error(OPERATION, "unsupported_operation", "Komiic comments belong to a work"),
        Err(code) => return write_error(OPERATION, code, "invalid comments request"),
    };
    let Some(comic_id) = request.manga_id.strip_prefix(b"kic:")
        .filter(|id| !id.is_empty() && id.iter().all(u8::is_ascii_digit))
        .and_then(|id| core::str::from_utf8(id).ok()) else {
        return write_error(OPERATION, "invalid_request", "invalid comic identity");
    };
    let offset = match comments::position(request.cursor) {
        Ok(position) => position.unwrap_or(0),
        Err(code) => return write_error(OPERATION, code, "invalid comments cursor"),
    };
    let query = json!({"query": QUERY, "variables": {"comicId": comic_id,
        "pagination": {"limit": PAGE_SIZE + 1, "offset": offset, "orderBy": "DATE_UPDATED", "asc": true}}});
    let body = match serde_json::to_vec(&query) {
        Ok(body) => body,
        Err(_) => return write_error(OPERATION, "internal_error", "cannot encode comments query"),
    };
    let result = match post_graphql(input, &body) {
        Ok(result) => result,
        Err(error) => {
            let (code, message) = fetch_error_code(error);
            return write_error(OPERATION, code, message);
        }
    };
    let Some(page) = render(result, comic_id, offset) else {
        return write_error(OPERATION, "parse_error", "invalid Komiic comment data");
    };
    let output = payload_buf();
    match serde_json::to_vec(&page) {
        Ok(bytes) if bytes.len() <= output.len() => {
            output[..bytes.len()].copy_from_slice(&bytes);
            write_success_payload(OPERATION, bytes.len())
        }
        _ => write_error(OPERATION, "result_too_large", "comments response too large"),
    }
}
