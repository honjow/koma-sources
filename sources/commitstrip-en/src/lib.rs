#![no_std]
extern crate alloc;

use alloc::{
    collections::BTreeSet,
    format,
    string::{String, ToString},
    vec::Vec,
};
use koma_source_sdk::{
    chapters::{Chapter, ChapterNumber, ChapterResponse, ChapterResult},
    source::{SourceCapabilities, SourceInfo},
};
use serde_json::{json, Value};
use url::Url;

#[path = "../../port_support.rs"]
#[allow(dead_code)]
mod port;
#[path = "../../port_catalog.rs"]
mod port_catalog;

use port::{Node, Result};
use port_catalog::Labels;

#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

const SITE: &str = "https://www.commitstrip.com/";
const ENGLISH: &str = "https://www.commitstrip.com/en/";
const METADATA: &[u8] = include_bytes!("../metadata.json");
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.commitstrip.en.koma",
    name: "CommitStrip",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "en",
    description:
        "English edition of CommitStrip’s webcomic about programmers and life in a web agency.",
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

koma_source_sdk::koma_source_buffers! {
    payload: 192 * 1024, http_out: 1024 * 1024, body: 512 * 1024,
    http_req: 4096, scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

fn site_url(input: &str) -> Result<Url> {
    let url = port::absolute(SITE, input)?;
    if url.scheme() != "https"
        || url.host_str() != Some("www.commitstrip.com")
        || url.port().is_some()
    {
        return Err("invalid_request");
    }
    Ok(url)
}

fn year_number(raw: &str) -> Option<u16> {
    raw.parse::<u16>()
        .ok()
        .filter(|n| raw.len() == 4 && (2012..=9999).contains(n))
}

fn key_date(key: &str) -> Result<String> {
    let mut parts = key.split('/');
    let year = parts
        .next()
        .and_then(year_number)
        .ok_or("invalid_request")?;
    let month = parts
        .next()
        .filter(|m| m.len() == 2)
        .and_then(|m| m.parse::<u8>().ok())
        .filter(|m| (1..=12).contains(m))
        .ok_or("invalid_request")?;
    let day = parts
        .next()
        .filter(|d| d.len() == 2)
        .and_then(|d| d.parse::<u8>().ok())
        .filter(|d| (1..=31).contains(d))
        .ok_or("invalid_request")?;
    let _slug = parts
        .next()
        .filter(|s| {
            !s.is_empty()
                && s.len() <= 256
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'%'))
        })
        .ok_or("invalid_request")?;
    if parts.next().is_some() {
        return Err("invalid_request");
    }
    Ok(format!("{year:04}-{month:02}-{day:02}T00:00:00Z"))
}

fn strip_key(url: &Url) -> Result<&str> {
    let key = url.path().trim_matches('/');
    let key = key.strip_prefix("en/").unwrap_or(key);
    key_date(key)?;
    Ok(key)
}

fn strip_url(key: &str) -> Result<Url> {
    key_date(key)?;
    port::absolute(ENGLISH, &format!("{key}/"))
}

fn artwork(input: &str, cover: bool) -> Option<Url> {
    let url = site_url(input).ok()?;
    let path = url.path();
    let mut pieces = path.strip_prefix("/wp-content/uploads/")?.split('/');
    year_number(pieces.next()?)?;
    let month = pieces.next()?;
    if month.len() != 2 || !month.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let file = pieces.next()?.to_ascii_lowercase();
    let (_, extension) = file.rsplit_once('.')?;
    (pieces.next().is_none()
        && ["jpg", "jpeg", "gif", "png", "webp", "avif"].contains(&extension)
        && (cover || !file.starts_with("headlineimagetemplate")))
    .then_some(url)
}

fn manga(key: &str, title: &str, cover: Option<&Url>) -> Result<Value> {
    let cover = cover.map_or_else(
        || json!({"kind":"none"}),
        |url| json!({"kind":"url","url":url.as_str()}),
    );
    Ok(json!({
        "id":format!("strip:{key}"), "title":title, "url":strip_url(key)?.as_str(),
        "language":"en", "cover":cover, "status":"completed", "contentRating":"sfw"
    }))
}

