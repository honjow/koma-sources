use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use koma_source_sdk::comments::{self, OPERATION};
use serde_json::{json, Value};
use url::{form_urlencoded, Url};

use super::{get, port, work, Node, Result, BASE};

fn text(raw: &[u8]) -> Result<&str> {
    core::str::from_utf8(raw).map_err(|_| "invalid_request")
}

fn manga_topic(path: &str) -> Result<String> {
    let doc = Node::parse(&get(path)?)?;
    let href = doc.require("a[href*='showcomment'][href]")?.attr("href")?;
    let url = Url::parse(BASE)
        .and_then(|base| base.join(&href))
        .map_err(|_| "parse_error")?;
    if url.host_str() != Some("www.manben.com") || url.path() != "/showcomment/" {
        return Err("parse_error");
    }
    let id = url
        .query_pairs()
        .find(|(key, _)| key == "mid")
        .map(|(_, value)| value.into_owned())
        .ok_or("parse_error")?;
    if id.is_empty() || id.len() > 20 || !id.bytes().all(|c| c.is_ascii_digit()) {
        return Err("parse_error");
    }
    Ok(id)
}

fn first_page(mid: &str) -> Result<Vec<Value>> {
    let html = get(&format!("/showcomment/?mid={mid}"))?;
    let doc = Node::parse(&html)?;
    let list = doc.require("ul.commentList")?;
    let mut items = Vec::new();
    for row in list.all("li")? {
        let title = row.require("p.title")?;
        let classes = title.attr("class")?;
        let id = classes
            .split_whitespace()
            .find_map(|name| name.strip_prefix("n_"))
            .filter(|id| !id.is_empty() && id.bytes().all(|c| c.is_ascii_digit()))
            .ok_or("parse_error")?;
        let mut item =
            json!({"id":id, "author":title.text()?, "content":row.require("p.content")?.text()?});
        let subtitle = row.require("p.subtitle")?.text()?;
        let mut parts = subtitle.split_whitespace();
        if let (Some(date), Some(time)) = (parts.next(), parts.next()) {
            if date.len() == 10 && time.len() == 8 {
                item["createdAt"] = json!(format!("{date} {time}"));
            }
        }
        items.push(item);
    }
    Ok(items)
}

fn plain_text(value: &str) -> Result<String> {
    // The site inserts these strings into HTML; decode entities and tags before ABI output.
    Node::parse(&format!("<div>{value}</div>"))?
        .require("div")?
        .text()
}

fn later_page(mid: &str, page: usize) -> Result<Vec<Value>> {
    let form = form_urlencoded::Serializer::new(String::new())
        .extend_pairs([("t", "4"), ("pageindex", &page.to_string()), ("mid", mid)])
        .finish();
    let body = port::request(
        &format!("{BASE}/showcomment/pagerdata.ashx"),
        "POST",
        json!({"Content-Type":"application/x-www-form-urlencoded", "X-Requested-With":"XMLHttpRequest",
               "Referer":format!("{BASE}/showcomment/?mid={mid}"), "User-Agent":port::MOBILE_IMAGE_AGENT,
               "Accept":"application/json"}),
        Some(&form),
    )?;
    let rows: Vec<Value> = serde_json::from_str(&body).map_err(|_| "parse_error")?;
    let mut items = Vec::new();
    for row in rows {
        let id = row["Id"]
            .as_u64()
            .map(|id| id.to_string())
            .or_else(|| {
                row["Id"]
                    .as_str()
                    .filter(|id| !id.is_empty() && id.bytes().all(|c| c.is_ascii_digit()))
                    .map(ToString::to_string)
            })
            .ok_or("parse_error")?;
        let author = row["Poster"].as_str().ok_or("parse_error")?;
        let content = row["PostContent"].as_str().ok_or("parse_error")?;
        let mut item = json!({"id":id,"author":plain_text(author)?,"content":plain_text(content)?});
        if let Some(date) = row["PostTime"].as_str().filter(|date| !date.is_empty()) {
            item["createdAt"] = json!(date);
        }
        items.push(item);
    }
    Ok(items)
}

fn read(input: &[u8]) -> Result<Value> {
    let request = comments::request(input)?;
    if request.chapter_id.is_some() {
        return Err("unsupported_operation");
    }
    let path = work(text(request.manga_id)?)?;
    let page = comments::position(request.cursor)?.unwrap_or(1);
    if page == 0 || page > 100_000 {
        return Err("invalid_request");
    }
    let mid = manga_topic(&path)?;
    // The website renders page 1 as HTML and starts its AJAX stream at page 2.
    // The AJAX page 1 is a different list, so it must not replace the visible first page.
    let items = if page == 1 {
        first_page(&mid)?
    } else {
        later_page(&mid, page)?
    };
    let mut output = json!({"items":items});
    if !items.is_empty() {
        output["nextCursor"] = json!((page + 1).to_string());
    }
    Ok(output)
}

pub(super) fn run(input: &[u8]) -> u32 {
    port::write_result(OPERATION, read(input))
}
