// SPDX-License-Identifier: Apache-2.0
use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};
use alloc::{format, string::String, vec::Vec};
use base64::{engine::general_purpose::STANDARD, Engine};
use md5::{Digest, Md5};
use serde_json::{json, Value};
use url::Url;

use crate::{
    get, image_from, manga_path, numeric_id, path_segment, port, view_count, Node, Result, BASE,
};

fn chapter_route(args: &Value) -> Result<Url> {
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
        .unwrap_or_else(|| crate::series(work_id, None)?.canonical_path())?;
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

pub(crate) fn pages(args: &Value) -> core::result::Result<Value, PageError> {
    let mut url = chapter_route(args)?;
    let mut doc = get(&url)?;
    if doc.one("#single-pager").is_some() {
        url.query_pairs_mut().append_pair("style", "list");
        doc = get(&url)?;
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

pub(crate) fn image_request(args: &Value) -> Result<Value> {
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
    Ok(json!({"url":image.as_str(),"headers":{"Referer":referer.as_str()}}))
}
