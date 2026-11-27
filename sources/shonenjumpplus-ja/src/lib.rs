#![no_std]
extern crate alloc;

use alloc::{
    collections::BTreeSet,
    format,
    string::{String, ToString},
    vec::Vec,
};
use koma_source_sdk::{
    chapters::{Chapter, ChapterPage, ChapterPlacement, ChapterResult, GroupedChapters},
    host,
    source::{SourceCapabilities, SourceInfo},
    source_protocol,
};
use serde_json::{json, Value};
use url::Url;

#[path = "../../gigaviewer.rs"]
mod giga;
#[path = "../../port_support.rs"]
#[allow(dead_code)]
mod port;
#[path = "../../port_catalog.rs"]
mod port_catalog;
use giga::{Access, ProductKind, Tiles};
use port::{Node, Result};
use port_catalog::Labels;

#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

const METADATA: &[u8] = include_bytes!("../metadata.json");
const SITE: &str = "https://shonenjumpplus.com";
const DESKTOP_UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/142.0.0.0 Safari/537.36";
const HTML_BATCH: usize = 128;
const WORK: &str = "jump:";
const CHAPTER: &str = "jump-ch:";
const PAGE: &str = "jump-page:";
const DECODE: &[u8] = b"source_required_image_decode_wasm_v1";
const EXCLUSIONS: &[u8] = b"source_chapter_exclusions_v1";
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

fn content(url: &str, request: Option<Value>) -> Result<Value> {
    let body = if let Some(query) = request {
        port::request(
            url,
            "POST",
            json!({
                "Accept":"application/json","Content-Type":"application/json",
                "Origin":SITE,"Referer":format!("{SITE}/"),"X-Requested-With":"XMLHttpRequest"
            }),
            Some(&query.to_string()),
        )?
    } else {
        port::request(
            url,
            "GET",
            json!({"Accept":"application/json","Referer":format!("{SITE}/"),"X-Requested-With":"XMLHttpRequest"}),
            None,
        )?
    };
    serde_json::from_str(&body).map_err(|_| "parse_error")
}

fn graphql(query: Value) -> Result<Value> {
    let mut response = content(&format!("{SITE}/graphql"), Some(query))?;
    giga::graphql_data(&response)?;
    Ok(response["data"].take())
}

fn document(url: &str) -> Result<Node> {
    Node::parse(&port::request(
        url,
        "GET",
        json!({"Accept":"text/html"}),
        None,
    )?)
}

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

fn image_url(raw: &str) -> Result<Url> {
    let url = port::absolute(SITE, raw)?;
    if url.scheme() != "https"
        || url.port().is_some()
        || url.fragment().is_some()
        || !matches!(
            url.host_str(),
            Some(
                "cdn-ak-img.shonenjumpplus.com"
                    | "cdn-ak.shonenjumpplus.com"
                    | "cdn-scissors.gigaviewer.com"
            )
        )
    {
        return Err("parse_error");
    }
    Ok(url)
}

fn page_image(raw: &str) -> Result<Url> {
    let url = image_url(raw)?;
    if url.host_str() != Some("cdn-ak-img.shonenjumpplus.com")
        || !url.path().starts_with("/public/page/")
    {
        return Err("parse_error");
    }
    Ok(url)
}

fn thumbnail(raw: Option<&str>) -> Result<Value> {
    let Some(raw) = raw.filter(|s| !s.is_empty()) else {
        return Ok(json!({"kind":"none"}));
    };
    let image = image_url(raw)?;
    Ok(json!({"kind":"url","url":image.as_str(),"headers":{"Referer":format!("{SITE}/")}}))
}

fn series_id_from_thumbnail(raw: &str) -> Option<String> {
    let url = image_url(raw).ok()?;
    let original = if url.host_str() == Some("cdn-scissors.gigaviewer.com") {
        let encoded = url.path_segments()?.next_back()?;
        let pair = format!("url={encoded}");
        let (_, decoded) = url::form_urlencoded::parse(pair.as_bytes()).next()?;
        image_url(&decoded).ok()?
    } else {
        url
    };
    if original.host_str() != Some("cdn-ak-img.shonenjumpplus.com") {
        return None;
    }
    let mut segments = original.path_segments()?.rev();
    let file = segments.next()?;
    if !segments.next()?.starts_with("series-") {
        return None;
    }
    let (id, _) = file.split_once('-')?;
    giga::id(id).then(|| String::from(id))
}

