// SPDX-License-Identifier: Apache-2.0
// Behavior ported from Keiyoushi's YiqiqiManhua, Dto and Filters at 8947c37.
#![no_std]
extern crate alloc;

use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use koma_source_sdk::{
    chapters::{Chapter, ChapterNumber, ChapterResponse, ChapterResult},
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

const BASE: &str = "https://www.177pica.com/";
const CATEGORIES: &[(&str, &str, &str)] = &[
    ("30", "中文成人H漫畫", "html/category/tt"),
    ("24", "日文成人H漫畫", "html/category/jj"),
    ("32", "全彩CG", "html/category/cg"),
    ("36", "全彩CG(中文全彩)", "html/category/cg/cg-cn"),
    ("37", "全彩CG(日文全彩)", "html/category/cg/cg-jp"),
    ("35", "全彩CG(純圖無字)", "html/category/cg/cg-no"),
];

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.yiqiqimanhua.koma",
    name: "177漫畫",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "177漫畫的公開圖集。",
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
koma_source_sdk::koma_source_buffers! {
    payload: 1024*1024, http_out: 4*1024*1024, body: 3*1024*1024, http_req: 8192, scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

fn post_id(id: &str) -> Result<&str> {
    if !id.is_empty() && id.len() <= 20 && id.bytes().all(|byte| byte.is_ascii_digit()) {
        Ok(id)
    } else {
        Err("invalid_request")
    }
}

fn identity<'a>(args: &'a Value, field: &str, prefix: &str) -> Result<&'a str> {
    post_id(
        args[field]
            .as_str()
            .and_then(|id| id.strip_prefix(prefix))
            .ok_or("invalid_request")?,
    )
}

fn site(url: &Url) -> bool {
    matches!(url.host_str(), Some("www.177pica.com" | "177pica.com"))
}

fn permalink_id(url: &Url) -> Result<String> {
    if !site(url) {
        return Err("not_found");
    }
    if let Some((_, id)) = url.query_pairs().find(|(key, _)| key == "p") {
        return Ok(post_id(&id)?.to_string());
    }
    let filename = url.path().rsplit('/').next().unwrap_or("");
    Ok(post_id(filename.strip_suffix(".html").ok_or("not_found")?)?.to_string())
}

fn post_link(id: &str) -> String {
    format!("{BASE}?p={id}")
}

fn secure_image(base: &str, raw: &str) -> Result<Url> {
    let mut url = port::absolute(base, raw)?;
    url.set_scheme("https").map_err(|_| "parse_error")?;
    Ok(url)
}

fn get_doc(url: &Url) -> Result<Node> {
    Node::parse(&port::request(
        url.as_str(),
        "GET",
        json!({"Accept":"text/html"}),
        None,
    )?)
}

fn rest(route: &str, query: &[(&str, &str)]) -> Result<Value> {
    let mut url = Url::parse(BASE).map_err(|_| "internal_error")?;
    url.query_pairs_mut()
        .append_pair("rest_route", route)
        .extend_pairs(query.iter().copied());
    let body = port::request(
        url.as_str(),
        "GET",
        json!({"Accept":"application/json","X-Requested-With":"XMLHttpRequest"}),
        None,
    )?;
    serde_json::from_str(&body).map_err(|_| "parse_error")
}

fn post(id: &str) -> Result<Value> {
    rest(
        &format!("/wp/v2/posts/{id}"),
        &[("_fields", "title,date_gmt,categories,content")],
    )
}

fn item(id: &str, title: &str, cover: Option<Url>) -> Value {
    let cover = cover.map_or_else(
        || json!({"kind":"none"}),
        |url| json!({"kind":"url","url":url.as_str(),"headers":{}}),
    );
    json!({"id":format!("manga:{id}"),"title":title,"cover":cover,
        "url":post_link(id),"links":[{"kind":"source","url":post_link(id)}]})
}

