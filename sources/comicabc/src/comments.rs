use koma_source_sdk::comments::{self, OPERATION};
use koma_source_sdk::host;
use koma_source_sdk::json_utils::{
    append_json_unescaped_then_escaped, write_bytes, write_usize, JsonArrayIter,
};
use koma_source_sdk::plain_text::decode_json_content;
use koma_source_sdk::source_protocol::{direct, raw_string};

use super::{
    failure, fetch_body_with_referer, payload_buf, source_error, trim_ascii,
    write_success_payload, BASE, BODY_BUF,
};

fn parse_usize(bytes: &[u8]) -> Option<usize> {
    if bytes.is_empty() || !bytes.iter().all(u8::is_ascii_digit) {
        return None;
    }
    core::str::from_utf8(bytes).ok()?.parse().ok()
}

fn parse_item_number(raw: &[u8]) -> Option<usize> {
    if raw.len() >= 2 && raw[0] == b'"' && raw[raw.len() - 1] == b'"' {
        parse_usize(&raw[1..raw.len() - 1])
    } else {
        parse_usize(raw)
    }
}

fn non_empty_quote_name(row: &[u8]) -> Result<Option<&[u8]>, (&'static str, &'static str)> {
    match direct(row, b"quoteName") {
        Ok(Some(b"null")) | Ok(None) => Ok(None),
        Ok(Some(v)) if v.len() >= 2 && v[0] == b'"' && v[v.len() - 1] == b'"' => {
            let inner = &v[1..v.len() - 1];
            if inner.is_empty() {
                return Ok(None);
            }
            let mut buf = [0u8; 1024];
            match decode_json_content(inner, &mut buf) {
                Ok(decoded) if trim_ascii(decoded).is_empty() => Ok(None),
                Ok(_) => Ok(Some(inner)),
                Err(_) => Ok(Some(inner)),
            }
        }
        _ => Err(("parse_error", "invalid comment quoteName")),
    }
}

