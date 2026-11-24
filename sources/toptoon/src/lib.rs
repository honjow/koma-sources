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
use serde_core::de::{Deserialize, Deserializer, IgnoredAny, MapAccess, Visitor};
use serde_json::{json, Value};
use url::Url;

#[allow(dead_code)]
#[path = "../../port_support.rs"]
mod port;
use port::{Node, Result};

const WEB: &str = "https://www.toptoon.net";
const COVERS: &str = "https://tw-contents-image.toptoon.net";
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.toptoon.koma",
    name: "TOPTOON頂通",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "zh-Hant",
    author: "Koma",
    description: "TOPTOON頂通目錄與原站開放閱讀的章節。",
    content_rating: "unknown",
};
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    listings: true,
    manga_list: true,
    image_request: true,
    ..SourceCapabilities::CORE
};

#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;
koma_source_sdk::koma_source_buffers! {payload:1024*1024,http_out:6*1024*1024,body:5*1024*1024,http_req:4096,scratch:8192,}
koma_source_sdk::koma_source_helpers!();

fn official(url: &Url) -> bool {
    url.scheme() == "https"
        && url.port().is_none()
        && matches!(url.host_str(), Some("www.toptoon.net" | "toptoon.net"))
}

fn document(url: &Url) -> Result<String> {
    if !official(url) {
        return Err("invalid_request");
    }
    port::request(
        url.as_str(),
        "GET",
        json!({
            "Cookie":"adultStatus=1",
            "Accept":"text/html,application/xhtml+xml"
        }),
        None,
    )
}

fn work_url(id: &str) -> Result<Url> {
    if id.is_empty()
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err("invalid_request");
    }
    port::absolute(WEB, &format!("/comic/epList/{id}"))
}

fn manga_from_link(link: &Url) -> Result<Url> {
    if !official(link) {
        return Err("invalid_request");
    }
    let mut path = link.path_segments().ok_or("invalid_request")?;
    match (path.next(), path.next(), path.next()) {
        (Some("comic"), Some("epList" | "epView"), Some(id)) => work_url(id),
        _ => Err("invalid_request"),
    }
}

fn episode_manga(link: &Url) -> Result<Url> {
    if !link.path().starts_with("/comic/epView/") {
        return Err("invalid_request");
    }
    manga_from_link(link)
}

fn route(url: &Url) -> String {
    match url.query() {
        Some(query) => format!("{}?{query}", url.path()),
        None => url.path().to_string(),
    }
}

fn manga_card(url: &Url, title: &str, cover: Option<Url>, author: Option<&str>) -> Value {
    let mut manga = json!({
        "id":url.path(), "title":title, "language":"zh-Hant",
        "links":[{"kind":"source","url":url.as_str()}]
    });
    if let Some(cover) = cover {
        manga["cover"] = json!({"kind":"url","url":cover.as_str()});
    }
    if let Some(author) = author.filter(|author| !author.is_empty()) {
        manga["authors"] = json!([author]);
    }
    manga
}

fn field_text(node: &Node, selector: &str) -> Result<Option<String>> {
    node.one(selector).map(|field| field.text()).transpose()
}

// The provider's search response is a JSON object in display order. Preserve that
// order, including ties in the latest list, without enabling std in the WASM.
struct SearchRows(Vec<Value>);

impl<'de> Deserialize<'de> for SearchRows {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> core::result::Result<Self, D::Error> {
        struct SearchVisitor;
        impl<'de> Visitor<'de> for SearchVisitor {
            type Value = SearchRows;
            fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter.write_str("the TOPTOON search catalog")
            }
            fn visit_map<M: MapAccess<'de>>(
                self,
                mut map: M,
            ) -> core::result::Result<SearchRows, M::Error> {
                let mut rows = Vec::new();
                while let Some((_, row)) = map.next_entry::<IgnoredAny, Value>()? {
                    rows.push(row);
                }
                Ok(SearchRows(rows))
            }
        }
        deserializer.deserialize_map(SearchVisitor)
    }
}

