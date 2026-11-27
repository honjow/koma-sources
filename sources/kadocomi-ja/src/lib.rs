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

const METADATA: &[u8] = include_bytes!("../metadata.json");
const SITE: &str = "https://comic-walker.com";
const CDN: &str = "cdn.comic-walker.com";
const REFERER: &str = "https://comic-walker.com/";
// The API accepts an ordinary browser user agent. CloudFront rejected curl's default.
const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/142.0.0.0 Safari/537.36";
const DECODE: &[u8] = b"source_required_image_decode_wasm_v1";
const EXCLUSIONS: &[u8] = b"source_chapter_exclusions_v1";
const LIST_LIMIT: usize = 30;
const CHAPTER_LIMIT: usize = 100;
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.kadocomi.ja.koma",
    name: "カドコミ",
    language: "ja",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    description: "KADOKAWAの公式マンガサイト。『蜘蛛ですが、なにか？』など、複数のレーベルの連載や無料公開話を掲載しています。",
    author: "Koma source contributors",
    content_rating: "mixed",
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

koma_source_sdk::koma_source_buffers! {
    payload: 512 * 1024, http_out: 8 * 1024 * 1024, body: 4 * 1024 * 1024,
    http_req: 4096, scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

fn api(path: &str, query: &[(&str, &str)]) -> Result<Value> {
    let mut url = port::absolute(SITE, path)?;
    url.query_pairs_mut().extend_pairs(query);
    let body = port::request(
        url.as_str(),
        "GET",
        json!({
            "Accept":"application/json","Referer":REFERER,"X-Requested-With":"XMLHttpRequest","User-Agent":USER_AGENT
        }),
        None,
    )?;
    serde_json::from_str(&body).map_err(|_| "parse_error")
}

fn code(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
}

fn uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(i, b)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}

fn field<'a>(row: &'a Value, name: &str, valid: impl FnOnce(&str) -> bool) -> Result<&'a str> {
    row[name].as_str().filter(|s| valid(s)).ok_or("parse_error")
}

fn work_id(args: &Value) -> Result<&str> {
    args["mangaId"]
        .as_str()
        .and_then(|s| s.strip_prefix("kado:"))
        .filter(|s| code(s))
        .ok_or("invalid_request")
}

fn image_url(raw: &str) -> Result<Url> {
    let url = port::absolute(SITE, raw)?;
    if url.scheme() != "https"
        || url.host_str() != Some(CDN)
        || url.port().is_some()
        || url.fragment().is_some()
    {
        return Err("parse_error");
    }
    Ok(url)
}

fn manga(row: &Value) -> Result<Value> {
    let code = field(row, "code", code)?;
    let title = field(row, "title", |s| !s.trim().is_empty())?;
    let cover = ["bookCover", "originalThumbnail", "thumbnail"]
        .iter()
        .find_map(|key| row[key].as_str().filter(|s| !s.is_empty()))
        .map(image_url)
        .transpose()?;
    let url = format!("{SITE}/detail/{code}");
    Ok(json!({
        "id":format!("kado:{code}"),"title":title,"language":"ja","url":url,
        "links":[{"kind":"source","url":url}],
        "cover":cover.map_or_else(|| json!({"kind":"none"}), |url| json!({
            "kind":"url","url":url.as_str(),"headers":{"Referer":REFERER}
        }))
    }))
}

fn list(args: &Value, search: bool) -> Result<Value> {
    let offset = match &args["page"]["cursor"] {
        Value::Null => 0,
        next => next
            .as_str()
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n > 0 && *n <= 1_000_000 && *n % LIST_LIMIT == 0)
            .ok_or("invalid_request")?,
    };
    let offset_text = offset.to_string();
    let mut query = alloc::vec![("limit", "30"), ("offset", offset_text.as_str())];
    let path = if search {
        let keyword = args["query"].as_str().unwrap_or("").trim();
        if keyword.len() > 1024 {
            return Err("invalid_request");
        }
        let sort = match &args["filters"]["sort"] {
            Value::Null => "popularity",
            Value::String(sort) if matches!(sort.as_str(), "popularity" | "update") => sort,
            _ => return Err("invalid_request"),
        };
        query.extend([("keywords", keyword), ("sortBy", sort)]);
        "/api/search/keywords"
    } else {
        match args["listingId"].as_str().unwrap_or("popular") {
            "popular" => "/api/ranking",
            "latest" => "/api/series/new",
            _ => return Err("invalid_request"),
        }
    };
    let root = api(path, &query)?;
    let total = root["total"]
        .as_u64()
        .or_else(|| root["pagination"]["total"].as_u64())
        .ok_or("parse_error")?;
    let rows = root["result"].as_array().ok_or("parse_error")?;
    let next = offset + LIST_LIMIT;
    if rows.len() > LIST_LIMIT || (rows.is_empty() && (offset as u64) < total) {
        return Err("catalog_changed");
    }
    let items = rows.iter().map(manga).collect::<Result<Vec<_>>>()?;
    Ok(port::items(items, ((next as u64) < total).then_some(next)))
}

