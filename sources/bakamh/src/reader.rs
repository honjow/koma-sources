// SPDX-License-Identifier: Apache-2.0
use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};
use alloc::{format, string::String, vec::Vec};
use base64::{engine::general_purpose::STANDARD, Engine};
use md5::{Digest, Md5};
use koma_source_sdk::{
    body::BodyRef,
    http::{CallError, Client, Header, Request, RequestSpec, RequireSuccess, ResponseKind},
    runtime::Context,
};
use serde_json::{json, Value};
use url::Url;

use crate::{
    get, image_from, manga_path, numeric_id, path_segment, port, view_count, Node, Result, BASE,
};

fn chapter_route(args: &Value, request: &[u8]) -> Result<Url> {
    let chapter = args["chapterId"]
        .as_str()
        .and_then(|id| id.strip_prefix("chapter:"))
        .ok_or("invalid_request")?;
    let (work_id, slug) = chapter.split_once(':').ok_or("invalid_request")?;
    if !numeric_id(work_id) || !path_segment(slug) {
        return Err("invalid_request");
    }
    if let Some(id) = args["mangaId"].as_str() {
        if id.strip_prefix("manga:") != Some(work_id) {
            return Err("invalid_request");
        }
    }
    let path = args["mangaUrl"]
        .as_str()
        .and_then(manga_path)
        .map(Ok)
        .unwrap_or_else(|| crate::series(work_id, None, request)?.canonical_path())?;
    port::absolute(BASE, &format!("{path}{slug}/"))
}

fn encrypted_pages(script: &Node, base: &Url) -> Result<Vec<Url>> {
    const DATA_PREFIX: &str = "data:text/javascript;base64,";
    let code = match script
        .attr("src")
        .ok()
        .as_deref()
        .and_then(|src| src.strip_prefix(DATA_PREFIX))
    {
        Some(data) => String::from_utf8(decode64(data)?).map_err(|_| "parse_error")?,
        None => script.text()?,
    };
    let named = |name| {
        code.split_once(name)
            .and_then(|(_, suffix)| suffix.split_once("';"))
            .map(|(value, _)| value)
            .ok_or("parse_error")
    };
    let password = named("wpmangaprotectornonce='")?;
    let content = named("chapter_data='")?.replace("\\/", "/");
    let data: Value = serde_json::from_str(&content).map_err(|_| "parse_error")?;
    let salt = data["s"].as_str().ok_or("parse_error")?;
    if salt.len() % 2 != 0 || salt.is_empty() || !salt.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("parse_error");
    }
    let salt = (0..salt.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&salt[index..index + 2], 16).map_err(|_| "parse_error"))
        .collect::<Result<Vec<_>>>()?;
    let mut generated = [0u8; 48];
    for i in 0..3 {
        let mut digest = Md5::new();
        if i > 0 {
            digest.update(&generated[(i - 1) * 16..i * 16]);
        }
        digest.update(password.as_bytes());
        digest.update(&salt);
        generated[i * 16..(i + 1) * 16].copy_from_slice(&digest.finalize());
    }
    let mut bytes = decode64(data["ct"].as_str().ok_or("parse_error")?)?;
    let cipher = cbc::Decryptor::<aes::Aes256>::new_from_slices(&generated[..32], &generated[32..])
        .map_err(|_| "parse_error")?;
    let clear = cipher
        .decrypt_padded_mut::<Pkcs7>(&mut bytes)
        .map_err(|_| "parse_error")?;
    let inner: String = serde_json::from_slice(clear).map_err(|_| "parse_error")?;
    let urls: Vec<String> = serde_json::from_str(&inner).map_err(|_| "parse_error")?;
    urls.into_iter()
        .map(|url| port::absolute(base.as_str(), &url))
        .collect()
}

fn decode64(input: &str) -> Result<Vec<u8>> {
    let clean: Vec<u8> = input
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect();
    STANDARD.decode(clean).map_err(|_| "parse_error")
}

pub(crate) struct PageError {
    pub(crate) code: &'static str,
    pub(crate) message: Option<String>,
}

impl From<&'static str> for PageError {
    fn from(code: &'static str) -> Self {
        Self {
            code,
            message: None,
        }
    }
}