fn browse(args: &Value, search: bool) -> Result<Value> {
    if !search && !matches!(args["listingId"].as_str(), Some("latest") | None) {
        return Err("invalid_request");
    }
    let query = match &args["query"] {
        Value::Null => "",
        Value::String(q) if search && q.len() <= 512 => q.trim(),
        _ => return Err("invalid_request"),
    };
    let year = match &args["filters"]["year"] {
        Value::Null => "",
        Value::String(year) => year.trim(),
        _ => return Err("invalid_request"),
    };
    if !year.is_empty() && (!query.is_empty() || year_number(year).is_none()) {
        return Err("invalid_request");
    }
    let page = match &args["page"]["cursor"] {
        Value::Null => 1,
        Value::String(cursor) => cursor
            .parse::<usize>()
            .ok()
            .filter(|n| (2..=10_000).contains(n) && n.to_string() == *cursor)
            .ok_or("invalid_request")?,
        _ => return Err("invalid_request"),
    };
    let archive = if year.is_empty() {
        String::new()
    } else {
        format!("{year}/")
    };
    let path = if page == 1 {
        archive.clone()
    } else {
        format!("{archive}page/{page}/")
    };
    let mut url = if path.is_empty() {
        site_url(ENGLISH)?
    } else {
        port::absolute(ENGLISH, &path)?
    };
    if !query.is_empty() {
        url.query_pairs_mut().append_pair("s", query);
    }

    let html = match port::get(url.as_str()) {
        Err("not_found") if !year.is_empty() => return Ok(port::items(Vec::new(), None)),
        result => result?,
    };
    let doc = Node::parse(&html)?;
    let cards = doc.all(".excerpt")?;
    if cards.is_empty() && doc.one(".no-results, .not-found").is_none() {
        return Err("parse_error");
    }
    let full_search_page = !query.is_empty() && cards.len() >= 20;
    let mut seen = BTreeSet::new();
    let mut items = Vec::new();
    for card in cards {
        let link = card.require("section > a")?;
        let Ok(url) = site_url(&link.attr("href")?) else {
            continue;
        };
        let Ok(key) = strip_key(&url) else {
            // WordPress search also returns ordinary pages, such as About.
            continue;
        };
        if !seen.insert(String::from(key)) {
            continue;
        }
        let cover = card
            .one("section > a > img")
            .and_then(|node| artwork(&node.attr("src").ok()?, true));
        if let Some(cover) = cover {
            items.push(manga(
                key,
                &link.require("span strong")?.text()?,
                Some(&cover),
            )?);
        } else {
            // Older strips can share the text-only tile template. Inspect
            // the post to distinguish a comic from a text announcement.
            match detail(key) {
                Ok(strip) => items.push(manga(key, &strip.title, strip.images.first())?),
                Err("chapter_unavailable" | "not_found") => {}
                Err(code) => return Err(code),
            }
        }
    }
    let next = doc
        .one(".wp-pagenavi a[rel=next]")
        .map(|node| -> Result<usize> {
            let next = site_url(&node.attr("href")?)?;
            let path = next.path().trim_start_matches('/');
            if path
                .strip_prefix("en/")
                .unwrap_or(path)
                .trim_end_matches('/')
                != format!("{archive}page/{}", page + 1)
            {
                return Err("parse_error");
            }
            Ok(page + 1)
        })
        .transpose()?;
    Ok(port::items(
        items,
        next.or_else(|| full_search_page.then_some(page + 1)),
    ))
}

struct Strip {
    title: String,
    url: Url,
    images: Vec<Url>,
}

fn detail(key: &str) -> Result<Strip> {
    let url = strip_url(key)?;
    let doc = Node::parse(&port::get(url.as_str())?)?;
    let canonical = site_url(&doc.require("link[rel=canonical]")?.attr("href")?)?;
    if strip_key(&canonical)? != key {
        return Err("parse_error");
    }
    let main = doc.require(".swiper-slide article.post")?;
    let title = main.require(".entry-title")?.text()?;
    let mut images = Vec::new();
    let mut seen = BTreeSet::new();
    for image in main.all(".entry-content p img")? {
        if image
            .attr("height")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .is_some_and(|height| height < 128)
            || image
                .attr("width")
                .ok()
                .and_then(|s| s.parse::<usize>().ok())
                .is_some_and(|width| width < 300)
        {
            continue;
        }
        if let Some(url) = artwork(&image.attr("src")?, false) {
            if seen.insert(url.clone()) {
                images.push(url);
            }
        }
    }
    if images.is_empty() {
        return Err("chapter_unavailable");
    }
    Ok(Strip { title, url, images })
}

