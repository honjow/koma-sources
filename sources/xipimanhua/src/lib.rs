#![no_std]
extern crate alloc;
use alloc::{
    collections::BTreeSet,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use koma_source_sdk::{
    chapters::{Chapter, ChapterOrder, ChapterResponse, ChapterResult},
    json_utils::request_page_number,
    source::{SourceCapabilities, SourceInfo},
};
use serde_json::{json, Value};
use url::Url;
#[path = "../../port_support.rs"]
mod port_support;
use port_support::{self as port, Result};

const WEB: &str = "https://m.hipmh.com";
const READER: &str = "https://reader.hipmh.top";
const API: &str = "https://hipapi1.s3file.top/";
const COVER: &str = "https://cover.s3imgs.top";
const SORTS: &[(&str, &str)] = &[("", "热门"), ("updated", "更新"), ("latest", "最新")];
const AREAS: &[(&str, &str)] = &[("", "全部"), ("1", "韩漫"), ("2", "国漫"), ("3", "日漫")];
const STATES: &[(&str, &str)] = &[("", "全部"), ("ongoing", "连载中"), ("completed", "已完结")];
const GENRES: &[(&str, &str)] = &[
    ("", "全部"),
    ("3", "剧情"),
    ("20", "穿越"),
    ("27", "玄幻"),
    ("30", "大女主"),
    ("31", "复仇"),
    ("32", "逆袭"),
    ("38", "冒险"),
    ("39", "武侠"),
    ("40", "动作"),
    ("44", "宫斗"),
    ("46", "重生"),
    ("51", "异能"),
    ("67", "系统"),
    ("288", "大人系"),
];

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.xipimanhua.koma",
    name: "嬉皮漫画",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "嬉皮漫画的韩漫、国漫与日漫书库，涵盖穿越、玄幻、复仇、冒险与武侠等题材。",
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
koma_source_sdk::koma_source_buffers! {payload:1024*1024,http_out:4*1024*1024,body:3*1024*1024,http_req:4096,scratch:8192,}
koma_source_sdk::koma_source_helpers!();

fn decoded(value: &str) -> Result<String> {
    let bytes = URL_SAFE_NO_PAD.decode(value).map_err(|_| "parse_error")?;
    String::from_utf8(bytes).map_err(|_| "parse_error")
}
fn number(value: &str) -> bool {
    !value.is_empty() && value.len() <= 24 && value.bytes().all(|b| b.is_ascii_digit())
}
fn manga_key(id: &str) -> Result<&str> {
    if !decoded(id)?.strip_prefix("m:").is_some_and(number) {
        return Err("invalid_request");
    }
    Ok(id)
}
fn parts(hid: &str) -> Result<(String, String, &str)> {
    let (encoded, suffix) = hid.split_once('-').ok_or("invalid_request")?;
    let value = decoded(encoded)?;
    let (manga, chapter) = value.split_once('-').ok_or("invalid_request")?;
    if !manga.strip_prefix("m:").is_some_and(number)
        || !chapter.strip_prefix("c:").is_some_and(number)
        || !suffix
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        || suffix.is_empty()
        || suffix.len() > 128
    {
        return Err("invalid_request");
    }
    Ok((
        URL_SAFE_NO_PAD.encode(manga),
        URL_SAFE_NO_PAD.encode(chapter),
        suffix,
    ))
}
fn api(path: &str, query: &[(&str, &str)]) -> Result<Value> {
    let mut url = port::absolute(API, path)?;
    url.query_pairs_mut().extend_pairs(query.iter().copied());
    let mut headers = json!({"Accept":"application/json","Referer":WEB});
    if path == "v2/chapter" {
        // The same endpoint receives its web requests from the reader origin.
        headers = port::mobile_image_headers(&format!("{READER}/"));
        headers["Accept"] = json!("application/json");
        headers["Origin"] = json!(READER);
    }
    let body = port::request(url.as_str(), "GET", headers, None)?;
    let response: Value = serde_json::from_str(&body).map_err(|_| "parse_error")?;
    response
        .get("data")
        .filter(|data| !data.is_null())
        .cloned()
        .ok_or("parse_error")
}
fn manga(id: &str, row: &Value, details: bool) -> Result<Value> {
    let key = manga_key(id)?;
    let title = row["title"].as_str().ok_or("parse_error")?.trim();
    let mut item = json!({"id":key,"title":title,"links":[{"kind":"source","url":format!("{WEB}/works/{key}")}]});
    if let Some(cover) = row["vertical_image_url"]
        .as_str()
        .filter(|value| !value.is_empty())
    {
        item["cover"] = json!({"kind":"url","url":port::absolute(COVER,cover)?.as_str()});
    }
    if details {
        item["description"] = json!(row["description"].as_str().unwrap_or(""));
        let named = |field| {
            row[field]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|name| name["name"].as_str().map(str::trim))
        };
        item["authors"] = json!(named("authors").collect::<Vec<_>>());
        item["tags"] = json!(named("categories")
            .chain(named("genres"))
            .collect::<Vec<_>>());
        item["status"] = json!(match row["status"].as_str() {
            Some("ongoing") => "ongoing",
            Some("completed") => "completed",
            Some("hiatus") => "hiatus",
            _ => "unknown",
        });
    }
    Ok(item)
}
fn detail(id: &str) -> Result<Value> {
    let key = manga_key(id)?;
    manga(key, &api("v1/manga", &[("mid", key)])?, true)
}
fn work_for_url(url: &Url) -> Result<String> {
    if !matches!(url.host_str(), Some("m.hipmh.com" | "reader.hipmh.top")) {
        return Err("not_found");
    }
    let segments = url.path_segments().ok_or("not_found")?.collect::<Vec<_>>();
    for pair in segments.windows(2) {
        match pair {
            ["works", id] => {
                return Ok(manga_key(id.split('-').next().ok_or("not_found")?)?.to_string())
            }
            ["chapter", id] => {
                let hid = if *id == "go" {
                    url.query_pairs()
                        .find(|(key, _)| key == "hid")
                        .ok_or("not_found")?
                        .1
                        .into_owned()
                } else {
                    id.to_string()
                };
                return Ok(parts(&hid)?.0);
            }
            _ => {}
        }
    }
    Err("not_found")
}
fn list(input: &Value, request: &[u8], search: bool) -> Result<Value> {
    let args = &input["args"];
    let page = request_page_number(request);
    let page_param = page.to_string();
    let keyword = args["query"].as_str().unwrap_or("").trim();
    let typed = search && !keyword.is_empty();
    let data = if typed {
        if let Ok(url) = Url::parse(keyword) {
            if matches!(url.scheme(), "http" | "https") {
                return Ok(port::items(vec![detail(&work_for_url(&url)?)?], None));
            }
        }
        api(
            "v1/search",
            &[("q", keyword), ("page", &page_param), ("page_size", "20")],
        )?
    } else {
        let sort = port::filter(args, "sort", SORTS)?;
        let sort = if sort.is_empty() {
            if args["listingId"] == "latest" {
                "updated"
            } else {
                "popular"
            }
        } else {
            sort
        };
        let mut query = vec![
            ("sort", sort),
            ("page", page_param.as_str()),
            ("per_page", "18"),
        ];
        for (key, options) in [("category", AREAS), ("genre", GENRES), ("status", STATES)] {
            let selected = port::filter(args, key, options)?;
            if !selected.is_empty() {
                query.push((key, selected));
            }
        }
        api("v1/mangas", &query)?
    };
    let include_summary =
        koma_source_sdk::source_protocol::feature(request, koma_source_sdk::list_info::HOST_FEATURE);
    let mut seen = BTreeSet::new();
    let mut items = Vec::new();
    for row in data[if typed { "data" } else { "items" }]
        .as_array()
        .ok_or("parse_error")?
    {
        let raw = row[if typed { "id" } else { "mid" }]
            .as_str()
            .ok_or("parse_error")?;
        let key = raw.split('-').next().ok_or("parse_error")?;
        if seen.insert(key) {
            let mut item = manga(key, row, false)?;
            // These names and genres are in the existing list response. Do not
            // fetch detail or alter legacy fields and their fallback behavior.
            if include_summary {
                let authors: Vec<&str> = if typed {
                    row["authors"].as_array().into_iter().flatten()
                        .filter_map(|v| v["name"].as_str()).map(str::trim)
                        .filter(|s| !s.is_empty()).collect()
                } else {
                    row["author_names"].as_array().into_iter().flatten()
                        .filter_map(Value::as_str).map(str::trim)
                        .filter(|s| !s.is_empty()).collect()
                };
                let author = authors.join(", ");
                let tags: Vec<&str> = row["genres"].as_array().into_iter().flatten()
                    .filter_map(|v| v.as_str().or_else(|| v["name"].as_str())).map(str::trim)
                    .filter(|s| !s.is_empty() && s.len() <= 96).take(8).collect();
                let mut info = Vec::new();
                if !author.is_empty() && author.len() <= 512 {
                    info.push(json!({"type":"text","text":author}));
                }
                if !tags.is_empty() {
                    info.push(json!({"type":"tags","items":tags}));
                }
                if !info.is_empty() {
                    item["listInfo"] = json!(info);
                }
            }
            items.push(item);
        }
    }
    let next = if data["page"].as_u64().ok_or("parse_error")?
        < data["total_pages"].as_u64().ok_or("parse_error")?
    {
        page.checked_add(1)
    } else {
        None
    };
    Ok(port::items(items, next))
}
fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(request)?;
    let id = manga_key(input["args"]["mangaId"].as_str().ok_or("invalid_request")?)?;
    let mut rows = Vec::new();
    let mut seen = BTreeSet::new();
    let mut page = 1usize;
    loop {
        let data = api(
            "v1/manga/chapters",
            &[
                ("mid", id),
                ("page", &page.to_string()),
                ("per_page", "50"),
                ("order", "desc"),
            ],
        )?;
        let items = data["items"].as_array().ok_or("parse_error")?;
        for row in items {
            let hid = row["hid"].as_str().ok_or("parse_error")?;
            if parts(hid)?.0 != id {
                return Err("parse_error");
            }
            if seen.insert(hid.to_string()) {
                rows.push(row.clone());
            }
        }
        let max_page = data["total_pages"].as_u64().ok_or("parse_error")?;
        if page as u64 >= max_page || items.is_empty() {
            break;
        }
        if page >= 164 {
            return Err("result_too_large");
        }
        page += 1;
    }
    let chapters = rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let mut chapter = Chapter::new(
                row["hid"].as_str().ok_or("parse_error")?.as_bytes(),
                id.as_bytes(),
                row["title"].as_str().ok_or("parse_error")?.as_bytes(),
            );
            chapter.index = Some(index);
            chapter.published_at = row["updated_at"].as_str().map(str::as_bytes);
            Ok(chapter)
        })
        .collect::<Result<Vec<_>>>()?;
    ChapterResponse::ordered(payload_buf(), request, ChapterOrder::NewestFirst, &chapters)
        .map_err(port::chapter_error)
}

