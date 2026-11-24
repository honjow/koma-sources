#![no_std]
extern crate alloc;

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use koma_source_sdk::{
    chapters::{Chapter, ChapterNumber, ChapterOrder, ChapterResponse, ChapterResult},
    json_utils::request_page_number,
    source::{SourceCapabilities, SourceInfo},
};
use serde_json::{json, Value};

#[path = "../../port_support.rs"]
mod port_support;
use port_support::{self as port, Node, Result};

const MIRRORS: &[&str] = &[
    "https://ikanmh.xyz",
    "https://ikanmh.cc",
    "https://www.jjmhw12.top",
];
const HOSTS: &[&str] = &["ikanmh.xyz", "ikanmh.cc", "jjmhw12.top"];
const TAGS: &[&str] = &[
    "全部", "青春", "性感", "长腿", "多人", "御姐", "巨乳", "新婚", "媳妇", "暧昧", "清纯", "调教",
    "少妇", "风骚", "同居", "淫乱", "好友", "女神", "诱惑", "偷情", "出轨", "正妹", "家教",
];
const AREAS: &[(&str, &str)] = &[("-1", "全部"), ("1", "韩国"), ("2", "日本"), ("3", "台湾")];
const STATES: &[(&str, &str)] = &[("-1", "全部"), ("0", "连载中"), ("1", "已完结")];

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.ikanmh.koma",
    name: "漫小肆韓漫",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "漫小肆韓漫的漫畫書庫，地區分類包含韓國、日本與台灣，作品依題材及連載進度整理。",
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
    settings: true,
    image_request: false,
    credentials: false,
};
#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;
koma_source_sdk::koma_source_buffers! {
    payload: 1024 * 1024, http_out: 4 * 1024 * 1024, body: 3 * 1024 * 1024,
    http_req: 4096, scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

fn mirror(input: &Value) -> &str {
    input["settings"]["preferredMirror"]
        .as_str()
        .filter(|value| MIRRORS.contains(value))
        .unwrap_or(MIRRORS[0])
}

fn valid_path(path: &str, prefix: &str) -> bool {
    path.strip_prefix(prefix).is_some_and(|id| {
        !id.is_empty() && id.len() <= 32 && id.bytes().all(|ch| ch.is_ascii_digit())
    })
}

fn local_path(base: &str, href: &str, prefix: &str) -> Result<String> {
    let url = port::absolute(base, href)?;
    let host = url.host_str().ok_or("parse_error")?;
    if !HOSTS.contains(&host.trim_start_matches("www.")) || !valid_path(url.path(), prefix) {
        return Err("parse_error");
    }
    Ok(url.path().to_string())
}

fn document(url: &str) -> Result<Node> {
    // All three mirrors select their mobile HTML using the document User-Agent.
    // The upstream mobile catalogue and reader selectors require that variant.
    let body = port::request(
        url,
        "GET",
        json!({"User-Agent": port::MOBILE_IMAGE_AGENT}),
        None,
    )?;
    Node::parse(&body)
}

fn cover(base: &str, item: &mut Value, node: &Node, selector: &str) -> Result<()> {
    if let Some(img) = node.one(selector) {
        let raw = img.attr("data-original")?;
        if !raw.is_empty() {
            item["cover"] = json!({"kind":"url","url":port::absolute(base, &raw)?.as_str()});
        }
    }
    Ok(())
}

fn manga(base: &str, id: &str, title: &str) -> Value {
    json!({"id":id,"title":title,"links":[{"kind":"source","url":format!("{base}{id}")}]})
}

fn cards(base: &str, page: &Node, search: bool) -> Result<Vec<Value>> {
    let selector = if search {
        "ul.book-list > li"
    } else {
        "ul.manga-list-2 > li"
    };
    let mut results = Vec::new();
    for node in page.all(selector)? {
        let Some(link) = node
            .one(".manga-list-2-title a[href]")
            .or_else(|| node.one("a[href]"))
        else {
            continue;
        };
        let id = local_path(base, &link.attr("href")?, "/book/")?;
        let title = if search {
            node.one("p.book-list-info-title")
                .map(|n| n.text())
                .transpose()?
                .unwrap_or_default()
        } else {
            let title = link.text()?;
            if title.is_empty() {
                node.one(".manga-list-2-title")
                    .map(|n| n.text())
                    .transpose()?
                    .unwrap_or_default()
            } else {
                title
            }
        };
        if title.is_empty() {
            continue;
        }
        let mut item = manga(base, &id, &title);
        let cover_selector = if search {
            "img.book-list-cover-img"
        } else {
            "img[data-original]"
        };
        cover(base, &mut item, &node, cover_selector)?;
        results.push(item);
    }
    Ok(results)
}

fn details(base: &str, id: &str) -> Result<Value> {
    let page = document(&format!("{base}{id}"))?;
    let Some(heading) = page.one("p.detail-main-info-title") else {
        // Two published mobile-domain snapshots serve the desktop markup at this URL.
        let desktop = page.require(".banner_detail_form")?;
        let title = desktop.require("h1")?.text()?;
        let mut item = manga(base, id, &title);
        let image = desktop.require(".cover > img")?.attr("src")?;
        item["cover"] = json!({"kind":"url","url":port::absolute(base,&image)?.as_str()});
        if let Some(node) = desktop.one("p.content") {
            item["description"] = json!(node.text()?);
        }
        for node in desktop.all("p.subtitle")? {
            if let Some(author) = node.text()?.strip_prefix("作者：") {
                item["authors"] = json!([author.trim()]);
                break;
            }
        }
        let tags = desktop
            .all("a[href*='tag=']")?
            .iter()
            .map(Node::text)
            .collect::<Result<Vec<_>>>()?;
        item["tags"] = json!(tags);
        return Ok(item);
    };
    let title = heading.text()?;
    if title.is_empty() {
        return Err("parse_error");
    }
    let mut item = manga(base, id, &title);
    cover(base, &mut item, &page, ".detail-main-cover img")?;
    if let Some(node) = page.one("p.detail-desc") {
        item["description"] = json!(node.text()?);
    }
    let tags = page
        .all("p.detail-main-info-class a")?
        .iter()
        .map(Node::text)
        .collect::<Result<Vec<_>>>()?;
    item["tags"] = json!(tags);
    for node in page.all("p.detail-main-info-author")? {
        if node.text()?.starts_with("作者") {
            if let Some(link) = node.one("a") {
                item["authors"] = json!([link.text()?]);
            }
            break;
        }
    }
    if let Some(node) = page.one("span.detail-list-title-1") {
        item["status"] = json!(match node.text()?.as_str() {
            "连载中" => "ongoing",
            "已完结" => "completed",
            _ => "unknown",
        });
    }
    Ok(item)
}

fn chapter_owner(base: &str, page: &Node) -> Result<String> {
    local_path(
        base,
        &page
            .require("a.view-fix-bottom-bar-item[href^='/book/']")?
            .attr("href")?,
        "/book/",
    )
}

fn search_link(base: &str, query: &str) -> Result<Option<Value>> {
    let Ok(url) = url::Url::parse(query) else {
        return Ok(None);
    };
    if !matches!(url.scheme(), "http" | "https")
        || !url
            .host_str()
            .is_some_and(|host| HOSTS.contains(&host.trim_start_matches("www.")))
    {
        return Ok(None);
    }
    if valid_path(url.path(), "/book/") {
        return details(base, url.path()).map(Some);
    }
    if valid_path(url.path(), "/chapter/") {
        let page = document(&format!("{base}{}", url.path()))?;
        return details(base, &chapter_owner(base, &page)?).map(Some);
    }
    Err("invalid_request")
}

fn filter_value<'a>(
    args: &'a Value,
    id: &str,
    default: &'static str,
    is_valid: impl FnOnce(&str) -> bool,
) -> Result<&'a str> {
    let Some(value) = args["filters"].get(id) else {
        return Ok(default);
    };
    let value = value.as_str().ok_or("invalid_request")?;
    is_valid(value).then_some(value).ok_or("invalid_request")
}

