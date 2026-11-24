#![no_std]
extern crate alloc;

use alloc::{
    collections::BTreeSet,
    format,
    string::{String, ToString},
    vec::Vec,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use koma_source_sdk::{
    chapters::{Chapter, ChapterOrder, ChapterResponse, ChapterResult},
    host,
    json_utils::request_page_number,
    source::{SourceCapabilities, SourceInfo},
    source_protocol,
};
use serde_json::{json, Value};
use url::{Position, Url};

#[allow(dead_code)]
#[path = "../../port_support.rs"]
mod port_support;
use port_support::{self as port, Node, Result};

const MIRRORS: &[&str] = &[
    "https://manwa.me",
    "https://manwass.cc",
    "https://manwatg.cc",
    "https://manwast.cc",
    "https://manwasy.cc",
];
const REDIRECT_URL: &str = "https://fuwt.cc/mw666";
const UPDATE_PAGE_SIZE: usize = 15;
const ENCODED_QUERY: &str = "v=20220724";
const ALL_TAGS: &str = "__all__";
// All three archived chapter documents present these same public query choices.
const IMAGE_HOSTS: &[(&str, &str)] = &[
    ("", "原站默认"),
    ("?img_host=0", "随机"),
    ("?img_host=1", "图源1"),
    ("?img_host=2", "图源2"),
    ("?img_host=3", "图源3"),
];
const STATES: &[(&str, &str)] = &[("all", "全部"), ("2", "连载中"), ("1", "完结")];
const GENDERS: &[(&str, &str)] = &[
    ("-1", "全部"),
    ("2", "一般向"),
    ("0", "BL向"),
    ("1", "禁漫"),
    ("3", "TL向"),
    ("4", "GL向"),
];
const AREAS: &[(&str, &str)] = &[
    ("all", "全部"),
    ("2", "韩国"),
    ("3", "日漫"),
    ("4", "国漫"),
    ("5", "台漫"),
    ("6", "其他"),
    ("1", "未分类"),
];
const SORTS: &[(&str, &str)] = &[("-1", "最新"), ("0", "最旧"), ("1", "收藏"), ("2", "新漫")];
const FILTERS: &[(&str, &str, &[(&str, &str)])] = &[
    ("end", "状态", STATES),
    ("gender", "类型", GENDERS),
    ("area", "地区", AREAS),
    ("sort", "排序", SORTS),
];

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.manwa.koma",
    name: "漫蛙",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "manwa.me 的公开目录和章节。",
    content_rating: "unknown",
};
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    listings: true,
    manga_list: true,
    filters: true,
    settings: true,
    image_request: true,
    ..SourceCapabilities::CORE
};
#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;
koma_source_sdk::koma_source_buffers! {
    payload: 1024 * 1024, http_out: 5 * 1024 * 1024, body: 3 * 1024 * 1024,
    http_req: 8192, scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

#[derive(Clone, Copy)]
enum Fetch {
    Document,
    Xhr,
    Image,
}
impl Fetch {
    fn headers(self) -> Value {
        match self {
            Self::Document => {
                json!({"Accept":"text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"})
            }
            Self::Xhr => {
                json!({"Accept":"application/json,*/*;q=0.8","X-Requested-With":"XMLHttpRequest"})
            }
            Self::Image => json!({"Accept":"image/*,*/*;q=0.8"}),
        }
    }
}

struct Site<'a> {
    input: &'a Value,
    base: Url,
}
impl<'a> Site<'a> {
    fn new(input: &'a Value) -> Result<Self> {
        let saved = &input["settings"];
        let custom = saved["customUrl"].as_str().unwrap_or("").trim();
        let selected = saved["mirror"].as_str().unwrap_or(MIRRORS[0]);
        let base = if !custom.is_empty() {
            origin_url(custom)?
        } else if MIRRORS.contains(&selected) {
            origin_url(selected)?
        } else {
            return Err("invalid_request");
        };
        Ok(Self { input, base })
    }

    fn url(&self, path: &str) -> Result<Url> {
        let url = port::absolute(self.base.as_str(), path)?;
        if url.origin() != self.base.origin() || url.fragment().is_some() {
            return Err("invalid_request");
        }
        Ok(url)
    }

    fn get(&self, url: &Url, kind: Fetch) -> Result<String> {
        port::request(url.as_str(), "GET", kind.headers(), None)
    }

    fn document(&self, url: &Url) -> Result<Node> {
        Node::parse(&self.get(url, Fetch::Document)?)
    }

