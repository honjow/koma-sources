#![no_std]
extern crate alloc;

use alloc::{collections::BTreeSet, format, vec::Vec};
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
use serde_json::{json, Value};

#[path = "../../gigaviewer.rs"]
mod giga;
#[path = "../../gigaviewer_browse.rs"]
#[allow(dead_code)]
mod giga_browse;
#[path = "../../port_support.rs"]
#[allow(dead_code)]
mod port;
#[path = "../../port_catalog.rs"]
mod port_catalog;
use giga_browse::{Collection, Sort};
use port::Result;

#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

const METADATA: &[u8] = include_bytes!("../metadata.json");
const SITE: &str = "https://www.sunday-webry.com";
const VIEWER: giga::Site = giga::Site {
    origin: SITE,
    page_host: "cdn-img.www.sunday-webry.com",
    cover_hosts: &["cdn.www.sunday-webry.com"],
    work_prefix: "webry:",
    chapter_prefix: "webry-ch:",
    page_prefix: "webry-page:",
    metadata: METADATA,
    lock_unavailable_titles: true,
};
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.sundaywebevery.ja.koma",
    name: "サンデーうぇぶり",
    language: "ja",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    description: "小学館の公式マンガサイト。『名探偵コナン』などの連載や読切、夜サンデーの公開話を掲載しています。",
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
    "3269754496454696025",
    "3269754496454696029",
    "3269754496454696030",
    "3269754496454696040",
    "3269754496454696045",
    "3269754496454696049",
    "3269754496454696050",
    "3269754496576310217",
    "3269754496454696059",
];
const ONESHOT: &[&str] = &["3269754496400962241"];
const LISTINGS: &[&str] = &["popular", "latest", "oneshot", "evening"];
const COLLECTIONS: &[Collection] = &[("serial", SERIAL), ("oneshot", ONESHOT), ("evening", &[])];

fn evening() -> Result<Vec<Value>> {
    // The official Night Sunday collection is an HTML-only directory.
    let doc = VIEWER.document(&format!("{SITE}/series/yoru-sunday"))?;
    let rows = doc.all("ul.webry-series-list li a.webry-series-item-link")?;
    if rows.is_empty() {
        return Err("parse_error");
    }
    let mut seen = BTreeSet::new();
    let mut items = Vec::new();
    for row in rows {
        let link = port::absolute(SITE, &row.attr("href")?)?;
        let (kind, product) = giga::product_path(SITE, &link).ok_or("parse_error")?;
        let cover = row
            .one("div.thumb-wrapper img")
            .and_then(|img| img.attr("data-src").ok())
            .filter(|raw| !raw.is_empty());
        let series = cover
            .as_deref()
            .and_then(|raw| VIEWER.series_id_from_thumbnail(raw))
            .unwrap_or_else(|| format!("{}:{product}", kind.name()));
        if seen.insert(series.clone()) {
            items.push(json!({
                "id":format!("{}{series}",VIEWER.work_prefix),
                "title":row.require("h4.series-title")?.text()?,
                "cover":VIEWER.thumbnail(cover.as_deref())?,
                "language":"ja","url":link.as_str(),
                "links":[{"kind":"source","url":link.as_str()}]
            }));
        }
    }
    Ok(items)
}

fn collection(ids: &[&str], sort: Sort) -> Result<Vec<Value>> {
    if ids.is_empty() {
        evening()
    } else {
        giga_browse::collection(&VIEWER, ids, sort)
    }
}

fn listing(args: &Value) -> Result<Vec<Value>> {
    match args["listingId"].as_str().unwrap_or("popular") {
        "popular" => collection(giga_browse::selected(args, COLLECTIONS)?, Sort::Likes),
        "latest" => collection(giga_browse::selected(args, COLLECTIONS)?, Sort::Published),
        "oneshot" => collection(ONESHOT, Sort::Site),
        "evening" => evening(),
        _ => Err("invalid_request"),
    }
}

mod browse {
    pub(crate) use crate::giga_browse::{filters, gigaviewer_exports, listings, page, settings};

    pub(crate) fn search(
        site: &super::giga::Site,
        args: &super::Value,
        collections: &[super::Collection],
    ) -> super::Result<super::Value> {
        if args["query"].as_str().unwrap_or("").trim().is_empty()
            && crate::giga_browse::selected(args, collections)?.is_empty()
        {
            return page(super::evening()?, &args["page"]["cursor"]);
        }
        crate::giga_browse::search(site, args, collections)
    }
}

browse::gigaviewer_exports!("sundaywebevery_ja");
