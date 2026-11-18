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
    chapters::{Chapter, ChapterPage, ChapterPlacement, ChapterResult, GroupedChapters},
    json_utils::request_page_number,
    source::{SourceCapabilities, SourceInfo},
};
use serde_json::{json, Value};
use url::{form_urlencoded, Url};
#[path = "../../javascript_packer.rs"]
mod javascript_packer;
#[path = "../../port_support.rs"]
mod port_support;
use port_support::{self as port, Node, Result};

const BASE: &str = "https://www.manben.com";
const SORT: &[(&str, &str)] = &[("", "跟随栏目"), ("4", "热门人气"), ("2", "更新时间")];
const STATUS: &[(&str, &str)] = &[("", "全部"), ("1", "连载"), ("2", "完结")];
const GENRE: &[(&str, &str)] = &[
    ("", "全部"),
    ("37", "搞笑"),
    ("14", "奇幻"),
    ("31", "热血"),
    ("17", "悬疑"),
    ("28", "格斗"),
    ("25", "科幻"),
    ("10", "励志"),
    ("1", "校园"),
    ("26", "爱情"),
    ("7", "日常"),
    ("12", "战争"),
    ("3", "其他"),
];
const READER: &[(&str, &str)] = &[
    ("", "全部"),
    ("1", "少年向"),
    ("2", "少女向"),
    ("3", "青年向"),
];
const AREA: &[(&str, &str)] = &[
    ("", "全部"),
    ("35", "港台"),
    ("36", "日韩"),
    ("37", "大陆"),
    ("52", "欧美"),
];
const FILTERS: &[(&str, &str, &[(&str, &str)])] = &[
    ("sort", "排序", SORT),
    ("status", "状态", STATUS),
    ("genre", "题材", GENRE),
    ("reader", "读者", READER),
    ("area", "地区", AREA),
];
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.manben.koma",
    name: "漫本",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "漫本公开漫画。",
    content_rating: "safe",
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

