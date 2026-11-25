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
    comments,
    json_utils::request_page_number,
    source::{SourceCapabilities, SourceInfo},
};
use serde_json::{json, Value};
use url::Url;
#[path = "../../javascript_packer.rs"]
mod javascript_packer;
#[path = "../../port_support.rs"]
mod port_support;
use port_support::{self as port, Node, Result};

const BASE: &str = "https://www.1kkk.com";
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:133.0) Gecko/20100101 Firefox/133.0";
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.jisumanhua.koma",
    name: "极速漫画",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "极速漫画的原创与日本漫画，包含热血冒险、奇幻、恋爱及校园等题材；原站锁定或下架章节依其状态保留。",
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
    settings: true,
    image_request: true,
    credentials: false,
};
#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;
koma_source_sdk::koma_source_buffers! {payload:1024*1024,http_out:5*1024*1024,body:4*1024*1024,http_req:4096,scratch:8192,}
koma_source_sdk::koma_source_helpers!();

fn numeric(value: &str) -> bool {
    !value.is_empty() && value.len() <= 18 && value.bytes().all(|b| b.is_ascii_digit())
}
fn safe_path(raw: &str) -> Result<String> {
    let url = port::absolute(BASE, raw)?;
    if !matches!(
        url.host_str(),
        Some("www.1kkk.com" | "m.1kkk.com" | "1kkk.com")
    ) {
        return Err("invalid_request");
    }
    Ok(format!("{}/", url.path().trim_end_matches('/')))
}
fn manga_path(raw: &str) -> Result<String> {
    let path = safe_path(raw)?;
    if !path
        .strip_prefix("/manhua")
        .and_then(|id| id.strip_suffix('/'))
        .is_some_and(numeric)
    {
        return Err("invalid_request");
    }
    Ok(path)
}
fn cid(path: &str) -> Result<&str> {
    let route = path.strip_suffix('/').ok_or("invalid_request")?;
    for prefix in ["/other", "/vol", "/m"] {
        if let Some(id) = route.strip_prefix(prefix).filter(|id| numeric(id)) {
            return Ok(id);
        }
    }
    let route = route
        .strip_prefix("/ch")
        .or_else(|| route.strip_prefix("/vol"))
        .ok_or("invalid_request")?;
    let (label, cid) = route.rsplit_once('-').ok_or("invalid_request")?;
    if !numeric(cid)
        || label.is_empty()
        || !label
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.'))
    {
        return Err("invalid_request");
    }
    Ok(cid)
}

