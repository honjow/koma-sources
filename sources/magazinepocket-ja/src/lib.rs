#![no_std]
extern crate alloc;

use alloc::{
    collections::BTreeSet,
    format,
    string::{String, ToString},
    vec::Vec,
};
use koma_source_sdk::{
    chapters::{
        Chapter, ChapterNumber, ChapterPage, ChapterPlacement, ChapterResult, GroupedChapters,
    },
    host,
    source::{SourceCapabilities, SourceInfo},
    source_protocol,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256, Sha512};
use url::Url;

#[path = "../../port_support.rs"]
#[allow(dead_code)]
mod port;
#[path = "../../port_catalog.rs"]
mod port_catalog;

use port::Result;
use port_catalog::Labels;

#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

const SITE: &str = "https://pocket.shonenmagazine.com";
const API: &str = "https://api.pocket.shonenmagazine.com";
const METADATA: &[u8] = include_bytes!("../metadata.json");
const IMAGE_PROCESSOR: &[u8] = b"source_required_image_decode_wasm_v1";
const EXCLUSIONS: &[u8] = b"source_chapter_exclusions_v1";
const LIST_LIMIT: usize = 25;
const CHAPTER_LIMIT: usize = 32;
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.magazinepocket.ja.koma",
    name: "マガポケ",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    language: "ja",
    description: "講談社の公式漫画サービス。『ブルーロック』などのマガジン作品とマガポケオリジナルを、公開話の範囲で読めます。",
    author: "Koma source contributors",
    content_rating: "sfw",
};
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: true,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: true,
    manga_list: true,
    filters: true,
    settings: true,
    home: false,
    credentials: false,
    image_request: true,
};
const RANKINGS: &[u32] = &[30, 20, 1, 31, 32, 21, 22, 23, 24, 25, 26, 27, 28, 29];

koma_source_sdk::koma_source_buffers! {
    payload: 224 * 1024, http_out: 1024 * 1024, body: 512 * 1024,
    http_req: 4096, scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

fn hex(bytes: impl AsRef<[u8]>) -> String {
    let alphabet = b"0123456789abcdef";
    let bytes = bytes.as_ref();
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        encoded.push(alphabet[(b >> 4) as usize] as char);
        encoded.push(alphabet[(b & 15) as usize] as char);
    }
    encoded
}

fn hash_param(key: &str, value: &str) -> String {
    format!(
        "{}_{}",
        hex(Sha256::digest(key)),
        hex(Sha512::digest(value))
    )
}

fn api(path: &str, mut params: Vec<(&str, String)>, post: bool) -> Result<Value> {
    params.sort_unstable_by_key(|&(key, _)| key);
    let component_hashes: Vec<_> = params
        .iter()
        .map(|(key, value)| hash_param(key, value))
        .collect();
    // These hashes encode the public site's anonymous request. Both account
    // suffix inputs remain empty and this adapter makes no login requests.
    let hash = hex(Sha512::digest(format!(
        "{}{}",
        hex(Sha256::digest(component_hashes.join(","))),
        hash_param("", "")
    )));
    let mut url = Url::parse(&format!("{API}{path}")).map_err(|_| "internal_error")?;
    let mut headers = json!({
        "Accept":"application/json","Origin":SITE,"Referer":format!("{SITE}/"),
        "X-Manga-Platform":"3","X-Manga-Hash":hash
    });
    let mut form = None;
    if post {
        let mut serializer = url::form_urlencoded::Serializer::new(String::new());
        serializer.extend_pairs(params.iter().map(|(key, value)| (*key, value.as_str())));
        form = Some(serializer.finish());
        headers["Content-Type"] = json!("application/x-www-form-urlencoded");
    } else if !params.is_empty() {
        url.query_pairs_mut()
            .extend_pairs(params.iter().map(|(key, value)| (*key, value.as_str())));
    }
    if host::check_cancel() {
        return Err("cancelled");
    }
    let mut envelope = json!({
        "version":1,"method":if post {"POST"} else {"GET"},"url":url.as_str(),
        "headers":headers,"responseKind":"bodyText","timeoutMs":15000
    });
    if let Some(form) = form {
        envelope["body"] = json!(form);
    }
    let encoded = serde_json::to_vec(&envelope).map_err(|_| "internal_error")?;
    let len = host::http_request(&encoded, http_out()).map_err(|_| "network_error")?;
    let raw = &http_out()[..len];
    if path == "/web/episode/viewer"
        && source_protocol::direct(raw, b"status").ok().flatten() == Some(b"400")
    {
        let text = source_protocol::direct(raw, b"bodyText")
            .ok()
            .flatten()
            .and_then(|value| serde_json::from_slice::<String>(value).ok());
        let error = text
            .as_deref()
            .and_then(|body| serde_json::from_str::<Value>(body).ok());
        if error
            .as_ref()
            .and_then(|value| value["response_code"].as_u64())
            .is_some_and(|code| matches!(code, 3104 | 3105))
        {
            return Err("chapter_locked");
        }
    }
    let len = source_protocol::body(raw, body_buf())?;
    let value: Value = serde_json::from_slice(&body_buf()[..len]).map_err(|_| "parse_error")?;
    if value["status"] != "success" || value["response_code"] != 0 {
        return Err("upstream_error");
    }
    Ok(value)
}

