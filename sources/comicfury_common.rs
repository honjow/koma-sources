// Opt-in implementation for Wave 27 only; published Comic Fury en/zh remain independent.
// Adapted from Keiyoushi ComicFury.kt, Apache-2.0, revision 90b6e6a905ca186945b21ec0b0be4ca9135b8e69.
// Preserve archive order/identity and page ownership, content-warning boundaries,
// default filter levels and capability-gated author notes from the accepted en source.
extern crate alloc;

use alloc::{collections::BTreeSet, format, string::String, vec::Vec};
use koma_source_sdk::{
    chapters::{
        Chapter, ChapterNumber, ChapterPage, ChapterPlacement, ChapterResult, GroupedChapters,
    },
    host,
    localization::Catalog,
    runtime::Context,
    source::{SourceCapabilities, SourceInfo},
};
use serde_json::{json, Value};
use url::Url;

#[path = "port_support.rs"]
#[allow(dead_code)]
mod port_support;
use port_support::{self as port, Node, Result};

#[global_allocator]
static ALLOCATOR: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

const SITE: &str = "https://comicfury.com";
const PROFILE: &str = "/comicprofile.php?url=";
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
    image_request: true,
};
const CHAPTER_BYTES: usize = 440 * 1024;
const ARCHIVE_PAGE_LIMIT: usize = 128;
koma_source_sdk::koma_source_buffers! {
    payload: CHAPTER_BYTES, http_out: 2 * 1024 * 1024, body: 1024 * 1024,
    http_req: 8192, scratch: 8192,
}
koma_source_sdk::koma_source_helpers!();

type Options = &'static [&'static str];
const SORT: Options = &["filter.relevance", "filter.popularity", "filter.updated"];
const LAST_UPDATE: Options = &[
    "filter.alltime",
    "filter.week",
    "filter.month",
    "filter.year",
    "filter.completed_only",
];
const VIOLENCE: Options = &["filter.none_minimal", "filter.violent", "filter.gore"];
const FREQUENCY: Options = &["filter.none", "filter.occasional", "filter.frequent"];
const SEXUAL: Options = &[
    "filter.no_sexual",
    "filter.situations",
    "filter.strong_sexual",
];
const FILTERS: &[(&str, &str, Options, Option<&str>)] = &[
    ("sort", "filter.sort", SORT, None),
    ("lastupdate", "filter.lastupdate", LAST_UPDATE, Some("0")),
    ("fv", "filter.violence", VIOLENCE, Some("2")),
    ("fn", "filter.nudity", FREQUENCY, Some("2")),
    ("fl", "filter.strong_language", FREQUENCY, Some("2")),
    ("fs", "filter.sexual", SEXUAL, Some("2")),
];

fn slug_valid(slug: &str) -> bool {
    !slug.is_empty()
        && slug.len() <= 128
        && slug
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}

fn profile_slug(id: &str) -> Result<&str> {
    id.strip_prefix(PROFILE)
        .filter(|slug| slug_valid(slug))
        .ok_or("invalid_request")
}

fn required_work(input: &Value) -> Result<(&str, &str)> {
    let id = input["args"]["mangaId"].as_str().ok_or("invalid_request")?;
    Ok((id, profile_slug(id)?))
}

fn decimal(raw: &str) -> Option<usize> {
    (!raw.is_empty() && raw.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| raw.parse().ok())
        .flatten()
}

fn comic_number(raw: &str) -> Option<usize> {
    (!raw.starts_with('0')).then(|| decimal(raw)).flatten()
}

fn validated_image(location: &str) -> Result<Url> {
    let url = Url::parse(location).map_err(|_| "invalid_request")?;
    if url.scheme() != "https"
        || !matches!(url.host_str(), Some("comicfury.com" | "img.comicfury.com"))
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.fragment().is_some()
        || url.path() == "/"
    {
        return Err("invalid_request");
    }
    Ok(url)
}

fn resolve_image(base: &str, raw: &str) -> Result<Url> {
    let url = port::absolute(base, raw)?;
    validated_image(url.as_str()).map_err(|_| "parse_error")
}

