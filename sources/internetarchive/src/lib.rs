#![no_std]
extern crate alloc;

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use koma_source_sdk::{
    chapters::{Chapter, ChapterPage, ChapterPlacement, ChapterResult, GroupedChapters},
    source::{SourceCapabilities, SourceInfo},
};
use serde_json::{json, Value};

#[path = "../../port_support.rs"]
#[allow(dead_code)]
mod port;
#[path = "../../port_catalog.rs"]
mod port_catalog;
mod reader;
use port::Result;
use port_catalog::Labels;

#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

const METADATA: &[u8] = include_bytes!("../metadata.json");
const SITE: &str = "https://archive.org";
const WORK: &str = "ia:";
const CHAPTER: &str = "ia-book:";
const OUTPUT: usize = 1000 * 1024;
const LIST_SIZE: usize = 24;
const SOURCE_INFO: SourceInfo = SourceInfo {
    id:"org.archive.texts.koma",name:"Internet Archive",language:"all",
    version:env!("CARGO_PKG_VERSION"),api_version:"0.2",
    description:"Browse publicly readable scans from the Internet Archive, including illustrated books, historical magazines, vintage comics and art albums.",
    author:"Koma source contributors",content_rating:"mixed",
};
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: true,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: true,
    manga_list: true,
    filters: true,
    settings: false,
    home: false,
    credentials: false,
    image_request: false,
};
koma_source_sdk::koma_source_buffers! {
    payload: OUTPUT, http_out: 6 * 1024 * 1024, body: 3 * 1024 * 1024,
    http_req: 8192,scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

// Search index formats narrow discovery. Current file permissions and
// BookReader access are checked when opening an item.
const SCANNED_TEXTS_QUERY: &str = concat!(
    "mediatype:texts AND NOT access-restricted-item:true ",
    "AND NOT collection:inlibrary AND NOT collection:printdisabled ",
    r#"AND (format:"Single Page Processed JP2 ZIP" OR format:"Single Page Processed JPEG ZIP") "#,
    r#"AND format:"Scandata""#,
);

// Indexed image counts keep popular periodical and comic bundles out of
// these listings. The reader still enforces the actual response limits.
const COLLECTIONS: &[(&str, &str, &str)] = &[
    (
        "picture_books",
        "Public-domain picture books",
        concat!(
            "(identifier:ost-english--peter-rabbit-by-beatrix-potter ",
            "OR (collection:iacl AND year:[1700 TO 1925]) ",
            r#"OR ((subject:"picture books" OR subject:"children's stories" OR subject:"illustrated") AND rights:"public domain")) "#,
            "AND NOT collection:gutenberg",
        ),
    ),
    (
        "magazines",
        "Historical magazines",
        concat!(
            "(collection:thestrandmagazine OR (collection:magazine_rack AND year:[1750 TO 1940])) ",
            "AND imagecount:[1 TO 1600]",
        ),
    ),
    (
        "comics",
        "Vintage comics",
        concat!(
            "(identifier:BlueBeetle0201955 ",
            "OR ((collection:comics OR collection:comicbooks OR collection:webcomicuniverse) ",
            "AND year:[1890 TO 1970] AND imagecount:[1 TO 1600]))",
        ),
    ),
    (
        "art",
        "Art albums",
        r#"(identifier:historyofjapanes00seid_2 OR (collection:getty AND (subject:painting OR subject:prints OR subject:illustration)))"#,
    ),
    ("all", "All public image archives", ""),
];

fn listing(input: &Value, search: bool) -> Result<Value> {
    let args = &input["args"];
    let key = if search {
        args["filters"]["collection"].as_str().unwrap_or("all")
    } else {
        args["listingId"].as_str().unwrap_or("picture_books")
    };
    let (_, _, category) = COLLECTIONS
        .iter()
        .find(|(id, _, _)| *id == key)
        .ok_or("invalid_request")?;
    let page = match &args["page"]["cursor"] {
        Value::Null => 1,
        cursor => cursor
            .as_str()
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| (2..=1000).contains(n))
            .ok_or("invalid_request")?,
    };
    let mut query = String::from(SCANNED_TEXTS_QUERY);
    if !category.is_empty() {
        query.push_str(" AND ");
        query.push_str(category);
    }
    if search {
        let term = args["query"].as_str().unwrap_or("").trim();
        if term.len() > 512 {
            return Err("invalid_request");
        }
        // Read the query as a phrase, leaving collection and access filters
        // under source control even if the input contains Lucene syntax.
        if !term.is_empty() {
            query.push_str(" AND \"");
            for c in term.chars() {
                if matches!(c, '"' | '\\') {
                    query.push('\\');
                }
                if !c.is_control() {
                    query.push(c);
                }
            }
            query.push('"');
        }
    }
    let mut url = port::absolute(SITE, "/advancedsearch.php")?;
    {
        let mut pairs = url.query_pairs_mut();
        pairs
            .append_pair("q", &query)
            .append_pair("rows", &LIST_SIZE.to_string())
            .append_pair("page", &page.to_string())
            .append_pair("output", "json");
        for field in [
            "identifier",
            "title",
            "creator",
            "year",
            "mediatype",
            "collection",
            "access-restricted-item",
        ] {
            pairs.append_pair("fl[]", field);
        }
        if !search || args["query"].as_str().unwrap_or("").trim().is_empty() {
            pairs
                .append_pair("sort[]", "downloads desc")
                .append_pair("sort[]", "identifier asc");
        }
    }
    let value = reader::json_get(url.as_str())?;
    let response = &value["response"];
    let rows = response["docs"].as_array().ok_or("parse_error")?;
    let total = response["numFound"].as_u64().ok_or("parse_error")?;
    let items = rows
        .iter()
        .filter_map(|row| {
            let id = row["identifier"]
                .as_str()
                .filter(|id| reader::identifier(id))?;
            let title = row["title"].as_str().filter(|s| !s.trim().is_empty())?;
            if reader::restricted(row) || row["mediatype"] != "texts" {
                return None;
            }
            Some(item(id, title, row))
        })
        .collect();
    let next = (!rows.is_empty() && page < 1000 && ((page * LIST_SIZE) as u64) < total)
        .then_some(page + 1);
    Ok(port::items(items, next))
}

