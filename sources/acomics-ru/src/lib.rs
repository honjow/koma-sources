#![no_std]
extern crate alloc;

use alloc::{format, string::String, vec::Vec};
use koma_source_sdk::{
    chapters::{
        Chapter, ChapterNumber, ChapterPage, ChapterPlacement, ChapterResult, GroupedChapters,
    },
    localization::Catalog,
    runtime::Context,
    source::{SourceCapabilities, SourceInfo},
};
use serde_json::{json, Value};
use url::Url;

#[path = "../../port_support.rs"]
#[allow(dead_code)]
mod port_support;
use port_support::{self as port, Node, Result};

#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

const SITE: &str = "https://acomics.ru";
const DESCRIPTION: &str = "AComics publishes independent webcomics in Russian, including originals and translations, with sections for staff picks and new creators.";
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.acomics.ru.koma",
    name: "AComics",
    language: "ru",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    description: DESCRIPTION,
    author: "Koma source contributors",
    content_rating: "unknown",
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
    image_request: true,
};
koma_source_sdk::koma_source_buffers! {
    payload: 440 * 1024, http_out: 1024 * 1024, body: 512 * 1024,
    http_req: 4096, scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

type Options = &'static [(&'static str, &'static str)];
const CATALOGUES: Options = &[
    ("comics", "filter.comics"),
    ("sandbox", "filter.sandbox"),
    ("featured", "filter.featured"),
];
const SORTS: Options = &[
    ("last_update", "filter.updated"),
    ("subscr_count", "filter.subscribers"),
    ("issue_count", "filter.issue_count"),
    ("serial_name", "filter.alphabetical"),
];
const GENRES: Options = &[
    ("1", "genre.animals"),
    ("2", "genre.drama"),
    ("3", "genre.fantasy"),
    ("4", "genre.games"),
    ("5", "genre.humor"),
    ("6", "genre.journal"),
    ("7", "genre.paranormal"),
    ("8", "genre.apocalypse"),
    ("9", "genre.romance"),
    ("10", "genre.scifi"),
    ("11", "genre.daily"),
    ("12", "genre.steampunk"),
    ("13", "genre.superheroes"),
    ("14", "genre.mystery"),
    ("15", "genre.historical"),
];
const RATINGS: Options = &[
    ("1", "rating.nr"),
    ("2", "rating.g"),
    ("3", "rating.pg"),
    ("4", "rating.pg13"),
    ("5", "rating.r"),
    ("6", "rating.nc17"),
];
const DEFAULT_RATINGS: &[&str] = &["1", "2", "3", "4", "5"];
const TYPES: Options = &[
    ("0", "filter.all"),
    ("orig", "filter.original"),
    ("trans", "filter.translation"),
];
const STATUSES: Options = &[
    ("0", "filter.all"),
    ("yes", "filter.ongoing"),
    ("no", "filter.finished"),
];
const SUBSCRIPTIONS: Options = &[
    ("0", "filter.all"),
    ("yes", "filter.subscribed"),
    ("no", "filter.not_subscribed"),
];
const LIST_SIZE: usize = 10;
const CHAPTER_PAGE_SIZE: usize = 400;

fn slug_valid(slug: &str) -> bool {
    !slug.is_empty()
        && slug.len() <= 128
        && slug != "."
        && slug != ".."
        && slug
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn work_slug(id: &str) -> Result<&str> {
    id.strip_prefix("/~")
        .and_then(|value| value.strip_suffix("/about"))
        .filter(|slug| slug_valid(slug))
        .ok_or("invalid_request")
}

fn required_work(input: &Value) -> Result<(&str, &str)> {
    let id = input["args"]["mangaId"].as_str().ok_or("invalid_request")?;
    Ok((id, work_slug(id)?))
}

fn count(raw: &str) -> Option<usize> {
    (!raw.is_empty() && raw.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| raw.parse().ok())
        .flatten()
}

fn number(raw: &str) -> Option<usize> {
    (!raw.starts_with('0')).then(|| count(raw)).flatten()
}

fn image_url(location: &str) -> Result<Url> {
    let url = Url::parse(location).map_err(|_| "invalid_request")?;
    if url.scheme() != "https"
        || url.host_str() != Some("acomics.ru")
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || !["/upload/", "/upload2/"]
            .iter()
            .any(|prefix| url.path().starts_with(prefix))
    {
        return Err("invalid_request");
    }
    Ok(url)
}

fn image(base: &str, raw: &str) -> Result<Url> {
    let url = port::absolute(base, raw)?;
    image_url(url.as_str()).map_err(|_| "parse_error")
}

fn cover(base: &str, raw: &str) -> Result<Value> {
    if raw.trim().is_empty() {
        return Ok(Value::Null);
    }
    Ok(json!({"kind":"url","url":image(base, raw)?.as_str(),"headers":{}}))
}

fn choice<'a>(
    filters: &'a Value,
    key: &str,
    options: Options,
    default: &'a str,
) -> Result<&'a str> {
    let Some(input) = filters.get(key).filter(|value| !value.is_null()) else {
        return Ok(default);
    };
    input
        .as_str()
        .filter(|input| options.iter().any(|(value, _)| input == value))
        .ok_or("invalid_request")
}

