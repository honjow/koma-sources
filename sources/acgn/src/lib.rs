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
    host,
    json_utils::request_page_number,
    source::{SourceCapabilities, SourceInfo},
    source_protocol as protocol,
};
use serde_json::{json, Value};
use url::Url;
mod dom;
use dom::Node;
#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;
const BASE: &str = "https://comic.acgn.cc";
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "cc.acgn.koma",
    name: "動漫戲說",
    version: "0.1.3",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "動漫戲說收錄連載漫畫、單行本與畫集，題材包含冒險、科幻、戀愛及推理。",
    content_rating: "safe",
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
    image_request: false,
    credentials: false,
};
koma_source_sdk::koma_source_buffers! {payload:1024*1024,http_out:3*1024*1024,body:2*1024*1024,http_req:16384,scratch:8192,}
koma_source_sdk::koma_source_helpers!();
koma_source_sdk::koma_source_exports!("acgn_source");
type Result<T> = core::result::Result<T, &'static str>;
fn absolute(base: &str, relative: &str) -> Result<String> {
    let u = Url::parse(base)
        .map_err(|_| "parse_error")?
        .join(relative)
        .map_err(|_| "parse_error")?;
    if u.scheme() != "https"
        || u.host_str().is_none()
        || !u.username().is_empty()
        || u.password().is_some()
    {
        return Err("parse_error");
    }
    Ok(u.to_string())
}
fn get(u: &str, referer: Option<&str>) -> Result<String> {
    if host::check_cancel() {
        return Err("cancelled");
    }
    let mut h = json!({});
    if let Some(r) = referer {
        h["Referer"] = json!(r)
    }
    let env = json!({"version":1,"method":"GET","url":u,"responseKind":"bodyText","timeoutMs":15000,"headers":h});
    let b = serde_json::to_vec(&env).map_err(|_| "internal_error")?;
    let n = host::http_request(&b, http_out()).map_err(|_| "network_error")?;
    let n = protocol::body(&http_out()[..n], body_buf())?;
    core::str::from_utf8(&body_buf()[..n])
        .map(String::from)
        .map_err(|_| "parse_error")
}
fn doc(u: &str) -> Result<Node> {
    Node::parse(get(u, None)?.as_bytes())
}
fn attr(n: &Node, k: &str) -> Result<String> {
    n.attr(k)?.filter(|x| !x.is_empty()).ok_or("parse_error")
}
fn slug(s: &str) -> Result<&str> {
    if s.is_empty()
        || s.len() > 256
        || !s
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
    {
        Err("invalid_request")
    } else {
        Ok(s)
    }
}
fn manga_id(v: &Value) -> Result<&str> {
    slug(
        v["mangaId"]
            .as_str()
            .and_then(|s| s.strip_prefix("acgn:"))
            .ok_or("invalid_request")?,
    )
}
fn slug_from_url(value: &str, prefix: &str) -> Result<String> {
    let u = Url::parse(&absolute(BASE, value)?).map_err(|_| "parse_error")?;
    if u.host_str() != Some("comic.acgn.cc") {
        return Err("invalid_request");
    }
    let v = u
        .path()
        .strip_prefix(prefix)
        .and_then(|s| s.strip_suffix(".htm"))
        .ok_or("parse_error")?;
    Ok(slug(v)?.to_string())
}
fn manga_url(id: &str) -> String {
    format!("{BASE}/manhua-{id}.htm")
}
fn chapter_url(id: &str) -> String {
    format!("{BASE}/view-{id}.htm")
}
fn card(id: &str, title: &str, cover: Option<String>) -> Value {
    let mut v =
        json!({"id":format!("acgn:{id}"),"title":title,"url":manga_url(id),"contentRating":"safe"});
    if let Some(c) = cover {
        v["cover"] = json!({"kind":"url","url":c})
    }
    v
}
fn result(items: Vec<Value>, next: Option<String>) -> Value {
    json!({"items":items,"page":{"hasMore":next.is_some(),"nextCursor":next}})
}
fn rows(d: &Node, selector: &str, a: &str, img: &str) -> Result<Vec<Value>> {
    let mut items = Vec::new();
    for n in d.all(selector)? {
        let l = n.require(a)?;
        let id = slug_from_url(&attr(&l, "href")?, "/manhua-")?;
        let title = l
            .attr("title")?
            .filter(|x| !x.is_empty())
            .unwrap_or(l.text()?);
        let cover = n
            .one(img)
            .map(|x| x.attr("src"))
            .transpose()?
            .flatten()
            .map(|s| absolute(BASE, &s))
            .transpose()?;
        items.push(card(&id, &title, cover))
    }
    Ok(items)
}
fn latest() -> Result<Value> {
    let d = doc(BASE)?;
    // An unexpected HTML document must not masquerade as an empty catalogue.
    if d.one(".tabs-panel").is_none() {
        return Err("parse_error");
    }
    Ok(result(rows(&d, ".tabs-panel li", "div > a", "img")?, None))
}
fn category_path(path: &str) -> bool {
    path.strip_prefix("/cate-")
        .and_then(|id| id.strip_suffix(".htm"))
        .is_some_and(|id| !id.is_empty() && id.bytes().all(|c| c.is_ascii_digit()))
}

