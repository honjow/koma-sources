#![no_std]

extern crate alloc;

use alloc::{
    collections::{BTreeMap, BTreeSet},
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

const SITE: &str = "https://www.peppercarrot.com";
const ART_PATH: &str = "/0_sources/0ther/";
const AUTHOR: &str = "David Revoy";
const DEFAULT_LANGUAGES: &[&str] = &["en", "cn", "tw", "ja", "fr", "kr"];
const ARTWORKS: &[(&str, &str)] = &[
    ("artworks", "Artworks"),
    ("wallpapers", "Wallpapers"),
    ("sketchbook", "Sketchbook"),
    ("misc", "Misc"),
    ("book-publishing", "Book publishing"),
    ("comissions", "Commissions"),
    ("eshop", "Shop"),
    ("framasoft", "Framasoft"),
    ("press", "Press"),
    ("references", "References"),
    ("wiki", "Wiki"),
];
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.peppercarrot.all.koma",
    name: "Pepper&Carrot",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "all",
    author: "Koma",
    description: "David Revoy 创作的开放版权奇幻漫画，跟随小女巫 Pepper 和她的猫 Carrot。官网收录多语言译本、迷你短篇和作者的艺术作品档案。",
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
    payload: 2 * 1024 * 1024,
    http_out: 5 * 1024 * 1024,
    body: 4 * 1024 * 1024,
    http_req: 4096,
    scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

#[derive(Clone, Copy, PartialEq, Eq)]
enum Collection {
    Comics,
    Theater,
    Artworks,
}

impl Collection {
    fn id(self) -> &'static str {
        match self {
            Self::Comics => "comics",
            Self::Theater => "mini_fantasy_theater",
            Self::Artworks => "artworks",
        }
    }

    fn from_id(id: &str) -> Result<Self> {
        match id {
            "comics" => Ok(Self::Comics),
            "mini_fantasy_theater" => Ok(Self::Theater),
            "artworks" => Ok(Self::Artworks),
            _ => Err("invalid_request"),
        }
    }

    fn work_url(self, key: &str) -> String {
        match self {
            Self::Comics => format!("{SITE}/{key}/webcomics/peppercarrot.html"),
            Self::Theater => format!("{SITE}/{key}/webcomics/miniFantasyTheater.html"),
            Self::Artworks => format!("{SITE}/en/files/{key}.html"),
        }
    }
}

fn language_key(key: &str) -> bool {
    !key.is_empty() && key.len() <= 12 && key.bytes().all(|c| c.is_ascii_lowercase() || c == b'-')
}

fn work_id(collection: Collection, key: &str) -> String {
    format!("pepper:{}:{key}", collection.id())
}

fn work_identity(id: &str) -> Result<(Collection, &str)> {
    let (category, key) = id
        .strip_prefix("pepper:")
        .and_then(|rest| rest.split_once(':'))
        .ok_or("invalid_request")?;
    let collection = Collection::from_id(category)?;
    let valid = if collection == Collection::Artworks {
        ARTWORKS.iter().any(|(entry, _)| *entry == key)
    } else {
        language_key(key)
    };
    if !valid {
        return Err("invalid_request");
    }
    Ok((collection, key))
}

fn json_file(name: &str) -> Result<Value> {
    let body = port::get(&format!("{SITE}/0_sources/{name}"))?;
    serde_json::from_str(&body).map_err(|_| "parse_error")
}

struct Language {
    key: String,
    code: String,
    name: String,
    translators: String,
    translated: usize,
}

