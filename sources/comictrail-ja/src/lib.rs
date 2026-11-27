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
    origin: "https://comic-trail.com",
    page_host: "cdn-img.comic-trail.com",
    cover_hosts: &["cdn-img.comic-trail.com"],
    work_prefix: "comictrail:",
    chapter_prefix: "comictrail-ch:",
    page_prefix: "comictrail-page:",
    metadata: METADATA,
    lock_unavailable_titles: true,
};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.comictrail.ja.koma",
    name: "コミックトレイル",
    language: "ja",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    description: "芳文社の公式マンガサイト。『妻、小学生になる。』などの作品を、連載中・連載終了・読切に分けて掲載。匿名で読める範囲は公式サイトの公開状況に従います。",
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

// Pinned upstream groups; titles without a public product remain unavailable.
const COLLECTIONS: &[Collection] = &[
    (
        "serial",
        &[
            "13933686331794204743",
            "13933686331794204744",
            "13933686331794204745",
        ],
    ),
    ("ongoing", &["13933686331794204743"]),
    ("completed", &["13933686331794204744"]),
    ("oneshot", &["13933686331794204745"]),
];
const LISTINGS: &[&str] = &["popular", "latest", "ongoing", "completed", "oneshot"];
fn listing(args: &Value) -> Result<Vec<Value>> {
    let id = args["listingId"].as_str().unwrap_or("popular");
    let (ids, sort) = match id {
        "popular" => (browse::selected(args, COLLECTIONS)?, Sort::Likes),
        "latest" => (browse::selected(args, COLLECTIONS)?, Sort::Published),
        "ongoing" => (COLLECTIONS[1].1, Sort::Site),
        "completed" => (COLLECTIONS[2].1, Sort::Site),
        "oneshot" => (COLLECTIONS[3].1, Sort::Site),
        _ => return Err("invalid_request"),
    };
    browse::collection(&VIEWER, ids, sort)
}
browse::gigaviewer_exports!("comictrail_ja");
