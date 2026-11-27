#![no_std]
// English archive and viewer compared with Keiyoushi Xkcd.kt (Apache-2.0),
// revision 90b6e6a905ca186945b21ec0b0be4ca9135b8e69.
extern crate alloc;

use alloc::{collections::BTreeSet, format, string::String, vec::Vec};
use koma_source_sdk::{
    chapters::{
        Chapter, ChapterNumber, ChapterPage, ChapterPlacement, ChapterResult, GroupedChapters,
    },
    host,
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

const SITE: &str = "https://xkcd.com/";
const ARCHIVE: &str = "https://xkcd.com/archive/";
const COVER: &str = "https://xkcd.com/s/0b7742.png";
const CHAPTERS_PER_RESPONSE: usize = 1000;
const ARCHIVE_SELECTION_SIZE: usize = 2048;
const DESCRIPTION: &str =
    "Randall Munroe’s original English webcomic about math, language and dry humor. Open interactive comics on the official site.";
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.xkcd.en.koma",
    name: "xkcd",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "en",
    author: "Koma source contributors",
    description: DESCRIPTION,
    content_rating: "safe",
};
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: false,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: true,
    manga_list: true,
    home: false,
    filters: false,
    settings: true,
    credentials: false,
    image_request: true,
};
koma_source_sdk::koma_source_buffers! {
    payload: 1024 * 1024, http_out: 4 * 1024 * 1024, body: 2 * 1024 * 1024,
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

    fn group(self, date: &str) -> &str {
        match self {
            Self::Single => "SINGLE",
            Self::Year => &date[..4],
            Self::Month => &date[..7],
        }
    }
}

fn group_kind(group: &str) -> Result<Organization> {
    if group == "SINGLE" {
        return Ok(Organization::Single);
    }
    let year = group.get(..4).and_then(decimal);
    if !matches!(year, Some(1900..=9999)) {
        return Err("invalid_request");
    }
    match group.len() {
        4 => Ok(Organization::Year),
        7 if group.as_bytes()[4] == b'-'
            && matches!(group.get(5..).and_then(decimal), Some(1..=12)) =>
        {
            Ok(Organization::Month)
        }
        _ => Err("invalid_request"),
    }
}

fn work_group(input: &Value) -> Result<&str> {
    let group = input["args"]["mangaId"]
        .as_str()
        .and_then(|id| id.strip_prefix("xkcd:en:"))
        .ok_or("invalid_request")?;
    group_kind(group)?;
    Ok(group)
}

fn decimal(raw: &str) -> Option<u32> {
    if raw.is_empty() || !raw.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    raw.parse().ok()
}

fn comic_number(raw: &str) -> Option<u32> {
    if raw.starts_with('0') {
        return None;
    }
    decimal(raw)
}

fn publication_date(raw: &str) -> Option<String> {
    let mut parts = raw.split('-');
    let year = decimal(parts.next()?)?;
    let month = decimal(parts.next()?)?;
    let day = decimal(parts.next()?)?;
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return None,
    };
    if !(1900..=9999).contains(&year) || !(1..=days).contains(&day) || parts.next().is_some() {
        return None;
    }
    Some(format!("{year:04}-{month:02}-{day:02}T00:00:00Z"))
}

struct Comic {
    number: u32,
    name: String,
    published_at: String,
}

