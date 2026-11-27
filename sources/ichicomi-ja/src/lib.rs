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
    origin: "https://ichicomi.com",
    page_host: "cdn-img.ichicomi.com",
    cover_hosts: &["cdn-img.ichicomi.com"],
    work_prefix: "ichicomi:",
    chapter_prefix: "ichicomi-ch:",
    page_prefix: "ichicomi-page:",
    metadata: METADATA,
    lock_unavailable_titles: true,
};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.ichicomi.ja.koma",
    name: "一迅プラス",
    language: "ja",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    description: "一迅社の公式Webマンガサイト。Comic REX、comic HOWL、comic POOL、コミック百合姫などの作品を編集部・ジャンル別に掲載しています。原サイトで無料公開中の話を読めます。",
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
    ("all", &["2550912965889047616"]),
    (
        "echo",
        &[
            "2550912965796689066",
            "2550912965796689067",
            "2550912965796689072",
        ],
    ),
    (
        "gateau",
        &[
            "2550912965796689074",
            "2550912965796689075",
            "2550912965796689080",
        ],
    ),
    (
        "colorful_happiness",
        &[
            "2550912965796689089",
            "2550912965796689092",
            "2550912965796689093",
        ],
    ),
    (
        "rex",
        &[
            "2550912965796689101",
            "2550912965796689105",
            "2550912965796689108",
        ],
    ),
    (
        "howl",
        &[
            "2550912965796689112",
            "2550912965796689114",
            "2550912965796689120",
        ],
    ),
    (
        "pool",
        &[
            "2550912965796689123",
            "2550912965796689131",
            "2550912965796689144",
        ],
    ),
    (
        "yurihime",
        &[
            "2550912965796689168",
            "2550912965796689175",
            "2550912965796689183",
        ],
    ),
    (
        "lake",
        &[
            "2550912965796689148",
            "2550912965796689158",
            "2550912965796689163",
        ],
    ),
    (
        "zerosum",
        &[
            "2550912965796689187",
            "2550912965796689195",
            "2550912965796689196",
        ],
    ),
    (
        "palette",
        &[
            "2550912965796689199",
            "2550912965796689206",
            "2550912965796689212",
        ],
    ),
    (
        "babydoll",
        &[
            "2550912965796689213",
            "2550912965796689214",
            "2550912965796689218",
        ],
    ),
    (
        "ichijin_plus",
        &[
            "2550912965796689248",
            "2550912965796689255",
            "2550912965796689266",
        ],
    ),
    ("lovebites", &["2550912965796689223", "2550912965796689234"]),
    ("romance", &["Genre:2550912965827315660"]),
    ("romcom", &["Genre:2550912965827315672"]),
    ("comedy", &["Genre:2550912965827315678"]),
    ("horror", &["Genre:2550912965827315681"]),
    ("suspense", &["Genre:2550912965827315687"]),
    ("action", &["Genre:2550912965827315689"]),
    ("history", &["Genre:2550912965827315693"]),
    ("daily", &["Genre:2550912965827315695"]),
    ("drama", &["Genre:2550912965827315700"]),
    ("isekai", &["Genre:2550912965827315707"]),
    ("fantasy", &["Genre:2550912965827315710"]),
    ("yuri", &["Genre:2550912965827315711"]),
    ("bl", &["Genre:2550912965827315722"]),
    ("tl", &["Genre:2550912965827315725"]),
    ("school", &["Genre:2550912965827315726"]),
    ("work", &["Genre:2550912965827315727"]),
    ("adapted", &["Genre:2550912965827315728"]),
    ("newcomers", &["Genre:2550912965827315729"]),
    ("completed", &["Genre:2550912965827315734"]),
    ("original", &["Genre:2550912965827315743"]),
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
browse::gigaviewer_exports!("ichicomi_ja");