fn rows<'a>(response: &'a Value, key: &str) -> Result<&'a [Value]> {
    response[key]
        .as_array()
        .map(Vec::as_slice)
        .ok_or("parse_error")
}

fn number(value: &Value) -> Result<u32> {
    value
        .as_u64()
        .and_then(|raw| u32::try_from(raw).ok())
        .filter(|&n| n > 0)
        .ok_or("parse_error")
}

fn id_number(raw: &str) -> Option<u32> {
    raw.parse::<u32>()
        .ok()
        .filter(|n| *n > 0 && n.to_string() == raw)
}

fn title_id(args: &Value) -> Result<u32> {
    args["mangaId"]
        .as_str()
        .and_then(|id| id.strip_prefix("mag:"))
        .and_then(id_number)
        .ok_or("invalid_request")
}

fn chapter_id(raw: &str) -> Result<(u32, u32)> {
    let (work, episode) = raw
        .strip_prefix("mag-ch:")
        .and_then(|id| id.split_once(':'))
        .ok_or("invalid_request")?;
    Ok((
        id_number(work).ok_or("invalid_request")?,
        id_number(episode).ok_or("invalid_request")?,
    ))
}

fn work_url(id: u32) -> String {
    format!("{SITE}/title/{id:05}")
}
fn chapter_url(work: u32, episode: u32) -> String {
    format!("{}/episode/{episode}", work_url(work))
}

fn cdn_url(raw: &str) -> Result<Url> {
    let url = Url::parse(raw).map_err(|_| "invalid_request")?;
    if url.scheme() != "https"
        || url.host_str() != Some("mgpk-cdn.magazinepocket.com")
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err("invalid_request");
    }
    Ok(url)
}

fn title_list(ids: String) -> Result<Value> {
    api("/title/list", alloc::vec![("title_id_list", ids)], false)
}

fn title(work: u32) -> Result<Value> {
    let response = title_list(format!("{work:05}"))?;
    rows(&response, "title_list")?
        .iter()
        .find(|row| number(&row["title_id"]) == Ok(work))
        .cloned()
        .ok_or("not_found")
}

fn manga(row: &Value) -> Result<Value> {
    let work = number(&row["title_id"])?;
    let name = row["title_name"]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .ok_or("parse_error")?;
    let cover = [
        "thumbnail_image_url",
        "banner_image_url",
        "thumbnail_rect_image_url",
    ]
    .iter()
    .filter_map(|&key| row[key].as_str())
    .find_map(|raw| cdn_url(raw).ok())
    .map_or_else(
        || json!({"kind":"none"}),
        |url| json!({"kind":"url","url":url.as_str(),"headers":{"Referer":format!("{SITE}/")}}),
    );
    let mut manga = json!({
        "id":format!("mag:{work}"),"url":work_url(work),"title":name,
        "language":"ja","cover":cover,"contentRating":"sfw"
    });
    if let Some(author) = row["author_text"].as_str() {
        manga["authors"] = json!([author]);
    } else if let Some(authors) = row["author_list"].as_array() {
        manga["authors"] = json!(authors.iter().filter_map(Value::as_str).collect::<Vec<_>>());
    }
    if let Some(text) = row["introduction_text"].as_str() {
        manga["description"] = json!(text);
    }
    Ok(manga)
}

