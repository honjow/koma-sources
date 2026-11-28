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
    origin: "https://comic-ogyaaa.com",
    page_host: "cdn-img.comic-ogyaaa.com",
    cover_hosts: &["cdn-img.comic-ogyaaa.com"],
    work_prefix: "comicogyaaa:",
    chapter_prefix: "comicogyaaa-ch:",
    page_prefix: "comicogyaaa-page:",
    metadata: METADATA,
    lock_unavailable_titles: true,
};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.comicogyaaa.ja.koma",
    name: "COMIC OGYAAA!!",
    language: "ja",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    description: "集英社グループのホーム社が運営する公式Webマンガサイト。『邦画プレゼン女子高生 邦キチ！ 映子さん』をはじめ、ギャグや日常系、ファンタジー、ラブコメ、サスペンスなどを掲載。無料公開中の話を読めます。",
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

// 固定上游合集与当前官网核对。缺失 status 不启用 Zenon 专属策略。
const COLLECTIONS: &[Collection] = &[
    ("all", &["2550912965179775890", "2550912965179775891"]),
    ("daily", &["Genre:2550912965179776778"]),
    ("fantasy", &["Genre:2550912965179776774"]),
    ("romcom", &["Genre:2550912965179776763"]),
    ("comedy", &["Genre:2550912965179776769"]),
    ("drama", &["Genre:2550912965179776783"]),
    ("action", &["Genre:2550912965179776779"]),
    ("horror_suspense", &["Genre:2550912965179776784"]),
];
const LISTINGS: &[&str] = &["popular", "latest"];
fn listing(args: &Value) -> Result<Vec<Value>> {
    let (ids, sort) = match args["listingId"].as_str().unwrap_or("popular") {
        "popular" => (browse::selected(args, COLLECTIONS)?, Sort::Likes),
        "latest" => (browse::selected(args, COLLECTIONS)?, Sort::Published),
        _ => return Err("invalid_request"),
    };
    browse::collection(&VIEWER, ids, sort)
}
browse::gigaviewer_exports!("comicogyaaa_ja");
