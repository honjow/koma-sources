#![no_std]
extern crate alloc;

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use koma_source_sdk::{
    host,
    json_utils::request_page_number,
    source::{SourceCapabilities, SourceInfo},
    source_protocol as protocol,
};
use miniz_oxide::{deflate::compress_to_vec_zlib, inflate::decompress_to_vec_zlib_with_limit};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use url::Url;

mod chapters;

#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

const BASE: &str = "https://bx.yaimer.com";
const REFERER: &str = "https://bx.yaimer.com/";
const API: &str = "https://bx.yaimer.com/api/manga/common";
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0";
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.utifa.koma",
    name: "Utifa",
    version: "0.1.0",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "Utifa 中文漫画。",
    content_rating: "unknown",
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
koma_source_sdk::koma_source_buffers! {payload:1024*1024,http_out:2*1024*1024,body:1024*1024,http_req:196608,scratch:8192,}
koma_source_sdk::koma_source_helpers!();
koma_source_sdk::koma_source_exports!("utifa_source");

type Result<T> = core::result::Result<T, &'static str>;

fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value[key].as_str().ok_or("parse_error")
}

fn positive(text: &str) -> Result<u64> {
    let value = text.parse::<u64>().map_err(|_| "invalid_request")?;
    if value == 0 || value > i64::MAX as u64 || value.to_string() != text {
        return Err("invalid_request");
    }
    Ok(value)
}

fn provider_id(value: &Value, key: &str) -> Result<u64> {
    value[key]
        .as_i64()
        .filter(|id| *id > 0)
        .map(|id| id as u64)
        .ok_or("parse_error")
}

fn manga_id(text: &str) -> Result<u64> {
    positive(text.strip_prefix("utifa:").ok_or("invalid_request")?)
}

fn manga_route(mid: u64) -> String {
    format!("{BASE}/manga/detail/{mid}")
}

fn chapter_route(cid: u64) -> String {
    format!("{BASE}/manga/chapter/{cid}")
}

fn chapter_id(mid: u64, cid: u64) -> String {
    format!("utifa-read:{mid}:{cid}")
}

fn chapter_owner(args: &Value) -> Result<(u64, u64)> {
    let id = string(args, "chapterId").map_err(|_| "invalid_request")?;
    let (book, chapter) = id
        .strip_prefix("utifa-read:")
        .and_then(|value| value.split_once(':'))
        .ok_or("invalid_request")?;
    let mid = positive(book)?;
    let cid = positive(chapter)?;
    if args
        .get("mangaId")
        .is_some_and(|m| m.as_str() != Some(&format!("utifa:{mid}")))
    {
        return Err("invalid_request");
    }
    Ok((mid, cid))
}

fn call(envelope: &Value) -> Result<usize> {
    if host::check_cancel() {
        return Err("cancelled");
    }
    let bytes = serde_json::to_vec(envelope).map_err(|_| "invalid_request")?;
    if bytes.len() > http_req_buf().len() {
        return Err("request_too_large");
    }
    host::http_request(&bytes, http_out()).map_err(|_| "network_error")
}

fn fetch(endpoint: &str, body: Option<Value>) -> Result<Value> {
    let mut request = json!({"version":1,"url":format!("{API}/{endpoint}"),"method":"GET",
        "headers":{"Accept":"application/json","Origin":BASE,"Referer":REFERER,"User-Agent":UA},
        "timeoutMs":15000,"responseKind":"bodyText"});
    if let Some(body) = body {
        request["method"] = json!("POST");
        request["headers"]["Content-Type"] = json!("application/json");
        // Koma transports send the selected request body as literal text.
        request["body"] = json!(serde_json::to_string(&body).map_err(|_| "invalid_request")?);
    }
    let length = call(&request)?;
    let length = protocol::body(&http_out()[..length], body_buf())?;
    let mut response: Value =
        serde_json::from_slice(&body_buf()[..length]).map_err(|_| "parse_error")?;
    let data = response["data"].take();
    if response["code"].as_i64() != Some(1000) || data.is_null() {
        return Err("parse_error");
    }
    Ok(data)
}