fn headers() -> Value {
    json!({"Referer":"https://comicfury.com/","Origin":SITE})
}

fn cover(base: &str, raw: &str) -> Result<Value> {
    Ok(json!({"kind":"url","url":resolve_image(base,raw)?.as_str(),"headers":headers()}))
}

fn document(html: &str) -> Result<Node> {
    let document = Node::parse(html)?;
    if document
        .one("form input[name=proceed][value='View Webcomic']")
        .is_some()
    {
        return Err("chapter_locked");
    }
    Ok(document)
}

fn fetch(url: &str) -> Result<Node> {
    document(&port::get(url)?)
}

fn option<'a>(filters: &'a Value, key: &str, max: usize, default: &'a str) -> Result<&'a str> {
    let Some(value) = filters.get(key).filter(|value| !value.is_null()) else {
        return Ok(default);
    };
    value
        .as_str()
        .filter(|raw| raw.len() == 1 && decimal(raw).is_some_and(|value| value < max))
        .ok_or("invalid_request")
}

fn browse(input: &Value, search: bool) -> Result<Value> {
    let args = &input["args"];
    let filters = &args["filters"];
    let page = match &args["page"]["cursor"] {
        Value::Null => 1,
        cursor => cursor
            .as_str()
            .and_then(comic_number)
            .filter(|page| *page <= 10000)
            .ok_or("invalid_request")?,
    };
    let query = if search {
        args["query"].as_str().unwrap_or("")
    } else {
        ""
    };
    let sort = if search {
        "0"
    } else {
        match args["listingId"].as_str() {
            None | Some("popular") => "1",
            Some("latest") => "2",
            _ => return Err("invalid_request"),
        }
    };
    let tags = match &filters["tags"] {
        Value::Null => "",
        value => value.as_str().ok_or("invalid_request")?,
    };
    let completed = match &filters["completed"] {
        Value::Null | Value::Bool(false) => "1",
        Value::Bool(true) => "0",
        _ => return Err("invalid_request"),
    };
    let mut url = Url::parse("https://comicfury.com/search.php").map_err(|_| "internal_error")?;
    {
        let mut params = url.query_pairs_mut();
        params
            .append_pair("query", query)
            .append_pair("page", &format!("{page}"))
            .append_pair("language", site_lang())
            .append_pair("tags", &tags.replace(", ", ","))
            .append_pair("completed", completed);
        for (id, _, values, default) in FILTERS {
            params.append_pair(
                id,
                option(filters, id, values.len(), default.unwrap_or(sort))?,
            );
        }
    }
    let doc = fetch(url.as_str())?;
    let mut items = Vec::new();
    for card in doc.all("div.webcomic-result")? {
        let link = card.require("div.webcomic-result-avatar a")?;
        let id = link.attr("href")?;
        profile_slug(&id).map_err(|_| "parse_error")?;
        let image = link.require("img")?.attr("src")?;
        let title = card.require("div.webcomic-result-title")?.attr("title")?;
        if title.is_empty() {
            return Err("parse_error");
        }
        items.push(
            json!({"id":id,"title":title,"cover":cover(SITE,&image)?,"authors":[],
            "status":"unknown","language":LANG,"contentRating":"unknown"}),
        );
    }
    if items.is_empty() {
        let empty = doc
            .one("form.search-wrap div.search-results > div.search-error")
            .ok_or("parse_error")?
            .text()?;
        if empty != "There were no results for your search!" {
            return Err("parse_error");
        }
    }
    let next = doc
        .one("div.search-next-page")
        .is_some()
        .then_some(page + 1);
    Ok(port::items(items, next))
}

fn texts(doc: &Node, selector: &str) -> Result<Vec<String>> {
    doc.all(selector)?.iter().map(Node::text).collect()
}

