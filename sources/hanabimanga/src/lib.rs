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
mod provider_config;
use port_support::{self as port, Result};
use provider_config::ANONYMOUS_TOKEN;

const API: &str = "https://uhkvqrxmcapgtpspglrp.moedot.net";
const WEB: &str = "https://web.hanabimanga.com";
const LOGIN: &str = "https://web.hanabimanga.com/zh-CN/auth/login";
const COMIC_FIELDS:&str="id,title,summary,cover_url,release_date,is_finished,authors,region,latest_chapter_title,tags(id,name),categories(id,name)";
const CHAPTER_FIELDS: &str = "chapters(id,idx,title,image_count,category,updated_at)";
const PAGE_SIZE: usize = 20;
const RANK: &[(&str, &str)] = &[
    ("popularity_daily.desc.nullslast", "日榜"),
    ("popularity_weekly.desc.nullslast", "周榜"),
    ("popularity_monthly.desc.nullslast", "月榜"),
];
const SORT: &[(&str, &str)] = &[
    ("id.asc.nullslast", "默认"),
    ("rating_average.desc.nullslast", "评分最高"),
    ("updated_at.desc.nullslast", "最近更新"),
    ("created_at.desc.nullslast", "最新上架"),
];
const CATEGORY: &[(&str, &str)] = &[
    ("", "全部"),
    ("1", "推理"),
    ("2", "后宫"),
    ("3", "科幻"),
    ("4", "百合"),
    ("5", "恐怖"),
    ("6", "恋爱"),
    ("7", "音乐"),
    ("8", "校园"),
    ("9", "穿越"),
    ("10", "战斗"),
    ("11", "运动"),
    ("12", "武侠"),
    ("13", "奇幻"),
    ("14", "惊悚"),
    ("15", "搞笑"),
    ("16", "日常"),
    ("17", "悬疑"),
    ("18", "冒险"),
    ("19", "历史"),
    ("20", "乙女"),
    ("21", "美食"),
    ("22", "职场"),
    ("23", "玄幻"),
    ("24", "机战"),
    ("26", "魔幻"),
    ("27", "伪娘"),
];
const REGION: &[(&str, &str)] = &[
    ("", "全部"),
    ("jp", "日漫"),
    ("kr", "韩漫"),
    ("us", "美漫"),
    ("others", "其他"),
];
const STATUS: &[(&str, &str)] = &[("", "全部"), ("false", "连载中"), ("true", "已完结")];

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.hanabimanga.koma",
    name: "花火漫画",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "花火首页主推中文日漫，能找到《孤独摇滚！》《败犬女主太多了！》《赛马娘：芦毛灰姑娘》，还有按日、周、月更新的人气榜。",
    content_rating: "mixed",
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
koma_source_sdk::koma_source_buffers! {payload:1024*1024,http_out:4*1024*1024,body:3*1024*1024,http_req:4096,scratch:8192,}
koma_source_sdk::koma_source_helpers!();

