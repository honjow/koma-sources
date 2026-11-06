#![no_std]
extern crate alloc;
// Independent Rust port of the complete Keiyoushi Jinmantiantang source,
// Apache-2.0, de743aed846da53e00c45ab26ee0e77f7bb4355b (extension v64).
// This is the real JM API source, NOT the historical hanman18 source.
use aes::cipher::{BlockDecrypt, KeyInit};
use alloc::{
    collections::BTreeSet,
    format,
    string::{String, ToString},
    vec::Vec,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use koma_source_sdk::{
    host,
    json_utils::*,
    source::{SourceCapabilities, SourceInfo},
    source_protocol as protocol,
};
use md5::{Digest, Md5};
use serde_json::{json, Value};
#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.jmcomic.koma",
    name: "禁漫天堂",
    version: "0.1.1",
    api_version: "0.2",
    language: "zh",
    author: "Koma",
    description: "禁漫天堂 JM 漫画。支持匿名浏览与阅读，账号功能在原站使用。",
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
koma_source_sdk::koma_source_buffers! {payload:1024*1024,http_out:4*1024*1024,body:3*1024*1024,http_req:32768,scratch:8192,}
koma_source_sdk::koma_source_helpers!();
const MIRRORS: [&str; 4] = [
    "18comic.vip",
    "18comic.ink",
    "jmcomic-zzz.one",
    "jmcomic-zzz.org",
];
fn cover_url(mirror: &str, aid: &str) -> String {
    format!("https://cdn-msp.{}/media/albums/{}_3x4.jpg", mirror, aid)
}
const FALLBACK: [&str; 4] = [
    "www.cdnhjk.net",
    "www.cdnutc.cc",
    "www.cdngwc.net",
    "www.cdngwc.club",
];
const SERVER_URL: &str = "https://rup4a04-c02.tos-cn-hongkong.bytepluses.com/newsvr-2025.txt";
const SERVER_SECRET: &str = "diosfjckwpqpdfjkvnqQjsik";
const TOKEN_SECRET: &str = "185Hcomic3PAPP7R";
const APP_VERSION: &str = "2.1.12";
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0";
type R<T> = Result<T, E>;
struct E {
    code: &'static str,
    message: String,
}
impl From<&'static str> for E {
    fn from(code: &'static str) -> Self {
        Self {
            code: match code {
                "host_feature_required" => "unsupported",
                "account_site_only" => "auth_required",
                "capacity_exceeded" => "result_too_large",
                _ => code,
            },
            message: match code {
                "account_site_only" => "收藏夹和签到需要登录，请在原站使用",
                "auth_required" => "禁漫要求登录，请在原站确认访问权限",
                "forbidden" => "禁漫拒绝了本次请求，请在原网页确认访问状态",
                "rate_limit" => "请求过于频繁，请稍后重试",
                "not_found" => "禁漫未提供此作品，请在原网页确认是否已移除",
                "chapter_unavailable" => "禁漫未提供此章节，请在原网页确认",
                "network_error" => "网络传输未完成，请稍后重试",
                "host_feature_required" => "当前应用缺少此书源所需能力，请更新应用",
                _ => "禁漫服务器返回异常，请稍后重试",
            }
            .to_string(),
        }
    }
}
fn bad<T>() -> R<T> {
    Err("parse_error".into())
}
fn strv<'a>(v: &'a Value, k: &str) -> R<&'a str> {
    v.get(k)
        .and_then(Value::as_str)
        .ok_or_else(|| E::from("parse_error"))
}
fn ary<'a>(v: &'a Value, k: &str) -> R<&'a Vec<Value>> {
    v.get(k)
        .and_then(Value::as_array)
        .ok_or_else(|| E::from("parse_error"))
}
fn number_id(v: &Value) -> R<String> {
    let s = if let Some(s) = v.as_str() {
        s.to_string()
    } else if let Some(n) = v.as_u64() {
        n.to_string()
    } else {
        return bad();
    };
    if s.is_empty() || s.len() > 20 || !s.bytes().all(|b| b.is_ascii_digit()) {
        return bad();
    }
    Ok(s)
}
fn json_value(b: &[u8]) -> R<Value> {
    serde_json::from_slice(b).map_err(|_| "parse_error".into())
}
fn hex_md5(b: &[u8]) -> String {
    let sum = Md5::digest(b);
    let mut out = String::with_capacity(32);
    for b in sum {
        out.push(b"0123456789abcdef"[(b >> 4) as usize] as char);
        out.push(b"0123456789abcdef"[(b & 15) as usize] as char)
    }
    out
}
fn decrypt(text: &str, secret: &str) -> R<Vec<u8>> {
    let mut b = STANDARD
        .decode(
            text.bytes()
                .filter(|c| !c.is_ascii_whitespace())
                .collect::<Vec<_>>(),
        )
        .map_err(|_| E::from("parse_error"))?;
    if b.is_empty() || b.len() % 16 != 0 {
        return bad();
    }
    let key = hex_md5(secret.as_bytes());
    let aes = aes::Aes256::new_from_slice(key.as_bytes()).map_err(|_| E::from("parse_error"))?;
    for block in b.chunks_exact_mut(16) {
        aes.decrypt_block(block.into())
    }
    let pad = *b.last().ok_or_else(|| E::from("parse_error"))? as usize;
    if pad == 0
        || pad > 16
        || pad > b.len()
        || !b[b.len() - pad..].iter().all(|x| *x as usize == pad)
    {
        return bad();
    }
    b.truncate(b.len() - pad);
    Ok(b)
}
fn encoded(s: &str) -> String {
    let mut b = alloc::vec![0;s.len()*3+1];
    let mut n = 0;
    write_url_encoded(&mut b, &mut n, s.as_bytes());
    b.truncate(n);
    String::from_utf8(b).unwrap_or_default()
}
fn rpc(value: Value) -> R<Value> {
    let b = serde_json::to_vec(&value).map_err(|_| E::from("capacity_exceeded"))?;
    let n = host::http_request(&b, http_out()).map_err(|_| E::from("network_error"))?;
    let v = json_value(&http_out()[..n])?;
    if v.get("ok") != Some(&Value::Bool(true)) {
        return Err("network_error".into());
    }
    Ok(v)
}
fn clock() -> R<String> {
    let v = rpc(json!({"type":"private_session","version":1,"op":"clock"}))?;
    let ms = strv(&v, "nowMs")?;
    let n = ms.parse::<u64>().map_err(|_| E::from("network_error"))?;
    Ok((n / 1000).to_string())
}
fn http(url: &str, headers: Value) -> R<Vec<u8>> {
    if host::check_cancel() {
        return Err("cancelled".into());
    }
    let request = serde_json::to_vec(&json!({"url":url,"method":"GET","headers":headers}))
        .map_err(|_| E::from("capacity_exceeded"))?;
    let n = host::http_request(&request, http_out()).map_err(|_| E::from("network_error"))?;
    let n = protocol::body(&http_out()[..n], body_buf()).map_err(E::from)?;
    Ok(body_buf()[..n].to_vec())
}
fn valid_host(h: &str) -> bool {
    h.len() > 3
        && h.len() < 254
        && h.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
        && !h.starts_with('.')
        && !h.ends_with('.')
        && !h.contains("..")
}
struct Context {
    host: String,
    tag: String,
    mirror: String,
    block: Vec<String>,
}
impl Context {
    fn new(r: &[u8], input: &Value) -> R<Self> {
        if !protocol::feature(r, b"source_rate_limit_jm_v1") {
            return Err("host_feature_required".into());
        }
        if !protocol::feature(r, b"source_private_session_v1") {
            return Err("host_feature_required".into());
        }
        let s = &input["settings"];
        let idx = s["preferred_mirror"]
            .as_str()
            .unwrap_or("0")
            .parse::<usize>()
            .map_err(|_| E::from("invalid_request"))?;
        let mirror = (*MIRRORS.get(idx).ok_or_else(|| E::from("invalid_request"))?).to_string();
        let block = s["BLOCK_GENRES_LIST"]
            .as_str()
            .unwrap_or("")
            .split("//")
            .next()
            .unwrap_or("")
            .trim()
            .to_lowercase()
            .split(' ')
            .map(ToString::to_string)
            .collect();
        let v = rpc(json!({"type":"private_session","version":1,"op":"begin"}))?;
        match strv(&v, "state")? {
            "ready" => {
                let state = json_value(strv(&v, "value")?.as_bytes())?;
                if state["v"] != 1 {
                    return bad();
                }
                let h = strv(&state, "apiHost")?;
                if !valid_host(h) {
                    return bad();
                }
                Ok(Self {
                    host: h.to_string(),
                    tag: strv(&v, "versionTag")?.to_string(),
                    mirror,
                    block,
                })
            }
            "owner" => {
                let lease = strv(&v, "lease")?.to_string();
                let result = (|| {
                    let hosts=match http(SERVER_URL,json!({"User-Agent":UA,"Referer":format!("https://{}/",mirror),"Origin":format!("https://{}",mirror),"Accept":"*/*","Sec-Fetch-Site":"cross-site","Sec-Fetch-Mode":"cors","Sec-Fetch-Dest":"empty"})).and_then(|v|{let t=core::str::from_utf8(&v).map_err(|_|E::from("parse_error"))?.trim_start_matches('\u{feff}');let d=decrypt(t,SERVER_SECRET)?;let j=json_value(&d)?;let a=ary(&j,"Server")?;let mut out=Vec::new();for x in a{let h=x.as_str().ok_or_else(||E::from("parse_error"))?;if !valid_host(h){return bad()}out.push(h.to_string())}if out.is_empty(){return bad()}Ok(out)}){Ok(h)=>h,Err(e) if e.code!="cancelled"=>FALLBACK.iter().map(|h|h.to_string()).collect(),Err(e)=>return Err(e)};
                    let random = strv(&v, "randomHex")?;
                    let index = u64::from_str_radix(
                        random.get(..8).ok_or_else(|| E::from("network_error"))?,
                        16,
                    )
                    .map_err(|_| E::from("network_error"))?
                        as usize
                        % hosts.len();
                    let h = hosts[index].clone();
                    let cache = serde_json::to_string(&json!({"v":1,"apiHost":h}))
                        .map_err(|_| E::from("capacity_exceeded"))?;
                    let done = rpc(
                        json!({"type":"private_session","version":1,"op":"commit","lease":lease,"value":cache}),
                    )?;
                    Ok(Self {
                        host: h,
                        tag: strv(&done, "versionTag")?.to_string(),
                        mirror,
                        block,
                    })
                })();
                if result.is_err() {
                    let _ = rpc(
                        json!({"type":"private_session","version":1,"op":"abort","lease":lease}),
                    );
                }
                result
            }
            _ => bad(),
        }
    }
    fn api(&self, path: &str, pairs: &[(&str, String)]) -> R<Value> {
        let stamp = clock()?;
        let mut url = format!("https://{}/{}?", self.host, path);
        for (k, v) in pairs {
            url.push_str(&encoded(k));
            url.push('=');
            url.push_str(&encoded(v));
            url.push('&')
        }
        url.push_str("lang=CN");
        let token = hex_md5(format!("{}{}", stamp, TOKEN_SECRET).as_bytes());
        let bytes = match http(
            &url,
            json!({"User-Agent":UA,"Referer":format!("https://{}/",self.mirror),"Origin":format!("https://{}",self.mirror),"Accept":"*/*","Sec-Fetch-Site":"cross-site","Sec-Fetch-Mode":"cors","Sec-Fetch-Dest":"empty","Token":token,"Tokenparam":format!("{},{}",stamp,APP_VERSION)}),
        ) {
            Ok(b) => b,
            Err(e) => {
                if e.code == "network_error" {
                    let _ = rpc(
                        json!({"type":"private_session","version":1,"op":"invalidate","versionTag":self.tag}),
                    );
                }
                return Err(e);
            }
        };
        let response = json_value(&bytes).map_err(|_| E::from("server_error"))?;
        let code = response["code"]
            .as_i64()
            .ok_or_else(|| E::from("parse_error"))?;
        if code != 200 {
            let kind = match code {
                401 => "auth_required",
                403 => "forbidden",
                404 => "not_found",
                429 => "rate_limit",
                500..=599 => "server_error",
                _ => "client_error",
            };
            return Err(E {
                code: kind,
                message: response["errorMsg"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .unwrap_or("禁漫服务器返回异常，请稍后重试")
                    .to_string(),
            });
        }
        let data = response
            .get("data")
            .filter(|v| !v.is_null())
            .ok_or_else(|| E::from("parse_error"))?;
        if let Some(t) = data.as_str() {
            json_value(&decrypt(t, &format!("{}{}", stamp, TOKEN_SECRET))?)
        } else {
            Ok(data.clone())
        }
    }
    fn album(&self, id: &str) -> R<Option<Value>> {
        let a = self.api("album", &[("id", id.to_string())])?;
        if a.is_array() {
            if a.as_array().unwrap().is_empty() {
                return Ok(None);
            }
            return bad();
        }
        if !a.is_object() {
            return bad();
        }
        let got = number_id(&a["id"])?;
        if got != id {
            return bad();
        }
        Ok(Some(a))
    }
    fn canonical(&self, id: &str) -> R<Option<Value>> {
        let Some(a) = self.album(id)? else {
            return Ok(None);
        };
        let sid = number_id(&a["series_id"])?;
        if sid != "0" && sid != id {
            self.album(&sid)
        } else {
            Ok(Some(a))
        }
    }
    fn cover(&self, id: &str) -> String {
        cover_url(&self.mirror, id)
    }
    fn card(&self, v: &Value) -> R<Value> {
        let id = number_id(&v["id"])?;
        let title = entities(strv(v, "name")?)?;
        if title.trim().is_empty() {
            return bad();
        }
        let mut tags = Vec::new();
        for k in ["category", "category_sub"] {
            if let Some(t) = v[k]["title"].as_str() {
                if !tags.contains(&t) {
                    tags.push(t)
                }
            }
        }
        Ok(
            json!({"id":format!("jmcomic:{}",id),"title":title,"authors":v["author"].as_str().map(|s|alloc::vec![s]).unwrap_or_default(),"tags":tags,"cover":{"kind":"url","url":self.cover(&id)},"url":format!("https://{}/album/{}/",self.mirror,id),"contentRating":"nsfw"}),
        )
    }
    fn blocked(&self, v: &Value) -> bool {
        let tags = v["tags"].as_array();
        tags.map(|a| {
            a.iter().filter_map(Value::as_str).any(|s| {
                self.block
                    .iter()
                    .any(|b| !b.is_empty() && *b == s.to_lowercase())
            })
        })
        .unwrap_or(false)
    }
}
fn entities(s: &str) -> R<String> {
    // Attribute parsing decodes entities without collapsing title whitespace or
    // interpreting literal title markup (Parser.unescapeEntities in the reference).
    let text = s.replace('"', "&quot;").replace('<', "&lt;");
    let h = format!("<i data-t=\"{}\"></i>", text);
    let d = host::html_parse(h.as_bytes()).map_err(|_| E::from("parse_error"))?;
    let node = match host::html_select(d, b"i") {
        Ok(n) => n,
        Err(_) => {
            let _ = host::html_close(d);
            return Err("parse_error".into());
        }
    };
    let mut b = alloc::vec![0;65536];
    let n = host::html_attr(node, b"data-t", &mut b);
    let _ = host::html_close(node);
    let _ = host::html_close(d);
    let n = n.map_err(|_| E::from("parse_error"))?;
    if n >= b.len() {
        return Err("capacity_exceeded".into());
    }
    b.truncate(n);
    String::from_utf8(b).map_err(|_| "parse_error".into())
}
fn req_id(input: &Value, key: &str, prefix: &str) -> R<String> {
    let id = input["args"][key]
        .as_str()
        .and_then(|s| s.strip_prefix(prefix))
        .ok_or_else(|| E::from("invalid_request"))?;
    number_id(&Value::String(id.to_string())).map_err(|_| "invalid_request".into())
}
fn strings(v: &Value, key: &str) -> R<Vec<String>> {
    match v.get(key) {
        None => Ok(Vec::new()),
        Some(Value::Array(a)) => a
            .iter()
            .map(|s| {
                s.as_str()
                    .map(ToString::to_string)
                    .ok_or_else(|| E::from("parse_error"))
            })
            .collect(),
        _ => bad(),
    }
}
fn detail(c: &Context, a: &Value) -> R<Value> {
    let id = number_id(&a["id"])?;
    let tags = strings(a, "tags")?;
    let state = if tags.iter().any(|s| s == "连载中" || s == "连载") {
        "ongoing"
    } else if tags.iter().any(|s| s == "完结") {
        "completed"
    } else {
        "unknown"
    };
    let mut description = a["description"].as_str().unwrap_or("").trim().to_string();
    for (key, label) in [("works", "作品"), ("actors", "登场人物")] {
        let list = strings(a, key)?;
        if !list.is_empty() {
            if !description.is_empty() {
                description.push_str("\n\n")
            }
            description.push_str(&format!("**{}:**", label));
            for s in list {
                description.push_str("\n- ");
                description.push_str(&s)
            }
        }
    }
    Ok(
        json!({"id":format!("jmcomic:{}",id),"title":entities(strv(a,"name")?)?,"authors":strings(a,"author")?,"description":description,"status":state,"tags":tags.iter().filter(|s|s.as_str()!="连载中"&&s.as_str()!="连载"&&s.as_str()!="完结").collect::<Vec<_>>(),"cover":{"kind":"url","url":c.cover(&id)},"url":format!("https://{}/album/{}/",c.mirror,id),"contentRating":"nsfw"}),
    )
}
fn chapter_ids(a: &Value) -> R<Vec<(String, String, Option<i64>)>> {
    let id = number_id(&a["id"])?;
    let series = strings_or_series(a)?;
    let stamp = a["addtime"]
        .as_str()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0)
        .checked_mul(1000)
        .ok_or_else(|| E::from("parse_error"))?;
    if series.is_empty() {
        return Ok(alloc::vec![(id, "单章节".to_string(), Some(stamp))]);
    }
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for (i, v) in series.iter().enumerate() {
        let cid = number_id(&v["id"])?;
        if !seen.insert(cid.clone()) {
            return bad();
        }
        let name = strv(v, "name")?;
        let prefix = format!("第{}话", i + 1);
        let title = if name.trim().is_empty() {
            prefix
        } else if name.starts_with(&prefix) {
            name.to_string()
        } else {
            format!("{} {}", prefix, name)
        };
        out.push((
            cid,
            title,
            if i + 1 == series.len() {
                Some(stamp)
            } else {
                None
            },
        ));
    }
    out.reverse();
    Ok(out)
}
fn strings_or_series(a: &Value) -> R<Vec<Value>> {
    match a.get("series") {
        None => Ok(Vec::new()),
        Some(Value::Array(a)) => Ok(a.clone()),
        _ => bad(),
    }
}
fn chapters(c: &Context, input: &Value) -> R<Value> {
    let id = req_id(input, "mangaId", "jmcomic:")?;
    let a = c.album(&id)?.ok_or_else(|| E::from("not_found"))?;
    let items=chapter_ids(&a)?.into_iter().enumerate().map(|(i,(cid,title,date))|{let mut v=json!({"id":format!("jmcomic-read:{}:{}",id,cid),"mangaId":format!("jmcomic:{}",id),"title":title,"index":i,"url":format!("https://{}/photo/{}",c.mirror,cid)});if let Some(date)=date{v["publishedAt"]=json!(date)}v}).collect::<Vec<_>>();
    Ok(json!({"chapters":items}))
}
fn chapter_owner(input: &Value) -> R<(String, String)> {
    let x = input["args"]["chapterId"]
        .as_str()
        .and_then(|s| s.strip_prefix("jmcomic-read:"))
        .ok_or_else(|| E::from("invalid_request"))?;
    let mut parts = x.split(':');
    let aid = parts.next().ok_or_else(|| E::from("invalid_request"))?;
    let cid = parts.next().ok_or_else(|| E::from("invalid_request"))?;
    if parts.next().is_some() {
        return Err("invalid_request".into());
    }
    let aid = number_id(&json!(aid)).map_err(|_| E::from("invalid_request"))?;
    let cid = number_id(&json!(cid)).map_err(|_| E::from("invalid_request"))?;
    if let Some(owner) = input["args"].get("mangaId") {
        if owner.as_str() != Some(&format!("jmcomic:{}", aid)) {
            return Err("invalid_request".into());
        }
    }
    Ok((aid, cid))
}
// The 9,142 frozen real page positions all use this path contract. CDN hosts
// are deliberately unrestricted: source identity comes from PID + photo CID,
// never from a guessed/fixed CDN list. The operation itself sends no HTTP.
fn photo_identity(url: &str) -> R<(&str, &str)> {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .ok_or_else(|| E::from("invalid_request"))?;
    if url
        .bytes()
        .any(|b| b.is_ascii_control() || b.is_ascii_whitespace() || b == b'\\')
        || url.contains('#')
    {
        return Err("invalid_request".into());
    }
    let (authority, path) = rest
        .split_once('/')
        .ok_or_else(|| E::from("invalid_request"))?;
    if authority.is_empty() || authority.contains('@') {
        return Err("invalid_request".into());
    }
    let path = path.split('?').next().unwrap_or(path);
    let tail = path
        .strip_prefix("media/photos/")
        .ok_or_else(|| E::from("invalid_request"))?;
    let (cid, file) = tail
        .split_once('/')
        .ok_or_else(|| E::from("invalid_request"))?;
    if cid.is_empty()
        || cid.len() > 20
        || !cid.bytes().all(|b| b.is_ascii_digit())
        || file.is_empty()
        || file.contains('/')
    {
        return Err("invalid_request".into());
    }
    let raw = file.as_bytes();
    let mut decoded = Vec::new();
    let mut i = 0;
    while i < raw.len() {
        let b = if raw[i] == b'%' {
            if i + 2 >= raw.len() {
                return Err("invalid_request".into());
            }
            let h = (raw[i + 1] as char)
                .to_digit(16)
                .ok_or_else(|| E::from("invalid_request"))?;
            let l = (raw[i + 2] as char)
                .to_digit(16)
                .ok_or_else(|| E::from("invalid_request"))?;
            i += 3;
            (h * 16 + l) as u8
        } else {
            let b = raw[i];
            i += 1;
            b
        };
        if b == b'/' || b == b'\\' || b.is_ascii_control() {
            return Err("invalid_request".into());
        }
        decoded.push(b);
    }
    if decoded == b"." || decoded == b".." {
        return Err("invalid_request".into());
    }
    Ok((cid, file))
}
// The existing cover consumer supplies the manga ID as pageId. Only the exact
// album cover this source emits may use that identity, without a stripe hint.
// Keep reader PageID/CID ownership and arbitrary external URLs out of this path.
fn cover_image_owner<'a>(request: &[u8], input: &'a Value) -> R<Option<&'a str>> {
    let args = &input["args"];
    let Some(owner) = args["pageId"]
        .as_str()
        .filter(|id| id.starts_with("jmcomic:"))
    else {
        return Ok(None);
    };
    let aid = req_id(input, "pageId", "jmcomic:")?;
    if args.get("chapterId").is_some()
        || args
            .get("mangaId")
            .is_some_and(|v| v.as_str() != Some(owner))
    {
        return Err("invalid_request".into());
    }
    let raw_args = protocol::direct(request, b"args")
        .map_err(E::from)?
        .unwrap_or(request);
    let mut manga_url = [0; 16384];
    if let Some(route) = protocol::text(raw_args, b"mangaUrl", &mut manga_url).map_err(E::from)? {
        if !MIRRORS.iter().any(|mirror| {
            let expected = format!("https://{}/album/{}", mirror, aid);
            route == expected.as_bytes() || route.strip_suffix(b"/") == Some(expected.as_bytes())
        }) {
            return Err("invalid_request".into());
        }
    }
    let uri = args["url"]
        .as_str()
        .ok_or_else(|| E::from("invalid_request"))?;
    // A saved cover remains valid when the user switches between supported mirrors.
    if !MIRRORS.iter().any(|mirror| uri == cover_url(mirror, &aid)) {
        return Err("invalid_request".into());
    }
    Ok(Some(uri))
}
fn image_owner<'a>(input: &'a Value) -> R<(&'a str, &'a str)> {
    let args = &input["args"];
    let pid = args["pageId"]
        .as_str()
        .and_then(|p| p.strip_prefix("jmcomic-page:"))
        .ok_or_else(|| E::from("invalid_request"))?;
    let mut parts = pid.split(':');
    let aid = parts.next().ok_or_else(|| E::from("invalid_request"))?;
    let cid = parts.next().ok_or_else(|| E::from("invalid_request"))?;
    let index = parts.next().ok_or_else(|| E::from("invalid_request"))?;
    if parts.next().is_some()
        || [aid, cid]
            .iter()
            .any(|s| s.is_empty() || s.len() > 20 || !s.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err("invalid_request".into());
    }
    let ordinal = index
        .parse::<usize>()
        .map_err(|_| E::from("invalid_request"))?;
    if ordinal.to_string() != index {
        return Err("invalid_request".into());
    }
    if let Some(v) = args.get("mangaId") {
        if v.as_str() != Some(&format!("jmcomic:{}", aid)) {
            return Err("invalid_request".into());
        }
    }
    if let Some(v) = args.get("chapterId") {
        if v.as_str() != Some(&format!("jmcomic-read:{}:{}", aid, cid)) {
            return Err("invalid_request".into());
        }
    }
    let url = args["url"]
        .as_str()
        .ok_or_else(|| E::from("invalid_request"))?;
    let (image_cid, _) = photo_identity(url)?;
    if image_cid != cid {
        return Err("invalid_request".into());
    }
    Ok((cid, url))
}
fn image_rows(url: &str) -> R<Option<usize>> {
    let lower = url.to_lowercase();
    if !lower.contains("media/photos") || lower.contains("_3x4.jpg") {
        return Ok(None);
    }
    let route = url.split('?').next().unwrap_or(url);
    let mut parts = route.rsplit('/');
    let filename = parts.next().ok_or_else(|| E::from("parse_error"))?;
    let aid = parts
        .next()
        .ok_or_else(|| E::from("parse_error"))?
        .parse::<u64>()
        .map_err(|_| E::from("parse_error"))?;
    if aid < 220980 {
        return Ok(None);
    }
    let name = filename
        .split('.')
        .next()
        .ok_or_else(|| E::from("parse_error"))?;
    let rows = if aid < 268850 {
        10
    } else {
        let modulus = if aid >= 421926 { 8 } else { 10 };
        let digest = hex_md5(format!("{}{}", aid, name).as_bytes());
        2 * (digest.as_bytes()[31] as usize % modulus) + 2
    };
    Ok(Some(rows))
}
fn pages(c: &Context, input: &Value, decode_supported: bool) -> R<Value> {
    let (aid, cid) = chapter_owner(input)?;
    let album = c.album(&aid)?.ok_or_else(|| E::from("not_found"))?;
    if !chapter_ids(&album)?.iter().any(|x| x.0 == cid) {
        return Err("invalid_request".into());
    }
    let data = c.api(
        "comic_read",
        &[("id", cid.clone()), ("express", String::new())],
    )?;
    if data.as_array().is_some_and(|a| a.is_empty()) {
        return Err("chapter_unavailable".into());
    }
    let images = ary(&data, "images")?;
    if images.is_empty() {
        return Err("chapter_unavailable".into());
    }
    let mut out = Vec::new();
    for (i, v) in images.iter().enumerate() {
        let u = strv(v, "image")?;
        if !(u.starts_with("https://") || u.starts_with("http://")) {
            return bad();
        }
        let (image_cid, _) = photo_identity(u).map_err(|_| E::from("parse_error"))?;
        if image_cid != cid {
            return Err("parse_error".into());
        }
        let rows = image_rows(u)?;
        if rows.is_some() && !decode_supported {
            return Err("host_feature_required".into());
        }
        let mut page = json!({"id":format!("jmcomic-page:{}:{}:{}",aid,cid,i),"index":i,"image":{"kind":"url","url":u}});
        if let Some(rows) = rows {
            page["image"]["requiredImageDecode"] = json!({"kind":"jm_stripes_v1","rows":rows})
        }
        out.push(page)
    }
    Ok(json!({"pages":out}))
}
fn selected<'a>(filters: &'a Value, key: &str, default: &'a str, allowed: &[&str]) -> R<&'a str> {
    let s = match filters.get(key) {
        None => default,
        Some(Value::String(s)) => s,
        _ => return Err("invalid_request".into()),
    };
    if !allowed.contains(&s) {
        return Err("invalid_request".into());
    }
    Ok(s)
}
fn list(c: &Context, input: &Value, search: bool) -> R<Value> {
    let a = &input["args"];
    let page =
        request_page_number(&serde_json::to_vec(input).map_err(|_| E::from("invalid_request"))?);
    if page == 0 || page > 100000 {
        return Err("invalid_request".into());
    }
    let filters = &a["filters"];
    if selected(filters, "favorites", "0", &["0", "1"])? == "1" {
        return Err("account_site_only".into());
    }
    let query = if search {
        a["query"].as_str().unwrap_or("")
    } else {
        ""
    };
    let prefix = query.get(..2).is_some_and(|s| s.eq_ignore_ascii_case("JM"));
    let direct_id = if prefix {
        Some(query[2..].strip_prefix(':').unwrap_or(&query[2..]).trim())
    } else if !query.is_empty() && query.chars().all(|c| c.is_ascii_digit()) {
        Some(query)
    } else {
        None
    };
    let url_id = MIRRORS.iter().find_map(|host| {
        let prefix = format!("https://{}/", host);
        query
            .strip_prefix(&prefix)
            .and_then(|p| {
                p.strip_prefix("album/")
                    .or_else(|| p.strip_prefix("photo/"))
            })
            .map(|p| p.split('/').next().unwrap_or(p))
    });
    if let Some(id) = direct_id.or(url_id) {
        let id = number_id(&json!(id)).map_err(|_| E::from("invalid_request"))?;
        let card = c.canonical(&id)?.map(|a| detail(c, &a)).transpose()?;
        return Ok(
            json!({"items":card.into_iter().collect::<Vec<_>>(),"page":{"hasMore":false,"nextCursor":null}}),
        );
    }
    let sort = selected(
        filters,
        "sort",
        if !search && a["listingId"] == "popular" {
            "mv"
        } else {
            "mr"
        },
        &["mr", "mv", "tf", "mp"],
    )?;
    let mut keywords = Vec::new();
    if !query.trim().is_empty() {
        keywords.push(query.to_string())
    }
    if let Some(f) = filters.as_object() {
        for (k, v) in f {
            if k.starts_with("tag:") {
                let s = v.as_str().ok_or_else(|| E::from("invalid_request"))?;
                if !s.is_empty() {
                    keywords.push(s.to_string())
                }
            }
        }
    }
    let data = if keywords.is_empty() {
        let time = selected(filters, "time", "", &["", "t", "w", "m"])?;
        let order = if sort == "mv" && !time.is_empty() {
            format!("mv_{}", time)
        } else {
            sort.to_string()
        };
        let cat = filters["category"].as_str().unwrap_or("0");
        if !cat.bytes().all(|b| b.is_ascii_digit() || b == b'_') {
            return Err("invalid_request".into());
        }
        c.api(
            "categories/filter",
            &[
                ("page", page.to_string()),
                ("c", cat.to_string()),
                ("o", order),
            ],
        )?
    } else {
        let ty = selected(
            filters,
            "type",
            "site",
            &["site", "work", "author", "tag", "character"],
        )?;
        c.api(
            "search",
            &[
                ("search_query", keywords.join(" +")),
                ("o", sort.to_string()),
                ("search_type", ty.to_string()),
                ("page", page.to_string()),
            ],
        )?
    };
    if let Some(id) = data.get("redirect_aid").filter(|v| !v.is_null()) {
        let id = number_id(id)?;
        let item = c.canonical(&id)?.map(|a| detail(c, &a)).transpose()?;
        return Ok(
            json!({"items":item.into_iter().collect::<Vec<_>>(),"page":{"hasMore":false,"nextCursor":null}}),
        );
    }
    let cards = match data.get("content") {
        None => &[][..],
        Some(Value::Array(a)) => a.as_slice(),
        _ => return bad(),
    };
    let mut out = Vec::new();
    for v in cards {
        let card = c.card(v)?;
        if !c.blocked(&card) {
            out.push(card)
        }
    }
    let more = cards.len() >= 80;
    Ok(
        json!({"items":out,"page":{"hasMore":more,"nextCursor":if more{Some((page+1).to_string())}else{None}}}),
    )
}
fn select_filter(id: &str, name: &str, default: &str, options: Vec<(String, String)>) -> Value {
    json!({"id":id,"name":name,"type":"select","default":default,"options":options.into_iter().map(|(l,v)|json!({"label":l,"value":v})).collect::<Vec<_>>()})
}
fn choices(a: &[(&str, &str)]) -> Vec<(String, String)> {
    a.iter()
        .map(|(l, v)| (l.to_string(), v.to_string()))
        .collect()
}
fn filters(r: &[u8], input: &Value) -> R<Value> {
    let mut out = alloc::vec![select_filter(
        "favorites",
        "收藏夹",
        "0",
        choices(&[("关闭", "0"), ("启用", "1")])
    )];
    if protocol::feature(r, b"source_filter_scope_header_v1") {
        out.push(json!({"type":"header","name":"搜索或选择标签时不按类型和时间筛选"}))
    }
    if protocol::feature(r, b"source_filter_fetching_v1")
        && input["args"]["fetchFilterData"] == true
    {
        let c = Context::new(r, input)?;
        let data = c.api("categories", &[])?;
        let mut cats = Vec::new();
        for g in ary(&data, "categories")? {
            let name = strv(g, "name")?;
            let slug = g["slug"].as_str().filter(|s| !s.is_empty()).unwrap_or("0");
            cats.push((name.to_string(), slug.to_string()));
            if let Some(Value::Array(a)) = g.get("sub_categories") {
                for sub in a {
                    cats.push((
                        format!("{} · {}", name, strv(sub, "name")?),
                        format!("{}_{}", slug, strv(sub, "slug")?),
                    ))
                }
            }
        }
        out.push(select_filter("category", "按类型", "0", cats));
        let mut seen = BTreeSet::new();
        for b in ary(&data, "blocks")? {
            let name = strv(b, "title")?;
            if !seen.insert(name) {
                return bad();
            }
            let mut tags = choices(&[("全部", "")]);
            for tag in ary(b, "content")? {
                let s = tag.as_str().ok_or_else(|| E::from("parse_error"))?;
                tags.push((s.to_string(), s.to_string()))
            }
            out.push(select_filter(&format!("tag:{}", name), name, "", tags));
        }
    }
    if !out.iter().any(|v| v["id"] == "category") {
        out.push(json!({"id":"category","name":"按类型","type":"select","default":"0"}));
    }
    out.push(select_filter(
        "sort",
        "排序",
        "mr",
        choices(&[
            ("最新", "mr"),
            ("最多浏览", "mv"),
            ("最多爱心", "tf"),
            ("最多图片", "mp"),
        ]),
    ));
    if protocol::feature(r, b"source_filter_scope_header_v1") {
        out.push(json!({"type":"header","name":"时间只用于最多浏览"}))
    }
    out.push(select_filter(
        "time",
        "时间",
        "",
        choices(&[("全部", ""), ("今天", "t"), ("这周", "w"), ("本月", "m")]),
    ));
    out.push(select_filter(
        "type",
        "搜索范围",
        "site",
        choices(&[
            ("站内搜索", "site"),
            ("作品", "work"),
            ("作者", "author"),
            ("标签", "tag"),
            ("登场人物", "character"),
        ]),
    ));
    Ok(json!({"supportsFilterFetching":true,"filters":out}))
}
fn dispatch(op: &str, r: &[u8]) -> R<Value> {
    protocol::request(r).map_err(E::from)?;
    let input = json_value(r)?;
    match op {
        "get_listings" => {
            Ok(json!({"listings":[{"id":"popular","name":"热门"},{"id":"latest","name":"最新"}]}))
        }
        "get_settings" => Ok(
            json!({"settings":[{"id":"preferred_mirror","name":"网站线路","kind":"select","default":"0","options":MIRRORS.iter().enumerate().map(|(i,h)|json!({"label":h,"value":i.to_string()})).collect::<Vec<_>>()},{"id":"BLOCK_GENRES_LIST","name":"屏蔽词列表","kind":"text","default":"// 例如 \"YAOI cos 扶他 毛絨絨 獵奇 韩漫 韓漫\", 关键词之间用空格分离, 大小写不敏感, \"//\"后的字符会被忽略"}]}),
        ),
        "get_filters" => filters(r, &input),
        "get_image_request" => {
            let (u, rows) = match cover_image_owner(r, &input)? {
                Some(uri) => (uri, None),
                None => {
                    let (_, uri) = image_owner(&input)?;
                    (uri, image_rows(uri)?)
                }
            };
            let mut out = json!({"url":u,"headers":{"User-Agent":UA,"Referer":format!("https://{}/",*MIRRORS.get(input["settings"]["preferred_mirror"].as_str().unwrap_or("0").parse::<usize>().map_err(|_|E::from("invalid_request"))?).ok_or_else(||E::from("invalid_request"))?)}});
            if let Some(rows) = rows {
                if !protocol::feature(r, b"source_required_image_decode_jm_stripes_v1") {
                    return Err("host_feature_required".into());
                }
                out["requiredImageDecode"] = json!({"kind":"jm_stripes_v1","rows":rows})
            }
            Ok(out)
        }
        "get_home" => Err("not_implemented".into()),
        _ => {
            match op {
                "get_manga" | "get_chapters" => {
                    req_id(&input, "mangaId", "jmcomic:")?;
                }
                "get_pages" => {
                    chapter_owner(&input)?;
                }
                _ => {}
            }
            let c = Context::new(r, &input)?;
            match op {
                "search" => list(&c, &input, true),
                "get_manga_list" => list(&c, &input, false),
                "get_manga" => {
                    let id = req_id(&input, "mangaId", "jmcomic:")?;
                    let a = c.album(&id)?.ok_or_else(|| E::from("not_found"))?;
                    Ok(json!({"manga":detail(&c,&a)?}))
                }
                "get_chapters" => chapters(&c, &input),
                "get_pages" => pages(
                    &c,
                    &input,
                    protocol::feature(r, b"source_required_image_decode_jm_stripes_v1"),
                ),
                _ => Err("not_implemented".into()),
            }
        }
    }
}
fn run(op: &str, r: &[u8]) -> u32 {
    match dispatch(op, r) {
        Ok(v) => match serde_json::to_vec(&v) {
            Ok(b) if b.len() <= payload_buf().len() => {
                payload_buf()[..b.len()].copy_from_slice(&b);
                write_success_payload(op, b.len())
            }
            _ => write_error(op, "result_too_large", "source result too large"),
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
koma_source_sdk::koma_source_exports!("jinmantiantang_source");