fn work(code: &str) -> Result<Value> {
    let root = api("/api/contents/details/work", &[("workCode", code)])?;
    if root["work"]["code"].as_str() != Some(code) {
        return Err("not_found");
    }
    Ok(root)
}

fn detail(input: &Value, labels: &Labels<'_>) -> Result<Value> {
    let root = work(work_id(&input["args"])?)?;
    let row = &root["work"];
    let mut item = manga(row)?;
    let mut summary = String::from(row["summary"].as_str().unwrap_or(""));
    if let Some(update) = row["nextUpdateDateText"].as_str().filter(|s| !s.is_empty()) {
        if !summary.is_empty() {
            summary.push_str("\n\n");
        }
        summary.push_str(&labels.text("detail.next_update", "Next update")?);
        summary.push(' ');
        summary.push_str(update);
    }
    let roles = |allowed: &[&str]| -> Vec<&str> {
        row["authors"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|a| {
                a["role"]
                    .as_str()
                    .is_some_and(|role| allowed.contains(&role))
            })
            .filter_map(|a| a["name"].as_str())
            .collect()
    };
    let mut tags = alloc::vec!["Manga"];
    tags.extend(
        [&row["genre"], &row["subGenre"]]
            .into_iter()
            .chain(row["tags"].as_array().into_iter().flatten())
            .filter_map(|t| t["name"].as_str()),
    );
    item["description"] = json!(summary);
    item["authors"] = json!(roles(&["原作", "著者"]));
    item["artists"] = json!(roles(&["漫画", "作画", "著者"]));
    item["tags"] = json!(tags);
    if let Some(status) = match row["serializationStatus"].as_str() {
        Some("ongoing") => Some("ongoing"),
        Some("finished") => Some("completed"),
        _ => None,
    } {
        item["status"] = json!(status);
    }
    Ok(json!({"manga":item}))
}

fn chapter_id(input: &str) -> Result<(&str, &str, &str)> {
    let mut parts = input
        .strip_prefix("kado-ch:")
        .ok_or("invalid_request")?
        .split(':');
    let work = parts.next().filter(|s| code(s)).ok_or("invalid_request")?;
    let episode = parts.next().filter(|s| code(s)).ok_or("invalid_request")?;
    let id = parts.next().filter(|s| uuid(s)).ok_or("invalid_request")?;
    if parts.next().is_some() {
        return Err("invalid_request");
    }
    Ok((work, episode, id))
}

