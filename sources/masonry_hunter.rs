// Opt-in protocol for Metart Hunter and Femjoy Hunter only. Existing sources remain independent.
// Keiyoushi 8d597e1bdbb0d9d961d941128703eaeb4bc8d1e3, Apache-2.0.

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
#[path = "port_support.rs"]
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
    s.ends_with('/') && !p.is_empty() && !p.contains('/') && !p.contains(['?', '#', '\\'])
}
fn gallery(d: &Node) -> Result<Vec<String>> {
    let mut rows = Vec::new();
    for a in d.all(".list-gallery a[href^='https://cdn.']")? {
        let u = valid_url(&a.attr("href")?, true).map_err(|_| "parse_error")?;
        rows.push(String::from(u.as_str()));
    }
    Ok(rows)
}
fn detail(id: &str, d: &Node) -> Result<Value> {
    let urls = gallery(d)?;
    if urls.is_empty() {
        return Err("chapter_unavailable");
    };
    let name = title(d, "header h1")?;
    let thumb = d
        .one(".list-gallery a[href^='https://cdn.'] img")
        .map(|n| attr_img(&n))
        .transpose()?
        .unwrap_or_else(|| urls[0].clone());
    let authors = d
        .all("p.link-btn a[href*='/model/']")?
        .iter()
        .map(Node::text)
        .collect::<Result<Vec<_>>>()?;
    let tags = d
        .all("p.link-btn a[href*='/tag/']")?
        .iter()
        .map(Node::text)
        .collect::<Result<Vec<_>>>()?;
    let description = d
        .one("#content > p")
        .map(|n| n.text())
        .transpose()?
        .unwrap_or_default();
    Ok(
        json!({"id":id,"title":name,"cover":image(&thumb)?,"authors":authors,"tags":tags,"description":description,"language":"all","status":"completed","contentRating":"nsfw","links":[{"kind":"source","url":format!("{BASE}{id}")}]}),
    )
}
fn opt<'a>(v: &'a Value, key: &str, choices: &[&str], default: &'a str) -> Result<&'a str> {
    match &v[key] {
        Value::Null => Ok(default),
        x => x
            .as_str()
            .filter(|s| choices.contains(s))
            .ok_or("invalid_request"),
    }
}
fn browse(v: &Value, search: bool) -> Result<Value> {
    let n = page(v)?;
    let args = &v["args"];
    let query = if search {
        args["query"].as_str().unwrap_or("").trim()
    } else {
        ""
    };
    let mut u = Url::parse(BASE).map_err(|_| "internal_error")?;
    let is_text = !query.is_empty();
    if is_text {
        u.path_segments_mut()
            .map_err(|_| "internal_error")?
            .extend(["search", "post", query, "mpage", &format!("{n}"), ""]);
    } else {
        u.set_path("/gridapi/");
        let content;
        let sort;
        if search {
            let f = &args["filters"];
            let quality = opt(f, "quality", &["0", "hd"], "0")?;
            let order = opt(f, "order", &["0", "popular", "newest"], "0")?;
            let mut tags = Vec::new();
            if let Some(x) = f.get("tags").filter(|x| !x.is_null()) {
                for t in x.as_array().ok_or("invalid_request")? {
                    let t = t
                        .as_str()
                        .filter(|s| {
                            !s.is_empty()
                                && s.len() <= 128
                                && s.bytes()
                                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
                        })
                        .ok_or("invalid_request")?;
                    tags.push(t)
                }
            }
            let label = if tags.is_empty() {
                String::from("0")
            } else {
                tags.join("+")
            };
            content=format!("{{\"content\":\"pix\",\"quality\":\"{quality}\",\"label\":\"{label}\",\"ord\":\"{order}\"}}");
            sort = "filter";
        } else {
            content = String::from("updates");
            sort = match args["listingId"].as_str() {
                None | Some("popular") => "trending",
                Some("latest") => "newest",
                _ => return Err("invalid_request"),
            };
        }
        u.query_pairs_mut()
            .append_pair("content", &content)
            .append_pair("sort", sort)
            .append_pair("mpage", &format!("{n}"));
    }
    let d = doc(u.as_str(), !is_text)?;
    let figures = d.all(if is_text {
        ".list-gallery:not(.static) figure"
    } else {
        "figure"
    })?;
    let raw_count = figures.len();
    let mut items = Vec::new();
    let mut seen = BTreeSet::new();
    for f in figures {
        if host::check_cancel() {
            return Err("cancelled");
        };
        if f.one(".icon-play").is_some() {
            continue;
        }
        let a = f.require("a[href]")?;
        let u = valid_url(&a.attr("href")?, false).map_err(|_| "parse_error")?;
        let id = u.path();
        if !valid_work_path(id) || !seen.insert(String::from(id)) {
            continue;
        }
        let mut name = a.attr("title").unwrap_or_default();
        if name.is_empty() {
            let t = f
                .all("a[href*='/model/']")?
                .iter()
                .map(Node::text)
                .collect::<Result<Vec<_>>>()?;
            name = t.join(", ");
        }
        if name.is_empty() {
            continue;
        }
        let cover = image(&attr_img(&f.require("img")?)?)?;
        items.push(json!({"id":id,"title":name,"cover":cover,"authors":[],"language":"all","status":"completed","contentRating":"nsfw"}));
    }
    let next = if is_text {
        d.one(".pagination-a li.next").is_some()
    } else {
        raw_count >= 20
    };
    Ok(port::items(items, next.then_some(n + 1)))
}
fn filters(r: &[u8]) -> Result<Value> {
    let mut rows = alloc::vec![
        select_filter(
            r,
            "order",
            "filter.order",
            &[
                ("0", "filter.any_order"),
                ("popular", "filter.popular"),
                ("newest", "filter.newest")
            ],
            "0"
        )?,
        select_filter(
            r,
            "quality",
            "filter.quality",
            &[("0", "filter.all_quality"), ("hd", "filter.hd")],
            "0"
        )?
    ];
    let d = doc(&format!("{BASE}/sort/newest/"), false)?;
    let mut options = Vec::new();
    for n in d.all("#filter-a section[data-slot=tags] label")? {
        let name = n.require(".txt")?.text()?;
        let value = n.require("input")?.attr("value")?;
        if !value.is_empty() {
            options.push(json!({"value":value,"label":name}))
        }
    }
    if !options.is_empty() {
        rows.push(json!({"id":"tags","type":"multiselect","name":label(r,"filter.tags")?,"default":[],"options":options}))
    }
    Ok(json!({"filters":rows}))
}
