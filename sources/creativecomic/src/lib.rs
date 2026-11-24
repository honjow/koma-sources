#![no_std]
extern crate alloc;

use aes::Aes256;
use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use cbc::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};
use koma_source_sdk::{
    chapters::{Chapter, ChapterOrder, ChapterResponse, ChapterResult},
    host,
    json_utils::request_page_number,
    source::{SourceCapabilities, SourceInfo},
    source_protocol,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha512};

#[path = "../../port_support.rs"]
mod port;
use port::Result;

const WEB: &str = "https://www.creative-comic.tw";
const API: &str = "https://api.creative-comic.tw";
const CDN: &str = "https://storage.googleapis.com";
const IMAGE_PATH: &str = "/ccc-www/fs/chapter_content/encrypt/";
const WEB_TOKEN_FEATURE: &[u8] = b"source_web_token_v1";
const LOGIN_URL: &str = "https://www.creative-comic.tw/login";
const TOKEN_KEY: &str = "accessToken";
const VERIFY_PATH: &str = "/member";
const IMAGE_DERIVATION: &str = "sha512_cbc";
const DECODER_FEATURE: &str = "source_required_image_decode_wasm_v1";

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.creativecomic.koma",
    name: "CCC追漫台",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "zh-Hant",
    author: "Koma",
    description: "CCC追漫台公开漫画目录及免费章节。",
    content_rating: "safe",
};
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    listings: true,
    manga_list: true,
    image_request: true,
    ..SourceCapabilities::CORE
};

#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;
koma_source_sdk::koma_source_buffers! {payload:1024*1024,http_out:4*1024*1024,body:3*1024*1024,http_req:4096,scratch:8192,}
koma_source_sdk::koma_source_helpers!();

struct MemberSession {
    generation: String,
    image_key: Option<([u8; 32], [u8; 16])>,
}

fn decode_hex<const N: usize>(value: &Value) -> Result<[u8; N]> {
    let hex = value.as_str().ok_or("parse_error")?.as_bytes();
    if hex.len() != N * 2 {
        return Err("parse_error");
    }
    let mut bytes = [0; N];
    for (index, pair) in hex.chunks_exact(2).enumerate() {
        let high = (pair[0] as char).to_digit(16).ok_or("parse_error")?;
        let low = (pair[1] as char).to_digit(16).ok_or("parse_error")?;
        bytes[index] = (high * 16 + low) as u8;
    }
    Ok(bytes)
}

fn member_session(request: &[u8], for_image: bool) -> Result<Option<MemberSession>> {
    if !source_protocol::feature(request, WEB_TOKEN_FEATURE) {
        return Ok(None);
    }
    if host::check_cancel() {
        return Err("cancelled");
    }
    let rpc = if for_image {
        br#"{"type":"private_session","version":1,"op":"web_token","deriveImageKey":true}"#
            .as_slice()
    } else {
        br#"{"type":"private_session","version":1,"op":"web_token"}"#.as_slice()
    };
    let response = scratch_a();
    let len = host::http_request(rpc, response).map_err(|_| "internal_error")?;
    let reply: Value = serde_json::from_slice(&response[..len]).map_err(|_| "parse_error")?;
    if reply["ok"] != true {
        return Err(
            if matches!(
                reply["error"]["code"].as_str(),
                Some("stale_version" | "inactive")
            ) {
                "cancelled"
            } else {
                "internal_error"
            },
        );
    }
    match reply["state"].as_str() {
        Some("empty" | "expired") => Ok(None),
        Some("ready") => {
            let generation = reply["generation"]
                .as_str()
                .filter(|value| {
                    value.len() == 32 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
                })
                .ok_or("parse_error")?
                .to_string();
            let image_key = if for_image {
                Some((
                    decode_hex(&reply["imageKeyHex"])?,
                    decode_hex(&reply["imageIvHex"])?,
                ))
            } else {
                None
            };
            Ok(Some(MemberSession {
                generation,
                image_key,
            }))
        }
        _ => Err("parse_error"),
    }
}

