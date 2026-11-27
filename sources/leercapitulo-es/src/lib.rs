#![no_std]
extern crate alloc;

use alloc::{
    collections::BTreeSet,
    format,
    string::{String, ToString},
    vec::Vec,
};
use koma_source_sdk::{
    chapters::{
        Chapter, ChapterNumber, ChapterOrder, ChapterResponse, ChapterResult, DecimalChapterNumber,
    },
    source::{SourceCapabilities, SourceInfo},
};
use serde_json::{json, Value};
use url::Url;

mod filters;
#[path = "../../port_support.rs"]
#[allow(dead_code)]
mod port;
#[path = "../../port_catalog.rs"]
mod port_catalog;

use port::{Node, Result};
use port_catalog::Labels;

#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

const SITE: &str = "https://www.leercapitulo.co/";
const DOMAIN: &str = "www.leercapitulo.co";
const METADATA: &[u8] = include_bytes!("../metadata.json");
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.leercapitulo.es.koma",
    name: "LeerCapitulo",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "es",
    description: "Spanish manga, manhwa and manhua, including One Piece and Nano máquina, with genre and theme catalogues.",
    author: "Koma source contributors",
    content_rating: "sfw",
};
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: true,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: true,
    manga_list: true,
    filters: true,
    settings: false,
    home: false,
    credentials: false,
    image_request: false,
};
const FILTERS: &[(&str, &[(&str, &str)])] = &[
    ("genre", filters::GENRE),
    ("theme", filters::THEME),
    ("type", filters::TYPE),
    ("status", filters::STATUS),
    ("sort", filters::SORT),
];
koma_source_sdk::koma_source_buffers! {
    payload: 768 * 1024, http_out: 2 * 1024 * 1024, body: 1024 * 1024,
    http_req: 4096, scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

fn site_url(raw: &str) -> Result<Url> {
    let url = port::absolute(SITE, raw)?;
    if url.scheme() != "https"
        || url.host_str() != Some(DOMAIN)
        || url.port().is_some()
        || url.fragment().is_some()
    {
        return Err("invalid_request");
    }
    Ok(url)
}

fn work_key(key: &str) -> Result<&str> {
    let (id, slug) = key.split_once('/').ok_or("invalid_request")?;
    if !matches!(id.len(), 8 | 10)
        || !id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        || slug.is_empty()
        || slug.len() > 250
        || !slug
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'%'))
    {
        return Err("invalid_request");
    }
    Ok(key)
}

fn work(args: &Value) -> Result<&str> {
    work_key(
        args["mangaId"]
            .as_str()
            .and_then(|id| id.strip_prefix("manga:"))
            .ok_or("invalid_request")?,
    )
}

fn link_key(url: &Url, prefix: &str) -> Result<String> {
    if url.query().is_some() {
        return Err("parse_error");
    }
    Ok(url
        .path()
        .strip_prefix(prefix)
        .and_then(|s| s.strip_suffix('/'))
        .ok_or("parse_error")?
        .into())
}

fn chapter_key(key: &str) -> Result<(&str, &str)> {
    let (work, chapter) = key.rsplit_once('/').ok_or("invalid_request")?;
    work_key(work)?;
    if chapter.is_empty()
        || chapter.len() > 64
        || matches!(chapter, "." | "..")
        || !chapter
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
    {
        return Err("invalid_request");
    }
    Ok((work, chapter))
}

fn doc(url: &Url) -> Result<Node> {
    let body = port::get(url.as_str())?;
    Node::parse(&body)
}

fn cover(node: &Node, selector: &str) -> Value {
    node.one(selector)
        .and_then(|img| site_url(&img.attr("src").ok()?).ok())
        .filter(|url| url.path().starts_with("/covers/"))
        .map_or_else(
            || json!({"kind":"none"}),
            |url| json!({"kind":"url","url":url.as_str(),"headers":{}}),
        )
}

fn card(key: &str, title: &str, cover: Value) -> Value {
    json!({"id":format!("manga:{key}"),"title":title,"url":format!("{SITE}manga/{key}/"),
        "cover":cover,"language":"es","contentRating":"sfw"})
}

fn page_number(args: &Value) -> Result<usize> {
    match &args["page"]["cursor"] {
        Value::Null => Ok(1),
        Value::String(s) => s
            .parse::<usize>()
            .ok()
            .filter(|p| (1..=100_000).contains(p) && p.to_string() == *s)
            .ok_or("invalid_request"),
        _ => Err("invalid_request"),
    }
}

fn selected<'a>(args: &'a Value, id: &str, options: &[(&str, &str)]) -> Result<&'a str> {
    match &args["filters"][id] {
        Value::Null => Ok(if id == "sort" { "az" } else { "" }),
        Value::String(s) if s.is_empty() => Ok(if id == "sort" { "az" } else { "" }),
        Value::String(s) if options.iter().any(|(key, _)| key == s) => Ok(s),
        _ => Err("invalid_request"),
    }
}

