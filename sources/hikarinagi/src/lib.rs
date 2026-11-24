#![no_std]
extern crate alloc;

use alloc::{
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec::Vec,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use koma_source_sdk::{
    chapters::{
        Chapter, ChapterNumber, ChapterPage, ChapterPlacement, ChapterResult, DecimalChapterNumber,
        GroupedChapters,
    },
    json_utils::request_page_number,
    source::{SourceCapabilities, SourceInfo},
    source_protocol,
};
use percent_encoding::percent_decode_str;
use serde_json::{json, Value};
use url::Url;

#[path = "../../port_support.rs"]
mod port;
mod body_decode;
use port::Result;

const BASE: &str = "https://www.hikarinagi.org";
const LOGIN: &str = "https://www.hikarinagi.org/login";
const CDN: &str = "https://imagesp.yurari.moe";
const CONTENT_PATH: &str = "/api/v3/reader/mangas/";
const SORTS: &[(&str, &str)] = &[
    ("latest_chapter_at:desc", "更新时间（降序）"),
    ("latest_chapter_at:asc", "更新时间（升序）"),
    ("heat:desc", "热度（降序）"),
    ("heat:asc", "热度（升序）"),
    ("created_at:desc", "收录时间（降序）"),
    ("created_at:asc", "收录时间（升序）"),
    ("publication_date:desc", "发布时间（降序）"),
    ("publication_date:asc", "发布时间（升序）"),
    ("name:desc", "标题（降序）"),
    ("name:asc", "标题（升序）"),
];
const FILTERS: &[(&str, &str, &[(&str, &str)])] = &[
    ("sort", "排序", SORTS),
    (
        "region",
        "地区",
        &[
            ("", "全部"),
            ("jp", "日漫"),
            ("kr", "韩漫"),
            ("cn", "国漫"),
            ("other", "其他"),
        ],
    ),
    (
        "audience",
        "受众",
        &[
            ("", "全部"),
            ("shonen", "少年"),
            ("seinen", "青年"),
            ("shojo", "少女"),
            ("josei", "女性"),
        ],
    ),
    (
        "status",
        "状态",
        &[
            ("", "全部"),
            ("serializing", "连载中"),
            ("finished", "已完结"),
            ("paused", "休刊"),
        ],
    ),
    (
        "decade",
        "年代",
        &[
            ("", "全部"),
            ("2020s", "2020 年代"),
            ("2010s", "2010 年代"),
            ("2000s", "2000 年代"),
            ("earlier", "更早"),
        ],
    ),
    (
        "magazine_id",
        "杂志",
        &[
            ("", "全部"),
            ("10659", "週刊少年ジャンプ"),
            ("10752", "少年ジャンプ＋"),
            ("10678", "週刊少年サンデー"),
            ("10702", "週刊少年マガジン"),
            ("10855", "カドコミ"),
            ("11315", "アルファポリス電網浮遊都市"),
            ("10860", "コミックDAYS"),
            ("10724", "週刊少年チャンピオン"),
            ("10673", "モーニング"),
            ("10786", "ガンガンONLINE"),
            ("10754", "週刊ヤングマガジン"),
            ("10687", "週刊ヤングジャンプ"),
        ],
    ),
];

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.hikarinagi.koma",
    name: "Hikarinagi",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "zh-Hans",
    author: "Koma",
    description: "Hikarinagi 的公开漫画目录。阅读依原站网页登录。",
    content_rating: "unknown",
};
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    listings: true,
    manga_list: true,
    filters: true,
    image_request: true,
    ..SourceCapabilities::CORE
};

#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;
koma_source_sdk::koma_source_buffers! {payload:1024*1024,http_out:5*1024*1024,body:4*1024*1024,http_req:4096,scratch:8192,}
koma_source_sdk::koma_source_helpers!();

#[link(wasm_import_module = "koma_host")]
extern "C" {
    fn random_bytes(out: *mut u8, capacity: u32) -> i32;
}

fn id_arg(args: &Value, name: &str) -> Result<u64> {
    args[name]
        .as_str()
        .and_then(|id| id.parse::<u64>().ok())
        .filter(|id| *id > 0)
        .ok_or("invalid_request")
}

