#![no_std]
extern crate alloc;

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use koma_source_sdk::{
    chapters::{Chapter, ChapterOrder, ChapterResponse, ChapterResult},
    json_utils::request_page_number,
    source::{SourceCapabilities, SourceInfo},
    source_protocol,
};
use serde_json::{json, Value};
use url::{Position, Url};

#[allow(dead_code)]
#[path = "../../port_support.rs"]
mod port_support;
use port_support::{self as port, Node, Result};

const WEB: &str = "https://jcomic.net";
// The public web client sets this marker. It carries no personal session or account data.
const PUBLIC_COOKIE: &str = "jcomic_access=verified_user";
const SEARCH_MODES: &[(&str, &str)] = &[("search", "默认"), ("author", "作者")];
const CATEGORIES: &[&str] = &[
    "全彩",
    "長篇",
    "單行本",
    "同人",
    "短篇",
    "Cosplay",
    "歐美",
    "WEBTOON",
    "圓神領域",
    "碧藍幻想",
    "CG雜圖",
    "英語 ENG",
    "生肉",
    "純愛",
    "百合花園",
    "耽美花園",
    "偽娘哲學",
    "後宮閃光",
    "扶他樂園",
    "姐姐系",
    "妹妹系",
    "SM",
    "性轉換",
    "足の恋",
    "重口地帶",
    "人妻",
    "NTR",
    "強暴",
    "非人類",
    "艦隊收藏",
    "Love Live",
    "SAO 刀劍神域",
    "Fate",
    "東方",
    "禁書目錄",
];

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.jcomic.koma",
    name: "JComic",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "JComic 的公开分类、搜索和章节。",
    content_rating: "nsfw",
};
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    listings: true,
    manga_list: true,
    filters: true,
    image_request: true,
    ..SourceCapabilities::CORE
};

#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;
koma_source_sdk::koma_source_buffers! {
    payload: 1024 * 1024, http_out: 8 * 1024 * 1024, body: 5 * 1024 * 1024,
    http_req: 8192, scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

fn document(url: &Url) -> Result<Node> {
    let html = port::request(
        url.as_str(),
        "GET",
        json!({
            "Accept":"text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            "Cookie":PUBLIC_COOKIE
        }),
        None,
    )?;
    Node::parse(&html)
}

fn route(value: &str, chapter: bool) -> Result<Url> {
    let url = port::absolute(WEB, value)?;
    let path = url.path();
    let valid = path
        .strip_prefix("/page/")
        .is_some_and(|stem| !stem.is_empty())
        || (!chapter
            && path
                .strip_prefix("/eps/")
                .is_some_and(|stem| !stem.is_empty()));
    if url.origin() != Url::parse(WEB).map_err(|_| "internal_error")?.origin()
        || !valid
        || url.fragment().is_some()
    {
        return Err("invalid_request");
    }
    Ok(url)
}

fn id(url: &Url) -> &str {
    &url[Position::BeforePath..Position::AfterQuery]
}

fn work_url(input: &Value) -> Result<Url> {
    route(
        input["args"]["mangaId"].as_str().ok_or("invalid_request")?,
        false,
    )
}

fn strip_page_counts(raw: &str) -> (String, Option<&str>) {
    let mut rest = raw;
    let mut title = String::new();
    let mut page_count = None;
    while let Some((prefix, tail)) = rest.split_once('(') {
        title.push_str(prefix);
        let digits = tail.bytes().take_while(u8::is_ascii_digit).count();
        if digits > 0 && tail.as_bytes().get(digits) == Some(&b')') {
            page_count = Some(&tail[..digits]);
            rest = &tail[digits + 1..];
        } else {
            title.push('(');
            rest = tail;
        }
    }
    title.push_str(rest);
    (title.trim().to_string(), page_count)
}

fn work_card(url: &Url, data: &Node, cover: Option<&Node>) -> Result<Value> {
    let raw_title = data
        .one(".comic-title")
        .or_else(|| data.one("h1"))
        .ok_or("parse_error")?
        .text()?;
    let (title, count) = strip_page_counts(&raw_title);
    if title.is_empty() {
        return Err("parse_error");
    }
    let mut item = json!({
        "id":id(url), "title":title, "links":[{"kind":"source","url":url.as_str()}]
    });
    if let Some(count) = count {
        item["description"] = json!(format!("共 {count} 页"));
    }
    if let Some(author) = data.one(".list-content a[href^='/author/'], a[href^='/author/']") {
        item["authors"] = json!([author.text()?]);
    }
    let tags = data
        .all(".list-content a[href^='/cat/']")?
        .iter()
        .map(Node::text)
        .collect::<Result<Vec<_>>>()?;
    if !tags.is_empty() {
        item["tags"] = json!(tags);
    }
    if let Some(img) = cover {
        let src = img.attr("src").unwrap_or_default();
        if !src.is_empty() {
            let image = port::absolute(url.as_str(), &src)?;
            // Cookie-bearing requests use the image resolver, not the public-cover shortcut.
            item["cover"] = json!({"kind":"url","url":image.as_str()});
        }
    }
    Ok(item)
}

fn listing_url(kind: &str, key: &str, page: usize) -> Result<Url> {
    let mut url = port::absolute(WEB, "/")?;
    let page = page.to_string();
    {
        let mut segments = url.path_segments_mut().map_err(|_| "internal_error")?;
        segments.push(kind);
        for segment in key.split(['/', '\\']) {
            segments.push(segment);
        }
        segments.push(&page);
    }
    Ok(url)
}

fn list(input: &Value, request: &[u8], search: bool) -> Result<Value> {
    let args = &input["args"];
    let page = request_page_number(request);
    let (kind, key) = if search {
        let query = args["query"].as_str().ok_or("invalid_request")?;
        if query.trim().is_empty() {
            let category = args["filters"]["category"]
                .as_str()
                .unwrap_or(CATEGORIES[0]);
            if !CATEGORIES.contains(&category) {
                return Err("invalid_request");
            }
            ("cat", category)
        } else {
            let mode = port::filter(args, "mode", SEARCH_MODES)?;
            (if mode.is_empty() { "search" } else { mode }, query)
        }
    } else {
        match args["listingId"].as_str() {
            Some("popular") => ("cat", "隨機"),
            Some("latest") => ("cat", "最近更新"),
            _ => return Err("invalid_request"),
        }
    };
    let url = listing_url(kind, key, page)?;
    let doc = document(&url)?;
    let mut items = Vec::new();
    for row in doc.all(".container .col-lg-4")? {
        let link = row.require("a:nth-child(1)")?;
        let work = route(&link.attr("href")?, false)?;
        items.push(work_card(&work, &row, link.one("img").as_ref())?);
    }
    let next = doc
        .one(".pagination > li.active:not(:last-child)")
        .and_then(|_| page.checked_add(1));
    Ok(port::items(items, next))
}

fn catalog_date(doc: &Node) -> Option<String> {
    let text = doc.one(".comic-date")?.text().ok()?;
    let (_, value) = text.split_once("最後更新:")?;
    let value = value.trim();
    // The site omits a zone. Keep local wall time as the upstream parser does.
    let (day, clock) = value.split_once(' ')?;
    if day.len() != 10
        || clock.len() != 5
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || b"-: ".contains(&b))
    {
        return None;
    }
    Some(format!("{day}T{clock}:00"))
}

