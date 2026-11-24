use super::{
    body_buf, bytes, diagnostic_error, diagnostic_status, http_out, http_req_buf, json_string_end,
    json_value_end, number, payload_buf, positive, q, transport, trim_ascii, unescape, unique, ws,
    write_error, write_success_payload, BASE, LANGUAGE, LAST_HTTP_STATUS, UA,
};
use koma_source_sdk::{
    comments, host,
    json_utils::{append_json_escaped, json_object_member, write_bytes, write_usize},
};

const PAGE_SIZE: usize = 30;
const MAX_COMMENTS_INDEX: usize = 2048;
const MAX_SEEN_IDS: usize = 512;

static mut COMMENT_KEYS_BUF: [u8; 64 * 1024] = [0; 64 * 1024];
static mut COMMENT_INDEX: [(u32, u16, u32, u32); MAX_COMMENTS_INDEX] =
    [(0, 0, 0, 0); MAX_COMMENTS_INDEX];
static mut SEEN_IDS_BUF: [u8; 16 * 1024] = [0; 16 * 1024];
static mut SEEN_IDS: [(u16, u16); MAX_SEEN_IDS] = [(0, 0); MAX_SEEN_IDS];
static mut ITEM_ID_BUF: [u8; 256] = [0; 256];
static mut AUTHOR_BUF: [u8; 4096] = [0; 4096];
static mut PARENT_BUF: [u8; 4096] = [0; 4096];
static mut RAW_CONTENT_BUF: [u8; 64 * 1024] = [0; 64 * 1024];
static mut HTML_WRAP_BUF: [u8; 64 * 1024 + 32] = [0; 64 * 1024 + 32];
static mut CLEAN_CONTENT_BUF: [u8; 64 * 1024] = [0; 64 * 1024];
static mut ENTRY_BUF: [u8; 16 * 1024] = [0; 16 * 1024];
static mut TIME_BUF: [u8; 128] = [0; 128];

type ErrInfo = (&'static str, &'static str);

fn map_fetch_error(e: &'static str) -> ErrInfo {
    let code = match e {
        "cancelled" => "cancelled",
        "request timed out" => "timeout",
        "network request failed"
        | "transport unavailable"
        | "transport rejected"
        | "invalid transport response" => "network_error",
        "not found" => "not_found",
        "rate limited" => "rate_limited",
        "public request rejected" => "permission_denied",
        "authentication required" => "authentication_required",
        "server error" => "server_error",
        "HTTP client error" => "client_error",
        _ => "network_error",
    };
    (code, e)
}

