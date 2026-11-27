use super::ui::error as write_error;
use super::{body_buf, http_out, http_req_buf, payload_buf, write_success_payload};
use koma_source_sdk::{
    build_get_request, decode_json_body_into, fetch_error_code, host,
    json_utils::{
        append_json_unescaped_then_escaped, write_bytes, write_url_encoded, JsonArrayIter,
    },
    source_protocol::{direct, raw_string},
};

type Result<T> = core::result::Result<T, &'static str>;
const OP: &str = koma_source_sdk::comments::OPERATION;

pub(super) struct Chapter<'a> {
    pub title: &'a [u8],
    pub episode: &'a [u8],
    pub canvas: bool,
}

fn write(out: &mut [u8], offset: &mut usize, value: &[u8]) -> Result<()> {
    write_bytes(out, offset, value)
        .then_some(())
        .ok_or("result_too_large")
}

fn quote(out: &mut [u8], offset: &mut usize, value: &[u8]) -> Result<()> {
    write(out, offset, b"\"")?;
    if !append_json_unescaped_then_escaped(out, offset, value) {
        return Err("result_too_large");
    }
    write(out, offset, b"\"")
}

fn required<'a>(value: &'a [u8], key: &[u8]) -> Result<&'a [u8]> {
    direct(value, key)?.ok_or("parse_error")
}

fn text<'a>(value: &'a [u8], key: &[u8]) -> Result<&'a [u8]> {
    raw_string(value, key)?.ok_or("parse_error")
}

fn render(data: &[u8], page_id: &[u8], cursor: Option<&[u8]>, out: &mut [u8]) -> Result<usize> {
    if text(data, b"status")? != b"success" {
        return Err("parse_error");
    }
    let result = required(data, b"result")?;
    let mut rows =
        JsonArrayIter::from_raw_array(required(result, b"posts")?).ok_or("parse_error")?;
    let mut offset = 0;
    let mut count = 0;
    write(out, &mut offset, br#"{"items":["#)?;
    while let Some(row) = rows.next_object() {
        if text(row, b"pageId")? != page_id {
            return Err("parse_error");
        }
        // The public reader hides removed/blind posts and hides spoiler content until a reveal.
        if text(row, b"status")? != b"SERVICE"
            || raw_string(required(row, b"settings")?, b"spoilerFilter")? == Some(b"ON")
        {
            continue;
        }
        if text(required(row, b"bodyFormat")?, b"type")? != b"PLAIN" {
            continue;
        }
        if count > 0 {
            write(out, &mut offset, b",")?;
        }
        write(out, &mut offset, br#"{"id":"#)?;
        quote(out, &mut offset, text(row, b"id")?)?;
        write(out, &mut offset, br#", "author":"#)?;
        quote(
            out,
            &mut offset,
            text(required(row, b"createdBy")?, b"name")?,
        )?;
        write(out, &mut offset, br#", "content":"#)?;
        quote(out, &mut offset, text(row, b"body")?)?;
        let mut iso = [0; 20];
        if let Some(len) = direct(row, b"createdAt")?
            .and_then(|raw| koma_source_sdk::comments::utc_iso_millis(raw, &mut iso))
        {
            write(out, &mut offset, br#", "createdAt":"#)?;
            quote(out, &mut offset, &iso[..len])?;
        }
        write(out, &mut offset, b"}")?;
        count += 1;
    }
    write(out, &mut offset, b"]")?;
    if let Some(next) = raw_string(required(result, b"pagination")?, b"next")? {
        if count > 0 && !next.is_empty() && Some(next) != cursor {
            write(out, &mut offset, br#", "nextCursor":"#)?;
            quote(out, &mut offset, next)?;
        }
    }
    write(out, &mut offset, b"}")?;
    Ok(offset)
}

fn fetch(chapter: Chapter<'_>, cursor: Option<&[u8]>) -> Result<usize> {
    let mut page_id = [0; 64];
    let mut id_len = 0;
    write(
        &mut page_id,
        &mut id_len,
        if chapter.canvas { b"c_" } else { b"w_" },
    )?;
    write(&mut page_id, &mut id_len, chapter.title)?;
    write(&mut page_id, &mut id_len, b"_")?;
    write(&mut page_id, &mut id_len, chapter.episode)?;
    let page_id = &page_id[..id_len];
    // French reader declares contentLanguage FRENCH; keep this new source isolated.
    let (origin, language): (&[u8], &[u8]) = (b"https://m.webtoons.com", b"FRENCH");
    let mut url = [0; 1024];
    let mut offset = 0;
    write(&mut url, &mut offset, origin)?;
    write(&mut url, &mut offset, b"/p/api/community/v2/posts?pageId=")?;
    write(&mut url, &mut offset, page_id)?;
    write(&mut url, &mut offset, b"&prevSize=0&nextSize=20&withCursor=false&pinRepresentation=none&displayBlindCommentAsService=false")?;
    if let Some(cursor) = cursor {
        write(&mut url, &mut offset, b"&cursor=")?;
        if !write_url_encoded(&mut url, &mut offset, cursor) {
            return Err("invalid_request");
        }
    }
    let req = http_req_buf();
    let len = build_get_request(
        req,
        &url[..offset],
        Some(origin),
        &[
            (b"Accept", b"application/json"),
            (b"Service-Ticket-Id", b"epicom"),
            (b"Service-Type", b"GW"),
            (b"Platform", b"WEB_MOBILE"),
            (b"Language", language),
        ],
    )
    .ok_or("invalid_request")?;
    let response = http_out();
    let len = host::http_request(&req[..len], response).map_err(|_| "network_error")?;
    let body = body_buf();
    let len =
        decode_json_body_into(&response[..len], body).map_err(|error| fetch_error_code(error).0)?;
    render(&body[..len], page_id, cursor, payload_buf())
}

pub(super) fn run(chapter: Result<Chapter<'_>>, cursor: Option<&[u8]>) -> u32 {
    match chapter.and_then(|chapter| fetch(chapter, cursor)) {
        Ok(len) => write_success_payload(OP, len),
        Err(code) => write_error(OP, code, "Could not load WEBTOON episode comments."),
    }
}
