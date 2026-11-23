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
    chapters::{Chapter, ChapterResponse, ChapterResult},
    json_utils::request_page_number,
    source::{SourceCapabilities, SourceInfo},
};
use serde_json::{json, Value};
use url::Url;
#[path = "../../port_support.rs"]
mod port_support;
mod tags;
use port_support::{self as port, Node, Result};
use tags::TAGS;

const BASE: &str = "https://m.xsnvshen.com";
const UA: &str ="Mozilla/5.0 (Linux; Android 13; Mobile) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Mobile Safari/537.36";
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.xsnvshen.koma",
    name: "秀色女神",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "秀色女神的模特寫真相冊，可按模特、服飾、風格、地區與拍攝機構尋找圖集。",
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
    image_request: true,
    credentials: false,
};
#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;
koma_source_sdk::koma_source_buffers! {payload:1024*1024,http_out:4*1024*1024,body:3*1024*1024,http_req:4096,scratch:8192,}
koma_source_sdk::koma_source_helpers!();

fn number(value: &str) -> bool {
    !value.is_empty() && value.len() < 18 && value.bytes().all(|b| b.is_ascii_digit())
}
fn album(value: &str) -> Result<&str> {
    if value.strip_prefix("/album/").is_some_and(number) {
        Ok(value)
    } else {
        Err("invalid_request")
    }
}
fn site_url(url: &Url) -> bool {
    matches!(
        url.host_str(),
        Some("xsnvshen.com" | "www.xsnvshen.com" | "m.xsnvshen.com")
    )
}
fn document(url: &Url) -> Result<Node> {
    Node::parse(&port::request(
        url.as_str(),
        "GET",
        json!({"User-Agent":UA,"Accept":"text/html"}),
        None,
    )?)
}
fn paged(path: &str, page: usize) -> Result<Url> {
    let mut url = port::absolute(BASE, path)?;
    if !site_url(&url) {
        return Err("invalid_request");
    }
    if page > 1 {
        url.query_pairs_mut().append_pair("p", &page.to_string());
    }
    Ok(url)
}
fn item(path: &str, title: &str) -> Value {
    json!({"id":path,"title":title,"links":[{"kind":"source","url":format!("{BASE}{path}")}]})
}
fn can_continue(doc: &Node) -> Result<bool> {
    let options = doc.all("#touch_page option")?;
    Ok(options
        .iter()
        .position(|option| option.attr("selected").is_ok())
        .is_some_and(|idx| idx + 1 < options.len()))
}
fn browse(req: &[u8], args: &Value, search: bool) -> Result<Value> {
    let page = request_page_number(req);
    let query = args["query"].as_str().unwrap_or("").trim();
    let tag = port::filter(args, "tag", TAGS)?;
    let mut method = "list";
    let mut path = if !tag.is_empty() {
        format!("/album/{tag}/")
    } else if args["listingId"] == "latest" {
        "/album/new/".into()
    } else {
        "/album/".into()
    };
    let url = if search && !query.is_empty() {
        if let Ok(url) = Url::parse(query) {
            if matches!(url.scheme(), "http" | "https") {
                if !site_url(&url) {
                    return Err("not_found");
                }
                let path_url = url.path().trim_end_matches('/');
                if let Ok(work) = album(path_url) {
                    return Ok(port::items(vec![details(work)?], None));
                }
                if path_url.strip_prefix("/girl/").is_some_and(number) {
                    method = "model";
                } else if !path_url.strip_prefix("/album/t").is_some_and(number) {
                    return Err("not_found");
                }
                path = path_url.to_string();
                if method == "list" {
                    path.push('/');
                }
                paged(&path, page)?
            } else {
                return Err("invalid_request");
            }
        } else {
            method = "keyword";
            let mut url = port::absolute(BASE, "/search")?;
            url.query_pairs_mut().append_pair("w", query);
            url
        }
    } else {
        paged(&path, page)?
    };
    let doc = document(&url)?;
    let selector = match method {
        "keyword" => "ul.list_3c li",
        "model" => ".star-mod-ul li > a[href^='/album/']",
        _ => "ul.list > li:not([aria-hidden]) > a[href^='/album/']",
    };
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for n in doc.all(selector)? {
        let link = if method == "keyword" {
            let Some(link) = n.one(".titlebox a[href^='/album/']") else {
                continue;
            };
            link
        } else {
            // In listing/model mode the selected node already is the album link.
            let path = n.attr("href")?;
            let Ok(path) = album(&path) else { continue };
            if !seen.insert(path.to_string()) {
                continue;
            }
            let title = if method == "model" {
                n.attr("title")?
            } else {
                n.require(".txtbts3")?.text()?
            };
            let mut row = item(path, &title);
            if let Some(img) = n.one("img[src]") {
                row["cover"] = json!({"kind":"url","url":port::absolute(url.as_str(),&img.attr("src")?)?.as_str()});
            }
            out.push(row);
            continue;
        };
        let path = link.attr("href")?;
        let Ok(path) = album(&path) else { continue };
        if !seen.insert(path.to_string()) {
            continue;
        }
        let mut row = item(path, &link.text()?);
        if let Some(img) = n.one(".picbox img[src]") {
            row["cover"] = json!({"kind":"url","url":port::absolute(url.as_str(),&img.attr("src")?)?.as_str()});
        }
        out.push(row);
    }
    let next = if method != "keyword" && can_continue(&doc)? {
        page.checked_add(1)
    } else {
        None
    };
    Ok(port::items(out, next))
}
fn info(doc: &Node) -> Result<(Option<usize>, Option<String>)> {
    for n in doc.all("em")? {
        let text = n.text()?;
        let Some((_, info)) = text.split_once("更新：") else {
            continue;
        };
        let date = info
            .split_whitespace()
            .next()
            .filter(|v| v.len() == 10)
            .map(String::from);
        let count = info
            .split_once('共')
            .and_then(|(_, photos)| photos.split_once('张'))
            .and_then(|(count, _)| count.trim().parse().ok());
        return Ok((count, date));
    }
    Ok((None, None))
}
fn details(path: &str) -> Result<Value> {
    let url = paged(album(path)?, 1)?;
    let doc = document(&url)?;
    let title = doc.require(".titlebox h1")?.text()?;
    let mut row = item(path, &title);
    row["status"] = json!("completed");
    if let Some(author) = doc.one(".position a[href^='/girl/']") {
        row["authors"] = json!([author.text()?]);
    }
    let mut tags = Vec::new();
    for node in doc.all(".poster-nav a[href^='/album/t'] span")? {
        tags.push(node.text()?);
    }
    row["tags"] = json!(tags);
    if let Some(photo) = doc.one("#arcbox img[datafull]") {
        let image = port::absolute(url.as_str(), &photo.attr("datafull")?)?;
        if let Some((dir, _)) = image.path().rsplit_once('/') {
            let cover = port::absolute(image.as_str(), &format!("/thumb_205x308{dir}/cover.jpg"))?;
            row["cover"] = json!({"kind":"url","url":cover.as_str()});
        }
    }
    let (count, date) = info(&doc)?;
    let mut description = doc
        .one("#arcbox > p:not(:has(img))")
        .map(|n| n.text())
        .transpose()?
        .unwrap_or_default();
    if let Some(count) = count {
        if !description.is_empty() {
            description.push_str("\n\n");
        }
        description.push_str(&format!("照片：{count} 张"));
    }
    row["description"] = json!(description);
    if let Some(date) = date {
        row["updatedAt"] = json!(date);
    }
    Ok(row)
}
fn run_get_chapters(req: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(req)?;
    let path = album(input["args"]["mangaId"].as_str().ok_or("invalid_request")?)?;
    let doc = document(&paged(path, 1)?)?;
    doc.require(".titlebox h1")?;
    let (count, date) = info(&doc)?;
    let name = count
        .map(|n| format!("共 {n} 张"))
        .unwrap_or_else(|| "图集".into());
    let mut row = Chapter::new(path.as_bytes(), path.as_bytes(), name.as_bytes());
    row.page_count = count;
    row.published_at = date.as_deref().map(str::as_bytes);
    ChapterResponse::single(payload_buf(), row).map_err(port::chapter_error)
}
fn pages(args: &Value) -> Result<Value> {
    let path = album(args["chapterId"].as_str().ok_or("invalid_request")?)?;
    if args["mangaId"].as_str().is_some_and(|id| id != path) {
        return Err("invalid_request");
    }
    let first = paged(path, 1)?;
    let document_first = document(&first)?;
    let count = document_first.all("#touch_page option")?.len().max(1);
    if count > 100 {
        return Err("result_too_large");
    }
    let mut output = Vec::new();
    for page in 1..=count {
        let url = paged(path, page)?;
        let extra = if page > 1 {
            Some(document(&url)?)
        } else {
            None
        };
        let d = extra.as_ref().unwrap_or(&document_first);
        let nodes = d.all("#arcbox img[datafull]")?;
        if nodes.is_empty() {
            return Err("chapter_unavailable");
        }
        for n in nodes {
            let uri = port::absolute(url.as_str(), &n.attr("datafull")?)?;
            output.push(port::page(output.len(), path, uri));
        }
    }
    Ok(json!({"pages":output}))
}
fn page_owner(args: &Value) -> Option<&str> {
    let id = args["pageId"].as_str()?;
    let (owner, index) = id.rsplit_once(':')?;
    index.parse::<usize>().ok()?;
    Some(owner)
}
fn image_request(args: &Value) -> Result<Value> {
    let url = port::absolute(BASE, args["url"].as_str().ok_or("invalid_request")?)?;
    let referer = match page_owner(args).and_then(|id| album(id).ok()) {
        Some(path) => format!("{BASE}{path}"),
        None => format!("{BASE}/"),
    };
    // Reader image request headers are returned here. Cover headers still
    // travel with the work card whenever the host supports them.
    Ok(json!({"url":url.as_str(),"headers":port::mobile_image_headers(&referer)}))
}
fn dispatch(op: &str, req: &[u8]) -> Result<Value> {
    let input = port::input(req)?;
    let args = &input["args"];
    match op {
        "get_listings" => Ok(port::listing_list(true, true, true)),
        "get_filters" => Ok(json!({"filters":[port::select("tag","分类","browse",TAGS)]})),
        "get_manga_list" => browse(req, args, false),
        "search" => browse(req, args, true),
        "get_manga" => {
            Ok(json!({"manga":details(args["mangaId"].as_str().ok_or("invalid_request")?)?}))
        }
        "get_pages" => pages(args),
        "get_image_request" => image_request(args),
        _ => Err("not_implemented"),
    }
}
fn run(op: &str, r: &[u8]) -> u32 {
    let result = dispatch(op, r).map(|mut value| {
        if matches!(op, "get_manga_list" | "search" | "get_manga") {
            port::mobile_cover_headers(&mut value, "https://m.xsnvshen.com/", r);
        }
        value
    });
    port::write_result(op, result)
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
koma_source_sdk::koma_source_exports!("xsnvshen", typed_chapters, write_chapter_failure);
