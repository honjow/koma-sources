#![no_std]

extern crate alloc;

use alloc::{
    collections::BTreeSet,
    format,
    string::{String, ToString},
    vec::Vec,
};
use koma_source_sdk::{
    chapters::{Chapter, ChapterNumber, ChapterOrder, ChapterResponse, ChapterResult},
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

const SITE: &str = "https://sai-zen-sen.jp";
const HOME: &str = "https://sai-zen-sen.jp/comics/twi4/";
const WORK_PATH: &str = "/comics/twi4/";
const LINEUP: &str =
    "#lineup_recent > div > section, #lineup > div > section:not(.zadankai):not([id])";
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "jp.saizensen.twi4.ja.koma",
    name: "Twi4",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "ja",
    author: "Koma",
    description: "星海社「最前線」的日文四格漫画栏目，刊载《トモちゃんは女の子！》等原创四格连载。",
    content_rating: "safe",
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
    credentials: false,
    image_request: false,
};

koma_source_sdk::koma_source_buffers! {
    payload: 2 * 1024 * 1024,
    http_out: 5 * 1024 * 1024,
    body: 4 * 1024 * 1024,
    http_req: 4096,
    scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

fn valid_slug(slug: &str) -> bool {
    !slug.is_empty()
        && !slug.starts_with("zadankai")
        && !slug.starts_with("others")
        && slug
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
}

fn work_slug(id: &str) -> Result<&str> {
    id.strip_prefix("twi4:")
        .filter(|slug| valid_slug(slug))
        .ok_or("invalid_request")
}

fn is_site(url: &Url) -> bool {
    url.scheme() == "https" && url.host_str() == Some("sai-zen-sen.jp") && url.port().is_none()
}

fn lineup() -> Result<Vec<Value>> {
    let doc = Node::parse(&port::get(HOME)?)?;
    let mut seen = BTreeSet::new();
    let mut items = Vec::new();
    for row in doc.all(LINEUP)? {
        let Some(anchor) = row.one("div.hgroup > h3 > a") else {
            continue;
        };
        let link = port::absolute(HOME, &anchor.attr("href")?)?;
        if !is_site(&link) {
            continue;
        }
        let Some(slug) = link
            .path()
            .strip_prefix(WORK_PATH)
            .map(|path| path.trim_end_matches('/'))
            .filter(|slug| valid_slug(slug))
        else {
            continue;
        };
        if !seen.insert(slug.to_string()) {
            continue;
        }
        let status = if row.one("ul > li:last-child > em.is-completed").is_some() {
            "completed"
        } else {
            "ongoing"
        };
        let author = row
            .one("div.hgroup > p")
            .map(|node| node.text())
            .transpose()?;
        let mut item = json!({
            "id":format!("twi4:{slug}"), "title":anchor.text()?, "language":"ja",
            "status":status,"contentRating":"safe","authors":author.into_iter().collect::<Vec<_>>(),
            "links":[{"kind":"source","url":format!("{HOME}{slug}/")}]
        });
        add_cover(&row, "div.figgroup > figure > a > img", HOME, &mut item)?;
        items.push(item);
    }
    if items.is_empty() {
        return Err("parse_error");
    }
    Ok(items)
}

fn add_cover(node: &Node, selector: &str, base: &str, item: &mut Value) -> Result<()> {
    if let Some(img) = node.one(selector) {
        let src = img.attr("src")?;
        if !src.is_empty() {
            item["cover"] = json!({"kind":"url","url":port::absolute(base,&src)?.as_str()});
        }
    }
    Ok(())
}

fn get_detail(id: &str) -> Result<Value> {
    let slug = work_slug(id)?;
    let url = format!("{HOME}{slug}/");
    let doc = Node::parse(&port::get(&url)?)?;
    let full_title = doc.require("title")?.text()?;
    let title = full_title
        .split_once('『')
        .and_then(|(_, tail)| tail.rsplit_once('』'))
        .map_or(full_title.as_str(), |(title, _)| title);
    let intro = doc.require("#introduction")?;
    let description = intro
        .one("div > div > p")
        .map(|node| node.text())
        .transpose()?;
    let mut author = None;
    let mut original = None;
    let mut artist = None;
    for staff in intro.all("div > section > header > div > h3")? {
        let (Some(role), Some(name)) = (staff.one("small"), staff.one("span")) else {
            continue;
        };
        match role.text()?.trim_end_matches(['：', ':']).trim() {
            "作者" => {
                author = Some(name.text()?);
            }
            "原作" => original = Some(name.text()?),
            "漫画" => artist = Some(name.text()?),
            _ => {}
        }
    }
    let artist = artist.or_else(|| author.clone());
    let author = original.or(author);
    // As in the upstream update path, only the public lineup knows the
    // completion badge. The work page is the source for credits and synopsis.
    let catalog = lineup()?;
    let status = catalog
        .iter()
        .find(|item| item["id"] == id)
        .and_then(|item| item["status"].as_str())
        .unwrap_or("ongoing");
    let mut item = json!({
        "id":id,"title":title,"description":description.unwrap_or_default(),
        "authors":author.into_iter().collect::<Vec<_>>(),
        "artists":artist.into_iter().collect::<Vec<_>>(),
        "language":"ja","status":status,"contentRating":"safe","links":[{"kind":"source","url":url}]
    });
    add_cover(&intro, "header > div > h2 > img", &url, &mut item)?;
    Ok(json!({"manga":item}))
}

struct Episode {
    id: String,
    url: String,
    title: String,
    number: Option<usize>,
}

fn episode_title(text: String) -> (String, Option<usize>) {
    let Some((title, number)) = text.rsplit_once(" #") else {
        return (text, None);
    };
    let Ok(number) = number.parse::<usize>() else {
        return (text, None);
    };
    let title = title
        .rsplit_once('『')
        .and_then(|(_, tail)| tail.strip_suffix('』'))
        .unwrap_or(title);
    (format!("{number} - {title}"), Some(number))
}

fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(request)?;
    let id = input["args"]["mangaId"].as_str().ok_or("invalid_request")?;
    let slug = work_slug(id)?;
    let url = format!("{HOME}{slug}/");
    let doc = Node::parse(&port::get(&url)?)?;
    let mut seen = BTreeSet::new();
    let mut episodes = Vec::new();
    let chapter_path = format!("{WORK_PATH}{slug}/");
    doc.require("#backnumbers")?;
    for anchor in doc.all("#backnumbers > div > ul > li > a")? {
        let link = port::absolute(&url, &anchor.attr("href")?)?;
        if !is_site(&link)
            || !link.path().starts_with(&chapter_path)
            || link.path() == chapter_path
            || link.query().is_some()
        {
            continue;
        }
        let id = format!("twi4-ch:{}", link.path());
        if !seen.insert(id.clone()) {
            continue;
        }
        let (title, number) = episode_title(anchor.text()?);
        episodes.push(Episode {
            id,
            url: link.to_string(),
            title,
            number,
        });
    }
    // Upstream sorts by parsed episode number. Preserve DOM order for ties
    // and the unnumbered links at the end of that same catalog.
    episodes.sort_by(|a, b| b.number.cmp(&a.number));
    let chapters: Vec<_> = episodes
        .iter()
        .map(|episode| {
            let mut chapter = Chapter::new(
                episode.id.as_bytes(),
                id.as_bytes(),
                episode.title.as_bytes(),
            );
            chapter.url = Some(episode.url.as_bytes());
            chapter.chapter_number = episode.number.map(ChapterNumber::Integer);
            chapter.language = Some(b"ja");
            chapter.page_count = Some(1);
            chapter
        })
        .collect();
    ChapterResponse::ordered(payload_buf(), request, ChapterOrder::NewestFirst, &chapters)
        .map_err(port::chapter_error)
}

