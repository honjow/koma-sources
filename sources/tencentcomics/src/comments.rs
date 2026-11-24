use super::{
    body_buf, child_attr, child_text, decode_json_body, http_out, http_req_buf, numeric_id,
    payload_buf, quoted, select_all, trim_ascii, Doc, BASE, UA,
};
use koma_source_sdk::comments;
use koma_source_sdk::host::{
    html_attr, html_close, html_parse, html_select, html_text, http_request, HtmlDescriptor,
};
use koma_source_sdk::json_utils::{extract_json_number, write_bytes, write_usize};
use koma_source_sdk::{build_get_request, fetch_error_code, FetchError};

type Error = (&'static str, &'static str);

const MAX_COMMENT_NODES: usize = 256;
const MAX_TIME_NODES: usize = 16;

struct NodeList<const N: usize> {
    handles: [i32; N],
    len: usize,
}

impl<const N: usize> NodeList<N> {
    fn select(parent: HtmlDescriptor, selector: &[u8]) -> Result<Self, Error> {
        let mut list = Self {
            handles: [-1; N],
            len: 0,
        };
        let total = unsafe {
            select_all(
                parent.raw(),
                selector.as_ptr(),
                selector.len() as u32,
                list.handles.as_mut_ptr() as *mut u8,
                (N * 4) as u32,
            )
        };
        if total < 0 {
            return Err(("parse_error", "html select failed"));
        }
        let total = total as usize;
        list.len = total.min(N);
        if total > N {
            return Err(("parse_error", "html selection exceeded capacity"));
        }
        Ok(list)
    }

    fn at(&self, index: usize) -> HtmlDescriptor {
        HtmlDescriptor::from_raw(self.handles[index])
    }
}

impl<const N: usize> Drop for NodeList<N> {
    fn drop(&mut self) {
        for &raw in &self.handles[..self.len] {
            if raw > 0 {
                let _ = html_close(HtmlDescriptor::from_raw(raw));
            }
        }
    }
}

fn trim_text(bytes: &[u8]) -> &[u8] {
    match core::str::from_utf8(bytes) {
        Ok(s) => s.trim().as_bytes(),
        Err(_) => trim_ascii(bytes),
    }
}

fn safe_utf8_prefix(bytes: &[u8]) -> &[u8] {
    let mut end = bytes.len();
    while end > 0 && core::str::from_utf8(&bytes[..end]).is_err() {
        end -= 1;
    }
    &bytes[..end]
}

fn parse_usize_bytes(bytes: &[u8]) -> Option<usize> {
    let trimmed = trim_text(bytes);
    if trimmed.is_empty() || !trimmed.iter().all(u8::is_ascii_digit) {
        return None;
    }
    core::str::from_utf8(trimmed).ok()?.parse::<usize>().ok()
}

fn read_hidden_usize(doc: HtmlDescriptor, selector: &[u8]) -> Option<usize> {
    let mut buf = [0u8; 32];
    let raw = child_attr(doc, selector, b"value", &mut buf)?;
    parse_usize_bytes(raw)
}

fn looks_like_datetime(bytes: &[u8]) -> bool {
    let trimmed = trim_text(bytes);
    if trimmed.is_empty() || trimmed.starts_with("来自".as_bytes()) {
        return false;
    }
    if trimmed == "刚刚".as_bytes() || trimmed == "今天".as_bytes() || trimmed == "昨天".as_bytes() {
        return true;
    }
    let has_digit = trimmed.iter().any(u8::is_ascii_digit);
    if !has_digit {
        return false;
    }
    if trimmed.iter().any(|&b| matches!(b, b'-' | b'/' | b':')) {
        return true;
    }
    let Ok(s) = core::str::from_utf8(trimmed) else {
        return false;
    };
    s.contains("年")
        || s.contains("月")
        || s.contains("日")
        || s.contains("天前")
        || s.contains("小时")
        || s.contains("分钟")
        || s.contains("秒前")
        || s.contains("今天")
        || s.contains("昨天")
}

