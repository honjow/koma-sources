// SPDX-License-Identifier: Apache-2.0
// Behavior ported from Keiyoushi's ManhuaManga and MCCMSWeb at 8947c37.
#![no_std]
extern crate alloc;

use alloc::{
    collections::BTreeSet,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use koma_source_sdk::{
    chapters::{Chapter, ChapterOrder, ChapterResponse, ChapterResult},
    json_utils::request_page_number,
    source::{SourceCapabilities, SourceInfo},
    source_protocol,
};
use serde_json::{json, Value};
use url::Url;

#[path = "../../port_support.rs"]
#[allow(dead_code)]
mod port_support;
use port_support::{self as port, Node, Result};

const BASE: &str = "https://www.mh03.com";
const AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:150.0) Gecko/20100101 Firefox/150.0";
const VERIFY_ID: &str = "manga:/search";
const VERIFY_MESSAGE: &str = "请点击 WebView 按钮输入验证码，完成后返回重新搜索";
const STATUS: &[(&str, &str)] = &[("", "全部"), ("1", "连载中"), ("2", "已完结")];
const SORT: &[(&str, &str)] = &[("hits", "热门人气"), ("addtime", "最新更新")];

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.manhuamanga.koma",
    name: "漫画manga",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "漫畫manga（mh03.com）的中文漫畫書庫，收錄《一人之下》等分話連載，並按題材整理連載及完結作品。",
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
#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;
koma_source_sdk::koma_source_buffers! {
    payload: 1024*1024, http_out: 6*1024*1024, body: 4*1024*1024, http_req: 8192, scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

fn document(url: &Url) -> Result<Node> {
    let html = port::request(
        url.as_str(),
        "GET",
        json!({"Accept":"text/html","User-Agent":AGENT}),
        None,
    )?;
    if html
        .trim_start()
        .starts_with("很遗憾，该漫画不存在或章节已被删除。")
    {
        return Err("not_found");
    }
    Node::parse(&html)
}

fn site(url: &Url) -> bool {
    matches!(url.host_str(), Some("mh03.com" | "www.mh03.com"))
}
fn route(raw: &str, prefix: &str) -> Result<String> {
    let url = port::absolute(BASE, raw)?;
    let path = url.path().strip_prefix("/index.php").unwrap_or(url.path());
    let valid = path
        .strip_prefix(prefix)
        .and_then(|slug| slug.strip_suffix(".html"))
        .is_some_and(|slug| !slug.is_empty() && !slug.contains('/'));
    if !site(&url) || !valid {
        return Err("invalid_request");
    }
    Ok(path.to_string())
}

fn identity(args: &Value, key: &str, kind: &str, route_prefix: &str) -> Result<String> {
    let route_value = args[key]
        .as_str()
        .and_then(|value| value.strip_prefix(kind))
        .ok_or("invalid_request")?;
    route(route_value, route_prefix)
}

fn cover(doc: &Node, selector: &str, base: &Url) -> Value {
    let image = doc
        .one(selector)
        .and_then(|image| image.attr("src").ok())
        .and_then(|raw| port::absolute(base.as_str(), &raw).ok());
    image.map_or_else(
        || json!({"kind":"none"}),
        |url| json!({"kind":"url","url":url.as_str(),"headers":{"Referer":format!("{BASE}/")}}),
    )
}

fn row(path: &str, title: &str, cover: Value) -> Value {
    json!({"id":format!("manga:{path}"),"title":title,"cover":cover,
        "url":format!("{BASE}{path}"),"links":[{"kind":"source","url":format!("{BASE}{path}")}]})
}

fn verification() -> Value {
    let mut manga = row("/search", "验证码", json!({"kind":"none"}));
    manga["description"] = json!(VERIFY_MESSAGE);
    manga
}

fn tag_segment(value: &str) -> Result<&str> {
    if value.is_empty()
        || value == "."
        || value == ".."
        || value.contains(['/', '\\', '?', '#'])
        || value.bytes().any(|byte| byte.is_ascii_control())
    {
        Err("invalid_request")
    } else {
        Ok(value)
    }
}

fn browse(args: &Value, request: &[u8], search: bool) -> Result<Value> {
    let page = request_page_number(request);
    let query = if search {
        args["query"].as_str().unwrap_or("")
    } else {
        ""
    };
    if let Ok(url) = Url::parse(query.trim()) {
        if matches!(url.scheme(), "https" | "http") {
            return Ok(port::items(vec![details(&work_for_url(&url)?)?], None));
        }
    }
    let mut url = port::absolute(BASE, "/")?;
    if !query.trim().is_empty() {
        url.path_segments_mut()
            .map_err(|_| "internal_error")?
            .extend(["search", query, &page.to_string()]);
    } else {
        let mut path = String::from("/category/");
        if search {
            let status = port::filter(args, "status", STATUS)?;
            if !status.is_empty() {
                path.push_str(&format!("finish/{status}/"));
            }
            let sort = args["filters"]
                .get("sort")
                .map(|_| port::filter(args, "sort", SORT))
                .transpose()?
                .unwrap_or("hits");
            path.push_str(&format!("order/{sort}/"));
            if let Some(value) = args["filters"].get("genre") {
                let genre = value.as_str().ok_or("invalid_request")?;
                if !genre.is_empty() {
                    path.push_str(&format!("tags/{}/", tag_segment(genre)?));
                }
            }
        } else {
            let sort = match args["listingId"].as_str() {
                Some("popular") => "hits",
                Some("latest") => "addtime",
                _ => return Err("invalid_request"),
            };
            path.push_str(&format!("order/{sort}/"));
        }
        path.push_str(&format!("page/{page}"));
        url.set_path(&path);
    }
    let doc = document(&url)?;
    if search && doc.one("#code-div").is_some() {
        return Ok(port::items(vec![verification()], None));
    }
    let list = doc.require(".comic-list")?;
    let rows = list
        .all(".comic-item")?
        .into_iter()
        .map(|card| {
            let link = card.require("h3 a[href]")?;
            let path = route(&link.attr("href")?, "/comic_")?;
            Ok(row(&path, &link.text()?, cover(&card, "img[src]", &url)))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut next = None;
    if let Some(pagination) = doc.one(".pagination") {
        if let Some(link) = pagination.one("a.next[href]") {
            let href = link.attr("href")?;
            let current = pagination
                .one("a.on[href]")
                .map(|link| link.attr("href"))
                .transpose()?;
            if !href.is_empty() && Some(href.as_str()) != current.as_deref() {
                next = page.checked_add(1);
            }
        }
    }
    Ok(port::items(rows, next))
}

fn work_for_url(url: &Url) -> Result<String> {
    if !site(url) {
        return Err("not_found");
    }
    if url.path().starts_with("/comic_") {
        return route(url.as_str(), "/comic_");
    }
    route(url.as_str(), "/chapter_")?;
    let doc = document(url)?;
    for script in doc.all("script[type='application/ld+json']")? {
        let Ok(data) = serde_json::from_str::<Value>(&script.text()?) else {
            continue;
        };
        if data["@type"] == "BreadcrumbList" {
            if let Some(series) = data["itemListElement"][1]["item"].as_str() {
                return route(series, "/comic_");
            }
        }
    }
    Err("not_found")
}

fn details(path: &str) -> Result<Value> {
    let url = port::absolute(BASE, path)?;
    let doc = document(&url)?;
    let info = doc.require(".comic-main-info")?;
    let title = info.require("h1")?.text()?;
    let title = title.trim_start_matches('《').trim_end_matches('》');
    if title.is_empty() {
        return Err("parse_error");
    }
    let mut value = row(
        path,
        title,
        cover(&doc, ".comic-cover-large img[src]", &url),
    );
    let stats = info
        .all(".stat-item span")?
        .into_iter()
        .map(|item| item.text())
        .collect::<Result<Vec<_>>>()?;
    if let Some(author) = stats.iter().find_map(|s| {
        s.split_once("作者：")
            .map(|(_, name)| name.trim())
            .filter(|name| !name.is_empty())
    }) {
        value["authors"] = json!([author]);
    }
    let tags = info
        .all(".comic-tags .tag")?
        .into_iter()
        .map(|item| item.text())
        .collect::<Result<Vec<_>>>()?;
    value["tags"] = json!(tags
        .first()
        .map(|text| text.split_whitespace().collect::<Vec<_>>())
        .unwrap_or_default());
    value["status"] = json!(match tags.get(1).map(String::as_str) {
        Some("连载" | "连载中") => "ongoing",
        Some("完结" | "已完结") => "completed",
        _ => "unknown",
    });
    let description = doc
        .one(".comic-description p")
        .map(|node| node.text())
        .transpose()?
        .unwrap_or_default();
    let rating = stats
        .iter()
        .find_map(|s| {
            s.split_once("评分：")
                .and_then(|(_, rating)| rating.trim().parse::<f32>().ok())
        })
        .filter(|rating| rating.is_finite());
    value["description"] = json!(rating.map_or(description.clone(), |rating| format!(
        "评分：{rating}\n\n{description}"
    )));
    Ok(value)
}

fn filters(args: &Value, request: &[u8]) -> Result<Value> {
    if !source_protocol::feature(request, b"source_filter_scope_header_v1")
        || !source_protocol::feature(request, b"source_filter_search_without_query_v1")
    {
        return Ok(json!({"filters":[]}));
    }
    let mut sort = port::select("sort", "排序", "search", SORT);
    sort["default"] = json!("hits");
    let mut schema = vec![
        json!({"type":"header","scope":"search","name":"分类"}),
        port::select("status", "状态", "search", STATUS),
        sort,
    ];
    let dynamic = source_protocol::feature(request, b"source_filter_fetching_v1");
    if dynamic {
        let mut genre =
            json!({"type":"select","scope":"search","id":"genre","name":"漫画类型","default":""});
        if args["fetchFilterData"] == true {
            let doc = document(&port::absolute(BASE, "/category/")?)?;
            doc.require(".filter-options")?;
            let mut values = vec![json!({"value":"","label":"全部"})];
            for link in doc.all(".filter-options a[href*='/tags/']")? {
                let url = port::absolute(BASE, &link.attr("href")?)?;
                if site(&url) {
                    let value = url.path().rsplit('/').next().ok_or("parse_error")?;
                    values.push(json!({"value":tag_segment(value)?,"label":link.text()?}));
                }
            }
            if values.len() > 1 {
                genre["options"] = json!(values);
            } else {
                return Err("parse_error");
            }
        }
        schema.push(genre);
    }
    Ok(json!({"filters":schema,"supportsFilterFetching":dynamic,"useSearchForFilters":true}))
}

fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(request)?;
    let args = &input["args"];
    let mut loaded = Vec::new();
    let owner = args["mangaId"].as_str().ok_or("invalid_request")?;
    if owner != VERIFY_ID {
        let path = identity(args, "mangaId", "manga:", "/comic_")?;
        let doc = document(&port::absolute(BASE, &path)?)?;
        doc.require(".comic-main-info h1")?;
        let mut seen = BTreeSet::new();
        for link in doc
            .all("#chapter-list .chapter-item > a[href]")?
            .into_iter()
            .rev()
        {
            let path = route(&link.attr("href")?, "/chapter_")?;
            if seen.insert(path.clone()) {
                loaded.push((format!("chapter:{path}"), link.text()?));
            }
        }
    }
    let chapters = loaded
        .iter()
        .map(|(id, title)| {
            let mut chapter = Chapter::new(id.as_bytes(), owner.as_bytes(), title.as_bytes());
            chapter.language = Some(b"zh");
            chapter
        })
        .collect::<Vec<_>>();
    ChapterResponse::ordered(payload_buf(), request, ChapterOrder::NewestFirst, &chapters)
        .map_err(port::chapter_error)
}

fn pages(args: &Value) -> Result<Value> {
    let path = identity(args, "chapterId", "chapter:", "/chapter_")?;
    let chapter = args["chapterId"].as_str().ok_or("invalid_request")?;
    let url = port::absolute(BASE, &path)?;
    let doc = document(&url)?;
    let pages = doc
        .all("img[data-src]")?
        .into_iter()
        .enumerate()
        .map(|(i, row)| {
            let image = port::absolute(url.as_str(), &row.attr("data-src")?)?;
            Ok(port::page(i, chapter, image))
        })
        .collect::<Result<Vec<_>>>()?;
    if pages.is_empty() {
        return Err("chapter_unavailable");
    }
    Ok(json!({"pages":pages}))
}

fn image_request(args: &Value) -> Result<Value> {
    let url = Url::parse(args["url"].as_str().ok_or("invalid_request")?)
        .map_err(|_| "invalid_request")?;
    let url = port::absolute(BASE, url.as_str())?;
    let page_id = args["pageId"].as_str().unwrap_or("");
    let mut headers = if page_id.starts_with("manga:") {
        json!({"Referer":format!("{BASE}/")})
    } else {
        json!({"User-Agent":AGENT})
    };
    // This CDN returns a 200 HTML document for direct requests. The chapter
    // Referer makes it serve the same bytes that the browser's reader gets.
    if url.host_str() == Some("manhua.5um.net") {
        if let Some((path, _index)) = page_id
            .strip_prefix("chapter:")
            .and_then(|id| id.rsplit_once(':'))
        {
            let path = route(path, "/chapter_")?;
            headers["Referer"] = json!(port::absolute(BASE, &path)?.as_str());
        }
    }
    Ok(json!({"url":url.as_str(),"headers":headers}))
}

fn dispatch(operation: &str, request: &[u8]) -> Result<Value> {
    let input = port::input(request)?;
    let args = &input["args"];
    match operation {
        "get_listings" => Ok(port::listing_list(true, true, false)),
        "get_manga_list" => browse(args, request, false),
        "search" => browse(args, request, true),
        "get_filters" => filters(args, request),
        "get_manga" if args["mangaId"] == VERIFY_ID => Ok(json!({"manga":verification()})),
        "get_manga" => {
            Ok(json!({"manga":details(&identity(args, "mangaId", "manga:", "/comic_")?)?}))
        }
        "get_pages" => pages(args),
        "get_image_request" => image_request(args),
        _ => Err("not_implemented"),
    }
}
fn run_search(r: &[u8]) -> u32 {
    port::write_result("search", dispatch("search", r))
}
fn run_get_manga(r: &[u8]) -> u32 {
    port::write_result("get_manga", dispatch("get_manga", r))
}
fn run_get_pages(r: &[u8]) -> u32 {
    port::write_result("get_pages", dispatch("get_pages", r))
}
fn run_get_listings(r: &[u8]) -> u32 {
    port::write_result("get_listings", dispatch("get_listings", r))
}
fn run_get_manga_list(r: &[u8]) -> u32 {
    port::write_result("get_manga_list", dispatch("get_manga_list", r))
}
fn run_get_filters(r: &[u8]) -> u32 {
    port::write_result("get_filters", dispatch("get_filters", r))
}
fn run_get_settings(_: &[u8]) -> u32 {
    port::write_failure("get_settings", "not_implemented")
}
fn run_get_home(_: &[u8]) -> u32 {
    port::write_failure("get_home", "not_implemented")
}
fn run_get_image_request(r: &[u8]) -> u32 {
    port::write_result("get_image_request", dispatch("get_image_request", r))
}
fn write_chapter_failure(code: &str) -> u32 {
    port::write_failure("get_chapters", code)
}
koma_source_sdk::koma_source_exports!("manhuamanga", typed_chapters, write_chapter_failure);
