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
    host,
    json_utils::request_page_number,
    source::{SourceCapabilities, SourceInfo},
    source_protocol,
};
use serde_json::{json, Value};
use url::Url;

#[path = "../../port_support.rs"]
mod port_support;
use port_support::{self as port, Node, Result};

const BASE: &str = "https://www.90mh.cc";
const PHONE: &str = "https://m.90mh.cc";
const CHAPTER_CONTEXT: &[u8] = b"source_detail_chapter_context_v1";
// The 1.5.5 WASM request heap is 256 KiB. Leave room for the request envelope.
const MAX_CONTEXT_DETAIL_BYTES: usize = 240 * 1024;
const SORTS: &[(&str, &str)] = &[
    ("", "跟随栏目"),
    ("post", "按发布时间"),
    ("-post", "按发布时间升序"),
    ("update", "按更新时间"),
    ("-update", "按更新时间升序"),
    ("click", "按点击量"),
    ("-click", "按点击量升序"),
];
const CATEGORY_FIELDS: &[(&str, &str)] = &[
    ("type", "按类型"),
    ("area", "按地区"),
    ("genre", "按剧情"),
    ("audience", "按受众"),
    ("year", "按年份"),
    ("sex", "按性别"),
    ("letter", "按字母"),
    ("status", "按进度"),
];
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.jiulingmanhua.koma",
    name: "90漫画网",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "90漫畫網收錄日本、韓國、國漫、港台及歐美漫畫，題材包含戀愛、校園、冒險與科幻，兼有連載及完結作品。",
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

fn site(url: &Url) -> bool {
    matches!(
        url.host_str(),
        Some("www.90mh.cc" | "m.90mh.cc" | "90mh.cc")
    )
}
fn key(input: &str) -> bool {
    !input.is_empty()
        && input.len() < 180
        && input
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'%'))
}
fn work(raw: &str) -> Result<String> {
    let url = port::absolute(PHONE, raw)?;
    let slug = url
        .path()
        .strip_prefix("/manhua/")
        .ok_or("invalid_request")?
        .trim_end_matches('/');
    if !site(&url) || !key(slug) {
        return Err("invalid_request");
    }
    Ok(format!("/manhua/{slug}/"))
}
fn chapter(raw: &str) -> Result<Url> {
    let url = port::absolute(PHONE, raw)?;
    let (owner, filename) = url.path().rsplit_once('/').ok_or("invalid_request")?;
    work(owner)?;
    let page = filename.strip_suffix(".html").ok_or("invalid_request")?;
    if !site(&url)
        || !page
            .split('-')
            .take(3)
            .all(|number| !number.is_empty() && number.bytes().all(|ch| ch.is_ascii_digit()))
        || page.matches('-').count() > 1
    {
        return Err("invalid_request");
    }
    Ok(url)
}
fn doc(url: &str) -> Result<Node> {
    if host::check_cancel() {
        return Err("cancelled");
    }
    let mut headers = port::mobile_image_headers(BASE);
    headers["Accept"] = json!("text/html");
    // The site's HTML has taken over 50 seconds to respond in ordinary reads.
    // Use the host's 60-second ceiling here so other sources keep their own timing.
    let request = serde_json::to_vec(&json!({
        "version": 1,
        "method": "GET",
        "url": url,
        "headers": headers,
        "responseKind": "bodyText",
        "timeoutMs": 60_000,
    }))
    .map_err(|_| "internal_error")?;
    let received = host::http_request(&request, http_out()).map_err(|_| "network_error")?;
    let body_len = source_protocol::body(&http_out()[..received], body_buf())?;
    let html = core::str::from_utf8(&body_buf()[..body_len]).map_err(|_| "parse_error")?;
    Node::parse(html)
}
fn item(id: &str, title: &str) -> Value {
    json!({"id":id,"title":title,"links":[{"kind":"source","url":format!("{PHONE}{id}")}]})
}
fn tag_values(doc: &Node, label: &str) -> Result<Vec<String>> {
    for metadata in doc.all("dl.pic_zi")? {
        let Some(title) = metadata.one("dt") else {
            continue;
        };
        if title.text()?.trim_end_matches([':', '：']) != label {
            continue;
        }
        let value = metadata.require("dd")?;
        let links = value.all("a")?;
        return if links.is_empty() {
            Ok(vec![value.text()?])
        } else {
            links
                .iter()
                .map(|link| Ok(link.text()?.trim_start_matches('#').to_string()))
                .collect()
        };
    }
    Ok(Vec::new())
}
fn details(id: &str) -> Result<(Value, Node)> {
    let url = format!("{PHONE}{id}");
    let doc = doc(&url)?;
    let title = doc.require(".view-sub h1.title")?.text()?;
    let mut row = item(id, &title);
    if let Some(cover) = doc.one("#Cover mip-img[src]") {
        row["cover"] =
            json!({"kind":"url","url":port::absolute(&url,&cover.attr("src")?)?.as_str()});
    }
    row["authors"] = json!(tag_values(&doc, "作者")?);
    let mut tags = tag_values(&doc, "主题")?;
    tags.extend(tag_values(&doc, "类别")?);
    tags.extend(tag_values(&doc, "地区")?);
    row["tags"] = json!(tags);
    row["status"] = json!(
        match tag_values(&doc, "状态")?.first().map(String::as_str) {
            Some("连载中") => "ongoing",
            Some("已完结") => "completed",
            _ => "unknown",
        }
    );
    let mut description = doc
        .one("p.txtDesc")
        .map(|n| n.text())
        .transpose()?
        .unwrap_or_default();
    description = description.trim_start_matches("简介：").trim().to_string();
    let aliases: Vec<_> = tag_values(&doc, "别名")?
        .into_iter()
        .filter(|alias| !alias.is_empty() && alias != &title)
        .collect();
    if !aliases.is_empty() {
        description.push_str(&format!("\n\n别名：{}", aliases.join("、")));
    }
    row["description"] = json!(description);
    Ok((row, doc))
}

