#![no_std]
use koma_source_sdk::json_utils::*;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{build_get_request, decode_json_body_into, host};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.weicomic.ronshaonien.zh-hant.koma",
    name: "創漫繁中・隆少年",
    version: "0.1.2",
    api_version: "0.2",
    language: "zh-Hant",
    author: "Koma",
    description: "Public original Traditional Chinese comic Ronshaonien by ULTIMATE from Weicomic.",
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

const MAIN: &[u8] = b"https://www.weicomic.com/";
const DETAIL: &[u8] = b"https://www.weicomic.com/comicview-shocomic/ronshaonien/";
const XML: &[u8] = b"https://www.weicomic.com/shocomic/ronshaonien.xml";
const MID: &[u8] = b"weicomic:zh-hant:ronshaonien";
const CID: &[u8] = b"weicomicread:zh-hant:ronshaonien:all";
const TITLE: &str = "隆少年";
const COVER: &[u8] = b"https://www.weicomic.com/image/shocomic/ronshaonien.jpg";
const IMAGE_BASE: &[u8] = b"https://www.weicomic.com/shocomic/";
// The existing complete request ABI is the URI budget, not a filename slot.
static mut IMAGE_REQUEST_URI: [u8; REQUEST_CAP] = [0; REQUEST_CAP];
#[link(wasm_import_module = "koma_host")]
extern "C" {
    #[link_name = "html_select_all"]
    fn select_all_raw(desc: i32, sel: *const u8, len: u32, out: *mut u8, cap: u32) -> i32;
}
fn count(doc: host::HtmlDescriptor, sel: &[u8]) -> R<usize> {
    let mut buf = [0; 16];
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
    for i in 0..core::cmp::min(n as usize, 4) {
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

fn fetch(u: &[u8]) -> R<usize> {
    let n = build_get_request(http_req_buf(), u, Some(DETAIL), &[]).ok_or("capacity_exceeded")?;
    let n = host::http_request(&http_req_buf()[..n], http_out()).map_err(|_| "network_error")?;
    let response = &http_out()[..n];
    // A formal transport failure has no HTTP status. Preserve the HTTP guard
    // when a current or legacy status field is present, even if ok is false.
    if json_object_member(response, b"ok") == Some(b"false")
        && json_object_member(response, b"status").is_none()
        && json_object_member(response, b"statusCode").is_none()
    {
        return Err("network_error");
    }
    if extract_json_number(response, b"status") != Some(b"200") {
        return Err("http_error");
    }
    decode_json_body_into(&http_out()[..n], body_buf()).map_err(|_| "network_error")
}
fn catalogue() -> R<()> {
    let n = fetch(MAIN)?;
    let doc = host::html_parse(&body_buf()[..n]).map_err(|_| "html_error")?;
    let r = (|| {
        let mut b = [0; 1024];
        let sel = b"article.comic-card:has(> a[href='comicview-shocomic/ronshaonien/'])";
        let mut target = [0; 256];
        let mut c = 0;
        w(&mut target, &mut c, sel)?;
        w(&mut target, &mut c, b" .comic-title")?;
        let n = text(doc, &target[..c], &mut b)?;
        if trim_ascii(&b[..n]) != TITLE.as_bytes() {
            return Err("wrong_owner");
        }
        c = 0;
        w(&mut target, &mut c, sel)?;
        w(&mut target, &mut c, b" .comic-author")?;
        let n = text(doc, &target[..c], &mut b)?;
        if trim_ascii(&b[..n]) != b"ULTIMATE" {
            return Err("wrong_owner");
        }
        c = 0;
        w(&mut target, &mut c, sel)?;
        w(&mut target, &mut c, b" .zone-badge")?;
        let n = text(doc, &target[..c], &mut b)?;
        if trim_ascii(&b[..n]) != "一般".as_bytes() {
            return Err("wrong_owner");
        }
        let n = attr(
            doc,
            b"a[href='comicview-shocomic/ronshaonien/'] > img[src]",
            b"src",
            &mut b,
        )?;
        if &b[..n] != b"image/shocomic/ronshaonien.jpg" {
            return Err("wrong_owner");
        }
        Ok(())
    })();
    host::html_close(doc).map_err(|_| "html_error")?;
    r
}
// Bounded source-local parser for this original UTF8 XML grammar. No XML entities,
// DTD, external resources, JS eval, conversion-to-HTML, or synthetic HTML response.
fn section<'a>(v: &'a [u8], tag: &str) -> R<(&'a [u8], &'a [u8])> {
    let v = trim_ascii(v);
    let mut a = [0; 96];
    let mut c = 0;
    w(&mut a, &mut c, b"<")?;
    w(&mut a, &mut c, tag.as_bytes())?;
    w(&mut a, &mut c, b">")?;
    let v = v.strip_prefix(&a[..c]).ok_or("parse_error")?;
    c = 0;
    w(&mut a, &mut c, b"</")?;
    w(&mut a, &mut c, tag.as_bytes())?;
    w(&mut a, &mut c, b">")?;
    let at = find_subslice(v, &a[..c]).ok_or("parse_error")?;
    Ok((&v[..at], &v[at + c..]))
}
fn tag<'a>(v: &'a [u8], name: &str) -> R<([(&'a [u8], &'a [u8]); 8], usize, &'a [u8])> {
    let v = trim_ascii(v);
    let mut marker = [0; 64];
    let mut c = 0;
    w(&mut marker, &mut c, b"<")?;
    w(&mut marker, &mut c, name.as_bytes())?;
    let v = v.strip_prefix(&marker[..c]).ok_or("parse_error")?;
    if !v.first().map_or(false, |x| x.is_ascii_whitespace()) {
        return Err("parse_error");
    }
    let mut at = 0;
    let mut arr = [(&b""[..], &b""[..]); 8];
    let mut n = 0;
    loop {
        while at < v.len() && v[at].is_ascii_whitespace() {
            at += 1;
        }
        if v.get(at..at + 2) == Some(b"/>") {
            return Ok((arr, n, &v[at + 2..]));
        }
        if n == 8 {
            return Err("parse_error");
        }
        let start = at;
        while at < v.len() && !v[at].is_ascii_whitespace() && v[at] != b'=' {
            if matches!(v[at], b'<' | b'>' | b'/' | b'\'' | b'"') {
                return Err("parse_error");
            }
            at += 1;
        }
        if at == start {
            return Err("parse_error");
        }
        let key = &v[start..at];
        for row in &arr[..n] {
            if row.0 == key {
                return Err("parse_error");
            }
        }
        while at < v.len() && v[at].is_ascii_whitespace() {
            at += 1;
        }
        if v.get(at) != Some(&b'=') {
            return Err("parse_error");
        }
        at += 1;
        while at < v.len() && v[at].is_ascii_whitespace() {
            at += 1;
        }
        if v.get(at) != Some(&b'"') {
            return Err("parse_error");
        }
        at += 1;
        let start = at;
        while at < v.len() && v[at] != b'"' {
            if matches!(v[at], b'<' | b'>' | b'&') {
                return Err("parse_error");
            }
            at += 1;
        }
        if at == v.len() {
            return Err("parse_error");
        }
        arr[n] = (key, &v[start..at]);
        n += 1;
        at += 1;
        if v.get(at..at + 2) != Some(b"/>") && !v.get(at).map_or(false, |b| b.is_ascii_whitespace())
        {
            return Err("parse_error");
        }
    }
}
fn xa<'a>(a: &[(&'a [u8], &'a [u8])], key: &str) -> R<&'a [u8]> {
    a.iter()
        .find(|x| x.0 == key.as_bytes())
        .map(|x| x.1)
        .ok_or("parse_error")
}
struct Pages<'a> {
    data: &'a [u8],
    count: usize,
}
// Preserve declared bytes; reject normalization that could leave this work.
fn page_path(v: &[u8]) -> R<&[u8]> {
    let tail = v.strip_prefix(b"comic/ronshaonien/").ok_or("wrong_owner")?;
    if v.iter().any(|b| *b <= 32 || *b == 127 || *b == b'\\') { return Err("wrong_owner"); }
    let end = tail.iter().position(|b| matches!(*b, b'?' | b'#')).unwrap_or(tail.len());
    for segment in tail[..end].split(|b| *b == b'/') {
        if segment.is_empty() { return Err("wrong_owner"); }
        let mut at = 0;
        let mut dots = 0;
        let mut only_dots = true;
        while at < segment.len() {
            let mut b = segment[at];
            at += 1;
            if b == b'%' {
                if at + 2 > segment.len() || !segment[at..at+2].iter().all(u8::is_ascii_hexdigit) { return Err("wrong_owner"); }
                b = (hex_to_u16(&[b'0', b'0', segment[at], segment[at+1]]) & 255) as u8;
                at += 2;
            }
            if b <= 32 || b == 127 || matches!(b, b'/' | b'\\') { return Err("wrong_owner"); }
            if b == b'.' { dots += 1; } else { only_dots = false; }
        }
        if only_dots && (dots == 1 || dots == 2) { return Err("wrong_owner"); }
    }
    Ok(v)
}
fn next_page(data: &[u8]) -> R<(&[u8], &[u8])> {
    let (a, count, rest) = tag(data, "頁")?;
    if count != 5 { return Err("parse_error"); }
    // The official viewer reads title and image; the other values are unused.
    // Keep the old five-attribute grammar without freezing their values.
    let _ = xa(&a[..count], "名稱")?;
    let _ = xa(&a[..count], "超連結")?;
    let _ = xa(&a[..count], "寬度")?;
    let _ = xa(&a[..count], "高度")?;
    Ok((page_path(xa(&a[..count], "圖片")?)?, rest))
}
fn page_url(p: &mut [u8], c: &mut usize, path: &[u8]) -> R<()> {
    w(p, c, b"\"")?;
    w(p, c, IMAGE_BASE)?;
    if !append_json_unescaped_then_escaped(p, c, path) { return Err("capacity_exceeded"); }
    w(p, c, b"\"")
}
fn parse_xml(v: &[u8]) -> R<Pages<'_>> {
    core::str::from_utf8(v).map_err(|_| "parse_error")?;
    if contains_bytes(v, b"<!") || contains_bytes(v, b"&") {
        return Err("unsupported_xml");
    }
    let v = v.strip_prefix(&[239, 187, 191]).unwrap_or(v);
    let v = trim_ascii(v)
        .strip_prefix(b"<?xml version=\"1.0\" encoding=\"utf-8\"?>")
        .ok_or("parse_error")?;
    let (root, end) = section(v, "漫畫")?;
    if !trim_ascii(end).is_empty() {
        return Err("parse_error");
    }
    let (head, rest) = section(root, "漫畫標題區")?;
    let (a, n, end) = tag(head, "漫畫")?;
    if n != 5 || !trim_ascii(end).is_empty() {
        return Err("parse_error");
    }
    if xa(&a[..n], "名")? != TITLE.as_bytes()
        || xa(&a[..n], "作者")? != b"ULTIMATE"
        || xa(&a[..n], "集數")? != "全".as_bytes()
        || xa(&a[..n], "作者介紹")? != b"http://www.weicomic.com/g07.htm#ultimate"
        || xa(&a[..n], "留言位置")? != b"http://forums.weicomic.com/comicview-shocomic/ronshaonien/"
    {
        return Err("wrong_owner");
    }
    let (data, end) = section(rest, "漫畫內容區")?;
    if !trim_ascii(end).is_empty() { return Err("parse_error"); }
    let mut rest = data;
    let mut count = 0usize;
    while !trim_ascii(rest).is_empty() {
        let (_, tail) = next_page(rest)?;
        count = count.checked_add(1).ok_or("capacity_exceeded")?;
        rest = tail;
    }
    if count == 0 { return Err("incomplete_pages"); }
    Ok(Pages { data, count })
}

