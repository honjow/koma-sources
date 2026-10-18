#![no_std]
use koma_source_sdk::{host, json_utils::*};
type E = &'static str;
const MAX: usize = 1048576;
koma_source_sdk::koma_source_buffers! {payload:1048000,http_out:MAX,body:MAX,http_req:32768,scratch:8192,}
koma_source_sdk::koma_source_helpers!();
include!("parser.rs");
static mut REQUEST: [u8; MAX] = [0; MAX];
static mut DETAIL: [u8; MAX] = [0; MAX];
static mut IDS: [usize; 4096] = [0; 4096];
fn fetch(url: &[u8]) -> Result<usize, E> {
    if host::check_cancel() {
        return Err("cancelled");
    }
    if !url.starts_with(b"https://api.simply-hentai.com/v3/") {
        return Err("API owner");
    }
    let req = http_req_buf();
    let mut n = 0;
    bytes(req, &mut n, b"{\"version\":1,\"method\":\"GET\",\"url\":")?;
    q(req, &mut n, url)?;
    bytes(req,&mut n,b",\"headers\":{\"User-Agent\":\"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/121.0\",\"Referer\":\"https://www.simply-hentai.com/\",\"Origin\":\"https://www.simply-hentai.com\",\"Cache-Control\":\"max-age=600\"}}")?;
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
fn read_text<'a>(v: &[u8], key: &[u8], out: &'a mut [u8]) -> Result<&'a [u8], E> {
    unescape(text(v, key)?, out)
}
fn enc(b: &mut [u8], n: &mut usize, s: &[u8]) -> Result<(), E> {
    const H: &[u8] = b"0123456789ABCDEF";
    for &c in s {
        if c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.' | b'~') {
            bytes(b, n, &[c])?
        } else {
            bytes(b, n, &[b'%', H[(c >> 4) as usize], H[(c & 15) as usize]])?
        }
    }
    Ok(())
}
fn segment(s: &[u8]) -> Result<(), E> {
    core::str::from_utf8(s).map_err(|_| "route UTF8")?;
    if s.is_empty()
        || s.len() > 2048
        || s == b"."
        || s == b".."
        || s.iter()
            .any(|c| *c <= 32 || matches!(c, b'/' | b'\\' | b'%' | b'?' | b'#' | b':'))
    {
        return Err("route segment");
    }
    Ok(())
}
fn nib(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}
fn decode_segment<'a>(s: &[u8], out: &'a mut [u8]) -> Result<&'a [u8], E> {
    let (mut i, mut n) = (0, 0);
    while i < s.len() {
        let c = if s[i] == b'%' {
            let h = *s.get(i + 1).ok_or("route encoding")?;
            let l = *s.get(i + 2).ok_or("route encoding")?;
            i += 3;
            (nib(h).ok_or("route encoding")? << 4) | nib(l).ok_or("route encoding")?
        } else {
            let c = s[i];
            i += 1;
            c
        };
        bytes(out, &mut n, &[c])?;
    }
    let value = &out[..n];
    segment(value)?;
    let mut canonical = [0; 8192];
    let mut cn = 0;
    enc(&mut canonical, &mut cn, value)?;
    if s != &canonical[..cn] {
        return Err("route noncanonical encoding");
    }
    Ok(value)
}
fn route<'a>(u: &'a [u8]) -> Result<(&'a [u8], &'a [u8]), E> {
    let path = u
        .strip_prefix(b"https://www.simply-hentai.com/")
        .ok_or("canonical route")?;
    let i = path
        .iter()
        .position(|c| *c == b'/')
        .ok_or("route structure")?;
    if path[i + 1..].contains(&b'/') {
        return Err("route structure");
    }
    let (mut a, mut b) = ([0; 2048], [0; 2048]);
    decode_segment(&path[..i], &mut a)?;
    decode_segment(&path[i + 1..], &mut b)?;
    Ok((&path[..i], &path[i + 1..]))
}
fn url_for(v: &[u8], out: &mut [u8]) -> Result<usize, E> {
    let mut a = [0; 2048];
    let mut b = [0; 2048];
    let series = read_text(required(v, b"series")?, b"slug", &mut a)?;
    let slug = read_text(v, b"slug", &mut b)?;
    segment(series)?;
    segment(slug)?;
    let mut n = 0;
    bytes(out, &mut n, b"https://www.simply-hentai.com/")?;
    enc(out, &mut n, series)?;
    bytes(out, &mut n, b"/")?;
    enc(out, &mut n, slug)?;
    Ok(n)
}
fn allowed_image(u: &[u8], owner: usize) -> Result<(), E> {
    let rest = u
        .strip_prefix(b"https://images.sh-cdn.com/")
        .ok_or("image host")?;
    let mut tail = rest;
    for _ in 0..4 {
        if tail.len() < 2 || !tail[0].is_ascii_hexdigit() || tail[1] != b'/' {
            return Err("image path");
        }
        tail = &tail[2..];
    }
    let i = tail.iter().position(|c| *c == b'/').ok_or("image owner")?;
    if numeric(&tail[..i])? != owner {
        return Err("image owner");
    }
    let file = &tail[i + 1..];
    if file.is_empty()
        || file.len() > 128
        || !file
            .iter()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.'))
        || !file.ends_with(b".webp")
        || file.windows(2).any(|x| x == b"..")
    {
        return Err("image filename");
    }
    Ok(())
}
fn validate_work(v: &[u8], owner: Option<usize>) -> Result<usize, E> {
    let id = field_num(v, b"id")?;
    if id == 0 || owner.is_some_and(|x| x != id) {
        return Err("work owner");
    }
    if text(v, b"type")? != b"Album" || text(required(v, b"language")?, b"slug")? != b"chinese" {
        return Err("Chinese album owner");
    }
    quoted(required(v, b"title")?)?;
    let s = required(v, b"series")?;
    if field_num(s, b"id")? == 0 || text(s, b"type")? != b"Series" {
        return Err("series owner");
    }
    quoted(required(s, b"title")?)?;
    let mut r = [0; 16384];
    url_for(v, &mut r)?;
    let count = field_num(v, b"image_count")?;
    if count == 0 || count > 4096 {
        return Err("page count capacity");
    }
    let preview = required(v, b"preview")?;
    if field_num(preview, b"page_num")? == 0 || field_num(preview, b"id")? == 0 {
        return Err("preview identity");
    }
    let mut image = [0; 4096];
    for key in [b"thumb" as &[u8], b"full"] {
        let u = read_text(required(preview, b"sizes")?, key, &mut image)?;
        allowed_image(u, id)?;
    }
    Ok(id)
}
fn names(v: &[u8], key: &[u8], b: &mut [u8], n: &mut usize) -> Result<(), E> {
    if let Some(items) = member(v, key)? {
        let mut it = Array::new(items)?;
        bytes(b, n, b",")?;
        q(b, n, key)?;
        bytes(b, n, b":[")?;
        let mut count = 0;
        while let Some(x) = it.next()? {
            if count > 0 {
                bytes(b, n, b",")?;
            }
            bytes(b, n, required(x, b"title")?)?;
            quoted(required(x, b"title")?)?;
            count += 1;
        }
        bytes(b, n, b"]")?;
    }
    Ok(())
}
fn emit_work(v: &[u8], b: &mut [u8], n: &mut usize) -> Result<(), E> {
    let id = validate_work(v, None)?;
    let mut route = [0; 16384];
    let rn = url_for(v, &mut route)?;
    let mut u = [0; 4096];
    let cover = read_text(
        required(required(v, b"preview")?, b"sizes")?,
        b"thumb",
        &mut u,
    )?;
    bytes(b, n, b"{\"id\":\"sh:zh:")?;
    wn(b, n, id)?;
    bytes(b, n, b"\",\"title\":")?;
    bytes(b, n, required(v, b"title")?)?;
    bytes(b, n, b",\"url\":")?;
    q(b, n, &route[..rn])?;
    bytes(b, n, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(b, n, cover)?;
    bytes(
        b,
        n,
        b"},\"status\":\"unknown\",\"contentRating\":\"unknown\"",
    )?;
    if let Some(x) = member(v, b"description")? {
        if x != b"null" {
            quoted(x)?;
            bytes(b, n, b",\"description\":")?;
            bytes(b, n, x)?;
        }
    }
    names(v, b"artists", b, n)?;
    names(v, b"tags", b, n)?;
    bytes(b, n, b"}")
}
fn owned_detail(work: &[u8], u: &[u8]) -> Result<usize, E> {
    let owner = numeric(work)?;
    let (_, slug) = route(u)?;
    let mut api = [0; 8192];
    let mut n = 0;
    bytes(&mut api, &mut n, b"https://api.simply-hentai.com/v3/manga/")?;
    bytes(&mut api, &mut n, slug)?;
    let len = fetch(&api[..n])?;
    let v = required(body(len), b"data")?;
    validate_work(v, Some(owner))?;
    let mut actual = [0; 16384];
    let an = url_for(v, &mut actual)?;
    if u != &actual[..an] {
        return Err("detail route owner");
    }
    unsafe {
        DETAIL[..v.len()].copy_from_slice(v);
    }
    Ok(v.len())
}
fn args_work_route<'a>(
    a: &[u8],
    work: &'a mut [u8],
    url: &'a mut [u8],
) -> Result<(&'a [u8], &'a [u8]), E> {
    let id = read_text(a, b"mangaId", work)?;
    let work = id.strip_prefix(b"sh:zh:").ok_or("work namespace")?;
    numeric(work)?;
    let url = read_text(a, b"mangaUrl", url)?;
    route(url)?;
    Ok((work, url))
}
fn listing(r: &[u8]) -> Result<usize, E> {
    let a = args(r)?;
    if text(a, b"listingId")? != b"listing:chinese" {
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
    let mut u = [0; 256];
    let mut un = 0;
    bytes(
        &mut u,
        &mut un,
        b"https://api.simply-hentai.com/v3/tag/chinese?type=language&page=",
    )?;
    wn(&mut u, &mut un, page)?;
    let len = fetch(&u[..un])?;
    let v = body(len);
    let p = required(v, b"pagination")?;
    let per = field_num(p, b"per_page")?;
    let total = field_num(p, b"count")?;
    let last = field_num(p, b"pages")?;
    if field_num(p, b"current")? != page || per == 0 || per > 4096 || last == 0 || page > last {
        return Err("pagination owner capacity");
    }
    let next = required(p, b"next")?;
    let has = next != b"null";
    if has {
        if numeric(next)? != page.checked_add(1).ok_or("page overflow")? || page >= last {
            return Err("next owner");
        }
    } else if page != last {
        return Err("missing next");
    }
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"items\":[")?;
    let mut it = Array::new(required(required(v, b"data")?, b"albums")?)?;
    let mut count = 0;
    while let Some(x) = it.next()? {
        if count >= per {
            return Err("list count");
        }
        let owner = validate_work(x, None)?;
        unsafe {
            if IDS[..count].contains(&owner) {
                return Err("duplicate work");
            }
            IDS[count] = owner;
        }
        if count > 0 {
            bytes(b, &mut n, b",")?;
        }
        emit_work(x, b, &mut n)?;
        count += 1;
    }
    let offset = (page - 1).checked_mul(per).ok_or("offset overflow")?;
    if count == 0 || count != core::cmp::min(per, total.checked_sub(offset).ok_or("list total")?) {
        return Err("list incomplete");
    }
    bytes(b, &mut n, b"],\"page\":{\"nextCursor\":")?;
    if has {
        bytes(b, &mut n, b"\"")?;
        wn(b, &mut n, numeric(next)?)?;
        bytes(b, &mut n, b"\",\"hasMore\":true}}")?;
    } else {
        bytes(b, &mut n, b"null,\"hasMore\":false}}")?;
    }
    Ok(n)
}
fn detail(r: &[u8]) -> Result<usize, E> {
    let a = args(r)?;
    let (mut w, mut u) = ([0; 128], [0; 16384]);
    let (work, url) = args_work_route(a, &mut w, &mut u)?;
    let len = owned_detail(work, url)?;
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
            if c != b"null" {
                return Err("single album cursor");
            }
        }
    }
    let (mut w, mut u) = ([0; 128], [0; 16384]);
    let (work, url) = args_work_route(a, &mut w, &mut u)?;
    let len = owned_detail(work, url)?;
    let v = dt(len);
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"items\":[{\"id\":\"shch:zh:")?;
    bytes(b, &mut n, work)?;
    bytes(b, &mut n, b"\",\"mangaId\":\"sh:zh:")?;
    bytes(b, &mut n, work)?;
    bytes(b,&mut n,b"\",\"title\":\"Chapter\",\"chapterNumber\":null,\"volumeNumber\":null,\"language\":\"zh\",\"pageCount\":")?;
    wn(b, &mut n, field_num(v, b"image_count")?)?;
    bytes(
        b,
        &mut n,
        b"}],\"page\":{\"nextCursor\":null,\"hasMore\":false}}",
    )?;
    Ok(n)
}
fn pages_body(work: &[u8], url: &[u8]) -> Result<(usize, usize), E> {
    let dl = owned_detail(work, url)?;
    let dv = dt(dl);
    let expected = field_num(dv, b"image_count")?;
    let (_, slug) = route(url)?;
    let mut api = [0; 8192];
    let mut n = 0;
    bytes(&mut api, &mut n, b"https://api.simply-hentai.com/v3/manga/")?;
    bytes(&mut api, &mut n, slug)?;
    bytes(&mut api, &mut n, b"/pages")?;
    let len = fetch(&api[..n])?;
    let v = required(body(len), b"data")?;
    let owner = validate_work(v, Some(numeric(work)?))?;
    let mut actual = [0; 16384];
    let an = url_for(v, &mut actual)?;
    if &actual[..an] != url
        || field_num(required(v, b"series")?, b"id")? != field_num(required(dv, b"series")?, b"id")?
        || field_num(v, b"image_count")? != expected
    {
        return Err("pages work owner");
    }
    let mut it = Array::new(required(v, b"pages")?)?;
    let mut count = 0;
    let mut ub = [0; 4096];
    while let Some(x) = it.next()? {
        if count >= expected || field_num(x, b"page_num")? != count + 1 {
            return Err("page order count");
        }
        let image = field_num(x, b"id")?;
        if image == 0 {
            return Err("page identity");
        }
        let url = read_text(required(x, b"sizes")?, b"full", &mut ub)?;
        allowed_image(url, owner)?;
        unsafe {
            if IDS[..count].contains(&image) {
                return Err("duplicate imageID");
            }
            IDS[count] = image;
        }
        let mut previous = Array::new(required(v, b"pages")?)?;
        let mut prevbuf = [0; 4096];
        for _ in 0..count {
            let row = previous.next()?.ok_or("page missing")?;
            if read_text(required(row, b"sizes")?, b"full", &mut prevbuf)? == url {
                return Err("duplicate imageURL");
            }
        }
        count += 1;
    }
    if count != expected {
        return Err("pages incomplete");
    }
    Ok((len, count))
}
fn pages(r: &[u8]) -> Result<usize, E> {
    let a = args(r)?;
    let (mut idbuf, mut ub) = ([0; 128], [0; 16384]);
    let c = read_text(a, b"chapterId", &mut idbuf)?;
    let work = id(c, b"shch:zh:")?;
    if let Some(m) = member(a, b"mangaId")? {
        if id(quoted(m)?, b"sh:zh:")? != work {
            return Err("chapter work owner");
        }
    }
    let url = read_text(a, b"mangaUrl", &mut ub)?;
    let (len, count) = pages_body(work, url)?;
    let v = required(body(len), b"data")?;
    let mut it = Array::new(required(v, b"pages")?)?;
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"pages\":[")?;
    let mut u = [0; 4096];
    for i in 0..count {
        let x = it.next()?.ok_or("page missing")?;
        if i > 0 {
            bytes(b, &mut n, b",")?;
        }
        bytes(b, &mut n, b"{\"id\":\"shpage:zh:")?;
        bytes(b, &mut n, work)?;
        bytes(b, &mut n, b":")?;
        wn(b, &mut n, field_num(x, b"id")?)?;
        bytes(b, &mut n, b"\",\"index\":")?;
        wn(b, &mut n, i)?;
        bytes(b, &mut n, b",\"image\":{\"kind\":\"url\",\"url\":")?;
        q(
            b,
            &mut n,
            read_text(required(x, b"sizes")?, b"full", &mut u)?,
        )?;
        bytes(b, &mut n, b"}}")?;
    }
    bytes(b, &mut n, b"]}")?;
    Ok(n)
}
fn descriptor(url: &[u8]) -> Result<usize, E> {
    let b = payload_buf();
    let mut n = 0;
    bytes(b, &mut n, b"{\"imageRequest\":{\"method\":\"GET\",\"url\":")?;
    q(b, &mut n, url)?;
    bytes(b,&mut n,b",\"headers\":{\"Referer\":\"https://www.simply-hentai.com/\",\"Origin\":\"https://www.simply-hentai.com\"}}}")?;
    Ok(n)
}
fn image_request(r: &[u8]) -> Result<usize, E> {
    let a = args(r)?;
    let (mut pb, mut ub, mut rb) = ([0; 256], [0; 16384], [0; 4096]);
    let pid = read_text(a, b"pageId", &mut pb)?;
    let url = read_text(a, b"mangaUrl", &mut ub)?;
    let requested = read_text(a, b"url", &mut rb)?;
    if pid.starts_with(b"sh:zh:") {
        let work = id(pid, b"sh:zh:")?;
        let dl = owned_detail(work, url)?;
        let mut x = [0; 4096];
        let actual = read_text(
            required(required(dt(dl), b"preview")?, b"sizes")?,
            b"thumb",
            &mut x,
        )?;
        if actual != requested {
            return Err("cover URL owner");
        }
        return descriptor(actual);
    }
    let parts = pid.strip_prefix(b"shpage:zh:").ok_or("page namespace")?;
    let i = parts
        .iter()
        .position(|c| *c == b':')
        .ok_or("page identity")?;
    let work = &parts[..i];
    numeric(work)?;
    let image = numeric(&parts[i + 1..])?;
    if let Some(m) = member(a, b"mangaId")? {
        if id(quoted(m)?, b"sh:zh:")? != work {
            return Err("image work owner");
        }
    }
    let (len, count) = pages_body(work, url)?;
    let mut it = Array::new(required(required(body(len), b"data")?, b"pages")?)?;
    let mut actualbuf = [0; 4096];
    for _ in 0..count {
        let x = it.next()?.ok_or("page missing")?;
        if field_num(x, b"id")? == image {
            let actual = read_text(required(x, b"sizes")?, b"full", &mut actualbuf)?;
            if actual != requested {
                return Err("page URL owner");
            }
            return descriptor(actual);
        }
    }
    Err("page imageID owner")
}
fn execute(op: &str, res: Result<usize, E>) -> u32 {
    match res {
        Ok(n) => write_success_payload(op, n),
        Err(e) => write_error(op, "source_contract_error", e),
    }
}
fn run_search(_: &[u8]) -> u32 {
    write_error("search", "not_supported", "search not observed")
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
    let s="{\"listings\":[{\"id\":\"listing:chinese\",\"name\":\"中文列表\",\"kind\":\"popular\",\"supportsPaging\":true}]}".as_bytes();
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

const INFO_JSON: &str = r#"{"sourceInfo":{"id":"com.simplyhentai.zh.koma","name":"Simply Hentai","version":"0.1.0","apiVersion":"0.2","language":"zh","author":"Koma","contentRating":"nsfw","description":"Chinese language metadata listing; NSFW source; per-work rating and usage unknown.","localizedNames":{"zh-Hans":"Simply Hentai（中文）","zh-Hant":"Simply Hentai（中文）","en":"Simply Hentai (Chinese)"}},"capabilities":{"search":false,"mangaDetail":true,"chapters":true,"pages":true,"listings":true,"mangaList":true,"imageRequest":true,"home":false,"filters":false,"settings":false,"future":{"login":false,"auth":false,"process_page_image":false,"page_description":false,"base_url":false,"deeplink":false,"migration":false}}}"#;
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