fn chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(request)?;
    let args = &input["args"];
    let series = work_id(args)?;
    let root = work(series)?;
    let listing = &root["latestEpisodes"];
    let rows = listing["result"].as_array().ok_or("parse_error")?;
    if listing["total"].as_u64() != Some(rows.len() as u64) {
        return Err("catalog_changed");
    }
    let wanted = args["chapterId"].as_str().map(chapter_id).transpose()?;
    if wanted.is_some_and(|(owner, _, _)| owner != series) {
        return Err("invalid_request");
    }
    let mut seen = BTreeSet::new();
    for row in rows {
        let id = field(row, "id", uuid)?;
        if !seen.insert(id) {
            return Err("catalog_changed");
        }
    }
    let start = match &args["page"]["cursor"] {
        Value::Null => 0,
        cursor => {
            let (offset, anchor) = cursor
                .as_str()
                .and_then(|s| s.split_once(':'))
                .ok_or("invalid_request")?;
            let offset = offset
                .parse::<usize>()
                .ok()
                .filter(|n| *n > 0 && *n % CHAPTER_LIMIT == 0)
                .ok_or("invalid_request")?;
            if offset >= rows.len() || rows[offset - 1]["id"].as_str() != Some(anchor) {
                return Err("catalog_changed");
            }
            offset
        }
    };
    let end = (start + CHAPTER_LIMIT).min(rows.len());
    let next = if end < rows.len() {
        Some(format!("{end}:{}", field(&rows[end - 1], "id", uuid)?))
    } else {
        None
    };
    let page = next.as_ref().map_or(ChapterPage::Complete, |c| {
        ChapterPage::NextCursor(c.as_bytes())
    });
    let owner = format!("kado:{series}");
    let hide = input["settings"]["hide_locked"].as_bool() == Some(true)
        && source_protocol::feature(request, EXCLUSIONS);
    let mut output = GroupedChapters::new(payload_buf(), request).map_err(port::chapter_error)?;
    let mut excluded = Vec::new();
    for (index, row) in rows.iter().enumerate().take(end).skip(start) {
        if host::check_cancel() {
            return Err("cancelled");
        }
        let id = field(row, "id", uuid)?;
        let episode = field(row, "code", code)?;
        if wanted.is_some_and(|(_, expected_code, expected_id)| {
            (episode, id) != (expected_code, expected_id)
        }) {
            continue;
        }
        let full_id = format!("kado-ch:{series}:{episode}:{id}");
        let active = row["isActive"].as_bool().ok_or("parse_error")?;
        if !active && hide && wanted.is_none() {
            excluded.push(full_id);
            continue;
        }
        let title = field(row, "title", |s| !s.trim().is_empty())?;
        let lock = if active { "" } else { "🔒 " };
        let title = match row["subTitle"].as_str().filter(|s| !s.trim().is_empty()) {
            Some(subtitle) => format!("{lock}{title} - {subtitle}"),
            None => format!("{lock}{title}"),
        };
        let url = format!("{SITE}/detail/{series}/episodes/{episode}");
        let mut chapter = Chapter::new(full_id.as_bytes(), owner.as_bytes(), title.as_bytes());
        chapter.url = Some(url.as_bytes());
        chapter.language = Some(b"ja");
        chapter.locked = Some(!active);
        chapter.published_at = row["updateDate"].as_str().map(str::as_bytes);
        chapter.chapter_number = row["internal"]["episodeNo"]
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .map(ChapterNumber::Integer);
        let placement = if active {
            ChapterPlacement::InGroup {
                group: b"episodes",
                position: (rows.len() - 1 - index) as u64,
            }
        } else {
            ChapterPlacement::ManualOnly
        };
        output
            .push(chapter, placement)
            .map_err(port::chapter_error)?;
        if wanted.is_some() {
            return output
                .finish(ChapterPage::Complete)
                .map_err(port::chapter_error);
        }
    }
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

fn viewer(work: &str, episode: &str, id: &str) -> Result<Value> {
    let detail = api(
        "/api/contents/details/episode",
        &[
            ("workCode", work),
            ("episodeCode", episode),
            ("episodeType", "latest"),
        ],
    )?;
    let row = &detail["episode"];
    if row["id"].as_str() != Some(id) || row["code"].as_str() != Some(episode) {
        return Err("not_found");
    }
    if row["isActive"].as_bool() != Some(true) {
        return Err("chapter_locked");
    }
    let root = api(
        "/api/contents/viewer",
        &[("episodeId", id), ("imageSizeType", "width:1284")],
    )?;
    let pages = root["manuscripts"].as_array().ok_or("parse_error")?;
    if pages.is_empty() {
        return Err("chapter_locked");
    }
    Ok(root)
}

fn manuscript(row: &Value) -> Result<(Url, Option<&str>)> {
    let url = image_url(row["drmImageUrl"].as_str().ok_or("parse_error")?)?;
    if !url.path().starts_with("/images/") {
        return Err("parse_error");
    }
    let config = match row["drmMode"].as_str() {
        Some("xor") => Some(field(row, "drmHash", |s| {
            (2..=64).contains(&s.len())
                && s.len() % 2 == 0
                && s.bytes().all(|b| b.is_ascii_hexdigit())
        })?),
        Some("none") if row["drmHash"].as_str().unwrap_or("").is_empty() => None,
        _ => return Err("unsupported_scramble"),
    };
    Ok((url, config))
}

fn pages(input: &Value) -> Result<Value> {
    let chapter = input["args"]["chapterId"]
        .as_str()
        .ok_or("invalid_request")?;
    let (work, episode, id) = chapter_id(chapter)?;
    let root = viewer(work, episode, id)?;
    let rows = root["manuscripts"].as_array().ok_or("parse_error")?;
    let items = rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let (url, _) = manuscript(row)?;
            Ok(json!({
                "id":format!("kado-page:{chapter}:{i}"),"index":i,
                "image":{"kind":"url","url":url.as_str()}
            }))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(json!({"chapterId":chapter,"pages":items,"readingDirection":root["scrollDirection"]}))
}