fn build_comment_request(
    dst: &mut [u8],
    url: &[u8],
    referer: &[u8],
) -> core::result::Result<usize, &'static str> {
    let mut n = 0;
    bytes(dst, &mut n, br#"{"version":1,"method":"GET","url":"#)?;
    q(dst, &mut n, url)?;
    bytes(dst, &mut n, br#","headers":{"User-Agent":"#)?;
    q(dst, &mut n, UA)?;
    bytes(dst, &mut n, br#","Accept-Language":"#)?;
    q(dst, &mut n, LANGUAGE)?;
    bytes(dst, &mut n, br#","Referer":"#)?;
    q(dst, &mut n, referer)?;
    bytes(dst, &mut n, br#","Origin":"#)?;
    q(dst, &mut n, BASE)?;
    bytes(
        dst,
        &mut n,
        br#","X-Requested-With":"XMLHttpRequest"},"timeoutMs":15000,"responseKind":"bodyText"}"#,
    )?;
    Ok(n)
}

fn fetch_comments_json(manga_id: &[u8], page: usize) -> core::result::Result<usize, ErrInfo> {
    unsafe { LAST_HTTP_STATUS = 0 };
    host::log_info(b"manhuagui fetch stage=enter");
    if host::check_cancel() {
        return Err(("cancelled", "cancelled"));
    }
    let mut url_buf = [0u8; 512];
    let mut url_len = 0usize;
    if !write_bytes(&mut url_buf, &mut url_len, BASE)
        || !write_bytes(
            &mut url_buf,
            &mut url_len,
            b"/tools/submit_ajax.ashx?action=comment_list&book_id=",
        )
        || !write_bytes(&mut url_buf, &mut url_len, manga_id)
        || !write_bytes(&mut url_buf, &mut url_len, b"&page_index=")
        || !write_usize(&mut url_buf, &mut url_len, page)
    {
        return Err(("invalid_request", "comment URL overflow"));
    }
    let mut referer_buf = [0u8; 256];
    let mut referer_len = 0usize;
    if !write_bytes(&mut referer_buf, &mut referer_len, BASE)
        || !write_bytes(&mut referer_buf, &mut referer_len, b"/comic/")
        || !write_bytes(&mut referer_buf, &mut referer_len, manga_id)
        || !write_bytes(&mut referer_buf, &mut referer_len, b"/")
    {
        return Err(("invalid_request", "comment referer overflow"));
    }
    let req_buf = http_req_buf();
    let req_len = build_comment_request(
        req_buf,
        &url_buf[..url_len],
        &referer_buf[..referer_len],
    )
    .map_err(|_| ("invalid_request", "request overflow"))?;
    host::log_info(b"manhuagui fetch stage=request_ready");
    let size = host::http_request(&req_buf[..req_len], http_out())
        .map_err(|_| map_fetch_error("network request failed"))?;
    host::log_info(b"manhuagui fetch stage=http_return");
    if size >= http_out().len() {
        return Err(("result_too_large", "HTTP response may be truncated"));
    }
    let response = &http_out()[..size];
    let status = json_object_member(response, b"status")
        .or_else(|| json_object_member(response, b"statusCode"))
        .map(koma_source_sdk::parse_status_code)
        .unwrap_or(0);
    if let Some(e) = transport::failed(response, status) {
        host::log_info(b"manhuagui fetch stage=transport_failed");
        return Err(map_fetch_error(e));
    }
    unsafe { LAST_HTTP_STATUS = status };
    diagnostic_status(status);
    if let Some(e) = transport::http_error(status) {
        return Err(map_fetch_error(e));
    }
    let n = koma_source_sdk::decode_json_body_into(response, body_buf())
        .map_err(|_| ("parse_error", "invalid HTTP body"))?;
    if n >= body_buf().len() {
        return Err(("result_too_large", "HTTP body may be truncated"));
    }
    host::log_info(b"manhuagui fetch stage=body_ready");
    Ok(n)
}

struct CommentsMap<'a> {
    raw: &'a [u8],
    keys_buf: &'a [u8],
    entries: &'a [(u32, u16, u32, u32)],
}

impl<'a> CommentsMap<'a> {
    fn parse(
        raw: &'a [u8],
        keys_buf: &'a mut [u8],
        index: &'a mut [(u32, u16, u32, u32)],
    ) -> core::result::Result<Self, ErrInfo> {
        let mut p = ws(raw, 0);
        if raw.get(p) != Some(&b'{') {
            return Err(("parse_error", "comments map must be object"));
        }
        p = ws(raw, p + 1);
        let mut keys_used = 0usize;
        let mut count = 0usize;
        if raw.get(p) == Some(&b'}') {
            if ws(raw, p + 1) != raw.len() {
                return Err(("parse_error", "comments map trailing data"));
            }
            return Ok(Self {
                raw,
                keys_buf: &keys_buf[..0],
                entries: &index[..0],
            });
        }
        loop {
            let k_end = json_string_end(raw, p)
                .ok_or(("parse_error", "invalid comments map key"))?;
            if count >= index.len() {
                return Err(("result_too_large", "too many comment entries"));
            }
            let decoded_len = unescape(&raw[p + 1..k_end - 1], &mut keys_buf[keys_used..])
                .map_err(|e| {
                    if e == "payload overflow" {
                        ("result_too_large", "comment keys overflow")
                    } else {
                        ("parse_error", "invalid comments map key")
                    }
                })?
                .len();
            if decoded_len > u16::MAX as usize {
                return Err(("result_too_large", "comment key too long"));
            }
            p = ws(raw, k_end);
            if raw.get(p) != Some(&b':') {
                return Err(("parse_error", "invalid comments map separator"));
            }
            p = ws(raw, p + 1);
            let v_start = p;
            let v_end = json_value_end(raw, v_start, 0)
                .ok_or(("parse_error", "invalid comments map value"))?;
            index[count] = (
                keys_used as u32,
                decoded_len as u16,
                v_start as u32,
                v_end as u32,
            );
            keys_used += decoded_len;
            count += 1;
            p = ws(raw, v_end);
            match raw.get(p) {
                Some(&b',') => p = ws(raw, p + 1),
                Some(&b'}') => {
                    if ws(raw, p + 1) != raw.len() {
                        return Err(("parse_error", "comments map trailing data"));
                    }
                    break;
                }
                _ => return Err(("parse_error", "invalid comments map")),
            }
        }
        Ok(Self {
            raw,
            keys_buf: &keys_buf[..keys_used],
            entries: &index[..count],
        })
    }

    fn find(&self, key: &[u8]) -> Option<&'a [u8]> {
        for &(k_off, k_len, v_start, v_end) in self.entries {
            let off = k_off as usize;
            let len = k_len as usize;
            if &self.keys_buf[off..off + len] == key {
                return Some(&self.raw[v_start as usize..v_end as usize]);
            }
        }
        None
    }
}

fn decode_json_str<'a>(
    v: &[u8],
    out: &'a mut [u8],
) -> core::result::Result<&'a [u8], ErrInfo> {
    if v.first() != Some(&b'"') || json_string_end(v, 0) != Some(v.len()) {
        return Err(("parse_error", "expected JSON string"));
    }
    unescape(&v[1..v.len() - 1], out).map_err(|_| ("parse_error", "invalid JSON string"))
}

