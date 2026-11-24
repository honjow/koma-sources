use super::{
    body_buf, fetch_json, payload_buf, split_manga_id, write_error, write_success_payload,
    API_BASE,
};
use koma_source_sdk::comments::{self, OPERATION};
use koma_source_sdk::fetch_error_code;
use koma_source_sdk::json_utils::{
    append_json_unescaped_then_escaped, json_object_member, write_bytes, write_url_encoded,
    write_usize, JsonArrayIter,
};
use koma_source_sdk::plain_text::decode_json_content;

const MAX_EMPTY_PAGES: usize = 5;

fn parse_strict_usize(raw: &[u8]) -> Option<usize> {
    if raw.is_empty() || !raw.iter().all(u8::is_ascii_digit) {
        return None;
    }
    let mut value = 0usize;
    for &b in raw {
        value = value
            .checked_mul(10)?
            .checked_add((b - b'0') as usize)?;
    }
    Some(value)
}

fn valid_book_group_id(id: &[u8]) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || matches!(*b, b'-' | b'_'))
}

fn valid_is_serial(is_serial: &[u8]) -> bool {
    matches!(is_serial, b"true" | b"false" | b"True" | b"False")
}

fn json_string_inner(raw: &[u8]) -> Option<&[u8]> {
    if raw.len() >= 2 && raw[0] == b'"' && raw[raw.len() - 1] == b'"' {
        Some(&raw[1..raw.len() - 1])
    } else {
        None
    }
}

fn hex_digit(b: u8) -> Option<u16> {
    match b {
        b'0'..=b'9' => Some((b - b'0') as u16),
        b'a'..=b'f' => Some((b - b'a' + 10) as u16),
        b'A'..=b'F' => Some((b - b'A' + 10) as u16),
        _ => None,
    }
}

fn is_json_text_blank(raw: &[u8]) -> bool {
    let mut i = 0usize;
    while i < raw.len() {
        match raw[i] {
            b' ' | b'\t' | b'\n' | b'\r' => i += 1,
            0xE3 if raw.get(i..i + 3) == Some(&[0xE3, 0x80, 0x80]) => i += 3,
            b'\\' => {
                let Some(&esc) = raw.get(i + 1) else {
                    return false;
                };
                match esc {
                    b'n' | b'r' | b't' => i += 2,
                    b'u' => {
                        let Some(hex) = raw.get(i + 2..i + 6) else {
                            return false;
                        };
                        let mut cp = 0u16;
                        for &h in hex {
                            let Some(d) = hex_digit(h) else {
                                return false;
                            };
                            cp = (cp << 4) | d;
                        }
                        if matches!(cp, 0x0009 | 0x000A | 0x000D | 0x0020 | 0x00A0 | 0x3000) {
                            i += 6;
                        } else {
                            return false;
                        }
                    }
                    _ => return false,
                }
            }
            _ => return false,
        }
    }
    true
}

fn build_comments_url(dst: &mut [u8], book_group_id: &[u8], page: usize) -> Option<usize> {
    let mut c = 0usize;
    (write_bytes(dst, &mut c, API_BASE)
        && write_bytes(dst, &mut c, b"/Comment/List/")
        && write_url_encoded(dst, &mut c, book_group_id)
        && write_bytes(dst, &mut c, b"?sortType=0&page=")
        && write_usize(dst, &mut c, page))
    .then_some(c)
}

