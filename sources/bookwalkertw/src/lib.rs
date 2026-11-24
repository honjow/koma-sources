#![no_std]
extern crate alloc;

use alloc::{
    collections::BTreeSet,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use koma_source_sdk::{
    chapters::{Chapter, ChapterNumber, ChapterOrder, ChapterResponse, ChapterResult},
    host,
    json_utils::request_page_number,
    source::{SourceCapabilities, SourceInfo},
    source_protocol,
};
use serde_json::{json, Value};
use url::Url;

#[allow(dead_code)]
#[path = "../../port_support.rs"]
mod port;
mod publus;
use port::{Node, Result};

const WEB: &str = "https://www.bookwalker.com.tw";
const LOGIN: &str = "https://www.bookwalker.com.tw/login";
const CARD: &str = ".bw_searchbox, .bwbookitem";
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.bookwalkertw.koma",
    name: "BookWalker Taiwan",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "zh-Hant",
    author: "Koma",
    description: "BOOK☆WALKER 台灣漫畫目錄與原站試讀。",
    content_rating: "unknown",
};
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    listings: true,
    manga_list: true,
    filters: true,
    image_request: true,
    credentials: true,
    ..SourceCapabilities::CORE
};

#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;
koma_source_sdk::koma_source_buffers! {payload:1024*1024,http_out:8*1024*1024,body:5*1024*1024,http_req:4096,scratch:8192,}
koma_source_sdk::koma_source_helpers!();

const SORTS: &[(&str, &str)] = &[
    ("rank", "人氣度"),
    ("sell_desc", "發售日新至舊"),
    ("sell_asc", "發售日舊至新"),
    ("price_asc", "價格低至高"),
    ("price_desc", "價格高至低"),
];
const CATEGORIES: &[(&str, &str)] = &[
    ("", "全部漫畫"),
    ("19", "少年漫畫"),
    ("20", "少女漫畫"),
    ("21", "青年漫畫"),
    ("22", "淑女漫畫"),
    ("23", "BL漫畫"),
    ("24", "百合漫畫"),
    ("25", "兒童漫畫"),
    ("26", "漫畫雜誌"),
    ("27", "畫集．設定集"),
    ("69", "日文書"),
    ("78", "同人誌"),
    ("91", "其他"),
];

fn id(value: &str) -> Result<u64> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("invalid_request");
    }
    value
        .parse::<u64>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or("invalid_request")
}

fn series_url(series: u64) -> String {
    format!("/search?series={series}")
}
fn product_url(book: u64) -> String {
    format!("/product/{book}")
}

fn official(url: &Url) -> bool {
    url.scheme() == "https"
        && url.port().is_none()
        && matches!(
            url.host_str(),
            Some("www.bookwalker.com.tw" | "bookwalker.com.tw")
        )
}

fn document_headers(url: &Url) -> Value {
    let mut headers = json!({"Accept":"text/html,application/xhtml+xml"});
    if url.path() == "/search" {
        // Catalog searches use the package's 15/minute shared window.
        headers["Sec-Fetch-User"] = json!("?1");
    }
    headers
}

fn document(url: &Url) -> Result<String> {
    if !official(url) {
        return Err("invalid_request");
    }
    let html = port::request(url.as_str(), "GET", document_headers(url), None)?;
    if html.contains("<title>請登入會員</title>") {
        return Err("auth_required");
    }
    Ok(html)
}

fn catalogue_document(url: &Url, cause: Option<&mut Option<String>>) -> Result<String> {
    let Some(cause) = cause.filter(|_| url.host_str() == Some("www.bookwalker.com.tw") && url.path() == "/search")
    else {
        return document(url);
    };
    if !official(url) {
        return Err("invalid_request");
    }
    if host::check_cancel() {
        return Err("cancelled");
    }
    let request = serde_json::to_vec(&json!({
        "version":1, "method":"GET", "url":url.as_str(), "headers":document_headers(url),
        "responseKind":"bodyText", "timeoutMs":15000, "wantChallengeRequestId":"document"
    })).map_err(|_| "internal_error")?;
    let len = host::http_request(&request, http_out()).map_err(|_| "network_error")?;
    let response = &http_out()[..len];
    let size = match source_protocol::body(response, body_buf()) {
        // The official public catalogue is readable in the visible WebView.
        // Offer that same request when its native access is refused.
        Err("forbidden") => {
            let mut id = [0; 64];
            *cause = source_protocol::text(response, b"challengeRequestId", &mut id)
                .ok().flatten().and_then(|id| core::str::from_utf8(id).ok()).map(String::from);
            return Err("catalogue_web_required");
        }
        other => other?,
    };
    let html = String::from_utf8(body_buf()[..size].to_vec()).map_err(|_| "parse_error")?;
    if html.contains("<title>請登入會員</title>") {
        return Err("auth_required");
    }
    Ok(html)
}

