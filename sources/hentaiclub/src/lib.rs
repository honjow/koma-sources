#![no_std]
extern crate alloc;
// Complete anonymous reference: Keiyoushi HentaiClub.kt/Filters.kt v1, Apache2,
// de743aed846da53e00c45ab26ee0e77f7bb4355b. No login/preferences/latest in reference.
use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use koma_source_sdk::{
    host,
    json_utils::*,
    source::{SourceCapabilities, SourceInfo},
    source_protocol as protocol,
};
use serde_json::{json, Value};
use url::Url;
#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;
const BASE: &str = "https://www.hentaiclub.net";
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0";
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.hentaiclub.koma",
    name: "绅士会所",
    version: "0.1.0",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "绅士会所公开漫画，包含成人内容。",
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
    filters: true,
    settings: false,
    image_request: true,
    credentials: false,
};
koma_source_sdk::koma_source_buffers! {payload:1024*1024,http_out:4*1024*1024,body:3*1024*1024,http_req:16384,scratch:8192,}
koma_source_sdk::koma_source_helpers!();
type R<T> = Result<T, &'static str>;
#[link(wasm_import_module = "koma_host")]
extern "C" {
    #[link_name = "html_select_all"]
    fn select_all(d: i32, p: *const u8, n: u32, o: *mut i32, c: u32) -> i32;
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
    fn one(&self, s: &str) -> Option<Self> {
        host::html_select(self.0, s.as_bytes()).ok().map(Self)
    }
    fn required(&self, s: &str) -> R<Self> {
        self.one(s).ok_or("parse_error")
    }
    fn text(&self) -> R<String> {
        let mut b = alloc::vec![0;65536];
        let n = host::html_text(self.0, &mut b).map_err(|_| "result_too_large")?;
        if n >= b.len() {
            return Err("result_too_large");
        }
        b.truncate(n);
        String::from_utf8(b).map_err(|_| "parse_error")
    }
    fn attr(&self, key: &str) -> R<Option<String>> {
        let mut b = alloc::vec![0;16384];
        let n = match host::html_attr(self.0, key.as_bytes(), &mut b) {
            Ok(n) => n,
            Err(-1) => return Ok(None),
            Err(_) => return Err("parse_error"),
        };
        if n >= b.len() {
            return Err("result_too_large");
        }
        b.truncate(n);
        String::from_utf8(b).map(Some).map_err(|_| "parse_error")
    }
    fn all(&self, s: &str) -> R<Vec<Self>> {
        let mut ids = alloc::vec![0i32;8192];
        let n = unsafe {
            select_all(
                self.0.raw(),
                s.as_ptr(),
                s.len() as u32,
                ids.as_mut_ptr(),
                (ids.len() * 4) as u32,
            )
        };
        if n < 0 {
            return Err("parse_error");
        }
        if n as usize >= ids.len() {
            for id in ids.iter().filter(|id| **id > 0) {
                let _ = host::html_close(host::HtmlDescriptor::from_raw(*id));
            }
            return Err("result_too_large");
        }
        ids.truncate(n as usize);
        Ok(ids
            .into_iter()
            .map(|x| Self(host::HtmlDescriptor::from_raw(x)))
            .collect())
    }
}
fn absolute(raw: &str, document: &str) -> R<Url> {
    let base = Url::parse(document).map_err(|_| "parse_error")?;
    let u = base.join(raw).map_err(|_| "parse_error")?;
    if !matches!(u.scheme(), "https" | "http") {
        return Err("parse_error");
    }
    Ok(u)
}
fn local(raw: &str) -> R<String> {
    let u = absolute(raw, &format!("{}/", BASE)).map_err(|_| "invalid_request")?;
    let b = Url::parse(BASE).map_err(|_| "invalid_request")?;
    if u.origin() != b.origin()
        || u.query().is_some()
        || u.fragment().is_some()
        || u.path() == "/"
        || u.path().starts_with("/page/")
        || u.path().starts_with("/search/")
        || u.path().starts_with("/tag/")
        || u.path().starts_with("/sort/")
    {
        return Err("invalid_request");
    }
    Ok(u.path().to_string())
}
fn work(input: &Value, key: &str, prefix: &str) -> R<String> {
    let path = input["args"][key]
        .as_str()
        .and_then(|s| s.strip_prefix(prefix))
        .ok_or("invalid_request")?;
    if !path.starts_with('/') || path.starts_with("//") {
        return Err("invalid_request");
    }
    let normalized = local(path)?;
    if normalized != path {
        return Err("invalid_request");
    }
    Ok(normalized)
}
fn rate(r: &[u8]) -> R<()> {
    if protocol::feature(r, b"source_rate_limit_hentaiclub_v1") {
        Ok(())
    } else {
        Err("unsupported")
    }
}
fn fetch(url: &str) -> R<Node> {
    if host::check_cancel() {
        return Err("cancelled");
    }
    let n = koma_source_sdk::build_get_request(
        http_req_buf(),
        url.as_bytes(),
        Some(format!("{}/", BASE).as_bytes()),
        &[],
    )
    .ok_or("request_too_large")?;
    let n = host::http_request(&http_req_buf()[..n], http_out()).map_err(|_| "network_error")?;
    let n = protocol::body(&http_out()[..n], body_buf())?;
    Node::parse(&body_buf()[..n])
}
fn input_filter<'a>(input: &'a Value, key: &str) -> R<&'a str> {
    match input["args"]["filters"].get(key) {
        None => Ok(""),
        Some(Value::String(s)) => Ok(s),
        _ => Err("invalid_request"),
    }
}
fn list(r: &[u8], input: &Value, search: bool) -> R<Value> {
    let page = request_page_number(r);
    if page == 0 || page > 100000 {
        return Err("invalid_request");
    }
    let query = if search {
        input["args"]["query"].as_str().unwrap_or("")
    } else {
        ""
    };
    // KeiSource intercepts a valid URL search before this source; HentaiClub does
    // not override getMangaByUrl. Do not invent URL-import support.
    if Url::parse(query).is_ok_and(|u| matches!(u.scheme(), "https" | "http")) {
        return Err("not_implemented");
    }
    let tag = input_filter(input, "tag")?;
    let sort = input_filter(input, "sort")?;
    if !["", "r15", "r18"].contains(&sort) {
        return Err("invalid_request");
    }
    let u = if !query.trim().is_empty() {
        let mut u = Url::parse(BASE).map_err(|_| "parse_error")?;
        u.path_segments_mut()
            .map_err(|_| "parse_error")?
            .extend(["search", query.trim()]);
        let raw = if page > 1 {
            format!("{}/{}/", u, page)
        } else {
            format!("{}/", u)
        };
        Url::parse(&raw).map_err(|_| "invalid_request")?
    } else if !tag.trim().is_empty() {
        let raw = if page > 1 {
            format!("{}/tag/{}/{}/", BASE, tag.trim(), page)
        } else {
            format!("{}/tag/{}/", BASE, tag.trim())
        };
        Url::parse(&raw).map_err(|_| "invalid_request")?
    } else if !sort.is_empty() {
        Url::parse(&format!("{}/sort/{}.html", BASE, sort)).map_err(|_| "invalid_request")?
    } else {
        Url::parse(&if page == 1 {
            format!("{}/", BASE)
        } else {
            format!("{}/page/{}/", BASE, page)
        })
        .map_err(|_| "invalid_request")?
    };
    rate(r)?;
    let d = fetch(u.as_str())?;
    let nodes = d.all("div.item")?;
    if nodes.is_empty() {
        let title = d.required("title")?.text()?;
        if !title.contains("绅士会所") || d.one(".content").is_none() {
            return Err("parse_error");
        }
    }
    let mut items = Vec::new();
    for node in &nodes {
        let a = node.required("a.item-link")?;
        let path = local(&a.attr("href")?.ok_or("parse_error")?).map_err(|_| "parse_error")?;
        let title = node.required(".item-link-text")?.text()?;
        if title.trim().is_empty() {
            return Err("parse_error");
        }
        let mut card = json!({"id":format!("hentaiclub:{}",path),"title":title,"contentRating":"unknown","url":format!("{}{}",BASE,path)});
        if let Some(img) = node.one("img.item-img") {
            let raw = img
                .attr("data-original")?
                .filter(|s| !s.is_empty())
                .or(img.attr("src")?);
            if let Some(raw) = raw {
                card["cover"] = json!({"kind":"url","url":absolute(&raw,u.as_str())?.as_str()})
            }
        }
        items.push(card)
    }
    let more = nodes.len() >= 24;
    Ok(
        json!({"items":items,"page":{"hasMore":more,"nextCursor":if more{Some((page+1).to_string())}else{None}}}),
    )
}
fn views(text: &str) -> Option<String> {
    let mut start = 0;
    while let Some(i) = text[start..].find("浏览") {
        let rest = &text[start + i + "浏览".len()..];
        if let Some(rest) = rest.strip_prefix('：').or_else(|| rest.strip_prefix(':')) {
            let rest = rest.trim_start();
            let n = rest.bytes().take_while(u8::is_ascii_digit).count();
            if n > 0 && rest[n..].starts_with('次') {
                return Some(format!("浏览量：{}次", &rest[..n]));
            }
        }
        start += i + "浏览".len();
    }
    None
}
fn detail(r: &[u8], input: &Value) -> R<Value> {
    let path = work(input, "mangaId", "hentaiclub:")?;
    rate(r)?;
    let u = format!("{}{}", BASE, path);
    let d = fetch(&u)?;
    let title = d.required("title")?.text()?;
    let title = title.split(" - 绅士会所").next().unwrap_or(&title);
    if title.trim().is_empty() {
        return Err("parse_error");
    }
    let content = d.required(".content")?;
    let mut manga = json!({"id":format!("hentaiclub:{}",path),"title":title,"contentRating":"unknown","status":if path.contains("/r18/"){"completed"}else{"ongoing"},"links":[{"kind":"source","url":u}]});
    if let Some(first) = content.one("div.post-item[data-src]") {
        let raw = first.attr("data-src")?.ok_or("parse_error")?;
        if !raw.is_empty() {
            manga["cover"] = json!({"kind":"url","url":absolute(&raw,&u)?.as_str()})
        }
    }
    let tags = content
        .all("a[href*='/tag/']")?
        .iter()
        .map(Node::text)
        .collect::<R<Vec<_>>>()?;
    if let Some(first) = tags.first() {
        manga["authors"] = json!([first])
    }
    manga["tags"] = json!(tags);
    if let Some(v) = views(&content.text()?) {
        manga["description"] = json!(v)
    }
    Ok(json!({"manga":manga}))
}
fn chapters(input: &Value) -> R<Value> {
    let path = work(input, "mangaId", "hentaiclub:")?;
    Ok(
        json!({"chapters":[{"id":format!("hentaiclub-read:{}",path),"mangaId":format!("hentaiclub:{}",path),"title":"章节 1","index":0,"url":format!("{}{}",BASE,path)}]}),
    )
}
fn chapter(input: &Value) -> R<String> {
    let path = work(input, "chapterId", "hentaiclub-read:")?;
    if let Some(m) = input["args"].get("mangaId") {
        if m.as_str() != Some(&format!("hentaiclub:{}", path)) {
            return Err("invalid_request");
        }
    }
    Ok(path)
}
fn pages(r: &[u8], input: &Value) -> R<Value> {
    let path = chapter(input)?;
    rate(r)?;
    let u = format!("{}{}", BASE, path);
    let d = fetch(&u)?;
    let images = d.all("div.post-item[data-src]")?;
    if images.is_empty() {
        if d.one(".content").is_some() && d.required("title")?.text()?.contains("绅士会所") {
            return Err("chapter_unavailable");
        }
        return Err("parse_error");
    }
    let mut pages = Vec::new();
    for (i, node) in images.iter().enumerate() {
        let raw = node.attr("data-src")?.ok_or("parse_error")?;
        if raw.is_empty() {
            return Err("parse_error");
        }
        let image = absolute(&raw, &u)?;
        pages.push(json!({"id":format!("hentaiclub-page:{}:{}",path,i),"index":i,"image":{"kind":"url","url":image.as_str()}}))
    }
    Ok(json!({"pages":pages}))
}
fn image_request(input: &Value) -> R<Value> {
    let args = &input["args"];
    let page_id = args["pageId"].as_str().ok_or("invalid_request")?;
    let path = if page_id.starts_with("hentaiclub:") {
        // Cover resolution uses the manga ID in place of a reader page ID.
        let path = work(input, "pageId", "hentaiclub:")?;
        if args.get("chapterId").is_some() {
            return Err("invalid_request");
        }
        if let Some(manga_id) = args.get("mangaId") {
            if manga_id.as_str() != Some(page_id) {
                return Err("invalid_request");
            }
        }
        path
    } else {
        let raw = page_id
            .strip_prefix("hentaiclub-page:")
            .ok_or("invalid_request")?;
        let (path, index) = raw.rsplit_once(':').ok_or("invalid_request")?;
        let ordinal = index.parse::<usize>().map_err(|_| "invalid_request")?;
        if ordinal.to_string() != index || local(path)? != path {
            return Err("invalid_request");
        }
        if let Some(manga) = args.get("mangaId") {
            if manga.as_str() != Some(&format!("hentaiclub:{}", path)) {
                return Err("invalid_request");
            }
        }
        if let Some(chapter) = args.get("chapterId") {
            if chapter.as_str() != Some(&format!("hentaiclub-read:{}", path)) {
                return Err("invalid_request");
            }
        }
        path.to_string()
    };
    if let Some(route) = args.get("mangaUrl") {
        if local(route.as_str().ok_or("invalid_request")?)? != path {
            return Err("invalid_request");
        }
    }
    let u = input["args"]["url"].as_str().ok_or("invalid_request")?;
    let image = Url::parse(u).map_err(|_| "invalid_request")?;
    if !matches!(image.scheme(), "https" | "http")
        || !image.username().is_empty()
        || image.password().is_some()
    {
        return Err("invalid_request");
    }
    Ok(json!({"url":image.as_str(),"headers":{"User-Agent":UA,"Referer":format!("{}/",BASE)}}))
}
fn dispatch(op: &str, r: &[u8]) -> R<Value> {
    protocol::request(r)?;
    let args = protocol::direct(r, b"args")?.unwrap_or(r);
    let mut owner = [0; 16384];
    let _ = protocol::text(args, b"mangaUrl", &mut owner)?;
    let v: Value = serde_json::from_slice(r).map_err(|_| "invalid_request")?;
    match op {
        "get_listings" => {
            Ok(json!({"listings":[{"id":"popular","name":"热门","supportsPaging":true}]}))
        }
        "get_filters" => Ok(
            json!({"filters":[{"id":"sort","name":"分类","type":"select","default":"","options":[{"value":"","label":"全部"},{"value":"r15","label":"R-15"},{"value":"r18","label":"R-18"}]},{"id":"tag","name":"标签 (输入标签名)","type":"text","default":""}]}),
        ),
        "search" => list(r, &v, true),
        "get_manga_list" => {
            if v["args"]["listingId"]
                .as_str()
                .is_some_and(|s| s != "popular")
            {
                return Err("not_implemented");
            }
            list(r, &v, false)
        }
        "get_manga" => detail(r, &v),
        "get_chapters" => chapters(&v),
        "get_pages" => pages(r, &v),
        "get_image_request" => image_request(&v),
        _ => Err("not_implemented"),
    }
}
fn run(op: &str, r: &[u8]) -> u32 {
    match dispatch(op, r) {
        Ok(v) => match serde_json::to_vec(&v) {
            Ok(b) if b.len() <= payload_buf().len() => {
                payload_buf()[..b.len()].copy_from_slice(&b);
                write_success_payload(op, b.len())
            }
            _ => write_error(op, "result_too_large", "书源结果过大。"),
        },
        Err(c) => write_error(
            op,
            c,
            match c {
                "unsupported" => "当前应用缺少此书源所需限流能力，请更新应用。",
                "forbidden" => "网站拒绝了本次请求，请打开原网页确认访问状态。",
                "not_found" => "网站未提供此内容，请打开原网页确认。",
                "rate_limit" => "请求过于频繁，请稍后重试。",
                "network_error" => "网络传输未完成，请稍后重试。",
                "chapter_unavailable" => "相册网页没有提供图片，请打开原网页确认。",
                _ => "漫画获取失败，请稍后重试或打开原网页确认。",
            },
        ),
    }
}
fn run_search(r: &[u8]) -> u32 {
    run("search", r)
}
fn run_get_manga(r: &[u8]) -> u32 {
    run("get_manga", r)
}
fn run_get_chapters(r: &[u8]) -> u32 {
    run("get_chapters", r)
}
fn run_get_pages(r: &[u8]) -> u32 {
    run("get_pages", r)
}
fn run_get_listings(r: &[u8]) -> u32 {
    run("get_listings", r)
}
fn run_get_manga_list(r: &[u8]) -> u32 {
    run("get_manga_list", r)
}
fn run_get_filters(r: &[u8]) -> u32 {
    run("get_filters", r)
}
fn run_get_settings(r: &[u8]) -> u32 {
    run("get_settings", r)
}
fn run_get_home(r: &[u8]) -> u32 {
    run("get_home", r)
}
fn run_get_image_request(r: &[u8]) -> u32 {
    run("get_image_request", r)
}
koma_source_sdk::koma_source_exports!("hentaiclub_source");