fn detail(input: &Value) -> Result<Value> {
    let (id, slug) = required_work(input)?;
    let url = format!("{SITE}{id}");
    let doc = fetch(&url)?;
    let profile = doc.require("div.profilewrap > div.profile")?;
    let reader = profile
        .require("span.mobile-webcomic-link > a")?
        .attr("href")?;
    if reader != format!("/read/{slug}") {
        return Err("parse_error");
    }
    let title = profile
        .require(".username-and-title div.authorname")?
        .text()?;
    let avatar = match profile.one("div.profile-avatar img") {
        Some(img) => cover(SITE, &img.attr("src")?)?,
        None => Value::Null,
    };
    let description = if let Some(category) = profile
        .all("div.pccontent")?
        .into_iter()
        .find(|node| node.one("div.description-tags").is_some())
    {
        let full = category.text()?;
        let tags = category.require("div.description-tags")?.text()?;
        String::from(full.strip_suffix(&tags).unwrap_or(&full).trim())
    } else if let Some(em) = profile.one("div.username-and-title em") {
        em.text()?
    } else {
        String::new()
    };
    let mut tags = texts(&profile, "div.description-tags > a")?;
    if tags.is_empty() {
        for block in profile.all("div.authorinfo")? {
            if block.text()?.starts_with("Genre") {
                tags.extend(texts(&block, "a")?);
            }
        }
    }
    Ok(json!({"manga": {
        "id":id,"title":title,"cover":avatar,"description":description,
        "authors":texts(&profile,"a.authorname")?,"tags":tags,
        "status":"unknown","language":LANG,"contentRating":"unknown",
        "links":[{"kind":"source","url":url}]
    }}))
}

struct Comic {
    id: String,
    title: String,
    published_at: Option<String>,
}

fn published_date(text: &str) -> Option<String> {
    let date = text.split_once(':').map_or(text, |(date, _)| {
        date.rsplit_once(',').map_or(date, |(date, _)| date)
    });
    let date = date.replace(',', "");
    let date = date.trim();
    let (year, month, day) = if date.contains('.') {
        let mut parts = date.split('.');
        let (day, month, year) = (parts.next()?, parts.next()?, parts.next()?);
        if parts.next().is_some() {
            return None;
        }
        (decimal(year)?, decimal(month)?, decimal(day)?)
    } else {
        let mut parts = date.split_whitespace();
        let (first, second, year) = (parts.next()?, parts.next()?, parts.next()?);
        if parts.next().is_some() {
            return None;
        }
        let (month, day) = if first.starts_with(|c: char| c.is_ascii_digit()) {
            (second, first)
        } else {
            (first, second)
        };
        let day = day.trim_end_matches(|c: char| c.is_ascii_alphabetic());
        let month = month.get(..3)?;
        let month = [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ]
        .iter()
        .position(|value| value.eq_ignore_ascii_case(month))?
            + 1;
        (decimal(year)?, month, decimal(day)?)
    };
    if !(1900..=9999).contains(&year) || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some(format!("{year:04}-{month:02}-{day:02}T00:00:00Z"))
}

fn archive_url(base: &str, target: &str, slug: &str) -> Result<Url> {
    let url = port::absolute(base, target)?;
    let prefix = format!("/read/{slug}/archive");
    let suffix = url.path().strip_prefix(&prefix).ok_or("parse_error")?;
    if url.scheme() != "https"
        || url.host_str() != Some("comicfury.com")
        || url.port().is_some()
        || !(suffix.is_empty() || suffix.starts_with('/'))
    {
        return Err("parse_error");
    }
    Ok(url)
}

fn default_comic(slug: &str, href: &str) -> Option<usize> {
    let prefix = format!("/read/{slug}/comics/");
    href.strip_prefix(SITE)
        .unwrap_or(href)
        .strip_prefix(&prefix)
        .and_then(comic_number)
}

