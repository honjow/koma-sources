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
    chapters::{Chapter, ChapterNumber, ChapterOrder, ChapterResponse, ChapterResult},
    source::{SourceCapabilities, SourceInfo},
};
use serde_json::{json, Value};
use url::Url;

#[path = "../../port_support.rs"]
#[allow(dead_code)]
mod port;
#[path = "../../port_catalog.rs"]
mod port_catalog;

use port::Result;
use port_catalog::Labels;

#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

const SITE: &str = "https://aralosbd.fr/";
const METADATA: &[u8] = include_bytes!("../metadata.json");
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.aralosbd.fr.koma",
    name: "AralosBD",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "fr",
    description: "French-language comics with original works and translations, including GhostBlade and Ramia-Yana.",
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
const TAGS: &[(&str, &str)] = &[
    ("", "Any"),
    ("Action", "Action"),
    ("Fantastique", "Fantasy"),
    ("Webtoon", "Webtoon"),
    ("Couleurs", "Color"),
    ("Seinen", "Seinen"),
];
koma_source_sdk::koma_source_buffers! {
    payload: 192 * 1024, http_out: 1024 * 1024, body: 512 * 1024,
    http_req: 4096, scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

fn numeric(raw: &str) -> Result<u64> {
    raw.parse::<u64>()
        .ok()
        .filter(|n| *n > 0 && n.to_string() == raw)
        .ok_or("invalid_request")
}

fn id(args: &Value, field: &str, prefix: &str) -> Result<u64> {
    let raw = args[field]
        .as_str()
        .and_then(|s| s.strip_prefix(prefix))
        .ok_or("invalid_request")?;
    numeric(raw)
}

fn image(raw: &str) -> Result<Url> {
    let url = port::absolute(SITE, raw)?;
    if url.scheme() != "https"
        || url.host_str() != Some("aralosbd.fr")
        || url.port().is_some()
        || !url.path().starts_with("/data/")
        || url.fragment().is_some()
    {
        return Err("parse_error");
    }
    Ok(url)
}

fn json_get(route: &str, params: &[(&str, &str)]) -> Result<Value> {
    let mut url = port::absolute(SITE, route)?;
    url.query_pairs_mut().extend_pairs(params.iter().copied());
    let body = port::get(url.as_str())?;
    let data: Value = serde_json::from_str(&body).map_err(|_| "parse_error")?;
    if data
        .get("error")
        .is_some_and(|error| error.as_u64() != Some(0))
    {
        return Err("upstream_error");
    }
    Ok(data)
}

fn text<'a>(v: &'a Value, field: &str) -> Result<&'a str> {
    v[field]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or("parse_error")
}

fn manga(item: &Value, key: u64, title: &str) -> Result<Value> {
    let cover = image(text(item, "icon")?)?;
    Ok(json!({
        "id":format!("manga:{key}"), "url":format!("{SITE}manga/display?id={key}"),
        "title":title, "language":"fr", "contentRating":"sfw",
        "cover":{"kind":"url","url":cover.as_str(),"headers":{}}
    }))
}

fn catalog(args: &Value, search: bool) -> Result<Value> {
    let page = match &args["page"]["cursor"] {
        Value::Null => 1,
        Value::String(cursor) => numeric(cursor)?.min(100_001),
        _ => return Err("invalid_request"),
    };
    if page > 100_000 {
        return Err("invalid_request");
    }
    let listing = args["listingId"].as_str().unwrap_or("latest");
    let sort = if search {
        "id"
    } else {
        match listing.strip_prefix("listing:").unwrap_or(listing) {
            "popular" => "allviews",
            "latest" => "id",
            _ => return Err("invalid_request"),
        }
    };
    let tag = port::filter(args, "tag", TAGS)?;
    let query = match (&args["query"], search) {
        (_, false) | (Value::Null, true) => "",
        (Value::String(query), true)
            if query.len() <= 512
                && !query
                    .chars()
                    .any(|c| c.is_control() || matches!(c, ';' | ':' | '~' | '(' | ')')) =>
        {
            query.trim()
        }
        _ => return Err("invalid_request"),
    };
    let mut spec = format!("sort:{sort};limit:24;-id:3;page:{};order:desc", page - 1);
    if !tag.is_empty() {
        spec.push(';');
        spec.push_str(tag);
    }
    if !query.is_empty() {
        spec.push_str(";text:");
        spec.push_str(query);
    }
    let data = json_get("manga/search", &[("s", &spec)])?;
    let mut items = Vec::new();
    let mut seen = BTreeSet::new();
    for item in data["mangas"].as_array().ok_or("parse_error")? {
        let key = item["id"]
            .as_u64()
            .filter(|n| *n > 0)
            .ok_or("parse_error")?;
        if seen.insert(key) {
            items.push(manga(item, key, text(item, "title")?)?);
        }
    }
    let count = data["page_count"].as_u64().ok_or("parse_error")?;
    Ok(port::items(
        items,
        (page < count).then_some(page as usize + 1),
    ))
}

fn values(data: &Value, array: &str, field: &str) -> Vec<String> {
    data[array]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| text(item, field).ok().map(String::from))
        .collect()
}

fn description(data: &Value) -> Result<String> {
    // The public API uses HTML entities in its presentation text. Decode
    // those as text while preserving the site's paragraph boundaries.
    let sections = ["description", "fulldescription"];
    let mut description = Vec::new();
    for field in sections {
        if let Some(raw) = data[field].as_str() {
            let raw = raw.split("---").next().unwrap_or_default().trim();
            let mut lines = Vec::new();
            for line in raw.lines() {
                let encoded = line.replace('<', "&lt;").replace('>', "&gt;");
                if encoded.trim().is_empty() {
                    lines.push(String::new());
                } else {
                    lines.push(
                        port::Node::parse(&format!("<p>{encoded}</p>"))?
                            .require("p")?
                            .text()?,
                    );
                }
            }
            description.push(lines.join("\n"));
        }
    }
    Ok(description.join("\n\n").trim().into())
}

