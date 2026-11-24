#![no_std]
extern crate alloc;

use alloc::{
    collections::{BTreeMap, BTreeSet},
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use koma_source_sdk::{
    chapters::{Chapter, ChapterPage, ChapterPlacement, ChapterResult, GroupedChapters},
    host,
    json_utils::request_page_number,
    source::{SourceCapabilities, SourceInfo},
    source_protocol,
};
use serde_json::{json, Value};
use url::Url;

#[allow(dead_code)]
#[path = "../../port_support.rs"]
mod port;
mod session;
use port::Result;

const WEB: &str = "https://vomicmh.com";
const API: &str = "https://api.vomicmh.com/pics_new/pics";
const LOGIN: &str = "https://api.vomicmh.com/pics_new/pics/login";
const USER_AGENT: &str = "Dart/3.13 (dart:io)";

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.vomicmh.koma",
    name: "vomic",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "zh-Hans",
    author: "Koma",
    description: "vomic漫目录、筛选和原站阅读。",
    content_rating: "safe",
};
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    listings: true,
    manga_list: true,
    filters: true,
    credentials: true,
    image_request: true,
    ..SourceCapabilities::CORE
};

#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;
koma_source_sdk::koma_source_buffers! {payload:1024*1024,http_out:4*1024*1024,body:3*1024*1024,http_req:4096,scratch:8192,}
koma_source_sdk::koma_source_helpers!();

const FILTERS: &[(&str, &str, &[(&str, &str)])] = &[
    (
        "sort",
        "排序",
        &[("", "默认"), ("hot_value", "最热"), ("process_at", "最新")],
    ),
    (
        "process",
        "状态",
        &[("", "全部"), ("连载中", "连载中"), ("已完结", "已完结")],
    ),
    (
        "creativity",
        "来源",
        &[
            ("", "全部"),
            ("原创", "原创"),
            ("汉化", "汉化"),
            ("转载", "转载"),
        ],
    ),
    (
        "region",
        "地区",
        &[
            ("", "全部"),
            ("大陆", "大陆"),
            ("日本", "日本"),
            ("韩国", "韩国"),
            ("港台", "港台"),
            ("欧美", "欧美"),
            ("其他", "其他"),
        ],
    ),
    (
        "dis_type",
        "类型",
        &[
            ("", "全部"),
            ("条漫", "条漫"),
            ("页漫", "页漫"),
            ("其他", "其他"),
        ],
    ),
];

fn check_feature(request: &[u8], feature: &[u8]) -> Result<()> {
    if source_protocol::feature(request, feature) {
        Ok(())
    } else {
        Err("host_feature_required")
    }
}

fn signature(url: &Url, timestamp: &str) -> String {
    const SALT: &str = "hhhasda";
    let params: BTreeMap<_, _> = url.query_pairs().collect();
    let payload = params
        .iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&");
    let encoded = STANDARD.encode(format!("{payload}{timestamp}{SALT}"));
    let key = SALT.as_bytes();
    let mut state = 0x9e37_79b9u32;
    for &byte in key {
        state = (state ^ u32::from(byte)).rotate_left(7);
    }
    for (index, byte) in encoded
        .bytes()
        .chain(timestamp.bytes())
        .chain(SALT.bytes())
        .enumerate()
    {
        let value = u32::from(key[index % key.len()]);
        let mix = if index % 2 == 0 {
            value * 8
        } else {
            value >> 1
        };
        state = (state ^ u32::from(byte)).rotate_left(5) ^ mix;
    }
    for &byte in key {
        state = (state ^ (u32::from(byte) * 16)).rotate_left(9);
    }
    format!("{state:08x}")
}

fn endpoint(path: &str) -> Result<Url> {
    port::absolute(API, &format!("{API}/{path}"))
}

fn client_headers() -> Value {
    json!({
        "User-Agent":USER_AGENT,"platform":"android",
        "hide-content":"0","name":"pics","version":"1.2.1"
    })
}

fn request_api(url: &Url, credentials: Option<Value>, bearer: Option<&str>) -> Result<Value> {
    let timestamp = session::clock_seconds()?;
    let mut headers = client_headers();
    headers["t"] = json!(timestamp);
    headers["s"] = json!(signature(url, &timestamp));
    if let Some(token) = bearer {
        headers["Authorization"] = json!(format!("Bearer {token}"));
    }
    let body = credentials.map(|credentials| credentials.to_string());
    if body.is_some() {
        headers["Content-Type"] = json!("application/json; charset=UTF-8");
    }
    let response = port::request(
        url.as_str(),
        if body.is_some() { "POST" } else { "GET" },
        headers,
        body.as_deref(),
    )?;
    serde_json::from_str(&response).map_err(|_| "parse_error")
}

