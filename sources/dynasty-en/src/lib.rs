#![no_std]
extern crate alloc;

use alloc::{
    collections::BTreeSet,
    format,
    string::{String, ToString},
    vec::Vec,
};
use koma_source_sdk::{
    chapters::{
        Chapter, ChapterPage, ChapterPlacement, ChapterResponse, ChapterResult, GroupedChapters,
    },
    host,
    source::{SourceCapabilities, SourceInfo},
    source_protocol,
};
use percent_encoding::percent_decode_str;
use serde_json::{json, Value};
use url::{form_urlencoded, Url};

#[path = "../../port_support.rs"]
#[allow(dead_code)]
mod port;
#[path = "../../port_catalog.rs"]
mod port_catalog;
use port::{Node, Result};
use port_catalog::Labels;

#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

const SITE: &str = "https://dynasty-scans.com";
const METADATA: &[u8] = include_bytes!("../metadata.json");
const COVER_DATA: &str = include_str!("covers.json");
const TAG_DATA: &str = include_str!("tags.json");
const LISTINGS: &[&str] = &[
    "popular",
    "latest",
    "series",
    "chapters",
    "anthologies",
    "issues",
    "doujins",
];
const KINDS: &[(&str, &str)] = &[
    ("series", "Series"),
    ("chapters", "Chapter"),
    ("anthologies", "Anthology"),
    ("issues", "Issue"),
    ("doujins", "Doujin"),
];
const SORTS: &[(&str, &str)] = &[
    ("_smart_", "smart"),
    ("", "match"),
    ("name", "name"),
    ("created_at", "added"),
    ("released_on", "released"),
];
const PEOPLE: &[(&str, &str)] = &[
    ("author", "Author"),
    ("scanlator", "Scanlator"),
    ("pairing", "Pairing"),
];
const LIMITS: &[&str] = &["2", "5", "10", "all"];
const TRISTATE: &[u8] = b"source_filter_tristate_v1";

const SOURCE_INFO: SourceInfo = SourceInfo {
    id: "com.dynastyscans.en.koma",
    name: "Dynasty Scans",
    language: "en",
    version: env!("CARGO_PKG_VERSION"),
    api_version: "0.2",
    description: "An English manga reader with series, anthologies, magazine issues, and doujinshi, with a focus on yuri. Some entries are adult.",
    author: "Koma source contributors",
    content_rating: "mixed",
};
const SOURCE_CAPS: SourceCapabilities = SourceCapabilities {
    search: true,
    manga_detail: true,
    chapters: true,
    pages: true,
    listings: true,
    manga_list: true,
    filters: true,
    settings: true,
    home: false,
    credentials: false,
    image_request: false,
};

koma_source_sdk::koma_source_buffers! {
    payload: 512 * 1024, http_out: 8 * 1024 * 1024, body: 4 * 1024 * 1024,
    http_req: 4096, scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

fn nonempty(value: &Value) -> Option<&str> {
    value.as_str().filter(|s| !s.trim().is_empty())
}

fn kind_from_type(kind: &str) -> Option<&'static str> {
    KINDS
        .iter()
        .find(|(_, name)| *name == kind)
        .map(|(kind, _)| *kind)
}

fn url(path: &str) -> Result<Url> {
    let url = port::absolute(SITE, path)?;
    if url.origin().ascii_serialization() != SITE || url.fragment().is_some() {
        return Err("parse_error");
    }
    Ok(url)
}

fn slug(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && !matches!(value, "." | "..")
        && !value.chars().any(|c| {
            c.is_control() || c.is_whitespace() || matches!(c, '/' | '\\' | '?' | '#' | ':' | '%')
        })
}

struct Entry<'a> {
    kind: &'a str,
    slug: String,
}

impl<'a> Entry<'a> {
    fn new(kind: &'a str, value: &str) -> Result<Self> {
        if !KINDS.iter().any(|(key, _)| *key == kind) || !slug(value) {
            return Err("invalid_request");
        }
        Ok(Self {
            kind,
            slug: String::from(value),
        })
    }

    fn from_path(path: &'a str) -> Result<Self> {
        let (kind, value) = path
            .strip_prefix('/')
            .and_then(|p| p.split_once('/'))
            .ok_or("invalid_request")?;
        let value = percent_decode_str(value)
            .decode_utf8()
            .map_err(|_| "invalid_request")?;
        Self::new(kind, &value)
    }