fn optional_string<'a>(
    obj: &[u8],
    key: &[u8],
    out: &'a mut [u8],
) -> core::result::Result<&'a [u8], ErrInfo> {
    match json_object_member(obj, key) {
        None | Some(b"null") => Ok(b""),
        Some(v) => decode_json_str(v, out),
    }
}

fn scalar_id<'a>(
    obj: &[u8],
    fallback: &[u8],
    out: &'a mut [u8],
) -> core::result::Result<&'a [u8], ErrInfo> {
    match json_object_member(obj, b"id") {
        None | Some(b"null") => {
            if fallback.is_empty() || fallback.len() > out.len() {
                return Err(("parse_error", "invalid comment id"));
            }
            out[..fallback.len()].copy_from_slice(fallback);
            Ok(&out[..fallback.len()])
        }
        Some(v) if v.starts_with(b"\"") => {
            let s = decode_json_str(v, out)?;
            if s.is_empty() {
                return Err(("parse_error", "empty comment id"));
            }
            Ok(s)
        }
        Some(v) => {
            if v.is_empty() || !v.iter().all(u8::is_ascii_digit) || v.len() > out.len() {
                return Err(("parse_error", "invalid comment id"));
            }
            out[..v.len()].copy_from_slice(v);
            Ok(&out[..v.len()])
        }
    }
}

fn match_br_tag(raw: &[u8], start: usize) -> Option<usize> {
    if raw.get(start) != Some(&b'<') {
        return None;
    }
    let b1 = raw.get(start + 1)?;
    let b2 = raw.get(start + 2)?;
    if !matches!(b1, b'b' | b'B') || !matches!(b2, b'r' | b'R') {
        return None;
    }
    let mut i = start + 3;
    while raw.get(i).is_some_and(|c| c.is_ascii_whitespace()) {
        i += 1;
    }
    if raw.get(i) == Some(&b'/') {
        i += 1;
        while raw.get(i).is_some_and(|c| c.is_ascii_whitespace()) {
            i += 1;
        }
    }
    if raw.get(i) == Some(&b'>') {
        Some(i + 1)
    } else {
        None
    }
}

