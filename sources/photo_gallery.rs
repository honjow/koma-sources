//! Shared Koma representation of a public photo post as one complete gallery.
use alloc::{
    collections::BTreeSet,
    format,
    string::{String, ToString},
    vec::Vec,
};
use koma_source_sdk::{
    chapters::{Chapter, ChapterNumber, ChapterResponse, ChapterResult},
    host,
    source::{SourceCapabilities, SourceInfo},
    source_protocol,
};
use serde_json::{json, Value};
use url::Url;

use crate::{
    port::{self, Node, Result},
    port_catalog::{self, Labels},
    LANGUAGE, METADATA,
};

pub(crate) const CAPS: SourceCapabilities = SourceCapabilities {
    search: true,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: true,
    manga_list: true,
    filters: true,
    settings: false,
    home: false,
    credentials: false,
    image_request: false,
};

pub(crate) const fn info(
    id: &'static str,
    name: &'static str,
    description: &'static str,
) -> SourceInfo {
    SourceInfo {
        id,
        name,
        language: LANGUAGE,
        version: env!("CARGO_PKG_VERSION"),
        api_version: "0.2",
        description,
        author: "Koma source contributors",
        content_rating: "nsfw",
    }
}

pub(crate) struct Http {
    pub(crate) status: u64,
    pub(crate) headers: Value,
    pub(crate) body: String,
}

pub(crate) fn fetch(url: &Url, accept: &str) -> Result<Http> {
    fetch_with_headers(url, accept, false)
}

pub(crate) fn fetch_with_headers(url: &Url, accept: &str, include_headers: bool) -> Result<Http> {
    if host::check_cancel() {
        return Err("cancelled");
    }
    let mut request = json!({
        "version":1, "method":"GET", "url":url.as_str(),
        "headers":{"Accept":accept},
        "timeoutMs":10000, "responseKind":"bodyText"
    });
    if include_headers {
        request["responseMetadata"] = json!({"headers":true});
    }
    let bytes = serde_json::to_vec(&request).map_err(|_| "internal_error")?;
    let len = host::http_request(&bytes, crate::http_out()).map_err(|_| "network_error")?;
    let raw = &crate::http_out()[..len];
    let member = |key: &[u8]| source_protocol::direct(raw, key).map_err(|_| "network_error");
    let status = member(b"status")?
        .and_then(|v| serde_json::from_slice(v).ok())
        .unwrap_or(0);
    if status == 400 {
        return Ok(Http {
            status,
            headers: Value::Null,
            body: member(b"bodyText")?
                .and_then(|v| serde_json::from_slice(v).ok())
                .unwrap_or_default(),
        });
    }
    let len = source_protocol::body(raw, crate::body_buf())?;
    Ok(Http {
        status,
        headers: if include_headers {
            member(b"headers")?
                .and_then(|v| serde_json::from_slice(v).ok())
                .unwrap_or(Value::Null)
        } else {
            Value::Null
        },
        body: String::from_utf8(crate::body_buf()[..len].to_vec()).map_err(|_| "parse_error")?,
    })
}

pub(crate) fn document(url: &Url) -> Result<Node> {
    let response = fetch(url, "text/html")?;
    if response.status == 400 {
        return Err("invalid_request");
    }
    Node::parse(&response.body)
}

pub(crate) fn https(base: &str, raw: &str) -> Result<Url> {
    let mut url = port::absolute(base, raw)?;
    if url.port().is_some() || url.set_scheme("https").is_err() {
        return Err("parse_error");
    }
    url.set_fragment(None);
    Ok(url)
}

pub(crate) fn work_id(key: &str) -> String {
    format!("manga:{key}")
}

fn chapter_id(key: &str) -> String {
    format!("chapter:{key}")
}

pub(crate) fn work(args: &Value) -> Result<&str> {
    args["mangaId"]
        .as_str()
        .and_then(|id| id.strip_prefix("manga:"))
        .filter(|s| !s.is_empty())
        .ok_or("invalid_request")
}

pub(crate) fn chapter(args: &Value) -> Result<&str> {
    args["chapterId"]
        .as_str()
        .and_then(|id| id.strip_prefix("chapter:"))
        .filter(|s| !s.is_empty())
        .ok_or("invalid_request")
}

pub(crate) struct Gallery {
    pub(crate) key: String,
    pub(crate) url: Url,
    pub(crate) title: String,
    pub(crate) cover: Option<Url>,
    pub(crate) tags: Vec<String>,
    pub(crate) published: Option<String>,
}

impl Gallery {
    pub(crate) fn manga(&self) -> Value {
        let cover = self.cover.as_ref().map_or_else(
            || json!({"kind":"none"}),
            |url| json!({"kind":"url","url":url.as_str(),"headers":{}}),
        );
        json!({
            "id":work_id(&self.key), "url":self.url.as_str(), "title":self.title,
            "language":LANGUAGE, "cover":cover, "tags":self.tags,
            "status":"completed", "contentRating":"nsfw"
        })
    }
}

pub(crate) fn pages(key: &str, urls: Vec<Url>) -> Result<Value> {
    if urls.is_empty() {
        return Err("chapter_unavailable");
    }
    let id = chapter_id(key);
    let pages: Vec<_> = urls
        .into_iter()
        .enumerate()
        .map(|(i, url)| port::page(i, &id, url))
        .collect();
    Ok(json!({"pages":pages}))
}