fn manga_data(id: &str, request: &[u8]) -> Result<Value> {
    let (row, doc) = details(id)?;
    let mut data = serde_json::Map::new();
    data.insert("manga".to_string(), row);
    // The same mobile HTML contains details and the entire catalogue.
    if source_protocol::feature(request, CHAPTER_CONTEXT) {
        if let Some(context) = ChapterCatalogue::parse(&doc)
            .ok()
            .and_then(|catalogue| catalogue.context(id))
        {
            data.insert("chapterContext".to_string(), context);
            if !serde_json::to_vec(&data)
                .is_ok_and(|encoded| encoded.len() <= MAX_CONTEXT_DETAIL_BYTES)
            {
                data.remove("chapterContext");
            }
        }
    }
    Ok(Value::Object(data))
}
fn browse(args: &Value, request: &[u8], search: bool) -> Result<Value> {
    let page = request_page_number(request);
    let query = args["query"].as_str().unwrap_or("").trim();
    let url = if search && !query.is_empty() {
        if let Ok(url) = Url::parse(query) {
            if matches!(url.scheme(), "https" | "http") {
                let id = if let Ok(work_id) = work(url.as_str()) {
                    work_id
                } else {
                    let page = chapter(url.as_str())?;
                    work(page.path().rsplit_once('/').ok_or("not_found")?.0)?
                };
                return Ok(port::items(vec![details(&id)?.0], None));
            }
        }
        let mut url = port::absolute(BASE, "/search/")?;
        url.query_pairs_mut().append_pair("keywords", query);
        url
    } else {
        let mut sort = port::filter(args, "sort", SORTS)?;
        if sort.is_empty() {
            sort = if args["listingId"] == "latest" {
                "update"
            } else {
                "click"
            };
        }
        let mut categories = String::new();
        for (id, _) in CATEGORY_FIELDS {
            if let Some(value) = args["filters"][id]
                .as_str()
                .filter(|value| !value.is_empty())
            {
                if !key(value) || value.contains(['%', '-']) {
                    return Err("invalid_request");
                }
                categories.push_str(value);
                categories.push('-');
            }
        }
        let path = if categories.is_empty() {
            format!("/list/{sort}/")
        } else {
            format!("/list/{categories}/{sort}/")
        };
        port::absolute(BASE, &path)?
    };
    let mut url = url;
    url.query_pairs_mut().append_pair("page", &page.to_string());
    let doc = doc(url.as_str())?;
    let mut rows = Vec::new();
    let mut seen = BTreeSet::new();
    for card in doc.all("#contList > li, li.list-comic")? {
        let Some(link) = card.one("p > a[href], h3 > a[href], a.txtA[href]") else {
            continue;
        };
        let Ok(id) = work(&link.attr("href")?) else {
            continue;
        };
        if !seen.insert(id.clone()) {
            continue;
        }
        let mut row = item(&id, &link.text()?);
        if let Some(cover) = card.one("img[src], mip-img[src], img[data-src]") {
            let raw = cover
                .attr("src")
                .ok()
                .filter(|src| !src.is_empty())
                .or_else(|| cover.attr("data-src").ok());
            if let Some(raw) = raw {
                row["cover"] =
                    json!({"kind":"url","url":port::absolute(url.as_str(),&raw)?.as_str()});
            }
        }
        rows.push(row);
    }
    let next = if doc.one("ul.pagination > li.next:not(.disabled)").is_some() {
        page.checked_add(1)
    } else {
        None
    };
    Ok(port::items(rows, next))
}
fn filters() -> Result<Value> {
    let doc = doc(&format!("{BASE}/list/"))?;
    let mut filters = Vec::new();
    let mut seen = BTreeSet::new();
    for group in doc.all(".filter-nav .filter-item")? {
        let name = group.require("label")?.text()?;
        let Some((id, _)) = CATEGORY_FIELDS.iter().find(|(_, label)| *label == name) else {
            continue;
        };
        if !seen.insert(*id) {
            continue;
        }
        let mut options = Vec::new();
        for link in group.all("a[href]")? {
            let url = port::absolute(BASE, &link.attr("href")?)?;
            let value = url
                .path()
                .strip_prefix("/list/")
                .ok_or("parse_error")?
                .trim_matches('/');
            options.push(json!({"value":value,"label":link.text()?}));
        }
        filters.push(json!({"type":"select","scope":"browse","id":id,"name":name,"default":"","options":options}));
    }
    filters.push(port::select("sort", "排序", "browse", SORTS));
    Ok(json!({"filters":filters}))
}
struct CatalogueChapter {
    id: String,
    title: String,
    group: String,
    position: u64,
}

