#![no_std]
extern crate alloc;
// Independent complete reference port: Keiyoushi de743aed846da53e00c45ab26ee0e77f7bb4355b, Picacomic v14, Apache2.
use alloc::{
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
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use url::Url;
mod authentication;
use authentication::{request_api, Session};
#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;
const API: &str = "https://picaapi.picacomic.com";
const BASE: &str = "https://manhuabika.com";
const KEY: &str = "C69BAF41DA5ABD1FFEDC6D2FEA56B";
const HMAC_KEY: &str = r#"~d}$Q7$eIni=V)9\RK/P.RM4;9[7|@/CA}b~OW!3?EV`:<>M7pddUBL5n|0/*Cn"#;
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.picacomic.koma",
    name: "哔咔漫画",
    version: "0.1.0",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "哔咔漫画。浏览与阅读需要账号登录。",
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
    credentials: true,
};
koma_source_sdk::koma_source_buffers! {payload:1024*1024,http_out:6*1024*1024,body:4*1024*1024,http_req:32768,scratch:8192,}
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
                "auth_required" => "此书源需要登录账号后浏览和阅读，请先连接账号。",
                "unsupported" => "当前应用尚不具备此书源所需能力，请更新应用。",
                "forbidden" => "哔咔拒绝了本次请求，请在原服务确认访问状态。",
                "rate_limit" => "请求过于频繁，请稍后重试。",
                "network_error" => "网络传输未完成，请稍后重试。",
                "not_found" => "哔咔未提供此内容，请在原服务确认。",
                _ => "哔咔返回异常，请稍后重试。",
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
fn array<'a>(v: &'a Value, k: &str) -> R<&'a Vec<Value>> {
    v.get(k)
        .and_then(Value::as_array)
        .ok_or_else(|| "parse_error".into())
}
fn setting<'a>(v: &'a Value, k: &str, d: &'a str) -> R<&'a str> {
    match v.get(k) {
        None => Ok(d),
        Some(Value::String(s)) => Ok(s),
        _ => Err("invalid_request".into()),
    }
}
fn require(r: &[u8]) -> R<()> {
    for f in [
        b"source_authentication_v1".as_slice(),
        b"source_private_session_v1",
        b"source_private_session_replace_v1",
    ] {
        if !protocol::feature(r, f) {
            return Err("unsupported".into());
        }
    }
    Ok(())
}
struct Options {
    channel: String,
    quality: String,
    blocked: Vec<String>,
    dns_override: Option<Value>,
}
impl Options {
    fn new(v: &Value) -> R<Self> {
        let s = &v["settings"];
        let channel = match setting(s, "APP_CHANNEL", "2")? {
            "1" => "1",
            "2" | "3" => "2",
            _ => return Err("invalid_request".into()),
        }
        .to_string();
        let quality = setting(s, "IMAGE_QUALITY", "original")?;
        if !["original", "low", "medium", "high"].contains(&quality) {
            return Err("invalid_request".into());
        }
        let blocked = setting(s, "BLOCK_GENRES", "")?
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
            .collect();
        Ok(Self {
            channel,
            quality: quality.to_string(),
            blocked,
            dns_override: None,
        })
    }
    fn require_transport(&self, request: &[u8]) -> R<()> {
        if self.channel == "2" && !protocol::feature(request, b"source_dns_pica_channel_v1") {
            return Err(E {
                code: "unsupported",
                message: "当前应用尚不支持分流2所需的DNS与原证书主机能力。请等待对应应用更新，或在书源设置选择分流1。".to_string(),
            });
        }
        Ok(())
    }
    fn transport(&mut self, request: &[u8]) -> R<()> {
        self.require_transport(request)?;
        if self.channel == "2" && self.dns_override.is_none() {
            self.dns_override = Some(initialize_channel()?);
        }
        Ok(())
    }
    fn apply_transport(&self, descriptor: &mut Value) {
        if let Some(dns_override) = &self.dns_override {
            descriptor["dnsOverride"] = dns_override.clone();
        }
    }
}
fn initialize_channel() -> R<Value> {
    if host::check_cancel() {
        return Err("cancelled".into());
    }
    // The Host chooses the committed init URL and public headers. This proof
    // is used only in the current operation's descriptors and is never saved.
    let request = br#"{"type":"pica_channel_init","version":1}"#;
    let n = host::http_request(request, http_out()).map_err(|_| E::from("network_error"))?;
    let response = &http_out()[..n];
    if host::check_cancel() {
        return Err("cancelled".into());
    }
    let error = protocol::direct(response, b"error").map_err(E::from)?;
    if let Some(error) = error {
        match protocol::direct(error, b"code").map_err(E::from)? {
            Some(b"\"source_request_cancelled\"") => return Err("cancelled".into()),
            Some(b"\"unsupported\"") => return Err("unsupported".into()),
            Some(b"\"invalid_response\"") => return Err("parse_error".into()),
            _ => {}
        }
    }
    protocol::body(response, body_buf()).map_err(|code| {
        // This public initialization has no account challenge. Do not offer
        // another sign-in when the initialization endpoint itself denies it.
        E::from(if code == "auth_required" {
            "forbidden"
        } else {
            code
        })
    })?;
    let proof = protocol::direct(response, b"dnsOverride")
        .map_err(|_| E::from("parse_error"))?
        .ok_or_else(|| E::from("parse_error"))?;
    let kind = protocol::direct(proof, b"kind").map_err(|_| E::from("parse_error"))?;
    let raw_proof = protocol::direct(proof, b"proof").map_err(|_| E::from("parse_error"))?;
    let value: Value = serde_json::from_slice(proof).map_err(|_| E::from("parse_error"))?;
    if kind != Some(b"\"pica_channel_v1\"")
        || raw_proof.is_none()
        || !value.as_object().is_some_and(|map| map.len() == 2)
        || !value["proof"]
            .as_str()
            .is_some_and(|token| !token.is_empty() && token.encode_utf16().count() <= 128)
    {
        return Err("parse_error".into());
    }
    Ok(value)
}
fn encoded(s: &str) -> String {
    let mut b = alloc::vec![0;s.len()*3+1];
    let mut n = 0;
    write_url_encoded(&mut b, &mut n, s.as_bytes());
    b.truncate(n);
    String::from_utf8(b).unwrap_or_default()
}
fn id(raw: &str) -> R<String> {
    if raw.is_empty() || raw.len() > 2048 {
        return Err("invalid_request".into());
    }
    let decoded = percent_encoding::percent_decode_str(raw)
        .decode_utf8()
        .map_err(|_| E::from("invalid_request"))?;
    if decoded.is_empty()
        || matches!(decoded.as_ref(), "." | "..")
        || decoded.contains('/')
        || decoded.contains(char::from(92u8))
        || decoded.chars().any(char::is_control)
        || encoded(&decoded) != raw
    {
        return Err("invalid_request".into());
    }
    Ok(raw.to_string())
}
fn manga_id(v: &Value) -> R<String> {
    id(v["args"]["mangaId"]
        .as_str()
        .and_then(|s| s.strip_prefix("picacomic:"))
        .ok_or_else(|| E::from("invalid_request"))?)
}
fn order(raw: &str) -> R<u32> {
    let n = raw.parse::<u32>().map_err(|_| E::from("invalid_request"))?;
    if n > i32::MAX as u32 || n.to_string() != raw {
        return Err("invalid_request".into());
    }
    Ok(n)
}
fn chapter_id(v: &Value) -> R<(String, u32)> {
    let text = v["args"]["chapterId"]
        .as_str()
        .and_then(|s| s.strip_prefix("picacomic-read:"))
        .ok_or_else(|| E::from("invalid_request"))?;
    let (m, n) = text
        .rsplit_once(':')
        .ok_or_else(|| E::from("invalid_request"))?;
    let m = id(m)?;
    let n = order(n)?;
    if let Some(owner) = v["args"].get("mangaId") {
        if owner.as_str() != Some(&format!("picacomic:{}", m)) {
            return Err("invalid_request".into());
        }
    }
    Ok((m, n))
}
fn by_url(raw: &str) -> R<String> {
    let u = Url::parse(raw).map_err(|_| E::from("invalid_request"))?;
    if u.host_str() != Some("manhuabika.com")
        || !matches!(u.scheme(), "http" | "https")
        || !u.username().is_empty()
        || u.password().is_some()
    {
        return Err("invalid_request".into());
    }
    let p = u
        .path_segments()
        .ok_or_else(|| E::from("invalid_request"))?
        .collect::<Vec<_>>();
    if p.first() != Some(&"comic") {
        return Err("invalid_request".into());
    }
    let at = if p.get(1) == Some(&"reader") { 2 } else { 1 };
    let raw = p.get(at).ok_or_else(|| E::from("invalid_request"))?;
    let decoded = percent_encoding::percent_decode_str(raw)
        .decode_utf8()
        .map_err(|_| E::from("invalid_request"))?;
    id(&encoded(&decoded))
}
fn image_url(v: &Value) -> R<String> {
    let raw = format!("{}/static/{}", field(v, "fileServer")?, field(v, "path")?);
    let u = Url::parse(&raw).map_err(|_| E::from("parse_error"))?;
    if !matches!(u.scheme(), "http" | "https") || !u.username().is_empty() || u.password().is_some()
    {
        return Err("parse_error".into());
    }
    Ok(raw)
}
fn genres(comic: &Value) -> R<Vec<String>> {
    let a = match comic.get("tags") {
        None | Some(Value::Null) => array(comic, "categories")?,
        Some(Value::Array(a)) => a,
        _ => return Err("parse_error".into()),
    };
    a.iter()
        .map(|v| {
            v.as_str()
                .map(|s| s.trim().to_string())
                .ok_or_else(|| "parse_error".into())
        })
        .collect()
}
fn manga(comic: &Value) -> R<Value> {
    let raw = field(comic, "_id")?;
    if raw.is_empty() {
        return Err("parse_error".into());
    }
    let mid = encoded(raw);
    let mut tags = Vec::new();
    for x in genres(comic)? {
        if !tags.contains(&x) {
            tags.push(x)
        }
    }
    let finished = comic["finished"]
        .as_bool()
        .ok_or_else(|| E::from("parse_error"))?;
    let mut item = json!({"id":format!("picacomic:{}",mid),"title":field(comic,"title")?,"cover":{"kind":"url","url":image_url(&comic["thumb"])?},"contentRating":"nsfw","status":if finished{"completed"}else{"ongoing"},"tags":tags,"links":[{"kind":"source","url":format!("{}/comic/{}",BASE,mid)}]});
    for (wire, key) in [("author", "authors"), ("artist", "artists")] {
        match comic.get(wire) {
            None | Some(Value::Null) => {}
            Some(Value::String(s)) => item[key] = json!([s]),
            _ => return Err("parse_error".into()),
        }
    }
    match comic.get("description") {
        None | Some(Value::Null) => {}
        Some(Value::String(s)) => item["description"] = json!(s),
        _ => return Err("parse_error".into()),
    }
    Ok(item)
}
fn api(r: &[u8], v: &Value, path: &str, method: &str, body: Option<Value>) -> R<Value> {
    require(r)?;
    let mut options = Options::new(v)?;
    let mut s = Session::existing()?;
    options.transport(r)?;
    request_api(&mut s, &options, path, method, body)
}
fn detail(r: &[u8], v: &Value, mid: &str) -> R<Value> {
    let body = api(r, v, &format!("/comics/{}", mid), "GET", None)?;
    if encoded(field(&body["data"]["comic"], "_id")?) != mid {
        return Err("parse_error".into());
    }
    manga(&body["data"]["comic"])
}
fn page(v: &Value) -> R<u32> {
    let raw = &v["args"]["page"]["cursor"];
    match raw {
        Value::Null => Ok(1),
        Value::String(s) => {
            let n = s.parse::<u32>().map_err(|_| E::from("invalid_request"))?;
            if n == 0 || n > i32::MAX as u32 || n.to_string() != *s {
                return Err("invalid_request".into());
            }
            Ok(n)
        }
        _ => Err("invalid_request".into()),
    }
}
fn list(r: &[u8], v: &Value, search: bool) -> R<Value> {
    let page = page(v)?;
    let query = if search {
        v["args"]["query"].as_str().unwrap_or("")
    } else {
        ""
    };
    if Url::parse(query).is_ok_and(|u| matches!(u.scheme(), "https" | "http")) {
        let mid = by_url(query)?;
        let m = detail(r, v, &mid)?;
        return Ok(json!({"items":[m],"page":{"hasMore":false,"nextCursor":null}}));
    }
    let options = Options::new(v)?;
    let mut random = false;
    let mut single = false;
    let (path, method, body) = if !search {
        match v["args"]["listingId"].as_str().unwrap_or("random") {
            "popular" => (format!("/comics?page={}&s=dd", page), "GET", None),
            "random" => {
                random = true;
                single = true;
                ("/comics/random".to_string(), "GET", None)
            }
            _ => return Err("invalid_request".into()),
        }
    } else {
        let f = &v["args"]["filters"];
        let sort = setting(f, "sort", "dd")?;
        if !["dd", "da", "ld", "vd"].contains(&sort) {
            return Err("invalid_request".into());
        }
        let category = setting(f, "category", "")?;
        if !category.is_empty() && !CATEGORIES.contains(&category) {
            return Err("invalid_request".into());
        }
        let rank = setting(f, "rank", "")?;
        let favourite = match f.get("favourite") {
            None => false,
            Some(Value::Bool(b)) => *b,
            Some(Value::String(s)) if s == "true" || s == "false" => s == "true",
            _ => return Err("invalid_request".into()),
        };
        if favourite {
            (
                format!("/users/favourite?page={}&s={}", page, sort),
                "GET",
                None,
            )
        } else if !rank.is_empty() {
            if !["H24", "D7", "D30"].contains(&rank) {
                return Err("invalid_request".into());
            }
            single = true;
            (
                format!("/comics/leaderboard?tt={}&ct=VC", rank),
                "GET",
                None,
            )
        } else if query.is_empty() {
            (
                format!(
                    "/comics?page={}&s={}{}",
                    page,
                    sort,
                    if category.is_empty() {
                        String::new()
                    } else {
                        format!("&c={}", encoded(category))
                    }
                ),
                "GET",
                None,
            )
        } else {
            (
                format!("/comics/advanced-search?page={}", page),
                "POST",
                Some(json!({"keyword":query,"sort":sort})),
            )
        }
    };
    let data = api(r, v, &path, method, body)?;
    let comics = &data["data"]["comics"];
    let (docs, more) = if single {
        (
            comics.as_array().ok_or_else(|| E::from("parse_error"))?,
            random,
        )
    } else {
        let actual = comics["page"]
            .as_u64()
            .ok_or_else(|| E::from("parse_error"))?;
        let total = comics["pages"]
            .as_u64()
            .ok_or_else(|| E::from("parse_error"))?;
        if actual != page as u64 {
            return Err("parse_error".into());
        }
        (array(comics, "docs")?, actual < total)
    };
    let mut items = Vec::new();
    for c in docs {
        if genres(c)?.iter().any(|t| options.blocked.contains(t)) {
            continue;
        }
        items.push(manga(c)?)
    }
    Ok(
        json!({"items":items,"page":{"hasMore":more,"nextCursor":if more{Some(page.checked_add(1).ok_or_else(||E::from("invalid_request"))?.to_string())}else{None}}}),
    )
}