enum Work {
    Product(u64),
    Series(u64),
}

impl Work {
    fn url(&self) -> String {
        match *self {
            Work::Product(book) => product_url(book),
            Work::Series(series) => series_url(series),
        }
    }
}

fn work_link(url: &Url) -> Result<Work> {
    if !official(url) {
        return Err("invalid_request");
    }
    let mut parts = url.path_segments().ok_or("invalid_request")?;
    match (parts.next(), parts.next()) {
        (Some("search"), Some("") | None) => url
            .query_pairs()
            .find(|(key, _)| key == "series")
            .ok_or("invalid_request")
            .and_then(|(_, value)| id(&value))
            .map(Work::Series),
        (Some("serial"), Some(value)) => id(value).map(Work::Series),
        (Some("product" | "browserViewer"), Some(value)) => id(value).map(Work::Product),
        _ => Err("invalid_request"),
    }
}

fn manga_arg(args: &Value) -> Result<Work> {
    let raw = args["mangaId"].as_str().ok_or("invalid_request")?;
    work_link(&port::absolute(WEB, raw)?)
}

fn selected_sort(args: &Value) -> Result<&str> {
    match args["filters"].get("order") {
        None => Ok("rank"),
        _ => port::filter(args, "order", SORTS),
    }
}

fn toggle(args: &Value, field: &str) -> Result<bool> {
    match args["filters"].get(field) {
        None => Ok(false),
        Some(value) => value.as_bool().ok_or("invalid_request"),
    }
}

fn filters(request: &[u8]) -> Value {
    let search_route = source_protocol::feature(request, b"source_filter_search_without_query_v1")
        && source_protocol::feature(request, b"source_filter_scope_header_v1");
    let mut sort = port::select("order", "排序", "search", SORTS);
    sort["default"] = json!("rank");
    let category = port::select("category", "分類", "search", CATEGORIES);
    let mut filters = [
        sort,
        category,
        json!({"type":"boolean","id":"serial","name":"連載","default":false}),
        json!({"type":"boolean","id":"all_ages","name":"全年齡","default":false}),
    ];
    for filter in &mut filters {
        if search_route {
            filter["scope"] = json!("search");
        } else if let Some(object) = filter.as_object_mut() {
            object.remove("scope");
        }
    }
    json!({"filters":filters,"useSearchForFilters":search_route})
}

fn text(node: &Node, selector: &str) -> Result<Option<String>> {
    node.one(selector).map(|node| node.text()).transpose()
}

fn cover(url: &Url, raw: &str) -> Result<Url> {
    port::absolute(url.as_str(), raw)
}

fn large_cover(mut url: Url) -> Url {
    let parts: Vec<_> = url
        .path_segments()
        .into_iter()
        .flatten()
        .map(str::to_string)
        .collect();
    if parts.len() >= 3 && parts[0] == "product" && id(&parts[1]).is_ok() {
        let mut parts = parts;
        parts[2] = format!("zoom_big_{}.jpg", parts[1]);
        url.set_path(&format!("/{}", parts.join("/")));
    }
    url
}

fn card(work: &Work, title: &str, image: Option<Url>) -> Value {
    let work_id = work.url();
    let mut manga = json!({
        "id":work_id,"title":title,"language":"zh-Hant",
        "links":[{"kind":"source","url":format!("{WEB}{work_id}")}]
    });
    if let Some(image) = image {
        manga["cover"] = json!({"kind":"url","url":image.as_str(),"headers":{}});
    }
    manga
}

