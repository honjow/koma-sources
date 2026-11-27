#![no_std]
extern crate alloc;

use alloc::{collections::BTreeSet, format, string::String, vec, vec::Vec};
use koma_source_sdk::{
    chapters::{
        Chapter, ChapterNumber, ChapterPage, ChapterPlacement, ChapterResult, GroupedChapters,
    },
    host,
    localization::Catalog,
    plain_text,
    runtime::Context,
    source::{SourceCapabilities, SourceInfo},
    source_protocol,
};
use serde_json::{json, Value};
use url::Url;

#[path = "../../port_support.rs"]
#[allow(dead_code)]
mod port_support;
use port_support::{self as port, Node, Result};

#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

const SITE: &str = "https://www.smbc-comics.com";
const ARCHIVE: &str = "https://www.smbc-comics.com/comic/archive";
const COVER: &str = "https://www.smbc-comics.com/images/moblogo.webp";
const TITLE: &str = "Saturday Morning Breakfast Comics";
const WORK_PREFIX: &str = "smbc:en:archive:";
const CHAPTER_PREFIX: &str = "smbcch:en:";
const SINGLE: &str = "SINGLE";
const CHAPTER_BYTES: usize = 440 * 1024;
const CHAPTER_COUNT: usize = 1000;
const ARCHIVE_BATCH: usize = 1500;
const LIST_SIZE: usize = 10;
const DESCRIPTION: &str = "Zach Weinersmith’s English comic turns science, philosophy and daily life into jokes. Some strips have an extra panel behind a red button.";
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.smbc.en.koma",
    name: TITLE,
    language: "en",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    description: DESCRIPTION,
    author: "Koma source contributors",
    content_rating: "safe",
};
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: false,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: true,
    manga_list: true,
    filters: false,
    settings: true,
    home: false,
    credentials: false,
    image_request: true,
};
koma_source_sdk::koma_source_buffers! {
    payload: CHAPTER_BYTES, http_out: 2 * 1024 * 1024, body: 1024 * 1024,
    http_req: 4096, scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

#[derive(Clone, Copy, PartialEq, Eq)]
enum Organization {
    Single,
    Year,
    Month,
}

impl Organization {
    fn from_settings(input: &Value) -> Self {
        match input["settings"]["organization_method"].as_str() {
            Some("BY_YEAR") => Self::Year,
            Some("BY_YEAR_MONTH") => Self::Month,
            _ => Self::Single,
        }
    }

    fn from_group(group: &str) -> Result<Self> {
        if group == SINGLE {
            return Ok(Self::Single);
        }
        if !matches!(group.get(..4).and_then(decimal), Some(1900..=9999)) {
            return Err("invalid_request");
        }
        match (
            group.len(),
            group.get(4..5),
            group.get(5..).and_then(decimal),
        ) {
            (4, _, _) => Ok(Self::Year),
            (7, Some("-"), Some(1..=12)) => Ok(Self::Month),
            _ => Err("invalid_request"),
        }
    }

    fn group(self, date: &str) -> &str {
        match self {
            Self::Single => SINGLE,
            Self::Year => &date[..4],
            Self::Month => &date[..7],
        }
    }
}

fn decimal(value: &str) -> Option<usize> {
    (!value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| value.parse().ok())
        .flatten()
}

fn work_group(input: &Value) -> Result<&str> {
    let group = input["args"]["mangaId"]
        .as_str()
        .and_then(|id| id.strip_prefix(WORK_PREFIX))
        .ok_or("invalid_request")?;
    Organization::from_group(group)?;
    Ok(group)
}

fn slug_valid(slug: &str) -> bool {
    !slug.is_empty()
        && slug.len() <= 256
        && slug
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn publication(raw: &str) -> Option<String> {
    let (month, rest) = raw.split_once(' ')?;
    let (day, year) = rest.split_once(", ")?;
    let month = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ]
    .iter()
    .position(|name| *name == month)?
        + 1;
    let year = decimal(year)?;
    let day = decimal(day)?;
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    if !(1900..=9999).contains(&year) || !(1..=days).contains(&day) {
        return None;
    }
    Some(format!("{year:04}-{month:02}-{day:02}T00:00:00Z"))
}

fn fetch_document(url: &str) -> Result<Node> {
    if host::check_cancel() {
        return Err("cancelled");
    }
    let request = serde_json::to_vec(&json!({
        "version":1, "method":"GET", "url":url, "headers":{"Accept":"text/html"},
        "responseKind":"bodyText", "timeoutMs":15000
    }))
    .map_err(|_| "internal_error")?;
    let len = host::http_request(&request, http_out()).map_err(|_| "network_error")?;
    let response = &http_out()[..len];
    let body = body_buf();
    // This particular site serves normal archive and viewer HTML with HTTP 500.
    // Accept only a completed transfer. The callers still require their own
    // structural markers, so ordinary error pages never become a catalog.
    let status = source_protocol::direct(response, b"status").map_err(|_| "network_error")?;
    let body = if status == Some(b"500") {
        if source_protocol::direct(response, b"ok").map_err(|_| "network_error")? != Some(b"true") {
            return Err("network_error");
        }
        let encoded = source_protocol::raw_string(response, b"bodyText")
            .map_err(|_| "network_error")?
            .ok_or("server_error")?;
        plain_text::decode_json_content(encoded, body).map_err(|_| "network_error")?
    } else {
        let len = source_protocol::body(response, body)?;
        &body[..len]
    };
    Node::parse(core::str::from_utf8(body).map_err(|_| "parse_error")?)
}

struct Comic {
    slug: String,
    title: String,
    published_at: String,
}

fn archive() -> Result<Vec<Comic>> {
    let doc = fetch_document(ARCHIVE)?;
    let menu = doc
        .require("select[name=comic]")
        .map_err(|_| "archive_missing")?;
    let mut comics = Vec::new();
    let mut seen = BTreeSet::new();
    // The DOM host returns a bounded number of handles per call. Request
    // explicit batches so a long archive cannot silently truncate.
    for first in (1..).step_by(ARCHIVE_BATCH) {
        let last = first + ARCHIVE_BATCH - 1;
        let selector = format!("option:nth-of-type(n+{first}):nth-of-type(-n+{last})");
        let nodes = menu.all(&selector)?;
        let last_batch = nodes.len() < ARCHIVE_BATCH;
        for node in nodes {
            if host::check_cancel() {
                return Err("cancelled");
            }
            let value = node.attr("value")?;
            if first == 1 && comics.is_empty() && value.is_empty() {
                continue;
            }
            let slug = value
                .strip_prefix("comic/")
                .filter(|slug| slug_valid(slug))
                .ok_or("parse_error")?;
            let text = node.text()?;
            let (date, title) = text.split_once(" - ").ok_or("parse_error")?;
            if title.is_empty() || !seen.insert(String::from(slug)) {
                return Err("parse_error");
            }
            comics.push(Comic {
                slug: String::from(slug),
                title: String::from(title),
                published_at: publication(date).ok_or("parse_error")?,
            });
        }
        if last_batch {
            break;
        }
    }
    if comics.is_empty() {
        return Err("parse_error");
    }
    Ok(comics)
}

fn work(group: &str) -> Value {
    let title = if group == SINGLE {
        String::from(TITLE)
    } else {
        format!("SMBC {group}")
    };
    json!({
        "id":format!("{WORK_PREFIX}{group}"), "title":title,
        "authors":["Zach Weinersmith"], "description":DESCRIPTION,
        "language":"en", "status":"ongoing", "contentRating":"safe",
        "cover":{"kind":"url","url":COVER,"headers":{}},
        "links":[{"kind":"source","url":ARCHIVE}]
    })
}

fn listing(input: &Value) -> Result<Value> {
    let args = &input["args"];
    if args
        .get("listingId")
        .is_some_and(|value| value.as_str() != Some("popular"))
    {
        return Err("invalid_request");
    }
    let number = match &args["page"]["cursor"] {
        Value::Null => 1,
        cursor => cursor
            .as_str()
            .and_then(decimal)
            .filter(|value| *value > 0)
            .ok_or("invalid_request")?,
    };
    let organization = Organization::from_settings(input);
    if organization == Organization::Single {
        let items = if number == 1 {
            vec![work(SINGLE)]
        } else {
            Vec::new()
        };
        return Ok(port::items(items, None));
    }
    let comics = archive()?;
    let groups: BTreeSet<_> = comics
        .iter()
        .map(|comic| organization.group(&comic.published_at))
        .collect();
    let start = (number - 1).saturating_mul(LIST_SIZE);
    let items = groups
        .iter()
        .rev()
        .skip(start)
        .take(LIST_SIZE)
        .map(|group| work(group))
        .collect();
    let next = (start.saturating_add(LIST_SIZE) < groups.len()).then(|| number + 1);
    Ok(port::items(items, next))
}

fn detail(input: &Value) -> Result<Value> {
    let group = work_group(input)?;
    let mut manga = work(group);
    let organization = Organization::from_group(group)?;
    if organization != Organization::Single {
        let comics = archive()?;
        let newest = comics
            .iter()
            .rev()
            .find(|comic| organization.group(&comic.published_at) == group)
            .ok_or("not_found")?;
        match read_viewer(&newest.slug) {
            Ok((url, _, _)) => manga["cover"]["url"] = json!(url.as_str()),
            Err("cancelled") => return Err("cancelled"),
            Err(_) => {}
        }
    }
    Ok(json!({"manga":manga}))
}

fn comic_url(slug: &str) -> String {
    format!("{SITE}/comic/{slug}")
}

fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(request)?;
    let group = work_group(&input)?;
    let organization = Organization::from_group(group)?;
    let work_id = input["args"]["mangaId"].as_str().ok_or("invalid_request")?;
    let prefix = format!("{CHAPTER_PREFIX}{group}:");
    let after = match &input["args"]["page"]["cursor"] {
        Value::Null => None,
        cursor => Some(
            cursor
                .as_str()
                .and_then(|id| id.strip_prefix(&prefix))
                .filter(|slug| slug_valid(slug))
                .ok_or("invalid_request")?,
        ),
    };
    let comics = archive()?;
    let selected: Vec<_> = comics
        .iter()
        .enumerate()
        .filter(|(_, comic)| organization.group(&comic.published_at) == group)
        .collect();
    if selected.is_empty() {
        return Err("not_found");
    }
    // An append to the upstream oldest-first archive cannot shift existing
    // positions. Resume immediately before the last emitted stable identity.
    let end = match after {
        Some(slug) => selected
            .iter()
            .position(|(_, comic)| comic.slug == slug)
            .ok_or("invalid_request")?,
        None => selected.len(),
    };
    let mut output = GroupedChapters::new(payload_buf(), request).map_err(port::chapter_error)?;
    let mut budget = CHAPTER_BYTES - 2048;
    let mut last = String::new();
    for (offset, (position, comic)) in selected[..end].iter().rev().enumerate() {
        let id = format!("{prefix}{}", comic.slug);
        let url = comic_url(&comic.slug);
        // IDs, URLs and the date use validated ASCII with no JSON escapes.
        // A title byte expands to at most two bytes in the SDK writer.
        // The fixed allowance covers the chapter fields, including reading order.
        let row = 320
            + id.len()
            + work_id.len()
            + url.len()
            + 2 * comic.title.len()
            + comic.published_at.len();
        if offset == CHAPTER_COUNT || row > budget {
            if last.is_empty() {
                return Err("result_too_large");
            }
            return output
                .finish(ChapterPage::NextCursor(last.as_bytes()))
                .map_err(port::chapter_error);
        }
        let mut chapter = Chapter::new(id.as_bytes(), work_id.as_bytes(), comic.title.as_bytes());
        chapter.chapter_number = Some(ChapterNumber::Integer(position + 1));
        chapter.published_at = Some(comic.published_at.as_bytes());
        chapter.language = Some(b"en");
        chapter.url = Some(url.as_bytes());
        output
            .push(
                chapter,
                ChapterPlacement::InGroup {
                    group: b"archive",
                    position: *position as u64,
                },
            )
            .map_err(port::chapter_error)?;
        budget -= row;
        last = id;
    }
    output
        .finish(ChapterPage::Complete)
        .map_err(port::chapter_error)
}