fn id(args: &Value, field: &str, prefix: &str) -> Result<String> {
    let key = args[field]
        .as_str()
        .and_then(|id| id.strip_prefix(prefix))
        .ok_or("invalid_request")?;
    key_date(key)?;
    Ok(key.into())
}

fn respond(request: &[u8], op: &str, action: impl FnOnce(&Value) -> Result<Value>) -> u32 {
    let result = port::input(request).and_then(|input| action(&input["args"]));
    port_catalog::write_localized(METADATA, request, op, result)
}

fn run_search(req: &[u8]) -> u32 {
    respond(req, "search", |a| browse(a, true))
}
fn run_get_manga_list(req: &[u8]) -> u32 {
    respond(req, "get_manga_list", |a| browse(a, false))
}
fn run_get_manga(req: &[u8]) -> u32 {
    respond(req, "get_manga", |args| {
        let key = id(args, "mangaId", "strip:")?;
        let strip = detail(&key)?;
        Ok(json!({"manga":manga(&key, &strip.title, strip.images.first())?}))
    })
}
fn run_get_pages(req: &[u8]) -> u32 {
    respond(req, "get_pages", |args| {
        let key = id(args, "chapterId", "strip-ch:")?;
        let chapter = format!("strip-ch:{key}");
        let images = detail(&key)?.images;
        let pages: Vec<_> = images
            .into_iter()
            .enumerate()
            .map(|(i, image)| port::page(i, &chapter, image))
            .collect();
        Ok(json!({"pages":pages}))
    })
}
fn run_get_listings(req: &[u8]) -> u32 {
    respond(req, "get_listings", |_| {
        let name = Labels::new(METADATA, req)?.text("listing.latest", "Latest strips")?;
        Ok(
            json!({"listings":[{"id":"latest","name":name,"supportsPaging":true,"supportsFilters":true}]}),
        )
    })
}
fn run_get_filters(req: &[u8]) -> u32 {
    respond(req, "get_filters", |_| {
        let label = Labels::new(METADATA, req)?;
        Ok(json!({"filters":[{"type":"text","id":"year","default":"",
            "name":label.text("filter.year","Archive year")?,
            "description":label.text("filter.year_hint","Enter a year from 2012 and clear the keyword.")?
        }]}))
    })
}
fn run_get_settings(req: &[u8]) -> u32 {
    respond(req, "get_settings", |_| Ok(json!({"settings":[]})))
}
fn run_get_home(req: &[u8]) -> u32 {
    respond(req, "get_home", |_| Err("not_implemented"))
}
fn run_get_image_request(req: &[u8]) -> u32 {
    respond(req, "get_image_request", |_| Err("not_implemented"))
}

fn run_get_chapters(req: &[u8]) -> ChapterResult<'static, (&'static str, String)> {
    let result = || {
        let input = port::input(req)?;
        let args = &input["args"];
        if !args["page"]["cursor"].is_null() {
            return Err("invalid_request");
        }
        let key = id(args, "mangaId", "strip:")?;
        let strip = detail(&key)?;
        let id = format!("strip-ch:{key}");
        let owner = format!("strip:{key}");
        let date = key_date(&key)?;
        let title = Labels::new(METADATA, req)?.text("chapter.strip", "Comic strip")?;
        let mut chapter = Chapter::new(id.as_bytes(), owner.as_bytes(), title.as_bytes());
        chapter.url = Some(strip.url.as_str().as_bytes());
        chapter.language = Some(b"en");
        chapter.chapter_number = Some(ChapterNumber::Integer(1));
        chapter.published_at = Some(date.as_bytes());
        ChapterResponse::single(payload_buf(), chapter).map_err(port::chapter_error)
    };
    result().map_err(|code| (code, port_catalog::failure(METADATA, req, code)))
}

fn write_chapter_failure((code, message): (&str, String)) -> u32 {
    write_error("get_chapters", code, &message)
}

koma_source_sdk::koma_source_exports!("commitstrip-en", typed_chapters, write_chapter_failure);
