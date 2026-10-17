#![no_std]
extern crate koma_source_sdk;
use koma_source_sdk::host::{self, HtmlDescriptor};
use koma_source_sdk::json_utils::*;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{build_get_request, decode_json_body_into, fetch_error_code};
koma_source_sdk::koma_source_buffers! {payload:1024*1024-1024,http_out:1024*1024,body:1024*1024,http_req:4096,scratch:8192,}
koma_source_sdk::koma_source_helpers!();
const BASE: &[u8] = b"https://www.dragonball-multiverse.com";
const LIST: &[u8] = b"https://www.dragonball-multiverse.com/cn/read.html";
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.dragonballmultiverse.zh.koma",
    name: "Dragon Ball Multiverse",
    version: "0.1.1",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "Chinese translated public image comics; overlay chapters unsupported.",
    content_rating: "safe",
};
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: false,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: true,
    manga_list: true,
    home: false,
    filters: false,
    settings: false,
    image_request: true,
    credentials: false,
};
type R<T> = Result<T, &'static str>;
#[link(wasm_import_module = "koma_host")]
extern "C" {
    #[link_name = "html_select_all"]
    fn all_raw(d: i32, s: *const u8, n: u32, o: *mut u8, c: u32) -> i32;
}
struct Owned(HtmlDescriptor);
impl Drop for Owned {
    fn drop(&mut self) {
        let _ = host::html_close(self.0);
    }
}
struct All {
    ids: [i32; 1000],
    len: usize,
}
impl All {
    fn new(d: HtmlDescriptor, s: &[u8]) -> R<Self> {
        let mut a = Self {
            ids: [0; 1000],
            len: 0,
        };
        let count = unsafe {
            all_raw(
                d.raw(),
                s.as_ptr(),
                s.len() as u32,
                a.ids.as_mut_ptr() as *mut u8,
                4000,
            )
        };
        if count < 0 {
            return Err("parse_error");
        }
        a.len = (count as usize).min(1000);
        if count as usize > 1000 {
            return Err("capacity_exceeded");
        }
        if a.ids[..a.len].iter().any(|x| *x <= 0) {
            return Err("parse_error");
        }
        Ok(a)
    }
    fn at(&self, i: usize) -> HtmlDescriptor {
        HtmlDescriptor::from_raw(self.ids[i])
    }
}
impl Drop for All {
    fn drop(&mut self) {
        for id in &self.ids[..self.len] {
            if *id > 0 {
                let _ = host::html_close(HtmlDescriptor::from_raw(*id));
            }
        }
    }
}
fn attr<'a>(d: HtmlDescriptor, k: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    let n = host::html_attr(d, k, b).map_err(|_| "parse_error")?;
    if n == b.len() {
        return Err("capacity_exceeded");
    }
    Ok(&b[..n])
}
fn txt<'a>(d: HtmlDescriptor, b: &'a mut [u8]) -> R<&'a [u8]> {
    let n = host::html_text(d, b).map_err(|_| "parse_error")?;
    if n == b.len() {
        return Err("capacity_exceeded");
    }
    Ok(trim_ascii(&b[..n]))
}
fn sa<'a>(d: HtmlDescriptor, s: &[u8], k: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    let x = Owned(host::html_select(d, s).map_err(|_| "missing_field")?);
    attr(x.0, k, b)
}
fn st<'a>(d: HtmlDescriptor, s: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    let x = Owned(host::html_select(d, s).map_err(|_| "missing_field")?);
    txt(x.0, b)
}
fn fetch(url: &[u8]) -> R<Owned> {
    if host::check_cancel() {
        return Err("cancelled");
    }
    let n = build_get_request(
        http_req_buf(),
        url,
        None,
        &[(b"Cache-Control", b"max-age=600")],
    )
    .ok_or("capacity_exceeded")?;
    let z = host::http_request(&http_req_buf()[..n], http_out()).map_err(|_| "transport_error")?;
    if z >= http_out().len() {
        return Err("capacity_exceeded");
    }
    // Host transport success does not imply HTTP success.
    let status = extract_json_number(&http_out()[..z], b"status")
        .or_else(|| extract_json_number(&http_out()[..z], b"statusCode"))
        .ok_or("transport_error")?;
    if status.len() != 3 || status.iter().any(|x| !x.is_ascii_digit()) {
        return Err("transport_error");
    }
    let status = ((status[0] - b'0') as u16) * 100
        + ((status[1] - b'0') as u16) * 10
        + (status[2] - b'0') as u16;
    if !(200..300).contains(&status) {
        return Err("http_error");
    }
    let n =
        decode_json_body_into(&http_out()[..z], body_buf()).map_err(|e| fetch_error_code(e).0)?;
    if core::str::from_utf8(&body_buf()[..n]).is_err() {
        return Err("invalid_utf8");
    }
    let d = host::html_parse(&body_buf()[..n]).map_err(|_| "parse_error")?;
    Ok(Owned(d))
}
fn absolute<'a>(v: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    let mut n = 0;
    if v.starts_with(BASE) {
        if v.get(BASE.len()) != Some(&b'/') {
            return Err("invalid_url");
        }
        if !write_bytes(b, &mut n, v) {
            return Err("capacity_exceeded");
        }
    } else if v.starts_with(b"/") && !v.starts_with(b"//") {
        if !(write_bytes(b, &mut n, BASE) && write_bytes(b, &mut n, v)) {
            return Err("capacity_exceeded");
        }
    } else if !v.contains(&b':') && !v.starts_with(b"#") {
        if !(write_bytes(b, &mut n, BASE)
            && write_bytes(b, &mut n, b"/cn/")
            && write_bytes(b, &mut n, v))
        {
            return Err("capacity_exceeded");
        }
    } else {
        return Err("invalid_url");
    }
    let u = &b[..n];
    if u.contains(&b'#')
        || u.contains(&b'\\')
        || u.iter().any(|x| *x <= 32)
        || core::str::from_utf8(u).is_err()
    {
        return Err("invalid_url");
    }
    Ok(u)
}
fn comic(v: &[u8]) -> bool {
    [
        b"page".as_slice(),
        b"dbm-colors",
        b"hearts",
        b"chibi-son-bra",
        b"strip",
    ]
    .contains(&v)
}
fn manga_id(req: &[u8]) -> R<&[u8]> {
    let x = extract_json_string(req, b"mangaId").ok_or("invalid_request")?;
    let c = x.strip_prefix(b"dbm:zh:").ok_or("invalid_request")?;
    if !comic(c) {
        return Err("invalid_request");
    }
    Ok(c)
}
fn chapter_path(v: &[u8]) -> R<&[u8]> {
    let p = v.strip_prefix(BASE).unwrap_or(v);
    let q = p
        .strip_prefix(b"/cn/chapters.html?comic=")
        .ok_or("invalid_chapter_url")?;
    let split = find_subslice(q, b"&chapter=").ok_or("invalid_chapter_url")?;
    if !comic(&q[..split]) {
        return Err("invalid_chapter_url");
    }
    let n = &q[split + 9..];
    if n.is_empty()
        || n.len() > 6
        || n.iter().any(|c| !c.is_ascii_digit())
        || n.iter().all(|c| *c == b'0')
    {
        return Err("invalid_chapter_url");
    }
    Ok(p)
}
fn page_url(v: &[u8]) -> bool {
    let p = v.strip_prefix(BASE).unwrap_or(v);
    if !p.starts_with(b"/cn/") || !p.ends_with(b".html") {
        return false;
    }
    let p = &p[4..p.len() - 5];
    let Some(i) = p.iter().rposition(|x| *x == b'-') else {
        return false;
    };
    comic(&p[..i])
        && !p[i + 1..].is_empty()
        && p[i + 1..].len() <= 8
        && p[i + 1..].iter().all(u8::is_ascii_digit)
}
struct W {
    n: usize,
}
impl W {
    fn new() -> Self {
        Self { n: 0 }
    }
    fn put(&mut self, b: &[u8]) -> R<()> {
        write_bytes(payload_buf(), &mut self.n, b)
            .then_some(())
            .ok_or("capacity_exceeded")
    }
    fn q(&mut self, b: &[u8]) -> R<()> {
        self.put(b"\"")?;
        if !append_json_escaped(payload_buf(), &mut self.n, b) {
            return Err("capacity_exceeded");
        }
        self.put(b"\"")
    }
    fn num(&mut self, n: usize) -> R<()> {
        write_usize(payload_buf(), &mut self.n, n)
            .then_some(())
            .ok_or("capacity_exceeded")
    }
    fn end(self, op: &str) -> u32 {
        write_success_payload(op, self.n)
    }
}
fn card(w: &mut W, d: HtmlDescriptor, c: &[u8]) -> R<()> {
    let mut t = [0; 1024];
    let title = st(d, b"h3", &mut t)?;
    let mut image = [0; 2048];
    let raw = sa(d, b"img", b"src", &mut image)?;
    let mut abs = [0; 2048];
    let cover = absolute(raw, &mut abs)?;
    let mut desc = [0; 4096];
    let description = st(d, b".dbm-read > div", &mut desc)?;
    w.put(br#"{"id":"dbm:zh:"#)?;
    if !append_json_escaped(payload_buf(), &mut w.n, c) {
        return Err("capacity_exceeded");
    }
    w.put(br#"","title":"#)?;
    w.q(title)?;
    w.put(br#", "cover":{"kind":"url","url":"#)?;
    w.q(cover)?;
    w.put(br#"},"authors":[],"status":"unknown","contentRating":"safe","description":"#)?;
    w.q(description)?;
    w.put(br#", "tags":["#)?;
    let tags = All::new(d, b".dbm-tags span")?;
    for i in 0..tags.len {
        if i > 0 {
            w.put(b",")?;
        }
        let mut text = [0; 512];
        w.q(txt(tags.at(i), &mut text)?)?;
    }
    w.put(br#"],"links":[{"kind":"source","url":"#)?;
    let mut path = [0; 256];
    let mut n = 0;
    if !(write_bytes(&mut path, &mut n, BASE)
        && write_bytes(&mut path, &mut n, b"/cn/chapters.html?comic=")
        && write_bytes(&mut path, &mut n, c))
    {
        return Err("capacity_exceeded");
    }
    w.q(&path[..n])?;
    w.put(b"}]}")
}
fn cards(detail: Option<&[u8]>, op: &str) -> R<W> {
    let doc = fetch(LIST)?;
    let items = All::new(doc.0, b"#dbm-reads .dbm-read")?;
    if items.len == 0 {
        return Err("missing_field");
    }
    let mut w = W::new();
    w.put(if detail.is_some() {
        br#"{"manga":"#
    } else {
        br#"{"items":["#
    })?;
    let mut written = 0;
    for i in 0..items.len {
        let d = items.at(i);
        let blocked = All::new(d, b".dbm-read-not-released")?;
        if blocked.len > 0 {
            continue;
        }
        let mut h = [0; 1024];
        let raw = sa(d, b"a[href]", b"href", &mut h)?;
        let mut u = [0; 1024];
        let url = absolute(raw, &mut u)?;
        let Some(c) =
            url.strip_prefix(b"https://www.dragonball-multiverse.com/cn/chapters.html?comic=")
        else {
            continue;
        };
        if !comic(c) {
            continue;
        }
        if let Some(only) = detail {
            if c != only {
                continue;
            }
        }
        if written > 0 {
            if detail.is_some() {
                return Err("ambiguous_card");
            }
            w.put(b",")?
        }
        card(&mut w, d, c)?;
        written += 1;
    }
    if written == 0 {
        return Err("not_found");
    }
    w.put(if detail.is_some() {
        b"}"
    } else {
        br#"],"page":{"nextCursor":null,"hasMore":false}}"#
    })?;
    let _ = op;
    Ok(w)
}
fn chapters(req: &[u8]) -> R<W> {
    let c = manga_id(req)?;
    let mut url = [0; 256];
    let mut n = 0;
    if !(write_bytes(&mut url, &mut n, BASE)
        && write_bytes(&mut url, &mut n, b"/cn/chapters.html?comic=")
        && write_bytes(&mut url, &mut n, c))
    {
        return Err("capacity_exceeded");
    }
    let doc = fetch(&url[..n])?;
    let all = All::new(doc.0, b".cadrelect.chapter")?;
    if all.len == 0 {
        return Err("missing_field");
    }
    let mut w = W::new();
    w.put(br#"{"chapters":["#)?;
    for j in 0..all.len {
        let d = all.at(all.len - 1 - j);
        let mut h = [0; 1024];
        let raw = sa(d, b"a[href]", b"href", &mut h)?;
        let mut abs = [0; 1024];
        let href = absolute(raw, &mut abs)?;
        let path = chapter_path(href)?;
        let q = path.strip_prefix(b"/cn/chapters.html?comic=").unwrap();
        if !q.starts_with(c) || q.get(c.len()) != Some(&b'&') {
            return Err("invalid_chapter_url");
        }
        let mut t = [0; 1024];
        let title = st(d, b"h4", &mut t)?;
        if j > 0 {
            w.put(b",")?
        }
        w.put(br#"{"id":"dbmch:zh:"#)?;
        if !append_json_escaped(payload_buf(), &mut w.n, path) {
            return Err("capacity_exceeded");
        }
        w.put(br#"","mangaId":"dbm:zh:"#)?;
        if !append_json_escaped(payload_buf(), &mut w.n, c) {
            return Err("capacity_exceeded");
        }
        w.put(br#"","title":"#)?;
        w.q(title)?;
        w.put(br#", "chapterNumber":"#)?;
        let num = title
            .strip_prefix(b"Chapter ")
            .and_then(|s| s.iter().position(|x| *x == b':').map(|i| &s[..i]));
        if let Some(v) = num.filter(|v| !v.is_empty() && v.iter().all(u8::is_ascii_digit)) {
            w.put(v)?
        } else {
            w.put(b"null")?
        }
        w.put(br#", "volumeNumber":null,"publishedAt":null,"updatedAt":null,"pageCount":null}"#)?;
    }
    w.put(b"]}")?;
    Ok(w)
}
fn image<'a>(url: &[u8], out: &'a mut [u8]) -> R<&'a [u8]> {
    let doc = fetch(url)?;
    let containers = All::new(doc.0, b"#balloonsimg")?;
    if containers.len != 1 {
        return Err("missing_or_ambiguous_image");
    }
    let d = containers.at(0);
    let bubbles = All::new(d, b".balloon")?;
    if bubbles.len > 0 {
        return Err("unsupported_chapter_overlay");
    }
    let mut src = [0; 4096];
    if let Ok(v) = attr(d, b"src", &mut src) {
        if !v.is_empty() {
            return absolute(v, out);
        }
    }
    let imgs = All::new(d, b"img")?;
    if imgs.len > 1 {
        return Err("ambiguous_image");
    }
    if imgs.len == 1 {
        return absolute(attr(imgs.at(0), b"src", &mut src)?, out);
    }
    let style = attr(d, b"style", &mut src)?;
    let i = find_subslice(style, b"url(").ok_or("missing_image")?;
    let s = &style[i + 4..];
    let end = s.iter().position(|x| *x == b')').ok_or("invalid_image")?;
    let mut v = trim_ascii(&s[..end]);
    if v.len() >= 2
        && ((v[0] == b'\'' && v[v.len() - 1] == b'\'') || (v[0] == b'"' && v[v.len() - 1] == b'"'))
    {
        v = &v[1..v.len() - 1]
    }
    absolute(v, out)
}
fn pages(req: &[u8]) -> R<W> {
    let id = extract_json_string(req, b"chapterId").ok_or("invalid_request")?;
    let p = id.strip_prefix(b"dbmch:zh:").ok_or("invalid_request")?;
    let p = chapter_path(p)?;
    let mut url = [0; 1024];
    let n = absolute(p, &mut url)?.len();
    let doc = fetch(&url[..n])?;
    let all = All::new(doc.0, b".pageslist a[href]")?;
    if all.len == 0 {
        return Err("missing_pages");
    }
    let mut w = W::new();
    w.put(br#"{"chapterId":"#)?;
    w.q(id)?;
    w.put(br#", "pages":["#)?;
    for i in 0..all.len {
        let mut href = [0; 1024];
        let v = attr(all.at(i), b"href", &mut href)?;
        let mut actual = [0; 1024];
        let u = absolute(v, &mut actual)?;
        if !page_url(u) {
            return Err("invalid_page_url");
        }
        let mut direct = [0; 4096];
        let img = image(u, &mut direct)?;
        if !img.starts_with(b"https://www.dragonball-multiverse.com/image.php?") {
            return Err("unsupported_image_url");
        }
        if i > 0 {
            w.put(b",")?
        }
        w.put(br#"{"id":"dbmp:zh:"#)?;
        if !append_json_escaped(payload_buf(), &mut w.n, u) {
            return Err("capacity_exceeded");
        }
        w.put(br#"","index":"#)?;
        w.num(i)?;
        w.put(br#", "image":{"kind":"url","url":"#)?;
        w.q(img)?;
        w.put(b"}}")?;
    }
    w.put(b"]}")?;
    Ok(w)
}
fn done(op: &str, r: R<W>) -> u32 {
    match r {
        Ok(w) => w.end(op),
        Err(e) => write_error(
            op,
            e,
            "DB Multiverse public contract could not be completed",
        ),
    }
}
fn run_search(_req: &[u8]) -> u32 {
    write_error("search", "unsupported", "search is not supported")
}
fn run_get_manga(req: &[u8]) -> u32 {
    done(
        "get_manga",
        manga_id(req).and_then(|c| cards(Some(c), "get_manga")),
    )
}
fn run_get_manga_list(req: &[u8]) -> u32 {
    if extract_json_string(req, b"listingId").unwrap_or(b"popular") != b"popular" {
        return write_error(
            "get_manga_list",
            "unsupported",
            "only popular listing is supported",
        );
    }
    done("get_manga_list", cards(None, "get_manga_list"))
}
fn run_get_chapters(req: &[u8]) -> u32 {
    done("get_chapters", chapters(req))
}
fn run_get_pages(req: &[u8]) -> u32 {
    done("get_pages", pages(req))
}
fn run_get_listings(_req: &[u8]) -> u32 {
    response_buffer().write_success(
        "get_listings",
        br#"{"listings":[{"id":"popular","name":"Popular","supportsPaging":false}]}"#,
    )
}
fn run_get_image_request(req: &[u8]) -> u32 {
    let Some(url) = extract_json_string(req, b"url") else {
        return write_error("get_image_request", "invalid_request", "missing image url");
    };
    if !url.starts_with(b"https://www.dragonball-multiverse.com/image.php?")
        && !url.starts_with(b"https://www.dragonball-multiverse.com/imgs/read/")
    {
        return write_error(
            "get_image_request",
            "invalid_url",
            "only canonical image URLs are supported",
        );
    }
    let mut abs = [0; 4096];
    let r=absolute(url,&mut abs).and_then(|u|{let mut w=W::new();w.put(br#"{"url":"#)?;w.q(u)?;w.put(br#", "headers":{"Referer":"https://www.dragonball-multiverse.com/","Origin":"https://www.dragonball-multiverse.com"}}"#)?;Ok(w)});
    done("get_image_request", r)
}
fn run_get_home(_req: &[u8]) -> u32 {
    write_error("get_home", "unsupported", "not supported")
}
fn run_get_filters(_req: &[u8]) -> u32 {
    write_error("get_filters", "unsupported", "not supported")
}
fn run_get_settings(_req: &[u8]) -> u32 {
    write_error("get_settings", "unsupported", "not supported")
}
koma_source_sdk::koma_source_exports!("dbmultiverse_zh");