fn browse(args: &Value, search: bool) -> Result<Value> {
    let page = page_number(args)?;
    let listing = args["listingId"].as_str().unwrap_or("catalog");
    let listing = listing.strip_prefix("listing:").unwrap_or(listing);
    let catalog = search || listing == "catalog";
    let (mut url, selector, title_selector) = if catalog {
        (site_url("manga/")?, "article.lc-card", "a.lc-card-name")
    } else {
        if page != 1 {
            return Err("invalid_request");
        }
        match listing {
            "popular" => (site_url(SITE)?, ".lc-slide", "a.lc-slide-name"),
            "latest" => (site_url(SITE)?, "article.lc-release", "a.lc-release-title"),
            _ => return Err("invalid_request"),
        }
    };
    if catalog {
        let query = match &args["query"] {
            Value::Null => "",
            Value::String(q) if search && q.len() <= 512 && !q.chars().any(char::is_control) => {
                q.trim()
            }
            _ => return Err("invalid_request"),
        };
        let mut params = url.query_pairs_mut();
        if !query.is_empty() {
            params.append_pair("q", query);
        }
        for (id, options) in FILTERS {
            let value = selected(args, id, options)?;
            if !value.is_empty() {
                params.append_pair(id, value);
            }
        }
        if page > 1 {
            params.append_pair("page", &page.to_string());
        }
    }
    let document = doc(&url)?;
    if catalog {
        document.require("form[data-filter-form]")?;
    }
    let mut seen = BTreeSet::new();
    let mut items = Vec::new();
    for item in document.all(selector)? {
        let title = item.require(title_selector)?;
        let link = site_url(&title.attr("href")?)?;
        let key = link_key(&link, "/manga/")?;
        work_key(&key)?;
        if seen.insert(key.clone()) {
            items.push(card(&key, &title.text()?, cover(&item, "img")));
        }
    }
    if !catalog && items.is_empty() {
        return Err("parse_error");
    }
    let next = catalog && !items.is_empty() && document.one("a.page-link[rel=next]").is_some();
    Ok(port::items(items, next.then_some(page + 1)))
}

fn detail(args: &Value, request: &[u8]) -> Result<Value> {
    let key = work(args)?;
    let document = doc(&site_url(&format!("manga/{key}/"))?)?;
    let title = document.require("h1")?.text()?;
    let mut manga = card(key, &title, cover(&document, ".lc-cover-lg img"));
    let mut synopsis = document
        .one("#sinopsis p:not(.lc-muted)")
        .map(|node| node.text())
        .transpose()?
        .unwrap_or_default();
    if let Some(node) = document.one("h1 + p.lc-muted") {
        let alternate_names = node.text()?;
        let alternate_names = alternate_names
            .split(" · ")
            .filter(|s| !s.is_empty() && *s != title)
            .collect::<Vec<_>>()
            .join(" · ");
        if !alternate_names.is_empty() {
            let label = Labels::new(METADATA, request)?
                .text("detail.alternate_titles", "Alternative titles")?;
            if !synopsis.is_empty() {
                synopsis.push_str("\n\n");
            }
            synopsis.push_str(&format!("{label}: {alternate_names}"));
        }
    }
    manga["description"] = json!(synopsis);
    manga["tags"] = json!(document
        .all("a.badge[href^='/manga/?genre='], a.badge[href^='/manga/?theme=']")?
        .iter()
        .map(Node::text)
        .collect::<Result<Vec<_>>>()?);
    for entry in document.all(".lc-facts li")? {
        let Some(label) = entry.one(".k") else {
            continue;
        };
        let Some(value) = entry.one("span:last-child") else {
            continue;
        };
        let value = value.text()?;
        match label.text()?.as_str() {
            "Autor" => manga["authors"] = json!([value]),
            "Dibujo" => manga["artists"] = json!([value]),
            "Estado" => {
                manga["status"] = json!(match value.as_str() {
                    "Ongoing" => "ongoing",
                    "Paused" => "hiatus",
                    "Completed" => "completed",
                    "Cancelled" => "cancelled",
                    _ => "unknown",
                })
            }
            _ => {}
        }
    }
    Ok(json!({"manga":manga}))
}

