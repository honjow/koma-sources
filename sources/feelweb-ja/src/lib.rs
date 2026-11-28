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
    origin: "https://feelweb.jp",
    page_host: "cdn-img.feelweb.jp",
    cover_hosts: &["cdn-img.feelweb.jp"],
    work_prefix: "feelweb:",
    chapter_prefix: "feelweb-ch:",
    page_prefix: "feelweb-page:",
    metadata: METADATA,
    lock_unavailable_titles: true,
};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.feelweb.ja.koma",
    name: "FEEL web",
    language: "ja",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    description: "『女の園の星』など、FEEL YOUNG・マンガJam・onBLUE・SHODENSHA COMICSの作品が揃う祥伝社の無料マンガサイト。毎週金曜日更新で、原サイトの公開期間中の話を読めます。",
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
    (
        "all",
        &[
            "3269754496312166681",
            "3269754496312166688",
            "3269754496312166692",
            "3269754496312166700",
            "3269754496312166718",
            "3269754496312166727",
            "3269754496312166710",
        ],
    ),
    (
        "feel_young",
        &["3269754496312166681", "3269754496312166688"],
    ),
    ("manga_jam", &["3269754496312166692", "3269754496312166700"]),
    ("onblue", &["3269754496312166718", "3269754496312166727"]),
    ("shodensha_comics", &["3269754496312166710"]),
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
browse::gigaviewer_exports!("feelweb_ja");