fn series_item(series: &giga::Series<'_>) -> Result<Value> {
    let url = port::absolute(SITE, series.permalink)?;
    giga::product_path(SITE, &url).ok_or("parse_error")?;
    Ok(
        json!({"id":format!("{WORK}{}",series.id),"title":series.title,"cover":thumbnail(series.thumbnail)?,
        "language":"ja","url":url.as_str(),"links":[{"kind":"source","url":url.as_str()}]}),
    )
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
    let doc = document(&format!("{SITE}/series{path}"))?;
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
                .and_then(series_id_from_thumbnail)
                .unwrap_or_else(|| format!("{}:{id}", kind.name()));
            if !seen.insert(series.clone()) {
                continue;
            }
            items.push(json!({"id":format!("{WORK}{series}"),"title":title.text()?,
                "cover":thumbnail(cover.as_deref())?,"language":"ja","url":link.as_str(),
                "links":[{"kind":"source","url":link.as_str()}]}));
        }
    }
    Ok(port::items(items, next.then_some(end)))
}

fn search(input: &Value) -> Result<Value> {
    let args = &input["args"];
    let query = args["query"].as_str().unwrap_or("").trim();
    if query.is_empty() {
        return listings(
            args["filters"]["collection"].as_str().unwrap_or("serial"),
            &args["page"]["cursor"],
        );
    }
    let data = graphql(giga::search_query(query))?;
    let edges = data["searchSeries"]["edges"]
        .as_array()
        .ok_or("parse_error")?;
    let mut series = Vec::new();
    let mut seen = BTreeSet::new();
    for edge in edges {
        if let Some(item) = giga::Series::read(&edge["node"])? {
            if seen.insert(item.id) {
                series.push(item);
            }
        }
    }
    series.sort_by_key(|item| item.volume_only);
    let items = series.iter().map(series_item).collect::<Result<Vec<_>>>()?;
    Ok(port::items(items, None))
}

fn work(input: &Value) -> Result<(&str, String)> {
    let work = input["args"]["mangaId"].as_str().ok_or("invalid_request")?;
    let suffix = work.strip_prefix(WORK).ok_or("invalid_request")?;
    if giga::id(suffix) {
        return Ok((work, String::from(suffix)));
    }
    let (kind, id) = suffix.split_once(':').ok_or("invalid_request")?;
    let kind = ProductKind::parse(kind).ok_or("invalid_request")?;
    if !giga::id(id) {
        return Err("invalid_request");
    }
    let doc = document(&kind.url(SITE, id))?;
    let series = doc
        .require("script.js-valve[data-giga_series]")?
        .attr("data-giga_series")?;
    if !giga::id(&series) {
        return Err("parse_error");
    }
    Ok((work, series))
}

fn series_details(id: &str) -> Result<Value> {
    let mut data = graphql(giga::series_query(id))?;
    if data["series"].is_null() {
        return Err("not_found");
    }
    if data["series"]["databaseId"].as_str() != Some(id) {
        return Err("parse_error");
    }
    Ok(data["series"].take())
}

fn detail(input: &Value) -> Result<Value> {
    let (work, id) = work(input)?;
    let node = series_details(&id)?;
    let series = giga::Series::read(&node)?.ok_or("not_found")?;
    let mut item = series_item(&series)?;
    item["id"] = json!(work);
    item["description"] = json!(node["description"].as_str().unwrap_or(""));
    item["authors"] = json!(node["author"]["name"]
        .as_str()
        .into_iter()
        .collect::<Vec<_>>());
    // This is the format tag consumed by Koma's source reader preferences.
    item["tags"] = json!(["Manga"]);
    Ok(json!({"manga":item}))
}

