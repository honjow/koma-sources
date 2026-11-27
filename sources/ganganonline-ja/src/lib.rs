#![no_std]
extern crate alloc;

use alloc::{collections::BTreeSet, format, string::String, vec::Vec};
use koma_source_sdk::{
    chapters::{Chapter, ChapterPage, ChapterPlacement, ChapterResult, GroupedChapters},
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

use port::Result;
use port_catalog::Labels;

#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

const SITE: &str = "https://www.ganganonline.com";
const LANGUAGE: &str = "ja";
const METADATA: &[u8] = include_bytes!("../metadata.json");
const WORK_ID: &str = "gangan:ja:";
const CHAPTER_ID: &str = "gangan:ja:chapter:";
const CATEGORIES: &[(&str, &str)] = &[
    ("ongoing", "/rensai"),
    ("finished", "/finish"),
    ("pixiv", "/pixiv"),
    ("ga", "/ga"),
];
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.ganganonline.ja.koma",
    name: "ガンガンONLINE",
    language: LANGUAGE,
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    description: "スクウェア・エニックスの公式ウェブマンガ誌。連載、読み切り、ガンガンpixiv・ガンガンGAの公開話を読めます。",
    author: "Koma source contributors",
    content_rating: "safe",
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
    payload: 512 * 1024, http_out: 2 * 1024 * 1024, body: 1024 * 1024,
    http_req: 4096, scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

fn decimal(value: &str) -> Option<u32> {
    if value.starts_with('0') || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}

fn provider_id(value: &Value) -> Result<u32> {
    value
        .as_u64()
        .and_then(|id| u32::try_from(id).ok())
        .filter(|&id| id != 0)
        .ok_or("parse_error")
}

fn work_id(value: &Value) -> Result<u32> {
    value
        .as_str()
        .and_then(|value| value.strip_prefix(WORK_ID))
        .and_then(decimal)
        .ok_or("invalid_request")
}

fn chapter_ids(value: &Value) -> Result<(u32, u32)> {
    value
        .as_str()
        .and_then(|value| value.strip_prefix(CHAPTER_ID))
        .and_then(|value| value.split_once(':'))
        .and_then(|(work, chapter)| Some((decimal(work)?, decimal(chapter)?)))
        .ok_or("invalid_request")
}

fn next_data(path: &str, query: Option<&str>) -> Result<Value> {
    let mut url = port::absolute(SITE, path)?;
    if let Some(query) = query {
        url.query_pairs_mut().append_pair("keyword", query);
    }
    let html = port::get(url.as_str())?;
    // Next JSON may exceed the DOM text buffer on the full catalog. Read the
    // raw script body to preserve provider strings and parse only that JSON.
    let raw = html
        .split("<script")
        .skip(1)
        .filter_map(|node| node.split_once('>'))
        .find(|(tag, _)| {
            tag.split_ascii_whitespace()
                .any(|attr| attr == "id=\"__NEXT_DATA__\"")
        })
        .and_then(|(_, body)| body.split_once("</script>").map(|(json, _)| json))
        .ok_or("parse_error")?;
    let mut next: Value = serde_json::from_str(raw).map_err(|_| "parse_error")?;
    let page = &mut next["props"]["pageProps"];
    if !page["error"].is_null() {
        return Err("upstream_error");
    }
    let data = page["data"].take();
    data.is_object().then_some(data).ok_or("parse_error")
}

fn cover_url(raw: &str) -> Result<Url> {
    let url = port::absolute(SITE, raw)?;
    if url.scheme() != "https"
        || url.host_str() != Some("www.ganganonline.com")
        || url.port().is_some()
        || url.fragment().is_some()
        || !url.path().starts_with("/secure/")
    {
        return Err("parse_error");
    }
    Ok(url)
}

fn work_card(row: &Value, id: u32, title: &str) -> Result<Value> {
    let cover = row["imageUrl"]
        .as_str()
        .filter(|url| !url.is_empty())
        .map(cover_url)
        .transpose()?;
    let cover = cover.map_or_else(
        || json!({"kind":"none"}),
        |url| json!({"kind":"url","url":url.as_str(),"headers":{}}),
    );
    Ok(
        json!({"id":format!("{WORK_ID}{id}"),"url":format!("{SITE}/title/{id}"),
        "title":title,"language":LANGUAGE,"contentRating":"safe","cover":cover}),
    )
}

fn append_cards(rows: &Value, seen: &mut BTreeSet<u32>, cards: &mut Vec<Value>) -> Result<()> {
    for row in rows.as_array().ok_or("parse_error")? {
        if row["isNovel"].as_bool() == Some(true) {
            continue;
        }
        let id = provider_id(&row["titleId"])?;
        if !seen.insert(id) {
            continue;
        }
        let title = row["title"]
            .as_str()
            .or_else(|| row["header"].as_str())
            .or_else(|| row["name"].as_str())
            .filter(|title| !title.is_empty())
            .ok_or("parse_error")?;
        cards.push(work_card(row, id, title)?);
    }
    Ok(())
}

fn catalog(path: &str, query: Option<&str>) -> Result<Value> {
    let data = next_data(path, query)?;
    let mut seen = BTreeSet::new();
    let mut cards = Vec::new();
    match path {
        "/" => {
            // This is the publisher's dedicated "today's updates" section.
            let section = data["sections"]
                .as_array()
                .ok_or("parse_error")?
                .iter()
                .map(|item| &item["titleSection"])
                .find(|item| item["header"] == "今日の更新作品")
                .ok_or("parse_error")?;
            append_cards(&section["titles"], &mut seen, &mut cards)?;
        }
        "/rensai" | "/finish" => {
            for section in data["titleSections"].as_array().ok_or("parse_error")? {
                append_cards(&section["titles"], &mut seen, &mut cards)?;
            }
        }
        "/search/result" => {
            for section in data["sections"].as_array().ok_or("parse_error")? {
                append_cards(&section["titleLinks"], &mut seen, &mut cards)?;
            }
        }
        "/pixiv" => append_cards(&data["ganganTitles"], &mut seen, &mut cards)?,
        "/ga" => {
            append_cards(
                &data["ongoingTitleSection"]["titles"],
                &mut seen,
                &mut cards,
            )?;
            append_cards(
                &data["finishedTitleSection"]["titles"],
                &mut seen,
                &mut cards,
            )?;
        }
        _ => return Err("invalid_request"),
    }
    Ok(port::items(cards, None))
}

fn unpaged(args: &Value) -> Result<()> {
    args["page"]["cursor"]
        .is_null()
        .then_some(())
        .ok_or("invalid_request")
}

fn browse(args: &Value, search: bool) -> Result<Value> {
    unpaged(args)?;
    if search {
        match &args["query"] {
            Value::Null => {}
            Value::String(query) if query.len() <= 512 => {
                let query = query.trim();
                if !query.is_empty() {
                    return catalog("/search/result", Some(query));
                }
            }
            _ => return Err("invalid_request"),
        }
    } else {
        match &args["listingId"] {
            Value::String(id) if id == "latest" => return catalog("/", None),
            Value::String(id) if id == "popular" => {}
            Value::Null => {}
            _ => return Err("invalid_request"),
        }
    }
    let category = match &args["filters"]["category"] {
        Value::Null => "ongoing",
        Value::String(category) => category,
        _ => return Err("invalid_request"),
    };
    let &(_, path) = CATEGORIES
        .iter()
        .find(|(id, _)| *id == category)
        .ok_or("invalid_request")?;
    catalog(path, None)
}

fn detail(id: u32) -> Result<Value> {
    let mut data = next_data(&format!("/title/{id}"), None)?;
    let detail = data["default"].take();
    if provider_id(&detail["titleId"])? != id {
        return Err("parse_error");
    }
    Ok(detail)
}

fn chapter_locked(row: &Value) -> bool {
    // The public website enum uses 0 (also the default) for PUBLISHED,
    // 1 for UNPUBLISHED, 2 for FINISHED and 3 for PRE_RELEASE.
    !row["status"].is_null() && row["status"].as_u64() != Some(0)
}

fn published(row: &Value) -> Option<String> {
    let raw = row["publishingPeriod"].as_str()?.split('〜').next()?;
    let mut pieces = raw.split('.');
    let year = decimal(pieces.next()?)?;
    let month = pieces.next()?.parse::<u8>().ok()?;
    let day = pieces.next()?.parse::<u8>().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || pieces.next().is_some() {
        return None;
    }
    Some(format!("{year:04}-{month:02}-{day:02}"))
}

fn chapters(args: &Value, req: &[u8]) -> ChapterResult<'static, &'static str> {
    unpaged(args)?;
    let id = work_id(&args["mangaId"])?;
    let work = format!("{WORK_ID}{id}");
    let detail = detail(id)?;
    let rows = detail["chapters"].as_array().ok_or("parse_error")?;
    let mut seen = BTreeSet::new();
    let mut output = GroupedChapters::new(payload_buf(), req).map_err(port::chapter_error)?;
    for (index, row) in rows.iter().enumerate() {
        if host::check_cancel() {
            return Err("cancelled");
        }
        if row["id"].is_null() {
            continue;
        }
        let number = provider_id(&row["id"])?;
        if !seen.insert(number) {
            continue;
        }
        let locked = chapter_locked(row);
        let title = row["mainText"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or("parse_error")?;
        let prefix = if locked { "🔒 " } else { "" };
        let title = match row["subText"].as_str().filter(|text| !text.is_empty()) {
            Some(subtitle) => format!("{prefix}{title} - {subtitle}"),
            None => format!("{prefix}{title}"),
        };
        let url = format!("{SITE}/title/{id}/chapter/{number}");
        let chapter_id = format!("{CHAPTER_ID}{id}:{number}");
        let date = published(row);
        let mut chapter = Chapter::new(chapter_id.as_bytes(), work.as_bytes(), title.as_bytes());
        chapter.url = Some(url.as_bytes());
        chapter.published_at = date.as_deref().map(str::as_bytes);
        chapter.language = Some(LANGUAGE.as_bytes());
        chapter.locked = Some(locked);
        let placement = if locked {
            ChapterPlacement::ManualOnly
        } else {
            ChapterPlacement::InGroup {
                group: b"catalog",
                position: (rows.len() - 1 - index) as u64,
            }
        };
        output
            .push(chapter, placement)
            .map_err(port::chapter_error)?;
    }
    output
        .finish(ChapterPage::Complete)
        .map_err(port::chapter_error)
}

fn pages(args: &Value) -> Result<Value> {
    let (work, number) = chapter_ids(&args["chapterId"])?;
    if args.get("mangaId").is_some() && work_id(&args["mangaId"])? != work {
        return Err("invalid_request");
    }
    let detail = detail(work)?;
    let entry = detail["chapters"]
        .as_array()
        .ok_or("parse_error")?
        .iter()
        .find(|row| row["id"].as_u64() == Some(u64::from(number)))
        .ok_or("chapter_unavailable")?;
    if chapter_locked(entry) {
        return Err("chapter_locked");
    }
    let chapter_path = format!("/title/{work}/chapter/{number}");
    let viewer = next_data(&chapter_path, None)?;
    if viewer["titleDetailUrl"].as_str() != Some(&format!("/title/{work}")) {
        return Err("chapter_unavailable");
    }
    let images = viewer["pages"].as_array().ok_or("chapter_unavailable")?;
    if images.is_empty() {
        return Err("chapter_unavailable");
    }
    let body_prefix = format!("/secure{chapter_path}/manga_page/");
    let chapter = format!("{CHAPTER_ID}{work}:{number}");
    let mut pages = Vec::with_capacity(images.len());
    let mut has_body = false;
    for (index, image) in images.iter().enumerate() {
        let raw = image["image"]["imageUrl"]
            .as_str()
            .or_else(|| image["linkImage"]["imageUrl"].as_str())
            .ok_or("parse_error")?;
        let url = cover_url(raw)?;
        let is_body = url.path().starts_with(&body_prefix);
        if !is_body && !url.path().starts_with("/secure/extra_manga_page/") {
            return Err("parse_error");
        }
        has_body |= is_body;
        // Keep the provider's signed URL and the reader's independent page ID.
        pages.push(port::page(index, &chapter, url));
    }
    if !has_body {
        return Err("chapter_unavailable");
    }
    Ok(json!({"chapterId":chapter,"pages":pages}))
}

fn call(req: &[u8], op: &str, action: impl FnOnce(&Value) -> Result<Value>) -> u32 {
    let result = port::input(req).and_then(|input| action(&input["args"]));
    port_catalog::write_localized(METADATA, req, op, result)
}

fn run_get_manga_list(req: &[u8]) -> u32 {
    call(req, "get_manga_list", |args| browse(args, false))
}

fn run_search(req: &[u8]) -> u32 {
    call(req, "search", |args| browse(args, true))
}

fn run_get_manga(req: &[u8]) -> u32 {
    call(req, "get_manga", |args| {
        let id = work_id(&args["mangaId"])?;
        let detail = detail(id)?;
        let title = detail["titleName"].as_str().ok_or("parse_error")?;
        let mut work = work_card(&detail, id, title)?;
        work["description"] = detail["description"].clone();
        if let Some(author) = detail["author"].as_str().filter(|s| !s.is_empty()) {
            work["authors"] = json!([author]);
        }
        Ok(json!({"manga":work}))
    })
}

fn run_get_chapters(req: &[u8]) -> ChapterResult<'static, (&'static str, String)> {
    port::input(req)
        .and_then(|input| chapters(&input["args"], req))
        .map_err(|code| (code, port_catalog::failure(METADATA, req, code)))
}

fn write_chapter_failure((code, message): (&str, String)) -> u32 {
    write_error("get_chapters", code, &message)
}

fn run_get_pages(req: &[u8]) -> u32 {
    call(req, "get_pages", pages)
}

fn run_get_listings(req: &[u8]) -> u32 {
    call(req, "get_listings", |_| {
        let labels = Labels::new(METADATA, req)?;
        Ok(json!({"listings":[
            {"id":"popular","name":labels.text("listing.popular","Series catalog")?,
             "supportsPaging":false,"supportsFilters":true},
            {"id":"latest","name":labels.text("listing.latest","Updated today")?,
             "supportsPaging":false,"supportsFilters":false}
        ]}))
    })
}

fn run_get_filters(req: &[u8]) -> u32 {
    call(req, "get_filters", |_| {
        let labels = Labels::new(METADATA, req)?;
        let options = CATEGORIES
            .iter()
            .map(|(id, _)| {
                Ok(json!({"value":id,"label":labels.text(&format!("category.{id}"),id)?}))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(json!({"filters":[{
            "type":"select","id":"category","name":labels.text("filter.category","Catalog")?,
            "description":labels.text("filter.category_hint","Applies when the search is blank.")?,
            "default":"ongoing","options":options
        }]}))
    })
}

fn run_get_settings(req: &[u8]) -> u32 {
    call(req, "get_settings", |_| Ok(json!({"settings":[]})))
}

fn run_get_home(req: &[u8]) -> u32 {
    call(req, "get_home", |_| Err("not_implemented"))
}

fn run_get_image_request(req: &[u8]) -> u32 {
    call(req, "get_image_request", |_| Err("not_implemented"))
}

koma_source_sdk::koma_source_exports!("ganganonline_ja", typed_chapters, write_chapter_failure);
