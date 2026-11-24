use alloc::{
    collections::BTreeSet,
    format,
    string::{String, ToString},
    vec::Vec,
};
use koma_source_sdk::{comments, host, source_protocol as protocol};
use serde_json::{json, Value};

use super::{array, chapter_id, encoded, field, manga_id, path_id, Context, E, R};

const PAGE_SIZE: usize = 20;

enum ScopeTarget {
    Manga(String),
    Chapter(String),
}

fn parse_offset(cursor: Option<&[u8]>) -> R<usize> {
    let Some(raw) = cursor else {
        return Ok(0);
    };
    let n = comments::position(Some(raw))
        .map_err(E::from)?
        .ok_or_else(|| E::from("invalid_request"))?;
    if n > i32::MAX as usize || n.to_string().as_bytes() != raw {
        return Err("invalid_request".into());
    }
    Ok(n)
}

fn comment_id(value: &Value) -> R<String> {
    if let Some(n) = value.as_u64() {
        return Ok(n.to_string());
    }
    if let Some(s) = value.as_str() {
        if !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) {
            let n = s.parse::<u64>().map_err(|_| E::from("parse_error"))?;
            if n.to_string() == s {
                return Ok(s.to_string());
            }
        }
    }
    Err("parse_error".into())
}

fn parse_author(cx: &Context, row: &Value) -> R<String> {
    match row.get("user_name") {
        None | Some(Value::Null) => Ok(String::new()),
        Some(Value::String(s)) => Ok(cx.display(s)),
        Some(_) => Err("parse_error".into()),
    }
}

fn parse_parent(cx: &Context, row: &Value) -> R<Option<String>> {
    match row.get("parent_user_name") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                Ok(None)
            } else {
                Ok(Some(cx.display(trimmed)))
            }
        }
        Some(_) => Err("parse_error".into()),
    }
}

fn parse_created_at(row: &Value) -> R<Option<&str>> {
    match row.get("create_at") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => {
            if s.trim().is_empty() {
                Ok(None)
            } else {
                Ok(Some(s.as_str()))
            }
        }
        Some(_) => Err("parse_error".into()),
    }
}

fn parse_item(cx: &Context, row: &Value, seen: &mut BTreeSet<String>) -> R<Option<Value>> {
    let cid = comment_id(&row["id"])?;
    if !seen.insert(cid.clone()) {
        return Ok(None);
    }
    let author = parse_author(cx, row)?;
    let raw_comment = field(row, "comment")?;
    let body = cx.display(raw_comment);
    let content = match parse_parent(cx, row)? {
        Some(parent) => format!("回复 {}：{}", parent, body),
        None => body,
    };
    let mut item = json!({
        "id": cid,
        "author": author,
        "content": content,
    });
    if let Some(date) = parse_created_at(row)? {
        item["createdAt"] = json!(date);
    }
    Ok(Some(item))
}

pub(super) fn get(r: &[u8], v: &Value) -> R<Value> {
    let request = comments::request(r).map_err(E::from)?;
    let offset = parse_offset(request.cursor)?;
    let target = if request.chapter_id.is_some() {
        let (_slug, chapter_uuid) = chapter_id(v)?;
        ScopeTarget::Chapter(chapter_uuid)
    } else {
        ScopeTarget::Manga(manga_id(v)?)
    };

    let api_transport = protocol::feature(r, b"source_http_copymanga_api_v1")
        || protocol::feature(r, b"source_http_profiles_v1");
    let cx = Context::new(v, false, api_transport, false)?;
    if host::check_cancel() {
        return Err("cancelled".into());
    }

    let data = match target {
        ScopeTarget::Chapter(chapter_uuid) => cx.api(
            "roasts",
            &[
                ("chapter_id", chapter_uuid),
                ("limit", PAGE_SIZE.to_string()),
                ("offset", offset.to_string()),
            ],
        )?,
        ScopeTarget::Manga(slug) => {
            let detail = cx.api(&format!("comic2/{}", slug), &[])?;
            if encoded(field(&detail["comic"], "path_word")?) != slug {
                return Err("parse_error".into());
            }
            let comic_uuid = encoded(field(&detail["comic"], "uuid")?);
            path_id(&comic_uuid).map_err(|_| E::from("parse_error"))?;
            if host::check_cancel() {
                return Err("cancelled".into());
            }
            cx.api(
                "comments",
                &[
                    ("comic_id", comic_uuid),
                    ("limit", PAGE_SIZE.to_string()),
                    ("offset", offset.to_string()),
                ],
            )?
        }
    };

    if host::check_cancel() {
        return Err("cancelled".into());
    }

    let list = array(&data, "list")?;
    let total = data["total"]
        .as_u64()
        .ok_or_else(|| E::from("parse_error"))?;
    let mut items = Vec::new();
    let mut seen = BTreeSet::new();
    for row in list {
        if let Some(item) = parse_item(&cx, row, &mut seen)? {
            items.push(item);
        }
    }

    let next_offset = (offset as u64)
        .checked_add(list.len() as u64)
        .ok_or_else(|| E::from("parse_error"))?;
    let has_more = !list.is_empty() && !items.is_empty() && next_offset < total;

    let mut result = json!({ "items": items });
    if has_more {
        result["nextCursor"] = json!(next_offset.to_string());
    }
    Ok(result)
}

pub(super) fn run(r: &[u8]) -> u32 {
    super::run("get_comments", r)
}
