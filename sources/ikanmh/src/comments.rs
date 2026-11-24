use alloc::{format, string::String, vec::Vec};
use koma_source_sdk::comments::{self, OPERATION};
use serde_json::{json, Value};

use super::{document, mirror, port, valid_path, Node, Result};

fn comment_id(author: &str, date: &str, content: &str) -> u64 {
    // The site does not expose a comment ID: desktop vote IDs are repeated.
    let mut hash = 0xcbf29ce484222325u64;
    for field in [author, date, content] {
        for byte in field.bytes().chain(core::iter::once(0)) {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
        }
    }
    hash
}

fn read(request: &[u8]) -> Result<Value> {
    let query = comments::request(request)?;
    if query.chapter_id.is_some() || query.cursor.is_some() {
        return Err("invalid_request");
    }
    let id = core::str::from_utf8(query.manga_id).map_err(|_| "invalid_request")?;
    if !valid_path(id, "/book/") {
        return Err("invalid_request");
    }
    let input = port::input(request)?;
    let page = document(&format!("{}{id}", mirror(&input)))?;
    if page.require("input#book_id")?.attr("value")? != &id[6..] {
        return Err("parse_error");
    }
    let (rows, mobile): (Vec<Node>, bool) =
        if let Some(list) = page.one("#comment ul.detail-list-comment") {
            (list.all("li > div.detail-list-comment-info")?, true)
        } else {
            let list = page.require(".view-comment-main ul.list")?;
            (list.all("li > ul.postlist > li.dashed > div.info")?, false)
        };
    let mut items = Vec::new();
    let mut seen = Vec::new();
    for row in rows {
        let (author, date, content) = if mobile {
            let title = row.require("p.detail-list-comment-title")?;
            let text = title.text()?;
            let vote = title.require("a.zanbtn")?.text()?;
            let author = if vote.is_empty() {
                text.as_str()
            } else {
                text.strip_suffix(&vote).ok_or("parse_error")?.trim_end()
            };
            (
                String::from(author),
                row.require("p.detail-list-comment-subtitle")?.text()?,
                row.require("p.detail-list-comment-content")?.text()?,
            )
        } else {
            let date = row.require("p.bottom")?.text()?;
            (
                row.require("p.title")?.text()?,
                date.split_whitespace().next().unwrap_or("").into(),
                row.require("p.content")?.text()?,
            )
        };
        let hash = comment_id(&author, &date, &content);
        let duplicate = seen.iter().filter(|&&previous| previous == hash).count();
        seen.push(hash);
        items.push(
            json!({"id":format!("{hash:016x}-{duplicate}"), "author":author,
                          "content":content, "createdAt":date}),
        );
    }
    // Both observed site templates render their displayed list in the page and expose
    // no next-page operation. The hard-coded 999+ label is not a pagination signal.
    Ok(json!({"items":items}))
}

pub(super) fn run(request: &[u8]) -> u32 {
    port::write_result(OPERATION, read(request))
}