fn get_pages(chapter: &str) -> Result<Value> {
    let path = chapter.strip_prefix("twi4-ch:").ok_or("invalid_request")?;
    let (slug, file) = path
        .strip_prefix(WORK_PATH)
        .and_then(|rest| rest.split_once('/'))
        .ok_or("invalid_request")?;
    if !valid_slug(slug) || file.is_empty() {
        return Err("invalid_request");
    }
    let url = port::absolute(SITE, path).map_err(|_| "invalid_request")?;
    if !is_site(&url) || url.path() != path || url.query().is_some() || url.fragment().is_some() {
        return Err("invalid_request");
    }
    let doc = Node::parse(&port::get(url.as_str())?)?;
    let image = doc
        .one("article.comic div > div > p > img")
        .ok_or("chapter_unavailable")?;
    let image = port::absolute(url.as_str(), &image.attr("src")?)?;
    Ok(json!({"chapterId":chapter,"pages":[port::page(0,chapter,image)]}))
}

fn listings(request: &[u8]) -> Result<Value> {
    port::input(request)?;
    let catalog =
        Catalog::from_metadata(include_bytes!("../metadata.json")).map_err(|_| "internal_error")?;
    let context = Context::from_request(request);
    let mut buffer = [0; 128];
    let name = catalog
        .text(&context, "listing.popular", "作品目录", &mut buffer)
        .map_err(|_| "internal_error")?;
    Ok(json!({"listings":[{"id":"popular","name":name,"supportsPaging":false}]}))
}