fn details(args: &Value) -> Result<Value> {
    let row = title(title_id(args)?)?;
    let mut result = manga(&row)?;
    if let Some(ids) = row["genre_id_list"]
        .as_array()
        .filter(|ids| !ids.is_empty())
    {
        let ids: Vec<_> = ids
            .iter()
            .map(|id| number(id).map(|n| n.to_string()))
            .collect::<Result<_>>()?;
        let response = api(
            "/genre/list",
            alloc::vec![("genre_id_list", ids.join(", "))],
            false,
        )?;
        let tags: Vec<_> = rows(&response, "genre_list")?
            .iter()
            .filter_map(|g| g["genre_name"].as_str())
            .collect();
        result["tags"] = json!(tags);
    }
    Ok(json!({"manga":result}))
}

fn offset(args: &Value) -> Result<usize> {
    match &args["page"]["cursor"] {
        Value::Null => Ok(0),
        Value::String(value) => value
            .parse::<usize>()
            .ok()
            .filter(|n| *n > 0 && *n <= 100_000 && *n % LIST_LIMIT == 0 && n.to_string() == *value)
            .ok_or("invalid_request"),
        _ => Err("invalid_request"),
    }
}

fn items(list: &[Value], offset: usize, more: bool) -> Result<Value> {
    let mut seen = BTreeSet::new();
    let mut results = Vec::new();
    for row in list {
        if seen.insert(number(&row["title_id"])?) {
            results.push(manga(row)?);
        }
    }
    Ok(port::items(results, more.then_some(offset + LIST_LIMIT)))
}

fn ranking(id: u32, offset: usize) -> Result<Value> {
    let response = api(
        "/ranking/all",
        alloc::vec![
            ("ranking_id", id.to_string()),
            ("offset", offset.to_string()),
            ("limit", (LIST_LIMIT + 1).to_string())
        ],
        false,
    )?;
    let ranked = rows(&response, "ranking_title_list")?;
    let ids = ranked
        .iter()
        .take(LIST_LIMIT)
        .map(|r| number(&r["id"]).or_else(|_| number(&r["title_id"])))
        .collect::<Result<Vec<_>>>()?;
    if ids.is_empty() {
        return Ok(port::items(Vec::new(), None));
    }
    let response = title_list(
        ids.iter()
            .map(|n| format!("{n:05}"))
            .collect::<Vec<_>>()
            .join(", "),
    )?;
    let list = rows(&response, "title_list")?;
    let ordered: Vec<_> = ids
        .iter()
        .filter_map(|&id| {
            list.iter()
                .find(|row| number(&row["title_id"]) == Ok(id))
                .cloned()
        })
        .collect();
    if ordered.is_empty() {
        return Err("parse_error");
    }
    items(&ordered, offset, ranked.len() > LIST_LIMIT)
}

fn browse(args: &Value, search: bool) -> Result<Value> {
    let offset = offset(args)?;
    let query = match &args["query"] {
        Value::Null => "",
        Value::String(q) if search && q.len() <= 512 => q.trim(),
        _ => return Err("invalid_request"),
    };
    if !search && args["listingId"] == "latest" {
        let response = api("/web/title/weekly", Vec::new(), false)?;
        let mut list = rows(&response, "title_list")?.to_vec();
        list.sort_by(|a, b| {
            b["episode_free_updated"]
                .as_str()
                .cmp(&a["episode_free_updated"].as_str())
        });
        let start = offset.min(list.len());
        let end = (start + LIST_LIMIT).min(list.len());
        return items(&list[start..end], offset, end < list.len());
    }
    if !search && !matches!(args["listingId"].as_str(), Some("popular") | None) {
        return Err("invalid_request");
    }
    let category = match &args["filters"]["category"] {
        Value::Null => "",
        Value::String(category) => category.as_str(),
        _ => return Err("invalid_request"),
    };
    let (path, key, value, result_key) = if !query.is_empty() {
        (
            "/web/search/title",
            "keyword",
            query.into(),
            "search_title_list",
        )
    } else if let Some(genre) = category.strip_prefix("genre:") {
        let genre = id_number(genre)
            .filter(|id| (1..=66).contains(id))
            .ok_or("invalid_request")?;
        ("/search/title", "genre_id", genre.to_string(), "title_list")
    } else {
        let id = if category.is_empty() {
            30
        } else {
            category
                .strip_prefix("rank:")
                .and_then(id_number)
                .filter(|id| RANKINGS.contains(id))
                .ok_or("invalid_request")?
        };
        return ranking(id, offset);
    };
    // Both search endpoints honor offset. An extra result is used only to
    // signal the next page, keeping each response small even for broad terms.
    let response = api(
        path,
        alloc::vec![
            (key, value),
            ("offset", offset.to_string()),
            ("limit", (LIST_LIMIT + 1).to_string())
        ],
        false,
    )?;
    let list = rows(&response, result_key)?;
    items(
        &list[..list.len().min(LIST_LIMIT)],
        offset,
        list.len() > LIST_LIMIT,
    )
}

