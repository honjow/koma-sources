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
    host,
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
const SITE: &str = "https://mangapill.com/";
const METADATA: &[u8] = include_bytes!("../metadata.json");
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.mangapill.en.koma", name: "Mangapill", language: "en",
    version: env!("CARGO_PKG_VERSION"), api_version: "0.2",
    description: "An English manga library featuring One Piece, Berserk and Gachiakuta, with one-shots, doujinshi and original English comics. The site no longer carries manhwa.",
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
    settings: false,
    home: false,
    credentials: false,
    image_request: true,
};
const UA: &str = "Mozilla/5.0 (Linux; Android 13; Mobile) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Mobile Safari/537.36";
const TYPES: &[&str] = &[
    "",
    "manga",
    "novel",
    "one-shot",
    "doujinshi",
    "manhua",
    "oel",
];
const STATUS: &[&str] = &[
    "",
    "publishing",
    "finished",
    "on hiatus",
    "discontinued",
    "not yet published",
];
// Values are the live form's checkbox values, not translated labels.
const GENRES: &[&str] = &[
    "Action",
    "Adventure",
    "Cars",
    "Comedy",
    "Dementia",
    "Demons",
    "Doujinshi",
    "Drama",
    "Ecchi",
    "Fantasy",
    "Game",
    "Gender Bender",
    "Harem",
    "Historical",
    "Horror",
    "Isekai",
    "Josei",
    "Kids",
    "Magic",
    "Martial Arts",
    "Mecha",
    "Military",
    "Music",
    "Mystery",
    "Parody",
    "Police",
    "Psychological",
    "Romance",
    "Samurai",
    "School",
    "Sci-Fi",
    "Seinen",
    "Shoujo",
    "Shoujo Ai",
    "Shounen",
    "Shounen Ai",
    "Slice of Life",
    "Space",
    "Sports",
    "Super Power",
    "Supernatural",
    "Thriller",
    "Tragedy",
    "Vampire",
    "Yaoi",
    "Yuri",
];
koma_source_sdk::koma_source_buffers! {payload: 2 * 1024 * 1024, http_out: 8 * 1024 * 1024, body: 4 * 1024 * 1024, http_req: 4096, scratch: 8192,}
koma_source_sdk::koma_source_helpers!();

