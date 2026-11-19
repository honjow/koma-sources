#![no_std]
extern crate alloc;

use alloc::{
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec::Vec,
};
use base64::{
    alphabet,
    engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig},
    Engine,
};
use koma_source_sdk::{
    chapters::{Chapter, ChapterOrder, ChapterResponse, ChapterResult},
    json_utils::request_page_number,
    source::{SourceCapabilities, SourceInfo},
    source_protocol,
};
use md5::{Digest, Md5};
use percent_encoding::percent_decode_str;
use serde_json::{json, Value};
use url::{Host, Url};

#[path = "../../port_support.rs"]
mod port;
use port::{Node, Result};

const BASE: &str = "https://rouman5.com";
const READER_FALLBACK: &str = "https://roum29.xyz";
const STATUS: &[(&str, &str)] = &[("all", "全部"), ("true", "連載中"), ("false", "已完結")];

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.roumanwu.koma",
    name: "肉漫屋",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "zh-Hant",
    author: "Koma",
    description: "肉漫屋的公開目錄與原站可讀章節。",
    content_rating: "nsfw",
};
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    listings: true,
    manga_list: true,
    filters: true,
    settings: true,
    image_request: true,
    ..SourceCapabilities::CORE
};

#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;
koma_source_sdk::koma_source_buffers! {payload:1024*1024,http_out:6*1024*1024,body:5*1024*1024,http_req:4096,scratch:8192,}
koma_source_sdk::koma_source_helpers!();

fn base(input: &Value) -> Result<String> {
    let value = input["settings"]["baseUrl"]
        .as_str()
        .unwrap_or_default()
        .trim();
    if value.is_empty() {
        return Ok(BASE.to_string());
    }
    // Base URL is a user-selected public site origin. It grants no cookie access.
    let url = Url::parse(value).map_err(|_| "invalid_request")?;
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
        || !matches!(url.host(), Some(Host::Domain(host)) if host.contains('.') && !host.ends_with(".local") && !host.ends_with(".localhost"))
    {
        return Err("invalid_request");
    }
    Ok(url.origin().ascii_serialization())
}

fn document(url: &Url) -> Result<String> {
    port::request(
        url.as_str(),
        "GET",
        json!({"Accept":"text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8","Referer":format!("{}/",url.origin().ascii_serialization())}),
        None,
    )
}

fn book_path(id: &str, chapter: bool) -> Result<&str> {
    let path = id.strip_prefix("/books/").ok_or("invalid_request")?;
    let valid_piece = |piece: &str| {
        !piece.is_empty()
            && piece.len() <= 128
            && piece
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
            && !matches!(piece, "." | "..")
    };
    let valid = match path.split_once('/') {
        Some((book, chapter_id)) => chapter && valid_piece(book) && valid_piece(chapter_id),
        None => !chapter && valid_piece(path),
    };
    valid.then_some(id).ok_or("invalid_request")
}

fn local_path(base: &str, raw: &str, chapter: bool) -> Result<String> {
    let url = port::absolute(base, raw)?;
    let origin = port::absolute(base, "/")?.origin();
    if url.origin() != origin || url.query().is_some() || url.fragment().is_some() {
        return Err("parse_error");
    }
    Ok(book_path(url.path(), chapter)?.to_string())
}

fn card(base: &str, id: &str, title: &str, cover: &str) -> Result<Value> {
    let cover = port::absolute(base, cover)?;
    Ok(json!({"id":id,"title":title,"language":"zh-Hant",
        "cover":{"kind":"url","url":cover.as_str(),"headers":{"Referer":format!("{base}/")}},
        "links":[{"kind":"source","url":format!("{base}{id}")}]
    }))
}

fn entries(base: &str, node: &Node) -> Result<Vec<Value>> {
    node.all("a.site-comic")?
        .iter()
        .map(|link| {
            let id = local_path(base, &link.attr("href")?, false)?;
            let title = link.require("h3")?.text()?;
            let cover = link.require("img")?.attr("src")?;
            card(base, &id, &title, &cover)
        })
        .collect()
}

