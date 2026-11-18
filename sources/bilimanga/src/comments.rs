use super::{fetch, manga_path, mid, positive, q, require_request_policy, w, Nodes, R};
use koma_source_sdk::comments;

pub(super) fn load(request: &[u8], out: &mut [u8]) -> R<usize> {
    let query = comments::request(request)?;
    if query.chapter_id.is_some() || query.cursor.is_some() {
        return Err("invalid_request");
    }
    require_request_policy(request)?;
    let mut route = [0; 128];
    let document = fetch(manga_path(mid(request)?, &mut route)?)?;
    let section = document.select(b".trigger-book-comment")?;
    let list = section.select(b"#book-commnet-list-container")?;
    let rows = Nodes::get(&list, b".review-item")?;
    let mut offset = 0;
    let mut count = 0;
    w(out, &mut offset, br#"{"items":["#)?;
    for index in 0..rows.n {
        let row = rows.at(index);
        let body = row.select(b".book-comment-p")?;
        let mut attr = [0; 16];
        if body.attr(b"data-netabare", &mut attr).ok() == Some(b"true") {
            continue;
        }
        let link = row.select(b"a.book-layout")?;
        let mut attr = [0; 256];
        let href = link.attr(b"href", &mut attr)?;
        let id = href
            .strip_prefix(b"/reviewshow_")
            .and_then(|tail| tail.split(|c| *c == b'_').next())
            .ok_or("parse_error")?;
        positive(id).map_err(|_| "parse_error")?;
        let mut author = [0; 512];
        let mut content = [0; 16384];
        let mut date = [0; 64];
        let author = row
            .select(b".book-meta-comment .book-meta-l")?
            .text(&mut author)?;
        let content = body.text(&mut content)?;
        let date = row.select(b".book-comment-time")?.text(&mut date)?;
        if count != 0 {
            w(out, &mut offset, b",")?;
        }
        w(out, &mut offset, br#"{"id":"#)?;
        q(out, &mut offset, id)?;
        w(out, &mut offset, br#", "author":"#)?;
        q(out, &mut offset, author)?;
        w(out, &mut offset, br#", "content":"#)?;
        q(out, &mut offset, content)?;
        w(out, &mut offset, br#", "createdAt":"#)?;
        q(out, &mut offset, date)?;
        w(out, &mut offset, b"}")?;
        count += 1;
    }
    // The full reviews link leads to a login page for guests. Read the reviews
    // that the provider exposes directly on the public work page.
    w(out, &mut offset, b"]}")?;
    Ok(offset)
}