fn episode_ids(row: &Value) -> Result<Vec<u32>> {
    if row["episode_id_list"].is_null() {
        return Ok(Vec::new());
    }
    let mut ids = rows(row, "episode_id_list")?
        .iter()
        .map(number)
        .collect::<Result<Vec<_>>>()?;
    let mut seen = BTreeSet::new();
    ids.retain(|&id| seen.insert(id));
    ids.reverse();
    Ok(ids)
}

fn catalog(request: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(request)?;
    let args = &input["args"];
    let work = title_id(args)?;
    let ids = episode_ids(&title(work)?)?;
    let wanted = args["chapterId"].as_str().map(chapter_id).transpose()?;
    let start = match (&args["page"]["cursor"], wanted) {
        (Value::Null, None) => 0,
        (Value::Null, Some((owner, episode))) if owner == work => {
            ids.iter().position(|&n| n == episode).ok_or("not_found")?
        }
        (Value::String(anchor), None) => {
            let anchor = id_number(anchor).ok_or("invalid_request")?;
            ids.iter()
                .position(|&id| id == anchor)
                .ok_or("catalog_changed")?
                + 1
        }
        _ => return Err("invalid_request"),
    };
    let end = (start + if wanted.is_some() { 1 } else { CHAPTER_LIMIT }).min(ids.len());
    let mut output = GroupedChapters::new(payload_buf(), request).map_err(port::chapter_error)?;
    if start == end {
        return output
            .finish(ChapterPage::Complete)
            .map_err(port::chapter_error);
    }
    let joined = ids[start..end]
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    let response = api(
        "/episode/list",
        alloc::vec![("episode_id_list", joined)],
        true,
    )?;
    let episodes = rows(&response, "episode_list")?;
    let owner = format!("mag:{work}");
    let hide = wanted.is_none()
        && input["settings"]["hide_locked"] == true
        && source_protocol::feature(request, EXCLUSIONS);
    let mut excluded = Vec::new();
    for (index, &id) in ids.iter().enumerate().take(end).skip(start) {
        let row = episodes
            .iter()
            .find(|row| {
                number(&row["episode_id"]) == Ok(id) && number(&row["title_id"]) == Ok(work)
            })
            .ok_or("catalog_changed")?;
        let chapter_id = format!("mag-ch:{work}:{id}");
        let locked = row["point"].as_u64().unwrap_or(0) > 0 && row["badge"].as_u64() != Some(3);
        if locked && hide {
            excluded.push(chapter_id);
            continue;
        }
        let label = row["episode_name"]
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .ok_or("parse_error")?;
        let label = if locked {
            format!("🔒 {label}")
        } else {
            label.into()
        };
        let url = chapter_url(work, id);
        let date = row["start_time"]
            .as_str()
            .and_then(|s| s.split_once(' '))
            .map(|(day, time)| format!("{day}T{time}+09:00"));
        let mut chapter = Chapter::new(chapter_id.as_bytes(), owner.as_bytes(), label.as_bytes());
        chapter.url = Some(url.as_bytes());
        chapter.language = Some(b"ja");
        chapter.locked = Some(locked);
        chapter.published_at = date.as_deref().map(str::as_bytes);
        chapter.chapter_number = row["index"]
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .map(ChapterNumber::Integer);
        let placement = if locked {
            ChapterPlacement::ManualOnly
        } else {
            ChapterPlacement::InGroup {
                group: b"episodes",
                position: (ids.len() - 1 - index) as u64,
            }
        };
        output
            .push(chapter, placement)
            .map_err(port::chapter_error)?;
    }
    let anchor = (wanted.is_none() && end < ids.len()).then(|| ids[end - 1].to_string());
    let page = anchor.as_deref().map_or(ChapterPage::Complete, |anchor| {
        ChapterPage::NextCursor(anchor.as_bytes())
    });
    if excluded.is_empty() {
        return output.finish(page).map_err(port::chapter_error);
    }
    let mut output = output
        .with_excluded_ids(page)
        .map_err(port::chapter_error)?;
    for id in excluded {
        output.push(id.as_bytes()).map_err(port::chapter_error)?;
    }
    output.finish().map_err(port::chapter_error)
}

