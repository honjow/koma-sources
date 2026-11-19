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

#[path = "../../port_support.rs"]
mod port;
use port::{Node, Result};

const WEB: &str = "https://comic.tibiu.net";
const PAGE_SIZE: usize = 30;
const DESKTOP: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:150.0) Gecko/20100101 Firefox/150.0";
const SORTS: &[(&str, &str)] = &[
    ("", "默认"),
    ("hits", "人气"),
    ("addtime", "更新"),
    ("score", "评分"),
];
const STATES: &[(&str, &str)] = &[("", "全部"), ("连载", "连载"), ("完结", "完结")];

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.tibiucomics.koma",
    name: "TIBIU漫画",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "TIBIU漫画官网目录与公开章节。",
    content_rating: "nsfw",
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
koma_source_sdk::koma_source_buffers! {payload:1024*1024,http_out:3*1024*1024,body:2*1024*1024,http_req:4096,scratch:8192,}
koma_source_sdk::koma_source_helpers!();

#[link(wasm_import_module = "koma_host")]
extern "C" {
    #[link_name = "unix_time_ms"]
    fn host_unix_time_ms(out: *mut u8, capacity: u32) -> i32;
}

fn xhr(url: &str, referer: Option<&str>) -> Result<Value> {
    let mut headers = json!({"Accept":"application/json, text/javascript, */*; q=0.01","X-Requested-With":"XMLHttpRequest"});
    if let Some(referer) = referer {
        headers["Referer"] = json!(referer);
    }
    serde_json::from_str(&port::request(url, "GET", headers, None)?).map_err(|_| "parse_error")
}

fn comic_rows(value: &Value) -> Result<&Vec<Value>> {
    if value["code"] != 1 {
        return Err("parse_error");
    }
    value["data"].as_array().ok_or("parse_error")
}

fn numeric(raw: &str) -> Result<u64> {
    if !raw.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("invalid_request");
    }
    raw.parse::<u64>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or("invalid_request")
}

fn manga_id(args: &Value) -> Result<u64> {
    let raw = args["mangaId"].as_str().ok_or("invalid_request")?;
    if let Ok(id) = numeric(raw) {
        return Ok(id);
    }
    let url = port::absolute(WEB, raw)?;
    if url.host_str() != Some("comic.tibiu.net") {
        return Err("invalid_request");
    }
    let path = url
        .path()
        .strip_prefix("/index.php/")
        .unwrap_or(url.path().trim_start_matches('/'));
    let mut parts = path.split('/');
    if !matches!(parts.next(), Some("comic" | "chapter")) {
        return Err("invalid_request");
    }
    numeric(parts.next().ok_or("invalid_request")?)
}

fn chapter_ids(path: &str) -> Result<(u64, u64)> {
    let (work, chapter) = path
        .strip_prefix("/chapter/")
        .and_then(|tail| tail.split_once('/'))
        .ok_or("invalid_request")?;
    Ok((numeric(work)?, numeric(chapter)?))
}

fn decode(text: &str) -> Result<String> {
    if !text.contains('&') {
        return Ok(text.to_string());
    }
    // Decode text entities without interpreting source-supplied literal tags as markup.
    let escaped = text.replace('<', "&lt;");
    Node::parse(&format!("<span>{escaped}</span>"))?
        .require("span")?
        .text()
}

fn date_parts(date: &str) -> Option<(i64, i64, i64)> {
    if date.len() != 10 || !date.is_ascii() || &date[4..5] != "-" || &date[7..8] != "-" {
        return None;
    }
    let (y, m, d) = (
        date[..4].parse::<i64>().ok()?,
        date[5..7].parse::<i64>().ok()?,
        date[8..].parse::<i64>().ok()?,
    );
    let days = match m {
        4 | 6 | 9 | 11 => 30,
        2 => 28 + i64::from(y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)),
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        _ => return None,
    };
    (y >= 1970 && (1..=days).contains(&d)).then_some((y, m, d))
}

fn recent_update(date: &str) -> bool {
    let seconds = (|| {
        if !date.is_ascii() || date.len() != 19 || &date[10..11] != " " {
            return None;
        }
        let (year, month, day) = date_parts(&date[..10])?;
        let mut time = date[11..].split(':');
        let (hour, minute, second) = (
            time.next()?.parse::<i64>().ok()?,
            time.next()?.parse::<i64>().ok()?,
            time.next()?.parse::<i64>().ok()?,
        );
        if time.next().is_some()
            || !(0..24).contains(&hour)
            || !(0..60).contains(&minute)
            || !(0..60).contains(&second)
        {
            return None;
        }
        let leaps = |y: i64| y / 4 - y / 100 + y / 400;
        let offsets = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
        let days =
            (year - 1970) * 365 + leaps(year - 1) - leaps(1969) + offsets[month as usize - 1] + day
                - 1
                + i64::from(month > 2 && leaps(year) > leaps(year - 1));
        Some(days * 86400 + (hour - 8) * 3600 + minute * 60 + second)
    })();
    let mut now = [0; 8];
    seconds.is_some_and(|updated| {
        (unsafe { host_unix_time_ms(now.as_mut_ptr(), 8) == 8 })
            && (u64::from_le_bytes(now) / 1000) as i64 - updated <= 30 * 86400
    })
}

