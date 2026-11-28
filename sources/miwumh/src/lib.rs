#![no_std]
extern crate alloc;

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use koma_source_sdk::{
    body::BodyRef,
    chapters::{Chapter, ChapterNumber, ChapterOrder, ChapterResponse, ChapterResult},
    comments,
    http::{CallError, Client, Request, RequestSpec, RequireSuccess, ResponseKind},
    image_headers::{ImageHeaders, SiteRelation},
    json_utils::request_page_number,
    runtime::Context,
    session::SessionRef,
    source::{SourceCapabilities, SourceInfo},
};
use serde_json::{json, Value};
use url::Url;

#[path = "../../port_support.rs"]
#[allow(dead_code)]
mod port_support;
use port_support::{self as port, Node, Result};

const BASE: &str = "https://miwumh.com";
const LOGIN: &str = "https://miwumh.com/user-settings/?tab=account-settings";
const CDN: &str = "https://t1.bakamh.de";
const CDN_REFERER: &str = "https://miwumh.com/";
const IMAGE_LANGUAGE: &[u8] = b"zh-CN,zh;q=0.9,en-US;q=0.8,en;q=0.7";
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.miwumh.koma",
    name: "迷雾漫画",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "迷雾漫画收录中文翻译的韩国条漫，另设 BL、GL、全年龄和英文专区。登录后可查看作品，部分作品的后记和番外也单独列章。",
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
    filters: false,
    settings: false,
    image_request: true,
    credentials: false,
};

const SECTIONS: &[(&str, &str, &str)] = &[
    ("latest", "最近更新", "/"),
    ("new", "新作", "/new-manhwa/"),
    ("korean", "韩漫", "/zh-manhwa/"),
    ("bl", "BL 漫画", "/bl-manhwa/"),
    ("gl", "GL 漫画", "/gl-manhwa/"),
    ("allages", "全年龄漫画", "/allages-manhwa/"),
    ("english", "英文漫画", "/en-manhwa/"),
    ("ongoing", "连载漫画", "/ongoing-manhwa/"),
    ("ongoing-zh", "中文连载", "/ongoing-manhwa/zh-ongoing/"),
    ("ongoing-bl", "BL 连载", "/ongoing-manhwa/bl-ongoing/"),
    ("ongoing-en", "英文连载", "/ongoing-manhwa/en-ongoing/"),
    ("completed", "完结漫画", "/end-manhwa/"),
    ("completed-zh", "中文完结", "/end-manhwa/zh-completed/"),
    ("completed-bl", "BL 完结", "/end-manhwa/bl-completed/"),
    ("completed-en", "英文完结", "/end-manhwa/en-completed/"),
    ("rank-day", "日榜", "/rankings/?range=day"),
    ("rank-week", "周榜", "/rankings/"),
    ("rank-month", "月榜", "/rankings/?range=month"),
];