    fn url(&self, suffix: &str) -> Result<Url> {
        let mut url = url("/")?;
        let file = format!("{}{suffix}", self.slug);
        url.path_segments_mut()
            .map_err(|_| "internal_error")?
            .pop_if_empty()
            .push(self.kind)
            .push(&file);
        Ok(url)
    }

    fn id(&self) -> String {
        format!("dynasty:{}/{}", self.kind, self.slug)
    }

    fn chapter_id(&self) -> String {
        format!("dynasty-ch:{}", self.slug)
    }
}

fn input_entry(input: &Value) -> Result<Entry<'_>> {
    let id = input["args"]["mangaId"].as_str().ok_or("invalid_request")?;
    if let Some(path) = id.strip_prefix("dynasty:") {
        let (kind, value) = path.split_once('/').ok_or("invalid_request")?;
        return Entry::new(kind, value);
    }
    let href = id.strip_prefix(SITE).unwrap_or(id);
    Entry::from_path(href)
}

fn xhr(uri: &Url) -> Result<Value> {
    let response = port::request(
        uri.as_str(),
        "GET",
        json!({"Accept":"application/json","X-Requested-With":"XMLHttpRequest","Referer":SITE}),
        None,
    )?;
    serde_json::from_str(&response).map_err(|_| "parse_error")
}

fn fetch_entry(entry: &Entry<'_>, page: usize) -> Result<Value> {
    let mut uri = entry.url(".json")?;
    if page > 1 {
        uri.query_pairs_mut().append_pair("page", &page.to_string());
    }
    let data = xhr(&uri)?;
    if data["permalink"].as_str() != Some(entry.slug.as_str())
        || (entry.kind != "chapters"
            && data["type"].as_str().and_then(kind_from_type) != Some(entry.kind))
    {
        return Err("parse_error");
    }
    Ok(data)
}

fn cover(raw: Option<&str>) -> Result<Value> {
    let Some(raw) = raw else {
        return Ok(json!({"kind":"none"}));
    };
    let path = if raw.starts_with("/system/") {
        String::from(raw)
    } else {
        format!(
            "/system/tag_contents_covers/000/{raw}",
            raw = raw.trim_start_matches('/')
        )
    };
    let uri = url(&path)?;
    if !uri.path().starts_with("/system/") {
        return Err("parse_error");
    }
    Ok(json!({"kind":"url","url":uri.as_str(),"headers":{"Referer":SITE}}))
}

fn item(entry: &Entry<'_>, title: &str, file: Option<&str>) -> Result<Value> {
    let uri = entry.url("")?;
    Ok(
        json!({"id":entry.id(),"title":title,"url":uri.as_str(),"language":"en",
        "cover":cover(file)?,"links":[{"kind":"source","url":uri.as_str()}]}),
    )
}

fn text_values<'a>(tags: impl IntoIterator<Item = &'a Value>, kind: &str) -> Vec<&'a str> {
    let mut seen = BTreeSet::new();
    tags.into_iter()
        .filter(|tag| tag["type"] == kind)
        .filter_map(|tag| nonempty(&tag["name"]))
        .filter(|value| seen.insert(*value))
        .collect()
}

fn set_summary(
    card: &mut Value,
    request: &[u8],
    update: &str,
    authors: &[&str],
    tags: &[&str],
) -> Result<()> {
    card["authors"] = json!(authors);
    card["tags"] = json!(tags);
    if source_protocol::feature(request, b"source_manga_list_info_v1") {
        let label = Labels::new(METADATA, request)?.text("list.update", "Latest chapter")?;
        let display_tags: Vec<_> = tags
            .iter()
            .copied()
            .filter(|s| s.len() <= 96)
            .take(8)
            .collect();
        let mut info = Vec::new();
        if !update.trim().is_empty() && update.len() <= 512 && update != card["title"] {
            info.push(json!({"type":"text","label":label,"text":update}));
        }
        if !display_tags.is_empty() {
            info.push(json!({"type":"tags","items":display_tags}));
        }
        if !info.is_empty() {
            card["listInfo"] = json!(info);
        }
    }
    Ok(())
}

fn page_number(cursor: &Value) -> Result<usize> {
    if cursor.is_null() {
        return Ok(1);
    }
    cursor
        .as_str()
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|n| (2..=100_000).contains(n))
        .ok_or("invalid_request")
}