fn choices<'a>(
    filters: &'a Value,
    key: &str,
    options: Options,
    defaults: &[&'a str],
) -> Result<Vec<&'a str>> {
    let Some(input) = filters.get(key).filter(|value| !value.is_null()) else {
        return Ok(defaults.to_vec());
    };
    input
        .as_array()
        .ok_or("invalid_request")?
        .iter()
        .map(|input| {
            input
                .as_str()
                .filter(|input| options.iter().any(|(value, _)| input == value))
                .ok_or("invalid_request")
        })
        .collect()
}

fn browse_url(input: &Value, search: bool) -> Result<(Url, usize)> {
    let args = &input["args"];
    let page = match &args["page"]["cursor"] {
        Value::Null => 1,
        value => value.as_str().and_then(number).ok_or("invalid_request")?,
    };
    let query = if search {
        args["query"].as_str().unwrap_or("").trim()
    } else {
        ""
    };
    let filters = &args["filters"];
    let sort = match args["listingId"].as_str() {
        None | Some("popular") => "subscr_count",
        Some("latest") => "last_update",
        _ => return Err("invalid_request"),
    };
    let path = if query.is_empty() {
        choice(filters, "catalog", CATALOGUES, "comics")?
    } else {
        "search"
    };
    let mut url = Url::parse(&format!("{SITE}/{path}")).map_err(|_| "internal_error")?;
    {
        let mut params = url.query_pairs_mut();
        if query.is_empty() {
            params.append_pair("sort", choice(filters, "sort", SORTS, sort)?);
            for (id, options) in [
                ("type", TYPES),
                ("updatable", STATUSES),
                ("subscribe", SUBSCRIPTIONS),
            ] {
                params.append_pair(id, choice(filters, id, options, "0")?);
            }
            let minimum = filters["issue_count"]
                .as_str()
                .and_then(|raw| raw.trim().parse::<i64>().ok())
                .map_or(2, |value| value.clamp(0, 9999));
            params.append_pair("issue_count", &format!("{minimum}"));
            for genre in choices(filters, "categories", GENRES, &[])? {
                params.append_pair("categories[]", genre);
            }
            for rating in choices(filters, "ratings", RATINGS, DEFAULT_RATINGS)? {
                params.append_pair("ratings[]", rating);
            }
        } else {
            if query.chars().count() < 3 {
                return Err("query_too_short");
            }
            params.append_pair("keyword", query);
        }
        if page > 1 {
            let skip = (page - 1).checked_mul(LIST_SIZE).ok_or("invalid_request")?;
            params.append_pair("skip", &format!("{skip}"));
        }
    }
    Ok((url, page))
}

fn fetch(url: &str) -> Result<Node> {
    // Anonymous requests carry no age-confirmation cookie or account.
    Node::parse(&port::get(url)?)
}

