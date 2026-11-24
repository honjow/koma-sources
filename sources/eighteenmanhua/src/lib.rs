#![no_std]
extern crate alloc;
use alloc::{
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

const BASE: &str = "https://18mh.org";
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "org.18mh.koma",
    name: "18漫画",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "18mh.org 漫画目录",
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

fn segment(part: &str) -> bool {
    !part.is_empty()
        && part.len() <= 320
        && !matches!(part, "." | "..")
        && part
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'%'))
        && !part.to_ascii_lowercase().contains("%2f")
        && !part.to_ascii_lowercase().contains("%5c")
}
fn same_site(host: &str) -> bool {
    matches!(host, "18mh.org" | "www.18mh.org")
}
fn numeric(id: &str) -> bool {
    !id.is_empty() && id.len() <= 24 && id.bytes().all(|b| b.is_ascii_digit())
}
fn page_doc(url: &str) -> Result<Node> {
    Node::parse(&port::get(url)?)
}
fn request_doc(url: &str) -> Result<Node> {
    Node::parse(&port::request(
        url,
        "GET",
        json!({
            "Accept":"text/html,application/json","X-Requested-With":"XMLHttpRequest","Referer":format!("{BASE}/")
        }),
        None,
    )?)
}
fn manga_url(slug: &str) -> String {
    format!("{BASE}/manga/{slug}")
}
fn path_key(href: &str) -> Result<String> {
    let url = port::absolute(BASE, href)?;
    let key = url
        .path()
        .trim_end_matches('/')
        .strip_prefix("/manga/")
        .ok_or("parse_error")?;
    if !url.host_str().is_some_and(same_site) || !key.split('/').all(segment) {
        return Err("parse_error");
    }
    Ok(key.to_string())
}
fn manga(slug: &str, title: &str) -> Value {
    json!({"id":slug,"title":title,"links":[{"kind":"source","url":manga_url(slug)}]})
}
fn cover(raw: &str) -> Result<Value> {
    let url = port::absolute(BASE, raw)?;
    let direct = url.query_pairs().find(|(key, _)| key == "url");
    let url = match direct {
        Some((_, value)) => port::absolute(BASE, &value)?,
        None => url,
    };
    Ok(json!({"kind":"url","url":url.as_str()}))
}
fn filters() -> Result<Value> {
    let doc = page_doc(&format!("{BASE}/hots/page/1"))?;
    let mut options = alloc::vec![json!({"label":"全部","value":""})];
    for node in doc.all("main a[href^='/manga-genre/']")? {
        let href = node.attr("href")?;
        let genre = href.strip_prefix("/manga-genre/").unwrap_or("");
        if segment(genre) && !options.iter().any(|item| item["value"] == href) {
            let label = node.text()?;
            options.push(json!({"label":label.trim_start_matches('#'),"value":href}));
        }
    }
    Ok(
        json!({"filters":[{"id":"genre","name":"分类","type":"select","scope":"listing","default":"","options":options}]}),
    )
}
fn list(args: &Value, request: &[u8], search: bool) -> Result<Value> {
    let page = request_page_number(request);
    let term = args["query"].as_str().unwrap_or("").trim();
    let listing = args["listingId"].as_str().unwrap_or("popular");
    let url = if search && !term.is_empty() {
        let mut url = port::absolute(BASE, "/s")?;
        url.path_segments_mut()
            .map_err(|_| "invalid_request")?
            .push(term);
        url.query_pairs_mut().append_pair("page", &page.to_string());
        url
    } else if listing == "latest" {
        port::absolute(BASE, &format!("/newss/page/{page}"))?
    } else if listing == "popular" {
        let genre = args["filters"]["genre"].as_str().filter(|v| !v.is_empty());
        let path = match genre {
            Some(path) if path.strip_prefix("/manga-genre/").is_some_and(segment) => path,
            Some(_) => return Err("invalid_request"),
            None => "/hots",
        };
        port::absolute(BASE, &format!("{path}/page/{page}"))?
    } else {
        return Err("invalid_request");
    };
    let doc = page_doc(url.as_str())?;
    let mut items = Vec::new();
    for link in doc.all(".container > .cardlist .pb-2 a")? {
        let key = path_key(&link.attr("href")?)?;
        if !segment(&key) {
            continue;
        }
        let mut item = manga(&key, &link.require("h3")?.text()?);
        item["cover"] = cover(&link.require("img")?.attr("src")?)?;
        items.push(item);
    }
    let next = if doc.one("a[aria-label='下一頁'] button").is_some() {
        page.checked_add(1)
    } else {
        None
    };
    Ok(port::items(items, next))
}
fn details(slug: &str, doc: &Node) -> Result<Value> {
    let heading = doc.require("main h1")?;
    let raw = heading.text()?;
    let state = heading
        .one("span")
        .map(|node| node.text())
        .transpose()?
        .unwrap_or_default();
    let title = if state.is_empty() {
        raw.as_str()
    } else {
        raw.trim_end_matches(&state)
            .trim_start_matches(&state)
            .trim()
    };
    let mut row = manga(slug, title);
    row["cover"] = cover(&doc.require("main img.object-cover")?.attr("src")?)?;
    row["status"] = json!(match state.as_str() {
        "連載中" | "Ongoing" => "ongoing",
        "完結" => "completed",
        "停止更新" => "cancelled",
        "休刊" => "hiatus",
        _ => "unknown",
    });
    let mut tags = Vec::new();
    let mut authors = Vec::new();
    for node in doc.all("main a[href^='/manga-genre/'], main a[href^='/manga-tag/'], main a[href^='/manga-author/']")?{
        let href=node.attr("href")?;
        let text=node.text()?;
        if href.starts_with("/manga-author/"){authors.push(text);}
        else{tags.push(text.trim_start_matches('#').to_string());}
    }
    if !tags.is_empty() {
        row["tags"] = json!(tags);
    }
    if !authors.is_empty() {
        row["authors"] = json!(authors);
    }
    if let Some(desc) = doc.one("main p.line-clamp-4") {
        row["description"] = json!(desc.text()?);
    }
    Ok(row)
}
fn work(args: &Value) -> Result<&str> {
    args["mangaId"]
        .as_str()
        .filter(|s| segment(s))
        .ok_or("invalid_request")
}

fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(request)?;
    let owner = work(&input["args"])?;
    let doc = page_doc(&manga_url(owner))?;
    let manga_id = doc.require("#mangachapters")?.attr("data-mid")?;
    if !numeric(&manga_id) {
        return Err("parse_error");
    }
    let doc = request_doc(&format!("{BASE}/manga/get?mid={manga_id}&mode=all"))?;
    let mut entries = Vec::new();
    for row in doc.all(".chapteritem")? {
        let anchor = row.require("a")?;
        let key = path_key(&anchor.attr("href")?)?;
        if !key.split('/').next().is_some_and(|slug| slug == owner) {
            return Err("parse_error");
        }
        let chapter_id = anchor.attr("data-cs")?;
        if !numeric(&chapter_id) {
            return Err("parse_error");
        }
        entries.push((
            format!("{key}:{manga_id}:{chapter_id}"),
            anchor.attr("data-ct")?,
        ));
    }
    let mut chapters = Vec::new();
    for (index, (id, title)) in entries.iter().rev().enumerate() {
        let mut chapter = Chapter::new(id.as_bytes(), owner.as_bytes(), title.as_bytes());
        chapter.index = Some(index);
        chapters.push(chapter);
    }
    ChapterResponse::ordered(payload_buf(), request, ChapterOrder::NewestFirst, &chapters)
        .map_err(port::chapter_error)
}
fn pages(args: &Value) -> Result<Value> {
    let id = args["chapterId"].as_str().ok_or("invalid_request")?;
    let (key, rest) = id.split_once(':').ok_or("invalid_request")?;
    let (manga_id, chapter_id) = rest.split_once(':').ok_or("invalid_request")?;
    if !numeric(manga_id) || !numeric(chapter_id) || !key.split('/').all(segment) {
        return Err("invalid_request");
    }
    if let Some(owner) = args["mangaId"].as_str() {
        if key.split('/').next() != Some(owner) {
            return Err("invalid_request");
        }
    }
    let doc = request_doc(&format!(
        "{BASE}/chapter/getcontent?m={manga_id}&c={chapter_id}"
    ))?;
    let rows = doc.all("#chapcontent > div > img")?;
    let mut pages = Vec::new();
    for (index, image) in rows.iter().enumerate() {
        let raw = match image
            .attr("data-src")
            .ok()
            .filter(|value| !value.is_empty())
        {
            Some(url) => url,
            None => image.attr("src")?,
        };
        pages.push(port::page(index, id, port::absolute(BASE, &raw)?));
    }
    if pages.is_empty() {
        return Err("chapter_unavailable");
    }
    Ok(json!({"pages":pages}))
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
        "get_manga" => {
            let slug = work(args)?;
            Ok(json!({"manga":details(slug,&page_doc(&manga_url(slug))?)?}))
        }
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
koma_source_sdk::koma_source_exports!("eighteenmanhua", typed_chapters, write_chapter_failure);