#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;
koma_source_sdk::koma_source_buffers! {
    payload: 1024 * 1024, http_out: 6 * 1024 * 1024, body: 4 * 1024 * 1024,
    http_req: 8192, scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

fn numeric(id: &str) -> bool {
    !id.is_empty() && id.len() <= 20 && id.bytes().all(|byte| byte.is_ascii_digit())
}

fn site_url(raw: &str) -> Result<Url> {
    let url = port::absolute(BASE, raw)?;
    (url.origin().ascii_serialization() == BASE)
        .then_some(url)
        .ok_or("invalid_request")
}

fn work_id(path: &str) -> Option<&str> {
    let id = path.strip_prefix("/manhwa/")?.trim_end_matches('/');
    numeric(id).then_some(id)
}

fn chapter_parts(path: &str) -> Option<(&str, &str)> {
    let (work, slug) = path
        .strip_prefix("/manhwa/")?
        .trim_end_matches('/')
        .split_once('/')?;
    let number = slug.strip_prefix("c-")?;
    (numeric(work) && numeric(number)).then_some((work, slug))
}

fn work_url(work: &str) -> Result<Url> {
    if !numeric(work) {
        return Err("invalid_request");
    }
    site_url(&format!("/manhwa/{work}/"))
}

fn requested_work(args: &Value) -> Result<&str> {
    args["mangaId"]
        .as_str()
        .filter(|id| numeric(id))
        .ok_or("invalid_request")
}

fn document(url: &Url) -> Result<Node> {
    let doc = Node::parse(&port::get(url.as_str())?)?;
    if doc.one("body.logged-in").is_none() {
        if doc.one("#form-login, #mkjp-auth-login-title").is_some() {
            return Err("auth_required");
        }
        return Err("parse_error");
    }
    Ok(doc)
}

fn image_url(base: &Url, image: &Node) -> Result<Url> {
    let value = ["data-src", "data-lazy-src", "src"]
        .into_iter()
        .find_map(|attribute| {
            let raw = image.attr(attribute).ok()?;
            let url = port::absolute(base.as_str(), raw.trim()).ok()?;
            matches!(url.origin().ascii_serialization().as_str(), BASE | CDN).then_some(url)
        });
    value.ok_or("parse_error")
}

fn image_descriptor(url: &Url) -> Value {
    if url.origin().ascii_serialization() == BASE {
        // Omitting public headers keeps these covers on get_image_request.
        return json!({"kind":"url","url":url.as_str()});
    }
    json!({"kind":"url","url":url.as_str(),"headers":{"Referer":CDN_REFERER}})
}

fn image_request(args: &Value, request: &[u8]) -> Result<Value> {
    let image = port::absolute(BASE, args["url"].as_str().ok_or("invalid_request")?)?;
    let origin = image.origin().ascii_serialization();
    if !matches!(origin.as_str(), BASE | CDN) {
        return Err("invalid_request");
    }

    let mut referer = args["mangaUrl"]
        .as_str()
        .and_then(|raw| site_url(raw).ok())
        .filter(|url| work_id(url.path()).is_some())
        .unwrap_or(site_url("/")?);
    if let Some((work, chapter, position)) = args["pageId"].as_str()
        .and_then(|id| {
            let (work, remainder) = id.split_once(':')?;
            let (chapter, position) = remainder.split_once(':')?;
            Some((work, chapter, position))
        })
    {
        let path = format!("/manhwa/{work}/{chapter}/");
        if chapter_parts(&path) != Some((work, chapter)) || !numeric(position) {
            return Err("invalid_request");
        }
        referer = site_url(&path)?;
    }

    let referer = if origin == CDN { CDN_REFERER } else { referer.as_str() };
    let context = Context::from_request(request);
    if !host_image_bodies(&context) {
        return Ok(json!({"url":image.as_str(),"headers":{"Referer":referer,"Accept":"image/*,*/*;q=0.8"}}));
    }
    let body = image_body(&image, referer, &context)?;
    let size = body.image_result(scratch_a())?;
    serde_json::from_slice(&scratch_a()[..size]).map_err(|_| "internal_error")
}

fn host_image_bodies(context: &Context<'_>) -> bool {
    [b"source_http_body_v1".as_slice(), b"source_body_v1", b"source_image_body_v1"]
        .iter()
        .all(|feature| context.supports(feature))
}

fn image_body(image: &Url, referer: &str, context: &Context<'_>) -> Result<BodyRef> {
    if context.cancelled() {
        return Err("cancelled");
    }
    let web = context.supports(b"source_web_http_v1");
    let cdn = image.origin().ascii_serialization() == CDN;
    let response = if !web && cdn {
        // Continue to use the isolated public HTTP jar on older hosts.
        let policy = ImageHeaders::new(referer.as_bytes(), IMAGE_LANGUAGE)?;
        let image_headers = policy.headers(SiteRelation::CrossSite);
        let mut spec = RequestSpec::get(image.as_str().as_bytes());
        spec.headers = image_headers.as_slice();
        spec.response = ResponseKind::BodyRef;
        if context.supports(b"source_http_cookie_session_v1") {
            spec.session = SessionRef::SourceCookies;
        }
        Client::<CallError>::new(&[&RequireSuccess])
            .send(spec.encode(http_req_buf())?, http_out())
            .map_err(image_http_error)?
    } else {
        // The host selects an independently authorized session for this exact
        // image origin. It never attaches the main site's cookie to the CDN.
        let mut envelope = json!({
            "version":1,"method":"GET","url":image.as_str(),
            "headers":{"Referer":referer,"Accept":"image/*,*/*;q=0.8"},
            "responseKind":"bodyRef","timeoutMs":15000
        });
        if web {
            envelope["webFetch"] = json!({"version":1});
            if cdn {
                // A same-origin CDN fetch lets WebView choose its Referer.
                // The website's main-origin Referer is for the native lane.
                envelope["headers"] = json!({"Accept":"image/*,*/*;q=0.8"});
            }
        }
        let wire = serde_json::to_vec(&envelope).map_err(|_| "internal_error")?;
        Client::<CallError>::new(&[&RequireSuccess])
            .send(Request::from_json(&wire), http_out())
            .map_err(|error| match error {
                CallError::Rejected("forbidden") if !cdn => "challenge_required",
                _ => image_http_error(error),
            })?
    };
    let body = response.body_ref()?;
    let result = (|| {
        let mut prefix = [0; 32];
        let length = body.read(0, &mut prefix, http_out())?;
        let bytes = &prefix[..length];
        if !bytes.starts_with(b"\xff\xd8\xff")
            && !bytes.starts_with(b"\x89PNG\r\n\x1a\n")
            && !bytes.starts_with(b"GIF8")
            && !(bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP"))
            && !(bytes.get(4..8) == Some(b"ftyp") && matches!(bytes.get(8..12), Some(b"avif" | b"avis")))
        {
            return Err("invalid_response");
        }
        Ok(())
    })();
    if let Err(error) = result {
        let _ = body.close();
        return Err(error);
    }
    Ok(body)
}

fn image_http_error(error: CallError) -> &'static str {
    match error {
        CallError::Rejected(code) => code,
        _ => "network_error",
    }
}

fn manga(id: &str, name: &str, url: &Url) -> Value {
    json!({"id":id,"title":name,"links":[{"kind":"source","url":url.as_str()}]})
}

fn listing_page(path: &str, page: usize) -> Result<Url> {
    if page <= 1 {
        return site_url(path);
    }
    site_url(&format!("{path}page/{page}/"))
}

fn catalogue(url: &Url, page: usize) -> Result<Value> {
    let doc = document(url)?;
    let mut items = Vec::new();
    for card in doc.all(".page-item-detail.manga, article.mkjp-sr__item, .mkjp-rk__card")? {
        let Some(anchor) = card.one(".post-title a, .mkjp-sr__title a, .mkjp-rk__name a") else {
            continue;
        };
        let link = site_url(&anchor.attr("href")?)?;
        let Some(id) = work_id(link.path()) else {
            continue;
        };
        let title = anchor.text()?;
        if title.trim().is_empty() || items.iter().any(|item: &Value| item["id"] == id) {
            continue;
        }
        let mut item = manga(id, title.trim(), &link);
        if let Some(image) = card.one(".item-thumb img, .mkjp-sr__cover img, .mkjp-rk__cover img") {
            if let Ok(cover) = image_url(url, &image) {
                item["cover"] = image_descriptor(&cover);
            }
        }
        if let Some(latest) = card.one(".list-chapter .chapter a, .mkjp-sr__latest a") {
            let title = latest.text()?;
            if !title.trim().is_empty() {
                item["listInfo"] = json!([{"type":"text","text":title.trim()}]);
            }
        }
        items.push(item);
    }

    if items.is_empty() && doc.one(".no-results, .not-found").is_none() {
        let headings = doc.all("h1, h2, h3, .mkjp-rk")?;
        let known_empty = headings.iter().any(|heading| {
            heading.text().is_ok_and(|text| {
                text.contains("未找到任何内容")
                    || text.contains("没有找到")
                    || text.contains("排行榜数据累积中")
            })
        });
        if !known_empty {
            return Err("parse_error");
        }
    }
    let next = if url.path().starts_with("/rankings/") {
        None
    } else {
        doc.one("a.nextpostslink, a.next.page-numbers, a[rel='next']")
            .and_then(|_| page.checked_add(1))
    };
    Ok(port::items(items, next))
}

fn details(work: &str) -> Result<Value> {
    let url = work_url(work)?;
    let doc = document(&url)?;
    let title = doc.require("h1.mkjp-mh__title")?.text()?;
    let mut result = manga(work, title.trim(), &url);
    if let Some(image) = doc.one(".mkjp-mh__cover img") {
        if let Ok(cover) = image_url(&url, &image) {
            result["cover"] = image_descriptor(&cover);
        }
    }
    if let Some(original) = doc.one(".mkjp-mh__alt") {
        let aliases = original.text()?;
        result["alternateTitles"] = json!(aliases
            .split([',', '，'])
            .map(str::trim)
            .filter(|name| !name.is_empty() && *name != title.trim())
            .collect::<Vec<_>>());
    }
    let mut genres = texts(&doc, ".mkjp-mh__genres a")?;
    for tag in texts(&doc, ".mkjp-tagc-item a")? {
        if !genres.contains(&tag) {
            genres.push(tag);
        }
    }
    let authors = texts(&doc, ".mkjp-mh__by a")?;
    if !genres.is_empty() {
        result["tags"] = json!(genres);
    }
    if !authors.is_empty() {
        result["authors"] = json!(authors);
    }
    if let Some(status) = doc.one(".mkjp-mh__state .mkjp-mh__badge") {
        match status.text()?.trim() {
            "完结" | "完結" => result["status"] = json!("completed"),
            "连载" | "連載" | "连载中" | "連載中" => result["status"] = json!("ongoing"),
            _ => {}
        }
    }
    if let Some(summary) = doc.one(".mkjp-summary__text, .summary__content") {
        let paragraphs = texts(&summary, "p")?;
        result["description"] = json!(if paragraphs.is_empty() {
            summary.text()?
        } else {
            paragraphs.join("\n\n")
        });
    }
    Ok(json!({"manga":result}))
}

fn texts(doc: &Node, selector: &str) -> Result<Vec<String>> {
    let mut values = Vec::new();
    for node in doc.all(selector)? {
        let text = node.text()?;
        let value = text.trim();
        if !value.is_empty() {
            values.push(value.to_string());
        }
    }
    Ok(values)
}

fn chapter_number(title: &str) -> Option<&str> {
    let text = title.trim().strip_prefix('第')?.trim_start();
    let end = text.find(|c: char| !c.is_ascii_digit() && c != '.')?;
    let number = text[..end].trim_end_matches('.');
    let unit = text[end..].trim_start().chars().next()?;
    (!number.is_empty() && matches!(unit, '话' | '話' | '章')).then_some(number)
}

fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(request)?;
    let owner = requested_work(&input["args"])?;
    let url = work_url(owner)?;
    let doc = document(&url)?;
    let list = doc.require("ul.version-chap")?;
    let mut entries = Vec::new();
    for row in list.all("li.mkjp-chapter")? {
        // The thumbnail precedes the title and is an empty link to the same chapter.
        let anchor = row.one(".mkjp-chapter__text a[href]")
            .or_else(|| row.one("a[href]:not(.mkjp-chapter__thumb)"))
            .or_else(|| row.one("a[href]"))
            .ok_or("parse_error")?;
        let link = site_url(&anchor.attr("href")?)?;
        let (work, slug) = chapter_parts(link.path()).ok_or("parse_error")?;
        if work != owner {
            return Err("parse_error");
        }
        let id = format!("{work}:{slug}");
        if entries.iter().any(|(other, _, _, _)| *other == id) {
            continue;
        }
        let mut name = anchor.text()?;
        if name.is_empty() {
            if let Some(title) = row.one(".mkjp-ea-lock__title") {
                name = title.text()?;
            }
        }
        let locked = row.one(".mkjp-ea-lock__title, .mkjp-ea-lock").is_some();
        entries.push((id, link, name, locked));
    }
    if entries.is_empty() {
        return Err("parse_error");
    }
    let chapters = entries
        .iter()
        .map(|(id, link, title, locked)| {
            let mut chapter =
                Chapter::new(id.as_bytes(), owner.as_bytes(), title.trim().as_bytes());
            chapter.url = Some(link.as_str().as_bytes());
            chapter.chapter_number =
                chapter_number(title).map(|number| ChapterNumber::Text(number.as_bytes()));
            if *locked {
                chapter.locked = Some(true);
            }
            chapter
        })
        .collect::<Vec<_>>();
    // The directory starts with the site's Latest link and ends at Start Reading.
    // Specials and chapters whose route ID disagrees with their label keep that order.
    ChapterResponse::ordered(payload_buf(), request, ChapterOrder::NewestFirst, &chapters)
        .map_err(port::chapter_error)
}