fn api(url: &str, member: Option<&MemberSession>) -> Result<Value> {
    let mut headers = json!({
        "Accept":"application/json", "Origin":WEB, "Referer":format!("{WEB}/"),
        "device":"web_desktop"
    });
    let body = if let Some(member) = member {
        if host::check_cancel() {
            return Err("cancelled");
        }
        let request = json!({
            "version":1,"method":"GET","url":url,"headers":headers,
            "responseKind":"bodyText","timeoutMs":15000,"webTokenGeneration":member.generation
        });
        let input = serde_json::to_vec(&request).map_err(|_| "internal_error")?;
        let len = host::http_request(&input, http_out()).map_err(|_| "network_error")?;
        let len = source_protocol::body(&http_out()[..len], body_buf())?;
        core::str::from_utf8(&body_buf()[..len])
            .map_err(|_| "parse_error")?
            .to_string()
    } else {
        headers["uuid"] = json!("null");
        port::request(url, "GET", headers, None)?
    };
    let mut response: Value = serde_json::from_str(&body).map_err(|_| "parse_error")?;
    match response["code"].as_i64() {
        Some(0) if !response["data"].is_null() => Ok(response["data"].take()),
        Some(401) => Err("auth_required"),
        Some(403) => Err("forbidden"),
        Some(404) => Err("not_found"),
        Some(429) => Err("rate_limit"),
        _ => Err("parse_error"),
    }
}

fn id_arg(args: &Value, name: &str) -> Result<u64> {
    args[name]
        .as_str()
        .and_then(|id| id.parse::<u64>().ok())
        .filter(|id| *id > 0)
        .ok_or("invalid_request")
}

fn card(row: &Value, id: u64) -> Result<Value> {
    let title = row["name"].as_str().ok_or("parse_error")?;
    let mut manga = json!({"id":id.to_string(),"title":title,"language":"zh-Hant",
        "links":[{"kind":"source","url":format!("{WEB}/zh/book/{id}/content")}]});
    if let Some(cover) = row["image1"].as_str().filter(|cover| !cover.is_empty()) {
        manga["cover"] = json!({"kind":"url","url":port::absolute(WEB, cover)?.as_str()});
    }
    Ok(manga)
}

fn list(args: &Value, page: usize, search: bool, member: Option<&MemberSession>) -> Result<Value> {
    let page_size = if search { 12 } else { 24 };
    let sort = if search {
        "updated_at"
    } else {
        match args["listingId"].as_str() {
            Some("popular") => "like_count",
            Some("latest") => "updated_at",
            _ => return Err("invalid_request"),
        }
    };
    let mut url = port::absolute(API, "/book")?;
    {
        let mut query = url.query_pairs_mut();
        query
            .append_pair("page", &page.to_string())
            .append_pair("rows_per_page", &page_size.to_string());
        if search {
            query
                .append_pair("keyword", args["query"].as_str().unwrap_or_default())
                .append_pair("category", "all");
        }
        query.append_pair("sort_by", sort).append_pair("class", "2");
    }
    let data = api(url.as_str(), member)?;
    let total = data["total"].as_u64().ok_or("parse_error")?;
    let items = data["data"]
        .as_array()
        .ok_or("parse_error")?
        .iter()
        .map(|row| card(row, row["id"].as_u64().ok_or("parse_error")?))
        .collect::<Result<Vec<_>>>()?;
    let next = (total > (page as u64).saturating_mul(page_size)).then(|| page + 1);
    Ok(port::items(items, next))
}

fn details(args: &Value, member: Option<&MemberSession>) -> Result<Value> {
    let id = id_arg(args, "mangaId")?;
    let data = api(&format!("{API}/book/{id}/info"), member)?;
    let mut manga = card(&data, id)?;
    manga["description"] = json!(data["description"].as_str().unwrap_or_default());
    manga["authors"] = json!(names(&data["author"]).collect::<Vec<_>>());
    manga["tags"] = json!(data["type"]["name"]
        .as_str()
        .into_iter()
        .chain(names(&data["tags"]))
        .collect::<Vec<_>>());
    manga["status"] = json!(if data["completed"] == 1 {
        "completed"
    } else {
        "ongoing"
    });
    Ok(json!({"manga":manga}))
}

fn names(rows: &Value) -> impl Iterator<Item = &str> {
    rows.as_array()
        .into_iter()
        .flatten()
        .filter_map(|row| row["name"].as_str())
}