struct ChapterCatalogue {
    entries: Vec<CatalogueChapter>,
    updated_at: Option<String>,
}

impl ChapterCatalogue {
    fn parse(doc: &Node) -> Result<Self> {
        let groups = doc.all("ul.Drama")?;
        let newest_group = groups.len().saturating_sub(1);
        let mut updated = tag_values(doc, "更新于")?.into_iter().next();
        let mut updated_at = None;
        let mut entries = Vec::new();
        let mut seen = BTreeSet::new();
        for (group_index, group) in groups.iter().enumerate().rev() {
            let key = group.attr("id")?;
            let links = group.all("li > a[href]")?;
            for (position, link) in links.iter().enumerate().rev() {
                let target = chapter(&link.attr("href")?)?;
                if !seen.insert(target.path().to_string()) {
                    continue;
                }
                let title = if let Some(label) = link.one("span") {
                    label.text()?
                } else {
                    link.text()?
                };
                if group_index == newest_group && position + 1 == links.len() {
                    updated_at = updated.take();
                }
                entries.push(CatalogueChapter {
                    id: target.path().to_string(),
                    title,
                    group: key.clone(),
                    position: position as u64,
                });
            }
        }
        Ok(Self {
            entries,
            updated_at,
        })
    }

    fn context(&self, owner: &str) -> Option<Value> {
        if self.entries.is_empty()
            || self
                .entries
                .iter()
                .any(|entry| entry.group.trim().is_empty() || !entry.id.starts_with(owner))
        {
            return None;
        }
        let entries: Vec<_> = self
            .entries
            .iter()
            .map(|entry| json!([entry.id, entry.title, entry.group, entry.position]))
            .collect();
        Some(json!({
            "version":1, "sourceVersion":SOURCE_INFO.version, "mangaId":owner,
            "count":entries.len(), "entries":entries, "updatedAt":self.updated_at,
        }))
    }

    fn from_context(value: &Value, owner: &str) -> Option<Self> {
        if value["version"].as_u64()? != 1
            || value["sourceVersion"].as_str()? != SOURCE_INFO.version
            || value["mangaId"].as_str()? != owner
        {
            return None;
        }
        let raw_entries = value["entries"].as_array()?;
        if raw_entries.is_empty() || value["count"].as_u64()? != raw_entries.len() as u64 {
            return None;
        }
        let updated_at = match value.get("updatedAt")? {
            Value::Null => None,
            date => Some(date.as_str()?.to_string()),
        };
        let mut entries = Vec::new();
        let mut seen = BTreeSet::new();
        // Validate every entry before emitting a chapter. An invalid snapshot
        // takes the entire original network path.
        for entry in raw_entries {
            let [id, title, group, position] = entry.as_array()?.as_slice() else {
                return None;
            };
            let (id, title, group, position) = (
                id.as_str()?,
                title.as_str()?,
                group.as_str()?,
                position.as_u64()?,
            );
            if !id.starts_with(owner)
                || chapter(id).ok()?.path() != id
                || !seen.insert(id)
                || group.trim().is_empty()
                || position > u32::MAX as u64
            {
                return None;
            }
            entries.push(CatalogueChapter {
                id: id.to_string(),
                title: title.to_string(),
                group: group.to_string(),
                position,
            });
        }
        Some(Self {
            entries,
            updated_at,
        })
    }

