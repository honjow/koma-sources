#![no_std]
// Independent Rust port of Keiyoushi BiliManga, Apache-2.0,
// ref 16cde44564d45c3fa9c14371b4d9152606a9821d.
// The guest search capability isolates the site-issued cookies from Web and account sessions.
use koma_source_sdk::{
    host,
    json_utils::*,
    source::{SourceCapabilities, SourceInfo},
};
use koma_source_sdk::source_protocol as protocol;
mod schema;
const BASE: &[u8] = b"https://www.bilimanga.net";
// This is Mihon's default mobile browser profile. Its global UA can be customized
// by its user; the Koma source uses the verified default with matching client hints.
const UA: &[u8] = b"Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/149.0.0.0 Mobile Safari/537.36";
const DOCUMENT_ACCEPT: &[u8] = b"text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7";
const IMAGE_ACCEPT: &[u8] = b"image/avif,image/webp,image/apng,image/svg+xml,image/*,*/*;q=0.8";
const CHROME_BRANDS: &[u8] = br#""Google Chrome";v="149", "Chromium";v="149", "Not)A;Brand";v="24""#;
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "net.bilimanga.koma",
    name: "嗶哩漫畫",
    version: "0.1.1",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "嗶哩漫畫（bilimanga.net）。包含不同內容分級的作品。",
    content_rating: "suggestive",
};
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: true,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: true,
    manga_list: true,
    home: false,
    filters: true,
    settings: true,
    image_request: true,
    credentials: false,
};
koma_source_sdk::koma_source_buffers! {payload:1024*1024-1024,http_out:2*1024*1024,body:2*1024*1024,http_req:16384,scratch:8192,}
koma_source_sdk::koma_source_helpers!();
type R<T> = Result<T, &'static str>;
#[link(wasm_import_module = "koma_host")]
extern "C" {
    #[link_name = "html_select_all"]
    fn all(d: i32, s: *const u8, n: u32, o: *mut i32, c: u32) -> i32;
}
struct Node(host::HtmlDescriptor);
impl Drop for Node {
    fn drop(&mut self) {
        let _ = host::html_close(self.0);
    }
}
impl Node {
    fn parse(b: &[u8]) -> R<Self> {
        host::html_parse(b).map(Self).map_err(|_| "parse_error")
    }
    fn select(&self, s: &[u8]) -> R<Self> {
        host::html_select(self.0, s)
            .map(Self)
            .map_err(|_| "parse_error")
    }
    fn text<'a>(&self, b: &'a mut [u8]) -> R<&'a [u8]> {
        let n = host::html_text(self.0, b).map_err(|_| "capacity_exceeded")?;
        if n >= b.len() {
            return Err("capacity_exceeded");
        };
        Ok(trim_ascii(&b[..n]))
    }
    fn attr<'a>(&self, k: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
        let n = host::html_attr(self.0, k, b).map_err(|_| "parse_error")?;
        if n >= b.len() {
            return Err("capacity_exceeded");
        };
        Ok(&b[..n])
    }
}
struct Nodes {
    ids: [i32; 8192],
    n: usize,
}
impl Nodes {
    fn get(d: &Node, s: &[u8]) -> R<Self> {
        let mut a = Self {
            ids: [0; 8192],
            n: 0,
        };
        let n = unsafe {
            all(
                d.0.raw(),
                s.as_ptr(),
                s.len() as u32,
                a.ids.as_mut_ptr(),
                (a.ids.len() * 4) as u32,
            )
        };
        if n < 0 || n as usize >= a.ids.len() {
            return Err("capacity_exceeded");
        };
        a.n = n as usize;
        Ok(a)
    }
    fn at(&self, i: usize) -> core::mem::ManuallyDrop<Node> {
        core::mem::ManuallyDrop::new(Node(host::HtmlDescriptor::from_raw(self.ids[i])))
    }
}
impl Drop for Nodes {
    fn drop(&mut self) {
        for id in &self.ids[..self.n] {
            let _ = host::html_close(host::HtmlDescriptor::from_raw(*id));
        }
    }
}
fn w(o: &mut [u8], n: &mut usize, v: &[u8]) -> R<()> {
    if write_bytes(o, n, v) {
        Ok(())
    } else {
        Err("capacity_exceeded")
    }
}
fn q(o: &mut [u8], n: &mut usize, v: &[u8]) -> R<()> {
    w(o, n, b"\"")?;
    if !append_json_escaped(o, n, v) {
        return Err("capacity_exceeded");
    };
    w(o, n, b"\"")
}
fn num(o: &mut [u8], n: &mut usize, v: usize) -> R<()> {
    if write_usize(o, n, v) {
        Ok(())
    } else {
        Err("capacity_exceeded")
    }
}
fn args(r: &[u8]) -> &[u8] {
    json_object_member(r, b"args").unwrap_or(r)
}
fn string<'a>(r: &'a [u8], k: &[u8]) -> R<&'a [u8]> {
    protocol::raw_string(r, k)?.ok_or("invalid_request")
}
fn positive(s: &[u8]) -> R<usize> {
    if s.is_empty() || s[0] == b'0' {
        return Err("invalid_request");
    };
    let mut n = 0usize;
    for c in s {
        if !c.is_ascii_digit() {
            return Err("invalid_request");
        };
        n = n
            .checked_mul(10)
            .and_then(|n| n.checked_add((c - b'0') as usize))
            .ok_or("invalid_request")?;
    }
    Ok(n)
}
fn mid(r: &[u8]) -> R<&[u8]> {
    let id = string(args(r), b"mangaId")?;
    let m = id.strip_prefix(b"bilimanga:").ok_or("invalid_request")?;
    positive(m)?;
    Ok(m)
}
fn path<'a>(prefix: &[u8], id: &[u8], suffix: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    let mut n = 0;
    w(b, &mut n, prefix)?;
    w(b, &mut n, id)?;
    w(b, &mut n, suffix)?;
    Ok(&b[..n])
}
fn manga_path<'a>(id: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    path(b"/detail/", id, b".html", b)
}
fn absolute<'a>(p: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
    let mut n = 0;
    if p.starts_with(b"https://") {
        w(b, &mut n, p)?
    } else if p.starts_with(b"//") {
        w(b, &mut n, b"https:")?;
        w(b, &mut n, p)?
    } else if p.starts_with(b"/") {
        w(b, &mut n, BASE)?;
        w(b, &mut n, p)?
    } else {
        return Err("parse_error");
    };
    Ok(&b[..n])
}
enum DocumentRequest<'a> {
    Direct,
    Neighbor,
    Guest { cookies: &'a [&'a [u8]], revalidate: bool },
}
fn write_document_request(req: &mut [u8], n: &mut usize, p: &[u8], mode: DocumentRequest<'_>) -> R<()> {
    let mut u = [0; 8192];
    let u = path(BASE, p, b"", &mut u)?;
    w(req, n, b"{\"method\":\"GET\",\"url\":")?;
    q(req, n, u)?;
    w(req, n, b",\"headers\":{\"User-Agent\":")?;
    q(req, n, UA)?;
    w(req, n, b",\"Referer\":\"https://www.bilimanga.net/\",\"Accept\":")?;
    q(req, n, DOCUMENT_ACCEPT)?;
    w(req, n, b",\"Sec-CH-UA\":")?;
    q(req, n, CHROME_BRANDS)?;
    w(
        req,
        n,
        br#","Accept-Language":"zh","Sec-CH-UA-Mobile":"?1","Sec-CH-UA-Platform":"\"Android\"","Sec-Fetch-Site":"same-origin","Sec-Fetch-Mode":"navigate","Sec-Fetch-User":"?1","Sec-Fetch-Dest":"document","Upgrade-Insecure-Requests":"1""#,
    )?;
    match mode {
        DocumentRequest::Direct => w(req, n, b",\"Cookie\":\"night=1\"")?,
        // The adjacent reader probe does not add the source's night cookie.
        DocumentRequest::Neighbor => (),
        DocumentRequest::Guest { cookies, revalidate } => {
            if 7 + cookies.iter().map(|c| c.len() + 2).sum::<usize>() > 8192 {
                return Err("search_ticket_missing");
            }
            w(req, n, b",\"Cookie\":\"night=1")?;
            for cookie in cookies {
                w(req, n, b"; ")?;
                if !append_json_escaped(req, n, cookie) {
                    return Err("capacity_exceeded");
                }
            }
            w(req, n, b"\"")?;
            if revalidate {
                w(req, n, b",\"Cache-Control\":\"no-cache\"")?;
            }
            return w(req, n, b"},\"guestSearch\":true}");
        }
    }
    w(req, n, b"}}")
}
fn send_http(request: &[u8]) -> R<&'static [u8]> {
    if host::check_cancel() {
        return Err("cancelled");
    }
    let n = host::http_request(request, http_out()).map_err(|_| "network_error")?;
    Ok(&http_out()[..n])
}
fn parse_document(body: &[u8]) -> R<Node> {
    let root = Node::parse(body)?;
    if root.select(b".aui-ver-form").is_ok() {
        return Err("challenge_required");
    }
    Ok(root)
}
fn fetch_document(p: &[u8], mode: DocumentRequest<'_>) -> R<(&'static [u8], usize)> {
    let req = http_req_buf();
    let mut n = 0;
    write_document_request(req, &mut n, p, mode)?;
    let response = send_http(&req[..n])?;
    let size = protocol::body(response, body_buf())?;
    Ok((response, size))
}
fn fetch(p: &[u8]) -> R<Node> {
    let (_, size) = fetch_document(p, DocumentRequest::Direct)?;
    parse_document(&body_buf()[..size])
}
fn require_request_policy(r: &[u8]) -> R<()> {
    if protocol::feature(r, b"source_rate_limit_bilimanga_v1")
        || protocol::feature(r, b"source_request_limits_v1") {
        Ok(())
    } else {
        Err("unsupported_source_rate_limit")
    }
}