fn public_url(url: &str, prefix: &str) -> Result<Url> {
    let url = Url::parse(url).map_err(|_| "invalid_request")?;
    if url.scheme() != "https"
        || url.host_str() != Some("bx.yaimer.com")
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || !url.path().starts_with(prefix)
        || url.path().len() <= prefix.len()
    {
        return Err("invalid_request");
    }
    Ok(url)
}

fn card(data: &Value, full: bool) -> Result<Value> {
    let mid = provider_id(data, "id")?;
    let route = manga_route(mid);
    let mut manga = json!({"id":format!("utifa:{mid}"),"title":string(data,"title")?,
        "contentRating":"unknown","url":route});
    if full {
        manga["links"] = json!([{"kind":"source","url":route}]);
    }
    if let Some(relative) = data["cover"].as_str().filter(|value| !value.is_empty()) {
        let cover = public_url(&format!("{BASE}/api{relative}"), "/api/media/common/")
            .map_err(|_| "parse_error")?;
        manga["cover"] = json!({"kind":"url","url":cover.as_str(),"headers":{"Referer":REFERER}});
    }
    if let Some(value) = data["description"].as_str() {
        manga["description"] = json!(value);
    }
    if let Some(value) = data["author"].as_str() {
        manga["authors"] = json!([value]);
    }
    if let Some(tags) = data["genres"].as_array() {
        manga["tags"] = json!(tags);
    }
    manga["status"] = json!(match data["updateStatus"].as_str() {
        Some("0") => "ongoing",
        Some("1") => "completed",
        _ => "unknown",
    });
    Ok(manga)
}

fn detail(mid: u64) -> Result<Value> {
    let data = fetch(&format!("get?id={mid}"), None)?;
    if provider_id(&data, "id")? != mid {
        return Err("parse_error");
    }
    Ok(data)
}

fn filter<'a>(args: &'a Value, key: &str) -> Result<Option<&'a str>> {
    match args["filters"].get(key) {
        None => Ok(None),
        Some(Value::String(value)) if value.len() <= 128 => Ok(Some(value)),
        _ => Err("invalid_request"),
    }
}

fn list(request: &[u8], args: &Value, search: bool) -> Result<Value> {
    let page = request_page_number(request);
    if page > 100000 {
        return Err("invalid_request");
    }
    let listing = args["listingId"].as_str().unwrap_or("popular");
    let sort = if search {
        filter(args, "orderBy")?.unwrap_or("views")
    } else {
        match listing {
            "popular" => "views",
            "latest" => "updateTime",
            _ => return Err("invalid_request"),
        }
    };
    let direction = if search {
        filter(args, "order")?.unwrap_or("desc")
    } else {
        "desc"
    };
    if !matches!(direction, "asc" | "desc") {
        return Err("invalid_request");
    }
    let mut body = json!({"pageNum":page,"pageSize":24,"orderBy":sort,"order":direction});
    if search {
        let query = args["query"].as_str().unwrap_or("").trim();
        if let Ok(link) = Url::parse(query) {
            if matches!(link.scheme(), "https" | "http") {
                let link = public_url(query, "/manga/detail/").map_err(|_| "not_implemented")?;
                let mid = positive(
                    link.path()
                        .trim_end_matches('/')
                        .strip_prefix("/manga/detail/")
                        .unwrap_or(""),
                )?;
                let item = card(&detail(mid)?, true)?;
                return Ok(json!({"items":[item],"page":{"hasMore":false}}));
            }
        }
        if !query.is_empty() {
            body["keyword"] = json!(query);
        }
        for key in ["theme", "grade", "updateStatus"] {
            if let Some(value) =
                filter(args, key)?.filter(|value| !value.is_empty() && *value != "all")
            {
                body[key] = json!(value);
            }
        }
    }
    let result = fetch("list", Some(body))?;
    let current = result["current"].as_u64().ok_or("parse_error")?;
    let total = result["pages"].as_u64().ok_or("parse_error")?;
    if current != page as u64 {
        return Err("parse_error");
    }
    let items = result["records"]
        .as_array()
        .ok_or("parse_error")?
        .iter()
        .map(|m| card(m, false))
        .collect::<Result<Vec<_>>>()?;
    let next = (current < total).then(|| (current + 1).to_string());
    Ok(json!({"items":items,"page":{"hasMore":next.is_some(),"nextCursor":next}}))
}

