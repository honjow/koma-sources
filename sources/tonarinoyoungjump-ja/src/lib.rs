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
    origin: "https://tonarinoyj.jp",
    page_host: "cdn-img.tonarinoyj.jp",
    cover_hosts: &["cdn.tonarinoyj.jp"],
    work_prefix: "tonari:",
    chapter_prefix: "tonari-ch:",
    page_prefix: "tonari-page:",
    metadata: METADATA,
    lock_unavailable_titles: true,
};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.tonarinoyoungjump.ja.koma",
    name: "となりのヤングジャンプ",
    language: "ja",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    description:
        "集英社の公式マンガサイト。『ワンパンマン』などの連載、読切、出張掲載の公開話を読めます。",
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

const SERIAL: &[&str] = &["3269632237330951493", "3269632237330951504"];
const ONESHOT: &[&str] = &[
    "3269632237330951514",
    "3269632237330951522",
    "10834108156636443058",
];
const GUEST: &[&str] = &["3269632237330951523", "3269632237330951526"];
const LISTINGS: &[&str] = &["popular", "latest", "oneshot", "guest"];
const COLLECTIONS: &[Collection] = &[
    ("serial", SERIAL),
    ("oneshot", ONESHOT),
    ("guest", GUEST),
    ("action", &["Genre:2550912966013206559"]),
    ("comedy", &["Genre:2550912966013206575"]),
    ("fanservice", &["Genre:2550912966013206580"]),
    ("fantasy", &["Genre:2550912966013206593"]),
    ("food", &["Genre:2550912966013206600"]),
    ("horror", &["Genre:2550912966013206611"]),
    ("revenge", &["Genre:2550912966013206615"]),
    ("sports", &["Genre:2550912966013206624"]),
    ("isekai", &["Genre:2550912966013206635"]),
];

fn listing(args: &Value) -> Result<Vec<Value>> {
    let (ids, sort) = match args["listingId"].as_str().unwrap_or("popular") {
        "popular" => (browse::selected(args, COLLECTIONS)?, Sort::Likes),
        "latest" => (browse::selected(args, COLLECTIONS)?, Sort::Published),
        "oneshot" => (ONESHOT, Sort::Site),
        "guest" => (GUEST, Sort::Site),
        _ => return Err("invalid_request"),
    };
    browse::collection(&VIEWER, ids, sort)
}

browse::gigaviewer_exports!("tonarinoyoungjump_ja");