fn item(id: &str, title: &str, metadata: &Value) -> Value {
    let url = format!("{SITE}/details/{id}");
    json!({"id":format!("{WORK}{id}"),"title":title,"authors":reader::strings(&metadata["creator"]),"url":url,
        "cover":{"kind":"url","url":format!("{SITE}/services/img/{id}")},"links":[{"kind":"source","url":url}]})
}

fn work(input: &Value) -> Result<&str> {
    input["args"]["mangaId"]
        .as_str()
        .and_then(|s| s.strip_prefix(WORK))
        .filter(|id| reader::identifier(id))
        .ok_or("invalid_request")
}

fn detail(input: &Value) -> Result<Value> {
    let id = work(input)?;
    let book = reader::metadata(id)?;
    // Discovery candidates may be unavailable. Opening a work requires one
    // fully validated public scan before exposing it as a readable work.
    let title = book.verify_any()?;
    let metadata = &book.value["metadata"];
    let mut content = item(id, metadata["title"].as_str().unwrap_or(&title), metadata);
    let description = reader::strings(&metadata["description"]).join("\n");
    content["description"] = json!(description);
    content["tags"] = json!(reader::strings(&metadata["subject"]));
    Ok(json!({"manga":content}))
}

fn chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(request)?;
    let id = work(&input)?;
    let book = reader::metadata(id)?;
    let manga = format!("{WORK}{id}");
    let args = &input["args"];
    let exact = args["chapterId"].as_str();
    let mut output = GroupedChapters::new(payload_buf(), request).map_err(port::chapter_error)?;
    let mut accepted = 0;
    let mut refusal = "no_public_scan";
    for scan in &book.scans {
        let key = reader::encode(&scan.prefix);
        let chapter_id = format!("{CHAPTER}{id}:{key}");
        if exact.is_some_and(|wanted| wanted != chapter_id) {
            continue;
        }
        let volume = match book.open(scan) {
            Ok(volume) => volume,
            Err(reason @ ("archive_restricted" | "no_public_scan" | "result_too_large")) => {
                refusal = reason;
                continue;
            }
            Err(reason) => return Err(reason),
        };
        let volume_title = if book.scans.len() > 1 {
            format!("{} · {}", volume.title, scan.prefix)
        } else {
            volume.title
        };
        let url = format!("{SITE}/details/{id}");
        let mut chapter = Chapter::new(
            chapter_id.as_bytes(),
            manga.as_bytes(),
            volume_title.as_bytes(),
        );
        chapter.url = Some(url.as_bytes());
        chapter.page_count = Some(volume.pages.len());
        // Multiple scans are separate volumes or editions. There is no
        // verified cross-scan continuation in the metadata.
        output
            .push(chapter, ChapterPlacement::ManualOnly)
            .map_err(port::chapter_error)?;
        accepted += 1;
    }
    if accepted == 0 {
        return Err(refusal);
    }
    output
        .finish(ChapterPage::Complete)
        .map_err(port::chapter_error)
}

