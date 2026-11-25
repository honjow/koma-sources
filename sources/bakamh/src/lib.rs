// SPDX-License-Identifier: Apache-2.0
// Behavior ported from Keiyoushi Bakamh 8947c37; Madara search/alt-title synced at 8f5540b.
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
    chapters::{Chapter, ChapterPage, ChapterPlacement, ChapterResult, GroupedChapters},
    json_utils::request_page_number,
    source::{SourceCapabilities, SourceInfo},
    source_protocol,
};
use serde_json::{json, Value};
use url::Url;

mod dates;
mod document_body;
mod document_text;
mod filters;
mod reader;

#[path = "../../port_support.rs"]
#[allow(dead_code)]
mod port_support;
use port_support::{self as port, Node, Result};

const BASE: &str = "https://bakamh.com";
const LANGUAGE: &str = "zh-CN,zh;q=0.9,en-US;q=0.8,en;q=0.7";
// Pinned upstream BrowserHeaders Chrome DOCUMENT Accept, on the same public
// Chromium baseline as IMAGE_ACCEPT; the source does not invent UA/Sec-* hints.
const DOCUMENT_ACCEPT: &str = "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7";
const AJAX: &str = "https://bakamh.com/wp-admin/admin-ajax.php";
const PAGE_SIZE: usize = 25;
const DETAILS_TITLE: &str = "div.post-title h3, div.post-title h1, #manga-title > h1";

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.bakamh.koma",
    name: "巴卡漫画",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "巴卡以中文翻译的韩国条漫为主，有独立的 BL 区，也收录英文版和全年龄作品。部分新章与外传需站点账号。",
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
    filters: true,
    settings: false,
    image_request: true,
    credentials: false,
};
#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;
koma_source_sdk::koma_source_buffers! {
    payload: 1024*1024, http_out: 6*1024*1024, body: 4*1024*1024, http_req: 8192, scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

fn get_document(url: &Url, request: &[u8]) -> Result<(Node, bool)> {
    // BodyRef is generic response data. Only the already-declared exact origin
    // uses this Native document lane; legacy/foreign inputs keep the R10 path.
    if url.origin().ascii_serialization() == BASE
        && source_protocol::feature(request, b"source_http_body_v1")
        && source_protocol::feature(request, b"source_body_v1")
    {
        return document_body::run(url, request);
    }
    let headers = json!({
        "Accept":DOCUMENT_ACCEPT,"Accept-Language":LANGUAGE,"Referer":format!("{BASE}/")
    });
    // Only already canonical site documents enter Web HTTP. In particular,
    // WordPress ?p=ID and noncanonical pasted URLs keep their redirect resolver:
    // the negotiated text lane deliberately rejects even same-origin redirects.
    let canonical = url.origin().ascii_serialization() == BASE
        && url.path().starts_with("/manga/")
        && url.path().ends_with('/')
        && url.query_pairs().all(|(key, value)| key == "style" && value == "list");
    let html = if canonical && source_protocol::feature(request, b"source_web_http_v1") {
        if koma_source_sdk::host::check_cancel() {
            return Err("cancelled");
        }
        // Web fetch is a subresource request, not upstream's DOCUMENT navigation.
        // Its Sec-Fetch-* metadata belongs to the browser; change only this Accept.
        let mut headers = headers;
        headers["Accept"] = json!("*/*");
        let envelope = json!({
            "version":1,"method":"GET","url":url.as_str(),"headers":headers,
            "responseKind":"bodyText","timeoutMs":15000,"webFetch":{"version":1}
        });
        let bytes = serde_json::to_vec(&envelope).map_err(|_| "internal_error")?;
        let len = koma_source_sdk::host::http_request(&bytes, http_out())
            .map_err(|_| "network_error")?;
        let len = source_protocol::body(&http_out()[..len], body_buf())?;
        String::from_utf8(body_buf()[..len].to_vec()).map_err(|_| "parse_error")?
    } else {
        port::request(url.as_str(), "GET", headers, None)?
    };
    // A repaired DOM alone cannot establish that a complete MangaPage arrived.
    // Require the document tail before granting the verified flat catalogue.
    let lower = html.to_ascii_lowercase();
    let complete = matches!((lower.rfind("</body>"), lower.rfind("</html>")),
        (Some(body), Some(end)) if body < end);
    Ok((Node::parse(&html)?, complete))
}

// Only work lookups consume an effective document URL. Reader/filter/image
// routes retain their existing transport and identity behavior.
fn work_document(url: &Url, request: &[u8], reader_url_search: bool) -> Result<document_text::WorkDocument> {
    if document_text::target(url)
        && source_protocol::feature(request, b"source_http_response_metadata_v1")
    {
        document_text::run(url, request, reader_url_search)
    } else {
        let (doc, complete) = get_document(url, request)?;
        Ok(document_text::WorkDocument { doc, complete, final_url: None })
    }
}

fn get(url: &Url, request: &[u8]) -> Result<Node> {
    get_document(url, request).map(|(document, _)| document)
}

fn post(mut form: filters::Form, referer: &str, request: &[u8]) -> Result<String> {
    let headers = json!({
        "Accept-Language":LANGUAGE,"X-Requested-With":"XMLHttpRequest",
        "Content-Type":"application/x-www-form-urlencoded","Referer":referer
    });
    let body = form.finish();
    if !source_protocol::feature(request, b"source_web_http_v1") {
        return port::request(AJAX, "POST", headers, Some(&body));
    }
    if koma_source_sdk::host::check_cancel() { return Err("cancelled"); }
    // Source-owned explicit selection: ordinary POSTs in other sources remain
    // Native. Preserve the original method/body/headers/budget and error policy.
    let envelope = json!({
        "version":1,"url":AJAX,"method":"POST","headers":headers,"body":body,
        "responseKind":"bodyText","timeoutMs":15000,"webFetch":{"version":1}
    });
    let bytes = serde_json::to_vec(&envelope).map_err(|_| "internal_error")?;
    let len = koma_source_sdk::host::http_request(&bytes, http_out()).map_err(|_| "network_error")?;
    let len = source_protocol::body(&http_out()[..len], body_buf())?;
    String::from_utf8(body_buf()[..len].to_vec()).map_err(|_| "parse_error")
}

fn numeric_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 20 && id.bytes().all(|byte| byte.is_ascii_digit())
}