fn languages() -> Result<(Vec<Language>, usize)> {
    let episodes = json_file("episodes.json")?;
    let episodes = episodes
        .as_array()
        .filter(|episodes| !episodes.is_empty())
        .ok_or("parse_error")?;
    let mut counts = BTreeMap::new();
    for episode in episodes {
        let translated = episode["translated_languages"]
            .as_array()
            .ok_or("parse_error")?;
        let mut seen = BTreeSet::new();
        for key in translated
            .iter()
            .filter_map(Value::as_str)
            .filter(|key| language_key(key))
        {
            if seen.insert(key) {
                *counts.entry(key).or_insert(0) += 1;
            }
        }
    }
    let data = json_file("langs.json")?;
    let mut result = Vec::new();
    for (key, row) in data.as_object().ok_or("parse_error")? {
        let Some(&translated) = counts.get(key.as_str()) else {
            continue;
        };
        let name = row["local_name"].as_str().ok_or("parse_error")?;
        let code = match key.as_str() {
            "cn" => "zh-Hans",
            "tw" => "zh-Hant",
            _ => row["iso_code"]
                .as_str()
                .filter(|s| !s.is_empty())
                .unwrap_or(key),
        };
        let translators = row["translators"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        result.push(Language {
            key: key.clone(),
            code: code.to_string(),
            name: name.to_string(),
            translators,
            translated,
        });
    }
    if result.is_empty() {
        return Err("parse_error");
    }
    Ok((result, episodes.len()))
}

fn selected_languages(input: &Value) -> Result<Vec<&str>> {
    let Some(value) = input["settings"].get("languages") else {
        return Ok(DEFAULT_LANGUAGES.to_vec());
    };
    let values = value.as_array().ok_or("invalid_request")?;
    let keys = values
        .iter()
        .map(|value| {
            value
                .as_str()
                .filter(|key| language_key(key))
                .ok_or("invalid_request")
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(keys)
}

fn describe_work(
    collection: Collection,
    key: &str,
    language: Option<(&Language, usize)>,
) -> Result<Value> {
    let (title, description, code, cover) = match collection {
        Collection::Artworks => {
            let title = ARTWORKS
                .iter()
                .find(|(id, _)| *id == key)
                .ok_or("not_found")?
                .1;
            (
                title.to_string(),
                String::new(),
                "all",
                "press/low-res/2015-10-12_logo_by-David-Revoy.jpg",
            )
        }
        Collection::Comics | Collection::Theater => {
            let (language, total) = language.ok_or("not_found")?;
            let name = if collection == Collection::Comics {
                "Pepper&Carrot"
            } else {
                "Mini Fantasy Theater"
            };
            let title = if key == "en" {
                name.to_string()
            } else {
                format!("{name} ({})", key.to_ascii_uppercase())
            };
            let (description, cover) = if collection == Collection::Comics {
                (
                    format!(
                        "{} · {}/{total}\n{}",
                        language.name, language.translated, language.translators
                    ),
                    "artworks/low-res/2016-02-24_vertical-cover_remake_by-David-Revoy.jpg",
                )
            } else {
                (
                    language.name.clone(),
                    "artworks/low-res/2018-11-22_vertical-cover-book-three_by-David-Revoy.jpg",
                )
            };
            (title, description, language.code.as_str(), cover)
        }
    };
    Ok(json!({
        "id":work_id(collection,key),"title":title,"description":description,"authors":[AUTHOR],
        "language":code,"status":"ongoing","contentRating":"safe",
        "cover":{"kind":"url","url":format!("{SITE}{ART_PATH}{cover}"),"headers":{}},
        "links":[{"kind":"source","url":collection.work_url(key)}]
    }))
}

fn get_manga_list(input: &Value) -> Result<Value> {
    let args = &input["args"];
    let collection = Collection::from_id(args["listingId"].as_str().unwrap_or("comics"))?;
    if !args["page"]["cursor"].is_null() {
        return Ok(port::items(Vec::new(), None));
    }
    if collection == Collection::Artworks {
        let items = ARTWORKS
            .iter()
            .map(|(key, _)| describe_work(collection, key, None))
            .collect::<Result<Vec<_>>>()?;
        return Ok(port::items(items, None));
    }
    let selected = selected_languages(input)?;
    let (languages, total) = languages()?;
    let mut items = Vec::new();
    let mut seen = BTreeSet::new();
    // Preferences select stable site keys. Unknown or unpublished languages
    // are kept in settings but don't become books until the feed lists them.
    for key in selected {
        if key == "all" {
            for language in &languages {
                if seen.insert(language.key.as_str()) {
                    items.push(describe_work(
                        collection,
                        &language.key,
                        Some((language, total)),
                    )?);
                }
            }
        } else if let Some(language) = languages.iter().find(|row| row.key == key) {
            if seen.insert(language.key.as_str()) {
                items.push(describe_work(
                    collection,
                    &language.key,
                    Some((language, total)),
                )?);
            }
        }
    }
    Ok(port::items(items, None))
}

fn get_detail(id: &str) -> Result<Value> {
    let (collection, key) = work_identity(id)?;
    let item = if collection == Collection::Artworks {
        describe_work(collection, key, None)?
    } else {
        let (languages, total) = languages()?;
        let language = languages
            .iter()
            .find(|row| row.key == key)
            .ok_or("not_found")?;
        describe_work(collection, key, Some((language, total)))?
    };
    Ok(json!({"manga":item}))
}

fn is_site(url: &Url) -> bool {
    url.scheme() == "https"
        && url.host_str() == Some("www.peppercarrot.com")
        && url.port().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
}

fn date(text: &str) -> Option<String> {
    let value = text.as_bytes().windows(10).find(|s| {
        s[4] == b'-'
            && s[7] == b'-'
            && s[..4]
                .iter()
                .chain(&s[5..7])
                .chain(&s[8..])
                .all(u8::is_ascii_digit)
    })?;
    let value = core::str::from_utf8(value).ok()?;
    let month = value[5..7].parse::<u16>().ok()?;
    let day = value[8..].parse::<u16>().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some(format!("{value}T00:00:00Z"))
}

struct Episode {
    id: String,
    title: String,
    url: String,
    number: Option<usize>,
    date: Option<String>,
}

fn episode_catalog(collection: Collection, key: &str) -> Result<Vec<Episode>> {
    let url = collection.work_url(key);
    let doc = Node::parse(&port::get(&url)?)?;
    let figures = doc.all("figure")?;
    if figures.is_empty() {
        return Err("parse_error");
    }
    let count = figures.len();
    let mut episodes = Vec::new();
    let mut seen = BTreeSet::new();
    for (index, figure) in figures.iter().enumerate() {
        if !figure
            .attr("class")
            .unwrap_or_default()
            .split_ascii_whitespace()
            .any(|class| class == "translated")
        {
            continue;
        }
        let link = port::absolute(&url, &figure.require("a")?.attr("href")?)?;
        if !is_site(&link) || !link.path().ends_with(".html") {
            return Err("parse_error");
        }
        if !seen.insert(link.path().to_string()) {
            continue;
        }
        let title = figure.require("img")?.attr("title")?;
        let title = title
            .rsplit_once('（')
            .or_else(|| title.rsplit_once('('))
            .map_or(title.as_str(), |(title, _)| title.trim_end());
        let caption = figure
            .one("figcaption")
            .map(|node| node.text())
            .transpose()?;
        episodes.push(Episode {
            id: format!("pepper-ch:{}:{}", collection.id(), link.path()),
            url: link.to_string(),
            title: title.to_string(),
            number: Some(count - index),
            date: caption.as_deref().and_then(date),
        });
    }
    Ok(episodes)
}

fn artwork_catalog(key: &str) -> Result<Vec<Episode>> {
    let directory = format!("{ART_PATH}{key}/low-res/");
    let url = format!("{SITE}{directory}");
    let doc = Node::parse(&port::get(&url)?)?;
    let mut seen = BTreeSet::new();
    let mut episodes = Vec::new();
    for anchor in doc.all("a[href$='.jpg']")?.into_iter().rev() {
        let link = port::absolute(&url, &anchor.attr("href")?)?;
        if !is_site(&link) {
            continue;
        }
        let Some(file) = link
            .path()
            .strip_prefix(&directory)
            .and_then(|s| s.strip_suffix(".jpg"))
            .filter(|s| !s.contains('/'))
        else {
            continue;
        };
        if !seen.insert(link.path().to_string()) {
            continue;
        }
        let title = file.strip_suffix("_by-David-Revoy").unwrap_or(file);
        let published = title.get(..10).and_then(date);
        let title = if published.is_some() {
            &title[10..]
        } else {
            title
        };
        let title = title.replace(['_', '-'], " ");
        let title = title.trim();
        let mut chars = title.chars();
        let normalized = if let Some(first) = chars.next() {
            format!("{}{}", first.to_uppercase(), chars.as_str())
        } else {
            file.to_string()
        };
        episodes.push(Episode {
            id: format!("pepper-ch:artworks:{}", link.path()),
            url: link.to_string(),
            title: normalized,
            number: None,
            date: published,
        });
    }
    Ok(episodes)
}

fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(request)?;
    let id = input["args"]["mangaId"].as_str().ok_or("invalid_request")?;
    let (collection, key) = work_identity(id)?;
    let episodes = if collection == Collection::Artworks {
        artwork_catalog(key)?
    } else {
        episode_catalog(collection, key)?
    };
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
            chapter.published_at = episode.date.as_deref().map(str::as_bytes);
            if collection == Collection::Artworks {
                chapter.page_count = Some(1);
            }
            chapter
        })
        .collect();
    // Match the upstream order: newest translated figures first, or the
    // directory's filename order reversed for an artwork collection.
    ChapterResponse::ordered(payload_buf(), request, ChapterOrder::NewestFirst, &chapters)
        .map_err(port::chapter_error)
}