fn provider_page(data: &Value, fetched: usize) -> Result<usize> {
    if data["total_pages"].is_null() && fetched == 1 {
        return Ok(1);
    }
    if data["current_page"].as_u64() != Some(fetched as u64) {
        return Err("catalog_changed");
    }
    data["total_pages"]
        .as_u64()
        .filter(|total| *total <= 100_000)
        .map(|total| total as usize)
        .ok_or("parse_error")
}

fn add(items: &mut Vec<Value>, seen: &mut BTreeSet<String>, entry: &Entry<'_>, row: Value) {
    if seen.insert(entry.id()) {
        items.push(row);
    }
}

fn cached_cover<'a>(covers: &'a Value, entry: &Entry<'_>) -> Option<&'a str> {
    covers[entry.kind][&entry.slug].as_str()
}

fn entry_for_chapter<'a>(entry: Entry<'a>, covers: &Value) -> Entry<'a> {
    if entry.kind != "chapters" {
        return entry;
    }
    // A numbered release is grouped into a series only when the upstream
    // catalog actually knows that series, so doujin slugs stay directly usable.
    if let Some((prefix, _)) = entry.slug.match_indices('_').find(|(at, _)| {
        let suffix = &entry.slug[at + 1..];
        suffix
            .strip_prefix("ch")
            .is_some_and(|n| n.starts_with(|c: char| c.is_ascii_digit()))
            || suffix
                .strip_prefix("volume_")
                .is_some_and(|n| n.starts_with(|c: char| c.is_ascii_digit()))
    }) {
        let name = &entry.slug[..prefix];
        if covers["series"].get(name).is_some() {
            return Entry {
                kind: "series",
                slug: String::from(name),
            };
        }
    }
    entry
}

fn recent(added: bool, page: usize, request: &[u8], covers: &Value) -> Result<Value> {
    let path = if added {
        "/chapters/added.json"
    } else {
        "/chapters.json"
    };
    let mut uri = url(path)?;
    uri.query_pairs_mut().append_pair("page", &page.to_string());
    let data = xhr(&uri)?;
    let total = provider_page(&data, page)?;
    let mut items = Vec::new();
    let mut seen = BTreeSet::new();
    for row in data["chapters"].as_array().ok_or("parse_error")? {
        let tags = row["tags"].as_array().ok_or("parse_error")?;
        let title = nonempty(&row["title"]).ok_or("parse_error")?;
        let authors = text_values(tags, "Author");
        let topics = text_values(tags, "General");
        let mut grouped = false;
        if added {
            for tag in tags {
                let Some(kind) = tag["type"].as_str().and_then(kind_from_type) else {
                    continue;
                };
                let entry = Entry::new(kind, nonempty(&tag["permalink"]).ok_or("parse_error")?)?;
                let mut card = item(
                    &entry,
                    nonempty(&tag["name"]).ok_or("parse_error")?,
                    cached_cover(covers, &entry),
                )?;
                set_summary(&mut card, request, title, &authors, &topics)?;
                add(&mut items, &mut seen, &entry, card);
                grouped |= kind == "series";
            }
        }
        if !grouped {
            let entry = Entry::new(
                "chapters",
                nonempty(&row["permalink"]).ok_or("parse_error")?,
            )?;
            let mut card = item(&entry, title, None)?;
            set_summary(&mut card, request, title, &authors, &topics)?;
            add(&mut items, &mut seen, &entry, card);
        }
    }
    Ok(port::items(items, (page < total).then_some(page + 1)))
}

fn directory(listing: &str, page: usize, covers: &Value) -> Result<Value> {
    let mut uri = url(&format!("/{listing}"))?;
    if page > 1 {
        uri.query_pairs_mut().append_pair("page", &page.to_string());
    }
    let doc = Node::parse(&port::get(uri.as_str())?)?;
    let list = doc.require("#main dl.tag-list")?;
    let rows = list.all("dd a[href]")?;
    let mut items = Vec::new();
    for row in rows {
        let href = row.attr("href")?;
        let entry = Entry::from_path(&href)?;
        if entry.kind == listing {
            items.push(item(&entry, &row.text()?, cached_cover(covers, &entry))?);
        }
    }
    Ok(port::items(items, next_page(&doc, page)))
}

fn next_page(doc: &Node, page: usize) -> Option<usize> {
    doc.one(".pagination a[rel=next]").map(|_| page + 1)
}