fn filters(request: &[u8], args: &Value) -> Result<Value> {
    let mut items = Vec::new();
    let enabled = protocol::feature(request, b"source_filter_fetching_v1");
    let fetch_data = enabled && args["fetchFilterData"].as_bool() == Some(true);
    if fetch_data {
        let data = fetch("categoryTree", None)?;
        for group in data.as_array().ok_or("parse_error")? {
            let key = string(group, "key")?;
            let default = match key {
                "theme" | "grade" | "updateStatus" => "all",
                "orderBy" => "views",
                _ => continue,
            };
            let options = group["value"]
                .as_array()
                .ok_or("parse_error")?
                .iter()
                .map(|o| Ok(json!({"value":string(o,"value")?,"label":string(o,"label")?})))
                .collect::<Result<Vec<_>>>()?;
            if options.is_empty()
                || items
                    .iter()
                    .any(|item: &Value| item["id"].as_str() == Some(key))
            {
                return Err("parse_error");
            }
            let selected = if options
                .iter()
                .any(|item| item["value"].as_str() == Some(default))
            {
                default
            } else {
                string(&options[0], "value")?
            };
            items.push(json!({"id":key,"name":string(group,"label")?,"type":"select","default":selected,"options":options}));
        }
    } else if enabled {
        for (id, name, default) in [
            ("theme", "题材", "all"),
            ("grade", "分级", "all"),
            ("updateStatus", "进度", "all"),
            ("orderBy", "排序", "views"),
        ] {
            items.push(json!({"id":id,"name":name,"type":"select","default":default}));
        }
    }
    if enabled {
        // Koma uses two existing selects to express the reference's field/direction pair.
        items.push(
            json!({"id":"order","name":"顺序","type":"select","default":"desc","options":[
                {"value":"desc","label":"降序"},{"value":"asc","label":"升序"}
            ]}),
        );
    }
    Ok(json!({"filters":items,"supportsFilterFetching":enabled}))
}

fn page_url(value: &str) -> Result<(Url, u64)> {
    let url = public_url(value, "/api/drive/file/stream/")?;
    let mut expiry = None;
    let mut scope = false;
    let mut signature = false;
    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "se" if expiry.is_none() => expiry = Some(positive(&value)?),
            "scope" if !scope && !value.is_empty() => scope = true,
            "sig" if !signature && !value.is_empty() => signature = true,
            "se" | "scope" | "sig" => return Err("invalid_request"),
            _ => {}
        }
    }
    if !scope || !signature {
        return Err("invalid_request");
    }
    Ok((url, expiry.ok_or("invalid_request")?))
}

fn page_urls(mid: u64, cid: u64) -> Result<Vec<String>> {
    let result = fetch(&format!("getChapterList?chapterId={cid}"), None)?;
    if provider_id(&result, "mangaId")? != mid || provider_id(&result, "chapterId")? != cid {
        return Err("invalid_request");
    }
    let pages = result["chapters"].as_array().ok_or("parse_error")?;
    if pages.is_empty() {
        return Err("chapter_unavailable");
    }
    pages
        .iter()
        .map(|page| {
            let candidate = format!("{BASE}/api{}", string(page, "url")?);
            let (url, _) = page_url(&candidate).map_err(|_| "parse_error")?;
            Ok(url.to_string())
        })
        .collect()
}

fn page_identity(mid: u64, cid: u64, index: usize, path: &str) -> String {
    format!(
        "utifa-page:{mid}:{cid}:{index}:{:x}",
        Sha256::digest(path.as_bytes())
    )
}

fn get_pages(args: &Value) -> Result<Value> {
    let (mid, cid) = chapter_owner(args)?;
    let pages = page_urls(mid,cid)?.into_iter().enumerate().map(|(index,url)| {
        let (parsed, _) = page_url(&url)?;
        Ok(json!({"id":page_identity(mid,cid,index,parsed.path()),"index":index,"image":{"kind":"url","url":url}}))
    }).collect::<Result<Vec<_>>>()?;
    Ok(json!({"pages":pages}))
}

