#![no_std]
extern crate alloc;
// Independent Apache2 port, coffee522/extensions-copymanga 655896cce1dd8a235c10d4c9d4b370d7f265c651.
// No live provider requests are part of the current frozen-body development.
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
use regex::Regex;
use serde_json::{json, Value};
use url::Url;
mod dom;
mod literal;
mod operations;
mod simplify;
use dom::Node;
#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.copymanga.koma",
    name: "拷贝漫画",
    version: "0.1.2",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "拷贝漫画。支持公开列表和漫画阅读。",
    content_rating: "unknown",
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
                "forbidden" => "网站暂不接受当前书源请求，请打开原网页查看，或稍后重试。",
                "network_error" => "网络传输未完成，请稍后重试。",
                "not_found" => "拷贝未提供此内容，请打开原网页确认。",
                "rate_limit" => "请求过于频繁，请稍后重试。",
                "unsupported" => "当前系统或应用不支持拷贝漫画的 API 请求方式。请更新后重试。",
                _ => "拷贝返回异常，请稍后重试或打开原网页确认。",
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
fn encoded(s: &str) -> String {
    let mut b = alloc::vec![0;s.len()*3+1];
    let mut n = 0;
    write_url_encoded(&mut b, &mut n, s.as_bytes());
    b.truncate(n);
    String::from_utf8(b).unwrap_or_default()
}
fn path_id(raw: &str) -> R<String> {
    let decoded = percent_encoding::percent_decode_str(raw)
        .decode_utf8()
        .map_err(|_| E::from("invalid_request"))?;
    if decoded.is_empty()
        || matches!(decoded.as_ref(), "." | "..")
        || decoded
            .chars()
            .any(|c| c.is_control() || c == '/' || c == '\\')
        || encoded(&decoded) != raw
    {
        return Err("invalid_request".into());
    }
    Ok(raw.to_string())
}
const DOMAINS: [(&str, &str); 3] = [
    ("https://api.mangacopy.com", "https://www.mangacopy.com"),
    ("https://api.copy3000.com", "https://www.copy3000.com"),
    ("https://api.copy4000.com", "https://www.copy4000.com"),
];
const DEFAULT_UA: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/114.0";