fn get_pages(chapter: &str) -> Result<Value> {
    let (category, path) = chapter
        .strip_prefix("pepper-ch:")
        .and_then(|rest| rest.split_once(':'))
        .ok_or("invalid_request")?;
    let collection = Collection::from_id(category)?;
    if !path.starts_with('/') {
        return Err("invalid_request");
    }
    let url = port::absolute(SITE, path).map_err(|_| "invalid_request")?;
    if !is_site(&url) || url.path() != path {
        return Err("invalid_request");
    }
    if collection == Collection::Artworks {
        let (key, file) = path
            .strip_prefix(ART_PATH)
            .and_then(|rest| rest.split_once("/low-res/"))
            .ok_or("invalid_request")?;
        if !ARTWORKS.iter().any(|(entry, _)| *entry == key)
            || file.contains('/')
            || !file.ends_with(".jpg")
        {
            return Err("invalid_request");
        }
        return Ok(json!({"chapterId":chapter,"pages":[port::page(0,chapter,url)]}));
    }
    if !path.ends_with(".html") {
        return Err("invalid_request");
    }
    let doc = Node::parse(&port::get(url.as_str())?)?;
    let mut images = Vec::new();
    for selector in [".webcomic-page img", ".mft-cv-image"] {
        for image in doc.all(selector)? {
            let src = image.attr("src")?;
            if !src.is_empty() {
                images.push(port::absolute(url.as_str(), &src)?);
            }
        }
    }
    let Some(first) = images.first() else {
        return Err("chapter_unavailable");
    };
    if collection == Collection::Comics {
        // The upstream prepends the title illustration only where the first
        // file has the P00 suffix. Preserve existing page order and avoid
        // duplicating page zero if that illustration is already present.
        if let Some(stem) = first.path().strip_suffix("P00.jpg") {
            let path = format!("{stem}.jpg");
            let mut title_image = first.clone();
            title_image.set_path(&path);
            if !images.contains(&title_image) {
                images.insert(0, title_image);
            }
        }
    }
    let pages: Vec<_> = images
        .into_iter()
        .enumerate()
        .map(|(index, url)| port::page(index, chapter, url))
        .collect();
    Ok(json!({"chapterId":chapter,"pages":pages}))
}

