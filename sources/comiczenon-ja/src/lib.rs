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
    origin: "https://comic-zenon.com",
    page_host: "cdn-img.comic-zenon.com",
    cover_hosts: &["cdn-img.comic-zenon.com"],
    work_prefix: "comiczenon:",
    chapter_prefix: "comiczenon-ch:",
    page_prefix: "comiczenon-page:",
    metadata: METADATA,
    lock_unavailable_titles: true,
};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.comiczenon.ja.koma",
    name: "ゼノンプラス",
    language: "ja",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    description: "コアミックスの公式WEBマンガサイト「ゼノンプラス」。『終末のワルキューレ』シリーズをはじめ、連載作品や読切、漫画賞の作品を掲載しています。",
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
    (
        "serial",
        &[
            "12207421983695504181",
            "12207421983695504182",
            "12207421983695504186",
            "12207421983695504189",
            "12207421983695504190",
            "12207421983695504198",
            "12207421983695504201",
            "10834108156697286365",
            "10834108156697286367",
            "10834108156697286369",
        ],
    ),
    ("oneshot", &["10834108156697286370"]),
    ("awards", &["14079602755177733270"]),
    ("underground", &["Genre:2550912965876210064"]),
    ("horror", &["Genre:2550912965876210087"]),
    ("suspense", &["Genre:2550912965876210090"]),
    ("revenge", &["Genre:2550912965876210101"]),
    ("work", &["Genre:2550912965876210110"]),
    ("medical", &["Genre:2550912965876210117"]),
    ("sports", &["Genre:2550912965876210128"]),
    ("arts", &["Genre:2550912965876210140"]),
    ("food", &["Genre:2550912965876210143"]),
    ("daily", &["Genre:2550912965876210150"]),
    ("romcom", &["Genre:2550912965876210156"]),
    ("battle", &["Genre:2550912965876210160"]),
    ("fandom", &["Genre:2550912965876210168"]),
    ("drama", &["Genre:2550912965876210181"]),
    ("isekai_men", &["Genre:2550912965876210184"]),
    ("isekai_women", &["Genre:2550912965876210188"]),
    ("history_men", &["Genre:2550912965876210190"]),
    ("history_women", &["Genre:2550912965876210194"]),
    ("romance_adult", &["Genre:2550912965876210204"]),
    ("romance_youth", &["Genre:2550912965876210217"]),
];
const LISTINGS: &[&str] = &["popular", "latest", "oneshot", "awards"];
fn listing(args: &Value) -> Result<Vec<Value>> {
    let (ids, sort) = match args["listingId"].as_str().unwrap_or("popular") {
        "popular" => (browse::selected(args, COLLECTIONS)?, Sort::Likes),
        "latest" => (browse::selected(args, COLLECTIONS)?, Sort::Published),
        "oneshot" => (COLLECTIONS[1].1, Sort::Site),
        "awards" => (COLLECTIONS[2].1, Sort::Site),
        _ => return Err("invalid_request"),
    };
    browse::collection(&VIEWER, ids, sort)
}
// Zenon 的旧目录会把下架话标为免费；缺失 status 按上游视为下架。
browse::gigaviewer_exports!("comiczenon_ja", |req| VIEWER
    .chapters_with_missing_status_unavailable(req));