fn get_pages(args: &Value) -> Result<Value> {
    let id = args["chapterId"].as_str().ok_or("invalid_request")?;
    let key = id.strip_prefix("chapter:").ok_or("invalid_request")?;
    let (chapter_work, _) = chapter_key(key)?;
    if !args["mangaId"].is_null() && work(args)? != chapter_work {
        return Err("invalid_request");
    }
    let url = site_url(&format!("leer/{key}/"))?;
    let document = doc(&url)?;
    let images = document.all("#lcPages img[data-src]")?;
    if images.is_empty() {
        return Err("chapter_unavailable");
    }
    let pages = images
        .iter()
        .enumerate()
        .map(|(index, image)| {
            // The public reader rotates image hosts. Use only HTTPS image URLs
            // supplied by this chapter, with the standard host image transport.
            let image = port::absolute(url.as_str(), &image.attr("data-src")?)?;
            if image.scheme() != "https" || image.port().is_some() || image.fragment().is_some() {
                return Err("parse_error");
            }
            Ok(port::page(index, id, image))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(json!({"pages":pages}))
}

fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, (&'static str, String)> {
    let result = || {
        let input = port::input(request)?;
        let args = &input["args"];
        if !args["page"]["cursor"].is_null() {
            return Err("invalid_request");
        }
        let key = work(args)?;
        let manga = format!("manga:{key}");
        let document = doc(&site_url(&format!("manga/{key}/"))?)?;
        document.require("h1")?;
        let mut rows = Vec::new();
        let mut seen = BTreeSet::new();
        for row in document.all("a.lc-chapter-row")? {
            let url = site_url(&row.attr("href")?)?;
            let raw = link_key(&url, "/leer/")?;
            let (chapter_work, number) = chapter_key(&raw)?;
            if chapter_work != key {
                return Err("parse_error");
            }
            if seen.insert(raw.clone()) {
                let date = row
                    .one(".d")
                    .and_then(|node| node.text().ok())
                    .filter(|day| day.len() == 10 && day.split('-').count() == 3);
                rows.push((
                    format!("chapter:{raw}"),
                    url,
                    row.require(".n")?.text()?,
                    date,
                    String::from(number),
                ));
            }
        }
        let chapters: Vec<_> = rows
            .iter()
            .map(|(id, url, title, date, number)| {
                let mut chapter = Chapter::new(id.as_bytes(), manga.as_bytes(), title.as_bytes());
                chapter.url = Some(url.as_str().as_bytes());
                chapter.language = Some(b"es");
                chapter.published_at = date.as_deref().map(str::as_bytes);
                chapter.chapter_number =
                    number.parse().ok().map(ChapterNumber::Integer).or_else(|| {
                        DecimalChapterNumber::new(number.as_bytes()).map(ChapterNumber::Decimal)
                    });
                chapter
            })
            .collect();
        ChapterResponse::ordered(payload_buf(), request, ChapterOrder::NewestFirst, &chapters)
            .map_err(port::chapter_error)
    };
    result().map_err(|code| (code, port_catalog::failure(METADATA, request, code)))
}

fn chapter_failure((code, message): (&str, String)) -> u32 {
    write_error("get_chapters", code, &message)
}

fn run_get_listings(request: &[u8]) -> u32 {
    call("get_listings", request, |_| {
        let labels = Labels::new(METADATA, request)?;
        let listings = [
            ("popular", "Popular", false),
            ("latest", "Latest updates", false),
            ("catalog", "Catalogue", true),
        ]
        .iter()
        .map(|(id, fallback, catalog)| {
            Ok(json!({
                "id":id,"name":labels.text(&format!("listing.{id}"),fallback)?,
                "supportsPaging":catalog,"supportsFilters":catalog
            }))
        })
        .collect::<Result<Vec<_>>>()?;
        Ok(json!({"listings":listings}))
    })
}

fn run_get_filters(request: &[u8]) -> u32 {
    call("get_filters", request, |_| {
        let labels = Labels::new(METADATA, request)?;
        let any = labels.text("filter.any", "Any")?;
        let mut filters = Vec::new();
        for (id, choices) in FILTERS {
            let mut options = Vec::new();
            if *id != "sort" {
                options.push(json!({"value":"","label":any}));
            }
            for (key, label) in *choices {
                let label = labels.text(&format!("option.{id}.{key}"), label)?;
                options.push(json!({"value":key,"label":label}));
            }
            filters.push(
                json!({"type":"select","id":id,"name":labels.text(&format!("filter.{id}"),id)?,
                "default":if *id == "sort" {"az"} else {""},"options":options}),
            );
        }
        Ok(json!({"filters":filters}))
    })
}

fn call(op: &str, request: &[u8], action: impl FnOnce(&Value) -> Result<Value>) -> u32 {
    let result = port::input(request).and_then(|input| action(&input["args"]));
    port_catalog::write_localized(METADATA, request, op, result)
}

fn run_search(request: &[u8]) -> u32 {
    call("search", request, |args| browse(args, true))
}
fn run_get_manga_list(request: &[u8]) -> u32 {
    call("get_manga_list", request, |args| browse(args, false))
}
fn run_get_manga(request: &[u8]) -> u32 {
    call("get_manga", request, |args| detail(args, request))
}
fn run_get_pages(request: &[u8]) -> u32 {
    call("get_pages", request, get_pages)
}
fn run_get_settings(request: &[u8]) -> u32 {
    call("get_settings", request, |_| Ok(json!({"settings":[]})))
}
fn run_get_home(request: &[u8]) -> u32 {
    call("get_home", request, |_| Err("not_implemented"))
}
fn run_get_image_request(request: &[u8]) -> u32 {
    call("get_image_request", request, |_| Err("not_implemented"))
}

koma_source_sdk::koma_source_exports!("leercapitulo-es", typed_chapters, chapter_failure);