fn load(input: &[u8]) -> core::result::Result<usize, (&'static str, &'static str)> {
    let request = match comments::request(input) {
        Ok(value) if value.chapter_id.is_none() => value,
        Ok(_) => {
            return Err((
                "unsupported_operation",
                "tongli comments belong to a work",
            ))
        }
        Err(code) => return Err((code, "invalid comments request")),
    };

    let mut manga_buf = [0u8; 256];
    let manga_id = decode_json_content(request.manga_id, &mut manga_buf)
        .map_err(|_| ("invalid_request", "invalid mangaId"))?;
    let (book_group_id, _) = split_manga_id(manga_id)
        .filter(|(bg, ser)| valid_book_group_id(bg) && valid_is_serial(ser))
        .ok_or(("invalid_request", "bad mangaId"))?;

    let mut page = comments::position(request.cursor)
        .map_err(|code| (code, "invalid comments cursor"))?
        .unwrap_or(1);
    if page == 0 {
        return Err(("invalid_request", "invalid comments cursor"));
    }

    let payload = payload_buf();
    let mut c = 0usize;
    if !write_bytes(payload, &mut c, br#"{"items":["#) {
        return Err(("internal_error", "overflow"));
    }
    let mut written = 0usize;
    let mut has_next_cursor = false;
    let mut next_page = page;

    for _ in 0..MAX_EMPTY_PAGES {
        let mut url_buf = [0u8; 256];
        let url_len = build_comments_url(&mut url_buf, book_group_id, page)
            .ok_or(("internal_error", "url overflow"))?;
        let len = fetch_json(&url_buf[..url_len]).map_err(fetch_error_code)?;
        let json = &body_buf()[..len];

        let current_page = json_object_member(json, b"Page")
            .and_then(parse_strict_usize)
            .ok_or(("parse_error", "missing Page"))?;
        if current_page != page {
            return Err(("parse_error", "comment page mismatch"));
        }
        let total_page = json_object_member(json, b"TotalPage")
            .and_then(parse_strict_usize)
            .ok_or(("parse_error", "missing TotalPage"))?;
        let comments_raw =
            json_object_member(json, b"Comments").ok_or(("parse_error", "missing Comments"))?;
        if comments_raw == b"null" {
            break;
        }
        let mut iter = JsonArrayIter::from_raw_array(comments_raw)
            .ok_or(("parse_error", "invalid Comments"))?;

        let mut saw_any_row = false;
        while let Some(row) = iter.next_object() {
            saw_any_row = true;
            let id_raw = json_object_member(row, b"ID")
                .and_then(json_string_inner)
                .filter(|id| !id.is_empty())
                .ok_or(("parse_error", "missing comment ID"))?;
            let body_raw = match json_object_member(row, b"Body").and_then(json_string_inner) {
                Some(body) if !is_json_text_blank(body) => body,
                _ => continue,
            };
            let vol_raw = json_object_member(row, b"Vol")
                .and_then(json_string_inner)
                .filter(|v| !is_json_text_blank(v));
            let author_raw = json_object_member(row, b"UserNickName")
                .and_then(json_string_inner)
                .unwrap_or(b"");
            let created_raw = json_object_member(row, b"DBCreate")
                .and_then(json_string_inner)
                .filter(|v| !is_json_text_blank(v));

            if written > 0 && !write_bytes(payload, &mut c, b",") {
                return Err(("internal_error", "overflow"));
            }
            let mut ok = write_bytes(payload, &mut c, br#"{"id":""#)
                && append_json_unescaped_then_escaped(payload, &mut c, id_raw)
                && write_bytes(payload, &mut c, br#"","author":""#)
                && append_json_unescaped_then_escaped(payload, &mut c, author_raw)
                && write_bytes(payload, &mut c, br#"","content":""#);
            if ok {
                if let Some(vol) = vol_raw {
                    ok = write_bytes(payload, &mut c, b"[")
                        && append_json_unescaped_then_escaped(payload, &mut c, vol)
                        && write_bytes(payload, &mut c, b"] ");
                }
                ok = ok && append_json_unescaped_then_escaped(payload, &mut c, body_raw);
            }
            if ok {
                if let Some(created) = created_raw {
                    ok = write_bytes(payload, &mut c, br#"","createdAt":""#)
                        && append_json_unescaped_then_escaped(payload, &mut c, created);
                }
            }
            ok = ok && write_bytes(payload, &mut c, br#""}"#);
            if !ok {
                return Err(("internal_error", "overflow"));
            }
            written += 1;
        }

        if saw_any_row && current_page < total_page {
            let candidate_next = current_page
                .checked_add(1)
                .ok_or(("parse_error", "page overflow"))?;
            if written > 0 {
                next_page = candidate_next;
                has_next_cursor = true;
                break;
            }
            page = candidate_next;
        } else {
            break;
        }
    }

    let mut ok = write_bytes(payload, &mut c, b"]");
    if ok && has_next_cursor {
        ok = write_bytes(payload, &mut c, br#","nextCursor":""#)
            && write_usize(payload, &mut c, next_page)
            && write_bytes(payload, &mut c, b"\"");
    }
    ok = ok && write_bytes(payload, &mut c, b"}");
    if !ok {
        return Err(("internal_error", "overflow"));
    }
    Ok(c)
}

pub(super) fn run(input: &[u8]) -> u32 {
    match load(input) {
        Ok(len) => write_success_payload(OPERATION, len),
        Err((code, message)) => write_error(OPERATION, code, message),
    }
}