fn selected_link(node: &Node, base: &Url) -> Result<Url> {
    port::absolute(base.as_str(), &node.require("a")?.attr("href")?)
}

fn search_page(url: &Url, page: usize, cause: Option<&mut Option<String>>) -> Result<Value> {
    let doc = Node::parse(&catalogue_document(url, cause)?)?;
    let mut entries = Vec::new();
    for node in doc.all(CARD)? {
        let link = selected_link(&node, url)?;
        let work = work_link(&link).map_err(|_| "parse_error")?;
        let title = node.require("h4")?.text()?;
        let cover = node
            .one("img[data-src]")
            .map(|image| image.attr("data-src"))
            .transpose()?
            .filter(|image| !image.is_empty())
            .map(|image| cover(url, &image))
            .transpose()?
            .map(large_cover);
        entries.push(card(&work, &title, cover));
    }
    Ok(port::items(
        entries,
        if doc.one("a[rel=next]").is_some() {
            page.checked_add(1)
        } else {
            None
        },
    ))
}

fn list(args: &Value, page: usize, listing: Option<&str>, cause: Option<&mut Option<String>>) -> Result<Value> {
    let query = args["query"].as_str().unwrap_or_default().trim();
    if listing.is_none() {
        if let Ok(mut url) = Url::parse(query) {
            if official(&url) {
                if let Ok(work) = work_link(&url) {
                    if page > 1 {
                        return Ok(port::items(Vec::new(), None));
                    }
                    return Ok(port::items(vec![linked_manga(&work, &url)?], None));
                }
                if url.path() == "/search" {
                    let pairs: Vec<_> = url
                        .query_pairs()
                        .filter(|(k, _)| k != "page")
                        .map(|(k, v)| (k.into_owned(), v.into_owned()))
                        .collect();
                    url.query_pairs_mut()
                        .clear()
                        .extend_pairs(pairs)
                        .append_pair("page", &page.to_string());
                    return search_page(&url, page, cause);
                }
            }
        }
    }
    let order = match listing {
        Some("popular") => "rank",
        Some("latest") => "sell_desc",
        _ => selected_sort(args)?,
    };
    let mut url = port::absolute(WEB, "/search")?;
    {
        let mut pairs = url.query_pairs_mut();
        pairs
            .append_pair("m", "2")
            .append_pair("series_display", "1")
            .append_pair("order", order)
            .append_pair("page", &page.to_string());
        if listing.is_none() {
            if !query.is_empty() {
                pairs.append_pair("w", query);
            }
            let category = port::filter(args, "category", CATEGORIES)?;
            if !category.is_empty() {
                pairs.append_pair("s", category);
            }
            if toggle(args, "serial")? {
                pairs.append_pair("tp", "13");
            }
            if toggle(args, "all_ages")? {
                pairs.append_pair("restricted", "2");
            }
        }
    }
    search_page(&url, page, cause)
}

fn product(book: u64) -> Result<Value> {
    let url = port::absolute(WEB, &product_url(book))?;
    let html = document(&url)?;
    // One real Inertia response has a 94 KiB data-page attribute.
    let doc = host::html_parse(html.as_bytes()).map_err(|_| "parse_error")?;
    let selected = host::html_select(doc, b"#app");
    let result = match selected {
        Ok(app) => {
            let mut bytes = vec![0; 1024 * 1024];
            let result = host::html_attr(app, b"data-page", &mut bytes).map_err(|_| "parse_error");
            let _ = host::html_close(app);
            result.and_then(|len| {
                if len >= bytes.len() {
                    return Err("result_too_large");
                }
                serde_json::from_slice(&bytes[..len]).map_err(|_| "parse_error")
            })
        }
        Err(_) => Err("parse_error"),
    };
    let _ = host::html_close(doc);
    let mut json: Value = result?;
    let result = json["props"]["productData"].take();
    if result.is_object() {
        Ok(result)
    } else {
        Err("parse_error")
    }
}

fn parse_html_text(raw: &str) -> Result<String> {
    if raw.trim().is_empty() {
        return Ok(String::new());
    }
    Node::parse(raw)?.text()
}

