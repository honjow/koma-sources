use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use koma_source_sdk::{host, source_protocol};
use serde_json::{json, Value};
use url::Url;

pub(crate) type Result<T> = core::result::Result<T, &'static str>;

pub(crate) fn absolute(base: &str, input: &str) -> Result<Url> {
    if input.trim().is_empty() {
        return Err("parse_error");
    }
    let url = Url::parse(base)
        .and_then(|url| url.join(input))
        .map_err(|_| "parse_error")?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.host_str().is_none()
    {
        return Err("parse_error");
    }
    Ok(url)
}

pub(crate) fn request(
    url: &str,
    method: &str,
    headers: Value,
    body: Option<&str>,
) -> Result<String> {
    if host::check_cancel() {
        return Err("cancelled");
    }
    let mut envelope = json!({
        "version":1, "method":method, "url":url, "headers":headers,
        "responseKind":"bodyText", "timeoutMs":15000
    });
    if let Some(body) = body {
        envelope["body"] = json!(body);
    }
    let body = serde_json::to_vec(&envelope).map_err(|_| "internal_error")?;
    for attempt in 0..2 {
        if host::check_cancel() {
            return Err("cancelled");
        }
        let len = match host::http_request(&body, super::http_out()) {
            Ok(n) => n,
            Err(_) if attempt == 0 => continue,
            Err(_) => return Err("network_error"),
        };
        match source_protocol::body(&super::http_out()[..len], super::body_buf()) {
            Ok(n) => {
                return String::from_utf8(super::body_buf()[..n].to_vec())
                    .map_err(|_| "parse_error")
            }
            Err("network_error") if attempt == 0 => {
                let e: Value =
                    serde_json::from_slice(&super::http_out()[..len]).map_err(|_| "parse_error")?;
                if e["status"].as_u64().is_none_or(|s| (200..300).contains(&s)) {
                    continue;
                }
                return Err("network_error");
            }
            Err(code) => return Err(code),
        }
    }
    Err("network_error")
}

pub(crate) fn get(url: &str) -> Result<String> {
    request(
        url,
        "GET",
        json!({"Accept":"text/html,application/json"}),
        None,
    )
}

pub(crate) fn input(request: &[u8]) -> Result<Value> {
    source_protocol::request(request)?;
    serde_json::from_slice(request).map_err(|_| "invalid_request")
}

pub(crate) fn listing_list(latest_paged: bool, popular_paged: bool, filters: bool) -> Value {
    json!({"listings":[
        {"id":"popular","name":"热门","supportsPaging":popular_paged,"supportsFilters":filters},
        {"id":"latest","name":"最近更新","supportsPaging":latest_paged,"supportsFilters":filters}
    ]})
}

pub(crate) fn items(items: Vec<Value>, next: Option<usize>) -> Value {
    json!({"items":items,"page":{"hasMore":next.is_some(),"nextCursor":next.map(|page| page.to_string())}})
}

pub(crate) fn select(id: &str, label: &str, scope: &str, options: &[(&str, &str)]) -> Value {
    let options: Vec<_> = options
        .iter()
        .map(|(value, label)| json!({"value":value,"label":label}))
        .collect();
    json!({"type":"select","id":id,"name":label,"scope":scope,"default":"","options":options})
}

pub(crate) fn filter<'a>(args: &'a Value, id: &str, options: &[(&str, &str)]) -> Result<&'a str> {
    let Some(value) = args["filters"].get(id) else {
        return Ok("");
    };
    let value = value.as_str().ok_or("invalid_request")?;
    options
        .iter()
        .any(|(option, _)| *option == value)
        .then_some(value)
        .ok_or("invalid_request")
}

pub(crate) fn write_result(operation: &str, result: Result<Value>) -> u32 {
    match result {
        Ok(value) => match serde_json::to_vec(&value) {
            Ok(body) if body.len() <= super::payload_buf().len() => {
                super::payload_buf()[..body.len()].copy_from_slice(&body);
                super::write_success_payload(operation, body.len())
            }
            _ => write_failure(operation, "result_too_large"),
        },
        Err(code) => write_failure(operation, code),
    }
}

pub(crate) fn write_failure(operation: &str, code: &str) -> u32 {
    let message = match code {
        "forbidden" => "网站拒绝了本次请求，请检查原网页能否访问。",
        "not_found" => "原站未提供此内容。",
        "rate_limit" => "请求过于频繁，请稍后重试。",
        "network_error" => "原站请求未完成，请稍后重试。",
        "parse_error" => "原站返回的内容格式有变化，暂时无法读取。",
        "chapter_unavailable" => "原站未提供本章图片。",
        "chapter_locked" => "原站将此章节标为锁定。",
        "auth_required" => "请先在原站登录。",
        _ => "暂时无法获取此内容。",
    };
    super::write_error(operation, code, message)
}