fn browse(input: &Value, search: bool) -> Result<Value> {
    let (url, page) = browse_url(input, search)?;
    let doc = fetch(url.as_str())?;
    let catalog = doc.require("article.common-article")?;
    let mut items = Vec::new();
    for card in catalog.all("section.serial-card")? {
        let title_link = card.require("h2 > a")?;
        let path = title_link.attr("href")?;
        let id = format!("{path}/about");
        work_slug(&id).map_err(|_| "parse_error")?;
        let title = title_link.text()?;
        if title.is_empty() {
            return Err("parse_error");
        }
        let thumbnail = match card.one("a.cover > img[data-real-src]") {
            Some(img) => cover(url.as_str(), &img.attr("data-real-src")?)?,
            None => Value::Null,
        };
        items.push(json!({
            "id":id,"title":title,"cover":thumbnail,"authors":[],"language":"ru",
            "status":"unknown","contentRating":"unknown"
        }));
    }
    let has_more = catalog.one("a.infinite-scroll").is_some();
    Ok(port::items(items, has_more.then(|| page + 1)))
}

fn texts(doc: &Node, selector: &str) -> Result<Vec<String>> {
    doc.all(selector)?.iter().map(Node::text).collect()
}

fn detail(input: &Value) -> Result<Value> {
    let (id, _) = required_work(input)?;
    let url = format!("{SITE}{id}");
    let doc = fetch(&url)?;
    let article = doc.require("article.common-article")?;
    let title = article.require(".page-header-with-menu h1")?.text()?;
    let cover_image = match doc.one("meta[property='og:image']") {
        Some(meta) => cover(&url, &meta.attr("content")?)?,
        None => Value::Null,
    };
    let description = article
        .one("section.serial-about-text")
        .map(|node| node.text())
        .transpose()?;
    let status = if article.one(".serial-about-badges .completed").is_some() {
        "completed"
    } else {
        "ongoing"
    };
    Ok(json!({"manga":{
        "id":id, "title":title, "cover":cover_image, "description":description,
        "authors":texts(&article,"p.serial-about-authors a")?,
        "tags":texts(&article,"p.serial-about-badges a.category")?,
        "status":status, "language":"ru", "contentRating":"unknown",
        "links":[{"kind":"source","url":url}]
    }}))
}

fn issue_count(article: &Node) -> Result<usize> {
    const LABEL: &str = "Количество выпусков:";
    for entry in article.all("p")? {
        if let Some(label) = entry.one("b") {
            if label.text()? == LABEL {
                return entry
                    .text()?
                    .strip_prefix(LABEL)
                    .and_then(|raw| count(raw.trim()))
                    .ok_or("parse_error");
            }
        }
    }
    Err("parse_error")
}

fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(request)?;
    let (id, slug) = required_work(&input)?;
    let prefix = format!("acomics:{slug}:");
    let doc = fetch(&format!("{SITE}{id}"))?;
    let total = issue_count(&doc.require("article.common-article")?)?;
    let mut issue = match &input["args"]["page"]["cursor"] {
        Value::Null => total,
        cursor => {
            cursor
                .as_str()
                .and_then(|value| value.strip_prefix(&prefix))
                .and_then(number)
                .filter(|value| *value <= total)
                .ok_or("invalid_request")?
                - 1
        }
    };
    let mut output = GroupedChapters::new(payload_buf(), request).map_err(port::chapter_error)?;
    let mut last = String::new();
    for _ in 0..CHAPTER_PAGE_SIZE.min(issue) {
        let issue_id = format!("{prefix}{issue}");
        let title = format!("{issue}");
        let url = format!("{SITE}/~{slug}/{issue}");
        let mut chapter = Chapter::new(issue_id.as_bytes(), id.as_bytes(), title.as_bytes());
        chapter.chapter_number = Some(ChapterNumber::Integer(issue));
        chapter.language = Some(b"ru");
        chapter.url = Some(url.as_bytes());
        chapter.page_count = Some(1);
        output
            .push(
                chapter,
                ChapterPlacement::InGroup {
                    group: b"issues",
                    position: (issue - 1) as u64,
                },
            )
            .map_err(port::chapter_error)?;
        issue -= 1;
        last = issue_id;
    }
    let page = if issue == 0 {
        ChapterPage::Complete
    } else {
        ChapterPage::NextCursor(last.as_bytes())
    };
    output.finish(page).map_err(port::chapter_error)
}