fn api(url: &str, navigation: bool) -> Result<Value> {
    let accept = if navigation {
        "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"
    } else {
        "application/json"
    };
    let data = port::request(
        url,
        "GET",
        json!({"Accept":accept,"Referer":format!("{BASE}/")}),
        None,
    )?;
    serde_json::from_str(&data).map_err(|_| "parse_error")
}

fn card(item: &Value, req: &[u8]) -> Result<Value> {
    let id = item["id"].as_u64().ok_or("parse_error")?.to_string();
    let title = item["name_cn"]
        .as_str()
        .or_else(|| item["name"].as_str())
        .ok_or("parse_error")?;
    let status = match item["serial_status"].as_str() {
        Some("SERIALIZING") => "ongoing",
        Some("FINISHED") => "completed",
        _ => "unknown",
    };
    let mut manga = json!({"id":id,"title":title,"language":"zh-Hans","status":status,
        "links":[{"kind":"source","url":format!("{BASE}/mangas/{id}")}]});
    if let Some(description) = item["summary_cn"].as_str().filter(|text| !text.trim().is_empty())
        .or_else(|| item["summary"].as_str().filter(|text| !text.trim().is_empty())) {
        manga["description"] = json!(description);
    }
    if let Some(src) = item["covers"][0]["media"]["src"]
        .as_str()
        .filter(|s| !s.is_empty())
    {
        let cover = port::absolute(CDN, src)?;
        manga["cover"] =
            json!({"kind":"url","url":cover.as_str(),"headers":{"Referer":format!("{BASE}/")}});
    }
    if koma_source_sdk::source_protocol::feature(req, koma_source_sdk::list_info::HOST_FEATURE) {
        if let Some(stamp) = item["latest_chapter_at"]
            .as_str()
            .map(|s| s.chars().take(10).collect::<String>())
            .filter(|s| !s.is_empty())
        {
            manga["listInfo"] =
                json!([{"type":"text","label":"最新","text":stamp}]);
        }
    }
    Ok(manga)
}

fn search_current(query: &str, page: usize) -> Result<Value> {
    // The current site's browse-search middleware forwards keywords to
    // /search?types=manga&q=..., whose public response keeps its ranked order.
    let mut url = port::absolute(BASE, "/api/pages/search")?;
    {
        let mut params = url.query_pairs_mut();
        params.append_pair("q", query).append_pair("types", "manga");
        if page > 1 {
            params.append_pair("page", &page.to_string());
        }
    }
    let data = api(url.as_str(), true)?;
    let list = &data["list"];
    let items = list["items"].as_array().ok_or("parse_error")?.iter().map(|hit| {
        if hit["type"] != "manga" { return Err("parse_error"); }
        let id = hit["id"].as_u64().ok_or("parse_error")?.to_string();
        let title = hit["title"].as_str().ok_or("parse_error")?;
        let mut card = json!({"id":id,"title":title,"language":"zh-Hans",
            "links":[{"kind":"source","url":format!("{BASE}/mangas/{id}")}]});
        if let Some(src) = hit["cover"]["src"].as_str().filter(|s|!s.is_empty()) {
            let cover = port::absolute(CDN,src)?;
            card["cover"] = json!({"kind":"url","url":cover.as_str(),"headers":{"Referer":format!("{BASE}/")}});
        }
        Ok(card)
    }).collect::<Result<Vec<_>>>()?;
    Ok(port::items(items, next_page(list, page)?))
}

fn next_page(list: &Value, page: usize) -> Result<Option<usize>> {
    let current = list["meta"]["page"].as_u64().ok_or("parse_error")?;
    let total = list["meta"]["total_pages"].as_u64().ok_or("parse_error")?;
    Ok(if current < total {
        page.checked_add(1)
    } else {
        None
    })
}