fn list(input: &Value, request: &[u8], search: bool) -> Result<Value> {
    let args = &input["args"];
    let base = mirror(input);
    let query = args["query"].as_str().unwrap_or("").trim();
    let page = request_page_number(request);
    let listing = args["listingId"].as_str().unwrap_or("popular");
    let text_search = search && !query.is_empty();
    if text_search {
        if page > 1 {
            return Ok(port::items(Vec::new(), None));
        }
        if let Some(item) = search_link(base, query)? {
            return Ok(port::items(alloc::vec![item], None));
        }
    } else if !matches!(listing, "latest" | "popular") {
        return Err("invalid_request");
    }
    let path = if text_search {
        "/search"
    } else if listing == "latest" {
        "/update"
    } else {
        "/booklist/"
    };
    let mut url = port::absolute(base, path)?;
    if text_search {
        url.query_pairs_mut().append_pair("keyword", query);
    } else {
        if listing == "popular" {
            let tag = filter_value(args, "tag", "全部", |v| TAGS.contains(&v))?;
            let area = filter_value(args, "area", "-1", |v| {
                AREAS.iter().any(|(key, _)| *key == v)
            })?;
            let end = filter_value(args, "end", "-1", |v| {
                STATES.iter().any(|(key, _)| *key == v)
            })?;
            url.query_pairs_mut()
                .append_pair("tag", tag)
                .append_pair("area", area)
                .append_pair("end", end);
        }
        if page > 1 {
            url.query_pairs_mut().append_pair("page", &page.to_string());
        }
    }
    let view = document(url.as_str())?;
    let items = cards(base, &view, text_search)?;
    let next = if !text_search
        && view
            .one("div.pagination a.paginate-btn[title='下一页']")
            .is_some()
    {
        page.checked_add(1)
    } else {
        None
    };
    Ok(port::items(items, next))
}