fn comic_path(row: &Value) -> Result<&str> {
    let path = row["url"].as_str().ok_or("parse_error")?;
    Ok(path.strip_prefix("/index.php").unwrap_or(path))
}

fn card(row: &Value) -> Result<Value> {
    let path = comic_path(row)?;
    let id =
        numeric(path.strip_prefix("/comic/").ok_or("parse_error")?).map_err(|_| "parse_error")?;
    let title = decode(row["name"].as_str().ok_or("parse_error")?)?;
    let author = decode(row["author"].as_str().unwrap_or_default())?;
    let description = row["content"].as_str().unwrap_or_default();
    let description = description
        .split('\n')
        .map(decode)
        .collect::<Result<Vec<_>>>()?
        .join("\n")
        .replace("\"br /\"", "\n");
    let tags = row["tags"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(decode)
        .collect::<Result<Vec<_>>>()?;
    let status = match row["serialize"].as_str() {
        Some("连载" | "连载中" | "連載中" | "En cours" | "OnGoing") => "ongoing",
        Some("完结" | "已完结" | "已完結" | "Terminé" | "Complete" | "Complété") => {
            "completed"
        }
        _ if row["addtime"].as_str().is_some_and(recent_update) => "ongoing",
        _ => "unknown",
    };
    let mut item = json!({"id":id.to_string(),"title":title,"language":"zh","description":description,
        "tags":tags,"status":status,"links":[{"kind":"source","url":format!("{WEB}{path}")}]});
    if !author.is_empty() {
        item["authors"] = json!([author]);
    }
    if let Some(cover) = row["pic"].as_str().filter(|value| !value.is_empty()) {
        item["cover"] = json!({"kind":"url","url":port::absolute(WEB, cover)?.as_str()});
    }
    Ok(item)
}

fn list(args: &Value, page: usize, search: bool) -> Result<Value> {
    let mut url = port::absolute(WEB, "/api/data/comic")?;
    let sort = match port::filter(args, "sort", SORTS)? {
        "" if search => "hits",
        "" => match args["listingId"].as_str() {
            Some("popular") => "hits",
            Some("latest") => "addtime",
            _ => return Err("invalid_request"),
        },
        sort => sort,
    };
    {
        let mut query = url.query_pairs_mut();
        query
            .append_pair("page", &page.to_string())
            .append_pair("size", &PAGE_SIZE.to_string());
        let keyword = args["query"].as_str().unwrap_or_default();
        if search && !keyword.trim().is_empty() {
            query.append_pair("key", keyword);
        }
        query.append_pair("order", sort);
        let state = port::filter(args, "state", STATES)?;
        if !state.is_empty() {
            query.append_pair("serialize", state);
        }
    }
    let value = xhr(url.as_str(), None)?;
    let rows = comic_rows(&value)?;
    Ok(port::items(
        rows.iter().map(card).collect::<Result<Vec<_>>>()?,
        (rows.len() >= PAGE_SIZE).then_some(page.saturating_add(1)),
    ))
}

fn details(id: u64) -> Result<Value> {
    let info = xhr(
        &format!("{WEB}/index.php/api/data/comicinfo?cid={id}"),
        None,
    )?;
    let title = decode(info["data"]["name"].as_str().ok_or("not_found")?)?;
    let mut url = port::absolute(WEB, "/api/data/comic")?;
    url.query_pairs_mut().append_pair("key", &title);
    let value = xhr(url.as_str(), None)?;
    let wanted = format!("/comic/{id}");
    let row = comic_rows(&value)?
        .iter()
        .find(|row| comic_path(row).ok() == Some(wanted.as_str()))
        .ok_or("not_found")?;
    Ok(json!({"manga":card(row)?}))
}

fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    let args = port::input(request)?;
    let id = manga_id(&args["args"])?;
    let work = id.to_string();
    let value = xhr(&format!("{WEB}/api/comic/chapter?mid={id}"), None)?;
    if value["code"] != 1 {
        return Err("chapter_unavailable");
    }
    let rows = value["data"]
        .as_array()
        .ok_or("parse_error")?
        .iter()
        .rev()
        .map(|row| {
            let path = row["link"].as_str().ok_or("parse_error")?;
            if chapter_ids(path).map_err(|_| "parse_error")?.0 != id {
                return Err("parse_error");
            }
            let title = decode(row["name"].as_str().ok_or("parse_error")?)?;
            let date = row["addtime"]
                .as_str()
                .filter(|date| date_parts(date).is_some())
                .map(|date| format!("{date}T00:00:00+08:00"));
            Ok((path, title, date))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut titles = BTreeSet::new();
    let mut paths = BTreeSet::new();
    let mut ambiguous = false;
    for (path, title, _) in &rows {
        if !paths.insert(path) {
            return Err("parse_error");
        }
        ambiguous |= !titles.insert(title.trim());
    }
    let mut catalog = GroupedChapters::new(payload_buf(), request).map_err(port::chapter_error)?;
    for (index, (path, title, date)) in rows.iter().enumerate() {
        let mut chapter = Chapter::new(path.as_bytes(), work.as_bytes(), title.as_bytes());
        chapter.language = Some(b"zh");
        chapter.published_at = date.as_deref().map(str::as_bytes);
        // Repeated labels coexist in the source, with distinct links and no authoritative edition IDs.
        let placement = if ambiguous {
            ChapterPlacement::ManualOnly
        } else {
            ChapterPlacement::InGroup {
                group: b"source-catalog",
                position: (rows.len() - index - 1) as u64,
            }
        };
        catalog
            .push(chapter, placement)
            .map_err(port::chapter_error)?;
    }
    catalog
        .finish(ChapterPage::Complete)
        .map_err(port::chapter_error)
}

fn pages(args: &Value) -> Result<Value> {
    let path = args["chapterId"].as_str().ok_or("invalid_request")?;
    let (work, chapter) = chapter_ids(path)?;
    if work != manga_id(args)? {
        return Err("invalid_request");
    }
    let value = xhr(
        &format!("{WEB}/index.php/api/data/pic?cid={chapter}"),
        Some(&format!("{WEB}{path}")),
    )?;
    if value["code"] != 1 {
        return Err("chapter_unavailable");
    }
    let mut images = value["data"]
        .as_array()
        .ok_or("chapter_unavailable")?
        .iter()
        .map(|row| {
            Ok((
                row["xid"]
                    .as_i64()
                    .or_else(|| row["xid"].as_str()?.parse().ok())
                    .ok_or("parse_error")?,
                port::absolute(WEB, row["img"].as_str().ok_or("parse_error")?)?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    if images.is_empty() {
        return Err("chapter_unavailable");
    }
    images.sort_by_key(|(order, _)| *order);
    let pages = images
        .into_iter()
        .enumerate()
        .map(|(index, (_, image))| port::page(index, path, image))
        .collect::<Vec<_>>();
    Ok(json!({"chapterId":path,"pages":pages}))
}

fn filters() -> Value {
    let mut sort = port::select("sort", "排序", "", SORTS);
    let mut state = port::select("state", "状态", "", STATES);
    for filter in [&mut sort, &mut state] {
        if let Some(fields) = filter.as_object_mut() {
            fields.remove("scope");
        }
    }
    json!({"filters":[state,sort]})
}

fn run(op: &str, request: &[u8]) -> u32 {
    let result = port::input(request).and_then(|input| {
        let args = &input["args"];
        match op {
            "get_listings" => Ok(port::listing_list(true, true, true)),
            "get_manga_list" | "search" => list(args, request_page_number(request), op == "search"),
            "get_manga" => details(manga_id(args)?),
            "get_pages" => pages(args),
            "get_image_request" => {
                let image = port::absolute(WEB, args["url"].as_str().ok_or("invalid_request")?)?;
                Ok(json!({"url":image.as_str(),"headers":{"User-Agent":DESKTOP}}))
            }
            "get_filters" => Ok(filters()),
            _ => Err("unsupported_operation"),
        }
    });
    port::write_result(op, result)
}

fn run_search(request: &[u8]) -> u32 {
    run("search", request)
}
fn run_get_listings(request: &[u8]) -> u32 {
    run("get_listings", request)
}
fn run_get_manga_list(request: &[u8]) -> u32 {
    run("get_manga_list", request)
}
fn run_get_manga(request: &[u8]) -> u32 {
    run("get_manga", request)
}
fn run_get_pages(request: &[u8]) -> u32 {
    run("get_pages", request)
}
fn run_get_image_request(request: &[u8]) -> u32 {
    run("get_image_request", request)
}
fn run_get_filters(request: &[u8]) -> u32 {
    run("get_filters", request)
}
fn run_get_settings(request: &[u8]) -> u32 {
    run("get_settings", request)
}
fn run_get_home(request: &[u8]) -> u32 {
    run("get_home", request)
}
fn chapter_failure(code: &str) -> u32 {
    port::write_failure("get_chapters", code)
}
koma_source_sdk::koma_source_exports!("tibiucomics", typed_chapters, chapter_failure);