fn session(request: Value) -> Result<Value> {
    let length = call(&request)?;
    serde_json::from_slice(&http_out()[..length]).map_err(|_| "network_error")
}

fn rpc(op: &str) -> Result<Value> {
    let response = session(json!({"type":"private_session","version":1,"op":op}))?;
    if response["ok"].as_bool() != Some(true) {
        return Err("network_error");
    }
    Ok(response)
}

fn now() -> Result<u64> {
    let response = rpc("clock")?;
    Ok(string(&response, "nowMs")?
        .parse::<u64>()
        .map_err(|_| "network_error")?
        / 1000)
}

fn matching_image(urls: &[String], path: &str, time: u64) -> Result<String> {
    for url in urls {
        let (parsed, expiry) = page_url(url).map_err(|_| "parse_error")?;
        if parsed.path() == path && expiry > time {
            return Ok(url.clone());
        }
    }
    Err("chapter_unavailable")
}

fn refreshed_image(mid: u64, cid: u64, path: &str, time: u64) -> Result<String> {
    let process = rpc("process_scope")?;
    let tag = string(&process, "processTag")?;
    for _ in 0..2 {
        let mut begin = rpc("begin")?;
        if begin["state"] == "ready" {
            let cached: Value =
                serde_json::from_str(string(&begin, "value")?).map_err(|_| "network_error")?;
            if cached["process"].as_str() == Some(tag)
                && cached["mangaId"].as_u64() == Some(mid)
                && cached["chapterId"].as_u64() == Some(cid)
                && cached["until"].as_u64().is_some_and(|until| until > time)
            {
                let bytes = STANDARD
                    .decode(string(&cached, "images")?)
                    .map_err(|_| "network_error")?;
                let restored = decompress_to_vec_zlib_with_limit(&bytes, 1024 * 1024)
                    .map_err(|_| "network_error")?;
                let urls = serde_json::from_slice::<Vec<String>>(&restored)
                    .map_err(|_| "network_error")?;
                return matching_image(&urls, path, time);
            }
            begin = session(json!({"type":"private_session","version":1,"op":"begin",
                "replaceVersionTag":string(&begin,"versionTag")?}))?;
            if begin["error"]["code"] == "stale_version" {
                continue;
            }
        }
        if begin["ok"] != true || begin["state"] != "owner" {
            return Err("network_error");
        }
        let lease = string(&begin, "lease")?;
        let result = (|| {
            let urls = page_urls(mid, cid)?;
            let selected = matching_image(&urls, path, time)?;
            // Keep the reference's 25-minute cache, bounded by the server's
            // own expiry when it issues a shorter-lived table.
            let until = urls
                .iter()
                .try_fold(time.saturating_add(25 * 60), |until, url| {
                    page_url(url).map(|(_, expiry)| until.min(expiry))
                })?;
            let encoded = serde_json::to_vec(&urls).map_err(|_| "parse_error")?;
            let value = json!({"process":tag,"mangaId":mid,"chapterId":cid,"until":until,
                "images":STANDARD.encode(compress_to_vec_zlib(&encoded,6))})
            .to_string();
            if value.len() > 65536 {
                return Err("result_too_large");
            }
            let result = session(
                json!({"type":"private_session","version":1,"op":"commit","lease":lease,"value":value}),
            )?;
            if result["ok"] != true {
                return Err("network_error");
            }
            Ok(selected)
        })();
        if result.is_err() {
            // Host also releases the call's lease on cancellation before commit.
            let _ =
                session(json!({"type":"private_session","version":1,"op":"abort","lease":lease}));
        }
        return result;
    }
    Err("network_error")
}