fn archive() -> Result<Vec<Comic>> {
    let html = port::get(ARCHIVE)?;
    let document = Node::parse(&html)?;
    let mut comics = Vec::new();
    let mut numbers = BTreeSet::new();
    let mut append = |nodes: Vec<Node>| -> Result<()> {
        for node in nodes {
            if host::check_cancel() {
                return Err("cancelled");
            }
            let href = node.attr("href")?;
            let id = href
                .strip_prefix('/')
                .and_then(|path| path.strip_suffix('/'));
            let number = id.and_then(comic_number).ok_or("parse_error")?;
            let published_at = publication_date(&node.attr("title")?).ok_or("parse_error")?;
            let name = node.text()?;
            if name.is_empty() || !numbers.insert(number) {
                return Err("parse_error");
            }
            comics.push(Comic {
                number,
                name,
                published_at,
            });
        }
        Ok(())
    };
    match document.all("#middleContainer > a") {
        Ok(nodes) => append(nodes)?,
        Err("result_too_large") => {
            // Split the DOM query once the archive outgrows the shared 4096-handle buffer.
            for first in (1..).step_by(ARCHIVE_SELECTION_SIZE) {
                let last = first + ARCHIVE_SELECTION_SIZE - 1;
                let selector =
                    format!("#middleContainer > a:nth-of-type(n+{first}):nth-of-type(-n+{last})");
                let nodes = document.all(&selector)?;
                let last_batch = nodes.len() < ARCHIVE_SELECTION_SIZE;
                append(nodes)?;
                if last_batch {
                    break;
                }
            }
        }
        Err(error) => return Err(error),
    }
    if comics.is_empty() {
        return Err("parse_error");
    }
    Ok(comics)
}

fn work(group: &str) -> Value {
    let title = if group == "SINGLE" {
        "xkcd".into()
    } else {
        format!("xkcd {group}")
    };
    json!({
        "id": format!("xkcd:en:{group}"), "title": title,
        "authors": ["Randall Munroe"], "description": DESCRIPTION,
        "language": "en", "status": "ongoing", "contentRating": "safe",
        "cover": {"kind": "url", "url": COVER},
        "links": [{"kind": "source", "url": SITE}]
    })
}

fn get_manga_list(input: &Value) -> Result<Value> {
    let args = &input["args"];
    if let Some(listing) = args.get("listingId") {
        if listing.as_str() != Some("popular") {
            return Err("invalid_request");
        }
    }
    let page = match &args["page"]["cursor"] {
        Value::Null => 1,
        cursor => cursor
            .as_str()
            .and_then(decimal)
            .filter(|number| *number > 0)
            .ok_or("invalid_request")? as usize,
    };
    let organization = Organization::from_settings(input);
    if organization == Organization::Single {
        return Ok(port::items(
            if page == 1 {
                alloc::vec![work("SINGLE")]
            } else {
                Vec::new()
            },
            None,
        ));
    }
    let comics = archive()?;
    let groups: BTreeSet<_> = comics
        .iter()
        .map(|comic| organization.group(&comic.published_at))
        .collect();
    let start = (page - 1).saturating_mul(10);
    let items = groups
        .iter()
        .rev()
        .skip(start)
        .take(10)
        .map(|group| work(group))
        .collect();
    let next = (start.saturating_add(10) < groups.len()).then_some(page.saturating_add(1));
    Ok(port::items(items, next))
}

fn viewer_url(number: u32) -> String {
    format!("{SITE}{number}/")
}

fn get_manga(input: &Value) -> Result<Value> {
    let group = work_group(input)?;
    let mut manga = work(group);
    let organization = group_kind(group)?;
    if organization != Organization::Single {
        let comics = archive()?;
        let newest = comics
            .iter()
            .find(|comic| organization.group(&comic.published_at) == group)
            .ok_or("not_found")?;
        // Listings can use the official logo without making a detail request per card.
        // On the detail route prefer the first comic's image, like the reference.
        match read_viewer(newest.number, false) {
            Ok((url, _)) => manga["cover"]["url"] = json!(url.as_str()),
            Err("cancelled") => return Err("cancelled"),
            Err(_) => {}
        }
    }
    Ok(json!({"manga": manga}))
}

fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(request)?;
    let group = work_group(&input)?;
    let organization = group_kind(group)?;
    let manga_id = input["args"]["mangaId"].as_str().ok_or("invalid_request")?;
    let chapter_prefix = format!("xkcdch:en:{group}:");
    let after = match &input["args"]["page"]["cursor"] {
        Value::Null => None,
        cursor => Some(
            cursor
                .as_str()
                .and_then(|id| id.strip_prefix(&chapter_prefix))
                .and_then(comic_number)
                .ok_or("invalid_request")?,
        ),
    };
    let comics = archive()?;
    let selected: Vec<_> = comics
        .iter()
        .filter(|comic| organization.group(&comic.published_at) == group)
        .collect();
    if selected.is_empty() {
        return Err("not_found");
    }
    // Resolve the last emitted chapter rather than replaying an offset. A
    // comic published between calls only adds a row before the cursor.
    let start = match after {
        Some(number) => {
            selected
                .iter()
                .position(|comic| comic.number == number)
                .ok_or("invalid_request")?
                + 1
        }
        None => 0,
    };
    let mut catalog = GroupedChapters::new(payload_buf(), request).map_err(port::chapter_error)?;
    // Reserve space for the envelope and page cursor. The title can take up
    // to six encoded bytes per input byte; 512 covers the bounded other fields.
    let mut remaining = PAYLOAD_CAP - 1024;
    let mut last_id = String::new();
    for (index, comic) in selected.iter().enumerate().skip(start) {
        let title = format!("{}: {}", comic.number, comic.name);
        let row_budget = 512 + 6 * title.len();
        if index - start == CHAPTERS_PER_RESPONSE || row_budget > remaining {
            if last_id.is_empty() {
                return Err("result_too_large");
            }
            return catalog
                .finish(ChapterPage::NextCursor(last_id.as_bytes()))
                .map_err(port::chapter_error);
        }
        let id = format!("{chapter_prefix}{}", comic.number);
        let url = viewer_url(comic.number);
        let mut chapter = Chapter::new(id.as_bytes(), manga_id.as_bytes(), title.as_bytes());
        chapter.chapter_number = Some(ChapterNumber::Integer(comic.number as usize));
        chapter.published_at = Some(comic.published_at.as_bytes());
        chapter.language = Some(b"en");
        chapter.url = Some(url.as_bytes());
        catalog
            .push(
                chapter,
                ChapterPlacement::InGroup {
                    group: b"source-catalog",
                    position: (selected.len() - index - 1) as u64,
                },
            )
            .map_err(port::chapter_error)?;
        remaining -= row_budget;
        last_id = id;
    }
    catalog
        .finish(ChapterPage::Complete)
        .map_err(port::chapter_error)
}

fn validated_image(location: &str) -> Result<Url> {
    let url = Url::parse(location).map_err(|_| "invalid_request")?;
    if url.scheme() != "https"
        || !matches!(url.host_str(), Some("xkcd.com" | "imgs.xkcd.com"))
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() == "/"
    {
        return Err("invalid_request");
    }
    Ok(url)
}

fn viewer_image(base: &str, image: &Node) -> Result<Url> {
    for attr in ["srcset", "src"] {
        let Ok(value) = image.attr(attr) else {
            continue;
        };
        let value = if attr == "srcset" {
            value
                .split(',')
                .next()
                .and_then(|candidate| candidate.split_ascii_whitespace().next())
                .unwrap_or("")
        } else {
            value.trim()
        };
        if let Ok(url) = port::absolute(base, value) {
            if let Ok(url) = validated_image(url.as_str()) {
                return Ok(url);
            }
        }
    }
    Err("parse_error")
}

fn read_viewer(number: u32, notes: bool) -> Result<(Url, Vec<String>)> {
    let base = viewer_url(number);
    let html = port::get(&base)?;
    let document = Node::parse(&html)?;
    let image = document
        .one("#comic > img")
        .ok_or("interactive comic unsupported")?;
    // The English reference also rejects an image followed by interactive markup.
    if document.one("#comic > img ~ *").is_some() {
        return Err("interactive comic unsupported");
    }
    let url = viewer_image(&base, &image)?;
    let mut paragraphs = Vec::new();
    if notes {
        for name in ["alt", "title"] {
            if let Ok(text) = image.attr(name) {
                if !text.trim().is_empty() {
                    paragraphs.push(text);
                }
            }
        }
    }
    Ok((url, paragraphs))
}