fn detail(args: &Value) -> Result<Value> {
    let key = id(args, "mangaId", "manga:")?;
    let item = json_get("manga/api", &[("get", "manga"), ("id", &key.to_string())])?;
    if item["id"].as_u64() != Some(key) {
        return Err("parse_error");
    }
    let mut result = manga(&item, key, text(&item, "main_title")?)?;
    result["description"] = json!(description(&item)?);
    result["authors"] = json!(values(&item, "authors", "name"));
    result["tags"] = json!(values(&item, "tags", "tag"));
    Ok(json!({"manga":result}))
}

fn pages(args: &Value) -> Result<Value> {
    let key = args["chapterId"]
        .as_str()
        .and_then(|id| id.strip_prefix("chapter:"))
        .ok_or("invalid_request")?;
    let (work, chapter) = key.split_once(':').ok_or("invalid_request")?;
    numeric(work)?;
    numeric(chapter)?;
    if !args["mangaId"].is_null() && id(args, "mangaId", "manga:")? != numeric(work)? {
        return Err("invalid_request");
    }
    let data = json_get("manga/api", &[("get", "pages"), ("chapter", chapter)])?;
    let links = data["links"].as_array().ok_or("parse_error")?;
    if links.is_empty() {
        return Err("chapter_unavailable");
    }
    let chapter_id = format!("chapter:{key}");
    let pages = links
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let url = image(entry.as_str().ok_or("parse_error")?)?;
            Ok(port::page(index, &chapter_id, url))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(json!({"pages":pages}))
}

fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, (&'static str, String)> {
    let chapters = || {
        let input = port::input(request)?;
        let args = &input["args"];
        if !args["page"]["cursor"].is_null() {
            return Err("invalid_request");
        }
        let key = id(args, "mangaId", "manga:")?;
        let work = format!("manga:{key}");
        let data = json_get(
            "manga/api",
            &[("get", "chapters"), ("manga", &key.to_string())],
        )?;
        let mut seen = BTreeSet::new();
        let mut rows = Vec::new();
        for chapter in data.as_array().ok_or("parse_error")? {
            if chapter["chapter_released"].as_u64() != Some(1) {
                continue;
            }
            let chapter_key = chapter["chapter_id"]
                .as_u64()
                .filter(|n| *n > 0)
                .ok_or("parse_error")?;
            if !seen.insert(chapter_key) {
                continue;
            }
            let number = text(chapter, "chapter_number")?;
            let title = chapter["chapter_title"].as_str().unwrap_or_default().trim();
            let title = if title.is_empty() {
                number.into()
            } else {
                format!("{number} - {title}")
            };
            rows.push((
                format!("chapter:{key}:{chapter_key}"),
                format!("{SITE}manga/chapter?id={chapter_key}"),
                title,
                number,
                chapter["chapter_translator"].as_str(),
            ));
        }
        let entries: Vec<_> = rows
            .iter()
            .map(|(id, url, title, number, translator)| {
                let mut chapter = Chapter::new(id.as_bytes(), work.as_bytes(), title.as_bytes());
                chapter.url = Some(url.as_bytes());
                chapter.language = Some(b"fr");
                chapter.chapter_number = number.parse().ok().map(ChapterNumber::Integer);
                chapter.scanlator = translator.map(str::as_bytes);
                chapter
            })
            .collect();
        ChapterResponse::ordered(payload_buf(), request, ChapterOrder::NewestFirst, &entries)
            .map_err(port::chapter_error)
    };
    chapters().map_err(|code| (code, port_catalog::failure(METADATA, request, code)))
}

fn chapter_failure((code, message): (&str, String)) -> u32 {
    write_error("get_chapters", code, &message)
}

fn run_get_listings(request: &[u8]) -> u32 {
    call("get_listings", request, |_| {
        let labels = Labels::new(METADATA, request)?;
        Ok(json!({"listings":[
            {"id":"popular","name":labels.text("listing.popular", "Popular")?,"supportsPaging":true,"supportsFilters":true},
            {"id":"latest","name":labels.text("listing.latest", "New series")?,"supportsPaging":true,"supportsFilters":true}
        ]}))
    })
}

fn run_get_filters(request: &[u8]) -> u32 {
    call("get_filters", request, |_| {
        let labels = Labels::new(METADATA, request)?;
        let mut options = vec![json!({"value":"","label":labels.text("filter.any", "Any")?})];
        for (id, name) in &TAGS[1..] {
            options.push(json!({"value":id,"label":labels.text(&format!("tag.{id}"), name)?}));
        }
        Ok(
            json!({"filters":[{"type":"select","id":"tag","name":labels.text("filter.tag","Genre or tag")?,
            "default":"","options":options}]}),
        )
    })
}

fn call(op: &str, request: &[u8], action: impl FnOnce(&Value) -> Result<Value>) -> u32 {
    let result = port::input(request).and_then(|input| action(&input["args"]));
    port_catalog::write_localized(METADATA, request, op, result)
}

fn run_search(request: &[u8]) -> u32 {
    call("search", request, |args| catalog(args, true))
}
fn run_get_manga_list(request: &[u8]) -> u32 {
    call("get_manga_list", request, |args| catalog(args, false))
}
fn run_get_manga(request: &[u8]) -> u32 {
    call("get_manga", request, detail)
}
fn run_get_pages(request: &[u8]) -> u32 {
    call("get_pages", request, pages)
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

koma_source_sdk::koma_source_exports!("aralosbd-fr", typed_chapters, chapter_failure);