fn api_data(url: &Url, bearer: Option<&str>) -> Result<Value> {
    let mut response = request_api(url, None, bearer)?;
    match response["code"].as_u64() {
        Some(200) if !response["data"].is_null() => Ok(response["data"].take()),
        Some(401) => Err("auth_required"),
        Some(403) => Err("forbidden"),
        Some(404) => Err("not_found"),
        Some(429) => Err("rate_limit"),
        Some(_) => Err("source_error"),
        None => Err("parse_error"),
    }
}

fn id(value: &str) -> Result<u64> {
    value
        .parse::<u64>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or("invalid_request")
}

fn arg_id(args: &Value, key: &str) -> Result<u64> {
    id(args[key].as_str().ok_or("invalid_request")?)
}

fn datestr(value: Option<&str>) -> Option<String> {
    value
        .filter(|date| date.len() == 19 && date.is_ascii() && &date[10..11] == " ")
        .map(|date| format!("{}T{}+08:00", &date[..10], &date[11..]))
}

fn grouped_number(number: u64) -> String {
    let text = number.to_string();
    let mut formatted = String::new();
    for (index, digit) in text.bytes().enumerate() {
        if index != 0 && (text.len() - index) % 3 == 0 {
            formatted.push(',');
        }
        formatted.push(char::from(digit));
    }
    formatted
}

fn card(row: &Value) -> Result<Value> {
    let id = row["id"]
        .as_u64()
        .filter(|id| *id > 0)
        .ok_or("parse_error")?;
    let title = row["name"].as_str().ok_or("parse_error")?;
    let mut manga = json!({
        "id":id.to_string(), "title":title, "language":"zh-Hans",
        "links":[{"kind":"source","url":format!("{WEB}/detail/{id}")}]
    });
    if let Some(cover) = row["cover"].as_str().filter(|cover| !cover.is_empty()) {
        let url = port::absolute(WEB, cover)?;
        manga["cover"] = json!({"kind":"url","url":url.as_str()});
    }
    if let Some(author) = row["author"].as_str().filter(|author| !author.is_empty()) {
        manga["authors"] = json!([author]);
    }
    let mut tags = Vec::new();
    for tag in ["cat", "tag"]
        .iter()
        .flat_map(|key| row[*key].as_array().into_iter().flatten())
        .filter_map(|tag| tag["name"].as_str())
        .chain(row["tags"].as_str().unwrap_or_default().split_whitespace())
    {
        if !tags.contains(&tag) {
            tags.push(tag);
        }
    }
    manga["tags"] = json!(tags);
    manga["status"] = json!(match row["process"].as_str() {
        Some("连载中") => "ongoing",
        Some("已完结") => "completed",
        _ => "unknown",
    });
    let mut description = format!(
        " 👁️ {} - ❤️ {}",
        grouped_number(row["view_num"].as_u64().unwrap_or_default()),
        grouped_number(row["likes_num"].as_u64().unwrap_or_default())
    );
    if let Some(intro) = row["intro"]
        .as_str()
        .filter(|intro| !intro.trim().is_empty())
    {
        description.push_str("\n\n");
        description.push_str(intro);
    }
    manga["description"] = json!(description);
    Ok(manga)
}

fn work(manga: u64) -> Result<Value> {
    let mut url = endpoint("c/getCartoonInfoV1")?;
    url.query_pairs_mut()
        .append_pair("car_id", &manga.to_string());
    let row = api_data(&url, None)?;
    if row["id"].as_u64() != Some(manga) {
        return Err("parse_error");
    }
    Ok(row)
}

fn link_id(query: &str) -> Option<u64> {
    let url = Url::parse(query).ok()?;
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || !matches!(url.host_str(), Some("vomicmh.com" | "www.vomicmh.com"))
    {
        return None;
    }
    id(url.path().strip_prefix("/detail/")?.trim_end_matches('/')).ok()
}

fn list(args: &Value, page: usize, listing: Option<&str>) -> Result<Value> {
    let query = args["query"].as_str().unwrap_or_default();
    if listing.is_none() && page == 1 {
        if let Some(id) = link_id(query) {
            return Ok(port::items(vec![card(&work(id)?)?], None));
        }
    }
    let limit = match listing {
        Some("latest") => 15,
        Some("popular") => 20,
        _ if query.is_empty() => 20,
        _ => 10,
    };
    let mut url = endpoint(if listing == Some("latest") {
        "c/getUpdateBook"
    } else {
        "c/getCartoonList"
    })?;
    {
        let mut pairs = url.query_pairs_mut();
        pairs
            .append_pair("page", &page.to_string())
            .append_pair("limit", &limit.to_string());
        if listing == Some("popular") {
            pairs.append_pair("sort", "hot_value");
        } else if listing.is_none() {
            if !query.is_empty() {
                pairs.append_pair("keyword", query);
            } else {
                for &(field, _, options) in FILTERS {
                    pairs.append_pair(field, port::filter(args, field, options)?);
                }
                if let Some(category) = args["filters"].get("cat_id") {
                    let category = category.as_str().ok_or("invalid_request")?;
                    let category = category.parse::<u64>().map_err(|_| "invalid_request")?;
                    pairs.append_pair("cat_id", &category.to_string());
                }
            }
        }
    }
    let data = api_data(&url, None)?;
    let rows = data.as_array().ok_or("parse_error")?;
    let result = rows.iter().map(card).collect::<Result<Vec<_>>>()?;
    Ok(port::items(
        result,
        if rows.len() >= limit {
            page.checked_add(1)
        } else {
            None
        },
    ))
}

