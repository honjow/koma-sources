#![no_std]
extern crate alloc;

use alloc::{
    collections::{BTreeMap, BTreeSet},
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use koma_source_sdk::{
    source::{SourceCapabilities, SourceInfo},
    source_protocol,
};
use serde_json::{json, Value};
use url::Url;

#[path = "../../photo_gallery.rs"]
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

const SITE: &str = "https://everia.club";
const LANGUAGE: &str = "all";
const METADATA: &[u8] = include_bytes!("../metadata.json");
const SOURCE_INFO: SourceInfo = info("com.everiaclub.koma", "Everia.club",
    "An Asian photobook archive with Japanese gravure magazines, Korean and Thai photo sets, Chinese photography and cosplay.");
const SOURCE_CAPS: SourceCapabilities = CAPS;

const LISTINGS: &[(&str, bool, bool)] = &[
    ("latest", true, true),
    ("popular", false, false),
    ("japan", true, false),
    ("korea", true, false),
    ("thailand", true, false),
    ("china", true, false),
    ("cosplay", true, false),
];
const CATEGORIES: &[(&str, &str)] = &[
    ("2", "japan"),
    ("11", "korea"),
    ("1984", "thailand"),
    ("42", "china"),
    ("7", "cosplay"),
    ("1", "uncategorized"),
];
// Stable options for hosts that do not support lazy filter loading.
const TAGS: &[(&str, &str)] = &[
    ("38", "FRIDAY フライデー"),
    ("121", "FLASH フラッシュ"),
    ("173", "Weekly Playboy 週刊プレイボーイ"),
    ("244", "Nogizaka46 乃木坂46"),
    ("139", "Young Jump ヤングジャンプ"),
    ("41", "XiuRen秀人网"),
];
const FIELDS: &str = "id,link,title,_links,_embedded";
const MAX_SUBPAGES: usize = 128;

koma_source_sdk::koma_source_buffers! {
    payload: 768 * 1024, http_out: 4 * 1024 * 1024, body: 2 * 1024 * 1024,
    http_req: 4096, scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

fn own_url(raw: &str) -> Result<Url> {
    let url = https(SITE, raw)?;
    (url.host_str() == Some("everia.club"))
        .then_some(url)
        .ok_or("invalid_request")
}

fn article(raw: &str) -> Result<Url> {
    let mut url = own_url(raw)?;
    let path = url.path().trim_end_matches('/');
    let segments: Vec<_> = path.trim_start_matches('/').split('/').collect();
    if segments.len() != 4
        || segments[3].is_empty()
        || !segments[..3]
            .iter()
            .zip([4, 2, 2])
            .all(|(s, n)| s.len() == n && s.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err("invalid_request");
    }
    // The hexadecimal spelling of a URL escape must not change work identity.
    let mut canonical = format!("{path}/").into_bytes();
    let mut i = 0;
    while i + 2 < canonical.len() {
        if canonical[i] == b'%' && canonical[i + 1..i + 3].iter().all(u8::is_ascii_hexdigit) {
            canonical[i + 1..i + 3].make_ascii_lowercase();
            i += 3;
        } else {
            i += 1;
        }
    }
    url.set_path(core::str::from_utf8(&canonical).map_err(|_| "invalid_request")?);
    url.set_query(None);
    Ok(url)
}

fn image(raw: &str) -> Option<Url> {
    let media_host = |name: &str| {
        matches!(
            name,
            "everia.club"
                | "karubox.top"
                | "reiobox.top"
                | "airibox.top"
                | "remu.airibox.top"
                | "isukiyaki.airibox.top"
                | "miya.fujibox.top"
                | "yuhabox.top"
        )
    };
    wp_gallery::image(SITE, raw, |url| {
        let host = url.host_str().unwrap_or_default();
        if matches!(host, "i0.wp.com" | "i1.wp.com" | "i2.wp.com") {
            url.path()
                .strip_prefix('/')
                .and_then(|path| path.split_once('/'))
                .is_some_and(|(host, path)| {
                    media_host(host) && path.starts_with("wp-content/uploads/")
                })
        } else {
            media_host(host) && url.path().starts_with("/wp-content/uploads/")
        }
    })
}

fn card(url: Url, title: String, cover: Option<Url>) -> Gallery {
    let published = url
        .path()
        .get(1..11)
        .map(|date| format!("{}T00:00:00Z", date.replace('/', "-")));
    Gallery {
        key: url.path().into(),
        url,
        title,
        cover,
        tags: Vec::new(),
        published,
    }
}

fn from_post(post: wp_gallery::Post<'_>) -> Result<Gallery> {
    Ok(card(
        article(post.link()?)?,
        post.title()?,
        post.cover().and_then(image),
    ))
}

fn from_node(node: &Node, selector: &str) -> Result<Gallery> {
    let title = node.require(selector)?;
    let url = article(&title.attr("href")?)?;
    let cover = node
        .one("img")
        .and_then(|node| lazy_image(&node, wp_gallery::LAZY_FIRST, image));
    Ok(card(url, title.text()?, cover))
}

fn html_listing(url: &Url, page: usize, popular: bool) -> Result<Value> {
    if popular && page != 1 {
        return Ok(port::items(Vec::new(), None));
    }
    let doc = document(url)?;
    let (rows, title) = if popular {
        (".wli_popular_posts-class li", "h3 > a")
    } else {
        (
            "#blog-entries > article, #content > article",
            ".entry-title > a",
        )
    };
    let cards = doc
        .all(rows)?
        .iter()
        .filter_map(|node| from_node(node, title).ok())
        .map(|gallery| gallery.manga())
        .collect();
    Ok(port::items(
        cards,
        (!popular && doc.one(".next").is_some()).then_some(page + 1),
    ))
}

fn term_filters(args: &Value) -> Result<(String, String)> {
    let mut include = BTreeSet::new();
    let mut exclude = BTreeSet::new();
    match &args["filters"]["tags"] {
        Value::Null => {
            for (key, set) in [("tag", &mut include), ("exclude_tag", &mut exclude)] {
                let id = selection(args, key)?;
                if !id.is_empty() {
                    set.insert(wp_gallery::positive_id(id)?);
                }
            }
        }
        Value::Array(tags) if tags.len() <= 100 => {
            for tag in tags {
                let tag = tag.as_str().ok_or("invalid_request")?;
                if let Some(id) = tag.strip_prefix('!') {
                    exclude.insert(wp_gallery::positive_id(id)?);
                } else {
                    include.insert(wp_gallery::positive_id(tag)?);
                }
            }
        }
        _ => return Err("invalid_request"),
    }
    if include.iter().any(|id| exclude.contains(id)) {
        return Err("invalid_request");
    }
    let joined = |set: BTreeSet<u32>| set.iter().map(u32::to_string).collect::<Vec<_>>().join(",");
    Ok((joined(include), joined(exclude)))
}

fn search_terms(raw: &str) -> String {
    // WordPress treats long multiword input as a single exact sentence.
    // Bound it consistently for every page and never cut a UTF-8 code point.
    let query = raw.split_whitespace().take(9).collect::<Vec<_>>().join(" ");
    if query.chars().count() <= 120 {
        return query;
    }
    let prefix: String = query.chars().take(120).collect();
    match prefix.rsplit_once(' ') {
        Some((whole, _)) => whole.into(),
        None => prefix,
    }
}

fn browse(args: &Value, search: bool, request: &[u8]) -> Result<Value> {
    let page = page_number(args)?;
    let raw_query = if search {
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
    if !category.is_empty() {
        wp_gallery::positive_id(category)?;
    }
    let (tags, excluded) = term_filters(args)?;
    if let Ok(url) = Url::parse(raw_query) {
        if let Ok(article) = article(url.as_str()) {
            return if page == 1 {
                Ok(port::items(vec![detail(article.path())?.manga()], None))
            } else {
                Ok(port::items(Vec::new(), None))
            };
        }
        if let Ok(url) = own_url(url.as_str()) {
            let parts: Vec<_> = url.path().trim_matches('/').split('/').collect();
            if matches!(parts.as_slice(), ["category" | "tag", slug, ..] if !slug.is_empty()) {
                let path = format!("/{}/{}/", parts[0], parts[1]);
                let url = if page == 1 {
                    own_url(&path)?
                } else {
                    own_url(&format!("{path}page/{page}/"))?
                };
                return html_listing(&url, page, false);
            }
        }
        return Ok(port::items(Vec::new(), None));
    }

    if listing == "popular" && category.is_empty() && tags.is_empty() && excluded.is_empty() {
        return html_listing(&own_url("/")?, page, true);
    }
    let fixed = CATEGORIES
        .iter()
        .find(|(_, id)| *id == listing)
        .map(|(id, _)| *id);
    if fixed.is_none() && !matches!(listing, "latest" | "popular") {
        return Err("invalid_request");
    }
    let category = fixed.unwrap_or(category);
    let query = search_terms(raw_query);
    // Full-content WordPress searches can exceed the normal HTTP deadline on
    // this archive. Album, person and issue names are indexed in post titles.
    let search_columns = if query.is_empty() { "" } else { "post_title" };
    wp_gallery::list(
        own_url("/wp-json/wp/v2/posts")?,
        page,
        FIELDS,
        &[
            ("search", &query),
            ("categories", category),
            ("tags", &tags),
            ("tags_exclude", &excluded),
            ("search_columns", search_columns),
        ],
        request,
        from_post,
    )
}

fn detail(key: &str) -> Result<Gallery> {
    let url = article(key)?;
    let doc = document(&url)?;
    let title = doc.require(".entry-title")?.text()?;
    let cover = doc
        .one("meta[property='og:image']")
        .and_then(|n| n.attr("content").ok())
        .and_then(|s| image(&s))
        .or_else(|| {
            doc.one(".entry-content img")
                .and_then(|n| lazy_image(&n, wp_gallery::LAZY_FIRST, image))
        });
    let mut gallery = card(url, title, cover);
    gallery.tags = doc
        .all(".post-tags > a")?
        .iter()
        .filter_map(|node| node.text().ok())
        .collect();
    Ok(gallery)
}

fn collect_album_links(
    doc: &Node,
    url: &Url,
    after: usize,
    pending: &mut BTreeMap<usize, Url>,
) -> Result<()> {
    for node in doc.all(".page-links a.post-page-numbers")? {
        let Ok(target) = own_url(&node.attr("href")?) else {
            continue;
        };
        let Some(suffix) = target.path().strip_prefix(url.path()) else {
            continue;
        };
        let Ok(number) = suffix.trim_end_matches('/').parse::<usize>() else {
            continue;
        };
        if number > MAX_SUBPAGES {
            return Err("result_too_large");
        }
        if number > after && target.query().is_none() {
            pending.insert(number, target);
        }
    }
    Ok(())
}

fn get_pages(key: &str) -> Result<Value> {
    let url = article(key)?;
    let mut pending = BTreeMap::new();
    pending.insert(1, url.clone());
    let mut all = Vec::new();
    let mut previous = 0;
    while let Some((number, target)) = pending.pop_first() {
        if number != previous + 1 {
            return Err("parse_error");
        }
        let doc = document(&target)?;
        let root = doc.require(".entry-content")?;
        let images = wp_gallery::images(&root, false, wp_gallery::LAZY_FIRST, image)?;
        if images.is_empty() {
            return Err("chapter_unavailable");
        }
        all.extend(images);
        collect_album_links(&doc, &url, number, &mut pending)?;
        previous = number;
    }
    pages(key, unique(all))
}

fn filters(args: &Value, req: &[u8], labels: &Labels<'_>) -> Result<Value> {
    let any = visible_name(labels, "filter", "any")?;
    let mut categories = vec![option("", &any)];
    for (value, id) in CATEGORIES {
        categories.push(option(value, &visible_name(labels, "listing", id)?));
    }
    let mut filters = vec![select_filter(labels, "category", categories, true)?];
    let lazy = source_protocol::feature(req, b"source_filter_fetching_v1");
    let tri = source_protocol::feature(req, b"source_filter_tristate_v1");
    let fetch = lazy && args["fetchFilterData"] == true;
    let mut tags = Vec::new();
    if fetch {
        let endpoint =
            own_url("/wp-json/wp/v2/tags?per_page=100&hide_empty=true&orderby=count&order=desc")?;
        for tag in wp_gallery::rest(&endpoint)?
            .as_array()
            .ok_or("parse_error")?
        {
            if let (Some(id), Some(label)) = (tag["id"].as_u64(), tag["name"].as_str()) {
                tags.push(option(&id.to_string(), &wp_gallery::html_text(label)?));
            }
        }
    } else if !lazy || !tri {
        tags.extend(TAGS.iter().map(|(id, name)| option(id, name)));
    }
    if tri {
        let mut filter = json!({"type":"tristate","id":"tags","default":[],
            "name":visible_name(labels,"filter","tags")?,
            "description":visible_name(labels,"filter","tags_hint")?});
        if fetch || !lazy {
            filter["options"] = json!(tags);
        }
        filters.push(filter);
    } else {
        tags.insert(0, option("", &any));
        filters.push(select_filter(labels, "tag", tags.clone(), false)?);
        filters.push(select_filter(labels, "exclude_tag", tags, false)?);
    }
    Ok(json!({"filters":filters,"supportsFilterFetching":lazy}))
}

koma_source_sdk::koma_source_exports!("everiaclub", typed_chapters, write_chapter_failure);
