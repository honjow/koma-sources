use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use koma_source_sdk::comments::{self, OPERATION};
use serde_json::{json, Value};
use url::form_urlencoded;

use super::{
    port_support::{self as port, Node, Result},
    SITE,
};

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

fn text(value: &[u8]) -> Result<&str> {
    core::str::from_utf8(value).map_err(|_| "invalid_request")
}

fn read(input: &[u8]) -> Result<Value> {
    let request = comments::request(input)?;
    let work = text(request.manga_id)?;
    let slug = work
        .strip_prefix('/')
        .and_then(|id| id.strip_suffix('/'))
        .filter(|id| valid_id(id))
        .ok_or("invalid_request")?;

    let chapter = if let Some(chapter_id) = request.chapter_id {
        let (owner, chapter) = text(chapter_id)?.split_once('/').ok_or("invalid_request")?;
        if owner != slug || !valid_id(chapter) {
            return Err("invalid_request");
        }
        Some(chapter)
    } else {
        None
    };
    let page = comments::position(request.cursor)?.unwrap_or(1);
    if page == 0 || page > 100_000 {
        return Err("invalid_request");
    }
    let route = match chapter {
        Some(chapter) => format!("{}/{slug}/{chapter}.html", SITE.mobile),
        None => format!("{}/{slug}/", SITE.mobile),
    };
    let html = port::get(&route)?;
    let doc = Node::parse(&html)?;
    let signature = doc.require("input.signkey[data-sign][value]")?;
    let form = form_urlencoded::Serializer::new(String::new())
        .extend_pairs([
            ("id", slug),
            ("vid", chapter.unwrap_or("nullvid")),
            ("signkey", &signature.attr("data-sign")?),
            ("time", &signature.attr("value")?),
            ("pl_page", &page.to_string()),
            ("iua", "2"),
            ("ref", "6"),
        ])
        .finish();
    let body = port::request(
        &format!("{}/getpl", SITE.mobile),
        "POST",
        json!({"Referer":route,"Origin":SITE.mobile,"X-Requested-With":"XMLHttpRequest",
               "Content-Type":"application/x-www-form-urlencoded","Accept":"application/json"}),
        Some(&form),
    )?;
    let response: Value = serde_json::from_str(&body).map_err(|_| "parse_error")?;
    match response["code"].as_str() {
        Some("201") => return Ok(json!({"items":[]})),
        Some("200") => {}
        Some("-1") => return Err("server_error"),
        _ => return Err("parse_error"),
    }
    let rows = response["data"].as_array().ok_or("parse_error")?;
    let mut items = Vec::new();
    for row in rows {
        let id = public_scalar(&row["id"])?;
        let author = row["nick"].as_str().unwrap_or("");
        let content = row["cont"].as_str().ok_or("parse_error")?;
        let mut item = json!({"id":id,"author":author,"content":content});
        if let Ok(stamp) = public_scalar(&row["addtime"]) {
            if let Ok(seconds) = stamp.parse::<u64>() {
                if let Some(milliseconds) = seconds.checked_mul(1000) {
                    let mut out = [0; 20];
                    if comments::utc_iso_millis(milliseconds.to_string().as_bytes(), &mut out)
                        .is_some()
                    {
                        item["createdAt"] = json!(text(&out)?);
                    }
                }
            }
        }
        items.push(item);
    }
    let mut result = json!({"items":items});
    // The public endpoint signals exhaustion via its next response (code 201).
    if !items.is_empty() {
        result["nextCursor"] = json!((page + 1).to_string());
    }
    Ok(result)
}

fn public_scalar(value: &Value) -> Result<String> {
    if let Some(raw) = value.as_str() {
        return Ok(raw.to_string());
    }
    value
        .as_u64()
        .map(|value| value.to_string())
        .ok_or("parse_error")
}

pub(super) fn run(request: &[u8]) -> u32 {
    port::write_result(OPERATION, read(request))
}
