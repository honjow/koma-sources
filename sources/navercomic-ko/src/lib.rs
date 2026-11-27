#![no_std]

extern crate alloc;

use alloc::{
    collections::BTreeSet,
    format,
    string::{String, ToString},
    vec::Vec,
};
use koma_source_sdk::{
    chapters::{Chapter, ChapterNumber, ChapterOrder, ChapterResponse, ChapterResult},
    localization::Catalog,
    runtime::Context,
    source::{SourceCapabilities, SourceInfo},
};
use serde_json::{json, Value};
use url::Url;

#[path = "../../port_support.rs"]
#[allow(dead_code)]
mod port_support;
use port_support::{self as port, Node, Result};

#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

const BASE: &str = "https://comic.naver.com";
const MOBILE: &str = "https://m.comic.naver.com";
const REFERER: &str = "https://comic.naver.com/";
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.naver.comic.ko.koma",
    name: "Naver Webtoon",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "ko",
    author: "Koma",
    description: "Naver 韩国官方条漫平台，既有按星期更新的正式连载，也有创作者投稿的「最佳挑战」与「挑战漫画」。",
    content_rating: "safe",
};
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: true,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: true,
    manga_list: true,
    home: false,
    filters: false,
    settings: false,
    credentials: false,
    image_request: true,
};

