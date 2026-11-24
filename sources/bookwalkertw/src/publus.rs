use super::*;
use koma_source_sdk::{host, source_protocol};

#[path = "publus_pack.rs"]
mod pack;

const IMAGE_FEATURE: &[u8] = b"source_required_image_decode_wasm_v1";
const ROUTE_FEATURE: &[u8] = b"source_http_response_url_v1";
const PREVIEW: &str = "preview.bookwalker.com.tw";
const PURCHASED: &str = "pcreader.bookwalker.com.tw";

struct Content {
    url: Url,
    headers: Value,
    auth: Vec<(String, String)>,
    config: Value,
    keys: Option<pack::PageKeys>,
}

fn https_site(url: &Url, host: &str) -> bool {
    url.scheme() == "https" && url.host_str() == Some(host) && url.port().is_none()
}

fn browser_viewer(book: u64, viewer: Viewer) -> Result<Url> {
    port::absolute(WEB, &format!("/browserViewer/{book}/{}", viewer.path()))
}

fn viewer_route(request: &[u8], book: u64, viewer: Viewer) -> Result<Url> {
    if !source_protocol::feature(request, ROUTE_FEATURE) {
        return Err("host_feature_required");
    }
    if host::check_cancel() {
        return Err("cancelled");
    }
    let entry = browser_viewer(book, viewer)?;
    // The public viewer gets its content ID from the actual redirect URL.
    // Opt in only for this document request, with the manifest's exact origins.
    let request = json!({
        "version":1,"method":"GET","url":entry.as_str(),"includeResponseUrl":true,
        "headers":{"Accept":"text/html,application/xhtml+xml"},"responseKind":"bodyText","timeoutMs":15000
    });
    let encoded = serde_json::to_vec(&request).map_err(|_| "internal_error")?;
    let count = host::http_request(&encoded, http_out()).map_err(|_| "network_error")?;
    let size = source_protocol::body(&http_out()[..count], body_buf())?;
    let response: Value =
        serde_json::from_slice(&http_out()[..count]).map_err(|_| "network_error")?;
    let final_url = response["responseUrl"]
        .as_str()
        .ok_or("host_feature_required")?;
    let url = Url::parse(final_url).map_err(|_| "parse_error")?;
    if !url.username().is_empty() || url.password().is_some() || url.fragment().is_some() {
        return Err("parse_error");
    }
    if (official(&url) && url.path() == "/login")
        || core::str::from_utf8(&body_buf()[..size])
            .is_ok_and(|text| text.contains("<title>請登入會員</title>"))
    {
        return Err("auth_required");
    }
    let host = match viewer {
        Viewer::Trial => PREVIEW,
        Viewer::Read => PURCHASED,
    };
    if !https_site(&url, host) || !url.path().ends_with("/viewer.html") {
        return Err("chapter_unavailable");
    }
    Ok(url)
}

fn content(request: &[u8], book: u64, viewer: Viewer) -> Result<Content> {
    let viewer_url = viewer_route(request, book, viewer)?;
    let cid = viewer_url
        .query_pairs()
        .find(|(key, _)| key == "cid")
        .map(|(_, value)| value)
        .filter(|value| !value.is_empty())
        .ok_or("chapter_unavailable")?;
    let config_url = port::absolute(viewer_url.as_str(), "js/config.js")?;
    let config = port::request(
        config_url.as_str(),
        "GET",
        json!({
            "Accept":"*/*","Referer":viewer_url.as_str()
        }),
        None,
    )?;
    let domain = config
        .split_once("serverDomain = !location.port ? '")
        .and_then(|(_, contents)| contents.split_once('\'').map(|(domain, _)| domain))
        .ok_or("parse_error")?;
    let mut api = port::absolute(viewer_url.as_str(), domain)?;
    if !https_site(&api, viewer_url.host_str().ok_or("parse_error")?) {
        return Err("parse_error");
    }
    api.path_segments_mut()
        .map_err(|_| "parse_error")?
        .pop_if_empty()
        .push("c");
    api.query_pairs_mut()
        .append_pair("cid", &cid)
        .append_pair("BID", "0");
    let response = port::request(
        api.as_str(),
        "GET",
        json!({
            "Accept":"application/json","Referer":viewer_url.as_str()
        }),
        None,
    )?;
    let response: Value = serde_json::from_str(&response).map_err(|_| "parse_error")?;
    let raw = response["url"]
        .as_str()
        .filter(|url| !url.is_empty())
        .ok_or("chapter_unavailable")?;
    let url = port::absolute(api.as_str(), raw)?;
    if url.scheme() != "https" {
        return Err("parse_error");
    }
    let auth = &response["auth_info"];
    let mut pairs = Vec::new();
    if matches!(viewer, Viewer::Read) && auth.is_object() {
        for key in ["hti", "cfg", "uuid"] {
            if let Some(value) = auth[key]
                .as_str()
                .map(str::to_string)
                .or_else(|| auth[key].as_i64().map(|n| n.to_string()))
            {
                pairs.push((key.to_string(), value));
            }
        }
        pairs.push(("BID".to_string(), "0".to_string()));
    }
    for key in ["pfCd", "Policy", "Signature", "Key-Pair-Id"] {
        if let Some(value) = auth[key].as_str() {
            pairs.push((key.to_string(), value.to_string()));
        }
    }
    let headers = json!({"Accept":"application/json"});
    let config_url = content_url(&url, "configuration_pack.json", &pairs)?;
    let config = port::request(config_url.as_str(), "GET", headers.clone(), None)?;
    let config = serde_json::from_str(&config).map_err(|_| "parse_error")?;
    // Only unpack the configuration received through the official viewer and
    // its supplied content authorization. The derived keys are page metadata.
    let (config, keys) = pack::unpack(config).ok_or("parse_error")?;
    Ok(Content {
        url,
        headers: json!({}),
        auth: pairs,
        config,
        keys,
    })
}

