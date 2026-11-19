#![no_std]
extern crate alloc;
use alloc::{
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec::Vec,
};
use koma_source_sdk::{
    chapters::{Chapter, ChapterOrder, ChapterResponse, ChapterResult},
    json_utils::request_page_number,
    source::{SourceCapabilities, SourceInfo},
};
use serde_json::{json, Value};
#[path = "../../port_support.rs"]
mod port_support;
use port_support::{self as port, Node, Result};

const BASE: &str = "https://m.bikamanhua.com";
const PAGE_SIZE: usize = 30;
// The upstream MCCMS client uses its desktop reader header for chapter documents and images.
const READER_UA: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:150.0) Gecko/20100101 Firefox/150.0";
const SORTS: &[(&str, &str)] = &[("hits", "热门"), ("addtime", "最近更新"), ("score", "评分")];
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.bikamanhua.koma",
    name: "哔咔漫画（同名站）",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "bikamanhua.com 网页漫画目录",
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
    filters: true,
    settings: false,
    image_request: false,
    credentials: false,
};
#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;
koma_source_sdk::koma_source_buffers! {
    payload: 1024 * 1024, http_out: 4 * 1024 * 1024, body: 3 * 1024 * 1024,
    http_req: 4096, scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

fn station(host: &str) -> bool {
    matches!(
        host,
        "bikamanhua.com" | "m.bikamanhua.com" | "www.bikamanhua.com"
    )
}

fn number(part: &str) -> bool {
    !part.is_empty() && part.len() <= 24 && part.bytes().all(|b| b.is_ascii_digit())
}

fn work_path(path: &str) -> bool {
    path.strip_prefix('/').is_some_and(|s| {
        !s.is_empty()
            && s.len() <= 150
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
    })
}

fn chapter_parts(path: &str) -> Option<(&str, &str)> {
    let (series, chapter) = path
        .strip_prefix('/')?
        .strip_suffix(".html")?
        .split_once('-')?;
    (number(series) && number(chapter)).then_some((series, chapter))
}

fn clean_path(href: &str, valid: fn(&str) -> bool) -> Result<String> {
    let url = port::absolute(BASE, href)?;
    let path = url.path().strip_prefix("/index.php").unwrap_or(url.path());
    if !url.host_str().is_some_and(station) || !valid(path) {
        return Err("parse_error");
    }
    Ok(path.to_string())
}

fn plain(url: &str) -> Result<Node> {
    Node::parse(&port::get(url)?)
}

fn api(url: &str) -> Result<Value> {
    let data = port::request(
        url,
        "GET",
        json!({
            "X-Requested-With":"XMLHttpRequest","Accept":"application/json, text/javascript, */*; q=0.01",
            "Referer":format!("{BASE}/")
        }),
        None,
    )?;
    let value: Value = serde_json::from_str(&data).map_err(|_| "parse_error")?;
    let rows = value
        .get("data")
        .filter(|rows| rows.is_array())
        .ok_or("parse_error")?;
    Ok(rows.clone())
}

fn decoded(s: &str) -> Result<String> {
    if !s.contains('&') {
        return Ok(s.to_string());
    }
    Node::parse(&format!(
        "<div>{}</div>",
        s.replace('<', "&lt;").replace('>', "&gt;")
    ))?
    .require("div")?
    .text()
}

fn manga(id: &str, title: &str) -> Value {
    json!({"id":id,"title":title,"links":[{"kind":"source","url":format!("{BASE}{id}")}]})
}

fn add_cover(item: &mut Value, raw: &str) -> Result<()> {
    if !raw.is_empty() {
        item["cover"] = json!({"kind":"url","url":port::absolute(BASE,raw)?.as_str()});
    }
    Ok(())
}

fn from_api(row: &Value) -> Result<Value> {
    let id = clean_path(row["url"].as_str().ok_or("parse_error")?, work_path)?;
    let title = decoded(row["name"].as_str().ok_or("parse_error")?)?;
    let mut entry = manga(&id, &title);
    add_cover(&mut entry, row["pic"].as_str().unwrap_or(""))?;
    if let Some(author) = row["author"].as_str().filter(|s| !s.is_empty()) {
        entry["authors"] = json!([decoded(author)?]);
    }
    if let Some(description) = row["content"].as_str() {
        entry["description"] = json!(decoded(description)?);
    }
    if let Some(tags) = row["tags"].as_array() {
        let tags = tags
            .iter()
            .filter_map(Value::as_str)
            .map(decoded)
            .collect::<Result<Vec<_>>>()?;
        entry["tags"] = json!(tags);
    }
    entry["status"] = json!(match row["serialize"].as_str().unwrap_or("") {
        "连载" | "连载中" | "連載中" => "ongoing",
        "完结" | "已完结" | "已完結" => "completed",
        _ => "unknown",
    });
    Ok(entry)
}

fn meta(page: &Node, id: &str) -> Result<Value> {
    let hero = page.require(".book-hero")?;
    let title = hero.require(".title")?.text()?;
    let mut entry = manga(id, &title);
    if let Some(img) = hero.one("img.bg") {
        add_cover(&mut entry, &img.attr("src")?)?;
    }
    if let Some(author) = page.one(".book-container__author") {
        let author = author.text()?;
        entry["authors"] = json!([author.trim_start_matches("作者：").trim()]);
    }
    let mut tags = Vec::new();
    for node in hero.all(".tags .item")? {
        tags.extend(node.text()?.split_whitespace().map(ToString::to_string));
    }
    entry["tags"] = json!(tags);
    if let Some(desc) = page.one(".book-container__detail") {
        entry["description"] = json!(desc.text()?);
    }
    Ok(entry)
}

fn details(id: &str) -> Result<Value> {
    meta(&plain(&format!("{BASE}{id}"))?, id)
}

fn search_text(query: &str, page: usize) -> Result<Value> {
    if let Ok(url) = url::Url::parse(query) {
        if url.host_str().is_some_and(station) && matches!(url.scheme(), "http" | "https") {
            let path = url.path();
            let id = if chapter_parts(path).is_some() {
                let chapter = plain(&format!("{BASE}{path}"))?;
                clean_path(
                    &chapter.require("a.btn-text[href]")?.attr("href")?,
                    work_path,
                )?
            } else if work_path(path) {
                path.to_string()
            } else {
                return Err("invalid_request");
            };
            return Ok(port::items(alloc::vec![details(&id)?], None));
        }
    }
    let mut url = port::absolute(BASE, "/search")?;
    if page == 1 {
        url.query_pairs_mut().append_pair("key", query);
    } else {
        url.path_segments_mut()
            .map_err(|_| "invalid_request")?
            .push(query)
            .push(&page.to_string());
    }
    let body = port::request(
        url.as_str(),
        "GET",
        json!({"Accept":"text/html","Sec-Fetch-User":"?1","User-Agent":port::MOBILE_IMAGE_AGENT}),
        None,
    )?;
    let doc = Node::parse(&body)?;
    if doc
        .one("body")
        .map(|node| node.text())
        .transpose()?
        .is_some_and(|s| s.contains("搜索繁忙"))
    {
        return Err("rate_limit");
    }
    let mut list = Vec::new();
    for link in doc.all(".cate-rows .item > a.main")? {
        let id = clean_path(&link.attr("href")?, work_path)?;
        let mut item = manga(&id, &link.require(".title")?.text()?);
        if let Some(image) = link.one("img") {
            add_cover(&mut item, &image.attr("src")?)?;
        }
        list.push(item);
    }
    let mut next = None;
    for anchor in doc.all(".fetch-status a.page")? {
        if anchor.text()? == "下一页" {
            next = anchor
                .attr("href")?
                .rsplit('/')
                .next()
                .and_then(|n| n.parse::<usize>().ok())
                .filter(|n| *n > page);
            break;
        }
    }
    Ok(port::items(list, next))
}

fn list(args: &Value, request: &[u8], search: bool) -> Result<Value> {
    let page = request_page_number(request);
    let query = args["query"].as_str().unwrap_or("").trim();
    if search && !query.is_empty() {
        return search_text(query, page);
    }
    let listing = args["listingId"].as_str().unwrap_or("popular");
    let sort = port::filter(args, "sort", SORTS)?;
    let order = match listing {
        "latest" => "addtime",
        "popular" if sort.is_empty() => "hits",
        "popular" => sort,
        _ => return Err("invalid_request"),
    };
    let mut url = port::absolute(BASE, "/api/data/comic")?;
    url.query_pairs_mut()
        .append_pair("page", &page.to_string())
        .append_pair("size", &PAGE_SIZE.to_string())
        .append_pair("order", order);
    if listing == "popular" {
        if let Some(tag) = args["filters"]["tag"].as_str().filter(|s| !s.is_empty()) {
            if !number(tag) {
                return Err("invalid_request");
            }
            url.query_pairs_mut().append_pair("type[tags]", tag);
        }
    }
    let data = api(url.as_str())?;
    let items = data
        .as_array()
        .ok_or("parse_error")?
        .iter()
        .map(from_api)
        .collect::<Result<Vec<_>>>()?;
    let next = if items.len() >= PAGE_SIZE {
        page.checked_add(1)
    } else {
        None
    };
    Ok(port::items(items, next))
}

fn work(args: &Value) -> Result<&str> {
    args["mangaId"]
        .as_str()
        .filter(|id| work_path(id))
        .ok_or("invalid_request")
}

fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(request)?;
    let owner = work(&input["args"])?;
    let page = plain(&format!("{BASE}{owner}"))?;
    page.require(".book-hero")?;
    let mut list = Vec::new();
    for node in page.all(".episode-list .desc_item > a")? {
        let id = clean_path(&node.attr("href")?, |p| chapter_parts(p).is_some())?;
        list.push((id, node.text()?));
    }
    let mut dates = BTreeMap::new();
    if let Some((first, _)) = list.first() {
        let (series, _) = chapter_parts(first).ok_or("parse_error")?;
        let rows = api(&format!("{BASE}/api/data/chapter?mid={series}"))?;
        for row in rows.as_array().ok_or("parse_error")? {
            if let (Some(id), Some(date)) = (row["id"].as_str(), row["addtime"].as_str()) {
                dates.insert(id.to_string(), date.to_string());
            }
        }
    }
    let mut chapters = Vec::new();
    for (j, (id, title)) in list.iter().rev().enumerate() {
        let (_, native) = chapter_parts(id).ok_or("parse_error")?;
        let mut chapter = Chapter::new(id.as_bytes(), owner.as_bytes(), title.as_bytes());
        chapter.index = Some(j);
        chapter.updated_at = dates.get(native).map(String::as_bytes);
        chapters.push(chapter);
    }
    ChapterResponse::ordered(payload_buf(), request, ChapterOrder::NewestFirst, &chapters)
        .map_err(port::chapter_error)
}

