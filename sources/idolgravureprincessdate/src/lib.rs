#![no_std]
extern crate alloc;

use alloc::{
    collections::BTreeSet,
    format,
    string::{String, ToString},
    vec::Vec,
};
use koma_source_sdk::{
    source::{SourceCapabilities, SourceInfo},
    source_protocol,
};
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

const SITE: &str = "https://idol.gravureprincess.date";
const LANGUAGE: &str = "ja";
const METADATA: &[u8] = include_bytes!("../metadata.json");
const SOURCE_INFO: SourceInfo = info("com.idolgravureprincessdate.koma", "Idol.gravureprincess.date",
    "A Japanese idol and voice-actor photo archive organized by groups and magazine issues, including Young Champion and Seigura.");
const SOURCE_CAPS: SourceCapabilities = CAPS;
const LISTINGS: &[(&str, bool, bool)] = &[("latest", true, true)];
const PAGE_SIZE: usize = 25;
const IMAGE_ATTRS: &[&str] = &["data-lazy-src", "data-src", "src"];

koma_source_sdk::koma_source_buffers! {
    payload: 768 * 1024, http_out: 5 * 1024 * 1024, body: 3 * 1024 * 1024,
    http_req: 4096, scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

fn article(raw: &str) -> Result<Url> {
    let mut url = https(SITE, raw)?;
    if url.host_str() != Some("idol.gravureprincess.date") {
        return Err("invalid_request");
    }
    let mut segments = url.path().trim_start_matches('/').split('/');
    let date_part = |s: &str, n| s.len() == n && s.bytes().all(|b| b.is_ascii_digit());
    if !segments.next().is_some_and(|s| date_part(s, 4))
        || !segments.next().is_some_and(|s| date_part(s, 2))
        || !segments
            .next()
            .is_some_and(|s| s.ends_with(".html") && s.len() > 5)
        || segments.next().is_some()
    {
        return Err("invalid_request");
    }
    url.set_query(None);
    Ok(url)
}

fn image(raw: &str) -> Option<Url> {
    let url = https(SITE, raw).ok()?;
    let ok = match url.host_str().unwrap_or_default() {
        "blogger.googleusercontent.com" => url.path().starts_with("/img/"),
        "1.bp.blogspot.com"
        | "2.bp.blogspot.com"
        | "3.bp.blogspot.com"
        | "4.bp.blogspot.com"
        | "lh3.googleusercontent.com" => url.path().len() > 1,
        _ => false,
    };
    ok.then_some(url)
}

fn json_get(url: &Url) -> Result<Value> {
    let response = fetch(url, "application/json")?;
    if response.status == 400 {
        return Err("invalid_request");
    }
    serde_json::from_str(&response.body).map_err(|_| "parse_error")
}

fn feed_url() -> Result<Url> {
    https(SITE, "/feeds/posts/default?alt=json")
}

fn term(raw: &Value) -> Option<&str> {
    raw["term"].as_str().filter(|s| valid_label(s))
}

fn valid_label(s: &str) -> bool {
    !s.trim().is_empty()
        && s.len() <= 256
        && !s.chars().any(|c| c.is_control() || matches!(c, '"' | '\\'))
}

fn feed_card(entry: &Value) -> Result<Gallery> {
    let link = entry["link"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|x| x["rel"] == "alternate")
        .and_then(|x| x["href"].as_str())
        .ok_or("parse_error")?;
    let url = article(link)?;
    let title = entry["title"]["$t"]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .ok_or("parse_error")?;
    let cover_from_content = || {
        let html = Node::parse(entry["content"]["$t"].as_str()?).ok()?;
        lazy_image(&html.one("img")?, IMAGE_ATTRS, image)
    };
    let cover =
        cover_from_content().or_else(|| entry["media$thumbnail"]["url"].as_str().and_then(image));
    let tags = entry["category"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(term)
        .map(String::from)
        .collect();
    Ok(Gallery {
        key: url.path().into(),
        url,
        title: title.into(),
        cover,
        tags,
        published: entry["published"]["$t"].as_str().map(String::from),
    })
}

fn query(args: &Value, search: bool) -> Result<String> {
    let mut all = Vec::new();
    let mut seen = BTreeSet::new();
    for key in ["idol", "magazine", "other"] {
        match &args["filters"][key] {
            Value::Null => (),
            Value::Array(tags) if tags.len() <= 32 => {
                for value in tags {
                    let label = value
                        .as_str()
                        .filter(|s| valid_label(s))
                        .ok_or("invalid_request")?;
                    if seen.insert(label) {
                        all.push(format!("label:\"{label}\""));
                    }
                }
            }
            _ => return Err("invalid_request"),
        }
    }
    if search {
        if let Some(words) = args["query"]
            .as_str()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            all.push(words.into());
        }
    }
    let query = all.join(" ");
    if query.len() > 1200 {
        return Err("invalid_request");
    }
    Ok(query)
}

fn browse(args: &Value, search: bool, _: &[u8]) -> Result<Value> {
    let page = page_number(args)?;
    if !search {
        let listing = args["listingId"].as_str().unwrap_or("latest");
        if !matches!(
            listing.strip_prefix("listing:").unwrap_or(listing),
            "latest" | "popular"
        ) {
            return Err("invalid_request");
        }
    }
    let start = (page - 1) * PAGE_SIZE + 1;
    let query = query(args, search)?;
    let mut url = feed_url()?;
    url.query_pairs_mut()
        .append_pair("max-results", &PAGE_SIZE.to_string())
        .append_pair("start-index", &start.to_string());
    if !query.is_empty() {
        url.query_pairs_mut().append_pair("q", &query);
    }
    let data = json_get(&url)?;
    let feed = data["feed"].as_object().ok_or("parse_error")?;
    let entries = feed
        .get("entry")
        .map(|value| value.as_array().map(Vec::as_slice).ok_or("parse_error"))
        .transpose()?
        .unwrap_or_default();
    let total = feed
        .get("openSearch$totalResults")
        .and_then(|total| total["$t"].as_str())
        .and_then(|total| total.parse::<usize>().ok());
    // Blogger's q feed reports a page-local total (25 on both pages 1 and 2
    // of Nogizaka46). Advance a full query page as the upstream extension does.
    let more = if query.is_empty() {
        !entries.is_empty()
            && total.map_or(entries.len() == PAGE_SIZE, |total| {
                start - 1 + entries.len() < total
            })
    } else {
        entries.len() == PAGE_SIZE
    };
    let items: Vec<_> = entries
        .iter()
        .filter_map(|e| feed_card(e).ok())
        .map(|g| g.manga())
        .collect();
    if items.is_empty() && !entries.is_empty() {
        return Err("parse_error");
    }
    Ok(port::items(items, more.then_some(page + 1)))
}

fn detail(key: &str) -> Result<Gallery> {
    let url = article(key)?;
    let doc = document(&url)?;
    let title = doc.require(".post-title")?.text()?;
    let root = doc.require(".post-body")?;
    let cover = doc
        .one("link[rel='image_src']")
        .and_then(|n| n.attr("href").ok())
        .and_then(|s| image(&s))
        .or_else(|| {
            root.one("img")
                .and_then(|n| lazy_image(&n, IMAGE_ATTRS, image))
        });
    let tags = doc
        .all(".post-labels a[rel='tag']")?
        .iter()
        .filter_map(|n| n.text().ok())
        .collect();
    // Article HTML can omit the timestamp. Blogger's stable numeric post ID
    // links it to the original Atom entry without putting a date in the work ID.
    let id_attribute = root.attr("id")?;
    let post_id = id_attribute
        .strip_prefix("post-body-")
        .filter(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()));
    let published = post_id
        .and_then(|id| https(SITE, &format!("/feeds/posts/default/{id}?alt=json")).ok())
        .and_then(|url| json_get(&url).ok())
        .and_then(|entry| entry["entry"]["published"]["$t"].as_str().map(String::from));
    Ok(Gallery {
        key: url.path().into(),
        url,
        title,
        cover,
        tags,
        published,
    })
}