fn write_out(out: &mut [u8], offset: &mut usize, bytes: &[u8]) -> Result<(), Error> {
    write_bytes(out, offset, bytes)
        .then_some(())
        .ok_or(("result_too_large", "comments payload overflow"))
}

fn write_quoted(out: &mut [u8], offset: &mut usize, bytes: &[u8]) -> Result<(), Error> {
    quoted(out, offset, bytes)
        .then_some(())
        .ok_or(("result_too_large", "comments payload overflow"))
}

pub(super) fn load(request: &[u8]) -> Result<usize, Error> {
    let query = comments::request(request).map_err(|code| (code, "invalid comments request"))?;
    if query.chapter_id.is_some() {
        return Err(("invalid_request", "only manga scope supported"));
    }
    let comic_id = numeric_id(query.manga_id, b"tencent:manga:")
        .filter(|id| id.iter().any(|&b| b != b'0'))
        .ok_or(("invalid_request", "invalid manga identity"))?;
    let page = comments::position(query.cursor)
        .map_err(|code| (code, "invalid cursor"))?
        .unwrap_or(1);
    if page == 0 || page > 100_000 {
        return Err(("invalid_request", "invalid comment page"));
    }

    let mut url_buf = [0u8; 256];
    let mut url_len = 0;
    if !(write_bytes(&mut url_buf, &mut url_len, BASE)
        && write_bytes(&mut url_buf, &mut url_len, b"/Community/topicList?targetId=")
        && write_bytes(&mut url_buf, &mut url_len, comic_id)
        && write_bytes(&mut url_buf, &mut url_len, b"&page=")
        && write_usize(&mut url_buf, &mut url_len, page)
        && write_bytes(&mut url_buf, &mut url_len, b"&type=0&hd=0"))
    {
        return Err(("invalid_request", "comment URL overflow"));
    }

    let mut ref_buf = [0u8; 128];
    let mut ref_len = 0;
    if !(write_bytes(&mut ref_buf, &mut ref_len, BASE)
        && write_bytes(&mut ref_buf, &mut ref_len, b"/Comic/comicInfo/id/")
        && write_bytes(&mut ref_buf, &mut ref_len, comic_id))
    {
        return Err(("invalid_request", "comment referer overflow"));
    }

    let req_len = build_get_request(
        http_req_buf(),
        &url_buf[..url_len],
        Some(&ref_buf[..ref_len]),
        &[
            (b"User-Agent", UA),
            (b"X-Requested-With", b"XMLHttpRequest"),
        ],
    )
    .ok_or(("internal_error", "request encode error"))?;
    let resp_len = http_request(&http_req_buf()[..req_len], http_out())
        .map_err(|_| ("network_error", "network request failed"))?;
    let raw_resp = &http_out()[..resp_len];
    let status = extract_json_number(raw_resp, b"statusCode")
        .or_else(|| extract_json_number(raw_resp, b"status"))
        .and_then(|digits| {
            digits.iter().try_fold(0u32, |acc, &b| {
                b.is_ascii_digit()
                    .then(|| acc.checked_mul(10)?.checked_add((b - b'0') as u32))
                    .flatten()
            })
        })
        .ok_or(("network_error", "missing http status"))?;
    let fetch_err = match status {
        200..=299 => None,
        404 => Some(FetchError::NotFound),
        429 => Some(FetchError::RateLimit),
        400..=499 => Some(FetchError::ClientError),
        500..=599 => Some(FetchError::ServerError),
        _ => Some(FetchError::Network),
    };
    if let Some(err) = fetch_err {
        return Err(fetch_error_code(err));
    }
    let body_len = decode_json_body(raw_resp).map_err(fetch_error_code)?;
    let html = trim_ascii(&body_buf()[..body_len]);
    if html.is_empty() {
        return Err(("parse_error", "empty comments html"));
    }

    let doc = Doc(html_parse(html).map_err(|_| ("parse_error", "invalid comments html"))?);
    let total_num = read_hidden_usize(doc.0, b"input#totalNum");
    let page_index = read_hidden_usize(doc.0, b"input#pageIndex");
    let page_size = read_hidden_usize(doc.0, b"input#pageSize");

    if let Some(returned_page) = page_index {
        if returned_page != page {
            let out = payload_buf();
            let empty = br#"{"items":[]}"#;
            out[..empty.len()].copy_from_slice(empty);
            return Ok(empty.len());
        }
    }

    let cards = NodeList::<MAX_COMMENT_NODES>::select(doc.0, b".comment-list-content-wr")?;
    if cards.len == 0 && total_num.is_none() {
        return Err(("parse_error", "missing comments metadata"));
    }

    let out = payload_buf();
    let mut offset = 0;
    let mut written = 0usize;
    write_out(out, &mut offset, br#"{"items":["#)?;

    for i in 0..cards.len {
        let node = cards.at(i);

        let mut id_buf = [0u8; 128];
        let id_len = html_attr(node, b"data-c", &mut id_buf)
            .map_err(|_| ("parse_error", "missing comment id"))?;
        if id_len >= id_buf.len() {
            return Err(("parse_error", "comment id too long"));
        }
        let id = trim_text(&id_buf[..id_len]);
        if id.is_empty() {
            return Err(("parse_error", "empty comment id"));
        }

        let mut nick_buf = [0u8; 1024];
        let mut name_buf = [0u8; 1024];
        let nick = html_attr(node, b"data-nick", &mut nick_buf)
            .ok()
            .map(|n| trim_text(safe_utf8_prefix(&nick_buf[..n.min(nick_buf.len())])))
            .unwrap_or(b"");
        let author = if !nick.is_empty() {
            nick
        } else {
            child_text(node, b".comment-content-name", &mut name_buf)
                .map(|v| trim_text(safe_utf8_prefix(v)))
                .unwrap_or(b"")
        };

        let mut content_buf = [0u8; 16384];
        let detail_node = Doc(
            html_select(node, b".comment-content-detail")
                .map_err(|_| ("parse_error", "missing comment content"))?,
        );
        let content_raw_len = html_text(detail_node.0, &mut content_buf)
            .map_err(|_| ("parse_error", "failed to read comment content"))?;
        let content = trim_text(safe_utf8_prefix(
            &content_buf[..content_raw_len.min(content_buf.len())],
        ));
        if content.is_empty() {
            continue;
        }

        let times = NodeList::<MAX_TIME_NODES>::select(node, b".comment-remark .comment-time")?;
        let mut time_buf = [0u8; 128];
        let mut time_len = 0usize;
        for j in 0..times.len {
            let mut cand_buf = [0u8; 128];
            if let Ok(n) = html_text(times.at(j), &mut cand_buf) {
                let cand = trim_text(safe_utf8_prefix(&cand_buf[..n.min(cand_buf.len())]));
                if looks_like_datetime(cand) {
                    time_len = cand.len();
                    time_buf[..time_len].copy_from_slice(cand);
                }
            }
        }

        if written > 0 {
            write_out(out, &mut offset, b",")?;
        }
        write_out(out, &mut offset, br#"{"id":"#)?;
        write_quoted(out, &mut offset, id)?;
        write_out(out, &mut offset, br#","author":"#)?;
        write_quoted(out, &mut offset, author)?;
        write_out(out, &mut offset, br#","content":"#)?;
        write_quoted(out, &mut offset, content)?;
        if time_len > 0 {
            write_out(out, &mut offset, br#","createdAt":"#)?;
            write_quoted(out, &mut offset, &time_buf[..time_len])?;
        }
        write_out(out, &mut offset, b"}")?;
        written += 1;
    }

    write_out(out, &mut offset, b"]")?;
    let has_next = written > 0
        && matches!(
            (page_size, total_num),
            (Some(ps), Some(total)) if ps > 0 && page.checked_mul(ps).is_some_and(|covered| covered < total)
        );
    if has_next {
        write_out(out, &mut offset, br#","nextCursor":""#)?;
        write_usize(out, &mut offset, page + 1)
            .then_some(())
            .ok_or(("result_too_large", "comments payload overflow"))?;
        write_out(out, &mut offset, b"\"")?;
    }
    write_out(out, &mut offset, b"}")?;
    Ok(offset)
}
