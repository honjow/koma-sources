extern crate alloc;
use alloc::{
    collections::BTreeSet,
    format,
    string::{String, ToString},
    vec::Vec,
};
use koma_source_sdk::{
    chapters::{Chapter, ChapterPage, ChapterPlacement, GroupedChapters},
    host,
    localization::Catalog,
    runtime::Context,
    source::{SourceCapabilities, SourceInfo},
};
use serde_json::{json, Value};
use url::Url;
#[allow(dead_code)]
mod port_support;
use port_support::{Node, Result};
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
    credentials: false,
    image_request: true,
};
koma_source_sdk::koma_source_buffers! {payload:1024*1024,http_out:10*1024*1024,body:6*1024*1024,http_req:16384,scratch:16384,}
koma_source_sdk::koma_source_helpers!();
fn label(r: &[u8], k: &str) -> Result<String> {
    let mut b = [0; 2048];
    Catalog::from_metadata(METADATA)
        .map_err(|_| "internal_error")?
        .text(&Context::from_request(r), k, k, &mut b)
        .map(String::from)
        .map_err(|_| "internal_error")
}
fn fail(r: &[u8], op: &str, c: &str) -> u32 {
    let s = label(r, &format!("error.{c}")).unwrap_or_else(|_| String::from("Unavailable"));
    write_error(op, c, &s)
}
fn out(r: &[u8], op: &str, v: Result<Value>) -> u32 {
    match v {
        Ok(v) => match serde_json::to_vec(&v) {
            Ok(b) if b.len() <= payload_buf().len() => {
                payload_buf()[..b.len()].copy_from_slice(&b);
                write_success_payload(op, b.len())
            }
            _ => fail(r, op, "result_too_large"),
        },
        Err(e) => fail(r, op, e),
    }
}
fn input(r: &[u8]) -> Result<Value> {
    port_support::input(r)
}
fn pg(v: &Value) -> Result<usize> {
    match &v["args"]["page"]["cursor"] {
        Value::Null => Ok(1),
        x => x
            .as_str()
            .and_then(|s| s.parse().ok())
            .filter(|n| *n > 0 && *n < 20000)
            .ok_or("invalid_request"),
    }
}
fn http(u: &str, body: Option<Value>) -> Result<String> {
    port_support::request(
        u,
        if body.is_some() { "POST" } else { "GET" },
        json!({"Accept":"text/html,application/json","Content-Type":"application/json","Referer":BASE,"User-Agent":"okhttp/4.12.0"}),
        body.as_ref().map(Value::to_string).as_deref(),
    )
}
fn jget(u: &str) -> Result<Value> {
    serde_json::from_str(&http(u, None)?).map_err(|_| "parse_error")
}
fn html(u: &str) -> Result<Node> {
    Node::parse(&http(u, None)?)
}
fn abs(u: &str) -> Result<String> {
    Ok(port_support::absolute(BASE, u)?.into())
}
fn args_id<'a>(v: &'a Value, k: &str) -> Result<&'a str> {
    v["args"][k]
        .as_str()
        .filter(|s| !s.is_empty() && s.len() < 4096)
        .ok_or("invalid_request")
}
fn text(v: &Value) -> String {
    v.as_str()
        .map(String::from)
        .unwrap_or_else(|| v.to_string())
}
fn clean(s: &str) -> String {
    Node::parse(s).and_then(|n| n.text()).unwrap_or_default()
}
fn item(id: &str, title: &str, cover: &str) -> Value {
    json!({"id":id,"title":title,"cover":{"kind":"url","url":cover,"headers":{"Referer":BASE,"User-Agent":"okhttp/4.12.0"}},"authors":[],"language":SOURCE_INFO.language,"status":"unknown","contentRating":"safe"})
}
fn list_out(xs: Vec<Value>, next: Option<usize>) -> Value {
    port_support::items(xs, next)
}
struct C {
    id: String,
    title: String,
    url: String,
    count: Option<usize>,
    order: usize,
}
fn run_get_listings(r: &[u8]) -> u32 {
    out(
        r,
        "get_listings",
        (move || {
            input(r)?;
            Ok(
                json!({"listings":[{"id":"popular","kind":"popular","name":label(r,"listing.popular")?,"supportsPaging":true,"supportsFilters":false},{"id":"latest","kind":"latest","name":label(r,"listing.latest")?,"supportsPaging":true,"supportsFilters":false}]}),
            )
        })(),
    )
}
fn run_get_manga_list(r: &[u8]) -> u32 {
    out(
        r,
        "get_manga_list",
        input(r).and_then(|v| browse(&v, false)),
    )
}
fn run_search(r: &[u8]) -> u32 {
    out(r, "search", input(r).and_then(|v| browse(&v, true)))
}
fn run_get_filters(r: &[u8]) -> u32 {
    out(r, "get_filters", input(r).and_then(|_| filters(r)))
}
fn run_get_settings(r: &[u8]) -> u32 {
    out(r, "get_settings", input(r).map(|_| json!({"settings":[]})))
}
fn run_get_manga(r: &[u8]) -> u32 {
    out(
        r,
        "get_manga",
        input(r)
            .and_then(|v| details(&v))
            .map(|m| json!({"manga":m})),
    )
}
fn run_get_pages(r: &[u8]) -> u32 {
    out(r,"get_pages",input(r).and_then(|v|{let id=args_id(&v,"chapterId")?;let urls=reader(&v)?;if urls.is_empty(){return Err("chapter_unavailable")};Ok(json!({"chapterId":id,"pages":urls.into_iter().enumerate().map(|(i,u)|json!({"id":format!("{id}:{}",i+1),"index":i,"image":{"kind":"url","url":u}})).collect::<Vec<_>>()}))}))
}
fn run_get_image_request(r: &[u8]) -> u32 {
    out(
        r,
        "get_image_request",
        input(r).and_then(|v| {
            let u = image_request(args_id(&v, "url")?)?;
            Ok(json!({"url":u,"headers":{"Referer":BASE,"User-Agent":"okhttp/4.12.0"}}))
        }),
    )
}
fn run_get_home(r: &[u8]) -> u32 {
    fail(r, "get_home", "unsupported")
}
#[no_mangle]
pub extern "C" fn koma_source_get_chapters(p: u32, n: u32) -> u32 {
    let r = match read_request(p, n) {
        Some(r) => r,
        None => return fail(&[], "get_chapters", "invalid_request"),
    };
    let result = (move || {
        let v = input(r)?;
        let mid = args_id(&v, "mangaId")?;
        let (cs, next) = chapters(&v)?;
        let mut out =
            GroupedChapters::new(payload_buf(), r).map_err(port_support::chapter_error)?;
        for c in cs {
            let mut x = Chapter::new(c.id.as_bytes(), mid.as_bytes(), c.title.as_bytes());
            x.language = Some(LANG_BYTES);
            x.url = Some(c.url.as_bytes());
            x.page_count = c.count;
            out.push(
                x,
                ChapterPlacement::InGroup {
                    group: b"main",
                    position: c.order as u64,
                },
            )
            .map_err(port_support::chapter_error)?;
        }
        let cursor = next.map(|n| n.to_string());
        out.finish(cursor.as_ref().map_or(ChapterPage::Complete, |s| {
            ChapterPage::NextCursor(s.as_bytes())
        }))
        .map_err(port_support::chapter_error)
    })();
    match result {
        Ok(x) => x.write(response_buffer()),
        Err(e) => fail(r, "get_chapters", e),
    }
}
fn select(r: &[u8], id: &str, key: &str, vals: &[(&str, &str)], default: &str) -> Result<Value> {
    Ok(
        json!({"id":id,"name":label(r,key)?,"type":"select","default":default,"options":vals.iter().map(|(v,k)|Ok(json!({"value":v,"label":label(r,k)?}))).collect::<Result<Vec<_>>>()?}),
    )
}
exports!();
