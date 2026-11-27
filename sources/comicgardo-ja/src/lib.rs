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
    origin: "https://comic-gardo.com",
    page_host: "cdn-img.comic-gardo.com",
    cover_hosts: &["cdn-img.comic-gardo.com"],
    work_prefix: "comicgardo:",
    chapter_prefix: "comicgardo-ch:",
    page_prefix: "comicgardo-page:",
    metadata: METADATA,
    lock_unavailable_titles: true,
};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.comicgardo.ja.koma",
    name: "コミックガルド",
    language: "ja",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    description: "オーバーラップの公式マンガサイト。『ありふれた職業で世界最強』などのライトノベルのコミカライズに加え、アンソロジーや連載終了作品を掲載しています。",
    author: "Koma source contributors",
    content_rating: "mixed",
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

// 上游固定分组，以当前官网节点核对；空合集仍保留原身份。
const COLLECTIONS: &[Collection] = &[
    ("serial", &["10834108156664921090"]),
    ("anthology", &["2551460909910482483"]),
    ("completed", &["10834108156664921091"]),
];
const LISTINGS: &[&str] = &["popular", "latest", "ongoing", "anthology", "completed"];
fn listing(args: &Value) -> Result<Vec<Value>> {
    let (ids, sort) = match args["listingId"].as_str().unwrap_or("popular") {
        "popular" => (browse::selected(args, COLLECTIONS)?, Sort::Likes),
        "latest" => (browse::selected(args, COLLECTIONS)?, Sort::Published),
        "ongoing" => (COLLECTIONS[0].1, Sort::Site),
        "anthology" => (COLLECTIONS[1].1, Sort::Site),
        "completed" => (COLLECTIONS[2].1, Sort::Site),
        _ => return Err("invalid_request"),
    };
    browse::collection(&VIEWER, ids, sort)
}
browse::gigaviewer_exports!("comicgardo_ja");
