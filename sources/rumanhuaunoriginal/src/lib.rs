#![no_std]
extern crate alloc;

use alloc::{
    collections::BTreeSet,
    format,
    string::{String, ToString},
    vec::Vec,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use koma_source_sdk::{
    chapters::{
        Chapter, ChapterOrder, ChapterPage, ChapterPlacement, ChapterResponse, ChapterResult,
        GroupedChapters,
    },
    json_utils::request_page_number,
    source::{SourceCapabilities, SourceInfo},
};
use serde_json::{json, Value};

#[path = "../../port_support.rs"]
mod port_support;
#[path = "../../sixmh/src/crypto.rs"]
mod reader_metadata;
use port_support::{self as port, Node, Result};

const BASE: &str = "https://m.rumanhua.org";
const IMAGE_HOST: &str = "https://img1.baipiaoguai.org";
const LISTS: &[(&str, &str)] = &[
    ("custom/top", "排行榜"),
    ("category", "漫画大全"),
    ("custom/day", "日榜"),
    ("custom/week", "周榜"),
    ("custom/month", "月榜"),
    ("custom/update", "最新更新"),
    ("custom/serialize", "连载"),
    ("custom/end", "完结"),
];
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "org.rumanhua.koma",
    name: "如漫画（同名站）",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "rumanhua.org 漫画目录",
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
    filters: true,
    settings: false,
    image_request: false,
    credentials: false,
};
#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;
koma_source_sdk::koma_source_buffers! {
    payload: 1024 * 1024, http_out: 4 * 1024 * 1024, body: 3 * 1024 * 1024,
    http_req: 4096, scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

fn same_site(host: &str) -> bool {
    matches!(host, "rumanhua.org" | "www.rumanhua.org" | "m.rumanhua.org")
}

fn valid_work(path: &str) -> bool {
    path.strip_prefix("/news/").is_some_and(|id| {
        !id.is_empty() && id.len() <= 32 && id.bytes().all(|byte| byte.is_ascii_digit())
    })
}

fn valid_chapter(path: &str) -> bool {
    path.strip_prefix("/show/")
        .and_then(|p| p.strip_suffix(".html"))
        .is_some_and(|code| {
            !code.is_empty()
                && code.len() <= 200
                && code
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        })
}

fn local_path(href: &str, predicate: fn(&str) -> bool) -> Result<String> {
    let url = port::absolute(BASE, href)?;
    if !url.host_str().is_some_and(same_site) || !predicate(url.path()) {
        return Err("parse_error");
    }
    Ok(url.path().to_string())
}

fn manga(id: &str, title: &str) -> Value {
    json!({"id":id,"title":title,"links":[{"kind":"source","url":format!("{BASE}{id}")}]})
}

fn assign_cover(item: &mut Value, node: &Node, selector: &str) -> Result<()> {
    if let Some(img) = node.one(selector) {
        let src = img.attr("src")?;
        if !src.is_empty() {
            item["cover"] = json!({"kind":"url","url":port::absolute(BASE,&src)?.as_str()});
        }
    }
    Ok(())
}

fn metadata(id: &str, doc: &Node) -> Result<Value> {
    let info = doc.require(".detailTop .info")?;
    let title = info.require(".title")?.text()?;
    let mut item = manga(id, &title);
    assign_cover(&mut item, doc, ".detailTop img.cover")?;
    for node in info.all("p.subtitle")? {
        let text = node.text()?;
        let Some((field, value)) = text.split_once('：') else {
            continue;
        };
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        match field.trim() {
            "作者" => item["authors"] = json!([value]),
            "类型" => item["tags"] = json!(value.split_whitespace().collect::<Vec<_>>()),
            _ => {}
        }
    }
    if let Some(desc) = doc.one(".detailContent > p") {
        item["description"] = json!(desc.text()?);
    }
    Ok(item)
}

fn encoded_parameters(raw: &str) -> Result<&str> {
    for (start, _) in raw.match_indices("params") {
        if raw[..start]
            .chars()
            .next_back()
            .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        {
            continue;
        }
        let tail = raw[start + "params".len()..].trim_start();
        let Some(assigned) = tail.strip_prefix('=') else {
            continue;
        };
        let Some(quoted) = assigned.trim_start().strip_prefix('\'') else {
            continue;
        };
        if let Some((value, _)) = quoted.split_once('\'') {
            return Ok(value);
        }
    }
    Err("parse_error")
}

fn parameters(chapter: &str) -> Result<Value> {
    let html = port::get(&format!("{BASE}{chapter}"))?;
    let input = encoded_parameters(&html)?;
    if input.len() > 700 * 1024 {
        return Err("result_too_large");
    }
    let mut bytes = STANDARD.decode(input).map_err(|_| "parse_error")?;
    let json = reader_metadata::decode_metadata(&mut bytes).map_err(|_| "parse_error")?;
    serde_json::from_slice(json).map_err(|_| "parse_error")
}

fn read_id<'a>(args: &'a Value, key: &str, is_valid: fn(&str) -> bool) -> Result<&'a str> {
    args[key]
        .as_str()
        .filter(|id| is_valid(id))
        .ok_or("invalid_request")
}