koma_source_sdk::koma_source_buffers! {
    payload: 2 * 1024 * 1024,
    http_out: 5 * 1024 * 1024,
    body: 4 * 1024 * 1024,
    http_req: 4096,
    scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

fn api(path: &str) -> Result<Value> {
    let body = port::request(
        &format!("{BASE}{path}"),
        "GET",
        json!({"Accept":"application/json", "Referer":REFERER}),
        None,
    )?;
    serde_json::from_str(&body).map_err(|_| "parse_error")
}

// The three upstream extensions share article APIs but use separate public
// routes. Both work and chapter IDs retain that route for saved entries.
fn work_id(kind: &str, id: usize) -> String {
    format!("naver:{kind}:{id}")
}

fn site_url(kind: &str, id: usize, episode: Option<usize>) -> String {
    match episode {
        Some(no) => format!("{BASE}/{kind}/detail?titleId={id}&no={no}"),
        None => format!("{BASE}/{kind}/list?titleId={id}"),
    }
}

fn identity(id: &str, is_chapter: bool) -> Result<(&str, usize, Option<usize>)> {
    let prefix = if is_chapter { "naver-ch:" } else { "naver:" };
    let mut parts = id.strip_prefix(prefix).ok_or("invalid_request")?.split(':');
    let kind = parts
        .next()
        .filter(|kind| matches!(*kind, "webtoon" | "bestChallenge" | "challenge"))
        .ok_or("invalid_request")?;
    let work = parts
        .next()
        .and_then(|n| n.parse::<usize>().ok())
        .filter(|n| *n > 0)
        .ok_or("invalid_request")?;
    let episode = if is_chapter {
        Some(
            parts
                .next()
                .and_then(|n| n.parse::<usize>().ok())
                .filter(|n| *n > 0)
                .ok_or("invalid_request")?,
        )
    } else {
        None
    };
    if parts.next().is_some() {
        return Err("invalid_request");
    }
    Ok((kind, work, episode))
}

fn positive(value: &Value) -> Result<usize> {
    value
        .as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .filter(|n| *n > 0)
        .ok_or("parse_error")
}

fn manga(kind: &str, row: &Value) -> Result<Value> {
    let id = positive(&row["titleId"])?;
    let title = row["titleName"]
        .as_str()
        .filter(|title| !title.is_empty())
        .ok_or("parse_error")?;
    let mut authors: Vec<_> = row["communityArtists"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|artist| artist["name"].as_str())
        .collect();
    if authors.is_empty() {
        authors.extend(
            row["displayAuthor"]
                .as_str()
                .into_iter()
                .flat_map(|s| s.split(" / ")),
        );
    }
    let status = if row["rest"] == true {
        "hiatus"
    } else if row["finished"] == true {
        "completed"
    } else if kind == "webtoon" || row.get("finished").is_some() {
        "ongoing"
    } else {
        "unknown"
    };
    let mut item = json!({
        "id":work_id(kind,id), "title":title, "authors":authors, "status":status,
        "description":row["synopsis"].as_str().unwrap_or_default(), "language":"ko",
        "contentRating":if row["adult"] == true || row["nineteen"] == true {"nsfw"} else {"safe"},
        "links":[{"kind":"source","url":site_url(kind,id,None)}]
    });
    if let Some(cover) = row["thumbnailUrl"].as_str().filter(|url| !url.is_empty()) {
        item["cover"] = json!({"kind":"url","url":port::absolute(BASE,cover)?.as_str()});
    }
    Ok(item)
}

fn cursor(args: &Value) -> Result<usize> {
    match args["page"]["cursor"].as_str() {
        Some(value) => value
            .parse()
            .ok()
            .filter(|n| *n > 0)
            .ok_or("invalid_request"),
        None if args["page"]["cursor"].is_null() => Ok(1),
        None => Err("invalid_request"),
    }
}

fn next_page(row: &Value, current: usize) -> Result<Option<usize>> {
    match row["pageInfo"]["nextPage"]
        .as_u64()
        .and_then(|n| usize::try_from(n).ok())
    {
        Some(0) => Ok(None),
        Some(next) if next > current => Ok(Some(next)),
        _ => Err("parse_error"),
    }
}

fn api_listing(kind: &str, path: &str, key: &str, page: usize) -> Result<Value> {
    let data = api(path)?;
    let rows = data[key].as_array().ok_or("parse_error")?;
    let entries = rows
        .iter()
        .map(|row| manga(kind, row))
        .collect::<Result<Vec<_>>>()?;
    let next = if key == "list" && data["pageInfo"].is_null() {
        if entries.is_empty() {
            None
        } else {
            page.checked_add(1)
        }
    } else {
        next_page(&data, page)?
    };
    Ok(port::items(entries, next))
}

fn weekday(sort: &str, page: usize) -> Result<Value> {
    if page != 1 {
        return Ok(port::items(Vec::new(), None));
    }
    let url = format!("{MOBILE}/webtoon/weekday?sort={sort}");
    let body = port::request(&url, "GET", port::mobile_image_headers(REFERER), None)?;
    let doc = Node::parse(&body)?;
    let mut items = Vec::new();
    let mut seen = BTreeSet::new();
    for row in doc.all(".list_toon > [class='item ']")? {
        let Some(anchor) = row.one("a") else { continue };
        let link = port::absolute(&url, &anchor.attr("href")?)?;
        let Some(id) = link
            .query_pairs()
            .find(|(key, _)| key == "titleId")
            .and_then(|(_, value)| value.parse::<usize>().ok())
            .filter(|n| *n > 0)
        else {
            continue;
        };
        let Some(title) = row.one("strong") else {
            continue;
        };
        if !seen.insert(id) {
            continue;
        }
        let author = row
            .one("span.author")
            .map(|node| node.text())
            .transpose()?
            .unwrap_or_default();
        let mut item = json!({
            "id":work_id("webtoon",id), "title":title.text()?,
            "authors":author.split(" / ").filter(|s| !s.is_empty()).collect::<Vec<_>>(),
            "language":"ko", "links":[{"kind":"source","url":site_url("webtoon",id,None)}]
        });
        if let Some(img) = row.one("img") {
            let src = img.attr("src")?;
            if !src.is_empty() {
                item["cover"] = json!({"kind":"url","url":port::absolute(&url,&src)?.as_str()});
            }
        }
        items.push(item);
    }
    if items.is_empty() {
        return Err("parse_error");
    }
    Ok(port::items(items, None))
}

fn episode_date(value: &Value) -> Option<String> {
    let mut fields = value.as_str()?.split('.');
    let year = fields.next()?.parse::<u32>().ok()?;
    let month = fields.next()?.parse::<u32>().ok()?;
    let day = fields.next()?.parse::<u32>().ok()?;
    if year > 99
        || !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || fields.next().is_some()
    {
        return None;
    }
    Some(format!(
        "{:04}-{month:02}-{day:02}T00:00:00+09:00",
        2000 + year
    ))
}

struct Episode {
    id: String,
    no: usize,
    title: String,
    url: String,
    date: Option<String>,
}

fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(request)?;
    let (kind, id, _) = identity(
        input["args"]["mangaId"].as_str().ok_or("invalid_request")?,
        false,
    )?;
    let manga_id = work_id(kind, id);
    let mut episodes = Vec::new();
    let mut seen = BTreeSet::new();
    let mut page = 1;
    loop {
        let result = api(&format!("/api/article/list?titleId={id}&page={page}"))?;
        if positive(&result["titleId"])? != id {
            return Err("parse_error");
        }
        for row in result["articleList"].as_array().ok_or("parse_error")? {
            let no = positive(&row["no"])?;
            if !seen.insert(no) {
                continue;
            }
            episodes.push(Episode {
                id: format!("naver-ch:{kind}:{id}:{no}"),
                no,
                title: row["subtitle"].as_str().ok_or("parse_error")?.to_string(),
                url: site_url(kind, id, Some(no)),
                date: episode_date(&row["serviceDateDescription"]),
            });
        }
        let Some(next) = next_page(&result, page)? else {
            break;
        };
        page = next;
    }
    let chapters: Vec<_> = episodes
        .iter()
        .map(|episode| {
            let mut chapter = Chapter::new(
                episode.id.as_bytes(),
                manga_id.as_bytes(),
                episode.title.as_bytes(),
            );
            chapter.url = Some(episode.url.as_bytes());
            chapter.language = Some(b"ko");
            chapter.chapter_number = Some(ChapterNumber::Integer(episode.no));
            chapter.published_at = episode.date.as_deref().map(str::as_bytes);
            chapter
        })
        .collect();
    ChapterResponse::ordered(payload_buf(), request, ChapterOrder::NewestFirst, &chapters)
        .map_err(port::chapter_error)
}

