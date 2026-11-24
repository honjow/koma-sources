use super::*;
use koma_source_sdk::comments;

fn positive_manga_id(v: &Value) -> R<String> {
    let s = v["args"]["mangaId"]
        .as_str()
        .and_then(|s| s.strip_prefix("wnacg:"))
        .ok_or_else(|| E::from("invalid_request"))?;
    if !digits(s) || s.starts_with('0') {
        return Err("invalid_request".into());
    }
    Ok(s.to_string())
}

fn parse_page(cursor: Option<&[u8]>) -> R<u32> {
    let Some(raw) = cursor else {
        return Ok(1);
    };
    let page = comments::position(Some(raw)).map_err(E::from)?.unwrap_or(0);
    if page == 0 || page > i32::MAX as usize || raw != page.to_string().as_bytes() {
        return Err("invalid_request".into());
    }
    Ok(page as u32)
}

fn find_tag_end(b: &[u8], start: usize) -> Option<(usize, bool)> {
    let mut i = start;
    let mut quote: Option<u8> = None;
    while i < b.len() {
        let c = b[i];
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
        } else if c == b'\'' || c == b'"' {
            quote = Some(c);
        } else if c == b'>' {
            let mut k = i;
            while k > start && b[k - 1].is_ascii_whitespace() {
                k -= 1;
            }
            return Some((i + 1, k > start && b[k - 1] == b'/'));
        }
        i += 1;
    }
    None
}

fn ascii_lower_contains(hay: &str, needle: &str) -> bool {
    let h = hay.as_bytes();
    let n = needle.as_bytes();
    if n.is_empty() || h.len() < n.len() {
        return false;
    }
    h.windows(n.len()).any(|w| w.eq_ignore_ascii_case(n))
}

fn has_action_attr(attrs: &str) -> bool {
    if ascii_lower_contains(attrs, "onclick") || ascii_lower_contains(attrs, "javascript:") {
        return true;
    }
    let lower = attrs.to_ascii_lowercase();
    for key in [
        "like", "likes", "pllike", "zan", "plzan", "vote", "plvote", "agree", "thumb", "thumbs",
        "good", "digg", "btn", "button", "plup", "plop", "plact", "plbtn", "action", "actions",
    ] {
        for token in lower.split(|c: char| !c.is_ascii_alphanumeric()) {
            if token == key {
                return true;
            }
        }
    }
    false
}

fn should_strip_tag(tag: &str, attrs: &str) -> bool {
    match tag {
        "button" | "svg" | "script" | "style" => true,
        "a" | "span" | "div" | "em" | "i" | "small" => has_action_attr(attrs),
        _ => false,
    }
}

fn find_matching_end(html: &str, tag: &str, after_open: usize) -> Option<usize> {
    let b = html.as_bytes();
    let mut i = after_open;
    let mut depth = 1usize;
    while i < b.len() {
        let Some(rel) = b[i..].iter().position(|&c| c == b'<') else {
            return None;
        };
        let lt = i + rel;
        if b[lt..].starts_with(b"<!--") {
            if let Some(end) = html[lt + 4..].find("-->") {
                i = lt + 4 + end + 3;
                continue;
            }
            return None;
        }
        let Some((tag_end, self_closing)) = find_tag_end(b, lt + 1) else {
            return None;
        };
        let inner = html[lt + 1..tag_end - 1].trim_start();
        if let Some(rest) = inner.strip_prefix('/') {
            let name_end = rest
                .find(|c: char| c.is_ascii_whitespace() || c == '/')
                .unwrap_or(rest.len());
            if rest[..name_end].eq_ignore_ascii_case(tag) {
                depth -= 1;
                if depth == 0 {
                    return Some(tag_end);
                }
            }
        } else {
            let name_end = inner
                .find(|c: char| c.is_ascii_whitespace() || c == '/')
                .unwrap_or(inner.len());
            if !self_closing && inner[..name_end].eq_ignore_ascii_case(tag) {
                depth += 1;
            }
        }
        i = tag_end;
    }
    None
}