fn pages(args: &Value, request: &[u8]) -> Result<Value> {
    let id = args["chapterId"].as_str().ok_or("invalid_request")?;
    let (owner, slug) = id.split_once(':').ok_or("invalid_request")?;
    let path = format!("/manhwa/{owner}/{slug}/");
    if chapter_parts(&path) != Some((owner, slug))
        || args["mangaId"].as_str().is_some_and(|work| work != owner)
    {
        return Err("invalid_request");
    }
    let mut url = site_url(&path)?;
    let mut doc = document(&url)?;
    if doc.one("#single-pager").is_some() {
        url.query_pairs_mut().append_pair("style", "list");
        doc = document(&url)?;
    }
    if doc.one(".mkjp-ea-lock__title").is_some() {
        return Err(
            if doc
                .one(".mkjp-ea-lock [data-target='#form-login']")
                .is_some()
            {
                "auth_required"
            } else {
                "chapter_locked"
            },
        );
    }
    let mut pages = Vec::new();
    let context = Context::from_request(request);
    let mut check_cdn = context.supports(b"source_web_http_v1") && host_image_bodies(&context);
    for node in doc.all(".reading-content .page-break img.wp-manga-chapter-img")? {
        let image = image_url(&url, &node)?;
        if check_cdn && image.origin().ascii_serialization() == CDN {
            // The chapter operation owns verification and retries its exact
            // rejected request. A reader page ID stays independent of it.
            image_body(&image, CDN_REFERER, &context)?.close()?;
            check_cdn = false;
        }
        let index = pages.len();
        pages.push(json!({"id":format!("{id}:{}",index + 1),"index":index,
            "image":image_descriptor(&image)}));
    }
    if pages.is_empty() {
        return Err("chapter_unavailable");
    }
    Ok(json!({"pages":pages}))
}

