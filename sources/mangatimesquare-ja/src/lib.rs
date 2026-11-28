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
    origin: "https://mangatime-square.com",
    page_host: "cdn-img.mangatime-square.com",
    cover_hosts: &["cdn-img.mangatime-square.com"],
    work_prefix: "mangatimesquare:",
    chapter_prefix: "mangatimesquare-ch:",
    page_prefix: "mangatimesquare-page:",
    metadata: METADATA,
    lock_unavailable_titles: true,
};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.mangatimesquare.ja.koma",
    name: "まんがタイムスクエア",
    language: "ja",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    description: "芳文社の公式マンガサイト。まんがタイム・まんがホーム・まんがタイムオリジナルの連載とExtraを掲載。『夜の缶詰』など、原サイトで無料公開中の話を読めます。",
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
    (
        "all",
        &[
            "12207421983452274168",
            "12207421983452274175",
            "12207421983452274180",
        ],
    ),
    ("mangatime", &["12207421983452274168"]),
    ("mangahome", &["12207421983452274175"]),
    ("original", &["12207421983452274180"]),
    ("extra", &["12207421983452274185"]),
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
browse::gigaviewer_exports!("mangatimesquare_ja");