fn archive_group(
    start: Url,
    doc: Node,
    slug: &str,
    group: &str,
    comics: &mut Vec<Comic>,
) -> Result<()> {
    let mut current = Some(doc);
    let mut url = start;
    let mut visited = BTreeSet::new();
    loop {
        if host::check_cancel() {
            return Err("cancelled");
        }
        if visited.len() >= ARCHIVE_PAGE_LIMIT || !visited.insert(String::from(url.as_str())) {
            return Err("parse_error");
        }
        let doc = match current.take() {
            Some(doc) => doc,
            None => fetch(url.as_str())?,
        };
        for link in doc.all(".archive-comics a[href]")? {
            let Some(name) = link.one(".archive-comic-title") else {
                continue;
            };
            let href = link.attr("href")?;
            let number = default_comic(slug, &href).ok_or("parse_error")?;
            let name = name.text()?;
            if name.is_empty() {
                return Err("parse_error");
            }
            let title = if group.is_empty() {
                name
            } else {
                format!("{group} - {name}")
            };
            let date = link
                .one(".archive-comic-date")
                .map(|node| node.text())
                .transpose()?;
            comics.push(Comic {
                id: format!("/read/{slug}/comics/{number}"),
                title,
                published_at: date.as_deref().and_then(published_date),
            });
        }
        let Some(next) = doc.one("span.vfpagecurrent + a.vfpage") else {
            break;
        };
        url = archive_url(url.as_str(), &next.attr("href")?, slug)?;
    }
    Ok(())
}

fn custom_url(location: &str, slug: &str) -> Result<Url> {
    let url = Url::parse(location).map_err(|_| "invalid_request")?;
    let host = url.host_str().ok_or("invalid_request")?;
    if url.scheme() != "https"
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || !["webcomic.ws", "thecomicseries.com", "comicfury.com"]
            .iter()
            .any(|domain| host == format!("{slug}.{domain}"))
        || url.path().len() > 1024
        || url.as_str().len() > 1500
        || url.path() == "/"
    {
        return Err("invalid_request");
    }
    Ok(url)
}

fn custom_comics(slug: &str) -> Result<Vec<Comic>> {
    let archive = format!("https://{slug}.webcomic.ws/archive/comics");
    let doc = fetch(&archive)?;
    let mut comics = Vec::new();
    for item in doc.all("div.archivecomic, div.nl-archivecomic")? {
        let link = item.require("a")?;
        let url = port::absolute(&archive, &link.attr("href")?)?;
        let url = custom_url(url.as_str(), slug).map_err(|_| "parse_error")?;
        let title = link.text()?;
        if title.is_empty() {
            return Err("parse_error");
        }
        let date = item
            .one(".comicposttime, .nl-archivecomicposttime")
            .map(|node| node.text())
            .transpose()?;
        comics.push(Comic {
            id: format!("custom:{slug}:{url}"),
            title,
            published_at: date.as_deref().and_then(published_date),
        });
    }
    Ok(comics)
}

fn archive(slug: &str) -> Result<Vec<Comic>> {
    let first = archive_url(SITE, &format!("/read/{slug}/archive"), slug)?;
    let doc = fetch(first.as_str())?;
    let mut groups = Vec::new();
    for anchor in doc.all("#scroll-content a[href]")? {
        if let Some(group) = anchor.one(".archive-chapter") {
            let url = archive_url(first.as_str(), &anchor.attr("href")?, slug)?;
            let header = group
                .one(".archive-chapter-title")
                .map_or_else(|| group.text(), |node| node.text())?;
            groups.push((url, header));
        }
    }
    let mut comics = Vec::new();
    if groups.is_empty() {
        archive_group(first, doc, slug, "", &mut comics)?;
    } else {
        drop(doc);
        for (url, group) in groups {
            let doc = fetch(url.as_str())?;
            archive_group(url, doc, slug, &group, &mut comics)?;
        }
    }
    if comics.is_empty() {
        comics = custom_comics(slug)?;
    }
    if comics.is_empty() {
        return Err("chapter_unavailable");
    }
    let mut seen = BTreeSet::new();
    comics.retain(|comic| seen.insert(comic.id.clone()));
    Ok(comics)
}

