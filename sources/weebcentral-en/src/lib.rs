#![no_std]
extern crate alloc;

use alloc::{collections::BTreeSet, format, string::String, vec::Vec};
use koma_source_sdk::{
    chapters::{
        Chapter, ChapterNumber, ChapterPage, ChapterPlacement, ChapterResult, DecimalChapterNumber,
        GroupedChapters,
    },
    host,
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

const SITE: &str = "https://weebcentral.com";
const UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/142.0.0.0 Safari/537.36";
const METADATA: &[u8] = include_bytes!("../metadata.json");
const WORK: &str = "weeb:";
const CHAPTER: &str = "weeb-ch:";
const OUTPUT: usize = 960 * 1024;
const LIST_SIZE: usize = 32;
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.weebcentral.en.koma", name: "Weeb Central", language: "en",
    version: env!("CARGO_PKG_VERSION"), api_version: "0.2",
    description: "An English catalogue of manga, manhwa and manhua, including Kingdom and other long-running series.",
    author: "Koma source contributors", content_rating: "mixed",
};
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: true,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: true,
    manga_list: true,
    filters: true,
    settings: true,
    home: false,
    credentials: false,
    image_request: true,
};
koma_source_sdk::koma_source_buffers! {
    payload: OUTPUT, http_out: 8 * 1024 * 1024, body: 4 * 1024 * 1024,
    http_req: 4096, scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

fn document(url: &str) -> Result<Node> {
    let html = port::request(
        url,
        "GET",
        json!({"Accept":"text/html","User-Agent":UA}),
        None,
    )?;
    Node::parse(&html)
}

fn ulid(id: &str) -> bool {
    id.len() == 26
        && id
            .bytes()
            .all(|b| b"0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(&b))
}

fn work_id(input: &Value) -> Result<(&str, &str)> {
    let full = input["args"]["mangaId"].as_str().ok_or("invalid_request")?;
    let id = full
        .strip_prefix(WORK)
        .filter(|id| ulid(id))
        .ok_or("invalid_request")?;
    Ok((full, id))
}

fn site_path(input: &str, kind: &str) -> Result<(Url, String)> {
    let url = port::absolute(SITE, input)?;
    if url.origin().ascii_serialization() != SITE {
        return Err("parse_error");
    }
    let mut parts = url.path().split('/').skip(1);
    if parts.next() != Some(kind) {
        return Err("parse_error");
    }
    let id = parts.next().filter(|id| ulid(id)).ok_or("parse_error")?;
    Ok((url.clone(), String::from(id)))
}

fn image_url(raw: &str) -> Result<Url> {
    let url = port::absolute(SITE, raw)?;
    if url.scheme() != "https" || url.port().is_some() || url.fragment().is_some() {
        return Err("parse_error");
    }
    Ok(url)
}

fn cover(node: &Node) -> Result<Value> {
    let raw = if let Some(source) = node.one("source[srcset]") {
        source.attr("srcset")?.replace("/small/", "/normal/")
    } else if let Some(img) = node.one("img[src]") {
        img.attr("src")?
    } else {
        return Ok(json!({"kind":"none"}));
    };
    let url = image_url(raw.split_ascii_whitespace().next().ok_or("parse_error")?)?;
    Ok(json!({"kind":"url","url":url.as_str(),"headers":{"Referer":SITE}}))
}

fn listing(input: &Value, search: bool) -> Result<Value> {
    let args = &input["args"];
    let default_sort = if search {
        "Best Match"
    } else {
        match args["listingId"].as_str().unwrap_or("popular") {
            "popular" => "Popularity",
            "latest" => "Latest Updates",
            "recent" => "Recently Added",
            _ => return Err("invalid_request"),
        }
    };
    let query = args["query"]
        .as_str()
        .unwrap_or("")
        .replace(['!', '#', ':', '(', ')', ',', '-'], " ");
    let offset = match &args["page"]["cursor"] {
        Value::Null => 0,
        value => value
            .as_str()
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n > 0 && n % LIST_SIZE == 0 && *n <= 100_000)
            .ok_or("invalid_request")?,
    };
    let mut url = port::absolute(SITE, "/search/data")?;
    {
        let mut pairs = url.query_pairs_mut();
        pairs.append_pair("text", query.trim());
        filters::append(&args["filters"], default_sort, &mut pairs)?;
        pairs
            .append_pair("limit", "32")
            .append_pair("offset", &format!("{offset}"))
            .append_pair("display_mode", "Full Display");
    }
    let doc = document(url.as_str())?;
    let mut items = Vec::new();
    let mut seen = BTreeSet::new();
    for item in doc.all("article > section > a[href]")? {
        let (url, id) = site_path(&item.attr("href")?, "series")?;
        if seen.insert(id.clone()) {
            items.push(json!({
                "id":format!("{WORK}{id}"), "title":item.require("div:not([class]):last-child")?.text()?,
                "cover":cover(&item)?, "url":url.as_str(), "language":"en",
                "links":[{"kind":"source","url":url.as_str()}]
            }));
        }
    }
    let next = doc.one("button[hx-get^='/search/data']");
    let next = next
        .map(|button| -> Result<usize> {
            let next_url = port::absolute(SITE, &button.attr("hx-get")?)?;
            let suggested = next_url
                .query_pairs()
                .find(|(name, _)| name == "offset")
                .and_then(|(_, value)| value.parse::<usize>().ok())
                .ok_or("parse_error")?;
            if suggested != offset + LIST_SIZE || items.is_empty() {
                return Err("parse_error");
            }
            Ok(suggested)
        })
        .transpose()?;
    Ok(port::items(items, next))
}

fn text_if(node: &Node, selector: &str) -> Result<String> {
    node.one(selector)
        .map(|n| n.text())
        .transpose()
        .map(Option::unwrap_or_default)
}

fn texts(node: &Node, selector: &str) -> Result<Vec<String>> {
    node.all(selector)?.iter().map(Node::text).collect()
}

fn detail(input: &Value, labels: &Labels<'_>) -> Result<Value> {
    let (work, id) = work_id(input)?;
    let default_url = format!("{SITE}/series/{id}");
    let doc = document(&default_url)?;
    let sections = doc.all("section[x-data] > section")?;
    let [info, story, ..] = sections.as_slice() else {
        return Err("parse_error");
    };
    let title = story.require("h1")?.text()?;
    let title = if input["settings"]["pref_clean_title"].as_bool() == Some(true) {
        clean_title(&title)
    } else {
        &title
    };
    let url = doc
        .one("link[rel=canonical]")
        .map(|n| n.attr("href"))
        .transpose()?
        .unwrap_or(default_url);
    let (url, canonical_id) = site_path(&url, "series")?;
    if canonical_id != id {
        return Err("parse_error");
    }
    let mut authors = Vec::new();
    let mut tags = Vec::new();
    let mut status = "unknown";
    let mut rating = "unknown";
    let mut trackers = Vec::new();
    for row in info.all("ul > li")? {
        match text_if(&row, "strong")?.trim_end_matches(':') {
            "Author(s)" | "Author" => authors = texts(&row, "a")?,
            "Tags(s)" | "Tag(s)" | "Tags" | "Type" => tags.extend(texts(&row, "a")?),
            "Status" => {
                status = match text_if(&row, "a")?.as_str() {
                    "Ongoing" => "ongoing",
                    "Complete" => "completed",
                    "Hiatus" => "hiatus",
                    "Canceled" => "cancelled",
                    _ => "unknown",
                }
            }
            "Adult Content" => {
                rating = match text_if(&row, "a")?.as_str() {
                    "Yes" => "adult",
                    "No" => "safe",
                    _ => "unknown",
                }
            }
            "Track" => {
                for tracker in row.all("span[data-tip]")? {
                    if let Some(anchor) = tracker.one("a[href]") {
                        trackers.push((tracker.attr("data-tip")?, anchor.attr("href")?));
                    }
                }
            }
            _ => {}
        }
    }
    let mut description = String::new();
    let mut alternate_titles = Vec::new();
    let mut related = Vec::new();
    for row in story.all("ul > li")? {
        match text_if(&row, "strong")?.trim_end_matches(':') {
            "Description" => description = text_if(&row, "p")?.replace("NOTE: ", "\n\nNOTE: "),
            "Associated Name(s)" | "Associated Name" | "Associated Names" => {
                alternate_titles = texts(&row, "li")?
            }
            "Related Series" => {
                for item in row.all("li")? {
                    if let Some(a) = item.one("a[href]") {
                        let (link, _) = site_path(&a.attr("href")?, "series")?;
                        related.push((a.text()?, link, text_if(&item, "span")?));
                    }
                }
            }
            _ => {}
        }
    }
    if !related.is_empty() {
        description.push_str(&format!(
            "\n\n{}",
            labels.text("detail.related", "Related series")?
        ));
        for (title, link, relation) in related {
            description.push_str(&format!("\n- [{title}]({link}) {relation}").trim_end());
        }
    }
    if !alternate_titles.is_empty() {
        description.push_str(&format!(
            "\n\n{}",
            labels.text("detail.alternate", "Alternate titles")?
        ));
        for title in &alternate_titles {
            description.push_str(&format!("\n- {title}"));
        }
    }
    let mut links = Vec::from([json!({"kind":"source","url":url.as_str()})]);
    if !trackers.is_empty() {
        description.push_str(&format!(
            "\n\n{}",
            labels.text("detail.trackers", "Trackers")?
        ));
        for (name, raw) in trackers {
            if let Ok(link) = port::absolute(SITE, &raw) {
                description.push_str(&format!("\n- [{name}]({link})"));
                links.push(json!({"kind":"tracker","url":link.as_str(),"label":name}));
            }
        }
    }
    Ok(json!({"manga":{
        "id":work,"title":title,"description":description,"alternateTitles":alternate_titles,
        "cover":cover(info)?,"authors":authors,"status":status,"tags":tags,"contentRating":rating,
        "language":"en","url":url.as_str(),"links":links
    }}))
}

fn clean_title(title: &str) -> &str {
    let (open, close) = match title.as_bytes().last() {
        Some(b')') => ('(', ')'),
        Some(b']') => ('[', ']'),
        _ => return title,
    };
    title
        .rfind(open)
        .filter(|start| !title[*start + 1..title.len() - 1].contains(close))
        .map_or(title, |start| title[..start].trim_end())
}

fn chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(request)?;
    let (work, id) = work_id(&input)?;
    let prefix = format!("{CHAPTER}{id}:");
    let doc = document(&format!("{SITE}/series/{id}/full-chapter-list"))?;
    let rows = doc.all("div[x-data] > a[href^='/chapters/']")?;
    if rows.is_empty() {
        return Err("parse_error");
    }
    let mut seen = BTreeSet::new();
    let mut catalog = Vec::with_capacity(rows.len());
    for row in rows {
        if host::check_cancel() {
            return Err("cancelled");
        }
        let (url, id) = site_path(&row.attr("href")?, "chapters")?;
        if seen.insert(id.clone()) {
            catalog.push((id, url, row));
        }
    }
    let args = &input["args"];
    let exact = args["chapterId"].as_str();
    let start = match &args["page"]["cursor"] {
        Value::Null => 0,
        value => {
            let anchor = value
                .as_str()
                .and_then(|s| s.strip_prefix(&prefix))
                .ok_or("invalid_request")?;
            catalog
                .iter()
                .position(|(id, _, _)| id == anchor)
                .ok_or("invalid_request")?
                + 1
        }
    };
    let mut output = GroupedChapters::new(payload_buf(), request).map_err(port::chapter_error)?;
    let mut last = String::new();
    let mut budget = OUTPUT - 2048;
    for (position, (id, url, row)) in catalog.iter().enumerate().skip(start) {
        let id = format!("{prefix}{id}");
        if exact.is_some_and(|exact| exact != id) {
            continue;
        }
        let title = row.require("span.flex > span")?.text()?;
        let date = row
            .one("time[datetime]")
            .map(|n| n.attr("datetime"))
            .transpose()?;
        let official = row.all("img[src]")?.iter().any(|n| {
            n.attr("src")
                .is_ok_and(|src| src.to_ascii_lowercase().contains("official"))
        });
        // Compact rows normally fit in one call. Resume by stable ID if this
        // upstream catalog ever exceeds the envelope budget.
        let estimated = 350
            + 2 * title.len()
            + id.len()
            + work.len()
            + url.as_str().len()
            + date.as_ref().map_or(0, |s| 2 * s.len());
        if estimated > budget {
            if last.is_empty() {
                return Err("result_too_large");
            }
            return output
                .finish(ChapterPage::NextCursor(last.as_bytes()))
                .map_err(port::chapter_error);
        }
        let mut chapter = Chapter::new(id.as_bytes(), work.as_bytes(), title.as_bytes());
        chapter.url = Some(url.as_str().as_bytes());
        chapter.language = Some(b"en");
        chapter.published_at = date.as_deref().map(str::as_bytes);
        chapter.scanlator = Some(if official { b"Official" } else { b"Unknown" });
        let lower = title.to_ascii_lowercase();
        let season = lower
            .split(|c: char| !c.is_ascii_alphanumeric())
            .collect::<Vec<_>>();
        let season = season.windows(2).any(|parts| {
            matches!(parts[0], "season" | "s")
                && !parts[1].is_empty()
                && parts[1].bytes().all(|b| b.is_ascii_digit())
        }) || season.iter().any(|s| {
            s.strip_prefix("season")
                .or_else(|| s.strip_prefix('s'))
                .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        });
        chapter.chapter_number = if season {
            Some(ChapterNumber::Integer(catalog.len() - position))
        } else {
            lower
                .strip_prefix("chapter ")
                .and_then(|s| s.split_whitespace().next())
                .and_then(|s| DecimalChapterNumber::new(s.as_bytes()).map(ChapterNumber::Decimal))
        };
        output
            .push(
                chapter,
                ChapterPlacement::InGroup {
                    group: b"chapters",
                    position: (catalog.len() - position - 1) as u64,
                },
            )
            .map_err(port::chapter_error)?;
        if exact.is_some() {
            break;
        }
        budget -= estimated;
        last = id;
    }
    output
        .finish(ChapterPage::Complete)
        .map_err(port::chapter_error)
}

