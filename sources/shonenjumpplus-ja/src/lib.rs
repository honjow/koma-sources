#![no_std]
extern crate alloc;

use alloc::{collections::BTreeSet, format, string::String, vec::Vec};
use koma_source_sdk::{
    chapters::ChapterResult,
    host,
    source::{SourceCapabilities, SourceInfo},
};
use serde_json::{json, Value};

#[path = "../../gigaviewer.rs"]
mod giga;
#[path = "../../port_support.rs"]
#[allow(dead_code)]
mod port;
#[path = "../../port_catalog.rs"]
mod port_catalog;
use port::Result;
use port_catalog::Labels;

#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

const METADATA: &[u8] = include_bytes!("../metadata.json");
const SITE: &str = "https://shonenjumpplus.com";
const HTML_BATCH: usize = 128;
const WORK: &str = "jump:";
const VIEWER: giga::Site = giga::Site {
    origin: SITE,
    page_host: "cdn-ak-img.shonenjumpplus.com",
    cover_hosts: &["cdn-ak.shonenjumpplus.com"],
    work_prefix: WORK,
    chapter_prefix: "jump-ch:",
    page_prefix: "jump-page:",
    metadata: METADATA,
    lock_unavailable_titles: false,
};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.shonenjumpplus.ja.koma",
    name: "少年ジャンプ＋",
    language: "ja",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    description:
        "集英社の公式マンガサービス。『SPY×FAMILY』などの連載や読切を掲載し、公開中の話を読めます。",
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

const COLLECTIONS: &[(&str, &str, &str)] = &[
    ("serial", "連載一覧", ""),
    ("latest", "今日の更新", ""),
    ("oneshot", "読切シリーズ", "/oneshot"),
    ("finished", "連載終了作品", "/finished"),
];

fn tokyo_weekday() -> Result<&'static str> {
    let message = br#"{"type":"private_session","version":1,"op":"clock"}"#;
    let out = http_out();
    let size = host::http_request(message, out).map_err(|_| "clock_unavailable")?;
    let clock: Value = serde_json::from_slice(&out[..size]).map_err(|_| "clock_unavailable")?;
    let milliseconds = clock["nowMs"]
        .as_str()
        .and_then(|v| v.parse::<u64>().ok())
        .ok_or("clock_unavailable")?;
    const DAYS: [&str; 7] = [
        "thursday",
        "friday",
        "saturday",
        "sunday",
        "monday",
        "tuesday",
        "wednesday",
    ];
    Ok(DAYS[((milliseconds / 3_600_000 + 9) / 24 % 7) as usize])
}

fn paged_collection(collection: &str) -> bool {
    matches!(collection, "oneshot" | "finished")
}

fn listings(collection: &str, cursor: &Value) -> Result<Value> {
    let (_, _, path) = COLLECTIONS
        .iter()
        .find(|(key, _, _)| *key == collection)
        .ok_or("invalid_request")?;
    let offset = if cursor.is_null() {
        0
    } else if paged_collection(collection) {
        cursor
            .as_str()
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n > 0 && *n <= 25_600 && *n % HTML_BATCH == 0)
            .ok_or("invalid_request")?
    } else {
        return Err("invalid_request");
    };
    let doc = VIEWER.document(&format!("{SITE}/series{path}"))?;
    let selector = if collection == "latest" {
        let day = tokyo_weekday()?;
        format!("h2.series-list-date-week.{day} + ul.series-list")
    } else {
        String::from("ul.series-list")
    };
    let groups = doc.all(&selector)?;
    if groups.is_empty() || (paged_collection(collection) && groups.len() != 1) {
        return Err("parse_error");
    }
    let first = offset + 1;
    let end = offset + HTML_BATCH;
    let row_selector = if paged_collection(collection) {
        format!("li:nth-child(n+{first}):nth-child(-n+{end}) > a[href]")
    } else {
        String::from("li a[href]")
    };
    let mut items = Vec::new();
    let mut seen = BTreeSet::new();
    let next = paged_collection(collection)
        && groups[0]
            .one(&format!("li:nth-child({})", end + 1))
            .is_some();
    for group in groups {
        for row in group.all(&row_selector)? {
            if host::check_cancel() {
                return Err("cancelled");
            }
            let Some(title) = row.one("h2.series-list-title") else {
                continue;
            };
            let link = port::absolute(SITE, &row.attr("href")?)?;
            // The original collections contain editorial and external links
            // alongside products. Only the provider viewer can resolve a series.
            let Some((kind, id)) = giga::product_path(SITE, &link) else {
                continue;
            };
            let cover = row
                .one("div.series-list-thumb img")
                .map(|n| n.attr("data-src"))
                .transpose()?;
            let series = cover
                .as_deref()
                .and_then(|cover| VIEWER.series_id_from_thumbnail(cover))
                .unwrap_or_else(|| format!("{}:{id}", kind.name()));
            if !seen.insert(series.clone()) {
                continue;
            }
            items.push(json!({"id":format!("{WORK}{series}"),"title":title.text()?,
                "cover":VIEWER.thumbnail(cover.as_deref())?,"language":"ja","url":link.as_str(),
                "links":[{"kind":"source","url":link.as_str()}]}));
        }
    }
    Ok(port::items(items, next.then_some(end)))
}

