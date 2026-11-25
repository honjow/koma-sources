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
    source_protocol,
};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde_json::{json, Value};
use url::Url;

#[path = "../../port_support.rs"]
mod port;
use port::{Node, Result};

const MIRRORS: [&str; 2] = ["https://boylove.cc", "https://boylove4.xyz"];
const COVER_ORIGINS: [&str; 2] = ["https://img.boylove.cc", "https://img.boylove4.xyz"];
const READER_CDN: &str = "https://blcnimghost2.cc";
const WORK_PATH: &str = "/home/book/index/id/";
const CHAPTER_PATH: &str = "/home/book/capter/id/";
const PARTS_PARAM: &str = "scrambled_parts_count";
const STATES: &[(&str, &str)] = &[("2", "全部"), ("0", "连载中"), ("1", "已完结")];
const TYPES: &[(&str, &str)] = &[("0", "全部"), ("1", "清水"), ("2", "有肉")];
const REGIONS: &[(&str, &str)] = &[
    ("0", "全部"),
    ("日漫", "日漫"),
    ("韩漫", "韩漫"),
    ("国漫", "国漫"),
    ("台漫", "台漫"),
];
const VIP: &[(&str, &str)] = &[("2", "全部"), ("0", "非会员可观看"), ("1", "VIP 漫画")];

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.boylove.koma",
    name: "香香腐宅",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "zh-Hans",
    author: "Koma",
    description: "香香腐宅的成人向漫画书库，设有独立的 BL 区。站上有分话连载与完结作品，VIP 章节需要站点账号和相应权限。",
    content_rating: "nsfw",
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
koma_source_sdk::koma_source_buffers! {payload:1024*1024,http_out:5*1024*1024,body:4*1024*1024,http_req:4096,scratch:8192,}
koma_source_sdk::koma_source_helpers!();

fn mirror(input: &Value) -> usize {
    usize::from(input["settings"]["preferredMirror"] == MIRRORS[1])
}

fn number(value: &str) -> Result<&str> {
    value
        .parse::<u32>()
        .ok()
        .filter(|id| *id > 0)
        .filter(|_| value.bytes().all(|b| b.is_ascii_digit()))
        .map(|_| value)
        .ok_or("invalid_request")
}

fn api(url: &Url) -> Result<Value> {
    let body = port::request(
        url.as_str(),
        "GET",
        json!({"Accept":"application/json","Referer":format!("{}/",url.origin().ascii_serialization())}),
        None,
    )?;
    let mut data: Value = serde_json::from_str(&body).map_err(|_| "parse_error")?;
    // This endpoint can report a server error inside a successful HTTP response.
    if matches!(data["code"].as_u64(), Some(500..=599)) {
        return Err("server_error");
    }
    let result = data
        .get_mut("result")
        .filter(|value| !value.is_null())
        .ok_or("parse_error")?;
    Ok(result.take())
}

fn document(url: &str) -> Result<String> {
    port::request(
        url,
        "GET",
        json!({"Accept":"text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8","Referer":format!("{}/",Url::parse(url).map_err(|_|"invalid_request")?.origin().ascii_serialization())}),
        None,
    )
}

fn search_url(base: &str, query: &str, page: usize) -> Result<Url> {
    let mut url = port::absolute(base, "/home/api/searchk")?;
    url.query_pairs_mut()
        .append_pair("keyword", query)
        .append_pair("type", "1")
        .append_pair("pageNo", &page.to_string());
    Ok(url)
}

fn utc_timestamp(seconds: i64) -> Option<String> {
    // Gregorian calendar conversion using 400-year eras.
    let days = seconds.div_euclid(86_400).checked_add(719_468)?;
    let era = days.div_euclid(146_097);
    let day = days.rem_euclid(146_097);
    let year_of_era = (day - day / 1460 + day / 36524 - day / 146096) / 365;
    let ordinal = day - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_from_march = (5 * ordinal + 2) / 153;
    let month = (month_from_march + 2) % 12 + 1;
    let year = 400 * era + year_of_era + i64::from(month <= 2);
    if !(0..=9999).contains(&year) {
        return None;
    }
    let date = ordinal - (153 * month_from_march + 2) / 5 + 1;
    let time = seconds.rem_euclid(86_400);
    Some(format!(
        "{year:04}-{month:02}-{date:02} {:02}:{:02}:{:02}",
        time / 3600,
        time / 60 % 60,
        time % 60
    ))
}