fn catalog(manga: u64, member: Option<&MemberSession>) -> Result<Vec<Value>> {
    let mut data = api(&format!("{API}/book/{manga}/chapter"), member)?;
    match data["chapters"].take() {
        Value::Array(rows) => Ok(rows),
        _ => Err("parse_error"),
    }
}

fn entitled(row: &Value) -> bool {
    row["is_free"] == 1 || row["is_buy"] == 1 || row["is_rent"] == 1 || row["sales_plan"] == 0
}

fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(request)?;
    let manga = id_arg(&input["args"], "mangaId")?;
    let manga_id = manga.to_string();
    let member = member_session(request, false)?;
    let rows = catalog(manga, member.as_ref())?
        .into_iter()
        .rev()
        .map(|row| {
            let id = row["id"].as_u64().ok_or("parse_error")?;
            let locked = !entitled(&row);
            // Match the upstream catalog's visible lock marker. Keep the structured
            // flag for consumers that display chapter access separately.
            let access_marker = if locked { "🔒" } else { "" };
            let title = format!(
                "{access_marker}{} {}",
                row["vol_name"].as_str().unwrap_or_default(),
                row["name"].as_str().ok_or("parse_error")?
            );
            let date = row["online_at"]
                .as_str()
                .filter(|date| date.len() == 19 && date.is_ascii())
                .map(|date| format!("{}T{}+08:00", &date[..10], &date[11..]));
            Ok((id.to_string(), title, date, locked))
        })
        .collect::<Result<Vec<_>>>()?;
    let chapters = rows
        .iter()
        .map(|(id, title, date, locked)| {
            let mut chapter = Chapter::new(id.as_bytes(), manga_id.as_bytes(), title.as_bytes());
            chapter.language = Some(b"zh-Hant");
            chapter.locked = Some(*locked);
            chapter.published_at = date.as_deref().map(str::as_bytes);
            chapter
        })
        .collect::<Vec<_>>();
    ChapterResponse::ordered(payload_buf(), request, ChapterOrder::NewestFirst, &chapters)
        .map_err(port::chapter_error)
}

fn pages(args: &Value, request: &[u8]) -> Result<Value> {
    let manga = id_arg(args, "mangaId")?;
    let chapter = id_arg(args, "chapterId")?;
    let member = member_session(request, false)?;
    let rows = catalog(manga, member.as_ref())?;
    let row = rows
        .iter()
        .find(|row| row["id"] == chapter)
        .ok_or("not_found")?;
    if !entitled(row) {
        return Err(
            if member.is_none() && source_protocol::feature(request, WEB_TOKEN_FEATURE) {
                "auth_required"
            } else {
                "chapter_locked"
            },
        );
    }
    let data = api(&format!("{API}/book/chapter/{chapter}"), member.as_ref())?;
    let pages = data["chapter"]["proportion"]
        .as_array()
        .ok_or("parse_error")?
        .iter()
        .enumerate()
        .map(|(index, page)| {
            let id = page["id"].as_u64().ok_or("parse_error")?;
            Ok(json!({"id":format!("page:{id}"),"index":index,
            "image":{"kind":"url","url":format!("{CDN}{IMAGE_PATH}{id}/2")}}))
        })
        .collect::<Result<Vec<_>>>()?;
    if pages.is_empty() {
        return Err("chapter_unavailable");
    }
    Ok(json!({"chapterId":chapter.to_string(),"pages":pages}))
}

