#![no_std]
extern crate alloc;

use alloc::{
    collections::{BTreeMap, BTreeSet},
    format,
    string::{String, ToString},
    vec::Vec,
};
use koma_source_sdk::{
    chapters::{
        Chapter, ChapterPage, ChapterPlacement, ChapterResponse, ChapterResult, GroupedChapters,
    },
    json_utils::request_page_number,
    source::{SourceCapabilities, SourceInfo},
};
use serde_json::{json, Value};

#[path = "../../port_support.rs"]
mod port_support;
use port_support::{self as port, Node, Result};

const BASE: &str = "https://hanimeone.me";
const SORTS: &[(&str, &str)] = &[
    ("", "最新"),
    ("popular-today", "日热门"),
    ("popular-week", "周热门"),
    ("popular", "总热门"),
];

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "me.hanimeone.koma",
    name: "Hanime1.me",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "Hanime1 成人站的漫画专区。作品页会标注作者或社团，系列可在同一作品下找到关联集数。",
    content_rating: "nsfw",
};
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: true,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: true,
    manga_list: true,
    home: false,
    filters: true,
    settings: false,
    image_request: false,
    credentials: false,
};
#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

koma_source_sdk::koma_source_buffers! {
    payload: 1024 * 1024,
    http_out: 4 * 1024 * 1024,
    body: 3 * 1024 * 1024,
    http_req: 4096,
    scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

fn slug(id: &str) -> Result<&str> {
    id.strip_prefix("/comic/")
        .filter(|slug| !slug.is_empty() && slug.bytes().all(|c| c.is_ascii_digit()))
        .ok_or("invalid_request")
}

fn work_path(raw: &str) -> Result<String> {
    let url = port::absolute(BASE, raw)?;
    if url.host_str() != Some("hanimeone.me") || url.query().is_some() || url.fragment().is_some() {
        return Err("parse_error");
    }
    let path = url.path().trim_end_matches('/');
    slug(path).map_err(|_| "parse_error")?;
    Ok(path.to_string())
}

fn first_src(set: &str) -> Result<&str> {
    set.split(',')
        .next()
        .and_then(|candidate| candidate.split_whitespace().next())
        .ok_or("parse_error")
}

fn cover(image: &Node) -> Result<Value> {
    let srcset = image.attr("data-srcset")?;
    Ok(json!({"kind":"url","url":port::absolute(BASE,first_src(&srcset)?)?.as_str()}))
}

fn heading_section<'a>(html: &'a str, heading: &str) -> Result<Option<&'a str>> {
    // The upstream uses Jsoup's containsOwn pseudo selector. Detect the heading
    // separately so both the app and the development host use the same CSS.
    let mut rest = html;
    while let Some((prefix, tail)) = rest.split_once("</h3>") {
        if let Some(start) = prefix.rfind("<h3") {
            if Node::parse(&prefix[start..])?
                .require("h3")?
                .text()?
                .contains(heading)
            {
                let end = tail.find("<h3").unwrap_or(tail.len());
                return Ok(Some(&tail[..end]));
            }
        }
        rest = tail;
    }
    Ok(None)
}

fn card(node: &Node) -> Result<(String, String)> {
    Ok((
        work_path(&node.require("a[href]")?.attr("href")?)?,
        node.require("div.comic-rows-videos-title")?.text()?,
    ))
}

fn list(request: &[u8], args: &Value, search: bool) -> Result<Value> {
    let page = request_page_number(request);
    let listing = args["listingId"].as_str().unwrap_or("popular");
    let mut url = port::absolute(BASE, if search { "/comics/search" } else { "/comics" })?;
    if search {
        let sort = port::filter(args, "sort", SORTS)?;
        url.query_pairs_mut()
            .append_pair("query", args["query"].as_str().unwrap_or(""))
            .append_pair("page", &page.to_string());
        if !sort.is_empty() {
            url.query_pairs_mut().append_pair("sort", sort);
        }
    } else {
        match listing {
            "latest" => {
                url.query_pairs_mut().append_pair("page", &page.to_string());
            }
            "popular" => {}
            _ => return Err("invalid_request"),
        }
    }
    let html = port::get(url.as_str())?;
    let document = Node::parse(&html)?;
    let cards = if search {
        document.all("#comics-search-tag-top-row + div div.comic-rows-videos-div")?
    } else {
        let heading = if listing == "latest" {
            "最新上傳"
        } else {
            "發燒漫畫"
        };
        let section = heading_section(&html, heading)?.ok_or("parse_error")?;
        // Keep the document alive throughout selection and conversion.
        let section = Node::parse(section)?;
        return list_cards(
            section.all("div.comic-rows-videos-div")?,
            listing == "latest" && document.one("ul.pagination a[rel=next]").is_some(),
            page,
        );
    };
    list_cards(
        cards,
        document.one("ul.pagination a[rel=next]").is_some(),
        page,
    )
}