fn card(row: &Value, chosen: usize, req: &[u8]) -> Result<Value> {
    let id = row["id"].as_u64().ok_or("parse_error")?.to_string();
    let title = row["title"].as_str().ok_or("parse_error")?;
    let mut card = json!({"id":id,"title":title,"language":"zh-Hans",
        "links":[{"kind":"source","url":format!("{}{WORK_PATH}{id}",MIRRORS[chosen])}]});
    if let Some(raw) = row["image"].as_str().filter(|s| !s.is_empty()) {
        // The current official templates use each mirror's own image origin for relative covers.
        let cover = port::absolute(COVER_ORIGINS[chosen], raw)?;
        card["cover"] = json!({"kind":"url","url":cover.as_str(),"headers":{"Referer":format!("{}/",MIRRORS[chosen])}});
    }
    if let Some(author) = row["auther"].as_str().filter(|s| !s.is_empty()) {
        card["authors"] = json!([author]);
    }
    if let Some(tags) = row["keyword"].as_str() {
        card["tags"] = json!(tags
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>());
    }
    card["status"] = json!(match row["mhstatus"].as_i64() {
        Some(0) => "ongoing",
        Some(1) => "completed",
        _ => "unknown",
    });
    let summary = row["desc"].as_str().unwrap_or_default().trim();
    let date = row["update_time"]
        .as_str()
        .map(str::to_string)
        .or_else(|| utc_timestamp(row["update_time"].as_i64()?));
    card["description"] = json!(date.as_ref().map_or_else(
        || summary.to_string(),
        |date| format!("更新时间：{date}\n\n{summary}")
    ));
    // Optional list summary: the listing row already carries the update time.
    // Only surface it when the host negotiates the feature; otherwise the item
    // bytes stay unchanged.
    if koma_source_sdk::source_protocol::feature(req, koma_source_sdk::list_info::HOST_FEATURE) {
        if let Some(date) = &date {
            card["listInfo"] = json!([{"type":"text","label":"更新","text":date}]);
        }
    }
    Ok(card)
}