fn image_request(input: &Value, request: &[u8]) -> Result<Value> {
    let args = &input["args"];
    let original = image_url(args["url"].as_str().ok_or("invalid_request")?)?;
    let page = args["pageId"]
        .as_str()
        .and_then(|s| s.strip_prefix("kado-page:"));
    let is_page = original.path().starts_with("/images/");
    match (page, is_page) {
        (None, false) => {
            Ok(json!({"imageRequest":{"url":original.as_str(),"headers":{"Referer":REFERER}}}))
        }
        (Some(page), true) => {
            let (chapter, index) = page.rsplit_once(':').ok_or("invalid_request")?;
            let index = index.parse::<usize>().map_err(|_| "invalid_request")?;
            let (work, episode, id) = chapter_id(chapter)?;
            // Signed URLs expire. Resolve the live page, matching its immutable
            // CDN path before using its fresh signature and associated XOR key.
            let root = viewer(work, episode, id)?;
            let row = root["manuscripts"].get(index).ok_or("page_unavailable")?;
            let (url, config) = manuscript(row)?;
            if url.path() != original.path() {
                return Err("page_unavailable");
            }
            let mut data =
                json!({"imageRequest":{"url":url.as_str(),"headers":{"Referer":REFERER}}});
            if let Some(key) = config {
                if !source_protocol::feature(request, DECODE) {
                    return Err("host_feature_required");
                }
                data["requiredImageDecode"] = json!({"kind":"wasm_image_v1","bytes":key});
            }
            Ok(data)
        }
        _ => Err("invalid_request"),
    }
}

fn dispatch(
    operation: &str,
    request: &[u8],
    f: impl FnOnce(&Value, &Labels<'_>) -> Result<Value>,
) -> u32 {
    let value = port::input(request).and_then(|v| f(&v, &Labels::new(METADATA, request)?));
    port_catalog::write_localized(METADATA, request, operation, value)
}

fn run_get_manga_list(req: &[u8]) -> u32 {
    dispatch("get_manga_list", req, |v, _| list(&v["args"], false))
}
fn run_search(req: &[u8]) -> u32 {
    dispatch("search", req, |v, _| list(&v["args"], true))
}
fn run_get_manga(req: &[u8]) -> u32 {
    dispatch("get_manga", req, detail)
}
fn run_get_pages(req: &[u8]) -> u32 {
    dispatch("get_pages", req, |v, _| pages(v))
}
fn run_get_image_request(req: &[u8]) -> u32 {
    dispatch("get_image_request", req, |v, _| image_request(v, req))
}
fn run_get_home(req: &[u8]) -> u32 {
    dispatch("get_home", req, |_, _| Err("not_implemented"))
}

fn run_get_chapters(req: &[u8]) -> ChapterResult<'static, (&'static str, String)> {
    chapters(req).map_err(|code| (code, port_catalog::failure(METADATA, req, code)))
}
fn write_chapter_failure((code, message): (&str, String)) -> u32 {
    write_error("get_chapters", code, &message)
}

fn run_get_listings(req: &[u8]) -> u32 {
    dispatch("get_listings", req, |_, labels| {
        Ok(json!({"listings":[
            {"id":"popular","name":labels.text("listing.popular", "Popular")?,"supportsPaging":true,"supportsFilters":false},
            {"id":"latest","name":labels.text("listing.latest", "Latest")?,"supportsPaging":true,"supportsFilters":false}
        ]}))
    })
}
fn run_get_filters(req: &[u8]) -> u32 {
    dispatch("get_filters", req, |_, labels| {
        let mut sort = json!({
            "id":"sort","type":"select","default":"popularity","name":labels.text("filter.sort","Sort by")?,
            "options":[{"value":"popularity","label":labels.text("sort.popularity","Popularity")?},
                       {"value":"update","label":labels.text("sort.update","Recently updated")?}]
        });
        let scoped = source_protocol::feature(req, b"source_filter_scope_header_v1");
        if scoped {
            sort["scope"] = json!("search");
        }
        Ok(
            json!({"filters":[sort],"useSearchForFilters":scoped && source_protocol::feature(req,b"source_filter_search_without_query_v1")}),
        )
    })
}
fn run_get_settings(req: &[u8]) -> u32 {
    dispatch("get_settings", req, |_, labels| {
        Ok(json!({"settings":[{
            "id":"hide_locked","kind":"boolean","default":false,"name":labels.text("setting.hide_locked","Hide unavailable chapters")?
        }]}))
    })
}

koma_source_sdk::koma_source_exports!("kadocomi_ja", typed_chapters, write_chapter_failure);