fn result(op: &str, r: R<usize>) -> u32 {
    match r {
        Ok(n) => write_success_payload(op, n),
        Err("unsupported_source_rate_limit") => write_error(
            op,
            "unsupported",
            "目前的應用程式版本不支援嗶哩漫畫的請求限速，更新後即可使用",
        ),
        Err("unsupported_search_session") => write_error(
            op,
            "unsupported",
            "目前的應用程式版本不支援網站的游客搜尋，請更新後重試",
        ),
        Err("desktop_reader") => write_error(
            op,
            "chapter_unavailable",
            "網站將這次請求識別為電腦端，未提供章節圖片。可在手機原生瀏覽器開啟原網頁",
        ),
        Err("search_ticket_missing") => write_error(
            op,
            "parse_error",
            "未取得網站的游客搜尋憑證，請稍後重試",
        ),
        Err(c) => write_error(
            op,
            c,
            match c {
                "not_found" => "网站未提供该作品或章节，请在网页确认是否已移除",
                "forbidden" => "网站拒绝了这次请求，请在网页确认访问状态",
                "auth_required" => "网站要求登录后访问",
                "network_error" => "無法連線到原網站，請稍後重試",
                "rate_limit" => "请求过于频繁，请稍后重试",
                "challenge_required" => "网站要求完成验证，请先打开网页",
                "chapter_unavailable" => "网页没有提供正文图片，可能受访问权限限制，请在网页确认",
                "chapter_navigation_missing" => {
                    "章节跳转信息不完整，请更新目录后重试或打开原网页"
                }
                _ => "书源处理失败，请稍后重试或在原网页核对",
            },
        ),
    }
}
fn digits(s: &[u8], out: &mut [u8]) -> R<usize> {
    let mut n = 0;
    let mut i = 0;
    while i < s.len() {
        if s[i..].len() >= 3 && s[i..i + 2] == [0xef, 0xbc] && (0x90..=0x99).contains(&s[i + 2]) {
            w(out, &mut n, &[b'0' + s[i + 2] - 0x90])?;
            i += 3
        } else {
            w(out, &mut n, &s[i..i + 1])?;
            i += 1
        }
    }
    Ok(n)
}
fn detail_id(p: &[u8]) -> R<&[u8]> {
    let p = p.strip_prefix(BASE).unwrap_or(p);
    let id = p
        .strip_prefix(b"/detail/")
        .and_then(|v| v.strip_suffix(b".html"))
        .ok_or("parse_error")?;
    positive(id).map_err(|_| "parse_error")?;
    Ok(id)
}
fn canonical_detail_id(url: &[u8]) -> R<&[u8]> {
    let path = url.strip_prefix(BASE).filter(|path| path.starts_with(b"/detail/")).ok_or("invalid_request")?;
    let route = path.split(|b| matches!(b, b'?' | b'#')).next().ok_or("invalid_request")?;
    detail_id(route)
}
const SITE_COVER_PREFIX: &[u8] = b"https://www.bilimanga.net/files/cartoon/image/";
const CDN_COVER_PREFIX: &[u8] = b"https://c.motiezw.com/";