fn get_pages(key: &str) -> Result<Value> {
    let url = article(key)?;
    let doc = document(&url)?;
    let root = doc.require(".post-body")?;
    let mut images = Vec::new();
    for anchor in root.all("a[href]")? {
        if anchor.one("img").is_some() {
            if let Some(url) = image(&anchor.attr("href")?) {
                images.push(url);
            }
        }
    }
    if images.is_empty() {
        images.extend(
            root.all("img")?
                .iter()
                .filter_map(|node| lazy_image(node, IMAGE_ATTRS, image)),
        );
    }
    pages(url.path(), unique(images))
}

fn label_filter(
    id: &str,
    labels: &Labels<'_>,
    options: Option<impl IntoIterator<Item = impl AsRef<str>>>,
) -> Result<Value> {
    let mut filter = json!({
        "type":"multiselect","id":id,"name":visible_name(labels,"filter",id)?,
        "description":visible_name(labels,"filter",&format!("{id}_hint"))?,"default":[]
    });
    if let Some(options) = options {
        filter["options"] = json!(options
            .into_iter()
            .map(|label| {
                let label = label.as_ref();
                option(label, label)
            })
            .collect::<Vec<_>>());
    }
    Ok(filter)
}

fn filters(args: &Value, req: &[u8], labels: &Labels<'_>) -> Result<Value> {
    let mut filters = alloc::vec![
        label_filter("idol", labels, Some(IDOLS.iter().copied()))?,
        label_filter("magazine", labels, Some(MAGAZINES.iter().copied()))?,
    ];
    let lazy = source_protocol::feature(req, b"source_filter_fetching_v1");
    if lazy {
        let options = if args["fetchFilterData"] == true {
            let mut url = feed_url()?;
            url.query_pairs_mut().append_pair("max-results", "0");
            let data = json_get(&url)?;
            let other = data["feed"]["category"].as_array().ok_or("parse_error")?;
            Some(
                other
                    .iter()
                    .filter_map(term)
                    .filter(|label| !IDOLS.contains(label) && !MAGAZINES.contains(label))
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .map(String::from)
                    .collect::<Vec<_>>(),
            )
        } else {
            None
        };
        filters.push(label_filter("other", labels, options)?);
    }
    Ok(json!({"filters":filters,"supportsFilterFetching":lazy}))
}