fn add_field(description: &mut Vec<String>, label: &str, value: Option<&str>) {
    if let Some(value) = value.filter(|value| !value.trim().is_empty()) {
        description.push(format!("{label}：{value}"));
    }
}

fn product_manga(work: &Work, product: &Value) -> Result<Value> {
    let name = product["product_name"].as_str().ok_or("parse_error")?;
    let series_id = product["product_series_id"].as_u64().unwrap_or(0);
    let title = if series_id > 0 {
        product["product_series_name"].as_str().unwrap_or(name)
    } else {
        name
    };
    let cover = product["product_big_image"]
        .as_str()
        .filter(|url| !url.is_empty())
        .map(|url| port::absolute(WEB, url))
        .transpose()?;
    let mut manga = card(work, title, cover);
    let info = &product["product_detail_info"];
    let credits = |fields: &[&str]| -> Value {
        json!(fields
            .iter()
            .filter_map(|field| info[*field].as_str())
            .filter(|name| !name.trim().is_empty())
            .collect::<Vec<_>>())
    };
    manga["authors"] = credits(&["author", "original_author", "screenwriter"]);
    manga["artists"] = credits(&["manga_artist", "illustrator"]);
    manga["tags"] = json!(product["type_tag"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|tag| tag["name"].as_str())
        .collect::<Vec<_>>());
    let mut description = Vec::new();
    let detail = &product["product_detail"];
    for raw in detail["name"]
        .as_str()
        .filter(|headline| *headline != name && *headline != title)
        .into_iter()
        .chain(detail["introduction"].as_str())
    {
        let text = parse_html_text(raw)?;
        if !text.is_empty() {
            description.push(text);
        }
    }
    for (label, field) in [
        ("角色設定", "character_setting"),
        ("機械設定", "mechanical_setting"),
        ("攝影", "photographer"),
        ("監製", "producer"),
        ("譯者", "translator"),
    ] {
        add_field(&mut description, label, info[field].as_str());
    }
    add_field(
        &mut description,
        "出版社",
        product["publisher"]["text"].as_str(),
    );
    manga["description"] = json!(description.join("\n\n"));
    if series_id == 0 {
        manga["status"] = json!("completed");
    }
    Ok(manga)
}

fn serial_manga(work: &Work, doc: &Node, url: &Url) -> Result<Value> {
    let title = doc.require("h1.bookName")?.text()?;
    let cover = doc
        .one(".mainDescCover img")
        .map(|image| image.attr("src"))
        .transpose()?
        .filter(|image| !image.is_empty())
        .map(|image| cover(url, &image.replace("_mask.jpg", ".jpg")))
        .transpose()?;
    let mut manga = card(work, &title, cover);
    let mut authors = Vec::new();
    let mut artists = Vec::new();
    let mut description = Vec::new();
    if let Some(intro) = text(doc, ".bookIntroConBox .bookIntroCon")? {
        description.push(intro);
    }
    for li in doc.all("ul.bookDetailList li")? {
        let line = li.text()?;
        let key = line.split('：').next().unwrap_or_default().trim();
        let values = li
            .all("a")?
            .into_iter()
            .map(|link| link.text())
            .collect::<Result<Vec<_>>>()?;
        match key {
            "作者" | "原作" | "編劇" => authors.extend(values),
            "漫畫" | "插畫" => artists.extend(values),
            "譯者" | "出版社" => add_field(&mut description, key, Some(&values.join(", "))),
            _ => {}
        }
    }
    add_field(
        &mut description,
        "連載時間",
        text(doc, ".bookUpdateTime")?.as_deref(),
    );
    manga["authors"] = json!(authors);
    manga["artists"] = json!(artists);
    manga["tags"] = json!(doc
        .all(".tagKeywordBox a.tagKeyword")?
        .into_iter()
        .map(|tag| tag.text())
        .collect::<Result<Vec<_>>>()?);
    manga["description"] = json!(description.join("\n\n"));
    manga["status"] = json!(match text(doc, ".bookTags span")?.as_deref() {
        Some("連載中") => "ongoing",
        Some("已完結") => "completed",
        _ => "unknown",
    });
    Ok(manga)
}