pub(crate) fn chapter_error(error: koma_source_sdk::source::SourceError) -> &'static str {
    error.code()
}

#[link(wasm_import_module = "koma_host")]
extern "C" {
    #[link_name = "html_select_all"]
    fn select_all(
        node: i32,
        selector: *const u8,
        selector_len: u32,
        out: *mut i32,
        out_cap: u32,
    ) -> i32;
}

pub(crate) struct Node(host::HtmlDescriptor);

impl Drop for Node {
    fn drop(&mut self) {
        let _ = host::html_close(self.0);
    }
}

impl Node {
    pub(crate) fn parse(html: &str) -> Result<Self> {
        host::html_parse(html.as_bytes())
            .map(Self)
            .map_err(|_| "parse_error")
    }

    pub(crate) fn require(&self, selector: &str) -> Result<Self> {
        self.one(selector).ok_or("parse_error")
    }

    pub(crate) fn one(&self, selector: &str) -> Option<Self> {
        host::html_select(self.0, selector.as_bytes())
            .ok()
            .map(Self)
    }

    pub(crate) fn all(&self, selector: &str) -> Result<Vec<Self>> {
        let mut handles = vec![0i32; 16384];
        let count = unsafe {
            select_all(
                self.0.raw(),
                selector.as_ptr(),
                selector.len() as u32,
                handles.as_mut_ptr(),
                (handles.len() * core::mem::size_of::<i32>()) as u32,
            )
        };
        if count < 0 {
            return Err("parse_error");
        }
        if count as usize > handles.len() {
            for id in handles.into_iter().filter(|id| *id > 0) {
                let _ = host::html_close(host::HtmlDescriptor::from_raw(id));
            }
            return Err("result_too_large");
        }
        handles.truncate(count as usize);
        Ok(handles
            .into_iter()
            .map(|handle| Self(host::HtmlDescriptor::from_raw(handle)))
            .collect())
    }

    pub(crate) fn text(&self) -> Result<String> {
        let mut out = vec![0; 64 * 1024];
        let len = host::html_text(self.0, &mut out).map_err(|_| "parse_error")?;
        if len >= out.len() {
            return Err("result_too_large");
        }
        let value = core::str::from_utf8(&out[..len]).map_err(|_| "parse_error")?;
        Ok(value.split_whitespace().collect::<Vec<_>>().join(" "))
    }

    pub(crate) fn attr(&self, name: &str) -> Result<String> {
        self.attr_bounded(name, 8 * 1024 - 1)
    }

    pub(crate) fn attr_bounded(&self, name: &str, max_bytes: usize) -> Result<String> {
        let mut out = vec![0; max_bytes + 1];
        let len = host::html_attr(self.0, name.as_bytes(), &mut out).map_err(|_| "parse_error")?;
        if len >= out.len() {
            return Err("result_too_large");
        }
        out.truncate(len);
        String::from_utf8(out).map_err(|_| "parse_error")
    }
}

pub(crate) const MOBILE_IMAGE_AGENT: &str =
    "Mozilla/5.0 (Linux; Android 13; Mobile) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Mobile Safari/537.36";

pub(crate) fn mobile_image_headers(referer: &str) -> Value {
    json!({"Referer":referer,"User-Agent":MOBILE_IMAGE_AGENT})
}

pub(crate) fn mobile_cover_headers(value: &mut Value, referer: &str, request: &[u8]) {
    let mut headers = mobile_image_headers(referer);
    if !source_protocol::feature(request, b"source_public_cover_user_agent_v1") {
        if let Some(values) = headers.as_object_mut() {
            values.remove("User-Agent");
        }
    }
    let apply = |manga: &mut Value| {
        if let Some(cover) = manga.get_mut("cover") {
            cover["headers"] = headers.clone();
        }
    };
    if let Some(items) = value["items"].as_array_mut() {
        for entry in items {
            apply(entry);
        }
    } else if let Some(manga) = value.get_mut("manga") {
        apply(manga);
    }
}

pub(crate) fn page(index: usize, chapter: &str, image: Url) -> Value {
    json!({"id":format!("{chapter}:{}",index + 1),"index":index,"image":{"kind":"url","url":image.as_str()}})
}
