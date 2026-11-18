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
    chapters::{Chapter, ChapterResponse, ChapterResult},
    comments,
    json_utils::request_page_number,
    source::{SourceCapabilities, SourceInfo},
};
use serde_json::{json, Value};
use url::{form_urlencoded, Url};

#[path = "../../port_support.rs"]
mod port_support;
use port_support::{self as port, Node, Result};

const BASE: &str = "https://mikagogo.com";
const CATEGORIES: &[(&str, &str)] = &[
    ("", "全部"),
    ("7", "插画精选"),
    ("3", "P站画师"),
    ("2", "P站美图"),
    ("8", "图集画册"),
    ("5", "动漫壁纸"),
    ("6", "电脑壁纸"),
    ("4", "插画"),
    ("12", "COSPLAY"),
    ("9", "动漫快讯"),
    ("617", "动漫下载"),
    ("387", "周边"),
    ("1", "未分类"),
];
const SORTS: &[(&str, &str)] = &[
    ("", "跟随栏目"),
    ("date", "发布时间"),
    ("comment_count", "评论最多"),
    ("views", "浏览数量"),
    ("zan", "点赞最多"),
];
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.mikagogo.koma",
    name: "米卡插画",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "米卡插画公开图集及作品评论。",
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
    filters: true,
    settings: false,
    image_request: false,
    credentials: false,
};

#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;
koma_source_sdk::koma_source_buffers! {payload:1024*1024,http_out:4*1024*1024,body:3*1024*1024,http_req:4096,scratch:8192,}
koma_source_sdk::koma_source_helpers!();

