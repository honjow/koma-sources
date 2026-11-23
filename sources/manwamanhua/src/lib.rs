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
    chapters::{Chapter, ChapterOrder, ChapterResponse, ChapterResult},
    json_utils::request_page_number,
    source::{SourceCapabilities, SourceInfo},
};
use serde_json::{json, Value};
use url::Url;

#[path = "../../port_support.rs"]
mod port_support;
use port_support::{self as port, Node, Result};

const WEB: &str = "https://manwacn.cc";
const MIRRORS: &[&str] = &[
    "manwadz.cc",
    "manwauu.cc",
    "manwamm.cc",
    "manwawz.cc",
    "manwabb.cc",
    "manwaxu.cc",
    "mwmwbu.cc",
    "mwmwff.cc",
    "manwarr.cc",
    "manwayy.cc",
    "manwacn.cc",
    "manwahh.cc",
];
// Keiyoushi's public reader key, encoded as UTF-8 hex for the shared CBC processor.
const IMAGE_KEY: &str = "30423636363641302d424235392d313338312d423734362d6130453443394143";
const CHANNELS: &[(&str, &str)] = &[
    ("0", "全部"),
    ("1", "一般向"),
    ("2", "BL向"),
    ("3", "TL向"),
    ("5", "GL向"),
    ("4", "禁漫"),
];
const FIELDS: &[(&str, &str)] = &[("keyword", "标题"), ("author", "作者"), ("tags", "标签")];
const SORTS: &[(&str, &str)] = &[("0", "更新"), ("1", "新作"), ("3", "热门")];
const STATES: &[(&str, &str)] = &[("-1", "全部"), ("0", "连载中"), ("1", "已完结")];
const AREAS: &[(&str, &str)] = &[
    ("0", "全部"),
    ("2", "韩国"),
    ("3", "日漫"),
    ("4", "国漫"),
    ("5", "台漫"),
    ("6", "其他"),
    ("1", "未分类"),
];
const LEVELS: &[(&str, &str)] = &[("0", "全部"), ("1", "普通"), ("2", "VIP无码")];
const DAYS: &[(&str, &str)] = &[
    ("0", "全部"),
    ("1", "周一"),
    ("2", "周二"),
    ("3", "周三"),
    ("4", "周四"),
    ("5", "周五"),
    ("6", "周六"),
    ("7", "周日"),
];

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.manwamanhua.koma",
    name: "漫蛙漫画",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "漫蛙收錄韓漫、日漫、國漫與台漫，設有一般向、BL、GL 及 TL 等作品分類。",
    content_rating: "nsfw",
};
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    listings: true,
    manga_list: true,
    settings: true,
    filters: true,
    image_request: true,
    ..SourceCapabilities::CORE
};
#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;
koma_source_sdk::koma_source_buffers! {
    payload: 1024 * 1024, http_out: 4 * 1024 * 1024, body: 3 * 1024 * 1024,
    http_req: 4096, scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

fn numeric(id: &str) -> Result<&str> {
    (!id.is_empty() && id.len() <= 16 && id.bytes().all(|b| b.is_ascii_digit()))
        .then_some(id)
        .ok_or("invalid_request")
}
fn text(value: &str) -> String {
    if !value.contains('&') {
        return value.trim().to_string();
    }
    Node::parse(&format!("<span>{value}</span>"))
        .and_then(|doc| doc.require("span")?.text())
        .unwrap_or_else(|_| value.trim().to_string())
}
fn selected<'a>(
    args: &'a Value,
    key: &str,
    options: &[(&str, &str)],
    default: &'a str,
) -> Result<&'a str> {
    let value = port::filter(args, key, options)?;
    Ok(if value.is_empty() { default } else { value })
}
fn request(input: &Value, url: Url, referer: &str, body: Option<Value>) -> Result<String> {
    let channel = input["settings"]["channel"].as_str().unwrap_or("0");
    if !CHANNELS.iter().any(|(id, _)| *id == channel) {
        return Err("invalid_request");
    }
    let mut headers = json!({
        "Accept":"application/json",
        "Referer":format!("{WEB}{referer}"),
        "X-Requested-With":"XMLHttpRequest",
        "Cookie":format!("typeId={channel}")
    });
    if let Some(body) = body {
        headers["Content-Type"] = json!("application/json");
        return port::request(url.as_str(), "POST", headers, Some(&body.to_string()));
    }
    port::request(url.as_str(), "GET", headers, None)
}
fn api(input: &Value, url: Url, referer: &str, body: Option<Value>) -> Result<Value> {
    let raw = request(input, url, referer, body)?;
    serde_json::from_str(&raw).map_err(|_| "parse_error")
}
fn data(input: &Value, path: &str, referer: &str) -> Result<Value> {
    let mut response = api(input, port::absolute(WEB, path)?, referer, None)?;
    response
        .as_object_mut()
        .and_then(|x| x.remove("data"))
        .filter(|x| !x.is_null())
        .ok_or("parse_error")
}
fn card(row: &Value) -> Result<Value> {
    let id = if let Some(id) = row["id"].as_u64() {
        id.to_string()
    } else {
        let route = port::absolute(WEB, row["url"].as_str().ok_or("parse_error")?)?;
        numeric(
            route
                .path()
                .trim_end_matches('/')
                .rsplit('/')
                .next()
                .ok_or("parse_error")?,
        )?
        .to_string()
    };
    let title = text(row["title"].as_str().ok_or("parse_error")?);
    if title.is_empty() {
        return Err("parse_error");
    }
    let mut item = json!({"id":id,"title":title,"links":[{"kind":"source","url":format!("{WEB}/comic/{id}")}]});
    if let Some(raw) = row["cover"]
        .as_str()
        .or_else(|| row["pic"].as_str())
        .filter(|s| !s.is_empty())
    {
        let image = port::absolute(WEB, raw)?;
        item["cover"] = if encrypted(&image) {
            json!({"kind":"url","url":image.as_str()})
        } else {
            json!({"kind":"url","url":image.as_str(),"headers":{}})
        };
    }
    if let Some(author) = row["author"].as_str() {
        item["authors"] = json!(author
            .split([',', '，'])
            .map(text)
            .filter(|v| !v.is_empty())
            .collect::<Vec<_>>());
    }
    if let Some(tags) = row["tags"].as_str() {
        item["tags"] = json!(tags
            .split(',')
            .map(text)
            .filter(|v| !v.is_empty())
            .collect::<Vec<_>>());
    }
    Ok(item)
}
fn work(input: &Value, id: &str) -> Result<Value> {
    let id = numeric(id)?;
    let provider = format!("/comic/{id}");
    let row = data(input, &format!("/api{provider}"), &provider)?;
    let mut item = card(&row)?;
    let mut description = row["intro"].as_str().map(text).unwrap_or_default();
    if let Some(alias) = row["alias"].as_str().filter(|v| !v.trim().is_empty()) {
        description.push_str(&format!("\n\n别名：{}", text(alias)));
    }
    item["description"] = json!(description.trim());
    item["status"] = json!(match row["status"].as_i64() {
        Some(1) => "ongoing",
        Some(0) => "completed",
        _ => "unknown",
    });
    if let Some(area) = row["areaId"].as_i64().and_then(|id| match id {
        2 => Some("韩漫"),
        3 => Some("日漫"),
        4 => Some("国漫"),
        5 => Some("台漫"),
        _ => None,
    }) {
        if let Some(tags) = item["tags"].as_array_mut() {
            if !tags.iter().any(|t| t == area) {
                tags.insert(0, json!(area));
            }
        } else {
            item["tags"] = json!([area]);
        }
    }

    // Related works are optional. A failed recommendation request cannot hide the opened title.
    if let Ok(Value::Array(rows)) = api(
        input,
        port::absolute(WEB, &format!("/api{provider}/recommends"))?,
        &provider,
        None,
    ) {
        item["relatedManga"] = json!(rows
            .iter()
            .filter_map(|row| card(row).ok())
            .collect::<Vec<_>>());
    }
    Ok(item)
}
fn list(input: &Value, request_bytes: &[u8], search: bool) -> Result<Value> {
    let args = &input["args"];
    let page = request_page_number(request_bytes);
    let query = args["query"].as_str().unwrap_or("").trim();
    let (payload, count) = if search && !query.is_empty() {
        if let Ok(url) = Url::parse(query) {
            let id = url
                .path()
                .trim_end_matches('/')
                .strip_prefix("/comic/")
                .ok_or("not_found")?;
            if !url.host_str().is_some_and(|host| MIRRORS.contains(&host)) {
                return Err("not_found");
            }
            return Ok(port::items(vec![work(input, id)?], None));
        }
        let count = 20;
        let field = selected(args, "field", FIELDS, "keyword")?;
        let mut url = port::absolute(WEB, "/api/search")?;
        url.query_pairs_mut().extend_pairs([
            ("type", "mh"),
            ("page", page.to_string().as_str()),
            ("pageSize", "20"),
            (field, query),
        ]);
        (api(input, url, "/search/", None)?, count)
    } else {
        let count = 36;
        let mut path = String::from("/api/cate/");
        if let Some(tags) = args["filters"]["tags"].as_array() {
            let tags = tags
                .iter()
                .map(|tag| {
                    let tag = tag.as_str().ok_or("invalid_request")?;
                    (!tag.is_empty()
                        && tag.len() <= 64
                        && tag.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'))
                    .then_some(tag)
                    .ok_or("invalid_request")
                })
                .collect::<Result<Vec<_>>>()?;
            path.push_str(&tags.join("-"));
        }
        let fallback = if args["listingId"] == "latest" {
            "0"
        } else {
            "3"
        };
        let field = |key, options, default| {
            selected(args, key, options, default)?
                .parse::<i32>()
                .map_err(|_| "invalid_request")
        };
        let body = json!({"page":{"page":page,"pageSize":count},"sort":field("sort",SORTS,fallback)?,
            "comic":{"status":field("status",STATES,"-1")?,"day":field("day",DAYS,"0")?,
            "level":field("level",LEVELS,"0")?,"areaId":field("area",AREAS,"0")?}});
        (
            api(input, port::absolute(WEB, &path)?, "/cate/", Some(body))?,
            count,
        )
    };
    let result = &payload["data"];
    let rows = result["list"].as_array().ok_or("parse_error")?;
    let total = usize::try_from(result["total"].as_u64().ok_or("parse_error")?)
        .map_err(|_| "parse_error")?;
    let next = if page.saturating_mul(count) < total && !rows.is_empty() {
        page.checked_add(1)
    } else {
        None
    };
    Ok(port::items(
        rows.iter().map(card).collect::<Result<Vec<_>>>()?,
        next,
    ))
}
fn drop_trailing_note(title: &str) -> &str {
    let end = title.trim_end();
    if end.ends_with([')', '）']) {
        if let Some(pos) = end.rfind(['(', '（']) {
            return end[..pos].trim_end();
        }
    }
    title
}
fn taiwan_title(title: &str) -> String {
    let Some(start) = title.find(['(', '（']) else {
        return title.to_string();
    };
    let Some(end) = title[start..].find([')', '）']) else {
        return title.to_string();
    };
    if !title[start..start + end].contains("台版") {
        return title.to_string();
    }
    let end = start + end;
    format!(
        "{}{}",
        &title[..start],
        &title[end + title[end..].chars().next().unwrap().len_utf8()..]
    )
}
fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(request)?;
    let id = numeric(input["args"]["mangaId"].as_str().ok_or("invalid_request")?)?;
    let provider = format!("/comic/{id}");
    let payload = data(&input, &format!("/api{provider}/chapters"), &provider)?;
    let mut rows = payload["list"]
        .as_array()
        .ok_or("parse_error")?
        .iter()
        .map(|row| {
            let chapter = row["id"].as_u64().ok_or("parse_error")?;
            let owner = row["comicId"].as_u64().ok_or("parse_error")?;
            if owner.to_string() != id {
                return Err("parse_error");
            }
            let title = text(row["title"].as_str().ok_or("parse_error")?);
            let sort = row["sortId"].as_f64().ok_or("parse_error")?;
            let updated = row["createTime"].as_u64().and_then(|v| v.checked_mul(1000));
            Ok((
                sort,
                chapter,
                format!("{provider}/{chapter}"),
                title,
                updated,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    rows.sort_by(|a, b| b.0.total_cmp(&a.0).then(b.1.cmp(&a.1)));
    let originals = rows
        .iter()
        .filter(|row| !row.3.contains("台版"))
        .flat_map(|row| [row.3.as_str(), drop_trailing_note(&row.3)])
        .collect::<BTreeSet<_>>();
    let dedupe = matches!(input["settings"]["dedupe_taiwan"], Value::Bool(true))
        || input["settings"]["dedupe_taiwan"] == "true";
    let mut chapters = Vec::new();
    for (_, _, route, title, date) in &rows {
        let taiwan = title.contains("台版");
        if dedupe && taiwan && originals.contains(taiwan_title(title).as_str()) {
            continue;
        }
        let mut chapter = Chapter::new(route.as_bytes(), id.as_bytes(), title.as_bytes());
        chapter.index = Some(chapters.len());
        chapter.scanlator = taiwan.then_some("台版".as_bytes());
        chapter.updated_at_millis = *date;
        chapters.push(chapter);
    }
    ChapterResponse::ordered(payload_buf(), request, ChapterOrder::NewestFirst, &chapters)
        .map_err(port::chapter_error)
}
fn pages(input: &Value) -> Result<Value> {
    let args = &input["args"];
    let route = args["chapterId"].as_str().ok_or("invalid_request")?;
    let path = route.strip_prefix("/comic/").ok_or("invalid_request")?;
    let (owner, id) = path.split_once('/').ok_or("invalid_request")?;
    let id = numeric(id)?;
    let owner = numeric(owner)?;
    if args["mangaId"].as_str().is_some_and(|id| id != owner) {
        return Err("invalid_request");
    }
    let mut all = Vec::new();
    let mut current = 1usize;
    loop {
        let mut url = port::absolute(WEB, &format!("/api/comic/image/{id}"))?;
        url.query_pairs_mut().extend_pairs([
            ("page", current.to_string().as_str()),
            ("page_size", "100"),
            ("image_source", ""),
        ]);
        let response = api(input, url, route, None)?;
        let data = &response["data"];
        for image in data["images"].as_array().ok_or("parse_error")? {
            if image["width"] == -1 {
                continue;
            }
            let url = port::absolute(WEB, image["url"].as_str().ok_or("parse_error")?)?;
            all.push(port::page(all.len(), route, url));
        }
        let end = data["pagination"]["total_pages"]
            .as_u64()
            .ok_or("parse_error")?;
        if current as u64 >= end {
            break;
        }
        if current >= 50 {
            return Err("result_too_large");
        }
        current += 1;
    }
    if all.is_empty() {
        return Err("chapter_unavailable");
    }
    Ok(json!({"pages":all}))
}
fn encrypted(url: &Url) -> bool {
    url.path().starts_with("/en_images/")
}
fn image(input: &Value) -> Result<Value> {
    let url = port::absolute(WEB, input["args"]["url"].as_str().ok_or("invalid_request")?)?;
    let mut result = json!({"url":url.as_str(),"headers":{}});
    if encrypted(&url) {
        if !input["hostFeatures"].as_array().is_some_and(|v| {
            v.iter()
                .any(|f| f == "source_required_image_decode_wasm_v1")
        }) {
            return Err("unsupported_feature");
        }
        result["requiredImageDecode"] = json!({"kind":"wasm_image_v1","bytes":IMAGE_KEY});
    }
    Ok(result)
}
fn has(input: &Value, feature: &str) -> bool {
    input["hostFeatures"]
        .as_array()
        .is_some_and(|features| features.iter().any(|f| f == feature))
}
fn filters(input: &Value) -> Result<Value> {
    let mut fields = vec![
        port::select("field", "搜索范围", "search", FIELDS),
        port::select("sort", "排序", "browse", SORTS),
        port::select("status", "状态", "browse", STATES),
        port::select("area", "地区", "browse", AREAS),
        port::select("level", "等级", "browse", LEVELS),
        port::select("day", "更新日", "browse", DAYS),
    ];
    for (field, value) in fields.iter_mut().zip(["keyword", "3", "-1", "0", "0", "0"]) {
        field["default"] = json!(value);
    }
    let fetch = has(input, "source_filter_fetching_v1");
    if fetch {
        let mut option =
            json!({"type":"multiselect","id":"tags","name":"标签","scope":"browse","default":[]});
        if input["args"]["fetchFilterData"] == true {
            let doc = Node::parse(&port::get(&format!("{WEB}/cate/"))?)?;
            let tags = doc
                .all(".ctag a[href^='/cate/'][data-value]")?
                .iter()
                .filter_map(|a| {
                    let href = a.attr("href").ok()?;
                    let tag = href.trim_end_matches('/').rsplit('/').next()?;
                    let title = a.text().ok()?;
                    (!tag.is_empty() && !title.is_empty())
                        .then(|| json!({"value":tag,"label":title}))
                })
                .collect::<Vec<_>>();
            option["options"] = json!(tags);
        }
        fields.push(option);
    }
    Ok(json!({"filters":fields,"supportsFilterFetching":fetch}))
}
fn run(op: &str, request: &[u8]) -> u32 {
    let result = port::input(request).and_then(|input| match op {
        "get_listings" => Ok(port::listing_list(true,true,true)),
        "get_filters" => filters(&input),
        "get_settings" => Ok(json!({"settings":[{"id":"channel","name":"内容类型","kind":"select","default":"0",
            "options":CHANNELS.iter().map(|(value,label)| json!({"value":value,"label":label})).collect::<Vec<_>>()},
            {"id":"dedupe_taiwan","name":"隐藏重复的台版章节","kind":"toggle","default":"false"}]})),
        "get_manga_list" => list(&input, request, false),
        "search" => list(&input, request, true),
        "get_manga" => work(&input,input["args"]["mangaId"].as_str().ok_or("invalid_request")?).map(|manga|json!({"manga":manga})),
        "get_pages" => pages(&input),
        "get_image_request" => image(&input),
        _ => Err("not_implemented"),
    });
    port::write_result(op, result)
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
fn chapter_failure(code: &str) -> u32 {
    port::write_failure("get_chapters", code)
}
koma_source_sdk::koma_source_exports!("manwamanhua", typed_chapters, chapter_failure);

fn load_comments(request: &[u8]) -> Result<Value> {
    let query = koma_source_sdk::comments::request(request)?;
    let input = port::input(request)?;
    if input["args"]["scope"] != "manga" {
        return Err("invalid_request");
    }
    let id = numeric(input["args"]["mangaId"].as_str().ok_or("invalid_request")?)?;
    let page: u32 = query.cursor.map_or(Ok(1), |cursor| {
        core::str::from_utf8(cursor)
            .map_err(|_| "invalid_request")?
            .parse::<u32>()
            .ok()
            .filter(|page| *page > 0)
            .ok_or("invalid_request")
    })?;
    let mut url = port::absolute(WEB, "/api/comments")?;
    url.query_pairs_mut().extend_pairs([
        ("comic_id", id),
        ("page", page.to_string().as_str()),
        ("page_size", "20"),
        ("sort", "time"),
    ]);
    let payload = api(&input, url, &format!("/comic/{id}"), None)?;
    if payload["code"] != 200 {
        return Err(match payload["code"].as_u64() {
            Some(401) => "auth_required",
            Some(403) => "forbidden",
            Some(429) => "rate_limit",
            _ => "parse_error",
        });
    }
    let data = &payload["data"];
    let rows = data["list"].as_array().ok_or("parse_error")?;
    let items = rows
        .iter()
        .map(|row| {
            let id = row["id"].as_u64().ok_or("parse_error")?.to_string();
            let user = &row["user"];
            let author = user["nickname"]
                .as_str()
                .filter(|v| !v.trim().is_empty())
                .or_else(|| user["username"].as_str())
                .unwrap_or("未署名");
            let content = row["content"].as_str().ok_or("parse_error")?;
            let mut item = json!({"id":id,"author":author,"content":content.trim()});
            if let Some(time) = row["created_at"].as_str() {
                item["createdAt"] = json!(time);
            }
            Ok(item)
        })
        .collect::<Result<Vec<_>>>()?;
    let total = data["total"].as_u64().ok_or("parse_error")?;
    let mut result = json!({"items":items});
    if !rows.is_empty() && u64::from(page) * 20 < total {
        result["nextCursor"] = json!(page.checked_add(1).ok_or("invalid_request")?.to_string());
    }
    Ok(result)
}

fn run_get_comments(request: &[u8]) -> u32 {
    port::write_result("get_comments", load_comments(request))
}
koma_source_sdk::koma_source_comments_export!(run_get_comments);