fn path_segment(slug: &str) -> bool {
    !slug.is_empty()
        && slug != "."
        && slug != ".."
        && !slug.contains(['/', '\\', '?', '#', ':'])
        && !slug.bytes().any(|byte| byte.is_ascii_control())
}

fn manga_path(raw: &str) -> Option<String> {
    let url = port::absolute(BASE, raw).ok()?;
    if url.host_str() != Some("bakamh.com") {
        return None;
    }
    let slug = url.path().strip_prefix("/manga/")?.trim_end_matches('/');
    if path_segment(slug) {
        // Fixed upstream stores encodedPath in memo.path; lowercasing its numeric
        // manga.url does not authorize rewriting this provider-owned opaque slug.
        Some(format!("/manga/{slug}/"))
    } else {
        None
    }
}

fn shortlink_id(url: &Url) -> Option<String> {
    url.query_pairs()
        .find(|(name, value)| name == "p" && numeric_id(value))
        .map(|(_, id)| id.into_owned())
}

fn post_id(doc: &Node) -> Option<String> {
    for (selector, attr) in [
        ("[id^='manga-chapters-holder'][data-id]", "data-id"),
        ("input.rating-post-id[value]", "value"),
        ("a[data-post]", "data-post"),
    ] {
        if let Some(id) = doc
            .one(selector)
            .and_then(|node| node.attr(attr).ok())
            .filter(|id| numeric_id(id))
        {
            return Some(id);
        }
    }
    let link = doc.one("link[rel='shortlink'][href]")?.attr("href").ok()?;
    shortlink_id(&port::absolute(BASE, &link).ok()?)
}

struct Series {
    id: String,
    doc: Node,
    url: Url,
    complete_document: bool,
    resolved_path: Option<String>,
}

