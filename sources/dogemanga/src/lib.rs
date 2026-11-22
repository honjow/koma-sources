#![no_std]
extern crate alloc;
use alloc::{
    collections::{BTreeMap, BTreeSet},
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use koma_source_sdk::{
    chapters::{Chapter, ChapterPage, ChapterPlacement, ChapterResult, GroupedChapters},
    json_utils::request_page_number,
    source::{SourceCapabilities, SourceInfo},
};
use serde_json::{json, Value};
use url::Url;
#[path = "../../port_support.rs"]
mod port_support;
use port_support::{self as port, Node, Result};

const BASE: &str = "https://dogemanga.com";
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.dogemanga.koma",
    name: "漫畫狗",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "漫畫狗。",
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
    image_request: true,
    credentials: false,
};
#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;
koma_source_sdk::koma_source_buffers! {payload:1024*1024,http_out:8*1024*1024,body:6*1024*1024,http_req:4096,scratch:8192,}
koma_source_sdk::koma_source_helpers!();

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}
fn id(value: &Value) -> Result<&str> {
    value
        .as_str()
        .filter(|id| valid_id(id))
        .ok_or("invalid_request")
}
fn work(id: &str) -> Result<Node> {
    Node::parse(&port::get(&format!("{BASE}/m/{id}?l=en"))?)
}
fn card(node: &Node, id: &str, detail: bool) -> Result<Value> {
    let title = node.require(".site-card__manga-title")?.text()?;
    let mut manga =
        json!({"id":id,"title":title,"links":[{"kind":"source","url":format!("{BASE}/m/{id}")}]});
    let cover = if detail {
        "img.site-manga__cover-image"
    } else {
        "img.card-img-top"
    };
    if let Some(cover) = node.one(cover) {
        let raw = cover.attr("src")?;
        manga["cover"] = json!({"kind":"url","url":port::absolute(BASE,&raw)?.as_str()});
    }
    if detail {
        let mut authors = Vec::new();
        for author in node.all("h4 a")? {
            authors.push(author.text()?);
        }
        manga["authors"] = json!(authors);
        if let Some(description) = node.one(".site-card__brief") {
            manga["description"] = json!(description.text()?);
        }
        if let Some(status) = node.one("small") {
            let text = status.text()?;
            manga["status"] = json!(if text.contains("status: Ongoing") {
                "ongoing"
            } else if text.contains("status: Finished") {
                "completed"
            } else {
                "unknown"
            });
        }
    }
    Ok(manga)
}
fn detail(id: &str) -> Result<Value> {
    card(
        &work(id)?.require(".site-main-content > .site-card")?,
        id,
        true,
    )
}
fn cursor(raw: &str) -> Result<Url> {
    let url = port::absolute(BASE, raw)?;
    if url.scheme() != "https"
        || url.host_str() != Some("dogemanga.com")
        || url.path() != "/_search"
        || url.fragment().is_some()
    {
        return Err("invalid_request");
    }
    Ok(url)
}
fn directory(request: &[u8], args: &Value, search: bool) -> Result<Value> {
    let keyword = args["query"].as_str().unwrap_or("").trim();
    let n = request_page_number(request);
    let mut url = port::absolute(BASE, "/_search")?;
    if search {
        if let Ok(url) = Url::parse(keyword) {
            if matches!(url.scheme(), "https" | "http") {
                let id = url
                    .path()
                    .strip_prefix("/m/")
                    .filter(|id| valid_id(id))
                    .ok_or("not_found")?;
                if url.host_str() != Some("dogemanga.com") {
                    return Err("not_found");
                }
                return Ok(port::items(vec![detail(id)?], None));
            }
        }
        let start = (n.saturating_sub(1))
            .checked_mul(24)
            .ok_or("invalid_request")?;
        url.query_pairs_mut().append_pair("q", keyword);
        if start > 0 {
            url.query_pairs_mut().append_pair("o", &start.to_string());
        }
    } else if let Some(next) = args["page"]["cursor"].as_str() {
        url = cursor(next)?;
    } else if args["listingId"] == "latest" {
        url.query_pairs_mut().append_pair("s", "1");
    }
    let html = port::request(
        url.as_str(),
        "GET",
        json!({"X-Requested-With":"XMLHttpRequest","Referer":BASE,"Accept":"application/json"}),
        None,
    )?;
    let data: Value = serde_json::from_str(&html).map_err(|_| "parse_error")?;
    let rows = data["manga_cards"].as_array().ok_or("parse_error")?;
    let mut items = Vec::new();
    for raw in rows {
        let doc = Node::parse(raw.as_str().ok_or("parse_error")?)?;
        let Some(node) = doc.one(".site-card[data-manga-id]") else {
            continue;
        };
        if node.one(".site-card__manga-title").is_none() {
            continue;
        }
        let id = node.attr("data-manga-id")?;
        if !valid_id(&id) {
            return Err("parse_error");
        }
        items.push(card(&node, &id, false)?);
    }
    let next = match data["next"].as_str() {
        Some(raw) if !raw.is_empty() => Some(if search {
            n.checked_add(1).ok_or("invalid_request")?.to_string()
        } else {
            cursor(raw)?.to_string()
        }),
        _ => None,
    };
    Ok(json!({"items":items,"page":{"hasMore":next.is_some(),"nextCursor":next}}))
}
fn chapter_link_id(link: &Node) -> Result<String> {
    let url = port::absolute(BASE, &link.attr("href")?)?;
    if url.host_str() != Some("dogemanga.com") {
        return Err("parse_error");
    }
    url.path()
        .strip_prefix("/p/")
        .filter(|id| valid_id(id))
        .map(ToString::to_string)
        .ok_or("parse_error")
}
fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(request)?;
    let owner = id(&input["args"]["mangaId"])?;
    let doc = work(owner)?;
    let mut seen = BTreeSet::new();
    let mut rows = Vec::new();
    for link in doc.all("#site-manga__tab-pane-all .site-manga-thumbnail__link")? {
        let chapter = chapter_link_id(&link)?;
        if seen.insert(chapter.clone()) {
            rows.push((chapter, link.require("span.text-center")?.text()?));
        }
    }

    // The provider's complete panes run newest first. Keep the upstream All
    // display array, but use its actual Chapter/Volume panes for independent
    // reading sequences. Titles/numbers never decide identity or eligibility.
    let mut placements = BTreeMap::new();
    for pane in doc.all(".tab-pane[id^=site-manga__tab-pane-]")? {
        let pane_id = pane.attr("id")?;
        let kind = pane_id.strip_prefix("site-manga__tab-pane-").ok_or("parse_error")?;
        if kind == "all" {
            continue;
        }
        if !valid_id(kind) {
            return Err("parse_error");
        }
        let group = format!("dogemanga:{owner}:{kind}");
        let mut group_seen = BTreeSet::new();
        let mut chapters = Vec::new();
        for link in pane.all(".site-manga-thumbnail__link")? {
            let chapter = chapter_link_id(&link)?;
            if seen.contains(&chapter) && group_seen.insert(chapter.clone()) {
                chapters.push(chapter);
            }
        }
        for (index, chapter) in chapters.iter().enumerate() {
            placements.entry(chapter.clone()).or_insert_with(|| {
                (group.clone(), (chapters.len() - index - 1) as u64)
            });
        }
    }
    // Preserve entries present only in All as their own complete provider
    // subsequence instead of dropping them or guessing a type from the title.
    let remaining: Vec<_> = rows.iter().filter(|(id, _)| !placements.contains_key(id)).collect();
    for (index, (chapter, _)) in remaining.iter().enumerate() {
        placements.insert(chapter.to_string(), (
            format!("dogemanga:{owner}:all"),
            (remaining.len() - index - 1) as u64,
        ));
    }
    let mut catalog = GroupedChapters::new(payload_buf(), request).map_err(port::chapter_error)?;
    for (index, (chapter, title)) in rows.iter().enumerate() {
        let mut row = Chapter::new(chapter.as_bytes(), owner.as_bytes(), title.as_bytes());
        row.index = Some(index);
        let (group, position) = placements.get(chapter).ok_or("parse_error")?;
        catalog.push(row, ChapterPlacement::InGroup {
            group: group.as_bytes(), position: *position,
        }).map_err(port::chapter_error)?;
    }
    catalog.finish(ChapterPage::Complete).map_err(port::chapter_error)
}

