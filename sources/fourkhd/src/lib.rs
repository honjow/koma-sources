#![no_std]
extern crate alloc;

use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use koma_source_sdk::source::{SourceCapabilities, SourceInfo};
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
#[path = "../../wp_gallery.rs"]
#[allow(dead_code)]
mod wp_gallery;

use photo::*;
use port::{Node, Result};
use port_catalog::Labels;

#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

const SITE: &str = "https://www.4khd.com";
const LANGUAGE: &str = "all";
const METADATA: &[u8] = include_bytes!("../metadata.json");
const SOURCE_INFO: SourceInfo = info("com.fourkhd.koma", "4KHD",
    "An Asian photo gallery archive with Korean magazine and studio sets, Chinese photography and cosplay, using the site's full-size gallery images.");
const SOURCE_CAPS: SourceCapabilities = CAPS;
const LISTINGS: &[(&str, bool, bool)] = &[
    ("latest", true, true),
    ("modified", true, true),
    ("photo", true, false),
    ("cosplay", true, false),
];
const CATEGORIES: &[(&str, &str)] = &[
    ("3", "photo"),
    ("4", "cosplay"),
    ("9", "miscellaneous"),
    ("21", "popular"),
    ("26", "bare"),
];
const LIST_FIELDS: &str = "id,link,title,jetpack_featured_media_url,_links,_embedded";

koma_source_sdk::koma_source_buffers! {
    payload: 768 * 1024, http_out: 4 * 1024 * 1024, body: 2 * 1024 * 1024,
    http_req: 4096, scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

fn endpoint(route: &str) -> Result<Url> {
    let mut url = https(SITE, "/index.php")?;
    url.query_pairs_mut().append_pair("rest_route", route);
    Ok(url)
}

fn article(raw: &str) -> Result<(Url, String)> {
    let mut url = https(SITE, raw)?;
    if !matches!(url.host_str(), Some("4khd.com" | "www.4khd.com")) {
        return Err("invalid_request");
    }
    url.set_host(Some("www.4khd.com"))
        .map_err(|_| "invalid_request")?;
    let segments: Vec<_> = url.path().trim_matches('/').split('/').collect();
    let slug = match segments.as_slice() {
        ["content", _, file] => *file,
        ["content", _, file, page]
            if !page.is_empty() && page.bytes().all(|b| b.is_ascii_digit()) =>
        {
            *file
        }
        _ => return Err("invalid_request"),
    };
    let slug = slug
        .strip_suffix(".html")
        .filter(|s| !s.is_empty())
        .ok_or("invalid_request")?
        .to_string();
    let path = format!("/content/{}/{}.html", segments[1], &slug);
    url.set_path(&path);
    url.set_query(None);
    Ok((url, slug))
}

fn image(raw: &str) -> Option<Url> {
    wp_gallery::image(SITE, raw, |url| match url.host_str().unwrap_or_default() {
        "www.4khd.com" | "4khd.com" => url.path().starts_with("/wp-content/uploads/"),
        "pic.4khd.com" => true,
        "i0.wp.com" | "i1.wp.com" | "i2.wp.com" => url
            .path()
            .split('/')
            .nth(1)
            .is_some_and(|host| matches!(host, "pic.4khd.com" | "4khd.com" | "www.4khd.com")),
        _ => false,
    })
}

fn album_images(post: &wp_gallery::Post<'_>) -> Result<Vec<Url>> {
    let doc = Node::parse(post.content()?)?;
    let root = doc.one(".entry-content");
    // Full-size gallery anchors preserve the site's order and sizing. The
    // render-only width query on inline copies is unnecessary for these URLs.
    wp_gallery::images(
        root.as_ref().unwrap_or(&doc),
        true,
        &["data-src", "data-lazy-src", "src"],
        |raw| {
            let mut url = image(raw)?;
            let pairs = url
                .query_pairs()
                .into_owned()
                .filter(|(k, v)| {
                    !(k == "w" && !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()))
                })
                .collect::<Vec<_>>();
            if url.query_pairs().count() != pairs.len() {
                url.set_query(None);
                if !pairs.is_empty() {
                    url.query_pairs_mut().extend_pairs(pairs);
                }
            }
            Some(url)
        },
    )
}