fn viewer_url(id: &str, slug: &str) -> Result<String> {
    if default_comic(slug, id).is_some() {
        return Ok(format!("{SITE}{id}"));
    }
    let url = id
        .strip_prefix("custom:")
        .and_then(|rest| rest.split_once(':'))
        .filter(|(owner, _)| *owner == slug)
        .map(|(_, url)| url)
        .ok_or("invalid_request")?;
    custom_url(url, slug).map(|url| String::from(url.as_str()))
}

fn run_get_chapters(request: &[u8]) -> ChapterResult<'static, &'static str> {
    let input = port::input(request)?;
    let (id, slug) = required_work(&input)?;
    let comics = archive(slug)?;
    let end = match &input["args"]["page"]["cursor"] {
        Value::Null => comics.len(),
        cursor => comics
            .iter()
            .position(|comic| cursor.as_str() == Some(comic.id.as_str()))
            .ok_or("invalid_request")?,
    };
    let mut output = GroupedChapters::new(payload_buf(), request).map_err(port::chapter_error)?;
    let mut budget = CHAPTER_BYTES - 16 * 1024;
    let mut last = None;
    for (position, comic) in comics[..end].iter().enumerate().rev() {
        let url = viewer_url(&comic.id, slug)?;
        // Chapter identifiers and resolved URLs are ASCII and do not escape.
        // The SDK writes at most two bytes for each title byte.
        let row = 350 + comic.id.len() + id.len() + 2 * comic.title.len() + url.len();
        if row > budget {
            let previous: &Comic = last.ok_or("result_too_large")?;
            return output
                .finish(ChapterPage::NextCursor(previous.id.as_bytes()))
                .map_err(port::chapter_error);
        }
        let mut chapter = Chapter::new(comic.id.as_bytes(), id.as_bytes(), comic.title.as_bytes());
        chapter.chapter_number = Some(ChapterNumber::Integer(position));
        chapter.published_at = comic.published_at.as_deref().map(str::as_bytes);
        chapter.language = Some(LANG_BYTES);
        chapter.url = Some(url.as_bytes());
        output
            .push(
                chapter,
                ChapterPlacement::InGroup {
                    group: b"archive",
                    position: position as u64,
                },
            )
            .map_err(port::chapter_error)?;
        last = Some(comic);
        budget -= row;
    }
    output
        .finish(ChapterPage::Complete)
        .map_err(port::chapter_error)
}

fn supports_notes(request: &[u8]) -> bool {
    Context::from_request(request).supports(b"text_page_image_v1")
}

fn notes_enabled(input: &Value, request: &[u8]) -> bool {
    let setting = &input["settings"]["showAuthorsNotes"];
    supports_notes(request) && (setting.as_bool() == Some(true) || setting.as_str() == Some("true"))
}

// The HTML host joins text nodes. Add separators at the HTML block boundaries
// before it decodes entities so a note keeps its visible paragraphs.
fn note_markup(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(start) = rest.find('<') {
        let Some(end) = rest[start..].find('>') else {
            break;
        };
        let end = start + end + 1;
        let tag = &rest[start + 1..end - 1];
        let name = tag
            .trim_start_matches('/')
            .split(|c: char| c.is_ascii_whitespace() || c == '/')
            .next()
            .unwrap_or("");
        result.push_str(&rest[..end]);
        if ["br", "p", "div", "li", "blockquote"]
            .iter()
            .any(|value| name.eq_ignore_ascii_case(value))
        {
            result.push_str("&#57344;");
        }
        rest = &rest[end..];
    }
    result.push_str(rest);
    result
}

fn note_text(node: &Node) -> Result<Vec<String>> {
    Ok(node
        .text()?
        .split('\u{e000}')
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(String::from)
        .collect())
}