impl Series {
    fn canonical_path(&self) -> Result<String> {
        // Upstream memo uses actual document.location encodedPath. Only the
        // matching numeric postID below grants this authoritative route; do not
        // replace it with a possibly stale HTML canonical alias.
        if let Some(path) = &self.resolved_path { return Ok(path.clone()); }
        for (selector, attr) in [
            ("link[rel='canonical'][href]", "href"),
            ("meta[property='og:url'][content]", "content"),
        ] {
            if let Some(path) = self
                .doc
                .one(selector)
                .and_then(|node| node.attr(attr).ok())
                .and_then(|url| manga_path(&url))
            {
                return Ok(path);
            }
        }
        manga_path(self.url.as_str()).ok_or("parse_error")
    }

    fn link(&self) -> Result<Url> {
        if let Ok(path) = self.canonical_path() {
            return port::absolute(BASE, &path);
        }
        Ok(self.url.clone())
    }
}

fn resolved_series(id: String, requested: Url, document: document_text::WorkDocument) -> Series {
    let resolved_path = if post_id(&document.doc).as_deref() == Some(id.as_str()) {
        document.final_url.as_ref().and_then(|url| manga_path(url.as_str()))
    } else { None };
    Series {
        id,
        doc: document.doc,
        url: document.final_url.unwrap_or(requested),
        complete_document: document.complete,
        resolved_path,
    }
}

fn work_id(args: &Value) -> Result<&str> {
    args["mangaId"]
        .as_str()
        .and_then(|id| id.strip_prefix("manga:"))
        .filter(|id| numeric_id(id))
        .ok_or("invalid_request")
}

fn series(id: &str, route: Option<&str>, request: &[u8]) -> Result<Series> {
    if let Some(path) = route.and_then(manga_path) {
        let url = port::absolute(BASE, &path)?;
        match work_document(&url, request, false) {
            Ok(document) if document.doc.one(DETAILS_TITLE).is_some()
                && post_id(&document.doc).as_deref() == Some(id)
                // Fixed Madara supportsPostId flow: a redirected canonical
                // response still uses the already-known numeric resolver.
                && document.final_url.as_ref().map_or(true, |effective| effective == &url) => {
                return Ok(resolved_series(id.to_string(), url, document));
            }
            // Keep a Host-confirmed challenge and its observed canonical GET.
            // Ordinary forbidden still uses Madara's known numeric post-ID resolver;
            // authorization, rate-limit and cancellation remain terminal.
            Err(code @ ("challenge_required" | "auth_required" | "rate_limit" | "cancelled")) => {
                return Err(code)
            }
            _ => {}
        }
    }
    let url = port::absolute(BASE, &format!("/?p={id}"))?;
    let document = work_document(&url, request, false)?;
    document.doc.require(DETAILS_TITLE)?;
    if post_id(&document.doc).is_some_and(|reported| reported != id) {
        return Err("not_found");
    }
    Ok(resolved_series(id.to_string(), url, document))
}

fn series_for_url(url: Url, request: &[u8]) -> Result<Series> {
    if url.host_str() != Some("bakamh.com") {
        return Err("not_found");
    }
    // Direct reader URLs keep get_document's original route; a known work or
    // numeric URL that resolves to a reader retains this owner-lookup branch.
    let document = work_document(&url, request, true)?;
    if document.doc.one("body.reading-manga").is_some() {
        return series(&post_id(&document.doc).ok_or("not_found")?, None, request);
    }
    document.doc.require(DETAILS_TITLE)?;
    let id = post_id(&document.doc)
        .or_else(|| shortlink_id(&url))
        .ok_or("not_found")?;
    Ok(resolved_series(id, url, document))
}

fn image_from(node: &Node, base: &Url) -> Option<Url> {
    for attr in [
        "data-src",
        "data-lazy-src",
        "data-lzl-src",
        "data-cfsrc",
        "data-manga-src",
    ] {
        if let Ok(value) = node.attr(attr) {
            return port::absolute(base.as_str(), &value).ok();
        }
    }
    if let Ok(srcset) = node.attr("srcset") {
        let candidates = srcset
            .split(',')
            .filter_map(|pair| {
                let mut fields = pair.split_whitespace();
                let url = port::absolute(base.as_str(), fields.next()?).ok()?;
                let descriptor = fields
                    .next()
                    .and_then(|text| text.strip_suffix(['w', 'x']))
                    .and_then(|width| width.parse::<f32>().ok())
                    .filter(|width| width.is_finite());
                Some((url, descriptor))
            })
            .collect::<Vec<_>>();
        return candidates
            .iter()
            .filter_map(|(url, size)| Some((url, (*size)?)))
            .max_by(|(_, x), (_, y)| x.total_cmp(y))
            .map(|(url, _)| url.clone())
            .or_else(|| {
                candidates
                    .into_iter()
                    .map(|(url, _)| url)
                    .max_by(|a, b| a.as_str().cmp(b.as_str()))
            });
    }
    port::absolute(base.as_str(), &node.attr("src").ok()?).ok()
}

