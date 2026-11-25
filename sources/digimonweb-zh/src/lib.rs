#![no_std]
extern crate alloc;

use alloc::{
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use koma_source_sdk::{
    chapters::{Chapter, ChapterPage, ChapterPlacement, ChapterResult, GroupedChapters},
    source::{SourceCapabilities, SourceInfo},
};
use serde_json::{json, Value};
use url::Url;

#[path = "../../port_support.rs"]
mod port_support;
use port_support::{self as port, Node, Result};

const BASE: &str = "https://digimon.net";
const WORK: &str = "/digimondreamers/cn/";
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "net.digimon.web.zh.koma",
    name: "数码宝贝官网",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "数码宝贝官网的《DIGIMON DREAMERS》中文连载。薮野展也画少年鼓堂律被召进数码世界，帮助搭档脉冲兽打破无法进化的诅咒。",
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
    image_request: false,
    credentials: false,
};
#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;
koma_source_sdk::koma_source_buffers! {payload:1024*1024,http_out:4*1024*1024,body:3*1024*1024,http_req:4096,scratch:8192,}
koma_source_sdk::koma_source_helpers!();

fn work_doc() -> Result<Node> {
    Node::parse(&port::get(&format!("{BASE}{WORK}"))?)
}
fn episode(path: &str) -> Result<u64> {
    let number = path
        .strip_prefix(WORK)
        .and_then(|path| path.trim_end_matches('/').strip_prefix("ep_"))
        .and_then(|number| number.parse().ok())
        .filter(|number| *number > 0)
        .ok_or("invalid_request")?;
    Ok(number)
}
fn manga(doc: &Node) -> Result<Value> {
    let cover = doc.require("h3.pg-tit img[src]")?;
    let title = cover.attr("alt")?;
    let url = format!("{BASE}{WORK}");
    let image = port::absolute(&url, &cover.attr("src")?)?;
    let authors = doc
        .all(".pg-tit__explanation dl dd")?
        .iter()
        .map(Node::text)
        .collect::<Result<Vec<_>>>()?;
    let description = doc.require(".pg-intro__text--01")?.text()?;
    Ok(
        json!({"id":WORK,"title":title,"authors":authors,"description":description,
        "cover":{"kind":"url","url":image.as_str()},"links":[{"kind":"source","url":url}]}),
    )
}
fn search(args: &Value) -> Result<Value> {
    let keyword = args["query"].as_str().unwrap_or("").trim();
    let item = manga(&work_doc()?)?;
    let matches = if let Ok(url) = Url::parse(keyword) {
        matches!(url.scheme(), "https" | "http")
            && matches!(url.host_str(), Some("www.digimon.net" | "digimon.net"))
            && (url.path().trim_end_matches('/') == WORK.trim_end_matches('/')
                || episode(url.path()).is_ok())
    } else {
        item["title"]
            .as_str()
            .unwrap_or("")
            .to_lowercase()
            .contains(&keyword.to_lowercase())
    };
    Ok(port::items(
        if matches { vec![item] } else { Vec::new() },
        None,
    ))
}
fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(request)?;
    if input["args"]["mangaId"] != WORK {
        return Err("invalid_request");
    }
    let document = work_doc()?;
    let links = document.all("ul.pg-episodesList a[href]")?;
    let url = format!("{BASE}{WORK}");
    let mut chapters = GroupedChapters::new(payload_buf(), request).map_err(port::chapter_error)?;
    for (index, link) in links.iter().rev().enumerate() {
        let target = port::absolute(&url, &link.attr("href")?)?;
        if target.host_str() != Some("digimon.net") {
            return Err("parse_error");
        }
        let number = episode(target.path())?;
        let title = link.text()?;
        let mut chapter = Chapter::new(target.path().as_bytes(), WORK.as_bytes(), title.as_bytes());
        chapter.index = Some(index);
        chapters
            .push(
                chapter,
                ChapterPlacement::InGroup {
                    group: b"dreamers",
                    position: number,
                },
            )
            .map_err(port::chapter_error)?;
    }
    chapters
        .finish(ChapterPage::Complete)
        .map_err(port::chapter_error)
}
fn pages(args: &Value) -> Result<Value> {
    let path = args["chapterId"].as_str().ok_or("invalid_request")?;
    episode(path)?;
    if args["mangaId"].as_str().is_some_and(|id| id != WORK) {
        return Err("invalid_request");
    }
    let url = format!("{BASE}{path}");
    let doc = Node::parse(&port::get(&url)?)?;
    let mut images = BTreeMap::new();
    for image in doc.all("img[src*='/img_comic_']")? {
        let target = port::absolute(&url, &image.attr("src")?)?;
        let number = target
            .path()
            .rsplit_once("/img_comic_")
            .and_then(|(_, name)| name.split_once('.'))
            .and_then(|(number, _)| number.parse::<usize>().ok())
            .ok_or("parse_error")?;
        images.insert(number, target);
    }
    if images.is_empty() {
        return Err("chapter_unavailable");
    }
    let pages: Vec<_> = images
        .into_values()
        .enumerate()
        .map(|(index, image)| port::page(index, path, image))
        .collect();
    Ok(json!({"pages":pages}))
}
fn dispatch(operation: &str, request: &[u8]) -> Result<Value> {
    let input = port::input(request)?;
    let args = &input["args"];
    match operation {
        "get_listings" => Ok(
            json!({"listings":[{"id":"popular","name":"作品","supportsPaging":false,"supportsFilters":false}]}),
        ),
        "get_manga_list" => Ok(port::items(vec![manga(&work_doc()?)?], None)),
        "search" => search(args),
        "get_manga" if args["mangaId"] == WORK => Ok(json!({"manga":manga(&work_doc()?)?})),
        "get_manga" => Err("invalid_request"),
        "get_pages" => pages(args),
        _ => Err("not_implemented"),
    }
}
fn run(operation: &str, request: &[u8]) -> u32 {
    port::write_result(operation, dispatch(operation, request))
}
fn run_search(request: &[u8]) -> u32 {
    run("search", request)
}
fn run_get_manga(request: &[u8]) -> u32 {
    run("get_manga", request)
}
fn write_chapter_failure(code: &str) -> u32 {
    port::write_failure("get_chapters", code)
}
fn run_get_pages(request: &[u8]) -> u32 {
    run("get_pages", request)
}
fn run_get_listings(request: &[u8]) -> u32 {
    run("get_listings", request)
}
fn run_get_manga_list(request: &[u8]) -> u32 {
    run("get_manga_list", request)
}
fn run_get_filters(request: &[u8]) -> u32 {
    run("get_filters", request)
}
fn run_get_settings(request: &[u8]) -> u32 {
    run("get_settings", request)
}
fn run_get_home(request: &[u8]) -> u32 {
    run("get_home", request)
}
fn run_get_image_request(request: &[u8]) -> u32 {
    run("get_image_request", request)
}
koma_source_sdk::koma_source_exports!("digimonweb-zh", typed_chapters, write_chapter_failure);