fn filters(request: &[u8], args: &Value) -> Result<Value> {
    let search_route = source_protocol::feature(request, b"source_filter_search_without_query_v1")
        && source_protocol::feature(request, b"source_filter_scope_header_v1");
    let dynamic = source_protocol::feature(request, b"source_filter_fetching_v1");
    let mut items: Vec<_> = FILTERS
        .iter()
        .map(|&(id, label, options)| {
            let mut item = port::select(id, label, "search", options);
            if !search_route {
                item.as_object_mut().unwrap().remove("scope");
            }
            item
        })
        .collect();
    if dynamic {
        let mut category = json!({"type":"select","id":"cat_id","name":"分类","default":"0"});
        if search_route {
            category["scope"] = json!("search");
        }
        if args["fetchFilterData"] == true {
            let data = api_data(&endpoint("c/getCatList")?, None)?;
            let categories = data.as_array().ok_or("parse_error")?;
            if !categories.is_empty() {
                let mut options = vec![json!({"value":"0","label":"全部"})];
                let mut used = BTreeSet::new();
                for item in categories {
                    let id = item["id"]
                        .as_u64()
                        .filter(|id| *id > 0)
                        .ok_or("parse_error")?;
                    let label = item["name"].as_str().ok_or("parse_error")?;
                    if used.insert(id) {
                        options.push(json!({"value":id.to_string(),"label":label}));
                    }
                }
                category["options"] = json!(options);
            }
        }
        items.push(category);
    }
    Ok(json!({"filters":items,"supportsFilterFetching":dynamic,"useSearchForFilters":search_route}))
}

fn details(request: &[u8], args: &Value) -> Result<Value> {
    let mut manga = card(&work(arg_id(args, "mangaId")?)?)?;
    if source_protocol::feature(request, b"source_related_manga_v1") {
        if let Ok(data) = api_data(&endpoint("c/getDetailRecommendList")?, None) {
            if let Some(rows) = data.as_array() {
                manga["relatedManga"] = json!(rows
                    .iter()
                    .take(100)
                    .filter_map(|row| card(row).ok())
                    .collect::<Vec<_>>());
            }
        }
    }
    Ok(json!({"manga":manga}))
}

fn chapter_parts(chapter: &str) -> Result<(u64, u64)> {
    let (manga, chapter) = chapter.split_once('/').ok_or("invalid_request")?;
    Ok((id(manga)?, id(chapter)?))
}

fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    check_feature(request, b"source_private_session_v1")?;
    let input = port::input(request)?;
    let manga = arg_id(&input["args"], "mangaId")?;
    let manga_id = manga.to_string();
    let data = work(manga)?;
    let groups = data["chapter_list"].as_array().ok_or("parse_error")?;
    let mut catalog = GroupedChapters::new(payload_buf(), request).map_err(port::chapter_error)?;
    let mut seen = BTreeSet::new();
    for group in groups {
        let Some(rows) = group["values"].as_array() else {
            continue;
        };
        let sort_values: Option<Vec<_>> = rows
            .iter()
            .map(|row| {
                row["sort"]
                    .as_str()?
                    .parse::<f64>()
                    .ok()
                    .filter(|value| value.is_finite())
            })
            .collect();
        let group_id = group["name"].as_str().filter(|name| {
            !name.trim().is_empty()
                && groups
                    .iter()
                    .filter(|group| group["name"].as_str() == Some(*name))
                    .count()
                    == 1
        });
        let order = sort_values.as_ref().and_then(|values| {
            if values.windows(2).all(|pair| pair[0] < pair[1]) {
                Some(false)
            } else if values.windows(2).all(|pair| pair[0] > pair[1]) {
                Some(true)
            } else {
                None
            }
        });
        for (index, row) in rows.iter().enumerate() {
            let chapter = row["id"]
                .as_u64()
                .filter(|id| *id > 0)
                .ok_or("parse_error")?;
            if row["car_id"].as_u64() != Some(manga) || !seen.insert(chapter) {
                continue;
            }
            let chapter_id = format!("{manga}/{chapter}");
            let title = row["name"].as_str().ok_or("parse_error")?;
            let group_name = row["group_name"]
                .as_str()
                .filter(|name| !name.trim().is_empty());
            let date = datestr(
                row["created_at"]
                    .as_str()
                    .or_else(|| row["updated_at"].as_str()),
            );
            let mut entry =
                Chapter::new(chapter_id.as_bytes(), manga_id.as_bytes(), title.as_bytes());
            entry.language = Some(b"zh-Hans");
            entry.scanlator = group_name.map(str::as_bytes);
            entry.published_at = date.as_deref().map(str::as_bytes);
            // Upstream flattens groups in response order. Keep that presentation
            // order. Only a uniquely ranked provider group allows continuation.
            let placement = match (group_id, order) {
                (Some(group), Some(newest_first)) => ChapterPlacement::InGroup {
                    group: group.as_bytes(),
                    position: if newest_first {
                        (rows.len() - index - 1) as u64
                    } else {
                        index as u64
                    },
                },
                _ => ChapterPlacement::ManualOnly,
            };
            catalog
                .push(entry, placement)
                .map_err(port::chapter_error)?;
        }
    }
    catalog
        .finish(ChapterPage::Complete)
        .map_err(port::chapter_error)
}