fn list(input: &Value, request: &[u8], search: bool) -> Result<Value> {
    let base = base(input)?;
    let page = request_page_number(request).max(1);
    let args = &input["args"];
    let query = args["query"].as_str().unwrap_or_default();
    if !search && args["listingId"] == "latest" {
        let html = document(&port::absolute(&base, "/home")?)?;
        let root = Node::parse(&html)?;
        let mut items = Vec::<Value>::new();
        for section in root.all("div.site-home > *")? {
            if section
                .one(".site-section-heading")
                .map(|heading| heading.text())
                .transpose()?
                .is_some_and(|heading| heading.contains("最近更新"))
            {
                for card in entries(&base, &section)? {
                    if !items.iter().any(|seen| seen["id"] == card["id"]) {
                        items.push(card);
                    }
                }
            }
        }
        root.require("div.site-home")?;
        return Ok(port::items(items, None));
    }
    let text_search = search && !query.trim().is_empty();
    let mut url = port::absolute(&base, if text_search { "/search" } else { "/books" })?;
    {
        let mut params = url.query_pairs_mut();
        if text_search {
            params.append_pair("term", query);
        }
        params.append_pair("page", &(page - 1).to_string());
        if !text_search {
            let status = match args["filters"]["status"].as_str() {
                Some("") => "",
                _ => port::filter(args, "status", STATUS)?,
            };
            if matches!(status, "true" | "false") {
                params.append_pair("continued", status);
            }
        }
    }
    let html = document(&url)?;
    let root = Node::parse(&html)?;
    let more = root
        .one(".site-pagination-mobile")
        .map(|n| n.text())
        .transpose()?
        .and_then(|text| {
            let (current, total) = text.split_once('/')?;
            Some(current.trim().parse::<usize>().ok()? < total.trim().parse::<usize>().ok()?)
        })
        .unwrap_or(false);
    let items = entries(&base, &root)?;
    Ok(port::items(
        items,
        if more { page.checked_add(1) } else { None },
    ))
}

fn details(input: &Value) -> Result<Value> {
    let base = base(input)?;
    let id = book_path(input["args"]["mangaId"].as_str().unwrap_or_default(), false)?;
    let html = document(&port::absolute(&base, id)?)?;
    let doc = Node::parse(&html)?;
    let info = doc.require("div.site-book-info")?;
    let title = info.require("h1")?.text()?;
    let cover = doc.require("img.site-detail-cover")?.attr("src")?;
    let mut manga = card(&base, id, &title, &cover)?;
    let alias = info
        .one("p.site-book-alias")
        .map(|node| node.text())
        .transpose()?
        .unwrap_or_default();
    let synopsis = doc
        .one("div.site-book-synopsis")
        .map(|node| node.text())
        .transpose()?
        .unwrap_or_default();
    manga["description"] = json!(if alias.is_empty() || alias == title {
        synopsis
    } else {
        format!("別名: {alias}\n\n{synopsis}")
    });
    let mut tags = Vec::new();
    for (key, value) in fields(&info)? {
        match key.as_str() {
            "作者" if !value.is_empty() => manga["authors"] = json!([value]),
            "狀態" => {
                manga["status"] = json!(if value.starts_with("連載中") {
                    "ongoing"
                } else if value.starts_with("已完結") {
                    "completed"
                } else {
                    "unknown"
                })
            }
            "地區" if !value.is_empty() => tags.push(value),
            _ => {}
        }
    }
    if let Some(eyebrow) = info.one("p.site-eyebrow") {
        let text = eyebrow.text()?;
        let title = text.split('/').next().unwrap_or_default().trim();
        if !title.is_empty() {
            tags.push(title.to_string());
        }
    }
    manga["tags"] = json!(tags);
    Ok(json!({"manga":manga}))
}

