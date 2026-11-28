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
use port::Result;

#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

const METADATA: &[u8] = include_bytes!("../metadata.json");
const VIEWER: giga::Site = giga::Site {
    origin: "https://comicborder.com",
    page_host: "cdn-img.comicborder.com",
    cover_hosts: &["cdn-img.comicborder.com"],
    work_prefix: "comicborder:",
    chapter_prefix: "comicborder-ch:",
    page_prefix: "comicborder-page:",
    metadata: METADATA,
    lock_unavailable_titles: true,
};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.comicborder.ja.koma",
    name: "コミックボーダー",
    language: "ja",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    description: "リイド社の公式マンガサイト。『皮肉屋魔神と死にたがりの少女 シン・悪魔の辞典』『カムヤライド』などを掲載し、毎週金曜更新。作品は公式トップページの掲載順で並び、無料公開中の話を読めます。",
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
    filters: false,
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

use alloc::{collections::BTreeSet, format, string::ToString};
use serde_json::json;
fn popular() -> Result<Vec<Value>> {
    let document = VIEWER.document(VIEWER.origin)?;
    let mut items = Vec::new();
    let mut seen = BTreeSet::new();
    for card in document.all("section.top-series")? {
        let title = card.require("h3")?.text()?;
        let thumbnail = card
            .one(".top-key-image")
            .map(|node| node.attr("data-src"))
            .transpose()?;
        let episode = card
            .require("li.first-episode a, li.this-episode a")?
            .attr("href")?;
        let url = port::absolute(VIEWER.origin, &episode)?;
        let (kind, _) = giga::product_path(VIEWER.origin, &url).ok_or("parse_error")?;
        if kind != giga::ProductKind::Episode {
            return Err("parse_error");
        }
        let thumbnail = thumbnail
            .map(|raw| port::absolute(VIEWER.origin, &raw).map(|url| url.to_string()))
            .transpose()?;
        let series = match thumbnail
            .as_deref()
            .and_then(|raw| VIEWER.series_id_from_thumbnail(raw))
        {
            Some(series) => series,
            None => {
                let episode = VIEWER.document(url.as_str())?;
                let series = episode
                    .require("script.js-valve[data-giga_series]")?
                    .attr("data-giga_series")?;
                if !giga::id(&series) {
                    return Err("parse_error");
                }
                series
            }
        };
        if seen.insert(series.clone()) {
            items.push(
                json!({"id":format!("{}{}",VIEWER.work_prefix,series),"title":title,
                "cover":VIEWER.thumbnail(thumbnail.as_deref())?,"language":"ja","url":url.as_str(),
                "links":[{"kind":"source","url":url.as_str()}]}),
            );
        }
    }
    if items.is_empty() {
        return Err("parse_error");
    }
    Ok(items)
}
fn listing(args: &Value) -> Result<Vec<Value>> {
    if args["listingId"].as_str().unwrap_or("popular") != "popular" {
        return Err("invalid_request");
    }
    listing_search(args)
}
fn listing_search(args: &Value) -> Result<Vec<Value>> {
    if args["filters"]
        .as_object()
        .is_some_and(|filters| !filters.is_empty())
    {
        return Err("invalid_request");
    }
    popular()
}
fn source_listings(labels: &Labels<'_>) -> Result<Value> {
    Ok(
        json!({"listings":[{"id":"popular","name":labels.text("listing.popular","作品一覧")?,"supportsPaging":true,"supportsFilters":false}]}),
    )
}
fn source_filters(_: &Labels<'_>) -> Result<Value> {
    Ok(json!({"filters":[]}))
}

use koma_source_sdk::chapters::ChapterResult;
use port_catalog::Labels;
fn dispatch(operation: &str, request: &[u8], f: impl FnOnce(&Value) -> Result<Value>) -> u32 {
    port_catalog::write_localized(
        METADATA,
        request,
        operation,
        port::input(request).and_then(|input| f(&input)),
    )
}
fn run_search(req: &[u8]) -> u32 {
    dispatch("search", req, |input| {
        let args = &input["args"];
        let query = args["query"].as_str().unwrap_or("").trim();
        if query.is_empty() {
            return browse::page(listing_search(args)?, &args["page"]["cursor"]);
        }
        if !args["page"]["cursor"].is_null() {
            return Err("invalid_request");
        }
        VIEWER.search(query)
    })
}
fn run_get_manga_list(req: &[u8]) -> u32 {
    dispatch("get_manga_list", req, |input| {
        browse::page(listing(&input["args"])?, &input["args"]["page"]["cursor"])
    })
}
fn run_get_manga(req: &[u8]) -> u32 {
    dispatch("get_manga", req, |input| VIEWER.detail(input))
}
fn run_get_pages(req: &[u8]) -> u32 {
    dispatch("get_pages", req, |input| VIEWER.pages(input))
}
fn run_get_image_request(req: &[u8]) -> u32 {
    dispatch("get_image_request", req, |input| {
        VIEWER.image_request(req, input)
    })
}
fn run_get_chapters(req: &[u8]) -> ChapterResult<'static, (&'static str, alloc::string::String)> {
    VIEWER
        .chapters(req)
        .map_err(|code| (code, port_catalog::failure(METADATA, req, code)))
}
fn write_chapter_failure((code, message): (&str, alloc::string::String)) -> u32 {
    write_error("get_chapters", code, &message)
}
fn run_get_listings(req: &[u8]) -> u32 {
    dispatch("get_listings", req, |_| {
        source_listings(&Labels::new(METADATA, req)?)
    })
}
fn run_get_filters(req: &[u8]) -> u32 {
    dispatch("get_filters", req, |_| {
        source_filters(&Labels::new(METADATA, req)?)
    })
}
fn run_get_settings(req: &[u8]) -> u32 {
    dispatch("get_settings", req, |_| {
        browse::settings(&Labels::new(METADATA, req)?)
    })
}
fn run_get_home(req: &[u8]) -> u32 {
    dispatch("get_home", req, |_| Err("not_implemented"))
}
koma_source_sdk::koma_source_exports!("comicborder_ja", typed_chapters, write_chapter_failure);
