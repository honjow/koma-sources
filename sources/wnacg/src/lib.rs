#![no_std]
extern crate alloc;
// Independent complete anonymous WNACG port, Apache-2.0. Fixed Keiyoushi
// de743aed846da53e00c45ab26ee0e77f7bb4355b; see NOTICE and README for adaptations.
use alloc::{
    collections::BTreeSet,
    format,
    string::{String, ToString},
    vec::Vec,
};
use koma_source_sdk::{
    host,
    json_utils::*,
    source::{SourceCapabilities, SourceInfo},
    source_protocol as protocol,
};
use regex::Regex;
use serde_json::{json, Value};
use url::Url;
mod dom;
use dom::Node;
mod session;
use session::Context;
#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;
const DEFAULTS: [&str; 2] = ["https://www.wnacg.buzz", "https://www.wnacg.site"];
const DOMAIN_LIST: &str = "https://stevenyomi.github.io/source-domains/wnacg.txt";
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/121.0";
const THUMBNAIL_PATH: &str = r"(?i)^/data/t/\d+/\d{2}/[a-z0-9_-]+\.(?:jpe?g|png|webp|gif)$";
const LOGIN_HINT: &str = "内容不存在或需要登录，请在原网页登录后重试。";
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.wnacg.koma",
    name: "紳士漫畫",
    version: "0.1.1",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "紳士漫畫公开相册与合集；部分内容需要登录。",
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
    settings: true,
    image_request: true,
    credentials: false,
};
koma_source_sdk::koma_source_buffers! {payload:2*1024*1024,http_out:6*1024*1024,body:4*1024*1024,http_req:32768,scratch:8192,}
koma_source_sdk::koma_source_helpers!();
type R<T> = Result<T, E>;
struct E {
    code: &'static str,
    message: String,
}
impl From<&'static str> for E {
    fn from(code: &'static str) -> Self {
        Self {
            code,
            message: match code {
                "unsupported" => "当前应用缺少此书源所需能力，请更新应用。",
                "not_found" => LOGIN_HINT,
                "auth_required" => "网站要求登录，请先打开原网页登录。",
                "forbidden" => "网站拒绝了本次请求，请打开原网页确认访问状态。",
                "network_error" => "网络传输未完成，请稍后重试。",
                "rate_limit" => "请求过于频繁，请稍后重试。",
                "chapter_unavailable" => "相册网页未提供图片，请打开原网页确认。",
                _ => "紳士漫畫返回异常，请稍后重试或打开原网页确认。",
            }
            .to_string(),
        }
    }
}
fn field<'a>(v: &'a Value, k: &str) -> R<&'a str> {
    v.get(k)
        .and_then(Value::as_str)
        .ok_or_else(|| "parse_error".into())
}
fn setting<'a>(v: &'a Value, k: &str, default: &'a str) -> R<&'a str> {
    match v.get(k) {
        None => Ok(default),
        Some(Value::String(s)) => Ok(s),
        _ => Err("invalid_request".into()),
    }
}
fn rx(p: &str) -> R<Regex> {
    Regex::new(p).map_err(|_| "internal_error".into())
}
fn http(url: &str, base: &str) -> R<Vec<u8>> {
    if host::check_cancel() {
        return Err("cancelled".into());
    }
    let b=serde_json::to_vec(&json!({"url":url,"method":"GET","headers":{"User-Agent":UA,"Referer":format!("{}/",base)}})).map_err(|_|E::from("request_too_large"))?;
    let n = host::http_request(&b, http_out()).map_err(|_| E::from("network_error"))?;
    let n = protocol::body(&http_out()[..n], body_buf()).map_err(E::from)?;
    Ok(body_buf()[..n].to_vec())
}
fn document(url: &str, base: &str) -> R<Node> {
    let b = http(url, base)?;
    // html_text has no br node callback. Replace only genuine br tags by a text
    // newline before parsing, preserving Jsoup wholeText description line breaks;
    // Node::text separately performs Jsoup-style whitespace normalization.
    let text = core::str::from_utf8(&b).map_err(|_| E::from("parse_error"))?;
    let text = rx(r"(?i)<br(?:\s[^>]*)?\s*/?>")?.replace_all(text, "\n");
    Node::parse(text.as_bytes())
}
fn digits(s: &str) -> bool {
    !s.is_empty() && s.len() <= 20 && s.bytes().all(|b| b.is_ascii_digit())
}
fn manga_id(v: &Value) -> R<String> {
    let s = v["args"]["mangaId"]
        .as_str()
        .and_then(|s| s.strip_prefix("wnacg:"))
        .ok_or_else(|| E::from("invalid_request"))?;
    if !digits(s) {
        return Err("invalid_request".into());
    }
    Ok(s.to_string())
}
fn chapter_id(v: &Value) -> R<(String, String)> {
    let s = v["args"]["chapterId"]
        .as_str()
        .and_then(|s| s.strip_prefix("wnacg-read:"))
        .ok_or_else(|| E::from("invalid_request"))?;
    let (aid, cid) = s
        .split_once(':')
        .ok_or_else(|| E::from("invalid_request"))?;
    if !digits(aid) || !digits(cid) {
        return Err("invalid_request".into());
    }
    if let Some(m) = v["args"].get("mangaId") {
        if m.as_str() != Some(&format!("wnacg:{}", aid)) {
            return Err("invalid_request".into());
        }
    }
    Ok((aid.to_string(), cid.to_string()))
}
fn work_path(aid: &str) -> String {
    format!("/photos-index-aid-{}.html", aid)
}
fn url_aid(raw: &str, base: &str) -> R<String> {
    let b = Url::parse(base).map_err(|_| E::from("invalid_request"))?;
    let u = b.join(raw).map_err(|_| E::from("invalid_request"))?;
    if !matches!(u.scheme(), "https" | "http") || !u.username().is_empty() || u.password().is_some()
    {
        return Err("invalid_request".into());
    }
    if u.host_str() != b.host_str()
        && !rx(r"^(?:www\.)?wn(?:acg\w*|\d+)\.")?.is_match(u.host_str().unwrap_or(""))
    {
        return Err("invalid_request".into());
    }
    let p = rx(
        r"^/photos-(?:index|slide|list|gallery)(?:-page-\d+)?-aid-(\d+)(?:-sid-(\d+))?(?:-page-\d+)?\.html$",
    )?;
    let c = p
        .captures(u.path())
        .ok_or_else(|| E::from("invalid_request"))?;
    let aid = c
        .get(2)
        .or_else(|| c.get(1))
        .ok_or_else(|| E::from("invalid_request"))?
        .as_str();
    if !digits(aid) {
        return Err("invalid_request".into());
    }
    Ok(aid.to_string())
}
fn image_url(raw: &str) -> R<String> {
    let u = format!("https://{}", raw.trim_start_matches('/'));
    let parsed = Url::parse(&u).map_err(|_| E::from("parse_error"))?;
    if parsed.host_str().is_none() || !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("parse_error".into());
    }
    Ok(u)
}
fn thumbnail_url_allowed(url: &Url, path: &Regex) -> bool {
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none()
        && url.fragment().is_none()
        && matches!(url.host_str(), Some("t1.wnimg2.cfd" | "t2.wnimg2.cfd" | "t3.wnimg2.cfd" | "t4.wnimg2.cfd"))
        && path.is_match(url.path())
}