fn utc_time(s: &str) -> Option<i64> {
    if s.len() < 20 {
        return None;
    }
    let b = s.as_bytes();
    if b[4] != b'-' || b[7] != b'-' || b[10] != b'T' || b[13] != b':' || b[16] != b':' {
        return None;
    }
    let num = |a: usize, b: usize| s.get(a..b)?.parse::<i64>().ok();
    let y = num(0, 4)?;
    let mo = num(5, 7)?;
    let d = num(8, 10)?;
    let hh = num(11, 13)?;
    let mm = num(14, 16)?;
    let sec = num(17, 19)?;
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let dm = [
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
    if !(1..=12).contains(&mo)
        || d < 1
        || d > dm[(mo - 1) as usize]
        || hh > 23
        || mm > 59
        || sec > 59
    {
        return None;
    }
    let mut pos = 19;
    let mut millis = 0;
    if b.get(pos) == Some(&b'.') {
        pos += 1;
        let start = pos;
        while b.get(pos).is_some_and(u8::is_ascii_digit) {
            pos += 1
        }
        if pos == start || pos - start > 9 {
            return None;
        }
        let mut first = s[start..pos].chars().take(3).collect::<String>();
        while first.len() < 3 {
            first.push('0')
        }
        millis = first.parse().ok()?
    }
    let tail = s.get(pos..)?;
    let offset = if tail == "Z" {
        0
    } else {
        let b = tail.as_bytes();
        if b.len() != 6 || !matches!(b[0], b'+' | b'-') || b[3] != b':' {
            return None;
        }
        let h = tail.get(1..3)?.parse::<i64>().ok()?;
        let m = tail.get(4..6)?.parse::<i64>().ok()?;
        if h > 18 || m > 59 || (h == 18 && m != 0) {
            return None;
        }
        (if b[0] == b'+' { 1 } else { -1 }) * (h * 3600 + m * 60)
    };
    let yy = y - if mo <= 2 { 1 } else { 0 };
    let era = yy.div_euclid(400);
    let yoe = yy - era * 400;
    let mp = mo + if mo > 2 { -3 } else { 9 };
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + (153 * mp + 2) / 5 + d - 1;
    Some(
        ((era * 146097 + doe - 719468) * 86400 + hh * 3600 + mm * 60 + sec - offset) * 1000
            + millis,
    )
}
fn chapters(r: &[u8], v: &Value) -> R<Value> {
    let mid = manga_id(v)?;
    let first = api(r, v, &format!("/comics/{}/eps?page=1", mid), "GET", None)?;
    let total = first["data"]["eps"]["pages"]
        .as_u64()
        .filter(|n| *n >= 1 && *n <= 100000)
        .ok_or_else(|| E::from("parse_error"))?;
    let mut out = Vec::new();
    for p in 1..=total {
        let b = if p == 1 {
            first.clone()
        } else {
            api(
                r,
                v,
                &format!("/comics/{}/eps?page={}", mid, p),
                "GET",
                None,
            )?
        };
        let eps = &b["data"]["eps"];
        if eps["page"] != p || eps["pages"] != total {
            return Err("parse_error".into());
        }
        for n in array(eps, "docs")? {
            let order = n["order"]
                .as_u64()
                .filter(|n| *n <= i32::MAX as u64)
                .ok_or_else(|| E::from("parse_error"))?;
            let mut c = json!({"id":format!("picacomic-read:{}:{}",mid,order),"mangaId":format!("picacomic:{}",mid),"title":field(n,"title")?,"chapterNumber":order,"index":out.len(),"url":format!("{}/comic/reader/{}/{}",BASE,mid,order)});
            if let Some(t) = utc_time(field(n, "updated_at")?) {
                c["dateUpload"] = json!(t)
            }
            out.push(c)
        }
    }
    Ok(json!({"chapters":out,"page":{"hasMore":false,"nextCursor":null}}))
}
fn pages(r: &[u8], v: &Value) -> R<Value> {
    let (mid, order) = chapter_id(v)?;
    let route = format!("/comics/{}/order/{}/pages", mid, order);
    let first = api(r, v, &format!("{}?page=1", route), "GET", None)?;
    let total = first["data"]["pages"]["pages"]
        .as_u64()
        .filter(|n| *n >= 1 && *n <= 100000)
        .ok_or_else(|| E::from("parse_error"))?;
    let mut out = Vec::new();
    for p in 1..=total {
        let b = if p == 1 {
            first.clone()
        } else {
            api(r, v, &format!("{}?page={}", route, p), "GET", None)?
        };
        let pages = &b["data"]["pages"];
        if pages["page"] != p || pages["pages"] != total {
            return Err("parse_error".into());
        }
        let limit = pages["limit"]
            .as_u64()
            .filter(|n| *n > 0 && *n <= 100000)
            .ok_or_else(|| E::from("parse_error"))?;
        for (i, n) in array(pages, "docs")?.iter().enumerate() {
            let index = i as u64 + (p - 1) * limit;
            out.push(json!({"id":format!("picacomic-page:{}:{}:{}",mid,order,index),"index":index,"image":{"kind":"url","url":image_url(&n["media"])?}}))
        }
    }
    if out.is_empty() {
        return Err("chapter_unavailable".into());
    }
    Ok(json!({"pages":out}))
}
fn image_request(r: &[u8], v: &Value) -> R<Value> {
    let raw = v["args"]["pageId"].as_str().ok_or_else(|| E::from("invalid_request"))?;
    let (mid, chapter) = if let Some(raw) = raw.strip_prefix("picacomic-page:") {
        let parts = raw.split(':').collect::<Vec<_>>();
        if parts.len() != 3 {
            return Err("invalid_request".into());
        }
        let mid = id(parts[0])?;
        let order = order(parts[1])?;
        let index = parts[2].parse::<u64>().map_err(|_| E::from("invalid_request"))?;
        if index.to_string() != parts[2] {
            return Err("invalid_request".into());
        }
        let chapter = format!("picacomic-read:{}:{}", mid, order);
        (mid, Some(chapter))
    } else if let Some(raw) = raw.strip_prefix("picacomic:") {
        // Koma passes the manga ID for list/detail cover requests.
        (id(raw)?, None)
    } else {
        return Err("invalid_request".into());
    };
    if let Some(m) = v["args"].get("mangaId") {
        if m.as_str() != Some(&format!("picacomic:{}", mid)) {
            return Err("invalid_request".into());
        }
    }
    if let Some(c) = v["args"].get("chapterId") {
        if chapter.is_none() || c.as_str() != chapter.as_deref() {
            return Err("invalid_request".into());
        }
    }
    if let Some(m) = v["args"].get("mangaUrl") {
        if by_url(m.as_str().ok_or_else(|| E::from("invalid_request"))?)? != mid {
            return Err("invalid_request".into());
        }
    }
    let raw = v["args"]["url"]
        .as_str()
        .ok_or_else(|| E::from("invalid_request"))?;
    let u = Url::parse(raw).map_err(|_| E::from("invalid_request"))?;
    if !matches!(u.scheme(), "http" | "https") || !u.username().is_empty() || u.password().is_some()
    {
        return Err("invalid_request".into());
    }
    // The Host contract scopes the DNS override to picacomic.com and its true
    // subdomains. Other opaque image CDNs keep the existing system DNS path.
    let mut descriptor = json!({"url":raw,"headers":{"Referer":format!("{}/",BASE)}});
    if u.host_str()
        .is_some_and(|host| host == "picacomic.com" || host.ends_with(".picacomic.com"))
    {
        let mut options = Options::new(v)?;
        if options.channel == "2" {
            options.require_transport(r)?;
            if u.scheme() != "https" {
                return Err("invalid_request".into());
            }
            require(r)?;
            Session::existing()?;
            options.transport(r)?;
            options.apply_transport(&mut descriptor);
        }
    }
    Ok(descriptor)
}
const CATEGORIES: [&str; 40] = [
    "大家都在看",
    "牛牛不哭",
    "那年今天",
    "官方都在看",
    "嗶咔漢化",
    "全彩",
    "長篇",
    "同人",
    "短篇",
    "圓神領域",
    "碧藍幻想",
    "CG雜圖",
    "純愛",
    "百合花園",
    "後宮閃光",
    "單行本",
    "姐姐系",
    "妹妹系",
    "SM",
    "人妻",
    "NTR",
    "強暴",
    "艦隊收藏",
    "Love Live",
    "SAO 刀劍神域",
    "Fate",
    "東方",
    "禁書目錄",
    "Cosplay",
    "英語 ENG",
    "生肉",
    "性轉換",
    "足の恋",
    "非人類",
    "耽美花園",
    "偽娘哲學",
    "扶他樂園",
    "重口地帶",
    "歐美",
    "WEBTOON",
];
fn filters() -> Value {
    let mut cats = alloc::vec![json!({"label":"全部","value":""})];
    for s in CATEGORIES {
        cats.push(json!({"value":s,"label":s}))
    }
    json!({"filters":[{"id":"sort","name":"排序","type":"select","default":"dd","options":[{"value":"dd","label":"新到旧"},{"value":"da","label":"旧到新"},{"value":"ld","label":"最多爱心"},{"value":"vd","label":"最多绅士指名"}]},{"id":"category","name":"类型","type":"select","default":"","options":cats},{"id":"rank","name":"榜单","type":"select","default":"","options":[{"value":"","label":"无"},{"value":"H24","label":"过去24小时最热门"},{"value":"D7","label":"过去7天最热门"},{"value":"D30","label":"过去30天最热门"}]},{"id":"favourite","name":"收藏列表","type":"check","default":false}]})
}
fn settings() -> Value {
    json!({"settings":[
     {"id":"BLOCK_GENRES","name":"屏蔽词列表","kind":"text","default":"","description":"根据关键词过滤漫画，关键词之间用','分离。关键词分为分类和标签两种，在热门和最新中只能按分类过滤（即在filter的类型中出现的词），而在搜索中两者都可以"},
     {"id":"IMAGE_QUALITY","name":"图片质量","kind":"select","default":"original","options":[{"value":"original","label":"原图"},{"value":"low","label":"低"},{"value":"medium","label":"中"},{"value":"high","label":"高"}]},
     {"id":"APP_CHANNEL","name":"分流","kind":"select","default":"2","options":[{"value":"1","label":"1"},{"value":"2","label":"2"}]},
     {"id":"APP_CHANNEL_URL","name":"分流url","kind":"text","default":"","description":"自定义用于获取分流2的目标地址；分流1不受影响；（如果之前获取成功了需要重启才能生效，如果出现超时可以多重试几次）"}
    ]})
}
fn dispatch(op: &str, r: &[u8]) -> R<Value> {
    protocol::request(r).map_err(E::from)?;
    let args = protocol::direct(r, b"args").map_err(E::from)?.unwrap_or(r);
    let mut buf = [0; 16384];
    let _ = protocol::text(args, b"mangaUrl", &mut buf).map_err(E::from)?;
    let v: Value = serde_json::from_slice(r).map_err(|_| E::from("invalid_request"))?;
    match op {
        "get_listings" => Ok(
            json!({"listings":[{"id":"random","name":"最新","supportsPaging":true},{"id":"popular","name":"热门","supportsPaging":true}]}),
        ),
        "get_filters" => Ok(filters()),
        "get_settings" => Ok(settings()),
        "get_authentication" => authentication::status(r),
        "authenticate" => authentication::authenticate(r, &v),
        "logout" => authentication::logout(r),
        "search" => list(r, &v, true),
        "get_manga_list" => list(r, &v, false),
        "get_manga" => Ok(json!({"manga":detail(r,&v,&manga_id(&v)?)?})),
        "get_chapters" => chapters(r, &v),
        "get_pages" => pages(r, &v),
        "get_image_request" => image_request(r, &v),
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
koma_source_sdk::koma_source_exports!("picacomic_source");
koma_source_sdk::koma_source_authentication_exports!("picacomic_source");
