#![no_std]
extern crate alloc;

use alloc::{
    collections::{BTreeMap, BTreeSet},
    format,
    string::String,
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

use photo::*;
use port::{Node, Result};
use port_catalog::Labels;

include!("tags.rs");

#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

const SITE: &str = "https://misskon.com/";
const LANGUAGE: &str = "all";
const METADATA: &[u8] = include_bytes!("../metadata.json");
const SOURCE_INFO: koma_source_sdk::source::SourceInfo = info(
    "com.misskon.koma",
    "MissKon",
    "Asian photo galleries with cosplay and studio sets, official view rankings and photo tags.",
);
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    image_request: true,
    ..CAPS
};
const LISTINGS: &[(&str, bool, bool)] = &[
    ("popular", false, false),
    ("latest", true, true),
    ("cosplay", true, false),
    ("jp", true, false),
];
const CATEGORIES: &[(&str, &str)] = &[
    ("top3", "top3/"),
    ("top7", "top7/"),
    ("top30", "top30/"),
    ("top60", "top60/"),
    ("top_year", "top-year/"),
    ("cosplay", "tag/cosplay/"),
    ("jp", "tag/jp/"),
    ("ai_generated", "tag/ai-generated/"),
    ("ai_enhanced", "tag/ai-enhanced/"),
];