fn pages(input: &Value, request: &[u8]) -> Result<Value> {
    let args = &input["args"];
    let id = args["chapterId"].as_str().ok_or("invalid_request")?;
    let slug = if let Some(path) = id.strip_prefix("/read/") {
        path.split_once('/').map(|(slug, _)| slug)
    } else {
        id.strip_prefix("custom:")
            .and_then(|path| path.split_once(':'))
            .map(|(slug, _)| slug)
    }
    .filter(|slug| slug_valid(slug))
    .ok_or("invalid_request")?;
    if let Some(work) = args.get("mangaId") {
        if work.as_str().and_then(|id| profile_slug(id).ok()) != Some(slug) {
            return Err("invalid_request");
        }
    }
    let url = viewer_url(id, slug)?;
    let html = port::get(&url)?;
    let doc = document(&html)?;
    let number = default_comic(slug, id);
    let owner = if let Some(number) = number {
        let owner = doc
            .require("div.is--comic-page")
            .map_err(|_| "chapter_unavailable")?;
        if owner
            .attr("data-comicid")
            .ok()
            .and_then(|raw| comic_number(&raw))
            != Some(number)
        {
            return Err("parse_error");
        }
        Some(owner)
    } else {
        None
    };
    let images = if let Some(owner) = &owner {
        owner.all("div.is--image-segment div img")?
    } else {
        doc.all("img#comicimage")?
    };
    if images.is_empty() {
        return Err("chapter_unavailable");
    }
    let mut pages = Vec::new();
    for (index, image) in images.iter().enumerate() {
        let image = resolve_image(&url, &image.attr("src")?)?;
        pages.push(port::page(index, id, image));
    }
    if number.is_some() && notes_enabled(input, request) {
        let notes_doc = document(&note_markup(&html))?;
        let owner = notes_doc.require("div.is--comic-page")?;
        if owner
            .attr("data-comicid")
            .ok()
            .and_then(|raw| comic_number(&raw))
            != number
        {
            return Err("parse_error");
        }
        for (i, note) in owner
            .all(".is--author-notes .is--comment-box")?
            .iter()
            .enumerate()
        {
            let Some(content) = note.one(".is--comment-content") else {
                continue;
            };
            let paragraphs = note_text(&content)?;
            if paragraphs.is_empty() {
                continue;
            }
            let mut text = json!({"template":"webtoon-author-notes","paragraphs":paragraphs});
            if let Some(author) = note.one("a.is--comment-author") {
                let author = author.text()?;
                if !author.is_empty() {
                    text["heading"] =
                        json!(label(request, "chapter.author_notes")?.replace("{author}", &author));
                }
            }
            pages.push(
                json!({"id":format!("{id}:author-note:{i}"),"index":pages.len(),
                "image":{"kind":"text","text":text}}),
            );
        }
    }
    Ok(json!({"chapterId":id,"pages":pages}))
}

fn label(request: &[u8], key: &str) -> Result<String> {
    let context = Context::from_request(request);
    let catalog = Catalog::from_metadata(METADATA).map_err(|_| "internal_error")?;
    let mut buffer = [0; 1024];
    catalog
        .text(&context, key, key, &mut buffer)
        .map(String::from)
        .map_err(|_| "internal_error")
}
fn failure(request: &[u8], operation: &str, code: &str) -> u32 {
    let key = format!("error.{code}");
    let message = label(request, &key)
        .ok()
        .filter(|value| value != &key)
        .or_else(|| label(request, "error.unavailable").ok())
        .unwrap_or_else(|| String::from("This content is temporarily unavailable."));
    write_error(operation, code, &message)
}
fn write_result(request: &[u8], operation: &str, result: Result<Value>) -> u32 {
    match result {
        Ok(value) => match serde_json::to_vec(&value) {
            Ok(body) if body.len() <= payload_buf().len() => {
                payload_buf()[..body.len()].copy_from_slice(&body);
                write_success_payload(operation, body.len())
            }
            _ => failure(request, operation, "result_too_large"),
        },
        Err(code) => failure(request, operation, code),
    }
}