fn image_body_supported(request: &[u8]) -> bool {
    [b"source_http_body_v1".as_slice(), b"source_image_body_v1", b"source_body_v1"]
        .iter().all(|feature| source_protocol::feature(request, feature))
}

fn manga_item(id: &str, title: &str, cover: Option<Url>, link: &Url, request: &[u8]) -> Value {
    let cover = cover.map_or_else(
        || json!({"kind":"none"}),
        |image| {
            if image.scheme() == "https" && image_body_supported(request) {
                // A granted image needs the source resolver, not the public-cover
                // shortcut that sends only static headers through NetworkKit.
                json!({"kind":"url","url":image.as_str()})
            } else {
                json!({"kind":"url","url":image.as_str(),"headers":{"Referer":format!("{BASE}/")}})
            }
        },
    );
    json!({"id":format!("manga:{id}"),"title":title,"cover":cover,
        "url":link.as_str(),"links":[{"kind":"source","url":link.as_str()}]})
}

fn parse_archive(html: &str, request: &[u8]) -> Result<Vec<Value>> {
    let html = html.trim();
    if html.is_empty() {
        return Ok(Vec::new());
    }
    // WordPress returns this sentinel when the AJAX action has no result handler.
    if html == "0" {
        return Err("invalid_response");
    }
    let base = port::absolute(BASE, "/")?;
    let doc = Node::parse(html)?;
    let cards = doc.all("div.page-item-detail, .manga__item, .c-tabs-item__content")?;
    let mut rows = Vec::new();
    for card in cards {
        let id = card
            .attr("data-post-id")
            .ok()
            .filter(|id| numeric_id(id))
            .or_else(|| {
                card.one("[data-post-id]")
                    .and_then(|node| node.attr("data-post-id").ok())
                    .filter(|id| numeric_id(id))
            });
        let (Some(id), Some(link)) = (id, card.one(".post-title a[href]")) else {
            continue;
        };
        let url = port::absolute(BASE, &link.attr("href")?)?;
        if manga_path(url.as_str()).is_some() {
            rows.push(manga_item(
                &id,
                &link.text()?,
                card.one("img").and_then(|img| image_from(&img, &base)),
                &url,
                request,
            ));
        }
    }
    // An unrelated whole page is not a successful empty AJAX catalogue.
    if rows.is_empty() && !doc.all("header, body.error404, form#loginform")?.is_empty() {
        return Err("parse_error");
    }
    Ok(rows)
}

fn archive_page(
    args: &Value,
    request: &[u8],
    search: bool,
    page: usize,
    by_alt_name: bool,
) -> Result<Vec<Value>> {
    let mut form = filters::archive_form(page - 1);
    form.append_pair("vars[paged]", "1")
        .append_pair("vars[meta_query][0][key]", "_wp_manga_chapter_type")
        .append_pair("vars[meta_query][0][value]", "manga");
    if search {
        filters::search(&mut form, args, by_alt_name)?;
    } else {
        match args["listingId"].as_str() {
            Some("popular") => filters::sort(&mut form, "_wp_manga_views"),
            Some("latest") => filters::sort(&mut form, "_latest_update"),
            _ => return Err("invalid_request"),
        }
    }
    let html = post(form, &format!("{BASE}/"), request)?;
    // A successful HTTP completion can race with cancellation. Do not turn that
    // cancelled search into a successful title/alternative result.
    if koma_source_sdk::host::check_cancel() { return Err("cancelled"); }
    parse_archive(&html, request)
}