fn fields(info: &Node) -> Result<Vec<(String, String)>> {
    let keys = info.all("dl.site-book-data dt")?;
    let values = info.all("dl.site-book-data dt + dd")?;
    keys.iter()
        .zip(values)
        .map(|(key, value)| Ok((key.text()?, value.text()?)))
        .collect()
}

fn taipei_date(value: &str) -> Option<String> {
    let mut parts = value.split('/');
    let month = parts.next()?.trim().parse::<u32>().ok()?;
    let day = parts.next()?.trim().parse::<u32>().ok()?;
    let year = parts.next()?.trim().parse::<u32>().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some(format!("{year:04}-{month:02}-{day:02}T00:00:00+08:00"))
}

fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(request)?;
    let base = base(&input)?;
    let id = book_path(input["args"]["mangaId"].as_str().unwrap_or_default(), false)?;
    let html = document(&port::absolute(&base, id)?)?;
    let doc = Node::parse(&html)?;
    let info = doc.require("div.site-book-info")?;
    let date = fields(&info)?
        .into_iter()
        .find(|(key, _)| key.contains("更新"))
        .and_then(|(_, value)| taipei_date(&value));
    let entries = doc
        .all("a.site-chapter-link")?
        .into_iter()
        .rev()
        .map(|link| {
            let chapter = local_path(&base, &link.attr("href")?, true)?;
            if chapter.rsplit_once('/').map(|(owner, _)| owner) != Some(id) {
                return Err("parse_error");
            }
            let title = link.require("span")?.attr("title")?;
            Ok((chapter, title))
        })
        .collect::<Result<Vec<_>>>()?;
    let chapters = entries
        .iter()
        .enumerate()
        .map(|(index, (chapter, title))| {
            let mut out = Chapter::new(chapter.as_bytes(), id.as_bytes(), title.as_bytes());
            out.index = Some(index);
            if index == 0 {
                out.published_at = date.as_deref().map(str::as_bytes);
            }
            out
        })
        .collect::<Vec<_>>();
    ChapterResponse::ordered(payload_buf(), request, ChapterOrder::NewestFirst, &chapters)
        .map_err(port::chapter_error)
}

fn json_start<'a, T: serde::Deserialize<'a>>(value: &'a str) -> Option<T> {
    serde_json::Deserializer::from_str(value)
        .into_iter::<T>()
        .next()?
        .ok()
}

fn nextjs_images(html: &str) -> Option<Vec<String>> {
    let mut stream = Vec::new();
    for script in html
        .split("<script")
        .skip(1)
        .filter_map(|s| s.split_once("</script>").map(|(body, _)| body))
    {
        let Some((tag, body)) = script.split_once('>') else {
            continue;
        };
        if tag.contains("__NEXT_DATA__") {
            if let Some(paths) = embedded_paths(body, &BTreeMap::new()) {
                return Some(paths);
            }
        }
        for push in body.split("self.__next_f.push(").skip(1) {
            let Some(tuple) = json_start::<Vec<Value>>(push) else {
                continue;
            };
            let Some(chunk) = tuple.get(1).and_then(Value::as_str) else {
                continue;
            };
            match tuple.first().and_then(Value::as_u64) {
                Some(1) => stream.extend_from_slice(chunk.as_bytes()),
                Some(3) => {
                    if let Ok(decoded) = flight_base64().decode(chunk) {
                        stream.extend(decoded);
                    }
                }
                _ => {}
            }
        }
    }
    let mut flight = core::str::from_utf8(&stream).ok()?;
    let mut rows = BTreeMap::new();
    let mut order = Vec::new();
    while !flight.is_empty() {
        flight = flight.trim_start_matches('\n');
        let Some((id, rest)) = flight.split_once(':') else {
            break;
        };
        if id.is_empty() || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
            let Some((_, tail)) = flight.split_once('\n') else {
                break;
            };
            flight = tail;
            continue;
        }
        order.push(id);
        if let Some((len, data)) = rest.strip_prefix('T').and_then(|tail| tail.split_once(',')) {
            let len = usize::from_str_radix(len, 16).ok()?;
            rows.insert(id, data.get(..len)?);
            flight = data.get(len..)?;
        } else {
            let (row, tail) = rest.split_once('\n').unwrap_or((rest, ""));
            rows.insert(id, row);
            flight = tail;
        }
    }
    order
        .iter()
        .find_map(|id| embedded_paths(rows.get(id)?, &rows))
}