fn browse(input: &Value, request: &[u8], search: bool) -> Result<Value> {
    let page = request_page_number(request).max(1);
    let args = &input["args"];
    let query = args["query"].as_str().unwrap_or_default().trim();
    if search && !query.is_empty() {
        // Old deployments may only have the fixed upstream browse endpoint.
        // A permission error on the current route retains its original status.
        match search_current(query, page) {
            Err("not_found") => {}
            result => return result,
        }
    }
    let mut url = port::absolute(BASE, "/api/pages/mangas/browse")?;
    {
        let mut params = url.query_pairs_mut();
        params
            .append_pair("page", &page.to_string())
            .append_pair("page_size", "24");
        if search && !query.is_empty() {
            params.append_pair("search", query);
        }
        for (name, _, choices) in FILTERS {
            // The declared default for a select is empty, even when its
            // choices (such as sort) only contain explicit values.
            let supplied = if args["filters"].get(*name).and_then(Value::as_str) == Some("") {
                ""
            } else {
                port::filter(args, name, choices)?
            };
            let value = if supplied.is_empty() && *name == "sort" {
                if !search && args["listingId"] == "popular" {
                    "heat:desc"
                } else {
                    "latest_chapter_at:desc"
                }
            } else {
                supplied
            };
            if !value.is_empty() {
                params.append_pair(name, value);
                if *name == "decade" {
                    let years = match value {
                        "2020s" => "2020-2029",
                        "2010s" => "2010-2019",
                        "2000s" => "2000-2009",
                        "earlier" => "1900-1999",
                        _ => return Err("invalid_request"),
                    };
                    params.append_pair("years", years);
                }
            }
        }
    }
    let result = api(url.as_str(), true)?;
    let list = &result["list"];
    let items = list["items"]
        .as_array()
        .ok_or("parse_error")?
        .iter()
        .map(|item| card(item, request))
        .collect::<Result<Vec<_>>>()?;
    Ok(port::items(items, next_page(list, page)?))
}

fn details(args: &Value) -> Result<Value> {
    let id = id_arg(args, "mangaId")?;
    let data = api(&format!("{BASE}/api/pages/mangas/{id}"), false)?;
    if data["manga"]["id"] != id {
        return Err("not_found");
    }
    let mut manga = card(&data["manga"], &[])?;
    let people = data["people"].as_array().ok_or("parse_error")?;
    let person = |role: &str| {
        people
            .iter()
            .rev()
            .find(|row| row["role"] == role)
            .and_then(|row| {
                row["person"]["trans_name"]
                    .as_str()
                    .or_else(|| row["person"]["name"].as_str())
            })
    };
    if let Some(author) = person("ORIGINAL_CREATOR").or_else(|| person("AUTHOR")) {
        manga["authors"] = json!([author]);
    }
    if let Some(artist) = person("ART") {
        manga["artists"] = json!([artist]);
    }
    manga["tags"] = json!(data["tags"]
        .as_array()
        .ok_or("parse_error")?
        .iter()
        .filter_map(|row| row["tag"]["name"].as_str())
        .collect::<Vec<_>>());
    Ok(json!({"manga":manga}))
}

fn normalize_chapter_number(value: &str) -> Option<String> {
    let value = value.trim();
    let (sign, unsigned) = value.strip_prefix('-').map(|rest| ("-", rest)).unwrap_or(("", value));
    let end = unsigned.find(|c| matches!(c, '.' | 'e' | 'E')).unwrap_or(unsigned.len());
    let integer = &unsigned[..end];
    if integer.is_empty() || !integer.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let integer = integer.trim_start_matches('0');
    let integer = if integer.is_empty() { "0" } else { integer };
    let result = format!("{sign}{integer}{}", &unsigned[end..]);
    DecimalChapterNumber::new(result.as_bytes())?;
    Some(result)
}

fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(request)?;
    let id = id_arg(&input["args"], "mangaId")?;
    let data = api(&format!("{BASE}/api/pages/mangas/{id}"), false)?;
    if data["manga"]["id"] != id {
        return Err("not_found");
    }
    let updated = data["manga"]["latest_chapter_at"].as_str();
    let manga_id = id.to_string();
    let mut group_positions = BTreeMap::<String, u64>::new();
    let rows = data["chapters"]
        .as_array()
        .ok_or("parse_error")?
        .iter()
        .map(|row| {
            let id = row["id"].as_u64().ok_or("parse_error")?.to_string();
            let number = row["chapter_number"].as_str();
            let normalized_number = number.and_then(normalize_chapter_number);
            let volume = row["volume_number"].as_u64().and_then(|n| usize::try_from(n).ok());
            let title = row["title"]
                .as_str()
                .filter(|text| !text.trim().is_empty())
                .map(str::to_string)
                .or_else(|| number.map(|n| format!("第{n}话")))
                .or_else(|| match row["chapter_type"].as_str() {
                    Some("VOLUME") => row["volume_number"].as_u64()
                        .map(|n| format!("第{n}卷"))
                        .or_else(|| row["volume_number"].as_str().map(|n| format!("第{n}卷"))),
                    Some("EXTRA") => row["ordinal"].as_u64().map(|n| format!("番外 {n}")),
                    _ => None,
                })
                .unwrap_or_default();
            let pages = row["page_count"]
                .as_u64()
                .and_then(|n| usize::try_from(n).ok());
            let group = match row["chapter_type"].as_str() {
                Some("SERIALIZATION") => "连载".to_string(),
                Some("VOLUME") => "单行本".to_string(),
                Some("EXTRA") => "番外".to_string(),
                Some(other) if !other.is_empty() => other.to_string(),
                _ => "其他".to_string(),
            };
            let position = *group_positions.get(&group).unwrap_or(&0);
            group_positions.insert(group.clone(), position + 1);
            Ok((id, title, normalized_number, volume, pages, group, position))
        })
        .collect::<Result<Vec<_>>>()?;
    // The API orders each chapter type oldest first, but groups serialization,
    // volumes and extras separately. Never join those groups into one reader
    // sequence merely because their entries share an array.
    let mut catalog = GroupedChapters::new(payload_buf(), request).map_err(port::chapter_error)?;
    for (index, (chapter, title, number, volume, pages, group, position)) in rows.iter().rev().enumerate() {
        let mut out = Chapter::new(chapter.as_bytes(), manga_id.as_bytes(), title.as_bytes());
        out.index = Some(index);
        out.chapter_number = number.as_deref()
            .and_then(|n| DecimalChapterNumber::new(n.as_bytes()))
            .map(ChapterNumber::Decimal);
        out.volume_number = volume.map(ChapterNumber::Integer);
        out.page_count = *pages;
        out.scanlator = Some(group.as_bytes());
        out.published_at = updated.map(str::as_bytes);
        catalog.push(out, ChapterPlacement::InGroup { group: group.as_bytes(), position: *position })
            .map_err(port::chapter_error)?;
    }
    catalog.finish(ChapterPage::Complete).map_err(port::chapter_error)
}

fn pages(args: &Value) -> Result<Value> {
    let work = id_arg(args, "mangaId")?;
    let chapter = id_arg(args, "chapterId")?;
    let data = api(
        &format!("{BASE}/api/pages/mangas/reader/{work}/{chapter}"),
        false,
    )?;
    let pages = data["manifest"]["pages"].as_array().ok_or("parse_error")?.iter().enumerate().map(|(index,page)| {
        let pid = page["id"].as_str().filter(|s|!s.is_empty() && s.len() <= 512).ok_or("parse_error")?;
        let mime = page["mime_type"].as_str().ok_or("parse_error")?;
        let mut url = port::absolute(BASE,&format!("{CONTENT_PATH}{work}/chapters/{chapter}/pages/"))?;
        url.path_segments_mut().map_err(|_|"parse_error")?.pop_if_empty().push(pid).push("content");
        url.set_fragment(Some(mime));
        Ok(json!({"id":format!("{chapter}:{pid}"),"index":index,"image":{"kind":"url","url":url.as_str()}}))
    }).collect::<Result<Vec<_>>>()?;
    Ok(json!({"pages":pages}))
}