fn popular(covers: &Value) -> Result<Value> {
    let doc = Node::parse(&port::get(SITE)?)?;
    let mut seen = BTreeSet::new();
    let mut items = Vec::new();
    // The homepage also has a Recently Added image list. Use only the list
    // under the site's named seven-day ranking.
    let sidebar = doc
        .all(".span4")?
        .into_iter()
        .find(|section| {
            section.all("h4").is_ok_and(|headings| {
                headings.iter().any(|h| {
                    h.text()
                        .is_ok_and(|text| text == "Most Popular of Past 7 Days")
                })
            })
        })
        .ok_or("parse_error")?;
    for row in sidebar.all("h4 ~ ul.cover-list a.thumbnail[href]")? {
        let href = row.attr("href")?;
        let original = Entry::from_path(&href)?;
        let title = row.require(".caption b")?.text()?;
        let img = row.require("img[src]")?.attr("src")?;
        let entry = entry_for_chapter(original, covers);
        let (title, image) = if entry.kind == "chapters" {
            (title, Some(img.as_str()))
        } else {
            (pretty_slug(&entry.slug), cached_cover(covers, &entry))
        };
        let card = item(&entry, &title, image)?;
        add(&mut items, &mut seen, &entry, card);
    }
    Ok(port::items(items, None))
}

