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
    origin: "https://comic-action.com",
    page_host: "cdn-img.comic-action.com",
    cover_hosts: &["cdn-img.comic-action.com"],
    work_prefix: "comicaction:",
    chapter_prefix: "comicaction-ch:",
    page_prefix: "comicaction-page:",
    metadata: METADATA,
    lock_unavailable_titles: true,
};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.comicaction.ja.koma",
    name: "webアクション",
    language: "ja",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    description: "双葉社の公式マンガサイト。webアクションと「漫画アクション」の連載、読切、期間限定の無料公開作品を掲載しています。",
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
            "14079602755641808866",
            "14079602755641808844",
            "14079602755641808862",
            "14079602755641808870",
            "14079602755641808869",
            "14079602755641808856",
            "14079602755641808865",
            "14079602755641808871",
            "10834108156759733726",
        ],
    ),
    ("magazine", &["10834108156759733716"]),
    ("oneshot", &["10834108156759733722"]),
    ("campaign", &["10834108156759733723"]),
];
const LISTINGS: &[&str] = &["popular", "latest", "oneshot", "magazine", "campaign"];
fn listing(args: &Value) -> Result<Vec<Value>> {
    let (ids, sort) = match args["listingId"].as_str().unwrap_or("popular") {
        "popular" => (browse::selected(args, COLLECTIONS)?, Sort::Likes),
        "latest" => (browse::selected(args, COLLECTIONS)?, Sort::Published),
        "oneshot" => (COLLECTIONS[2].1, Sort::Site),
        "magazine" => (COLLECTIONS[1].1, Sort::Site),
        "campaign" => (COLLECTIONS[3].1, Sort::Site),
        _ => return Err("invalid_request"),
    };
    browse::collection(&VIEWER, ids, sort)
}
browse::gigaviewer_exports!("comicaction_ja");