fn work(id: &str) -> Result<&str> {
    if id.is_empty() || id.len() > 16 || !id.bytes().all(|ch| ch.is_ascii_digit()) {
        return Err("invalid_request");
    }
    Ok(id)
}
fn from_url(url: &Url) -> Result<&str> {
    if !matches!(url.host_str(), Some("mikagogo.com" | "www.mikagogo.com")) {
        return Err("not_found");
    }
    let id = url.path_segments().and_then(|mut segments| segments.next());
    work(id.ok_or("not_found")?).map_err(|_| "not_found")
}
fn entry(id: &str, title: &str) -> Value {
    json!({"id":id,"title":title,"links":[{"kind":"source","url":format!("{BASE}/{id}")}]})
}
fn doc(path: &str) -> Result<Node> {
    Node::parse(&port::get(&format!("{BASE}/{path}"))?)
}
fn cards(html: &str) -> Result<Vec<Value>> {
    let document = Node::parse(html)?;
    let mut items = Vec::new();
    let mut seen = BTreeSet::new();
    for tile in document.all(".kzpost-data")? {
        let Some(title) = tile.one(".posr-tit") else {
            continue;
        };
        let title = title.text()?;
        let url = port::absolute(BASE, &tile.require("a[href]")?.attr("href")?)?;
        let Ok(id) = from_url(&url) else {
            continue;
        };
        if title.is_empty() || !seen.insert(id.to_string()) {
            continue;
        }
        let mut item = entry(id, &title);
        if let Some(cover) = tile.one("img.cardImage[data-original]") {
            let image = port::absolute(BASE, &cover.attr("data-original")?)?;
            item["cover"] = json!({"kind":"url","url":image.as_str()});
        }
        items.push(item);
    }
    Ok(items)
}
fn list(args: &Value, request: &[u8], search: bool) -> Result<Value> {
    let page = request_page_number(request);
    let query = args["query"].as_str().unwrap_or("").trim();
    let category = port::filter(args, "category", CATEGORIES)?;
    if search && !query.is_empty() {
        if let Ok(url) = Url::parse(query) {
            if matches!(url.scheme(), "http" | "https") {
                return Ok(port::items(vec![details(from_url(&url)?)?], None));
            }
        }
        let path = if page > 1 {
            format!("/page/{page}")
        } else {
            "/".into()
        };
        let mut url = port::absolute(BASE, &path)?;
        url.query_pairs_mut().append_pair("s", query);
        if !category.is_empty() {
            url.query_pairs_mut().append_pair("cat", category);
        }
        let html = port::get(url.as_str())?;
        let has_next = Node::parse(&html)?.one("a.next.page-numbers").is_some();
        return Ok(port::items(cards(&html)?, next_page(has_next, page)));
    }
    let sort = port::filter(args, "sort", SORTS)?;
    let sort = match (sort, args["listingId"].as_str()) {
        ("", Some("latest")) => "date",
        ("", _) => "views",
        _ => sort,
    };
    let form = form_urlencoded::Serializer::new(String::new())
        .append_pair("action", "postlist_newajax")
        .append_pair("type", "sxpost")
        .append_pair("cat", if category.is_empty() { "0" } else { category })
        .append_pair("term", "category")
        .append_pair("paged", &page.to_string())
        .append_pair("orderby", sort)
        .finish();
    let body = port::request(
        &format!("{BASE}/wp-admin/admin-ajax.php"),
        "POST",
        json!({
            "Accept":"application/json",
            "Content-Type":"application/x-www-form-urlencoded",
            "Referer":format!("{BASE}/"),
            "X-Requested-With":"XMLHttpRequest"
        }),
        Some(&form),
    )?;
    let response: Value = serde_json::from_str(&body).map_err(|_| "parse_error")?;
    let data = response.get("data").ok_or("parse_error")?;
    let items = cards(data["html"].as_str().unwrap_or(""))?;
    let has_next = !items.is_empty() && data["starus"] == 1;
    Ok(port::items(items, next_page(has_next, page)))
}
fn next_page(more: bool, page: usize) -> Option<usize> {
    if more {
        page.checked_add(1)
    } else {
        None
    }
}
fn details(id: &str) -> Result<Value> {
    let document = doc(work(id)?)?;
    let title = document.require(".kz-single-data h1")?.text()?;
    let content = document.require("#content .content_left")?;
    let mut item = entry(id, &title);
    item["status"] = json!("completed");
    if let Some(author) = document.one(".kz-post-meta a[href*='/author/']") {
        item["authors"] = json!([author.text()?]);
    }
    let tags = document
        .all(".kz-post_tags_list a")?
        .iter()
        .map(Node::text)
        .collect::<Result<BTreeSet<_>>>()?;
    item["tags"] = json!(tags);
    let paragraphs = document
        .all("#content .content_left > p")?
        .iter()
        .filter(|paragraph| paragraph.one("img").is_none())
        .map(Node::text)
        .collect::<Result<Vec<_>>>()?;
    item["description"] = json!(paragraphs.join("\n\n"));
    if let Some(image) = content.one("img[src]") {
        let image = port::absolute(BASE, &image.attr("src")?)?;
        item["cover"] = json!({"kind":"url","url":image.as_str()});
    }
    Ok(item)
}
fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(request)?;
    let id = work(input["args"]["mangaId"].as_str().ok_or("invalid_request")?)?;
    let document = doc(id)?;
    let title = document.require(".kz-single-data h1")?.text()?;
    let chapter = Chapter::new(id.as_bytes(), id.as_bytes(), title.as_bytes());
    ChapterResponse::single(payload_buf(), chapter).map_err(port::chapter_error)
}
fn pages(args: &Value) -> Result<Value> {
    let id = work(args["chapterId"].as_str().ok_or("invalid_request")?)?;
    if args["mangaId"].as_str().is_some_and(|owner| owner != id) {
        return Err("invalid_request");
    }
    let first = doc(id)?;
    let pages = first
        .all(".page_imges a.page-numbers")?
        .iter()
        .filter_map(|link| link.text().ok()?.trim().parse::<usize>().ok())
        .max()
        .unwrap_or(1);
    if pages > 100 {
        return Err("result_too_large");
    }
    let mut output = Vec::new();
    for number in 1..=pages {
        let additional = if number > 1 {
            Some(doc(&format!("{id}/{number}"))?)
        } else {
            None
        };
        let document = additional.as_ref().unwrap_or(&first);
        let images = document.all("#content .content_left img[src]")?;
        if images.is_empty() {
            return Err("chapter_unavailable");
        }
        for image in images {
            let url = port::absolute(BASE, &image.attr("src")?)?;
            let mut page = port::page(output.len(), id, url);
            page["image"]["headers"] = port::mobile_image_headers(&format!("{BASE}/{id}"));
            output.push(page);
        }
    }
    Ok(json!({"pages":output}))
}
fn fetch_comments(request: &[u8]) -> Result<Value> {
    let input = comments::request(request)?;
    if input.chapter_id.is_some() || input.cursor.is_some() {
        return Err("invalid_request");
    }
    let id = work(core::str::from_utf8(input.manga_id).map_err(|_| "invalid_request")?)?;
    let document = doc(id)?;
    document.require(".kz-single-data h1")?;
    let mut items = Vec::new();
    for row in document.all(".kz-comment-list li.comment[id^='comment-']")? {
        let id = row.attr("id")?;
        let body = row.require(".commemt-main")?.text()?;
        let author = row.require(".user_info_name")?.text()?;
        let mut item = json!({"id":id, "author":author, "content":body});
        if let Some(time) = row.one("time[datetime]") {
            item["createdAt"] = json!(time.attr("datetime")?);
        }
        items.push(item);
    }
    Ok(json!({"items":items}))
}
fn dispatch(operation: &str, request: &[u8]) -> Result<Value> {
    let input = port::input(request)?;
    let args = &input["args"];
    match operation {
        "get_listings" => Ok(port::listing_list(true, true, true)),
        "get_filters" => Ok(json!({"filters":[
            port::select("category", "分类", "both", CATEGORIES),
            port::select("sort", "排序", "browse", SORTS)
        ]})),
        "get_manga_list" => list(args, request, false),
        "search" => list(args, request, true),
        "get_manga" => {
            Ok(json!({"manga":details(args["mangaId"].as_str().ok_or("invalid_request")?)?}))
        }
        "get_pages" => pages(args),
        _ => Err("not_implemented"),
    }
}
fn run(operation: &str, request: &[u8]) -> u32 {
    let result = dispatch(operation, request).map(|mut payload| {
        port::mobile_cover_headers(&mut payload, &format!("{BASE}/"), request);
        payload
    });
    port::write_result(operation, result)
}
fn run_search(request: &[u8]) -> u32 {
    run("search", request)
}
fn run_get_manga(request: &[u8]) -> u32 {
    run("get_manga", request)
}
fn write_chapter_failure(code: &str) -> u32 {
    port::write_failure("get_chapters", code)
}
fn run_get_pages(request: &[u8]) -> u32 {
    run("get_pages", request)
}
fn run_get_listings(request: &[u8]) -> u32 {
    run("get_listings", request)
}
fn run_get_manga_list(request: &[u8]) -> u32 {
    run("get_manga_list", request)
}
fn run_get_filters(request: &[u8]) -> u32 {
    run("get_filters", request)
}
fn run_get_settings(request: &[u8]) -> u32 {
    run("get_settings", request)
}
fn run_get_home(request: &[u8]) -> u32 {
    run("get_home", request)
}
fn run_get_image_request(request: &[u8]) -> u32 {
    run("get_image_request", request)
}
fn run_get_comments(request: &[u8]) -> u32 {
    port::write_result("get_comments", fetch_comments(request))
}
koma_source_sdk::koma_source_comments_export!(run_get_comments);
koma_source_sdk::koma_source_exports!("mikagogo", typed_chapters, write_chapter_failure);
