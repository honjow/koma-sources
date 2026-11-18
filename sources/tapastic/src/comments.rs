use super::{api, chapter_owner, positive, Node, Result, Value};
use alloc::{format, vec::Vec};
use koma_source_sdk::comments;
use serde_json::json;

pub(super) fn load(request: &[u8], args: &Value) -> Result<Value> {
    let query = comments::request(request)?;
    let (_, episode) = chapter_owner(args)?;
    let (page, since) = match query.cursor {
        None => (1, 0),
        Some(raw) => {
            let cursor = core::str::from_utf8(raw).map_err(|_| "invalid_request")?;
            let (page, since) = cursor.split_once(':').ok_or("invalid_request")?;
            (positive(page)?, positive(since)?)
        }
    };
    // Tapas uses a site-provided since token for subsequent requests, so new comments
    // arriving between pages do not change the offsets of an open list.
    let api_url = format!(
        "https://tapas.io/comment/{episode}?page={page}&sort=NEWEST&since={since}&init_load=0"
    );
    let response = api(&api_url)?;
    match response["code"].as_u64() {
        Some(200) => {}
        Some(401) => return Err("auth_required"),
        Some(403) => return Err("forbidden"),
        Some(404) => return Err("not_found"),
        _ => return Err("parse_error"),
    }
    let data = &response["data"];
    let html = data["html"].as_str().ok_or("parse_error")?;
    let pagination = &data["pagination"];
    let mut items = Vec::new();
    if !html.trim().is_empty() {
        let document = Node::parse(html.as_bytes())?;
        for row in document.all(".js-comment-parent-row[id^='comment-row-']")? {
            let attr = row.attr("id")?.ok_or("parse_error")?;
            let id = attr.strip_prefix("comment-row-").ok_or("parse_error")?;
            positive(id).map_err(|_| "parse_error")?;
            let Some(body) = row.one(".body__row > .js-comment-body") else {
                continue;
            };
            let content = body.whole()?;
            let author = row.require(".body__writer .writer__name")?.text()?;
            let mut item = json!({"id":id,"author":author,"content":content.trim()});
            if let Some(date) = row.one(".body__writer .writer__date") {
                item["createdAt"] = json!(date.text()?);
            }
            items.push(item);
        }
    }
    let more = pagination["has_next"].as_bool().ok_or("parse_error")?;
    let mut result = json!({"items":items});
    if more && !items.is_empty() {
        let next = pagination["page"]
            .as_u64()
            .filter(|n| *n > page)
            .ok_or("parse_error")?;
        let since = pagination["since"]
            .as_u64()
            .filter(|n| *n > 0)
            .ok_or("parse_error")?;
        result["nextCursor"] = json!(format!("{next}:{since}"));
    }
    Ok(result)
}