pub(crate) fn pages(args: &Value, request: &[u8]) -> core::result::Result<Value, PageError> {
    let mut url = chapter_route(args, request)?;
    let mut doc = get(&url, request)?;
    if doc.one("#single-pager").is_some() {
        url.query_pairs_mut().append_pair("style", "list");
        doc = get(&url, request)?;
    }
    view_count(&doc, &url);
    if let Some(lock) = doc.one(".mkjp-ea-lock__title") {
        if doc
            .one(".mkjp-ea-lock [data-target='#form-login']")
            .is_some()
        {
            return Err(PageError {
                code: "auth_required",
                message: Some("需在 WebView 中登录，账号有封禁风险".into()),
            });
        }
        return Err(PageError {
            code: "chapter_locked",
            message: Some(lock.text()?),
        });
    }
    if let Some(warning) = doc.one(".mkjp-tool-warning") {
        return Err(PageError {
            code: "chapter_unavailable",
            message: Some(warning.text()?),
        });
    }
    let images = if let Some(script) = doc.one("#chapter-protector-data") {
        encrypted_pages(&script, &url)?
            .into_iter()
            .enumerate()
            .collect::<Vec<_>>()
    } else {
        let mut images = Vec::new();
        let mut index = 0;
        for node in
            doc.all("div.page-break, li.blocks-gallery-item, .reading-content .text-left")?
        {
            let classes = node.attr("class").unwrap_or_default();
            let is_page = classes
                .split_whitespace()
                .any(|class| matches!(class, "page-break" | "blocks-gallery-item"));
            if !is_page && node.one(".blocks-gallery-item").is_some() {
                continue;
            }
            if let Some(image) = node.one("img").and_then(|image| image_from(&image, &url)) {
                images.push((index, image));
            }
            index += 1;
        }
        images
    };
    if images.is_empty() {
        return Err("chapter_unavailable".into());
    }
    // Page IDs carry the chapter document path so the image resolver can set
    // the exact Referer without loading the series or replaying view counts.
    let referer = format!(
        "{}{}",
        url.path(),
        url.query().map(|q| format!("?{q}")).unwrap_or_default()
    );
    let pages: Vec<_> = images.into_iter().map(|(index, image)| {
        json!({"id":format!("page:{index}:{referer}"),"index":index,"image":{"kind":"url","url":image.as_str()}})
    }).collect();
    Ok(json!({"pages":pages}))
}

pub(crate) fn image_request(args: &Value, request: &[u8]) -> Result<Value> {
    let page_id = args["pageId"].as_str().ok_or("invalid_request")?;
    let image = Url::parse(args["url"].as_str().ok_or("invalid_request")?)
        .map_err(|_| "invalid_request")?;
    let image = port::absolute(BASE, image.as_str())?;
    let referer = if let Some(page) = page_id.strip_prefix("page:") {
        let (_, path) = page.split_once(':').ok_or("invalid_request")?;
        let url = port::absolute(BASE, path)?;
        let mut segments = url.path().trim_matches('/').split('/');
        if url.host_str() != Some("bakamh.com")
            || segments.next() != Some("manga")
            || !segments.next().is_some_and(path_segment)
            || !segments.next().is_some_and(path_segment)
            || segments.next().is_some()
        {
            return Err("invalid_request");
        }
        url
    } else {
        port::absolute(BASE, "/")?
    };
    if image.scheme() == "https" && crate::image_body_supported(request) {
        return image_body(&image, &referer, request);
    }
    // No negotiated body channel: preserve the original descriptor byte shape.
    Ok(json!({"url":image.as_str(),"headers":{"Referer":referer.as_str()}}))
}

// Public Fetch.IMAGE headers from pinned upstream BrowserHeaders/Bakamh. The
// host owns the granted origin's UA/Cookie; no UA, Cookie or Sec-* is forged.
const IMAGE_ACCEPT: &[u8] = b"image/avif,image/webp,image/apng,image/svg+xml,image/*,*/*;q=0.8";

