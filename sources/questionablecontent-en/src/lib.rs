#![no_std]
extern crate alloc;

use alloc::{
    collections::BTreeSet,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use koma_source_sdk::{
    chapters::{
        Chapter, ChapterNumber, ChapterPage, ChapterPlacement, ChapterResult, GroupedChapters,
    },
    host,
    runtime::Context,
    source::{SourceCapabilities, SourceInfo},
};
use serde_json::{json, Value};
use url::Url;

#[path = "../../port_support.rs"]
#[allow(dead_code)]
mod port;
#[path = "../../port_catalog.rs"]
mod port_catalog;

use port::{Node, Result};
use port_catalog::Labels;

#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

const SITE: &str = "https://www.questionablecontent.net";
const ARCHIVE: &str = "https://www.questionablecontent.net/archive.php";
const COVER: &str = "https://www.questionablecontent.net/images/logo2026.png";
const WORK: &str = "qc:en:archive";
const CHAPTER: &str = "qc:en:strip:";
const TITLE: &str = "Questionable Content";
const AUTHOR: &str = "Jeph Jacques";
const DESCRIPTION: &str =
    "Jeph Jacques’ English webcomic about friendship, romance and robots, running since 2003.";
const METADATA: &[u8] = include_bytes!("../metadata.json");
const NOTES: &str = "showAuthorsNotes";
const CHAPTER_BYTES: usize = 320 * 1024;
const CHAPTER_COUNT: usize = 750;
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.questionablecontent.en.koma",
    name: TITLE,
    language: "en",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    description: DESCRIPTION,
    author: "Koma source contributors",
    content_rating: "safe",
};
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: false,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: true,
    manga_list: true,
    filters: false,
    settings: true,
    home: false,
    credentials: false,
    image_request: false,
};

