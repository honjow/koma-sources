#![no_std]
use koma_source_sdk::json_utils::*;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{build_get_request, decode_json_body_into, host};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.oldmasterq.zh-hant.koma",
    name: "老夫子繁中・授權關聯站",
    version: "0.1.0",
    api_version: "0.2",
    language: "zh-Hant",
    author: "Koma",
    description: "Public Traditional Chinese comic strips from the independently operated licensed Oldmasterq.com affiliate. Individual creators are unspecified.",
    content_rating: "unknown",
};
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: true,
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
koma_source_sdk::koma_source_buffers! {payload:1024*1024-1024,http_out:1024*1024,body:1024*1024,http_req:4096,scratch:8192,}
koma_source_sdk::koma_source_helpers!();
type R<T> = Result<T, &'static str>;
fn member<'a>(v: &'a [u8], k: &[u8]) -> R<&'a [u8]> {
    json_object_member(v, k).ok_or("parse_error")
}
fn string<'a>(v: &'a [u8], k: &[u8]) -> R<&'a [u8]> {
    let x = member(v, k)?;
    if x.len() < 2 || x[0] != b'"' || x[x.len() - 1] != b'"' {
        return Err("parse_error");
    };
    Ok(&x[1..x.len() - 1])
}
fn number(v: &[u8]) -> R<usize> {
    if v.is_empty() {
        return Err("parse_error");
    };
    let mut n = 0usize;
    for b in v {
        if !b.is_ascii_digit() {
            return Err("parse_error");
        };
        n = n
            .checked_mul(10)
            .and_then(|n| n.checked_add((b - b'0') as usize))
            .ok_or("parse_error")?;
    }
    Ok(n)
}
fn raw_string(v: &[u8], out: &mut [u8]) -> R<usize> {
    let mut i = 0;
    let mut c = 0;
    while i < v.len() {
        let b = v[i];
        i += 1;
        if b != b'\\' {
            if b < 32 || !write_bytes(out, &mut c, &[b]) {
                return Err("parse_error");
            };
            continue;
        }
        if i == v.len() {
            return Err("parse_error");
        };
        let e = v[i];
        i += 1;
        match e {
            b'"' | b'\\' | b'/' => {
                if !write_bytes(out, &mut c, &[e]) {
                    return Err("capacity_exceeded");
                }
            }
            b'n' | b'r' | b't' | b'b' | b'f' => {
                let b = match e {
                    b'n' => 10,
                    b'r' => 13,
                    b't' => 9,
                    b'b' => 8,
                    _ => 12,
                };
                if !write_bytes(out, &mut c, &[b]) {
                    return Err("capacity_exceeded");
                }
            }
            b'u' => {
                if i + 4 > v.len() || !v[i..i + 4].iter().all(|b| b.is_ascii_hexdigit()) {
                    return Err("parse_error");
                };
                let mut cp = hex_to_u16(&v[i..i + 4]) as u32;
                i += 4;
                if (0xd800..=0xdbff).contains(&cp) {
                    if i + 6 > v.len()
                        || &v[i..i + 2] != b"\\u"
                        || !v[i + 2..i + 6].iter().all(|b| b.is_ascii_hexdigit())
                    {
                        return Err("parse_error");
                    };
                    let low = hex_to_u16(&v[i + 2..i + 6]) as u32;
                    if !(0xdc00..=0xdfff).contains(&low) {
                        return Err("parse_error");
                    };
                    cp = 0x10000 + ((cp - 0xd800) << 10) + (low - 0xdc00);
                    i += 6;
                } else if (0xdc00..=0xdfff).contains(&cp) {
                    return Err("parse_error");
                };
                let mut b = [0; 4];
                let n = encode_utf8(cp, &mut b);
                if !write_bytes(out, &mut c, &b[..n]) {
                    return Err("capacity_exceeded");
                }
            }
            _ => return Err("parse_error"),
        }
    }
    Ok(c)
}
fn q(p: &mut [u8], c: &mut usize, v: &[u8]) -> R<()> {
    if write_bytes(p, c, b"\"")
        && append_json_unescaped_then_escaped(p, c, v)
        && write_bytes(p, c, b"\"")
    {
        Ok(())
    } else {
        Err("capacity_exceeded")
    }
}
fn w(p: &mut [u8], c: &mut usize, v: &[u8]) -> R<()> {
    if write_bytes(p, c, v) {
        Ok(())
    } else {
        Err("capacity_exceeded")
    }
}
fn n(p: &mut [u8], c: &mut usize, v: usize) -> R<()> {
    if write_usize(p, c, v) {
        Ok(())
    } else {
        Err("capacity_exceeded")
    }
}

