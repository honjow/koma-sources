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
const SITE: &str = "https://www.meitulu.me/";
const LANGUAGE: &str = "zh-Hans";
const METADATA: &[u8] = include_bytes!("../metadata.json");
const SOURCE_INFO: koma_source_sdk::source::SourceInfo = info(
    "com.meitulu.koma", "美图录",
    "按模特、写真机构与主题整理的高清真人写真图库，收录内地、港台及日韩套图。图集按原站分页完整合并，含成人内容。",
);
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    image_request: true,
    ..CAPS
};
const LISTINGS: &[(&str, bool, bool)] = &[("popular", true, true), ("latest", true, true)];
const CATEGORIES: &[(&str, &str)] = &[
    ("", "All"),
    ("xihuan", "Featured"),
    ("guochan", "Mainland China"),
    ("gangtai", "Hong Kong and Taiwan"),
    ("rihan", "Japan and Korea"),
];
koma_source_sdk::koma_source_buffers! {payload:512*1024,http_out:4*1024*1024,body:2*1024*1024,http_req:4096,scratch:8192,}
koma_source_sdk::koma_source_helpers!();
fn safe(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 120
        && s.chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '@'))
}
fn valid_id(s: &str) -> bool {
    !s.is_empty() && s.len() <= 12 && s.bytes().all(|b| b.is_ascii_digit())
}
fn site(raw: &str) -> Result<Url> {
    let u = https(SITE, raw)?;
    if u.host_str() != Some("www.meitulu.me") || u.query().is_some() {
        return Err("invalid_request");
    }
    Ok(u)
}
// This is an ordinary public website request, not a WebView verification UA.
fn document(u: &Url) -> Result<Node> {
    Node::parse(&port::request(
        u.as_str(),
        "GET",
        json!({"Accept":"text/html", "User-Agent":"Mozilla/5.0", "Referer":SITE}),
        None,
    )?)
}
fn album(raw: &str) -> Result<Url> {
    if !valid_id(raw) {
        return Err("invalid_request");
    }
    site(&format!("item/{raw}.html"))
}
fn link_key(u: &Url) -> Result<&str> {
    u.path()
        .strip_prefix("/item/")
        .and_then(|s| s.strip_suffix(".html"))
        .filter(|s| valid_id(s))
        .ok_or("parse_error")
}
fn media(raw: &str, full_id: Option<&str>) -> Option<Url> {
    let u = site(raw).ok()?;
    let p = u.path();
    let valid = match full_id {
        Some(id) => p
            .strip_prefix(&format!("/p/{id}-"))
            .and_then(|s| s.strip_suffix(".jpg"))
            .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_hexdigit())),
        None => {
            p.strip_prefix("/poster/")
                .and_then(|s| s.strip_suffix(".jpg"))
                .is_some_and(valid_id)
                || p.strip_prefix("/p/")
                    .and_then(|s| s.strip_suffix(".jpg"))
                    .and_then(|s| s.split_once('-'))
                    .is_some_and(|(id, hash)| {
                        valid_id(id)
                            && !hash.is_empty()
                            && hash.bytes().all(|b| b.is_ascii_hexdigit())
                    })
        }
    };
    valid.then_some(u)
}
fn detail(raw: &str) -> Result<Gallery> {
    let u = album(raw)?;
    let d = document(&u)?;
    let title = d.require("h5.top-title")?.text()?;
    let cover = d.require(".container-inner-fix-m > img")?.attr("src")?;
    let cover = media(&cover, Some(raw)).ok_or("parse_error")?;
    let mut seen = BTreeSet::new();
    let tags = d
        .all("section.mb-3 a[href^='/t/']")?
        .iter()
        .map(Node::text)
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .filter(|s| seen.insert(s.clone()))
        .collect();
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
    if !search && !matches!(listing, "popular" | "latest") {
        return Err("invalid_request");
    }
    let category = selection(a, "category")?;
    if !CATEGORIES.iter().any(|(k, _)| *k == category) {
        return Err("invalid_request");
    }
    let query = if search {
        a["query"].as_str().unwrap_or("").trim()
    } else {
        ""
    };
    if query.len() > 512 || query.chars().any(char::is_control) {
        return Err("invalid_request");
    }
    let codes = [
        selection(a, "tag")?.trim(),
        selection(a, "model")?.trim(),
        selection(a, "group")?.trim(),
    ];
    if codes.iter().filter(|s| !s.is_empty()).count() > 1 {
        return Err("invalid_request");
    }
    let code = codes.iter().copied().find(|s| !s.is_empty());
    let home = query.is_empty()
        && code.is_none()
        && category.is_empty()
        && (search || listing == "latest");
    let mut u = if !query.is_empty() {
        // The current search form returns a bounded, unpaged result set.
        if page != 1 {
            return Err("invalid_request");
        }
        let mut u = https(SITE, "search")?;
        u.query_pairs_mut().append_pair("q", query);
        u
    } else if let Some(code) = code {
        if !safe(code) {
            return Err("invalid_request");
        }
        site(&if page == 1 {
            format!("t/{code}/")
        } else {
            format!("t/{code}/index_{page}.html")
        })?
    } else if home {
        if page != 1 {
            return Err("invalid_request");
        }
        site("/")?
    } else {
        let cat = if category.is_empty() {
            "xihuan"
        } else {
            category
        };
        if cat == "xihuan" && page != 1 {
            return Err("invalid_request");
        }
        site(&if page == 1 {
            format!("{cat}/")
        } else {
            format!("{cat}/index_{page}.html")
        })?
    };
    u.set_fragment(None);
    // Isolate the official latest section before parsing. This excludes featured,
    // random galleries and popular-model cards without inventing a sorted feed.
    let d = if home {
        let html = port::request(
            u.as_str(),
            "GET",
            json!({"Accept":"text/html", "User-Agent":"Mozilla/5.0", "Referer":SITE}),
            None,
        )?;
        let start = [">最新发布</h5>", ">最新發布</h5>"]
            .iter()
            .find_map(|marker| html.find(marker).map(|i| i + marker.len()))
            .ok_or("parse_error")?;
        let rest = &html[start..];
        let end = rest.find("<h5").ok_or("parse_error")?;
        Node::parse(&rest[..end])?
    } else {
        document(&u)?
    };
    let root = if home {
        d.require(".row.my-gutters-b")?
    } else {
        d.require(".container.pt-3")?
    };
    let mut seen = BTreeSet::new();
    let mut items = Vec::new();
    for row in root.all(".my-card")? {
        let Some(a) = row.one("a[href^='/item/']") else {
            continue;
        };
        let link = site(&a.attr("href")?)?;
        let key = link_key(&link)?;
        if !seen.insert(key.to_string()) {
            continue;
        }
        let cover = media(&a.require("img")?.attr("src")?, None).ok_or("parse_error")?;
        // Title links are siblings of .my-card. Resolve by exact item path in root.
        let selector = format!("a[href='{}'] .my-card-title", link.path());
        let title = root.require(&selector)?.text()?;
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
    if items.is_empty() && !search {
        return Err("parse_error");
    }
    let next = d.all(".my-pagination a[href]")?.iter().any(|n| {
        n.text().is_ok_and(|s| s == "下一页" || s == "下一頁")
            && n.attr("href")
                .ok()
                .and_then(|s| https(u.as_str(), &s).ok())
                .is_some_and(|next| next != u)
    });
    Ok(port::items(items, next.then_some(page + 1)))
}
fn filters(_: &Value, _: &[u8], l: &Labels<'_>) -> Result<Value> {
    let opts = CATEGORIES
        .iter()
        .map(|(k, t)| Ok(option(k, &l.text(&format!("option.category.{k}"), t)?)))
        .collect::<Result<Vec<_>>>()?;
    let mut out = vec![
        json!({"id":"category","type":"select","name":l.text("filter.category","Category")?,"default":"","options":opts}),
    ];
    for id in ["tag", "model", "group"] {
        out.push(json!({"id":id,"type":"text","name":l.text(&format!("filter.{id}"),id)?,"description":l.text("filter.hint","Use one site code and leave the keyword empty.")?,"default":""}));
    }
    Ok(json!({"filters":out}))
}
fn part_url(root: &Url, n: usize) -> Result<Url> {
    if n == 1 {
        return Ok(root.clone());
    }
    let base = root.path().strip_suffix(".html").ok_or("parse_error")?;
    site(&format!("{base}_{n}.html"))
}
fn total(d: &Node, root: &Url) -> Result<usize> {
    let mut max = 1;
    for n in d.all(".my-pagination a")? {
        if let Ok(page) = n.text()?.parse::<usize>() {
            if page == 0 || page > 1000 {
                return Err("result_too_large");
            }
            if let Ok(href) = n.attr("href") {
                if https(root.as_str(), &href)? != part_url(root, page)? {
                    return Err("catalog_changed");
                }
            }
            max = max.max(page);
        }
    }
    Ok(max)
}
fn image_count(d: &Node) -> Result<usize> {
    for n in d.all("section.mb-3 > div")? {
        let s = n.text()?;
        if s.starts_with("图片数量：") || s.starts_with("圖片數量：") {
            let v = s.split_once('：').ok_or("parse_error")?.1;
            return v
                .trim_end_matches(['张', '張'])
                .parse::<usize>()
                .ok()
                .filter(|n| *n > 0 && *n <= 6000)
                .ok_or("parse_error");
        }
    }
    Err("parse_error")
}
fn get_pages(raw: &str) -> Result<Value> {
    let root = album(raw)?;
    let first = document(&root)?;
    let expected = total(&first, &root)?;
    let count = image_count(&first)?;
    let title = first.require("h5.top-title")?.text()?;
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
        if total(d, &root)? != expected
            || image_count(d)? != count
            || d.require("h5.top-title")?.text()? != title
        {
            return Err("catalog_changed");
        }
        if expected > 1
            && !d
                .all(".my-pagination .active a")?
                .iter()
                .any(|n| n.text().ok().and_then(|s| s.parse::<usize>().ok()) == Some(part))
        {
            return Err("catalog_changed");
        }
        let images = d.all(".container-inner-fix-m > img")?;
        if images.is_empty() {
            return Err("chapter_unavailable");
        }
        // Only this album's original /p/<id>-<hash>.jpg images qualify. UI assets
        // and recommendation /poster/ thumbnails cannot enter the reading list.
        for n in images {
            let u = media(&n.attr("src")?, Some(raw)).ok_or("parse_error")?;
            if !seen.insert(u.as_str().to_string()) {
                return Err("catalog_changed");
            }
            urls.push(u);
        }
    }
    // Preserve complete provider order and reject partial or repeated pagination.
    if urls.len() != count {
        return Err("catalog_changed");
    }
    let mut out = pages(raw, urls)?;
    for p in out["pages"].as_array_mut().ok_or("internal_error")? {
        p["image"]["headers"] = json!({"Referer":root.as_str()});
    }
    Ok(out)
}
fn run_get_image_request(r: &[u8]) -> u32 {
    let result=port::input(r).and_then(|v| {
        let u=v["args"]["url"].as_str().and_then(|s|media(s,None)).ok_or("invalid_request")?;
        Ok(json!({"imageRequest":{"url":u.as_str(),"headers":{"Referer":SITE,"User-Agent":"Mozilla/5.0"}}}))
    });
    port_catalog::write_localized(METADATA, r, "get_image_request", result)
}
use photo::{
    run_get_chapters, run_get_filters, run_get_home, run_get_listings, run_get_manga,
    run_get_manga_list, run_get_pages, run_get_settings, run_search, write_chapter_failure,
};
koma_source_sdk::koma_source_exports!("meitulu", typed_chapters, write_chapter_failure);