fn filter_or<'a>(args: &'a Value, id: &str, options: &[(&'static str, &str)]) -> Result<&'a str> {
    let value = port::filter(args, id, options)?;
    Ok(if value.is_empty() {
        options[0].0
    } else {
        value
    })
}

fn category_url(base: &str, args: &Value, page: usize) -> Result<Url> {
    let status = filter_or(args, "status", STATES)?;
    let kind = filter_or(args, "type", TYPES)?;
    let region = filter_or(args, "region", REGIONS)?;
    let vip = filter_or(args, "vip", VIP)?;
    let genre = args["filters"]["genre"].as_str().unwrap_or("0");
    let genre = if genre.is_empty() { "0" } else { genre };
    if genre.len() > 150 || genre.chars().any(char::is_control) {
        return Err("invalid_request");
    }
    let tags = match (region, genre) {
        ("0", genre) => utf8_percent_encode(genre, NON_ALPHANUMERIC).to_string(),
        (region, "0") => utf8_percent_encode(region, NON_ALPHANUMERIC).to_string(),
        (region, genre) => format!(
            "{}+{}",
            utf8_percent_encode(region, NON_ALPHANUMERIC),
            utf8_percent_encode(genre, NON_ALPHANUMERIC)
        ),
    };
    port::absolute(
        base,
        &format!("/home/api/cate/tp/1-{tags}-{status}-1-{page}-{kind}-1-{vip}"),
    )
}

fn list(input: &Value, request: &[u8], search: bool) -> Result<Value> {
    let chosen = mirror(input);
    let base = MIRRORS[chosen];
    let args = &input["args"];
    let page = request_page_number(request).max(1);
    let query = args["query"].as_str().unwrap_or_default();
    let latest = !search && args["listingId"] == "latest";
    let has_filter = args["filters"].as_object().is_some_and(|filters| {
        filters.iter().any(|(key, value)| {
            matches!(key.as_str(), "status" | "type" | "region" | "genre" | "vip")
                && value.as_str().is_some_and(|v| {
                    !v.is_empty()
                        && v != match key.as_str() {
                            "status" | "vip" => "2",
                            _ => "0",
                        }
                })
        })
    });
    let url = if search && !query.trim().is_empty() {
        search_url(base, query, page)?
    } else if latest {
        port::absolute(
            base,
            &format!(
                "/home/Api/getDailyUpdate.html?widx=4&page={}&limit=10",
                page - 1
            ),
        )?
    } else if search || has_filter {
        category_url(base, args, page)?
    } else {
        port::absolute(
            base,
            &format!("/home/api/getpage/tp/1-topestmh-{}", page - 1),
        )?
    };
    let result = api(&url)?;
    let (rows, more) = if latest {
        let rows = result.as_array().ok_or("parse_error")?;
        (rows.as_slice(), rows.len() >= 10)
    } else {
        let more = !result["lastPage"].as_bool().ok_or("parse_error")?;
        (
            result["list"]
                .as_array()
                .map(Vec::as_slice)
                .unwrap_or_default(),
            more,
        )
    };
    let items = rows
        .iter()
        .map(|row| card(row, chosen, request))
        .collect::<Result<Vec<_>>>()?;
    Ok(port::items(
        items,
        more.then(|| page.checked_add(1)).flatten(),
    ))
}

fn details(input: &Value, request: &[u8]) -> Result<Value> {
    let id = number(input["args"]["mangaId"].as_str().unwrap_or_default())?;
    let chosen = mirror(input);
    let base = MIRRORS[chosen];
    // Mihon has the existing card title, while Koma restore and get_manga may only have the ID.
    let html = document(&format!("{base}{WORK_PATH}{id}"))?;
    let root = Node::parse(&html)?;
    let title = root.require("meta[property='og:title']")?.attr("content")?;
    let title = title
        .rsplit_once(" - 香香腐宅")
        .map_or(title.as_str(), |(name, _)| name)
        .trim();
    if title.is_empty() {
        return Err("parse_error");
    }
    let result = api(&search_url(base, title, 1)?)?;
    let manga = result["list"]
        .as_array()
        .ok_or("parse_error")?
        .iter()
        .find(|row| row["id"].as_u64() == id.parse::<u64>().ok())
        .ok_or("not_found")?;
    Ok(json!({"manga":card(manga,chosen,request)?}))
}

fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(request)?;
    let id = number(input["args"]["mangaId"].as_str().unwrap_or_default())?;
    let url = port::absolute(
        MIRRORS[mirror(&input)],
        &format!("/home/api/chapter_list/tp/{id}"),
    )?;
    let data = api(&url)?;
    let rows = data["list"]
        .as_array()
        .ok_or("parse_error")?
        .iter()
        .rev()
        .map(|row| {
            let id = row["id"].as_u64().ok_or("parse_error")?;
            let title = row["title"].as_str().ok_or("parse_error")?.trim();
            let date = row["create_time"]
                .as_str()
                .filter(|date| date.is_ascii() && date.len() == 19 && date.as_bytes()[10] == b' ')
                .map(|date| format!("{}T{}Z", &date[..10], &date[11..]));
            Ok((format!("{CHAPTER_PATH}{id}"), title, date))
        })
        .collect::<Result<Vec<_>>>()?;
    let chapters = rows
        .iter()
        .enumerate()
        .map(|(index, (chapter_id, title, date))| {
            let mut chapter = Chapter::new(chapter_id.as_bytes(), id.as_bytes(), title.as_bytes());
            chapter.index = Some(index);
            chapter.published_at = date.as_deref().map(str::as_bytes);
            chapter
        })
        .collect::<Vec<_>>();
    ChapterResponse::ordered(payload_buf(), request, ChapterOrder::NewestFirst, &chapters)
        .map_err(port::chapter_error)
}

fn chapter_pages(input: &Value) -> Result<Value> {
    let chapter = input["args"]["chapterId"]
        .as_str()
        .ok_or("invalid_request")?;
    let urls = if let Some(raw) = chapter.strip_prefix(CHAPTER_PATH) {
        number(raw)?;
        let html = document(&format!("{}{chapter}", MIRRORS[mirror(input)]))?;
        let root = Node::parse(&html)?;
        let section = root.one("section").ok_or_else(|| {
            // The site returns only a VIP alert and a redirect when it denies
            // access. Its chapter list may report isvip=0 for the same chapter.
            let notice = html.trim();
            if notice.starts_with("<script>alert(")
                && notice.ends_with("</script>")
                && notice.contains("本章节只限")
                && notice.contains("VIP会员")
                && notice.contains("location.href")
            {
                "chapter_locked"
            } else {
                "chapter_unavailable"
            }
        })?;
        let nodes = section.all(".reader-cartoon-image")?;
        let mut urls = Vec::new();
        if nodes.is_empty() {
            for img in section.all("img")? {
                let src = img.attr("src")?;
                let src = src.trim();
                if !src.is_empty() && !src.ends_with(".gif") {
                    urls.push(src.to_string());
                }
            }
        } else {
            for container in nodes {
                if let Some(img) = container.one("img") {
                    if img.attr("src")?.ends_with("load.png") {
                        let image = img.attr("data-original")?;
                        if !image.trim().is_empty() {
                            urls.push(image.trim().to_string());
                        }
                    }
                }
            }
        }
        let parts = html
            .split("<script")
            .filter_map(|script| script.split_once("</script>").map(|(s, _)| s))
            .find(|script| script.contains("firstMergeImg") && script.contains("imageData"))
            .and_then(|script| script.split("var scrollTop").next())
            .and_then(|script| script.rsplit_once("var randomClass = "))
            .and_then(|(_, parts)| parts.split(';').next())
            .and_then(|parts| parts.split_whitespace().last())
            .and_then(|parts| parts.parse::<u32>().ok())
            .filter(|parts| *parts > 0);
        urls.iter()
            .map(|raw| {
                let mut url = port::absolute(READER_CDN, raw)?;
                if let Some(parts) = parts {
                    url.query_pairs_mut()
                        .append_pair(PARTS_PARAM, &parts.to_string());
                }
                Ok(url)
            })
            .collect::<Result<Vec<_>>>()?
    } else {
        let (_, urls) = chapter.split_once(':').ok_or("invalid_request")?;
        if urls.is_empty() {
            return Ok(json!({"pages":[]}));
        }
        urls.split(',')
            .map(|raw| port::absolute(READER_CDN, raw))
            .collect::<Result<Vec<_>>>()?
    };
    if urls.is_empty() {
        return Err("chapter_unavailable");
    }
    Ok(
        json!({"pages":urls.into_iter().enumerate().map(|(index,url)| port::page(index,chapter,url)).collect::<Vec<_>>() }),
    )
}

fn filters(input: &Value, request: &[u8]) -> Result<Value> {
    let scoped = source_protocol::feature(request, b"source_filter_scope_header_v1");
    let fetching = source_protocol::feature(request, b"source_filter_fetching_v1");
    let select = |id, name, choices: &[(&str, &str)]| {
        let mut filter = port::select(id, name, "browse", choices);
        filter["default"] = json!(choices[0].0);
        if !scoped {
            filter.as_object_mut().unwrap().remove("scope");
        }
        filter
    };
    let mut values = vec![
        select("status", "状态", STATES),
        select("type", "类型", TYPES),
        select("region", "地区", REGIONS),
    ];
    if fetching {
        let mut genre = json!({"id":"genre","name":"标签","type":"select","default":"0"});
        if scoped {
            genre["scope"] = json!("browse");
        }
        if input["args"]["fetchFilterData"] == true {
            let html = document(&format!("{}/home/book/cate.html", MIRRORS[mirror(input)]))?;
            let root = Node::parse(&html)?;
            let buttons = root.all("div[data-str=tag] > a.button")?;
            let mut options = vec![json!({"value":"0","label":"全部"})];
            let mut seen = BTreeSet::from(["0".to_string()]);
            if buttons.is_empty() {
                // Both mirrors now use a list, while older category pages can use a div.
                for node in root.all("ul[data-str=tag] a.cate-option, div[data-str=tag] a.cate-option")? {
                    let value = node.attr("data-value")?;
                    if !value.is_empty() && seen.insert(value.clone()) {
                        options.push(json!({"value":value,"label":value}));
                    }
                }
            } else {
                for node in buttons.into_iter().skip(1) {
                    let value = node.text()?;
                    if !value.is_empty() && seen.insert(value.clone()) {
                        options.push(json!({"value":value,"label":value}));
                    }
                }
            }
            if options.len() == 1 {
                return Err("parse_error");
            }
            genre["options"] = json!(options);
        }
        values.push(genre);
    }
    if scoped {
        values.insert(
            0,
            json!({"name":"分类筛选（搜索文本时无效）","type":"header","scope":"browse"}),
        );
        values.push(json!({"name":"VIP 漫画需要在原站登录，且原站账户需达到 Lv3","type":"header","scope":"browse"}));
    }
    values.push(select("vip", "漫画权限", VIP));
    Ok(json!({"filters":values,"supportsFilterFetching":fetching}))
}

fn image_request(input: &Value, request: &[u8]) -> Result<Value> {
    let raw = input["args"]["url"].as_str().ok_or("invalid_request")?;
    let mut url = port::absolute(READER_CDN, raw)?;
    let parts = {
        url.query_pairs()
            .find(|(key, _)| key == PARTS_PARAM)
            .map(|(_, value)| value.parse::<u32>())
            .transpose()
            .map_err(|_| "invalid_request")?
    };
    let headers = json!({"Referer":format!("{}/",MIRRORS[mirror(input)])});
    if let Some(parts) = parts {
        if parts == 0 {
            return Err("invalid_request");
        }
        if !source_protocol::feature(request, b"source_required_image_decode_wasm_v1") {
            return Err("host_feature_required");
        }
        let pairs = url
            .query_pairs()
            .filter(|(name, _)| name != PARTS_PARAM)
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect::<Vec<_>>();
        url.set_query(None);
        if !pairs.is_empty() {
            url.query_pairs_mut().extend_pairs(pairs);
        }
        return Ok(
            json!({"url":url.as_str(),"headers":headers,"requiredImageDecode":{"kind":"wasm_image_v1","pixels":format!("x|{parts}")}}),
        );
    }
    Ok(json!({"url":url.as_str(),"headers":headers}))
}

fn dispatch(operation: &str, request: &[u8]) -> Result<Value> {
    let input = port::input(request)?;
    match operation {
        "get_listings" => Ok(json!({"listings":[
            {"id":"popular","name":"热门","supportsPaging":true},
            {"id":"latest","name":"最近更新","supportsPaging":true,"supportsFilters":false}
        ]})),
        "get_filters" => filters(&input, request),
        "get_settings" => Ok(
            json!({"settings":[{"id":"preferredMirror","kind":"select","name":"备用域名",
            "default":MIRRORS[0],"options":MIRRORS.map(|value| json!({"value":value,"label":value}))}]}),
        ),
        "get_authentication" => Ok(
            json!({"authentication":{"kind":"web","loginUrl":format!("{}/",MIRRORS[mirror(&input)])},"authenticated":false}),
        ),
        "get_manga_list" => list(&input, request, false),
        "search" => list(&input, request, true),
        "get_manga" => details(&input, request),
        "get_pages" => chapter_pages(&input),
        "get_image_request" => image_request(&input, request),
        _ => Err("not_implemented"),
    }
}
fn run(operation: &str, request: &[u8]) -> u32 {
    match dispatch(operation, request) {
        Err("server_error") => {
            write_error(operation, "server_error", "原站处理请求失败，请稍后重试。")
        }
        result => port::write_result(operation, result),
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
koma_source_sdk::koma_source_exports!("boylove", typed_chapters, chapter_failure);
koma_source_sdk::koma_source_authentication_exports!("boylove");
