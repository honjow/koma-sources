#![no_std]
extern crate alloc;

use alloc::{format, vec::Vec};
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use serde_json::{json, Value};

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
    origin: "https://comic-days.com",
    page_host: "cdn-img.comic-days.com",
    cover_hosts: &["comic-days.com"],
    work_prefix: "days:",
    chapter_prefix: "days-ch:",
    page_prefix: "days-page:",
    metadata: METADATA,
    lock_unavailable_titles: true,
};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.comicdays.ja.koma",
    name: "コミックDAYS",
    language: "ja",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    description:
        "講談社のマンガサービス。無料連載、雑誌のバラ読み、ランキングから公開話を探せます。",
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

const SERIAL: &[&str] = &[
    "13932016480029138639",
    "13932016480029138640",
    "13932016480029138641",
    "13932016480029138642",
    "13932016480029138643",
    "13932016480029138644",
    "13932016480029138645",
    "13932016480029255192",
];
const LISTINGS: &[&str] = &["ranking", "latest", "serial", "free", "magazine", "oneshot"];
const COLLECTIONS: &[Collection] = &[
    ("serial", SERIAL),
    ("adaptations", &["Genre:3269754496379892693"]),
    ("romance", &["Genre:3269754496379892700"]),
    ("romcom", &["Genre:3269754496379892701"]),
    ("comedy", &["Genre:3269754496379892706"]),
    ("suspense", &["Genre:3269754496379892707"]),
    ("action", &["Genre:3269754496379892708"]),
    ("historical", &["Genre:3269754496379892709"]),
    ("slice_of_life", &["Genre:3269754496379892710"]),
    ("drama", &["Genre:3269754496379892711"]),
    ("isekai", &["Genre:3269754496379892712"]),
    ("family", &["Genre:3269754496379892713"]),
    ("parenting", &["Genre:3269754496379892714"]),
    ("infidelity", &["Genre:3269754496379892715"]),
    ("underworld", &["Genre:3269754496379892716"]),
    ("survival", &["Genre:3269754496379892717"]),
    ("fanservice", &["Genre:3269754496379892720"]),
    ("sports", &["Genre:3269754496379892723"]),
    ("sf", &["Genre:3269754496379892724"]),
    ("short", &["Genre:3269754496379892725"]),
    ("work", &["Genre:3269754496379892726"]),
    ("medical", &["Genre:3269754496379892727"]),
    ("society", &["Genre:3269754496379892728"]),
    ("arts", &["Genre:3269754496379892737"]),
    ("food", &["Genre:3269754496379892740"]),
    ("military", &["Genre:3269754496379892741"]),
    ("essays", &["Genre:3269754496379892747"]),
    ("animals", &["Genre:3269754496379892748"]),
    ("bl", &["Genre:3269754496379892749"]),
    ("yuri", &["Genre:3269754496379892750"]),
    ("spinoff", &["Genre:3269754496379892753"]),
];

fn query(fields: &str) -> Result<Value> {
    let series = giga::SERIES_FIELDS;
    VIEWER.graphql(json!({"query":format!("{{{fields}}} fragment Work on Series {{{series}}}")}))
}

fn ranking(name: &str) -> Result<Vec<Value>> {
    let data = query(&format!(
        r#"seriesRanking(name:"{name}"){{series(first:100){{edges{{node{{...Work}}}}}}}}"#
    ))?;
    let ranked = browse::edges(&data["seriesRanking"]["series"])?;
    browse::series_items(&VIEWER, ranked, Sort::Site)
}

fn free() -> Result<Vec<Value>> {
    // This is the public campaign collection used by the official homepage.
    let data = query("comicdaysFreeSeriesCampaignList{seriesSlice{seriesList{...Work}}}")?;
    let campaigns = data["comicdaysFreeSeriesCampaignList"]
        .as_array()
        .ok_or("parse_error")?;
    let mut nodes = Vec::new();
    for campaign in campaigns {
        nodes.extend(
            campaign["seriesSlice"]["seriesList"]
                .as_array()
                .ok_or("parse_error")?,
        );
    }
    browse::series_items(&VIEWER, nodes, Sort::Site)
}

fn magazine() -> Result<Vec<Value>> {
    // /magazine offers the individual series in the publishers' current issues.
    // A magazine issue or subscription is not a GigaViewer series.
    let data =
        query("comicdaysMagazineGroup(sort:OPEN_AT_DESC){latestIssue{episodes{series{...Work}}}}")?;
    let groups = data["comicdaysMagazineGroup"]
        .as_array()
        .ok_or("parse_error")?;
    let mut nodes = Vec::new();
    for group in groups {
        if let Some(episodes) = group["latestIssue"]["episodes"].as_array() {
            nodes.extend(
                episodes
                    .iter()
                    .map(|episode| &episode["series"])
                    .filter(|node| !node.is_null()),
            );
        }
    }
    browse::series_items(&VIEWER, nodes, Sort::Site)
}

fn listing(args: &Value) -> Result<Vec<Value>> {
    match args["listingId"].as_str().unwrap_or("ranking") {
        "popular" | "ranking" => ranking("ALL"),
        "oneshot" => ranking("ONESHOT_AND_NEWCOMER"),
        "free" => free(),
        "magazine" => magazine(),
        "serial" => browse::collection(&VIEWER, browse::selected(args, COLLECTIONS)?, Sort::Site),
        "latest" => browse::collection(
            &VIEWER,
            browse::selected(args, COLLECTIONS)?,
            Sort::Published,
        ),
        _ => Err("invalid_request"),
    }
}

browse::gigaviewer_exports!("comicdays_ja");