fn merge_search_pages(title: Vec<Value>, alternate: Vec<Value>, page: usize) -> Result<Value> {
    // Count each raw route before distinctBy(SManga::url). Even the same 25
    // works in both routes must retain the next page.
    let has_more = title.len() == PAGE_SIZE || alternate.len() == PAGE_SIZE;
    let mut seen = BTreeSet::new();
    let mut rows = Vec::new();
    for manga in title.into_iter().chain(alternate) {
        let id = manga["id"].as_str().ok_or("parse_error")?;
        if seen.insert(id.to_string()) {
            rows.push(manga);
        }
    }
    Ok(port::items(rows, if has_more { page.checked_add(1) } else { None }))
}

fn browse(args: &Value, request: &[u8], search: bool) -> Result<Value> {
    let query = args["query"].as_str().unwrap_or("");
    if search {
        if let Ok(url) = Url::parse(query.trim()) {
            if matches!(url.scheme(), "https" | "http") {
                let series = series_for_url(url, request)?;
                return Ok(port::items(vec![details(&series, false, request)?], None));
            }
        }
    }
    let page = request_page_number(request);
    let rows = archive_page(args, request, search, page, false)?;
    if search && !query.trim().is_empty() {
        // Synchronous HTTP retains upstream's all-or-fail contract. Never
        // return the title route if the alternative route fails or is cancelled.
        let alternate = archive_page(args, request, true, page, true)?;
        return merge_search_pages(rows, alternate, page);
    }
    let next = if rows.len() == PAGE_SIZE { page.checked_add(1) } else { None };
    Ok(port::items(rows, next))
}

fn genre_links(doc: &Node, selector: &str, url: &Url) -> Result<Vec<(String, String)>> {
    let mut result = Vec::new();
    let mut seen = BTreeSet::new();
    for element in doc.all(selector)? {
        let link = port::absolute(url.as_str(), &element.attr("href")?)?;
        if let Some(slug) = link
            .path()
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .filter(|slug| path_segment(slug))
        {
            let name = element.text()?;
            if !name.is_empty() && seen.insert(slug.to_string()) {
                result.push((slug.to_string(), name));
            }
        }
    }
    Ok(result)
}

fn view_count(doc: &Node, url: &Url, request: &[u8]) {
    let data = doc
        .one("script#wp-manga-js-extra")
        .and_then(|script| script.text().ok())
        .and_then(|script| {
            let (_, json) = script.split_once("var manga = ")?;
            serde_json::from_str::<Value>(json.rsplit_once(';')?.0.trim()).ok()
        });
    let Some(data) = data else { return };
    if !(data["enable_manga_view"] == "1" || data["enable_manga_view"] == 1) {
        return;
    }
    let id = data["manga_id"]
        .as_str()
        .map(ToString::to_string)
        .or_else(|| data["manga_id"].as_u64().map(|id| id.to_string()));
    let Some(id) = id.filter(|id| numeric_id(id)) else {
        return;
    };
    let mut form = url::form_urlencoded::Serializer::new(String::new());
    form.append_pair("action", "manga_views")
        .append_pair("manga", &id);
    for (key, field) in [("chapter", "chapter_slug"), ("nonce", "nonce")] {
        if let Some(value) = data[field]
            .as_str()
            .filter(|value| !value.trim().is_empty())
        {
            form.append_pair(key, value);
        }
    }
    // The runtime provides synchronous HTTP. Keep the optional view count
    // within a short budget, and never interpret its response as chapter data.
    if !koma_source_sdk::host::check_cancel() {
        let mut envelope = json!({"version":1,"url":AJAX,"method":"POST","headers":{
            "Accept-Language":LANGUAGE,"X-Requested-With":"XMLHttpRequest",
            "Content-Type":"application/x-www-form-urlencoded","Referer":url.as_str()
        },"body":form.finish(),"responseKind":"bodyText","timeoutMs":1200});
        if source_protocol::feature(request, b"source_web_http_v1") {
            envelope["webFetch"] = json!({"version":1});
        }
        if let Ok(bytes) = serde_json::to_vec(&envelope) {
            let _ = koma_source_sdk::host::http_request(&bytes, http_out());
        }
    }
}

fn updating(text: &str) -> bool {
    let text = text.to_lowercase();
    text.contains("updating") || text.contains("atualizando")
}

fn text_list(doc: &Node, selector: &str) -> Result<Vec<String>> {
    doc.all(selector)?
        .into_iter()
        .map(|node| node.text())
        .collect()
}