fn pages(args: &Value) -> Result<Value> {
    let id = args["chapterId"]
        .as_str()
        .filter(|p| chapter_parts(p).is_some())
        .ok_or("invalid_request")?;
    let body = port::request(
        &format!("{BASE}{id}"),
        "GET",
        json!({"User-Agent":READER_UA,"Accept":"text/html"}),
        None,
    )?;
    let page = Node::parse(&body)?;
    if let Some(owner) = args["mangaId"].as_str() {
        let back = page.require("a.btn-text[href]")?;
        if clean_path(&back.attr("href")?, work_path)? != owner {
            return Err("invalid_request");
        }
    }
    let entries = page.all("img[data-original]")?;
    let mut images = Vec::new();
    for (index, node) in entries.iter().enumerate() {
        let mut item = port::page(
            index,
            id,
            port::absolute(BASE, &node.attr("data-original")?)?,
        );
        item["image"]["headers"] = json!({"User-Agent":READER_UA});
        images.push(item);
    }
    if images.is_empty() {
        return Err("chapter_unavailable");
    }
    Ok(json!({"pages":images}))
}

fn filters() -> Result<Value> {
    let doc = plain(&format!("{BASE}/category/"))?;
    let mut options = alloc::vec![json!({"label":"全部","value":""})];
    for node in doc.all(".cate-box a[href*='/tags/']")? {
        let href = node.attr("href")?;
        let tag = href.trim_end_matches('/').rsplit('/').next().unwrap_or("");
        if number(tag) {
            options.push(json!({"label":node.text()?,"value":tag}));
        }
    }
    let mut sort = port::select("sort", "排序", "listing", SORTS);
    sort["default"] = json!("hits");
    Ok(
        json!({"filters":[sort,{"type":"select","name":"题材","id":"tag","scope":"listing","default":"","options":options}]}),
    )
}

fn dispatch(operation: &str, request: &[u8]) -> Result<Value> {
    let input = port::input(request)?;
    let args = &input["args"];
    match operation {
        "get_listings" => Ok(json!({"listings":[
            {"id":"popular","name":"热门","supportsPaging":true,"supportsFilters":true},
            {"id":"latest","name":"最近更新","supportsPaging":true,"supportsFilters":false}
        ]})),
        "get_filters" => filters(),
        "get_manga_list" => list(args, request, false),
        "search" => list(args, request, true),
        "get_manga" => Ok(json!({"manga":details(work(args)?)?})),
        "get_pages" => pages(args),
        _ => Err("not_implemented"),
    }
}

fn run(operation: &str, r: &[u8]) -> u32 {
    port::write_result(operation, dispatch(operation, r))
}
fn run_search(r: &[u8]) -> u32 {
    run("search", r)
}
fn run_get_manga(r: &[u8]) -> u32 {
    run("get_manga", r)
}
fn write_chapter_failure(code: &str) -> u32 {
    port::write_failure("get_chapters", code)
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
koma_source_sdk::koma_source_exports!("bikamanhua", typed_chapters, write_chapter_failure);