fn cover_owner(raw: &[u8]) -> R<&[u8]> {
    let mut parts = raw.splitn(3, |c| *c == b'/');
    let bucket = parts.next().ok_or("invalid_request")?;
    if bucket.is_empty() || !bucket.iter().all(u8::is_ascii_digit) {
        return Err("invalid_request");
    }
    let owner = parts.next().ok_or("invalid_request")?;
    positive(owner)?;
    let mut parts = parts.next().ok_or("invalid_request")?.splitn(2, |c| *c == b'?');
    let file = parts.next().ok_or("invalid_request")?;
    let base = file.strip_suffix(b".jpg").ok_or("invalid_request")?;
    let variant = base.strip_prefix(owner).ok_or("identity_mismatch")?;
    if !matches!(variant, b"" | b"s" | b"l") {
        return Err("identity_mismatch");
    }
    if let Some(stamp) = parts.next() {
        if stamp.is_empty() || !stamp.iter().all(u8::is_ascii_digit) {
            return Err("invalid_request");
        }
    }
    Ok(owner)
}

// The CDN rejects image clients on some networks. The site's first-party route
// serves the same cover path, including its size variant and cache timestamp.
fn public_cover_url<'a>(url: &'a [u8], owner: &[u8], out: &'a mut [u8]) -> R<&'a [u8]> {
    let raw = url.strip_prefix(SITE_COVER_PREFIX)
        .or_else(|| url.strip_prefix(CDN_COVER_PREFIX))
        .ok_or("invalid_request")?;
    if cover_owner(raw)? != owner {
        return Err("identity_mismatch");
    }
    if url.starts_with(SITE_COVER_PREFIX) {
        return Ok(url);
    }
    path(SITE_COVER_PREFIX, raw, b"", out)
}

fn body_image(url: &[u8]) -> R<()> {
    let name = url.strip_prefix(b"https://i.motiezw.com/img/")
        .or_else(|| url.strip_prefix(b"https://w.motiezw.com/img/"))
        .and_then(|v| v.strip_suffix(b".avif").or_else(|| v.strip_suffix(b".jpg")))
        .ok_or("invalid_request")?;
    if name.len() != 32 || !name.iter().all(u8::is_ascii_hexdigit) {
        return Err("invalid_request");
    }
    Ok(())
}

fn write_cover(out: &mut [u8], n: &mut usize, url: &[u8], owner: &[u8], direct_headers: bool) -> R<()> {
    w(out, n, b"{\"kind\":\"url\",\"url\":")?;
    let mut resolved = [0; 8192];
    let public_url = public_cover_url(url, owner, &mut resolved);
    q(out, n, public_url.unwrap_or(url))?;
    if direct_headers && public_url.is_ok() {
        write_public_image_headers(out, n)?;
        w(out, n, b"}")?;
    }
    w(out, n, b"}")
}

fn write_public_image_headers(out: &mut [u8], n: &mut usize) -> R<()> {
    w(out, n, b",\"headers\":{\"Referer\":\"https://www.bilimanga.net/\",\"Accept\":")?;
    q(out, n, IMAGE_ACCEPT)?;
    w(out, n, b",\"User-Agent\":")?;
    q(out, n, UA)
}

fn card(o: &mut [u8], n: &mut usize, id: &[u8], title: &[u8], cover: &[u8], direct_headers: bool) -> R<()> {
    if title.is_empty() {
        return Err("parse_error");
    };
    let mut cb = [0; 4096];
    let cover = absolute(cover, &mut cb)?;
    w(o, n, b"{\"id\":\"bilimanga:")?;
    w(o, n, id)?;
    w(o, n, b"\",\"title\":")?;
    q(o, n, title)?;
    w(o, n, b",\"cover\":")?;
    write_cover(o, n, cover, id, direct_headers)?;
    w(o, n, b",\"contentRating\":\"suggestive\"}")
}
fn list(r: &[u8], search: bool) -> R<usize> {
    require_request_policy(r)?;
    let a = args(r);
    let mut qb = [0; 16384];
    let query = trim_ascii(protocol::text(a, b"query", &mut qb)?.unwrap_or(b""));
    if search && !query.is_empty() {
        // Public canonical work links resolve directly, before visitor search.
        if let Ok(id) = canonical_detail_id(query) {
            let out = payload_buf();
            let mut n = 0;
            w(out, &mut n, b"{\"items\":[")?;
            write_detail(r, id, out, &mut n)?;
            w(out, &mut n, b"],\"page\":{\"nextCursor\":null,\"hasMore\":false}}")?;
            return Ok(n);
        }
        return keyword_search(r, query);
    }
    let page = request_page_number(r);
    let mut p = [0; 2048];
    let mut pn = 0;
    let filters = json_object_member(a, b"filters").unwrap_or(b"{}");
    if search || extract_json_string(a, b"listingId") == Some(b"filter") {
        w(&mut p, &mut pn, b"/filter/")?;
        for (i, key) in [
            b"sort".as_slice(),
            b"theme",
            b"status",
            b"anime",
            b"region",
            b"type",
            b"time",
            b"novel",
        ]
        .iter()
        .enumerate()
        {
            if i > 0 {
                w(&mut p, &mut pn, b"_")?
            };
            w(&mut p, &mut pn, schema::filter_value(filters, key)?)?
        }
        w(&mut p, &mut pn, b"_")?;
        num(&mut p, &mut pn, page)?;
        w(&mut p, &mut pn, b"_0_")?;
        w(&mut p, &mut pn, schema::filter_value(filters, b"year")?)?;
        w(&mut p, &mut pn, b"_")?;
        w(&mut p, &mut pn, schema::filter_value(filters, b"award")?)?;
        w(&mut p, &mut pn, b".html")?;
    } else {
        w(&mut p, &mut pn, b"/top/")?;
        let latest = extract_json_string(a, b"listingId") == Some(b"latest");
        let settings = json_object_member(r, b"settings").unwrap_or(b"{}");
        let top = if latest {
            b"lastupdate".as_slice()
        } else {
            schema::popular(settings)?
        };
        w(&mut p, &mut pn, top)?;
        w(&mut p, &mut pn, b"/")?;
        num(&mut p, &mut pn, page)?;
        w(&mut p, &mut pn, b".html")?;
    }
    let doc = fetch(&p[..pn])?;
    let kind = if p[..pn].starts_with(b"/filter/") { ListKind::Filter } else { ListKind::Ranked };
    write_manga_list(r, &doc, kind)
}