fn pages(args: &Value) -> Result<Value> {
    let chapter = id(&args["chapterId"])?;
    let url = format!("{BASE}/p/{chapter}");
    let doc = Node::parse(&port::get(&url)?)?;
    let images = doc.all("img.site-reader__image[data-page-image-url]")?;
    if images.is_empty() {
        return Err("chapter_unavailable");
    }
    let pages = images
        .into_iter()
        .enumerate()
        .map(|(index, node)| {
            let raw = node.attr("data-page-image-url")?;
            Ok(port::page(index, chapter, port::absolute(&url, &raw)?))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(json!({"pages":pages}))
}
fn image_request(args: &Value) -> Result<Value> {
    let url = port::absolute(BASE, args["url"].as_str().ok_or("invalid_request")?)?;
    let referer = format!("{BASE}/");
    // Reader image request headers are returned here. Cover headers still
    // travel with the work card whenever the host supports them.
    Ok(json!({"url":url.as_str(),"headers":port::mobile_image_headers(&referer)}))
}
fn dispatch(op: &str, request: &[u8]) -> Result<Value> {
    let input = port::input(request)?;
    let args = &input["args"];
    match op {
        "get_listings" => Ok(port::listing_list(true, true, false)),
        "search" => directory(request, args, true),
        "get_manga_list" => directory(request, args, false),
        "get_manga" => Ok(json!({"manga":detail(id(&args["mangaId"])?)?})),
        "get_pages" => pages(args),
        "get_image_request" => image_request(args),
        _ => Err("not_implemented"),
    }
}
fn run(op: &str, r: &[u8]) -> u32 {
    let result = dispatch(op, r).map(|mut value| {
        if matches!(op, "get_manga_list" | "search" | "get_manga") {
            port::mobile_cover_headers(&mut value, "https://dogemanga.com/", r);
        }
        value
    });
    port::write_result(op, result)
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
koma_source_sdk::koma_source_exports!("dogemanga", typed_chapters, write_chapter_failure);