fn chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(request)?;
    let (work, series) = work(&input)?;
    let node = series_details(&series)?;
    let episode_count = ProductKind::Episode.count(&node)?;
    let volume_count = ProductKind::Volume.count(&node)?;
    let args = &input["args"];
    let exact = args["chapterId"].as_str();
    let (kind, offset, anchor) = match &args["page"]["cursor"] {
        Value::Null if episode_count == 0 => (ProductKind::Volume, 0, None),
        Value::Null => (ProductKind::Episode, 0, None),
        value => {
            let cursor = value.as_str().ok_or("invalid_request")?;
            let mut fields = cursor.split(':');
            let kind = fields
                .next()
                .and_then(ProductKind::parse)
                .ok_or("invalid_request")?;
            let offset = fields
                .next()
                .and_then(|s| s.parse::<usize>().ok())
                .ok_or("invalid_request")?;
            let anchor = fields.next().filter(|s| giga::id(s));
            if (offset == 0) != anchor.is_none() || fields.next().is_some() {
                return Err("invalid_request");
            }
            (kind, offset, anchor)
        }
    };
    let mut output = GroupedChapters::new(payload_buf(), request).map_err(port::chapter_error)?;
    let count = match kind {
        ProductKind::Episode => episode_count,
        ProductKind::Volume => volume_count,
    };
    if count == 0 {
        return output
            .finish(ChapterPage::Complete)
            .map_err(port::chapter_error);
    }
    if offset >= count {
        return Err("catalog_changed");
    }

    // Check the preceding row on continuation. A newly inserted or reordered
    // episode must fail the in-flight catalog instead of silently skipping it.
    let fetch_offset = offset.saturating_sub(1);
    let response = content(&kind.catalog_url(SITE, &series, fetch_offset), None)?;
    let rows = response.as_array().ok_or("parse_error")?;
    let rows = if let Some(anchor) = anchor {
        let (previous, rows) = rows.split_first().ok_or("catalog_changed")?;
        if giga::Product::read(previous, SITE, kind)?.id != anchor {
            return Err("catalog_changed");
        }
        rows
    } else {
        rows
    };
    if rows.is_empty() || rows.len() > 100 || offset + rows.len() > count {
        return Err("catalog_changed");
    }
    let labels = Labels::new(METADATA, request)?;
    let can_exclude = source_protocol::feature(request, EXCLUSIONS);
    let mut excluded = Vec::new();
    let mut seen = BTreeSet::new();
    if let Some(anchor) = anchor {
        seen.insert(anchor);
    }
    let mut last = "";
    for (position, row) in rows.iter().enumerate() {
        if host::check_cancel() {
            return Err("cancelled");
        }
        let product = giga::Product::read(row, SITE, kind)?;
        if !seen.insert(product.id) {
            return Err("catalog_changed");
        }
        last = product.id;
        let id = format!("{CHAPTER}{series}:{}:{}", kind.name(), product.id);
        if exact.is_some_and(|exact| exact != id) {
            continue;
        }
        let hide = match product.access {
            Access::Locked => input["settings"]["hide_locked"].as_bool() == Some(true),
            Access::Unavailable => input["settings"]["hide_unavailable"].as_bool() == Some(true),
            Access::Public => false,
        };
        if can_exclude && exact.is_none() && hide {
            excluded.push(id);
            continue;
        }
        let title = match product.access {
            Access::Unavailable => format!(
                "{} · {}",
                labels.text("chapter.unavailable", "配信終了")?,
                product.title
            ),
            Access::Locked | Access::Public => String::from(product.title),
        };
        let title = if kind == ProductKind::Volume {
            format!("{} · {title}", labels.text("chapter.volume", "単行本")?)
        } else {
            title
        };
        // Koma 1.6.3 displays the lock glyph from the title.
        let title = if product.access == Access::Locked {
            format!("🔒 {title}")
        } else {
            title
        };
        let url = kind.url(SITE, product.id);
        let mut chapter = Chapter::new(id.as_bytes(), work.as_bytes(), title.as_bytes());
        chapter.url = Some(url.as_bytes());
        chapter.language = Some(b"ja");
        chapter.published_at = product.published.map(str::as_bytes);
        chapter.locked = Some(product.access != Access::Public);
        let placement = if product.access == Access::Public {
            ChapterPlacement::InGroup {
                group: kind.name().as_bytes(),
                position: (count - offset - position - 1) as u64,
            }
        } else {
            ChapterPlacement::ManualOnly
        };
        output
            .push(chapter, placement)
            .map_err(port::chapter_error)?;
        if exact.is_some() {
            return output
                .finish(ChapterPage::Complete)
                .map_err(port::chapter_error);
        }
    }
    let next = if offset + rows.len() < count {
        Some(format!("{}:{}:{last}", kind.name(), offset + rows.len()))
    } else if kind == ProductKind::Episode && volume_count > 0 {
        Some(String::from("volume:0"))
    } else {
        None
    };
    let page = next.as_ref().map_or(ChapterPage::Complete, |next| {
        ChapterPage::NextCursor(next.as_bytes())
    });
    if excluded.is_empty() {
        return output.finish(page).map_err(port::chapter_error);
    }
    let mut output = output
        .with_excluded_ids(page)
        .map_err(port::chapter_error)?;
    for id in &excluded {
        output.push(id.as_bytes()).map_err(port::chapter_error)?;
    }
    output.finish().map_err(port::chapter_error)
}