enum ListKind { Ranked, Filter, Search }
fn write_manga_list(r: &[u8], doc: &Node, kind: ListKind) -> R<usize> {
    let cards = Nodes::get(doc, b".book-layout")?;
    let page = request_page_number(r);
    let direct_headers = protocol::feature(r, b"source_public_cover_user_agent_v1");
    let out = payload_buf();
    let mut n = 0;
    w(out, &mut n, b"{\"items\":[")?;
    for i in 0..cards.n {
        let c = cards.at(i);
        let mut hb = [0; 256];
        let h = c.attr(b"href", &mut hb)?;
        let id = detail_id(h)?;
        let im = c.select(b"img")?;
        let mut tb = [0; 2048];
        let title = im.attr(b"alt", &mut tb)?;
        let mut cb = [0; 4096];
        let cover = im.attr(b"data-src", &mut cb)?;
        if i > 0 {
            w(out, &mut n, b",")?
        };
        card(out, &mut n, id, title, cover, direct_headers)?;
    }
    let more = match kind {
        ListKind::Ranked => cards.n == 50,
        ListKind::Filter => {
            if let (Ok(last), Ok(cur)) = (doc.select(b"#pagelink > .last"), doc.select(b"#pagelink > strong")) {
                let mut total = [0; 32];
                let mut current = [0; 32];
                positive(last.text(&mut total)?).unwrap_or(0) > positive(cur.text(&mut current)?).unwrap_or(0)
            } else {
                false
            }
        }
        ListKind::Search => {
            if let Ok(label) = doc.select(b"#pagelink > span") {
                let mut text = [0; 256];
                has_search_next(label.text(&mut text)?)
            } else {
                false
            }
        }
    };
    w(out, &mut n, b"],\"page\":{\"nextCursor\":")?;
    if more {
        w(out, &mut n, b"\"")?;
        num(out, &mut n, page + 1)?;
        w(out, &mut n, b"\"")?
    } else {
        w(out, &mut n, b"null")?
    };
    w(
        out,
        &mut n,
        if more {
            b",\"hasMore\":true}}"
        } else {
            b",\"hasMore\":false}}"
        },
    )?;
    Ok(n)
}
const GUARD_CSS: &[u8] = b"/search.html?search_guard=css";
const GUARD_JS: &[u8] = b"/search.html?search_guard=js";
const GUARD_REDEEM: &[u8] = b"/search.html?search_guard=redeem";
const SEARCH_CSS: &[u8] = b"jieqiSearchCss=";
const SEARCH_JS: &[u8] = b"jieqiSearchJs=";
const SEARCH_TICKET: &[u8] = b"jieqiSearchTicket=";
const COOKIE_CAPACITY: usize = 4096 + 32;

fn has_search_next(label: &[u8]) -> bool {
    let start = "第".as_bytes();
    let Some(at) = find_subslice(label, start) else { return false; };
    let mut parts = label[at + start.len()..].splitn(2, |b| *b == b'/');
    let Some(current) = parts.next().and_then(|v| positive(v).ok()) else { return false; };
    let Some(tail) = parts.next() else { return false; };
    let end = find_subslice(tail, "页".as_bytes()).or_else(|| find_subslice(tail, "頁".as_bytes()));
    end.and_then(|i| positive(&tail[..i]).ok()).is_some_and(|last| current < last)
}

fn cookie_pair<'a>(pair: &'a [u8], name: &[u8]) -> R<&'a [u8]> {
    let value = pair.strip_prefix(name).ok_or("search_ticket_missing")?;
    if value.is_empty() || value.len() > 4096 || !value.iter().all(|b| {
        matches!(b, b'!' | b'#'..=b'+' | b'-'..=b':' | b'<'..=b'[' | b']'..=b'~')
    }) {
        return Err("search_ticket_missing");
    }
    Ok(pair)
}
fn ensure_guard_target(envelope: &[u8], path: &[u8]) -> R<()> {
    let mut buf = [0; 128];
    let target = protocol::text(envelope, b"guestSearchUrl", &mut buf)
        .map_err(|_| "search_ticket_missing")?.ok_or("search_ticket_missing")?;
    if target.strip_prefix(BASE) != Some(path) {
        return Err("search_ticket_missing");
    }
    Ok(())
}
fn guard_header_cookie<'a>(envelope: &[u8], key: &[u8], out: &'a mut [u8]) -> R<&'a [u8]> {
    let pair = protocol::text(envelope, b"guestSearchCookie", out)
        .map_err(|_| "search_ticket_missing")?.ok_or("search_ticket_missing")?;
    cookie_pair(pair, key)
}
fn guard_script_cookie(body: &[u8]) -> R<&[u8]> {
    let prefix = b"cookie=\"";
    let start = find_subslice(body, prefix).ok_or("search_ticket_missing")? + prefix.len();
    let rest = &body[start..];
    let end = find_subslice(rest, b"\";").ok_or("search_ticket_missing")?;
    let mut parts = rest[..end].split(|b| *b == b';');
    let pair = cookie_pair(parts.next().ok_or("search_ticket_missing")?, SEARCH_JS)?;
    for part in parts {
        let Some(at) = part.iter().position(|b| *b == b'=') else { continue; };
        let key = trim_ascii(&part[..at]);
        let value = trim_ascii(&part[at + 1..]);
        if key.eq_ignore_ascii_case(b"Path") && value != b"/" && value != br"\/" {
            return Err("search_ticket_missing");
        }
        if key.eq_ignore_ascii_case(b"Domain") {
            let host = value.strip_prefix(b".").unwrap_or(value);
            if !host.eq_ignore_ascii_case(b"www.bilimanga.net") && !host.eq_ignore_ascii_case(b"bilimanga.net") {
                return Err("search_ticket_missing");
            }
        }
        if key.eq_ignore_ascii_case(b"Max-Age") && positive(value).is_err() {
            return Err("search_ticket_missing");
        }
    }
    Ok(pair)
}