    fn image_route(&self, mut url: Url) -> Result<Url> {
        let suffix = self.input["settings"]["imageHost"]
            .as_str()
            .unwrap_or("")
            .trim();
        if suffix.is_empty() {
            return Ok(url);
        }
        // The upstream choices are query suffixes on this same chapter/update document.
        let query = suffix.strip_prefix('?').ok_or("invalid_request")?;
        if query.is_empty() || query.contains('#') || url.query().is_some() {
            return Err("invalid_request");
        }
        url.set_query(Some(query));
        Ok(url)
    }

    fn load_mirrors(&self) -> Result<Vec<String>> {
        let override_url = self.input["settings"]["redirectUrl"]
            .as_str()
            .unwrap_or("")
            .trim();
        let target = if override_url.is_empty() {
            REDIRECT_URL
        } else {
            override_url
        };
        let url = port::absolute(REDIRECT_URL, target)?;
        if url.scheme() != "https" || url.fragment().is_some() {
            return Err("invalid_request");
        }
        let html = self.get(&url, Fetch::Document)?;
        let (_, tail) = html
            .split_once("var lks = JSON.parse(atob(")
            .ok_or("parse_error")?;
        let quote = tail
            .chars()
            .next()
            .filter(|c| matches!(c, '\'' | '"'))
            .ok_or("parse_error")?;
        let (encoded, _) = tail[1..].split_once(quote).ok_or("parse_error")?;
        let bytes = STANDARD.decode(encoded).map_err(|_| "parse_error")?;
        let links: Vec<String> = serde_json::from_slice(&bytes).map_err(|_| "parse_error")?;
        let mut seen = BTreeSet::new();
        Ok(links
            .iter()
            .filter_map(|link| origin_url(link).ok())
            .map(|url| url.origin().ascii_serialization())
            .filter(|url| seen.insert(url.clone()))
            .take(5)
            .collect())
    }
}

fn origin_url(text: &str) -> Result<Url> {
    let url = Url::parse(text).map_err(|_| "invalid_request")?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("invalid_request");
    }
    Ok(url)
}

fn identity(url: &Url) -> &str {
    &url[Position::BeforePath..Position::AfterQuery]
}

fn numeric_work(url: &Url) -> bool {
    url.query().is_none()
        && url
            .path()
            .strip_prefix("/book/")
            .is_some_and(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()))
}

fn work_url(site: &Site<'_>) -> Result<Url> {
    let route = site.input["args"]["mangaId"]
        .as_str()
        .ok_or("invalid_request")?;
    let url = site.url(route)?;
    numeric_work(&url).then_some(url).ok_or("invalid_request")
}

fn card(work: &Url, title: String, image: Option<Url>) -> Result<Value> {
    if title.trim().is_empty() || !numeric_work(work) {
        return Err("parse_error");
    }
    let mut result = json!({"id":identity(work),"title":title,
        "links":[{"kind":"source","url":work.as_str()}]});
    if let Some(image) = image {
        result["cover"] = if encrypted(&image) {
            json!({"kind":"url","url":image.as_str()})
        } else {
            json!({"kind":"url","url":image.as_str(),"headers":{}})
        };
    }
    Ok(result)
}

fn cover(doc: &Node, selector: &str, attribute: &str, referrer: &Url) -> Result<Option<Url>> {
    let Some(img) = doc.one(selector) else {
        return Ok(None);
    };
    let raw = img.attr(attribute).unwrap_or_default();
    if raw.is_empty() {
        return Ok(None);
    }
    Ok(Some(port::absolute(referrer.as_str(), &raw)?))
}

fn row_work(site: &Site<'_>, reference: &Url, link: &Node) -> Result<Url> {
    let work = port::absolute(reference.as_str(), &link.attr("href")?)?;
    if work.origin() != site.base.origin() {
        return Err("parse_error");
    }
    Ok(work)
}

