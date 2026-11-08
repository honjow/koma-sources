// Shared HTML protocol for the two gallery sites. Each crate owns its own origin and IDs.
extern crate alloc;

use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::str;
use dom::{Node, Nodes, R};
use koma_source_sdk::{
    host,
    json_utils::*,
    source::{SourceCapabilities, SourceInfo},
    source_protocol as protocol,
};
use serde_json::{json, Value};

#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: SOURCE_ID,
    name: NAME,
    version: "0.1.1",
    api_version: "0.2",
    language: "all",
    author: "Koma",
    description: "公开图集，含成人内容",
    content_rating: "nsfw",
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
    image_request: false,
    credentials: false,
};
koma_source_sdk::koma_source_buffers! { payload:512*1024,http_out:2*1024*1024,body:1536*1024,http_req:8192,scratch:8192, }
koma_source_sdk::koma_source_helpers!();

fn text(node: &Node) -> R<String> {
    let mut buf = [0; 16384];
    let raw = str::from_utf8(node.text(&mut buf)?).map_err(|_| "parse_error")?;
    Ok(raw.split_whitespace().collect::<Vec<_>>().join(" "))
}
fn attr(node: &Node, key: &[u8]) -> R<String> {
    let mut buf = [0; 4096];
    let raw = node.attr(key, &mut buf)?;
    Ok(str::from_utf8(raw)
        .map_err(|_| "parse_error")?
        .trim()
        .to_string())
}
fn positive(s: &str) -> Option<u32> {
    (!s.starts_with('0') && s.bytes().all(|b| b.is_ascii_digit()))
        .then(|| s.parse::<u32>().ok())
        .flatten()
}
fn work_id(path: &str) -> Option<u32> {
    positive(path.strip_prefix('/')?.trim_end_matches('/'))
}
fn site_path(url: &str) -> Option<&str> {
    let path = if let Some(path) = url.strip_prefix(BASE) {
        path
    } else {
        url
    };
    (path.starts_with('/')
        && !path.starts_with("//")
        && !path.contains(['\\', '#'])
        && !path.bytes().any(|b| b.is_ascii_control() || b == b' '))
    .then_some(path)
}
fn fetch(path: &str) -> R<Node> {
    if host::check_cancel() {
        return Err("cancelled");
    }
    let url = format!("{BASE}{path}");
    let headers = [(
        b"Accept".as_slice(),
        b"text/html,application/xhtml+xml".as_slice(),
    )];
    let size = koma_source_sdk::build_get_request(http_req_buf(), url.as_bytes(), None, &headers)
        .ok_or("capacity_exceeded")?;
    let size =
        host::http_request(&http_req_buf()[..size], http_out()).map_err(|_| "network_error")?;
    let size = protocol::body(&http_out()[..size], body_buf())?;
    Node::parse(&body_buf()[..size])
}
fn image(url: &str) -> R<String> {
    let url = if url.starts_with("//") {
        format!("https:{url}")
    } else if url.starts_with('/') {
        format!("{BASE}{url}")
    } else {
        url.to_string()
    };
    let Some((host, path)) = url.strip_prefix("https://").and_then(|v| v.split_once('/')) else {
        return Err("parse_error");
    };
    if (host != IMAGE_HOST && host != BASE.trim_start_matches("https://"))
        || path.is_empty()
        || url
            .bytes()
            .any(|b| b.is_ascii_control() || matches!(b, b' ' | b'\\'))
    {
        return Err("parse_error");
    }
    Ok(url)
}
fn article_title(doc: &Node, id: u32) -> R<String> {
    let article = doc.select(b".article[data-id]")?;
    if positive(&attr(&article, b"data-id")?) != Some(id) {
        return Err("identity_mismatch");
    }
    let title = text(&doc.select(b".article-header")?)?;
    if title.is_empty() {
        return Err("parse_error");
    }
    Ok(title)
}
fn card(id: u32, title: String, cover: Option<String>) -> Value {
    let mut value = json!({"id":id.to_string(), "contentRating":"nsfw", "title":title, "url":format!("{BASE}/{id}")});
    if let Some(url) = cover {
        // Kiutaku's image server requires no Referer; keep an explicit empty header set.
        value["cover"] = json!({"kind":"url","url":url,"headers":{}});
    }
    value
}
fn card_from(node: &Node) -> R<Value> {
    let link = node.select(CARD_LINK)?;
    let href = attr(&link, b"href")?;
    let id = site_path(&href).and_then(work_id).ok_or("parse_error")?;
    let title = if KIUTAKU {
        text(&node.select(b"h2")?)?
    } else {
        text(&link)?
    };
    if title.is_empty() {
        return Err("parse_error");
    }
    let cover = node
        .select(b"img[src]")
        .ok()
        .and_then(|n| attr(&n, b"src").ok())
        .and_then(|url| image(&url).ok());
    Ok(card(id, title, cover))
}
fn query_start(path: &str) -> Option<usize> {
    path.split_once('?')?
        .1
        .split('&')
        .find_map(|param| param.strip_prefix("start="))?
        .parse()
        .ok()
}
fn listings(doc: &Node, start: usize) -> R<Value> {
    let container = doc.select(b".blog");
    let rows = Nodes::get(doc, b".blog > div.items-row")?;
    if container.is_err() {
        // Both sites replace the catalogue wrapper with this explicit empty state.
        if let Ok(empty) = doc.select(b".main-body.view-items > pre") {
            if text(&empty)? == "No item here!" && rows.n == 0 {
                return Ok(json!({"items":[],"page":{"hasMore":false,"nextCursor":null}}));
            }
        }
        return Err("parse_error");
    }
    let mut items = Vec::with_capacity(rows.n);
    for i in 0..rows.n {
        items.push(card_from(&rows.at(i))?);
    }
    let next = if let Ok(link) = doc.select(b"a.pagination-next:not([disabled])") {
        let href = attr(&link, b"href")?;
        let next = site_path(&href)
            .and_then(query_start)
            .ok_or("parse_error")?;
        if next != start + 20 {
            return Err("parse_error");
        }
        Some((next / 20 + 1).to_string())
    } else {
        None
    };
    if items.is_empty() && next.is_some() {
        return Err("parse_error");
    }
    Ok(json!({"items":items,"page":{"hasMore":next.is_some(),"nextCursor":next}}))
}
fn result_list(r: &[u8], search: bool) -> R<Value> {
    let args = dom::args(r);
    let page = request_page_number(r);
    if page > 100_000 {
        return Err("invalid_request");
    }
    let start = (page - 1) * 20;
    let mut buf = [0; 4096];
    let q = if search {
        protocol::text(args, b"query", &mut buf)?.unwrap_or(b"")
    } else {
        b""
    };
    let q = str::from_utf8(q).map_err(|_| "invalid_request")?.trim();
    if q.starts_with("https://") || q.starts_with("http://") {
        let canonical = q.strip_prefix("http://").map(|v| format!("https://{v}"));
        let url = canonical.as_deref().unwrap_or(q);
        let path = url
            .strip_prefix(BASE)
            .and_then(site_path)
            .ok_or("invalid_request")?;
        let route = path.split('?').next().ok_or("invalid_request")?;
        if let Some(id) = work_id(route) {
            let doc = fetch(&format!("/{id}"))?;
            return Ok(
                json!({"items":[detail_card(&doc,id)?],"page":{"hasMore":false,"nextCursor":null}}),
            );
        }
        if KIUTAKU
            || !(matches!(route, "/" | "/hot")
                || ["/tag/", "/brand/", "/girl/"]
                    .iter()
                    .any(|prefix| route.strip_prefix(prefix).and_then(positive).is_some()))
        {
            return Err("invalid_request");
        }
        let path = if page > 1 {
            let query = path.split_once('?').map(|(_, v)| v).unwrap_or("");
            let mut retained = query
                .split('&')
                .filter(|p| !p.is_empty() && !p.starts_with("start="))
                .collect::<Vec<_>>()
                .join("&");
            if !retained.is_empty() {
                retained.push('&');
            }
            format!("{route}?{retained}start={start}")
        } else {
            path.to_string()
        };
        return listings(&fetch(&path)?, query_start(&path).unwrap_or(0));
    }
    let path = if search {
        let mut encoded = [0; 8192];
        let mut size = 0;
        if !write_url_encoded(&mut encoded, &mut size, q.as_bytes()) {
            return Err("capacity_exceeded");
        }
        let query = str::from_utf8(&encoded[..size]).map_err(|_| "invalid_request")?;
        format!("/?search={query}&start={start}")
    } else if protocol::raw_string(args, b"listingId")? == Some(b"latest") {
        format!("/?start={start}")
    } else {
        format!("/hot?start={start}")
    };
    listings(&fetch(&path)?, start)
}
fn manga_id(r: &[u8]) -> R<u32> {
    let id = dom::string(dom::args(r), b"mangaId")?;
    str::from_utf8(id)
        .ok()
        .and_then(positive)
        .ok_or("invalid_request")
}
fn first_image(doc: &Node) -> Option<String> {
    let node = doc.select(b".article-fulltext img[src]").ok()?;
    image(&attr(&node, b"src").ok()?).ok()
}
fn detail_card(doc: &Node, id: u32) -> R<Value> {
    let mut value = card(id, article_title(doc, id)?, first_image(doc));
    value["status"] = json!("completed");
    if !KIUTAKU {
        if let Ok(desc) = doc.select(b".article-info:not(:has(small))") {
            value["description"] = json!(text(&desc)?);
        }
    }
    let nodes = Nodes::get(doc, b".article-tags .tags .tag")?;
    let mut tags = Vec::new();
    for i in 0..nodes.n {
        let tag = text(&nodes.at(i))?;
        let tag = tag.trim_start_matches('#').trim();
        if !tag.is_empty() {
            tags.push(tag.to_string());
        }
    }
    value["tags"] = json!(tags);
    Ok(value)
}
// Kiutaku serves related cards in the very same HTML as the work details.
fn related_cards(doc: &Node, id: u32) -> R<Vec<Value>> {
    let rows = Nodes::get(doc, b".bottom-articles .items-row")?;
    let mut items = Vec::new();
    let mut seen = alloc::collections::BTreeSet::new();
    seen.insert(id.to_string());
    for i in 0..rows.n {
        let Ok(item) = card_from(&rows.at(i)) else {
            continue;
        };
        if let Some(candidate) = item["id"].as_str() {
            if seen.insert(candidate.to_string()) {
                items.push(item);
            }
        }
    }
    Ok(items)
}