fn bootstrap_guest<'a>(css: &'a mut [u8], js: &'a mut [u8]) -> R<(&'a [u8], &'a [u8])> {
    let req = http_req_buf();
    let mut n = 0;
    w(req, &mut n, b"{\"version\":1,\"requests\":[")?;
    write_document_request(req, &mut n, GUARD_CSS, DocumentRequest::Guest { cookies: &[], revalidate: false })?;
    w(req, &mut n, b",")?;
    write_document_request(req, &mut n, GUARD_JS, DocumentRequest::Guest { cookies: &[], revalidate: false })?;
    w(req, &mut n, b"]}")?;
    let response = send_http(&req[..n])?;
    if protocol::direct(response, b"ok").map_err(|_| "network_error")? != Some(b"true") {
        return Err("network_error");
    }
    let array = protocol::direct(response, b"responses").map_err(|_| "network_error")?.ok_or("network_error")?;
    let mut responses = JsonArrayIter::from_raw_array(array).ok_or("network_error")?;
    let css_response = responses.next_object().ok_or("network_error")?;
    let js_response = responses.next_object().ok_or("network_error")?;
    if responses.next_object().is_some() {
        return Err("network_error");
    }
    protocol::body(css_response, body_buf())?;
    ensure_guard_target(css_response, GUARD_CSS)?;
    let css_pair = guard_header_cookie(css_response, SEARCH_CSS, css)?;
    let size = protocol::body(js_response, body_buf())?;
    ensure_guard_target(js_response, GUARD_JS)?;
    let js_pair = guard_script_cookie(&body_buf()[..size])?;
    js[..js_pair.len()].copy_from_slice(js_pair);
    Ok((css_pair, &js[..js_pair.len()]))
}

fn keyword_search(r: &[u8], query: &[u8]) -> R<usize> {
    if (!protocol::feature(r, b"source_bilimanga_guest_search_v1")
        && !protocol::feature(r, b"source_anonymous_http_v1")) || !protocol::feature(r, b"source_http_batch_v1") {
        return Err("unsupported_search_session");
    }
    let mut route = [0; 6144];
    let mut n = 0;
    w(&mut route, &mut n, b"/search/")?;
    if !write_url_encoded(&mut route, &mut n, query) {
        return Err("invalid_request");
    }
    w(&mut route, &mut n, b"_")?;
    num(&mut route, &mut n, request_page_number(r))?;
    w(&mut route, &mut n, b".html")?;
    let route = &route[..n];

    // These public visitor cookies belong only to this source call. Never
    // persist them or mix in authenticated WebView/account cookies.
    let mut css = [0; COOKIE_CAPACITY];
    let mut js = [0; COOKIE_CAPACITY];
    let mut ticket = [0; COOKIE_CAPACITY];
    let (css, js) = bootstrap_guest(&mut css, &mut js)?;
    let (redeem, _) = fetch_document(GUARD_REDEEM, DocumentRequest::Guest { cookies: &[css, js], revalidate: true })?;
    ensure_guard_target(redeem, GUARD_REDEEM)?;
    let ticket = guard_header_cookie(redeem, SEARCH_TICKET, &mut ticket)?;
    let (response, size) = fetch_document(route, DocumentRequest::Guest { cookies: &[css, js, ticket], revalidate: false })?;
    let mut target = [0; 8192];
    let target = protocol::text(response, b"guestSearchUrl", &mut target)
        .map_err(|_| "network_error")?.ok_or("network_error")?;
    let target = target.strip_prefix(BASE).ok_or("parse_error")?;
    let doc = parse_document(&body_buf()[..size])?;
    if let Ok(id) = detail_id(target) {
        let out = payload_buf();
        let mut n = 0;
        w(out, &mut n, b"{\"items\":[")?;
        write_detail_fields(r, id, out, &mut n, &doc)?;
        w(out, &mut n, b"],\"page\":{\"nextCursor\":null,\"hasMore\":false}}")?;
        Ok(n)
    } else if target.starts_with(b"/search/") && target.ends_with(b".html") {
        write_manga_list(r, &doc, ListKind::Search)
    } else {
        Err("parse_error")
    }
}