koma_source_sdk::koma_source_buffers! {
    payload: CHAPTER_BYTES, http_out: 5 * 1024 * 1024, body: 4 * 1024 * 1024,
    http_req: 4096, scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

fn fetch_html(url: &str) -> Result<String> {
    // The upstream extension also excludes zstd, which has truncated this
    // archive in clients that stop at the first frame.
    port::request(
        url,
        "GET",
        json!({"Accept":"text/html","Accept-Encoding":"gzip"}),
        None,
    )
}

fn work(request: &[u8]) -> Result<Value> {
    Ok(json!({
        "id":WORK,"title":TITLE,"url":ARCHIVE,"language":"en","authors":[AUTHOR],
        "description":Labels::new(METADATA, request)?.text("source.description", DESCRIPTION)?,
        "status":"ongoing","contentRating":"safe","cover":{"kind":"url","url":COVER,"headers":{}}
    }))
}

fn require_work(args: &Value) -> Result<()> {
    (args["mangaId"].as_str() == Some(WORK))
        .then_some(())
        .ok_or("invalid_request")
}

fn decimal(raw: &str) -> Option<usize> {
    (!raw.is_empty() && !raw.starts_with('0') && raw.bytes().all(|c| c.is_ascii_digit()))
        .then(|| raw.parse().ok())
        .flatten()
}

fn comic_number(id: &str) -> Option<usize> {
    id.strip_prefix(CHAPTER).and_then(decimal)
}

fn comic_url(number: usize) -> String {
    format!("{SITE}/view.php?comic={number}")
}

fn open_archive() -> Result<Vec<(usize, String)>> {
    const ROW_START: &str = "<div class=\"strip_link\"";
    const BATCH: usize = 1500;
    let html = fetch_html(ARCHIVE)?;
    let mut rows = html.split(ROW_START).skip(1).peekable();
    let mut seen = BTreeSet::new();
    let mut entries = Vec::new();
    // Preserve the provider's document order, including out-of-order strip
    // numbers. Parse only archive rows to avoid repeatedly walking the
    // thousands of lazy images and ads with structural CSS selectors.
    while rows.peek().is_some() {
        let mut fragment = String::new();
        let mut row_count = 0;
        for row in rows.by_ref().take(BATCH) {
            let (contents, _) = row.split_once("</div>").ok_or("parse_error")?;
            fragment.push_str(ROW_START);
            fragment.push_str(contents);
            fragment.push_str("</div>");
            row_count += 1;
        }
        let document = Node::parse(&fragment)?;
        let anchors = document.all("div.strip_link > a[href^=\"view.php?comic=\"]")?;
        if anchors.len() != row_count {
            return Err("parse_error");
        }
        for anchor in anchors {
            if host::check_cancel() {
                return Err("cancelled");
            }
            let href = anchor.attr_bounded("href", 100)?;
            let number = href
                .strip_prefix("view.php?comic=")
                .and_then(decimal)
                .ok_or("parse_error")?;
            if seen.insert(number) {
                entries.push((number, anchor.text()?));
            }
        }
    }
    if entries.is_empty() {
        return Err("parse_error");
    }
    Ok(entries)
}

fn chapter_title(number: usize, raw: &str) -> String {
    let prefix = format!("See #{number}: \"");
    raw.strip_prefix(&prefix)
        .and_then(|text| text.strip_suffix("\" with newspost"))
        .map(|text| format!("{number}: {text}"))
        .unwrap_or_else(|| raw.to_string())
}

fn chapters(
    input: &Value,
    request: &[u8],
) -> Result<koma_source_sdk::chapters::ChapterResponse<'static>> {
    let args = &input["args"];
    require_work(args)?;
    let before = match &args["page"]["cursor"] {
        Value::Null => None,
        Value::String(id) => Some(comic_number(id).ok_or("invalid_request")?),
        _ => return Err("invalid_request"),
    };
    let archive = open_archive()?;
    let first = if let Some(number) = before {
        archive
            .iter()
            .position(|(id, _)| *id == number)
            .map(|index| index + 1)
            .ok_or("catalog_changed")?
    } else {
        0
    };
    let mut output = GroupedChapters::new(payload_buf(), request).map_err(port::chapter_error)?;
    let mut last = String::new();
    let mut budget = CHAPTER_BYTES - 2048;
    for (offset, (number, text)) in archive[first..].iter().enumerate() {
        let id = format!("{CHAPTER}{number}");
        let url = comic_url(*number);
        let title = chapter_title(*number, text);
        let row = 320 + id.len() + WORK.len() + url.len() + 2 * title.len();
        if offset == CHAPTER_COUNT || row > budget {
            if last.is_empty() {
                return Err("result_too_large");
            }
            return output
                .finish(ChapterPage::NextCursor(last.as_bytes()))
                .map_err(port::chapter_error);
        }
        let mut chapter = Chapter::new(id.as_bytes(), WORK.as_bytes(), title.as_bytes());
        chapter.chapter_number = Some(ChapterNumber::Integer(*number));
        chapter.language = Some(b"en");
        chapter.url = Some(url.as_bytes());
        output
            .push(
                chapter,
                ChapterPlacement::InGroup {
                    group: b"archive",
                    position: (archive.len() - 1 - first - offset) as u64,
                },
            )
            .map_err(port::chapter_error)?;
        last = id;
        budget -= row;
    }
    output
        .finish(ChapterPage::Complete)
        .map_err(port::chapter_error)
}

fn notes_supported(request: &[u8]) -> bool {
    Context::from_request(request).supports(b"text_page_image_v1")
}

fn comic_image(raw: &str) -> Result<Url> {
    let mut url = port::absolute(SITE, raw)?;
    if !matches!(
        url.host_str(),
        Some("www.questionablecontent.net" | "questionablecontent.net")
    ) || !url.path().starts_with("/comics/")
        || url.port().is_some()
        || url.set_scheme("https").is_err()
    {
        return Err("parse_error");
    }
    Ok(url)
}

