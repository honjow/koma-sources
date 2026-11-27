//! WordPress REST post projections, numbered cursors and gallery image parsing.
use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use serde_json::Value;
use url::Url;

use crate::{
    photo::{self, lazy_image, Gallery},
    port::{self, Node, Result},
};

pub(crate) const PAGE_SIZE: usize = 20;
const JSON: &str = "application/json";

pub(crate) struct Post<'a>(pub(crate) &'a Value);

impl Post<'_> {
    pub(crate) fn title(&self) -> Result<String> {
        html_text(self.0["title"]["rendered"].as_str().ok_or("parse_error")?)
    }

    pub(crate) fn link(&self) -> Result<&str> {
        self.0["link"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or("parse_error")
    }

    pub(crate) fn cover(&self) -> Option<&str> {
        self.0["jetpack_featured_media_url"]
            .as_str()
            .filter(|s| !s.is_empty())
            .or_else(|| self.0["_embedded"]["wp:featuredmedia"][0]["source_url"].as_str())
    }

    pub(crate) fn content(&self) -> Result<&str> {
        self.0["content"]["rendered"].as_str().ok_or("parse_error")
    }

    pub(crate) fn date(&self) -> Option<String> {
        self.0["date_gmt"]
            .as_str()
            .filter(|s| s.len() == 19)
            .map(|s| format!("{s}Z"))
            .or_else(|| self.0["date"].as_str().map(String::from))
    }

    pub(crate) fn tags(&self) -> Vec<String> {
        let mut tags = Vec::new();
        for group in self.0["_embedded"]["wp:term"]
            .as_array()
            .into_iter()
            .flatten()
        {
            for term in group.as_array().into_iter().flatten() {
                if let Some(name) = term["name"].as_str().and_then(|s| html_text(s).ok()) {
                    if !tags.contains(&name) {
                        tags.push(name);
                    }
                }
            }
        }
        tags
    }
}

pub(crate) fn html_text(value: &str) -> Result<String> {
    if !value.contains(['&', '<']) {
        return (!value.trim().is_empty())
            .then(|| value.trim().into())
            .ok_or("parse_error");
    }
    let node = Node::parse(&format!("<span>{value}</span>"))?;
    let text = node.require("span")?.text()?;
    (!text.is_empty()).then_some(text).ok_or("parse_error")
}

pub(crate) fn list(
    mut url: Url,
    page: usize,
    fields: &str,
    params: &[(&str, &str)],
    request: &[u8],
    item: impl Fn(Post<'_>) -> Result<Gallery>,
) -> Result<Value> {
    url.query_pairs_mut()
        .append_pair("page", &page.to_string())
        .append_pair("per_page", &PAGE_SIZE.to_string())
        .append_pair("_fields", fields)
        .append_pair("_embed", "wp:featuredmedia")
        .extend_pairs(params.iter().copied().filter(|(_, s)| !s.is_empty()));
    let headers = koma_source_sdk::source_protocol::feature(request, b"source_http_headers_v1");
    let response = photo::fetch_with_headers(&url, JSON, headers)?;
    let data: Value = serde_json::from_str(&response.body).map_err(|_| {
        if response.status == 400 {
            "invalid_request"
        } else {
            "parse_error"
        }
    })?;
    if response.status == 400 {
        if data["code"] == "rest_post_invalid_page_number" {
            return Ok(port::items(Vec::new(), None));
        }
        return Err("invalid_request");
    }
    let posts = data.as_array().ok_or("parse_error")?;
    let total = response
        .headers
        .as_object()
        .and_then(|headers| {
            headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("x-wp-totalpages"))
        })
        .and_then(|(_, v)| v.as_str().or_else(|| v[0].as_str()))
        .and_then(|s| s.parse::<usize>().ok());
    // The server's total is authoritative. Older hosts may omit response headers.
    let more = !posts.is_empty() && total.map_or(posts.len() == PAGE_SIZE, |total| page < total);
    let items = posts
        .iter()
        .filter_map(|post| item(Post(post)).ok())
        .map(|gallery| gallery.manga())
        .collect::<Vec<_>>();
    if items.is_empty() && !posts.is_empty() {
        return Err("parse_error");
    }
    Ok(port::items(items, more.then_some(page + 1)))
}

pub(crate) fn rest(url: &Url) -> Result<Value> {
    let response = photo::fetch(url, JSON)?;
    if response.status == 400 {
        return Err("invalid_request");
    }
    serde_json::from_str(&response.body).map_err(|_| "parse_error")
}

pub(crate) fn positive_id(raw: &str) -> Result<u32> {
    raw.parse::<u32>()
        .ok()
        .filter(|&n| n != 0 && n.to_string() == raw)
        .ok_or("invalid_request")
}

pub(crate) fn image(base: &str, raw: &str, permitted: impl Fn(&Url) -> bool) -> Option<Url> {
    let url = photo::https(base, raw).ok()?;
    let is_image = url.path().rsplit_once('.').is_some_and(|(_, ext)| {
        ["jpg", "jpeg", "png", "gif", "webp", "avif"]
            .iter()
            .any(|known| known.eq_ignore_ascii_case(ext))
    });
    (is_image && permitted(&url)).then_some(url)
}

pub(crate) const LAZY_FIRST: &[&str] = &["data-lazy-src", "data-src", "src"];

pub(crate) fn images(
    root: &Node,
    anchors_first: bool,
    attributes: &[&str],
    resolve: impl Fn(&str) -> Option<Url>,
) -> Result<Vec<Url>> {
    let mut urls = Vec::new();
    if anchors_first {
        // Walk links and standalone images in document order. A linked thumbnail
        // uses the large href, but does not hide standalone images in mixed posts.
        for node in root.all("a[href]:not(noscript a), img:not(a[href] img):not(noscript img)")? {
            if let Some(image) = resolve(&node.attr("href")?) {
                urls.push(image);
            } else if let Some(image) = lazy_image(&node, attributes, &resolve) {
                urls.push(image);
            } else {
                for child in node.all("img:not(noscript img)")? {
                    if let Some(image) = lazy_image(&child, attributes, &resolve) {
                        urls.push(image);
                    }
                }
            }
        }
    } else {
        for node in root.all("img:not(noscript img)")? {
            if let Some(url) = lazy_image(&node, attributes, &resolve) {
                urls.push(url);
            }
        }
    }
    Ok(photo::unique(urls))
}