fn detail(r: &[u8]) -> R<usize> {
    require_request_policy(r)?;
    let out = payload_buf();
    let mut n = 0;
    w(out, &mut n, b"{\"manga\":")?;
    write_detail(r, mid(r)?, out, &mut n)?;
    w(out, &mut n, b"}")?;
    Ok(n)
}
fn write_detail(r: &[u8], id: &[u8], out: &mut [u8], n: &mut usize) -> R<()> {
    let mut p = [0; 256];
    let d = fetch(manga_path(id, &mut p)?)?;
    write_detail_fields(r, id, out, n, &d)
}
fn write_detail_fields(r: &[u8], id: &[u8], out: &mut [u8], n: &mut usize, d: &Node) -> R<()> {
    let mut p = [0; 256];
    let mut tb = [0; 2048];
    let title = d.select(b".book-title")?.text(&mut tb)?;
    let mut cb = [0; 4096];
    let cover_node = d.select(b".book-cover")?;
    let cover = cover_node.attr(b"src", &mut cb)?;
    let mut cb2 = [0; 4096];
    let cover = absolute(cover, &mut cb2)?;
    w(out, n, b"{\"id\":\"bilimanga:")?;
    w(out, n, id)?;
    w(out, n, b"\",\"title\":")?;
    q(out, n, title)?;
    w(out, n, b",\"cover\":")?;
    write_cover(out, n, cover, id, protocol::feature(r, b"source_public_cover_user_agent_v1"))?;
    w(out, n, b",\"authors\":[")?;
    let author = d.select(b".illname").or_else(|_| d.select(b".authorname"));
    if let Ok(a) = author {
        let mut b = [0; 2048];
        q(out, n, a.text(&mut b)?)?
    };
    w(out, n, b"],\"artists\":[")?;
    if let Ok(a) = d.select(b".authorname") {
        let mut b = [0; 2048];
        q(out, n, a.text(&mut b)?)?
    };
    w(
        out,
        n,
        b"],\"contentRating\":\"suggestive\",\"description\":\"",
    )?;
    let settings = json_object_member(r, b"settings").unwrap_or(b"{}");
    let desc = json_object_member(settings, b"DESCRIPTION");
    let enabled = |x: &[u8]| desc.map(|s| contains_bytes(s, x)).unwrap_or(true);
    if enabled(b"\"A\"") {
        if let Ok(a) = d.select(b".notice") {
            let mut b = [0; 16384];
            let t = a.text(&mut b)?;
            if !append_json_escaped(out, n, b"> ")
                || !append_json_escaped(out, n, t)
                || !append_json_escaped(out, n, b"\n\n")
            {
                return Err("capacity_exceeded");
            }
        }
    }
    let mut b = [0; 65536];
    let t = d.select(b"#bookSummary > content")?.text(&mut b)?;
    if !append_json_escaped(out, n, t) {
        return Err("capacity_exceeded");
    };
    if enabled(b"\"B\"") {
        if let Ok(a) = d.select(b".backupname") {
            let mut b = [0; 4096];
            let t = a.text(&mut b)?;
            if !append_json_escaped(out, n, "\n\n\n***别名**：".as_bytes())
                || !append_json_escaped(out, n, t)
                || !append_json_escaped(out, n, b"* ")
            {
                return Err("capacity_exceeded");
            }
        }
    }
    if enabled(b"\"C\"") {
        let a = Nodes::get(d, b".book-detail-btn .btn-group-cell > a")?;
        for i in 0..a.n {
            let a = a.at(i);
            let mut b = [0; 128];
            if a.text(&mut b)? == "輕小說".as_bytes() {
                let mut h = [0; 1024];
                let h = a.attr(b"href", &mut h)?;
                if let Some(k) = h.iter().position(|x| *x == b'?') {
                    let novel = &h[k + 1..];
                    positive(novel).map_err(|_| "parse_error")?;
                    for x in ["\n\n\n***[跳轉至「哔哩轻小说」上的同名小說](https://www.bilinovel.com/novel/".as_bytes(),novel,b".html)*** "]{if !append_json_escaped(out,n,x){return Err("capacity_exceeded")}}
                }
            }
        }
    }
    w(out, n, b"\",\"tags\":[")?;
    let tags = Nodes::get(d, b".tag-small, .book-meta em")?;
    let mut count = 0;
    let mut status = b"unknown".as_slice();
    for i in 0..tags.n {
        let node = tags.at(i);
        let mut b = [0; 2048];
        let t = node.text(&mut b)?;
        if t == "連載中".as_bytes() {
            status = b"ongoing";
            continue;
        };
        if t == "已完結".as_bytes() {
            status = b"completed";
            continue;
        };
        if contains_bytes(t, "收藏".as_bytes()) || contains_bytes(t, "推薦".as_bytes()) {
            continue;
        };
        if count > 0 {
            w(out, n, b",")?
        };
        q(out, n, t)?;
        count += 1
    }
    w(out, n, b"],\"status\":")?;
    q(out, n, status)?;
    w(out, n, b",\"links\":[{\"kind\":\"source\",\"url\":\"")?;
    w(out, n, BASE)?;
    w(out, n, manga_path(id, &mut p)?)?;
    w(out, n, b"\"}]}")
}
fn valid_chapter<'a>(p: &'a [u8], mid: &[u8]) -> R<&'a [u8]> {
    let p = p.strip_prefix(BASE).unwrap_or(p);
    let v = p.strip_prefix(b"/read/").ok_or("parse_error")?;
    let k = v.iter().position(|x| *x == b'/').ok_or("parse_error")?;
    if &v[..k] != mid {
        return Err("identity_mismatch");
    };
    chapter_filename(&v[k + 1..]).map_err(|_| "parse_error")?;
    Ok(v)
}
fn catalogue_date(doc: &Node) -> Option<[u8; 25]> {
    // Mihon uses the update date in the catalogue heading for all its chapters.
    let heading = doc.select(b".chapter-sub-title").ok()?;
    let mut text = [0; 512];
    let text = heading.text(&mut text).ok()?;
    let (year, month, day) = text
        .split(|c| !c.is_ascii_digit() && *c != b'-')
        .find_map(|part| {
            let mut fields = part.split(|c| *c == b'-');
            let (year, month, day) = (fields.next()?, fields.next()?, fields.next()?);
            (year.len() == 4 && (1..=2).contains(&month.len()) && (1..=2).contains(&day.len())
                && fields.next().is_none())
                .then_some((year, month, day))
        })?;
    let number = |s: &[u8]| core::str::from_utf8(s).ok()?.parse::<u16>().ok();
    let (year_number, month, day) = (number(year)?, number(month)?, number(day)?);
    if year_number == 0 {
        return None;
    }
    let last_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year_number % 4 == 0 && (year_number % 100 != 0 || year_number % 400 == 0) => 29,
        2 => 28,
        _ => return None,
    };
    if day == 0 || day > last_day {
        return None;
    }
    let mut date = *b"0000-00-00T00:00:00+08:00";
    date[..4].copy_from_slice(year);
    date[5..7].copy_from_slice(&[b'0' + (month / 10) as u8, b'0' + (month % 10) as u8]);
    date[8..10].copy_from_slice(&[b'0' + (day / 10) as u8, b'0' + (day % 10) as u8]);
    Some(date)
}
fn chapter_group(
    o: &mut [u8],
    n: &mut usize,
    doc: &Node,
    mid: &[u8],
    volume: &[u8],
    date: Option<&[u8]>,
    written: &mut usize,
) -> R<()> {
    let a = Nodes::get(doc, b".chapter-li-a")?;
    for i in (0..a.n).rev() {
        let node = a.at(i);
        let mut h = [0; 512];
        let href = node.attr(b"href", &mut h)?;
        let mut pb = [0; 512];
        let mut pn = 0;
        if href == b"javascript:cid(1)" {
            let j = if i == 0 { 1 } else { i - 1 };
            if j >= a.n {
                return Err("chapter_navigation_missing");
            };
            let neighbor = a.at(j);
            let mut nb = [0; 512];
            let anchor = neighbor.attr(b"href", &mut nb)?;
            w(&mut pb, &mut pn, valid_chapter(anchor, mid)?)?;
            w(&mut pb, &mut pn, if i == 0 { b"#prev" } else { b"#next" })?;
        } else {
            w(&mut pb, &mut pn, valid_chapter(href, mid)?)?;
        }
        let mut t = [0; 4096];
        let t = node.text(&mut t)?;
        let mut half = [0; 4096];
        let len = digits(t, &mut half)?;
        if len == 0 {
            return Err("parse_error");
        };
        if *written > 0 {
            w(o, n, b",")?
        };
        w(o, n, b"{\"id\":\"bilimanga-read:")?;
        w(o, n, &pb[..pn])?;
        w(o, n, b"\",\"mangaId\":\"bilimanga:")?;
        w(o, n, mid)?;
        w(o, n, b"\",\"title\":")?;
        q(o, n, &half[..len])?;
        w(
            o,
            n,
            b",\"chapterNumber\":null,\"volumeNumber\":null,\"language\":\"zh\",\"scanlator\":",
        )?;
        q(o, n, volume)?;
        if let Some(date) = date {
            w(o, n, b",\"dateUpload\":")?;
            q(o, n, date)?;
        }
        w(o, n, b"}")?;
        *written += 1;
    }
    Ok(())
}
fn chapters(r: &[u8]) -> R<usize> {
    require_request_policy(r)?;
    let id = mid(r)?;
    let mut b = [0; 256];
    let d = fetch(path(b"/read/", id, b"/catalog", &mut b)?)?;
    let mut title = [0; 2048];
    let title = d.select(b".book-title")?.text(&mut title)?;
    let date = catalogue_date(&d);
    let date = date.as_ref().map(|date| &date[..]);
    let groups = Nodes::get(&d, b".catalog-volume")?;
    let out = payload_buf();
    let mut n = 0;
    let mut written = 0;
    w(out, &mut n, b"{\"chapters\":[")?;
    if groups.n == 0 {
        chapter_group(out, &mut n, &d, id, b"", date, &mut written)?
    } else {
        for i in (0..groups.n).rev() {
            let g = groups.at(i);
            let mut t = [0; 4096];
            let bar = g.select(b".chapter-bar")?.text(&mut t)?;
            let bar = bar
                .strip_prefix(title)
                .and_then(|v| v.strip_prefix(b" "))
                .ok_or("parse_error")?;
            let mut vol = [0; 4096];
            let mut vn = 0;
            if bar.first().is_some_and(|v| v.is_ascii_digit()) {
                w(&mut vol, &mut vn, b"Vol.")?;
                w(&mut vol, &mut vn, bar)?
            } else {
                vn = digits(bar, &mut vol)?
            };
            chapter_group(out, &mut n, &g, id, &vol[..vn], date, &mut written)?
        }
    }
    if written == 0 {
        return Err("no_chapters");
    };
    w(
        out,
        &mut n,
        b"],\"page\":{\"nextCursor\":null,\"hasMore\":false}}",
    )?;
    Ok(n)
}
fn chapter_filename(filename: &[u8]) -> R<()> {
    let stem = filename.strip_suffix(b".html").ok_or("invalid_request")?;
    positive(stem.strip_suffix(b"_2").unwrap_or(stem))?;
    Ok(())
}