fn card(base: &Url, link: Node, image: Option<Node>) -> Result<Value> {
    let id = permalink_id(&port::absolute(base.as_str(), &link.attr("href")?)?)?;
    let cover = image
        .and_then(|image| image.attr("src").ok())
        .and_then(|raw| port::absolute(base.as_str(), &raw).ok())
        .and_then(|thumb| {
            thumb
                .query_pairs()
                .find(|(key, _)| key == "src")
                .and_then(|(_, raw)| secure_image(base.as_str(), &raw).ok())
        });
    Ok(item(&id, &link.text()?, cover))
}

fn popular() -> Result<Value> {
    let newest = rest("/wp/v2/posts", &[("per_page", "1"), ("_fields", "link")])?;
    let url = port::absolute(BASE, newest[0]["link"].as_str().ok_or("parse_error")?)?;
    if !site(&url) {
        return Err("parse_error");
    }
    let doc = get_doc(&url)?;
    let ranking = doc.require("#hot_post_widget")?;
    let items = ranking
        .all("li")?
        .into_iter()
        .map(|row| card(&url, row.require(".new-title a[href]")?, row.one("img")))
        .collect::<Result<Vec<_>>>()?;
    Ok(port::items(items, None))
}

fn browse(args: &Value, request: &[u8], search: bool) -> Result<Value> {
    let query = if search {
        args["query"].as_str().unwrap_or("").trim()
    } else {
        ""
    };
    if let Ok(url) = Url::parse(query) {
        if matches!(url.scheme(), "http" | "https") {
            return Ok(port::items(vec![details(&permalink_id(&url)?)?], None));
        }
    }
    let category = if search {
        match args["filters"].get("category") {
            None => None,
            Some(Value::String(value)) if value.is_empty() => None,
            Some(Value::String(value)) => Some(
                CATEGORIES
                    .iter()
                    .find(|(id, _, _)| id == value)
                    .ok_or("invalid_request")?,
            ),
            Some(_) => return Err("invalid_request"),
        }
    } else {
        None
    };
    let page = request_page_number(request);
    let path = if query.is_empty() {
        category.map_or("", |(_, _, path)| path)
    } else {
        ""
    };
    let mut path = if path.is_empty() {
        String::new()
    } else {
        format!("{path}/")
    };
    if page > 1 {
        path.push_str(&format!("page/{page}/"));
    }
    let mut url = port::absolute(BASE, if path.is_empty() { "/" } else { &path })?;
    if !query.is_empty() {
        url.query_pairs_mut().append_pair("s", query);
        if let Some((id, _, _)) = category {
            url.query_pairs_mut().append_pair("cat", id);
        }
    }
    // A 429 remains rate_limited at the host boundary. The WASM ABI has no delayed retry.
    let doc = get_doc(&url)?;
    let main = doc.require("#main")?;
    let items = main
        .all("article")?
        .into_iter()
        .map(|article| {
            card(
                &url,
                article.require(".grid-title a[href]")?,
                article.one("figure img"),
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let next = doc
        .one("a.next.page-numbers")
        .and_then(|_| page.checked_add(1));
    Ok(port::items(items, next))
}

fn images(post: &Value) -> Result<Vec<Url>> {
    let content = post["content"]["rendered"].as_str().ok_or("parse_error")?;
    let doc = Node::parse(content)?;
    doc.all("img[src]")?
        .into_iter()
        .map(|img| secure_image(BASE, &img.attr("src")?))
        .collect()
}

fn details(id: &str) -> Result<Value> {
    let post = post(id)?;
    let rendered = post["title"]["rendered"].as_str().ok_or("parse_error")?;
    let title_doc = Node::parse(&format!(
        "<textarea>{}</textarea>",
        rendered.replace('<', "&lt;")
    ))?;
    let title = title_doc.require("textarea")?.text()?;
    if title.is_empty() {
        return Err("parse_error");
    }
    let tags = post["categories"]
        .as_array()
        .ok_or("parse_error")?
        .iter()
        .filter_map(|category| {
            let number = category.as_u64()?;
            CATEGORIES
                .iter()
                .find(|(id, _, _)| id.parse::<u64>().ok() == Some(number))
                .map(|(_, label, _)| *label)
        })
        .collect::<Vec<_>>();
    let mut value = item(id, &title, images(&post)?.into_iter().next());
    value["status"] = json!("completed");
    value["tags"] = json!(tags);
    Ok(value)
}

fn filters(request: &[u8]) -> Value {
    let mut options = vec![("", "全部")];
    options.extend(CATEGORIES.iter().map(|(id, label, _)| (*id, *label)));
    let filter_search = source_protocol::feature(request, b"source_filter_scope_header_v1")
        && source_protocol::feature(request, b"source_filter_search_without_query_v1");
    let mut filter = port::select("category", "分類", "search", &options);
    if !filter_search {
        filter.as_object_mut().unwrap().remove("scope");
    }
    json!({"filters":[filter],"useSearchForFilters":filter_search})
}

fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(request)?;
    let id = identity(&input["args"], "mangaId", "manga:")?;
    let post = post(id)?;
    let date = post["date_gmt"]
        .as_str()
        .filter(|date| !date.is_empty() && *date != "0000-00-00T00:00:00")
        .map(|date| format!("{date}Z"));
    let manga = format!("manga:{id}");
    let chapter_id = format!("chapter:{id}");
    let mut chapter = Chapter::new(chapter_id.as_bytes(), manga.as_bytes(), b"Ch. 1");
    chapter.chapter_number = Some(ChapterNumber::Integer(1));
    chapter.language = Some(b"zh");
    chapter.published_at = date.as_deref().map(str::as_bytes);
    ChapterResponse::single(payload_buf(), chapter).map_err(port::chapter_error)
}

fn pages(args: &Value) -> Result<Value> {
    let chapter = args["chapterId"].as_str().ok_or("invalid_request")?;
    let id = post_id(chapter.strip_prefix("chapter:").ok_or("invalid_request")?)?;
    if !args["mangaId"].is_null() && identity(args, "mangaId", "manga:")? != id {
        return Err("invalid_request");
    }
    let images = images(&post(id)?)?;
    if images.is_empty() {
        return Err("chapter_unavailable");
    }
    let pages = images
        .into_iter()
        .enumerate()
        .map(|(index, image)| port::page(index, chapter, image))
        .collect::<Vec<_>>();
    Ok(json!({"pages":pages}))
}

fn dispatch(operation: &str, request: &[u8]) -> Result<Value> {
    let input = port::input(request)?;
    let args = &input["args"];
    match operation {
        "get_listings" => Ok(port::listing_list(true, false, false)),
        "get_manga_list" if args["listingId"] == "popular" => popular(),
        "get_manga_list" if args["listingId"] == "latest" => browse(args, request, false),
        "get_manga_list" => Err("invalid_request"),
        "search" => browse(args, request, true),
        "get_filters" => Ok(filters(request)),
        "get_manga" => Ok(json!({"manga":details(identity(args, "mangaId", "manga:")?)?})),
        "get_pages" => pages(args),
        _ => Err("not_implemented"),
    }
}

fn run_search(request: &[u8]) -> u32 {
    port::write_result("search", dispatch("search", request))
}
fn run_get_manga(request: &[u8]) -> u32 {
    port::write_result("get_manga", dispatch("get_manga", request))
}
fn run_get_pages(request: &[u8]) -> u32 {
    port::write_result("get_pages", dispatch("get_pages", request))
}
fn run_get_listings(request: &[u8]) -> u32 {
    port::write_result("get_listings", dispatch("get_listings", request))
}
fn run_get_manga_list(request: &[u8]) -> u32 {
    port::write_result("get_manga_list", dispatch("get_manga_list", request))
}
fn run_get_filters(request: &[u8]) -> u32 {
    port::write_result("get_filters", dispatch("get_filters", request))
}
fn run_get_settings(_: &[u8]) -> u32 {
    port::write_failure("get_settings", "not_implemented")
}
fn run_get_home(_: &[u8]) -> u32 {
    port::write_failure("get_home", "not_implemented")
}
fn run_get_image_request(_: &[u8]) -> u32 {
    port::write_failure("get_image_request", "not_implemented")
}
fn write_chapter_failure(code: &str) -> u32 {
    port::write_failure("get_chapters", code)
}
koma_source_sdk::koma_source_exports!("yiqiqimanhua", typed_chapters, write_chapter_failure);