fn status(doc: &Node) -> Result<&'static str> {
    let mut value = String::new();
    for row in doc.all(".post-content_item")? {
        if row.text()?.contains("状态") {
            if let Some(content) = row.one(".summary-content") {
                value = content.text()?.to_lowercase();
            }
        }
    }
    for (state, names) in [
        (
            "completed",
            &[
                "已完结",
                "completed",
                "completo",
                "completado",
                "concluído",
                "concluido",
                "finalizado",
                "achevé",
                "terminé",
                "hoàn thành",
                "مكتملة",
                "مكتمل",
                "tamamlandı",
                "đã hoàn thành",
                "завершено",
                "tamamlanan",
                "complété",
            ][..],
        ),
        (
            "ongoing",
            &[
                "连载中",
                "ongoing",
                "on going",
                "updating",
                "продолжается",
                "em lançamento",
                "em andamento",
                "en cours",
                "ativo",
                "lançando",
                "đang tiến hành",
                "còn nữa",
                "devam ediyor",
                "in corso",
                "in arrivo",
                "مستمرة",
                "مستمر",
                "en curso",
                "emision",
                "curso",
                "en marcha",
                "publicandose",
                "publicándose",
                "en emision",
                "đang làm",
                "em postagem",
                "devam eden",
                "em progresso",
                "atualizações semanais",
                "виходить",
                "đang ra",
            ][..],
        ),
        (
            "hiatus",
            &[
                "on hold",
                "hiatus",
                "pausado",
                "en espera",
                "durduruldu",
                "beklemede",
                "đang chờ",
                "متوقف",
                "en pause",
                "заморожено",
                "en attente",
                "tạm dừng",
            ][..],
        ),
        (
            "cancelled",
            &[
                "canceled",
                "cancelled",
                "cancelado",
                "iptal edildi",
                "đã hủy",
                "ملغي",
                "abandonné",
                "заброшено",
                "annulé",
            ][..],
        ),
    ] {
        if names.iter().any(|name| value.contains(name)) {
            return Ok(state);
        }
    }
    Ok("unknown")
}

fn details(series: &Series, related: bool, request: &[u8]) -> Result<Value> {
    let doc = &series.doc;
    let url = series.link()?;
    let title = doc.require(DETAILS_TITLE)?.text()?;
    if title.is_empty() {
        return Err("parse_error");
    }
    let cover = doc
        .one("div.summary_image img")
        .and_then(|node| image_from(&node, &url));
    let mut work = manga_item(&series.id, &title, cover, &url, request);
    let authors: Vec<_> = text_list(doc, "div.author-content > a, div.manga-authors > a")?
        .into_iter()
        .filter(|value| !value.is_empty() && !updating(value))
        .collect();
    let artists: Vec<_> = text_list(doc, "div.artist-content > a")?
        .into_iter()
        .filter(|value| !value.is_empty() && !updating(value))
        .collect();
    work["authors"] = json!(authors);
    work["artists"] = json!(artists);
    work["status"] = json!(status(doc)?);
    let genres = genre_links(doc, "div.genres-content a[href]", &url)?;
    let mut tags = genres
        .iter()
        .map(|(_, name)| name.clone())
        .collect::<Vec<_>>();
    tags.extend(text_list(doc, "div.tags-content a")?);
    let mut alternate = None;
    for row in doc.all(".post-content_item")? {
        let label = row
            .one(".summary-heading")
            .map(|node| node.text())
            .transpose()?
            .unwrap_or_default()
            .to_lowercase();
        if let Some(content) = row.one(".summary-content") {
            let value = content.text()?;
            if !value.is_empty() {
                if label.contains("type") {
                    tags.push(value);
                } else if label.contains("alt") && value != title && !updating(&value) {
                    alternate = Some(value);
                }
            }
        }
    }
    let mut seen = BTreeSet::new();
    tags.retain(|tag| !tag.is_empty() && seen.insert(tag.to_lowercase()));
    work["tags"] = json!(tags);
    let description = doc.one("div.description-summary div.summary__content, div.summary_content div.post-content_item > h5 + div, div.summary_content div.manga-excerpt")
        .map(|node| {
            let paragraphs = text_list(&node, "p")?;
            if paragraphs.is_empty() { node.text() } else { Ok(paragraphs.join("\n\n")) }
        }).transpose()?.unwrap_or_default();
    if let Some(alternate) = alternate {
        work["alternateTitles"] = json!([alternate]);
    }
    work["description"] = json!(description);
    if related && !genres.is_empty() {
        // Related works are optional in the Koma detail contract.
        if let Ok(items) = related_manga(&series.id, genres, &url, request) {
            work["relatedManga"] = json!(items);
        }
    }
    Ok(work)
}

