#![no_std]
extern crate alloc;

use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use koma_source_sdk::{
    chapters::{Chapter, ChapterOrder, ChapterResponse, ChapterResult},
    json_utils::request_page_number,
    source::{SourceCapabilities, SourceInfo},
};
use serde_json::{json, Value};
use url::Url;

#[path = "../../port_support.rs"]
mod port_support;
mod provider_tags;
use port_support::{self as port, Node, Result};

const WEB: &str = "https://www.favcomic.com";
// Order and default follow the upstream extension's generated selector.
const MIRRORS: &[(&str, &str)] = &[
    (WEB, "www.favcomic.com"),
    ("https://www.favcomic.xyz", "www.favcomic.xyz"),
    ("https://www.favcomic.net", "www.favcomic.net"),
    ("https://www.favcomic.cc", "www.favcomic.cc"),
];
// Public reader key from the pinned upstream extension.
const IMAGE_KEY: &str = "36582b62362e453e627358627d2b3d4e";
const KINDS: &[(&str, &str)] = &[
    ("search", "全部"),
    ("boy", "少男漫画"),
    ("girl", "少女漫画"),
    ("picture", "性感图库"),
    ("r18", "成人漫画"),
];
const ORIGINS: &[(&str, &str)] = &[
    ("0", "全部"),
    ("2", "日本"),
    ("3", "韩国"),
    ("1", "大陆及港台"),
    ("4", "其它"),
];
const FINISHED: &[(&str, &str)] = &[("0", "全部"), ("1", "连载"), ("2", "完结")];
const FREE: &[(&str, &str)] = &[("0", "全部"), ("1", "免费"), ("2", "付费")];
const SORT: &[(&str, &str)] = &[("1", "人气推荐"), ("2", "更新时间")];
const RANK: &[(&str, &str)] = &[("1", "周排名"), ("2", "月排名"), ("3", "总排名")];

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.favcomic.koma",
    name: "喜漫漫画",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "喜漫漫画。",
    content_rating: "mixed",
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
    payload:1024*1024,http_out:4*1024*1024,body:3*1024*1024,http_req:4096,scratch:8192,
}
koma_source_sdk::koma_source_helpers!();

fn is_favcomic(url: &Url) -> bool {
    url.scheme() == "https"
        && url.port().is_none()
        && url.username().is_empty()
        && url.password().is_none()
        && url.host_str().is_some_and(|host| {
            MIRRORS
                .iter()
                .any(|(_, domain)| *domain == host || domain.strip_prefix("www.") == Some(host))
        })
}
// Keep stored work and chapter IDs independent of the selected mirror.
fn route(base: &str, raw: &str) -> Result<String> {
    let url = port::absolute(base, raw)?;
    if !is_favcomic(&url) || url.path() == "/" || url.path().starts_with("//") {
        return Err("not_found");
    }
    Ok(url[url::Position::BeforePath..url::Position::AfterQuery].to_string())
}
fn html(base: &str, url: &Url) -> Result<Node> {
    let text = port::request(
        url.as_str(),
        "GET",
        json!({"Accept":"text/html,application/xhtml+xml","Referer":format!("{base}/")}),
        None,
    )?;
    if text.len() < 4096 && text.contains("window.location.href") && text.contains("/lander") {
        return Err("network_error");
    }
    Node::parse(&text)
}
fn load(base: &str, path: &str) -> Result<Node> {
    html(base, &port::absolute(base, path)?)
}
fn selected<'a>(
    args: &'a Value,
    key: &str,
    options: &[(&str, &str)],
    default: &'a str,
) -> Result<&'a str> {
    let val = port::filter(args, key, options)?;
    Ok(if val.is_empty() { default } else { val })
}
fn setting<'a>(
    input: &'a Value,
    key: &str,
    options: &[(&str, &str)],
    default: &'a str,
) -> Result<&'a str> {
    let value = input["settings"][key].as_str().unwrap_or(default);
    options
        .iter()
        .any(|(id, _)| value == *id)
        .then_some(value)
        .ok_or("invalid_request")
}
fn class(node: &Node, name: &str) -> bool {
    node.attr("class")
        .is_ok_and(|value| value.split_ascii_whitespace().any(|v| v == name))
}
fn mirror(input: &Value) -> Result<&str> {
    setting(input, "preferredMirror", MIRRORS, WEB)
}

