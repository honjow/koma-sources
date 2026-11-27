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
    origin: "https://kuragebunch.com",
    page_host: "cdn-img.kuragebunch.com",
    cover_hosts: &["cdn.kuragebunch.com"],
    work_prefix: "kuragebunch:",
    chapter_prefix: "kuragebunch-ch:",
    page_prefix: "kuragebunch-page:",
    metadata: METADATA,
    lock_unavailable_titles: true,
};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.kuragebunch.ja.koma",
    name: "くらげバンチ",
    language: "ja",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    description: "新潮社の公式マンガサイト。『極主夫道』『働かないふたり』などの連載に加え、読切や KANATA の作品を掲載しています。",
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

// Keep the pinned upstream collection IDs, including empty legacy groups.
const COLLECTIONS: &[Collection] = &[
    (
        "serial",
        &[
            "13933686331612818223",
            "316112896902234328",
            "316112896902234338",
            "316112896902234351",
        ],
    ),
    ("oneshot", &["13933686331612818248"]),
    ("bunch", &["316112896902234380", "316112896902234391"]),
    (
        "kanata",
        &[
            "14079602755178190495",
            "14079602755178190507",
            "14079602755178190520",
        ],
    ),
    ("drama", &["Genre:3270375685434986756"]),
    ("food", &["Genre:3270375685434986819"]),
    ("romance", &["Genre:3270375685434986861"]),
    ("erotic", &["Genre:3270375685434986895"]),
    ("comedy", &["Genre:3270375685434986939"]),
    ("animals", &["Genre:3270375685434986972"]),
    ("essays", &["Genre:3270375685434987001"]),
    ("fantasy", &["Genre:3270375685434987045"]),
    ("action", &["Genre:3270375685434987087"]),
    ("hobbies", &["Genre:3270375685434987121"]),
    ("daily", &["Genre:3270375685434987160"]),
    ("horror", &["Genre:3270375685434987197"]),
    ("sports", &["Genre:3270375685434987232"]),
    ("sf", &["Genre:3270375685434987276"]),
    ("history", &["Genre:2551460910062589035"]),
];
const LISTINGS: &[&str] = &["popular", "latest", "oneshot", "bunch", "kanata"];
fn listing(args: &Value) -> Result<Vec<Value>> {
    let (ids, sort) = match args["listingId"].as_str().unwrap_or("popular") {
        "popular" => (browse::selected(args, COLLECTIONS)?, Sort::Likes),
        "latest" => (browse::selected(args, COLLECTIONS)?, Sort::Published),
        "oneshot" => (COLLECTIONS[1].1, Sort::Site),
        "bunch" => (COLLECTIONS[2].1, Sort::Site),
        "kanata" => (COLLECTIONS[3].1, Sort::Site),
        _ => return Err("invalid_request"),
    };
    browse::collection(&VIEWER, ids, sort)
}
browse::gigaviewer_exports!("kuragebunch_ja");