fn decode_images(encoded: &str) -> Result<Vec<String>> {
    const FROM: &[u8] = b"_-9876543210abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    const TO: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let body = encoded
        .strip_prefix("qM9")
        .and_then(|value| value.strip_suffix("Z7"))
        .ok_or("parse_error")?;
    if !body.is_ascii() || body.len() <= 5 {
        return Err("parse_error");
    }
    let tail = (body.len() - 5) / 3;
    let head = (body.len() - 5 - tail) / 2;
    let (head_str, middle) = body.split_at(head);
    let middle = middle.strip_prefix("Vx").ok_or("parse_error")?;
    let (middle, tail_str) = middle.split_at(middle.len() - tail);
    let middle = middle.strip_suffix("pL0").ok_or("parse_error")?;
    let joined = format!("{tail_str}{head_str}{middle}");
    let mut shifted = Vec::with_capacity(joined.len());
    for (i, block) in joined.as_bytes().chunks(7).enumerate() {
        for pos in 0..block.len() {
            let index = if i % 2 == 0 {
                pos
            } else {
                block.len() - 1 - pos
            };
            let symbol = FROM
                .iter()
                .position(|&byte| byte == block[index])
                .ok_or("parse_error")?;
            shifted.push(TO[symbol]);
        }
    }
    let decoded = URL_SAFE_NO_PAD.decode(shifted).map_err(|_| "parse_error")?;
    serde_json::from_slice(&decoded).map_err(|_| "parse_error")
}
fn pages(input: &Value) -> Result<Value> {
    let args = &input["args"];
    let hid = args["chapterId"].as_str().ok_or("invalid_request")?;
    let (owner, chapter, suffix) = parts(hid)?;
    if args["mangaId"]
        .as_str()
        .is_some_and(|provided| provided != owner)
    {
        return Err("invalid_request");
    }
    let data = api("v2/chapter", &[("hid", &format!("{chapter}-{suffix}"))])?;
    let mut images = if let Some(encoded) = data["images"].as_str() {
        decode_images(encoded)?
    } else {
        serde_json::from_value(data["images"].clone()).map_err(|_| "parse_error")?
    };
    if !images.is_empty() {
        let count = images.len() as u128;
        let sid = data["sid"].as_u64().ok_or("parse_error")? as u128;
        let order = data["order_id"].as_u64().ok_or("parse_error")? as u128;
        let key = ((sid * 0x9E3779B1) ^ (count * 0x85EBCA6B)) % count;
        let index = order ^ key;
        if index < count {
            images.remove(index as usize);
        }
    }
    if images.is_empty() {
        return Err("chapter_unavailable");
    }
    let path = match (
        input["settings"]["line"].as_str().unwrap_or("line1"),
        data["line"].as_u64(),
    ) {
        ("line1", Some(9)) => "https://hip-tx-s1.s3imgs.top",
        ("line2", Some(9)) => "https://hip-cf-s1.s3imgs.top",
        ("line1", _) => "https://hip-tx-1.s3imgs.top",
        ("line2", _) => "https://hip-cf-1.s3imgs.top",
        _ => return Err("invalid_request"),
    };
    let pages = images
        .iter()
        .enumerate()
        .map(|(index, raw)| Ok(port::page(index, hid, port::absolute(path, raw)?)))
        .collect::<Result<Vec<_>>>()?;
    Ok(json!({"pages":pages}))
}
fn dispatch(op: &str, request: &[u8]) -> Result<Value> {
    let input = port::input(request)?;
    let args = &input["args"];
    match op {
        "get_listings" => Ok(port::listing_list(true, true, true)),
        "get_filters" => Ok(
            json!({"filters":[port::select("sort","排序","browse",SORTS),
            port::select("category","地区","browse",AREAS),port::select("genre","类型","browse",GENRES),
            port::select("status","状态","browse",STATES)]}),
        ),
        "get_settings" => Ok(
            json!({"settings":[{"id":"line","name":"线路","kind":"select","default":"line1",
            "options":[{"value":"line1","label":"线路 1"},{"value":"line2","label":"线路 2"}]}]}),
        ),
        "search" => list(&input, request, true),
        "get_manga_list" => list(&input, request, false),
        "get_manga" => {
            Ok(json!({"manga":detail(args["mangaId"].as_str().ok_or("invalid_request")?)?}))
        }
        "get_pages" => pages(&input),
        _ => Err("not_implemented"),
    }
}
fn run(op: &str, r: &[u8]) -> u32 {
    let result = dispatch(op, r).map(|mut payload| {
        port::mobile_cover_headers(&mut payload, &format!("{WEB}/"), r);
        payload
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
koma_source_sdk::koma_source_exports!("xipimanhua", typed_chapters, write_chapter_failure);