fn page_link(href: &str, id: u32) -> R<u32> {
    let path = site_path(href).ok_or("identity_mismatch")?;
    let (route, query) = path.split_once('?').unwrap_or((path, ""));
    if work_id(route) != Some(id) {
        return Err("identity_mismatch");
    }
    if query.is_empty() {
        return Ok(1);
    }
    query
        .strip_prefix("page=")
        .and_then(positive)
        .ok_or("parse_error")
}
fn catalogue(doc: &Node, id: u32) -> R<Vec<u32>> {
    let links = Nodes::get(
        doc,
        b"nav.pagination:first-of-type .pagination-list a.pagination-link",
    )?;
    if links.n == 0 {
        return if first_image(doc).is_some() {
            Ok(vec![1])
        } else {
            Err("chapter_unavailable")
        };
    }
    let mut pages = Vec::new();
    for i in 0..links.n {
        let node = links.at(i);
        let p = page_link(&attr(&node, b"href")?, id)?;
        if positive(&text(&node)?) != Some(p) {
            return Err("parse_error");
        }
        pages.push(p);
    }
    if !KIUTAKU {
        let last = *pages.last().ok_or("parse_error")?;
        if last > 1000 {
            return Err("capacity_exceeded");
        }
        pages = (1..=last).collect();
    }
    pages.reverse();
    Ok(pages)
}
fn uploaded_at(doc: &Node) -> Option<String> {
    let raw = text(&doc.select(b".article-info > small").ok()?).ok()?;
    let (time, date) = raw.trim_start_matches('🕒').trim().split_once(' ')?;
    let (hour, minute) = time.split_once(':')?;
    let (day, rest) = date.split_once('-')?;
    let (month, year) = rest.split_once('-')?;
    let (h, m, d, mon, y) = (
        hour.parse::<u32>().ok()?,
        minute.parse::<u32>().ok()?,
        day.parse::<u32>().ok()?,
        month.parse::<u32>().ok()?,
        year.parse::<u32>().ok()?,
    );
    if h > 23
        || m > 59
        || !(1..=31).contains(&d)
        || !(1..=12).contains(&mon)
        || !(1..=9999).contains(&y)
    {
        return None;
    }
    Some(format!("{y:04}-{mon:02}-{d:02}T{h:02}:{m:02}:00Z"))
}
fn chapters(r: &[u8]) -> R<Value> {
    let id = manga_id(r)?;
    let doc = fetch(&format!("/{id}"))?;
    article_title(&doc, id)?;
    let date = if KIUTAKU { None } else { uploaded_at(&doc) };
    let chapters: Vec<_> = catalogue(&doc, id)?
        .into_iter()
        .map(|page| {
            let mut row = json!({"id":format!("{id}:{page}"), "title":format!("Page {page}"),
            "chapterNumber":page,"url":format!("{BASE}/{id}?page={page}")});
            if let Some(date) = &date {
                row["publishedAt"] = json!(date);
            }
            row
        })
        .collect();
    Ok(json!({"chapters":chapters,"page":{"hasMore":false,"nextCursor":null}}))
}
fn pages(r: &[u8]) -> R<Value> {
    let args = dom::args(r);
    let raw = str::from_utf8(dom::string(args, b"chapterId")?).map_err(|_| "invalid_request")?;
    let (id, page) = raw.split_once(':').ok_or("invalid_request")?;
    let (id, page) = (
        positive(id).ok_or("invalid_request")?,
        positive(page).ok_or("invalid_request")?,
    );
    if protocol::raw_string(args, b"mangaId")?.is_some() && manga_id(r)? != id {
        return Err("identity_mismatch");
    }
    let doc = fetch(&format!("/{id}?page={page}"))?;
    article_title(&doc, id)?;
    if let Ok(current) = doc.select(b"nav.pagination:first-of-type a.pagination-link.is-current") {
        if page_link(&attr(&current, b"href")?, id)? != page {
            return Err("chapter_unavailable");
        }
    } else if page != 1 {
        return Err("chapter_unavailable");
    }
    let nodes = Nodes::get(&doc, b".article-fulltext img[src]")?;
    if nodes.n == 0 {
        return Err("chapter_unavailable");
    }
    let mut pages = Vec::with_capacity(nodes.n);
    for index in 0..nodes.n {
        let url = image(&attr(&nodes.at(index), b"src")?)?;
        pages.push(json!({"id":format!("{id}:{page}:{index}"), "index":index,
            "image":{"kind":"url","url":url,"headers":{}}}));
    }
    Ok(json!({"pages":pages}))
}