fn pages(input: &Value) -> Result<Value> {
    let chapter = input["args"]["chapterId"]
        .as_str()
        .ok_or("invalid_request")?;
    let (owner, id) = chapter
        .strip_prefix(CHAPTER)
        .and_then(|c| c.split_once(':'))
        .ok_or("invalid_request")?;
    if !ulid(owner)
        || !ulid(id)
        || input["args"]["mangaId"]
            .as_str()
            .is_some_and(|work| work.strip_prefix(WORK) != Some(owner))
    {
        return Err("invalid_request");
    }
    let url = format!("{SITE}/chapters/{id}/images?is_prev=False&reading_style=long_strip");
    let doc = document(&url)?;
    let pages = doc
        .all("section#chapter-images > img[src]")?
        .into_iter()
        .enumerate()
        .map(|(index, node)| Ok(port::page(index, chapter, image_url(&node.attr("src")?)?)))
        .collect::<Result<Vec<_>>>()?;
    if pages.is_empty() {
        return Err("chapter_unavailable");
    }
    Ok(json!({"chapterId":chapter,"pages":pages}))
}

fn run_search(request: &[u8]) -> u32 {
    port::write_result(
        "search",
        port::input(request).and_then(|input| listing(&input, true)),
    )
}
fn run_get_manga_list(request: &[u8]) -> u32 {
    port::write_result(
        "get_manga_list",
        port::input(request).and_then(|input| listing(&input, false)),
    )
}
fn run_get_manga(request: &[u8]) -> u32 {
    port::write_result(
        "get_manga",
        port::input(request).and_then(|input| detail(&input, &Labels::new(METADATA, request)?)),
    )
}
fn run_get_pages(request: &[u8]) -> u32 {
    port::write_result(
        "get_pages",
        port::input(request).and_then(|input| pages(&input)),
    )
}
fn run_get_image_request(request: &[u8]) -> u32 {
    let result = port::input(request).and_then(|input| {
        let url = image_url(input["args"]["url"].as_str().ok_or("invalid_request")?)?;
        // The CDN can serve JPEG bytes with a PNG URL and MIME. The host
        // decodes from the actual bytes, so preserve the original request.
        Ok(json!({"url":url.as_str(),"headers":{"Referer":SITE,"User-Agent":UA}}))
    });
    port::write_result("get_image_request", result)
}
fn run_get_listings(request: &[u8]) -> u32 {
    let result = Labels::new(METADATA, request).and_then(|labels| {
        let listings = [
            ("popular", "Popular"),
            ("latest", "Latest updates"),
            ("recent", "Recently added"),
        ]
        .into_iter()
        .map(|(id, name)| {
            Ok(
                json!({"id":id,"name":labels.text(&format!("listing.{id}"),name)?,
                "supportsPaging":true,"supportsFilters":true}),
            )
        })
        .collect::<Result<Vec<_>>>()?;
        Ok(json!({"listings":listings}))
    });
    port::write_result("get_listings", result)
}
fn run_get_settings(request: &[u8]) -> u32 {
    let result = Labels::new(METADATA, request).and_then(|labels| Ok(json!({"settings":[{
        "id":"pref_clean_title","kind":"boolean","default":false,
        "name":labels.text("setting.clean","Clean title")?,
        "description":labels.text("setting.clean.description","Remove a trailing bracketed note from the title.")?
    }]})));
    port::write_result("get_settings", result)
}
fn run_get_filters(request: &[u8]) -> u32 {
    port::write_result(
        "get_filters",
        Labels::new(METADATA, request).and_then(|labels| filters::describe(&labels)),
    )
}
fn run_get_home(_: &[u8]) -> u32 {
    port::write_failure("get_home", "not_implemented")
}
fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    chapters(request)
}
fn write_chapter_failure(code: &str) -> u32 {
    port::write_failure("get_chapters", code)
}
koma_source_sdk::koma_source_exports!("weebcentral_en", typed_chapters, write_chapter_failure);