struct Chapter<'a> {
    id: &'a [u8],
    anchor: &'a [u8],
    owner: &'a [u8],
    marker: Option<&'static [u8]>,
}

fn chapter_identity(id: &[u8]) -> R<Chapter<'_>> {
    let mut stored = id.splitn(2, |c| *c == b'#');
    let anchor = stored.next().ok_or("invalid_request")?;
    let marker = match stored.next() {
        None => None,
        Some(b"prev") => Some(b"url_previous:'".as_slice()),
        Some(b"next") => Some(b"url_next:'".as_slice()),
        _ => return Err("invalid_request"),
    };
    let mut parts = anchor.splitn(2, |c| *c == b'/');
    let owner = parts.next().ok_or("invalid_request")?;
    positive(owner)?;
    chapter_filename(parts.next().ok_or("invalid_request")?)?;
    Ok(Chapter { id, anchor, owner, marker })
}

fn optional_image_owner(a: &[u8], owner: &[u8]) -> R<()> {
    if let Some(id) = protocol::raw_string(a, b"mangaId")? {
        if id.strip_prefix(b"bilimanga:") != Some(owner) {
            return Err("identity_mismatch");
        }
    }
    let mut route = [0; 1024];
    if let Some(route) = protocol::text(a, b"mangaUrl", &mut route)? {
        if canonical_detail_id(route) != Ok(owner) {
            return Err("identity_mismatch");
        }
    }
    Ok(())
}

fn navigation_target<'a>(body: &'a [u8], marker: &[u8], mid: &[u8]) -> R<&'a [u8]> {
    let start = find_subslice(body, marker).ok_or("chapter_navigation_missing")? + marker.len();
    let end = body[start..]
        .iter()
        .position(|x| *x == b'\'')
        .ok_or("chapter_navigation_missing")?
        + start;
    valid_chapter(&body[start..end], mid).map_err(|_| "chapter_navigation_missing")
}