fn related_manga(id: &str, genres: Vec<(String, String)>, referer: &Url, request: &[u8]) -> Result<Vec<Value>> {
    let specific = genres
        .iter()
        .filter(|(slug, _)| {
            !matches!(
                slug.to_ascii_lowercase().as_str(),
                "manga" | "manhwa" | "manhua" | "webtoon"
            )
        })
        .collect::<Vec<_>>();
    let preferred = if specific.is_empty() {
        genres.iter().collect::<Vec<_>>()
    } else {
        specific
    };
    let mut form = filters::archive_form(0);
    form.append_pair("vars[orderby]", "rand")
        .append_pair("vars[sidebar]", "right")
        .append_pair("vars[post__not_in][0]", id);
    filters::add_genres(
        &mut form,
        0,
        preferred.into_iter().take(3).map(|(slug, _)| slug.as_str()),
    );
    parse_archive(&post(form, referer.as_str(), request)?, request)
}

// The real 5929 MangaPage and its referenced template.js bind btn-read-first
// ("观看最新章") to DOM index 0 and btn-read-last ("观看第一章") to the last row.
// Preserve that newest-first array, including unnumbered entries, independently
// of titles/dates/slug numbers. Only the complete single no-volume layout gains
// one group; nested or multiple sections keep manual access until their actual
// edition/volume relationship is established. See this source README.
fn verified_flat_catalogue(series: &Series, count: usize) -> Result<bool> {
    let doc = &series.doc;
    if !series.complete_document || count == 0 || post_id(doc).as_deref() != Some(series.id.as_str()) {
        return Ok(false);
    }
    if doc.one("body.single-wp-manga").is_none()
        || doc.all("ul.version-chap")?.len() != 1
        || doc.all("ul.main.version-chap.no-volumn")?.len() != 1
        || !doc.all("ul.version-chap ul")?.is_empty()
        || doc.all("ul.main.version-chap.no-volumn > li")?.len() != count
        || doc.all("#btn-read-first")?.len() != 1
        || doc.all("#btn-read-last")?.len() != 1
    {
        return Ok(false);
    }
    let newest = doc.require("#btn-read-first")?.text()?;
    let first = doc.require("#btn-read-last")?.text()?;
    Ok(newest.contains("最新") && first.contains("第一"))
}

fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(request)?;
    let id = work_id(&input["args"])?;
    let owner = format!("manga:{id}");
    let series = series(id, input["args"]["mangaUrl"].as_str(), request)?;
    let url = series.link()?;
    let items = series.doc.all("ul.version-chap > li")?;
    let mut verified = verified_flat_catalogue(&series, items.len())?;
    let canonical = series.canonical_path().ok();
    let mut rows = Vec::new();
    let mut seen = BTreeSet::new();
    let now = dates::now();
    for item in items {
        let Some(link) = item.one("a[href]") else {
            verified = false;
            continue;
        };
        let target = port::absolute(url.as_str(), &link.attr("href")?)?;
        if target.host_str() != Some("bakamh.com") {
            verified = false;
            continue;
        }
        let Some(slug) = target
            .path()
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .filter(|slug| path_segment(slug))
        else {
            verified = false;
            continue;
        };
        // Keep old IDs, but bind ordered rows to the current work's canonical
        // path. A foreign edition sharing the final segment cannot enter it.
        verified &= canonical.as_deref().is_some_and(|base| {
            target.scheme() == "https"
                && target.query().is_none() && target.fragment().is_none()
                && target.path().get(..base.len()).is_some_and(|prefix| prefix.eq_ignore_ascii_case(base))
                && target.path().get(base.len()..).is_some_and(|leaf| leaf.trim_end_matches('/') == slug)
        });
        if !seen.insert(slug.to_string()) {
            continue;
        }
        let title = link.text()?;
        let stamp = item
            .one("img:not(.thumb)")
            .and_then(|node| node.attr("alt").ok())
            .filter(|text| !text.trim().is_empty())
            .or_else(|| {
                item.one("span a")
                    .and_then(|node| node.attr("title").ok())
                    .filter(|text| !text.trim().is_empty())
            })
            .or_else(|| {
                item.one("span.chapter-release-date i")
                    .and_then(|node| node.text().ok())
            })
            .as_deref()
            .and_then(|text| dates::parse(text, now));
        rows.push((format!("chapter:{id}:{slug}"), title, stamp));
    }
    let mut chapters = GroupedChapters::new(payload_buf(), request).map_err(port::chapter_error)?;
    for (index, (chapter_id, title, stamp)) in rows.iter().enumerate() {
        let mut chapter = Chapter::new(chapter_id.as_bytes(), owner.as_bytes(), title.as_bytes());
        chapter.language = Some(b"zh");
        chapter.published_at = stamp.as_deref().map(str::as_bytes);
        let placement = if verified {
            ChapterPlacement::InGroup { group: b"bakamh-main", position: (rows.len() - index - 1) as u64 }
        } else {
            ChapterPlacement::ManualOnly
        };
        chapters.push(chapter, placement).map_err(port::chapter_error)?;
    }
    chapters
        .finish(ChapterPage::Complete)
        .map_err(port::chapter_error)
}

fn dispatch(operation: &str, request: &[u8]) -> Result<Value> {
    let input = port::input(request)?;
    let args = &input["args"];
    match operation {
        "get_listings" => Ok(port::listing_list(true, true, false)),
        "get_manga_list" => browse(args, request, false),
        "search" => browse(args, request, true),
        "get_filters" => filters::schema(args, request),
        "get_manga" => {
            let series = series(work_id(args)?, args["mangaUrl"].as_str(), request)?;
            view_count(&series.doc, &series.link()?, request);
            Ok(
                json!({"manga":details(&series, source_protocol::feature(request,b"source_related_manga_v1"), request)?}),
            )
        }
        "get_image_request" => reader::image_request(args, request),
        _ => Err("not_implemented"),
    }
}
fn write_catalogue_result(operation: &str, result: Result<Value>) -> u32 {
    match result {
        // The SDK maps HTTP 403 to this error before HTML parsing.
        Err("forbidden") => write_error(operation, "forbidden", "HTTP 403"),
        other => port::write_result(operation, other),
    }
}

fn run_search(r: &[u8]) -> u32 {
    write_catalogue_result("search", dispatch("search", r))
}
fn run_get_manga(r: &[u8]) -> u32 {
    port::write_result("get_manga", dispatch("get_manga", r))
}
fn run_get_pages(r: &[u8]) -> u32 {
    let result = port::input(r)
        .map_err(reader::PageError::from)
        .and_then(|input| reader::pages(&input["args"], r));
    match result {
        Ok(payload) => port::write_result("get_pages", Ok(payload)),
        Err(error) => match error.message.as_deref() {
            Some(message) if !message.is_empty() => write_error("get_pages", error.code, message),
            _ => port::write_failure("get_pages", error.code),
        },
    }
}
fn run_get_listings(r: &[u8]) -> u32 {
    port::write_result("get_listings", dispatch("get_listings", r))
}
fn run_get_manga_list(r: &[u8]) -> u32 {
    write_catalogue_result("get_manga_list", dispatch("get_manga_list", r))
}
fn run_get_filters(r: &[u8]) -> u32 {
    port::write_result("get_filters", dispatch("get_filters", r))
}
fn run_get_settings(_: &[u8]) -> u32 {
    port::write_failure("get_settings", "not_implemented")
}
fn run_get_home(_: &[u8]) -> u32 {
    port::write_failure("get_home", "not_implemented")
}
fn run_get_image_request(r: &[u8]) -> u32 {
    port::write_result("get_image_request", dispatch("get_image_request", r))
}
fn write_chapter_failure(code: &str) -> u32 {
    port::write_failure("get_chapters", code)
}
koma_source_sdk::koma_source_exports!("bakamh", typed_chapters, write_chapter_failure);