fn catalog_data(popular: bool) -> Result<Vec<Value>> {
    let entry = port::absolute(WEB, if popular { "/ranking" } else { "/search" })?;
    let html = document(&entry)?;
    let (marker, end) = if popular {
        ("jsonFileUrl: [\"", '"')
    } else {
        ("var jsonFileUrl = '", '\'')
    };
    let raw = html
        .split_once(marker)
        .and_then(|(_, tail)| tail.split_once(end).map(|(url, _)| url))
        .ok_or("parse_error")?;
    let url = port::absolute(WEB, &raw.replace("\\/", "/"))?;
    if url.scheme() != "https" {
        return Err("parse_error");
    }
    let json = port::request(
        url.as_str(),
        "GET",
        json!({
            "Referer":entry.as_str(), "Accept":"application/json"
        }),
        None,
    )?;
    if popular {
        let mut catalog: Value = serde_json::from_str(&json).map_err(|_| "parse_error")?;
        match catalog["adult"].take() {
            Value::Array(rows) => Ok(rows),
            _ => Err("parse_error"),
        }
    } else {
        serde_json::from_str::<SearchRows>(&json)
            .map(|rows| rows.0)
            .map_err(|_| "parse_error")
    }
}

fn json_card(row: &Value) -> Result<Value> {
    let id = row["id"].as_str().ok_or("parse_error")?;
    let url = work_url(id).map_err(|_| "parse_error")?;
    let title = row["meta"]["title"].as_str().ok_or("parse_error")?;
    let author = row["meta"]["author"]["authorString"].as_str();
    let thumb = &row["thumbnail"]["standard"];
    let cover = thumb
        .as_str()
        .or_else(|| thumb.get(0).and_then(Value::as_str))
        .filter(|image| !image.is_empty())
        .map(|image| port::absolute(COVERS, image))
        .transpose()?;
    Ok(manga_card(&url, title, cover, author))
}

fn list(args: &Value, page: usize, listing: Option<&str>) -> Result<Value> {
    if page > 1 {
        return Ok(port::items(Vec::new(), None));
    }
    let query = args["query"].as_str().unwrap_or_default();
    if listing.is_none() && query.starts_with("https://") {
        if let Ok(link) = Url::parse(query) {
            if let Ok(url) = manga_from_link(&link) {
                return Ok(port::items(alloc::vec![details_url(&url)?], None));
            }
        }
    }
    let popular = listing == Some("popular");
    let mut rows = catalog_data(popular)?;
    if listing == Some("latest") {
        rows.sort_by(|a, b| publication(b).cmp(publication(a)));
    }
    let mut items = Vec::new();
    let query = query.to_lowercase();
    for row in rows {
        let card = json_card(&row)?;
        let matches = listing.is_some()
            || card["title"]
                .as_str()
                .is_some_and(|title| title.to_lowercase().contains(&query))
            || card["authors"]
                .get(0)
                .and_then(Value::as_str)
                .is_some_and(|author| author.to_lowercase().contains(&query));
        if matches {
            items.push(card);
        }
    }
    Ok(port::items(items, None))
}

fn publication(row: &Value) -> &str {
    row["lastUpdated"]
        .get("pubDate")
        .and_then(Value::as_str)
        .unwrap_or("0")
}

fn manga_arg(args: &Value) -> Result<Url> {
    let id = args["mangaId"].as_str().ok_or("invalid_request")?;
    work_url(id.strip_prefix("/comic/epList/").ok_or("invalid_request")?)
}

fn details_url(url: &Url) -> Result<Value> {
    let document = Node::parse(&document(url)?)?;
    let info = document.require("section.infoContent")?;
    let title = info.require("div.title")?.text()?;
    let cover = document
        .one("div.comicThumb img")
        .map(|image| image.attr("src"))
        .transpose()?
        .filter(|value| !value.is_empty())
        .map(|image| port::absolute(url.as_str(), &image))
        .transpose()?;
    let credits = field_text(&info, "div.etc")?;
    let author = credits.as_deref().map(|text| {
        text.split_once("作家 :")
            .map_or(text, |(_, author)| author)
            .split('|')
            .next()
            .unwrap_or_default()
            .trim()
    });
    let mut manga = manga_card(url, &title, cover, author);
    if let Some(description) = field_text(&document, "div.comic_story div.desc")? {
        manga["description"] = json!(description);
    }
    let mut tags = Vec::new();
    for node in info.all("div.hashTag a")? {
        let tag = node.text()?.trim_start_matches('#').to_string();
        if !tag.is_empty() && !tags.contains(&tag) {
            tags.push(tag);
        }
    }
    manga["tags"] = json!(tags);
    if document.one("div.etc span.comicDayBox").is_some() {
        manga["status"] = json!("ongoing");
    }
    Ok(manga)
}

fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(request)?;
    let url = manga_arg(&input["args"])?;
    let document = Node::parse(&document(&url)?)?;
    document.require("section.episode_area ul.list_area")?;
    let rows = document.all("section.episode_area ul.list_area li.episodeBox")?;
    let rows = rows
        .into_iter()
        .rev()
        .map(|row| {
            let href = row.require("a")?.attr("href")?;
            let chapter = port::absolute(url.as_str(), &href)?;
            if episode_manga(&chapter).map_err(|_| "parse_error")?.path() != url.path() {
                return Err("parse_error");
            }
            let locked = row
                .one("button.coin, button.gift, button.waitFree")
                .is_some();
            let lock = if locked { "🔒" } else { "" };
            let title = format!(
                "{lock}{} {}",
                row.require("div.title")?.text()?,
                row.require("div.subTitle")?.text()?
            );
            let date = field_text(&row, "div.pubDate")?
                .filter(|date| {
                    date.len() == 10
                        && date
                            .bytes()
                            .all(|byte| byte.is_ascii_digit() || byte == b'-')
                })
                .map(|date| format!("{date}T00:00:00+08:00"));
            Ok((route(&chapter), title, locked, date))
        })
        .collect::<Result<Vec<_>>>()?;
    let chapters: Vec<_> = rows
        .iter()
        .map(|(id, title, locked, date)| {
            let mut chapter = Chapter::new(id.as_bytes(), url.path().as_bytes(), title.as_bytes());
            chapter.locked = Some(*locked);
            chapter.language = Some(b"zh-Hant");
            chapter.published_at = date.as_deref().map(str::as_bytes);
            chapter
        })
        .collect();
    ChapterResponse::ordered(payload_buf(), request, ChapterOrder::NewestFirst, &chapters)
        .map_err(port::chapter_error)
}

fn pages(args: &Value) -> Result<Value> {
    let id = args["chapterId"].as_str().ok_or("invalid_request")?;
    let url = port::absolute(WEB, id)?;
    let manga = episode_manga(&url)?;
    let document = Node::parse(&document(&url)?)?;
    let mut pages = Vec::new();
    for img in document.all("article.epContent section.imgWrap div.cImg img")? {
        let image = port::absolute(url.as_str(), &img.attr("data-src")?)?;
        let mut page = port::page(pages.len(), id, image);
        page["image"]["headers"] = json!({"Referer":url.as_str()});
        pages.push(page);
    }
    if pages.is_empty() {
        // Only the provider's flags establish a lock. An empty viewer alone
        // can also be an unavailable episode or a site markup change.
        let catalog = Node::parse(&self::document(&manga)?)?;
        for row in catalog.all("section.episode_area ul.list_area li.episodeBox")? {
            let Some(link) = row.one("a") else { continue };
            if port::absolute(manga.as_str(), &link.attr("href")?)
                .ok()
                .as_ref()
                == Some(&url)
                && row
                    .one("button.coin, button.gift, button.waitFree")
                    .is_some()
            {
                return Err("chapter_locked");
            }
        }
        return Err("chapter_unavailable");
    }
    Ok(json!({"pages":pages}))
}

fn run(op: &str, request: &[u8]) -> u32 {
    port::write_result(
        op,
        port::input(request).and_then(|input| {
            let args = &input["args"];
            match op {
                "search" => list(args, request_page_number(request), None),
                "get_manga_list" => {
                    let listing = match args["listingId"].as_str() {
                        Some(value @ ("popular" | "latest")) => value,
                        _ => return Err("invalid_request"),
                    };
                    list(args, request_page_number(request), Some(listing))
                }
                "get_listings" => Ok(port::listing_list(false, false, false)),
                "get_manga" => Ok(json!({"manga":details_url(&manga_arg(args)?)?})),
                "get_pages" => pages(args),
                "get_image_request" => {
                    let url = port::absolute(WEB, args["url"].as_str().ok_or("invalid_request")?)?;
                    Ok(json!({"url":url.as_str(),"headers":{"Referer":format!("{WEB}/")}}))
                }
                _ => Err("unsupported_operation"),
            }
        }),
    )
}

fn run_search(r: &[u8]) -> u32 {
    run("search", r)
}
fn run_get_listings(r: &[u8]) -> u32 {
    run("get_listings", r)
}
fn run_get_manga_list(r: &[u8]) -> u32 {
    run("get_manga_list", r)
}
fn run_get_manga(r: &[u8]) -> u32 {
    run("get_manga", r)
}
fn run_get_pages(r: &[u8]) -> u32 {
    run("get_pages", r)
}
fn run_get_image_request(r: &[u8]) -> u32 {
    run("get_image_request", r)
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
fn chapter_failure(code: &str) -> u32 {
    port::write_failure("get_chapters", code)
}

koma_source_sdk::koma_source_exports!("toptoon", typed_chapters, chapter_failure);
