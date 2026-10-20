//! Guazimanhua public HTML source. See NOTICE for the upstream parser reference.
#![no_std]
use koma_source_sdk::host::{
    html_attr, html_close, html_parse, html_select, html_text, http_request, HtmlDescriptor,
};
use koma_source_sdk::json_utils::{
    append_json_escaped, extract_json_number, extract_json_string, request_page_number,
    write_bytes, write_url_encoded, write_usize,
};
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use koma_source_sdk::{build_get_request, fetch_error_code, FetchError};
const BASE: &[u8] = b"https://www.guazimanhua.com";
const UA: &[u8] = b"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.guazimanhua.koma",
    name: "瓜子漫画",
    version: "0.1.3",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "Guazimanhua public HTML manga source.",
    content_rating: "safe",
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
    credentials: false,
    image_request: true,
};
koma_source_sdk::koma_source_buffers! { payload: 1024*1024, http_out: 2*1024*1024, body: 2*1024*1024, http_req: 8192, scratch: 16384, }
koma_source_sdk::koma_source_helpers!();
#[link(wasm_import_module = "koma_host")]
extern "C" {
    #[link_name = "html_select_all"]
    fn select_all(d: i32, s: *const u8, n: u32, o: *mut u8, c: u32) -> i32;
}
struct Doc(HtmlDescriptor);
impl Drop for Doc {
    fn drop(&mut self) {
        let _ = html_close(self.0);
    }
}
// One bounded handle buffer is owned by one live Selection at a time.
// Avoid a 32 KiB local/return array under the production interpreter stack.
static mut HANDLES: [i32; 8192] = [-1; 8192];
static mut HANDLES_BORROWED: bool = false;
struct Selection {
    len: usize,
}
impl Selection {
    fn new(d: HtmlDescriptor, s: &[u8]) -> Result<Self, &'static str> {
        unsafe {
            if HANDLES_BORROWED {
                return Err("nested HTML handle selection");
            }
            HANDLES_BORROWED = true;
        }
        let mut out = Self { len: 0 };
        let n = unsafe {
            select_all(
                d.raw(),
                s.as_ptr(),
                s.len() as u32,
                core::ptr::addr_of_mut!(HANDLES) as *mut u8,
                (8192 * 4) as u32,
            )
        };
        if n < 0 {
            return Err("html selection failed or exceeded capacity");
        }
        out.len = (n as usize).min(8192);
        if n as usize >= 8192 {
            return Err("html selection exceeded capacity");
        }
        Ok(out)
    }
    fn at(&self, i: usize) -> HtmlDescriptor {
        unsafe { core::mem::transmute(HANDLES[i]) }
    }
}
impl Drop for Selection {
    fn drop(&mut self) {
        for i in 0..self.len {
            unsafe {
                if HANDLES[i] >= 0 {
                    let _ = html_close(self.at(i));
                }
                HANDLES[i] = -1;
            }
        }
        unsafe {
            HANDLES_BORROWED = false;
        }
    }
}
fn child_text<'a>(d: HtmlDescriptor, s: &[u8], o: &'a mut [u8]) -> Option<&'a [u8]> {
    let e = Doc(html_select(d, s).ok()?);
    let n = html_text(e.0, o).ok()?;
    Some(trim_ascii(&o[..n]))
}
fn child_attr<'a>(d: HtmlDescriptor, s: &[u8], a: &[u8], o: &'a mut [u8]) -> Option<&'a [u8]> {
    let e = Doc(html_select(d, s).ok()?);
    let n = html_attr(e.0, a, o).ok()?;
    Some(&o[..n])
}
fn fetch(url: &[u8]) -> Result<Doc, FetchError> {
    let n = build_get_request(http_req_buf(), url, Some(BASE), &[(b"User-Agent", UA)])
        .ok_or(FetchError::Network)?;
    let n = http_request(&http_req_buf()[..n], http_out()).map_err(|_| FetchError::Network)?;
    // HTTP transport success does not mean a successful site response.
    let response = &http_out()[..n];
    if let Some(code) = extract_json_number(response, b"statusCode")
        .or_else(|| extract_json_number(response, b"status"))
    {
        let status = code.iter().fold(0u32, |n, b| {
            n.saturating_mul(10).saturating_add((b - b'0') as u32)
        });
        match status {
            200..=299 => {}
            404 => return Err(FetchError::NotFound),
            429 => return Err(FetchError::RateLimit),
            400..=499 => return Err(FetchError::ClientError),
            500..=599 => return Err(FetchError::ServerError),
            _ => return Err(FetchError::Network),
        }
    }
    let n = decode_json_body(response)?;
    html_parse(&body_buf()[..n])
        .map(Doc)
        .map_err(|_| FetchError::ClientError)
}
// Decode JSON string contents before URL encoding (including UTF-16 pairs).
fn decode_string<'a>(b: &[u8], out: &'a mut [u8]) -> Option<&'a [u8]> {
    let mut i = 0;
    let mut n = 0;
    while i < b.len() {
        if b[i] != b'\\' {
            if !write_bytes(out, &mut n, &b[i..i + 1]) {
                return None;
            }
            i += 1;
            continue;
        }
        i += 1;
        let c = *b.get(i)?;
        i += 1;
        match c {
            b'"' | b'\\' | b'/' => {
                if !write_bytes(out, &mut n, &[c]) {
                    return None;
                }
            }
            b'n' | b'r' | b't' | b'b' | b'f' => {
                let c = match c {
                    b'n' => 10,
                    b'r' => 13,
                    b't' => 9,
                    b'b' => 8,
                    _ => 12,
                };
                if !write_bytes(out, &mut n, &[c]) {
                    return None;
                }
            }
            b'u' => {
                let mut cp = hex4(b.get(i..i + 4)?)?;
                i += 4;
                if (0xd800..=0xdbff).contains(&cp) {
                    if b.get(i..i + 2)? != b"\\u" {
                        return None;
                    }
                    i += 2;
                    let lo = hex4(b.get(i..i + 4)?)?;
                    i += 4;
                    if !(0xdc00..=0xdfff).contains(&lo) {
                        return None;
                    }
                    cp = 0x10000 + ((cp - 0xd800) << 10) + (lo - 0xdc00)
                } else if (0xdc00..=0xdfff).contains(&cp) {
                    return None;
                }
                let mut tmp = [0u8; 4];
                let len = koma_source_sdk::json_utils::encode_utf8(cp, &mut tmp);
                if !write_bytes(out, &mut n, &tmp[..len]) {
                    return None;
                }
            }
            _ => return None,
        }
    }
    Some(&out[..n])
}
fn hex4(b: &[u8]) -> Option<u32> {
    let mut n = 0;
    for &c in b {
        let v = match c {
            b'0'..=b'9' => c - b'0',
            b'a'..=b'f' => c - b'a' + 10,
            b'A'..=b'F' => c - b'A' + 10,
            _ => return None,
        };
        n = (n << 4) | v as u32
    }
    Some(n)
}
fn decimal(b: &[u8]) -> bool {
    !b.is_empty() && b.len() <= 12 && b.iter().all(u8::is_ascii_digit)
}
fn numeric_id<'a>(b: &'a [u8], prefix: &[u8]) -> Option<&'a [u8]> {
    let id = b.strip_prefix(prefix)?;
    if decimal(id) {
        Some(id)
    } else {
        None
    }
}
fn path_id<'a>(b: &'a [u8], path: &[u8]) -> Option<&'a [u8]> {
    let b = b.strip_prefix(BASE).unwrap_or(b);
    numeric_id(b, path)
}
fn chapter_parts(b: &[u8]) -> Option<(&[u8], &[u8])> {
    let b = b.strip_prefix(b"guazi:chapter:")?;
    let p = b.iter().position(|&x| x == b':')?;
    let (m, c) = (&b[..p], &b[p + 1..]);
    if decimal(m) && decimal(c) {
        Some((m, c))
    } else {
        None
    }
}
fn allowed_image(u: &[u8]) -> bool {
    (u.starts_with(b"https://img.guazicdn.com/") || u.starts_with(b"https://mhfm5tel.cdndm5.com/"))
        && !u.iter().any(|&b| b <= 32 || b == 127 || b == b'\\')
}
fn site_url(path: &[u8], id: &[u8], out: &mut [u8]) -> usize {
    let mut n = 0;
    if write_bytes(out, &mut n, BASE)
        && write_bytes(out, &mut n, path)
        && write_bytes(out, &mut n, id)
    {
        n
    } else {
        0
    }
}
fn quoted(out: &mut [u8], n: &mut usize, b: &[u8]) -> bool {
    write_bytes(out, n, b"\"") && append_json_escaped(out, n, b) && write_bytes(out, n, b"\"")
}
fn page_tail(out: &mut [u8], n: &mut usize, more: bool, page: usize) -> bool {
    if !write_bytes(out, n, br#"],"page":{"nextCursor":"#) {
        return false;
    }
    if more {
        write_bytes(out, n, b"\"")
            && write_usize(out, n, page + 1)
            && write_bytes(out, n, br#"","hasMore":true}}"#)
    } else {
        write_bytes(out, n, b"null,\"hasMore\":false}}")
    }
}
fn run_list(req: &[u8], op: &str) -> u32 {
    let page = request_page_number(req);
    let raw = if op == "search" {
        extract_json_string(req, b"query").unwrap_or(b"")
    } else {
        b""
    };
    let mut query = [0u8; 4096];
    let q = match decode_string(raw, &mut query) {
        Some(x) => x,
        None => return write_error(op, "invalid_request", "invalid or overlong query"),
    };
    let listing = extract_json_string(req, b"listingId").unwrap_or(b"popular");
    if op != "search" && listing != b"popular" && listing != b"latest" {
        return write_error(op, "invalid_request", "unknown listing");
    }
    let mut url = [0u8; 8192];
    let mut n = 0;
    let sort = if listing == b"latest" {
        b"update" as &[u8]
    } else {
        b"hits"
    };
    if !(write_bytes(&mut url, &mut n, BASE)
        && write_bytes(&mut url, &mut n, b"/category.php?sort=")
        && write_bytes(&mut url, &mut n, sort)
        && write_bytes(&mut url, &mut n, b"&page=")
        && write_usize(&mut url, &mut n, page)
        && write_bytes(&mut url, &mut n, b"&keyword=")
        && write_url_encoded(&mut url, &mut n, q))
    {
        return write_error(op, "invalid_request", "query too long");
    }
    let d = match fetch(&url[..n]) {
        Ok(d) => d,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error(op, c, m);
        }
    };
    let cards = match Selection::new(d.0, b"article.card") {
        Ok(x) => x,
        Err(m) => return write_error(op, "parse_error", m),
    };
    let out = payload_buf();
    let mut n = 0;
    let mut count = 0;
    if !write_bytes(out, &mut n, b"{\"items\":[") {
        return write_error(op, "internal_error", "overflow");
    }
    for i in 0..cards.len {
        let mut href = [0u8; 512];
        let mut title = [0u8; 4096];
        let mut cover = [0u8; 4096];
        let h = match child_attr(cards.at(i), b"a.cover-wrap", b"href", &mut href) {
            Some(x) => x,
            None => return write_error(op, "parse_error", "missing comic link"),
        };
        let id = match path_id(h, b"/comic.php?id=") {
            Some(x) => x,
            None => return write_error(op, "parse_error", "invalid comic link"),
        };
        let t = match child_text(cards.at(i), b"h3 a", &mut title) {
            Some(x) if !x.is_empty() => x,
            _ => return write_error(op, "parse_error", "missing title"),
        };
        let u = match child_attr(cards.at(i), b"img.cover", b"src", &mut cover) {
            Some(x) if allowed_image(x) => x,
            _ => return write_error(op, "parse_error", "unsupported cover URL"),
        };
        if count > 0 && !write_bytes(out, &mut n, b",") {
            return write_error(op, "internal_error", "overflow");
        }
        let ok = write_bytes(out, &mut n, br#"{"id":"guazi:manga:"#)
            && write_bytes(out, &mut n, id)
            && write_bytes(out, &mut n, br#"","title":"#)
            && quoted(out, &mut n, t)
            && write_bytes(out, &mut n, br#", "cover":{"kind":"url","url":"#)
            && quoted(out, &mut n, u)
            && write_bytes(
                out,
                &mut n,
                br#"},"authors":[],"status":"unknown","contentRating":"safe"}"#,
            );
        if !ok {
            return write_error(op, "internal_error", "overflow");
        }
        count += 1;
    }
    drop(cards);
    let pager = match Selection::new(d.0, b"nav.pager a") {
        Ok(x) => x,
        Err(m) => return write_error(op, "parse_error", m),
    };
    let mut more = false;
    for i in 0..pager.len {
        let mut b = [0u8; 128];
        if let Ok(n) = html_text(pager.at(i), &mut b) {
            if trim_ascii(&b[..n]) == b">" {
                more = true
            }
        }
    }
    if !page_tail(out, &mut n, more, page) {
        return write_error(op, "internal_error", "overflow");
    }
    write_success_payload(op, n)
}
fn run_search(r: &[u8]) -> u32 {
    run_list(r, "search")
}
fn run_get_manga_list(r: &[u8]) -> u32 {
    run_list(r, "get_manga_list")
}
fn run_get_manga(req: &[u8]) -> u32 {
    let id = match extract_json_string(req, b"mangaId").and_then(|b| numeric_id(b, b"guazi:manga:"))
    {
        Some(x) => x,
        None => return write_error("get_manga", "invalid_request", "invalid manga ID"),
    };
    let mut url = [0u8; 512];
    let ulen = site_url(b"/comic.php?id=", id, &mut url);
    let d = match fetch(&url[..ulen]) {
        Ok(x) => x,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_manga", c, m);
        }
    };
    let mut title = [0u8; 4096];
    let mut cover = [0u8; 4096];
    let mut desc = [0u8; 16384];
    let mut author = [0u8; 4096];
    let mut meta = [0u8; 512];
    let t = match child_text(d.0, b"div.mobile-comic-title", &mut title) {
        Some(x) if !x.is_empty() => x,
        _ => return write_error("get_manga", "parse_error", "missing title"),
    };
    let u = match child_attr(d.0, b"img.mobile-comic-cover", b"src", &mut cover) {
        Some(x) if allowed_image(x) => x,
        _ => return write_error("get_manga", "parse_error", "unsupported cover URL"),
    };
    let de = child_text(d.0, b"p.mobile-comic-desc", &mut desc).unwrap_or(b"");
    let a = child_text(d.0, b"div.cinema-strip > div:first-child b", &mut author).unwrap_or(b"");
    let me = child_text(d.0, b"p.mobile-comic-meta", &mut meta).unwrap_or(b"");
    let status = if koma_source_sdk::json_utils::contains_bytes(me, "完结".as_bytes()) {
        b"completed" as &[u8]
    } else if koma_source_sdk::json_utils::contains_bytes(me, "连载".as_bytes()) {
        b"ongoing"
    } else {
        b"unknown"
    };
    let out = payload_buf();
    let mut n = 0;
    let ok = write_bytes(out, &mut n, br#"{"manga":{"id":"guazi:manga:"#)
        && write_bytes(out, &mut n, id)
        && write_bytes(out, &mut n, br#"","title":"#)
        && quoted(out, &mut n, t)
        && write_bytes(out, &mut n, br#", "cover":{"kind":"url","url":"#)
        && quoted(out, &mut n, u)
        && write_bytes(out, &mut n, br#"},"description":"#)
        && quoted(out, &mut n, de)
        && write_bytes(out, &mut n, br#", "authors":["#)
        && quoted(out, &mut n, a)
        && write_bytes(out, &mut n, br#"],"status":"#)
        && quoted(out, &mut n, status)
        && write_bytes(
            out,
            &mut n,
            br#", "contentRating":"safe","tags":[],"links":[{"kind":"source","url":"#,
        )
        && quoted(out, &mut n, &url[..ulen])
        && write_bytes(out, &mut n, b"}]}}");
    if !ok {
        return write_error("get_manga", "internal_error", "overflow");
    }
    write_success_payload("get_manga", n)
}
// Only explicit leading chapter ordinals participate in ordinary continuation.
// Do not infer chapter numbers from array positions, site IDs, or release dates.
fn chapter_ordinal(title: &[u8]) -> Option<usize> {
    let title = trim_ascii(title);
    let title = title.strip_prefix("第".as_bytes()).unwrap_or(title);
    let digits = title.iter().take_while(|b| b.is_ascii_digit()).count();
    if digits == 0 || digits > 9 {
        return None;
    }
    let remainder = &title[digits..];
    let label = if remainder.is_empty() || remainder[0].is_ascii_whitespace() {
        trim_ascii(remainder)
    } else if let Some(label) = remainder
        .strip_prefix("话".as_bytes())
        .or_else(|| remainder.strip_prefix("話".as_bytes()))
        .or_else(|| remainder.strip_prefix("章".as_bytes()))
    {
        trim_ascii(label)
    } else {
        return None;
    };
    // A numbered announcement is still a special entry, not a story chapter.
    for prefix in [
        "公告",
        "停更",
        "致歉声明",
        "封面",
        "人物档案",
        "预告",
        "前情回顾",
    ] {
        if label.starts_with(prefix.as_bytes()) {
            return None;
        }
    }
    Some(
        title[..digits]
            .iter()
            .fold(0usize, |n, b| n * 10 + (b - b'0') as usize),
    )
}
fn run_get_chapters(req: &[u8]) -> u32 {
    let id = match extract_json_string(req, b"mangaId").and_then(|b| numeric_id(b, b"guazi:manga:"))
    {
        Some(x) => x,
        None => return write_error("get_chapters", "invalid_request", "invalid manga ID"),
    };
    let mut url = [0u8; 512];
    let n = site_url(b"/comic.php?id=", id, &mut url);
    let d = match fetch(&url[..n]) {
        Ok(x) => x,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_chapters", c, m);
        }
    };
    let rows = match Selection::new(
        d.0,
        b"section.mobile-comic-all-chapters div.mobile-chapter-grid a",
    ) {
        Ok(x) => x,
        Err(m) => return write_error("get_chapters", "parse_error", m),
    };
    if rows.len == 0 {
        return write_error("get_chapters", "parse_error", "chapter directory missing");
    }
    let out = payload_buf();
    let mut n = 0;
    if !write_bytes(out, &mut n, b"{\"items\":[") {
        return write_error("get_chapters", "internal_error", "overflow");
    }
    for i in 0..rows.len {
        let mut href = [0u8; 512];
        let mut title = [0u8; 4096];
        let h = match html_attr(rows.at(i), b"href", &mut href) {
            Ok(n) => &href[..n],
            Err(_) => return write_error("get_chapters", "parse_error", "missing chapter link"),
        };
        let c = match path_id(h, b"/chapter.php?id=") {
            Some(x) => x,
            None => return write_error("get_chapters", "parse_error", "invalid chapter link"),
        };
        let t = match html_text(rows.at(i), &mut title) {
            Ok(n) => trim_ascii(&title[..n]),
            Err(_) => return write_error("get_chapters", "parse_error", "missing chapter title"),
        };
        if i > 0 && !write_bytes(out, &mut n, b",") {
            return write_error("get_chapters", "internal_error", "overflow");
        }
        let ok = write_bytes(out, &mut n, br#"{"id":"guazi:chapter:"#)
            && write_bytes(out, &mut n, id)
            && write_bytes(out, &mut n, b":")
            && write_bytes(out, &mut n, c)
            && write_bytes(out, &mut n, br#"","mangaId":"guazi:manga:"#)
            && write_bytes(out, &mut n, id)
            && write_bytes(out, &mut n, br#"","title":"#)
            && quoted(out, &mut n, t)
            && write_bytes(out, &mut n, br#", "chapterNumber":"#)
            && if let Some(number) = chapter_ordinal(t) {
                write_usize(out, &mut n, number)
                    && write_bytes(
                        out,
                        &mut n,
                        br#", "readingGroup":"chapter","continuationEligible":true"#,
                    )
            } else {
                write_bytes(
                    out,
                    &mut n,
                    br#"null,"readingGroup":"special","continuationEligible":false"#,
                )
            }
            && write_bytes(out, &mut n, br#", "volumeNumber":null,"language":"zh"}"#);
        if !ok {
            return write_error("get_chapters", "internal_error", "overflow");
        }
    }
    if !page_tail(out, &mut n, false, 1) {
        return write_error("get_chapters", "internal_error", "overflow");
    }
    write_success_payload("get_chapters", n)
}
fn run_get_pages(req: &[u8]) -> u32 {
    let (m, c) = match extract_json_string(req, b"chapterId").and_then(chapter_parts) {
        Some(x) => x,
        None => return write_error("get_pages", "invalid_request", "invalid chapter ID"),
    };
    let mut url = [0u8; 512];
    let n = site_url(b"/chapter.php?id=", c, &mut url);
    let d = match fetch(&url[..n]) {
        Ok(x) => x,
        Err(e) => {
            let (c, m) = fetch_error_code(e);
            return write_error("get_pages", c, m);
        }
    };
    let imgs = match Selection::new(d.0, b"section.reader-images img") {
        Ok(x) => x,
        Err(m) => return write_error("get_pages", "parse_error", m),
    };
    if imgs.len == 0 {
        return write_error(
            "get_pages",
            "source_error",
            "no publicly available page images",
        );
    }
    let out = payload_buf();
    let mut n = 0;
    if !write_bytes(out, &mut n, b"{\"pages\":[") {
        return write_error("get_pages", "internal_error", "overflow");
    }
    for i in 0..imgs.len {
        let mut buf = [0u8; 8192];
        let u = match html_attr(imgs.at(i), b"src", &mut buf) {
            Ok(n) if allowed_image(&buf[..n]) => &buf[..n],
            _ => return write_error("get_pages", "parse_error", "unsupported page URL"),
        };
        if i > 0 && !write_bytes(out, &mut n, b",") {
            return write_error("get_pages", "internal_error", "overflow");
        }
        let ok = write_bytes(out, &mut n, br#"{"id":"guazi:page:"#)
            && write_bytes(out, &mut n, m)
            && write_bytes(out, &mut n, b":")
            && write_bytes(out, &mut n, c)
            && write_bytes(out, &mut n, b":")
            && write_usize(out, &mut n, i)
            && write_bytes(out, &mut n, br#"","index":"#)
            && write_usize(out, &mut n, i)
            && write_bytes(out, &mut n, br#", "image":{"kind":"url","url":"#)
            && quoted(out, &mut n, u)
            && write_bytes(out, &mut n, b"}}");
        if !ok {
            return write_error("get_pages", "internal_error", "overflow");
        }
    }
    if !write_bytes(out, &mut n, b"]}") {
        return write_error("get_pages", "internal_error", "overflow");
    }
    write_success_payload("get_pages", n)
}
fn run_get_image_request(req: &[u8]) -> u32 {
    let url = match extract_json_string(req, b"url") {
        Some(x) if allowed_image(x) => x,
        _ => {
            return write_error(
                "get_image_request",
                "invalid_request",
                "unsupported image URL",
            )
        }
    };
    let mut referer = [0u8; 512];
    let mut r = 0;
    write_bytes(&mut referer, &mut r, BASE);
    write_bytes(&mut referer, &mut r, b"/");
    if let Some(p) = extract_json_string(req, b"pageId").filter(|p| {
        !p.is_empty()
            && !url.starts_with(b"https://img.guazicdn.com/th/comics/cover/")
            && !url.starts_with(b"https://mhfm5tel.cdndm5.com/")
    }) {
        let p = match p.strip_prefix(b"guazi:page:") {
            Some(x) => x,
            None => return write_error("get_image_request", "invalid_request", "invalid page ID"),
        };
        let mut it = p.split(|&b| b == b':');
        let m = it.next().unwrap_or(b"");
        let c = it.next().unwrap_or(b"");
        let i = it.next().unwrap_or(b"");
        if !decimal(m) || !decimal(c) || !decimal(i) || it.next().is_some() {
            return write_error("get_image_request", "invalid_request", "invalid page ID");
        }
        r = site_url(b"/chapter.php?id=", c, &mut referer);
    }
    let out = payload_buf();
    let mut n = 0;
    let ok = write_bytes(out, &mut n, br#"{"url":"#)
        && quoted(out, &mut n, url)
        && write_bytes(out, &mut n, br#", "headers":{"Referer":"#)
        && quoted(out, &mut n, &referer[..r])
        && write_bytes(out, &mut n, br#", "User-Agent":"#)
        && quoted(out, &mut n, UA)
        && write_bytes(out, &mut n, b"}}");
    if !ok {
        return write_error("get_image_request", "internal_error", "overflow");
    }
    write_success_payload("get_image_request", n)
}
fn run_get_listings(_r: &[u8]) -> u32 {
    let b="{\"listings\":[{\"id\":\"popular\",\"name\":\"热门漫画\"},{\"id\":\"latest\",\"name\":\"最新更新\"}]}".as_bytes();
    payload_buf()[..b.len()].copy_from_slice(b);
    write_success_payload("get_listings", b.len())
}
fn run_get_home(_r: &[u8]) -> u32 {
    write_error("get_home", "unimplemented", "not supported")
}
fn run_get_filters(_r: &[u8]) -> u32 {
    write_error("get_filters", "unimplemented", "not supported")
}
fn run_get_settings(_r: &[u8]) -> u32 {
    write_error("get_settings", "unimplemented", "not supported")
}
koma_source_sdk::koma_source_exports!("guazimanhua");