fn render(body: &[u8], item_id: usize, offset: usize) -> Result<usize, (&'static str, &'static str)> {
    match direct(body, b"ok") {
        Ok(Some(b"true")) => {}
        Ok(Some(_)) => return Err(("source_error", "comments endpoint returned ok != true")),
        Ok(None) | Err(_) => return Err(("parse_error", "invalid comments response")),
    }
    match direct(body, b"item") {
        Ok(Some(b"null")) | Ok(None) => {}
        Ok(Some(val)) if parse_item_number(val) == Some(item_id) => {}
        _ => return Err(("parse_error", "comment item mismatch")),
    }

    let entries_raw = match direct(body, b"entries") {
        Ok(Some(v)) => v,
        _ => return Err(("parse_error", "missing comment entries")),
    };
    let mut rows = JsonArrayIter::from_raw_array(entries_raw)
        .ok_or(("parse_error", "invalid comment entries"))?;
    let open_bracket = entries_raw
        .iter()
        .position(|&b| b == b'[')
        .ok_or(("parse_error", "invalid comment entries"))?;
    let mut end = open_bracket + 1;
    let base_ptr = entries_raw.as_ptr() as usize;

    let out = payload_buf();
    let mut c = 0usize;
    if !write_bytes(out, &mut c, br#"{"items":["#) {
        return Err(("result_too_large", "comments response too large"));
    }

    let mut count = 0usize;
    while let Some(row) = rows.next_object() {
        let start = row.as_ptr() as usize - base_ptr;
        let gap = trim_ascii(&entries_raw[end..start]);
        if (count == 0 && !gap.is_empty()) || (count > 0 && gap != b",") {
            return Err(("parse_error", "invalid comment entries"));
        }
        end = start + row.len();

        let id_val = match direct(row, b"id") {
            Ok(Some(v)) => v,
            _ => return Err(("parse_error", "missing comment id")),
        };
        let author_raw = match direct(row, b"name") {
            Ok(Some(b"null")) | Ok(None) => b"".as_slice(),
            Ok(Some(v)) if v.len() >= 2 && v[0] == b'"' && v[v.len() - 1] == b'"' => {
                &v[1..v.len() - 1]
            }
            _ => return Err(("parse_error", "invalid comment author")),
        };
        let text_raw = match raw_string(row, b"text") {
            Ok(Some(v)) => v,
            _ => return Err(("parse_error", "missing comment text")),
        };
        let is_spoiler = match direct(row, b"spoiler") {
            Ok(Some(b"true")) => true,
            Ok(Some(b"false")) | Ok(Some(b"null")) | Ok(None) => false,
            _ => return Err(("parse_error", "invalid comment spoiler flag")),
        };
        let quote_name = non_empty_quote_name(row)?;
        let date_raw = match direct(row, b"date") {
            Ok(Some(b"null")) | Ok(None) => None,
            Ok(Some(v)) if v.len() >= 2 && v[0] == b'"' && v[v.len() - 1] == b'"' => {
                let inner = &v[1..v.len() - 1];
                if inner.is_empty() {
                    None
                } else {
                    Some(inner)
                }
            }
            _ => return Err(("parse_error", "invalid comment date")),
        };

        if count > 0 && !write_bytes(out, &mut c, b",") {
            return Err(("result_too_large", "comments response too large"));
        }
        if !write_bytes(out, &mut c, br#"{"id":""#) {
            return Err(("result_too_large", "comments response too large"));
        }
        if parse_usize(id_val).is_some() {
            if !write_bytes(out, &mut c, id_val) {
                return Err(("result_too_large", "comments response too large"));
            }
        } else if id_val.len() > 2 && id_val[0] == b'"' && id_val[id_val.len() - 1] == b'"' {
            if !append_json_unescaped_then_escaped(out, &mut c, &id_val[1..id_val.len() - 1]) {
                return Err(("result_too_large", "comments response too large"));
            }
        } else {
            return Err(("parse_error", "invalid comment id"));
        }

        if !write_bytes(out, &mut c, br#"","author":""#)
            || !append_json_unescaped_then_escaped(out, &mut c, author_raw)
            || !write_bytes(out, &mut c, br#"","content":""#)
        {
            return Err(("result_too_large", "comments response too large"));
        }

        if is_spoiler {
            let prefix = if quote_name.is_some() || !text_raw.is_empty() {
                "【劇透】 ".as_bytes()
            } else {
                "【劇透】".as_bytes()
            };
            if !write_bytes(out, &mut c, prefix) {
                return Err(("result_too_large", "comments response too large"));
            }
        }
        if let Some(qname) = quote_name {
            if !write_bytes(out, &mut c, "回复 ".as_bytes())
                || !append_json_unescaped_then_escaped(out, &mut c, qname)
                || !write_bytes(out, &mut c, "：".as_bytes())
            {
                return Err(("result_too_large", "comments response too large"));
            }
        }
        if !append_json_unescaped_then_escaped(out, &mut c, text_raw)
            || !write_bytes(out, &mut c, b"\"")
        {
            return Err(("result_too_large", "comments response too large"));
        }

        if let Some(date) = date_raw {
            if !write_bytes(out, &mut c, br#","createdAt":""#)
                || !append_json_unescaped_then_escaped(out, &mut c, date)
                || !write_bytes(out, &mut c, b"\"")
            {
                return Err(("result_too_large", "comments response too large"));
            }
        }
        if !write_bytes(out, &mut c, b"}") {
            return Err(("result_too_large", "comments response too large"));
        }
        count += 1;
    }
    if trim_ascii(&entries_raw[end..]) != b"]" {
        return Err(("parse_error", "invalid comment entries"));
    }
    if !write_bytes(out, &mut c, b"]") {
        return Err(("result_too_large", "comments response too large"));
    }

    let has_more = match direct(body, b"more") {
        Ok(Some(b"true")) => true,
        Ok(Some(b"false")) | Ok(None) => false,
        _ => return Err(("parse_error", "invalid comments pagination")),
    };
    if has_more && count > 0 {
        let next = match direct(body, b"next") {
            Ok(Some(v)) => parse_usize(v).filter(|&next| next > offset),
            Ok(None) => None,
            Err(_) => return Err(("parse_error", "invalid comments next offset")),
        };
        if let Some(next_offset) = next {
            if !(write_bytes(out, &mut c, br#","nextCursor":""#)
                && write_usize(out, &mut c, next_offset)
                && write_bytes(out, &mut c, b"\""))
            {
                return Err(("result_too_large", "comments response too large"));
            }
        }
    }
    if !write_bytes(out, &mut c, b"}") {
        return Err(("result_too_large", "comments response too large"));
    }
    Ok(c)
}

pub(super) fn run(input: &[u8]) -> u32 {
    let query = match comments::request(input) {
        Ok(v) => v,
        Err(code) => return source_error(OPERATION, code, "invalid comments request"),
    };
    if query.chapter_id.is_some() {
        return source_error(
            OPERATION,
            "unsupported_operation",
            "ComicABC comments belong to a work",
        );
    }
    let id_bytes = match query.manga_id.strip_prefix(b"comicabc:manga:") {
        Some(id)
            if !id.is_empty()
                && id.len() <= 12
                && id.iter().all(u8::is_ascii_digit)
                && !id.starts_with(b"0") =>
        {
            id
        }
        _ => return source_error(OPERATION, "invalid_request", "invalid manga ID"),
    };
    let item_id = match parse_usize(id_bytes).filter(|&n| n > 0) {
        Some(n) => n,
        None => return source_error(OPERATION, "invalid_request", "invalid manga ID"),
    };
    let offset = match comments::position(query.cursor) {
        Ok(pos) => pos.unwrap_or(0),
        Err(code) => return source_error(OPERATION, code, "invalid comments cursor"),
    };

    if host::check_cancel() {
        return source_error(OPERATION, "cancelled", "cancelled");
    }

    let mut url = [0u8; 512];
    let mut url_len = 0usize;
    let ok_url = write_bytes(&mut url, &mut url_len, BASE)
        && write_bytes(&mut url, &mut url_len, b"/member/comment/js.aspx?item=")
        && write_bytes(&mut url, &mut url_len, id_bytes)
        && write_bytes(&mut url, &mut url_len, b"&order=new&offset=")
        && write_usize(&mut url, &mut url_len, offset)
        && write_bytes(&mut url, &mut url_len, b"&version=0");
    if !ok_url {
        return source_error(OPERATION, "invalid_request", "comment URL overflow");
    }

    let mut referer = [0u8; 256];
    let mut ref_len = 0usize;
    let ok_ref = write_bytes(&mut referer, &mut ref_len, BASE)
        && write_bytes(&mut referer, &mut ref_len, b"/html/")
        && write_bytes(&mut referer, &mut ref_len, id_bytes)
        && write_bytes(&mut referer, &mut ref_len, b".html");
    if !ok_ref {
        return source_error(OPERATION, "invalid_request", "comment referer overflow");
    }

    let size = match fetch_body_with_referer(&url[..url_len], &referer[..ref_len], false) {
        Ok(n) => n,
        Err(e) => return failure(OPERATION, e),
    };
    if host::check_cancel() {
        return source_error(OPERATION, "cancelled", "cancelled");
    }

    let raw = trim_ascii(unsafe {
        core::slice::from_raw_parts(core::ptr::addr_of!(BODY_BUF).cast::<u8>(), size)
    });
    let body = trim_ascii(raw.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(raw));

    match render(body, item_id, offset) {
        Ok(len) => {
            let result = write_success_payload(OPERATION, len);
            if result == 0 {
                source_error(OPERATION, "result_too_large", "comments response overflow")
            } else {
                result
            }
        }
        Err((code, message)) => source_error(OPERATION, code, message),
    }
}