fn body_document(url: &str) -> Result<Node> {
    Node::parse(&port::get(url)?)
}

fn details(id: &str) -> Result<Value> {
    metadata(id, &body_document(&format!("{BASE}{id}"))?)
}

fn owner(params: &Value) -> Result<String> {
    let item = params.get("comic_id").ok_or("parse_error")?;
    let id = match item {
        Value::String(id) => id.clone(),
        Value::Number(number) => number.to_string(),
        _ => return Err("parse_error"),
    };
    let path = format!("/news/{id}");
    valid_work(&path).then_some(path).ok_or("parse_error")
}

fn routes(input: &Value, request: &[u8], search: bool) -> Result<Value> {
    let args = &input["args"];
    let page = request_page_number(request);
    let term = args["query"].as_str().unwrap_or("").trim();
    let phrase = search && !term.is_empty();
    if phrase && page > 1 {
        return Ok(port::items(Vec::new(), None));
    }

    let mut url = if phrase {
        if let Ok(url) = url::Url::parse(term) {
            if url.host_str().is_some_and(same_site) && matches!(url.scheme(), "http" | "https") {
                let id = if valid_work(url.path()) {
                    url.path().to_string()
                } else if valid_chapter(url.path()) {
                    owner(&parameters(url.path())?)?
                } else {
                    return Err("invalid_request");
                };
                return Ok(port::items(alloc::vec![details(&id)?], None));
            }
        }
        let mut url = port::absolute(BASE, "/index.php/search")?;
        url.query_pairs_mut().append_pair("key", term);
        url
    } else {
        let listing = args["listingId"].as_str().unwrap_or("popular");
        let filter = port::filter(args, "list", LISTS)?;
        let path = if listing == "latest" {
            "custom/update"
        } else if listing == "popular" {
            if filter.is_empty() {
                "custom/top"
            } else {
                filter
            }
        } else {
            return Err("invalid_request");
        };
        if page > 1 && path != "category" {
            return Ok(port::items(Vec::new(), None));
        }
        let suffix = if page > 1 {
            format!("/page/{page}")
        } else {
            String::new()
        };
        port::absolute(BASE, &format!("/{path}{suffix}"))?
    };
    url.set_fragment(None);
    let doc = body_document(url.as_str())?;
    let mut items = Vec::new();
    for row in doc.all("ul.rankList > li")? {
        let id = local_path(&row.require("a")?.attr("href")?, valid_work)?;
        let title = row.require(".title")?.text()?;
        let mut item = manga(&id, &title);
        assign_cover(&mut item, &row, "img")?;
        if let Some(node) = row.one(".bottom") {
            let author = node.text()?;
            if !author.is_empty() {
                item["authors"] = json!([author]);
            }
        }
        items.push(item);
    }
    let next = if !phrase && url.path().starts_with("/category") {
        page.checked_add(1)
            .filter(|n| doc.one(&format!("a[href$='/page/{n}']")).is_some())
    } else {
        None
    };
    Ok(port::items(items, next))
}

fn entry_chapter<'a>(
    index: usize,
    owner: &'a str,
    id: &'a str,
    title: &'a str,
    updated: Option<&'a str>,
) -> Chapter<'a> {
    let mut chapter = Chapter::new(id.as_bytes(), owner.as_bytes(), title.as_bytes());
    chapter.index = Some(index);
    if index == 0 {
        chapter.updated_at = updated.map(str::as_bytes);
    }
    chapter
}

fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(request)?;
    let id = read_id(&input["args"], "mangaId", valid_work)?;
    let doc = body_document(&format!("{BASE}{id}"))?;
    doc.require(".detailTop .info")?;
    let raw_date = doc
        .one("#chapter .top span")
        .map(|n| n.text())
        .transpose()?
        .unwrap_or_default();
    let updated = raw_date
        .split_once('：')
        .map_or("", |(_, value)| value.trim());
    let valid_date =
        updated.len() == 10 && updated.bytes().all(|ch| ch.is_ascii_digit() || ch == b'-');
    let mut entries = Vec::new();
    let mut names = BTreeSet::new();
    let mut urls = BTreeSet::new();
    let mut named_duplicates = false;
    for link in doc.all("ul.chapterList a")? {
        let chapter_id = local_path(&link.attr("href")?, valid_chapter)?;
        if !urls.insert(chapter_id.clone()) {
            continue;
        }
        let title = link.text()?;
        if !names.insert(title.clone()) {
            named_duplicates = true;
        }
        entries.push((chapter_id, title));
    }
    if named_duplicates {
        // The provider shows multiple entries with the same title. Keep each distinct reader URL.
        let mut catalog =
            GroupedChapters::new(payload_buf(), request).map_err(port::chapter_error)?;
        for (i, (chapter_id, title)) in entries.iter().rev().enumerate() {
            catalog
                .push(
                    entry_chapter(i, id, chapter_id, title, valid_date.then_some(updated)),
                    ChapterPlacement::ManualOnly,
                )
                .map_err(port::chapter_error)?;
        }
        return catalog
            .finish(ChapterPage::Complete)
            .map_err(port::chapter_error);
    }
    let mut chapters = Vec::new();
    for (i, (chapter_id, title)) in entries.iter().rev().enumerate() {
        chapters.push(entry_chapter(
            i,
            id,
            chapter_id,
            title,
            valid_date.then_some(updated),
        ));
    }
    ChapterResponse::ordered(payload_buf(), request, ChapterOrder::NewestFirst, &chapters)
        .map_err(port::chapter_error)
}

fn pages(args: &Value) -> Result<Value> {
    let id = read_id(args, "chapterId", valid_chapter)?;
    let data = parameters(id)?;
    if let Some(work) = args["mangaId"].as_str() {
        if owner(&data)? != work {
            return Err("invalid_request");
        }
    }
    let urls = data["images"]
        .as_array()
        .filter(|rows| !rows.is_empty())
        .ok_or("chapter_unavailable")?;
    let pages = urls
        .iter()
        .enumerate()
        .map(|(i, image)| {
            let raw = image.as_str().ok_or("parse_error")?;
            let url = port::absolute(IMAGE_HOST, raw)?;
            Ok(port::page(i, id, url))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(json!({"pages":pages}))
}

fn dispatch(operation: &str, request: &[u8]) -> Result<Value> {
    let input = port::input(request)?;
    let args = &input["args"];
    match operation {
        "get_listings" => Ok(json!({"listings":[
            {"id":"popular","name":"排行榜","supportsPaging":true,"supportsFilters":true},
            {"id":"latest","name":"最新更新","supportsPaging":false,"supportsFilters":false}
        ]})),
        "get_filters" => {
            let mut filter = port::select("list", "列表", "listing", LISTS);
            filter["default"] = json!(LISTS[0].0);
            Ok(json!({"filters":[filter]}))
        }
        "get_manga_list" => routes(&input, request, false),
        "search" => routes(&input, request, true),
        "get_manga" => Ok(json!({"manga":details(read_id(args,"mangaId",valid_work)?)?})),
        "get_pages" => pages(args),
        _ => Err("not_implemented"),
    }
}

fn run(operation: &str, r: &[u8]) -> u32 {
    port::write_result(operation, dispatch(operation, r))
}
fn run_search(r: &[u8]) -> u32 {
    run("search", r)
}
fn run_get_manga(r: &[u8]) -> u32 {
    run("get_manga", r)
}
fn write_chapter_failure(code: &str) -> u32 {
    port::write_failure("get_chapters", code)
}
fn run_get_pages(r: &[u8]) -> u32 {
    run("get_pages", r)
}
fn run_get_listings(r: &[u8]) -> u32 {
    run("get_listings", r)
}
fn run_get_manga_list(r: &[u8]) -> u32 {
    run("get_manga_list", r)
}
fn run_get_filters(r: &[u8]) -> u32 {
    run("get_filters", r)
}
fn run_get_settings(r: &[u8]) -> u32 {
    run("get_settings", r)
}
fn run_get_home(r: &[u8]) -> u32 {
    run("get_home", r)
}
fn run_get_image_request(r: &[u8]) -> u32 {
    run("get_image_request", r)
}
koma_source_sdk::koma_source_exports!("rumanhuaunoriginal", typed_chapters, write_chapter_failure);