fn pretty_slug(slug: &str) -> String {
    slug.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            let Some(first) = chars.next() else {
                return String::new();
            };
            format!("{}{}", first.to_uppercase(), chars.as_str())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn list(input: &Value, request: &[u8]) -> Result<Value> {
    let args = &input["args"];
    let listing = args["listingId"].as_str().unwrap_or("popular");
    let page = page_number(&args["page"]["cursor"])?;
    let covers: Value = serde_json::from_str(COVER_DATA).map_err(|_| "internal_error")?;
    match listing {
        "latest" | "chapters" => recent(listing == "latest", page, request, &covers),
        "series" | "anthologies" | "issues" | "doujins" => directory(listing, page, &covers),
        "popular" if page == 1 => popular(&covers),
        _ => Err("invalid_request"),
    }
}

fn tags_data() -> Result<Value> {
    serde_json::from_str(TAG_DATA).map_err(|_| "internal_error")
}

fn resolve_person(name: &str, kind: &str) -> Result<u64> {
    let form: String = form_urlencoded::Serializer::new(String::new())
        .append_pair("query", name)
        .finish();
    let text = port::request(
        &format!("{SITE}/tags/suggest"),
        "POST",
        json!({"Accept":"application/json","Content-Type":"application/x-www-form-urlencoded",
            "X-Requested-With":"XMLHttpRequest","Referer":SITE,"Origin":SITE}),
        Some(&form),
    )?;
    let values: Vec<Value> = serde_json::from_str(&text).map_err(|_| "parse_error")?;
    let lower = name.to_lowercase();
    values
        .iter()
        .find_map(|tag| {
            (tag["type"] == kind
                && nonempty(&tag["name"])
                    .is_some_and(|actual| actual.trim().to_lowercase() == lower))
            .then(|| tag["id"].as_u64())
            .flatten()
        })
        .ok_or("unknown_filter")
}

fn search_url(args: &Value, query: &str, page: usize) -> Result<Url> {
    let filters = &args["filters"];
    let sort = match &filters["sort"] {
        Value::Null => "_smart_",
        value => value.as_str().ok_or("invalid_request")?,
    };
    if !SORTS.iter().any(|(id, _)| *id == sort) {
        return Err("invalid_request");
    }
    let mut uri = url("/search")?;
    let mut pairs = uri.query_pairs_mut();
    pairs.append_pair("q", query).append_pair(
        "sort",
        match (sort, query.is_empty()) {
            ("_smart_", true) => "released_on",
            ("_smart_", false) => "",
            _ => sort,
        },
    );
    let kinds = selected_kinds(filters)?;
    for kind in &kinds {
        pairs.append_pair("classes[]", kind);
    }
    if !kinds.contains(&"Chapter") && (kinds.contains(&"Series") || kinds.contains(&"Doujin")) {
        pairs.append_pair("classes[]", "Chapter");
    }
    if let Some(tags) = filters["tags"].as_array() {
        let catalogue = tags_data()?;
        let catalogue = catalogue.as_array().ok_or("internal_error")?;
        if tags.len() > 32 {
            return Err("invalid_request");
        }
        let mut seen = BTreeSet::new();
        for tag in tags {
            let raw = tag.as_str().ok_or("invalid_request")?;
            let (key, value) = match raw.strip_prefix('!') {
                Some(id) => ("without[]", id),
                None => ("with[]", raw),
            };
            let id = value.parse::<u64>().map_err(|_| "invalid_request")?;
            if !seen.insert(id) || !catalogue.iter().any(|tag| tag["id"].as_u64() == Some(id)) {
                return Err("invalid_request");
            }
            pairs.append_pair(key, value);
        }
    }
    let mut names_count = 0;
    for (key, kind) in PEOPLE {
        let names = filters[*key].as_str().unwrap_or("");
        if names.len() > 512 {
            return Err("invalid_request");
        }
        for name in names
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
        {
            names_count += 1;
            if names_count > 8 {
                return Err("invalid_request");
            }
            pairs.append_pair("with[]", &resolve_person(name, kind)?.to_string());
        }
    }
    if page > 1 {
        pairs.append_pair("page", &page.to_string());
    }
    drop(pairs);
    Ok(uri)
}

fn selected_kinds(filters: &Value) -> Result<Vec<&str>> {
    let Some(values) = filters["type"].as_array() else {
        return Ok(KINDS.iter().map(|(_, kind)| *kind).collect());
    };
    if values.is_empty() || values.len() > KINDS.len() {
        return Err("invalid_request");
    }
    let mut kinds = Vec::new();
    for value in values {
        let kind = value
            .as_str()
            .filter(|k| kind_from_type(k).is_some())
            .ok_or("invalid_request")?;
        if !kinds.contains(&kind) {
            kinds.push(kind);
        }
    }
    Ok(kinds)
}

fn search(input: &Value, request: &[u8]) -> Result<Value> {
    let args = &input["args"];
    let query = args["query"].as_str().unwrap_or("").trim();
    let page = page_number(&args["page"]["cursor"])?;
    let covers: Value = serde_json::from_str(COVER_DATA).map_err(|_| "internal_error")?;
    if let Some(path) = query.strip_prefix(SITE).filter(|p| p.starts_with('/')) {
        if page != 1 {
            return Err("invalid_request");
        }
        let uri = url(path)?;
        let entry = Entry::from_path(uri.path())?;
        let data = fetch_entry(&entry, 1)?;
        return Ok(port::items(
            alloc::vec![details_card(&entry, &data, &covers)?],
            None,
        ));
    }
    let uri = search_url(args, query, page)?;
    let doc = Node::parse(&port::get(uri.as_str())?)?;
    let mut all = Vec::new();
    let mut matched = Vec::new();
    let mut seen = BTreeSet::new();
    let kinds = selected_kinds(&args["filters"])?;
    for row in doc.all(".chapter-list dd")? {
        let topics = row
            .all(".tags a[href^='/tags/']")?
            .iter()
            .map(Node::text)
            .collect::<Result<Vec<_>>>()?;
        let authors = row
            .all("a[href^='/authors/']")?
            .iter()
            .map(Node::text)
            .collect::<Result<Vec<_>>>()?;
        for link in row.all("a.name[href], .doujin_tags a[href^='/doujins/']")? {
            let href = link.attr("href")?;
            let Ok(original) = Entry::from_path(&href) else {
                continue;
            };
            let original_kind = original.kind;
            let entry = entry_for_chapter(original, &covers);
            if !seen.insert(entry.id()) {
                continue;
            }
            let raw_title = link.text()?;
            let title = if original_kind == entry.kind {
                raw_title.clone()
            } else {
                pretty_slug(&entry.slug)
            };
            let mut card = item(&entry, &title, cached_cover(&covers, &entry))?;
            let authors: Vec<_> = authors.iter().map(String::as_str).collect();
            let topics: Vec<_> = topics.iter().map(String::as_str).collect();
            let update = if original_kind == "chapters" {
                raw_title.as_str()
            } else {
                ""
            };
            set_summary(&mut card, request, update, &authors, &topics)?;
            if KINDS
                .iter()
                .any(|(key, kind)| *key == entry.kind && kinds.contains(kind))
            {
                matched.push(card.clone());
            }
            all.push(card);
        }
    }
    // The upstream deliberately keeps the first related result when
    // chapter-assisted search finds no exact entry of a selected type.
    if matched.is_empty() {
        matched.extend(all.into_iter().take(1));
    }
    Ok(port::items(matched, next_page(&doc, page)))
}

fn image(raw: &str) -> Result<Url> {
    let uri = url(raw)?;
    if !uri.path().starts_with("/system/releases/") {
        return Err("parse_error");
    }
    Ok(uri)
}

fn chapter_pages(data: &Value) -> Result<&Vec<Value>> {
    data["pages"]
        .as_array()
        .filter(|pages| !pages.is_empty())
        .ok_or("chapter_unavailable")
}

fn details_card(entry: &Entry<'_>, data: &Value, covers: &Value) -> Result<Value> {
    let standalone = entry.kind == "chapters";
    let tags = data["tags"].as_array().ok_or("parse_error")?;
    let chapter_tags = data["taggings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|component| component["tags"].as_array())
        .flatten();
    let all_tags = tags.iter().chain(chapter_tags);
    let authors = text_values(all_tags.clone(), "Author");
    let genres = text_values(all_tags, "General");
    let standings = text_values(tags, "Status");
    let status = if standalone || standings.contains(&"Completed") {
        "completed"
    } else if standings.contains(&"Ongoing") {
        "ongoing"
    } else if standings.contains(&"On Hiatus") {
        "hiatus"
    } else if standings.iter().any(|value| {
        matches!(
            *value,
            "Dropped" | "Cancelled" | "Not Updated" | "Abandoned" | "Removed"
        )
    }) {
        "cancelled"
    } else {
        "unknown"
    };
    let cover = if standalone {
        nonempty(&chapter_pages(data)?[0]["url"])
    } else {
        nonempty(&data["cover"]).or_else(|| cached_cover(covers, entry))
    };
    let title = if standalone {
        &data["title"]
    } else {
        &data["name"]
    };
    let mut manga = item(entry, nonempty(title).ok_or("parse_error")?, cover)?;
    manga["authors"] = json!(authors);
    manga["artists"] = json!(authors);
    manga["tags"] = json!(genres);
    manga["contentRating"] = json!(if genres.contains(&"NSFW") {
        "adult"
    } else {
        "mixed"
    });
    manga["status"] = json!(status);
    if let Some(aliases) = data["aliases"].as_array() {
        manga["alternateTitles"] = json!(aliases.iter().filter_map(nonempty).collect::<Vec<_>>());
    }
    if let Some(raw) = nonempty(&data["description"]) {
        manga["description"] = json!(Node::parse(raw)?.text()?);
    }
    Ok(manga)
}

fn detail(input: &Value) -> Result<Value> {
    let entry = input_entry(input)?;
    let covers = serde_json::from_str(COVER_DATA).map_err(|_| "internal_error")?;
    let data = fetch_entry(&entry, 1)?;
    details_card(&entry, &data, &covers).map(|manga| json!({"manga":manga}))
}

fn limit(input: &Value) -> usize {
    match input["settings"]["chapterFetchLimit"].as_str() {
        Some("all") => usize::MAX,
        Some("5") => 5,
        Some("10") => 10,
        _ => 2,
    }
}

fn dates_and_group<'a>(chapter: &mut Chapter<'a>, data: &'a Value, scanlator: &'a str) {
    chapter.published_at = nonempty(&data["released_on"]).map(str::as_bytes);
    chapter.language = Some(b"en");
    if !scanlator.is_empty() {
        chapter.scanlator = Some(scanlator.as_bytes());
    }
}

fn chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    const BATCH: usize = 80;
    let input = port::input(request)?;
    let entry = input_entry(&input)?;
    let args = &input["args"];
    let work = args["mangaId"].as_str().ok_or("invalid_request")?;
    let cursor = &args["page"]["cursor"];
    let (page, offset, anchor) = if cursor.is_null() {
        (1, 0, None)
    } else {
        let mut fields = cursor.as_str().ok_or("invalid_request")?.splitn(3, ':');
        let page = fields
            .next()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|n| (1..=100_000).contains(n))
            .ok_or("invalid_request")?;
        let offset = fields
            .next()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|n| *n <= 20_000 && *n % BATCH == 0)
            .ok_or("invalid_request")?;
        let anchor = fields.next().filter(|s| slug(s));
        if anchor.is_none() || (page == 1 && offset == 0) {
            return Err("invalid_request");
        }
        (page, offset, anchor)
    };
    let cap = limit(&input);
    if page > cap {
        return Err("invalid_request");
    }
    let data = fetch_entry(&entry, page)?;
    if entry.kind == "chapters" {
        if !cursor.is_null() {
            return Err("invalid_request");
        }
        let id = entry.chapter_id();
        let uri = entry.url("")?;
        let words = Labels::new(METADATA, request)?.text("chapter.single", "Chapter")?;
        let tags = data["tags"].as_array().ok_or("parse_error")?;
        let group = text_values(tags, "Scanlator").join(", ");
        let mut chapter = Chapter::new(id.as_bytes(), work.as_bytes(), words.as_bytes());
        dates_and_group(&mut chapter, &data, &group);
        chapter.url = Some(uri.as_str().as_bytes());
        chapter.page_count = Some(chapter_pages(&data)?.len());
        return ChapterResponse::single(payload_buf(), chapter).map_err(port::chapter_error);
    }
    let count = provider_page(&data, page)?;
    if page > count {
        return Err("catalog_changed");
    }
    let rows = data["taggings"].as_array().ok_or("parse_error")?;
    let doujin = entry.kind == "doujins";
    let mut section = None;
    let mut seen = BTreeSet::new();
    let mut records = Vec::new();
    for row in rows {
        if host::check_cancel() {
            return Err("cancelled");
        }
        if let Some(header) = row.get("header") {
            section = nonempty(header);
            continue;
        }
        let name = nonempty(&row["permalink"]).ok_or("parse_error")?;
        if seen.insert(name) {
            records.push((name, section, row));
        }
    }
    if !doujin {
        records.reverse();
    }
    if let Some(anchor) = anchor {
        if offset > 0 {
            if records.get(offset - 1).map(|row| row.0) != Some(anchor) {
                return Err("catalog_changed");
            }
        } else if seen.contains(anchor) {
            return Err("catalog_changed");
        }
    }
    let exact = args["chapterId"].as_str();
    let mut out = GroupedChapters::new(payload_buf(), request).map_err(port::chapter_error)?;
    let mut last = None;
    for (index, (name, section, row)) in records.iter().enumerate().skip(offset).take(BATCH) {
        last = Some(*name);
        let chapter_entry = Entry::new("chapters", name)?;
        let id = chapter_entry.chapter_id();
        if exact.is_some_and(|exact| exact != id) {
            continue;
        }
        let uri = chapter_entry.url("")?;
        let title = nonempty(&row["title"]).ok_or("parse_error")?;
        let tags = row["tags"].as_array().ok_or("parse_error")?;
        let scanlator = text_values(tags, "Scanlator").join(", ");
        let mut title = section.map_or_else(
            || String::from(title),
            |section| format!("{section} {title}"),
        );
        if entry.kind != "series" {
            let authors = text_values(tags, "Author");
            if !authors.is_empty() {
                title.push_str(" — ");
                title.push_str(&authors.join(", "));
            }
        }
        let mut chapter = Chapter::new(id.as_bytes(), work.as_bytes(), title.as_bytes());
        dates_and_group(&mut chapter, row, &scanlator);
        chapter.url = Some(uri.as_str().as_bytes());
        // A doujin franchise lists unrelated stories by release date. Each is
        // selectable, but automatic continuation must stay within real series.
        let placement = if doujin {
            ChapterPlacement::ManualOnly
        } else {
            ChapterPlacement::InGroup {
                group: b"chapters",
                position: ((page - 1) * 20_000 + records.len() - index - 1) as u64,
            }
        };
        out.push(chapter, placement).map_err(port::chapter_error)?;
        if exact.is_some() {
            return out
                .finish(ChapterPage::Complete)
                .map_err(port::chapter_error);
        }
    }
    let next = if offset + BATCH < records.len() {
        Some(format!(
            "{page}:{}:{}",
            offset + BATCH,
            last.ok_or("catalog_changed")?
        ))
    } else if page < count.min(cap) {
        Some(format!("{}:0:{}", page + 1, last.ok_or("catalog_changed")?))
    } else {
        None
    };
    out.finish(next.as_ref().map_or(ChapterPage::Complete, |next| {
        ChapterPage::NextCursor(next.as_bytes())
    }))
    .map_err(port::chapter_error)
}