fn clean_content<'a>(
    raw: &[u8],
    clean_buf: &'a mut [u8],
) -> core::result::Result<&'a [u8], ErrInfo> {
    let trimmed = trim_ascii(raw);
    if trimmed.is_empty() {
        return Ok(b"");
    }
    if !trimmed.contains(&b'<') && !trimmed.contains(&b'&') {
        if trimmed.len() > clean_buf.len() {
            return Err(("result_too_large", "comment text overflow"));
        }
        clean_buf[..trimmed.len()].copy_from_slice(trimmed);
        return Ok(&clean_buf[..trimmed.len()]);
    }
    let wrap = unsafe { &mut *core::ptr::addr_of_mut!(HTML_WRAP_BUF) };
    let mut wn = 0usize;
    if !write_bytes(wrap, &mut wn, b"<div>") {
        return Err(("result_too_large", "comment HTML overflow"));
    }
    let mut i = 0usize;
    while i < trimmed.len() {
        if let Some(next) = match_br_tag(trimmed, i) {
            if !write_bytes(wrap, &mut wn, b"\n") {
                return Err(("result_too_large", "comment HTML overflow"));
            }
            i = next;
        } else {
            if wn >= wrap.len() {
                return Err(("result_too_large", "comment HTML overflow"));
            }
            wrap[wn] = trimmed[i];
            wn += 1;
            i += 1;
        }
    }
    if !write_bytes(wrap, &mut wn, b"</div>") {
        return Err(("result_too_large", "comment HTML overflow"));
    }
    let doc = super::Node::parse(&wrap[..wn])
        .map_err(|_| ("parse_error", "invalid comment HTML"))?;
    let n = host::html_text(doc.0, clean_buf)
        .map_err(|_| ("result_too_large", "comment text overflow"))?;
    drop(doc);
    if n >= clean_buf.len() {
        return Err(("result_too_large", "comment text overflow"));
    }
    Ok(trim_ascii(&clean_buf[..n]))
}

fn payload_bytes(out: &mut [u8], offset: &mut usize, s: &[u8]) -> core::result::Result<(), ErrInfo> {
    if write_bytes(out, offset, s) {
        Ok(())
    } else {
        Err(("result_too_large", "payload overflow"))
    }
}

fn payload_quote(out: &mut [u8], offset: &mut usize, s: &[u8]) -> core::result::Result<(), ErrInfo> {
    payload_bytes(out, offset, b"\"")?;
    if !append_json_escaped(out, offset, s) {
        return Err(("result_too_large", "payload overflow"));
    }
    payload_bytes(out, offset, b"\"")
}

