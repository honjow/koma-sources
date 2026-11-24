// Read-only forum adapter. JM's page total counts roots, not their replies.
use alloc::{format, string::{String, ToString}, vec::Vec, collections::BTreeSet};
use koma_source_sdk::{comments, host};
use serde_json::{json, Value};
use super::{Context, E, R, number_id, req_id};

fn text(raw: &str) -> R<String> {
    let document = host::html_parse(raw.as_bytes()).map_err(|_| E::from("parse_error"))?;
    let mut buffer = alloc::vec![0; 64 * 1024];
    let result = host::html_text(document, &mut buffer);
    let _ = host::html_close(document);
    let length = result.map_err(|_| E::from("parse_error"))?;
    if length >= buffer.len() { return Err("capacity_exceeded".into()); }
    buffer.truncate(length);
    String::from_utf8(buffer).map(|s| s.trim().to_string()).map_err(|_| "parse_error".into())
}
fn unsigned(v: &Value) -> Option<u64> {
    v.as_u64().or_else(|| v.as_str()?.parse().ok())
}
fn append(row: &Value, parent: Option<&str>, depth: usize, seen: &mut BTreeSet<String>, out: &mut Vec<Value>) -> R<()> {
    if host::check_cancel() { return Err("cancelled".into()); }
    if depth > 8 || out.len() >= 500 { return Err("capacity_exceeded".into()); }
    let cid = number_id(&row["CID"])?;
    if !seen.insert(cid.clone()) { return Ok(()); }
    let author = row["nickname"].as_str().filter(|s| !s.is_empty())
        .or_else(|| row["username"].as_str()).unwrap_or("");
    let mut content = text(row["content"].as_str().ok_or_else(|| E::from("parse_error"))?)?;
    if let Some(name) = parent { content = format!("回复 {}：{}", name, content); }
    if row["is_spoiler"] == true || matches!(row["is_spoiler"].as_str(), Some("1" | "2" | "true"))
        || unsigned(&row["is_spoiler"]).is_some_and(|n| n > 0)
        || row["spoiler"].as_str() == Some("2") || row["spoiler"].as_u64() == Some(2) {
        content = format!("【剧透】{}", content);
    }
    let mut item = json!({"id":cid,"author":author,"content":content});
    if let Some(stamp) = unsigned(&row["addtime"]).filter(|v| *v > 0) {
        let millis = if stamp < 1_000_000_000_000 { stamp.checked_mul(1000) } else { Some(stamp) };
        let mut iso = [0;20];
        if let Some(length) = millis.and_then(|v| comments::utc_iso_millis(v.to_string().as_bytes(), &mut iso)) {
            item["createdAt"] = json!(core::str::from_utf8(&iso[..length]).map_err(|_| E::from("parse_error"))?);
        }
    } else if let Some(date) = row["addtime"].as_str().filter(|s| !s.is_empty()) {
        item["createdAt"] = json!(date);
    }
    out.push(item);
    match row.get("replys") {
        None | Some(Value::Null) => {},
        Some(Value::Array(replies)) => for reply in replies { append(reply, Some(author), depth+1, seen, out)?; },
        _ => return Err("parse_error".into()),
    }
    Ok(())
}
pub(super) fn get(r: &[u8], input: &Value) -> R<Value> {
    let request = comments::request(r).map_err(E::from)?;
    if request.chapter_id.is_some() { return Err("unsupported_operation".into()); }
    let aid = req_id(input, "mangaId", "jmcomic:")?;
    let page = comments::position(request.cursor).map_err(E::from)?.unwrap_or(1);
    if page == 0 || page > i32::MAX as usize { return Err("invalid_request".into()); }
    if host::check_cancel() { return Err("cancelled".into()); }
    let context = Context::new(r, input)?;
    let response = context.api("forum", &[("mode", "all".to_string()), ("aid", aid), ("page", page.to_string())])?;
    if host::check_cancel() { return Err("cancelled".into()); }
    let rows = response["list"].as_array().ok_or_else(|| E::from("parse_error"))?;
    let mut items = Vec::new();
    let mut seen = BTreeSet::new();
    for row in rows { append(row, None, 0, &mut seen, &mut items)?; }
    let has_more = !rows.is_empty() && match response.get("total") {
        Some(Value::Null) | None => rows.len() == 10,
        Some(total) => (page as u64).checked_mul(10).ok_or_else(|| E::from("parse_error"))?
            < unsigned(total).ok_or_else(|| E::from("parse_error"))?,
    };
    let mut result = json!({"items":items});
    if has_more { result["nextCursor"] = json!(page.checked_add(1).ok_or_else(|| E::from("capacity_exceeded"))?.to_string()); }
    Ok(result)
}
pub(super) fn run(r: &[u8]) -> u32 { super::run("get_comments", r) }