fn list_cards(cards: Vec<Node>, has_more: bool, page: usize) -> Result<Value> {
    let mut items = Vec::new();
    for card_node in cards {
        let (id, title) = card(&card_node)?;
        let mut item = json!({
            "id":id,"title":title,
            "links":[{"kind":"source","url":format!("{BASE}{id}")}]
        });
        if let Some(image) = card_node.one("img[data-srcset]") {
            item["cover"] = cover(&image)?;
        }
        items.push(item);
    }
    Ok(port::items(
        items,
        if has_more { page.checked_add(1) } else { None },
    ))
}

fn work(args: &Value) -> Result<&str> {
    let id = args["mangaId"].as_str().ok_or("invalid_request")?;
    slug(id)?;
    Ok(id)
}

fn metadata(document: &Node, label: &str) -> Result<Option<String>> {
    for row in document.all(".comics-metadata-top-row")? {
        if row.text()?.trim_start().starts_with(label) {
            let field = row
                .one("div.no-select")
                .map(|node| node.text())
                .transpose()?;
            return Ok(field.filter(|value| !value.is_empty()));
        }
    }
    Ok(None)
}

fn details(args: &Value) -> Result<Value> {
    let id = work(args)?;
    let url = format!("{BASE}{id}");
    let html = port::get(&url)?;
    let document = Node::parse(&html)?;
    let title = document
        .require("h3.title.comics-metadata-top-row")?
        .text()?;
    let mut manga = json!({"id":id,"title":title,"links":[{"kind":"source","url":url}]});
    if let Some(image) = document.one("div.col-md-4 img[data-srcset]") {
        manga["cover"] = cover(&image)?;
    }
    let author = metadata(&document, "作者：")?.or(metadata(&document, "社團：")?);
    if let Some(author) = author {
        manga["authors"] = json!([author]);
    }
    if let Some(tags) = metadata(&document, "分類：")? {
        manga["tags"] = json!(tags
            .split(['、', ','])
            .map(str::trim)
            .filter(|tag| !tag.is_empty())
            .collect::<Vec<_>>());
    }
    Ok(json!({"manga":manga}))
}

fn chapter_id(owner: &str, episode: &str) -> String {
    format!("{owner}:{episode}/1")
}

fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    let value = port::input(request)?;
    let owner = work(&value["args"])?;
    let html = port::get(&format!("{BASE}{owner}"))?;
    let document = Node::parse(&html)?;
    document.require("h3.title.comics-metadata-top-row")?;
    let mut rows = Vec::new();
    let mut seen = BTreeSet::new();
    if let Some(section) = heading_section(&html, "相關集數列表")? {
        let episodes = Node::parse(section)?;
        for node in episodes.all("div.comic-rows-videos-div")? {
            let (episode, title) = card(&node)?;
            if seen.insert(episode.clone()) {
                let label = if episode == owner { "當前" } else { "關聯" };
                rows.push((chapter_id(owner, &episode), format!("{label}：{title}")));
            }
        }
    }
    if rows.is_empty() {
        return ChapterResponse::single(
            payload_buf(),
            Chapter::new(
                chapter_id(owner, owner).as_bytes(),
                owner.as_bytes(),
                "單章節".as_bytes(),
            ),
        )
        .map_err(port::chapter_error);
    }
    // Related galleries can be separate works or alternate releases.
    let mut catalog = GroupedChapters::new(payload_buf(), request).map_err(port::chapter_error)?;
    for (index, (id, title)) in rows.iter().enumerate() {
        let mut chapter = Chapter::new(id.as_bytes(), owner.as_bytes(), title.as_bytes());
        chapter.index = Some(index);
        catalog
            .push(chapter, ChapterPlacement::ManualOnly)
            .map_err(port::chapter_error)?;
    }
    catalog
        .finish(ChapterPage::Complete)
        .map_err(port::chapter_error)
}