pub(crate) fn unique(urls: Vec<Url>) -> Vec<Url> {
    let mut seen = BTreeSet::new();
    urls.into_iter()
        .filter(|url| seen.insert(url.as_str().to_string()))
        .collect()
}

pub(crate) fn lazy_image(
    node: &Node,
    attributes: &[&str],
    mut resolve: impl FnMut(&str) -> Option<Url>,
) -> Option<Url> {
    attributes
        .iter()
        .filter_map(|attr| node.attr(attr).ok())
        .find_map(|candidate| resolve(candidate.trim()))
}

pub(crate) fn page_number(args: &Value) -> Result<usize> {
    match &args["page"]["cursor"] {
        Value::Null => Ok(1),
        Value::String(raw) => raw
            .parse::<usize>()
            .ok()
            .filter(|&n| n > 0 && n <= 100_000 && n.to_string() == *raw)
            .ok_or("invalid_request"),
        _ => Err("invalid_request"),
    }
}

pub(crate) fn visible_name(labels: &Labels<'_>, prefix: &str, id: &str) -> Result<String> {
    labels.text(&format!("{prefix}.{id}"), id)
}

pub(crate) fn option(value: &str, label: &str) -> Value {
    json!({"value":value,"label":label})
}

pub(crate) fn select_filter(
    labels: &Labels<'_>,
    id: &str,
    options: Vec<Value>,
    description: bool,
) -> Result<Value> {
    let mut filter = json!({
        "type":"select", "id":id, "name":visible_name(labels,"filter",id)?,
        "default":"", "options":options
    });
    if description {
        filter["description"] = json!(visible_name(labels, "filter", &format!("{id}_hint"))?);
    }
    Ok(filter)
}

pub(crate) fn selection<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    match &args["filters"][key] {
        Value::Null => Ok(""),
        Value::String(s) => Ok(s),
        _ => Err("invalid_request"),
    }
}

pub(crate) fn labels(request: &[u8]) -> Result<Labels<'_>> {
    Labels::new(METADATA, request)
}

fn result(op: &str, req: &[u8], action: impl FnOnce(&Value) -> Result<Value>) -> u32 {
    let value = port::input(req).and_then(|input| action(&input["args"]));
    port_catalog::write_localized(METADATA, req, op, value)
}

pub(crate) fn run_search(req: &[u8]) -> u32 {
    result("search", req, |args| super::browse(args, true, req))
}

pub(crate) fn run_get_manga_list(req: &[u8]) -> u32 {
    result("get_manga_list", req, |args| {
        super::browse(args, false, req)
    })
}

pub(crate) fn run_get_manga(req: &[u8]) -> u32 {
    result("get_manga", req, |args| {
        let gallery = super::detail(work(args)?)?;
        Ok(json!({"manga":gallery.manga()}))
    })
}

pub(crate) fn run_get_pages(req: &[u8]) -> u32 {
    result("get_pages", req, |args| super::get_pages(chapter(args)?))
}

pub(crate) fn run_get_chapters(req: &[u8]) -> ChapterResult<'static, (&'static str, String)> {
    let single = || {
        let input = port::input(req)?;
        if !input["args"]["page"]["cursor"].is_null() {
            return Err("invalid_request");
        }
        let key = work(&input["args"])?;
        let gallery = super::detail(key)?;
        let id = chapter_id(&gallery.key);
        let manga = work_id(&gallery.key);
        let title = labels(req)?.text("chapter.gallery", "Gallery")?;
        let mut chapter = Chapter::new(id.as_bytes(), manga.as_bytes(), title.as_bytes());
        chapter.url = Some(gallery.url.as_str().as_bytes());
        chapter.chapter_number = Some(ChapterNumber::Integer(1));
        chapter.language = Some(LANGUAGE.as_bytes());
        chapter.published_at = gallery.published.as_deref().map(str::as_bytes);
        ChapterResponse::single(crate::payload_buf(), chapter).map_err(port::chapter_error)
    };
    single().map_err(|code| (code, port_catalog::failure(METADATA, req, code)))
}

pub(crate) fn write_chapter_failure((code, message): (&str, String)) -> u32 {
    crate::write_error("get_chapters", code, &message)
}

pub(crate) fn run_get_listings(req: &[u8]) -> u32 {
    result("get_listings", req, |_| {
        let labels = labels(req)?;
        let listings = super::LISTINGS
            .iter()
            .map(|(id, paging, filters)| {
                Ok(json!({
                    "id":id, "name":visible_name(&labels,"listing",id)?,
                    "supportsPaging":paging, "supportsFilters":filters
                }))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(json!({"listings":listings}))
    })
}

pub(crate) fn run_get_filters(req: &[u8]) -> u32 {
    result("get_filters", req, |args| {
        super::filters(args, req, &labels(req)?)
    })
}

pub(crate) fn run_get_settings(req: &[u8]) -> u32 {
    result("get_settings", req, |_| Ok(json!({"settings":[]})))
}

pub(crate) fn run_get_home(req: &[u8]) -> u32 {
    result("get_home", req, |_| Err("not_implemented"))
}

pub(crate) fn run_get_image_request(req: &[u8]) -> u32 {
    result("get_image_request", req, |_| Err("not_implemented"))
}