fn load(request: &[u8]) -> core::result::Result<usize, ErrInfo> {
    let query = comments::request(request).map_err(|code| (code, "invalid comments request"))?;
    if query.chapter_id.is_some() {
        return Err(("unsupported_operation", "chapter comments unsupported"));
    }
    let mut manga_buf = [0u8; 256];
    let decoded_manga = unescape(query.manga_id, &mut manga_buf)
        .map_err(|_| ("invalid_request", "invalid manga ID"))?;
    let id_slice = decoded_manga
        .strip_prefix(b"comic/")
        .ok_or(("invalid_request", "invalid manga ID"))?;
    if id_slice.len() > 32 || !positive(id_slice) {
        return Err(("invalid_request", "invalid manga ID"));
    }
    let mut manga_id = [0u8; 32];
    let manga_id_len = id_slice.len();
    manga_id[..manga_id_len].copy_from_slice(id_slice);
    let manga_id = &manga_id[..manga_id_len];

    let page = comments::position(query.cursor)
        .map_err(|code| (code, "invalid comment cursor"))?
        .unwrap_or(1);
    if page == 0 || page > 1_000_000 {
        return Err(("invalid_request", "invalid comment cursor"));
    }

    let body_len = fetch_comments_json(manga_id, page)?;
    let body = trim_ascii(&body_buf()[..body_len]);
    let root_end = json_value_end(body, 0, 0)
        .ok_or(("parse_error", "invalid comments JSON"))?;
    if !body.starts_with(b"{") || ws(body, root_end) != body.len() {
        return Err(("parse_error", "invalid comments JSON"));
    }

    let total_raw = trim_ascii(
        unique(body, b"total").map_err(|_| ("parse_error", "missing comment total"))?,
    );
    let total = number(total_raw).ok_or(("parse_error", "invalid comment total"))?;

    let comment_ids_raw = trim_ascii(
        unique(body, b"commentIds").map_err(|_| ("parse_error", "missing commentIds"))?,
    );
    if !comment_ids_raw.starts_with(b"[") || !comment_ids_raw.ends_with(b"]") {
        return Err(("parse_error", "commentIds must be array"));
    }

    let mut id_pos = ws(comment_ids_raw, 1);
    if comment_ids_raw.get(id_pos) == Some(&b']') {
        if ws(comment_ids_raw, id_pos + 1) != comment_ids_raw.len() {
            return Err(("parse_error", "commentIds trailing data"));
        }
        let empty = br#"{"items":[]}"#;
        payload_buf()[..empty.len()].copy_from_slice(empty);
        return Ok(empty.len());
    }

    let comments_raw = trim_ascii(
        unique(body, b"comments").map_err(|_| ("parse_error", "missing comments map"))?,
    );
    let keys_buf = unsafe { &mut *core::ptr::addr_of_mut!(COMMENT_KEYS_BUF) };
    let index_buf = unsafe { &mut *core::ptr::addr_of_mut!(COMMENT_INDEX) };
    let comments_map = CommentsMap::parse(comments_raw, keys_buf, index_buf)?;

    let out = payload_buf();
    let mut out_len = 0usize;
    payload_bytes(out, &mut out_len, br#"{"items":["#)?;

    let seen_buf = unsafe { &mut *core::ptr::addr_of_mut!(SEEN_IDS_BUF) };
    let seen = unsafe { &mut *core::ptr::addr_of_mut!(SEEN_IDS) };
    let mut seen_used = 0usize;
    let mut seen_count = 0usize;
    let mut written = 0usize;

    loop {
        let end = json_string_end(comment_ids_raw, id_pos)
            .ok_or(("parse_error", "invalid commentIds entry"))?;
        let entry_buf = unsafe { &mut *core::ptr::addr_of_mut!(ENTRY_BUF) };
        let entry = unescape(&comment_ids_raw[id_pos + 1..end - 1], entry_buf)
            .map_err(|_| ("parse_error", "invalid commentIds string"))?;

        id_pos = ws(comment_ids_raw, end);
        let has_more_entries = match comment_ids_raw.get(id_pos) {
            Some(&b',') => {
                id_pos = ws(comment_ids_raw, id_pos + 1);
                if comment_ids_raw.get(id_pos) == Some(&b']') {
                    return Err(("parse_error", "trailing comma in commentIds"));
                }
                true
            }
            Some(&b']') => {
                if ws(comment_ids_raw, id_pos + 1) != comment_ids_raw.len() {
                    return Err(("parse_error", "commentIds trailing data"));
                }
                false
            }
            _ => return Err(("parse_error", "invalid commentIds array")),
        };

        let mut parts = entry.split(|&b| b == b',');
        let primary_id = trim_ascii(parts.next().unwrap_or(b""));
        let parent_id = parts.next().map(trim_ascii).filter(|s| !s.is_empty());
        if !primary_id.is_empty() {
            if let Some(row) = comments_map.find(primary_id) {
                if row != b"null" {
                    if !row.starts_with(b"{") {
                        return Err(("parse_error", "comment entry must be object"));
                    }
                    let item_id_buf = unsafe { &mut *core::ptr::addr_of_mut!(ITEM_ID_BUF) };
                    let item_id = scalar_id(row, primary_id, item_id_buf)?;
                    let duplicate = seen[..seen_count]
                        .iter()
                        .any(|&(off, len)| &seen_buf[off as usize..off as usize + len as usize] == item_id);
                    if !duplicate {
                        let author_buf = unsafe { &mut *core::ptr::addr_of_mut!(AUTHOR_BUF) };
                        let author = trim_ascii(optional_string(row, b"user_name", author_buf)?);

                        let parent_buf = unsafe { &mut *core::ptr::addr_of_mut!(PARENT_BUF) };
                        let parent_author = match parent_id.and_then(|pid| comments_map.find(pid)) {
                            Some(prow) if prow.starts_with(b"{") => {
                                trim_ascii(optional_string(prow, b"user_name", parent_buf)?)
                            }
                            _ => b"",
                        };

                        let raw_content_buf =
                            unsafe { &mut *core::ptr::addr_of_mut!(RAW_CONTENT_BUF) };
                        let raw_content = optional_string(row, b"content", raw_content_buf)?;
                        let clean_buf =
                            unsafe { &mut *core::ptr::addr_of_mut!(CLEAN_CONTENT_BUF) };
                        let cleaned = clean_content(raw_content, clean_buf)?;

                        let time_buf = unsafe { &mut *core::ptr::addr_of_mut!(TIME_BUF) };
                        let created_at = trim_ascii(optional_string(row, b"add_time", time_buf)?);

                        if written > 0 {
                            payload_bytes(out, &mut out_len, b",")?;
                        }
                        payload_bytes(out, &mut out_len, br#"{"id":"#)?;
                        payload_quote(out, &mut out_len, item_id)?;
                        payload_bytes(out, &mut out_len, br#","author":"#)?;
                        payload_quote(out, &mut out_len, author)?;
                        payload_bytes(out, &mut out_len, br#","content":""#)?;
                        if !parent_author.is_empty() {
                            if !append_json_escaped(out, &mut out_len, "回复 ".as_bytes())
                                || !append_json_escaped(out, &mut out_len, parent_author)
                                || !append_json_escaped(out, &mut out_len, "：".as_bytes())
                            {
                                return Err(("result_too_large", "payload overflow"));
                            }
                        }
                        if !append_json_escaped(out, &mut out_len, cleaned) {
                            return Err(("result_too_large", "payload overflow"));
                        }
                        payload_bytes(out, &mut out_len, b"\"")?;
                        if !created_at.is_empty() {
                            payload_bytes(out, &mut out_len, br#","createdAt":"#)?;
                            payload_quote(out, &mut out_len, created_at)?;
                        }
                        payload_bytes(out, &mut out_len, b"}")?;

                        if seen_count < seen.len() && seen_used + item_id.len() <= seen_buf.len() {
                            seen_buf[seen_used..seen_used + item_id.len()].copy_from_slice(item_id);
                            seen[seen_count] = (seen_used as u16, item_id.len() as u16);
                            seen_used += item_id.len();
                            seen_count += 1;
                        }
                        written += 1;
                    }
                }
            }
        }

        if !has_more_entries {
            break;
        }
    }

    payload_bytes(out, &mut out_len, b"]")?;
    if written > 0 && page.checked_mul(PAGE_SIZE).is_some_and(|end| end < total) {
        payload_bytes(out, &mut out_len, br#","nextCursor":""#)?;
        if !write_usize(out, &mut out_len, page + 1) {
            return Err(("result_too_large", "payload overflow"));
        }
        payload_bytes(out, &mut out_len, b"\"")?;
    }
    payload_bytes(out, &mut out_len, b"}")?;
    Ok(out_len)
}

pub(super) fn run(request: &[u8]) -> u32 {
    unsafe { LAST_HTTP_STATUS = 0 };
    match load(request) {
        Ok(len) => write_success_payload(comments::OPERATION, len),
        Err((code, msg)) => {
            let mut diagnostic = [0u8; 192];
            let mut n = 0usize;
            let _ = write_bytes(&mut diagnostic, &mut n, msg.as_bytes());
            let status = unsafe { LAST_HTTP_STATUS };
            if status > 0 {
                let _ = write_bytes(&mut diagnostic, &mut n, b" (HTTP ");
                let _ = write_usize(&mut diagnostic, &mut n, status as usize);
                let _ = write_bytes(&mut diagnostic, &mut n, b")");
            }
            let message = core::str::from_utf8(&diagnostic[..n]).unwrap_or(msg);
            diagnostic_error(comments::OPERATION, code);
            write_error(comments::OPERATION, code, message)
        }
    }
}