fn image_origin(base: &str, mut url: Url) -> Result<Url> {
    if is_favcomic(&url) {
        // Persisted or absolute first-party links follow the selected site.
        // Absolute CDN URLs are supplied by the site's current page.
        let origin = port::absolute(base, "/")?;
        url.set_host(origin.host_str())
            .map_err(|_| "invalid_request")?;
    }
    Ok(url)
}

fn image_url(node: &Node, base: &str, document_url: &str) -> Result<Url> {
    let mut image = image_origin(base, port::absolute(document_url, &node.attr("data-src")?)?)?;
    image.set_fragment(class(node, "encrypted-image").then_some("true"));
    Ok(image)
}
fn image_headers(base: &str) -> Value {
    json!({"Referer":format!("{base}/")})
}

fn cover(node: &Node, base: &str, document_url: &str) -> Result<Value> {
    let url = image_url(node, base, document_url)?;
    if url.fragment().is_some() {
        Ok(json!({"kind":"url","url":url.as_str()}))
    } else {
        Ok(json!({"kind":"url","url":url.as_str(),"headers":image_headers(base)}))
    }
}
fn authors(node: &Node) -> Result<Vec<String>> {
    let Some(author) = node.one(".author") else {
        return Ok(Vec::new());
    };
    let values = author
        .all("a")?
        .iter()
        .map(Node::text)
        .collect::<Result<Vec<_>>>()?;
    if !values.is_empty() {
        return Ok(values);
    }
    let all = author.text()?;
    Ok(if all.is_empty() {
        Vec::new()
    } else {
        vec![all]
    })
}
fn item(base: &str, id: &str, title: &str, cover: Value) -> Result<Value> {
    if title.trim().is_empty() {
        return Err("parse_error");
    }
    Ok(json!({"id":id,"title":title.trim(),"cover":cover,"links":[{
        "kind":"source","url":port::absolute(base,id)?.as_str()
    }]}))
}
fn list(input: &Value, req: &[u8], search: bool) -> Result<Value> {
    let base = mirror(input)?;
    let args = &input["args"];
    let page = request_page_number(req);
    let kind = setting(input, "kind", &KINDS[1..], "boy")?;
    let rank = !search && args["listingId"] != "latest";
    let mut url = if rank {
        let range = setting(input, "rank", RANK, "1")?;
        let group = match kind {
            "boy" => "1",
            "girl" => "2",
            "picture" => "3",
            "r18" => "4",
            _ => return Err("invalid_request"),
        };
        let mut url = port::absolute(base, "/rank")?;
        url.query_pairs_mut()
            .extend_pairs([("range", range), ("comicType", group), ("vip", "0")]);
        url
    } else if search {
        let query = args["query"].as_str().unwrap_or("").trim();
        let kind = selected(args, "kind", KINDS, "search")?;
        let path = if query.is_empty() { kind } else { "search" };
        let mut url = port::absolute(base, path)?;
        url.query_pairs_mut().extend_pairs([
            ("keyword", query),
            ("origin", selected(args, "origin", ORIGINS, "0")?),
            ("finished", selected(args, "finished", FINISHED, "0")?),
            ("free", selected(args, "free", FREE, "0")?),
            ("sort", selected(args, "sort", SORT, "1")?),
        ]);
        if let Some((key, choices)) = provider_tags::for_kind(kind) {
            url.query_pairs_mut()
                .append_pair("tag", selected(args, key, choices, "0")?);
        }
        url
    } else {
        port::absolute(base, kind)?
    };
    if !rank {
        url.query_pairs_mut()
            .append_pair("page", page.to_string().as_str());
    }
    let doc = html(base, &url)?;
    let nodes = doc.all(if rank {
        ".rank_item > a"
    } else {
        ".cover_box > a"
    })?;
    let mut items = Vec::new();
    for link in nodes {
        let id = route(base, &link.attr("href")?)?;
        let image = link.require(if rank { ".cover > img" } else { ".cover" })?;
        let title = if rank {
            image.attr("alt")?
        } else {
            link.attr("title")?
        };
        let mut row = item(base, &id, &title, cover(&image, base, url.as_str())?)?;
        if rank {
            row["authors"] = json!(authors(&link)?);
            row["description"] = json!(link.require(".brief")?.text()?);
        }
        items.push(row);
    }
    let next = if !rank
        && !items.is_empty()
        && !class(
            &doc.require(".pagination_box > .content_box > div:nth-last-child(2) > a")?,
            "active",
        ) {
        page.checked_add(1)
    } else {
        None
    };
    Ok(port::items(items, next))
}
fn manga(input: &Value) -> Result<Value> {
    let base = mirror(input)?;
    let id = route(
        base,
        input["args"]["mangaId"].as_str().ok_or("invalid_request")?,
    )?;
    let url = port::absolute(base, &id)?;
    let doc = html(base, &url)?;
    let image = doc.require(".comic_cover_box > .flex_box > img")?;
    let mut row = item(
        base,
        &id,
        &doc.require(".comic_title")?.text()?,
        cover(&image, base, url.as_str())?,
    )?;
    row["authors"] = json!(authors(&doc)?);
    let intro = doc.require(".intro_box > .txt")?.text()?;
    let mut description = intro
        .split_once("作品介绍：")
        .map_or(intro.as_str(), |(_, body)| body)
        .trim()
        .to_string();
    if let Some(note) = doc.one(".translation_agency_box") {
        description.push_str(&format!("\n\n*译者/机构：{}*", note.text()?));
    }
    row["description"] = json!(description);
    let status = doc
        .one(".state_box > span:nth-of-type(2)")
        .and_then(|node| node.text().ok());
    row["status"] = json!(match status.as_deref() {
        Some("连载中") => "ongoing",
        Some("完结") => "completed",
        _ => "unknown",
    });
    row["tags"] = json!(doc
        .all(".tag_box a")?
        .iter()
        .map(Node::text)
        .collect::<Result<Vec<_>>>()?);
    Ok(json!({"manga":row}))
}
fn run_get_chapters(req: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(req)?;
    let base = mirror(&input)?;
    let id = route(
        base,
        input["args"]["mangaId"].as_str().ok_or("invalid_request")?,
    )?;
    let doc = load(base, &id)?;
    let rows = doc
        .all(".catalog_box a")?
        .iter()
        .rev()
        .map(|a| {
            let chapter = route(base, &a.attr("href")?)?;
            let title = a.require(".title")?.text()?;
            let price = a
                .one("span:last-child")
                .and_then(|node| node.text().ok())
                .and_then(|value| value.parse::<f32>().ok())
                .filter(|price| price.is_finite());
            let name = if price.is_some() {
                format!("🪙 {title}")
            } else {
                title
            };
            Ok((chapter, name, price.map(|value| format!("￥{value}"))))
        })
        .collect::<Result<Vec<_>>>()?;
    let chapters = rows
        .iter()
        .enumerate()
        .map(|(index, (chapter, title, price))| {
            let mut row = Chapter::new(chapter.as_bytes(), id.as_bytes(), title.as_bytes());
            row.index = Some(index);
            row.scanlator = price.as_deref().map(str::as_bytes);
            row
        })
        .collect::<Vec<_>>();
    ChapterResponse::ordered(payload_buf(), req, ChapterOrder::NewestFirst, &chapters)
        .map_err(port::chapter_error)
}
fn pages(input: &Value) -> Result<Value> {
    let base = mirror(input)?;
    let id = route(
        base,
        input["args"]["chapterId"]
            .as_str()
            .ok_or("invalid_request")?,
    )?;
    let url = port::absolute(base, &id)?;
    let doc = html(base, &url)?;
    let code = doc
        .one(".comic_chapter_box")
        .and_then(|node| node.attr("code").ok());
    match code.as_deref() {
        Some("1") => return Err("auth_required"),
        Some("3") => return Err("favcomic_insufficient_balance"),
        Some("4") => return Err("chapter_locked"),
        Some("444") => return Err("favcomic_daily_limit"),
        _ => {}
    }
    let pages = doc
        .all("#content > img")?
        .iter()
        .enumerate()
        .map(|(index, image)| {
            Ok(port::page(
                index,
                &id,
                image_url(image, base, url.as_str())?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    if pages.is_empty() {
        return Err("chapter_unavailable");
    }
    Ok(json!({"pages":pages}))
}
fn image(input: &Value) -> Result<Value> {
    let base = mirror(input)?;
    let raw = input["args"]["url"].as_str().ok_or("invalid_request")?;
    let mut url = image_origin(base, port::absolute(base, raw)?)?;
    let encoded = url.fragment() == Some("true");
    url.set_fragment(None);
    let mut request = json!({"url":url.as_str(),"headers":image_headers(base)});
    if encoded {
        if !input["hostFeatures"].as_array().is_some_and(|v| {
            v.iter()
                .any(|f| f == "source_required_image_decode_wasm_v1")
        }) {
            return Err("unsupported_feature");
        }
        request["requiredImageDecode"] = json!({"kind":"wasm_image_v1","bytes":IMAGE_KEY});
    }
    Ok(request)
}
fn filters() -> Value {
    let mut fields = vec![
        port::select("kind", "类型", "search", KINDS),
        port::select("origin", "地区", "search", ORIGINS),
        port::select("finished", "状态", "search", FINISHED),
        port::select("free", "门槛", "search", FREE),
        port::select("sort", "排序", "search", SORT),
    ];
    for (kind, label) in &KINDS[1..] {
        if let Some((id, choices)) = provider_tags::for_kind(kind) {
            fields.push(port::select(id, &format!("{label}标签"), "search", choices));
        }
    }
    for (entry, default) in fields
        .iter_mut()
        .zip(["search", "0", "0", "0", "1", "0", "0", "0", "0"])
    {
        entry["default"] = json!(default);
    }
    json!({"filters":fields,"useSearchForFilters":true})
}
fn settings() -> Value {
    let options = |values: &[(&str, &str)]| {
        values
            .iter()
            .map(|(value, label)| json!({"value":value,"label":label}))
            .collect::<Vec<_>>()
    };
    json!({"settings":[
        {"id":"preferredMirror","name":"备用域名","kind":"select","default":WEB,"options":options(MIRRORS)},
        {"id":"rank","name":"热门排行","kind":"select","default":"1","options":options(RANK)},
        {"id":"kind","name":"漫画类型","kind":"select","default":"boy","options":options(&KINDS[1..])}
    ]})
}
fn run(op: &str, req: &[u8]) -> u32 {
    let result = port::input(req).and_then(|input| match op {
        "get_listings" => Ok(port::listing_list(true, false, false)),
        "get_filters" => Ok(filters()),
        "get_settings" => Ok(settings()),
        "get_manga_list" => list(&input, req, false),
        "search" => list(&input, req, true),
        "get_manga" => manga(&input),
        "get_pages" => pages(&input),
        "get_image_request" => image(&input),
        "get_authentication" => {
            let login_url = port::absolute(mirror(&input)?, "/login")?;
            Ok(json!({"authentication":{"kind":"web","loginUrl":login_url.as_str()},"authenticated":false}))
        }
        _ => Err("not_implemented"),
    });
    match result {
        Err("favcomic_insufficient_balance") => {
            write_error(op, "chapter_locked", "金币不足，请在原站充值。")
        }
        Err("favcomic_daily_limit") => {
            write_error(op, "rate_limit", "今日免费额度已用完，明天零点重置。")
        }
        other => port::write_result(op, other),
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
fn run_get_settings(r: &[u8]) -> u32 {
    run("get_settings", r)
}
fn run_get_home(r: &[u8]) -> u32 {
    run("get_home", r)
}
fn run_get_image_request(r: &[u8]) -> u32 {
    run("get_image_request", r)
}
fn run_get_authentication(r: &[u8]) -> u32 {
    run("get_authentication", r)
}
fn run_authenticate(r: &[u8]) -> u32 {
    run("authenticate", r)
}
fn run_logout(r: &[u8]) -> u32 {
    run("logout", r)
}
fn chapter_failure(code: &str) -> u32 {
    port::write_failure("get_chapters", code)
}
koma_source_sdk::koma_source_exports!("favcomic", typed_chapters, chapter_failure);
koma_source_sdk::koma_source_authentication_exports!("favcomic");