fn page_images(chapter: &str) -> Result<Value> {
    let (kind, id, episode) = identity(chapter, true)?;
    let url = site_url(kind, id, episode);
    let doc = Node::parse(&port::get(&url)?)?;
    let mut images = Vec::new();
    for (selector, lazy) in [
        (".wt_viewer img", false),
        (".toon_view_lst img.toon_image", true),
    ] {
        for node in doc.all(selector)? {
            let src = if lazy {
                node.attr("data-src").unwrap_or_default()
            } else {
                String::new()
            };
            let src = if src.is_empty() {
                node.attr("src").unwrap_or_default()
            } else {
                src
            };
            if src.is_empty() {
                continue;
            }
            let image = port::absolute(&url, &src)?;
            images.push(port::page(images.len(), chapter, image));
        }
        if !images.is_empty() {
            break;
        }
    }
    if images.is_empty() {
        return Err("chapter_unavailable");
    }
    Ok(json!({"chapterId":chapter,"pages":images}))
}

fn listings(request: &[u8]) -> Result<Value> {
    port::input(request)?;
    let catalog =
        Catalog::from_metadata(include_bytes!("../metadata.json")).map_err(|_| "internal_error")?;
    let context = Context::from_request(request);
    let mut listings = Vec::new();
    for (id, key, fallback, paged) in [
        ("popular", "listing.popular", "热门", false),
        ("latest", "listing.latest", "最近更新", false),
        ("best_challenge", "listing.best_challenge", "最佳挑战", true),
        ("challenge", "listing.challenge", "挑战漫画", true),
    ] {
        let mut buffer = [0; 128];
        let name = catalog
            .text(&context, key, fallback, &mut buffer)
            .map_err(|_| "internal_error")?;
        listings.push(json!({"id":id,"name":name,"supportsPaging":paged}));
    }
    Ok(json!({"listings":listings}))
}

fn with_covers(mut result: Value, request: &[u8]) -> Value {
    port::mobile_cover_headers(&mut result, REFERER, request);
    result
}