fn chapter_identity(value: &str) -> Result<(&str, ProductKind, &str)> {
    let mut fields = value.split(':');
    let series = fields
        .next()
        .filter(|s| giga::id(s))
        .ok_or("invalid_request")?;
    let kind = fields
        .next()
        .and_then(ProductKind::parse)
        .ok_or("invalid_request")?;
    let product = fields
        .next()
        .filter(|s| giga::id(s))
        .ok_or("invalid_request")?;
    if fields.next().is_some() {
        return Err("invalid_request");
    }
    Ok((series, kind, product))
}

fn viewer(kind: ProductKind, id: &str) -> Result<Value> {
    // The same official episode can return usagi on mobile and baku on
    // desktop. The ordinary viewer GET selects the verified desktop variant.
    let body = port::request(
        &kind.url(SITE, id),
        "GET",
        json!({"Accept":"text/html","User-Agent":DESKTOP_UA}),
        None,
    )?;
    let doc = Node::parse(&body)?;
    let config = doc
        .require("script#episode-json[data-value]")?
        .attr_bounded("data-value", 1024 * 1024)?;
    serde_json::from_str(&config).map_err(|_| "parse_error")
}

fn pages(input: &Value) -> Result<Value> {
    let chapter = input["args"]["chapterId"]
        .as_str()
        .ok_or("invalid_request")?;
    let key = chapter.strip_prefix(CHAPTER).ok_or("invalid_request")?;
    let (series, kind, product) = chapter_identity(key)?;
    let value = viewer(kind, product)?;
    let viewer = giga::viewer(&value, SITE, kind, product, series)?;
    let pages = viewer.pages.iter().enumerate().map(|(index,raw)| -> Result<Value> {
        let url = page_image(raw)?;
        Ok(json!({"id":format!("{PAGE}{key}:{index}"),"index":index,"image":{"kind":"url","url":url.as_str()}}))
    }).collect::<Result<Vec<_>>>()?;
    // Keep the provider's indices intact. On Koma the Manga format tag sets
    // the default RTL reading preference without overriding the user's mode.
    Ok(json!({"chapterId":chapter,"pages":pages,"readingDirection":viewer.direction}))
}

fn image_request(request: &[u8], input: &Value) -> Result<Value> {
    let args = &input["args"];
    let raw = args["url"].as_str().ok_or("invalid_request")?;
    let url = image_url(raw)?;
    let key = args["pageId"].as_str().and_then(|id| id.strip_prefix(PAGE));
    let Some(key) = key else {
        if url.path().starts_with("/public/page/") {
            return Err("invalid_request");
        }
        return Ok(
            json!({"imageRequest":{"url":url.as_str(),"headers":{"Referer":format!("{SITE}/")}}}),
        );
    };
    let (identity, index) = key.rsplit_once(':').ok_or("invalid_request")?;
    let index = index.parse::<usize>().map_err(|_| "invalid_request")?;
    let (series, kind, product) = chapter_identity(identity)?;
    // A cold resolver must find the render mode and public access from the
    // current episode. The ID only locates the page, with no per-image config.
    let value = viewer(kind, product)?;
    let view = giga::viewer(&value, SITE, kind, product, series)?;
    let current = view.pages.get(index).ok_or("page_unavailable")?;
    if page_image(current)? != url {
        return Err("page_unavailable");
    }
    let mut result =
        json!({"imageRequest":{"url":url.as_str(),"headers":{"Referer":kind.url(SITE,product)}}});
    if matches!(view.tiles, Tiles::Baku) {
        if !source_protocol::feature(request, DECODE) {
            return Err("host_feature_required");
        }
        result["requiredImageDecode"] =
            json!({"kind":"wasm_image_v1","pixels":r#"{"mode":"baku"}"#});
    }
    Ok(result)
}

fn run_search(request: &[u8]) -> u32 {
    port::write_result(
        "search",
        port::input(request).and_then(|input| search(&input)),
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
        port::input(request).and_then(|input| detail(&input)),
    )
}
fn run_get_pages(request: &[u8]) -> u32 {
    port::write_result(
        "get_pages",
        port::input(request).and_then(|input| pages(&input)),
    )
}
fn run_get_image_request(request: &[u8]) -> u32 {
    port::write_result(
        "get_image_request",
        port::input(request).and_then(|input| image_request(request, &input)),
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
    chapters(request)
}
fn write_chapter_failure(code: &str) -> u32 {
    port::write_failure("get_chapters", code)
}
koma_source_sdk::koma_source_exports!("shonenjumpplus_ja", typed_chapters, write_chapter_failure);
