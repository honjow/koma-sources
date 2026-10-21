#![no_std]
use koma_source_sdk::{
    host,
    json_utils::*,
    source::{SourceCapabilities, SourceInfo},
};
type E = &'static str;
const MAX: usize = 1048576;
const INFO: SourceInfo = SourceInfo {
    id: "com.hentaihand.zh.koma",
    name: "HentaiHand",
    version: "0.1.1",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "Anonymous Chinese metadata; NSFW source; per-work rating and usage unknown.",
    content_rating: "nsfw",
};
const CAPS: SourceCapabilities = SourceCapabilities {
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
koma_source_sdk::koma_source_buffers! {payload:1048000,http_out:MAX,body:MAX,http_req:32768,scratch:8192,}
koma_source_sdk::koma_source_helpers!();
include!("parser.rs");
static mut REQUEST: [u8; MAX] = [0; MAX];
static mut DETAIL: [u8; MAX] = [0; MAX];
static mut IDS: [usize; 4096] = [0; 4096];
static mut URL_POS: [(usize, usize); 4096] = [(0, 0); 4096];
fn fetch(url: &[u8]) -> Result<usize, E> {
    if host::check_cancel() {
        return Err("cancelled");
    }
    if !url.starts_with(b"https://hentaihand.com/api/comics")
        && ![b"tags" as &[u8], b"artists", b"characters"]
            .iter()
            .any(|uri| {
                let mut prefix = [0; 128];
                let mut n = 0;
                bytes(&mut prefix, &mut n, b"https://hentaihand.com/api/").is_ok()
                    && bytes(&mut prefix, &mut n, uri).is_ok()
                    && bytes(&mut prefix, &mut n, b"?q=").is_ok()
                    && url.starts_with(&prefix[..n])
            })
    {
        return Err("API URL owner");
    }
    let req = http_req_buf();
    let mut n = 0;
    bytes(req, &mut n, b"{\"version\":1,\"method\":\"GET\",\"url\":")?;
    q(req, &mut n, url)?;
    bytes(req,&mut n,b",\"headers\":{\"User-Agent\":\"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/121.0\",\"Referer\":\"https://hentaihand.com/\",\"Origin\":\"https://hentaihand.com\",\"Cache-Control\":\"max-age=600\"}}")?;
    let len = host::http_request(&http_req_buf()[..n], http_out()).map_err(|_| "HTTP transport")?;
    if len == 0 || len > MAX {
        return Err("HTTP capacity");
    }
    valid_json(&http_out()[..len])?;
    let status = field_num(&http_out()[..len], b"status")?;
    if !(200..300).contains(&status) {
        return Err("HTTP status");
    }
    let size = koma_source_sdk::decode_json_body_into(&http_out()[..len], body_buf())
        .map_err(|_| "HTTP body")?;
    if size == 0 || size > MAX {
        return Err("body capacity");
    }
    valid_json(&body_buf()[..size])?;
    let size = normalize_json_solidus(body_buf(), size);
    valid_json(&body_buf()[..size])?;
    Ok(size)
}
// JSON-equivalent solidus unescaping only. Paired escaped backslashes remain paired,
// so literal backslashes cannot be turned into host/path delimiters. Original
// retained bodies are immutable; the whole original JSON is validated first.
fn normalize_json_solidus(b: &mut [u8], len: usize) -> usize {
    let mut read = 0;
    let mut write = 0;
    while read < len {
        if b[read] == b'\\' && read + 1 < len {
            if b[read + 1] == b'/' {
                b[write] = b'/';
                write += 1;
            } else {
                b[write] = b[read];
                b[write + 1] = b[read + 1];
                write += 2;
            }
            read += 2;
        } else {
            b[write] = b[read];
            read += 1;
            write += 1;
        }
    }
    write
}
fn body(n: usize) -> &'static [u8] {
    unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(BODY_BUF) as *const u8, n) }
}
fn dt(n: usize) -> &'static [u8] {
    unsafe { core::slice::from_raw_parts(core::ptr::addr_of!(DETAIL) as *const u8, n) }
}
fn numeric(s: &[u8]) -> Result<usize, E> {
    let n = number(s).ok_or("numeric ID")?;
    if n == 0 || s.len() > 1 && s[0] == b'0' {
        return Err("numeric ID");
    }
    Ok(n)
}
fn id<'a>(s: &'a [u8], prefix: &[u8]) -> Result<&'a [u8], E> {
    let x = s.strip_prefix(prefix).ok_or("identity prefix")?;
    numeric(x)?;
    Ok(x)
}
fn slug(s: &[u8]) -> Result<(), E> {
    if s.is_empty()
        || s.len() > 2048
        || !s
            .iter()
            .all(|x| x.is_ascii_alphanumeric() || *x == b'-' || *x == b'_')
    {
        return Err("slug shape");
    }
    Ok(())
}
fn route<'a>(u: &'a [u8]) -> Result<&'a [u8], E> {
    let s = u
        .strip_prefix(b"https://hentaihand.com/en/comic/")
        .ok_or("canonical route")?;
    slug(s)?;
    Ok(s)
}
fn read_text<'a>(v: &[u8], key: &[u8], out: &'a mut [u8]) -> Result<&'a [u8], E> {
    unescape(text(v, key)?, out)
}
fn owned_detail(work: &[u8], u: &[u8]) -> Result<usize, E> {
    let owner = numeric(work)?;
    let sl = route(u)?;
    let mut api = [0; 4096];
    let mut n = 0;
    bytes(&mut api, &mut n, b"https://hentaihand.com/api/comics/")?;
    bytes(&mut api, &mut n, sl)?;
    let len = fetch(&api[..n])?;
    let v = body(len);
    validate_work(v, Some(owner))?;
    if text(v, b"slug")? != sl {
        return Err("detail slug owner");
    }
    unsafe { DETAIL[..len].copy_from_slice(v) };
    Ok(len)
}
fn validate_work(v: &[u8], owner: Option<usize>) -> Result<usize, E> {
    let n = field_num(v, b"id")?;
    if n == 0 || owner.is_some_and(|x| x != n) {
        return Err("work owner");
    }
    slug(text(v, b"slug")?)?;
    quoted(required(v, b"title")?)?;
    let lang = field_num(required(v, b"language")?, b"id")?;
    if lang != 1 && lang != 50 {
        return Err("Chinese language owner");
    }
    let premium = required(v, b"premium")?;
    if premium != b"false" && premium != b"true" {
        return Err("premium boolean");
    }
    let count = field_num(v, b"pages")?;
    if count == 0 || count > 4096 {
        return Err("page count capacity");
    }
    Ok(n)
}
fn readable(v: &[u8]) -> Result<(), E> {
    if required(v, b"premium")? != b"false" {
        return Err("permission required");
    }
    if field_num(v, b"chapters_count")? != 0 {
        return Err("unsupported multi chapter");
    }
    Ok(())
}
fn cover_allowed(u: &[u8], work: usize) -> Result<(), E> {
    let path = u
        .strip_prefix(b"https://cdn.hentaihand.com/nhentai/storage/comics/")
        .ok_or("cover host")?;
    let mut expected = [0; 32];
    let mut n = 0;
    wn(&mut expected, &mut n, work)?;
    if !path.starts_with(&expected[..n]) || !matches!(&path[n..], b".jpg" | b".png") {
        return Err("cover owner");
    }
    Ok(())
}
fn emit_optional(v: &[u8], key: &[u8], b: &mut [u8], n: &mut usize) -> Result<(), E> {
    if let Some(x) = member(v, key)? {
        if x != b"null" {
            quoted(x)?;
            bytes(b, n, b",")?;
            q(b, n, key)?;
            bytes(b, n, b":")?;
            bytes(b, n, x)?;
        }
    }
    Ok(())
}
fn emit_names(v: &[u8], key: &[u8], b: &mut [u8], n: &mut usize) -> Result<(), E> {
    if let Some(x) = member(v, key)? {
        if x != b"null" {
            let mut it = Array::new(x)?;
            bytes(b, n, b",")?;
            q(b, n, key)?;
            bytes(b, n, b":[")?;
            let mut count = 0;
            while let Some(x) = it.next()? {
                if count > 0 {
                    bytes(b, n, b",")?;
                }
                bytes(b, n, required(x, b"name")?)?;
                quoted(required(x, b"name")?)?;
                count += 1;
                if count > 4096 {
                    return Err("names capacity");
                }
            }
            bytes(b, n, b"]")?;
        }
    }
    Ok(())
}
fn emit_work(v: &[u8], b: &mut [u8], n: &mut usize) -> Result<(), E> {
    let work = validate_work(v, None)?;
    let sl = text(v, b"slug")?;
    let cover = text(v, b"image_url")?;
    cover_allowed(cover, work)?;
    bytes(b, n, b"{\"id\":\"hh:zh:")?;
    wn(b, n, work)?;
    bytes(b, n, b"\",\"title\":")?;
    bytes(b, n, required(v, b"title")?)?;
    bytes(b, n, b",\"url\":\"https://hentaihand.com/en/comic/")?;
    bytes(b, n, sl)?;
    bytes(b, n, b"\",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(b, n, cover)?;
    bytes(b, n, b"},\"contentRating\":\"unknown\",\"status\":")?;
    let status = match member(v, b"status")? {
        Some(b"\"complete\"") => b"completed" as &[u8],
        Some(b"\"ongoing\"") => b"ongoing",
        Some(b"\"onhold\"") => b"hiatus",
        Some(b"\"canceled\"") => b"cancelled",
        _ => b"unknown",
    };
    q(b, n, status)?;
    emit_optional(v, b"description", b, n)?;
    emit_names(v, b"artists", b, n)?;
    emit_names(v, b"authors", b, n)?;
    emit_names(v, b"tags", b, n)?;
    bytes(b, n, b"}")
}
fn listing(r: &[u8]) -> Result<usize, E> {
    let a = args(r)?;
    if text(a, b"listingId")? != b"listing:popular" {
        return Err("listing identity");
    }
    let page = if let Some(p) = member(a, b"page")? {
        match member(p, b"cursor")? {
            Some(b"null") | None => 1,
            Some(x) => numeric(quoted(x)?)?,
        }
    } else {
        1
    };
    if page > 10000 {
        return Err("cursor capacity");
    }
    let mut url = [0; 4096];
    let mut un = 0;
    bytes(
        &mut url,
        &mut un,
        b"https://hentaihand.com/api/comics?page=",
    )?;
    wn(&mut url, &mut un, page)?;
    bytes(
        &mut url,
        &mut un,
        b"&sort=popularity&order=desc&duration=all&languages%5B-1%5D=1&languages%5B-2%5D=50",
    )?;
    let len = fetch(&url[..un])?;
    emit_listing(len, page, None)
}
fn emit_listing(len: usize, page: usize, query: Option<&[u8]>) -> Result<usize, E> {
    let v = body(len);
    if field_num(v, b"current_page")? != page {
        return Err("page owner");
    }
    let per = field_num(v, b"per_page")?;
    let total = field_num(v, b"total")?;
    let last = field_num(v, b"last_page")?;
    if per == 0 || per > 4096 || last == 0 || page > last {
        return Err("pagination capacity");
    }
    let next = required(v, b"next_page_url")?;
    let has = next != b"null";
    if has {
        let x = quoted(next)?;
        let mut expected = [0; 256];
        let mut en = 0;
        bytes(
            &mut expected,
            &mut en,
            b"http://hentaihand.com/api/comics?page=",
        )?;
        wn(
            &mut expected,
            &mut en,
            page.checked_add(1).ok_or("page overflow")?,
        )?;
        if x != &expected[..en] || page >= last {
            return Err("next page owner");
        }
    } else if page != last {
        return Err("missing next page");
    }
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"items\":[")?;
    let mut count = 0;
    let mut it = Array::new(required(v, b"data")?)?;
    while let Some(x) = it.next()? {
        if count >= per {
            return Err("listing count");
        };
        let owner = validate_work(x, None)?;
        unsafe {
            if IDS[..count].contains(&owner) {
                return Err("duplicate work");
            };
            IDS[count] = owner
        }
        if count > 0 {
            bytes(b, &mut n, b",")?
        }
        emit_work(x, b, &mut n)?;
        count += 1;
    }
    let from = (page - 1).checked_mul(per).ok_or("offset overflow")?;
    let end = from.checked_add(count).ok_or("offset overflow")?;
    if count == 0 && query.is_some() && total == 0 && page == 1 && last == 1 && !has {
        if required(v, b"from")? != b"null" || required(v, b"to")? != b"null" {
            return Err("empty search pagination");
        }
    } else if count == 0
        || end > total
        || count != core::cmp::min(per, total.checked_sub(from).ok_or("page total")?)
        || field_num(v, b"from")? != from + 1
        || field_num(v, b"to")? != end
    {
        return Err("listing completeness");
    }
    bytes(b, &mut n, b"],\"page\":{\"nextCursor\":")?;
    if has {
        bytes(b, &mut n, b"\"")?;
        if let Some(query) = query {
            bytes(b, &mut n, b"hhq:")?;
            hex_query(b, &mut n, query)?;
            bytes(b, &mut n, b":")?;
        }
        wn(b, &mut n, page.checked_add(1).ok_or("page overflow")?)?;
        bytes(b, &mut n, b"\",\"hasMore\":true}}")?;
    } else {
        bytes(b, &mut n, b"null,\"hasMore\":false}}")?;
    }
    Ok(n)
}
fn identity_from<'a>(
    a: &[u8],
    key: &[u8],
    prefix: &[u8],
    out: &'a mut [u8],
) -> Result<&'a [u8], E> {
    let s = read_text(a, key, out)?;
    id(s, prefix)
}
fn detail(r: &[u8]) -> Result<usize, E> {
    let a = args(r)?;
    let mut ib = [0; 128];
    let work = identity_from(a, b"mangaId", b"hh:zh:", &mut ib)?;
    let mut ub = [0; 4096];
    let u = read_text(a, b"mangaUrl", &mut ub)?;
    let len = owned_detail(work, u)?;
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"manga\":")?;
    emit_work(dt(len), b, &mut n)?;
    bytes(b, &mut n, b"}")?;
    Ok(n)
}
fn chapters(r: &[u8]) -> Result<usize, E> {
    let a = args(r)?;
    if let Some(p) = member(a, b"page")? {
        if let Some(c) = member(p, b"cursor")? {
            if c != b"null" && c != b"\"\"" {
                return Err("single chapter cursor");
            }
        }
    }
    let mut ib = [0; 128];
    let work = identity_from(a, b"mangaId", b"hh:zh:", &mut ib)?;
    let mut ub = [0; 4096];
    let u = read_text(a, b"mangaUrl", &mut ub)?;
    let len = owned_detail(work, u)?;
    let v = dt(len);
    readable(v)?;
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"items\":[{\"id\":\"hhch:zh:")?;
    bytes(b, &mut n, work)?;
    bytes(b, &mut n, b"\",\"mangaId\":\"hh:zh:")?;
    bytes(b, &mut n, work)?;
    bytes(b, &mut n, b"\",\"title\":")?;
    bytes(b, &mut n, required(v, b"title")?)?;
    bytes(
        b,
        &mut n,
        b",\"chapterNumber\":null,\"volumeNumber\":null,\"language\":\"zh\",\"pageCount\":",
    )?;
    wn(b, &mut n, field_num(v, b"pages")?)?;
    bytes(
        b,
        &mut n,
        b"}],\"page\":{\"nextCursor\":null,\"hasMore\":false}}",
    )?;
    Ok(n)
}
fn image_url<'a>(v: &'a [u8], work: usize) -> Result<&'a [u8], E> {
    let u = text(v, b"source_url")?;
    let mut prefix = [0; 256];
    let mut n = 0;
    bytes(
        &mut prefix,
        &mut n,
        b"https://cdn.hentaihand.com/nhentai/storage/images/",
    )?;
    wn(&mut prefix, &mut n, work)?;
    bytes(&mut prefix, &mut n, b"/")?;
    let tail = u.strip_prefix(&prefix[..n]).ok_or("image URL owner")?;
    if tail.is_empty()
        || tail.len() > 256
        || !tail
            .iter()
            .all(|c| c.is_ascii_alphanumeric() || *c == b'.' || *c == b'_' || *c == b'-')
        || tail == b"."
        || tail == b".."
    {
        return Err("image filename");
    }
    Ok(u)
}
fn pages_body(work: &[u8], u: &[u8]) -> Result<(usize, usize), E> {
    let owner = numeric(work)?;
    let dl = owned_detail(work, u)?;
    let detail = dt(dl);
    readable(detail)?;
    let expected = field_num(detail, b"pages")?;
    let sl = route(u)?;
    let mut url = [0; 4096];
    let mut n = 0;
    bytes(&mut url, &mut n, b"https://hentaihand.com/api/comics/")?;
    bytes(&mut url, &mut n, sl)?;
    bytes(&mut url, &mut n, b"/images")?;
    let len = fetch(&url[..n])?;
    let v = body(len);
    let comic = required(v, b"comic")?;
    if field_num(comic, b"id")? != owner
        || text(comic, b"slug")? != sl
        || field_num(comic, b"pages")? != expected
        || field_num(required(comic, b"language")?, b"id")?
            != field_num(required(detail, b"language")?, b"id")?
    {
        return Err("pages comic owner");
    }
    if required(v, b"chapter")? != b"null" || required(v, b"next_chapter")? != b"null" {
        return Err("single chapter contract");
    }
    let arr = required(v, b"images")?;
    let mut it = Array::new(arr)?;
    let mut count = 0;
    while let Some(x) = it.next()? {
        if field_num(x, b"page")? != count + 1 {
            return Err("page order");
        }
        if member(x, b"ntrchat")? == Some(b"true") {
            if count != expected || text(x, b"id")? != b"ntrchat" {
                return Err("promotion owner");
            }
            let pu = text(x, b"source_url")?;
            let tail = pu
                .strip_prefix(b"https://hentaihand.com/ntrchat/")
                .ok_or("promotion host")?;
            if tail.is_empty()
                || !tail
                    .iter()
                    .all(|c| c.is_ascii_alphanumeric() || *c == b'.' || *c == b'-' || *c == b'_')
                || tail == b"."
                || tail == b".."
            {
                return Err("promotion URL");
            }
            if text(x, b"thumbnail_url")? != pu {
                return Err("promotion URL consistency");
            }
            if it.next()?.is_some() {
                return Err("promotion not terminal");
            };
            break;
        }
        if let Some(flag) = member(x, b"ntrchat")? {
            if flag != b"false" {
                return Err("unknown promotion flag");
            }
        }
        if count >= expected || count >= 4096 {
            return Err("page capacity");
        };
        let uid = field_num(x, b"id")?;
        if uid == 0 {
            return Err("image identity");
        };
        let url = image_url(x, owner)?;
        unsafe {
            if IDS[..count].contains(&uid) {
                return Err("duplicate image ID");
            };
            for &(s, e) in &URL_POS[..count] {
                if &v[s..e] == url {
                    return Err("duplicate image URL");
                }
            }
            IDS[count] = uid;
            let start = url.as_ptr() as usize - v.as_ptr() as usize;
            URL_POS[count] = (start, start + url.len());
        }
        count += 1;
    }
    if count != expected {
        return Err("page completeness");
    };
    Ok((len, count))
}
fn pages(r: &[u8]) -> Result<usize, E> {
    let a = args(r)?;
    let mut ib = [0; 128];
    let work = identity_from(a, b"chapterId", b"hhch:zh:", &mut ib)?;
    if let Some(m) = member(a, b"mangaId")? {
        let mut mb = [0; 128];
        let mid = unescape(quoted(m)?, &mut mb)?;
        if id(mid, b"hh:zh:")? != work {
            return Err("chapter work owner");
        }
    }
    let mut ub = [0; 4096];
    let u = read_text(a, b"mangaUrl", &mut ub)?;
    let (len, count) = pages_body(work, u)?;
    let v = body(len);
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"pages\":[")?;
    let mut it = Array::new(required(v, b"images")?)?;
    for i in 0..count {
        let x = it.next()?.ok_or("page missing")?;
        if i > 0 {
            bytes(b, &mut n, b",")?
        }
        bytes(b, &mut n, b"{\"id\":\"hhpage:zh:")?;
        bytes(b, &mut n, work)?;
        bytes(b, &mut n, b":")?;
        wn(b, &mut n, i)?;
        bytes(b, &mut n, b"\",\"index\":")?;
        wn(b, &mut n, i)?;
        bytes(b, &mut n, b",\"image\":{\"kind\":\"url\",\"url\":")?;
        q(b, &mut n, text(x, b"source_url")?)?;
        bytes(b, &mut n, b"}}")?;
    }
    bytes(b, &mut n, b"]}")?;
    Ok(n)
}
fn descriptor(u: &[u8]) -> Result<usize, E> {
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"imageRequest\":{\"url\":")?;
    q(b, &mut n, u)?;
    bytes(b,&mut n,b",\"method\":\"GET\",\"headers\":{\"Referer\":\"https://hentaihand.com/\",\"Origin\":\"https://hentaihand.com\"}}}")?;
    Ok(n)
}
fn image_request(r: &[u8]) -> Result<usize, E> {
    let a = args(r)?;
    let mut pb = [0; 256];
    let pid = read_text(a, b"pageId", &mut pb)?;
    let mut ub = [0; 4096];
    let u = read_text(a, b"mangaUrl", &mut ub)?;
    let mut rb = [0; 4096];
    let requested = read_text(a, b"url", &mut rb)?;
    if pid.starts_with(b"hh:zh:") {
        let work = id(pid, b"hh:zh:")?;
        let n = owned_detail(work, u)?;
        let v = dt(n);
        let actual = text(v, b"image_url")?;
        cover_allowed(actual, numeric(work)?)?;
        if requested != actual {
            return Err("cover URL ownership");
        };
        return descriptor(actual);
    }
    let parts = pid.strip_prefix(b"hhpage:zh:").ok_or("page identity")?;
    let sep = parts
        .iter()
        .position(|x| *x == b':')
        .ok_or("page identity")?;
    let work = &parts[..sep];
    numeric(work)?;
    let index = number(&parts[sep + 1..]).ok_or("page index")?;
    if parts[sep + 1..].len() > 1 && parts[sep + 1] == b'0' {
        return Err("page index canonical");
    }
    let (len, count) = pages_body(work, u)?;
    if index >= count {
        return Err("page index owner");
    };
    let mut it = Array::new(required(body(len), b"images")?)?;
    let mut selected = None;
    for i in 0..count {
        let x = it.next()?.ok_or("page missing")?;
        if i == index {
            selected = Some(text(x, b"source_url")?)
        }
    }
    let actual = selected.ok_or("page missing")?;
    if actual != requested {
        return Err("page URL ownership");
    };
    descriptor(actual)
}
fn execute(op: &str, res: Result<usize, E>) -> u32 {
    match res {
        Ok(n) => write_success_payload(op, n),
        Err(e) => write_error(op, "source_contract_error", e),
    }
}
fn percent_query(b: &mut [u8], n: &mut usize, query: &[u8]) -> Result<(), E> {
    const HEX: &[u8] = b"0123456789ABCDEF";
    for &c in query {
        if c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.' | b'~') {
            bytes(b, n, &[c])?;
        } else {
            bytes(
                b,
                n,
                &[b'%', HEX[(c >> 4) as usize], HEX[(c & 15) as usize]],
            )?;
        }
    }
    Ok(())
}
fn hex_query(b: &mut [u8], n: &mut usize, query: &[u8]) -> Result<(), E> {
    const HEX: &[u8] = b"0123456789abcdef";
    for &c in query {
        bytes(b, n, &[HEX[(c >> 4) as usize], HEX[(c & 15) as usize]])?;
    }
    Ok(())
}
fn lookup(query: &[u8], uri: &[u8]) -> Result<Option<usize>, E> {
    let mut url = [0; 8192];
    let mut n = 0;
    bytes(&mut url, &mut n, b"https://hentaihand.com/api/")?;
    bytes(&mut url, &mut n, uri)?;
    bytes(&mut url, &mut n, b"?q=")?;
    percent_query(&mut url, &mut n, query)?;
    let len = fetch(&url[..n])?;
    let v = body(len);
    if field_num(v, b"current_page")? != 1 {
        return Err("lookup page owner");
    }
    let per = field_num(v, b"per_page")?;
    let total = field_num(v, b"total")?;
    let last = field_num(v, b"last_page")?;
    if per == 0 || per > 4096 || last == 0 {
        return Err("lookup capacity");
    }
    let mut it = Array::new(required(v, b"data")?)?;
    let mut count = 0;
    let mut exact = None;
    let mut name_buf = [0; 8192];
    while let Some(row) = it.next()? {
        if count >= per {
            return Err("lookup count");
        }
        let owner = field_num(row, b"id")?;
        if owner == 0 {
            return Err("lookup identity");
        }
        unsafe {
            if IDS[..count].contains(&owner) {
                return Err("lookup duplicate");
            }
            IDS[count] = owner;
        }
        let name = read_text(row, b"name", &mut name_buf)?;
        if name.eq_ignore_ascii_case(query) {
            if exact.is_some() {
                return Err("lookup exact ambiguous");
            }
            exact = Some(owner);
        }
        count += 1;
    }
    if count != core::cmp::min(per, total) {
        return Err("lookup completeness");
    }
    if let Some(id) = exact {
        return Ok(Some(id));
    }
    if last != 1 || required(v, b"next_page_url")? != b"null" || count != total {
        return Err("lookup incomplete exact miss");
    }
    if count == 0 {
        if required(v, b"from")? != b"null" || required(v, b"to")? != b"null" {
            return Err("lookup empty pagination");
        }
    } else if field_num(v, b"from")? != 1 || field_num(v, b"to")? != count {
        return Err("lookup pagination");
    }
    Ok(None)
}
fn search(r: &[u8]) -> Result<usize, E> {
    let a = args(r)?;
    let mut query_buf = [0; 2048];
    let query = read_text(a, b"query", &mut query_buf)?;
    let query = core::str::from_utf8(query)
        .map_err(|_| "query UTF8")?
        .trim()
        .as_bytes();
    if query.is_empty() {
        return Err("empty query");
    }
    let mut page = 1;
    let mut cursor_buf = [0; 8192];
    if let Some(p) = member(a, b"page")? {
        if let Some(c) = member(p, b"cursor")? {
            if c != b"null" {
                let c = read_text(p, b"cursor", &mut cursor_buf)?;
                let mut expected = [0; 8192];
                let mut en = 0;
                bytes(&mut expected, &mut en, b"hhq:")?;
                hex_query(&mut expected, &mut en, query)?;
                bytes(&mut expected, &mut en, b":")?;
                let tail = c
                    .strip_prefix(&expected[..en])
                    .ok_or("search cursor query")?;
                page = numeric(tail)?;
                if page <= 1 {
                    return Err("search cursor page");
                }
            }
        }
    }
    if page > 10000 {
        return Err("cursor capacity");
    }
    let mut matched = None;
    for uri in [b"tags" as &[u8], b"artists", b"characters"] {
        if let Some(id) = lookup(query, uri)? {
            matched = Some((uri, id));
            break;
        }
    }
    let mut url = [0; 8192];
    let mut n = 0;
    bytes(&mut url, &mut n, b"https://hentaihand.com/api/comics?page=")?;
    wn(&mut url, &mut n, page)?;
    if let Some((uri, id)) = matched {
        bytes(&mut url, &mut n, b"&")?;
        bytes(&mut url, &mut n, uri)?;
        bytes(&mut url, &mut n, b"%5B0%5D=")?;
        wn(&mut url, &mut n, id)?;
    } else {
        bytes(&mut url, &mut n, b"&q=")?;
        percent_query(&mut url, &mut n, query)?;
    }
    bytes(
        &mut url,
        &mut n,
        b"&languages%5B-1%5D=1&languages%5B-2%5D=50&sort=uploaded_at&order=desc&duration=day",
    )?;
    let len = fetch(&url[..n])?;
    emit_listing(len, page, Some(query))
}
fn run_search(r: &[u8]) -> u32 {
    execute("search", search(r))
}
fn run_get_manga(r: &[u8]) -> u32 {
    execute("get_manga", detail(r))
}
fn run_get_chapters(r: &[u8]) -> u32 {
    execute("get_chapters", chapters(r))
}
fn run_get_pages(r: &[u8]) -> u32 {
    execute("get_pages", pages(r))
}
fn run_get_manga_list(r: &[u8]) -> u32 {
    execute("get_manga_list", listing(r))
}
fn run_get_image_request(r: &[u8]) -> u32 {
    execute("get_image_request", image_request(r))
}
fn run_get_listings(_: &[u8]) -> u32 {
    let s="{\"listings\":[{\"id\":\"listing:popular\",\"name\":\"人气较多（全部时间）\",\"kind\":\"popular\",\"supportsPaging\":true}]}".as_bytes();
    payload_buf()[..s.len()].copy_from_slice(s);
    write_success_payload("get_listings", s.len())
}
fn run_get_home(_: &[u8]) -> u32 {
    write_error("get_home", "not_supported", "not supported")
}
fn run_get_filters(_: &[u8]) -> u32 {
    write_error("get_filters", "not_supported", "not supported")
}
fn run_get_settings(_: &[u8]) -> u32 {
    write_error("get_settings", "not_supported", "not supported")
}