fn run_search(request: &[u8]) -> u32 {
    let result = (|| {
        let input = port::input(request)?;
        let args = &input["args"];
        let query = args["query"].as_str().ok_or("invalid_request")?;
        let page = cursor(args)?;
        let encoded: String = url::form_urlencoded::byte_serialize(query.as_bytes()).collect();
        let result = api_listing(
            "webtoon",
            &format!("/api/search/webtoon?keyword={encoded}&page={page}"),
            "searchList",
            page,
        )?;
        Ok(with_covers(result, request))
    })();
    port::write_result("search", result)
}

fn run_get_manga_list(request: &[u8]) -> u32 {
    let result = (|| {
        let input = port::input(request)?;
        let args = &input["args"];
        let page = cursor(args)?;
        let response = match args["listingId"].as_str().unwrap_or("popular") {
            "popular" => weekday("ALL_READER", page),
            "latest" => weekday("UPDATE", page),
            "best_challenge" => api_listing(
                "bestChallenge",
                &format!("/api/bestChallenge/list?order=VIEW&page={page}"),
                "list",
                page,
            ),
            "challenge" => api_listing(
                "challenge",
                &format!("/api/challenge/list?order=VIEW&page={page}"),
                "list",
                page,
            ),
            _ => Err("invalid_request"),
        }?;
        Ok(with_covers(response, request))
    })();
    port::write_result("get_manga_list", result)
}

fn run_get_manga(request: &[u8]) -> u32 {
    let result = (|| {
        let input = port::input(request)?;
        let (kind, id, _) = identity(
            input["args"]["mangaId"].as_str().ok_or("invalid_request")?,
            false,
        )?;
        let row = api(&format!("/api/article/list/info?titleId={id}"))?;
        if positive(&row["titleId"])? != id {
            return Err("parse_error");
        }
        Ok(with_covers(json!({"manga":manga(kind,&row)?}), request))
    })();
    port::write_result("get_manga", result)
}

fn run_get_pages(request: &[u8]) -> u32 {
    let result = port::input(request).and_then(|input| {
        page_images(
            input["args"]["chapterId"]
                .as_str()
                .ok_or("invalid_request")?,
        )
    });
    if matches!(result, Err("chapter_unavailable")) {
        let mut buffer = [0; 256];
        let fallback = "原站未提供本章图片，请在原网页查看阅读条件。";
        let message = Catalog::from_metadata(include_bytes!("../metadata.json"))
            .ok()
            .and_then(|catalog| {
                catalog
                    .text(
                        &Context::from_request(request),
                        "error.chapter_unavailable",
                        fallback,
                        &mut buffer,
                    )
                    .ok()
            })
            .unwrap_or(fallback);
        return write_error("get_pages", "chapter_unavailable", message);
    }
    port::write_result("get_pages", result)
}

fn run_get_image_request(request: &[u8]) -> u32 {
    let result = port::input(request).and_then(|input| {
        let url = Url::parse(input["args"]["url"].as_str().ok_or("invalid_request")?)
            .map_err(|_| "invalid_request")?;
        if !matches!(url.scheme(), "http" | "https")
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err("invalid_request");
        }
        Ok(json!({"url":url.as_str(),"headers":port::mobile_image_headers(REFERER)}))
    });
    port::write_result("get_image_request", result)
}

fn run_get_listings(request: &[u8]) -> u32 {
    port::write_result("get_listings", listings(request))
}
fn run_get_home(_: &[u8]) -> u32 {
    port::write_failure("get_home", "not_implemented")
}
fn run_get_filters(_: &[u8]) -> u32 {
    port::write_failure("get_filters", "not_implemented")
}
fn run_get_settings(_: &[u8]) -> u32 {
    port::write_failure("get_settings", "not_implemented")
}
fn write_chapter_failure(code: &str) -> u32 {
    port::write_failure("get_chapters", code)
}

koma_source_sdk::koma_source_exports!("navercomic-ko", typed_chapters, write_chapter_failure);