fn flight_base64() -> GeneralPurpose {
    GeneralPurpose::new(
        &alphabet::STANDARD,
        GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent),
    )
}

fn embedded_paths(body: &str, rows: &BTreeMap<&str, &str>) -> Option<Vec<String>> {
    for piece in body.split("\"imagePaths\"").skip(1) {
        let Some(value) = piece
            .trim_start()
            .strip_prefix(':')
            .and_then(json_start::<Value>)
        else {
            continue;
        };
        let Some(value) = flight_value(value, rows, 0) else {
            continue;
        };
        let Some(items) = value.as_array() else {
            continue;
        };
        let urls = items
            .iter()
            .map(|url| {
                let resolved = flight_value(url.clone(), rows, 0)?;
                Some(resolved.as_str()?.to_string())
            })
            .collect::<Option<Vec<_>>>();
        if let Some(urls) = urls.filter(|urls| !urls.is_empty()) {
            return Some(urls);
        }
    }
    None
}

// React Flight can outline the whole image list or individual URL strings.
fn flight_value(value: Value, rows: &BTreeMap<&str, &str>, depth: usize) -> Option<Value> {
    let Some(reference) = value.as_str().and_then(|s| s.strip_prefix('$')) else {
        return Some(value);
    };
    if depth >= 16 {
        return None;
    }
    if reference.starts_with('$') {
        return Some(json!(reference));
    }
    let mut steps = reference.trim_start_matches(['L', '@']).split(':');
    let raw = rows.get(steps.next()?)?;
    let mut resolved = json_start::<Value>(raw).unwrap_or_else(|| json!(raw));
    for step in steps {
        resolved = flight_value(resolved, rows, depth + 1)?;
        resolved = if resolved.is_array() {
            let index = match step {
                "type" if resolved[0] == "$" => 1,
                "key" if resolved[0] == "$" => 2,
                "props" if resolved[0] == "$" => 3,
                index => index.parse::<usize>().ok()?,
            };
            resolved.get_mut(index)?.take()
        } else {
            resolved.get_mut(step)?.take()
        };
    }
    flight_value(resolved, rows, depth + 1)
}

fn page_paths(html: &str) -> Result<Vec<String>> {
    if let Some(paths) = nextjs_images(html) {
        return Ok(paths);
    }
    // The current TanStack payload assigns its array to the imagePaths reference.
    for value in html.split("imagePaths:").skip(1) {
        if let Some((reference, _)) = value.split_once("=[") {
            if reference.starts_with("$R[") && reference.len() < 32 {
                if let Some(paths) = json_start::<Vec<String>>(&value[reference.len() + 1..]) {
                    if !paths.is_empty() {
                        return Ok(paths);
                    }
                }
            }
        }
    }
    Err("chapter_unavailable")
}