fn image_url(location: &str) -> Result<Url> {
    let url = Url::parse(location).map_err(|_| "invalid_request")?;
    if url.scheme() != "https"
        || !matches!(
            url.host_str(),
            Some("www.smbc-comics.com" | "smbc-comics.com")
        )
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || !(url.path().starts_with("/comics/") && url.path() != "/comics/"
            || url.path() == "/images/moblogo.webp")
    {
        return Err("invalid_request");
    }
    Ok(url)
}

fn viewer_image(base: &str, node: &Node) -> Result<Url> {
    let url = port::absolute(base, &node.attr("src")?)?;
    image_url(url.as_str()).map_err(|_| "parse_error")
}

fn read_viewer(slug: &str) -> Result<(Url, Option<String>, Option<Url>)> {
    let url = comic_url(slug);
    let doc = fetch_document(&url)?;
    let main = doc
        .require("img#cc-comic")
        .map_err(|_| "chapter_unavailable")?;
    let image = viewer_image(&url, &main)?;
    let hint = main
        .attr("title")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let extra = doc
        .one("#aftercomic > img, #mobaftercomic > img")
        .map(|node| viewer_image(&url, &node))
        .transpose()?;
    Ok((image, hint, extra))
}

fn pages(input: &Value, request: &[u8]) -> Result<Value> {
    let args = &input["args"];
    let chapter = args["chapterId"].as_str().ok_or("invalid_request")?;
    let (group, slug) = chapter
        .strip_prefix(CHAPTER_PREFIX)
        .and_then(|tail| tail.split_once(':'))
        .filter(|(_, slug)| slug_valid(slug))
        .ok_or("invalid_request")?;
    Organization::from_group(group)?;
    if let Some(work) = args.get("mangaId") {
        if work.as_str().and_then(|id| id.strip_prefix(WORK_PREFIX)) != Some(group) {
            return Err("invalid_request");
        }
    }
    let (image, hint, extra) = read_viewer(slug)?;
    let mut pages = vec![json!({"id":format!("{chapter}:main"),"index":0,
        "image":{"kind":"url","url":image.as_str()}})];
    if let Some(text) =
        hint.filter(|_| Context::from_request(request).supports(b"text_page_image_v1"))
    {
        pages.push(json!({"id":format!("{chapter}:hover"),"index":pages.len(),
            "image":{"kind":"text","text":{"template":"webtoon-author-notes","paragraphs":[text]}}}));
    }
    if let Some(image) = extra {
        pages.push(json!({"id":format!("{chapter}:votey"),"index":pages.len(),
            "image":{"kind":"url","url":image.as_str()}}));
    }
    Ok(json!({"chapterId":chapter,"pages":pages}))
}

