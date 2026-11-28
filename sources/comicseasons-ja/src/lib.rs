#![no_std]
extern crate alloc;

use alloc::vec::Vec;
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use serde_json::Value;

#[path = "../../gigaviewer_browse.rs"]
mod browse;
#[path = "../../gigaviewer.rs"]
mod giga;
#[path = "../../port_support.rs"]
#[allow(dead_code)]
mod port;
#[path = "../../port_catalog.rs"]
mod port_catalog;
use browse::{Collection, Sort};
use port::Result;

#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

const METADATA: &[u8] = include_bytes!("../metadata.json");
const VIEWER: giga::Site = giga::Site {
    origin: "https://comic-seasons.com",
    page_host: "cdn-img.comic-seasons.com",
    cover_hosts: &["cdn-img.comic-seasons.com"],
    work_prefix: "comicseasons:",
    chapter_prefix: "comicseasons-ch:",
    page_prefix: "comicseasons-page:",
    metadata: METADATA,
    lock_unavailable_titles: true,
};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.comicseasons.ja.koma",
    name: "Seasons",
    language: "ja",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    description: "文藝春秋の恋愛マンガサイト。『悪役令嬢のお嫁さま』『ぼくは青くて透明で』などを掲載し、原作小説の試し読みも配信。毎週金曜更新で、原サイトの無料公開中の話を読めます。",
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
    settings: true,
    home: false,
    credentials: false,
    image_request: true,
};
koma_source_sdk::koma_source_buffers! {
    payload: 512 * 1024, http_out: 8 * 1024 * 1024, body: 4 * 1024 * 1024,
    http_req: 4096, scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

const COLLECTIONS: &[Collection] = &[
    ("all", &["2550912964592144518"]),
    ("novels", &["2550912964592144524"]),
    ("tag_2550912964583261787", &["Genre:2550912964583261787"]),
    ("tag_2550912964583261798", &["Genre:2550912964583261798"]),
    ("tag_2550912964583261807", &["Genre:2550912964583261807"]),
    ("tag_2550912964583261815", &["Genre:2550912964583261815"]),
    ("tag_2550912964583261826", &["Genre:2550912964583261826"]),
    ("tag_2550912964583261831", &["Genre:2550912964583261831"]),
    ("tag_2550912964583261841", &["Genre:2550912964583261841"]),
    ("tag_2550912964583261851", &["Genre:2550912964583261851"]),
    ("tag_2550912964583261853", &["Genre:2550912964583261853"]),
    ("tag_2550912964583261855", &["Genre:2550912964583261855"]),
    ("tag_2550912964583261858", &["Genre:2550912964583261858"]),
    ("tag_2550912964583261866", &["Genre:2550912964583261866"]),
    ("tag_2550912964583261867", &["Genre:2550912964583261867"]),
    ("tag_2550912964583261871", &["Genre:2550912964583261871"]),
    ("tag_2550912964583261881", &["Genre:2550912964583261881"]),
    ("tag_2550912964583261887", &["Genre:2550912964583261887"]),
    ("tag_2550912964583261894", &["Genre:2550912964583261894"]),
    ("tag_2550912964583261897", &["Genre:2550912964583261897"]),
    ("tag_2550912964583261906", &["Genre:2550912964583261906"]),
    ("tag_2550912964583261912", &["Genre:2550912964583261912"]),
    ("tag_2550912964583261921", &["Genre:2550912964583261921"]),
    ("tag_2550912964583261922", &["Genre:2550912964583261922"]),
    ("tag_2550912964583261931", &["Genre:2550912964583261931"]),
    ("tag_2550912964583261933", &["Genre:2550912964583261933"]),
    ("tag_2550912964583261939", &["Genre:2550912964583261939"]),
    ("tag_2550912964583261946", &["Genre:2550912964583261946"]),
    ("tag_2550912964583261948", &["Genre:2550912964583261948"]),
    ("tag_2550912964583261952", &["Genre:2550912964583261952"]),
    ("tag_2550912964583261957", &["Genre:2550912964583261957"]),
    ("tag_2550912964583261964", &["Genre:2550912964583261964"]),
    ("tag_2550912964583261965", &["Genre:2550912964583261965"]),
    ("tag_2550912964583261966", &["Genre:2550912964583261966"]),
    ("tag_2550912964583261973", &["Genre:2550912964583261973"]),
    ("tag_2550912964583261978", &["Genre:2550912964583261978"]),
    ("tag_2550912964583261986", &["Genre:2550912964583261986"]),
    ("tag_2550912964583261995", &["Genre:2550912964583261995"]),
];
const LISTINGS: &[&str] = &["popular", "latest"];

use alloc::format;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::json;
fn collection(args: &Value, sort: Sort) -> Result<Vec<Value>> {
    let ids = browse::selected(args, COLLECTIONS)?;
    if ids.len() == 1 && ids[0].starts_with("Genre:") {
        let data = VIEWER.graphql(json!({
            "query":format!("query($id:ID!){{genre:node(id:$id){{... on Genre{{series{{edges{{node{{{} likeCount latestEpisode{{publishedAt}}}}}}}}}}}}}}",giga::SERIES_FIELDS),
            "variables":{"id":STANDARD.encode(ids[0])}
        }))?;
        if data["genre"].is_null() {
            return Ok(Vec::new());
        }
        return browse::series_items(&VIEWER, browse::edges(&data["genre"]["series"])?, sort);
    }
    browse::collection(&VIEWER, ids, sort)
}
fn listing_search(args: &Value) -> Result<Vec<Value>> {
    collection(args, Sort::Site)
}
fn listing(args: &Value) -> Result<Vec<Value>> {
    let sort = match args["listingId"].as_str().unwrap_or("popular") {
        "popular" => Sort::Likes,
        "latest" => Sort::Published,
        _ => return Err("invalid_request"),
    };
    collection(args, sort)
}
fn source_listings(labels: &Labels<'_>) -> Result<Value> {
    browse::listings(labels, LISTINGS)
}
fn source_filters(labels: &Labels<'_>) -> Result<Value> {
    browse::filters(labels, COLLECTIONS)
}

use koma_source_sdk::chapters::ChapterResult;
use port_catalog::Labels;
fn dispatch(operation: &str, request: &[u8], f: impl FnOnce(&Value) -> Result<Value>) -> u32 {
    port_catalog::write_localized(
        METADATA,
        request,
        operation,
        port::input(request).and_then(|input| f(&input)),
    )
}
fn run_search(req: &[u8]) -> u32 {
    dispatch("search", req, |input| {
        let args = &input["args"];
        let query = args["query"].as_str().unwrap_or("").trim();
        if query.is_empty() {
            return browse::page(listing_search(args)?, &args["page"]["cursor"]);
        }
        if !args["page"]["cursor"].is_null() {
            return Err("invalid_request");
        }
        VIEWER.search(query)
    })
}
fn run_get_manga_list(req: &[u8]) -> u32 {
    dispatch("get_manga_list", req, |input| {
        browse::page(listing(&input["args"])?, &input["args"]["page"]["cursor"])
    })
}
fn run_get_manga(req: &[u8]) -> u32 {
    dispatch("get_manga", req, |input| VIEWER.detail(input))
}
fn run_get_pages(req: &[u8]) -> u32 {
    dispatch("get_pages", req, |input| VIEWER.pages(input))
}
fn run_get_image_request(req: &[u8]) -> u32 {
    dispatch("get_image_request", req, |input| {
        VIEWER.image_request(req, input)
    })
}
fn run_get_chapters(req: &[u8]) -> ChapterResult<'static, (&'static str, alloc::string::String)> {
    VIEWER
        .chapters(req)
        .map_err(|code| (code, port_catalog::failure(METADATA, req, code)))
}
fn write_chapter_failure((code, message): (&str, alloc::string::String)) -> u32 {
    write_error("get_chapters", code, &message)
}
fn run_get_listings(req: &[u8]) -> u32 {
    dispatch("get_listings", req, |_| {
        source_listings(&Labels::new(METADATA, req)?)
    })
}
fn run_get_filters(req: &[u8]) -> u32 {
    dispatch("get_filters", req, |_| {
        source_filters(&Labels::new(METADATA, req)?)
    })
}
fn run_get_settings(req: &[u8]) -> u32 {
    dispatch("get_settings", req, |_| {
        browse::settings(&Labels::new(METADATA, req)?)
    })
}
fn run_get_home(req: &[u8]) -> u32 {
    dispatch("get_home", req, |_| Err("not_implemented"))
}
koma_source_sdk::koma_source_exports!("comicseasons_ja", typed_chapters, write_chapter_failure);
