#![no_std]
extern crate alloc;

use alloc::{
    collections::BTreeSet,
    format,
    string::{String, ToString},
    vec::Vec,
};
use koma_source_sdk::{
    host,
    json_utils::request_page_number,
    source::{SourceCapabilities, SourceInfo},
    source_protocol as protocol,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use url::Url;

mod dom;
use dom::Node;

#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

const BASE: &str = "https://tapas.io";
const REFERER: &str = "https://m.tapas.io";
const API: &str = "https://story-api.tapas.io/cosmos/api/v1/landing";
const LOGIN: &str = "https://tapas.io/account/signin";
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:105.0) Gecko/20100101 Firefox/105.0";
const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.tapas.koma",
    name: "Tapas",
    version: "0.1.0",
    api_version: "0.2",
    language: "en",
    author: "Koma",
    description: "Tapas comics, including public episodes and the original site’s access limits.",
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
    filters: false,
    settings: true,
    image_request: true,
    credentials: false,
};
koma_source_sdk::koma_source_buffers! {payload:1024*1024,http_out:3*1024*1024,body:2*1024*1024,http_req:16384,scratch:8192,}
koma_source_sdk::koma_source_helpers!();
koma_source_sdk::koma_source_exports!("tapas_source");
koma_source_sdk::koma_source_authentication_exports!("tapas_source");

type Result<T> = core::result::Result<T, &'static str>;

