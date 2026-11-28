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
    origin: "https://ourfeel.jp",
    page_host: "cdn-img.ourfeel.jp",
    cover_hosts: &["cdn-img.ourfeel.jp"],
    work_prefix: "ourfeel:",
    chapter_prefix: "ourfeel-ch:",
    page_prefix: "ourfeel-page:",
    metadata: METADATA,
    lock_unavailable_titles: true,
};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.ourfeel.ja.koma",
    name: "OUR FEEL",
    language: "ja",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    description: "シュークリームが贈る女性向けマンガレーベル。『ふつうの女の子』などのオリジナル作品に加え、SNS作品や読切、出張掲載も楽しめます。毎月第1・第3木曜日更新、無料公開中の話を掲載。",
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
            "2550668105997034151",
            "2550668105997034166",
            "2550912965173938828",
            "2550668105997034155",
        ],
    ),
    ("our_feel", &["2550668105997034151"]),
    ("sns", &["2550668105997034166"]),
    ("oneshot", &["2550668105997034159"]),
    ("guest", &["2550912965173938828"]),
    ("completed", &["2550668105997034155"]),
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
browse::gigaviewer_exports!("ourfeel_ja");