fn category_options() -> Result<Vec<(String, String)>> {
    let d = doc(BASE)?;
    let mut known = BTreeSet::new();
    let mut categories = Vec::new();
    for n in d.all("#hd a[href^='/cate-']")? {
        let href = attr(&n, "href")?;
        let name = attr(&n, "title")?.trim_end_matches("漫畫").to_string();
        if category_path(&href) && known.insert(href.clone()) {
            categories.push((href, name));
        }
    }
    Ok(categories)
}

fn listings() -> Result<Value> {
    let mut items = vec![json!({"id":"latest","name":"最新漫畫","kind":"latest","supportsPaging":false,"supportsFilters":false})];
    for (id, name) in category_options()? {
        items.push(json!({"id":id,"name":name,"supportsPaging":true,"supportsFilters":false}));
    }
    Ok(json!({"listings":items}))
}

fn catalog_page(path: &str, page: usize) -> Result<Value> {
    let is_letter = path.strip_prefix("pinyin-")
        .and_then(|letter| letter.strip_suffix(".htm"))
        .is_some_and(|letter| letter == "0-9" || (letter.len() == 1 && letter.as_bytes()[0].is_ascii_lowercase()));
    if !category_path(path) && !is_letter {
        return Err("invalid_request");
    }
    let mut u = Url::parse(&absolute(BASE, path)?).map_err(|_| "invalid_request")?;
    u.query_pairs_mut().append_pair("page", &page.to_string());
    let d = doc(u.as_str())?;
    d.require("#display")?;
    let more = d.one(".pager a[rel=next]").is_some();
    Ok(result(
        rows(&d, "#display .content_block", ".list_r h2 a", "img.thumb")?,
        page.checked_add(1).filter(|_| more).map(|next| next.to_string()),
    ))
}

