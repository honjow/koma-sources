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
    origin: "https://comic-earthstar.com",
    page_host: "cdn-img.comic-earthstar.com",
    cover_hosts: &["cdn-img.comic-earthstar.com"],
    work_prefix: "comicearthstar:",
    chapter_prefix: "comicearthstar-ch:",
    page_prefix: "comicearthstar-page:",
    metadata: METADATA,
    lock_unavailable_titles: true,
};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.comicearthstar.ja.koma",
    name: "コミック アース・スター",
    language: "ja",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    description: "アース・スター エンターテイメントの公式マンガサイト。『ヤマノススメ』『戦国小町苦労譚』をはじめ、ライトノベルのコミカライズ、オリジナル連載、読切を掲載。原サイトで無料公開中の話を読めます。",
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
            "14079602755185553136",
            "14079602755185553138",
            "14079602755185553142",
        ],
    ),
    ("ongoing", &["14079602755185553136"]),
    ("completed", &["14079602755185553138"]),
    ("oneshot", &["14079602755185553142"]),
    ("adaptations", &["Genre:12207421983825514342"]),
    ("original", &["Genre:12207421983825514347"]),
    ("seinen", &["Genre:12207421983825514353"]),
    ("women", &["Genre:12207421983825514356"]),
    ("anime", &["Genre:12207421983825514357"]),
    ("fantasy", &["Genre:12207421983825514361"]),
    ("action", &["Genre:12207421983825514365"]),
    ("romance", &["Genre:12207421983825514369"]),
    ("horror_mystery", &["Genre:12207421983825514370"]),
    ("daily", &["Genre:12207421983825514371"]),
    ("history", &["Genre:12207421983825514374"]),
    ("sports", &["Genre:12207421983825514379"]),
    ("food", &["Genre:12207421983825514385"]),
    ("development", &["Genre:12207421983825514389"]),
    ("animals", &["Genre:12207421983825514390"]),
];
const LISTINGS: &[&str] = &["popular", "latest", "ongoing", "completed", "oneshot"];
fn listing(args: &Value) -> Result<Vec<Value>> {
    let (ids, sort) = match args["listingId"].as_str().unwrap_or("popular") {
        "popular" => (browse::selected(args, COLLECTIONS)?, Sort::Likes),
        "latest" => (browse::selected(args, COLLECTIONS)?, Sort::Published),
        "ongoing" => (COLLECTIONS[1].1, Sort::Site),
        "completed" => (COLLECTIONS[2].1, Sort::Site),
        "oneshot" => (COLLECTIONS[3].1, Sort::Site),
        _ => return Err("invalid_request"),
    };
    browse::collection(&VIEWER, ids, sort)
}
browse::gigaviewer_exports!("comicearthstar_ja");
