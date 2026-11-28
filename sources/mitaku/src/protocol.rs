// Independent Mitaku protocol. Excludes video-only and no-image posts; no fake latest.

extern crate alloc;
use alloc::{collections::BTreeSet, format, string::String, vec::Vec};
use koma_source_sdk::{
    chapters::{Chapter, ChapterPage, ChapterPlacement, ChapterResult, GroupedChapters},
    host,
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
koma_source_sdk::koma_source_buffers! {payload:1024*1024,http_out:4*1024*1024,body:2*1024*1024,http_req:8192,scratch:8192,}
koma_source_sdk::koma_source_helpers!();
fn label(request: &[u8], key: &str) -> Result<String> {
    let mut out = [0; 2048];
    Catalog::from_metadata(METADATA)
        .map_err(|_| "internal_error")?
        .text(&Context::from_request(request), key, key, &mut out)
        .map(String::from)
        .map_err(|_| "internal_error")
}
fn fail(request: &[u8], op: &str, code: &str) -> u32 {
    let key = format!("error.{code}");
    let message = label(request, &key)
        .ok()
        .filter(|x| x != &key)
        .or_else(|| label(request, "error.unavailable").ok())
        .unwrap_or_else(|| String::from("Unavailable"));
    write_error(op, code, &message)
}
fn output(request: &[u8], op: &str, v: Result<Value>) -> u32 {
    match v {
        Ok(v) => match serde_json::to_vec(&v) {
            Ok(b) if b.len() <= payload_buf().len() => {
                payload_buf()[..b.len()].copy_from_slice(&b);
                write_success_payload(op, b.len())
            }
            _ => fail(request, op, "result_too_large"),
        },
        Err(c) => fail(request, op, c),
    }
}
fn doc(url: &str, xhr: bool) -> Result<Node> {
    let headers = if xhr {
        json!({"Accept":"text/html","X-Requested-With":"XMLHttpRequest","Referer":BASE})
    } else {
        json!({"Accept":"text/html","Referer":BASE})
    };
    let s = port::request(url, "GET", headers, None)?;
    Node::parse(&s)
}
fn page(input: &Value) -> Result<usize> {
    match &input["args"]["page"]["cursor"] {
        Value::Null => Ok(1),
        v => v
            .as_str()
            .filter(|v| !v.is_empty() && v.bytes().all(|c| c.is_ascii_digit()))
            .and_then(|v| v.parse().ok())
            .filter(|p| *p > 0 && *p < 10000)
            .ok_or("invalid_request"),
    }
}
fn valid_url(raw: &str, image: bool) -> Result<Url> {
    let u = port::absolute(BASE, raw)?;
    if u.scheme() != "https" || u.port().is_some() || u.fragment().is_some() || u.path() == "/" {
        return Err("invalid_request");
    };
    let root = Url::parse(BASE).map_err(|_| "internal_error")?;
    let host = u.host_str().ok_or("invalid_request")?;
    if host != root.host_str().ok_or("internal_error")? && (!image || host != IMAGE_HOST) {
        return Err("invalid_request");
    };
    Ok(u)
}
fn work_url(id: &str) -> Result<Url> {
    let u = valid_url(id, false)?;
    if !id.starts_with('/')
        || id.starts_with("//")
        || u.query().is_some()
        || u.path() != id
        || !valid_work_path(id)
    {
        return Err("invalid_request");
    };
    Ok(u)
}
fn work(input: &Value) -> Result<(&str, Url)> {
    let id = input["args"]["mangaId"].as_str().ok_or("invalid_request")?;
    Ok((id, work_url(id)?))
}
fn image(raw: &str) -> Result<Value> {
    let u = valid_url(raw, true)?;
    Ok(json!({"kind":"url","url":u.as_str(),"headers":{"Referer":BASE}}))
}
fn attr_img(n: &Node) -> Result<String> {
    for k in ["srcset", "data-cfsrc", "data-src", "data-lazy-src", "src"] {
        if let Ok(s) = n.attr(k) {
            if !s.is_empty() {
                return Ok(if k == "srcset" {
                    String::from(s.split_whitespace().next().ok_or("parse_error")?)
                } else {
                    s
                });
            }
        }
    }
    Err("parse_error")
}
fn title(d: &Node, selector: &str) -> Result<String> {
    let t = d.require(selector)?.text()?;
    if t.is_empty() {
        Err("parse_error")
    } else {
        Ok(t)
    }
}
fn select_filter(
    request: &[u8],
    id: &str,
    key: &str,
    values: &[(&str, &str)],
    default: &str,
) -> Result<Value> {
    let mut options = Vec::new();
    for (v, k) in values {
        options.push(json!({"value":v,"label":label(request,k)?}))
    }
    Ok(
        json!({"id":id,"type":"select","name":label(request,key)?,"default":default,"options":options}),
    )
}
fn run_get_listings(r: &[u8]) -> u32 {
    output(
        r,
        "get_listings",
        (move || {
            port::input(r)?;
            let mut rows = alloc::vec![
                json!({"id":"popular","kind":"popular","name":label(r,POPULAR_KEY)?,"supportsPaging":true,"supportsFilters":true})
            ];
            if HAS_LATEST {
                rows.push(json!({"id":"latest","kind":"latest","name":label(r,"listing.latest")?,"supportsPaging":true,"supportsFilters":true}))
            }
            Ok(json!({"listings":rows}))
        })(),
    )
}
fn run_get_settings(r: &[u8]) -> u32 {
    output(
        r,
        "get_settings",
        port::input(r).map(|_| json!({"settings":[]})),
    )
}
fn run_get_filters(r: &[u8]) -> u32 {
    output(r, "get_filters", port::input(r).and_then(|_| filters(r)))
}
fn run_get_manga_list(r: &[u8]) -> u32 {
    output(
        r,
        "get_manga_list",
        port::input(r).and_then(|v| browse(&v, false)),
    )
}
fn run_search(r: &[u8]) -> u32 {
    output(r, "search", port::input(r).and_then(|v| browse(&v, true)))
}
fn run_get_manga(r: &[u8]) -> u32 {
    output(
        r,
        "get_manga",
        port::input(r).and_then(|v| {
            let (id, u) = work(&v)?;
            let d = doc(u.as_str(), false)?;
            detail(id, &d).map(|m| json!({"manga":m}))
        }),
    )
}
fn run_get_pages(r: &[u8]) -> u32 {
    output(r,"get_pages",port::input(r).and_then(|v|{let id=v["args"]["chapterId"].as_str().ok_or("invalid_request")?;let u=work_url(id)?;if let Some(m)=v["args"].get("mangaId"){if m.as_str()!=Some(id){return Err("invalid_request")}}let d=doc(u.as_str(),false)?;let images=gallery(&d)?;if images.is_empty(){return Err("chapter_unavailable")};let pages=images.into_iter().enumerate().map(|(i,u)|json!({"id":format!("{id}:{}",i+1),"index":i,"image":{"kind":"url","url":u}})).collect::<Vec<_>>();Ok(json!({"chapterId":id,"pages":pages}))}))
}
fn run_get_image_request(r: &[u8]) -> u32 {
    output(
        r,
        "get_image_request",
        port::input(r).and_then(|v| {
            let u = valid_url(v["args"]["url"].as_str().ok_or("invalid_request")?, true)?;
            Ok(json!({"url":u.as_str(),"headers":{"Referer":BASE}}))
        }),
    )
}
fn run_get_home(r: &[u8]) -> u32 {
    fail(r, "get_home", "unsupported")
}
fn chapters(r: &[u8]) -> ChapterResult<'static, &'static str> {
    let v = port::input(r)?;
    let (id, u) = work(&v)?;
    if !v["args"]["page"]["cursor"].is_null() {
        return Err("invalid_request");
    };
    let d = doc(u.as_str(), false)?;
    let urls = gallery(&d)?;
    if urls.is_empty() {
        return Err("chapter_unavailable");
    };
    let name = label(r, "chapter.gallery")?;
    let mut c = Chapter::new(id.as_bytes(), id.as_bytes(), name.as_bytes());
    c.language = Some(b"all");
    c.url = Some(u.as_str().as_bytes());
    c.page_count = Some(urls.len());
    let mut out = GroupedChapters::new(payload_buf(), r).map_err(port::chapter_error)?;
    out.push(
        c,
        ChapterPlacement::InGroup {
            group: b"gallery",
            position: 0,
        },
    )
    .map_err(port::chapter_error)?;
    out.finish(ChapterPage::Complete)
        .map_err(port::chapter_error)
}
#[no_mangle]
pub extern "C" fn koma_source_get_chapters(p: u32, n: u32) -> u32 {
    let r = match read_request(p, n) {
        Some(r) => r,
        None => return fail(&[], "get_chapters", "invalid_request"),
    };
    match chapters(r) {
        Ok(v) => v.write(response_buffer()),
        Err(c) => fail(r, "get_chapters", c),
    }
}
export_source!();

fn valid_work_path(s: &str) -> bool {
    let p = s.trim_matches('/');
    s.ends_with('/')
        && p.split('/').count() >= 2
        && !p.contains(['?', '#', '\\'])
        && !p.starts_with("category/")
        && !p.starts_with("tag/")
        && !p.starts_with("page/")
}
fn gallery(d: &Node) -> Result<Vec<String>> {
    if let Some(h) = d.one("article h1") {
        if video_title(&h.text()?) {
            return Ok(Vec::new());
        }
    }
    let mut rows = Vec::new();
    for a in d.all("a.msacwl-img-link")? {
        let s = a
            .attr("data-mfp-src")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| a.attr("href").ok())
            .ok_or("parse_error")?;
        let u = valid_url(&s, true).map_err(|_| "parse_error")?;
        let ext = u.path().to_ascii_lowercase();
        if ![".jpg", ".jpeg", ".png", ".webp", ".gif", ".avif"]
            .iter()
            .any(|e| ext.ends_with(e))
        {
            continue;
        }
        rows.push(String::from(u.as_str()));
    }
    Ok(rows)
}
fn video_title(t: &str) -> bool {
    t.trim().to_ascii_lowercase().ends_with("(video)")
}
fn detail(id: &str, d: &Node) -> Result<Value> {
    let name = title(d, "article h1")?;
    if video_title(&name) {
        return Err("chapter_unavailable");
    };
    let urls = gallery(d)?;
    if urls.is_empty() {
        return Err("chapter_unavailable");
    };
    let thumb = d
        .one("meta[property='og:image']")
        .map(|n| n.attr("content"))
        .transpose()?
        .unwrap_or_else(|| urls[0].clone());
    let tags = d
        .all("article span.cat-links a, article span.tag-links a")?
        .iter()
        .map(Node::text)
        .collect::<Result<Vec<_>>>()?;
    Ok(
        json!({"id":id,"title":name,"cover":image(&thumb)?,"authors":[],"tags":tags,"description":"","language":"all","status":"completed","contentRating":"nsfw","links":[{"kind":"source","url":format!("{BASE}{id}")}]}),
    )
}
fn browse(v: &Value, search: bool) -> Result<Value> {
    let n = page(v)?;
    let args = &v["args"];
    if !search && !matches!(args["listingId"].as_str(), None | Some("popular")) {
        return Err("unsupported");
    };
    let mut u = Url::parse(BASE).map_err(|_| "internal_error")?;
    let query = if search {
        args["query"].as_str().unwrap_or("").trim()
    } else {
        ""
    };
    if !query.is_empty() {
        u.set_path(&format!("/page/{n}"));
        u.query_pairs_mut().append_pair("s", query);
    } else {
        let f = &args["filters"];
        let category = f["category"].as_str().unwrap_or("");
        if !["", "ero-cosplay", "nude", "sexy-set"].contains(&category) {
            return Err("invalid_request");
        };
        let tag = f["tag"]
            .as_str()
            .unwrap_or("")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join("-")
            .to_lowercase();
        if !category.is_empty() {
            u.set_path(&format!("/category/{category}/page/{n}"));
        } else if !tag.is_empty() {
            u.path_segments_mut()
                .map_err(|_| "internal_error")?
                .extend(["tag", &tag, "page", &format!("{n}")]);
        } else {
            u.set_path(&format!("/category/ero-cosplay/page/{n}"));
        }
    }
    let d = doc(u.as_str(), false)?;
    let mut items = Vec::new();
    for card in d.all("div.article-container article")? {
        if host::check_cancel() {
            return Err("cancelled");
        };
        let a = card.require("a[href]")?;
        let url = valid_url(&a.attr("href")?, false).map_err(|_| "parse_error")?;
        if !valid_work_path(url.path()) {
            continue;
        };
        let name = a
            .attr("title")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| card.one("h1,h2,h3").and_then(|n| n.text().ok()))
            .ok_or("parse_error")?;
        if video_title(&name) {
            continue;
        };
        let album = doc(url.as_str(), false)?;
        if gallery(&album)?.is_empty() {
            continue;
        }
        let thumb = attr_img(&card.require("img")?)?;
        items.push(json!({"id":url.path(),"title":name,"cover":image(&thumb)?,"authors":[],"language":"all","status":"completed","contentRating":"nsfw"}));
    }
    Ok(port::items(
        items,
        d.one("div.wp-pagenavi a.page.larger")
            .is_some()
            .then_some(n + 1),
    ))
}
fn filters(r: &[u8]) -> Result<Value> {
    Ok(
        json!({"filters":[select_filter(r,"category","filter.category",&[("","filter.any"),("ero-cosplay","filter.ero_cosplay"),("nude","filter.nude"),("sexy-set","filter.sexy_set")],"")?,{"id":"tag","type":"text","name":label(r,"filter.tag")?}]}),
    )
}