fn empty(op: &str) -> u32 {
    let b = b"{\"items\":[],\"page\":{\"hasMore\":false}}";
    payload_buf()[..b.len()].copy_from_slice(b);
    write_success_payload(op, b.len())
}
fn result(op: &str, r: R<usize>) -> u32 {
    match r {
        Ok(n) => write_success_payload(op, n),
        Err(e) => write_error(op, e, "Official public contract unavailable or invalid"),
    }
}
fn page(req: &[u8]) -> R<usize> {
    let args = json_object_member(req, b"args").unwrap_or(req);
    let v = match json_object_member(args, b"page") {
        None => return Ok(1),
        Some(v) => v,
    };
    let v = if v.first() == Some(&b'{') {
        json_object_member(v, b"cursor").unwrap_or(b"null")
    } else {
        v
    };
    if v == b"null" {
        return Ok(1);
    }
    let v = if v.len() >= 2 && v[0] == b'"' && v[v.len() - 1] == b'"' {
        &v[1..v.len() - 1]
    } else {
        v
    };
    let n = number(v).map_err(|_| "invalid_request")?;
    if n == 0 {
        return Err("invalid_request");
    }
    Ok(n)
}

#[link(wasm_import_module = "koma_host")]
extern "C" {
    #[link_name = "html_select_all"]
    fn select_all_raw(desc: i32, sel: *const u8, len: u32, out: *mut u8, cap: u32) -> i32;
}
fn count(doc: host::HtmlDescriptor, sel: &[u8]) -> R<usize> {
    let mut buf = [0; 128];
    let n = unsafe {
        select_all_raw(
            doc.raw(),
            sel.as_ptr(),
            sel.len() as u32,
            buf.as_mut_ptr(),
            buf.len() as u32,
        )
    };
    if n < 0 {
        return Err("html_error");
    }
    for i in 0..core::cmp::min(n as usize, 32) {
        let at = i * 4;
        let d = i32::from_le_bytes([buf[at], buf[at + 1], buf[at + 2], buf[at + 3]]);
        host::html_close(host::HtmlDescriptor::from_raw(d)).map_err(|_| "html_error")?;
    }
    Ok(n as usize)
}
fn text(doc: host::HtmlDescriptor, sel: &[u8], dst: &mut [u8]) -> R<usize> {
    if count(doc, sel)? != 1 {
        return Err("parse_error");
    }
    let node = host::html_select(doc, sel).map_err(|_| "html_error")?;
    let r = host::html_text(node, dst).map_err(|_| "html_error");
    let closed = host::html_close(node).map_err(|_| "html_error");
    closed?;
    r
}
fn attr(doc: host::HtmlDescriptor, sel: &[u8], key: &[u8], dst: &mut [u8]) -> R<usize> {
    // Caller selector requires presence of the same key; Attr never probes absent attributes.
    if count(doc, sel)? != 1 {
        return Err("parse_error");
    }
    let node = host::html_select(doc, sel).map_err(|_| "html_error")?;
    let r = host::html_attr(node, key, dst).map_err(|_| "html_error");
    let closed = host::html_close(node).map_err(|_| "html_error");
    closed?;
    r
}