fn details(input: &Value) -> Result<Value> {
    let url = work_url(input)?;
    let detail_url = match id(&url).strip_prefix("/page/") {
        Some(stem) => route(&format!("/eps/{stem}"), false)?,
        None => url.clone(),
    };
    let mut doc = document(&detail_url)?;
    if url.path().starts_with("/page/") && doc.one(".comic-title, h1").is_none() {
        // One-shot works may have no /eps body. The public reader contains their title.
        doc = document(&url)?;
    }
    let cover = doc.one(".col-md-6:nth-child(1) img");
    let item = work_card(&url, &doc, cover.as_ref())?;
    Ok(json!({
        "manga":item,
        "chapterContext":{"mangaId":id(&url),"updatedAt":catalog_date(&doc)}
    }))
}

fn run_get_chapters(request: &[u8]) -> ChapterResult<'_, &'static str> {
    let input = port::input(request)?;
    let work = work_url(&input)?;
    let work_id = id(&work);
    if work.path().starts_with("/page/") {
        let context = &input["args"]["chapterContext"];
        let mut chapter = Chapter::new(work_id.as_bytes(), work_id.as_bytes(), "单章节".as_bytes());
        chapter.language = Some(b"zh");
        if context["mangaId"] == work_id {
            chapter.updated_at = context["updatedAt"].as_str().map(str::as_bytes);
        }
        return ChapterResponse::single(payload_buf(), chapter).map_err(port::chapter_error);
    }

    let doc = document(&work)?;
    let updated = catalog_date(&doc);
    let chapters = doc
        .all(".col-md-6:nth-child(2) a")?
        .iter()
        .rev()
        .map(|a| {
            let url = route(&a.attr("href")?, true)?;
            Ok((id(&url).to_string(), a.text()?))
        })
        .collect::<Result<Vec<_>>>()?;
    if chapters.is_empty() {
        return Err("parse_error");
    }
    let entries = chapters
        .iter()
        .map(|(id, name)| {
            let mut chapter = Chapter::new(id.as_bytes(), work_id.as_bytes(), name.as_bytes());
            chapter.language = Some(b"zh");
            chapter.updated_at = updated.as_deref().map(str::as_bytes);
            chapter
        })
        .collect::<Vec<_>>();
    ChapterResponse::ordered(payload_buf(), request, ChapterOrder::NewestFirst, &entries)
        .map_err(port::chapter_error)
}