fn site_path(raw: &str) -> Result<String> {
    let u = port::absolute(BASE, raw)?;
    if !matches!(u.host_str(), Some("www.manben.com" | "manben.com"))
        || u.path() == "/"
        || u.fragment().is_some()
    {
        return Err("invalid_request");
    }
    Ok(u.path().to_string())
}
fn work(raw: &str) -> Result<String> {
    let path = site_path(raw)?;
    if !path.starts_with("/mh-") || path.trim_matches('/').contains('/') || path.len() <= 5 {
        return Err("invalid_request");
    }
    Ok(path)
}
fn entry(path: &str, title: &str) -> Value {
    json!({"id":path,"title":title,"links":[{"kind":"source","url":format!("{BASE}{path}")}]})
}
fn get(path: &str) -> Result<String> {
    // The desktop and ArkWeb responses use a different work and reader template.
    // The mobile pages expose the catalog and embedded page list used by the upstream implementation.
    port::request(
        &format!("{BASE}{path}"),
        "GET",
        port::mobile_image_headers(&format!("{BASE}/")),
        None,
    )
}
fn details(path: &str) -> Result<Value> {
    let html = get(path)?;
    let d = Node::parse(&html)?;
    let info = d.require("div.detailTop")?;
    let mut item = entry(path, &info.require("p.title")?.text()?);
    if let Some(cover) = info.one("img.cover[src]") {
        item["cover"] =
            json!({"kind":"url","url":port::absolute(BASE,&cover.attr("src")?)?.as_str()});
    }
    for field in info.all("p.subtitle")? {
        let field = field.text()?;
        if let Some(value) = field.strip_prefix("作者：") {
            item["authors"] = json!(value
                .split(['、', '/'])
                .map(str::trim)
                .filter(|a| !a.is_empty())
                .collect::<Vec<_>>());
        } else if let Some(value) = field.strip_prefix("类型：") {
            item["tags"] = json!(value
                .split('/')
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .collect::<Vec<_>>());
        }
    }
    if let Some(description) = d.one("div.detailContent > p") {
        item["description"] = json!(description.text()?);
    }
    Ok(item)
}
fn direct_path(raw: &str) -> Result<String> {
    let path = site_path(raw)?;
    if let Ok(id) = work(&path) {
        return Ok(id);
    }
    let html = get(&path)?;
    let reference = html
        .split_once("referUrl = \"")
        .and_then(|(_, text)| text.split_once('"'))
        .ok_or("not_found")?
        .0;
    work(reference)
}
fn listing(req: &[u8], args: &Value, search: bool) -> Result<Value> {
    let page = request_page_number(req);
    let page_no = page.to_string();
    let keyword = args["query"].as_str().unwrap_or("").trim();
    let word_search = search && !keyword.is_empty();
    let (path, referer, params) = if word_search {
        if Url::parse(keyword).is_ok_and(|url| matches!(url.scheme(), "http" | "https")) {
            return Ok(port::items(vec![details(&direct_path(keyword)?)?], None));
        }
        (
            "/pagerdata.ashx",
            "/search",
            vec![
                ("t", "7"),
                ("pageindex", page_no.as_str()),
                ("title", keyword),
                ("f", "0"),
            ],
        )
    } else {
        let selected = port::filter(args, "sort", SORT)?;
        let sort = if selected.is_empty() {
            if args["listingId"] == "latest" {
                "2"
            } else {
                "4"
            }
        } else {
            selected
        };
        let mut pairs = vec![
            ("t", "2"),
            ("pageindex", page_no.as_str()),
            ("sc", "1"),
            ("tsort", sort),
            ("tnm", ""),
        ];
        for (filter, param, options) in [
            ("status", "tst", STATUS),
            ("genre", "tagid", GENRE),
            ("reader", "tgroup", READER),
            ("area", "tarea", AREA),
        ] {
            let choice = port::filter(args, filter, options)?;
            pairs.push((param, if choice.is_empty() { "0" } else { choice }));
        }
        ("/mh-list/pagerdata.ashx", "/mh-list/", pairs)
    };
    let form = form_urlencoded::Serializer::new(String::new())
        .extend_pairs(params)
        .finish();
    let body = port::request(
        &format!("{BASE}{path}"),
        "POST",
        json!({
            "Content-Type":"application/x-www-form-urlencoded","Referer":format!("{BASE}{referer}"),
            "X-Requested-With":"XMLHttpRequest","Accept":"application/json"
        }),
        Some(&form),
    )?;
    let rows: Vec<Value> = serde_json::from_str(&body).map_err(|_| "parse_error")?;
    let mut items = Vec::new();
    for row in &rows {
        let id = work(row["Url"].as_str().ok_or("parse_error")?)?;
        let mut item = entry(&id, row["Title"].as_str().ok_or("parse_error")?);
        if let Some(cover) = row["BigPic"].as_str().filter(|cover| !cover.is_empty()) {
            item["cover"] = json!({"kind":"url","url":port::absolute(BASE,cover)?.as_str()});
        }
        item["authors"] = row["Author"].clone();
        item["tags"] = row["TagList"].clone();
        item["status"] = json!(match row["Status"].as_str() {
            Some("连载中") => "ongoing",
            Some("已完结") => "completed",
            _ => "unknown",
        });
        items.push(item);
    }
    Ok(port::items(
        items,
        if rows.len() >= 10 {
            page.checked_add(1)
        } else {
            None
        },
    ))
}
fn run_get_chapters(req: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(req)?;
    let work_id = work(input["args"]["mangaId"].as_str().ok_or("invalid_request")?)?;
    let d = Node::parse(&get(&work_id)?)?;
    d.require("div.detailTop")?;
    let group_names = d
        .all("ul.detailSelectBar > li")?
        .iter()
        .map(Node::text)
        .collect::<Result<Vec<_>>>()?;
    let mut catalog = GroupedChapters::new(payload_buf(), req).map_err(port::chapter_error)?;
    let mut index = 0;
    let mut seen = BTreeSet::new();
    for (n, group) in d.all("div.chapter.swiper-slide")?.iter().enumerate() {
        let group_id = format!("chapters-{n}");
        let entries = group.all("ul.chapterList > li > a[href]")?;
        for (ordinal, link) in entries.iter().enumerate().rev() {
            let path = site_path(&link.attr("href")?)?;
            if !seen.insert(path.clone()) {
                continue;
            }
            let title = link.text()?;
            let mut chapter = Chapter::new(path.as_bytes(), work_id.as_bytes(), title.as_bytes());
            chapter.index = Some(index);
            chapter.scanlator = group_names.get(n).map(|name| name.as_bytes());
            catalog
                .push(
                    chapter,
                    ChapterPlacement::InGroup {
                        group: group_id.as_bytes(),
                        position: ordinal as u64,
                    },
                )
                .map_err(port::chapter_error)?;
            index += 1;
        }
    }
    catalog
        .finish(ChapterPage::Complete)
        .map_err(port::chapter_error)
}
fn pages(args: &Value) -> Result<Value> {
    let chapter = site_path(args["chapterId"].as_str().ok_or("invalid_request")?)?;
    let url = format!("{BASE}{chapter}");
    let html = get(&chapter)?;
    let packet = html
        .split("</script>")
        .find(|part| part.contains("eval(function") && part.contains("newImgs"))
        .ok_or("parse_error")?;
    let script = javascript_packer::expand(packet)?;
    let raw = script
        .split_once("newImgs=")
        .ok_or("parse_error")?
        .1
        .split(';')
        .next()
        .ok_or("parse_error")?;
    // The mobile reader currently writes escaped single-quoted URL literals.
    // Also accept its JSON array form when quotes are already JSON-compatible.
    let images = match serde_json::from_str::<Vec<String>>(raw) {
        Ok(images) => images,
        Err(_) => {
            let contents = raw
                .strip_prefix(r"['")
                .and_then(|value| value.strip_suffix(r"']"))
                .or_else(|| {
                    raw.strip_prefix(r"[\'")
                        .and_then(|value| value.strip_suffix(r"\']"))
                })
                .ok_or("parse_error")?;
            let delimiter = if raw.starts_with(r"[\'") {
                r"\',\'"
            } else {
                "','"
            };
            contents.split(delimiter).map(ToString::to_string).collect()
        }
    };
    if images.is_empty() {
        return Err("chapter_unavailable");
    }
    let pages = images
        .iter()
        .enumerate()
        .map(|(index, path)| Ok(port::page(index, &chapter, port::absolute(&url, path)?)))
        .collect::<Result<Vec<_>>>()?;
    Ok(json!({"pages":pages}))
}
fn page_owner(args: &Value) -> Option<&str> {
    let id = args["pageId"].as_str()?;
    let (owner, index) = id.rsplit_once(':')?;
    index.parse::<usize>().ok()?;
    Some(owner)
}
fn image_request(args: &Value) -> Result<Value> {
    let url = port::absolute(BASE, args["url"].as_str().ok_or("invalid_request")?)?;
    let referer = match page_owner(args).and_then(|id| site_path(id).ok()) {
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
        "get_filters" => Ok(
            json!({"filters":FILTERS.iter().map(|(key,label,values)|port::select(key,label,"browse",values)).collect::<Vec<_>>()}),
        ),
        "get_manga_list" => listing(req, args, false),
        "search" => listing(req, args, true),
        "get_manga" => Ok(
            json!({"manga":details(&work(args["mangaId"].as_str().ok_or("invalid_request")?)?)?}),
        ),
        "get_pages" => pages(args),
        "get_image_request" => image_request(args),
        _ => Err("not_implemented"),
    }
}
fn run(op: &str, r: &[u8]) -> u32 {
    port::write_result(op, dispatch(op, r))
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
koma_source_sdk::koma_source_exports!("manben", typed_chapters, write_chapter_failure);