fn pages(input: &Value) -> Result<Value> {
    let chapter = input["args"]["chapterId"]
        .as_str()
        .ok_or("invalid_request")?;
    let slug = chapter
        .strip_prefix("dynasty-ch:")
        .ok_or("invalid_request")?;
    let entry = Entry::new("chapters", slug)?;
    let data = fetch_entry(&entry, 1)?;
    let pages = chapter_pages(&data)?
        .iter()
        .enumerate()
        .map(|(index, page)| {
            Ok(port::page(
                index,
                chapter,
                image(nonempty(&page["url"]).ok_or("parse_error")?)?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(json!({"chapterId":chapter,"pages":pages}))
}

fn filters(labels: &Labels<'_>, request: &[u8]) -> Result<Value> {
    let sorts = SORTS
        .iter()
        .map(|(id, label)| {
            Ok(json!({"value":id,"label":labels.text(&format!("sort.{label}"),label)?}))
        })
        .collect::<Result<Vec<_>>>()?;
    let types = KINDS
        .iter()
        .map(|(id, kind)| {
            Ok(json!({"value":kind,"label":labels.text(&format!("listing.{id}"),kind)?}))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut filters = alloc::vec![
        json!({"id":"sort","type":"select","default":"_smart_","name":labels.text("filter.sort","Sort")?,"options":sorts}),
        json!({"id":"type","type":"multiselect","default":KINDS.iter().map(|(_, kind)| *kind).collect::<Vec<_>>(),
            "name":labels.text("filter.type","Work type")?,"options":types}),
    ];
    if source_protocol::feature(request, TRISTATE) {
        let tags = tags_data()?;
        let options = tags
            .as_array()
            .ok_or("internal_error")?
            .iter()
            .map(|tag| {
                Ok(
                    json!({"value":tag["id"].as_u64().ok_or("internal_error")?.to_string(),
                "label":nonempty(&tag["name"]).ok_or("internal_error")?}),
                )
            })
            .collect::<Result<Vec<_>>>()?;
        filters.push(json!({"id":"tags","type":"tristate","default":[],"name":labels.text("filter.tags","Tags")?,"options":options}));
    }
    for (id, name) in PEOPLE {
        filters.push(json!({"id":id,"type":"text","default":"","name":labels.text(&format!("filter.{id}"),name)?}));
    }
    if source_protocol::feature(request, b"source_filter_scope_header_v1") {
        for filter in &mut filters {
            filter["scope"] = json!("search");
        }
    }
    Ok(json!({"useSearchForFilters":true,"filters":filters}))
}

fn settings(labels: &Labels<'_>) -> Result<Value> {
    let options = LIMITS
        .iter()
        .map(|id| Ok(json!({"value":id,"label":labels.text(&format!("setting.pages_{id}"),id)?})))
        .collect::<Result<Vec<_>>>()?;
    Ok(
        json!({"settings":[{"id":"chapterFetchLimit","kind":"select","default":"2","options":options,
        "name":labels.text("setting.chapter_limit","Chapter catalog limit")?,
        "description":labels.text("setting.chapter_limit_description","Limits website catalog pages.")?}]}),
    )
}

fn run(op: &str, request: &[u8], f: impl FnOnce(&Value) -> Result<Value>) -> u32 {
    port_catalog::write_localized(
        METADATA,
        request,
        op,
        port::input(request).and_then(|input| f(&input)),
    )
}
fn run_search(req: &[u8]) -> u32 {
    run("search", req, |input| search(input, req))
}
fn run_get_manga_list(req: &[u8]) -> u32 {
    run("get_manga_list", req, |input| list(input, req))
}
fn run_get_manga(req: &[u8]) -> u32 {
    run("get_manga", req, detail)
}
fn run_get_pages(req: &[u8]) -> u32 {
    run("get_pages", req, pages)
}
fn run_get_listings(req: &[u8]) -> u32 {
    run("get_listings", req, |_| {
        let labels = Labels::new(METADATA, req)?;
        let listings = LISTINGS.iter().map(|id| {
            Ok(json!({"id":id,"name":labels.text(&format!("listing.{id}"),id)?,"supportsPaging":*id != "popular","supportsFilters":false}))
        }).collect::<Result<Vec<_>>>()?;
        Ok(json!({"listings":listings}))
    })
}
fn run_get_filters(req: &[u8]) -> u32 {
    run("get_filters", req, |_| {
        filters(&Labels::new(METADATA, req)?, req)
    })
}
fn run_get_settings(req: &[u8]) -> u32 {
    run("get_settings", req, |_| {
        settings(&Labels::new(METADATA, req)?)
    })
}
fn run_get_home(req: &[u8]) -> u32 {
    run("get_home", req, |_| Err("not_implemented"))
}
fn run_get_image_request(req: &[u8]) -> u32 {
    run("get_image_request", req, |_| Err("not_implemented"))
}
fn run_get_chapters(req: &[u8]) -> ChapterResult<'static, (&'static str, String)> {
    chapters(req).map_err(|code| (code, port_catalog::failure(METADATA, req, code)))
}
fn write_chapter_failure((code, message): (&str, String)) -> u32 {
    write_error("get_chapters", code, &message)
}

koma_source_sdk::koma_source_exports!("dynasty_en", typed_chapters, write_chapter_failure);
