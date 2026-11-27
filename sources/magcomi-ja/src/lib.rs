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
    origin: "https://magcomi.com",
    page_host: "cdn-img.magcomi.com",
    cover_hosts: &["cdn-img.magcomi.com"],
    work_prefix: "magcomi:",
    chapter_prefix: "magcomi-ch:",
    page_prefix: "magcomi-page:",
    metadata: METADATA,
    lock_unavailable_titles: true,
};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.magcomi.ja.koma",
    name: "MAGCOMI",
    language: "ja",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    description: "マッグガーデンの公式マンガサイト。『リィンカーネーションの花弁』『転生貴族の異世界冒険録』などの連載に加え、読切や漫画賞の受賞作品を掲載しています。",
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
    ("serial", &["2550689798442777403", "2550689798442777409"]),
    (
        "oneshot",
        &[
            "2550689798442777417",
            "2550689798442777419",
            "2550689798442777427",
        ],
    ),
    ("awards", &["2550689798442777432", "2550689798442777435"]),
    ("completed", &["2550689798442777437", "2550689798442777447"]),
    ("fantasy", &["Genre:3269754496757284079"]),
    ("sf", &["Genre:3269754496757284092"]),
    ("isekai_men", &["Genre:3269754496757284093"]),
    ("isekai_women", &["Genre:3269754496757284105"]),
    ("comedy", &["Genre:3269754496757284113"]),
    ("food", &["Genre:3269754496757284121"]),
    ("romance", &["Genre:3269754496757284122"]),
    ("bl", &["Genre:3269754496757284126"]),
    ("gl", &["Genre:3269754496757284132"]),
    ("history", &["Genre:3269754496757284140"]),
    ("horror", &["Genre:3269754496757284144"]),
    ("daily", &["Genre:3269754496757284147"]),
    ("drama", &["Genre:3269754496757284148"]),
    ("school", &["Genre:3269754496757284151"]),
    ("nature", &["Genre:3269754496757284155"]),
    ("sports", &["Genre:3269754496757284157"]),
];
const LISTINGS: &[&str] = &[
    "popular",
    "latest",
    "ongoing",
    "oneshot",
    "awards",
    "completed",
];
fn listing(args: &Value) -> Result<Vec<Value>> {
    let id = args["listingId"].as_str().unwrap_or("popular");
    let (ids, sort) = match id {
        "popular" => (browse::selected(args, COLLECTIONS)?, Sort::Likes),
        "latest" => (browse::selected(args, COLLECTIONS)?, Sort::Published),
        "ongoing" => (COLLECTIONS[0].1, Sort::Site),
        "oneshot" => (COLLECTIONS[1].1, Sort::Site),
        "awards" => (COLLECTIONS[2].1, Sort::Site),
        "completed" => (COLLECTIONS[3].1, Sort::Site),
        _ => return Err("invalid_request"),
    };
    browse::collection(&VIEWER, ids, sort)
}
browse::gigaviewer_exports!("magcomi_ja");