fn pages(input: &Value) -> Result<Value> {
    let args = &input["args"];
    let id = args["chapterId"].as_str().ok_or("invalid_request")?;
    let (slug, issue) = id
        .strip_prefix("acomics:")
        .and_then(|value| value.split_once(':'))
        .filter(|(slug, issue)| slug_valid(slug) && number(issue).is_some())
        .ok_or("invalid_request")?;
    if let Some(work) = args.get("mangaId") {
        if work.as_str().and_then(|value| work_slug(value).ok()) != Some(slug) {
            return Err("invalid_request");
        }
    }
    let url = format!("{SITE}/~{slug}/{issue}");
    let doc = fetch(&url)?;
    let entry = doc
        .one("section.reader-issue img.issue")
        .ok_or("chapter_unavailable")?;
    let original = image(&url, &entry.attr("src")?)?;
    Ok(json!({"chapterId":id,"pages":[port::page(0,id,original)]}))
}

fn labels(request: &[u8], operation: &str) -> Result<Value> {
    port::input(request)?;
    let context = Context::from_request(request);
    let catalog =
        Catalog::from_metadata(include_bytes!("../metadata.json")).map_err(|_| "internal_error")?;
    let label = |key: &str| {
        let mut buffer = [0; 512];
        catalog
            .text(&context, key, key, &mut buffer)
            .map(String::from)
            .map_err(|_| "internal_error")
    };
    if operation == "get_listings" {
        return Ok(json!({"listings":[
            {"id":"popular","name":label("listing.popular")?,"kind":"popular","supportsPaging":true,"supportsFilters":true},
            {"id":"latest","name":label("listing.latest")?,"kind":"latest","supportsPaging":true,"supportsFilters":true}
        ]}));
    }
    let mut filters = Vec::new();
    for (id, key, kind, options, default) in [
        (
            "catalog",
            "filter.catalog",
            "select",
            CATALOGUES,
            Some("comics"),
        ),
        ("sort", "filter.sort", "select", SORTS, None),
        ("categories", "filter.genres", "multiselect", GENRES, None),
        ("ratings", "filter.ratings", "multiselect", RATINGS, None),
        ("type", "filter.type", "select", TYPES, Some("0")),
        ("updatable", "filter.status", "select", STATUSES, Some("0")),
        (
            "subscribe",
            "filter.subscription",
            "select",
            SUBSCRIPTIONS,
            Some("0"),
        ),
    ] {
        let options = options
            .iter()
            .map(|(value, key)| Ok(json!({"value":value,"label":label(key)?})))
            .collect::<Result<Vec<_>>>()?;
        let mut filter =
            json!({"id":id,"name":label(key)?,"type":kind,"scope":"browse","options":options});
        if let Some(default) = default {
            filter["default"] = json!(default);
        } else if id == "ratings" {
            filter["default"] = json!(DEFAULT_RATINGS);
        } else if kind == "multiselect" {
            filter["default"] = json!([]);
        }
        filters.push(filter);
    }
    filters.push(json!({"id":"issue_count","name":label("filter.min_issues")?,"type":"text","scope":"browse","default":"2"}));
    Ok(json!({"filters":filters}))
}

fn run_get_listings(request: &[u8]) -> u32 {
    port::write_result("get_listings", labels(request, "get_listings"))
}
fn run_get_filters(request: &[u8]) -> u32 {
    port::write_result("get_filters", labels(request, "get_filters"))
}
fn run_get_settings(_: &[u8]) -> u32 {
    port::write_result("get_settings", Ok(json!({"settings":[]})))
}
fn run_get_manga_list(request: &[u8]) -> u32 {
    port::write_result(
        "get_manga_list",
        port::input(request).and_then(|input| browse(&input, false)),
    )
}
fn run_search(request: &[u8]) -> u32 {
    match port::input(request).and_then(|input| browse(&input, true)) {
        Err("query_too_short") => write_error(
            "search",
            "invalid_request",
            "AComics requires at least three characters.",
        ),
        result => port::write_result("search", result),
    }
}
fn run_get_manga(request: &[u8]) -> u32 {
    port::write_result(
        "get_manga",
        port::input(request).and_then(|input| detail(&input)),
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
        Ok(json!({"url":url.as_str(),"headers":{}}))
    });
    port::write_result("get_image_request", result)
}
fn run_get_home(_: &[u8]) -> u32 {
    write_error("get_home", "unsupported", "home unsupported")
}
fn write_chapter_failure(code: &str) -> u32 {
    port::write_failure("get_chapters", code)
}

koma_source_sdk::koma_source_exports!("acomics_ru", typed_chapters, write_chapter_failure);