fn run_search(request: &[u8]) -> u32 {
    let result = (|| {
        let input = port::input(request)?;
        let query = input["args"]["query"]
            .as_str()
            .ok_or("invalid_request")?
            .to_lowercase();
        let mut items = lineup()?;
        items.retain(|item| {
            item["title"]
                .as_str()
                .is_some_and(|s| s.to_lowercase().contains(&query))
        });
        Ok(port::items(items, None))
    })();
    port::write_result("search", result)
}

fn run_get_manga_list(request: &[u8]) -> u32 {
    let result = (|| {
        let input = port::input(request)?;
        let args = &input["args"];
        if !matches!(args["listingId"].as_str(), None | Some("popular")) {
            return Err("invalid_request");
        }
        let items = if args["page"]["cursor"].is_null() {
            lineup()?
        } else {
            Vec::new()
        };
        Ok(port::items(items, None))
    })();
    port::write_result("get_manga_list", result)
}

fn run_get_manga(request: &[u8]) -> u32 {
    let result = port::input(request)
        .and_then(|input| get_detail(input["args"]["mangaId"].as_str().ok_or("invalid_request")?));
    port::write_result("get_manga", result)
}

fn run_get_pages(request: &[u8]) -> u32 {
    let result = port::input(request).and_then(|input| {
        get_pages(
            input["args"]["chapterId"]
                .as_str()
                .ok_or("invalid_request")?,
        )
    });
    port::write_result("get_pages", result)
}

fn run_get_listings(request: &[u8]) -> u32 {
    port::write_result("get_listings", listings(request))
}
fn run_get_home(_: &[u8]) -> u32 {
    port::write_failure("get_home", "not_implemented")
}
fn run_get_filters(_: &[u8]) -> u32 {
    port::write_failure("get_filters", "not_implemented")
}
fn run_get_settings(_: &[u8]) -> u32 {
    port::write_failure("get_settings", "not_implemented")
}
fn run_get_image_request(_: &[u8]) -> u32 {
    port::write_failure("get_image_request", "not_implemented")
}
fn write_chapter_failure(code: &str) -> u32 {
    port::write_failure("get_chapters", code)
}

koma_source_sdk::koma_source_exports!("twi4-ja", typed_chapters, write_chapter_failure);