fn text<'a>(obj: &'a Value, key: &str) -> Result<&'a str> {
    obj[key].as_str().ok_or("parse_error")
}
fn provider_id(obj: &Value, key: &str) -> Result<u64> {
    obj[key]
        .as_i64()
        .filter(|id| *id > 0)
        .map(|id| id as u64)
        .ok_or("parse_error")
}
fn positive(raw: &str) -> Result<u64> {
    let id = raw.parse::<u64>().map_err(|_| "invalid_request")?;
    if id == 0 || id > i64::MAX as u64 || id.to_string() != raw {
        return Err("invalid_request");
    }
    Ok(id)
}
fn mid(args: &Value) -> Result<u64> {
    let value = args["mangaId"]
        .as_str()
        .and_then(|v| v.strip_prefix("tapas:"))
        .ok_or("invalid_request")?;
    positive(value)
}
fn manga_url(mid: u64) -> String {
    format!("{BASE}/series/{mid}/info")
}
fn episode_url(eid: u64) -> String {
    format!("{BASE}/episode/{eid}")
}
fn cid(mid: u64, eid: u64) -> String {
    format!("tapas-read:{mid}:{eid}")
}
fn chapter_owner(args: &Value) -> Result<(u64, u64)> {
    let (mid, eid) = args["chapterId"]
        .as_str()
        .and_then(|v| v.strip_prefix("tapas-read:"))
        .and_then(|v| v.split_once(':'))
        .ok_or("invalid_request")?;
    let mid = positive(mid)?;
    let eid = positive(eid)?;
    if args
        .get("mangaId")
        .is_some_and(|m| m.as_str() != Some(&format!("tapas:{mid}")))
    {
        return Err("invalid_request");
    }
    Ok((mid, eid))
}
fn web(url: &str) -> Result<Url> {
    let url = Url::parse(url).map_err(|_| "invalid_request")?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err("invalid_request");
    }
    Ok(url)
}
fn absolute(origin: &str, url: &str) -> Result<String> {
    let url = Url::parse(origin)
        .map_err(|_| "parse_error")?
        .join(url)
        .map_err(|_| "parse_error")?;
    Ok(web(url.as_str()).map_err(|_| "parse_error")?.to_string())
}
fn req(envelope: &Value) -> Result<usize> {
    if host::check_cancel() {
        return Err("cancelled");
    }
    let bytes = serde_json::to_vec(envelope).map_err(|_| "invalid_request")?;
    if bytes.len() > http_req_buf().len() {
        return Err("request_too_large");
    }
    host::http_request(&bytes, http_out()).map_err(|_| "network_error")
}
fn get(url: &str, as_json: bool) -> Result<&'static [u8]> {
    let accept = if as_json {
        "application/json"
    } else {
        "text/html,application/xhtml+xml"
    };
    // Do not inject a birth date. The normal anonymous path stays anonymous;
    // any official web login or age permission is supplied by the user session.
    let envelope = json!({"version":1,"method":"GET","url":url,"responseKind":"bodyText","timeoutMs":15000,
        "headers":{"User-Agent":UA,"Referer":REFERER,"Accept":accept}});
    let n = req(&envelope)?;
    let n = protocol::body(&http_out()[..n], body_buf())?;
    Ok(&body_buf()[..n])
}
fn api(url: &str) -> Result<Value> {
    serde_json::from_slice(get(url, true)?).map_err(|_| "parse_error")
}
fn doc(url: &str) -> Result<Node> {
    Node::parse(get(url, false)?)
}
fn clock(request: &[u8]) -> Result<String> {
    if !protocol::feature(request, b"source_private_session_v1") {
        return Err("unsupported");
    }
    let n = req(&json!({"type":"private_session","version":1,"op":"clock"}))?;
    let response: Value = serde_json::from_slice(&http_out()[..n]).map_err(|_| "network_error")?;
    let ms = text(&response, "nowMs").map_err(|_| "network_error")?;
    if response["ok"] != true || !ms.bytes().all(|b| b.is_ascii_digit()) || ms.is_empty() {
        return Err("network_error");
    }
    Ok(ms.to_string())
}
fn manga(id: u64, title: &str, cover: Option<String>) -> Value {
    let mut manga = json!({"id":format!("tapas:{id}"),"title":title,"url":manga_url(id),"contentRating":"unknown"});
    if let Some(cover) = cover {
        manga["cover"] = json!({"kind":"url","url":cover})
    }
    manga
}
fn page_result(items: Vec<Value>, page: usize, more: bool) -> Value {
    json!({"items":items,"page":{"hasMore":more,"nextCursor":more.then(||(page+1).to_string())}})
}
fn manga_list(request: &[u8], args: &Value) -> Result<Value> {
    let page = request_page_number(request);
    if page > 100000 {
        return Err("invalid_request");
    }
    let listing = args["listingId"].as_str().unwrap_or("popular");
    let url=match listing{
        "popular"=>format!("{API}/ranking?category_type=COMIC&subtab_id=17&size=25&page={}",page-1),
        "latest"=>format!("{API}/genre?category_type=COMIC&sort_option=NEWEST_EPISODE&subtab_id=17&pageSize=25&page={}",page-1),
        _=>return Err("invalid_request"),
    };
    let response = api(&url)?;
    let items = response["data"]["items"]
        .as_array()
        .ok_or("parse_error")?
        .iter()
        .map(|entry| {
            let cover = entry["assetProperty"]["bookCoverImage"]
                .as_object()
                .and_then(|v| v.values().next())
                .and_then(Value::as_str)
                .map(|value| absolute(BASE, &format!("{value}.png")))
                .transpose()?;
            let mut row = manga(
                provider_id(entry, "seriesId")?,
                text(entry, "title")?,
                cover,
            );
            row["description"] = json!(text(entry, "description")?);
            Ok(row)
        })
        .collect::<Result<Vec<_>>>()?;
    let more = response["meta"]["pagination"]["last"].as_bool() == Some(false);
    Ok(page_result(items, page, more))
}
fn provider_manga_identity(doc: &Node) -> Result<u64> {
    let value = doc
        .require("meta[content^='tapastic://series/']")?
        .attr("content")?
        .ok_or("parse_error")?;
    let mid = value
        .strip_prefix("tapastic://series/")
        .and_then(|v| v.split('/').next())
        .ok_or("parse_error")?;
    positive(mid).map_err(|_| "parse_error")
}
fn search(request: &[u8], args: &Value) -> Result<Value> {
    let page = request_page_number(request);
    if page > 100000 {
        return Err("invalid_request");
    }
    let query = args["query"].as_str().unwrap_or("");
    if let Ok(url) = Url::parse(query) {
        if matches!(url.scheme(), "http" | "https") {
            let url = web(query)?;
            if url.host_str() != Some("tapas.io") || url.port().is_some() {
                return Err("not_implemented");
            }
            let mut parts = url.path_segments().ok_or("invalid_request")?;
            if parts.next() != Some("series") {
                return Err("not_implemented");
            }
            let slug = parts
                .next()
                .filter(|x| !x.is_empty())
                .ok_or("invalid_request")?;
            let id = match positive(slug) {
                Ok(id) => id,
                Err(_) => provider_manga_identity(&doc(url.as_str())?)?,
            };
            let item = manga_detail(id)?;
            return Ok(page_result(alloc::vec![item], page, false));
        }
    }
    let mut url = Url::parse(&format!("{BASE}/search")).map_err(|_| "invalid_request")?;
    url.query_pairs_mut()
        .append_pair("pageNumber", &page.to_string())
        .append_pair("q", query)
        .append_pair("t", "COMICS");
    let document = doc(url.as_str())?;
    let nodes = document.all(".search-item-wrap")?;
    if nodes.is_empty() && document.one("title").is_none() {
        return Err("parse_error");
    }
    let rows = nodes
        .iter()
        .map(|item| {
            let anchor = item.require(".item__thumb a, .title-section .title a")?;
            let id = positive(&anchor.attr("data-series-id")?.ok_or("parse_error")?)
                .map_err(|_| "parse_error")?;
            let cover = item.one(".item__thumb img, .thumb-wrap img");
            let title = cover
                .as_ref()
                .map(|img| img.attr("alt"))
                .transpose()?
                .flatten()
                .filter(|v| !v.is_empty());
            let title = match title {
                Some(value) => value,
                None => item.require(".title-section .title a")?.text()?,
            };
            let image = cover
                .map(|img| img.attr("src"))
                .transpose()?
                .flatten()
                .map(|u| absolute(url.as_str(), &u))
                .transpose()?;
            let mut row = manga(id, &title, image);
            if let Some(description) = item.one(".desc.force.mbm") {
                row["description"] = json!(description.text()?)
            }
            Ok(row)
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(page_result(
        rows,
        page,
        document.one("a[class*='paging__button--next']").is_some(),
    ))
}
fn description(d: &Node, title: &str) -> Result<String> {
    let mut text = d
        .one(".description__body")
        .map(|x| x.text())
        .transpose()?
        .unwrap_or_default();
    if let Some(t) = d.one(".stats > a[href^='/static-landing/genre?category=']") {
        text.push_str(&format!("\n\nType: {}\n", t.text()?))
    }
    if let Some(credits) = d.one(".colophon") {
        let whole = credits.whole()?;
        let mut lines = whole.lines();
        let first = lines.next().unwrap_or("").trim();
        let heading = first
            .strip_suffix(" (Comic)")
            .or_else(|| first.strip_suffix(" (Novel)"))
            .unwrap_or(first);
        let clean = if heading.eq_ignore_ascii_case(title) {
            lines.collect::<Vec<_>>().join("\n")
        } else {
            whole
        };
        text.push_str(&format!("\n\n{clean}\n"));
    }
    Ok(text)
}
fn manga_detail(mid: u64) -> Result<Value> {
    let route = manga_url(mid);
    let d = doc(&route)?;
    if provider_manga_identity(&d)? != mid {
        return Err("invalid_request");
    }
    let title = d.require(".info__right .title")?.text()?;
    let image = d
        .one(".thumb.js-thumbnail img")
        .map(|v| v.attr("src"))
        .transpose()?
        .flatten()
        .map(|u| absolute(&route, &u))
        .transpose()?;
    let mut data = manga(mid, &title, image);
    data["description"] = json!(description(&d, &title)?);
    data["links"] = json!([{"kind":"source","url":route}]);
    let authors = d
        .all(".creator-section .name")?
        .iter()
        .map(Node::text)
        .collect::<Result<Vec<_>>>()?;
    data["authors"] = json!(authors);
    let mut seen = BTreeSet::new();
    let mut tags = Vec::new();
    for genre in d.all(".genre-btn")? {
        let genre = genre.text()?;
        if seen.insert(genre.clone()) {
            tags.push(genre)
        }
    }
    data["tags"] = json!(tags);
    if let Some(schedule) = d.one(".schedule-ico:has(.sp-ico-updated-line-pwt) + .schedule-label") {
        let status = schedule.text()?.to_ascii_lowercase();
        data["status"] = json!(if status.contains("updates") {
            "ongoing"
        } else if status.contains("completed") {
            "completed"
        } else {
            "unknown"
        });
    }
    Ok(data)
}
fn toggle(request: &Value, key: &str, default: bool) -> bool {
    match &request["settings"][key] {
        Value::Bool(value) => *value,
        Value::String(value) if value == "true" => true,
        Value::String(value) if value == "false" => false,
        _ => default,
    }
}
fn bool_field(data: &Value, key: &str) -> Result<bool> {
    data[key].as_bool().ok_or("parse_error")
}
fn date(input: &str) -> Option<i64> {
    let s = input.as_bytes();
    if s.len() != 20
        || s[4] != b'-'
        || s[7] != b'-'
        || s[10] != b'T'
        || s[13] != b':'
        || s[16] != b':'
        || s[19] != b'Z'
    {
        return None;
    }
    let num = |a, b| input.get(a..b)?.parse::<i64>().ok();
    let year = num(0, 4)?;
    let m = num(5, 7)?;
    let d = num(8, 10)?;
    let h = num(11, 13)?;
    let min = num(14, 16)?;
    let sec = num(17, 19)?;
    if !(1..=12).contains(&m)
        || !(1..=31).contains(&d)
        || !(0..=23).contains(&h)
        || !(0..=59).contains(&min)
        || !(0..=59).contains(&sec)
    {
        return None;
    }
    let y = year - i64::from(m <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = m + if m > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let days = era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468;
    Some((days * 86400 + h * 3600 + min * 60 + sec) * 1000)
}
fn chapters(request: &[u8], full: &Value, args: &Value) -> Result<Value> {
    let mid = mid(args)?;
    let since = clock(request)?;
    let exclusions = protocol::feature(request, b"source_chapter_exclusions_v1");
    let locked_visible = !exclusions || toggle(full, "lockedChapterVisibilityPref", true);
    let scheduled_visible = !exclusions || toggle(full, "scheduledChapterVisibilityPref", true);
    let show_number = toggle(full, "showEpisodeNumber", false);
    let mut items = Vec::new();
    let mut excluded = Vec::new();
    let mut known = BTreeSet::new();
    for page in 1..=1000 {
        let url=format!("{BASE}/series/{mid}/episodes?page={page}&sort=NEWEST&since={since}&large=true&last_access=0&=");
        let response = api(&url)?;
        let group = &response["data"];
        let more = bool_field(&group["pagination"], "has_next")?;
        for entry in group["episodes"].as_array().ok_or("parse_error")? {
            let eid = provider_id(entry, "id")?;
            if !known.insert(eid) {
                return Err("parse_error");
            }
            let locked = !(bool_field(entry, "free")? || bool_field(entry, "unlocked")?);
            let scheduled = bool_field(entry, "scheduled")?;
            let id = cid(mid, eid);
            if (locked && !locked_visible) || (scheduled && !scheduled_visible) {
                excluded.push(id);
                continue;
            }
            let title = text(entry, "title")?;
            let number = entry["scene"].as_f64().ok_or("parse_error")?;
            let visible = if show_number {
                format!("{number} - {title}")
            } else {
                title.to_string()
            };
            let title = if locked {
                format!("🔒 {visible}")
            } else {
                visible
            };
            let mut item = json!({"id":id,"mangaId":format!("tapas:{mid}"),"title":title,"index":items.len(),
                "url":episode_url(eid),"number":number,"locked":locked});
            if let Some(ts) = entry["publish_date"].as_str().and_then(date) {
                item["dateUpload"] = json!(ts)
            }
            items.push(item);
        }
        if !more {
            let mut result = json!({"chapters":items});
            if !excluded.is_empty() {
                result["excludedChapterIds"] = json!(excluded)
            }
            return Ok(result);
        }
    }
    Err("result_too_large")
}
fn image_id(mid: u64, eid: u64, index: usize, url: &str) -> String {
    format!(
        "tapas-page:{mid}:{eid}:{index}:{:x}",
        Sha256::digest(url.as_bytes())
    )
}
fn episodes_id(doc: &Node) -> Result<(u64, u64)> {
    let identity = doc
        .require("meta[content^='tapastic://series/']")?
        .attr("content")?
        .ok_or("parse_error")?;
    let (mid, eid) = identity
        .strip_prefix("tapastic://series/")
        .and_then(|v| v.split_once("/episodes/"))
        .ok_or("parse_error")?;
    Ok((
        positive(mid).map_err(|_| "parse_error")?,
        positive(eid).map_err(|_| "parse_error")?,
    ))
}
fn pages(request: &[u8], full: &Value, args: &Value) -> Result<Value> {
    let (mid, eid) = chapter_owner(args)?;
    let route = episode_url(eid);
    let doc = doc(&route)?;
    if episodes_id(&doc)? != (mid, eid) {
        return Err("invalid_request");
    }
    if doc.one(".toolbar a[data-type='style']").is_some() {
        return Err("unsupported_chapter");
    }
    let mut rows = Vec::new();
    for (index, image) in doc.all("img.content__img")?.iter().enumerate() {
        let raw = image
            .attr("data-src")?
            .or(image.attr("src")?)
            .filter(|url| !url.is_empty())
            .ok_or("parse_error")?;
        let url = absolute(&route, &raw)?;
        let parsed = web(&url)?;
        rows.push(
            json!({"id":image_id(mid,eid,index,parsed.as_str()),"index":index,
            "image":{"kind":"url","url":url}}),
        );
    }
    if rows.is_empty() {
        let selector = format!(
            "button.js-have-to-sign[data-id='{eid}'][data-permalink='/episode/{eid}'] .btn-label"
        );
        if doc
            .one(&selector)
            .map(|node| node.text())
            .transpose()?
            .is_some_and(|label| label.eq_ignore_ascii_case("Unlock episode"))
        {
            return Err("chapter_paid");
        }
        return Err("chapter_unavailable");
    }
    if protocol::feature(request, b"text_page_image_v1") && toggle(full, "showAuthorsNotes", true) {
        if let Some(note) = doc.one("p.js-episode-story") {
            let whole = note.whole()?;
            let paragraphs = whole
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>();
            if !paragraphs.is_empty() {
                let creator = doc.require("a.name.js-fb-tracking")?.text()?;
                rows.push(
                    json!({"id":format!("tapas-note:{mid}:{eid}"),"index":rows.len(),
                    "image":{"kind":"text","text":{"template":"webtoon-author-notes",
                    "heading":format!("Author's Notes from {creator}"),"paragraphs":paragraphs}}}),
                );
            }
        }
    }
    Ok(json!({"pages":rows}))
}
fn image(args: &Value) -> Result<Value> {
    let pid = args["pageId"].as_str().ok_or("invalid_request")?;
    let url = web(args["url"].as_str().ok_or("invalid_request")?)?;
    let mid = if let Some(raw) = pid.strip_prefix("tapas:") {
        if args.get("chapterId").is_some() {
            return Err("invalid_request");
        }
        positive(raw)?
    } else {
        let mut parts = pid
            .strip_prefix("tapas-page:")
            .ok_or("invalid_request")?
            .split(':');
        let mid = positive(parts.next().ok_or("invalid_request")?)?;
        let eid = positive(parts.next().ok_or("invalid_request")?)?;
        let index = parts
            .next()
            .and_then(|x| x.parse().ok())
            .ok_or("invalid_request")?;
        if pid != image_id(mid, eid, index, url.as_str())
            || args
                .get("chapterId")
                .is_some_and(|x| x.as_str() != Some(&cid(mid, eid)))
        {
            return Err("invalid_request");
        }
        mid
    };
    if args
        .get("mangaUrl")
        .is_some_and(|value| value.as_str() != Some(&manga_url(mid)))
        || args
            .get("mangaId")
            .is_some_and(|value| value.as_str() != Some(&format!("tapas:{mid}")))
    {
        return Err("invalid_request");
    }
    Ok(json!({"url":url.as_str(),"headers":{"Referer":REFERER,"User-Agent":UA}}))
}
fn settings(request: &[u8]) -> Value {
    let mut settings = Vec::new();
    if protocol::feature(request, b"source_chapter_exclusions_v1") {
        settings.push(json!({"id":"lockedChapterVisibilityPref","name":"Show paywalled chapters","kind":"toggle",
            "default":"true","description":"Tapas requires sign-in or unlocking for some episodes."}));
        settings.push(json!({"id":"scheduledChapterVisibilityPref","name":"Show scheduled chapters","kind":"toggle","default":"true"}));
    }
    if protocol::feature(request, b"text_page_image_v1") {
        settings.push(json!({"id":"showAuthorsNotes","name":"Show author's notes","kind":"toggle","default":"true",
            "description":"Show the creator's notes at the end of an episode, when available."}));
    }
    settings.push(json!({"id":"showEpisodeNumber","name":"Show episode number in chapter name","kind":"toggle","default":"false",
        "description":"Prefix chapter names with their episode number. Refresh the chapter list to apply."}));
    json!({"settings":settings})
}
fn run(op: &str, request: &[u8]) -> u32 {
    let result = (|| {
        protocol::request(request)?;
        let args = protocol::direct(request, b"args")?.unwrap_or(request);
        let mut buffer = [0; 16384];
        let _ = protocol::text(args, b"mangaUrl", &mut buffer)?;
        let full: Value = serde_json::from_slice(request).map_err(|_| "invalid_request")?;
        let args: Value = serde_json::from_slice(args).map_err(|_| "invalid_request")?;
        match op {
            "get_listings" => Ok(
                json!({"listings":[{"id":"popular","name":"Popular","supportsPaging":true},{"id":"latest","name":"Latest","supportsPaging":true}]}),
            ),
            "get_manga_list" => manga_list(request, &args),
            "search" => search(request, &args),
            "get_manga" => Ok(json!({"manga":manga_detail(mid(&args)?)?})),
            "get_chapters" => chapters(request, &full, &args),
            "get_pages" => pages(request, &full, &args),
            "get_image_request" => image(&args),
            "get_settings" => Ok(settings(request)),
            "get_authentication" => Ok(
                json!({"authentication":{"kind":"web","loginUrl":LOGIN,"userAgent":UA},"authenticated":false}),
            ),
            _ => Err("not_implemented"),
        }
    })();
    match result{
        Ok(value)=>match serde_json::to_vec(&value){
            Ok(bytes) if bytes.len()<=payload_buf().len()=>{
                payload_buf()[..bytes.len()].copy_from_slice(&bytes);
                write_success_payload(op,bytes.len())
            }
            _=>write_error(op,"result_too_large","Tapas returned too much content."),
        },
        Err(code)=>write_error(op,code,match code{
            "chapter_paid"=>"This episode is locked on Tapas. Sign in to read an already unlocked episode, or unlock it on Tapas.",
            "chapter_unavailable"=>"Tapas did not provide comic pages for this episode. Check its original page.",
            "auth_required"=>"Sign in on Tapas to continue.",
            "forbidden"=>"Tapas rejected this request. Check the original page for access requirements.",
            "not_found"=>"Tapas did not provide this page.",
            "unsupported_chapter"=>"This episode is a novel rather than a comic.",
            "unsupported"=>"Update Koma to load this source.",
            _=>"Could not load Tapas. Retry or check the original page.",
        }),
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