fn finish(op: &str, value: R<Value>) -> u32 {
    match value {
        Ok(value) => match serde_json::to_vec(&value) {
            Ok(bytes) if bytes.len() <= payload_buf().len() => {
                payload_buf()[..bytes.len()].copy_from_slice(&bytes);
                write_success_payload(op, bytes.len())
            }
            _ => write_error(op, "capacity_exceeded", "图集结果过大"),
        },
        Err(code) => write_error(
            op,
            code,
            match code {
                "not_found" | "chapter_unavailable" => "网站没有提供该图集或这一页",
                "forbidden" => "网站拒绝了请求，请在原网页确认",
                "rate_limit" => "请求过于频繁，请稍后重试",
                "host_feature_required" => "请更新 Koma 以支持此书源的请求限流",
                _ => "图集获取失败，请在原网页确认",
            },
        ),
    }
}
fn network_request(r: &[u8]) -> R<()> {
    protocol::request(r)?;
    if !KIUTAKU && !protocol::feature(r, b"source_rate_limit_xiutaku_v1")
        && !protocol::feature(r, b"source_request_limits_v1") {
        return Err("host_feature_required");
    }
    Ok(())
}

fn run_search(r: &[u8]) -> u32 {
    finish(
        "search",
        network_request(r).and_then(|_| result_list(r, true)),
    )
}
fn run_get_manga_list(r: &[u8]) -> u32 {
    finish(
        "get_manga_list",
        network_request(r).and_then(|_| result_list(r, false)),
    )
}
fn run_get_manga(r: &[u8]) -> u32 {
    finish(
        "get_manga",
        network_request(r).and_then(|_| {
            let id = manga_id(r)?;
            let doc = fetch(&format!("/{id}"))?;
            let mut detail = detail_card(&doc, id)?;
            if KIUTAKU && protocol::feature(r, b"source_related_manga_v1") {
                // Recommendations are optional metadata and cannot break the main detail.
                if let Ok(items) = related_cards(&doc, id) {
                    detail["relatedManga"] = json!(items);
                }
            }
            Ok(json!({"manga":detail}))
        }),
    )
}
fn run_get_chapters(r: &[u8]) -> u32 {
    finish("get_chapters", network_request(r).and_then(|_| chapters(r)))
}
fn run_get_pages(r: &[u8]) -> u32 {
    finish("get_pages", network_request(r).and_then(|_| pages(r)))
}
fn run_get_listings(_: &[u8]) -> u32 {
    finish(
        "get_listings",
        Ok(json!({"listings":[
            {"id":"popular","name":"热门","supportsPaging":true},
            {"id":"latest","name":"最新","supportsPaging":true}
        ]})),
    )
}
fn run_get_filters(_: &[u8]) -> u32 {
    write_error("get_filters", "not_implemented", "网站未提供筛选")
}
fn run_get_settings(_: &[u8]) -> u32 {
    write_error("get_settings", "not_implemented", "网站未提供设置")
}
fn run_get_home(_: &[u8]) -> u32 {
    write_error("get_home", "not_implemented", "使用图集栏目")
}
fn run_get_image_request(_: &[u8]) -> u32 {
    write_error("get_image_request", "not_implemented", "使用图集原图")
}