fn pages(input: &Value) -> Result<Value> {
    let base = base(input)?;
    let args = &input["args"];
    let id = book_path(args["chapterId"].as_str().unwrap_or_default(), true)?;
    if let Some(work) = args["mangaId"].as_str() {
        if id.rsplit_once('/').map(|(owner, _)| owner) != Some(work) {
            return Err("invalid_request");
        }
    }
    let url = port::absolute(&base, id)?;
    let html = document(&url)?;
    let (paths, image_base) = match page_paths(&html) {
        Ok(paths) => (paths, base.as_str()),
        // The default site's streamed reader shell sometimes ends before the
        // page table. Retry only this unfinished document at the official
        // alternate origin. A complete error page or access error stays local.
        Err(_) if base == BASE
            && html.contains("__TSR")
            && html.contains("site-reader")
            && !html.contains("</html>") =>
        {
            let alternate = port::absolute(READER_FALLBACK, id)?;
            let html = document(&alternate)?;
            let document = Node::parse(&html)?;
            if document.require("link[rel='canonical']")?.attr("href")? != alternate.as_str() {
                return Err("chapter_unavailable");
            }
            (page_paths(&html)?, READER_FALLBACK)
        }
        Err(error) => return Err(error),
    };
    let pages = paths
        .iter()
        .enumerate()
        .map(|(index, url)| Ok(port::page(index, id, port::absolute(image_base, url)?)))
        .collect::<Result<Vec<_>>>()?;
    Ok(json!({"pages":pages}))
}

fn image(input: &Value, request: &[u8]) -> Result<Value> {
    let base = base(input)?;
    let headers = json!({"Referer":format!("{base}/")});
    let url = port::absolute(
        &base,
        input["args"]["url"].as_str().ok_or("invalid_request")?,
    )?;
    let mut segments = url.path_segments().ok_or("invalid_request")?;
    let Some(file) = segments.next_back() else {
        return Err("invalid_request");
    };
    let scrambled = segments.any(|segment| percent_decode_str(segment).eq(b"sr:1".iter().copied()));
    if !scrambled {
        return Ok(json!({"url":url.as_str(),"headers":headers}));
    }
    if !source_protocol::feature(request, b"source_required_image_decode_wasm_v1") {
        return Err("host_feature_required");
    }
    let name = percent_decode_str(file)
        .decode_utf8()
        .map_err(|_| "parse_error")?;
    let encoded = name
        .rsplit_once('.')
        .map_or(name.as_ref(), |(stem, _)| stem);
    let raw = flight_base64().decode(encoded).map_err(|_| "parse_error")?;
    let blocks = Md5::digest(raw)[15] % 10 + 5;
    Ok(
        json!({"url":url.as_str(),"headers":headers,"requiredImageDecode":{"kind":"wasm_image_v1","pixels":format!("y|{blocks}")}}),
    )
}

fn dispatch(op: &str, request: &[u8]) -> Result<Value> {
    let input = port::input(request)?;
    match op {
        "get_listings" => Ok(json!({"listings":[
            {"id":"popular","name":"熱門","supportsPaging":true},
            {"id":"latest","name":"最近更新","supportsPaging":false,"supportsFilters":false}
        ]})),
        "get_filters" => {
            let mut status = port::select("status", "狀態", "browse", STATUS);
            status["default"] = json!("all");
            let filters = if source_protocol::feature(request, b"source_filter_scope_header_v1") {
                json!([{"type":"header","scope":"browse","name":"提示：搜尋時篩選無效"},status])
            } else {
                status.as_object_mut().unwrap().remove("scope");
                json!([status])
            };
            Ok(json!({"filters":filters}))
        }
        "get_settings" => Ok(
            json!({"settings":[{"id":"baseUrl","name":"自訂基礎 URL","kind":"text","default":"",
            "description":"留空使用 rouman5.com，或輸入原站公告的 HTTPS 網站網址。"}]}),
        ),
        "get_manga_list" => list(&input, request, false),
        "search" => list(&input, request, true),
        "get_manga" => details(&input),
        "get_pages" => pages(&input),
        "get_image_request" => image(&input, request),
        _ => Err("not_implemented"),
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
fn chapter_failure(code: &str) -> u32 {
    port::write_failure("get_chapters", code)
}
koma_source_sdk::koma_source_exports!("roumanwu", typed_chapters, chapter_failure);