fn direct_json(url: &Url, headers: Value, body: Option<Value>) -> Result<Value> {
    let body = body.map(|body| body.to_string());
    let raw = port::request(
        url.as_str(),
        if body.is_some() { "POST" } else { "GET" },
        headers,
        body.as_deref(),
    )?;
    serde_json::from_str(&raw).map_err(|_| "parse_error")
}
fn api(url: Url, body: Option<Value>) -> Result<Value> {
    let mut headers = json!({"Accept":"application/json","apikey":ANONYMOUS_TOKEN});
    if body.is_some() {
        headers["Content-Type"] = json!("application/json");
    }
    direct_json(&url, headers, body)
}
fn numeric(value: &str) -> Result<u64> {
    value.parse::<u64>().map_err(|_| "invalid_request")
}
fn manga_link(id: u64) -> String {
    format!("{WEB}/zh-CN/comic/{id}")
}
fn tag_values(row: &Value) -> Vec<&str> {
    let mut tags = row["tags"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v["name"].as_str())
        .collect::<Vec<_>>();
    if let Some(category) = row["categories"]["name"].as_str() {
        tags.push(category);
    }
    match row["region"].as_str() {
        Some("jp") => tags.push("日漫"),
        Some("kr") => tags.push("韩漫"),
        Some("us") => tags.push("美漫"),
        _ => {}
    }
    tags
}
fn card(row: &Value, req: &[u8]) -> Result<Value> {
    let id = row["id"].as_u64().ok_or("parse_error")?;
    let title = row["title"]
        .as_str()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or("parse_error")?;
    let mut result =
        json!({"id":id.to_string(),"title":title,"links":[{"kind":"source","url":manga_link(id)}]});
    if let Some(url) = row["cover_url"].as_str().filter(|v| !v.is_empty()) {
        result["cover"] =
            json!({"kind":"url","url":port::absolute(WEB,url)?.as_str(),"headers":{}});
    }
    if let Some(authors) = row["authors"].as_array() {
        result["authors"] = json!(authors.iter().filter_map(Value::as_str).collect::<Vec<_>>());
    }
    if let Some(summary) = row["summary"].as_str() {
        result["description"] = json!(summary.trim());
    }
    if let Some(done) = row["is_finished"].as_bool() {
        result["status"] = json!(if done { "completed" } else { "ongoing" });
    }
    result["tags"] = json!(tag_values(row));
    // Optional list summary: the catalogue row already carries the latest chapter
    // title. Only surface it when the host negotiates the feature; otherwise the
    // item bytes stay unchanged.
    if koma_source_sdk::source_protocol::feature(req, koma_source_sdk::list_info::HOST_FEATURE) {
        if let Some(lc) = row["latest_chapter_title"]
            .as_str()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            result["listInfo"] = json!([{"type":"text","label":"最新","text":lc}]);
        }
    }
    Ok(result)
}
fn comic(id: u64, columns: &str) -> Result<Value> {
    let mut url = port::absolute(API, "/rest/v1/comics")?;
    url.query_pairs_mut()
        .extend_pairs([("id", format!("eq.{id}").as_str()), ("select", columns)]);
    let mut payload = api(url, None)?;
    let rows = payload.as_array_mut().ok_or("parse_error")?;
    if rows.len() != 1 {
        return Err("not_found");
    }
    Ok(rows.remove(0))
}
fn get_manga(args: &Value, req: &[u8]) -> Result<Value> {
    let id = numeric(args["mangaId"].as_str().ok_or("invalid_request")?)?;
    let comic = comic(id, COMIC_FIELDS)?;
    let mut row = card(&comic, req)?;
    if let Some(category) = comic["categories"]["id"].as_u64() {
        let body =
            json!({"p_limit":9,"p_category_id":category,"p_exclude_comic_id":id.to_string()});
        if let Ok(Value::Array(related)) = api(
            port::absolute(API, "/rest/v1/rpc/get_random_comics")?,
            Some(body),
        ) {
            row["relatedManga"] = json!(related
                .iter()
                .filter_map(|r| card(r, req).ok())
                .collect::<Vec<_>>());
        }
    }
    Ok(json!({"manga":row}))
}
fn list(input: &Value, req: &[u8], search: bool) -> Result<Value> {
    let args = &input["args"];
    let query = args["query"].as_str().unwrap_or("").trim();
    let page = request_page_number(req);
    let rows = if search && !query.is_empty() {
        let data = json!({"search_term":query,"items_per_page":PAGE_SIZE.to_string(),"page_number":page.to_string()});
        api(
            port::absolute(API, "/rest/v1/rpc/search_comics_pgroonga")?,
            Some(data),
        )?
    } else {
        let mut url = port::absolute(API, "/rest/v1/comics")?;
        let order = if search {
            let value = port::filter(args, "sort", SORT)?;
            if value.is_empty() {
                SORT[0].0
            } else {
                value
            }
        } else if args["listingId"] == "latest" {
            "updated_at.desc.nullslast"
        } else {
            let selected = input["settings"]["rank"].as_str().unwrap_or(RANK[0].0);
            if !RANK.iter().any(|(id, _)| *id == selected) {
                return Err("invalid_request");
            }
            selected
        };
        let offset = page
            .saturating_sub(1)
            .checked_mul(PAGE_SIZE)
            .ok_or("invalid_request")?;
        url.query_pairs_mut().extend_pairs([
            ("select", COMIC_FIELDS),
            ("order", order),
            ("offset", offset.to_string().as_str()),
            ("limit", "20"),
        ]);
        if search {
            for (id, api_name, options) in [
                ("category", "category_id", CATEGORY),
                ("region", "region", REGION),
                ("status", "is_finished", STATUS),
            ] {
                let value = port::filter(args, id, options)?;
                if !value.is_empty() {
                    url.query_pairs_mut()
                        .append_pair(api_name, &format!("eq.{value}"));
                }
            }
        }
        api(url, None)?
    };
    let rows = rows.as_array().ok_or("parse_error")?;
    let next = if rows.len() == PAGE_SIZE {
        page.checked_add(1)
    } else {
        None
    };
    Ok(port::items(
        rows.iter().map(|row| card(row, req)).collect::<Result<Vec<_>>>()?,
        next,
    ))
}
fn run_get_chapters(req: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(req)?;
    let raw = input["args"]["mangaId"].as_str().ok_or("invalid_request")?;
    let id = numeric(raw)?;
    let info = comic(id, &format!("id,{CHAPTER_FIELDS}"))?;
    let mut rows = info["chapters"]
        .as_array()
        .ok_or("parse_error")?
        .iter()
        .map(|row| {
            let chapter = row["id"].as_u64().ok_or("parse_error")?;
            let idx = row["idx"].as_u64().ok_or("parse_error")?;
            let title = row["title"].as_str().ok_or("parse_error")?;
            let category = row["category"].as_str().unwrap_or("");
            let stable = format!("{id}:{chapter}:{idx}");
            Ok((
                stable,
                idx,
                category,
                title,
                row["updated_at"].as_str(),
                row["image_count"].as_u64(),
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    rows.sort_by(|a, b| {
        (a.2 == "volume")
            .cmp(&(b.2 == "volume"))
            .then(b.1.cmp(&a.1))
    });
    let chapters = rows
        .iter()
        .enumerate()
        .map(|(index, (stable, _, group, title, updated, size))| {
            let mut row = Chapter::new(stable.as_bytes(), raw.as_bytes(), title.as_bytes());
            row.index = Some(index);
            row.published_at = updated.map(str::as_bytes);
            row.page_count = size.and_then(|size| usize::try_from(size).ok());
            row.scanlator = match *group {
                "normal" => Some("连载".as_bytes()),
                "special" => Some("特典番外".as_bytes()),
                "volume" => Some("单行本".as_bytes()),
                _ => None,
            };
            row
        })
        .collect::<Vec<_>>();
    ChapterResponse::ordered(payload_buf(), req, ChapterOrder::NewestFirst, &chapters)
        .map_err(port::chapter_error)
}
fn chapter_identity(stable: &str) -> Result<(u64, u64, u64)> {
    let mut parts = stable.split(':');
    let result = (
        numeric(parts.next().ok_or("invalid_request")?)?,
        numeric(parts.next().ok_or("invalid_request")?)?,
        numeric(parts.next().ok_or("invalid_request")?)?,
    );
    if parts.next().is_some() {
        return Err("invalid_request");
    }
    Ok(result)
}
fn config(response: &Value) -> Result<Option<String>> {
    let Some(ticket) = response["metadata"]
        .get("scrambleInfo")
        .filter(|v| !v.is_null())
    else {
        return Ok(None);
    };
    let token = ticket["ticket"].as_str().ok_or("parse_error")?;
    let nonce = ticket["nonce"].as_str().ok_or("parse_error")?;
    let cols = ticket["cols"].as_u64().ok_or("parse_error")?;
    let rows = ticket["rows"].as_u64().ok_or("parse_error")?;
    if token.is_empty()
        || token.len() > 5500
        || nonce.len() != 16
        || cols == 0
        || rows == 0
        || !token
            .bytes()
            .chain(nonce.bytes())
            .all(|b| b.is_ascii_alphanumeric() || b"+/=".contains(&b))
    {
        return Err("parse_error");
    }
    Ok(Some(format!("{token}|{nonce}|{cols}|{rows}")))
}
fn reader_json(url: &Url, referer: &str) -> Result<Value> {
    use koma_source_sdk::{host, source_protocol};

    if host::check_cancel() {
        return Err("cancelled");
    }
    let request = json!({
        "version": 1, "method": "GET", "url": url.as_str(),
        "headers": {"Accept":"application/json", "Referer":referer},
        "responseKind":"bodyText", "timeoutMs":15000,
    });
    let bytes = serde_json::to_vec(&request).map_err(|_| "internal_error")?;
    let length = host::http_request(&bytes, http_out()).map_err(|_| "network_error")?;
    let envelope = &http_out()[..length];
    let response: Value = serde_json::from_slice(envelope).map_err(|_| "network_error")?;
    if response["status"] == 429 || response["statusCode"] == 429 {
        if let Some(code) = response["bodyText"]
            .as_str()
            .and_then(|body| serde_json::from_str::<Value>(body).ok())
            .and_then(|body| body["code"].as_str().map(str::to_string))
        {
            match code.as_str() {
                "ANON_QUOTA_EXCEEDED" => return Err("auth_required"),
                "FREE_QUOTA_EXCEEDED" => return Err("hanabi_enhanced_quota"),
                _ => {}
            }
        }
    }
    let size = source_protocol::body(envelope, body_buf())?;
    serde_json::from_slice(&body_buf()[..size]).map_err(|_| "parse_error")
}
fn pages(input: &Value) -> Result<Value> {
    let args = &input["args"];
    let stable = args["chapterId"].as_str().ok_or("invalid_request")?;
    let (cid, chapter, idx) = chapter_identity(stable)?;
    if args["mangaId"]
        .as_str()
        .is_some_and(|v| numeric(v) != Ok(cid))
    {
        return Err("invalid_request");
    }
    let mut url = port::absolute(WEB, &format!("/api/reader/comic/{cid}/chapter-{idx}"))?;
    if matches!(input["settings"]["enhanced"], Value::Bool(true))
        || input["settings"]["enhanced"] == "true"
    {
        url.query_pairs_mut().append_pair("quality", "hd");
    }
    let referer = format!("{WEB}/zh-CN/comic/{cid}/chapter-{idx}");
    // Use the provider's browser reader. The website keeps quota, login and
    // purchase checks, and returns its own URLs and optional tile ticket.
    let response = reader_json(&url, &referer)?;
    if response["chapter"]["comicId"].as_u64() != Some(cid)
        || response["chapter"]["chapterId"].as_u64() != Some(chapter)
    {
        return Err("chapter_unavailable");
    }
    let ticket = config(&response)?;
    let list = response["pages"].as_array().ok_or("parse_error")?;
    if list.is_empty() {
        return Err("chapter_unavailable");
    }
    if response["chapter"]["totalPages"].as_u64() != Some(list.len() as u64) {
        return Err("parse_error");
    }
    let pages = list
        .iter()
        .enumerate()
        .map(|(index, row)| {
            if row["index"].as_u64() != Some(index as u64) {
                return Err("parse_error");
            }
            let mut url = port::absolute(WEB, row["url"].as_str().ok_or("parse_error")?)?;
            url.set_fragment(ticket.as_deref());
            Ok(port::page(index, stable, url))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(json!({"pages":pages}))
}
fn image(input: &Value) -> Result<Value> {
    let mut url = port::absolute(WEB, input["args"]["url"].as_str().ok_or("invalid_request")?)?;
    let ticket = url.fragment().map(str::to_string);
    url.set_fragment(None);
    let mut result = json!({"url":url.as_str(),"headers":{}});
    if let Some(ticket) = ticket {
        if !input["hostFeatures"].as_array().is_some_and(|x| {
            x.iter()
                .any(|v| v == "source_required_image_decode_wasm_v1")
        }) {
            return Err("unsupported_feature");
        }
        result["requiredImageDecode"] = json!({"kind":"wasm_image_v1","pixels":ticket});
        result["headers"] = json!({"Referer":format!("{WEB}/")});
    }
    Ok(result)
}
fn dispatch(input: &Value, op: &str, req: &[u8]) -> Result<Value> {
    match op {
        "get_listings" => Ok(port::listing_list(true, true, false)),
        "get_filters" => Ok(json!({"filters":[
            port::select("category","分类","search",CATEGORY),port::select("sort","排序","search",SORT),
            port::select("region","分区","search",REGION),port::select("status","状态","search",STATUS)],
            "useSearchForFilters":true})),
        "get_settings" => Ok(
            json!({"settings":[{"id":"rank","name":"热门排行","kind":"select","default":RANK[0].0,
            "options":RANK.iter().map(|(v,n)|json!({"value":v,"label":n})).collect::<Vec<_>>()},
            {"id":"enhanced","name":"AI 超分辨率","kind":"toggle","default":"false"}]}),
        ),
        "get_manga_list" => list(input, req, false),
        "search" => list(input, req, true),
        "get_manga" => get_manga(&input["args"], req),
        "get_pages" => pages(input),
        "get_image_request" => image(input),
        "get_authentication" => {
            Ok(json!({"authentication":{"kind":"web","loginUrl":LOGIN},"authenticated":false}))
        }
        _ => Err("not_implemented"),
    }
}
fn run(op: &str, req: &[u8]) -> u32 {
    match port::input(req).and_then(|input| dispatch(&input, op, req)) {
        Err("hanabi_enhanced_quota") => write_error(op, "rate_limit", "今日超分额度已用完。"),
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
koma_source_sdk::koma_source_exports!("hanabimanga", typed_chapters, chapter_failure);
koma_source_sdk::koma_source_authentication_exports!("hanabimanga");

fn load_comments(request: &[u8]) -> Result<Value> {
    let query = koma_source_sdk::comments::request(request)?;
    let input = port::input(request)?;
    if input["args"]["scope"] != "manga" {
        return Err("invalid_request");
    }
    let id = numeric(input["args"]["mangaId"].as_str().ok_or("invalid_request")?)?;
    let offset: usize = query.cursor.map_or(Ok(0), |cursor| {
        core::str::from_utf8(cursor)
            .map_err(|_| "invalid_request")?
            .parse::<usize>()
            .ok()
            .filter(|offset| *offset % PAGE_SIZE == 0)
            .ok_or("invalid_request")
    })?;
    let mut url = port::absolute(API, "/rest/v1/comment_public_view")?;
    url.query_pairs_mut().extend_pairs([
        ("select", "id,display_name,content,is_spoiler,created_at"),
        ("comic_id", format!("eq.{id}").as_str()),
        ("parent_id", "is.null"),
        ("order", "created_at.desc"),
        ("offset", offset.to_string().as_str()),
        ("limit", "21"),
    ]);
    let response = api(url, None)?;
    let rows = response.as_array().ok_or("parse_error")?;
    let items = rows
        .iter()
        .take(PAGE_SIZE)
        .map(|row| {
            let id = row["id"].as_u64().ok_or("parse_error")?.to_string();
            let author = row["display_name"].as_str().unwrap_or("未署名");
            let content = if row["is_spoiler"] == true {
                "原站已隐藏剧透内容，可在原站查看。"
            } else {
                row["content"].as_str().ok_or("parse_error")?.trim()
            };
            let mut item = json!({"id":id,"author":author,"content":content});
            if let Some(time) = row["created_at"].as_str() {
                item["createdAt"] = json!(time);
            }
            Ok(item)
        })
        .collect::<Result<Vec<_>>>()?;
    let mut result = json!({"items":items});
    if rows.len() > PAGE_SIZE {
        result["nextCursor"] = json!(offset
            .checked_add(PAGE_SIZE)
            .ok_or("invalid_request")?
            .to_string());
    }
    Ok(result)
}

fn run_get_comments(request: &[u8]) -> u32 {
    port::write_result("get_comments", load_comments(request))
}
koma_source_sdk::koma_source_comments_export!(run_get_comments);