koma_source_sdk::koma_source_buffers! {
    payload: 256 * 1024, http_out: 1024 * 1024, body: 512 * 1024,
    http_req: 4096, scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

fn own_url(raw: &str) -> Result<Url> {
    let url = https(SITE, raw)?;
    (url.host_str() == Some("misskon.com"))
        .then_some(url)
        .ok_or("invalid_request")
}

fn album(raw: &str) -> Result<Url> {
    let mut url = own_url(raw)?;
    let path = url.path().trim_matches('/');
    if path.is_empty()
        || path.len() > 768
        || path.contains('/')
        || matches!(
            path,
            "categories" | "top3" | "top7" | "top30" | "top60" | "top-year"
        )
    {
        return Err("invalid_request");
    }
    url.set_path(&format!("/{path}/"));
    url.set_query(None);
    Ok(url)
}

fn media(raw: &str) -> Option<Url> {
    let url = https(SITE, raw).ok()?;
    let path = url.path();
    let allowed = match url.host_str()? {
        "misskon.com" => path.starts_with("/media/") || path.starts_with("/wp-content/uploads/"),
        "kul.misskon.com" => path.starts_with("/uploads/"),
        _ => false,
    };
    let extension = path.rsplit_once('.')?.1;
    (allowed
        && ["webp", "jpg", "jpeg", "png", "gif", "avif"]
            .iter()
            .any(|item| extension.eq_ignore_ascii_case(item)))
    .then_some(url)
}

fn tags(node: &Node, selector: &str) -> Result<Vec<String>> {
    node.all(selector)?.iter().map(Node::text).collect()
}

fn gallery(node: &Node) -> Result<Gallery> {
    let title = node.require(".post-box-title a")?;
    let url = album(&title.attr("href")?)?;
    let cover = node
        .one(".post-thumbnail img")
        .and_then(|image| lazy_image(&image, &["data-src", "src"], media));
    Ok(Gallery {
        key: url.path().into(),
        url,
        title: title.text()?,
        cover,
        tags: tags(node, ".post-cats a[rel=tag]")?,
        published: None,
    })
}

fn with_cover_headers(mut manga: Value) -> Value {
    if manga["cover"]["kind"] == "url" {
        manga["cover"]["headers"] = json!({"Referer":SITE});
    }
    manga
}

fn listing(url: &Url, page: usize, ranking: bool) -> Result<Value> {
    let document = document(url)?;
    let mut seen = BTreeSet::new();
    let mut items = Vec::new();
    for node in document.all("article.item-list")? {
        let album = gallery(&node)?;
        if seen.insert(album.key.clone()) {
            items.push(with_cover_headers(album.manga()));
        }
    }
    if items.is_empty() && document.one(".content .not-found").is_none() {
        return Err("parse_error");
    }
    let has_more = !ranking
        && !items.is_empty()
        && document
            .one(".pagination .current + a")
            .and_then(|node| node.text().ok())
            .and_then(|number| number.parse::<usize>().ok())
            == Some(page + 1);
    Ok(port::items(items, has_more.then_some(page + 1)))
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
    let (root, ranking) = match (search, args["listingId"].as_str()) {
        (false, Some("popular")) => ("top3/", true),
        (false, Some("cosplay")) => ("tag/cosplay/", false),
        (false, Some("jp")) => ("tag/jp/", false),
        (false, Some("latest") | None) | (true, _) => {
            let category = selection(args, "category")?;
            let tag = selection(args, "tag")?;
            if !category.is_empty() && !tag.is_empty() {
                return Err("invalid_request");
            }
            if !category.is_empty() {
                let &(_, path) = CATEGORIES
                    .iter()
                    .find(|(id, _)| *id == category)
                    .ok_or("invalid_request")?;
                (path, path.starts_with("top"))
            } else if !tag.is_empty() {
                if !TAGS.iter().any(|(id, _)| *id == tag) {
                    return Err("invalid_request");
                }
                if !query.is_empty() {
                    return Err("invalid_request");
                }
                return listing(&own_url(&paged(&format!("tag/{tag}/"), page))?, page, false);
            } else {
                ("", false)
            }
        }
        _ => return Err("invalid_request"),
    };
    // The site searches its entire catalog. A tag or ranked category has
    // its own archive and cannot be combined with a text query.
    if !root.is_empty() && !query.is_empty() {
        return Err("invalid_request");
    }
    if ranking && page != 1 {
        return Ok(port::items(Vec::new(), None));
    }
    let mut url = own_url(&paged(root, page))?;
    if !query.is_empty() {
        url.query_pairs_mut().append_pair("s", query);
    }
    listing(&url, page, ranking)
}

fn paged(root: &str, page: usize) -> String {
    if page == 1 {
        return format!("/{root}");
    }
    format!("/{root}page/{page}/")
}

fn filters(_: &Value, _: &[u8], labels: &Labels<'_>) -> Result<Value> {
    let any = visible_name(labels, "filter", "any")?;
    let mut categories = vec![option("", &any)];
    for (id, _) in CATEGORIES {
        categories.push(option(id, &visible_name(labels, "category", id)?));
    }
    let mut tags = vec![option("", &any)];
    tags.extend(TAGS.iter().map(|(id, brand)| option(id, brand)));
    Ok(json!({"filters":[
        select_filter(labels, "category", categories, true)?,
        select_filter(labels, "tag", tags, true)?
    ]}))
}

fn pictures(document: &Node) -> Result<Vec<Url>> {
    let mut images = Vec::new();
    for node in document
        .all("article > .post-inner > .entry p > img, article > .post-inner > .entry p > a > img")?
    {
        if let Some(url) = lazy_image(&node, &["data-src", "src"], media) {
            // The same entry also contains a Download button served from /media/.
            if !matches!(url.path().rsplit('/').next(), Some("dlp1.gif")) {
                images.push(url);
            }
        }
    }
    Ok(images)
}

fn detail(key: &str) -> Result<Gallery> {
    let url = album(key)?;
    let doc = document(&url)?;
    let title = doc.require("article > .post-inner .post-title")?.text()?;
    let cover = pictures(&doc)?.into_iter().next();
    let published = cover.as_ref().and_then(|image| {
        let mut path = image.path_segments()?;
        if path.next()? != "uploads" {
            return None;
        }
        let y = path.next()?;
        let m = path.next()?;
        let d = path.next()?;
        if [y, m, d]
            .iter()
            .zip([4, 2, 2])
            .all(|(part, size)| part.len() == size && part.bytes().all(|b| b.is_ascii_digit()))
        {
            Some(format!("{y}-{m}-{d}T00:00:00Z"))
        } else {
            None
        }
    });
    Ok(Gallery {
        key: url.path().into(),
        title,
        cover,
        tags: tags(&doc, "article > .post-inner .post-tag a[rel=tag]")?,
        url,
        published,
    })
}

fn discover_parts(document: &Node, album: &Url, parts: &mut BTreeMap<usize, Url>) -> Result<()> {
    let base = album.path().trim_end_matches('/');
    for link in document.all("article .page-link a")? {
        let Ok(number) = link.text()?.parse::<usize>() else {
            continue;
        };
        let target = album.join(&link.attr("href")?).map_err(|_| "parse_error")?;
        let url = own_url(target.as_str())?;
        let path = url.path().trim_end_matches('/');
        let suffix = path.strip_prefix(base).ok_or("parse_error")?;
        let same_album = if number == 1 {
            suffix.is_empty()
        } else {
            suffix == format!("/{number}") || suffix == format!("-page{number}")
        };
        if number == 0 || number > 128 || !same_album || url.query().is_some() {
            return Err("parse_error");
        }
        parts.entry(number).or_insert(url);
    }
    Ok(())
}

fn get_pages(key: &str) -> Result<Value> {
    let album = album(key)?;
    let mut parts = BTreeMap::from([(1, album.clone())]);
    let mut images = Vec::new();
    let mut expected = 1;
    // Each public HTML subpage must load before the chapter succeeds, so a
    // failed follow-up cannot leave a falsely complete, cacheable gallery.
    while let Some(url) = parts.remove(&expected) {
        let doc = document(&url)?;
        if let Some(current) = doc.one("article .page-link .current") {
            if current.text()?.parse::<usize>().ok() != Some(expected) {
                return Err("parse_error");
            }
        }
        let part = pictures(&doc)?;
        if part.is_empty() {
            return Err("chapter_unavailable");
        }
        images.extend(part);
        discover_parts(&doc, &album, &mut parts)?;
        parts.retain(|number, _| *number > expected);
        expected += 1;
    }
    if !parts.is_empty() {
        return Err("parse_error");
    }
    pages(album.path(), unique(images))
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
        let url = input["args"]["url"].as_str().ok_or("invalid_request")?;
        let url = media(url).ok_or("invalid_request")?;
        Ok(json!({"url":url.as_str(),"headers":{"Referer":SITE}}))
    });
    port_catalog::write_localized(METADATA, request, "get_image_request", result)
}

// The public gallery follows official post pages only. The site may offer
// additional files behind third-party download links outside this contract.
koma_source_sdk::koma_source_exports!("misskon", typed_chapters, write_chapter_failure);