fn labels(request: &[u8], operation: &str) -> Result<Value> {
    port::input(request)?;
    let context = Context::from_request(request);
    let catalog = Catalog::from_metadata(METADATA).map_err(|_| "internal_error")?;
    let label = |key: &str| {
        let mut buffer = [0; 512];
        catalog
            .text(&context, key, key, &mut buffer)
            .map(String::from)
            .map_err(|_| "internal_error")
    };
    match operation {
        "get_listings" => Ok(json!({"listings":[
            {"id":"popular","kind":"popular","name":label("listing.popular")?,"supportsPaging":true,"supportsFilters":true},
            {"id":"latest","kind":"latest","name":label("listing.latest")?,"supportsPaging":true,"supportsFilters":true}
        ]})),
        "get_settings" if supports_notes(request) => Ok(json!({"settings":[{
            "id":"showAuthorsNotes","kind":"toggle","default":"false",
            "name":label("setting.notes")?,"description":label("setting.notes.description")?
        }]})),
        "get_settings" => Ok(json!({"settings":[]})),
        _ => {
            let mut filters =
                alloc::vec![json!({"id":"tags","name":label("filter.tags")?,"type":"text"})];
            for (id, key, values, default) in FILTERS {
                let options = values
                    .iter()
                    .enumerate()
                    .map(|(index, key)| Ok(json!({"value":format!("{index}"),"label":label(key)?})))
                    .collect::<Result<Vec<_>>>()?;
                let mut filter =
                    json!({"id":id,"name":label(key)?,"type":"select","options":options});
                if let Some(default) = default {
                    filter["default"] = json!(default);
                }
                filters.push(filter);
                if *id == "lastupdate" {
                    filters.push(json!({"id":"completed","name":label("filter.completed")?,"type":"check","default":false}));
                }
            }
            Ok(json!({"filters":filters}))
        }
    }
}

fn run_get_listings(request: &[u8]) -> u32 {
    write_result(request, "get_listings", labels(request, "get_listings"))
}
fn run_get_filters(request: &[u8]) -> u32 {
    write_result(request, "get_filters", labels(request, "get_filters"))
}
fn run_get_settings(request: &[u8]) -> u32 {
    write_result(request, "get_settings", labels(request, "get_settings"))
}
fn run_get_manga_list(request: &[u8]) -> u32 {
    write_result(
        request,
        "get_manga_list",
        port::input(request).and_then(|input| browse(&input, false)),
    )
}
fn run_search(request: &[u8]) -> u32 {
    write_result(
        request,
        "search",
        port::input(request).and_then(|input| browse(&input, true)),
    )
}
fn run_get_manga(request: &[u8]) -> u32 {
    write_result(
        request,
        "get_manga",
        port::input(request).and_then(|input| detail(&input)),
    )
}
fn run_get_pages(request: &[u8]) -> u32 {
    write_result(
        request,
        "get_pages",
        port::input(request).and_then(|input| pages(&input, request)),
    )
}
fn run_get_image_request(request: &[u8]) -> u32 {
    let result = port::input(request).and_then(|input| {
        let url = validated_image(input["args"]["url"].as_str().ok_or("invalid_request")?)?;
        Ok(json!({"url":url.as_str(),"headers":headers()}))
    });
    write_result(request, "get_image_request", result)
}
fn run_get_home(request: &[u8]) -> u32 {
    failure(request, "get_home", "unsupported")
}
// The typed chapter export keeps the original request for localized failures.
#[no_mangle]
pub extern "C" fn koma_source_get_chapters(req_ptr: u32, req_len: u32) -> u32 {
    let request = match read_request(req_ptr, req_len) {
        Some(request) => request,
        None => return failure(&[], "get_chapters", "invalid_request"),
    };
    host::log_info(EXPORT_NAME.as_bytes());
    match run_get_chapters(request) {
        Ok(chapters) => chapters.write(response_buffer()),
        Err(code) => failure(request, "get_chapters", code),
    }
}
export_source!();

// Keiyoushi ComicFury.siteLang: only the Brazilian Portuguese query uses "pt".
// Keep content language/identity as pt-BR; all existing adapters retain LANG.
fn site_lang() -> &'static str {
    if LANG == "pt-BR" {
        "pt"
    } else {
        LANG
    }
}