fn pages(input: &Value) -> Result<Value> {
    let args = &input["args"];
    let chapter = args["chapterId"].as_str().ok_or("invalid_request")?;
    let identity = chapter.strip_prefix(CHAPTER).ok_or("invalid_request")?;
    let (id, key) = identity.split_once(':').ok_or("invalid_request")?;
    if !reader::identifier(id)
        || args["mangaId"]
            .as_str()
            .is_some_and(|s| s.strip_prefix(WORK) != Some(id))
    {
        return Err("invalid_request");
    }
    let book = reader::metadata(id)?;
    let scan = book
        .scans
        .iter()
        .find(|scan| reader::encode(&scan.prefix) == key)
        .ok_or("no_public_scan")?;
    let volume = book.open(scan)?;
    volume.payload(chapter, identity)
}

fn run_search(request: &[u8]) -> u32 {
    port::write_result(
        "search",
        port::input(request).and_then(|input| listing(&input, true)),
    )
}
fn run_get_manga_list(request: &[u8]) -> u32 {
    port::write_result(
        "get_manga_list",
        port::input(request).and_then(|input| listing(&input, false)),
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
fn run_get_listings(request: &[u8]) -> u32 {
    let result = Labels::new(METADATA, request).and_then(|labels| {
        let listings = COLLECTIONS
            .iter()
            .map(|(id, name, _)| {
                Ok(json!({"id":id,
            "name":labels.text(&format!("listing.{id}"),name)?,"supportsPaging":true}))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(json!({"listings":listings}))
    });
    port::write_result("get_listings", result)
}
fn run_get_filters(request: &[u8]) -> u32 {
    let result = Labels::new(METADATA, request).and_then(|labels| {
        let options = COLLECTIONS
            .iter()
            .map(|(id, name, _)| {
                Ok(json!({"value":id,"label":labels.text(&format!("listing.{id}"),name)?}))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(
            json!({"useSearchForFilters":true,"filters":[{"id":"collection","type":"select","scope":"search","default":"all",
            "name":labels.text("filter.collection","Archive collection")?,"options":options}]}),
        )
    });
    port::write_result("get_filters", result)
}
fn run_get_settings(_: &[u8]) -> u32 {
    port::write_result("get_settings", Ok(json!({"settings":[]})))
}
fn run_get_home(_: &[u8]) -> u32 {
    port::write_failure("get_home", "not_implemented")
}
fn run_get_image_request(_: &[u8]) -> u32 {
    port::write_failure("get_image_request", "not_implemented")
}
fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    chapters(request)
}
fn write_chapter_failure(code: &str) -> u32 {
    port::write_failure("get_chapters", code)
}
koma_source_sdk::koma_source_exports!("internetarchive", typed_chapters, write_chapter_failure);