fn image_prefix(bytes: &[u8]) -> bool {
    bytes.starts_with(b"\x89PNG\r\n\x1a\n")
        || bytes.starts_with(b"\xff\xd8\xff")
        || bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a")
        || (bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP"))
        || bytes.starts_with(b"BM") || bytes.starts_with(b"II*\0") || bytes.starts_with(b"MM\0*")
        || bytes.starts_with(b"\0\0\x01\0") || bytes.starts_with(b"\0\0\x02\0")
        || bytes.starts_with(b"\xff\x0a") || bytes.starts_with(b"\0\0\0\x0cJXL \r\n\x87\n")
        || (bytes.get(4..8) == Some(b"ftyp") && bytes.get(8..).is_some_and(|brands|
            brands.chunks_exact(4).take(15).any(|brand|
                matches!(brand, b"avif" | b"avis" | b"heic" | b"heix" | b"hevc" | b"hevx" | b"mif1" | b"msf1"))))
}

fn image_body(image: &Url, referer: &Url, request: &[u8]) -> Result<Value> {
    let context = Context::from_request(request);
    if context.cancelled() { return Err("cancelled"); }
    let headers = [
        Header { name: b"Referer", value: referer.as_str().as_bytes() },
        Header { name: b"Accept", value: IMAGE_ACCEPT },
        Header { name: b"Accept-Language", value: crate::LANGUAGE.as_bytes() },
    ];
    let mut spec = RequestSpec::get(image.as_str().as_bytes());
    spec.headers = &headers;
    spec.response = ResponseKind::BodyRef;
    // BrowserGrant is the SDK default: the ordinary Host HTTP path attaches
    // only the already-authorized exact-origin session, including on redirects.
    spec.response_headers = context.supports(b"source_http_headers_v1");
    let request_capacity = crate::http_req_buf().len();
    let encoded = spec.encode(crate::http_req_buf())?;
    let mut web_request = Vec::new();
    // image_body is entered only after all three body features were negotiated.
    // Extend that same request only for the exact granted BASE origin; external
    // images and hosts without Web HTTP retain the R3 Native BodyRef path.
    let encoded = if image.origin().ascii_serialization() == BASE
        && context.supports(b"source_web_http_v1")
    {
        let prefix = encoded.as_json().strip_suffix(b"}").ok_or("internal_error")?;
        let extension = br#","webFetch":{"version":1}}"#;
        let length = prefix.len().checked_add(extension.len())
            .filter(|length| *length <= request_capacity).ok_or("buffer_overflow")?;
        web_request.reserve(length);
        web_request.extend_from_slice(prefix);
        web_request.extend_from_slice(extension);
        Request::from_json(&web_request)
    } else {
        encoded
    };
    let response = Client::<CallError>::new(&[&RequireSuccess])
        .send(encoded, crate::http_out()).map_err(|error| match error {
            CallError::Rejected(code) => code,
            _ => "network_error",
        })?;
    if !response.status().is_some_and(|status| (200..300).contains(&status)) {
        return Err("invalid_response");
    }
    let mime = response.header_values_json(b"content-type")?
        .map(serde_json::from_slice::<Vec<String>>).transpose().map_err(|_| "invalid_response")?
        .and_then(|values| values.into_iter().next())
        .map(|value| value.split(';').next().unwrap_or("").trim().to_ascii_lowercase());
    let input = response.body_ref()?;
    // Own the reference before any read/write reuses http_out; Response borrows it.
    drop(response);
    let copied = (|| -> Result<BodyRef> {
        if input.is_empty() { return Err("body_empty_image"); }
        if context.limit(b"body", b"maxBytes").is_some_and(|max| input.len() > max) {
            return Err("body_limit_exceeded");
        }
        let chunk_limit = context.limit(b"body", b"chunkBytes")
            .unwrap_or(koma_source_sdk::body::CHUNK_BYTES)
            .min(koma_source_sdk::body::CHUNK_BYTES);
        if chunk_limit == 0 { return Err("body_limit_exceeded"); }
        let mut output = BodyRef::create()?;
        let result = (|| -> Result<()> {
            let mut offset = 0;
            while offset < input.len() {
                if context.cancelled() { return Err("cancelled"); }
                let chunk = &mut crate::body_buf()[..chunk_limit.min(input.len() - offset)];
                let length = input.read(offset, chunk, crate::http_out())?;
                if length == 0 || length > chunk.len() { return Err("invalid_response"); }
                if offset == 0 {
                    let prefix = String::from_utf8_lossy(&chunk[..length.min(512)]).to_ascii_lowercase();
                    let error_document = prefix.contains("<html") || prefix.contains("<!doctype html")
                        || prefix.trim_start().starts_with('{') || prefix.trim_start().starts_with('[');
                    let declared_image = mime.as_deref().is_some_and(|mime| mime.starts_with("image/"));
                    if !image_prefix(&chunk[..length]) && (!declared_image || error_document) {
                        return Err("parse_error");
                    }
                }
                // HTTP body references are read-only. Copy bounded chunks to a
                // new writable reference before transferring ownership as image.
                output.write(offset, &chunk[..length], crate::http_out())?;
                offset += length;
            }
            Ok(())
        })();
        if let Err(code) = result { let _ = output.close(); return Err(code); }
        Ok(output)
    })();
    let closed = input.close();
    let output = copied?;
    if let Err(code) = closed { let _ = output.close(); return Err(code); }
    let length = output.image_result(crate::scratch_a())?;
    serde_json::from_slice(&crate::scratch_a()[..length]).map_err(|_| "internal_error")
}