fn image(request: &[u8], args: &Value) -> Result<Value> {
    let page_id = string(args, "pageId").map_err(|_| "invalid_request")?;
    let raw = string(args, "url").map_err(|_| "invalid_request")?;
    if page_id.starts_with("utifa:") {
        let mid = manga_id(page_id)?;
        if args.get("chapterId").is_some()
            || args
                .get("mangaId")
                .is_some_and(|m| m.as_str() != Some(page_id))
            || args
                .get("mangaUrl")
                .is_some_and(|url| url.as_str() != Some(&manga_route(mid)))
        {
            return Err("invalid_request");
        }
        return Ok(
            json!({"url":public_url(raw,"/api/media/common/")?.as_str(),"headers":{"Referer":REFERER}}),
        );
    }
    let mut parts = page_id
        .strip_prefix("utifa-page:")
        .ok_or("invalid_request")?
        .split(':');
    let mid = positive(parts.next().ok_or("invalid_request")?)?;
    let cid = positive(parts.next().ok_or("invalid_request")?)?;
    let index = parts
        .next()
        .and_then(|v| v.parse::<usize>().ok())
        .ok_or("invalid_request")?;
    let (url, expiry) = page_url(raw)?;
    if page_id != page_identity(mid, cid, index, url.path())
        || args
            .get("mangaId")
            .is_some_and(|m| m.as_str() != Some(&format!("utifa:{mid}")))
        || args
            .get("chapterId")
            .is_some_and(|v| v.as_str() != Some(&chapter_id(mid, cid)))
        || args
            .get("mangaUrl")
            .is_some_and(|v| v.as_str() != Some(&manga_route(mid)))
    {
        return Err("invalid_request");
    }
    if !protocol::feature(request, b"source_private_session_v1") {
        return Err("unsupported");
    }
    let time = now()?;
    let resolved = if expiry > time {
        url.to_string()
    } else {
        if !protocol::feature(request, b"source_private_session_replace_v1")
            || !protocol::feature(request, b"source_private_session_process_v1")
        {
            return Err("unsupported");
        }
        // Refresh only a site-declared expired ticket, never a generic 403.
        refreshed_image(mid, cid, url.path(), time)?
    };
    Ok(json!({"url":resolved,"headers":{"Referer":REFERER}}))
}

fn run(op: &str, request: &[u8]) -> u32 {
    let result = (|| {
        protocol::request(request)?;
        let raw_args = protocol::direct(request, b"args")?.unwrap_or(request);
        let mut owner = [0; 16384];
        let _ = protocol::text(raw_args, b"mangaUrl", &mut owner)?;
        let parsed: Value = serde_json::from_slice(raw_args).map_err(|_| "invalid_request")?;
        match op {
            "get_listings" => Ok(
                json!({"listings":[{"id":"popular","name":"热门","supportsPaging":true},
                {"id":"latest","name":"最新","supportsPaging":true}]}),
            ),
            "get_filters" => filters(request, &parsed),
            "get_manga_list" => list(request, &parsed, false),
            "search" => list(request, &parsed, true),
            "get_manga" => Ok(
                json!({"manga":card(&detail(manga_id(string(&parsed,"mangaId").map_err(|_| "invalid_request")?)?)?,true)?}),
            ),
            "get_chapters" => chapters::catalogue(manga_id(
                string(&parsed, "mangaId").map_err(|_| "invalid_request")?,
            )?),
            "get_pages" => get_pages(&parsed),
            "get_image_request" => image(request, &parsed),
            _ => Err("not_implemented"),
        }
    })();
    match result {
        Ok(value) => match serde_json::to_vec(&value) {
            Ok(bytes) if bytes.len() <= payload_buf().len() => {
                payload_buf()[..bytes.len()].copy_from_slice(&bytes);
                write_success_payload(op, bytes.len())
            }
            _ => write_error(op, "result_too_large", "书源结果过大。"),
        },
        Err(code) => write_error(
            op,
            code,
            match code {
                "forbidden" => "网站拒绝了本次请求，请打开原网页确认。",
                "auth_required" => "网站要求登录，请打开原网页确认访问权限。",
                "not_found" => "网站未提供此内容，请打开原网页确认。",
                "rate_limit" => "请求太频繁，请稍后重试。",
                "chapter_unavailable" => "本章图片暂时无法取得，请打开原网页确认。",
                "unsupported" => "请更新 Koma 后再打开此书源。",
                "network_error" => "网络传输未完成，请稍后重试。",
                _ => "获取漫画失败，请稍后重试或打开原网页确认。",
            },
        ),
    }
}

fn run_search(r: &[u8]) -> u32 {
    run("search", r)
}
fn run_get_manga(r: &[u8]) -> u32 {
    run("get_manga", r)
}
fn run_get_chapters(r: &[u8]) -> u32 {
    run("get_chapters", r)
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
