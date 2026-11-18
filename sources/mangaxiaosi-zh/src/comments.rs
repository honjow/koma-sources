use super::{
    bytes, checked_request, child, detail_doc, execute, payload_buf, q, request_origin, sa, st, wn,
    work, All, R,
};
use koma_source_sdk::comments;

fn load(request: &[u8]) -> R<usize> {
    let query = comments::request(request)?;
    if query.chapter_id.is_some() || query.cursor.is_some() {
        return Err("invalid request");
    }
    let id = work(request)?;
    let detail = detail_doc(id, request_origin(request))?;
    let discussion = child(detail.0, b".view-comment-main")?;
    let mut owner = [0; 32];
    if sa(discussion.0, b"input#book_id", b"value", &mut owner)? != id {
        return Err("comment owner mismatch");
    }
    // The site prints its list into the work HTML. Its comment script is for posting only.
    let rows = All::new(
        discussion.0,
        b"ul.list > li > ul.postlist > li.dashed > div.info",
    )?;
    let out = payload_buf();
    let mut offset = 0;
    bytes(out, &mut offset, br#"{"items":["#)?;
    for i in 0..rows.len {
        let row = rows.at(i);
        let mut author = [0; 1024];
        let mut content = [0; 32768];
        let mut footer = [0; 512];
        let author = st(row, b"p.title", &mut author)?;
        let content = st(row, b"p.content", &mut content)?;
        let date = st(row, b"p.bottom", &mut footer)?
            .split(u8::is_ascii_whitespace)
            .find(|text| !text.is_empty())
            .filter(|text| text.len() == 10 && text[4] == b'-' && text[7] == b'-')
            .unwrap_or(b"");
        if i > 0 {
            bytes(out, &mut offset, b",")?;
        }
        // The voting control repeats IDs across multiple entries.
        bytes(out, &mut offset, br#"{"id":""#)?;
        wn(out, &mut offset, i + 1)?;
        bytes(out, &mut offset, br#"", "author":"#)?;
        q(out, &mut offset, author)?;
        bytes(out, &mut offset, br#", "content":"#)?;
        q(out, &mut offset, content)?;
        bytes(out, &mut offset, br#", "createdAt":"#)?;
        q(out, &mut offset, date)?;
        bytes(out, &mut offset, b"}")?;
    }
    bytes(out, &mut offset, b"]}")?;
    Ok(offset)
}

#[no_mangle]
pub extern "C" fn koma_source_get_comments(ptr: u32, len: u32) -> u32 {
    let result = checked_request(ptr, len)
        .ok_or("invalid request")
        .and_then(load);
    execute(comments::OPERATION, result)
}