fn extension(raw: &str) -> Result<&str> {
    raw.strip_prefix('.')
        .unwrap_or(raw)
        .split('?')
        .next()
        .filter(|value| {
            !value.is_empty() && value.len() < 8 && value.bytes().all(|b| b.is_ascii_alphanumeric())
        })
        .ok_or("parse_error")
}

fn pages(args: &Value) -> Result<Value> {
    let id = args["chapterId"].as_str().ok_or("invalid_request")?;
    let (owner, reader) = id.split_once(':').ok_or("invalid_request")?;
    let episode = reader.strip_suffix("/1").ok_or("invalid_request")?;
    slug(owner)?;
    slug(episode)?;
    if args["mangaId"]
        .as_str()
        .is_some_and(|provided| provided != owner)
    {
        return Err("invalid_request");
    }
    let reader_url = format!("{BASE}{reader}");
    let html = port::get(&reader_url)?;
    let reader_document = Node::parse(&html)?;
    let first = reader_document.require("img#current-page-image")?;
    let raw_default = first.attr("data-extension")?;
    let default = extension(&raw_default)?;
    let prefix = first.attr("data-prefix")?;
    let count = reader_document
        .require(".comic-show-content-nav")?
        .attr("data-pages")?
        .parse::<usize>()
        .ok()
        .filter(|n| (1..=4096).contains(n))
        .ok_or("chapter_unavailable")?;

    let gallery_url = format!("{BASE}{episode}");
    let gallery_html = port::get(&gallery_url)?;
    let gallery = Node::parse(&gallery_html)?;
    let mut extensions = BTreeMap::new();
    for link in gallery.all(&format!(
        r#"a[href^="{episode}/"], a[href^="{gallery_url}/"]"#
    ))? {
        let Some(image) = link.one("img[data-srcset]") else {
            continue;
        };
        let href = port::absolute(&gallery_url, &link.attr("href")?)?;
        let Some((work, number)) = href.path().rsplit_once('/') else {
            continue;
        };
        if href.host_str() != Some("hanimeone.me") || work != episode {
            continue;
        }
        let Some(number) = number
            .parse::<usize>()
            .ok()
            .filter(|n| (1..=count).contains(n))
        else {
            continue;
        };
        let srcset = image.attr("data-srcset")?;
        let thumb = port::absolute(&gallery_url, first_src(&srcset)?)?;
        let (_, suffix) = thumb.path().rsplit_once('.').ok_or("parse_error")?;
        extensions.insert(number, extension(suffix)?.to_string());
    }
    let pages = (1..=count)
        .map(|number| {
            let ext = extensions
                .get(&number)
                .map(String::as_str)
                .unwrap_or(default);
            let image = port::absolute(&reader_url, &format!("{prefix}{number}.{ext}"))?;
            Ok(port::page(number - 1, id, image))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(json!({"pages":pages}))
}

fn dispatch(operation: &str, request: &[u8]) -> Result<Value> {
    let input = port::input(request)?;
    let args = &input["args"];
    match operation {
        "get_listings" => Ok(port::listing_list(true, false, false)),
        "get_filters" => Ok(
            json!({"filters":[port::select("sort","排序","search",SORTS)],"useSearchForFilters":true}),
        ),
        "get_manga_list" => list(request, args, false),
        "search" => list(request, args, true),
        "get_manga" => details(args),
        "get_pages" => pages(args),
        _ => Err("not_implemented"),
    }
}

fn run(operation: &str, r: &[u8]) -> u32 {
    port::write_result(operation, dispatch(operation, r))
}
fn run_search(r: &[u8]) -> u32 {
    run("search", r)
}
fn run_get_manga(r: &[u8]) -> u32 {
    run("get_manga", r)
}
fn write_chapter_failure(code: &str) -> u32 {
    port::write_failure("get_chapters", code)
}
fn run_get_pages(r: &[u8]) -> u32 {
    run("get_pages", r)
}
fn run_get_listings(r: &[u8]) -> u32 {
    run("get_listings", r)
}
fn run_get_manga_list(r: &[u8]) -> u32 {
    run("get_manga_list", r)
}
fn run_get_filters(r: &[u8]) -> u32 {
    run("get_filters", r)
}
fn run_get_settings(r: &[u8]) -> u32 {
    run("get_settings", r)
}
fn run_get_home(r: &[u8]) -> u32 {
    run("get_home", r)
}
fn run_get_image_request(r: &[u8]) -> u32 {
    run("get_image_request", r)
}
koma_source_sdk::koma_source_exports!("hanime1", typed_chapters, write_chapter_failure);