fn page_image(img: &Node) -> Result<String> {
    let locked = img.attr("data-locked").unwrap_or_default();
    if locked.is_empty() {
        let lazy = img.attr("data-src").unwrap_or_default();
        return if lazy.is_empty() {
            img.attr("src")
        } else {
            Ok(lazy)
        };
    }
    // The snapshot uses TRAP; the live site's IntersectionObserver uses TRAPS.
    let encoded = locked
        .strip_prefix("JCOMIC_TRAPS_")
        .or_else(|| locked.strip_prefix("JCOMIC_TRAP_"))
        .unwrap_or(&locked);
    let reversed = encoded.bytes().rev().collect::<Vec<_>>();
    let url = STANDARD.decode(reversed).map_err(|_| "parse_error")?;
    String::from_utf8(url).map_err(|_| "parse_error")
}

fn pages(input: &Value) -> Result<Value> {
    let chapter = route(
        input["args"]["chapterId"]
            .as_str()
            .ok_or("invalid_request")?,
        true,
    )?;
    let doc = document(&chapter)?;
    let pages = doc
        .all(".comic-thumb")?
        .iter()
        .enumerate()
        .map(|(index, img)| {
            let image = port::absolute(chapter.as_str(), &page_image(img)?)?;
            Ok(port::page(index, id(&chapter), image))
        })
        .collect::<Result<Vec<_>>>()?;
    if pages.is_empty() {
        return Err("chapter_unavailable");
    }
    Ok(json!({"pages":pages}))
}

fn image_request(input: &Value, request: &[u8]) -> Result<Value> {
    let url = port::absolute(WEB, input["args"]["url"].as_str().ok_or("invalid_request")?)?;
    let host = url.host_str().ok_or("invalid_request")?;
    let headers = if url.scheme() == "https" && matches!(host, "jcomic.net" | "images.jcomic.net") {
        json!({"Cookie":PUBLIC_COOKIE})
    } else {
        json!({})
    };
    let mut image = json!({"url":url.as_str(),"headers":headers});
    if (host.starts_with("images.") || url.as_str().contains("jcomic-content"))
        && source_protocol::feature(request, b"source_image_response_errors_v1")
    {
        // The Host accepts permission_denied for 403. It has no expired-image error here.
        image["responseErrors"] = json!({"403":"permission_denied"});
    }
    Ok(image)
}

fn filters(request: &[u8]) -> Value {
    let modes = port::select("mode", "搜索类型", "search", SEARCH_MODES);
    let categories = CATEGORIES
        .iter()
        .map(|category| (*category, *category))
        .collect::<Vec<_>>();
    let mut category = port::select(
        "category",
        "分类（搜索关键字时无效）",
        "search",
        &categories,
    );
    category["default"] = json!(CATEGORIES[0]);
    let mut modes = modes;
    modes["default"] = json!("search");
    if source_protocol::feature(request, b"source_filter_search_without_query_v1") {
        json!({"filters":[modes,category],"useSearchForFilters":true})
    } else {
        json!({"filters":[modes]})
    }
}

fn dispatch(operation: &str, request: &[u8]) -> Result<Value> {
    let input = port::input(request)?;
    match operation {
        "get_listings" => Ok(port::listing_list(true, true, false)),
        "get_filters" => Ok(filters(request)),
        "search" => list(&input, request, true),
        "get_manga_list" => list(&input, request, false),
        "get_manga" => details(&input),
        "get_pages" => pages(&input),
        "get_image_request" => image_request(&input, request),
        _ => Err("not_implemented"),
    }
}
fn run(operation: &str, request: &[u8]) -> u32 {
    port::write_result(operation, dispatch(operation, request))
}
fn run_search(r: &[u8]) -> u32 {
    run("search", r)
}
fn run_get_manga(r: &[u8]) -> u32 {
    run("get_manga", r)
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
fn run_get_image_request(r: &[u8]) -> u32 {
    run("get_image_request", r)
}
fn run_get_settings(r: &[u8]) -> u32 {
    run("get_settings", r)
}
fn run_get_home(r: &[u8]) -> u32 {
    run("get_home", r)
}
fn chapter_failure(code: &str) -> u32 {
    port::write_failure("get_chapters", code)
}
koma_source_sdk::koma_source_exports!("jcomic", typed_chapters, chapter_failure);