fn localized(request: &[u8], settings: bool) -> Result<Value> {
    let input = port::input(request)?;
    let context = Context::from_request(request);
    let catalog =
        Catalog::from_metadata(include_bytes!("../metadata.json")).map_err(|_| "internal_error")?;
    let label = |key, fallback| {
        let mut buffer = [0; 512];
        catalog
            .text(&context, key, fallback, &mut buffer)
            .map(String::from)
            .map_err(|_| "internal_error")
    };
    if settings {
        let options = [
            (
                "SINGLE",
                "setting.organization.single",
                "Single book (all comics)",
            ),
            ("BY_YEAR", "setting.organization.year", "By year"),
            (
                "BY_YEAR_MONTH",
                "setting.organization.month",
                "By year and month",
            ),
        ]
        .into_iter()
        .map(|(value, key, fallback)| Ok(json!({"value":value,"label":label(key,fallback)?})))
        .collect::<Result<Vec<_>>>()?;
        return Ok(json!({"settings":[{
            "id":"organization_method","kind":"select","default":"SINGLE","options":options,
            "name":label("setting.organization","Organization method")?,
            "description":label("setting.organization.description","Each organization method keeps its own books and reading progress.")?
        }]}));
    }
    Ok(
        json!({"listings":[{"id":"popular","name":label("listing.popular","SMBC comics")?,
        "supportsPaging":Organization::from_settings(&input) != Organization::Single}]}),
    )
}