const BASE: &[u8] = b"https://www.oldmasterq.com/";
const LIST: &[u8] = b"https://www.oldmasterq.com/comics/";
const MEDIA: &[u8] = b"https://pub-ea4ffe6130c34b07a02c682abfde808a.r2.dev/strips/";
static mut IDS: [usize; 25] = [0; 25];
static mut TITLES: [[u8; 256]; 25] = [[0; 256]; 25];
static mut TL: [usize; 25] = [0; 25];
static mut COVERS: [[u8; 256]; 25] = [[0; 256]; 25];
static mut CL: [usize; 25] = [0; 25];
static mut IMAGE: [u8; 256] = [0; 256];
static mut IL: usize = 0;
fn fetch(u: &[u8]) -> R<usize> {
    let n = build_get_request(http_req_buf(), u, Some(BASE), &[]).ok_or("capacity_exceeded")?;
    let n = host::http_request(&http_req_buf()[..n], http_out()).map_err(|_| "network_error")?;
    if extract_json_number(&http_out()[..n], b"status") != Some(b"200") {
        return Err("http_error");
    }
    decode_json_body_into(&http_out()[..n], body_buf()).map_err(|_| "network_error")
}
fn path_id(v: &[u8]) -> R<usize> {
    let raw = v
        .strip_prefix(b"/comics/")
        .and_then(|x| x.strip_suffix(b"/"))
        .ok_or("invalid_request")?;
    idnum(raw)
}
fn idnum(v: &[u8]) -> R<usize> {
    if v.is_empty() || v.len() > 7 || v[0] == b'0' {
        return Err("invalid_request");
    }
    let id = number(v).map_err(|_| "invalid_request")?;
    if id == 0 {
        return Err("invalid_request");
    }
    Ok(id)
}
fn media(v: &[u8]) -> R<()> {
    let f = v.strip_prefix(MEDIA).ok_or("wrong_owner")?;
    let stem = f
        .strip_suffix(b".jpg")
        .or_else(|| f.strip_suffix(b".png"))
        .or_else(|| f.strip_suffix(b".gif"))
        .ok_or("wrong_owner")?;
    if stem.is_empty()
        || stem.len() > 100
        || !stem
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || matches!(*b, b'_' | b'-'))
    {
        return Err("wrong_owner");
    }
    Ok(())
}
fn identifier(p: &mut [u8], c: &mut usize, prefix: &[u8], id: usize) -> R<()> {
    w(p, c, prefix)?;
    n(p, c, id)
}
fn mid(p: &mut [u8], c: &mut usize, id: usize) -> R<()> {
    identifier(p, c, b"omq:zh-hant:strip:", id)
}
fn route(p: &mut [u8], c: &mut usize, id: usize) -> R<()> {
    w(p, c, b"https://www.oldmasterq.com/comics/")?;
    n(p, c, id)?;
    w(p, c, b"/")
}
fn find(id: usize) -> R<usize> {
    for i in 0..25 {
        if unsafe { IDS[i] } == id {
            return Ok(i);
        }
    }
    Err("not_found")
}
fn sid(v: &[u8], prefix: &[u8]) -> R<usize> {
    idnum(v.strip_prefix(prefix).ok_or("invalid_request")?)
}
fn owner(req: &[u8], id: usize) -> R<()> {
    if let Some(v) = extract_json_string(req, b"mangaUrl") {
        let mut b = [0; 256];
        let n = raw_string(v, &mut b)?;
        let mut u = [0; 256];
        let mut c = 0;
        route(&mut u, &mut c, id)?;
        if &b[..n] != &u[..c] {
            return Err("invalid_request");
        }
    }
    if let Some(v) = extract_json_string(req, b"mangaId") {
        if sid(v, b"omq:zh-hant:strip:")? != id {
            return Err("invalid_request");
        }
    }
    Ok(())
}
fn catalogue() -> R<()> {
    let size = fetch(LIST)?;
    let doc = host::html_parse(&body_buf()[..size]).map_err(|_| "html_error")?;
    let r = (|| {
        if count(doc, b"#main-content .comics-feature-item")? != 1
            || count(doc, b"#main-content .comics-thumbnail-list-item")? != 24
        {
            return Err("catalogue_changed");
        }
        for i in 0..25 {
            let mut root = [0; 256];
            let mut c = 0;
            if i == 0 {
                w(&mut root, &mut c, b"#main-content .comics-feature-item")?;
            } else {
                w(
                    &mut root,
                    &mut c,
                    b"#main-content .comics-thumbnail-list-item:nth-child(",
                )?;
                n(&mut root, &mut c, i)?;
                w(&mut root, &mut c, b")")?;
            }
            let len = c;
            let mut b = [0; 1024];
            w(
                &mut root,
                &mut c,
                if i == 0 {
                    b" > a.feature-strip-thumbnail-container[href]"
                } else {
                    b" > a.comic-strip-thumbnail[href]"
                },
            )?;
            let n = attr(doc, &root[..c], b"href", &mut b)?;
            let id = path_id(&b[..n]).map_err(|_| "wrong_owner")?;
            for j in 0..i {
                if unsafe { IDS[j] } == id {
                    return Err("duplicate_identity");
                }
            }
            unsafe {
                IDS[i] = id;
            }
            c = len;
            w(&mut root, &mut c, b" .title-ch")?;
            let n = text(doc, &root[..c], &mut b)?;
            let title = trim_ascii(&b[..n]);
            if title.is_empty() || title.len() > 256 || core::str::from_utf8(title).is_err() {
                return Err("parse_error");
            }
            unsafe {
                TITLES[i][..title.len()].copy_from_slice(title);
                TL[i] = title.len();
            }
            c = len;
            w(&mut root, &mut c, b" > a > img[data-src]")?;
            let k = if count(doc, &root[..c])? == 1 {
                b"data-src".as_slice()
            } else {
                c = len;
                w(&mut root, &mut c, b" > a > img[src]:not([data-src])")?;
                b"src".as_slice()
            };
            let n = attr(doc, &root[..c], k, &mut b)?;
            if n > 256 {
                return Err("capacity_exceeded");
            }
            media(&b[..n])?;
            unsafe {
                COVERS[i][..n].copy_from_slice(&b[..n]);
                CL[i] = n;
            }
        }
        Ok(())
    })();
    host::html_close(doc).map_err(|_| "html_error")?;
    r
}
fn current_image(i: usize) -> R<()> {
    let id = unsafe { IDS[i] };
    let mut u = [0; 256];
    let mut c = 0;
    route(&mut u, &mut c, id)?;
    let size = fetch(&u[..c])?;
    let doc = host::html_parse(&body_buf()[..size]).map_err(|_| "html_error")?;
    let r = (|| {
        let mut b = [0; 1024];
        let len_url = attr(doc, b"meta[property='og:url'][content]", b"content", &mut b)?;
        let mut expected = [0; 256];
        let mut c = 0;
        w(&mut expected, &mut c, b"https://oldmasterq.com/comics/")?;
        n(&mut expected, &mut c, id)?;
        w(&mut expected, &mut c, b"/")?;
        if &b[..len_url] != &expected[..c] {
            return Err("wrong_owner");
        }
        let len = attr(
            doc,
            b"meta[property='og:title'][content]",
            b"content",
            &mut b,
        )?;
        c = 0;
        w(&mut expected, &mut c, b"Strip #")?;
        n(&mut expected, &mut c, id)?;
        w(&mut expected, &mut c, b" ")?;
        w(&mut expected, &mut c, unsafe { &TITLES[i][..TL[i]] })?;
        w(&mut expected, &mut c, b" ")?;
        if !b[..len].starts_with(&expected[..c]) {
            return Err("wrong_owner");
        }
        if count(doc, b"article.comics-strip-container")? != 1
            || count(
                doc,
                b"article.comics-strip-container > figure.comic-image-container > img",
            )? != 1
        {
            return Err("incomplete_pages");
        }
        let n = attr(
            doc,
            b"article.comics-strip-container > figure.comic-image-container > img[data-src]",
            b"data-src",
            &mut b,
        )?;
        if n > 256 {
            return Err("capacity_exceeded");
        }
        media(&b[..n])?;
        unsafe {
            IMAGE[..n].copy_from_slice(&b[..n]);
            IL = n;
        }
        Ok(())
    })();
    host::html_close(doc).map_err(|_| "html_error")?;
    r
}
fn book(p: &mut [u8], c: &mut usize, i: usize, detail: bool) -> R<()> {
    let id = unsafe { IDS[i] };
    w(p, c, b"{\"id\":\"")?;
    mid(p, c, id)?;
    w(p, c, b"\",\"title\":")?;
    q(p, c, unsafe { &TITLES[i][..TL[i]] })?;
    w(p, c, b",\"url\":\"")?;
    route(p, c, id)?;
    w(p, c, b"\",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(p, c, unsafe { &COVERS[i][..CL[i]] })?;
    w(p,c,b"},\"language\":\"zh-Hant\",\"authors\":[],\"artists\":[],\"status\":\"unknown\",\"contentRating\":\"unknown\"")?;
    if detail {
        w(p, c, b",\"description\":")?;
        q(
            p,
            c,
            "Oldmasterq.com 為獲授權獨立營運關聯站。本條逐作作者未列；不將所有作品歸同一作者。"
                .as_bytes(),
        )?;
        w(p, c, b",\"links\":[{\"kind\":\"source\",\"url\":\"")?;
        route(p, c, id)?;
        w(p, c, b"\"}]")?;
    }
    w(p, c, b"}")
}
fn listing(query: &[u8]) -> R<usize> {
    catalogue()?;
    let p = payload_buf();
    let mut c = 0;
    let mut wrote = false;
    w(p, &mut c, b"{\"items\":[")?;
    for i in 0..25 {
        if !query.is_empty() && !contains_bytes(unsafe { &TITLES[i][..TL[i]] }, query) {
            continue;
        }
        if wrote {
            w(p, &mut c, b",")?;
        }
        book(p, &mut c, i, false)?;
        wrote = true;
    }
    w(
        p,
        &mut c,
        b"],\"page\":{\"nextCursor\":null,\"hasMore\":false}}",
    )?;
    Ok(c)
}
fn run_search(req: &[u8]) -> u32 {
    let mut b = [0; 512];
    let size = match raw_string(extract_json_string(req, b"query").unwrap_or(b""), &mut b) {
        Ok(n) => n,
        Err(e) => return write_error("search", e, ""),
    };
    match page(req) {
        Ok(1) => result("search", listing(&b[..size])),
        Ok(_) => empty("search"),
        Err(e) => write_error("search", e, ""),
    }
}
fn run_get_manga_list(req: &[u8]) -> u32 {
    if extract_json_string(req, b"listingId") != Some(b"listing:latest") {
        return write_error("get_manga_list", "invalid_request", "");
    }
    match page(req) {
        Ok(1) => result("get_manga_list", listing(b"")),
        Ok(_) => empty("get_manga_list"),
        Err(e) => write_error("get_manga_list", e, ""),
    }
}
fn request_work(req: &[u8], key: &[u8], prefix: &[u8]) -> R<usize> {
    let id = sid(
        extract_json_string(req, key).ok_or("invalid_request")?,
        prefix,
    )?;
    owner(req, id)?;
    catalogue()?;
    find(id)
}
fn run_get_manga(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let i = request_work(req, b"mangaId", b"omq:zh-hant:strip:")?;
        current_image(i)?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"manga\":")?;
        book(p, &mut c, i, true)?;
        w(p, &mut c, b"}")?;
        Ok(c)
    }
    result("get_manga", inner(req))
}
fn run_get_chapters(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let i = request_work(req, b"mangaId", b"omq:zh-hant:strip:")?;
        current_image(i)?;
        let id = unsafe { IDS[i] };
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"chapters\":[{\"id\":\"")?;
        identifier(p, &mut c, b"omqread:zh-hant:strip:", id)?;
        w(p, &mut c, b"\",\"mangaId\":\"")?;
        mid(p, &mut c, id)?;
        w(p, &mut c, b"\",\"title\":")?;
        q(p, &mut c, unsafe { &TITLES[i][..TL[i]] })?;
        w(p, &mut c, b",\"url\":\"")?;
        route(p, &mut c, id)?;
        w(
            p,
            &mut c,
            b"\",\"chapterNumber\":null,\"volumeNumber\":null,\"readingGroup\":\"",
        )?;
        mid(p, &mut c, id)?;
        w(
            p,
            &mut c,
            b"\",\"continuationEligible\":false,\"language\":\"zh-Hant\",\"pageCount\":1}]}",
        )?;
        Ok(c)
    }
    result("get_chapters", inner(req))
}
fn run_get_pages(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let i = request_work(req, b"chapterId", b"omqread:zh-hant:strip:")?;
        current_image(i)?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"pages\":[{\"id\":\"")?;
        identifier(p, &mut c, b"omqpage:zh-hant:strip:", unsafe { IDS[i] })?;
        w(
            p,
            &mut c,
            b":0\",\"index\":0,\"image\":{\"kind\":\"url\",\"url\":",
        )?;
        q(p, &mut c, unsafe { &IMAGE[..IL] })?;
        w(p, &mut c, b"}}]}")?;
        Ok(c)
    }
    result("get_pages", inner(req))
}
fn run_get_image_request(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        let mut url = [0; 512];
        let size = raw_string(
            extract_json_string(req, b"url").ok_or("invalid_request")?,
            &mut url,
        )?;
        let url = &url[..size];
        media(url).map_err(|_| "invalid_request")?;
        let pid = extract_json_string(req, b"pageId");
        let (id, body) = if let Some(v) = pid {
            if let Some(v) = v.strip_prefix(b"omqpage:zh-hant:strip:") {
                (
                    idnum(v.strip_suffix(b":0").ok_or("invalid_request")?)?,
                    true,
                )
            } else {
                (sid(v, b"omq:zh-hant:strip:")?, false)
            }
        } else {
            (0, false)
        };
        if id != 0 {
            owner(req, id)?;
        }
        catalogue()?;
        let i = if id != 0 {
            find(id)?
        } else {
            let mut selected = None;
            for i in 0..25 {
                if url == unsafe { &COVERS[i][..CL[i]] } {
                    if selected.is_some() {
                        return Err("invalid_request");
                    }
                    selected = Some(i);
                }
            }
            selected.ok_or("invalid_request")?
        };
        owner(req, unsafe { IDS[i] })?;
        if body {
            current_image(i)?;
            if url != unsafe { &IMAGE[..IL] } {
                return Err("invalid_request");
            }
        } else if url != unsafe { &COVERS[i][..CL[i]] } {
            return Err("invalid_request");
        }
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"imageRequest\":{\"url\":")?;
        q(p, &mut c, url)?;
        w(p, &mut c, b",\"headers\":{}}}")?;
        Ok(c)
    }
    result("get_image_request", inner(req))
}
fn run_get_listings(_req: &[u8]) -> u32 {
    let b = "{\"listings\":[{\"id\":\"listing:latest\",\"name\":\"最新25條獨立漫畫\"}]}".as_bytes();
    payload_buf()[..b.len()].copy_from_slice(b);
    write_success_payload("get_listings", b.len())
}
fn run_get_home(_req: &[u8]) -> u32 {
    write_error("get_home", "not_implemented", "")
}
fn run_get_filters(_req: &[u8]) -> u32 {
    write_error("get_filters", "not_implemented", "")
}
fn run_get_settings(_req: &[u8]) -> u32 {
    write_error("get_settings", "not_implemented", "")
}
const REQUEST_CAP: usize = 1024 * 1024;
static mut REQUEST: [u8; REQUEST_CAP] = [0; REQUEST_CAP];
fn checked_request(ptr: u32, len: u32) -> Option<&'static [u8]> {
    let cap = core::arch::wasm32::memory_size::<0>().checked_mul(65536)?;
    let start = ptr as usize;
    let end = start.checked_add(len as usize)?;
    if start == 0 || len == 0 || len as usize >= REQUEST_CAP || end > cap {
        return None;
    }
    read_request(ptr, len)
}
#[no_mangle]
pub extern "C" fn koma_source_alloc(len: u32) -> u32 {
    if len == 0 || len as usize >= REQUEST_CAP {
        0
    } else {
        core::ptr::addr_of_mut!(REQUEST) as *mut u8 as u32
    }
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
    response_buffer().write_source_metadata(&SOURCE_INFO, &SOURCE_CAPS)
}
#[no_mangle]
pub extern "C" fn koma_source_free(ptr: u32) {
    response_buffer().free(ptr)
}
macro_rules! export_op {
    ($f:ident,$op:literal,$run:ident) => {
        #[no_mangle]
        pub extern "C" fn $f(ptr: u32, len: u32) -> u32 {
            let req = match checked_request(ptr, len) {
                Some(r) => r,
                None => return write_error($op, "invalid_request", "invalid ABI range"),
            };
            if host::check_cancel() {
                return write_error($op, "cancelled", "");
            }
            let args = match json_object_member(req, b"args") {
                Some(a) if a.first() == Some(&b'{') => a,
                _ => return write_error($op, "invalid_request", "missing args"),
            };
            if string(req, b"operation").ok() != Some($op.as_bytes()) {
                return write_error($op, "invalid_request", "operation mismatch");
            }
            $run(args)
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