fn get_pages(input: &Value, request: &[u8]) -> Result<Value> {
    let args = &input["args"];
    let chapter = args["chapterId"].as_str().ok_or("invalid_request")?;
    let (group, number) = chapter
        .strip_prefix("xkcdch:en:")
        .and_then(|tail| tail.split_once(':'))
        .ok_or("invalid_request")?;
    group_kind(group)?;
    if let Some(work_id) = args["mangaId"].as_str() {
        if work_id.strip_prefix("xkcd:en:") != Some(group) {
            return Err("invalid_request");
        }
    }
    let id = comic_number(number).ok_or("invalid_request")?;
    // The same viewer supplies the image and plain Unicode notes. Older hosts
    // get the same original-image page ID and no unsupported text descriptor.
    let notes = Context::from_request(request).supports(b"text_page_image_v1");
    let (url, paragraphs) = read_viewer(id, notes)?;
    let mut pages = alloc::vec![json!({
        "id": format!("xkcdpage:en:{group}:{number}:0"), "index": 0,
        "image": {"kind": "url", "url": url.as_str()}
    })];
    if !paragraphs.is_empty() {
        pages.push(json!({
            "id": format!("xkcdpage:en:{group}:{number}:1"), "index": 1,
            "image": {"kind": "text", "text": {
                "template": "webtoon-author-notes", "paragraphs": paragraphs
            }}
        }));
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
        .iter()
        .map(|(value, key, fallback)| Ok(json!({"value":value, "label":label(key, fallback)?})))
        .collect::<Result<Vec<_>>>()?;
        return Ok(json!({"settings":[{
            "id":"organization_method", "kind":"select", "default":"SINGLE", "options":options,
            "name":label("setting.organization","Organization method")?,
            "description":label("setting.organization.description","Each grouping has its own books and reading progress.")?
        }]}));
    }
    Ok(json!({"listings":[{
        "id":"popular", "name":label("listing.popular","xkcd Comics")?,
        "supportsPaging":Organization::from_settings(&input) != Organization::Single
    }]}))
}

fn run_get_manga_list(request: &[u8]) -> u32 {
    port::write_result(
        "get_manga_list",
        port::input(request).and_then(|input| get_manga_list(&input)),
    )
}
fn run_get_manga(request: &[u8]) -> u32 {
    port::write_result(
        "get_manga",
        port::input(request).and_then(|input| get_manga(&input)),
    )
}
fn run_get_listings(request: &[u8]) -> u32 {
    port::write_result("get_listings", localized(request, false))
}
fn run_get_settings(request: &[u8]) -> u32 {
    port::write_result("get_settings", localized(request, true))
}
fn run_get_pages(request: &[u8]) -> u32 {
    match port::input(request).and_then(|input| get_pages(&input, request)) {
        Err("interactive comic unsupported") => {
            write_error("get_pages", "unsupported", "interactive comic unsupported")
        }
        result => port::write_result("get_pages", result),
    }
}
fn run_get_image_request(request: &[u8]) -> u32 {
    let result = port::input(request).and_then(|input| {
        let url = validated_image(input["args"]["url"].as_str().ok_or("invalid_request")?)?;
        Ok(json!({"url":url.as_str(),"headers":{}}))
    });
    port::write_result("get_image_request", result)
}
fn run_get_home(_: &[u8]) -> u32 {
    write_error("get_home", "unsupported", "home unsupported")
}
fn run_get_filters(_: &[u8]) -> u32 {
    write_error("get_filters", "unsupported", "filters unsupported")
}
fn run_search(_: &[u8]) -> u32 {
    write_error("search", "unsupported", "search unsupported")
}
fn write_chapter_failure(code: &str) -> u32 {
    port::write_failure("get_chapters", code)
}

koma_source_sdk::koma_source_exports!("xkcd_en", typed_chapters, write_chapter_failure);