fn run_search(request: &[u8]) -> u32 {
    port::write_result(
        "search",
        port::input(request).and_then(|input| {
            let args = &input["args"];
            let query = args["query"].as_str().unwrap_or("").trim();
            if query.is_empty() {
                return listings(
                    args["filters"]["collection"].as_str().unwrap_or("serial"),
                    &args["page"]["cursor"],
                );
            }
            VIEWER.search(query)
        }),
    )
}
fn run_get_manga_list(request: &[u8]) -> u32 {
    port::write_result(
        "get_manga_list",
        port::input(request).and_then(|input| {
            listings(
                input["args"]["listingId"].as_str().unwrap_or("serial"),
                &input["args"]["page"]["cursor"],
            )
        }),
    )
}
fn run_get_manga(request: &[u8]) -> u32 {
    port::write_result(
        "get_manga",
        port::input(request).and_then(|input| VIEWER.detail(&input)),
    )
}
fn run_get_pages(request: &[u8]) -> u32 {
    port::write_result(
        "get_pages",
        port::input(request).and_then(|input| VIEWER.pages(&input)),
    )
}
fn run_get_image_request(request: &[u8]) -> u32 {
    port::write_result(
        "get_image_request",
        port::input(request).and_then(|input| VIEWER.image_request(request, &input)),
    )
}
fn run_get_listings(request: &[u8]) -> u32 {
    let result = Labels::new(METADATA, request).and_then(|labels| {
        let listings = COLLECTIONS
            .iter()
            .map(|(id, name, _)| {
                Ok(json!({"id":id,
            "name":labels.text(&format!("listing.{id}"),name)?,"supportsPaging":paged_collection(id)}))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(json!({"listings":listings}))
    });
    port::write_result("get_listings", result)
}
fn run_get_filters(request: &[u8]) -> u32 {
    let result = Labels::new(METADATA,request).and_then(|labels| {
        let options = COLLECTIONS.iter().filter(|(id,_,_)| *id != "latest")
            .map(|(id,name,_)| Ok(json!({"value":id,"label":labels.text(&format!("listing.{id}"),name)?})))
            .collect::<Result<Vec<_>>>()?;
        Ok(json!({"useSearchForFilters":true,"filters":[{"id":"collection","type":"select","scope":"search","default":"serial",
            "name":labels.text("filter.collection","作品一覧")?,"options":options}]}))
    });
    port::write_result("get_filters", result)
}
fn run_get_settings(request: &[u8]) -> u32 {
    let result = Labels::new(METADATA, request).and_then(|labels| {
        let settings = [
            ("hide_locked", "有料話を非表示"),
            ("hide_unavailable", "配信終了話を非表示"),
        ]
        .into_iter()
        .map(|(id, name)| {
            Ok(json!({"id":id,"kind":"boolean","default":false,
                "name":labels.text(&format!("setting.{id}"),name)?}))
        })
        .collect::<Result<Vec<_>>>()?;
        Ok(json!({"settings":settings}))
    });
    port::write_result("get_settings", result)
}
fn run_get_home(_: &[u8]) -> u32 {
    port::write_failure("get_home", "not_implemented")
}
fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    VIEWER.chapters(request)
}
fn write_chapter_failure(code: &str) -> u32 {
    port::write_failure("get_chapters", code)
}
koma_source_sdk::koma_source_exports!("shonenjumpplus_ja", typed_chapters, write_chapter_failure);