    fn write(&self, request: &[u8], owner: &str) -> ChapterResult<'static, &'static str> {
        let mut catalogue =
            GroupedChapters::new(payload_buf(), request).map_err(port::chapter_error)?;
        for (index, entry) in self.entries.iter().enumerate() {
            let mut row = Chapter::new(
                entry.id.as_bytes(),
                owner.as_bytes(),
                entry.title.as_bytes(),
            );
            row.index = Some(index);
            if index == 0 {
                row.updated_at = self.updated_at.as_deref().map(str::as_bytes);
            }
            catalogue
                .push(
                    row,
                    ChapterPlacement::InGroup {
                        group: entry.group.as_bytes(),
                        position: entry.position,
                    },
                )
                .map_err(port::chapter_error)?;
        }
        catalogue
            .finish(ChapterPage::Complete)
            .map_err(port::chapter_error)
    }
}

fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(request)?;
    let args = &input["args"];
    let owner = work(args["mangaId"].as_str().ok_or("invalid_request")?)?;
    if host::check_cancel() {
        return Err("cancelled");
    }
    if args.get("chapterId").is_none() && source_protocol::feature(request, CHAPTER_CONTEXT) {
        if let Some(catalogue) = ChapterCatalogue::from_context(&args["chapterContext"], &owner) {
            return catalogue.write(request, &owner);
        }
    }
    let doc = doc(&format!("{PHONE}{owner}"))?;
    doc.require(".view-sub h1.title")?;
    ChapterCatalogue::parse(&doc)?.write(request, &owner)
}
fn page_image(doc: &Node, site: &Url) -> Result<Url> {
    let image = doc.require("#chapter-image img[src]")?;
    port::absolute(site.as_str(), &image.attr("src")?)
}
fn pages(args: &Value) -> Result<Value> {
    let original = chapter(args["chapterId"].as_str().ok_or("invalid_request")?)?;
    let document = doc(original.as_str())?;
    let count = document
        .require("#k_total")?
        .text()?
        .parse::<usize>()
        .map_err(|_| "parse_error")?;
    if count == 0 || count > 1500 {
        return Err("chapter_unavailable");
    }
    let mut first = port::page(0, original.path(), page_image(&document, &original)?);
    first["image"]["headers"] = port::mobile_image_headers(original.as_str());
    let mut output = vec![first];
    let stem = original.path().strip_suffix(".html").ok_or("parse_error")?;
    for page in 2..=count {
        let locator = port::absolute(PHONE, &format!("{stem}-{page}.html"))?;
        output.push(port::page(page - 1, original.path(), locator));
    }
    Ok(json!({"pages":output}))
}
fn resolve_image(args: &Value) -> Result<Value> {
    let image = port::absolute(PHONE, args["url"].as_str().ok_or("invalid_request")?)?;
    if site(&image) && image.path().ends_with(".html") {
        let target = chapter(image.as_str())?;
        let url = page_image(&doc(target.as_str())?, &target)?;
        return Ok(
            json!({"url":url.as_str(),"headers":port::mobile_image_headers(target.as_str())}),
        );
    }
    Ok(json!({"url":image.as_str(),"headers":port::mobile_image_headers(PHONE)}))
}
fn dispatch(operation: &str, request: &[u8]) -> Result<Value> {
    let input = port::input(request)?;
    let args = &input["args"];
    match operation {
        "get_listings" => Ok(port::listing_list(true, true, true)),
        "get_filters" => filters(),
        "get_manga_list" => browse(args, request, false),
        "search" => browse(args, request, true),
        "get_manga" => manga_data(
            &work(args["mangaId"].as_str().ok_or("invalid_request")?)?,
            request,
        ),
        "get_pages" => pages(args),
        "get_image_request" => resolve_image(args),
        _ => Err("not_implemented"),
    }
}
fn run(operation: &str, request: &[u8]) -> u32 {
    let result = dispatch(operation, request).map(|mut payload| {
        port::mobile_cover_headers(&mut payload, PHONE, request);
        payload
    });
    port::write_result(operation, result)
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
koma_source_sdk::koma_source_exports!("jiulingmanhua", typed_chapters, write_chapter_failure);
