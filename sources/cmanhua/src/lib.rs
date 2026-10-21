#![no_std]
extern crate koma_source_sdk;
mod html;
use html::{Tag, Tags};
use koma_source_sdk::{
    host,
    json_utils::*,
    source::{SourceCapabilities, SourceInfo},
    FetchError,
};
const BASE: &[u8] = b"https://cmanhua.com";
const INFO: SourceInfo = SourceInfo {
    id: "com.cmanhua.koma",
    name: "CManhua",
    version: "0.1.3",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "CManhua public Chinese catalogue and chapters.",
    content_rating: "nsfw",
};
const SOURCE_INFO: SourceInfo = INFO;
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: true,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: true,
    manga_list: true,
    image_request: true,
    home: false,
    filters: false,
    settings: false,
    credentials: false,
};
koma_source_sdk::koma_source_buffers! {payload:384*1024,http_out:1024*1024,body:512*1024,http_req:256*1024,scratch:16*1024,}
koma_source_sdk::koma_source_helpers!();
static mut CHAPTER_IDS: [[u8; 24]; 4096] = [[0; 24]; 4096];
static mut ORIGINAL: [u8; 384 * 1024] = [0; 384 * 1024];
static mut FORM: [u8; 192 * 1024] = [0; 192 * 1024];
static mut CURSOR: [u8; 192 * 1024] = [0; 192 * 1024];
fn body(n: usize) -> &'static [u8] {
    unsafe { core::slice::from_raw_parts(body_buf().as_ptr(), n) }
}
fn fetch(url: &[u8], form: Option<&[u8]>) -> Result<&'static [u8], &'static str> {
    if host::check_cancel() {
        return Err("cancelled");
    }
    let b = http_req_buf();
    let mut n = 0;
    let ok=write_bytes(b,&mut n,br#"{"version":1,"method":""#)&&write_bytes(b,&mut n,if form.is_some(){b"POST"}else{b"GET"})&&write_bytes(b,&mut n,br#"","url":""#)&&append_json_escaped(b,&mut n,url)&&write_bytes(b,&mut n,br#"","headers":{"User-Agent":"Koma/0.1","Referer":"https://cmanhua.com/","Accept-Language":"zh-CN,zh;q=0.9","Content-Type":"application/x-www-form-urlencoded"},"timeoutMs":15000,"responseKind":"bodyText""#);
    if !ok {
        return Err("request overflow");
    }
    if let Some(f) = form {
        if !write_bytes(b, &mut n, br#", "bodyBase64":""#)
            || !append_json_escaped(b, &mut n, f)
            || !write_bytes(b, &mut n, b"\"")
        {
            return Err("form overflow");
        }
    }
    if !write_bytes(b, &mut n, b"}") {
        return Err("request overflow");
    }
    let request = unsafe { core::slice::from_raw_parts(b.as_ptr(), n) };
    let r = host::http_request(request, http_out()).map_err(|_| "network request failed")?;
    let n =
        koma_source_sdk::decode_json_body_into(&http_out()[..r], body_buf()).map_err(
            |e| match e {
                FetchError::ClientError => "public request rejected",
                FetchError::NotFound => "not found",
                _ => "network response failed",
            },
        )?;
    Ok(body(n))
}
fn error(op: &str, e: &str) -> u32 {
    write_error(
        op,
        if e == "cancelled" {
            "cancelled"
        } else if e == "not found" {
            "not_found"
        } else {
            "source_error"
        },
        e,
    )
}
fn url(path: &[u8]) -> Option<&'static [u8]> {
    let b = scratch_a();
    let mut n = 0;
    if !write_bytes(b, &mut n, BASE) || !write_bytes(b, &mut n, path) {
        return None;
    }
    Some(unsafe { core::slice::from_raw_parts(b.as_ptr(), n) })
}
fn quoted(b: &mut [u8], n: &mut usize, s: &[u8]) -> bool {
    write_bytes(b, n, b"\"") && append_json_escaped(b, n, s) && write_bytes(b, n, b"\"")
}
fn decoded(s: &[u8]) -> Option<&'static [u8]> {
    if !s.contains(&b'&') && !s.contains(&b'<') {
        // Borrowed data is consumed before the next HTTP call; no retained state.
        return Some(unsafe { core::slice::from_raw_parts(s.as_ptr(), s.len()) });
    }
    let b = scratch_b();
    let n = html::decoded(s, b)?;
    Some(unsafe { core::slice::from_raw_parts(b.as_ptr(), n) })
}
fn qhtml(b: &mut [u8], n: &mut usize, s: &[u8]) -> bool {
    let s = match decoded(s) {
        Some(s) => s,
        None => return false,
    };
    let out = scratch_a();
    let mut len = 0;
    let mut pending = false;
    for &byte in s {
        if byte.is_ascii_whitespace() {
            pending = len > 0;
            continue;
        }
        if pending {
            if !write_bytes(out, &mut len, b" ") {
                return false;
            }
            pending = false;
        }
        if !write_bytes(out, &mut len, &[byte]) {
            return false;
        }
    }
    quoted(b, n, &out[..len])
}
fn absolute(b: &mut [u8], n: &mut usize, s: &[u8]) -> bool {
    if s.starts_with(b"https://") {
        append_json_escaped(b, n, s)
    } else if s.starts_with(b"/") && !s.starts_with(b"//") {
        write_bytes(b, n, BASE) && append_json_escaped(b, n, s)
    } else {
        false
    }
}
fn qurl(b: &mut [u8], n: &mut usize, s: &[u8]) -> bool {
    let s = match decoded(s) {
        Some(s) => s,
        None => return false,
    };
    write_bytes(b, n, b"\"") && absolute(b, n, s) && write_bytes(b, n, b"\"")
}
fn slug(req: &[u8]) -> Option<&[u8]> {
    let s = extract_json_string(req, b"mangaId")?.strip_prefix(b"cmm:")?;
    if !s.is_empty()
        && s.len() < 256
        && s.iter()
            .all(|b| b.is_ascii_alphanumeric() || *b == b'-' || *b == b'_')
    {
        Some(s)
    } else {
        None
    }
}
fn manga_web(slug: &[u8]) -> Result<&'static [u8], &'static str> {
    let b = scratch_b();
    let mut n = 0;
    write_bytes(b, &mut n, b"/comic/");
    if !write_bytes(b, &mut n, slug) {
        return Err("slug too long");
    }
    let p = unsafe { core::slice::from_raw_parts(b.as_ptr(), n) };
    fetch(url(p).ok_or("URL overflow")?, None)
}
fn member<'a>(j: &'a [u8], k: &[u8]) -> Option<&'a [u8]> {
    let v = json_object_member(j, k)?;
    if v.len() >= 2 && v[0] == b'"' {
        Some(&v[1..v.len() - 1])
    } else {
        None
    }
}
fn card(b: &mut [u8], n: &mut usize, s: &[u8], title: &[u8], cover: &[u8], json: bool) -> bool {
    let ok = write_bytes(b, n, br#"{"id":"cmm:"#)
        && append_json_escaped(b, n, s)
        && write_bytes(b, n, br#"","title":"#);
    let ok = ok
        && if json {
            write_bytes(b, n, b"\"") && write_bytes(b, n, title) && write_bytes(b, n, b"\"")
        } else {
            qhtml(b, n, title)
        };
    ok && write_bytes(b, n, br#", "cover":{"kind":"url","url":"#)
        && qurl(b, n, cover)
        && write_bytes(b, n, br#"},"contentRating":"nsfw"}"#)
}
fn page_end(op: &str, n: &mut usize, next: Option<&[u8]>) -> u32 {
    let b = payload_buf();
    let ok = write_bytes(b, n, br#"],"page":{"hasMore":"#)
        && write_bytes(b, n, if next.is_some() { b"true" } else { b"false" })
        && write_bytes(b, n, br#", "nextCursor":"#)
        && match next {
            Some(s) => quoted(b, n, s),
            None => write_bytes(b, n, b"null"),
        }
        && write_bytes(b, n, b"}}");
    if !ok {
        return write_error(op, "internal_error", "payload overflow");
    }
    write_success_payload(op, *n)
}
fn run_search(req: &[u8]) -> u32 {
    let q = extract_json_string(req, b"query").unwrap_or(b"");
    if q.is_empty() {
        return browse(req, "search");
    }
    if extract_json_string(req, b"cursor").is_some() {
        let mut n = 0;
        write_bytes(payload_buf(), &mut n, br#"{"items":["#);
        return page_end("search", &mut n, None);
    }
    let b = scratch_b();
    let mut n = 0;
    write_bytes(b, &mut n, b"/Modules/Search/SearchHandler.ashx?q=");
    if !write_url_encoded(b, &mut n, q) {
        return write_error("search", "invalid_request", "query too long");
    }
    let p = unsafe { core::slice::from_raw_parts(b.as_ptr(), n) };
    let j = match fetch(url(p).unwrap(), None) {
        Ok(j) => j,
        Err(e) => return error("search", e),
    };
    let mut it = match JsonArrayIter::from_raw_array(j) {
        Some(it) => it,
        None => return write_error("search", "parse_error", "invalid search data"),
    };
    let b = payload_buf();
    let mut n = 0;
    write_bytes(b, &mut n, br#"{"items":["#);
    let mut count = 0;
    while let Some(v) = it.next_object() {
        if count > 0 && !write_bytes(b, &mut n, b",") {
            return error("search", "payload overflow");
        }
        let (s, t, c) = match (member(v, b"slug"), member(v, b"title"), member(v, b"cover")) {
            (Some(s), Some(t), Some(c)) => (s, t, c),
            _ => return error("search", "invalid search entry"),
        };
        if !card(b, &mut n, s, t, c, true) {
            return error("search", "payload overflow");
        }
        count += 1;
    }
    page_end("search", &mut n, None)
}
fn target(a: Tag) -> Option<&'static [u8]> {
    let h = decoded(a.attr(b"href")?)?;
    let x = h.strip_prefix(b"javascript:__doPostBack('")?;
    let end = x.iter().position(|x| *x == b'\'')?;
    let t = &x[..end];
    if t.len() < 128 && t.iter().all(|x| x.is_ascii_alphanumeric() || *x == b'$') {
        Some(t)
    } else {
        None
    }
}
fn form_pair(b: &mut [u8], n: &mut usize, k: &[u8], v: &[u8]) -> bool {
    if *n > 0 && !write_bytes(b, n, b"&") {
        return false;
    }
    write_url_encoded(b, n, k) && write_bytes(b, n, b"=") && write_url_encoded(b, n, v)
}
fn make_form(doc: &[u8], event: &[u8]) -> Result<&'static [u8], &'static str> {
    let form = html::id(doc, b"ctl01")
        .and_then(Tag::inner)
        .ok_or("missing public form")?;
    let b = unsafe { &mut *core::ptr::addr_of_mut!(FORM) };
    let mut n = 0;
    if !form_pair(b, &mut n, b"__EVENTTARGET", event) {
        return Err("form overflow");
    }
    let mut hidden = 0;
    for t in Tags::new(form) {
        if t.name == b"input" {
            let k = match t.attr(b"name") {
                Some(k) => k,
                None => continue,
            };
            let typ = t.attr(b"type").unwrap_or(b"text");
            if k == b"__EVENTTARGET"
                || matches!(typ, b"submit" | b"button" | b"image" | b"file")
                || matches!(typ, b"checkbox" | b"radio") && t.attr(b"checked").is_none()
            {
                continue;
            }
            if k.starts_with(b"__") {
                hidden += 1;
            }
            let v = decoded(t.attr(b"value").unwrap_or(b"")).ok_or("invalid form value")?;
            if !form_pair(b, &mut n, k, v) {
                return Err("form overflow");
            }
        } else if t.name == b"select" {
            let k = t.attr(b"name").ok_or("missing select name")?;
            let inner = t.inner().ok_or("invalid select")?;
            let opt = Tags::new(inner)
                .find(|t| t.name == b"option" && t.attr(b"selected").is_some())
                .or_else(|| Tags::new(inner).find(|t| t.name == b"option"))
                .ok_or("empty select")?;
            let v = decoded(opt.attr(b"value").unwrap_or(b"")).ok_or("invalid option")?;
            if !form_pair(b, &mut n, k, v) {
                return Err("form overflow");
            }
        }
    }
    if hidden < 2 {
        return Err("missing public form state");
    }
    Ok(unsafe { core::slice::from_raw_parts(b.as_ptr(), n) })
}
fn next_cursor(doc: &[u8], sort: &[u8]) -> Result<Option<&'static [u8]>, &'static str> {
    let next = Tags::new(doc).find(|a| {
        a.name == b"a"
            && a.attr(b"id")
                .map(|x| x.starts_with(b"MainContent_rptPager_lnkPage"))
                .unwrap_or(false)
            && a.inner().map(|s| s == b">").unwrap_or(false)
    });
    let t = match next {
        Some(a) => target(a).ok_or("unknown paging target")?,
        None => return Ok(None),
    };
    let f = make_form(doc, t)?;
    let b = unsafe { &mut *core::ptr::addr_of_mut!(CURSOR) };
    let mut n = 0;
    if !write_bytes(b, &mut n, b"v1:")
        || !write_bytes(b, &mut n, sort)
        || !write_bytes(b, &mut n, b":")
        || !write_bytes(b, &mut n, f)
    {
        return Err("cursor overflow");
    }
    Ok(Some(unsafe { core::slice::from_raw_parts(b.as_ptr(), n) }))
}
fn browse(req: &[u8], op: &str) -> u32 {
    let sort = if extract_json_string(req, b"listingId") == Some(b"latest") {
        b"updated_desc".as_slice()
    } else {
        b"views_desc".as_slice()
    };
    let b = scratch_b();
    let mut n = 0;
    write_bytes(b, &mut n, b"/Browse?orderBy=");
    write_bytes(b, &mut n, sort);
    let p = unsafe { core::slice::from_raw_parts(b.as_ptr(), n) };
    let url = url(p).unwrap();
    let cursor = extract_json_string(req, b"cursor");
    let form = if let Some(c) = cursor {
        let c = match c
            .strip_prefix(b"v1:")
            .and_then(|s| s.strip_prefix(sort))
            .and_then(|s| s.strip_prefix(b":"))
        {
            Some(c) => c,
            _ => return write_error(op, "invalid_request", "cursor does not match listing"),
        };
        if c.len() > 192 * 1024
            || !c.starts_with(b"__EVENTTARGET=ctl00%24MainContent%24rptPager%24ctl")
            || !contains_bytes(c, b"%24lnkPage&")
        {
            return write_error(op, "invalid_request", "invalid public paging cursor");
        }
        Some(c)
    } else {
        None
    };
    let doc = match fetch(url, form) {
        Ok(d) => d,
        Err(e) => return error(op, e),
    };
    let sec = match html::id(doc, b"browseSection").and_then(Tag::inner) {
        Some(s) => s,
        None => return error(op, "missing browse section"),
    };
    let b = payload_buf();
    let mut n = 0;
    write_bytes(b, &mut n, br#"{"items":["#);
    let mut count = 0;
    for a in Tags::new(sec) {
        if a.name != b"a" {
            continue;
        }
        let s = match a.attr(b"href").and_then(|s| s.strip_prefix(b"/comic/")) {
            Some(s) => s,
            None => continue,
        };
        let inner = match a.inner() {
            Some(v) => v,
            None => return error(op, "invalid card"),
        };
        let title = match Tags::new(inner)
            .find(|t| t.class(b"comic-title"))
            .and_then(Tag::inner)
        {
            Some(t) => t,
            None => return error(op, "missing card title"),
        };
        let cover = Tags::new(inner)
            .find(|t| t.name == b"img")
            .and_then(|t| t.attr(b"src"))
            .unwrap_or(b"");
        if count > 0 && !write_bytes(b, &mut n, b",") {
            return error(op, "payload overflow");
        }
        if !card(b, &mut n, s, title, cover, false) {
            return error(op, "invalid card or payload overflow");
        }
        count += 1;
    }
    if count == 0 {
        return error(op, "empty or rejected listing");
    }
    let c = match next_cursor(doc, sort) {
        Ok(c) => c,
        Err(e) => return error(op, e),
    };
    page_end(op, &mut n, c)
}
fn run_get_listings(_: &[u8]) -> u32 {
    let d=r#"{"listings":[{"id":"popular","kind":"popular","name":"热门"},{"id":"latest","kind":"latest","name":"最近更新"}]}"#.as_bytes();
    payload_buf()[..d.len()].copy_from_slice(d);
    write_success_payload("get_listings", d.len())
}
fn run_get_manga_list(r: &[u8]) -> u32 {
    browse(r, "get_manga_list")
}
fn run_get_manga(req: &[u8]) -> u32 {
    let s = match slug(req) {
        Some(s) => s,
        None => return write_error("get_manga", "invalid_request", "bad mangaId"),
    };
    let d = match manga_web(s) {
        Ok(d) => d,
        Err(e) => return error("get_manga", e),
    };
    let title = match html::id(d, b"MainContent_lblTitle").and_then(Tag::inner) {
        Some(v) => v,
        None => return error("get_manga", "missing public title"),
    };
    let b = payload_buf();
    let mut n = 0;
    let ok = write_bytes(b, &mut n, br#"{"manga":{"id":"cmm:"#)
        && write_bytes(b, &mut n, s)
        && write_bytes(b, &mut n, br#"","title":"#)
        && qhtml(b, &mut n, title)
        && write_bytes(b, &mut n, br#", "cover":{"kind":"url","url":"#)
        && qurl(
            b,
            &mut n,
            html::id(d, b"MainContent_imgCover")
                .and_then(|t| t.attr(b"src"))
                .unwrap_or(b""),
        )
        && write_bytes(b, &mut n, br#"},"authors":["#)
        && qhtml(
            b,
            &mut n,
            html::id(d, b"MainContent_lblAuthor")
                .and_then(Tag::inner)
                .unwrap_or(b""),
        )
        && write_bytes(b, &mut n, br#"],"description":"#)
        && qhtml(
            b,
            &mut n,
            html::id(d, b"MainContent_lblDescription")
                .and_then(Tag::inner)
                .unwrap_or(b""),
        )
        && write_bytes(
            b,
            &mut n,
            br#", "status":"unknown","contentRating":"nsfw","links":[{"kind":"source","url":"#,
        )
        && url_for_manga(s).map(|u| qurl(b, &mut n, u)).unwrap_or(false)
        && write_bytes(b, &mut n, br#"}]}}"#);
    if !ok {
        return error("get_manga", "payload overflow");
    }
    write_success_payload("get_manga", n)
}
fn chapter_id(h: &[u8]) -> Option<&[u8]> {
    let id = h.strip_prefix(b"/ReadComic?id=")?;
    if id.len() == 24 && id.iter().all(u8::is_ascii_hexdigit) {
        Some(id)
    } else {
        None
    }
}
fn add_chapters(
    doc: &[u8],
    s: &[u8],
    n: &mut usize,
    count: &mut usize,
) -> Result<(), &'static str> {
    let b = payload_buf();
    for a in Tags::new(doc) {
        if !a.class(b"cd-chapter-link") {
            continue;
        }
        let cid = chapter_id(a.attr(b"href").ok_or("missing chapter link")?)
            .ok_or("unknown chapter identity")?;
        let ids = unsafe { &mut *core::ptr::addr_of_mut!(CHAPTER_IDS) };
        if *count >= ids.len() || ids[..*count].iter().any(|id| id.as_slice() == cid) {
            return Err("duplicate chapter or catalogue limit");
        }
        ids[*count].copy_from_slice(cid);
        let title = a.inner().ok_or("missing chapter title")?;
        if *count > 0 && !write_bytes(b, n, b",") {
            return Err("catalogue overflow");
        }
        let ok = write_bytes(b, n, br#"{"id":"cmc:"#)
            && write_bytes(b, n, s)
            && write_bytes(b, n, b":")
            && write_bytes(b, n, cid)
            && write_bytes(b, n, br#"","mangaId":"cmm:"#)
            && write_bytes(b, n, s)
            && write_bytes(b, n, br#"","title":"#)
            && qhtml(b, n, title)
            && write_bytes(b, n, br#", "sequenceIndex":"#)
            && write_usize(b, n, *count)
            && write_bytes(b, n, br#", "language":"zh","locked":false}"#);
        if !ok {
            return Err("catalogue overflow");
        }
        *count += 1;
    }
    Ok(())
}
fn run_get_chapters(req: &[u8]) -> u32 {
    let s = match slug(req) {
        Some(s) => s,
        None => return write_error("get_chapters", "invalid_request", "bad mangaId"),
    };
    let d = match manga_web(s) {
        Ok(d) => d,
        Err(e) => return error("get_chapters", e),
    };
    if d.len() > 384 * 1024 {
        return error("get_chapters", "catalogue HTML overflow");
    }
    let orig = unsafe { &mut *core::ptr::addr_of_mut!(ORIGINAL) };
    orig[..d.len()].copy_from_slice(d);
    let orig = unsafe { core::slice::from_raw_parts(orig.as_ptr(), d.len()) };
    let b = payload_buf();
    let mut n = 0;
    write_bytes(b, &mut n, br#"{"items":["#);
    let mut count = 0;
    let mut range_end = 0usize;
    let mut ranges = 0;
    for a in Tags::new(orig) {
        let href = a.attr(b"href").unwrap_or(b"");
        if a.name != b"a" || !contains_bytes(href, b"rptChapterRanges") {
            continue;
        }
        let label = match a.inner().and_then(decoded) {
            Some(v) => v,
            None => return error("get_chapters", "missing range label"),
        };
        let mut bounds = label.split(|b| *b == b'-');
        let start = bounds
            .next()
            .and_then(|s| core::str::from_utf8(s).ok())
            .and_then(|s| s.trim().parse::<usize>().ok());
        let end = bounds
            .next()
            .and_then(|s| core::str::from_utf8(s).ok())
            .and_then(|s| s.trim().parse::<usize>().ok());
        let (start, end) = match (start, end) {
            (Some(start), Some(end))
                if start == range_end + 1 && end >= start && bounds.next().is_none() =>
            {
                (start, end)
            }
            _ => return error("get_chapters", "invalid chapter range"),
        };
        let before = count;
        ranges += 1;
        if ranges > 128 {
            return error("get_chapters", "too many chapter ranges");
        }
        let range = if a.class(b"btn-primary") {
            orig
        } else {
            let t = match target(a) {
                Some(t) => t,
                None => return error("get_chapters", "unknown catalogue target"),
            };
            let f = match make_form(orig, t) {
                Ok(f) => f,
                Err(e) => return error("get_chapters", e),
            };
            let u = match url_for_manga(s) {
                Some(u) => u,
                None => return error("get_chapters", "URL overflow"),
            };
            match fetch(u, Some(f)) {
                Ok(d) => d,
                Err(e) => return error("get_chapters", e),
            }
        };
        if let Err(e) = add_chapters(range, s, &mut n, &mut count) {
            return error("get_chapters", e);
        }
        if count - before != end - start + 1 {
            return error("get_chapters", "incomplete chapter range");
        }
        range_end = end;
    }
    if ranges == 0 {
        if let Err(e) = add_chapters(orig, s, &mut n, &mut count) {
            return error("get_chapters", e);
        }
    }
    if count == 0 {
        return error("get_chapters", "missing public catalogue");
    }
    page_end("get_chapters", &mut n, None)
}
fn url_for_manga(s: &[u8]) -> Option<&'static [u8]> {
    let b = scratch_b();
    let mut n = 0;
    write_bytes(b, &mut n, b"/comic/");
    write_bytes(b, &mut n, s);
    let p = unsafe { core::slice::from_raw_parts(b.as_ptr(), n) };
    url(p)
}
fn split<'a>(s: &'a [u8], prefix: &[u8], page: bool) -> Option<(&'a [u8], &'a [u8], usize)> {
    let mut it = s.strip_prefix(prefix)?.split(|x| *x == b':');
    let slug = it.next()?;
    let cid = it.next()?;
    if slug.is_empty()
        || !slug
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || *b == b'-' || *b == b'_')
        || cid.len() != 24
        || !cid.iter().all(u8::is_ascii_hexdigit)
    {
        return None;
    }
    let i = if page {
        core::str::from_utf8(it.next()?).ok()?.parse().ok()?
    } else {
        0
    };
    if it.next().is_some() {
        return None;
    }
    Some((slug, cid, i))
}
fn chapter_web(cid: &[u8]) -> Result<&'static [u8], &'static str> {
    let b = scratch_b();
    let mut n = 0;
    write_bytes(b, &mut n, b"/ReadComic?id=");
    write_bytes(b, &mut n, cid);
    let p = unsafe { core::slice::from_raw_parts(b.as_ptr(), n) };
    fetch(url(p).ok_or("URL overflow")?, None)
}
fn run_get_pages(r: &[u8]) -> u32 {
    let (s, cid, _) =
        match extract_json_string(r, b"chapterId").and_then(|s| split(s, b"cmc:", false)) {
            Some(x) => x,
            None => return write_error("get_pages", "invalid_request", "bad chapterId"),
        };
    let d = match chapter_web(cid) {
        Ok(d) => d,
        Err(e) => return error("get_pages", e),
    };
    let b = payload_buf();
    let mut n = 0;
    write_bytes(b, &mut n, br#"{"pages":["#);
    let mut count = 0;
    for img in Tags::new(d) {
        if img.name != b"img" || !img.class(b"chapter-image") {
            continue;
        }
        let u = img
            .attr(b"data-src")
            .filter(|s| !s.is_empty())
            .or_else(|| img.attr(b"src"))
            .unwrap_or(b"");
        if count > 0 && !write_bytes(b, &mut n, b",") {
            return error("get_pages", "payload overflow");
        }
        let ok = write_bytes(b, &mut n, br#"{"id":"cmp:"#)
            && write_bytes(b, &mut n, s)
            && write_bytes(b, &mut n, b":")
            && write_bytes(b, &mut n, cid)
            && write_bytes(b, &mut n, b":")
            && write_usize(b, &mut n, count)
            && write_bytes(b, &mut n, br#"","index":"#)
            && write_usize(b, &mut n, count)
            && write_bytes(b, &mut n, br#", "image":{"kind":"url","url":"#)
            && qurl(b, &mut n, u)
            && write_bytes(b, &mut n, b"}}");
        if !ok {
            return error("get_pages", "invalid image or payload overflow");
        }
        count += 1;
    }
    if count == 0 {
        return write_error("get_pages", "permission_denied", "no public chapter images");
    }
    if !write_bytes(b, &mut n, b"]}") {
        return error("get_pages", "payload overflow");
    }
    write_success_payload("get_pages", n)
}
fn run_get_image_request(r: &[u8]) -> u32 {
    let supplied = match extract_json_string(r, b"url") {
        Some(u) => u,
        None => return write_error("get_image_request", "invalid_request", "missing URL"),
    };
    let page = extract_json_string(r, b"pageId").unwrap_or(b"");
    let url = if page.starts_with(b"cmp:") {
        let (_, cid, i) = match split(page, b"cmp:", true) {
            Some(x) => x,
            None => return write_error("get_image_request", "invalid_request", "bad pageId"),
        };
        let d = match chapter_web(cid) {
            Ok(d) => d,
            Err(e) => return error("get_image_request", e),
        };
        match Tags::new(d)
            .filter(|t| t.name == b"img" && t.class(b"chapter-image"))
            .nth(i)
            .and_then(|t| {
                t.attr(b"data-src")
                    .filter(|s| !s.is_empty())
                    .or_else(|| t.attr(b"src"))
            }) {
            Some(u) => u,
            None => return write_error("get_image_request", "not_found", "public page not found"),
        }
    } else {
        supplied
    };
    let b = payload_buf();
    let mut n = 0;
    let ok=write_bytes(b,&mut n,br#"{"url":"#)&&qurl(b,&mut n,url)&&write_bytes(b,&mut n,br#", "headers":{"User-Agent":"Koma/0.1","Referer":"https://cmanhua.com/","Accept-Language":"zh-CN,zh;q=0.9"}}"#);
    if !ok {
        return error("get_image_request", "invalid image URL");
    }
    write_success_payload("get_image_request", n)
}
fn run_get_home(_: &[u8]) -> u32 {
    write_error("get_home", "not_implemented", "not supported")
}
fn run_get_filters(_: &[u8]) -> u32 {
    write_error("get_filters", "not_implemented", "not supported")
}
fn run_get_settings(_: &[u8]) -> u32 {
    write_error("get_settings", "not_implemented", "not supported")
}
koma_source_sdk::koma_source_exports!("cmanhua");
