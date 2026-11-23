#![no_std]
extern crate alloc;

use alloc::{
    collections::BTreeSet,
    format,
    string::{String, ToString},
    vec::Vec,
};
use koma_source_sdk::{
    chapters::{Chapter, ChapterPage, ChapterPlacement, ChapterResult, GroupedChapters},
    json_utils::request_page_number,
    source::{SourceCapabilities, SourceInfo},
};
use serde_json::{json, Value};
use url::Url;

#[path = "../../port_support.rs"]
mod port_support;
use port_support::{self as port, Result};

const WEB: &str = "https://m.2024manga.com";
const API: &str = "https://api.manga2026.xyz/api/v3/";
const THEME: &[(&str, &str)] = &[
    ("", "全部"),
    ("aiqing", "愛情"),
    ("huanlexiang", "歡樂向"),
    ("maoxian", "冒險"),
    ("qihuan", "奇幻"),
    ("baihe", "百合"),
    ("xiaoyuan", "校園"),
    ("kehuan", "科幻"),
    ("dongfang", "東方"),
    ("danmei", "耽美"),
    ("gedou", "格鬥"),
    ("teenslove", "TL"),
    ("shenghuo", "生活"),
    ("qingxiaoshuo", "輕小說"),
    ("qita", "其他"),
    ("xuanyi", "懸疑"),
    ("mengxi", "萌系"),
    ("shengui", "神鬼"),
    ("zhichang", "職場"),
    ("zhiyu", "治癒"),
    ("jiecao", "節操"),
];
const ORDER: &[(&str, &str)] = &[
    ("", "更新時間・新到舊"),
    ("datetime_updated", "更新時間・舊到新"),
    ("-popular", "熱度・由高到低"),
    ("popular", "熱度・由低到高"),
];

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.relamanhua.koma",
    name: "熱辣漫畫",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "熱辣漫畫收錄戀愛、冒險、奇幻、校園等題材。普通瀏覽為免費作品，搜尋亦可能列出付費作品，其鎖定預覽不等於完整正文。",
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
    payload:1024*1024, http_out:4*1024*1024, body:3*1024*1024, http_req:4096, scratch:8192,
}
koma_source_sdk::koma_source_helpers!();

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 256
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn id(value: &Value) -> Result<&str> {
    value
        .as_str()
        .filter(|value| valid_id(value))
        .ok_or("invalid_request")
}

fn api(path: &str, query: &[(&str, &str)]) -> Result<Value> {
    api_document(path, query).map(|(value, _)| value)
}

fn api_document(path: &str, query: &[(&str, &str)]) -> Result<(Value, String)> {
    let mut url = port::absolute(API, path)?;
    url.query_pairs_mut().extend_pairs(query.iter().copied());
    let html = port::request(
        url.as_str(),
        "GET",
        json!({"Accept":"application/json","Referer":WEB,"X-Requested-With":"XMLHttpRequest"}),
        None,
    )?;
    let response: Value = serde_json::from_str(&html).map_err(|_| "parse_error")?;
    if response["code"].as_u64() != Some(200) {
        return Err(match response["code"].as_u64() {
            Some(401) => "auth_required",
            Some(403) => "forbidden",
            Some(404) => "not_found",
            _ => "server_error",
        });
    }
    let result = response.get("results").cloned().ok_or("parse_error")?;
    Ok((result, html))
}

fn name_list(value: &Value) -> impl Iterator<Item = &str> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| item["name"].as_str())
}