fn pages(input: &Value, request: &[u8]) -> Result<Value> {
    let args = &input["args"];
    if args.get("mangaId").is_some() {
        require_work(args)?;
    }
    let chapter = args["chapterId"].as_str().ok_or("invalid_request")?;
    let number = comic_number(chapter).ok_or("invalid_request")?;
    let doc = Node::parse(&fetch_html(&comic_url(number))?)?;
    let mut pages = Vec::new();
    for image in doc.all("img#strip")? {
        let url = comic_image(&image.attr("src")?)?;
        pages.push(port::page(pages.len(), chapter, url));
    }
    if pages.is_empty() {
        return Err("chapter_unavailable");
    }
    let show_notes = matches!(&input["settings"][NOTES], Value::Bool(true))
        || input["settings"][NOTES].as_str() == Some("true");
    if show_notes && notes_supported(request) {
        if let Some(note) = doc.one("#newspost") {
            let paragraphs = note.all("p")?;
            let mut paragraphs = paragraphs
                .iter()
                .map(Node::text)
                .collect::<Result<Vec<_>>>()?;
            if paragraphs.is_empty() {
                paragraphs.push(note.text()?);
            }
            paragraphs.retain(|text| !text.is_empty());
            if !paragraphs.is_empty() {
                let heading =
                    Labels::new(METADATA, request)?.text("notes.heading", "Author’s notes")?;
                pages.push(json!({
                    "id":format!("{chapter}:notes"),"index":pages.len(),
                    "image":{"kind":"text","text":{"template":"webtoon-author-notes","heading":heading,"paragraphs":paragraphs}}
                }));
            }
        }
    }
    Ok(json!({"chapterId":chapter,"pages":pages}))
}

fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, (&'static str, String)> {
    port::input(request)
        .and_then(|input| chapters(&input, request))
        .map_err(|code| (code, port_catalog::failure(METADATA, request, code)))
}

fn write_chapter_failure((code, message): (&str, String)) -> u32 {
    write_error("get_chapters", code, &message)
}

fn call(request: &[u8], op: &str, action: impl FnOnce(&Value) -> Result<Value>) -> u32 {
    let result = port::input(request).and_then(|input| action(&input));
    port_catalog::write_localized(METADATA, request, op, result)
}

fn run_get_manga(request: &[u8]) -> u32 {
    call(request, "get_manga", |input| {
        require_work(&input["args"])?;
        Ok(json!({"manga":work(request)?}))
    })
}

fn run_get_manga_list(request: &[u8]) -> u32 {
    call(request, "get_manga_list", |input| {
        let args = &input["args"];
        if !args["listingId"].is_null() && args["listingId"] != "popular" {
            return Err("unsupported");
        }
        if !args["page"]["cursor"].is_null() {
            return Err("invalid_request");
        }
        Ok(port::items(vec![work(request)?], None))
    })
}

fn run_get_listings(request: &[u8]) -> u32 {
    call(request, "get_listings", |_| {
        let name = Labels::new(METADATA, request)?.text("listing.popular", "Comic archive")?;
        Ok(
            json!({"listings":[{"id":"popular","name":name,"supportsPaging":false,"supportsFilters":false}]}),
        )
    })
}

fn run_get_settings(request: &[u8]) -> u32 {
    call(request, "get_settings", |_| {
        if !notes_supported(request) {
            return Ok(json!({"settings":[]}));
        }
        let labels = Labels::new(METADATA, request)?;
        Ok(json!({"settings":[{
            "id":NOTES,"kind":"toggle","default":"false",
            "name":labels.text("setting.showAuthorsNotes","Show author’s notes")?,
            "description":labels.text("setting.showAuthorsNotes_hint","Append notes when present.")?
        }]}))
    })
}

fn run_get_pages(request: &[u8]) -> u32 {
    call(request, "get_pages", |input| pages(input, request))
}

fn run_search(request: &[u8]) -> u32 {
    call(request, "search", |_| Ok(port::items(Vec::new(), None)))
}

fn run_get_filters(request: &[u8]) -> u32 {
    call(request, "get_filters", |_| Ok(json!({"filters":[]})))
}

fn run_get_home(request: &[u8]) -> u32 {
    call(request, "get_home", |_| Err("unsupported"))
}

fn run_get_image_request(request: &[u8]) -> u32 {
    call(request, "get_image_request", |_| Err("unsupported"))
}

koma_source_sdk::koma_source_exports!(
    "questionablecontent_en",
    typed_chapters,
    write_chapter_failure
);