#[derive(Clone, Copy)]
enum Viewer {
    Read,
    Trial,
}
impl Viewer {
    fn path(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Trial => "trial",
        }
    }
}
struct Book {
    id: u64,
    title: String,
    viewer: Option<Viewer>,
    date: Option<String>,
    number: Option<String>,
}

fn book(id: u64, name: &str, series: Option<&str>, owned: bool, preview: bool) -> Book {
    let title = series
        .and_then(|series| name.strip_prefix(series))
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .unwrap_or(name);
    let (viewer, title) = if owned {
        (Some(Viewer::Read), title.to_string())
    } else if preview {
        (Some(Viewer::Trial), format!("🔒（試讀）{title}"))
    } else {
        (None, format!("🔒 {title}"))
    };
    Book {
        id,
        title,
        viewer,
        date: None,
        number: None,
    }
}

fn product_date(value: Option<&str>) -> Option<String> {
    let (year, rest) = value?.split_once('年')?;
    let (month, day) = rest.split_once('月')?;
    let day = day.strip_suffix('日')?;
    Some(format!(
        "{:04}-{:02}-{:02}T00:00:00+08:00",
        year.parse::<u16>().ok()?,
        month.parse::<u8>().ok()?,
        day.parse::<u8>().ok()?
    ))
}

fn episode_date(value: Option<&str>) -> Option<String> {
    value
        .filter(|date| date.len() == 19 && date.is_ascii() && &date[10..11] == " ")
        .map(|date| format!("{}T{}+08:00", &date[..10], &date[11..]))
}

fn product_book(id: u64, product: &Value) -> Result<Book> {
    let name = product["product_name"].as_str().ok_or("parse_error")?;
    let mut book = book(
        id,
        name,
        None,
        product["is_owned_product"] == true,
        product["is_preview_product"] == true,
    );
    book.date = product_date(product["product_detail_info"]["sell_date_start"].as_str());
    Ok(book)
}

fn episodes(series: u64) -> Result<Vec<Book>> {
    let referer = format!("{WEB}/serial/{series}");
    let mut books = Vec::new();
    let mut seen = BTreeSet::new();
    let mut page = 1u64;
    loop {
        let url = format!("{WEB}/serial/next/{series}?order=desc&page={page}&page_type=pc");
        let body = port::request(
            &url,
            "GET",
            json!({
                "Accept":"application/json","X-Requested-With":"XMLHttpRequest","Referer":referer
            }),
            None,
        )?;
        let response: Value = serde_json::from_str(&body).map_err(|_| "parse_error")?;
        let last = response["last_page"].as_u64().ok_or("parse_error")?;
        let rows = response["data"].as_array().ok_or("parse_error")?;
        for row in rows {
            let id = row["product_id"]
                .as_u64()
                .filter(|id| *id > 0)
                .ok_or("parse_error")?;
            if !seen.insert(id) {
                continue;
            }
            let name = row["product_name"].as_str().ok_or("parse_error")?;
            let mut book = book(
                id,
                name,
                row["series_name"].as_str(),
                row["is_owned"] == true,
                row["is_preview"] == 1,
            );
            book.number = row["episode"]
                .as_str()
                .filter(|n| n.parse::<f32>().is_ok_and(f32::is_finite))
                .map(str::to_string);
            book.date = episode_date(row["product_sell_date_start"].as_str());
            books.push(book);
        }
        if page >= last {
            break;
        }
        if books.len() > 5_000 || page >= 100 {
            return Err("result_too_large");
        }
        if rows.is_empty() {
            return Err("parse_error");
        }
        page += 1;
    }
    Ok(books)
}

fn direct_manga_route(mut manga: Value, url: &str) -> Value {
    // The host keeps the route URL and the share link separately. Both point
    // to the same official document, while the series/work ID stays stable.
    manga["url"] = json!(url);
    manga["links"][0]["url"] = json!(url);
    manga
}