fn content_url(base: &Url, path: &str, auth: &[(String, String)]) -> Result<Url> {
    let mut url = base.clone();
    {
        let mut segments = url.path_segments_mut().map_err(|_| "parse_error")?;
        segments.pop_if_empty();
        for part in path.split('/') {
            if part.is_empty() || matches!(part, "." | "..") {
                return Err("parse_error");
            }
            segments.push(part);
        }
    }
    if !auth.is_empty() {
        url.query_pairs_mut().extend_pairs(auth);
    }
    Ok(url)
}

fn entries(config: &Value) -> Result<&Vec<Value>> {
    config["configuration"]["contents"]
        .as_array()
        .ok_or("parse_error")
}

fn page_entry(config: &Value, index: usize) -> Result<(&str, i32, &Value)> {
    let entry = entries(config)?.get(index).ok_or("not_found")?;
    let file = entry["file"]
        .as_str()
        .filter(|file| !file.is_empty())
        .ok_or("parse_error")?;
    let page = &config[file]["FileLinkInfo"]["PageLinkInfoList"][0]["Page"];
    if !page.is_object() {
        return Err("parse_error");
    }
    let no = i32::try_from(integer(&page["No"])?).map_err(|_| "parse_error")?;
    if no < 0 {
        return Err("parse_error");
    }
    Ok((file, no, page))
}

fn integer(field: &Value) -> Result<i64> {
    if field.is_null() {
        Ok(0)
    } else {
        field.as_i64().ok_or("parse_error")
    }
}

fn dimension(field: &Value) -> Result<u32> {
    u32::try_from(integer(field)?).map_err(|_| "parse_error")
}

fn pixel_config(
    file: &str,
    no: i32,
    page: &Value,
    keys: Option<&pack::PageKeys>,
) -> Result<String> {
    let _ = integer(&page["DummyWidth"])?;
    let size = &page["Size"];
    let config = serde_json::to_string(&json!({
        "file": file, "no": no,
        "width": dimension(&size["Width"])?,
        "height": dimension(&size["Height"])?,
        "blockWidth": dimension(&page["BlockWidth"])?,
        "blockHeight": dimension(&page["BlockHeight"])?,
        "ns": integer(&page["NS"])?,
        "ps": integer(&page["PS"])?,
        "rs": integer(&page["RS"])?,
        "keys": keys,
    }))
    .map_err(|_| "internal_error")?;
    if config.len() > 64 * 1024 {
        return Err("result_too_large");
    }
    Ok(config)
}

pub(super) fn pages(request: &[u8], book: u64, viewer: Viewer) -> Result<Value> {
    let content = content(request, book, viewer)?;
    let chapter = product_url(book);
    let viewer_url = browser_viewer(book, viewer)?;
    let mut pages = Vec::new();
    for index in 0..entries(&content.config)?.len() {
        page_entry(&content.config, index)?;
        // A page snapshot carries only its stable locator. The short-lived
        // CDN signature and (on owned books) book auth are resolved for the
        // concrete image request, never written in the public page list.
        let mut locator = viewer_url.clone();
        locator
            .query_pairs_mut()
            .append_pair("koma_publus_page", &index.to_string());
        pages.push(port::page(index, &chapter, locator));
    }
    if pages.is_empty() {
        return Err("chapter_unavailable");
    }
    Ok(json!({"pages":pages}))
}

pub(super) fn image(request: &[u8], args: &Value) -> Result<Value> {
    let raw = args["url"].as_str().ok_or("invalid_request")?;
    let url = port::absolute(WEB, raw)?;
    if !official(&url) || !url.path().starts_with("/browserViewer/") {
        return Ok(json!({"url":url.as_str(),"headers":{}}));
    }
    let mut parts = url.path_segments().ok_or("invalid_request")?;
    let _ = parts.next();
    let book = id(parts.next().ok_or("invalid_request")?)?;
    let mode = match parts.next() {
        Some("trial") => Viewer::Trial,
        Some("read") => Viewer::Read,
        _ => return Err("invalid_request"),
    };
    if parts.next().is_some() {
        return Err("invalid_request");
    }
    let mut pairs = url.query_pairs();
    let index = pairs
        .next()
        .filter(|(key, _)| key == "koma_publus_page")
        .and_then(|(_, index)| index.parse::<usize>().ok())
        .ok_or("invalid_request")?;
    if pairs.next().is_some()
        || args["pageId"].as_str()
            != Some(&format!(
                "{}:{}",
                product_url(book),
                index.checked_add(1).ok_or("invalid_request")?
            ))
    {
        return Err("invalid_request");
    }
    let content = content(request, book, mode)?;
    let (file, no, page) = page_entry(&content.config, index)?;
    let keys = content.keys.as_ref();
    let image = pack::filename(file, no, keys);
    let url = content_url(&content.url, &image, &content.auth)?;
    let mut result = json!({"url":url.as_str(),"headers":content.headers});
    if !page["DummyWidth"].is_null() {
        if !source_protocol::feature(request, IMAGE_FEATURE) {
            return Err("host_feature_required");
        }
        result["requiredImageDecode"] = json!({
            "kind": "wasm_image_v1",
            "pixels": pixel_config(file, no, page, keys)?,
        });
    }
    Ok(result)
}