fn localized(request: &[u8], settings: bool) -> Result<Value> {
    port::input(request)?;
    let catalog =
        Catalog::from_metadata(include_bytes!("../metadata.json")).map_err(|_| "internal_error")?;
    let context = Context::from_request(request);
    let label = |key, fallback| {
        let mut buffer = [0; 512];
        catalog
            .text(&context, key, fallback, &mut buffer)
            .map(String::from)
            .map_err(|_| "internal_error")
    };
    if settings {
        let mut options: Value = serde_json::from_str(include_str!("../languages.json"))
            .map_err(|_| "internal_error")?;
        options.as_array_mut().ok_or("internal_error")?.insert(
            0,
            json!({
                "value":"all","label":label("setting.languages.all","全部可用语言")?
            }),
        );
        return Ok(json!({"settings":[
            {"id":"languages","kind":"multiselect","name":label("setting.languages","漫画语言")?,
             "description":label("setting.languages.description","仅展示官网收录的译本。")?,
             "options":options,"default":DEFAULT_LANGUAGES},
            {"id":"hi_res","kind":"toggle","name":label("setting.hi_res","高清原图")?,
             "description":label("setting.hi_res.description","已缓存和下载的图片不会随设置改变。")?,"default":"false"}
        ]}));
    }
    let mut listings = Vec::new();
    for (collection, key, fallback) in [
        (Collection::Comics, "listing.comics", "主线漫画"),
        (Collection::Theater, "listing.theater", "迷你奇幻剧场"),
        (Collection::Artworks, "listing.artworks", "官方画集与壁纸"),
    ] {
        listings
            .push(json!({"id":collection.id(),"name":label(key,fallback)?,"supportsPaging":false}));
    }
    Ok(json!({"listings":listings}))
}

fn get_image_request(input: &Value) -> Result<Value> {
    let raw = input["args"]["url"].as_str().ok_or("invalid_request")?;
    let mut url = Url::parse(raw).map_err(|_| "invalid_request")?;
    if url.scheme() != "https"
        || url.host_str() != Some("www.peppercarrot.com")
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("invalid_request");
    }
    if matches!(&input["settings"]["hi_res"], Value::Bool(true))
        || input["settings"]["hi_res"] == "true"
    {
        let path = url.path().replace("/low-res/", "/hi-res/");
        url.set_path(&path);
    }
    Ok(json!({"url":url.as_str(),"headers":{}}))
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

fn run_get_image_request(request: &[u8]) -> u32 {
    port::write_result(
        "get_image_request",
        port::input(request).and_then(|input| get_image_request(&input)),
    )
}

fn run_get_manga_list(request: &[u8]) -> u32 {
    port::write_result(
        "get_manga_list",
        port::input(request).and_then(|input| get_manga_list(&input)),
    )
}

fn run_get_listings(request: &[u8]) -> u32 {
    port::write_result("get_listings", localized(request, false))
}
fn run_get_settings(request: &[u8]) -> u32 {
    port::write_result("get_settings", localized(request, true))
}
fn run_get_home(_: &[u8]) -> u32 {
    port::write_failure("get_home", "not_implemented")
}
fn run_get_filters(_: &[u8]) -> u32 {
    port::write_failure("get_filters", "not_implemented")
}
fn run_search(_: &[u8]) -> u32 {
    port::write_failure("search", "not_implemented")
}
fn write_chapter_failure(code: &str) -> u32 {
    port::write_failure("get_chapters", code)
}

koma_source_sdk::koma_source_exports!("peppercarrot", typed_chapters, write_chapter_failure);