fn pages(r: &[u8]) -> R<usize> {
    require_request_policy(r)?;
    let a = args(r);
    let chapter_id = string(a, b"chapterId")?.strip_prefix(b"bilimanga-read:").ok_or("invalid_request")?;
    let chapter = chapter_identity(chapter_id)?;
    optional_image_owner(a, chapter.owner)?;
    let mut pb = [0; 1024];
    let path_to_anchor = path(b"/read/", chapter.anchor, b"", &mut pb)?;
    let d = if let Some(marker) = chapter.marker {
        // The old persisted fragment is a local instruction, never an HTTP URL.
        // The actual navigation response controls the second same-title request.
        let (_, size) = fetch_document(path_to_anchor, DocumentRequest::Neighbor)?;
        let body = &body_buf()[..size];
        let probe = parse_document(body)?;
        let target = navigation_target(body, marker, chapter.owner)?;
        let stem = target.strip_suffix(b".html").ok_or("chapter_navigation_missing")?;
        let (stem, suffix) = if stem.ends_with(b"_2") {
            (stem, b".html".as_slice())
        } else {
            (stem, b"_2.html".as_slice())
        };
        let mut next = [0; 1024];
        let path_to_target = path(b"/read/", stem, suffix, &mut next)?;
        drop(probe);
        fetch(path_to_target)?
    } else {
        fetch(path_to_anchor)?
    };
    let images = Nodes::get(&d, b".imagecontent")?;
    if images.n == 0 {
        if let Ok(content) = d.select(b"#acontentz") {
            let mut notice = [0; 8192];
            let notice = content.text(&mut notice)?;
            if contains_bytes(notice, "電腦端".as_bytes()) || contains_bytes(notice, "电脑端".as_bytes()) {
                return Err("desktop_reader");
            }
        }
        return Err("parse_error");
    }
    let out = payload_buf();
    let mut n = 0;
    w(out, &mut n, b"{\"pages\":[")?;
    for i in 0..images.n {
        let node = images.at(i);
        let mut ub = [0; 4096];
        let u = node.attr(b"data-src", &mut ub)?;
        body_image(u).map_err(|_| "parse_error")?;
        if i > 0 {
            w(out, &mut n, b",")?
        };
        w(out, &mut n, b"{\"id\":\"bilimanga-page:")?;
        w(out, &mut n, chapter.id)?;
        w(out, &mut n, b":")?;
        num(out, &mut n, i)?;
        w(out, &mut n, b"\",\"index\":")?;
        num(out, &mut n, i)?;
        w(out, &mut n, b",\"image\":{\"kind\":\"url\",\"url\":")?;
        q(out, &mut n, u)?;
        w(out, &mut n, b"}}")?;
    }
    w(out, &mut n, b"]}")?;
    Ok(n)
}
fn run_search(r: &[u8]) -> u32 {
    if let Err(c) = protocol::request(r) {
        return write_error("search", c, "invalid request");
    }
    result("search", list(r, true))
}
fn run_get_manga_list(r: &[u8]) -> u32 {
    if let Err(c) = protocol::request(r) {
        return write_error("get_manga_list", c, "invalid request");
    }
    result("get_manga_list", list(r, false))
}
fn run_get_manga(r: &[u8]) -> u32 {
    if let Err(c) = protocol::request(r) {
        return write_error("get_manga", c, "invalid request");
    }
    result("get_manga", detail(r))
}
fn run_get_chapters(r: &[u8]) -> u32 {
    if let Err(c) = protocol::request(r) {
        return write_error("get_chapters", c, "invalid request");
    }
    result("get_chapters", chapters(r))
}
fn run_get_pages(r: &[u8]) -> u32 {
    if let Err(c) = protocol::request(r) {
        return write_error("get_pages", c, "invalid request");
    }
    result("get_pages", pages(r))
}
fn fixed(op: &str, s: &[u8]) -> u32 {
    payload_buf()[..s.len()].copy_from_slice(s);
    write_success_payload(op, s.len())
}
fn run_get_listings(_: &[u8]) -> u32 {
    fixed("get_listings",r#"{"listings":[{"id":"popular","name":"熱門漫畫"},{"id":"latest","name":"最新更新"},{"id":"filter","name":"分類瀏覽"}]}"#.as_bytes())
}
fn run_get_home(_: &[u8]) -> u32 {
    write_error("get_home", "not_implemented", "not supported")
}
fn run_get_filters(r: &[u8]) -> u32 {
    if let Err(c) = protocol::request(r) {
        return write_error("get_filters", c, "invalid request");
    }
    if !protocol::feature(r, b"source_filter_scope_header_v1") {
        return fixed("get_filters", b"{\"filters\":[]}");
    };
    fixed("get_filters", schema::FILTERS.as_bytes())
}
fn run_get_settings(_: &[u8]) -> u32 {
    fixed("get_settings", schema::SETTINGS.as_bytes())
}
fn image_request(r: &[u8]) -> R<usize> {
    require_request_policy(r)?;
    let a = args(r);
    let id = string(a, b"pageId")?;
    let mut url_buf = [0; 8192];
    let url = protocol::text(a, b"url", &mut url_buf)?.ok_or("invalid_request")?;
    let mut cover_buf = [0; 8192];
    let (url, is_cover) = if let Some(owner) = id.strip_prefix(b"bilimanga:") {
        positive(owner)?;
        optional_image_owner(a, owner)?;
        (public_cover_url(url, owner, &mut cover_buf)?, true)
    } else {
        let page = id.strip_prefix(b"bilimanga-page:").ok_or("invalid_request")?;
        let split = page.iter().rposition(|c| *c == b':').ok_or("invalid_request")?;
        let position = &page[split + 1..];
        if position != b"0" {
            positive(position)?;
        }
        let chapter = chapter_identity(&page[..split])?;
        optional_image_owner(a, chapter.owner)?;
        // The provider's image path is a content hash. Bind the request to the
        // exact known public image origins and file shape, not a fabricated CID.
        body_image(url)?;
        (url, false)
    };
    let out = payload_buf();
    let mut n = 0;
    w(out, &mut n, b"{\"url\":")?;
    q(out, &mut n, url)?;
    write_public_image_headers(out, &mut n)?;
    w(out, &mut n, b",\"Accept-Language\":\"zh\",\"Sec-CH-UA\":")?;
    q(out, &mut n, CHROME_BRANDS)?;
    w(out, &mut n, br#","Sec-CH-UA-Mobile":"?1","Sec-CH-UA-Platform":"\"Android\"","Sec-Fetch-Mode":"no-cors","Sec-Fetch-Dest":"image","Sec-Fetch-Site":"#)?;
    if is_cover {
        w(out, &mut n, br#""same-origin""#)?;
    } else {
        w(out, &mut n, br#""cross-site","Sec-Fetch-Storage-Access":"active""#)?;
    }
    w(out, &mut n, b"}}")?;
    Ok(n)
}
fn run_get_image_request(r: &[u8]) -> u32 {
    if let Err(c) = protocol::request(r) {
        return write_error("get_image_request", c, "invalid request");
    }
    result("get_image_request", image_request(r))
}
koma_source_sdk::koma_source_exports!("bilimanga_source");