fn card(post: wp_gallery::Post<'_>, full: bool) -> Result<Gallery> {
    let id = post.0["id"]
        .as_u64()
        .filter(|n| *n != 0)
        .ok_or("parse_error")?;
    let (url, _) = article(post.link()?)?;
    let mut cover = post.cover().and_then(image);
    if full && cover.is_none() {
        cover = album_images(&post)?.into_iter().next();
    }
    Ok(Gallery {
        key: id.to_string(),
        url,
        title: post.title()?,
        cover,
        tags: if full { post.tags() } else { Vec::new() },
        published: post.date(),
    })
}

fn fetch_post(key: &str) -> Result<Value> {
    let id = wp_gallery::positive_id(key)?;
    let mut url = endpoint(&format!("/wp/v2/posts/{id}"))?;
    url.query_pairs_mut().append_pair("_embed", "1");
    let data = wp_gallery::rest(&url)?;
    if data["id"].as_u64() != Some(u64::from(id)) {
        return Err("parse_error");
    }
    Ok(data)
}

fn detail(key: &str) -> Result<Gallery> {
    card(wp_gallery::Post(&fetch_post(key)?), true)
}

fn get_pages(key: &str) -> Result<Value> {
    pages(key, album_images(&wp_gallery::Post(&fetch_post(key)?))?)
}

fn browse(args: &Value, search: bool, request: &[u8]) -> Result<Value> {
    let page = page_number(args)?;
    let query = if search {
        args["query"].as_str().unwrap_or("").trim()
    } else {
        ""
    };
    let listing = if search {
        "latest"
    } else {
        let id = args["listingId"].as_str().unwrap_or("latest");
        id.strip_prefix("listing:").unwrap_or(id)
    };
    let category = selection(args, "category")?;
    if !category.is_empty() && !CATEGORIES.iter().any(|(id, _)| *id == category) {
        return Err("invalid_request");
    }
    let (orderby, category) = match listing {
        "latest" => ("date", category),
        "modified" | "popular" => ("modified", category),
        "photo" => ("date", "3"),
        "cosplay" => ("date", "4"),
        _ => return Err("invalid_request"),
    };
    if let Ok(url) = Url::parse(query) {
        let Ok((_, slug)) = article(url.as_str()) else {
            return Ok(port::items(Vec::new(), None));
        };
        if page != 1 {
            return Ok(port::items(Vec::new(), None));
        }
        let mut url = endpoint("/wp/v2/posts")?;
        url.query_pairs_mut()
            .append_pair("slug", &slug)
            .append_pair("_embed", "1");
        let data = wp_gallery::rest(&url)?;
        let posts = data.as_array().ok_or("parse_error")?;
        let items = posts
            .iter()
            .map(|p| card(wp_gallery::Post(p), true).map(|p| p.manga()))
            .collect::<Result<Vec<_>>>()?;
        return Ok(port::items(items, None));
    }
    wp_gallery::list(
        endpoint("/wp/v2/posts")?,
        page,
        LIST_FIELDS,
        &[
            ("orderby", orderby),
            ("categories", category),
            ("search", query),
        ],
        request,
        |post| card(post, false),
    )
}

fn filters(_: &Value, _: &[u8], labels: &Labels<'_>) -> Result<Value> {
    let mut categories = vec![option("", &visible_name(labels, "filter", "any")?)];
    for (value, id) in CATEGORIES {
        categories.push(option(value, &visible_name(labels, "listing", id)?));
    }
    Ok(json!({"filters":[select_filter(labels,"category",categories,true)?]}))
}

koma_source_sdk::koma_source_exports!("fourkhd", typed_chapters, write_chapter_failure);