fn dispatch(op: &str, request: &[u8]) -> Result<Value> {
    let input = port::input(request)?;
    let args = &input["args"];
    let page = request_page_number(request);
    match op {
        "get_listings" => Ok(json!({"listings":SECTIONS.iter().map(|(id, name, _)|
            json!({"id":id,"name":name,"supportsPaging":!id.starts_with("rank-"),"supportsFilters":false})
        ).collect::<Vec<_>>()})),
        "get_manga_list" => {
            let id = args["listingId"].as_str().unwrap_or("latest");
            let (_, _, path) = SECTIONS
                .iter()
                .find(|(name, _, _)| *name == id)
                .ok_or("invalid_request")?;
            catalogue(&listing_page(path, page)?, page)
        }
        "search" => {
            let query = args["query"].as_str().unwrap_or("").trim();
            if query.is_empty() {
                return Ok(port::items(Vec::new(), None));
            }
            if let Ok(link) = Url::parse(query) {
                let link = site_url(link.as_str())?;
                let id = work_id(link.path())
                    .or_else(|| chapter_parts(link.path()).map(|(id, _)| id))
                    .ok_or("invalid_request")?;
                return Ok(port::items(
                    alloc::vec![details(id)?["manga"].clone()],
                    None,
                ));
            }
            let mut url = listing_page("/", page)?;
            url.query_pairs_mut()
                .append_pair("s", query)
                .append_pair("post_type", "wp-manga");
            catalogue(&url, page)
        }
        "get_manga" => details(requested_work(args)?),
        "get_pages" => pages(args, request),
        "get_image_request" => image_request(args, request),
        "get_authentication" => {
            Ok(json!({"authentication":{"kind":"web","loginUrl":LOGIN},"authenticated":false}))
        }
        _ => Err("unsupported_operation"),
    }
}