fn manga(comic: &Value, details: bool) -> Result<Value> {
    let slug = id(&comic["path_word"])?;
    let name = comic["name"].as_str().ok_or("parse_error")?;
    let mut item = json!({
        "id":slug,"title":name,"links":[{"kind":"source","url":format!("{WEB}/v2h5/details/comic/{slug}")}]
    });
    if let Some(cover) = comic["cover"].as_str().filter(|cover| !cover.is_empty()) {
        item["cover"] = json!({"kind":"url","url":port::absolute(WEB,cover)?.as_str()});
    }
    if details {
        item["authors"] = json!(name_list(&comic["author"]).collect::<Vec<_>>());
        item["tags"] = json!(["theme", "females", "males"]
            .into_iter()
            .flat_map(|key| name_list(&comic[key]))
            .collect::<Vec<_>>());
        item["status"] = json!(match comic["status"]["value"].as_u64() {
            Some(0) => "ongoing",
            Some(1) => "completed",
            _ => "unknown",
        });
        let info = ["region", "restrict"]
            .into_iter()
            .filter_map(|key| {
                comic[key]["display"]
                    .as_str()
                    .filter(|value| !value.is_empty())
            })
            .collect::<Vec<_>>()
            .join(" · ");
        let brief = comic["brief"].as_str().unwrap_or("");
        let aliases = comic["alias"]
            .as_str()
            .unwrap_or("")
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty() && *value != name)
            .collect::<Vec<_>>()
            .join("、");
        let description = [
            info,
            brief.to_string(),
            if aliases.is_empty() {
                aliases
            } else {
                format!("別名：{aliases}")
            },
        ]
        .into_iter()
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
        item["description"] = json!(description);
    }
    Ok(item)
}

fn detail(slug: &str) -> Result<Value> {
    api(&format!("comic2/{slug}"), &[("platform", "1")])
}

fn work_from_url(url: &Url) -> Result<&str> {
    let host = url.host_str().ok_or("invalid_request")?;
    let domain = host
        .strip_prefix("m.")
        .or_else(|| host.strip_prefix("www."))
        .unwrap_or(host);
    if !matches!(domain, "2024manga.com" | "manga2026.xyz") {
        return Err("not_found");
    }
    let segments = url
        .path_segments()
        .ok_or("invalid_request")?
        .collect::<Vec<_>>();
    let slug = match segments.as_slice() {
        ["comic", slug, ..]
        | ["v2h5", "details", "comic", slug, ..]
        | ["v2h5", "comicContent", slug, ..] => *slug,
        _ => return Err("not_found"),
    };
    if !valid_id(slug) {
        return Err("not_found");
    }
    Ok(slug)
}

fn list(request: &[u8], args: &Value, search: bool) -> Result<Value> {
    const PAGE_SIZE: usize = 30;
    let page = request_page_number(request);
    let offset = page
        .saturating_sub(1)
        .checked_mul(PAGE_SIZE)
        .ok_or("invalid_request")?;
    let offset = offset.to_string();
    let query = args["query"].as_str().unwrap_or("").trim();
    let response = if search && !query.is_empty() {
        if let Ok(url) = Url::parse(query) {
            if matches!(url.scheme(), "http" | "https") {
                let details = detail(work_from_url(&url)?)?;
                return Ok(port::items(
                    alloc::vec![manga(&details["comic"], true)?],
                    None,
                ));
            }
        }
        api(
            "search/comic",
            &[
                ("platform", "1"),
                ("q", query),
                ("q_type", ""),
                ("limit", "30"),
                ("offset", &offset),
            ],
        )?
    } else {
        let theme = port::filter(args, "theme", THEME)?;
        let ordering = port::filter(args, "ordering", ORDER)?;
        let listing = args["listingId"].as_str().unwrap_or("popular");
        let ordering = if !ordering.is_empty() {
            ordering
        } else if search || listing == "latest" {
            "-datetime_updated"
        } else {
            "-popular"
        };
        let mut params = alloc::vec![
            ("free_type", "1"),
            ("ordering", ordering),
            ("limit", "30"),
            ("offset", offset.as_str())
        ];
        if !theme.is_empty() {
            params.push(("theme", theme));
        }
        api("comics", &params)?
    };
    let rows = response["list"].as_array().ok_or("parse_error")?;
    let items = rows
        .iter()
        .map(|row| manga(row, false))
        .collect::<Result<Vec<_>>>()?;
    let total = response["total"].as_u64().ok_or("parse_error")?;
    let offset = response["offset"].as_u64().ok_or("parse_error")?;
    let next = (!items.is_empty() && offset + (items.len() as u64) < total)
        .then(|| page.checked_add(1))
        .flatten();
    Ok(port::items(items, next))
}

fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(request)?;
    let slug = id(&input["args"]["mangaId"])?;
    let (response, raw) = api_document(&format!("comic2/{slug}"), &[("platform", "1")])?;
    let groups = response["groups"].as_object().ok_or("parse_error")?;
    let mut catalog = GroupedChapters::new(payload_buf(), request).map_err(port::chapter_error)?;
    let mut index = 0;
    let mut seen = BTreeSet::new();
    // The order of the provider's group map is also the presentation order.
    let raw_results = koma_source_sdk::source_protocol::direct(raw.as_bytes(), b"results")?
        .ok_or("parse_error")?;
    let raw_groups =
        koma_source_sdk::source_protocol::direct(raw_results, b"groups")?.ok_or("parse_error")?;
    let raw_groups = core::str::from_utf8(raw_groups).map_err(|_| "parse_error")?;
    let mut group_order = groups.iter().collect::<Vec<_>>();
    group_order.sort_by_key(|(key, _)| {
        raw_groups
            .find(&format!(r#""{key}""#))
            .unwrap_or(usize::MAX)
    });
    for (_, group) in group_order {
        let group_id = id(&group["path_word"])?;
        let group_name = group["name"].as_str().ok_or("parse_error")?;
        let mut rows = Vec::new();
        let path = format!("comic/{slug}/group/{group_id}/chapters");
        loop {
            let offset = rows.len().to_string();
            let data = api(&path, &[("limit", "500"), ("offset", &offset)])?;
            let total = data["total"].as_u64().ok_or("parse_error")? as usize;
            let page = data["list"].as_array().ok_or("parse_error")?;
            rows.extend(page.iter().cloned());
            if rows.len() >= total || page.is_empty() {
                break;
            }
            if rows.len() > 8192 {
                return Err("result_too_large");
            }
        }
        for (position, row) in rows.iter().enumerate().rev() {
            if id(&row["comic_path_word"])? != slug {
                return Err("parse_error");
            }
            let chapter_id = format!("{slug}/{}", id(&row["uuid"])?);
            if !seen.insert(chapter_id.clone()) {
                continue;
            }
            let name = row["name"].as_str().ok_or("parse_error")?;
            let title = if group_id == "default" {
                name.to_string()
            } else {
                format!("{group_name}：{name}")
            };
            let mut chapter =
                Chapter::new(chapter_id.as_bytes(), slug.as_bytes(), title.as_bytes());
            chapter.index = Some(index);
            chapter.published_at = row["datetime_created"].as_str().map(str::as_bytes);
            catalog
                .push(
                    chapter,
                    ChapterPlacement::InGroup {
                        group: group_id.as_bytes(),
                        position: position as u64,
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
    let chapter = args["chapterId"].as_str().ok_or("invalid_request")?;
    let (slug, uuid) = chapter.split_once('/').ok_or("invalid_request")?;
    if !valid_id(slug)
        || !valid_id(uuid)
        || args["mangaId"].as_str().is_some_and(|work| work != slug)
    {
        return Err("invalid_request");
    }
    let data = api(
        &format!("comic/{slug}/chapter/{uuid}"),
        &[("platform", "1")],
    )?;
    if data["is_lock"].as_bool() == Some(true) {
        return Err("chapter_locked");
    }
    let pages = data["chapter"]["contents"]
        .as_array()
        .ok_or("parse_error")?
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let url = port::absolute(WEB, row["url"].as_str().ok_or("parse_error")?)?;
            let mut item = port::page(index, chapter, url);
            item["image"]["headers"] = port::mobile_image_headers("https://m.2024manga.com/");
            Ok(item)
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(json!({"pages":pages}))
}

fn dispatch(operation: &str, request: &[u8]) -> Result<Value> {
    let input = port::input(request)?;
    let args = &input["args"];
    match operation {
        "get_listings" => Ok(port::listing_list(true, true, true)),
        "get_filters" => Ok(json!({"filters":[
            port::select("ordering","排序","browse",ORDER),
            port::select("theme","題材","browse",THEME),
        ]})),
        "search" => list(request, args, true),
        "get_manga_list" => list(request, args, false),
        "get_manga" => Ok(json!({"manga":manga(&detail(id(&args["mangaId"])?)?["comic"],true)?})),
        "get_pages" => pages(args),
        _ => Err("not_implemented"),
    }
}

fn run(operation: &str, request: &[u8]) -> u32 {
    let result = dispatch(operation, request).map(|mut value| {
        if matches!(operation, "get_manga_list" | "search" | "get_manga") {
            port::mobile_cover_headers(&mut value, "https://m.2024manga.com/", request);
        }
        value
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
koma_source_sdk::koma_source_exports!("relamanhua", typed_chapters, write_chapter_failure);