fn image_ids(url: &Url) -> Result<(u32, u32)> {
    let mut path = url
        .path()
        .strip_prefix("/static/web_titles/")
        .ok_or("invalid_request")?
        .split('/');
    let work = path.next().and_then(id_number).ok_or("invalid_request")?;
    if path.next() != Some("episodes") {
        return Err("invalid_request");
    }
    let episode = path.next().and_then(id_number).ok_or("invalid_request")?;
    let (name, extension) = path
        .next()
        .and_then(|file| file.rsplit_once('.'))
        .ok_or("invalid_request")?;
    if path.next().is_some()
        || name.is_empty()
        || name.len() > 128
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        || !matches!(extension, "jpg" | "jpeg" | "png" | "webp")
    {
        return Err("invalid_request");
    }
    Ok((work, episode))
}

fn viewer(work: u32, episode: u32) -> Result<Value> {
    let response = api(
        "/web/episode/viewer",
        alloc::vec![("episode_id", episode.to_string())],
        false,
    )?;
    if number(&response["title_id"])? != work || number(&response["episode_id"])? != episode {
        return Err("invalid_request");
    }
    Ok(response)
}

fn page_urls(response: &Value, work: u32, episode: u32) -> Result<Vec<Url>> {
    rows(response, "page_list")?
        .iter()
        .map(|url| {
            let url = cdn_url(url.as_str().ok_or("parse_error")?)?;
            if image_ids(&url)? != (work, episode) || url.query().is_none() {
                return Err("parse_error");
            }
            Ok(url)
        })
        .collect()
}

fn pages(args: &Value, request: &[u8]) -> Result<Value> {
    let chapter = args["chapterId"].as_str().ok_or("invalid_request")?;
    let (work, episode) = chapter_id(chapter)?;
    let response = viewer(work, episode)?;
    if !source_protocol::feature(request, IMAGE_PROCESSOR) {
        return Err("host_feature_required");
    }
    let urls = page_urls(&response, work, episode)?;
    if urls.is_empty() {
        return Err("chapter_unavailable");
    }
    let pages: Vec<_> = urls
        .into_iter()
        .enumerate()
        .map(|(i, mut url)| {
            url.set_query(None);
            port::page(i, chapter, url)
        })
        .collect();
    Ok(json!({"pages":pages}))
}

fn permutation(seed: &str, work: u32, episode: u32) -> Result<String> {
    let alphabet = if work % 2 == 0 {
        b"svdk0m7acl"
    } else {
        b"q6jtf2xnog"
    };
    if seed.is_empty() || seed.len() > 32 {
        return Err("parse_error");
    }
    let mut state = 0u32;
    for ch in seed.bytes() {
        let digit = alphabet
            .iter()
            .position(|&c| c == ch)
            .ok_or("parse_error")? as u32;
        state = state.wrapping_mul(10).wrapping_add(digit);
    }
    state ^= work.wrapping_add(episode);
    let mut order = [(0u32, 0u8); 16];
    for (index, pair) in order.iter_mut().enumerate() {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        *pair = (state, index as u8);
    }
    order.sort_unstable();
    Ok(order
        .iter()
        .map(|&(_, index)| b"0123456789abcdef"[index as usize] as char)
        .collect())
}