fn filters(req: &[u8], args: &Value) -> Result<Value> {
    let mut categories = vec![json!({"value":"","label":"全部"})];
    if args["fetchFilterData"] == true {
        for (path, name) in category_options()? {
            categories.push(json!({"value":path,"label":name}));
        }
    }
    let mut letters = vec![json!({"value":"","label":"全部"})];
    for c in b'A'..=b'Z' {
        let x = String::from(c as char);
        letters.push(json!({"value":format!("pinyin-{}.htm",x.to_ascii_lowercase()),"label":x}))
    }
    letters.push(json!({"value":"pinyin-0-9.htm","label":"0-9"}));
    let mut rows = Vec::new();
    let modern = protocol::feature(req, b"source_filter_scope_header_v1")
        && protocol::feature(req, b"source_filter_search_without_query_v1");
    if modern {
        rows.push(json!({"type":"header","name":"搜尋文字時忽略篩選；分類優先於字母"}))
    }
    let mut category = json!({"id":"category","name":"分類","type":"select","default":""});
    if modern {
        category["scope"] = json!("search");
    }
    if args["fetchFilterData"] == true {
        category["options"] = json!(categories)
    }
    rows.push(category);
    let mut letter =
        json!({"id":"letter","name":"字母","type":"select","default":"","options":letters});
    if modern {
        letter["scope"] = json!("search");
    }
    rows.push(letter);
    Ok(json!({"supportsFilterFetching":true,"useSearchForFilters":modern,"filters":rows}))
}
fn between<'a>(s: &'a str, start: &str, end: &str) -> Result<&'a str> {
    s.split_once(start)
        .and_then(|(_, x)| x.split_once(end))
        .map(|(x, _)| x)
        .ok_or("parse_error")
}
fn cse_config(s: &str, key: &str) -> Result<String> {
    let marker = format!("\"{key}\":");
    let x = s.split_once(&marker).ok_or("parse_error")?.1.trim_start();
    let val = if x.starts_with('"') {
        format!("\"{}\"", between(x, "\"", "\"")?)
    } else if x.starts_with('[') {
        format!("[{}]", between(x, "[", "]")?)
    } else {
        return Err("parse_error");
    };
    let v: Value = serde_json::from_str(&val).map_err(|_| "parse_error")?;
    if let Some(s) = v.as_str() {
        Ok(s.to_string())
    } else {
        Ok(v.as_array()
            .ok_or("parse_error")?
            .iter()
            .map(|x| x.as_str().unwrap_or(""))
            .collect::<Vec<_>>()
            .join(","))
    }
}
fn search(req: &[u8], v: &Value) -> Result<Value> {
    let q = v["query"].as_str().unwrap_or("").trim();
    let page = request_page_number(req);
    if q.starts_with("https://") || q.starts_with("http://") {
        let u = Url::parse(q).map_err(|_| "invalid_request")?;
        if u.host_str() != Some("comic.acgn.cc")
            || u.scheme() != "https"
            || !u.username().is_empty()
            || u.password().is_some()
            || u.port().is_some()
        {
            return Err("not_implemented");
        }
        let id = if u.path().starts_with("/manhua-") {
            slug_from_url(q, "/manhua-")?
        } else if u.path().starts_with("/view-") {
            let chapter_id = slug_from_url(q, "/view-")?;
            if !chapter_id.bytes().all(|c| c.is_ascii_digit()) {
                return Err("invalid_request");
            }
            let chapter = doc(&chapter_url(&chapter_id))?;
            slug_from_url(
                &attr(&chapter.require("#breadcrumb a[href*='/manhua-']")?, "href")?,
                "/manhua-",
            )?
        } else {
            return Err("not_implemented");
        };
        let d = detail(&json!({"mangaId":format!("acgn:{id}")}), req)?;
        return Ok(result(vec![d["manga"].clone()], None));
    }
    if q.is_empty() {
        let f = &v["filters"];
        let p = f["category"]
            .as_str()
            .filter(|x| !x.is_empty())
            .or_else(|| f["letter"].as_str().filter(|x| !x.is_empty()));
        return match p {
            Some(path) => catalog_page(path, page),
            None => latest(),
        };
    }
    let mut pg = Url::parse(BASE).unwrap();
    pg.set_path("searchcse");
    pg.query_pairs_mut().append_pair("q", q);
    let page_url = pg.to_string();
    let js = get(
        "https://cse.google.com/cse.js?cx=008124722090293425135:k7bx4juliog",
        Some(&page_url),
    )?;
    let tok = cse_config(&js, "cse_token")?;
    let ver = cse_config(&js, "cselibVersion")?;
    let exp = cse_config(&js, "exp")?;
    let mut start = v["page"]["cursor"]
        .as_str()
        .and_then(|x| x.parse::<usize>().ok())
        .unwrap_or(0);
    for _ in 0..10 {
        let mut u = Url::parse("https://cse.google.com/cse/element/v1").unwrap();
        {
            let mut p = u.query_pairs_mut();
            p.append_pair("rsz", "filtered_cse")
                .append_pair("num", "20")
                .append_pair("hl", "zh-TW")
                .append_pair("source", "gcsc");
            if start > 0 {
                p.append_pair("start", &start.to_string());
            }
            p.append_pair("cselibv", &ver)
                .append_pair("cx", "008124722090293425135:k7bx4juliog")
                .append_pair("q", q)
                .append_pair("safe", "off")
                .append_pair("cse_tok", &tok)
                .append_pair("sort", "")
                .append_pair("exp", &exp)
                .append_pair("callback", "google.search.cse.api1")
                .append_pair("rurl", &page_url);
        }
        let body = get(u.as_str(), Some(&page_url))?;
        let raw = body
            .split_once("google.search.cse.api1(")
            .map(|(_, x)| x)
            .and_then(|x| {
                x.trim_end()
                    .strip_suffix(");")
                    .or_else(|| x.trim_end().strip_suffix(')'))
            })
            .ok_or("parse_error")?;
        let data: Value = serde_json::from_str(raw).map_err(|_| "parse_error")?;
        if data.get("error").is_some() {
            return Err("network_error");
        }
        let mut items = Vec::new();
        let mut known = BTreeSet::new();
        for i in data["results"].as_array().into_iter().flatten() {
            let Some(u) = i["unescapedUrl"].as_str() else {
                continue;
            };
            let Ok(id) = slug_from_url(u, "/manhua-") else {
                continue;
            };
            let Some(t) = i["richSnippet"]["metatags"]["ogTitle"]
                .as_str()
                .and_then(|x| x.split_once('《'))
                .and_then(|(_, x)| x.split_once('》'))
                .map(|(x, _)| x)
            else {
                continue;
            };
            if known.insert(id.clone()) {
                items.push(card(
                    &id,
                    t,
                    i["richSnippet"]["metatags"]["ogImage"]
                        .as_str()
                        .map(String::from),
                ))
            }
        }
        let next = data["cursor"]["pages"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|x| x["start"].as_str()?.parse::<usize>().ok())
            .find(|x| *x > start);
        if !items.is_empty() || next.is_none() {
            return Ok(result(items, next.map(|x| x.to_string())));
        }
        start = next.unwrap()
    }
    Err("result_too_large")
}
fn detail(v: &Value, req: &[u8]) -> Result<Value> {
    let id = manga_id(v)?;
    let route = manga_url(id);
    let d = doc(&route)?;
    let title = d.require(".list_navbox h3 a[href*='/manhua-']")?.text()?;
    let cover = d
        .one(".gameshows img")
        .map(|x| x.attr("src"))
        .transpose()?
        .flatten()
        .map(|s| absolute(&route, &s))
        .transpose()?;
    let mut m = card(id, &title, cover);
    let field = |label: &str| -> Result<Option<Node>> {
        for n in d.all(".load01_r li.mss")? {
            if n.text()?.starts_with(label) {
                return Ok(n.one("span"));
            }
        }
        Ok(None)
    };
    if let Some(a) = field("漫畫作者")? {
        let mut authors = Vec::new();
        for n in a.all("a")? {
            let name = n.text()?;
            if !name.is_empty() {
                authors.push(name);
            }
        }
        if !authors.is_empty() {
            m["authors"] = json!(authors);
        }
    }
    if let Some(s) = field("漫畫狀態")? {
        m["status"] = json!(match s.text()?.as_str() {
            "連載中" => "ongoing",
            "已完結" => "completed",
            _ => "unknown",
        })
    }
    if let Some(desc) = d.one(".gameshows dd:last-of-type") {
        let mut text = desc.whole()?;
        for script in desc.all("script, style")? {
            text = text.replace(&script.whole()?, "")
        }
        let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        let text = text
            .trim_start_matches('　')
            .strip_prefix(&format!("{title}漫畫"))
            .unwrap_or(&text)
            .trim_start_matches([' ', '，']);
        if !text.is_empty() {
            m["description"] = json!(text)
        }
    }
    let mut tags = Vec::new();
    for label in ["漫畫類型", "漫畫分類", "漫畫標籤"] {
        if let Some(field) = field(label)? {
            let values = if label == "漫畫分類" {
                field
                    .all("a")?
                    .iter()
                    .map(Node::text)
                    .collect::<Result<Vec<_>>>()?
            } else {
                vec![field.text()?]
            };
            for value in values {
                for x in value.split([',', '，']) {
                    let x = x.trim();
                    if !x.is_empty() && !tags.iter().any(|v: &String| v == x) {
                        tags.push(x.to_string());
                    }
                }
            }
        }
    }
    m["tags"] = json!(tags);
    if protocol::feature(req, b"source_related_manga_v1") {
        let mut related = Vec::new();
        for n in d.all(".danji li a[href*='/manhua-']")? {
            let target = slug_from_url(&attr(&n, "href")?, "/manhua-")?;
            let title = attr(&n, "title")?;
            let cover = n
                .one("img")
                .map(|x| x.attr("src"))
                .transpose()?
                .flatten()
                .map(|s| absolute(&route, &s))
                .transpose()?;
            related.push(card(&target, &title, cover))
        }
        m["relatedManga"] = json!(related)
    }
    m["links"] = json!([{"kind":"source","url":route}]);
    Ok(json!({"manga":m}))
}
// Website dates use Taipei time; only the most recently linked chapter has a date.
fn site_date(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let year = s.get(0..4)?.parse::<i64>().ok()?;
    let m = s.get(5..7)?.parse::<i64>().ok()?;
    let d = s.get(8..10)?.parse::<i64>().ok()?;
    if !(1..=12).contains(&m) {
        return None;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let last = match m {
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    if d < 1 || d > last {
        return None;
    }
    let y = year - i64::from(m <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = m + if m > 2 { -3 } else { 9 };
    let days = era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + (153 * mp + 2) / 5 + d - 1 - 719468;
    Some((days * 86400 - 8 * 3600) * 1000)
}
fn chapters(v: &Value) -> Result<Value> {
    let id = manga_id(v)?;
    let d = doc(&manga_url(id))?;
    let mut items = Vec::new();
    for n in d.all("#comic_chapter li a")? {
        let cid = slug_from_url(&attr(&n, "href")?, "/view-")?;
        if !cid.bytes().all(|c| c.is_ascii_digit()) {
            return Err("parse_error");
        }
        items.push((cid.parse::<u64>().map_err(|_| "parse_error")?, n.text()?))
    }
    items.sort_by(|a, b| b.0.cmp(&a.0));
    let mut latest = None;
    for field in d.all(".load01_r li.mss")? {
        let text = field.text()?;
        if text.starts_with("最新收錄") {
            if let Some(a) = field.one("a") {
                let cid = slug_from_url(&attr(&a, "href")?, "/view-")?
                    .parse::<u64>()
                    .map_err(|_| "parse_error")?;
                let ts = text
                    .split_once('[')
                    .and_then(|(_, t)| t.split_once(']'))
                    .and_then(|(date, _)| site_date(date));
                latest = Some((cid, ts));
            }
            break;
        }
    }
    let chapters=items.iter().enumerate().map(|(i,(cid,title))|{let mut c=json!({"id":format!("acgn-ch:{id}:{cid}"),"mangaId":format!("acgn:{id}"),"title":title,"index":i,"url":chapter_url(&cid.to_string())});if let Some((latest_id,Some(ts)))=latest{if *cid==latest_id{c["dateUpload"]=json!(ts)}}c}).collect::<Vec<_>>();
    Ok(json!({"chapters":chapters}))
}
fn pages(v: &Value) -> Result<Value> {
    let (mid, cid) = v["chapterId"]
        .as_str()
        .and_then(|x| x.strip_prefix("acgn-ch:"))
        .and_then(|x| x.split_once(':'))
        .ok_or("invalid_request")?;
    slug(mid)?;
    slug(cid)?;
    if !cid.bytes().all(|c| c.is_ascii_digit()) {
        return Err("invalid_request");
    }
    if v.get("mangaId")
        .is_some_and(|x| x.as_str() != Some(&format!("acgn:{mid}")))
    {
        return Err("invalid_request");
    }
    let url = chapter_url(cid);
    let d = doc(&url)?;
    let owner = d.require("#breadcrumb a[href*='/manhua-']")?;
    if slug_from_url(&attr(&owner, "href")?, "/manhua-")? != mid {
        return Err("invalid_request");
    }
    let mut out = Vec::new();
    for n in d.all("#pic_list .pic[_src]")? {
        let u = absolute(&url, &attr(&n, "_src")?)?;
        out.push(json!({"id":format!("acgn-page:{mid}:{cid}:{}",out.len()),"index":out.len(),"image":{"kind":"url","url":u}}))
    }
    if out.is_empty() {
        return Err("chapter_unavailable");
    }
    Ok(json!({"pages":out}))
}
fn run(op: &str, r: &[u8]) -> u32 {
    let result = (|| {
        protocol::request(r)?;
        let a = protocol::direct(r, b"args")?.unwrap_or(r);
        let v: Value = serde_json::from_slice(a).map_err(|_| "invalid_request")?;
        let networking = matches!(
            op,
            "get_listings" | "get_manga_list" | "search" | "get_manga" | "get_chapters" | "get_pages"
        ) || (op == "get_filters" && v["fetchFilterData"] == true);
        if networking && !protocol::feature(r, b"source_request_limits_v1") {
            return Err("host_feature_required");
        }
        match op {
            "get_listings" => listings(),
            "get_manga_list" => match v["listingId"].as_str() {
                Some(id) if category_path(id) => catalog_page(id, request_page_number(r)),
                _ => search(r, &v),
            },
            "search" => search(r, &v),
            "get_filters" => filters(r, &v),
            "get_manga" => detail(&v, r),
            "get_chapters" => chapters(&v),
            "get_pages" => pages(&v),
            _ => Err("not_implemented"),
        }
    })();
    match result {
        Ok(v) => match serde_json::to_vec(&v) {
            Ok(b) if b.len() <= payload_buf().len() => {
                payload_buf()[..b.len()].copy_from_slice(&b);
                write_success_payload(op, b.len())
            }
            _ => write_error(op, "result_too_large", ""),
        },
        Err(e) => write_error(
            op,
            e,
            if e == "host_feature_required" {
                "請更新 Koma 以使用此書源"
            } else {
                "讀取動漫戲說失敗"
            },
        ),
    }
}
fn run_search(r: &[u8]) -> u32 {
    run("search", r)
}
fn run_get_manga(r: &[u8]) -> u32 {
    run("get_manga", r)
}
fn run_get_chapters(r: &[u8]) -> u32 {
    run("get_chapters", r)
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