fn pages(request: &[u8], args: &Value) -> Result<Value> {
    let chapter_id = args["chapterId"].as_str().ok_or("invalid_request")?;
    let (manga, chapter) = chapter_parts(chapter_id)?;
    check_feature(request, b"source_native_cookie_session_v1")?;
    if args["mangaId"].as_str().is_some() && arg_id(args, "mangaId")? != manga {
        return Err("invalid_request");
    }
    let mut url = endpoint("c/getChapterInfo")?;
    url.query_pairs_mut()
        .append_pair("c_id", &chapter.to_string());
    let session = session::current()?;
    let data = match api_data(
        &url,
        Some(session.as_ref().map_or("", |session| session.token())),
    ) {
        Err("auth_required") => {
            check_feature(request, b"source_authentication_v1")?;
            let current = session.ok_or("auth_required")?;
            let renewed = session::refresh(&current)?;
            api_data(&url, Some(renewed.token()))?
        }
        result => result?,
    };
    let rows = data["img_list"].as_array().ok_or("parse_error")?;
    let mut pages = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        let Some(image) = row["url"].as_str().filter(|url| !url.is_empty()) else {
            continue;
        };
        let url = port::absolute(WEB, image)?;
        pages.push(port::page(index, chapter_id, url));
    }
    if pages.is_empty() {
        return Err("chapter_unavailable");
    }
    Ok(json!({"pages":pages}))
}

fn run(op: &str, request: &[u8]) -> u32 {
    port::write_result(
        op,
        port::input(request).and_then(|input| {
            let args = &input["args"];
            if matches!(op, "search" | "get_manga_list" | "get_manga" | "get_pages")
                || (op == "get_filters" && args["fetchFilterData"] == true)
            {
                check_feature(request, b"source_private_session_v1")?;
            }
            match op {
                "search" => list(args, request_page_number(request), None),
                "get_manga_list" => {
                    let listing = match args["listingId"].as_str() {
                        Some(value @ ("popular" | "latest")) => value,
                        _ => return Err("invalid_request"),
                    };
                    list(args, request_page_number(request), Some(listing))
                }
                "get_listings" => Ok(port::listing_list(true, true, false)),
                "get_manga" => details(request, args),
                "get_pages" => pages(request, args),
                "get_filters" => filters(request, args),
                "get_image_request" => {
                    let url = port::absolute(WEB, args["url"].as_str().ok_or("invalid_request")?)?;
                    Ok(json!({"url":url.as_str(),"headers":client_headers()}))
                }
                "get_authentication" => session::status(request),
                "authenticate" => session::authenticate(request),
                "logout" => session::logout(request),
                _ => Err("unsupported_operation"),
            }
        }),
    )
}

fn run_search(r: &[u8]) -> u32 {
    run("search", r)
}
fn run_get_listings(r: &[u8]) -> u32 {
    run("get_listings", r)
}
fn run_get_manga_list(r: &[u8]) -> u32 {
    run("get_manga_list", r)
}
fn run_get_manga(r: &[u8]) -> u32 {
    run("get_manga", r)
}
fn run_get_pages(r: &[u8]) -> u32 {
    run("get_pages", r)
}
fn run_get_image_request(r: &[u8]) -> u32 {
    run("get_image_request", r)
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

koma_source_sdk::koma_source_exports!("vomic", typed_chapters, chapter_failure);
koma_source_sdk::koma_source_authentication_exports!("vomic");
