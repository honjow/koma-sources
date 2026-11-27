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
    host,
    source::{SourceCapabilities, SourceInfo},
    source_protocol,
};
use serde_json::{json, Value};
use url::Url;
#[path = "../../port_support.rs"]
#[allow(dead_code)]
mod port;
#[path = "../../port_catalog.rs"]
mod port_catalog;
use port::{Node, Result};
use port_catalog::Labels;
#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;
const SITE: &str = "https://manga.nicovideo.jp/";
const API: &str = "https://api.nicomanga.jp/api/v1/app/manga/";
const METADATA: &[u8] = include_bytes!("../metadata.json");
const SOURCE_INFO: SourceInfo = SourceInfo {id:"com.nicovideoseiga.ja.koma",name:"ニコニコ漫画",version:env!("CARGO_PKG_VERSION"),api_version:"0.2",language:"ja",description:"出版社の公式連載とユーザー投稿が集まるニコニコ漫画。『ワンパンマン』『蜘蛛ですが、なにか？』などの公開話を読めます。",author:"Koma source contributors",content_rating:"mixed"};
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
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
    image_request: true,
};
const CATEGORIES: &[(&str, &str)] = &[
    ("", "Any"),
    ("shonen", "Shonen"),
    ("seinen", "Seinen"),
    ("shojo", "Shojo"),
    ("josei", "Josei"),
    ("yonkoma", "Four-panel comics"),
    ("other", "Other"),
    ("fan", "Fan comics"),
];
const SORT: &[(&str, &str)] = &[
    ("score", "Relevance"),
    ("manga_updated", "Recently updated"),
    ("manga_updated_a", "Oldest update"),
    ("manga_created", "Newest series"),
    ("manga_created_a", "Oldest series"),
    ("manga_view", "Most views"),
    ("manga_view_a", "Fewest views"),
    ("comment_created", "Newest comment"),
    ("comment_created_a", "Oldest comment"),
    ("comment_count", "Most comments"),
    ("comment_count_a", "Fewest comments"),
];
koma_source_sdk::koma_source_buffers! { payload: 768 * 1024, http_out: 8 * 1024 * 1024, body: 4 * 1024 * 1024, http_req: 4096, scratch: 8192, }
koma_source_sdk::koma_source_helpers!();
fn number(s: &str) -> Result<&str> {
    if !s.is_empty()
        && s.len() <= 12
        && s.bytes().all(|b| b.is_ascii_digit())
        && !s.starts_with('0')
    {
        Ok(s)
    } else {
        Err("invalid_request")
    }
}
fn work(args: &Value) -> Result<&str> {
    number(
        args["mangaId"]
            .as_str()
            .and_then(|s| s.strip_prefix("manga:"))
            .ok_or("invalid_request")?,
    )
}
fn chapter(id: &str) -> Result<(&str, &str)> {
    let (w, e) = id
        .strip_prefix("chapter:")
        .and_then(|s| s.split_once(':'))
        .ok_or("invalid_request")?;
    Ok((number(w)?, number(e)?))
}
fn field<'a>(v: &'a Value, k: &str) -> Result<&'a str> {
    v[k].as_str().filter(|s| !s.is_empty()).ok_or("parse_error")
}
fn media(raw: &str) -> Result<Url> {
    let u = port::absolute(SITE, raw)?;
    if u.scheme() != "https"
        || !matches!(
            u.host_str(),
            Some("deliver.cdn.nicomanga.jp" | "drm.cdn.nicomanga.jp")
        )
        || u.port().is_some()
        || u.fragment().is_some()
    {
        return Err("parse_error");
    }
    Ok(u)
}
fn api(path: &str, query: &[(&str, &str)], reading: bool) -> Result<Value> {
    let mut url = port::absolute(API, path)?;
    url.query_pairs_mut().extend_pairs(query);
    // Read status and provider error code before the generic HTTP decoder: an
    // expired/publication-finished episode is chapter_locked, not a network error.
    let req=serde_json::to_vec(&json!({"version":1,"method":"GET","url":url.as_str(),"headers":{"Accept":"application/json","Referer":SITE},"responseKind":"bodyText","timeoutMs":15000})).map_err(|_|"internal_error")?;
    if host::check_cancel() {
        return Err("cancelled");
    }
    let len = host::http_request(&req, http_out()).map_err(|_| "network_error")?;
    let env: Value = serde_json::from_slice(&http_out()[..len]).map_err(|_| "network_error")?;
    let parsed = env["bodyText"]
        .as_str()
        .and_then(|b| serde_json::from_str::<Value>(b).ok());
    if reading
        && env["ok"].as_bool() == Some(true)
        && matches!(env["status"].as_u64(), Some(401 | 402 | 403))
        && parsed.as_ref().is_some_and(|v| {
            matches!(
                v["meta"]["error_code"].as_str(),
                Some(
                    "PUBLICATION_FINISHED"
                        | "FORBIDDEN"
                        | "AUTH_REQUIRED"
                        | "UNAUTHORIZED"
                        | "NOT_PURCHASED"
                        | "PAYMENT_REQUIRED"
                )
            )
        })
    {
        return Err("chapter_locked");
    }
    let len = source_protocol::body(&http_out()[..len], body_buf())?;
    let root: Value = serde_json::from_slice(&body_buf()[..len]).map_err(|_| "parse_error")?;
    match root["meta"]["status"].as_u64() {
        Some(200) => Ok(root["data"].clone()),
        Some(401 | 402 | 403) if reading => Err("chapter_locked"),
        Some(404) => Err("not_found"),
        _ => Err("upstream_error"),
    }
}
fn card(row: &Value) -> Result<Value> {
    let id = row["id"].as_u64().ok_or("parse_error")?;
    let m = &row["meta"];
    let cover = media(field(m, "square_image_url").or_else(|_| field(m, "icon_url"))?)?;
    Ok(
        json!({"id":format!("manga:{id}"),"title":field(m,"title")?,"url":format!("{SITE}comic/{id}"),"language":"ja","contentRating":"mixed","cover":{"kind":"url","url":cover.as_str(),"headers":{"Referer":SITE}}}),
    )
}
fn cursor(args: &Value) -> Result<usize> {
    match &args["page"]["cursor"] {
        Value::Null => Ok(1),
        Value::String(s) => s
            .parse::<usize>()
            .ok()
            .filter(|n| (1..=100_000).contains(n) && n.to_string() == *s)
            .ok_or("invalid_request"),
        _ => Err("invalid_request"),
    }
}
fn selection<'a>(
    args: &'a Value,
    id: &str,
    options: &[(&str, &str)],
    default: &'a str,
) -> Result<&'a str> {
    match &args["filters"][id] {
        Value::Null => Ok(default),
        Value::String(s) if options.iter().any(|(key, _)| key == s) => Ok(s),
        _ => Err("invalid_request"),
    }
}
fn list(args: &Value, search: bool) -> Result<Value> {
    let page = cursor(args)?;
    if search {
        // app API's guessed search routes return empty objects and silently ignore
        // keyword/q. Use the site's real search form, not unfiltered app listings.
        let query = args["query"].as_str().unwrap_or("").trim();
        if query.len() > 1024 || query.chars().any(char::is_control) {
            return Err("invalid_request");
        }
        let sort = selection(args, "sort", SORT, "score")?;
        let mut url = port::absolute(SITE, "search")?;
        url.query_pairs_mut().extend_pairs([
            ("q", query),
            ("sort", sort),
            ("page", &page.to_string()),
        ]);
        let doc = Node::parse(&port::get(url.as_str())?)?;
        doc.require("#sort")?;
        let mut items = Vec::new();
        let mut seen = BTreeSet::new();
        for row in doc.all(".search_result__item")? {
            let a = row.require(".search_result__item__info--title a")?;
            let url = port::absolute(SITE, &a.attr("href")?)?;
            let id = number(url.path().strip_prefix("/comic/").ok_or("parse_error")?)?;
            if seen.insert(id.to_string()) {
                let img = row.require("img[data-original]")?;
                let cover = media(&img.attr("data-original")?)?;
                items.push(json!({"id":format!("manga:{id}"),"title":a.text()?,"url":format!("{SITE}comic/{id}"),"language":"ja","cover":{"kind":"url","url":cover.as_str(),"headers":{"Referer":SITE}}}));
            }
        }
        let next = doc.all("a[href]")?.iter().any(|a| {
            a.attr("href")
                .ok()
                .and_then(|s| port::absolute(SITE, &s).ok())
                .is_some_and(|u| {
                    u.path() == "/search"
                        && u.query_pairs()
                            .any(|(k, v)| k == "page" && v == (page + 1).to_string())
                })
        });
        return Ok(port::items(items, next.then_some(page + 1)));
    }
    let category = selection(args, "category", CATEGORIES, "")?;
    let listing = args["listingId"].as_str().unwrap_or("popular");
    let listing = listing.strip_prefix("listing:").unwrap_or(listing);
    let path = match listing {
        "popular" => "contents/ranking",
        "latest" => "contents",
        _ => return Err("invalid_request"),
    };
    let offset = ((page - 1) * 20).to_string();
    let root = api(
        path,
        &[("limit", "20"), ("offset", &offset), ("category", category)],
        false,
    )?;
    let rows = root["result"].as_array().ok_or("parse_error")?;
    let next = root["extra"]["has_next"].as_bool().ok_or("parse_error")?;
    if next && rows.is_empty() {
        return Err("catalog_changed");
    }
    Ok(port::items(
        rows.iter().map(card).collect::<Result<Vec<_>>>()?,
        next.then_some(page + 1),
    ))
}
fn detail(args: &Value) -> Result<Value> {
    let id = work(args)?;
    let root = api(&format!("contents/{id}"), &[], false)?;
    let row = &root["result"];
    if row["id"].as_u64().map(|n| n.to_string()).as_deref() != Some(id) {
        return Err("parse_error");
    }
    let mut m = card(row)?;
    let r = &row["meta"];
    let desc = r["description"].as_str().unwrap_or("");
    m["description"] = json!(Node::parse(desc)?.text()?);
    m["authors"] = json!(r["authors"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|a| a["name"].as_str())
        .collect::<Vec<_>>());
    m["tags"] = json!([match r["category"].as_str() {
        Some("shonen") => "少年マンガ",
        Some("seinen") => "青年マンガ",
        Some("shojo") => "少女マンガ",
        Some("josei") => "女性マンガ",
        Some("yonkoma") => "4コママンガ",
        Some("fan") => "ファンコミック",
        _ => "その他マンガ",
    }]);
    m["status"] = json!(match r["serial_status"].as_str() {
        Some("serial") => "ongoing",
        Some("concluded") => "completed",
        _ => "unknown",
    });
    Ok(json!({"manga":m}))
}
fn locked(row: &Value) -> bool {
    row["meta"]["is_playable"].as_bool() != Some(true)
        || row["own_status"]["is_publication_finished"].as_bool() == Some(true)
        || row["own_status"]["sell_status"]
            .as_str()
            .is_some_and(|s| s != "free")
        || row["meta"]["price"].as_u64().unwrap_or(0) > 0
}
fn run_get_chapters(req: &[u8]) -> ChapterResult<'static, (&'static str, String)> {
    let go = || {
        let v = port::input(req)?;
        let args = &v["args"];
        if !args["page"]["cursor"].is_null() {
            return Err("invalid_request");
        }
        let work = work(args)?;
        let root = api(&format!("contents/{work}/episodes"), &[], false)?;
        let rows = root["result"].as_array().ok_or("parse_error")?;
        let owner = format!("manga:{work}");
        let mut seen = BTreeSet::new();
        let mut out = GroupedChapters::new(payload_buf(), req).map_err(port::chapter_error)?;
        for (position, row) in rows.iter().enumerate() {
            if row["meta"]["content_id"]
                .as_u64()
                .map(|n| n.to_string())
                .as_deref()
                != Some(work)
            {
                return Err("parse_error");
            }
            let id = row["id"].as_u64().ok_or("parse_error")?;
            if !seen.insert(id) {
                return Err("catalog_changed");
            }
            let id = format!("chapter:{work}:{id}");
            let is_locked = locked(row);
            let title = format!(
                "{}{}",
                if is_locked { "🔒 " } else { "" },
                field(&row["meta"], "title")?
            );
            let url = field(&row["meta"], "share_url")?;
            let mut ch = Chapter::new(id.as_bytes(), owner.as_bytes(), title.as_bytes());
            ch.url = Some(url.as_bytes());
            ch.language = Some(b"ja");
            ch.locked = Some(is_locked);
            ch.page_count = row["meta"]["counter"]["frame"]
                .as_u64()
                .and_then(|n| usize::try_from(n).ok());
            // API catalogue is oldest-first. Keep unavailable episodes visible, but do
            // not auto-continue into a paid, login-only or withdrawn episode.
            out.push(
                ch,
                if is_locked {
                    ChapterPlacement::ManualOnly
                } else {
                    ChapterPlacement::InGroup {
                        group: b"episodes",
                        position: position as u64,
                    }
                },
            )
            .map_err(port::chapter_error)?;
        }
        out.finish(ChapterPage::Complete)
            .map_err(port::chapter_error)
    };
    go().map_err(|c| (c, port_catalog::failure(METADATA, req, c)))
}
fn frames(work: &str, episode: &str) -> Result<Value> {
    let one = api(&format!("episodes/{episode}"), &[], true)?;
    let row = &one["result"];
    if row["meta"]["content_id"]
        .as_u64()
        .map(|n| n.to_string())
        .as_deref()
        != Some(work)
    {
        return Err("invalid_request");
    }
    if locked(row) {
        return Err("chapter_locked");
    }
    let root = api(&format!("episodes/{episode}/frames"), &[], true)?;
    if root["result"].as_array().is_none_or(|v| v.is_empty()) {
        return Err("chapter_locked");
    }
    Ok(root)
}
fn frame(row: &Value) -> Result<(Url, Option<&str>)> {
    let m = &row["meta"];
    let url = media(field(m, "source_url")?)?;
    let key = match &m["drm_hash"] {
        Value::Null => None,
        Value::String(s) if s.is_empty() => None,
        Value::String(s)
            if s.len() >= 16 && s.as_bytes()[..16].iter().all(u8::is_ascii_hexdigit) =>
        {
            Some(&s[..16])
        }
        _ => return Err("unsupported_scramble"),
    };
    Ok((url, key))
}
fn pages(args: &Value) -> Result<Value> {
    let id = field(args, "chapterId")?;
    let (w, e) = chapter(id)?;
    if !args["mangaId"].is_null() && work(args)? != w {
        return Err("invalid_request");
    }
    let root = frames(w, e)?;
    let rows = root["result"].as_array().ok_or("parse_error")?;
    let pages=rows.iter().enumerate().map(|(i,row)|{let(url,_)=frame(row)?;Ok(json!({"id":format!("nico-page:{id}:{i}"),"index":i,"image":{"kind":"url","url":url.as_str(),"headers":{"Referer":SITE}}}))}).collect::<Result<Vec<_>>>()?;
    Ok(json!({"pages":pages}))
}
fn image_request(args: &Value, req: &[u8]) -> Result<Value> {
    let original = media(field(args, "url")?)?;
    let Some(page) = args["pageId"]
        .as_str()
        .and_then(|s| s.strip_prefix("nico-page:"))
    else {
        if original.host_str() == Some("drm.cdn.nicomanga.jp") {
            return Err("invalid_request");
        }
        return Ok(json!({"imageRequest":{"url":original.as_str(),"headers":{"Referer":SITE}}}));
    };
    let (id, index) = page.rsplit_once(':').ok_or("invalid_request")?;
    let index = index.parse::<usize>().map_err(|_| "invalid_request")?;
    let (w, e) = chapter(id)?;
    let root = frames(w, e)?;
    let (url, key) = frame(root["result"].get(index).ok_or("page_unavailable")?)?;
    if original.path() != url.path() || original.host_str() != url.host_str() {
        return Err("page_unavailable");
    }
    let mut out = json!({"imageRequest":{"url":url.as_str(),"headers":{"Referer":SITE}}});
    if let Some(key) = key {
        if !source_protocol::feature(req, b"source_required_image_decode_wasm_v1") {
            return Err("host_feature_required");
        }
        out["requiredImageDecode"] = json!({"kind":"wasm_image_v1","bytes":key});
    }
    Ok(out)
}
fn call(op: &str, req: &[u8], f: impl FnOnce(&Value) -> Result<Value>) -> u32 {
    let v = port::input(req).and_then(|v| f(&v["args"]));
    port_catalog::write_localized(METADATA, req, op, v)
}
fn run_get_manga_list(r: &[u8]) -> u32 {
    call("get_manga_list", r, |a| list(a, false))
}
fn run_search(r: &[u8]) -> u32 {
    call("search", r, |a| list(a, true))
}
fn run_get_manga(r: &[u8]) -> u32 {
    call("get_manga", r, detail)
}
fn run_get_pages(r: &[u8]) -> u32 {
    call("get_pages", r, pages)
}
fn run_get_image_request(r: &[u8]) -> u32 {
    call("get_image_request", r, |a| image_request(a, r))
}
fn run_get_listings(r: &[u8]) -> u32 {
    call("get_listings", r, |_| {
        let l = Labels::new(METADATA, r)?;
        Ok(
            json!({"listings":[{"id":"popular","name":l.text("listing.popular","Popular")?,"supportsPaging":true,"supportsFilters":true},{"id":"latest","name":l.text("listing.latest","Latest updates")?,"supportsPaging":true,"supportsFilters":true}]}),
        )
    })
}
fn run_get_filters(r: &[u8]) -> u32 {
    call("get_filters", r, |args| {
        let l = Labels::new(METADATA, r)?;
        let listing = args["listingId"].as_str().unwrap_or("");
        let catalogue = matches!(listing.trim_start_matches("listing:"), "popular" | "latest");
        let (id, opts, default) = if catalogue {
            ("category", CATEGORIES, "")
        } else {
            ("sort", SORT, "score")
        };
        let options = opts
            .iter()
            .map(|(v, t)| Ok(json!({"value":v,"label":l.text(&format!("option.{id}.{v}"),t)?})))
            .collect::<Result<Vec<_>>>()?;
        Ok(
            json!({"filters":[{"type":"select","id":id,"name":l.text(&format!("filter.{id}"),id)?,"default":default,"options":options}]}),
        )
    })
}
fn run_get_settings(r: &[u8]) -> u32 {
    call("get_settings", r, |_| Ok(json!({"settings":[]})))
}
fn run_get_home(r: &[u8]) -> u32 {
    call("get_home", r, |_| Err("not_implemented"))
}
fn write_chapter_failure((c, m): (&str, String)) -> u32 {
    write_error("get_chapters", c, &m)
}
koma_source_sdk::koma_source_exports!("nicovideoseiga-ja", typed_chapters, write_chapter_failure);