fn image(input: &Value, request: &[u8]) -> Result<Value> {
    let args = &input["args"];
    let url = port::absolute(WEB, args["url"].as_str().ok_or("invalid_request")?)?;
    let mut result = json!({"url":url.as_str(),"headers":{}});
    if url.host_str() != Some("storage.googleapis.com") || !url.path().starts_with(IMAGE_PATH) {
        return Ok(result);
    }
    let id = url
        .path()
        .strip_prefix(IMAGE_PATH)
        .and_then(|path| path.strip_suffix("/2"))
        .and_then(|id| id.parse::<u64>().ok())
        .filter(|id| *id > 0)
        .ok_or("invalid_request")?;
    if url.as_str() != format!("{CDN}{IMAGE_PATH}{id}/2")
        || args["pageId"].as_str() != Some(&format!("page:{id}"))
    {
        return Err("invalid_request");
    }
    if !input["hostFeatures"]
        .as_array()
        .is_some_and(|features| features.iter().any(|feature| feature == DECODER_FEATURE))
    {
        return Err("unsupported_feature");
    }
    // The host pins the same account generation for the signing request and
    // its wrapping key. A token replacement must not mix these two values.
    let member = member_session(request, true)?;
    let data = api(&format!("{API}/book/chapter/image/{id}"), member.as_ref())?;
    let encoded = data["key"]
        .as_str()
        .filter(|key| key.len() <= 192)
        .ok_or("parse_error")?;
    let mut signed = STANDARD.decode(encoded).map_err(|_| "parse_error")?;
    let config = if let Some(member) = member {
        let (key, iv) = member.image_key.ok_or("parse_error")?;
        cbc::Decryptor::<Aes256>::new_from_slices(&key, &iv)
    } else {
        let digest = Sha512::digest(b"freeforccc2020reading");
        cbc::Decryptor::<Aes256>::new_from_slices(&digest[..32], &digest[15..31])
    }
    .map_err(|_| "parse_error")?
    .decrypt_padded_mut::<Pkcs7>(&mut signed)
    .map_err(|_| "parse_error")?;
    let config = core::str::from_utf8(config).map_err(|_| "parse_error")?;
    let (key, iv) = config.split_once(':').ok_or("parse_error")?;
    if !matches!(key.len(), 32 | 48 | 64)
        || iv.len() != 32
        || !key
            .bytes()
            .chain(iv.bytes())
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("parse_error");
    }
    result["requiredImageDecode"] = json!({"kind":"wasm_image_v1","bytes":config});
    Ok(result)
}

fn run(op: &str, request: &[u8]) -> u32 {
    let result = port::input(request).and_then(|input| {
        let args = &input["args"];
        match op {
            "search" => list(
                args,
                request_page_number(request),
                true,
                member_session(request, false)?.as_ref(),
            ),
            "get_manga_list" => list(
                args,
                request_page_number(request),
                false,
                member_session(request, false)?.as_ref(),
            ),
            "get_listings" => Ok(port::listing_list(true, true, false)),
            "get_manga" => details(args, member_session(request, false)?.as_ref()),
            "get_pages" => pages(args, request),
            "get_image_request" => image(&input, request),
            _ => Err("unsupported_operation"),
        }
    });
    port::write_result(op, result)
}

fn run_search(request: &[u8]) -> u32 {
    run("search", request)
}
fn run_get_listings(request: &[u8]) -> u32 {
    run("get_listings", request)
}
fn run_get_manga_list(request: &[u8]) -> u32 {
    run("get_manga_list", request)
}
fn run_get_manga(request: &[u8]) -> u32 {
    run("get_manga", request)
}
fn run_get_pages(request: &[u8]) -> u32 {
    run("get_pages", request)
}
fn run_get_image_request(request: &[u8]) -> u32 {
    run("get_image_request", request)
}
fn run_get_filters(request: &[u8]) -> u32 {
    run("get_filters", request)
}
fn run_get_settings(request: &[u8]) -> u32 {
    run("get_settings", request)
}
fn run_get_home(request: &[u8]) -> u32 {
    run("get_home", request)
}
fn run_get_authentication(request: &[u8]) -> u32 {
    let result = port::input(request).and_then(|_| {
        if !source_protocol::feature(request, WEB_TOKEN_FEATURE) {
            return Err("host_feature_required");
        }
        Ok(json!({
            "authentication":{"kind":"web","loginUrl":LOGIN_URL,"webToken":{
                "version":1,"storageKey":TOKEN_KEY,"apiOrigin":API,
                "verifyPath":VERIFY_PATH,"verifyIdentityPath":["data","id"],"imageKeyDerivation":IMAGE_DERIVATION
            }},
            "authenticated":member_session(request, false)?.is_some()
        }))
    });
    port::write_result("get_authentication", result)
}
fn run_authenticate(_: &[u8]) -> u32 {
    port::write_failure("authenticate", "unsupported_operation")
}
fn run_logout(_: &[u8]) -> u32 {
    port::write_failure("logout", "unsupported_operation")
}
fn chapter_failure(code: &str) -> u32 {
    port::write_failure("get_chapters", code)
}
koma_source_sdk::koma_source_exports!("creativecomic", typed_chapters, chapter_failure);
koma_source_sdk::koma_source_authentication_exports!("creativecomic");