fn field<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v[key].as_str().ok_or("invalid_request")
}
fn site(raw: &str) -> Result<Url> {
    let u = port::absolute(SITE, raw)?;
    if u.origin().ascii_serialization() != "https://mangapill.com" || u.fragment().is_some() {
        return Err("invalid_request");
    }
    Ok(u)
}
fn key_url(raw: &str, kind: &str) -> Result<Url> {
    let u = site(raw)?;
    let parts = u
        .path()
        .trim_start_matches('/')
        .split('/')
        .collect::<Vec<_>>();
    if parts.len() != 3
        || parts[0] != kind
        || parts[1].is_empty()
        || parts[2].is_empty()
        || !parts[1]
            .bytes()
            .all(|b| b.is_ascii_digit() || (kind == "chapters" && b == b'-'))
        || !parts[2]
            .bytes()
            // Real One Piece extras retain decimal numbers in their slugs,
            // e.g. one-piece-chapter-1151.5. Keep these distinct catalogue IDs.
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || (kind == "chapters" && b == b'.'))
        || u.query().is_some()
    {
        return Err("invalid_request");
    }
    Ok(u)
}
fn work(a: &Value) -> Result<Url> {
    key_url(
        field(a, "mangaId")?
            .strip_prefix("manga:")
            .ok_or("invalid_request")?,
        "manga",
    )
}
fn chapter(a: &Value) -> Result<Url> {
    let u = key_url(
        field(a, "chapterId")?
            .strip_prefix("chapter:")
            .ok_or("invalid_request")?,
        "chapters",
    )?;
    if !a["mangaId"].is_null() {
        let w = work(a)?;
        let wid = w.path().split('/').nth(2).ok_or("invalid_request")?;
        let cid = u
            .path()
            .split('/')
            .nth(2)
            .and_then(|v| v.split_once('-'))
            .ok_or("invalid_request")?
            .0;
        if wid != cid {
            return Err("invalid_request");
        }
    }
    Ok(u)
}
fn document(url: &Url) -> Result<Node> {
    Node::parse(&port::request(
        url.as_str(),
        "GET",
        json!({"User-Agent":UA,"Referer":SITE,"Accept":"text/html"}),
        None,
    )?)
}
fn media(raw: &str, pages: bool) -> Result<Url> {
    let u = port::absolute(SITE, raw)?;
    let host = u.host_str().ok_or("parse_error")?;
    let valid_path = if pages {
        (u.path().starts_with("/file/mangap/") || u.path().starts_with("/file/mangapill/"))
            && !u.path().starts_with("/file/mangapill/i/")
    } else {
        u.path().starts_with("/file/mangapill/i/")
    };
    // Different vintages may use different CDN domains. Accept only the site's
    // unique media paths from its actual DOM; never sweep generic img elements.
    if u.scheme() != "https"
        || !host.contains('.')
        || host.parse::<core::net::IpAddr>().is_ok()
        || host.ends_with(".local")
        || u.port().is_some()
        || u.fragment().is_some()
        || !valid_path
        || !u
            .path()
            .rsplit_once('.')
            .is_some_and(|(_, e)| matches!(e, "jpg" | "jpeg" | "png" | "webp" | "avif"))
    {
        return Err("parse_error");
    }
    Ok(u)
}
fn image(raw: &str, pages: bool) -> Result<Value> {
    Ok(json!({"kind":"url","url":media(raw,pages)?.as_str(),"headers":{"Referer":SITE}}))
}
fn card(row: &Node) -> Result<Value> {
    let u = key_url(&row.require("a[href^='/manga/']")?.attr("href")?, "manga")?;
    let title = row.require(".line-clamp-2")?.text()?;
    if title.is_empty() {
        return Err("parse_error");
    }
    Ok(
        json!({"id":format!("manga:{}",u.path()),"title":title,"url":u.as_str(),"language":"en","contentRating":"mixed","cover":image(&row.require("img[data-src]")?.attr("data-src")?,false)?}),
    )
}
fn page(a: &Value) -> Result<usize> {
    match &a["page"]["cursor"] {
        Value::Null => Ok(1),
        Value::String(s) => s
            .parse::<usize>()
            .ok()
            .filter(|n| (2..=100000).contains(n) && s == &n.to_string())
            .ok_or("invalid_request"),
        _ => Err("invalid_request"),
    }
}
fn choice<'a>(a: &'a Value, key: &str, values: &[&str]) -> Result<&'a str> {
    match &a["filters"][key] {
        Value::Null => Ok(""),
        Value::String(v) if values.contains(&v.as_str()) => Ok(v),
        _ => Err("invalid_request"),
    }
}
fn list(a: &Value, search: bool) -> Result<Value> {
    let number = page(a)?;
    let mut u = site(if search {
        "/search"
    } else {
        match a["listingId"].as_str().unwrap_or("popular") {
            "popular" => "/",
            "latest" => "/chapters",
            _ => return Err("invalid_request"),
        }
    })?;
    if search {
        let q = a["query"].as_str().unwrap_or("").trim();
        if q.len() > 512 || q.chars().any(char::is_control) {
            return Err("invalid_request");
        }
        let mut pairs = u.query_pairs_mut();
        pairs
            .append_pair("q", q)
            .append_pair("page", &number.to_string());
        for (key, values) in [("type", TYPES), ("status", STATUS)] {
            pairs.append_pair(key, choice(a, key, values)?);
        }
        match &a["filters"]["genre"] {
            Value::Null => (),
            Value::Array(genres) => {
                let mut seen = BTreeSet::new();
                for g in genres {
                    let g = g
                        .as_str()
                        .filter(|g| GENRES.contains(g))
                        .ok_or("invalid_request")?;
                    if seen.insert(g) {
                        pairs.append_pair("genre", g);
                    }
                }
            }
            _ => return Err("invalid_request"),
        }
    } else if number != 1 {
        return Err("invalid_request");
    }
    let doc = document(&u)?;
    let rows = if !search && a["listingId"].as_str().unwrap_or("popular") == "popular" {
        // Keep selectors within the host's supported CSS subset. A complex
        // :has() is not supported by the native dev host.
        let mut rows = None;
        for section in doc.all("div.container")? {
            if section.one("h4").map(|h| h.text()).transpose()?.as_deref()
                == Some("Trending Mangas")
            {
                rows = Some(section.all(".grid > div:not([class])")?);
                break;
            }
        }
        rows.ok_or("parse_error")?
    } else {
        doc.all(".grid > div:not([class])")?
    };
    let mut items = Vec::new();
    let mut seen = BTreeSet::new();
    for row in rows {
        let m = card(&row)?;
        if seen.insert(m["id"].as_str().ok_or("parse_error")?.to_string()) {
            items.push(m);
        }
    }
    // A no-results search is valid. Missing fixed discovery blocks is not.
    if !search && items.is_empty() {
        return Err("parse_error");
    }
    let mut next = false;
    if search {
        for a in doc.all("a.btn.btn-sm[href]")? {
            let n = site(&a.attr("href")?)?;
            if n.path() == "/search"
                && n.query_pairs()
                    .any(|(k, v)| k == "page" && v == (number + 1).to_string())
            {
                next = true;
            }
        }
    }
    Ok(port::items(items, next.then_some(number + 1)))
}
fn status(s: &str) -> &'static str {
    match s {
        "publishing" => "ongoing",
        "finished" => "completed",
        "on hiatus" => "hiatus",
        "discontinued" => "cancelled",
        _ => "unknown",
    }
}
fn detail(a: &Value) -> Result<Value> {
    let u = work(a)?;
    let doc = document(&u)?;
    let section = doc
        .all("div.container")?
        .into_iter()
        .find(|n| n.one("h1").is_some())
        .ok_or("parse_error")?;
    let title = section.require("h1")?.text()?;
    let mut m = json!({"id":field(a,"mangaId")?,"url":u.as_str(),"title":title,"language":"en","contentRating":"mixed","cover":image(&section.require("img[data-src]")?.attr("data-src")?,false)?,"authors":[],"artists":[]});
    m["description"] = json!(section.require("p")?.text()?);
    let mut tags = Vec::new();
    let mut seen = BTreeSet::new();
    let mut state = String::new();
    for row in section.all("div.grid > div")? {
        let Some(label) = row.one("label") else {
            continue;
        };
        let key = label.text()?;
        let value = row
            .one("div")
            .map(|n| n.text())
            .transpose()?
            .unwrap_or_default();
        match key.as_str() {
            "Status" => state = value,
            "Type" if !value.is_empty() => {
                seen.insert(value.clone());
                tags.push(value);
            }
            "Author" | "Authors" if !value.is_empty() => m["authors"] = json!([value]),
            "Artist" | "Artists" if !value.is_empty() => m["artists"] = json!([value]),
            _ => (),
        }
    }
    for tag in section.all("a[href*='genre=']")? {
        let t = tag.text()?;
        if seen.insert(t.clone()) {
            tags.push(t);
        }
    }
    m["status"] = json!(status(&state));
    m["tags"] = json!(tags);
    Ok(json!({"manga":m}))
}
fn chapters(req: &[u8]) -> ChapterResult<'static, &'static str> {
    let v = port::input(req)?;
    let a = &v["args"];
    let u = work(a)?;
    if !a["page"]["cursor"].is_null() {
        return Err("invalid_request");
    }
    let doc = document(&u)?;
    let mut data = Vec::new();
    let mut seen = BTreeSet::new();
    let owner = field(a, "mangaId")?;
    for row in doc.all("#chapters > div > a[href^='/chapters/']")? {
        if host::check_cancel() {
            return Err("cancelled");
        }
        let link = key_url(&row.attr("href")?, "chapters")?;
        let wid = u.path().split('/').nth(2).ok_or("parse_error")?;
        if !link
            .path()
            .split('/')
            .nth(2)
            .is_some_and(|c| c.starts_with(&format!("{wid}-")))
        {
            return Err("parse_error");
        }
        if seen.insert(link.path().to_string()) {
            let title = row.text()?;
            if title.is_empty() {
                return Err("parse_error");
            }
            data.push((format!("chapter:{}", link.path()), link, title));
        }
    }
    if data.is_empty() {
        return Err("chapter_unavailable");
    }
    let out = data
        .iter()
        .map(|(id, u, title)| {
            let mut c = Chapter::new(id.as_bytes(), owner.as_bytes(), title.as_bytes());
            c.url = Some(u.as_str().as_bytes());
            c.language = Some(b"en");
            let number = title
                .strip_prefix("Chapter ")
                .and_then(|s| s.split_whitespace().next());
            c.chapter_number = number.and_then(|s| {
                s.parse::<usize>()
                    .ok()
                    .map(ChapterNumber::Integer)
                    .or_else(|| DecimalChapterNumber::new(s.as_bytes()).map(ChapterNumber::Decimal))
            });
            c
        })
        .collect::<Vec<_>>();
    ChapterResponse::ordered(payload_buf(), req, ChapterOrder::NewestFirst, &out)
        .map_err(port::chapter_error)
}
fn pages(a: &Value) -> Result<Value> {
    let u = chapter(a)?;
    let doc = document(&u)?;
    let id = field(a, "chapterId")?;
    let mut pages = Vec::new();
    let mut seen = BTreeSet::new();
    for node in doc.all("picture img.js-page[data-src]")? {
        let u = media(&node.attr("data-src")?, true)?;
        if seen.insert(u.as_str().to_string()) {
            let mut p = port::page(pages.len(), id, u);
            p["image"]["headers"] = json!({"Referer":SITE});
            pages.push(p);
        }
    }
    if pages.is_empty() {
        return Err("chapter_unavailable");
    }
    Ok(json!({"chapterId":id,"pages":pages}))
}
fn call(op: &str, r: &[u8], f: impl FnOnce(&Value) -> Result<Value>) -> u32 {
    port_catalog::write_localized(METADATA, r, op, port::input(r).and_then(|v| f(&v["args"])))
}
fn run_get_manga_list(r: &[u8]) -> u32 {
    call("get_manga_list", r, |a| list(a, false))
}
fn run_search(r: &[u8]) -> u32 {
    call("search", r, |a| list(a, true))
}
fn run_get_manga(r: &[u8]) -> u32 {
    call("get_manga", r, detail)
}
fn run_get_pages(r: &[u8]) -> u32 {
    call("get_pages", r, pages)
}
fn run_get_chapters(r: &[u8]) -> ChapterResult<'static, (&'static str, String)> {
    chapters(r).map_err(|c| (c, port_catalog::failure(METADATA, r, c)))
}
fn write_chapter_failure((c, m): (&str, String)) -> u32 {
    write_error("get_chapters", c, &m)
}
fn run_get_image_request(r: &[u8]) -> u32 {
    call("get_image_request", r, |a| {
        let raw = field(a, "url")?;
        let u = media(raw, true).or_else(|_| media(raw, false))?;
        Ok(json!({"imageRequest":{"url":u.as_str(),"headers":{"Referer":SITE}}}))
    })
}
fn run_get_listings(r: &[u8]) -> u32 {
    call("get_listings", r, |_| {
        let l = Labels::new(METADATA, r)?;
        Ok(
            json!({"listings":[{"id":"popular","name":l.text("listing.popular","Popular")?,"supportsPaging":false,"supportsFilters":false},{"id":"latest","name":l.text("listing.latest","Latest updates")?,"supportsPaging":false,"supportsFilters":false}]}),
        )
    })
}
fn run_get_filters(r: &[u8]) -> u32 {
    call("get_filters", r, |a| {
        if a["listingId"].as_str().is_some_and(|s| !s.is_empty()) {
            return Ok(json!({"filters":[]}));
        }
        let l = Labels::new(METADATA, r)?;
        let mut filters = Vec::new();
        for (key, opts) in [("genre", GENRES), ("type", TYPES), ("status", STATUS)] {
            let options = opts
                .iter()
                .map(|v| Ok(json!({"value":v,"label":l.text(&format!("option.{key}.{v}"),v)?})))
                .collect::<Result<Vec<_>>>()?;
            filters.push(json!({"id":key,"type":if key=="genre"{"multiselect"}else{"select"},"name":l.text(&format!("filter.{key}"),key)?,"default":if key=="genre"{json!([])}else{json!("")},"options":options}));
        }
        Ok(json!({"filters":filters}))
    })
}
fn run_get_settings(r: &[u8]) -> u32 {
    call("get_settings", r, |_| Ok(json!({"settings":[]})))
}
fn run_get_home(r: &[u8]) -> u32 {
    call("get_home", r, |_| Err("not_implemented"))
}
koma_source_sdk::koma_source_exports!("mangapill-en", typed_chapters, write_chapter_failure);