fn strip_comment_buttons(html: &str) -> String {
    let b = html.as_bytes();
    let mut out = String::with_capacity(html.len());
    let mut i = 0usize;
    while i < b.len() {
        let Some(rel) = b[i..].iter().position(|&c| c == b'<') else {
            out.push_str(&html[i..]);
            break;
        };
        let lt = i + rel;
        out.push_str(&html[i..lt]);
        if b[lt..].starts_with(b"<!--") {
            if let Some(end) = html[lt + 4..].find("-->") {
                let end_pos = lt + 4 + end + 3;
                out.push_str(&html[lt..end_pos]);
                i = end_pos;
                continue;
            }
        }
        let Some((tag_end, self_closing)) = find_tag_end(b, lt + 1) else {
            out.push_str(&html[lt..]);
            break;
        };
        let inner = &html[lt + 1..tag_end - 1];
        let trimmed = inner.trim_start();
        let name_end = trimmed
            .find(|c: char| c.is_ascii_whitespace() || c == '/')
            .unwrap_or(trimmed.len());
        let name_raw = &trimmed[..name_end];
        if !name_raw.is_empty() && name_raw.bytes().all(|c| c.is_ascii_alphabetic()) {
            let tag = name_raw.to_ascii_lowercase();
            let attrs = trimmed[name_end..].trim_end_matches('/');
            if should_strip_tag(&tag, attrs) {
                if self_closing {
                    i = tag_end;
                    continue;
                }
                if let Some(close_end) = find_matching_end(html, &tag, tag_end) {
                    i = close_end;
                    continue;
                }
            }
        }
        out.push_str(&html[lt..tag_end]);
        i = tag_end;
    }
    out
}

fn clean_comment_text(raw: &str) -> String {
    let mut lines = Vec::new();
    let mut prev_empty = true;
    for line in raw.lines() {
        let cleaned = line.split_whitespace().collect::<Vec<_>>().join(" ");
        if cleaned.is_empty() {
            if !prev_empty {
                lines.push(String::new());
                prev_empty = true;
            }
        } else {
            lines.push(cleaned);
            prev_empty = false;
        }
    }
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    lines.join("\n")
}

pub(super) fn get(r: &[u8], v: &Value) -> R<Value> {
    let request = comments::request(r).map_err(E::from)?;
    if request.chapter_id.is_some() {
        return Err("unsupported_operation".into());
    }
    let aid = positive_manga_id(v)?;
    let page = parse_page(request.cursor)?;
    if host::check_cancel() {
        return Err("cancelled".into());
    }
    let cx = Context::network(r, v)?;
    let detail_url = format!("{}{}", cx.base, work_path(&aid));
    let first_url = format!("{}/comment-index-aid-{}.html", cx.base, aid);
    let (url, referer) = if page == 1 {
        (first_url, detail_url)
    } else {
        (
            format!(
                "{}/?ctl=comment&act=frag&aid={}&sort=hot&page={}",
                cx.base, aid, page
            ),
            first_url,
        )
    };
    let raw_html = http_with_referer(&url, &referer)?;
    let html_str = core::str::from_utf8(&raw_html).map_err(|_| E::from("parse_error"))?;
    let stripped = strip_comment_buttons(html_str);
    let doc = parse_html_with_br(&stripped)?;
    let nodes = if page == 1 {
        if doc.one("#plList").is_none() {
            return Err(E {
                code: "parse_error",
                message: LOGIN_HINT.to_string(),
            });
        }
        doc.all("#plList div.plItem")?
    } else {
        let list = doc.all("div.plItem")?;
        if list.is_empty() && doc.one("title").is_some() {
            return Err(E {
                code: "parse_error",
                message: LOGIN_HINT.to_string(),
            });
        }
        list
    };
    let raw_count = nodes.len();
    let mut seen = BTreeSet::new();
    let mut items = Vec::with_capacity(raw_count);
    for item in &nodes {
        if host::check_cancel() {
            return Err("cancelled".into());
        }
        let id = item
            .attr("data-id")?
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| E::from("parse_error"))?;
        if !seen.insert(id.clone()) {
            continue;
        }
        let author = item
            .one(".plName")
            .map(|n| n.text())
            .transpose()?
            .unwrap_or_default();
        let content_node = item.required(".plText")?;
        let content = clean_comment_text(&content_node.whole()?);
        let mut row = json!({
            "id": id,
            "author": author,
            "content": content,
        });
        if let Some(time_node) = item.one(".plTime") {
            let created = time_node.text()?;
            if !created.is_empty() {
                row["createdAt"] = json!(created);
            }
        }
        items.push(row);
    }
    let mut result = json!({ "items": items });
    if !items.is_empty() && raw_count >= 10 {
        let next = page
            .checked_add(1)
            .filter(|&n| n <= i32::MAX as u32)
            .ok_or_else(|| E::from("result_too_large"))?;
        result["nextCursor"] = json!(next.to_string());
    }
    Ok(result)
}

pub(super) fn run(r: &[u8]) -> u32 {
    super::run("get_comments", r)
}
