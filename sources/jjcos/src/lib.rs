#![no_std]
extern crate alloc;

use alloc::{
    collections::BTreeSet,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
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
mod search_index;

use photo::*;
use port::{Node, Result};
use port_catalog::Labels;

#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

const SITE: &str = "https://jjcos.com/";
const LANGUAGE: &str = "all";
const METADATA: &[u8] = include_bytes!("../metadata.json");
const SOURCE_INFO: koma_source_sdk::source::SourceInfo = info(
    "com.jjcos.koma", "JJCOS",
    "Cosplay collections and Japanese and Korean photo sets organized by performer and publication. Includes an R18 archive.",
);
const SOURCE_CAPS: SourceCapabilities = CAPS;
const LISTINGS: &[(&str, bool, bool)] = &[
    ("latest", true, true),
    ("cosplay", true, false),
    ("japan", true, false),
    ("korea", true, false),
    ("r18", true, false),
];
const CATEGORIES: &[(&str, &str)] = &[
    ("cosplay", "HSQ2151O0wZ"),
    ("japan", "_m1OhebEGKK"),
    ("korea", "HFU2FLGMsyDB"),
    ("r18", "a2_60MJyBG9S"),
];
const PAGE_BUDGET: usize = 160 * 1024;
const SEARCH_ARCHIVE_PAGES: usize = 4;

koma_source_sdk::koma_source_buffers! {
    payload: 192 * 1024, http_out: 1024 * 1024, body: 512 * 1024,
    http_req: 4096, scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

fn site_url(raw: &str) -> Result<Url> {
    let url = https(SITE, raw)?;
    if !matches!(url.host_str(), Some("jjcos.com" | "www.jjcos.com")) {
        return Err("invalid_request");
    }
    Ok(url)
}

fn album_slug(slug: &str) -> Result<&str> {
    if slug.is_empty()
        || slug.len() > 1400
        || matches!(slug, "." | "..")
        || slug
            .bytes()
            .any(|b| b.is_ascii_control() || matches!(b, b'/' | b'?' | b'#' | b'\\' | b' '))
    {
        return Err("invalid_request");
    }
    Ok(slug)
}

fn album(raw: &str) -> Result<Url> {
    let raw = album_slug(raw)?;
    let url = site_url(&format!("post/{raw}/"))?;
    if url.path().strip_prefix("/post/") != Some(format!("{raw}/").as_str()) {
        return Err("invalid_request");
    }
    Ok(url)
}

fn slug(url: &Url) -> Result<&str> {
    if url.query().is_some() {
        return Err("parse_error");
    }
    let key = url.path().strip_prefix("/post/").ok_or("parse_error")?;
    album_slug(key.trim_end_matches('/'))
}

fn tag_id(raw: &str) -> Result<&str> {
    if raw.is_empty()
        || raw.len() > 100
        || !raw
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
    {
        return Err("invalid_request");
    }
    Ok(raw)
}

fn media(raw: &str) -> Option<Url> {
    let url = https(SITE, raw).ok()?;
    let approved_path = match url.host_str() {
        Some("i1.wp.com") => url.path().contains("/wp-content/uploads/"),
        Some("jjcos.com") => {
            url.path().starts_with("/images/") && !url.path().ends_with("/avatar.png")
        }
        _ => false,
    };
    let extension = url.path().rsplit_once('.')?.1;
    (approved_path
        && ["jpg", "jpeg", "png", "webp", "avif", "gif"]
            .iter()
            .any(|kind| extension.eq_ignore_ascii_case(kind)))
    .then_some(url)
}

fn tags(node: &Node) -> Result<Vec<String>> {
    let mut seen = BTreeSet::new();
    let mut tags = Vec::new();
    for node in node.all("a.tag")? {
        let raw = node.text()?;
        let name = raw.trim_start_matches('#').trim();
        if !name.is_empty() && seen.insert(String::from(name)) {
            tags.push(String::from(name));
        }
    }
    Ok(tags)
}

fn archive(tag: &str, page: usize) -> Result<Url> {
    let base = if tag.is_empty() {
        String::new()
    } else {
        format!("tag/{}/", tag_id(tag)?)
    };
    if page == 1 {
        site_url(if base.is_empty() { SITE } else { &base })
    } else {
        site_url(&format!("{base}page/{page}/"))
    }
}

fn listing(tag: &str, page: usize, query: &str) -> Result<(Vec<Value>, bool)> {
    let doc = document(&archive(tag, page)?)?;
    let root = doc.require(".fh5co-post-entry")?;
    let mut items = Vec::new();
    let mut seen = BTreeSet::new();
    for card in root.all("article.custom-article")? {
        let title = card.require("h3 a")?;
        let link = site_url(&title.attr("href")?)?;
        let key = slug(&link)?;
        let title = title.text()?;
        let tags = tags(&card)?;
        if !query.is_empty()
            && !title.to_lowercase().contains(query)
            && !tags.iter().any(|tag| tag.to_lowercase().contains(query))
        {
            continue;
        }
        if seen.insert(String::from(key)) {
            items.push(
                Gallery {
                    key: key.into(),
                    url: album(key)?,
                    title,
                    cover: card
                        .one("figure img")
                        .and_then(|node| lazy_image(&node, &["data-src", "src"], media)),
                    tags,
                    published: None,
                }
                .manga(),
            );
        }
    }
    let next = doc
        .one("input.pagination-input[max]")
        .map(|node| {
            node.attr("max")?
                .parse::<usize>()
                .map(|max| page < max)
                .map_err(|_| "parse_error")
        })
        .transpose()?
        .unwrap_or(false);
    Ok((items, next))
}

fn browse(args: &Value, search: bool, _: &[u8]) -> Result<Value> {
    let mut page = page_number(args)?;
    let listing_id = args["listingId"].as_str().unwrap_or("latest");
    let listing_id = listing_id.strip_prefix("listing:").unwrap_or(listing_id);
    let (category, tag) = if search || listing_id == "latest" {
        (selection(args, "category")?, selection(args, "tag")?.trim())
    } else {
        (listing_id, "")
    };
    let tag = if category.is_empty() {
        tag.strip_prefix("https://jjcos.com/tag/")
            .unwrap_or(tag)
            .trim_end_matches('/')
    } else {
        if !tag.is_empty() {
            return Err("invalid_request");
        }
        CATEGORIES
            .iter()
            .find(|(key, _)| *key == category)
            .map(|(_, tag)| *tag)
            .ok_or("invalid_request")?
    };
    let query = match (&args["query"], search) {
        (_, false) | (Value::Null, true) => String::new(),
        (Value::String(q), true) if q.len() <= 512 && !q.chars().any(char::is_control) => {
            q.trim().to_lowercase()
        }
        _ => return Err("invalid_request"),
    };
    if !query.is_empty() && tag.is_empty() {
        return search_index::search(&query, page);
    }

    // The global index has no category/tag IDs. A scoped search matches
    // displayed titles and tags on the original archive pages.
    let batch = if query.is_empty() {
        1
    } else {
        SEARCH_ARCHIVE_PAGES
    };
    let mut results = Vec::new();
    let mut seen = BTreeSet::new();
    let mut next = None;
    for _ in 0..batch {
        let (items, more) = listing(tag, page, &query)?;
        for item in items {
            let id = item["id"].as_str().ok_or("parse_error")?;
            if seen.insert(String::from(id)) {
                results.push(item);
            }
        }
        if !more {
            next = None;
            break;
        }
        page += 1;
        next = Some(page);
    }
    Ok(port::items(results, next))
}

fn filters(_: &Value, _: &[u8], labels: &Labels<'_>) -> Result<Value> {
    let mut categories = vec![option("", &visible_name(labels, "filter", "any")?)];
    for (category, _) in CATEGORIES {
        categories.push(option(
            category,
            &visible_name(labels, "listing", category)?,
        ));
    }
    Ok(json!({"filters":[
        select_filter(labels, "category", categories, true)?,
        {"type":"text","id":"tag","name":visible_name(labels,"filter","tag")?,
         "description":visible_name(labels,"filter","tag_hint")?,"default":""}
    ]}))
}

fn detail(key: &str) -> Result<Gallery> {
    let url = album(key)?;
    let doc = document(&url)?;
    let title = doc.require("h1.fh5co-article-title")?.text()?;
    let content = doc.require("#post-content")?;
    let published = doc
        .one("meta[property='article:published_time']")
        .and_then(|meta| meta.attr("content").ok())
        .and_then(|raw| raw.split_whitespace().next().map(String::from))
        .filter(|day| day.len() == 10);
    Ok(Gallery {
        key: key.into(),
        url,
        title: title.strip_suffix(" - JJCOS").unwrap_or(&title).into(),
        cover: content
            .one("img")
            .and_then(|node| lazy_image(&node, &["data-src", "src"], media)),
        tags: tags(&doc)?,
        published,
    })
}

fn next_part(doc: &Node, root: &Url, current: &Url, part: usize) -> Result<Option<Url>> {
    // The post-navigation footer links separate galleries (including Set.01,
    // Set.02). Only a sequential page of the exact same post can extend it.
    let selector =
        "article a[rel='next'], #post-content .pagination a.next, article .page-links a.next";
    for link in doc.all(selector)? {
        let raw = link.attr("href")?;
        if raw.is_empty() {
            continue;
        }
        let next = https(current.as_str(), &raw)?;
        if next.host_str() != root.host_str() {
            continue;
        }
        let number = part + 1;
        let same_query_page = next.path() == root.path()
            && next
                .query()
                .is_some_and(|query| query == format!("page={number}"));
        let same_path_page = next.query().is_none()
            && (next.path() == format!("{}page/{number}/", root.path())
                || next.path() == format!("{}{number}/", root.path()));
        if same_query_page || same_path_page {
            return Ok(Some(next));
        }
        // An internal page selector must never turn a broken same-album
        // sequence into a successful but incomplete gallery.
        if next.path() == root.path() || next.path().starts_with(root.path()) {
            return Err("parse_error");
        }
    }
    Ok(None)
}

fn get_pages(key: &str) -> Result<Value> {
    let root = album(key)?;
    let mut current = root.clone();
    let mut images = Vec::new();
    let mut seen = BTreeSet::new();
    let mut budget = 0;
    for part in 1..=64 {
        let doc = document(&current)?;
        let content = doc.require("#post-content")?;
        let nodes = content.all("img")?;
        if nodes.is_empty() {
            return Err("chapter_unavailable");
        }
        for node in nodes {
            let image = lazy_image(&node, &["data-src", "src"], media).ok_or("parse_error")?;
            if seen.insert(image.as_str().to_string()) {
                // Per-page JSON repeats the album-derived ID as well as the URL.
                budget += image.as_str().len() + key.len() + 140;
                if budget > PAGE_BUDGET {
                    return Err("result_too_large");
                }
                images.push(image);
            }
        }
        match next_part(&doc, &root, &current, part)? {
            Some(next) => current = next,
            None => return pages(key, images),
        }
    }
    Err("result_too_large")
}

use photo::{
    run_get_chapters, run_get_filters, run_get_home, run_get_image_request, run_get_listings,
    run_get_manga, run_get_manga_list, run_get_pages, run_get_settings, run_search,
    write_chapter_failure,
};

koma_source_sdk::koma_source_exports!("jjcos", typed_chapters, write_chapter_failure);