fn run(op: &str, request: &[u8]) -> u32 {
    port::write_result(op, dispatch(op, request))
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
fn checked_comment_page(base: &Url, value: &str) -> Result<Url> {
    let mut page = site_url(value)?;
    let same_path = page.path() == base.path();
    let paged_path = page.path().strip_prefix(base.path())
        .and_then(|path| path.strip_prefix("comment-page-"))
        .is_some_and(|page| numeric(page.trim_end_matches('/')));
    let paged_query = {
        let mut parameters = page.query_pairs();
        same_path && parameters.next().is_some_and(|(key, value)| {
            key == "cpage" && numeric(&value)
        }) && parameters.next().is_none()
    };
    if !((paged_path && page.query().is_none()) || paged_query) {
        return Err("invalid_request");
    }
    page.set_fragment(None);
    Ok(page)
}

fn source_comments(request: &[u8]) -> Result<Value> {
    let input = comments::request(request)?;
    let work = core::str::from_utf8(input.manga_id).map_err(|_| "invalid_request")?;
    let mut base = work_url(work)?;
    if let Some(raw) = input.chapter_id {
        let id = core::str::from_utf8(raw).map_err(|_| "invalid_request")?;
        let (owner, slug) = id.split_once(':').ok_or("invalid_request")?;
        let path = format!("/manhwa/{owner}/{slug}/");
        if owner != work || chapter_parts(&path) != Some((owner, slug)) {
            return Err("invalid_request");
        }
        base = site_url(&path)?;
    }

    let (direction, page) = if let Some(raw) = input.cursor {
        let cursor = core::str::from_utf8(raw).map_err(|_| "invalid_request")?;
        let (direction, address) = cursor.split_once('|').ok_or("invalid_request")?;
        if !matches!(direction, "older" | "newer") {
            return Err("invalid_request");
        }
        (Some(direction), checked_comment_page(&base, address)?)
    } else {
        (None, base.clone())
    };
    let doc = document(&page)?;
    let Some(discussion) = doc.one("#madara-comments") else {
        return Ok(json!({"items":[]}));
    };
    let mut items = Vec::new();
    for row in discussion.all(".comment[id^='comment-']")? {
        let id = row.attr("id")?;
        let body = row.one(".comment-body").unwrap_or(row);
        if body.one(".comment-awaiting-moderation").is_some() {
            continue;
        }
        let author = body.require(".comment-author .fn, .comment-author a")?.text()?;
        let content_node = body.require(".comment-content")?;
        let paragraphs = texts(&content_node, "p")?;
        let text = if paragraphs.is_empty() { content_node.text()? } else { paragraphs.join("\n\n") };
        if author.is_empty() || text.is_empty() {
            continue;
        }
        let mut item = json!({"id":id,"author":author,"content":text});
        if let Some(time) = body.one("time[datetime]") {
            item["createdAt"] = json!(time.attr("datetime")?);
        }
        items.push(item);
    }
    let mut result = json!({"items":items});
    for (candidate, selector) in [
        ("older", ".comment-navigation .nav-previous a, a.prev.page-numbers"),
        ("newer", ".comment-navigation .nav-next a, a.next.page-numbers"),
    ] {
        if direction.is_some_and(|current| current != candidate) {
            continue;
        }
        let Some(link) = discussion.one(selector) else { continue; };
        let address = page.join(&link.attr("href")?).map_err(|_| "parse_error")?;
        if let Ok(next) = checked_comment_page(&base, address.as_str()) {
            if next != page {
                result["nextCursor"] = json!(format!("{candidate}|{next}"));
                break;
            }
        }
    }
    Ok(result)
}

fn run_get_comments(request: &[u8]) -> u32 {
    port::write_result(comments::OPERATION, source_comments(request))
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

koma_source_sdk::koma_source_exports!("miwumh", typed_chapters, chapter_failure);
koma_source_sdk::koma_source_authentication_exports!("miwumh");
koma_source_sdk::koma_source_comments_export!(run_get_comments);