fn current_pages() -> R<Pages<'static>> {
    let n = fetch(DETAIL)?;
    let h = &body_buf()[..n];
    if !contains_bytes(
        h,
        b"makeRequest('../..//shocomic/ronshaonien.xml','/shocomic');",
    ) || !contains_bytes(h, TITLE.as_bytes())
    {
        return Err("wrong_owner");
    }
    let n = fetch(XML)?;
    parse_xml(&body_buf()[..n])
}
fn owner(req: &[u8]) -> R<()> {
    if let Some(raw) = extract_json_string(req, b"mangaUrl") {
        let mut b = [0; 2048];
        let n = raw_string(raw, &mut b)?;
        if &b[..n] != DETAIL {
            return Err("invalid_request");
        }
    }
    Ok(())
}
fn book(p: &mut [u8], c: &mut usize, detail: bool) -> R<()> {
    w(p, c, b"{\"id\":")?;
    q(p, c, MID)?;
    w(p, c, b",\"title\":")?;
    q(p, c, TITLE.as_bytes())?;
    w(p, c, b",\"url\":")?;
    q(p, c, DETAIL)?;
    w(p, c, b",\"cover\":{\"kind\":\"url\",\"url\":")?;
    q(p, c, COVER)?;
    w(p,c,b"},\"language\":\"zh-Hant\",\"authors\":[\"ULTIMATE\"],\"artists\":[],\"status\":\"unknown\",\"contentRating\":\"unknown\"")?;
    if detail {
        w(p, c, b",\"description\":")?;
        q(
            p,
            c,
            "創意漫畫大亂鬥公開繁中漫畫《隆少年》，作者 ULTIMATE，原XML集數為全。".as_bytes(),
        )?;
        w(p, c, b",\"links\":[{\"kind\":\"source\",\"url\":")?;
        q(p, c, DETAIL)?;
        w(p, c, b"}]")?;
    }
    w(p, c, b"}")
}
fn listing() -> R<usize> {
    catalogue()?;
    let p = payload_buf();
    let mut c = 0;
    w(p, &mut c, b"{\"items\":[")?;
    book(p, &mut c, false)?;
    w(p, &mut c, b"],\"page\":{\"hasMore\":false}}")?;
    Ok(c)
}
fn run_search(req: &[u8]) -> u32 {
    let raw = extract_json_string(req, b"query").unwrap_or(b"");
    let mut b = [0; 1024];
    let len = match raw_string(raw, &mut b) {
        Ok(n) => n,
        Err(e) => return write_error("search", e, ""),
    };
    if len > 0
        && !contains_bytes(
            "隆少年 创漫 創漫 Ronshaonien ULTIMATE".as_bytes(),
            &b[..len],
        )
    {
        return empty("search");
    }
    match page(req) {
        Ok(1) => result("search", listing()),
        Ok(_) => empty("search"),
        Err(e) => write_error("search", e, ""),
    }
}
fn run_get_manga_list(req: &[u8]) -> u32 {
    if extract_json_string(req, b"listingId") != Some(b"listing:official") {
        return write_error("get_manga_list", "invalid_request", "");
    }
    match page(req) {
        Ok(1) => result("get_manga_list", listing()),
        Ok(_) => empty("get_manga_list"),
        Err(e) => write_error("get_manga_list", e, ""),
    }
}
fn run_get_manga(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        if extract_json_string(req, b"mangaId") != Some(MID) {
            return Err("not_found");
        }
        owner(req)?;
        catalogue()?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"manga\":")?;
        book(p, &mut c, true)?;
        w(p, &mut c, b"}")?;
        Ok(c)
    }
    result("get_manga", inner(req))
}
fn run_get_chapters(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        if extract_json_string(req, b"mangaId") != Some(MID) {
            return Err("not_found");
        }
        owner(req)?;
        catalogue()?;
        let pages = current_pages()?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"chapters\":[{\"id\":")?;
        q(p, &mut c, CID)?;
        w(p, &mut c, b",\"mangaId\":")?;
        q(p, &mut c, MID)?;
        w(p, &mut c, b",\"title\":")?;
        q(p, &mut c, TITLE.as_bytes())?;
        w(p, &mut c, b",\"url\":")?;
        q(p, &mut c, DETAIL)?;
        w(
            p,
            &mut c,
            b",\"chapterNumber\":null,\"volumeNumber\":null,\"readingGroup\":",
        )?;
        q(p, &mut c, MID)?;
        w(
            p,
            &mut c,
            b",\"continuationEligible\":false,\"language\":\"zh-Hant\",\"pageCount\":",
        )?;
        n(p, &mut c, pages.count)?;
        w(p, &mut c, b"}]}")?;
        Ok(c)
    }
    result("get_chapters", inner(req))
}
fn run_get_pages(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        if extract_json_string(req, b"chapterId") != Some(CID) {
            return Err("not_found");
        }
        if let Some(id) = extract_json_string(req, b"mangaId") {
            if id != MID {
                return Err("not_found");
            }
        }
        owner(req)?;
        let pages = current_pages()?;
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"pages\":[")?;
        let mut rest = pages.data;
        for i in 0..pages.count {
            let (uri, tail) = next_page(rest)?;
            rest = tail;
            if i > 0 {
                w(p, &mut c, b",")?;
            }
            w(
                p,
                &mut c,
                b"{\"id\":\"weicomicpage:zh-hant:ronshaonien:all:",
            )?;
            n(p, &mut c, i)?;
            w(p, &mut c, b"\",\"index\":")?;
            n(p, &mut c, i)?;
            w(p, &mut c, b",\"image\":{\"kind\":\"url\",\"url\":")?;
            page_url(p, &mut c, uri)?;
            w(p, &mut c, b"}}")?;
        }
        w(p, &mut c, b"]}")?;
        Ok(c)
    }
    result("get_pages", inner(req))
}
fn run_get_image_request(req: &[u8]) -> u32 {
    fn inner(req: &[u8]) -> R<usize> {
        owner(req)?;
        let raw = extract_json_string(req, b"url").ok_or("invalid_request")?;
        let b = unsafe { &mut IMAGE_REQUEST_URI[..] };
        let len = raw_string(raw, b)?;
        let u = &b[..len];
        let pid = extract_json_string(req, b"pageId");
        if u == COVER {
            if let Some(id) = pid {
                if id != MID {
                    return Err("invalid_request");
                }
            }
            catalogue()?;
        } else {
            let id = pid.ok_or("invalid_request")?;
            let raw = id
                .strip_prefix(b"weicomicpage:zh-hant:ronshaonien:all:")
                .ok_or("invalid_request")?;
            if raw.is_empty() || (raw.len() > 1 && raw[0] == b'0') {
                return Err("invalid_request");
            }
            let index = number(raw).map_err(|_| "invalid_request")?;
            let relative = u.strip_prefix(IMAGE_BASE).ok_or("invalid_request")?;
            page_path(relative).map_err(|_| "invalid_request")?;
            let pages = current_pages()?;
            if index >= pages.count { return Err("invalid_request"); }
            let mut rest = pages.data;
            for i in 0..=index {
                let (uri, tail) = next_page(rest)?;
                if i == index && relative != uri { return Err("invalid_request"); }
                rest = tail;
            }
        }
        let p = payload_buf();
        let mut c = 0;
        w(p, &mut c, b"{\"imageRequest\":{\"url\":")?;
        q(p, &mut c, u)?;
        w(p, &mut c, b",\"headers\":{}}}")?;
        Ok(c)
    }
    result("get_image_request", inner(req))
}
fn run_get_listings(_req: &[u8]) -> u32 {
    let b = "{\"listings\":[{\"id\":\"listing:official\",\"name\":\"創漫一般作品\"}]}".as_bytes();
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
