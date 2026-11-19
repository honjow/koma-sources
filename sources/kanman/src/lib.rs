#![no_std]
extern crate alloc;

use alloc::{
    collections::BTreeSet,
    format,
    string::{String, ToString},
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
mod port;
use port::{Node, Result};

const WEB: &str = "https://www.kanman.com";
const BUNDLE: usize = 4;
const SORTS: &[(&str, &str)] = &[
    ("", "默认"),
    ("click", "人气"),
    ("date", "更新"),
    ("shoucang", "收藏"),
];
const CATEGORIES: &[(&str, &str)] = &[
    ("", "全部"),
    ("rexue", "热血"),
    ("maoxian", "冒险"),
    ("gaoxiao", "搞笑"),
    ("zhanzhen", "战争"),
    ("shenmo", "神魔"),
    ("xuanyi", "悬疑"),
    ("shehui", "社会"),
    ("lianai", "恋爱"),
    ("xixue", "吸血"),
    ("luoli", "萝莉"),
    ("bazong", "霸总"),
    ("xuanhuan", "玄幻"),
    ("gufeng", "古风"),
    ("lishi", "历史"),
    ("mangai", "漫改"),
    ("youxi", "游戏"),
    ("chuanyue", "穿越"),
    ("kehuan", "科幻"),
    ("dushi", "都市"),
    ("wuxia", "武侠"),
    ("xiuzhen", "修真"),
    ("shenghuo", "生活"),
    ("dongzuo", "动作"),
    ("jingpin", "精品"),
    ("lianzai", "连载"),
    ("wanjie", "完结"),
];

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.kanman.koma",
    name: "看漫画",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "看漫画官网公开漫画目录与免费章节。",
    content_rating: "safe",
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
koma_source_sdk::koma_source_buffers! {payload:1024*1024,http_out:6*1024*1024,body:5*1024*1024,http_req:4096,scratch:8192,}
koma_source_sdk::koma_source_helpers!();

#[link(wasm_import_module = "koma_host")]
extern "C" {
    #[link_name = "unix_time_ms"]
    fn host_unix_time_ms(out: *mut u8, capacity: u32) -> i32;
}

fn api(url: &Url, referer: &str, fresh: bool) -> Result<Value> {
    let mut headers = json!({"Accept":"application/json, text/javascript, */*; q=0.01","X-Requested-With":"XMLHttpRequest","Referer":referer});
    if fresh {
        headers["Cache-Control"] = json!("no-cache");
    }
    let body = port::request(url.as_str(), "GET", headers, None)?;
    let mut response: Value = serde_json::from_str(&body).map_err(|_| "parse_error")?;
    if response["status"] != 0 || response["data"].is_null() {
        return Err(if fresh {
            "chapter_unavailable"
        } else {
            "parse_error"
        });
    }
    Ok(response["data"].take())
}

fn manga_id(args: &Value) -> Result<u64> {
    let raw = args["mangaId"].as_str().ok_or("invalid_request")?;
    let id = if raw.bytes().all(|b| b.is_ascii_digit()) {
        raw.parse::<u64>().ok()
    } else {
        let url = port::absolute(WEB, raw)?;
        if !url
            .host_str()
            .is_some_and(|host| host == "kanman.com" || host.ends_with(".kanman.com"))
        {
            return Err("invalid_request");
        }
        url.path()
            .trim_matches('/')
            .split('/')
            .next()
            .and_then(|id| id.parse().ok())
    };
    id.filter(|id| *id > 0).ok_or("invalid_request")
}

fn chapter_parts(path: &str) -> Result<(u64, &str)> {
    let (manga, file) = path
        .strip_prefix('/')
        .and_then(|path| path.split_once('/'))
        .ok_or("invalid_request")?;
    let manga = manga
        .parse::<u64>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or("invalid_request")?;
    let chapter = file
        .strip_suffix(".html")
        .filter(|name| {
            !name.is_empty()
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
        })
        .ok_or("invalid_request")?;
    Ok((manga, chapter))
}

fn card(id: u64, title: &str, cover: Option<&str>) -> Result<Value> {
    let mut manga = json!({"id":id.to_string(),"title":title,"language":"zh",
        "links":[{"kind":"source","url":format!("{WEB}/{id}/")}]});
    if let Some(cover) = cover.filter(|cover| !cover.is_empty()) {
        let mut url = port::absolute(WEB, cover)?;
        if url.scheme() == "http" {
            url.set_scheme("https").map_err(|_| "parse_error")?;
        }
        manga["cover"] = json!({"kind":"url","url":url.as_str()});
    }
    Ok(manga)
}

fn list_page(sort: &str, category: &str, keyword: &str, page: usize) -> Result<Vec<Value>> {
    let mut url = port::absolute(WEB, "/api/getsortlist/")?;
    {
        let mut query = url.query_pairs_mut();
        query
            .append_pair("orderby", sort)
            .append_pair("search_key", keyword);
        if !category.is_empty() {
            query.append_pair("comic_sort", category);
        }
        query
            .append_pair("page", &page.to_string())
            .append_pair("size", "48");
    }
    api(&url, &format!("{WEB}/sort/all.html"), false)?["data"]
        .as_array()
        .ok_or("parse_error")?
        .iter()
        .map(|row| {
            card(
                row["comic_id"].as_u64().ok_or("parse_error")?,
                row["comic_name"].as_str().ok_or("parse_error")?,
                row["cover_img"].as_str(),
            )
        })
        .collect()
}

fn tail_page(sort: &str) -> Result<usize> {
    let document = Node::parse(&port::get(&format!("{WEB}/sort/all.html"))?)?;
    let mut upper = document
        .all("div.J_pages a")?
        .iter()
        .filter_map(|link| link.text().ok()?.parse::<usize>().ok())
        .max()
        .unwrap_or(1);
    while upper > 1 {
        let lower = upper.saturating_sub(BUNDLE - 1).max(1);
        for page in (lower..=upper).rev() {
            if !list_page(sort, "", "", page)?.is_empty() {
                return Ok(page);
            }
        }
        upper = lower - 1;
    }
    Ok(1)
}

fn full_list(args: &Value, sort: &str, legacy_page: usize) -> Result<Value> {
    let (mut start, last) = if let Some(cursor) = args["page"]["cursor"]
        .as_str()
        .filter(|cursor| cursor.contains(':'))
    {
        let (order, range) = cursor.split_once(':').ok_or("invalid_request")?;
        let (start, last) = range.split_once(':').ok_or("invalid_request")?;
        let start = start.parse::<usize>().map_err(|_| "invalid_request")?;
        let last = last.parse::<usize>().map_err(|_| "invalid_request")?;
        if order != sort || start == 0 || start > last || last > 10_000 {
            return Err("invalid_request");
        }
        (start, last)
    } else {
        (
            legacy_page
                .saturating_sub(1)
                .saturating_mul(BUNDLE)
                .saturating_add(1),
            tail_page(sort)?,
        )
    };
    let mut items = Vec::new();
    while start <= last && items.is_empty() {
        let end = start.saturating_add(BUNDLE - 1).min(last);
        for page in start..=end {
            items.extend(list_page(sort, "", "", page)?);
        }
        start = end + 1;
    }
    let mut ids = BTreeSet::new();
    items.retain(|item| ids.insert(item["id"].as_str().unwrap_or_default().to_string()));
    Ok(
        json!({"items":items,"page":{"hasMore":start <= last,"nextCursor":(start <= last).then(|| format!("{sort}:{start}:{last}"))}}),
    )
}

fn filtered_list(sort: &str, category: &str, keyword: &str) -> Result<Value> {
    let mut items = Vec::new();
    for page in 1.. {
        let batch = list_page(sort, category, keyword, page)?;
        if batch.is_empty() {
            return Ok(port::items(items, None));
        }
        items.extend(batch);
        if items.len() > 4096 {
            return Err("result_too_large");
        }
    }
    Err("result_too_large")
}

fn search(args: &Value, page: usize, default_sort: &str) -> Result<Value> {
    let sort = match port::filter(args, "sort", SORTS)? {
        "" => default_sort,
        sort => sort,
    };
    let keyword = args["query"].as_str().unwrap_or_default();
    let category = if keyword.trim().is_empty() {
        port::filter(args, "category", CATEGORIES)?
    } else {
        ""
    };
    if category.is_empty() && keyword.trim().is_empty() {
        full_list(args, sort, page)
    } else {
        filtered_list(sort, category, keyword)
    }
}

fn document(id: u64) -> Result<Node> {
    Node::parse(&port::get(&format!("{WEB}/{id}/"))?)
}

fn meta(doc: &Node, name: &str) -> Option<String> {
    doc.one(&format!("meta[property='og:{name}']"))?
        .attr("content")
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn details(id: u64) -> Result<Value> {
    let doc = document(id)?;
    let mut manga = card(
        id,
        &doc.require("div.description h1.title")?.text()?,
        meta(&doc, "image").as_deref(),
    )?;
    if let Some(author) = meta(&doc, "novel:author") {
        manga["authors"] = json!([author]);
    }
    if let Some(tags) = meta(&doc, "novel:category") {
        manga["tags"] = json!(tags
            .split(',')
            .map(str::trim)
            .filter(|tag| !tag.is_empty())
            .collect::<Vec<_>>());
    }
    manga["status"] = json!(match meta(&doc, "novel:status").as_deref() {
        Some("连载") => "ongoing",
        Some("完结") => "completed",
        _ => "unknown",
    });
    let mut description = doc
        .one("div.mhjj .desc-box .content")
        .map(|node| node.text())
        .transpose()?
        .unwrap_or_default();
    if let Some(date) = doc
        .one("div.mhjj .update-date")
        .map(|node| node.text())
        .transpose()?
        .filter(|v| !v.is_empty())
    {
        if !description.is_empty() {
            description.push_str("\n\n");
        }
        description.push_str(&format!("更新：{date}"));
    }
    manga["description"] = json!(description);
    Ok(json!({"manga":manga}))
}

fn catalog(doc: &Node, manga: u64) -> Result<Vec<(String, String, bool)>> {
    doc.require("#j_chapter_list")?;
    doc.all("#j_chapter_list > li.item")?
        .iter()
        .rev()
        .map(|item| {
            let link = port::absolute(WEB, &item.require("a")?.attr("href")?)?;
            if link.host_str() != Some("www.kanman.com")
                || chapter_parts(link.path()).map_err(|_| "parse_error")?.0 != manga
            {
                return Err("parse_error");
            }
            let locked = item
                .attr("class")?
                .split_whitespace()
                .any(|class| class == "lock");
            Ok((
                link.path().to_string(),
                item.require("p.name")?.text()?,
                locked,
            ))
        })
        .collect()
}

fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(request)?;
    let id = manga_id(&input["args"])?;
    let doc = document(id)?;
    let rows = catalog(&doc, id)?;
    let manga = id.to_string();
    let latest_date = meta(&doc, "novel:update_time")
        .filter(|date| date.len() == 10)
        .map(|date| format!("{date}T00:00:00+08:00"));
    let chapters = rows
        .iter()
        .enumerate()
        .map(|(index, (path, title, locked))| {
            let mut chapter = Chapter::new(path.as_bytes(), manga.as_bytes(), title.as_bytes());
            chapter.locked = Some(*locked);
            chapter.language = Some(b"zh");
            if index == 0 {
                chapter.published_at = latest_date.as_deref().map(str::as_bytes);
            }
            chapter
        })
        .collect::<Vec<_>>();
    ChapterResponse::ordered(payload_buf(), request, ChapterOrder::NewestFirst, &chapters)
        .map_err(port::chapter_error)
}

fn page_urls(path: &str) -> Result<Vec<Value>> {
    let (manga, chapter) = chapter_parts(path)?;
    let doc = document(manga)?;
    let rows = catalog(&doc, manga)?;
    let (_, _, locked) = rows
        .iter()
        .find(|(current, _, _)| current == path)
        .ok_or("not_found")?;
    if *locked {
        return Err("chapter_locked");
    }
    let mut url = port::absolute(WEB, "/api/getchapterinfov2")?;
    url.query_pairs_mut()
        .append_pair("comic_id", &manga.to_string())
        .append_pair("chapter_newid", chapter);
    let mut payload = api(&url, &format!("{WEB}{path}"), true)?;
    match payload["current_chapter"]["chapter_img_list"].take() {
        Value::Array(images) if !images.is_empty() => Ok(images),
        _ => Err("chapter_unavailable"),
    }
}

fn pages(args: &Value) -> Result<Value> {
    let path = args["chapterId"].as_str().ok_or("invalid_request")?;
    if chapter_parts(path)?.0 != manga_id(args)? {
        return Err("invalid_request");
    }
    let pages = page_urls(path)?
        .iter()
        .enumerate()
        .map(|(index, image)| {
            Ok(port::page(
                index,
                path,
                port::absolute(WEB, image.as_str().ok_or("parse_error")?)?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(json!({"chapterId":path,"pages":pages}))
}

fn image(args: &Value) -> Result<Value> {
    let mut url = port::absolute(WEB, args["url"].as_str().ok_or("invalid_request")?)?;
    let expiry = url
        .query_pairs()
        .find(|(key, _)| key == "auth_key")
        .and_then(|(_, value)| value.split('-').next()?.parse::<u64>().ok());
    if let Some(expiry) = expiry {
        if let Some((chapter, index)) = args["pageId"].as_str().and_then(|id| id.rsplit_once(':')) {
            let mut now = [0; 8];
            if unsafe { host_unix_time_ms(now.as_mut_ptr(), 8) } == 8
                && expiry <= u64::from_le_bytes(now) / 1000 + 30
            {
                let index = index
                    .parse::<usize>()
                    .ok()
                    .and_then(|index| index.checked_sub(1))
                    .ok_or("invalid_request")?;
                let urls = page_urls(chapter)?;
                url = port::absolute(
                    WEB,
                    urls.get(index)
                        .and_then(Value::as_str)
                        .ok_or("chapter_unavailable")?,
                )?;
            }
        }
    }
    Ok(json!({"url":url.as_str(),"headers":{}}))
}

fn filters() -> Value {
    let mut sort = port::select("sort", "排序", "", SORTS);
    let mut category = port::select("category", "分类（仅无关键词时生效）", "", CATEGORIES);
    for filter in [&mut sort, &mut category] {
        if let Some(fields) = filter.as_object_mut() {
            fields.remove("scope");
        }
    }
    json!({"filters":[sort,category]})
}

fn run(op: &str, request: &[u8]) -> u32 {
    let result = port::input(request).and_then(|input| {
        let args = &input["args"];
        match op {
            "get_listings" => Ok(port::listing_list(true, true, true)),
            "get_manga_list" => {
                let sort = match args["listingId"].as_str() {
                    Some("popular") => "click",
                    Some("latest") => "date",
                    _ => return Err("invalid_request"),
                };
                search(args, request_page_number(request), sort)
            }
            "search" => search(args, request_page_number(request), "click"),
            "get_manga" => details(manga_id(args)?),
            "get_pages" => pages(args),
            "get_image_request" => image(args),
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
koma_source_sdk::koma_source_exports!("kanman", typed_chapters, chapter_failure);