fn run_get_manga_list(request: &[u8]) -> u32 {
    port::write_result(
        "get_manga_list",
        port::input(request).and_then(|input| listing(&input)),
    )
}
fn run_get_manga(request: &[u8]) -> u32 {
    port::write_result(
        "get_manga",
        port::input(request).and_then(|input| detail(&input)),
    )
}
fn run_get_listings(request: &[u8]) -> u32 {
    port::write_result("get_listings", localized(request, false))
}
fn run_get_settings(request: &[u8]) -> u32 {
    port::write_result("get_settings", localized(request, true))
}
fn run_get_pages(request: &[u8]) -> u32 {
    port::write_result(
        "get_pages",
        port::input(request).and_then(|input| pages(&input, request)),
    )
}
fn run_get_image_request(request: &[u8]) -> u32 {
    let result = port::input(request).and_then(|input| {
        let url = image_url(input["args"]["url"].as_str().ok_or("invalid_request")?)?;
        Ok(json!({"url":url.as_str(),"headers":{}}))
    });
    port::write_result("get_image_request", result)
}
fn run_get_filters(_: &[u8]) -> u32 {
    port::write_result("get_filters", Ok(json!({"filters":[]})))
}
fn run_get_home(_: &[u8]) -> u32 {
    write_error("get_home", "unsupported", "home unsupported")
}
fn run_search(_: &[u8]) -> u32 {
    write_error("search", "unsupported", "search unsupported")
}
fn write_chapter_failure(code: &str) -> u32 {
    port::write_failure("get_chapters", code)
}

koma_source_sdk::koma_source_exports!("smbc_en", typed_chapters, write_chapter_failure);
