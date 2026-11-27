#![no_std]
extern crate alloc;
use alloc::{collections::BTreeSet, format, string::ToString, vec, vec::Vec};
use koma_source_sdk::source::SourceCapabilities;
use serde_json::{json, Value};
use url::Url;
#[path = "../../photo_gallery.rs"]
#[allow(dead_code)]
mod photo;
#[path = "../../port_support.rs"]
#[allow(dead_code)]
mod port;
#[path = "../../port_catalog.rs"]
mod port_catalog;
use photo::*;
use port::{Node, Result};
use port_catalog::Labels;
#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;
const SITE: &str = "https://www.junmeitu.com/";
const LANGUAGE: &str = "zh-Hans";
const METADATA: &[u8] = include_bytes!("../metadata.json");
const SOURCE_INFO:koma_source_sdk::source::SourceInfo=info("com.junmeitu.koma","俊美图","按模特、写真机构与主题整理的真人写真图库，也收录街拍与男士写真。套图按原站分页合并，含成人内容。");
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    image_request: true,
    ..CAPS
};
const LISTINGS: &[(&str, bool, bool)] = &[("popular", true, true), ("latest", true, true)];
const CATEGORIES: &[(&str, &str)] = &[
    ("beauty", "Women’s photo sets"),
    ("handsome", "Men’s photo sets"),
    ("street", "Street photography"),
];
const SORT: &[(&str, &str)] = &[("index", "Latest"), ("hot", "Popular")];
koma_source_sdk::koma_source_buffers! {payload:512*1024,http_out:2*1024*1024,body:1024*1024,http_req:4096,scratch:8192,}
koma_source_sdk::koma_source_helpers!();
fn safe(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 400
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}
fn site(raw: &str) -> Result<Url> {
    let u = https(SITE, raw)?;
    if u.host_str() != Some("www.junmeitu.com") || u.query().is_some() {
        return Err("invalid_request");
    }
    Ok(u)
}
fn key(raw: &str) -> Result<(&str, &str)> {
    let (c, s) = raw.split_once('/').ok_or("invalid_request")?;
    if !CATEGORIES.iter().any(|(v, _)| *v == c) || !safe(s) {
        return Err("invalid_request");
    }
    Ok((c, s))
}
fn album(raw: &str) -> Result<Url> {
    let (c, s) = key(raw)?;
    site(&format!("{c}/{s}.html"))
}
fn link_key(u: &Url) -> Result<&str> {
    let s = u
        .path()
        .strip_prefix('/')
        .and_then(|s| s.strip_suffix(".html"))
        .ok_or("parse_error")?;
    key(s)?;
    Ok(s)
}
fn media(raw: &str, full: bool) -> Option<Url> {
    let u = https(SITE, raw).ok()?;
    let path = u.path();
    let valid = matches!(u.host_str(), Some("i.wujituku.com" | "s.wujituku.com"))
        && !path.contains("/statics/")
        && path
            .rsplit_once('.')
            .is_some_and(|(_, e)| matches!(e, "jpg" | "jpeg" | "webp" | "png" | "avif"));
    // /re/ and thumb_* are catalogue/recommendation derivatives. They may be
    // covers, but may NEVER enter a gallery's full-size page list.
    (valid && (!full || (!path.contains("thumb_") && !path.starts_with("/re/")))).then_some(u)
}
fn image(n: &Node, full: bool) -> Option<Url> {
    lazy_image(
        n,
        &["data-original", "data-src", "data-lazy-src", "src"],
        |s| media(s, full),
    )
}
fn detail(raw: &str) -> Result<Gallery> {
    let u = album(raw)?;
    let d = document(&u)?;
    let title = d.require("h1.title, h1.news-title")?.text()?;
    let content = d.require(".pictures, .news-body")?;
    let cover = content
        .all("img")?
        .iter()
        .find_map(|n| image(n, true))
        .ok_or("parse_error")?;
    let tags = d
        .all(".relation_tags > a")?
        .iter()
        .map(Node::text)
        .collect::<Result<Vec<_>>>()?;
    Ok(Gallery {
        key: raw.into(),
        url: u,
        title,
        cover: Some(cover),
        tags,
        published: None,
    })
}
fn browse(a: &Value, search: bool, _: &[u8]) -> Result<Value> {
    let page = page_number(a)?;
    let listing = a["listingId"].as_str().unwrap_or("latest");
    let listing = listing.strip_prefix("listing:").unwrap_or(listing);
    if !search && !matches!(listing, "popular" | "latest") {
        return Err("invalid_request");
    }
    let category = match selection(a, "category")? {
        "" => "beauty",
        s if CATEGORIES.iter().any(|(v, _)| *v == s) => s,
        _ => return Err("invalid_request"),
    };
    let sort = if search {
        match selection(a, "sort")? {
            "" | "index" => "index",
            "hot" => "hot",
            _ => return Err("invalid_request"),
        }
    } else if listing == "popular" {
        "hot"
    } else {
        "index"
    };
    let query = if search {
        a["query"].as_str().unwrap_or("").trim()
    } else {
        ""
    };
    if query.len() > 512 || query.chars().any(char::is_control) {
        return Err("invalid_request");
    }
    let tag = selection(a, "tag")?.trim();
    let model = selection(a, "model")?.trim();
    let group = selection(a, "group")?.trim();
    if [tag, model, group].iter().filter(|s| !s.is_empty()).count() > 1 {
        return Err("invalid_request");
    }
    let mut url = if !query.is_empty() {
        let mut u = https(SITE, "search/")?;
        u.query_pairs_mut()
            .extend_pairs([("q", query), ("page", &page.to_string())]);
        u
    } else if !tag.is_empty() {
        let t = tag
            .strip_prefix("https://www.junmeitu.com/tags/")
            .unwrap_or(tag)
            .trim_end_matches(".html");
        if !safe(t) {
            return Err("invalid_request");
        };
        let cat = match category {
            "beauty" => 6,
            "handsome" => 7,
            "street" => 63,
            _ => return Err("invalid_request"),
        };
        site(&format!("tags/{t}-{cat}-{page}.html"))?
    } else if !model.is_empty() || !group.is_empty() {
        let (folder, s) = if !model.is_empty() {
            ("model", model)
        } else {
            ("xzjg", group)
        };
        if !safe(s) {
            return Err("invalid_request");
        };
        site(&format!("{folder}/{s}-{page}.html"))?
    } else {
        site(&format!("{category}/{sort}-{page}.html"))?
    };
    url.set_fragment(None);
    let d = document(&url)?;
    let root = d.require(".pic-list")?;
    let mut seen = BTreeSet::new();
    let mut items = Vec::new();
    for row in root.all("ul > li")? {
        let Some(a) = row.one("a[href]") else {
            continue;
        };
        let link = site(&a.attr("href")?)?;
        let Ok(key) = link_key(&link) else { continue };
        if !seen.insert(key.to_string()) {
            continue;
        };
        let Some(img) = row.one("img") else { continue };
        let Some(cover) = image(&img, false) else {
            continue;
        };
        let title = row
            .one("p")
            .map(|n| n.text())
            .transpose()?
            .unwrap_or(img.attr("alt").unwrap_or_default());
        if title.is_empty() {
            return Err("parse_error");
        };
        items.push(
            Gallery {
                key: key.into(),
                url: link,
                title,
                cover: Some(cover),
                tags: vec![],
                published: None,
            }
            .manga(),
        );
    }
    let next = d.all(".pages a")?.iter().any(|a| {
        a.text().is_ok_and(|s| s == "下一頁" || s == "下一页")
            && a.attr("href")
                .ok()
                .and_then(|s| port::absolute(url.as_str(), &s).ok())
                .is_some_and(|u| u.as_str() != url.as_str())
    });
    Ok(port::items(items, next.then_some(page + 1)))
}
fn filters(a: &Value, _: &[u8], l: &Labels<'_>) -> Result<Value> {
    let mut filters = Vec::new();
    let search = a["listingId"].as_str().is_none_or(str::is_empty);
    for (id, choices) in [("category", CATEGORIES), ("sort", SORT)] {
        if id == "sort" && !search {
            continue;
        };
        let options = choices
            .iter()
            .map(|(v, t)| Ok(option(v, &l.text(&format!("option.{id}.{v}"), t)?)))
            .collect::<Result<Vec<_>>>()?;
        filters.push(json!({"id":id,"type":"select","name":l.text(&format!("filter.{id}"),id)?,"default":if id=="category"{"beauty"}else{"index"},"options":options}));
    }
    for id in ["tag", "model", "group"] {
        filters.push(json!({"id":id,"type":"text","name":l.text(&format!("filter.{id}"),id)?,"description":l.text("filter.hint","Use one site code; leave the keyword empty.")?,"default":""}));
    }
    Ok(json!({"filters":filters}))
}
fn total(d: &Node, root: &Url) -> Result<usize> {
    let mut max = 1;
    for a in d.all(".pages a")? {
        let t = a.text()?;
        if let Ok(n) = t.parse::<usize>() {
            if n == 0 || n > 1000 {
                return Err("result_too_large");
            };
            let u = site(&a.attr("href")?)?;
            let expected = part_url(root, n)?;
            if u != expected {
                return Err("catalog_changed");
            };
            max = max.max(n);
        }
    }
    if let Some(n) = d
        .one(".pages span")
        .and_then(|s| s.text().ok())
        .and_then(|s| s.parse::<usize>().ok())
    {
        max = max.max(n)
    }
    Ok(max)
}
fn part_url(root: &Url, n: usize) -> Result<Url> {
    if n == 1 {
        return Ok(root.clone());
    }
    let base = root.path().strip_suffix(".html").ok_or("parse_error")?;
    site(&format!("{base}-{n}.html"))
}
fn get_pages(raw: &str) -> Result<Value> {
    let root = album(raw)?;
    let first = document(&root)?;
    let expected = total(&first, &root)?;
    let title = first.require("h1.title, h1.news-title")?.text()?;
    let mut seen = BTreeSet::new();
    let mut urls = Vec::new();
    for part in 1..=expected {
        let current;
        let d = if part == 1 {
            &first
        } else {
            current = document(&part_url(&root, part)?)?;
            &current
        };
        let current_title = d.require("h1.title, h1.news-title")?.text()?;
        // The real site appends the current HTML page number to its title.
        // Accept only that exact suffix, never an unrelated redirected gallery.
        let expected_title = if part == 1 {
            title.clone()
        } else {
            format!("{title} ({part})")
        };
        if total(d, &root)? != expected || current_title != expected_title {
            return Err("catalog_changed");
        }
        if expected > 1 && d.require(".pages span")?.text()?.parse::<usize>().ok() != Some(part) {
            return Err("catalog_changed");
        }
        let container = d.require(".pictures, .news-body")?;
        let mut count = 0;
        for n in container.all("img")? {
            let u = image(&n, true).ok_or("parse_error")?;
            if !seen.insert(u.as_str().to_string()) {
                return Err("catalog_changed");
            };
            urls.push(u);
            count += 1;
        }
        if count == 0 {
            return Err("chapter_unavailable");
        }
    }
    // Every advertised HTML page is accounted for. Never report success after a
    // missing middle page, a repeated redirect, or a recommendation thumbnail.
    let mut out = pages(raw, urls)?;
    if let Some(list) = out["pages"].as_array_mut() {
        for page in list {
            page["image"]["headers"] = json!({"Referer":SITE});
        }
    }
    Ok(out)
}
fn run_get_image_request(r: &[u8]) -> u32 {
    let v = port::input(r).and_then(|v| {
        let u = v["args"]["url"]
            .as_str()
            .and_then(|s| media(s, false))
            .ok_or("invalid_request")?;
        Ok(json!({"imageRequest":{"url":u.as_str(),"headers":{"Referer":SITE}}}))
    });
    port_catalog::write_localized(METADATA, r, "get_image_request", v)
}
use photo::{
    run_get_chapters, run_get_filters, run_get_home, run_get_listings, run_get_manga,
    run_get_manga_list, run_get_pages, run_get_settings, run_search, write_chapter_failure,
};
koma_source_sdk::koma_source_exports!("junmeitu", typed_chapters, write_chapter_failure);