fn image(args: &Value, request: &[u8]) -> Result<Value> {
    let mut url = Url::parse(args["url"].as_str().ok_or("invalid_request")?)
        .map_err(|_| "invalid_request")?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("invalid_request");
    }
    let Some(path) = url.path().strip_prefix(CONTENT_PATH) else {
        return Ok(json!({"url":url.as_str(),"headers":{"Referer":format!("{BASE}/")}}));
    };
    if url.origin().ascii_serialization() != BASE || url.query().is_some() {
        return Err("invalid_request");
    }
    let pieces = path.split('/').collect::<Vec<_>>();
    let [work, "chapters", chapter, "pages", pid, "content"] = pieces.as_slice() else {
        return Err("invalid_request");
    };
    if work.parse::<u64>().is_err() || chapter.parse::<u64>().is_err() || pid.is_empty() {
        return Err("invalid_request");
    }

    // Reuse the host's scoped HTTP and body references: POST and decryption
    // remain source-owned, and no credentials or bytes enter page metadata.
    if ![b"source_http_body_v1".as_slice(), b"source_body_v1", b"source_image_body_v1"]
        .iter().all(|feature| source_protocol::feature(request, feature)) {
        return Err("host_feature_required");
    }
    let pid = percent_decode_str(pid).decode_utf8().map_err(|_| "invalid_request")?;
    let aad = format!("manga:page:{work}:{chapter}:{pid}");
    url.set_fragment(None);
    let mut token = [0u8; 64];
    if unsafe { random_bytes(token.as_mut_ptr(), token.len() as u32) } != token.len() as i32 {
        return Err("internal_error");
    }
    let mut key = [0u8; 32];
    for i in 0..key.len() { key[i] = token[i] ^ token[i + 32]; }
    let envelope = json!({"version":1,"method":"POST","url":url.as_str(),
        "headers":{"Origin":BASE,"Referer":format!("{BASE}/"),"Content-Type":"application/json; charset=utf-8"},
        "body":json!({"p":URL_SAFE_NO_PAD.encode(token)}).to_string(),
        "responseKind":"bodyRef","timeoutMs":60000});
    let encoded = serde_json::to_vec(&envelope).map_err(|_| "internal_error")?;
    let response = koma_source_sdk::http::Client::<koma_source_sdk::http::CallError>::new(&[])
        .send(koma_source_sdk::http::Request::from_json(&encoded), http_out())
        .map_err(|_| "network_error")?;
    match response.status() {
        Some(200..=299) => (),
        Some(401) => return Err("auth_required"),
        Some(403) => return Err("forbidden"),
        Some(429) => return Err("rate_limit"),
        Some(500..=599) => return Err("server_error"),
        _ => return Err("network_error"),
    }
    let input = response.body_ref()?;
    drop(response);
    let decoded = body_decode::decrypt(&input, &key, aad.as_bytes());
    let closed = input.close();
    let output = decoded?;
    if let Err(error) = closed { let _ = output.close(); return Err(error); }
    let n = output.image_result(scratch_a())?;
    serde_json::from_slice(&scratch_a()[..n]).map_err(|_| "internal_error")
}

fn dispatch(op: &str, request: &[u8]) -> Result<Value> {
    let input = port::input(request)?;
    let args = &input["args"];
    match op {
        "get_listings" => Ok(json!({"listings":[
            {"id":"popular","name":"热门","supportsPaging":true},
            {"id":"latest","name":"最近更新","supportsPaging":true}
        ]})),
        "get_filters" => Ok(json!({"filters":FILTERS.iter().map(|(id,name,choices)|{
            let mut filter = port::select(id,name,"browse",choices);
            if !source_protocol::feature(request,b"source_filter_scope_header_v1") {
                filter.as_object_mut().unwrap().remove("scope");
            }
            // An untouched filter preserves each listing's own default sort.
            filter
        }).collect::<Vec<_>>()})),
        "get_manga_list" => browse(&input, request, false),
        "search" => browse(&input, request, true),
        "get_manga" => details(args),
        "get_pages" => pages(args),
        "get_image_request" => image(args, request),
        "get_authentication" => {
            Ok(json!({"authentication":{"kind":"web","loginUrl":LOGIN},"authenticated":false}))
        }
        _ => Err("not_implemented"),
    }
}
fn run(op: &str, request: &[u8]) -> u32 {
    match dispatch(op, request) {
        Err("host_feature_required") if op == "get_image_request" => write_error(
            op,
            "host_feature_required",
            "当前客户端不支持此书源所需的图片数据处理。",
        ),
        result => port::write_result(op, result),
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
koma_source_sdk::koma_source_exports!("hikarinagi", typed_chapters, chapter_failure);
koma_source_sdk::koma_source_authentication_exports!("hikarinagi");