fn network(url: &str, referer: &str) -> Result<String> {
    port::request(
        url,
        "GET",
        json!({"Accept-Language":"zh-TW","User-Agent":UA,
        "Referer":referer,"Cookie":"isAdult=1"}),
        None,
    )
}
fn doc(path: &str) -> Result<Node> {
    Node::parse(&network(&format!("{BASE}{path}"), BASE)?)
}
fn cover(url: &str, path: &str) -> Result<Value> {
    let image = port::absolute(BASE, url)?;
    Ok(json!({"kind":"url","url":image.as_str(),"headers":{"Referer":format!("{BASE}{path}")}}))
}
fn card(node: &Node, req: &[u8]) -> Result<Value> {
    let link = node.require("h2.title > a, .title > a")?;
    let id = manga_path(&link.attr("href")?)?;
    let mut row = json!({"id":id,"title":link.text()?,"links":[{"kind":"source","url":format!("{BASE}{id}")}]});
    if let Some(p) = node.one("p.mh-cover") {
        let style = p.attr("style")?;
        if let Some((_, rest)) = style.split_once("url(") {
            if let Some((image, _)) = rest.split_once(')') {
                row["cover"] = cover(image.trim().trim_matches(['\'', '"']), &id)?;
            }
        }
    } else if let Some(img) = node.one("img[src]") {
        row["cover"] = cover(&img.attr("src")?, &id)?;
    }
    // Optional list summary: the listing card carries the latest chapter
    // (p.chapter > a). The p.zl rating is only a star CSS class with no text
    // value, so it is not surfaced. Only add the field when the host negotiates
    // the feature; otherwise the item bytes stay unchanged.
    if koma_source_sdk::source_protocol::feature(req, koma_source_sdk::list_info::HOST_FEATURE) {
        if let Some(ch) = node.one("p.chapter > a") {
            if let Ok(t) = ch.text() {
                if !t.is_empty() {
                    row["listInfo"] = json!([{"type":"text","label":"最新","text":t}]);
                }
            }
        }
    }
    Ok(row)
}
fn detail(path: &str, node: &Node) -> Result<Value> {
    let info = node.require(".banner_detail_form")?;
    let title_node = info.require("p.title")?;
    let title_with_score = title_node.text()?;
    let score = title_node
        .one("span.score")
        .map(|n| n.text())
        .transpose()?
        .unwrap_or_default();
    let title = if score.is_empty() {
        title_with_score.as_str()
    } else {
        title_with_score
            .strip_suffix(&score)
            .unwrap_or(&title_with_score)
            .trim_end()
    };
    let mut row =
        json!({"id":path,"title":title,"links":[{"kind":"source","url":format!("{BASE}{path}")}]});
    if let Some(img) = info.one("img[src]") {
        row["cover"] = cover(&img.attr("src")?, path)?;
    }
    let mut authors = Vec::new();
    for a in info.all(".subtitle a")? {
        authors.push(a.text()?);
    }
    if !authors.is_empty() {
        row["authors"] = json!(authors);
    }
    if let Some(desc) = info.one("p.content") {
        let text = desc.text()?;
        row["description"] = json!(if score.is_empty() {
            text
        } else {
            format!("评分：{score}\n\n{text}")
        });
    }
    let mut tags = Vec::new();
    for part in info.all(".tip a")? {
        tags.push(part.text()?);
    }
    row["tags"] = json!(tags);
    let status = info
        .one(".block span")
        .map(|node| node.text())
        .transpose()?
        .unwrap_or_default();
    row["status"] = json!(if status.contains("完结") {
        "completed"
    } else if status.contains("连载") {
        "ongoing"
    } else {
        "unknown"
    });
    Ok(row)
}
fn listing(req: &[u8], args: &Value, search: bool) -> Result<Value> {
    let page = request_page_number(req);
    let mut loc = port::absolute(BASE, if search { "/search" } else { "/" })?;
    if search {
        let query = args["query"].as_str().unwrap_or("").trim();
        if let Ok(url) = Url::parse(query) {
            if matches!(url.scheme(), "https" | "http") {
                let path = safe_path(url.as_str())?;
                let owner = if manga_path(&path).is_ok() {
                    path
                } else {
                    cid(&path)?;
                    let d = doc(&path)?;
                    manga_path(&d.require("a.back")?.attr("href")?)?
                };
                return Ok(port::items(
                    alloc::vec![detail(&owner, &doc(&owner)?)?],
                    None,
                ));
            }
        }
        loc.query_pairs_mut()
            .append_pair("title", query)
            .append_pair("language", "1")
            .append_pair("page", &page.to_string());
    } else {
        let sort = if args["listingId"] == "latest" {
            "-s2"
        } else {
            ""
        };
        loc.set_path(&format!("/manhua-list{sort}-p{page}/"));
    }
    let d = Node::parse(&network(loc.as_str(), BASE)?)?;
    let mut items = Vec::new();
    let mut seen = BTreeSet::new();
    for node in d.all("ul.mh-list > li > div.mh-item, div.banner_detail_form")? {
        let Ok(row) = card(&node, req) else { continue };
        if seen.insert(row["id"].as_str().unwrap_or("").to_string()) {
            items.push(row);
        }
    }
    let mut next = false;
    for link in d.all("div.page-pagination a[href]")? {
        if link.text()?.trim() == ">" {
            next = true;
            break;
        }
    }
    Ok(port::items(
        items,
        if next { page.checked_add(1) } else { None },
    ))
}
fn run_get_chapters(req: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(req)?;
    let owner = manga_path(input["args"]["mangaId"].as_str().ok_or("invalid_request")?)?;
    let d = doc(&owner)?;
    if d.one(".warning-bar").is_some() {
        return Err("chapter_unavailable");
    }
    d.require("#chapterlistload")?;
    let mut names = Vec::new();
    for name in d.all(".detail-list-title > a.block")? {
        let raw = name.text()?;
        names.push(raw.split('（').next().unwrap_or("").trim().to_string());
    }
    let ascending = d
        .one("div.detail-list-title a.order")
        .map(|n| n.text())
        .transpose()?
        .as_deref()
        == Some("正序");
    let sort = matches!(input["settings"]["sortChapter"].as_str(), Some("true"))
        || input["settings"]["sortChapter"].as_bool() == Some(true);
    let mut catalog = GroupedChapters::new(payload_buf(), req).map_err(port::chapter_error)?;
    let mut rows = Vec::new();
    let mut seen = BTreeSet::new();
    for (group, ul) in d.all("#chapterlistload > ul")?.into_iter().enumerate() {
        let mut group_rows = Vec::new();
        for link in ul.all("li > a[href]")? {
            let id = safe_path(&link.attr("href")?)?;
            let number = cid(&id)?.parse::<u64>().map_err(|_| "parse_error")?;
            if !seen.insert(id.clone()) {
                continue;
            }
            let title = link
                .one("p.title")
                .map(|node| node.text())
                .transpose()?
                .unwrap_or(link.text()?);
            let locked = link.one(".detail-lock, .view-lock").is_some();
            group_rows.push((id, title, locked, number));
        }
        if sort {
            group_rows.sort_by_key(|row| core::cmp::Reverse(row.3));
        } else if ascending {
            group_rows.reverse();
        }
        let size = group_rows.len();
        for (index, (id, title, locked, number)) in group_rows.into_iter().enumerate() {
            rows.push((id, title, locked, number, group, (size - index - 1) as u64));
        }
    }
    if sort {
        rows.sort_by_key(|row| core::cmp::Reverse(row.3));
    }
    for (index, (id, title, locked, _, group, position)) in rows.iter().enumerate() {
        let group_id = format!("directory-{group}");
        let mut chapter = Chapter::new(id.as_bytes(), owner.as_bytes(), title.as_bytes());
        chapter.index = Some(index);
        chapter.locked = locked.then_some(true);
        chapter.scanlator = names
            .get(*group)
            .filter(|name| !name.is_empty())
            .map(|name| name.as_bytes());
        catalog
            .push(
                chapter,
                ChapterPlacement::InGroup {
                    group: group_id.as_bytes(),
                    position: *position,
                },
            )
            .map_err(port::chapter_error)?;
    }
    catalog
        .finish(ChapterPage::Complete)
        .map_err(port::chapter_error)
}
fn field<'a>(html: &'a str, name: &str) -> Result<&'a str> {
    let marker = format!("var {name}");
    let value = html
        .match_indices(&marker)
        .find_map(|(index, _)| {
            html[index + marker.len()..]
                .trim_start()
                .strip_prefix('=')
                .map(str::trim_start)
        })
        .ok_or("parse_error")?;
    Ok(value
        .split(';')
        .next()
        .ok_or("parse_error")?
        .trim()
        .trim_matches(['"', '\'']))
}
fn chapter_page(args: &Value) -> Result<(String, String)> {
    let chapter = safe_path(args["chapterId"].as_str().ok_or("invalid_request")?)?;
    let expected = cid(&chapter)?;
    let url = format!("{BASE}{chapter}");
    let html = network(&url, BASE)?;
    if field(&html, "DM5_CID")? != expected {
        return Err("invalid_request");
    }
    if let Some(work) = args["mangaId"].as_str() {
        let work = manga_path(work)?;
        if work != format!("/manhua{}/", field(&html, "DM5_MID")?) {
            return Err("invalid_request");
        }
    }
    Ok((url, html))
}
fn pages(args: &Value) -> Result<Value> {
    let (url, html) = chapter_page(args)?;
    if field(&html, "DM5_VIEWSIGN_DT").is_err() {
        return Err("chapter_locked");
    }
    let chapter = Url::parse(&url).map_err(|_| "parse_error")?;
    let document = Node::parse(&html)?;
    let images = document.all("#barChapter img.load-src[data-src]")?;
    let mut output = Vec::new();
    if !images.is_empty() {
        for (index, image) in images.into_iter().enumerate() {
            let raw = image.attr("data-src")?;
            let mut page = port::page(index, chapter.path(), port::absolute(&url, &raw)?);
            page["image"]["headers"] = json!({"Referer":url});
            output.push(page);
        }
    } else {
        let count = field(&html, "DM5_IMAGE_COUNT")?
            .parse::<usize>()
            .ok()
            .filter(|count| (1..=4096).contains(count))
            .ok_or("chapter_unavailable")?;
        for index in 0..count {
            let mut page_url = chapter.clone();
            page_url
                .query_pairs_mut()
                .append_pair("koma-page", &(index + 1).to_string());
            output.push(port::page(index, chapter.path(), page_url));
        }
    }
    Ok(json!({"pages":output}))
}
fn image_request(args: &Value) -> Result<Value> {
    let raw = args["url"].as_str().ok_or("invalid_request")?;
    let image = port::absolute(BASE, raw)?;
    if image.host_str() != Some("www.1kkk.com")
        || image.query_pairs().all(|(k, _)| k != "koma-page")
    {
        let mut result = json!({"url":image.as_str()});
        if let Some(chapter) = args["chapterId"].as_str() {
            let path = safe_path(chapter)?;
            cid(&path)?;
            result["headers"] = json!({"Referer":format!("{BASE}{path}")});
        }
        return Ok(result);
    }
    let origin = safe_path(image.as_str())?;
    let number = image
        .query_pairs()
        .find(|(key, _)| key == "koma-page")
        .ok_or("invalid_request")?
        .1;
    let page = number
        .parse::<usize>()
        .ok()
        .filter(|n| *n > 0)
        .ok_or("invalid_request")?;
    let mut actual = args.clone();
    actual["chapterId"] = json!(origin);
    let (referer, html) = chapter_page(&actual)?;
    let count = field(&html, "DM5_IMAGE_COUNT")?
        .parse::<usize>()
        .map_err(|_| "parse_error")?;
    if page > count {
        return Err("invalid_request");
    }
    let mut ajax = port::absolute(&referer, "chapterfun.ashx")?;
    let cid = field(&html, "DM5_CID")?;
    ajax.query_pairs_mut().extend_pairs([
        ("cid", cid),
        ("page", number.as_ref()),
        ("key", ""),
        ("language", "1"),
        ("gtk", "6"),
        ("_cid", cid),
        ("_mid", field(&html, "DM5_MID")?),
        ("_dt", field(&html, "DM5_VIEWSIGN_DT")?),
        ("_sign", field(&html, "DM5_VIEWSIGN")?),
    ]);
    let packed = network(ajax.as_str(), &referer)?;
    let js = javascript_packer::expand(&packed)?;
    let pix = js
        .split_once("pix=\"")
        .and_then(|(_, s)| s.split_once('"'))
        .ok_or("parse_error")?
        .0;
    let path = js
        .split_once("pvalue=[\"")
        .and_then(|(_, s)| s.split_once('"'))
        .ok_or("parse_error")?
        .0;
    let suffix = if let Some((_, rest)) = js.split_once("pix+pvalue[i]+\\'") {
        rest.split_once("\\'").ok_or("parse_error")?.0
    } else if let Some((_, rest)) = js.split_once("pix+pvalue[i]+\"") {
        rest.split_once('"').ok_or("parse_error")?.0
    } else {
        return Err("parse_error");
    };
    let image = port::absolute(&referer, &format!("{pix}{path}{suffix}"))?;
    Ok(
        json!({"url":image.as_str(),"headers":{"Referer":referer,"User-Agent":UA,"Accept-Language":"zh-TW"}}),
    )
}
fn site_comments(req: &[u8]) -> Result<Value> {
    let input = comments::request(req)?;
    let chapter = input.chapter_id.ok_or("invalid_request")?;
    let text = |raw| core::str::from_utf8(raw).map_err(|_| "invalid_request");
    let (referer, html) =
        chapter_page(&json!({"mangaId":text(input.manga_id)?,"chapterId":text(chapter)?}))?;
    let cid = field(&html, "DM5_CID")?;
    let topic = field(&html, "DM5_TIEBATOPICID")?;
    let limit = field(&html, "DM5_PAGEPCOUNT")?
        .parse::<usize>()
        .map_err(|_| "parse_error")?;
    let page = comments::position(input.cursor)?.unwrap_or(1);
    if page == 0 || !numeric(topic) {
        return Err("invalid_request");
    }
    if topic == "0" || page > limit {
        return Ok(json!({"items":[]}));
    }
    let url = format!(
        "{BASE}/m{cid}/pagerdata.ashx?pageindex={page}&pagesize=10&tid={topic}&cid={cid}&t=9"
    );
    let body = network(&url, &referer)?;
    let rows: Vec<Value> = serde_json::from_str(&body).map_err(|_| "parse_error")?;
    let mut items = Vec::new();
    for row in &rows {
        let id = row["Id"].as_u64().ok_or("parse_error")?.to_string();
        let author = row["Poster"].as_str().unwrap_or("");
        let content = row["PostContent"].as_str().ok_or("parse_error")?;
        let mut row = json!({"id":id,"author":author,"content":content,"createdAt":row["PostTime"].as_str().unwrap_or("")});
        if row["createdAt"] == "" {
            row.as_object_mut()
                .ok_or("parse_error")?
                .remove("createdAt");
        }
        items.push(row);
    }
    let mut result = json!({"items":items});
    if rows.len() >= 10 && page < limit {
        result["nextCursor"] = json!((page + 1).to_string());
    }
    Ok(result)
}
fn run_comments(req: &[u8]) -> u32 {
    port::write_result(comments::OPERATION, site_comments(req))
}
koma_source_sdk::koma_source_comments_export!(run_comments);

fn dispatch(op: &str, req: &[u8]) -> Result<Value> {
    let input = port::input(req)?;
    let args = &input["args"];
    match op {
        "get_listings" => Ok(port::listing_list(true, true, false)),
        "get_settings" => Ok(
            json!({"settings":[{"id":"sortChapter","name":"依照上传时间排序章节","kind":"toggle","default":"false"}]}),
        ),
        "get_manga_list" => listing(req, args, false),
        "search" => listing(req, args, true),
        "get_manga" => {
            let path = manga_path(args["mangaId"].as_str().ok_or("invalid_request")?)?;
            Ok(json!({"manga":detail(&path,&doc(&path)?)?}))
        }
        "get_pages" => pages(args),
        "get_image_request" => image_request(args),
        _ => Err("not_implemented"),
    }
}
fn run(op: &str, r: &[u8]) -> u32 {
    port::write_result(op, dispatch(op, r))
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
koma_source_sdk::koma_source_exports!("jisumanhua", typed_chapters, write_chapter_failure);