const INFO_JSON: &str = r#"{"sourceInfo":{"id":"com.hentaihand.zh.koma","name":"HentaiHand","version":"0.1.1","apiVersion":"0.2","language":"zh","author":"Koma","contentRating":"nsfw","description":"Anonymous Chinese metadata; NSFW source; per-work rating and usage unknown.","localizedNames":{"zh-Hans":"HentaiHand（中文）","zh-Hant":"HentaiHand（中文）","en":"HentaiHand (Chinese)"}},"capabilities":{"search":true,"mangaDetail":true,"chapters":true,"pages":true,"listings":true,"mangaList":true,"imageRequest":true,"home":false,"filters":false,"settings":false,"future":{"login":false,"auth":false,"process_page_image":false,"page_description":false,"base_url":false,"deeplink":false,"migration":false}}}"#;
// Source-local ABI gate; SDK remains byte-identical. Check before helper forms a slice.
fn checked_request(ptr: u32, len: u32) -> Option<&'static [u8]> {
    let cap = core::arch::wasm32::memory_size::<0>().checked_mul(65536)?;
    let start = ptr as usize;
    let end = start.checked_add(len as usize)?;
    if start == 0 || len == 0 || len as usize >= MAX || end > cap {
        return None;
    };
    read_request(ptr, len)
}
#[no_mangle]
pub extern "C" fn koma_source_init(ptr: u32, len: u32) -> i32 {
    if checked_request(ptr, len).is_some() {
        0
    } else {
        -1
    }
}
#[no_mangle]
pub extern "C" fn koma_source_info() -> u32 {
    response_buffer().write_success_parts("source_info", &[INFO_JSON.as_bytes()])
}
macro_rules! export_op {
    ($f:ident,$op:literal,$run:ident) => {
        #[no_mangle]
        pub extern "C" fn $f(ptr: u32, len: u32) -> u32 {
            match checked_request(ptr, len) {
                Some(r) => $run(r),
                None => write_error($op, "invalid_request", "invalid ABI request range"),
            }
        }
    };
}
export_op!(koma_source_search, "search", run_search);
export_op!(koma_source_get_manga, "get_manga", run_get_manga);
export_op!(koma_source_get_chapters, "get_chapters", run_get_chapters);
export_op!(koma_source_get_pages, "get_pages", run_get_pages);
export_op!(koma_source_get_listings, "get_listings", run_get_listings);
export_op!(
    koma_source_get_manga_list,
    "get_manga_list",
    run_get_manga_list
);
export_op!(koma_source_get_home, "get_home", run_get_home);
export_op!(koma_source_get_filters, "get_filters", run_get_filters);
export_op!(koma_source_get_settings, "get_settings", run_get_settings);
export_op!(
    koma_source_get_image_request,
    "get_image_request",
    run_get_image_request
);
#[no_mangle]
pub extern "C" fn koma_source_alloc(len: u32) -> u32 {
    if len == 0 || len as usize >= MAX {
        0
    } else {
        core::ptr::addr_of_mut!(REQUEST) as *mut u8 as u32
    }
}
#[no_mangle]
pub extern "C" fn koma_source_free(ptr: u32) {
    response_buffer().free(ptr)
}