// Only the group dictionary has observable object-member order. Deserialize its
// values through Serde's streaming visitor; ordinary Value maps sort keys.
struct GroupOrder(Vec<Value>);
impl<'de> serde::Deserialize<'de> for GroupOrder {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> serde::de::Visitor<'de> for V {
            type Value = GroupOrder;
            fn expecting(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.write_str("ordered chapter group object")
            }
            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                mut map: M,
            ) -> Result<GroupOrder, M::Error> {
                let mut rows = Vec::new();
                let mut seen = alloc::collections::BTreeSet::new();
                while let Some((key, value)) = map.next_entry::<String, Value>()? {
                    if !seen.insert(key) {
                        return Err(serde::de::Error::custom("duplicate group key"));
                    }
                    rows.push(value)
                }
                Ok(GroupOrder(rows))
            }
        }
        d.deserialize_map(V)
    }
}
struct ApiData {
    json: Value,
    groups: Option<Vec<Value>>,
}
impl core::ops::Deref for ApiData {
    type Target = Value;
    fn deref(&self) -> &Value {
        &self.json
    }
}
struct Context {
    api: &'static str,
    api_transport: bool,
    public_cover_headers: bool,
    web: &'static str,
    ua: String,
    simplifier: Option<simplify::Simplifier>,
}
impl Context {
    fn new(v: &Value, metadata: bool, api_transport: bool, public_cover_headers: bool) -> R<Self> {
        let values = if metadata {
            v["args"]
                .get("settingValues")
                .filter(|v| v.is_object())
                .unwrap_or(&v["settings"])
        } else {
            &v["settings"]
        };
        let selected = setting(values, "domain", DOMAINS[0].0)?;
        let (api, web) = DOMAINS
            .iter()
            .find(|(a, _)| *a == selected)
            .copied()
            .unwrap_or(DOMAINS[0]);
        let ua = setting(values, "user_agent", DEFAULT_UA)?.trim();
        let ua = if ua.is_empty() { DEFAULT_UA } else { ua };
        if ua.chars().any(char::is_control) {
            return Err("invalid_request".into());
        }
        let simplified = setting(values, "display_language", "traditional")? == "simplified";
        Ok(Self {
            api,
            api_transport,
            public_cover_headers,
            web,
            ua: ua.to_string(),
            simplifier: if simplified {
                Some(simplify::Simplifier::new()?)
            } else {
                None
            },
        })
    }
    fn image_headers(&self) -> Value {
        json!({"User-Agent":self.ua,"Referer":format!("{}/",self.web),"Accept":"application/json","Version":"2025.11.21","Webp":"1","platform":"1"})
    }
    fn display(&self, s: &str) -> String {
        self.simplifier
            .as_ref()
            .map(|c| c.convert(s))
            .unwrap_or_else(|| s.to_string())
    }
    fn http(&self, url: &str, is_api: bool) -> R<(u16, Vec<u8>)> {
        if is_api && !self.api_transport {
            return Err("unsupported".into());
        }
        if host::check_cancel() {
            return Err("cancelled".into());
        }
        let mut headers = json!({"User-Agent":self.ua,"Referer":format!("{}/",self.web),"Accept":if is_api{"application/json"}else{"text/html"},"Cache-Control":"no-cache"});
        if is_api {
            headers["Version"] = json!("2025.11.21");
            headers["Webp"] = json!("1");
            headers["platform"] = json!("1");
        }
        let b=serde_json::to_vec(&json!({"version":1,"url":url,"method":"GET","headers":headers,"responseKind":"bodyText"})).map_err(|_|E::from("request_too_large"))?;
        let n = host::http_request(&b, http_out()).map_err(|_| E::from("network_error"))?;
        let status = protocol::direct(&http_out()[..n], b"status")
            .map_err(|_| E::from("network_error"))?
            .and_then(|s| core::str::from_utf8(s).ok())
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(0);
        if is_api && status == 210 {
            return Err("forbidden".into());
        }
        let n = protocol::body(&http_out()[..n], body_buf()).map_err(E::from)?;
        Ok((status, body_buf()[..n].to_vec()))
    }
    fn api(&self, path: &str, pairs: &[(&str, String)]) -> R<ApiData> {
        let query = pairs
            .iter()
            .map(|(k, v)| format!("{}={}", encoded(k), encoded(v)))
            .collect::<Vec<_>>()
            .join("&");
        let url = format!(
            "{}/api/v3/{}{}",
            self.api,
            path,
            if query.is_empty() {
                String::new()
            } else {
                format!("?{}", query)
            }
        );
        let (status, b) = self.http(&url, true)?;
        let payload: Value = serde_json::from_slice(&b).map_err(|_| E::from("parse_error"))?;
        if payload["code"] == 210 {
            return Err("forbidden".into());
        }
        if status != 200 {
            return Err("http_error".into());
        }
        if payload["code"] != 200 {
            return Err("server_error".into());
        }
        let data = protocol::direct(&b, b"results")
            .map_err(|_| E::from("parse_error"))?
            .ok_or_else(|| E::from("parse_error"))?;
        if data == b"null" {
            return Err("parse_error".into());
        }
        let json: Value = serde_json::from_slice(data).map_err(|_| E::from("parse_error"))?;
        let groups = match protocol::direct(data, b"groups").map_err(|_| E::from("parse_error"))? {
            Some(raw) if raw != b"null" => Some(
                serde_json::from_slice::<GroupOrder>(raw)
                    .map_err(|_| E::from("parse_error"))?
                    .0,
            ),
            _ => None,
        };
        Ok(ApiData { json, groups })
    }
}
fn manga_id(v: &Value) -> R<String> {
    path_id(
        v["args"]["mangaId"]
            .as_str()
            .and_then(|s| s.strip_prefix("copymanga:"))
            .ok_or_else(|| E::from("invalid_request"))?,
    )
}
fn chapter_id(v: &Value) -> R<(String, String)> {
    let raw = v["args"]["chapterId"]
        .as_str()
        .and_then(|s| s.strip_prefix("copymanga-read:"))
        .ok_or_else(|| E::from("invalid_request"))?;
    let (a, c) = raw
        .split_once(':')
        .ok_or_else(|| E::from("invalid_request"))?;
    let a = path_id(a)?;
    let c = path_id(c)?;
    if let Some(m) = v["args"].get("mangaId") {
        if m.as_str() != Some(&format!("copymanga:{}", a)) {
            return Err("invalid_request".into());
        }
    }
    Ok((a, c))
}
fn by_url(raw: &str) -> R<String> {
    let u = Url::parse(raw).map_err(|_| E::from("invalid_request"))?;
    if !matches!(u.scheme(), "http" | "https") || !u.username().is_empty() || u.password().is_some()
    {
        return Err("invalid_request".into());
    }
    let host = u.host_str().unwrap_or("");
    if !DOMAINS.iter().any(|(_, w)| {
        let h = w.trim_start_matches("https://");
        h == host || h.trim_start_matches("www.") == host
    }) {
        return Err("invalid_request".into());
    }
    let parts = u
        .path_segments()
        .ok_or_else(|| E::from("invalid_request"))?
        .collect::<Vec<_>>();
    let pos = parts
        .iter()
        .position(|p| *p == "comic" || *p == "comic2")
        .ok_or_else(|| E::from("invalid_request"))?;
    let slug = parts
        .get(pos + 1)
        .ok_or_else(|| E::from("invalid_request"))?;
    let decoded = percent_encoding::percent_decode_str(slug)
        .decode_utf8()
        .map_err(|_| E::from("invalid_request"))?;
    path_id(&encoded(&decoded))
}
fn comic(cx: &Context, v: &Value) -> R<Value> {
    let path = encoded(field(v, "path_word")?);
    path_id(&path).map_err(|_| E::from("parse_error"))?;
    let mut authors = Vec::new();
    for a in array(v, "author")? {
        authors.push(cx.display(field(a, "name")?))
    }
    let status_text = v["status"]["display"].as_str().unwrap_or("");
    let status_value = v["status"]["value"].as_i64();
    let status = if status_text.contains("完结") || status_text.contains("完結") {
        "completed"
    } else if status_text.contains("连载")
        || status_text.contains("連載")
        || status_value == Some(0)
    {
        "ongoing"
    } else if matches!(status_value, Some(1) | Some(2)) {
        "completed"
    } else {
        "unknown"
    };
    let cover_uri = field(v, "cover")?;
    let mut manga = json!({"id":format!("copymanga:{}",path),"title":cx.display(field(v,"name")?),"authors":authors,"artists":authors,"cover":{"kind":"url","url":cover_uri},"contentRating":"unknown","status":status,"links":[{"kind":"source","url":format!("{}/comic/{}",cx.web,path)}]});
    if cx.public_cover_headers && image_uri_allowed(cover_uri) {
        manga["cover"]["headers"] = cx.image_headers();
    }
    if let Some(s) = v["brief"].as_str() {
        manga["description"] = json!(cx.display(s))
    }
    let mut tags = Vec::new();
    if let Some(s) = v["region"]["display"].as_str() {
        tags.push(cx.display(s))
    }
    if let Some(a) = v["theme"].as_array() {
        for t in a {
            tags.push(cx.display(field(t, "name")?))
        }
    }
    manga["tags"] = json!(tags);
    Ok(manga)
}
fn image_uri_allowed(raw: &str) -> bool {
    Url::parse(raw).is_ok_and(|url| {
        matches!(url.scheme(), "http" | "https") && url.username().is_empty() && url.password().is_none()
    })
}
fn date(s: &str) -> Option<i64> {
    let s = s.get(..10)?;
    let b = s.as_bytes();
    if b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let y = s.get(..4)?.parse::<i64>().ok()?;
    let mo = s.get(5..7)?.parse::<i64>().ok()?;
    let d = s.get(8..10)?.parse::<i64>().ok()?;
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
    if !(1..=12).contains(&mo) || d < 1 || d > dm[(mo - 1) as usize] {
        return None;
    }
    let yy = y - if mo <= 2 { 1 } else { 0 };
    let era = yy.div_euclid(400);
    let yo = yy - era * 400;
    let mp = mo + if mo > 2 { -3 } else { 9 };
    Some(
        ((era * 146097 + yo * 365 + yo / 4 - yo / 100 + (153 * mp + 2) / 5 + d - 1 - 719468)
            * 86400
            - 8 * 3600)
            * 1000,
    )
}
fn run(op: &str, r: &[u8]) -> u32 {
    let result = (|| {
        protocol::request(r).map_err(E::from)?;
        let a = protocol::direct(r, b"args").map_err(E::from)?.unwrap_or(r);
        let mut b = [0; 16384];
        let _ = protocol::text(a, b"mangaUrl", &mut b).map_err(E::from)?;
        let v: Value = serde_json::from_slice(r).map_err(|_| E::from("invalid_request"))?;
        operations::dispatch(op, r, &v)
    })();
    match result {
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
koma_source_sdk::koma_source_exports!("copymanga_source");
