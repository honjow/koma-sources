// The Host v1 list is flat. Replies use the same list and carry their addressee;
// cursor continuations fetch one provider page, never all threads in a burst.
use alloc::{format, string::{String, ToString}, vec::Vec, collections::BTreeSet};
use koma_source_sdk::{comments, host};
use serde_json::{json, Value};
use super::{api, id, manga_id, E, R};

fn page(value: &Value) -> R<u32> {
    value.as_u64().filter(|v| *v > 0 && *v <= i32::MAX as u64)
        .map(|v| v as u32).ok_or_else(|| "invalid_request".into())
}
fn cursor(input: &Value, mid: &str) -> R<Value> {
    match &input["args"]["cursor"] {
        Value::Null => Ok(json!({"v":1,"manga":mid,"rootPage":1,"replies":[]})),
        Value::String(raw) if raw.len() <= 32768 => {
            let state: Value = serde_json::from_str(raw).map_err(|_| E::from("invalid_request"))?;
            if state["v"] != 1 || state["manga"] != mid { return Err("invalid_request".into()); }
            let replies = state["replies"].as_array().filter(|v| v.len() <= 100).ok_or_else(|| E::from("invalid_request"))?;
            if !state["rootPage"].is_null() { page(&state["rootPage"])?; }
            if state["rootPage"].is_null() && replies.is_empty() { return Err("invalid_request".into()); }
            for reply in replies {
                id(reply["id"].as_str().ok_or_else(|| E::from("invalid_request"))?)?;
                if reply["author"].as_str().is_none() { return Err("invalid_request".into()); }
                page(&reply["page"])?;
            }
            Ok(state)
        },
        _ => Err("invalid_request".into()),
    }
}
fn invalid(field: &str, value: &Value) -> E {
    let kind = match value { Value::Null => "null", Value::Bool(_) => "boolean", Value::Number(_) => "number", Value::String(_) => "string", Value::Array(_) => "array", Value::Object(_) => "object" };
    host::log_info(format!("Picacomic comments invalid field={} type={}", field, kind).as_bytes());
    "parse_error".into()
}
// The live API sends comments.page as a decimal string, while pages is
// numeric. Accept either representation without accepting partial/negative IDs.
fn provider_number(value: &Value, field: &str) -> R<u32> {
    let n = value.as_u64().or_else(|| value.as_str().and_then(|raw| {
        raw.parse::<u64>().ok().filter(|n| n.to_string() == raw)
    })).filter(|n| *n <= i32::MAX as u64).ok_or_else(|| invalid(field, value))?;
    Ok(n as u32)
}
fn parse(row: &Value, parent: Option<&str>, seen: &mut BTreeSet<String>) -> R<Option<Value>> {
    let raw_id = row["_id"].as_str().ok_or_else(|| invalid("_id", &row["_id"]))?;
    let cid = id(raw_id).map_err(|_| E::from("parse_error"))?;
    if !seen.insert(cid.clone()) { return Ok(None); }
    let author = row["_user"]["name"].as_str().unwrap_or("");
    let content = row["content"].as_str().ok_or_else(|| invalid("content", &row["content"]))?;
    let content = parent.map(|name| format!("回复 {}：{}", name, content)).unwrap_or_else(|| content.to_string());
    let mut item = json!({"id":cid,"author":author,"content":content});
    if let Some(date) = row["created_at"].as_str().filter(|s| !s.is_empty()) { item["createdAt"] = json!(date); }
    Ok(Some(item))
}
fn get_page(r: &[u8], input: &Value) -> R<Value> {
    let request = comments::request(r).map_err(E::from)?;
    if request.chapter_id.is_some() { return Err("unsupported_operation".into()); }
    let mid = manga_id(input)?;
    let mut state = cursor(input, &mid)?;
    let pending = state["replies"].as_array().ok_or_else(|| E::from("invalid_request"))?;
    let reply = pending.first().cloned();
    let current_page = page(reply.as_ref().map(|v| &v["page"]).unwrap_or(&state["rootPage"]))?;
    let path = if let Some(parent) = &reply {
        format!("/comments/{}/childrens?page={}", parent["id"].as_str().ok_or_else(|| E::from("invalid_request"))?, current_page)
    } else { format!("/comics/{}/comments?page={}", mid, current_page) };
    if host::check_cancel() { return Err("cancelled".into()); }
    let response = api(r, input, &path, "GET", None)?;
    if host::check_cancel() { return Err("cancelled".into()); }
    let data = &response["data"]["comments"];
    let docs = data["docs"].as_array().ok_or_else(|| invalid("comments.docs", &data["docs"]))?;
    let pages = provider_number(&data["pages"], "comments.pages")?;
    if let Some(returned) = data.get("page") {
        if provider_number(returned, "comments.page")? != current_page { return Err(invalid("comments.page", returned)); }
    }
    let has_more = !docs.is_empty() && current_page < pages;
    let mut items = Vec::new();
    let mut seen = BTreeSet::new();
    for row in docs {
        if let Some(item) = parse(row, reply.as_ref().and_then(|v| v["author"].as_str()), &mut seen)? {
            if reply.is_none() && row["commentsCount"].as_u64().unwrap_or(0) > 0 {
                state["replies"].as_array_mut().ok_or_else(|| E::from("internal_error"))?
                    .push(json!({"id":item["id"],"author":item["author"],"page":1}));
            }
            items.push(item);
        }
    }
    if reply.is_some() {
        if has_more { state["replies"][0]["page"] = json!(current_page+1); }
        else { state["replies"].as_array_mut().ok_or_else(|| E::from("internal_error"))?.remove(0); }
    } else { state["rootPage"] = if has_more { json!(current_page+1) } else { Value::Null }; }
    let mut result = json!({"items":items});
    if !state["rootPage"].is_null() || !state["replies"].as_array().ok_or_else(|| E::from("internal_error"))?.is_empty() {
        let encoded = serde_json::to_string(&state).map_err(|_| E::from("capacity_exceeded"))?;
        if encoded.len() > 32768 { return Err("capacity_exceeded".into()); }
        result["nextCursor"] = json!(encoded);
    }
    Ok(result)
}
pub(super) fn get(r: &[u8], input: &Value) -> R<Value> {
    let mut current = input.clone();
    // A thread can lose its last reply between the root page and continuation.
    // The Host correctly rejects an empty page that claims more rows. Advance
    // empty provider threads here without fabricating comments or ending roots.
    for _ in 0..=100 {
        let page = get_page(r, &current)?;
        if !page["items"].as_array().ok_or_else(|| E::from("parse_error"))?.is_empty()
            || page.get("nextCursor").is_none() { return Ok(page); }
        current["args"]["cursor"] = page["nextCursor"].clone();
    }
    Err("server_error".into())
}
pub(super) fn run(r: &[u8]) -> u32 { super::run("get_comments", r) }