fn popular(site: &Site<'_>) -> Result<Value> {
    let url = site.url("/rank")?;
    let doc = site.document(&url)?;
    doc.require("#rankList_2")?;
    let items = doc
        .all("#rankList_2 > a")?
        .iter()
        .map(|row| {
            let work = row_work(site, &url, row)?;
            card(
                &work,
                row.attr("title")?,
                cover(row, "img", "data-original", &url)?,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(port::items(items, None))
}

fn latest(site: &Site<'_>, page: usize) -> Result<Value> {
    let offset = (page - 1)
        .checked_mul(UPDATE_PAGE_SIZE)
        .ok_or("invalid_request")?;
    let url = site.url(&format!("/getUpdate?page={offset}&date="))?;
    let raw = site.get(&url, Fetch::Xhr)?;
    let data: Value = serde_json::from_str(&raw).map_err(|_| "parse_error")?;
    let empty = Vec::new();
    let books = match data.get("books") {
        Some(books) => books.as_array().ok_or("parse_error")?,
        None if data.is_object() => &empty,
        None => return Err("parse_error"),
    };
    let total = match data.get("total") {
        Some(total) => total.as_u64().ok_or("parse_error")?,
        None => 0,
    };
    let update = site.image_route(site.url("/update")?)?;
    let doc = site.document(&update)?;
    let prefix = doc
        .one(".manga-list-2-cover-img")
        .and_then(|img| img.attr(":src").ok())
        .unwrap_or_default();
    let host = prefix
        .strip_prefix('\'')
        .and_then(|s| s.split_once('\''))
        .map(|(host, _)| host)
        .unwrap_or_default();
    let items = books
        .iter()
        .map(|book| {
            let id = book["id"].as_u64().ok_or("parse_error")?;
            let work = site.url(&format!("/book/{id}"))?;
            let title = book["book_name"].as_str().ok_or("parse_error")?.to_string();
            let path = book["cover_url"].as_str().ok_or("parse_error")?;
            let image = port::absolute(update.as_str(), &format!("{host}{path}"))?;
            card(&work, title, Some(image))
        })
        .collect::<Result<Vec<_>>>()?;
    let end = offset
        .checked_add(UPDATE_PAGE_SIZE)
        .ok_or("invalid_request")? as u64;
    let next = (total > end).then(|| page + 1);
    Ok(port::items(items, next))
}

fn search(site: &Site<'_>, page: usize) -> Result<Value> {
    let args = &site.input["args"];
    let query = args["query"].as_str().ok_or("invalid_request")?;
    // The reference routes hyphenated queries to the same unkeyed taxonomy form as blank queries.
    let booklist = query.is_empty() || query.contains('-');
    let mut url = site.url(if booklist { "/booklist" } else { "/search" })?;
    {
        let mut params = url.query_pairs_mut();
        if booklist {
            for (key, _, options) in FILTERS {
                let value = args["filters"].get(key);
                let value = if value.is_some() {
                    port::filter(args, key, options)?
                } else {
                    options[0].0
                };
                params.append_pair(key, if value == "all" { "" } else { value });
            }
            let selected = &args["filters"]["tags"];
            if let Some(value) = selected.as_array() {
                let tags = value
                    .iter()
                    .map(|tag| tag.as_str().ok_or("invalid_request"))
                    .collect::<Result<Vec<_>>>()?;
                if !tags.contains(&ALL_TAGS) {
                    params.append_pair("tag", &tags.join(", "));
                }
            } else if selected.is_null() {
                params.append_pair("tag", "");
            } else {
                return Err("invalid_request");
            }
        } else {
            params.append_pair("keyword", query);
        }
        if page > 1 {
            params.append_pair("page", &page.to_string());
        }
    }
    let doc = site.document(&url)?;
    let (selector, heading, image_attr) = if booklist {
        ("ul.manga-list-2 > li", "p.manga-list-2-title", "src")
    } else {
        (
            "ul.book-list > li",
            "p.book-list-info-title",
            "data-original",
        )
    };
    let items = doc
        .all(selector)?
        .iter()
        .map(|row| {
            let work = row_work(site, &url, &row.require("a")?)?;
            card(
                &work,
                row.require(heading)?.text()?,
                cover(row, "img", image_attr, &url)?,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let next = doc
        .all("ul.pagination2 > li")?
        .last()
        .map(Node::text)
        .transpose()?
        .filter(|label| label == "下一页")
        .and_then(|_| page.checked_add(1));
    Ok(port::items(items, next))
}

fn details(site: &Site<'_>) -> Result<Value> {
    let url = work_url(site)?;
    let doc = site.document(&url)?;
    let title = doc.require(".detail-main-info-title")?.text()?;
    let mut work = card(
        &url,
        title,
        cover(&doc, "div.detail-main-cover > img", "data-original", &url)?,
    )?;
    let authors = doc
        .all("p.detail-main-info-author > span.detail-main-info-value > a")?
        .iter()
        .map(Node::text)
        .collect::<Result<Vec<_>>>()?;
    if !authors.is_empty() {
        work["authors"] = json!(authors);
        work["artists"] = json!(authors);
    }
    for field in doc.all("p.detail-main-info-author")? {
        if field.text()?.contains("更新状态") {
            if let Some(value) = field.one("span.detail-main-info-value") {
                work["status"] = match value.text()?.as_str() {
                    "连载中" => json!("ongoing"),
                    "已完结" => json!("completed"),
                    _ => json!("unknown"),
                };
            }
            break;
        }
    }
    let tags = doc
        .all("div.detail-main-info-class > a.info-tag")?
        .iter()
        .map(Node::text)
        .collect::<Result<Vec<_>>>()?;
    work["tags"] = json!(tags);
    if let Some(description) = doc.one("#detail > p.detail-desc") {
        work["description"] = json!(description.text()?);
    }
    Ok(json!({"manga":work}))
}

fn catalog_date(doc: &Node) -> Option<String> {
    let label = doc.one(".detail-list-title-3")?.text().ok()?;
    let mut parts = label.split("更新").next()?.trim().split('-');
    let year: u16 = parts.next()?.parse().ok()?;
    let month: u8 = parts.next()?.parse().ok()?;
    let day: u8 = parts.next()?.parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || parts.next().is_some() {
        return None;
    }
    Some(format!("{year:04}-{month:02}-{day:02}T00:00:00"))
}

fn run_get_chapters(request: &[u8]) -> ChapterResult<'_, &'static str> {
    let input = port::input(request)?;
    let site = Site::new(&input)?;
    let work = work_url(&site)?;
    let doc = site.document(&work)?;
    doc.require(".detail-main-info-title")?;
    let updated = catalog_date(&doc);
    let rows = doc
        .all("ul#detail-list-select > li > a")?
        .iter()
        .rev()
        .map(|row| {
            let chapter = site.url(&row.attr("href")?)?;
            if chapter.path() == "/" || numeric_work(&chapter) {
                return Err("parse_error");
            }
            Ok((identity(&chapter).to_string(), row.text()?))
        })
        .collect::<Result<Vec<_>>>()?;
    let chapters = rows
        .iter()
        .enumerate()
        .map(|(index, (id, name))| {
            let mut chapter =
                Chapter::new(id.as_bytes(), identity(&work).as_bytes(), name.as_bytes());
            chapter.language = Some(b"zh");
            // This timestamp describes the work, and the reference only assigns it to the newest chapter.
            if index == 0 {
                chapter.updated_at = updated.as_deref().map(str::as_bytes);
            }
            chapter
        })
        .collect::<Vec<_>>();
    ChapterResponse::ordered(payload_buf(), request, ChapterOrder::NewestFirst, &chapters)
        .map_err(port::chapter_error)
}

fn pages(site: &Site<'_>) -> Result<Value> {
    let route = site.input["args"]["chapterId"]
        .as_str()
        .ok_or("invalid_request")?;
    let chapter = site.url(route)?;
    if chapter.path() == "/" || numeric_work(&chapter) {
        return Err("invalid_request");
    }
    // Best-effort pixel sent to the selected site before the chapter, as in the reference.
    let _ = site.get(&site.url("/static/images/pv.gif")?, Fetch::Image);
    let url = site.image_route(chapter.clone())?;
    let doc = site.document(&url)?;
    let pages = doc
        .all("#cp_img > div.img-content > img[data-r-src]")?
        .iter()
        .enumerate()
        .map(|(index, img)| {
            Ok(port::page(
                index,
                identity(&chapter),
                port::absolute(url.as_str(), &img.attr("data-r-src")?)?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    if pages.is_empty() {
        return Err("chapter_unavailable");
    }
    Ok(json!({"pages":pages}))
}

fn encrypted(url: &Url) -> bool {
    url.fragment().is_none() && url.query() == Some(ENCODED_QUERY)
}

fn image_request(input: &Value, request: &[u8]) -> Result<Value> {
    let raw = input["args"]["url"].as_str().ok_or("invalid_request")?;
    let url = match Url::parse(raw) {
        Ok(_) => port::absolute(MIRRORS[0], raw)?,
        Err(_) => port::absolute(Site::new(input)?.base.as_str(), raw)?,
    };
    let mut image = json!({"url":url.as_str(),"headers":{}});
    if encrypted(&url) {
        if !source_protocol::feature(request, b"source_required_image_decode_wasm_v1") {
            return Err("unsupported_feature");
        }
        image["requiredImageDecode"] = json!({"kind":"wasm_image_v1","bytes":"manwa-cbc"});
    }
    Ok(image)
}

fn filters(site: &Site<'_>, request: &[u8]) -> Result<Value> {
    let mut fields = FILTERS
        .iter()
        .map(|(key, name, options)| {
            let mut filter = port::select(key, name, "search", options);
            filter["default"] = json!(options[0].0);
            filter
        })
        .collect::<Vec<_>>();
    let fetch = source_protocol::feature(request, b"source_filter_fetching_v1");
    if fetch {
        let mut field =
            json!({"type":"multiselect","id":"tags","name":"标签","scope":"search","default":[]});
        if site.input["args"]["fetchFilterData"] == true {
            let doc = site.document(&site.url("/booklist")?)?;
            let mut tags = Vec::new();
            let mut seen = BTreeSet::new();
            for option in doc.all("div.manga-filter-row.tags > a")? {
                let label = option.text()?;
                let value = option.attr("data-val").unwrap_or_default();
                if label.is_empty() {
                    continue;
                }
                let value = if tags.is_empty() || value.is_empty() {
                    ALL_TAGS.to_string()
                } else {
                    value
                };
                if seen.insert(value.clone()) {
                    tags.push(json!({"value":value,"label":label}));
                }
            }
            if tags.is_empty() {
                tags.push(json!({"value":ALL_TAGS,"label":"全部"}));
            }
            field["options"] = json!(tags);
        }
        fields.push(field);
    }
    Ok(json!({"filters":fields,"supportsFilterFetching":fetch,
        "useSearchForFilters":source_protocol::feature(request, b"source_filter_search_without_query_v1")}))
}

fn settings() -> Value {
    let mirrors = MIRRORS
        .iter()
        .map(|mirror| json!({"value":mirror,"label":mirror}))
        .collect::<Vec<_>>();
    let image_hosts = IMAGE_HOSTS
        .iter()
        .map(|(value, label)| json!({"value":value,"label":label}))
        .collect::<Vec<_>>();
    json!({"settings":[
        {"id":"mirror","kind":"select","name":"使用镜像网址","default":MIRRORS[0],"options":mirrors},
        {"id":"customUrl","kind":"text","name":"自定义 URL","default":"","description":"HTTPS 站点地址，优先于所选镜像。"},
        {"id":"imageHost","kind":"select","name":"图源","default":"","options":image_hosts},
        {"id":"redirectUrl","kind":"text","name":"重定向 URL","default":"","description":"读取动态目标地址列表。留空使用原上游地址；发现的地址可填入自定义 URL。"}
    ]})
}

fn dispatch(operation: &str, request: &[u8]) -> Result<Value> {
    let input = port::input(request)?;
    match operation {
        "get_listings" => return Ok(port::listing_list(true, false, false)),
        "get_settings" => return Ok(settings()),
        "get_image_request" => return image_request(&input, request),
        _ => {}
    }
    let site = Site::new(&input)?;
    let page = request_page_number(request);
    match operation {
        "get_filters" => filters(&site, request),
        "search" => search(&site, page),
        "get_manga_list" => match input["args"]["listingId"].as_str() {
            Some("popular") => popular(&site),
            Some("latest") => latest(&site, page),
            _ => Err("invalid_request"),
        },
        "get_manga" => details(&site),
        "get_pages" => pages(&site),
        _ => Err("not_implemented"),
    }
}

fn report_failure(operation: &str, request: &[u8], code: &str) -> u32 {
    if !matches!(
        code,
        "network_error" | "forbidden" | "not_found" | "http_error"
    ) || host::check_cancel()
    {
        return port::write_failure(operation, code);
    }
    // The Host's offline settings have no public write-back from a failed operation.
    // Expose discovered addresses in this response without replacing the selected origin.
    let mirrors = port::input(request).and_then(|input| Site::new(&input)?.load_mirrors());
    match mirrors {
        Ok(mirrors) if !mirrors.is_empty() => write_error(
            operation,
            code,
            &format!(
                "镜像地址已更新，可将下列同站镜像之一填入自定义 URL：{}",
                mirrors.join("、")
            ),
        ),
        _ => port::write_failure(operation, code),
    }
}

fn run(operation: &str, request: &[u8]) -> u32 {
    match dispatch(operation, request) {
        Ok(value) => port::write_result(operation, Ok(value)),
        Err(code) => report_failure(operation, request, code),
    }
}
fn run_search(r: &[u8]) -> u32 {
    run("search", r)
}
fn run_get_manga(r: &[u8]) -> u32 {
    run("get_manga", r)
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
fn run_get_image_request(r: &[u8]) -> u32 {
    run("get_image_request", r)
}
fn run_get_settings(r: &[u8]) -> u32 {
    run("get_settings", r)
}
fn run_get_home(r: &[u8]) -> u32 {
    run("get_home", r)
}
fn chapter_failure(code: &str) -> u32 {
    port::write_failure("get_chapters", code)
}
koma_source_sdk::koma_source_exports!("manwa", typed_chapters, chapter_failure);