fn explicit_number(title: &str) -> Option<usize> {
    let after = title.split_once('第')?.1;
    let end = after.bytes().take_while(u8::is_ascii_digit).count();
    after[..end].parse().ok()
}

fn manga_id(input: &Value) -> Result<&str> {
    input["args"]["mangaId"]
        .as_str()
        .filter(|id| valid_path(id, "/book/"))
        .ok_or("invalid_request")
}

fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(request)?;
    let id = manga_id(&input)?;
    let base = mirror(&input);
    let page = document(&format!("{base}{id}"))?;
    let desktop = page.one("p.detail-main-info-title").is_none();
    if desktop {
        page.require(".banner_detail_form h1")?;
    }
    let updated = page
        .one("span.detail-list-title-3")
        .map(|n| n.text())
        .transpose()?
        .unwrap_or_default();
    let updated = updated.trim_end_matches("更新").trim();
    let mut entries = Vec::new();
    let selector = if desktop {
        "ul#detail-list-select a[href^='/chapter/']"
    } else {
        "ul#detail-list-select a.chapteritem"
    };
    for link in page.all(selector)? {
        let id = local_path(base, &link.attr("href")?, "/chapter/")?;
        entries.push((id, link.text()?));
    }
    let mut chapters = Vec::new();
    for (index, (chapter_id, title)) in entries.iter().rev().enumerate() {
        let mut chapter = Chapter::new(chapter_id.as_bytes(), id.as_bytes(), title.as_bytes());
        chapter.index = Some(index);
        chapter.chapter_number = explicit_number(title).map(ChapterNumber::Integer);
        if index == 0
            && updated.len() == 10
            && updated
                .as_bytes()
                .iter()
                .all(|ch| ch.is_ascii_digit() || *ch == b'-')
        {
            chapter.updated_at = Some(updated.as_bytes());
        }
        chapters.push(chapter);
    }
    ChapterResponse::ordered(payload_buf(), request, ChapterOrder::NewestFirst, &chapters)
        .map_err(port::chapter_error)
}

fn pages(input: &Value) -> Result<Value> {
    let chapter = input["args"]["chapterId"]
        .as_str()
        .filter(|id| valid_path(id, "/chapter/"))
        .ok_or("invalid_request")?;
    let base = mirror(input);
    let page = document(&format!("{base}{chapter}"))?;
    if let Some(owner) = input["args"]["mangaId"].as_str() {
        if owner != chapter_owner(base, &page)? {
            return Err("invalid_request");
        }
    }
    let nodes = page.all("#cp_img img[data-original]")?;
    if nodes.is_empty() {
        return Err("chapter_unavailable");
    }
    let images = nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let image = port::absolute(base, &node.attr("data-original")?)?;
            Ok(port::page(index, chapter, image))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(json!({"pages":images}))
}

fn select(id: &str, title: &str, choices: &[(&str, &str)]) -> Value {
    let mut f = port::select(id, title, "listing", choices);
    f["default"] = json!(choices[0].0);
    f
}

fn dispatch(operation: &str, request: &[u8]) -> Result<Value> {
    let input = port::input(request)?;
    match operation {
        "get_listings" => Ok(json!({"listings":[
            {"id":"popular","name":"热门","supportsPaging":true,"supportsFilters":true},
            {"id":"latest","name":"最近更新","supportsPaging":true,"supportsFilters":false}
        ]})),
        "get_filters" => {
            let tags = TAGS.iter().map(|tag| (*tag, *tag)).collect::<Vec<_>>();
            Ok(
                json!({"filters":[select("tag","分类",&tags),select("area","地区",AREAS),select("end","进度",STATES)]}),
            )
        }
        "get_settings" => {
            let options = MIRRORS
                .iter()
                .map(|m| json!({"value":m,"label":m.trim_start_matches("https://")}))
                .collect::<Vec<_>>();
            Ok(
                json!({"settings":[{"id":"preferredMirror","kind":"select","name":"备用域名","default":MIRRORS[0],"options":options}]}),
            )
        }
        "get_manga_list" => list(&input, request, false),
        "search" => list(&input, request, true),
        "get_manga" => Ok(json!({"manga":details(mirror(&input),manga_id(&input)?)?})),
        "get_pages" => pages(&input),
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
koma_source_sdk::koma_source_exports!("ikanmh", typed_chapters, write_chapter_failure);

mod comments;
koma_source_sdk::koma_source_comments_export!(comments::run);
