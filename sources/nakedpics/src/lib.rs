#![no_std]
const SOURCE_INFO: SourceInfo = SourceInfo {
 id: "com.nakedpics.koma", name: "NakedPics", language: "all", version: env!("CARGO_PKG_VERSION"), api_version: "0.2", author: "Koma source contributors", description: "NakedPics connects photo galleries through model, studio and theme directories, including All Over 30’s Isadora and Femjoy’s Laria F. Each gallery retains all original photos without recommendation thumbnails or videos.", content_rating: "nsfw",
};
const BASE: &str = "https://hotnakedwomen.com";
const IMAGE_HOST: &str = "cdni.hotnakedwomen.com";
const METADATA: &[u8] = include_bytes!("../metadata.json");
const HAS_LATEST: bool = true;
const POPULAR_KEY: &str = "listing.popular";
macro_rules! export_source { () => { koma_source_sdk::koma_source_exports!(@common "nakedpics"); }; }
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
    if image && !image_extension(u.path()) {
        return Err("invalid_request");
    }
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
            detail(r, id, &d).map(|m| json!({"manga":m}))
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

// This source owns its protocol. It deliberately does not change Masonry behavior.
const PHOTO_SELECTOR: &str = "[itemprop=associatedMedia] > a[data-pswp-width]";
const CARD_SELECTOR: &str = "[itemprop=associatedMedia] > a[href*='/gals/']";
fn image_extension(path: &str) -> bool {
    let p = path.to_ascii_lowercase();
    [".jpg", ".jpeg", ".png", ".webp"]
        .iter()
        .any(|s| p.ends_with(s))
}
fn valid_work_path(path: &str) -> bool {
    let parts = path.trim_matches('/').split('/').collect::<Vec<_>>();
    path.ends_with('/')
        && parts.len() == 2
        && parts[0] == "gals"
        && !parts[1].is_empty()
        && !parts[1].contains(['?', '#', '\\'])
}
fn gallery(d: &Node) -> Result<Vec<String>> {
    let mut rows = Vec::new();
    for a in d.all(PHOTO_SELECTOR)? {
        let u = valid_url(&a.attr("href")?, true).map_err(|_| "parse_error")?;
        rows.push(String::from(u.as_str()));
    }
    Ok(rows)
}
fn detail(r: &[u8], id: &str, d: &Node) -> Result<Value> {
    let photos = gallery(d)?;
    if photos.is_empty() {
        return Err("chapter_unavailable");
    }
    let name = title(d, "h1[itemprop=name]")?;
    let first = d.require(PHOTO_SELECTOR)?;
    let cover = first.require("img")?.attr("itemprop")?;
    let mut models = Vec::new();
    let mut tags = Vec::new();
    let mut channel = String::new();
    for a in d.all("[itemtype$=ImageGallery] li > a")? {
        if a.one("i.icon-mod").is_some() {
            models.push(a.text()?);
        }
        if a.one("i.icon-tag2-1").is_some() {
            tags.push(a.text()?);
        }
        if channel.is_empty() && a.one("i.icon-cat2").is_some() {
            channel = a.text()?;
        }
    }
    let mut description = format!("{}: {}", label(r, "detail.photos")?, photos.len());
    if !models.is_empty() {
        description.push_str(&format!(
            "\n{}: {}",
            label(r, "detail.models")?,
            models.join(", ")
        ));
    }
    if !channel.is_empty() {
        description.push_str(&format!("\n{}: {channel}", label(r, "detail.channel")?));
    }
    Ok(
        json!({"id":id,"title":name,"cover":image(&cover)?,"authors":models,"tags":tags,"description":description,"language":"all","status":"completed","contentRating":"nsfw","links":[{"kind":"source","url":format!("{BASE}{id}")}]}),
    )
}
// Inspect finalUrl only after the SDK has accepted HTTP/transport completion.
// No response-header values are read; String/Array serialization cannot affect this code.
fn fetch_listing(url: &Url, xhr: bool) -> Result<(String, Url)> {
    if host::check_cancel() {
        return Err("cancelled");
    }
    let mut headers = json!({"Accept":"text/html,application/json","Referer":BASE});
    if xhr {
        headers["X-Requested-With"] = json!("XMLHttpRequest");
    }
    let req = serde_json::to_vec(&json!({"version":1,"method":"GET","url":url.as_str(),"headers":headers,"responseKind":"bodyText","timeoutMs":15000})).map_err(|_| "internal_error")?;
    let len = host::http_request(&req, http_out()).map_err(|_| "network_error")?;
    let body_len = koma_source_sdk::source_protocol::body(&http_out()[..len], body_buf())?;
    let envelope: Value =
        serde_json::from_slice(&http_out()[..len]).map_err(|_| "network_error")?;
    let mut final_url = envelope["finalUrl"]
        .as_str()
        .map(|u| valid_url(u, false))
        .transpose()?
        .unwrap_or_else(|| url.clone());
    // The base category index is a legal listing document; valid_url intentionally
    // disallows root URLs for public work/image operations only.
    if final_url.host_str() != Some("hotnakedwomen.com") {
        return Err("parse_error");
    }
    final_url.set_query(None);
    Ok((
        String::from_utf8(body_buf()[..body_len].to_vec()).map_err(|_| "parse_error")?,
        final_url,
    ))
}
fn cid(d: &Node) -> Result<String> {
    for script in d.all("script")? {
        let text = script.text()?;
        for (i, _) in text.match_indices("CID") {
            if i > 0
                && text
                    .as_bytes()
                    .get(i - 1)
                    .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
            {
                continue;
            }
            let Some(tail) = text[i + 3..].trim_start().strip_prefix('=') else {
                continue;
            };
            let tail = tail.trim_start();
            let Some(q) = tail.chars().next().filter(|q| *q == '\'' || *q == '"') else {
                continue;
            };
            if let Some(end) = tail[1..].find(q) {
                let value = &tail[1..end + 1];
                if value.len() <= 1024 {
                    return Ok(value.replace('+', " "));
                }
            }
        }
    }
    Err("parse_error")
}
fn card(id: &str, name: &str, cover: &str) -> Result<Value> {
    work_url(id)?;
    if name.trim().is_empty() {
        return Err("parse_error");
    }
    Ok(
        json!({"id":id,"title":name,"cover":image(cover)?,"authors":[],"language":"all","status":"completed","contentRating":"nsfw"}),
    )
}
fn html_cards(d: &Node) -> Result<(Vec<Value>, usize)> {
    let cards = d.all(CARD_SELECTOR)?;
    let raw_count = cards.len();
    let mut items = Vec::new();
    let mut seen = BTreeSet::new();
    for a in cards {
        let url = valid_url(&a.attr("href")?, false)?;
        let id = url.path();
        if !valid_work_path(id) || !seen.insert(String::from(id)) {
            continue;
        }
        let im = a.require("img")?;
        let alt = im.attr("alt")?;
        let tid = a.attr("data-tid").unwrap_or_default();
        let suffix = format!(" {}", tid.split('_').next().unwrap_or(""));
        let name = alt.strip_suffix(&suffix).unwrap_or(&alt);
        let cover = im
            .attr("data-src")
            .ok()
            .filter(|v| !v.is_empty())
            .or_else(|| {
                im.attr("itemprop")
                    .ok()
                    .filter(|v| v.starts_with("https://"))
            })
            .unwrap_or(im.attr("src")?);
        if let Ok(item) = card(id, name, &cover) {
            items.push(item);
        }
    }
    Ok((items, raw_count))
}
fn json_cards(raw: &str) -> Result<(Vec<Value>, usize)> {
    let rows: Value = serde_json::from_str(raw).map_err(|_| "parse_error")?;
    let rows = rows.as_array().ok_or("parse_error")?;
    let mut items = Vec::new();
    let mut seen = BTreeSet::new();
    for row in rows {
        let u = match row["g_url"].as_str().and_then(|v| valid_url(v, false).ok()) {
            Some(u) => u,
            None => continue,
        };
        if !valid_work_path(u.path()) || !seen.insert(String::from(u.path())) {
            continue;
        }
        let alt = row["desc"].as_str().ok_or("parse_error")?;
        let suffix = format!(" {}", row["gid"].as_str().ok_or("parse_error")?);
        let name = alt.strip_suffix(&suffix).unwrap_or(alt);
        if let Ok(item) = card(
            u.path(),
            name,
            row["t_url_460"].as_str().ok_or("parse_error")?,
        ) {
            items.push(item);
        }
    }
    Ok((items, rows.len()))
}
fn route(parts: &[&str]) -> Result<Url> {
    let mut u = Url::parse(BASE).map_err(|_| "internal_error")?;
    u.path_segments_mut()
        .map_err(|_| "internal_error")?
        .extend(parts.iter().copied())
        .push("");
    Ok(u)
}
fn browse(v: &Value, search: bool) -> Result<Value> {
    let n = page(v)?;
    let args = &v["args"];
    let mut newest = false;
    let seed;
    let query = if search {
        args["query"].as_str().unwrap_or("").trim()
    } else {
        ""
    };
    if search {
        let order = args["filters"]["order"].as_str().unwrap_or("popular");
        if !["popular", "newest"].contains(&order) {
            return Err("invalid_request");
        }
        newest = order == "newest";
        let category = args["filters"]["category"].as_str().unwrap_or("");
        if !query.is_empty() {
            seed = route(&[
                "search",
                &query.split_whitespace().collect::<Vec<_>>().join("-"),
            ])?;
        } else if !category.is_empty() {
            if category.len() > 128
                || !category
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'-')
            {
                return Err("invalid_request");
            }
            seed = if newest {
                route(&[category, "newest"])?
            } else {
                route(&[category])?
            };
            newest = false;
        } else {
            seed = route(&[if newest { "recent" } else { "popular" }])?;
            newest = false;
        }
    } else {
        let part = match args["listingId"].as_str() {
            None | Some("popular") => "popular",
            Some("latest") => "recent",
            _ => return Err("invalid_request"),
        };
        seed = route(&[part])?;
    }
    let (html, mut resolved) = match fetch_listing(&seed, false) {
        Err("not_found") if !query.is_empty() => return Ok(port::items(Vec::new(), None)),
        r => r?,
    };
    let d = Node::parse(&html)?;
    if n == 1 && !newest {
        let (items, raw_count) = html_cards(&d)?;
        return Ok(port::items(items, (raw_count >= 30).then_some(n + 1)));
    }
    if newest {
        resolved
            .path_segments_mut()
            .map_err(|_| "parse_error")?
            .pop_if_empty()
            .push("newest")
            .push("");
    }
    resolved
        .query_pairs_mut()
        .append_pair("cat", &cid(&d)?)
        .append_pair("limit", "30")
        .append_pair("offset", &format!("{}", (n - 1) * 30))
        .append_pair("lang", "en");
    let (raw, _) = fetch_listing(&resolved, true)?;
    let (items, raw_count) = json_cards(&raw)?;
    Ok(port::items(items, (raw_count >= 30).then_some(n + 1)))
}
fn filters(r: &[u8]) -> Result<Value> {
    let mut rows = alloc::vec![select_filter(
        r,
        "order",
        "filter.order",
        &[("popular", "filter.popular"), ("newest", "filter.newest")],
        "popular"
    )?];
    let html = port::get(BASE)?;
    let d = Node::parse(&html)?;
    let mut options = alloc::vec![json!({"value":"","label":label(r,"filter.any_category")?})];
    let mut seen = BTreeSet::new();
    for a in d.all("[itemprop=associatedMedia] > a")? {
        let Some(span) = a.one("span") else {
            continue;
        };
        let u = valid_url(&a.attr("href")?, false)?;
        let path = u.path().trim_matches('/');
        if path.is_empty() || path.contains('/') || !seen.insert(String::from(path)) {
            continue;
        }
        options.push(json!({"value":path,"label":span.text()?}));
    }
    rows.push(json!({"id":"category","type":"select","name":label(r,"filter.category")?,"default":"","options":options}));
    Ok(json!({"filters":rows}))
}