fn image_request(args: &Value, request: &[u8]) -> Result<Value> {
    let url = cdn_url(args["url"].as_str().ok_or("invalid_request")?)?;
    let headers = json!({"Referer":format!("{SITE}/")});
    if url.path().starts_with("/static/titles/") {
        return Ok(json!({"url":url.as_str(),"headers":headers}));
    }
    let (work, episode) = image_ids(&url)?;
    if !source_protocol::feature(request, IMAGE_PROCESSOR) {
        return Err("host_feature_required");
    }
    // Page snapshots identify the same official image without carrying
    // expiring signatures or the scramble seed. Resolve both from the viewer.
    let response = viewer(work, episode)?;
    let actual = page_urls(&response, work, episode)?
        .into_iter()
        .find(|image| image.path() == url.path())
        .ok_or("page_unavailable")?;
    let config = permutation(
        response["scramble_seed"].as_str().ok_or("parse_error")?,
        work,
        episode,
    )?;
    Ok(json!({"url":actual.as_str(),"headers":headers,
        "requiredImageDecode":{"kind":"wasm_image_v1","pixels":config}}))
}

fn respond(request: &[u8], op: &str, action: impl FnOnce(&Value) -> Result<Value>) -> u32 {
    let result = port::input(request).and_then(|input| action(&input["args"]));
    port_catalog::write_localized(METADATA, request, op, result)
}

fn run_search(req: &[u8]) -> u32 {
    respond(req, "search", |a| browse(a, true))
}
fn run_get_manga_list(req: &[u8]) -> u32 {
    respond(req, "get_manga_list", |a| browse(a, false))
}
fn run_get_manga(req: &[u8]) -> u32 {
    respond(req, "get_manga", details)
}
fn run_get_pages(req: &[u8]) -> u32 {
    respond(req, "get_pages", |a| pages(a, req))
}
fn run_get_image_request(req: &[u8]) -> u32 {
    respond(req, "get_image_request", |a| image_request(a, req))
}
fn run_get_home(req: &[u8]) -> u32 {
    respond(req, "get_home", |_| Err("not_implemented"))
}

fn run_get_listings(req: &[u8]) -> u32 {
    respond(req, "get_listings", |_| {
        let labels = Labels::new(METADATA, req)?;
        Ok(json!({"listings":[
            {"id":"popular","name":labels.text("listing.popular","Popular")?,"supportsPaging":true,"supportsFilters":true},
            {"id":"latest","name":labels.text("listing.latest","Recent releases")?,"supportsPaging":true,"supportsFilters":false}
        ]}))
    })
}

fn run_get_filters(req: &[u8]) -> u32 {
    respond(req, "get_filters", |_| {
        let labels = Labels::new(METADATA, req)?;
        let mut options = Vec::new();
        for &id in RANKINGS {
            let value = if id == 30 {
                String::new()
            } else {
                format!("rank:{id}")
            };
            options
                .push(json!({"value":value,"label":labels.text(&format!("rank.{id}"),"Ranking")?}));
        }
        for id in 1..=66 {
            options.push(json!({"value":format!("genre:{id}"),"label":labels.text(&format!("genre.{id}"),"Genre")?}));
        }
        Ok(
            json!({"filters":[{"type":"select","id":"category","default":"","options":options,
                "name":labels.text("filter.category","Category")?,
                "description":labels.text("filter.category_hint","Category applies when the keyword is empty.")?
            }]}),
        )
    })
}

fn run_get_settings(req: &[u8]) -> u32 {
    respond(req, "get_settings", |_| {
        let labels = Labels::new(METADATA, req)?;
        let settings = if source_protocol::feature(req, EXCLUSIONS) {
            alloc::vec![json!({"id":"hide_locked","kind":"boolean","default":false,
                "name":labels.text("setting.hide_locked","Hide locked chapters")?})]
        } else {
            Vec::new()
        };
        Ok(json!({"settings":settings}))
    })
}

fn run_get_chapters(req: &[u8]) -> ChapterResult<'static, (&'static str, String)> {
    if host::check_cancel() {
        return Err((
            "cancelled",
            port_catalog::failure(METADATA, req, "cancelled"),
        ));
    }
    catalog(req).map_err(|code| (code, port_catalog::failure(METADATA, req, code)))
}
fn write_chapter_failure((code, message): (&str, String)) -> u32 {
    write_error("get_chapters", code, &message)
}

koma_source_sdk::koma_source_exports!("magazinepocket-ja", typed_chapters, write_chapter_failure);