fn linked_manga(work: &Work, url: &Url) -> Result<Value> {
    let Work::Product(id) = *work else {
        return Ok(catalog(work, false, Some(url.as_str()))?.0);
    };
    let volume = product(id)?;
    let Some(series) = volume["product_series_id"].as_u64().filter(|id| *id > 0) else {
        let manga = product_manga(work, &volume)?;
        return Ok(direct_manga_route(manga, &format!("{WEB}/product/{id}")));
    };
    let identity = Work::Series(series);
    let (manga, _) = volume_catalog(&identity, series, id, volume, false)?;
    Ok(direct_manga_route(manga, &format!("{WEB}/product/{id}")))
}

fn volume_catalog(
    work: &Work,
    series: u64,
    volume_id: u64,
    volume: Value,
    chapters: bool,
) -> Result<(Value, Vec<Book>)> {
    let first_id = volume["product_series_list"]
        .get(0)
        .and_then(|row| row["id"].as_u64())
        .filter(|id| *id > 0)
        .unwrap_or(volume_id);
    let first = if first_id == volume_id {
        volume
    } else {
        product(first_id)?
    };
    if first["product_series_id"].as_u64() != Some(series) {
        return Err("parse_error");
    }
    let manga = product_manga(work, &first)?;
    let mut books = Vec::new();
    if chapters {
        for row in first["product_series_list"]
            .as_array()
            .ok_or("parse_error")?
            .iter()
            .rev()
        {
            let id = row["id"]
                .as_u64()
                .filter(|id| *id > 0)
                .ok_or("parse_error")?;
            let name = row["name"].as_str().ok_or("parse_error")?;
            books.push(book(
                id,
                name,
                manga["title"].as_str(),
                row["is_owned_series_product"] == true,
                row["is_preview_series_product"] == true,
            ));
        }
    }
    Ok((manga, books))
}

fn serial_catalog(
    work: &Work,
    series: u64,
    url: &Url,
    doc: &Node,
    chapters: bool,
) -> Result<(Value, Vec<Book>)> {
    let manga = direct_manga_route(serial_manga(work, doc, url)?, url.as_str());
    Ok((
        manga,
        if chapters {
            episodes(series)?
        } else {
            Vec::new()
        },
    ))
}

fn catalog(work: &Work, chapters: bool, route: Option<&str>) -> Result<(Value, Vec<Book>)> {
    match *work {
        Work::Product(id) => {
            let product = product(id)?;
            Ok((
                direct_manga_route(
                    product_manga(work, &product)?,
                    &format!("{WEB}/product/{id}"),
                ),
                if chapters {
                    vec![product_book(id, &product)?]
                } else {
                    Vec::new()
                },
            ))
        }
        Work::Series(series) => {
            // Use a direct official route only when the caller already has one
            // for this series. The normal series search retains its own route
            // and HTTP errors, including an access refusal.
            if let Some(hint) = route.and_then(|raw| port::absolute(WEB, raw).ok()) {
                match (
                    hint.path_segments().and_then(|mut parts| parts.next()),
                    work_link(&hint),
                ) {
                    (Some("serial"), Ok(Work::Series(id))) if id == series => {
                        let url = port::absolute(WEB, &format!("/serial/{series}"))?;
                        let doc = Node::parse(&document(&url)?)?;
                        return serial_catalog(work, series, &url, &doc, chapters);
                    }
                    (Some("product" | "browserViewer"), Ok(Work::Product(id))) => {
                        let volume = product(id)?;
                        if volume["product_series_id"].as_u64() == Some(series) {
                            let (manga, books) =
                                volume_catalog(work, series, id, volume, chapters)?;
                            return Ok((
                                direct_manga_route(manga, &format!("{WEB}/product/{id}")),
                                books,
                            ));
                        }
                    }
                    _ => {}
                }
            }
            let url = port::absolute(WEB, &series_url(series))?;
            let doc = Node::parse(&document(&url)?)?;
            if doc.one("h1.bookName").is_some() {
                let url = port::absolute(WEB, &format!("/serial/{series}"))?;
                return serial_catalog(work, series, &url, &doc, chapters);
            }
            let first_card = doc.require(CARD)?;
            let Work::Product(id) = work_link(&selected_link(&first_card, &url)?)? else {
                return Err("parse_error");
            };
            volume_catalog(work, series, id, product(id)?, chapters)
        }
    }
}

fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(request)?;
    let work = manga_arg(&input["args"])?;
    let manga_id = work.url();
    let books = catalog(&work, true, input["args"]["mangaUrl"].as_str())?.1;
    let ids: Vec<_> = books.iter().map(|book| product_url(book.id)).collect();
    let chapters: Vec<_> = books
        .iter()
        .zip(&ids)
        .map(|(book, id)| {
            let mut entry = Chapter::new(id.as_bytes(), manga_id.as_bytes(), book.title.as_bytes());
            entry.language = Some(b"zh-Hant");
            entry.locked = Some(book.viewer.is_none());
            entry.published_at = book.date.as_deref().map(str::as_bytes);
            entry.chapter_number = book
                .number
                .as_deref()
                .map(|number| ChapterNumber::Text(number.as_bytes()));
            entry
        })
        .collect();
    if let (Work::Product(_), [chapter]) = (&work, chapters.as_slice()) {
        return ChapterResponse::single(payload_buf(), *chapter).map_err(port::chapter_error);
    }
    ChapterResponse::ordered(payload_buf(), request, ChapterOrder::NewestFirst, &chapters)
        .map_err(port::chapter_error)
}

fn pages(request: &[u8], args: &Value) -> Result<Value> {
    let chapter = args["chapterId"].as_str().ok_or("invalid_request")?;
    let id = self::id(chapter.strip_prefix("/product/").ok_or("invalid_request")?)?;
    let book = if args["mangaId"].as_str().is_some() {
        let work = manga_arg(args)?;
        catalog(&work, true, args["mangaUrl"].as_str())?
            .1
            .into_iter()
            .find(|book| book.id == id)
            .ok_or("not_found")?
    } else {
        product_book(id, &product(id)?)?
    };
    let viewer = book.viewer.ok_or("chapter_locked")?;
    publus::pages(request, id, viewer)
}

fn run(op: &str, request: &[u8]) -> u32 {
    let mut request_id = None;
    let cause = source_protocol::feature(request, b"source_http_challenge_cause_v1").then_some(&mut request_id);
    let result = port::input(request).and_then(|input| {
        let args = &input["args"];
        match op {
            "search" => list(args, request_page_number(request), None, cause),
            "get_manga_list" => {
                let listing = match args["listingId"].as_str() {
                    Some(value @ ("popular" | "latest")) => value,
                    _ => return Err("invalid_request"),
                };
                list(args, request_page_number(request), Some(listing), cause)
            }
            "get_listings" => Ok(port::listing_list(true, true, false)),
            "get_filters" => Ok(filters(request)),
            "get_manga" => Ok(json!({"manga":catalog(&manga_arg(args)?, false, args["mangaUrl"].as_str())?.0})),
            "get_pages" => pages(request, args),
            "get_image_request" => publus::image(request, args),
            "get_authentication" => Ok(
                json!({"authentication":{"kind":"web","loginUrl":LOGIN},"authenticated":false}),
            ),
            _ => Err("unsupported_operation"),
        }
    });
    if matches!(result, Err("catalogue_web_required")) {
        return response_buffer().write_challenge(
            op,
            "請在原站確認可讀取這個公開書單，再回來重試。",
            request_id.as_deref().unwrap_or_default().as_bytes(),
        );
    }
    port::write_result(op, result)
}

fn run_search(r: &[u8]) -> u32 {
    run("search", r)
}
fn run_get_listings(r: &[u8]) -> u32 {
    run("get_listings", r)
}
fn run_get_manga_list(r: &[u8]) -> u32 {
    run("get_manga_list", r)
}
fn run_get_manga(r: &[u8]) -> u32 {
    run("get_manga", r)
}
fn run_get_pages(r: &[u8]) -> u32 {
    run("get_pages", r)
}
fn run_get_image_request(r: &[u8]) -> u32 {
    run("get_image_request", r)
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
fn run_get_authentication(r: &[u8]) -> u32 {
    run("get_authentication", r)
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

koma_source_sdk::koma_source_exports!("bookwalkertw", typed_chapters, chapter_failure);
koma_source_sdk::koma_source_authentication_exports!("bookwalkertw");