mod dom {
    use super::*;
    pub(super) type R<T> = Result<T, &'static str>;
    #[link(wasm_import_module = "koma_host")]
    extern "C" {
        #[link_name = "html_select_all"]
        fn all(d: i32, s: *const u8, n: u32, o: *mut i32, c: u32) -> i32;
    }
    pub(super) struct Node(host::HtmlDescriptor);
    impl Drop for Node {
        fn drop(&mut self) {
            let _ = host::html_close(self.0);
        }
    }
    impl Node {
        pub(super) fn parse(b: &[u8]) -> R<Self> {
            host::html_parse(b).map(Self).map_err(|_| "parse_error")
        }
        pub(super) fn select(&self, s: &[u8]) -> R<Self> {
            host::html_select(self.0, s)
                .map(Self)
                .map_err(|_| "parse_error")
        }
        pub(super) fn text<'a>(&self, b: &'a mut [u8]) -> R<&'a [u8]> {
            let n = host::html_text(self.0, b).map_err(|_| "capacity_exceeded")?;
            if n >= b.len() {
                return Err("capacity_exceeded");
            };
            Ok(trim_ascii(&b[..n]))
        }
        pub(super) fn attr<'a>(&self, k: &[u8], b: &'a mut [u8]) -> R<&'a [u8]> {
            let n = host::html_attr(self.0, k, b).map_err(|_| "parse_error")?;
            if n >= b.len() {
                return Err("capacity_exceeded");
            };
            Ok(&b[..n])
        }
    }
    pub(super) struct Nodes {
        ids: [i32; 8192],
        pub(super) n: usize,
    }
    impl Nodes {
        pub(super) fn get(d: &Node, s: &[u8]) -> R<Self> {
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
        pub(super) fn at(&self, i: usize) -> core::mem::ManuallyDrop<Node> {
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
    pub(super) fn args(r: &[u8]) -> &[u8] {
        json_object_member(r, b"args").unwrap_or(r)
    }
    pub(super) fn string<'a>(r: &'a [u8], k: &[u8]) -> R<&'a [u8]> {
        protocol::raw_string(r, k)?.ok_or("invalid_request")
    }
}