fn thumbnail_headers(base: &str) -> Value {
    json!({"User-Agent":UA,"Referer":format!("{}/",base)})
}

fn thumbnail_path_for_host(r: &[u8]) -> R<Option<Regex>> {
    if protocol::feature(r, b"source_public_cover_user_agent_v1") {
        return Ok(Some(rx(THUMBNAIL_PATH)?));
    }
    Ok(None)
}

fn cover(raw: &str, base: &str, public_path: Option<&Regex>) -> R<Value> {
    let uri = image_url(raw)?;
    let mut cover = json!({"kind":"url","url":uri});
    // The catalogue already selected the mirror. Use its complete public
    // request without a per-cover read of the private mirror session.
    if let Some(path) = public_path {
        if Url::parse(&uri).is_ok_and(|url| thumbnail_url_allowed(&url, path)) {
            cover["headers"] = thumbnail_headers(base);
        }
    }
    Ok(cover)
}

fn card(n: &Node, base: &str, public_path: Option<&Regex>) -> R<Value> {
    let a = n.required(".title > a")?;
    let aid = url_aid(
        &a.attr("href")?.ok_or_else(|| E::from("parse_error"))?,
        base,
    )
    .map_err(|_| E::from("parse_error"))?;
    let title = a.text()?;
    let mut item = json!({"id":format!("wnacg:{}",aid),"title":title,"contentRating":"nsfw","url":format!("{}{}",base,work_path(&aid))});
    if let Some(img) = n.one("img") {
        if let Some(src) = img.attr("src")?.filter(|s| !s.is_empty()) {
            item["cover"] = cover(&src, base, public_path)?
        }
    }
    Ok(item)
}
fn case_char(c: char, upper: bool) -> char {
    if upper {
        let mut it = c.to_uppercase();
        let a = it.next().unwrap_or(c);
        if it.next().is_none() {
            a
        } else {
            let t = unicode_titlecase::to_titlecase(c);
            if t[1] == '\0' {
                t[0]
            } else {
                c
            }
        }
    } else {
        let mut it = c.to_lowercase();
        let a = it.next().unwrap_or(c);
        if it.next().is_none() || c == '\u{0130}' {
            a
        } else {
            c
        }
    }
}
fn case_equal(a: char, b: char) -> bool {
    a == b || {
        let au = case_char(a, true);
        let bu = case_char(b, true);
        au == bu || case_char(au, false) == case_char(bu, false)
    }
}
fn contains_case(s: &str, key: &str) -> bool {
    let needle: Vec<char> = key.chars().collect();
    let all: Vec<char> = s.chars().collect();
    !needle.is_empty()
        && all
            .windows(needle.len())
            .any(|w| w.iter().zip(&needle).all(|(a, b)| case_equal(*a, *b)))
}
// Kotlin/JVM toIntOrNull parses UTF-16 BMP decimal digits, optional '+' and
// leading zeroes; it rejects whitespace, overflow and surrogate-pair numerals.
// Keep the getter's positive-or-default rule distinct from Host save validation.
fn positive_int(raw: &str) -> Option<u32> {
    let raw = raw.strip_prefix('+').unwrap_or(raw);
    if raw.is_empty() {
        return None;
    }
    const ZEROS: [u32; 37] = [
        0x30, 0x660, 0x6f0, 0x7c0, 0x966, 0x9e6, 0xa66, 0xae6, 0xb66, 0xbe6, 0xc66, 0xce6, 0xd66,
        0xde6, 0xe50, 0xed0, 0xf20, 0x1040, 0x1090, 0x17e0, 0x1810, 0x1946, 0x19d0, 0x1a80, 0x1a90,
        0x1b50, 0x1bb0, 0x1c40, 0x1c50, 0xa620, 0xa8d0, 0xa900, 0xa9d0, 0xa9f0, 0xaa50, 0xabf0,
        0xff10,
    ];
    let mut n = 0u32;
    for c in raw.chars() {
        let c = c as u32;
        let d = ZEROS.iter().find_map(|z| {
            if c >= *z && c < *z + 10 {
                Some(c - *z)
            } else {
                None
            }
        })?;
        n = n.checked_mul(10)?.checked_add(d)?;
        if n > i32::MAX as u32 {
            return None;
        }
    }
    if n > 0 {
        Some(n)
    } else {
        None
    }
}
fn page_number(v: &Value) -> R<u32> {
    let raw = &v["args"]["page"]["cursor"];
    match raw {
        Value::Null => Ok(1),
        Value::String(s) => {
            let s = s
                .strip_prefix("wn1:")
                .ok_or_else(|| E::from("invalid_request"))?;
            let n = s.parse::<u32>().map_err(|_| E::from("invalid_request"))?;
            if n == 0 || n.to_string() != s {
                return Err("invalid_request".into());
            }
            Ok(n)
        }
        _ => Err("invalid_request".into()),
    }
}
fn list_url(base: &str, v: &Value, search: bool, page: u32) -> R<String> {
    if !search {
        return Ok(format!(
            "{}/{}",
            base,
            match v["args"]["listingId"].as_str().unwrap_or("latest") {
                "popular" => format!("albums-favorite_ranking-page-{}-type-week.html", page),
                "latest" => format!("albums-index-page-{}.html", page),
                _ => return Err("invalid_request".into()),
            }
        ));
    }
    let query = v["args"]["query"].as_str().unwrap_or("");
    let f = &v["args"]["filters"];
    let sort = setting(f, "sort", "create_time_DESC")?;
    if ![
        "create_time_DESC",
        "heat_day_DESC",
        "heat_week_DESC",
        "heat_month_DESC",
        "heat_year_DESC",
    ]
    .contains(&sort)
    {
        return Err("invalid_request".into());
    }
    if !query.trim().is_empty() {
        let mut u = Url::parse(&format!("{}/search/index.php", base))
            .map_err(|_| E::from("parse_error"))?;
        // Url's form serializer emits '+' for spaces; OkHttp addQueryParameter uses
        // percent escapes. Build the same query through SDK byte percent encoding.
        let enc = |s: &str| {
            let mut b = alloc::vec![0;s.len()*3+1];
            let mut n = 0;
            write_url_encoded(&mut b, &mut n, s.as_bytes());
            b.truncate(n);
            String::from_utf8(b).unwrap_or_default()
        };
        u.set_query(Some(&format!(
            "s={}&q={}&p={}",
            enc(sort),
            enc(query),
            page
        )));
        return Ok(u.to_string());
    }
    let tag = setting(f, "tag", "")?;
    if !tag.is_empty() {
        let mut u = Url::parse(base).map_err(|_| E::from("parse_error"))?;
        u.path_segments_mut()
            .map_err(|_| E::from("parse_error"))?
            .push(&format!("albums-index-page-{}-tag-{}.html", page, tag));
        return Ok(u.to_string());
    }
    let category = setting(f, "category", "")?;
    if !category.is_empty() {
        if category == "updated" {
            return Ok(format!("{}/albums-index-page-{}.html", base, page));
        }
        if !digits(category) {
            return Err("invalid_request".into());
        }
        return Ok(format!(
            "{}/albums-index-page-{}-cate-{}.html",
            base, page, category
        ));
    }
    let rank = setting(f, "ranking", "week")?;
    if !["week", "day", "month", "year"].contains(&rank) {
        return Err("invalid_request".into());
    }
    Ok(format!(
        "{}/albums-favorite_ranking-page-{}-type-{}.html",
        base, page, rank
    ))
}
fn list(r: &[u8], v: &Value, search: bool) -> R<Value> {
    let mut page = page_number(v)?; // Validate inputs before private RPC or networking.
    let _ = list_url(DEFAULTS[0], v, search, page)?;
    let query = v["args"]["query"].as_str().unwrap_or("");
    let imported =
        if search && Url::parse(query).is_ok_and(|u| matches!(u.scheme(), "http" | "https")) {
            let current = Context::metadata(v)?;
            Some(url_aid(query, &current.base)?)
        } else {
            None
        };
    let mut cx = Context::network(r, v)?;
    let public_path = thumbnail_path_for_host(r)?;
    if let Some(aid) = imported {
        let manga = detail_for(&cx, &aid, public_path.as_ref())?;
        return Ok(json!({"items":[manga],"page":{"hasMore":false,"nextCursor":null}}));
    }
    let settings = &v["settings"];
    let blacklist = setting(settings, "titleBlacklist", "")?
        .split([',', '\n', '\r'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    let filter = match settings.get("filterSearchResults") {
        None => true,
        Some(Value::Bool(b)) => *b,
        Some(Value::String(s)) if s == "true" || s == "false" => s == "true",
        _ => return Err("invalid_request".into()),
    };
    let raw_max = setting(settings, "blacklistMaxScanPages", "5")?;
    let max = positive_int(raw_max).unwrap_or(5);
    let active = !blacklist.is_empty() && (!search || filter);
    let scans = if active { max } else { 1 };
    for _ in 0..scans {
        let u = list_url(&cx.base, v, search, page)?;
        let d = document(&u, &cx.base)?;
        cx.save_categories(&d)?;
        let nodes = d.all(".gallary_item")?;
        if nodes.is_empty() && d.one("#album_tabs").is_none() {
            return Err("parse_error".into());
        }
        let mut items = Vec::new();
        for node in &nodes {
            let card = card(node, &cx.base, public_path.as_ref())?;
            let title = card["title"]
                .as_str()
                .ok_or_else(|| E::from("parse_error"))?;
            if !active || !blacklist.iter().any(|k| contains_case(title, k)) {
                items.push(card)
            }
        }
        let more = d.one("span.thispage + a").is_some();
        if !items.is_empty() || !more {
            return Ok(
                json!({"items":items,"page":{"hasMore":more,"nextCursor":if more{Some(format!("wn1:{}",page.checked_add(1).ok_or_else(||E::from("invalid_request"))?))}else{None}}}),
            );
        }
        page = page
            .checked_add(1)
            .ok_or_else(|| E::from("invalid_request"))?;
    }
    Err(E {
        code: "parse_error",
        message: format!(
            "连续 {} 页均无可显示结果，请调整黑名单或扫描页数后刷新。",
            max
        ),
    })
}
fn detail_for(cx: &Context, aid: &str, public_path: Option<&Regex>) -> R<Value> {
    let u = format!("{}{}", cx.base, work_path(aid));
    let d = document(&u, &cx.base)?;
    let title = match d.one("h2") {
        Some(n) => n.text()?,
        None => {
            return Err(E {
                code: "parse_error",
                message: LOGIN_HINT.to_string(),
            })
        }
    };
    let mut manga = json!({"id":format!("wnacg:{}",aid),"title":title,"contentRating":"nsfw","status":"completed","links":[{"kind":"source","url":u}]});
    if let Some(a) = d.one("div.uwuinfo p") {
        let a = a.text()?;
        manga["authors"] = json!([a]);
        manga["artists"] = manga["authors"].clone()
    }
    let labels = d.all("div.uwconn label")?;
    let mut tags = Vec::new();
    let mut collection = false;
    let mut ongoing = false;
    for n in labels {
        let t = n.text()?;
        if let Some(s) = t.strip_prefix("分類：") {
            for tag in s.split('／') {
                if !tags.iter().any(|t| t == tag) {
                    tags.push(tag.to_string())
                }
            }
        }
        if t.contains("章節：") {
            collection = true
        }
        if t.contains("狀態") && t.contains("連載中") {
            ongoing = true
        }
    }
    for n in d.all("div.addtags a.tagshow")? {
        let t = n.text()?;
        if !tags.contains(&t) {
            tags.push(t)
        }
    }
    manga["tags"] = json!(tags);
    if collection && ongoing {
        manga["status"] = json!("ongoing")
    }
    if let Some(img) = d.one("div.uwthumb img") {
        if let Some(src) = img.attr("src")?.filter(|s| !s.is_empty()) {
            manga["cover"] = cover(&src, &cx.base, public_path)?
        }
    }
    for p in d.all("div.uwconn p")? {
        let text = p.whole()?;
        if let Some(t) = text.strip_prefix("簡介：") {
            if !t.trim().is_empty() {
                manga["description"] = json!(t.trim())
            }
            break;
        }
    }
    Ok(manga)
}
fn date(text: &str) -> Option<i64> {
    let re = Regex::new(r"\d{4}-\d{2}-\d{2}").ok()?;
    let m = re.find(text)?.as_str();
    let mut p = m.split('-').map(str::parse::<i64>);
    let y = p.next()?.ok()?;
    let mo = p.next()?.ok()?;
    let d = p.next()?.ok()?;
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if !(1..=12).contains(&mo) || d < 1 || d > days[(mo - 1) as usize] {
        return None;
    }
    let yy = y - if mo <= 2 { 1 } else { 0 };
    let era = yy.div_euclid(400);
    let yoe = yy - era * 400;
    let mp = mo + if mo > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(((era * 146097 + doe - 719468) * 86400 - 8 * 3600) * 1000)
}
fn chapters(r: &[u8], v: &Value) -> R<Value> {
    let aid = manga_id(v)?;
    let cx = Context::network(r, v)?;
    let u = format!("{}{}", cx.base, work_path(&aid));
    let d = document(&u, &cx.base)?;
    if d.one("h2").is_none() {
        return Err(E {
            code: "parse_error",
            message: LOGIN_HINT.to_string(),
        });
    }
    let mut max = 1u32;
    let re = rx(r"^/?photos-index-aid-(\d+)-page-(\d+)\.html$")?;
    for a in d.all("div.f_left.paginator a[href]")? {
        if let Some(h) = a.attr("href")? {
            if let Some(c) = re.captures(&h) {
                if &c[1] != aid {
                    return Err("parse_error".into());
                }
                max = max.max(c[2].parse().map_err(|_| E::from("parse_error"))?)
            }
        }
    }
    if max > 100000 {
        return Err("result_too_large".into());
    }
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for p in 1..=max {
        let more;
        if p > 1 {
            more = document(
                &format!("{}/photos-index-aid-{}-page-{}.html", cx.base, aid, p),
                &cx.base,
            )?
        } else {
            more = Node::parse(b"<html></html>")?
        }
        let root = if p == 1 { &d } else { &more };
        // A later-page failure never publishes a partial collection.
        if p > 1 && root.one("h2").is_none() {
            return Err("parse_error".into());
        }
        for n in root.all("div.sr_compact a.tagshow[data-chid]")? {
            let cid = n.attr("data-chid")?.ok_or_else(|| E::from("parse_error"))?;
            if !digits(&cid) {
                return Err("parse_error".into());
            }
            if !seen.insert(cid.clone()) {
                continue;
            }
            let mut item = json!({"id":format!("wnacg-read:{}:{}",aid,cid),"mangaId":format!("wnacg:{}",aid),"title":n.text()?,"url":format!("{}{}",cx.base,work_path(&cid))});
            if let Some(t) = n.attr("title")?.and_then(|s| date(&s)) {
                item["dateUpload"] = json!(t)
            }
            out.push(item)
        }
    }
    if out.is_empty() {
        let mut item = json!({"id":format!("wnacg-read:{}:{}",aid,aid),"mangaId":format!("wnacg:{}",aid),"title":"Ch. 1","url":u});
        if let Some(n) = d.one("li.gallary_item .info_col") {
            if let Some(t) = date(&n.text()?) {
                item["dateUpload"] = json!(t)
            }
        }
        out.push(item)
    } else {
        out.reverse()
    }
    for (i, n) in out.iter_mut().enumerate() {
        n["index"] = json!(i)
    }
    Ok(json!({"chapters":out,"page":{"hasMore":false,"nextCursor":null}}))
}
fn uri_owner(url: &str, cid: &str) -> R<()> {
    let u = Url::parse(url).map_err(|_| E::from("invalid_request"))?;
    if !matches!(u.scheme(), "http" | "https") || !u.username().is_empty() || u.password().is_some()
    {
        return Err("invalid_request".into());
    }
    // All 5807 frozen original URLs expose this CID partition. Only reject a
    // provable different CID in this path form; opaque other CDN layouts stay valid.
    if let Some(c) = rx(r"^/data/(\d+)/(\d{2})/")?.captures(u.path()) {
        if format!("{}{}", &c[1], &c[2]) != cid {
            return Err("invalid_request".into());
        }
    }
    Ok(())
}
fn pages(r: &[u8], v: &Value) -> R<Value> {
    let (aid, cid) = chapter_id(v)?;
    let cx = Context::network(r, v)?;
    let u = format!("{}/photos-gallery-aid-{}.html", cx.base, cid);
    let b = http(&u, &cx.base)?;
    let t = core::str::from_utf8(&b).map_err(|_| E::from("parse_error"))?;
    let re = rx(r#"(?i)//[^\s"'\\]+\.(?:jpeg|jpg|png|webp|gif)(?:\?[^\s"'\\]*)?"#)?;
    let mut out = Vec::new();
    for (i, m) in re.find_iter(t).enumerate() {
        let url = image_url(m.as_str())?;
        uri_owner(&url, &cid).map_err(|_| E::from("parse_error"))?;
        out.push(json!({"id":format!("wnacg-page:{}:{}:{}",aid,cid,i),"index":i,"image":{"kind":"url","url":url}}))
    }
    if out.is_empty() {
        let d = Node::parse(&b)?;
        if d.one("#album_tabs").is_some() && d.one("h2").is_some() {
            return Err("chapter_unavailable".into());
        }
        return Err("parse_error".into());
    }
    Ok(json!({"pages":out}))
}
fn image_request(r: &[u8], v: &Value) -> R<Value> {
    // Cover descriptors use the work ID as pageId. Keep this local: the
    // original thumbnail request does not refetch a detail page per image.
    if let Some(aid) = v["args"]["pageId"].as_str().and_then(|s| s.strip_prefix("wnacg:")) {
        if !digits(aid) || v["args"].get("chapterId").is_some() {
            return Err("invalid_request".into());
        }
        if let Some(m) = v["args"].get("mangaId") {
            if m.as_str() != Some(&format!("wnacg:{}", aid)) {
                return Err("invalid_request".into());
            }
        }
        let raw = v["args"]["url"].as_str().ok_or_else(|| E::from("invalid_request"))?;
        let url = Url::parse(raw).map_err(|_| E::from("invalid_request"))?;
        if !thumbnail_url_allowed(&url, &rx(THUMBNAIL_PATH)?) {
            return Err("invalid_request".into());
        }
        // Collection artwork may belong to a child gallery, so its thumbnail
        // path encodes that gallery ID, not necessarily the enclosing work ID.
        let cx = Context::metadata(v)?;
        if let Some(route) = v["args"].get("mangaUrl") {
            let route = route.as_str().ok_or_else(|| E::from("invalid_request"))?;
            if url_aid(route, &cx.base)? != aid {
                return Err("invalid_request".into());
            }
        }
        return Ok(json!({"url":raw,"headers":thumbnail_headers(&cx.base)}));
    }
    let pid = v["args"]["pageId"]
        .as_str()
        .and_then(|s| s.strip_prefix("wnacg-page:"))
        .ok_or_else(|| E::from("invalid_request"))?;
    let fields = pid.split(':').collect::<Vec<_>>();
    if fields.len() != 3 || !digits(fields[0]) || !digits(fields[1]) {
        return Err("invalid_request".into());
    }
    let index = fields[2]
        .parse::<usize>()
        .map_err(|_| E::from("invalid_request"))?;
    if index.to_string() != fields[2] {
        return Err("invalid_request".into());
    }
    if let Some(m) = v["args"].get("mangaId") {
        if m.as_str() != Some(&format!("wnacg:{}", fields[0])) {
            return Err("invalid_request".into());
        }
    }
    if let Some(c) = v["args"].get("chapterId") {
        if c.as_str() != Some(&format!("wnacg-read:{}:{}", fields[0], fields[1])) {
            return Err("invalid_request".into());
        }
    }
    let u = v["args"]["url"]
        .as_str()
        .ok_or_else(|| E::from("invalid_request"))?;
    uri_owner(u, fields[1])?;
    if let Some(route) = v["args"].get("mangaUrl") {
        let route = route.as_str().ok_or_else(|| E::from("invalid_request"))?;
        let owner = match url_aid(route, DEFAULTS[0]) {
            Ok(owner) => owner,
            Err(_) => {
                let metadata = Context::metadata(v)?;
                url_aid(route, &metadata.base)?
            }
        };
        if owner != fields[0] {
            return Err("invalid_request".into());
        }
    }
    // A reader can reuse saved pages after restarting the App. The original
    // BaseUrlFetchInterceptor still initializes before that first image request;
    // do not restrict per-launch discovery to catalogue/API operations.
    let cx = Context::network(r, v)?;
    let moved = cx.moved_image_uri(u)?;
    Ok(json!({"url":moved,"headers":{"User-Agent":UA,"Referer":format!("{}/",cx.base)}}))
}
const CATEGORIES: [(&str, &str); 21] = [
    ("", ""),
    ("更新", "updated"),
    ("同人志", "5"),
    ("同人志-汉化", "1"),
    ("同人志-日语", "12"),
    ("同人志-English（英语）", "16"),
    ("同人志-CG书籍", "2"),
    ("写真&Cosplay", "3"),
    ("单行本", "6"),
    ("单行本-汉化", "9"),
    ("单行本-English（英语）", "17"),
    ("单行本-日语", "13"),
    ("杂志&短篇", "7"),
    ("杂志&短篇-汉语", "10"),
    ("杂志&短篇-日语", "14"),
    ("杂志&短篇-English（英语）", "18"),
    ("3D&漫画", "22"),
    ("3D&漫画-汉语", "23"),
    ("3D&漫画-其他", "24"),
    ("AI图集", "37"),
    ("未分類相冊", "0"),
];
fn filters(r: &[u8], v: &Value) -> R<Value> {
    let cx = Context::metadata(v)?;
    let mut cats = CATEGORIES
        .iter()
        .map(|(l, id)| json!({"value":id,"label":l}))
        .collect::<Vec<_>>();
    for (l, id) in cx.categories() {
        cats.push(json!({"value":id,"label":l}))
    }
    let scope = protocol::feature(r, b"source_filter_scope_header_v1");
    let mut rows = Vec::new();
    if scope {
        rows.push(json!({"type":"header","name":"注意：排序仅用于关键词搜索"}))
    }
    rows.push(json!({"id":"sort","name":"排序","type":"select","default":"create_time_DESC","options":[{"value":"create_time_DESC","label":"最新"},{"value":"heat_day_DESC","label":"今日热门"},{"value":"heat_week_DESC","label":"本周热门"},{"value":"heat_month_DESC","label":"本月热门"},{"value":"heat_year_DESC","label":"今年热门"}]}));
    if scope {
        rows.push(json!({"type":"header","name":"注意：分类和标签均不支持搜索"}));
        rows.push(json!({"type":"header","name":"注意：在 WebView 中登录后刷新列表并重置，可显示韩漫等分类"}))
    }
    rows.push(json!({"id":"category","name":"分类","type":"select","default":"","options":cats}));
    if scope {
        rows.push(json!({"type":"header","name":"注意：排行仅在未选分类和标签时生效"}))
    }
    rows.push(json!({"id":"ranking","name":"排行","type":"select","default":"week","options":[{"value":"week","label":"本周"},{"value":"day","label":"今日"},{"value":"month","label":"本月"},{"value":"year","label":"今年"}]}));
    if scope {
        rows.push(json!({"type":"header","name":"注意：仅支持 1 个标签，不支持分类"}))
    }
    rows.push(json!({"id":"tag","name":"标签","type":"text","default":""}));
    Ok(json!({"filters":rows}))
}
fn settings(r: &[u8], v: &Value) -> R<Value> {
    // The upstream preference rejects invalid values before saving. Old Hosts
    // may still consume existing values with the unchanged getter fallback,
    // but must not present this form without the matching write validator.
    if !protocol::feature(r, b"source_setting_positive_integer_v1") {
        return Err("unsupported".into());
    }
    let cx = Context::metadata(v)?;
    let mut rows = Vec::new();
    let options = cx.mirror_options()?;
    if options.len() > 1 {
        rows.push(json!({"id":"preferred_mirror","name":"首选镜像","kind":"select","default":"0","options":options}))
    }
    rows.push(json!({"id":"titleBlacklist","name":"标题屏蔽关键词","kind":"text","default":"","description":"使用英文逗号或换行分隔多个关键词；忽略大小写，始终作用于热门和最新列表。"}));
    rows.push(json!({"id":"blacklistMaxScanPages","name":"黑名单过滤最大扫描页数","kind":"text","default":"5","description":"每次加载最多检查多少个网站页面，包含当前页。请输入不小于 1 的正整数。","validation":{"kind":"positive_integer","message":"请输入不小于 1 的正整数。"}}));
    rows.push(json!({"id":"filterSearchResults","name":"在搜索结果中使用标题屏蔽","kind":"toggle","default":true,"description":"开启后，关键词、分类和标签搜索结果也会应用标题屏蔽关键词。"}));
    Ok(json!({"settings":rows}))
}
fn authentication(v: &Value) -> R<Value> {
    let cx = Context::metadata(v)?;
    Ok(
        json!({"authentication":{"kind":"web","loginUrl":format!("{}/users-login.html",cx.base),"userAgent":UA},"authenticated":false}),
    )
}
fn dispatch(op: &str, r: &[u8]) -> R<Value> {
    protocol::request(r).map_err(E::from)?;
    let args = protocol::direct(r, b"args").map_err(E::from)?.unwrap_or(r);
    let mut owner = [0; 16384];
    let _ = protocol::text(args, b"mangaUrl", &mut owner).map_err(E::from)?;
    let v: Value = serde_json::from_slice(r).map_err(|_| E::from("invalid_request"))?;
    match op {
        "get_listings" => Ok(
            json!({"listings":[{"id":"latest","name":"最新","supportsPaging":true},{"id":"popular","name":"热门","supportsPaging":true}]}),
        ),
        "get_manga_list" => list(r, &v, false),
        "search" => list(r, &v, true),
        "get_manga" => {
            let aid = manga_id(&v)?;
            let cx = Context::network(r, &v)?;
            let public_path = thumbnail_path_for_host(r)?;
            Ok(json!({"manga":detail_for(&cx,&aid,public_path.as_ref())?}))
        }
        "get_chapters" => chapters(r, &v),
        "get_pages" => pages(r, &v),
        "get_image_request" => image_request(r, &v),
        "get_filters" => filters(r, &v),
        "get_settings" => settings(r, &v),
        "get_authentication" => {
            if !protocol::feature(r, b"source_authentication_v1") {
                return Err("unsupported".into());
            }
            authentication(&v)
        }
        "authenticate" | "logout" => Err("not_implemented".into()),
        _ => Err("not_implemented".into()),
    }
}
fn run(op: &str, r: &[u8]) -> u32 {
    match dispatch(op, r) {
        Ok(v) => match serde_json::to_vec(&v) {
            Ok(b) if b.len() <= payload_buf().len() => {
                payload_buf()[..b.len()].copy_from_slice(&b);
                write_success_payload(op, b.len())
            }
            _ => write_error(op, "result_too_large", "书源结果过大。"),
        },
        Err(e) => write_error(op, e.code, &e.message),
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
fn run_get_authentication(r: &[u8]) -> u32 {
    run("get_authentication", r)
}
fn run_authenticate(r: &[u8]) -> u32 {
    run("authenticate", r)
}
fn run_logout(r: &[u8]) -> u32 {
    run("logout", r)
}
koma_source_sdk::koma_source_exports!("wnacg_source");
koma_source_sdk::koma_source_authentication_exports!("wnacg_source");