koma_source_sdk::koma_source_exports!(
    "idolgravureprincessdate",
    typed_chapters,
    write_chapter_failure
);

const IDOLS: &[&str] = &[
    "Nogizaka46",
    "AKB48",
    "NMB48",
    "Keyakizaka46",
    "HKT48",
    "SKE48",
    "NGT48",
    "SUPER☆GiRLS",
    "Morning Musume",
    "Dempagumi.inc",
    "Angerme",
    "Juice=Juice",
    "NijiCon-虹コン",
    "Houkago Princess",
    "Magical Punchline",
    "Idoling!!!",
    "Rev. from DVL",
    "Link STAR`s",
    "LADYBABY",
    "℃-ute",
    "Country Girls",
    "Up Up Girls (Kakko Kari)",
    "Yumemiru Adolescence",
    "Shiritsu Ebisu Chugaku",
    "Tenkoushoujo Kagekidan",
    "Drop",
    "Steam Girls",
    "Kamen Joshi's",
    "LinQ",
    "Doll☆Element",
    "TrySail",
    "Akihabara Backstage Pass",
    "Palet",
    "Passport☆",
    "Ange☆Reve",
    "BiSH",
    "Ciao Bella Cinquetti",
    "Gekidanherbest",
    "Haraeki Stage Ace",
    "Ru:Run",
    "SDN48",
];

const MAGAZINES: &[&str] = &[
    "FLASH",
    "Weekly Playboy",
    "FRIDAY Magazine",
    "Young Jump",
    "Young Magazine",
    "BLT",
    "ENTAME",
    "EX-Taishu",
    "SPA! Magazine",
    "Young Gangan",
    "UTB",
    "Young Animal",
    "Young Champion",
    "Big Comic Spirtis",
    "Shonen Magazine",
    "BUBKA",
    "BOMB",
    "Shonen Champion",
    "Manga Action",
    "Weekly Shonen Sunday",
    "Photobooks",
    "BRODY",
    "Hustle Press",
    "ANAN Magazine",
    "SMART Magazine",
    "Young Sunday",
    "Gravure The Television",
    "CD&DL My Girl",
    "Daily LoGiRL",
    "Shukan Taishu",
    "Girls! Magazine",
    "Soccer Game King",
    "Weekly Georgia",
    "Sunday Magazine",
    "Mery Magazine",
];
