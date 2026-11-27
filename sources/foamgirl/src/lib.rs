#![no_std]
extern crate alloc;

use alloc::{collections::BTreeSet, format, string::String, vec, vec::Vec};
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

const SITE: &str = "https://foamgirl.net/";
const LANGUAGE: &str = "all";
const METADATA: &[u8] = include_bytes!("../metadata.json");
const SOURCE_INFO: koma_source_sdk::source::SourceInfo = info(
    "com.foamgirl.koma",
    "FoamGirl",
    "Asian photo sets and cosplay galleries, including Japanese gravure and Chinese studio shoots.",
);
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    image_request: true,
    ..CAPS
};
const LISTINGS: &[(&str, bool, bool)] = &[
    ("latest", true, true),
    ("chinese", true, false),
    ("korea", true, false),
    ("japan", true, false),
    ("thailand", true, false),
    ("cosplay", true, false),
];
const CATEGORIES: &[&str] = &["chinese", "korea", "japan", "thailand", "cosplay"];
koma_source_sdk::koma_source_buffers! {
    payload: 192 * 1024, http_out: 1024 * 1024, body: 512 * 1024,
    http_req: 4096, scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

fn album_id(raw: &str) -> Result<&str> {
    if raw.is_empty()
        || raw.len() > 12
        || raw.starts_with('0')
        || !raw.bytes().all(|b| b.is_ascii_digit())
    {
        return Err("invalid_request");
    }
    Ok(raw)
}

fn site_url(raw: &str) -> Result<Url> {
    let url = https(SITE, raw)?;
    if url.host_str() != Some("foamgirl.net") {
        return Err("invalid_request");
    }
    Ok(url)
}

fn album_url(id: &str, part: usize) -> Result<Url> {
    let id = album_id(id)?;
    let path = if part == 1 {
        format!("{id}.html")
    } else {
        format!("{id}_{part}.html")
    };
    site_url(&path)
}

fn media(raw: &str) -> Option<Url> {
    let url = https(SITE, raw).ok()?;
    let path = url.path();
    let extension = path.rsplit_once('.')?.1.split('!').next()?;
    (matches!(url.host_str(), Some("cdn.foamgirl.net" | "foamgirl.net"))
        && path.starts_with("/wp-content/uploads/")
        && ["jpg", "jpeg", "png", "webp", "avif", "gif"]
            .iter()
            .any(|ext| extension.eq_ignore_ascii_case(ext))
        && !path.ends_with("/1653737308-6b1ffe02fd54085.gif"))
    .then_some(url)
}

fn with_cover_headers(mut manga: Value) -> Value {
    if manga["cover"]["kind"] == "url" {
        manga["cover"]["headers"] = json!({"Referer":SITE});
    }
    manga
}

fn listing_url(archive: &str, query: &str, page: usize) -> Result<Url> {
    let mut url = site_url(if archive.is_empty() { SITE } else { archive })?;
    if page > 1 || !query.is_empty() {
        url.set_path(&format!("{}/page/{page}", url.path().trim_end_matches('/')));
    }
    if !query.is_empty() {
        url.query_pairs_mut()
            .append_pair("post_type", "post")
            .append_pair("s", query);
    }
    Ok(url)
}

fn browse(args: &Value, search: bool, _: &[u8]) -> Result<Value> {
    let page = page_number(args)?;
    let query = if search {
        match &args["query"] {
            Value::Null => "",
            Value::String(value) if value.len() <= 512 => value.trim(),
            _ => return Err("invalid_request"),
        }
    } else {
        ""
    };
    let listing = args["listingId"].as_str().unwrap_or("latest");
    let (category, tag) = if search || listing == "latest" {
        (selection(args, "category")?, selection(args, "tag")?)
    } else {
        (listing, "")
    };
    // WordPress category and tag archives have their own URLs. The site's
    // keyword search is global and cannot be scoped to either archive.
    if (!tag.is_empty() && !category.is_empty())
        || (!query.is_empty() && (!category.is_empty() || !tag.is_empty()))
        || (!category.is_empty() && !CATEGORIES.contains(&category))
    {
        return Err("invalid_request");
    }
    let tag_path = if tag.is_empty() {
        String::new()
    } else {
        let raw = tag.trim().replace(' ', "-");
        if raw.bytes().any(|b| matches!(b, b'&' | b'=' | b'+')) {
            return Err("invalid_request");
        }
        let (decoded, _) = url::form_urlencoded::parse(raw.as_bytes())
            .next()
            .ok_or("invalid_request")?;
        let slug = decoded.to_ascii_lowercase();
        if slug.is_empty()
            || slug.len() > 300
            || !slug
                .chars()
                .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_'))
        {
            return Err("invalid_request");
        }
        format!("tag/{slug}")
    };
    let archive = if tag.is_empty() { category } else { &tag_path };
    let url = listing_url(archive, query, page)?;
    let doc = document(&url)?;
    let area = doc.require(".update_area")?;
    let mut seen = BTreeSet::new();
    let mut items = Vec::new();
    for card in area.all(".i_list")? {
        let title = card.require("a.meta-title")?;
        let link = site_url(&title.attr("href")?)?;
        let key = album_id(
            link.path()
                .trim_start_matches('/')
                .strip_suffix(".html")
                .ok_or("parse_error")?,
        )?;
        if seen.insert(String::from(key)) {
            let gallery = Gallery {
                key: key.into(),
                url: album_url(key, 1)?,
                title: title.text()?,
                cover: card
                    .one("a.thumb-srcbox img")
                    .and_then(|img| lazy_image(&img, &["data-original", "src"], media)),
                tags: Vec::new(),
                published: None,
            };
            items.push(with_cover_headers(gallery.manga()));
        }
    }
    let more = area.one("a.next").or_else(|| doc.one("a.next"));
    let next = if let Some(node) = more.filter(|_| !items.is_empty()) {
        let next = site_url(&node.attr("href")?)?;
        let expected = listing_url(archive, query, page + 1)?;
        if next.path().trim_end_matches('/') != expected.path().trim_end_matches('/') {
            return Err("parse_error");
        }
        Some(page + 1)
    } else {
        None
    };
    Ok(port::items(items, next))
}

fn filters(_: &Value, _: &[u8], labels: &Labels<'_>) -> Result<Value> {
    let any = visible_name(labels, "filter", "any")?;
    let labels_by_category = CATEGORIES
        .iter()
        .map(|k| Ok(option(k, &visible_name(labels, "listing", k)?)));
    let mut categories = vec![option("", &any)];
    categories.extend(labels_by_category.collect::<Result<Vec<_>>>()?);
    Ok(json!({"filters":[
        select_filter(labels, "category", categories, true)?,
        {"type":"text", "id":"tag", "name":visible_name(labels,"filter","tag")?,
         "description":visible_name(labels,"filter","tag_hint")?, "default":""}
    ]}))
}

fn album_date(text: &str) -> Option<String> {
    let mut parts = text.split_whitespace().next_back()?.split('.');
    let year: u16 = parts.next()?.parse().ok()?;
    let month: u8 = parts.next()?.parse().ok()?;
    let day: u8 = parts.next()?.parse().ok()?;
    (parts.next().is_none() && (1..=12).contains(&month) && (1..=31).contains(&day))
        .then(|| format!("{year:04}-{month:02}-{day:02}T00:00:00Z"))
}

fn detail(key: &str) -> Result<Gallery> {
    let url = album_url(key, 1)?;
    let doc = document(&url)?;
    let title = doc.require(".s-article .item_title h1")?.text()?;
    let cover = doc
        .one(".imageclick-imgbox")
        .and_then(|node| lazy_image(&node, &["href"], media));
    let tags = doc
        .all(".single-tags a")?
        .iter()
        .map(Node::text)
        .collect::<Result<Vec<_>>>()?;
    let published = doc
        .one(".image-info-time")
        .and_then(|node| album_date(&node.text().ok()?));
    Ok(Gallery {
        key: key.into(),
        url,
        title,
        cover,
        tags,
        published,
    })
}

fn get_pages(key: &str) -> Result<Value> {
    let mut images = Vec::new();
    let mut image_budget = 0;
    for part in 1..=128 {
        let doc = document(&album_url(key, part)?)?;
        let anchors = doc.all(".image_div .imageclick-imgbox")?;
        if anchors.is_empty() {
            return Err("chapter_unavailable");
        }
        for node in anchors {
            let image = media(&node.attr("href")?).ok_or("parse_error")?;
            image_budget += image.as_str().len() + 130;
            if image_budget > 192 * 1024 {
                return Err("result_too_large");
            }
            images.push(image);
        }
        // A followed link must be the next part of this exact album. Do not
        // publish a partial gallery if a later official subpage fails.
        let Some(next) = doc.one(".page_imges a.page-numbers[title='Next page']") else {
            return pages(key, unique(images));
        };
        let next = site_url(&next.attr("href")?)?;
        let next_post = next.path().trim_start_matches('/').strip_suffix(".html");
        if next.query().is_none() && next_post.is_some_and(|id| album_id(id).is_ok() && id != key) {
            // At the last subpage the same control links to the next album.
            return pages(key, unique(images));
        }
        if next != album_url(key, part + 1)? {
            return Err("parse_error");
        }
    }
    Err("result_too_large")
}

use photo::{
    run_get_chapters, run_get_filters, run_get_home, run_get_listings, run_get_manga_list,
    run_get_pages, run_get_settings, run_search, write_chapter_failure,
};

fn run_get_manga(request: &[u8]) -> u32 {
    let result = port::input(request).and_then(|input| {
        let gallery = detail(work(&input["args"])?)?;
        Ok(json!({"manga":with_cover_headers(gallery.manga())}))
    });
    port_catalog::write_localized(METADATA, request, "get_manga", result)
}

fn run_get_image_request(request: &[u8]) -> u32 {
    let result = port::input(request).and_then(|input| {
        let url = input["args"]["url"]
            .as_str()
            .and_then(media)
            .ok_or("invalid_request")?;
        Ok(json!({"url":url.as_str(),"headers":{"Referer":SITE}}))
    });
    port_catalog::write_localized(METADATA, request, "get_image_request", result)
}

koma_source_sdk::koma_source_exports!("foamgirl", typed_chapters, write_chapter_failure);
